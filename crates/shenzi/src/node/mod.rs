use std::{
    fmt::Display,
    hash::{Hash, Hasher},
    path::PathBuf,
};

use anyhow::Result;

use crate::{manifest::Version, node::deps::Deps};

pub mod deps;

#[derive(Debug, Clone)]
pub struct PrefixPlain {
    pub _original_prefix: PathBuf,

    pub rel_path: PathBuf,

    pub version: Version,
}

#[derive(Debug, Clone)]
pub struct PrefixBinary {
    pub _original_prefix: PathBuf,

    pub rel_path: PathBuf,

    pub version: Version,

    pub sha: String,
}

#[derive(Debug, Clone)]
pub enum Pkg {
    SitePackagesPlain{
        // original site-packages path
        _site_packages: PathBuf,
        // to prevent collisions, we create an alias which is the name of this site-packages destination in dist
        alias: String,
        // the path relative to site-packages path, we simply copy data from node to this path inside alias in dist
        rel_path: PathBuf,
    },
    SitePackagesBinary {
        // original site-packages path
        _site_packages: PathBuf,
        // to prevent collisions, we create an alias which is the name of this site-packages destination in dist
        alias: String,
        // the path relative to site-packages path, we simply copy data from node to this path inside alias in dist
        rel_path: PathBuf,
        // the sha of the binary
        sha: String,
    },
    // a python file which is in path as a binary
    PlainPyBinaryFile,

    // Main script if not inside site-packages (has to be pushed to the base of dist)
    MainPyScript,

    ExecPrefixPlain(PrefixPlain),
    ExecPrefixBinary(PrefixBinary),
    PrefixPlain(PrefixPlain),
    PrefixBinary(PrefixBinary),

    Executable,
    Binary {sha: String},
    BinaryInLDPath { symlinks: Vec<String>, sha: String },
    BinaryInPath {sha: String},
}


#[derive(Debug, Clone)]
pub struct Node {
    pub path: PathBuf,

    pub deps: Deps,

    pub pkg: Pkg,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for Node {}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node{{")?;
        write!(f, "{}", self.path.display())?;
        write!(f, "}}")
    }
}

impl Node {
    pub fn new(path: PathBuf, pkg: Pkg, deps: Deps) -> Result<Node> {
        Ok(Node {
            path,
            deps,
            pkg,
        })
    }

    #[cfg(test)]
    pub fn mock(path: PathBuf, deps: Vec<PathBuf>) -> Result<Node> {
        use crate::digest::make_digest;

        let sha = make_digest(&path)?;
        Ok(Node {
            path,
            deps: Deps::mock(deps),
            pkg: Pkg::Binary {sha},
        })
    }
}