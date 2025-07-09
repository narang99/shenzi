use std::{collections::HashMap, path::PathBuf};

use log::info;
use rand::{Rng, rng};

mod comps;

pub use comps::PythonPathComponent;

use crate::{
    manifest::{Sys, ShenziManifest},
    site_pkgs::comps::get_python_path_mapping,
};

#[derive(Debug, Clone)]
pub struct SitePkgs {
    // all the site-packages we found in manifest including prefixes
    pub resolved: Vec<PathBuf>,

    // with aliases, only the top level ones are in this
    pub site_pkg_by_alias: HashMap<PathBuf, String>,

    // original lib dynload path
    pub lib_dynload: PathBuf,

    // original stdlib path
    pub stdlib: PathBuf,

    // the components to push in dist
    pub comps: Vec<PythonPathComponent>,
}

impl SitePkgs {
    pub fn from_manifest(manifest: &ShenziManifest) -> Self {
        let sys = &manifest.python.sys;
        let stdlib_path = get_stdlib_loc(sys);
        let lib_dynload_path = get_lib_dynload_loc(sys);
        let site_pkgs = get_site_pkgs_without_prefixes(&sys.path, &lib_dynload_path, &stdlib_path);
        let site_pkgs = only_top_level_site_pkgs(&site_pkgs, &lib_dynload_path, &stdlib_path);
        let site_pkg_by_alias = create_site_pkgs_alias(&site_pkgs);
        let py_path_comps = get_python_path_mapping(
            &site_pkg_by_alias,
            &stdlib_path,
            &lib_dynload_path,
            &sys.path,
        );
        Self {
            resolved: manifest.python.sys.path.iter().cloned().collect(),
            site_pkg_by_alias: site_pkg_by_alias,
            lib_dynload: lib_dynload_path,
            stdlib: stdlib_path,
            comps: py_path_comps,
        }
    }
}

fn get_site_pkgs_without_prefixes(
    site_pkgs: &Vec<PathBuf>,
    lib_dynload: &PathBuf,
    stdlib: &PathBuf,
) -> Vec<PathBuf> {
    site_pkgs
        .iter()
        .filter(|p| **p != *lib_dynload)
        .filter(|p| **p != *stdlib)
        .map(|p| p.clone())
        .collect()
}

fn only_top_level_site_pkgs(
    sys_path: &Vec<PathBuf>,
    lib_dynload: &PathBuf,
    stdlib: &PathBuf,
) -> Vec<PathBuf> {
    let mut all_paths_to_check: Vec<&PathBuf> = sys_path.iter().collect();
    all_paths_to_check.push(lib_dynload);
    all_paths_to_check.push(stdlib);

    sys_path
        .iter()
        .filter(|p| {
            let should_keep = !is_sub_path_of_other_pkgs(p, &all_paths_to_check);
            if !should_keep {
                info!("package {} is a nested site-package, ignoring", p.display());
            }
            should_keep
        })
        .map(|p| p.clone())
        .collect()
}

fn create_site_pkgs_alias(site_pkgs: &Vec<PathBuf>) -> HashMap<PathBuf, String> {
    let mut site_pkg_aliases = std::collections::HashMap::new();
    for site_pkg in site_pkgs {
        site_pkg_aliases.insert(site_pkg.clone(), random_string());
    }
    site_pkg_aliases
}

fn is_sub_path_of_other_pkgs(p: &PathBuf, sys_path: &Vec<&PathBuf>) -> bool {
    for other in sys_path {
        if *p != **other && p.starts_with(other) {
            return true;
        }
    }
    return false;
}

fn random_string() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    let mut rng = rng();

    (0..10)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn get_stdlib_loc(sys: &Sys) -> PathBuf {
    sys.prefix
        .join(&sys.platlibdir)
        .join(sys.version.get_python_version())
}

fn get_lib_dynload_loc(sys: &Sys) -> PathBuf {
    sys.exec_prefix
        .join(&sys.platlibdir)
        .join(sys.version.get_python_version())
        .join("lib-dynload")
}
