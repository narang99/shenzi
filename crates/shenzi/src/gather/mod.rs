// use core::unimplemented;
// // given a shenzi manifest, gather all the nodes that we can discover
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use anyhow::{Context, Error, Result, anyhow, bail};
use log::{info, warn};
use walkdir::WalkDir;

mod error;

pub use crate::factory::NodeFactory;
pub use crate::site_pkgs::PythonPathComponent;

use crate::{
    factory::Factory,
    gather::error::MultipleGatherErrors,
    graph::FileGraph,
    manifest::{LoadKind, ShenziManifest},
    node::{Node, deps::Deps},
    parse::{ErrDidNotFindDependencies, ErrDidNotFindDependency},
    paths::{
        file_name_as_str, marker_file_name, normalize_path,
        split_colon_separated_into_valid_search_paths,
    },
    site_pkgs::{PyPackage, SitePkgs, normalize_package_name},
    warnings::Warning,
};

pub fn build_graph_from_manifest(
    manifest: &ShenziManifest,
    cwd: &PathBuf,
) -> Result<(
    FileGraph<NodeFactory>,
    Vec<PythonPathComponent>,
    Vec<Warning>,
)> {
    let site_pkgs = SitePkgs::from_manifest(manifest);
    let factory = NodeFactory::new(
        site_pkgs.clone(),
        manifest.python.sys.version.clone(),
        manifest.python.sys.executable.clone(),
        cwd.clone(),
        manifest.env.clone(),
        manifest.skip.clone(),
    );
    let (g, warnings) = build_graph(manifest, &factory, &site_pkgs)?;

    Ok((g, site_pkgs.comps, warnings))
}

fn build_graph(
    manifest: &ShenziManifest,
    factory: &NodeFactory,
    site_pkgs: &SitePkgs,
) -> Result<(FileGraph<NodeFactory>, Vec<Warning>)> {
    let executable_path = &manifest.python.sys.executable;
    let known_libs = HashMap::new();
    let mut g = FileGraph::new(factory.clone());
    info!("Build graph: pass 1, begin");

    // first add the py executable and its whole tree, should not fail
    info!(
        "adding python executable, path={}",
        executable_path.display()
    );
    // extra search paths for executable will be empty
    // always serial
    g.add_tree(
        factory.make_py_executable(executable_path)?,
        &known_libs,
        true,
        &Vec::new(),
    )?;

    let executable_extra_paths_to_search = g
        .get_node_by_path(executable_path)
        .context(anyhow!(
            "fatal error: did not find node for successfully inserted python executable: path={}",
            executable_path.display()
        ))
        .unwrap()
        .deps
        .paths_to_add_for_next_search();

    // now add all loads, in the correct order, again, should not fail
    // always serial
    for l in &manifest.loads {
        info!(
            "adding load detected in manifest, path={}",
            l.path.display()
        );
        match l.kind {
            LoadKind::Dlopen => factory
                .make_with_symlinks(
                    &l.path,
                    &l.symlinks,
                    &known_libs,
                    &executable_extra_paths_to_search,
                )
                .and_then(|n| {
                    add_to_graph_if_some(
                        &mut g,
                        n,
                        &known_libs,
                        true,
                        &executable_extra_paths_to_search,
                    )
                })?,
            LoadKind::Extension => factory
                .make(&l.path, &known_libs, &executable_extra_paths_to_search)
                .and_then(|n| {
                    add_to_graph_if_some(
                        &mut g,
                        n,
                        &known_libs,
                        true,
                        &executable_extra_paths_to_search,
                    )
                })?,
        };
    }

    // binaries in shenzi.json cannot fail
    add_binaries(
        &mut g,
        &get_binaries_which_exist_in_path(manifest),
        factory,
        &known_libs,
        &executable_extra_paths_to_search,
    )?;

    let mut failures = Vec::new();
    // add exec prefix, can fail
    info!("adding stdlib, path={}", site_pkgs.lib_dynload.display());
    add_nodes_recursive(
        &mut g,
        &mut failures,
        &site_pkgs.lib_dynload,
        &factory,
        &known_libs,
        true,
        &executable_extra_paths_to_search,
    )?;

    // add prefix, can fail
    info!("adding stdlib, path={}", site_pkgs.stdlib.display());
    add_nodes_recursive(
        &mut g,
        &mut failures,
        &site_pkgs.stdlib,
        &factory,
        &known_libs,
        true,
        &executable_extra_paths_to_search,
    )?;

    // site-packages addition start
    // we only add the packages which are allowed

    let allowed_packages = get_normalized_allowed_packages(manifest);

    // now all site-packages, can fail
    for (pkg, _) in &site_pkgs.site_pkg_by_alias {
        info!("adding site-package: path={}", pkg.display());
        if pkg.exists() {
            // site-packages addition would replace
            add_site_packages(
                &mut g,
                &mut failures,
                pkg,
                &factory,
                &known_libs,
                true,
                &executable_extra_paths_to_search,
                &allowed_packages,
            )?;
        } else {
            info!(
                "site packages at path={} does not exist, skipping",
                pkg.display()
            );
        }
    }

    let warnings = add_failures(
        &mut g,
        failures,
        &factory,
        &executable_extra_paths_to_search,
    )?;

    Ok((g, warnings))
}

