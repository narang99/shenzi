// this defines the folder structure of the final tree
// there are some properties in the final tree
// does the file exist in reals?
// do we symlink from reals? do we copy?
// where are the dependencies of the file relative to the file? for patching?
// where is the destination?

use std::{os::unix::fs::symlink, path::{Path, PathBuf}};

use log::error;

use crate::{
    manifest::Version,
    node::{Node, Pkg},
};

pub trait ExportedFileTree {
    // returns the destination if there is an actual destination
    fn destination(&self, path: &PathBuf, dist: &PathBuf) -> Option<PathBuf>;

    // reals location, if needed
    fn reals(&self, path: &Node, dist: &PathBuf) -> Option<PathBuf>;

    // symlink farm location, if exists
    fn symlink_farm(&self, path: &PathBuf, dist: &PathBuf) -> Option<PathBuf>;
}

impl ExportedFileTree for Pkg {
    fn destination(&self, path: &PathBuf, dist: &PathBuf) -> Option<PathBuf> {
        match self {
            Pkg::SitePackagesPlain {
                _site_packages: _,
                alias,
                rel_path,
            }
            | Pkg::SitePackagesBinary {
                _site_packages: _,
                alias,
                rel_path,
                sha: _,
            } => Some(site_pkgs_path_in_dist(alias, rel_path, dist)),
            Pkg::ExecPrefixBinary(prefix_pkgs) => Some(exec_prefix_path_in_dist(
                &prefix_pkgs.version,
                &prefix_pkgs.rel_path,
                dist,
            )),
            Pkg::ExecPrefixPlain(prefix_pkgs) => Some(exec_prefix_path_in_dist(
                &prefix_pkgs.version,
                &prefix_pkgs.rel_path,
                dist,
            )),
            Pkg::PrefixBinary(prefix_pkgs) => Some(prefix_path_in_dist(
                &prefix_pkgs.version,
                &prefix_pkgs.rel_path,
                dist,
            )),
            Pkg::PrefixPlain(prefix_pkgs) => Some(prefix_path_in_dist(
                &prefix_pkgs.version,
                &prefix_pkgs.rel_path,
                dist,
            )),
            Pkg::BinaryInLDPath {
                symlinks: _,
                sha: _,
            } => path.file_name().map(|p| dist.join("lib").join("l").join(p)),
            Pkg::Binary { sha: _ } => None,
            Pkg::Executable => None,
        }
    }

    fn reals(&self, node: &Node, dist: &PathBuf) -> Option<PathBuf> {
        match self {
            Pkg::SitePackagesPlain {
                _site_packages: _,
                alias: _,
                rel_path: _,
            }
            // | Pkg::Executable
            | Pkg::ExecPrefixPlain(_)
            | Pkg::PrefixPlain(_) => None,
            

            // HACK: currently `reals` actually just means that the directory `reals/r`
            // this is the wrong definition
            // reals should be the place the file is actually copied, and should just be a single path
            // destination -> should be a vector of paths where we generate symlinks
            // currently destination means anything outside reals/r and that's a plain wrong semantic
            // TODO: fix the above hack
            Pkg::Executable => Some(dist.join("python").join("bin").join("python")),

            Pkg::SitePackagesBinary {
                _site_packages: _,
                alias: _,
                rel_path: _,
                sha,
            }
            | Pkg::Binary { sha }
            | Pkg::BinaryInLDPath { symlinks: _, sha } => {
                reals_path(&sha, &node.path, dist)
            }
            Pkg::PrefixBinary(pkg) | Pkg::ExecPrefixBinary(pkg) => {
                reals_path(&pkg.sha, &node.path, dist)
            }
        }
    }

