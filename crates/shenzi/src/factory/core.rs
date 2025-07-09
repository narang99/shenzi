use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;

use crate::node::Node;

pub trait Factory {
    fn make(
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
}