fn get_normalized_allowed_packages(manifest: &ShenziManifest) -> HashSet<String> {
    match manifest.python.allowed_packages {
        None => HashSet::new(),
        Some(ref pkgs) => pkgs.iter().map(|p| normalize_package_name(p)).collect(),
    }
}

fn add_failures(
    g: &mut FileGraph<NodeFactory>,
    failures: Vec<PathBuf>,
    factory: &NodeFactory,
    extra_search_paths: &Vec<PathBuf>,
) -> Result<Vec<Warning>> {
    // in each cycle, go through all the failures
    // add them to the graph
    // if any of them fail, keep them in the new failures vector
    // if failures do not decrease in a loop, then break and error out with all failures
    // else go to the next one with the new set of failures
    // if the failures are 0, break out
    // keep updating the known_libs values too
    // updating known_libs is a costly operation (it converts all paths stored in the graph to known_libs in every iteration)
    // TODO: fix known libs speed, mostly we should be able to fix it if the graph returns all the added nodes in add_tree instead of just the index
    let mut prev_failures: Vec<(PathBuf, Error)> = failures
        .iter()
        .map(|p| (p.clone(), anyhow!("unknown error")))
        .collect();
    let mut known_libs = get_libs_from_graph(g);

    let mut i = 0;

    let mut prev_len = prev_failures.len();
    while prev_len > 0 {
        i = i + 1;
        info!("adding failed nodes: Pass {}, length={}", i, prev_len);

        let mut new_failures: Vec<(PathBuf, anyhow::Error)> = Vec::new();

        // failures addition does not recursively replace stuff in the graph
        for (p, _) in prev_failures {
            let res = factory
                .make(&p, &known_libs, extra_search_paths)
                .and_then(|n| add_to_graph_if_some(g, n, &known_libs, false, extra_search_paths));
            if let Err(e) = res {
                new_failures.push((p, e));
            }
        }

        if new_failures.len() >= prev_len {
            return failures_to_error(new_failures);
        }

        prev_failures = new_failures;
        prev_len = prev_failures.len();

        known_libs = get_libs_from_graph(g);
    }
    Ok(vec![])
}

fn failures_to_error(failures: Vec<(PathBuf, anyhow::Error)>) -> Result<Vec<Warning>> {
    let mut dep_resolution_errs: Vec<ErrDidNotFindDependency> = Vec::new();
    let mut others: Vec<anyhow::Error> = Vec::new();
    for (_, e) in failures {
        match e.downcast_ref::<ErrDidNotFindDependency>() {
            Some(e) => dep_resolution_errs.push(e.clone()),
            None => others.push(e),
        };
    }

    let dep_res_err = ErrDidNotFindDependencies {
        causes: dep_resolution_errs,
    };
    if others.len() > 0 {
        others.push(anyhow::Error::from(dep_res_err));
        Err(anyhow::Error::from(MultipleGatherErrors { errors: others }))
    } else {
        let warnings: Vec<Warning> = dep_res_err
            .causes
            .into_iter()
            .map(|e| Warning::W001DependencyNotFound {
                dependency: e.name,
                path: e.lib,
            })
            .collect();
        Ok(warnings)
    }
}