    fn symlink_farm(&self, path: &PathBuf, dist: &PathBuf) -> Option<PathBuf> {
        match self {
            Pkg::SitePackagesPlain {
                _site_packages: _,
                alias: _,
                rel_path: _,
            }
            | Pkg::ExecPrefixPlain(_)
            | Pkg::PrefixPlain(_) => None,

            Pkg::SitePackagesBinary {
                _site_packages: _,
                alias: _,
                rel_path: _,
                sha,
            }
            | Pkg::Binary { sha }
            | Pkg::BinaryInLDPath {
                symlinks: _,
                sha,
            } => symlink_farm_path(path, dist, sha),

            | Pkg::ExecPrefixBinary(pkgs)
            | Pkg::PrefixBinary(pkgs) => symlink_farm_path(path, dist, &pkgs.sha),

            | Pkg::Executable => symlink_farm_path_from_file_name(path, dist),
        }
    }
}

fn site_pkgs_path_in_dist(alias: &str, rel_path: &PathBuf, dist: &PathBuf) -> PathBuf {
    dist.join(site_pkgs_relative_path(alias)).join(rel_path)
}

pub fn site_pkgs_relative_path(alias: &str) -> PathBuf {
    PathBuf::from("site_packages").join(alias)
}

fn exec_prefix_path_in_dist(version: &Version, rel_path: &PathBuf, dist: &PathBuf) -> PathBuf {
    dist.join(lib_dynload_relative_path(version)).join(rel_path)
}

pub fn lib_dynload_relative_path(version: &Version) -> PathBuf {
    PathBuf::from("python")
        .join("lib")
        .join(version.get_python_version())
        .join("lib-dynload")
}

fn prefix_path_in_dist(version: &Version, rel_path: &PathBuf, dist: &PathBuf) -> PathBuf {
    dist.join(stdlib_relative_path(version)).join(rel_path)
}

pub fn stdlib_relative_path(version: &Version) -> PathBuf {
    PathBuf::from("python")
        .join("lib")
        .join(version.get_python_version())
}

fn reals_path(sha: &str, path: &PathBuf, dist: &PathBuf) -> Option<PathBuf> {
    loose_validate_path_is_file(path);
    let os = std::env::consts::OS;
    if os == "macos" {
        return reals_path_for_sha(sha, path, dist);
    } else {
        return reals_path_using_file_name(path, dist);
    }
}

fn reals_path_for_sha(sha: &str, path: &PathBuf, dist: &PathBuf) -> Option<PathBuf> {
    let fname = match path.extension() {
        Some(ext) => format!(
            "{}.{}",
            sha,
            ext.to_str().expect(&format!(
                "failed in converting extension {} to string",
                ext.display()
            ))
        ),
        None => sha.to_string(),
    };
    let reals_dir = dist.join("reals").join("r");
    Some(reals_dir.join(fname))
}


fn reals_path_using_file_name(path: &Path, dist: &Path) -> Option<PathBuf> {
    let fname = match path.file_name() {
        Some(f) => f,
        None => return None,
    };
    let reals_dir = dist.join("reals").join("r");
    return Some(reals_dir.join(fname));
}

fn symlink_farm_path(path: &PathBuf, dist: &PathBuf, sha: &str) -> Option<PathBuf> {
    loose_validate_path_is_file(path);
    let os = std::env::consts::OS;
    if os == "macos" {
        Some(dist.join("symlinks").join(sha))
    } else {
        symlink_farm_path_from_file_name(path, dist)
    }
}

fn symlink_farm_path_from_file_name(path: &PathBuf, dist: &PathBuf) -> Option<PathBuf> {
    loose_validate_path_is_file(path);
    let os = std::env::consts::OS;
    if os == "macos" || os == "linux" {
        let symlinks_farm_dir = dist.join("symlinks");
        path.file_name()
            .map(|file_name| symlinks_farm_dir.join(file_name))
    } else {
        None
    }
}

fn loose_validate_path_is_file(path: &PathBuf) {
    if !path.is_file() {
        if cfg!(debug_assertions) {
            panic!(
                "got a path which is not a file or does not exist for moving to reals={}",
                path.display()
            );
        } else {
            error!(
                "error: found a path which is not a file or does not exist for moving to reals directory, please raise this with the developer, shenzi will ignore this path and move on, path={}",
                path.display()
            );
        }
    }
}
