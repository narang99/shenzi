// main function which moves stuff to dist

use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use log::info;
use pathdiff::diff_paths;

use crate::{
    external::download_patchelf,
    gather::NodeFactory,
    graph::FileGraph,
    node::Node,
    pkg::{
        export::{Export, mk_parent_dirs},
        paths::ExportedFileTree,
    },
    warnings::Warning,
};

pub use patch::LibPatch;

pub mod bootstrap;
pub mod export;
pub mod patch;
pub mod paths;

pub fn move_all_nodes(
    graph: &FileGraph<NodeFactory>,
    dist: &PathBuf,
    main_script_path: &PathBuf,
) -> Result<PathBuf> {
    info!("exporting files to dist");
    download_patchelf().context("error in downloading patchelf")?;

    // TODO: parallelize each step
    let nodes: Vec<&Node> = graph.iter_nodes().collect();
    move_reals(&nodes, dist)?;
    mk_symlink_farms(&nodes, graph, dist)?;
    cp_to_destinations(&nodes, dist)?;

    graph
        .get_node_by_path(main_script_path)
        .map(|n| n.path.clone())
        .ok_or(anyhow!(
            "could not find the final path for main script, script={}",
            main_script_path.display()
        ))
}

pub fn write_warnings(warnings: Vec<Warning>, dist: &PathBuf) -> Result<(PathBuf, bool)> {
    let p = dist.join("warnings.txt");
    if warnings.len() == 0 {
        Ok((p, false))
    } else {
        let contents = warnings
            .into_iter()
            .map(|w| format!("{}", w))
            .collect::<Vec<String>>()
            .join("\n");
        fs::write(&p, contents)?;
        Ok((p, true))
    }
}

pub fn move_reals(nodes: &Vec<&Node>, dist: &PathBuf) -> Result<()> {
    info!("Step: copy assets to dist/reals");
    let total = nodes.len();
    let mut i = 0;
    for node in nodes {
        mk_reals(node, dist).with_context(|| {
            format!(
                "could not create reals directory for path={} dist={}",
                node.path.display(),
                dist.display()
            )
        })?;
        i += 1;
        if total / 10 != 0 && i % (total / 10) == 0 {
            info!("\tprogress: {}/{} files", i, total);
        }
    }
    Ok(())
}

pub fn mk_symlink_farms(
    nodes: &Vec<&Node>,
    g: &FileGraph<NodeFactory>,
    dist: &PathBuf,
) -> Result<()> {
    info!("Step: make symlink farms");
    let total = nodes.len();
    let mut i = 0;
    for node in nodes {
        let deps = g.get_node_dependencies(node);
        let symlink_farm = mk_symlink_farm(node, &deps, dist).with_context(|| {
            format!(
                "could not create symlink farm for path={} dist={}",
                node.path.display(),
                dist.display()
            )
        })?;
        let real_path = node.pkg.reals(&node, dist);
        symlink_farm
            .as_ref()
            .map(|p| -> Result<()> {
                match real_path {
                    Some(ref real_path) => node.deps.patch(real_path, &p).with_context(|| {
                        anyhow!(
                            "failed in patching shared library at node_path={} real_path={} symlink_farm={}",
                            node.path.display(),
                            real_path.display(),
                            p.display()
                        )
                    }),
                    None => Ok(()),
                }
            })
            .transpose()
            .with_context(|| {
                anyhow!(
                    "failed in patching library for node, path={}",
                    node.path.display()
                )
            })?;
        i += 1;
        if total / 10 != 0 && i % (total / 10) == 0 {
            info!("\tprogress: {}/{} files", i, total);
        }
    }
    Ok(())
}

pub fn cp_to_destinations(nodes: &Vec<&Node>, dist: &PathBuf) -> Result<()> {
    info!("Step: copy/move/symlink to destination (site-packages)");
    let total = nodes.len();
    let mut i = 0;
    for node in nodes {
        let real_path = node.pkg.reals(&node, dist);
        let symlink_farm = node.pkg.symlink_farm(&node.path, dist);
        let path_to_cp_to_destination = real_path.as_ref().unwrap_or(&node.path);
        let destination = node.pkg.destination(&node.path, dist);
        destination
            .as_ref()
            .map(|dest| {
                node.pkg
                    .to_destination(&path_to_cp_to_destination, &dest, &dist)
            })
            .transpose()
            .with_context(|| {
                format!(
                    "could not move to destination for path={} dist={}",
                    node.path.display(),
                    dist.display()
                )
            })?;

        if let (Some(dest_path), Some(symlink_farm_path), Some(real_path)) = (
            destination.as_ref(),
            symlink_farm.as_ref(),
            real_path.as_ref(),
        ) {
            // hack: this is very bad
            // need better code for this
            node.deps.patch_for_destination(dest_path, real_path, symlink_farm_path).with_context(|| {
                anyhow!("failure in patching destination for destination={} real_path={} symlink_farm={}", dest_path.display(), real_path.display(), symlink_farm_path.display())
            })?;
        }
        i += 1;
        if total / 10 != 0 && i % (total / 10) == 0 {
            info!("\tprogress: {}/{} files", i, total);
        }
    }
    Ok(())
}

fn mk_reals(node: &Node, dist: &PathBuf) -> Result<Option<PathBuf>> {
    node.pkg
        .reals(&node, dist)
        .map(|dest| -> Result<PathBuf> {
            mk_parent_dirs(&dest).with_context(|| {
                anyhow!(
                    "failed in creating parent dirs for destination, dest={}",
                    dest.display()
                )
            })?;
            if dest.exists() {
                fs::remove_file(&dest).with_context(|| {
                    anyhow!(
                        "failed in removing existing file at destination, dest={}",
                        dest.display()
                    )
                })?;
            }
            fs::copy(&node.path, &dest).with_context(|| {
                anyhow!(
                    "failed in copying reals to destination, dest={}",
                    dest.display()
                )
            })?;
            Ok(dest)
        })
        .transpose()
}

// todo: return path
fn mk_symlink_farm(node: &Node, deps: &Vec<Node>, dist: &PathBuf) -> Result<Option<PathBuf>> {
    node.pkg.symlink_farm(&node.path, dist).map(|symlink_dir| -> Result<PathBuf> {
        fs::create_dir_all(&symlink_dir)?;
        for dep in deps {
            let dep_reals_path = dep.pkg.reals(&dep, dist);
            match dep_reals_path {
                None => {},
                Some(dep_reals_path) => {
                    let file_name = dep.path.file_name().ok_or_else(|| {
                        anyhow!("could not find file_name for creating symlink for dependency, path={}", dep_reals_path.display())
                    })?;
                    let rel_path = diff_paths(&dep_reals_path, &symlink_dir).ok_or_else(|| {
                        anyhow!(
                            "failed in finding relative path for creating symlink farm, symlink_dir={} path={}",
                            symlink_dir.display(),
                            dep_reals_path.display()
                        )
                    })?;
                    let dest = symlink_dir.join(file_name);
                    if dest.exists() {
                        fs::remove_file(&dest)?;
                    }
                    std::os::unix::fs::symlink(&rel_path, &dest)?;
                }
            };
        }
        Ok(symlink_dir)
    }).transpose()
}