fn get_binaries_which_exist_in_path(manifest: &ShenziManifest) -> Vec<PathBuf> {
    let paths = split_colon_separated_into_valid_search_paths(manifest.env.get("PATH"));

    let mut res = Vec::new();
    for bin in &manifest.bins {
        if bin.path.contains(std::path::MAIN_SEPARATOR) {
            // if there is a separator inside, its a relative path or an absolute path
            let mut p = PathBuf::from(bin.path.clone());
            if !p.is_absolute() {
                p = manifest.python.cwd.join(p);
            }
            if !p.exists() {
                warn!(
                    "could not find binary in shenzi.json, skipping path={}",
                    bin.path
                );
            } else {
                res.push(normalize_path(&p))
            }
        } else {
            // find in the path
            let mut found = false;
            for p in &paths {
                let candidate = p.join(&bin.path);
                if candidate.exists() {
                    found = true;
                    res.push(candidate);
                }
            }
            if !found {
                warn!(
                    "could not find binary in shenzi.json, skipping path={}",
                    bin.path
                );
            }
        }
    }

    res
}

fn add_binaries(
    g: &mut FileGraph<NodeFactory>,
    bins: &Vec<PathBuf>,
    factory: &NodeFactory,
    known_libs: &HashMap<String, PathBuf>,
    extra_search_paths: &Vec<PathBuf>,
) -> Result<()> {
    for bin in bins {
        factory
            .make_binary(&bin, &known_libs, &extra_search_paths)
            .with_context(|| anyhow!("failed in adding binary: {}", bin.display()))
            .and_then(|n| add_to_graph_if_some(g, n, &known_libs, true, &extra_search_paths))?;
    }

    Ok(())
}

fn add_site_packages(
    g: &mut FileGraph<NodeFactory>,
    failures: &mut Vec<PathBuf>,
    directory: &PathBuf,
    factory: &NodeFactory,
    known_libs: &HashMap<String, PathBuf>,
    replace: bool,
    extra_search_paths: &Vec<PathBuf>,
    allowed_packages: &HashSet<String>,
) -> Result<()> {
    // dist-info gives the exact files we should include
    let added_packages = add_using_dist_info(
        g,
        failures,
        directory,
        factory,
        known_libs,
        replace,
        extra_search_paths,
        allowed_packages,
    )?;

    // fallback, we include all directories which are not added by dist-info
    // have an __init__.py and are in allowed packages
    add_remaining_in_site_packages(
        g,
        failures,
        directory,
        factory,
        known_libs,
        replace,
        extra_search_paths,
        allowed_packages,
        &added_packages,
    )?;

    Ok(())
}

fn add_using_dist_info(
    g: &mut FileGraph<NodeFactory>,
    failures: &mut Vec<PathBuf>,
    directory: &PathBuf,
    factory: &NodeFactory,
    known_libs: &HashMap<String, PathBuf>,
    replace: bool,
    extra_search_paths: &Vec<PathBuf>,
    allowed_packages: &HashSet<String>,
) -> Result<HashSet<String>> {
    // we go through all folders directly inside this directory, get dist-info
    // ask dist-info what all we can add
    // then build from those paths
    let dist_infos = PyPackage::get_dist_infos_in_dir(directory)?;
    let mut added_packages = HashSet::new();
    for dist_info in dist_infos {
        let py_pkg =
            PyPackage::new(dist_info).context("failed in building PyPackage for dist_info")?;
        if py_pkg.should_include_in_dist(allowed_packages) {
            let paths = py_pkg.get_installed_files()?;
            build_graph_from_paths(
                paths,
                g,
                failures,
                factory,
                known_libs,
                replace,
                extra_search_paths,
            );
            let binaries = py_pkg.get_binaries()?;
            if binaries.len() > 0 {
                info!(
                    "found binaries in package (dist-info), dist-info={}",
                    py_pkg.dist_info().display()
                );
            }
            add_binaries(g, &binaries, factory, known_libs, extra_search_paths)?;
            added_packages.insert(py_pkg.normalized_name().to_string());
        } else {
            info!(
                "excluding site-package as its not in allowed packages, dist-info={}",
                py_pkg.dist_info().display()
            );
        }
    }

    Ok(added_packages)
}

