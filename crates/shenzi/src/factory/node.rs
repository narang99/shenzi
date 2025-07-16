use std::{collections::HashMap, path::PathBuf};

use anyhow::{Result, bail};
use log::info;

pub use crate::factory::core::Factory;

use crate::{
    digest::make_digest,
    factory::{
        deps::create_deps,
        pkg::{get_exec_prefix_pkg, get_prefix_pkg, get_site_packages_pkg},
    },
    manifest::{Skip, Version},
    node::{Node, Pkg, deps::Deps},
    paths::is_maybe_object_file,
    paths::normalize_path,
    site_pkgs::SitePkgs,
};

#[derive(Debug, Clone)]
pub struct NodeFactory {
    site_pkgs: SitePkgs,
    version: Version,
    executable: PathBuf,
    cwd: PathBuf,
    env: HashMap<String, String>,
    skip: Skip,
}

impl NodeFactory {
    pub fn new(
        site_pkgs: SitePkgs,
        version: Version,
        executable: PathBuf,
        cwd: PathBuf,
        env: HashMap<String, String>,
        skip: Skip,
    ) -> NodeFactory {
        Self {
            site_pkgs,
            version,
            executable: executable,
            cwd: cwd,
            env,
            skip,
        }
    }
}

impl NodeFactory {
    fn create_deps(
        &self,
        path: &PathBuf,
        known_libs: &HashMap<String, PathBuf>,
        extra_search_paths: &Vec<PathBuf>,
        force_object_file_parsing: bool,
    ) -> Result<Deps> {
        create_deps(
            path,
            &self.executable,
            &self.cwd,
            &self.env,
            known_libs,
            extra_search_paths,
            force_object_file_parsing,
        )
    }

    fn should_skip(&self, path: &PathBuf, is_shared_library: bool) -> bool {
        for prefix in &self.skip.prefixes {
            if path.starts_with(prefix) {
                return true;
            }
        }
        if is_shared_library && self.is_path_in_skipped_shared_libs(path) {
            return true;
        }
        false
    }

    fn is_path_in_skipped_shared_libs(&self, path: &PathBuf) -> bool {
        match path.file_name().and_then(|file_name| file_name.to_str()) {
            Some(s) => {
                for lib in &self.skip.libs {
                    if lib.trim() == s {
                        return true;
                    }
                }
                false
            }
            None => false,
        }
    }
}

impl Factory for NodeFactory {
    fn make_with_symlinks(
        &self,
        path: &PathBuf,
        symlinks: &Vec<String>,
        known_libs: &HashMap<String, PathBuf>,
        extra_search_paths: &Vec<PathBuf>,
    ) -> Result<Option<Node>> {
        let deps = self.create_deps(&path, known_libs, extra_search_paths, true)?;
        let is_shared_library = deps.is_shared_library();
        if !is_shared_library {
            bail!(
                "cannot make_with_symlinks if the path is not a shared library, path={}",
                path.display()
            );
        }
        if self.should_skip(path, is_shared_library) {
            info!("skip: {}", path.display());
            return Ok(None);
        }
        Ok(Some(Node::new(
            path.clone(),
            Pkg::BinaryInLDPath {
                symlinks: symlinks.clone(),
                sha: make_digest(path)?,
            },
            deps,
        )?))
    }

    fn make_binary(
        &self,
        path: &PathBuf,
        known_libs: &HashMap<String, PathBuf>,
        extra_search_paths: &Vec<PathBuf>,
    ) -> Result<Option<Node>> {
        let deps = self.create_deps(&path, known_libs, extra_search_paths, true)?;
        let is_shared_library = deps.is_shared_library();
        let pkg = if is_shared_library {
            if self.should_skip(path, is_shared_library) {
                info!("skip: {}", path.display());
                return Ok(None);
            }
            Pkg::BinaryInPath {
                sha: make_digest(path)?,
            }
        } else {
            Pkg::PlainPyBinaryFile
        };
        Ok(Some(Node::new(path.clone(), pkg, deps)?))
    }

    fn make(
        &self,
        path: &PathBuf,
        known_libs: &HashMap<String, PathBuf>,
        extra_search_paths: &Vec<PathBuf>,
    ) -> Result<Option<Node>> {
        let p = normalize_path(path);
        if self.should_skip(&p, is_maybe_object_file(&p)) {
            info!("skip: {}", p.display());
            return Ok(None);
        }
        if !p.exists() {
            bail!(
                "tried creating node from path={}, but the path does not exist",
                path.display()
            );
        }

        let deps = self.create_deps(&p, known_libs, extra_search_paths, false)?;
        let is_shared_library = deps.is_shared_library();
        if p.starts_with(&self.site_pkgs.lib_dynload) {
            return Ok(Some(Node::new(
                p.clone(),
                get_exec_prefix_pkg(
                    &p,
                    &self.site_pkgs.lib_dynload,
                    &self.version,
                    is_shared_library,
                )?,
                deps,
            )?));
        }

        if p.starts_with(&self.site_pkgs.stdlib) {
            return Ok(Some(Node::new(
                p.clone(),
                get_prefix_pkg(&p, &self.site_pkgs.stdlib, &self.version, is_shared_library)?,
                deps,
            )?));
        }

        for (site_pkg, alias) in &self.site_pkgs.site_pkg_by_alias {
            if p.starts_with(site_pkg) {
                return Ok(Some(Node::new(
                    p.clone(),
                    get_site_packages_pkg(&p, site_pkg, alias, &self.version, is_shared_library)?,
                    deps,
                )?));
            }
        }

        if !is_shared_library {
            bail!(
                "found a path which is not inside site packages and is not a shared library. Only plain files inside site-packages are allowed, path={}",
                p.display()
            );
        }
        Ok(Some(Node::new(
            p.clone(),
            Pkg::Binary {
                sha: make_digest(&p)?,
            },
            deps,
        )?))
    }

    fn make_py_executable(&self, path: &PathBuf) -> Result<Node> {
        Node::new(
            path.clone(),
            Pkg::Executable,
            self.create_deps(path, &HashMap::new(), &Vec::new(), true)?,
        )
    }
}
