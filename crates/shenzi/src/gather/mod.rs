// use core::unimplemented;
// // given a shenzi manifest, gather all the nodes that we can discover
use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Error, Result, anyhow, bail};
use log::{error, info};
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
    site_pkgs::SitePkgs,
};

pub fn build_graph_from_manifest(
    manifest: &ShenziManifest,
    cwd: &PathBuf,
) -> Result<(FileGraph<NodeFactory>, Vec<PythonPathComponent>)> {
    let site_pkgs = SitePkgs::from_manifest(manifest);
    let factory = NodeFactory::new(
        site_pkgs.clone(),
        manifest.python.sys.version.clone(),
        manifest.python.sys.executable.clone(),
        cwd.clone(),
        manifest.env.clone(),
        manifest.skip.clone(),
    );
    let g = build_graph(manifest, &factory, &site_pkgs)?;

    Ok((g, site_pkgs.comps))
}

fn build_graph(
    manifest: &ShenziManifest,
    factory: &NodeFactory,
    site_pkgs: &SitePkgs,
) -> Result<FileGraph<NodeFactory>> {
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

    // now all site-packages, can fail
    for (pkg, _) in &site_pkgs.site_pkg_by_alias {
        info!("adding site-package: path={}", pkg.display());
        if pkg.exists() {
            // site-packages addition would replace
            add_nodes_recursive(
                &mut g,
                &mut failures,
                pkg,
                &factory,
                &known_libs,
                true,
                &executable_extra_paths_to_search,
            )?;
        } else {
            info!(
                "site packages at path={} does not exist, skipping",
                pkg.display()
            );
        }
    }

    add_failures(
        &mut g,
        failures,
        &factory,
        &executable_extra_paths_to_search,
    )?;

    Ok(g)
}

fn add_failures(
    g: &mut FileGraph<NodeFactory>,
    failures: Vec<PathBuf>,
    factory: &NodeFactory,
    extra_search_paths: &Vec<PathBuf>,
) -> Result<()> {
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
            return Err(failures_to_error(new_failures));
        }

        prev_failures = new_failures;
        prev_len = prev_failures.len();

        known_libs = get_libs_from_graph(g);
    }
    Ok(())
}

fn failures_to_error(failures: Vec<(PathBuf, anyhow::Error)>) -> anyhow::Error {
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
    others.push(anyhow::Error::from(dep_res_err));

    anyhow::Error::from(MultipleGatherErrors { errors: others })
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
    let paths = get_paths_recursive_from_dir(directory)?;
    let mut i = 0;
    let total = paths.len();

    // TODO: filter out all plain files (!is_maybe_shared_library()) and do them in parallel
    // we can parallelize the shared library gather to some extent (it would end up doing duplicate node parsing and creation though)
    // since graph is not safe to use across threads, its difficult to keep track of what all is made
    // we can create a function which takes a list of known nodes and generates a fresh graph for given nodes in parallel
    // then we can do batches and keep updating the list of known nodes

    // path_by_node = graph.path_by_node
    // batch = nodes[:10]
    // batches = nodes(n batches of size k each)
    // parallel: for batch in batches:
    // small_graph = generate_graph(batch, path_by_node)
    // graph.extend(small_graph)

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
            // this is a hack
            // TODO: if the load is already successful before and we are retrying to reload it, we give up and use the older one
            // if there is a shared library inside site-packages which is `dlopen`ed, it should ideally be kept in the library path
            // and also be symlinked to the destination inside site-packages
            // we won't need to rewrite stuff if we do that (replace=False), we actually wont even try making this node
            if let None = g.get_node_by_path(&p) {
                failures.push(p);
            }
        }
        i += 1;
        if total / 10 != 0 && i % (total / 10) == 0 {
            info!("graph: pass 1: {}/{} nodes", i, total);
        }
    }
    Ok(())
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