fn add_remaining_in_site_packages(
    g: &mut FileGraph<NodeFactory>,
    failures: &mut Vec<PathBuf>,
    directory: &PathBuf,
    factory: &NodeFactory,
    known_libs: &HashMap<String, PathBuf>,
    replace: bool,
    extra_search_paths: &Vec<PathBuf>,
    allowed_packages: &HashSet<String>,
    added_packages: &HashSet<String>,
) -> Result<()> {
    for entry in std::fs::read_dir(directory)
        .with_context(|| format!("Failed to read directory: {}", directory.display()))?
    {
        let entry = entry.context("Failed to read a directory entry")?;
        let path = entry.path();
        if path.join("__init__.py").exists() {
            if let Ok(file_name) = file_name_as_str(&path) {
                let normalized = normalize_package_name(&file_name);
                if added_packages.contains(&normalized) {
                    continue;
                }
                if allowed_packages.contains(&normalized) {
                    add_nodes_recursive(
                        g,
                        failures,
                        &path,
                        factory,
                        known_libs,
                        replace,
                        extra_search_paths,
                    )?;
                } else {
                    info!("{} skipped (not allowed)", path.display());
                }
            }
        }
    }

    Ok(())
}

fn add_nodes_recursive(
    g: &mut FileGraph<NodeFactory>,
    failures: &mut Vec<PathBuf>,
    directory: &PathBuf,
    factory: &NodeFactory,
    known_libs: &HashMap<String, PathBuf>,
    replace: bool,
    extra_search_paths: &Vec<PathBuf>,
) -> Result<()> {
    if !directory.exists() {
        bail!(
            "fatal: tried finding nodes recursively for directory={}, but it does not exist",
            directory.display()
        );
    }
    let maybe_marker = directory.join(marker_file_name());
    if maybe_marker.exists() {
        warn!(
            "found marker file at {}. This is most likely the dist folder generated by shenzi. skipping",
            maybe_marker.display()
        );
        return Ok(());
    }
    let paths = get_paths_recursive_from_dir(directory)?;
    build_graph_from_paths(
        paths,
        g,
        failures,
        factory,
        known_libs,
        replace,
        extra_search_paths,
    );
    Ok(())
}

fn build_graph_from_paths(
    paths: Vec<PathBuf>,
    g: &mut FileGraph<NodeFactory>,
    failures: &mut Vec<PathBuf>,
    factory: &NodeFactory,
    known_libs: &HashMap<String, PathBuf>,
    replace: bool,
    extra_search_paths: &Vec<PathBuf>,
) {
    let mut i = 0;
    let total = paths.len();
    for p in paths {
        if !replace {
            if let Some(_) = g.get_node_by_path(&p) {
                // skip already done
                continue;
            }
        }
        let res = factory
            .make(&p, known_libs, extra_search_paths)
            .and_then(|n| add_to_graph_if_some(g, n, known_libs, replace, extra_search_paths));
        if let Err(_) = res {
            if let None = g.get_node_by_path(&p) {
                failures.push(p);
            }
        }
        i += 1;
        if total / 10 != 0 && i % (total / 10) == 0 {
            info!("graph: pass 1: {}/{} nodes", i, total);
        }
    }
}

fn add_to_graph_if_some(
    g: &mut FileGraph<NodeFactory>,
    maybe_node: Option<Node>,
    known_libs: &HashMap<String, PathBuf>,
    replace: bool,
    extra_search_paths: &Vec<PathBuf>,
) -> Result<()> {
    match maybe_node {
        Some(node) => {
            g.add_tree(node, known_libs, replace, extra_search_paths)?;
            Ok(())
        }
        None => Ok(()),
    }
}

fn get_libs_from_graph(g: &FileGraph<NodeFactory>) -> HashMap<String, PathBuf> {
    let mut known_libs = HashMap::new();
    for n in g.iter_nodes() {
        match n.deps {
            Deps::Plain => {}
            Deps::Binary(_) => {
                n.path
                    .file_name()
                    .and_then(|file_name| file_name.to_str())
                    .map(|f| f.to_string())
                    .map(|f| known_libs.insert(f, n.path.clone()));
            }
            #[cfg(test)]
            Deps::Mock { paths: _ } => {}
        };
    }
    known_libs
}

fn get_paths_recursive_from_dir(base_path: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for maybe_d in WalkDir::new(base_path).into_iter() {
        match maybe_d {
            Ok(d) => {
                let p = d.into_path();
                if p.is_file() {
                    paths.push(p);
                }
            }
            Err(e) => {
                return Err(e)?;
            }
        }
    }
    Ok(paths)
}
