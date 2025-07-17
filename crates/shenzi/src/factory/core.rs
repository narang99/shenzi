use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;

use crate::node::{Node, Pkg, deps::Deps};

pub trait Factory {
    fn make(
        &self,
        path: &PathBuf,
        known_libs: &HashMap<String, PathBuf>,
        extra_search_paths: &Vec<PathBuf>,
    ) -> Result<Option<Node>>;

    fn make_binary(
        &self,
        path: &PathBuf,
        known_libs: &HashMap<String, PathBuf>,
        extra_search_paths: &Vec<PathBuf>,
    ) -> Result<Option<Node>>;

    fn make_with_symlinks(
        &self,
        path: &PathBuf,
        symlinks: &Vec<String>,
        known_libs: &HashMap<String, PathBuf>,
        extra_search_paths: &Vec<PathBuf>,
    ) -> Result<Option<Node>>;

    fn make_py_executable(&self, path: &PathBuf) -> Result<Node>;

    fn make_main_py_script(&self, path: &PathBuf) -> Result<Node> {
        Ok(Node::new(path.clone(), Pkg::MainPyScript, Deps::Plain)?)
    }
}
