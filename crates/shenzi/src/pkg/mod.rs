// main function which moves stuff to dist

use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use pathdiff::diff_paths;

use crate::{
    node::Node,
    pkg::{
        export::{Export, mk_parent_dirs},
        paths::ExportedFileTree,
    },
};

pub use patch::LibPatch;

pub mod bootstrap;
pub mod export;
pub mod patch;
pub mod paths;

pub fn move_to_dist(node: &Node, deps: &Vec<Node>, dist: &PathBuf) -> Result<()> {
    // todo: python executable does not have a symlink farm, fix that
    // for that we need to also remove the hardcoding we have done for patching
    // deps are already exported, now we export node

    let real_path = mk_reals(node, dist).with_context(|| {
        format!(
            "could not create reals directory for path={} dist={}",
            node.path.display(),
            dist.display()
        )
    })?;

    let symlink_farm = mk_symlink_farm(node, deps, dist).with_context(|| {
        format!(
            "could not create symlink farm for path={} dist={}",
            node.path.display(),
            dist.display()
        )
    })?;

    // todo: chain from mk_symlink_farm directly, it should return the path like reals
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
            // println!("dep of node={}, path={} reals={:?}", node.path.display(), dep.path.display(), dep_reals_path);
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
