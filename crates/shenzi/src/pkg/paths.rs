// this defines the folder structure of the final tree
// there are some properties in the final tree
// does the file exist in reals?
// do we symlink from reals? do we copy?
// where are the dependencies of the file relative to the file? for patching?
// where is the destination?

use std::path::PathBuf;

use anyhow::{Context, anyhow};
use lazy_static::lazy_static;
use log::error;
use regex::Regex;

use crate::{
    manifest::Version,
    node::{Node, Pkg},
    paths::get_lib_name,
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
                site_packages: _,
                alias,
                rel_path,
            }
            | Pkg::SitePackagesBinary {
                site_packages: _,
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
            Pkg::Plain => None,
            Pkg::BinaryInLDPath {
                symlinks: _,
                sha: _,
            } => path.file_name().map(|p| dist.join("lib").join("l").join(p)),
            Pkg::Binary { sha: _ } => None,
            Pkg::Executable => Some(dist.join("python").join("bin").join("python")),
        }
    }

    fn reals(&self, node: &Node, dist: &PathBuf) -> Option<PathBuf> {
        match self {
            Pkg::SitePackagesPlain {
                site_packages: _,
                alias: _,
                rel_path: _,
            }
            | Pkg::Plain
            | Pkg::Executable
            | Pkg::ExecPrefixPlain(_)
            | Pkg::PrefixPlain(_) => None,

            Pkg::SitePackagesBinary {
                site_packages: _,
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
                site_packages: _,
                alias: _,
                rel_path: _,
            }
            | Pkg::Plain
            | Pkg::ExecPrefixPlain(_)
            | Pkg::PrefixPlain(_) => None,

            Pkg::SitePackagesBinary {
                site_packages: _,
                alias: _,
                rel_path: _,
                sha,
            }
            | Pkg::Binary { sha }
            | Pkg::BinaryInLDPath {
                symlinks: _,
                sha,
            } => symlink_farm_path_using_sha(path, dist, sha),

            | Pkg::ExecPrefixBinary(pkgs)
            | Pkg::PrefixBinary(pkgs) => symlink_farm_path_using_sha(path, dist, &pkgs.sha),

            | Pkg::Executable => symlink_farm_path(path, dist),
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
    return reals_path_for_sha(sha, path, dist);
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

fn symlink_farm_path_using_sha(path: &PathBuf, dist: &PathBuf, sha: &str) -> Option<PathBuf> {
    loose_validate_path_is_file(path);
    let os = std::env::consts::OS;
    if os == "macos" || os == "linux" {
        Some(dist.join("symlinks").join(sha))
    } else {
        None
    }
}

fn symlink_farm_path(path: &PathBuf, dist: &PathBuf) -> Option<PathBuf> {
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

lazy_static! {
    static ref SONAME_RE: Regex = Regex::new(r"\.so([.\da-zA-Z]*)$")
        .expect("failed to compiled regex for detecting shared library names");
}

pub fn is_maybe_shared_library(path: &PathBuf) -> bool {
    // this only checks the path extensions
    // they are not very reliable
    // you should try parsing after this to see if they really are a shared library
    match path.to_str() {
        None => false,
        Some(path) => {
            if path.ends_with(".dylib") {
                true
            } else if SONAME_RE.is_match(path) {
                true
            } else {
                false
            }
        }
    }
}
