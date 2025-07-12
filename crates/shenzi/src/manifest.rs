use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
/// the module defining types for deserializing shenzi.json (or called shenzi manifest)
/// an example json is in this test module, code is duplicated between `python/shenzi` and our crate
/// both should always be synced
use serde::{Deserialize, Serialize};

use crate::paths::normalize_path;

pub type Env = HashMap<String, String>;

#[derive(Debug, Serialize, Deserialize)]
pub struct ShenziManifest {
    pub loads: Vec<Load>,
    pub libs: Vec<Lib>,
    pub python: Python,
    pub env: Env,
    pub skip: Skip,
}

impl ShenziManifest {
    pub fn from_str(manifest_contents: &str) -> Result<Self> {
        let mut manifest: ShenziManifest = serde_json::from_str(manifest_contents)
            .context("Failed to parse shenzi manifest as JSON")?;
        manifest.python.sys.path = manifest
            .python
            .sys
            .path
            .iter()
            .map(|p| normalize_path(p))
            .collect();
        Ok(manifest)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Skip {
    pub prefixes: Vec<PathBuf>,
    pub libs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoadKind {
    Extension,
    Dlopen,
}

/// these are the ones which are dlopen-ed
/// they would be kept in ld-library-path
#[derive(Debug, Serialize, Deserialize)]
pub struct Load {
    pub kind: LoadKind,
    pub path: PathBuf,
    pub symlinks: Vec<String>,
}

/// only dependent libraries, only kept in reals and their symlink farms are created, but not kept in path
#[derive(Debug, Serialize, Deserialize)]
pub struct Lib {
    pub path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Python {
    pub sys: Sys,
    // path to the main script
    pub main: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sys {
    pub prefix: PathBuf,
    pub exec_prefix: PathBuf,
    pub platlibdir: PathBuf,
    pub version: Version,
    pub path: Vec<PathBuf>,
    pub executable: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub abi_thread: String,
}

impl Version {
    pub fn get_python_version(&self) -> String {
        format!("python{}.{}{}", self.major, self.minor, self.abi_thread)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_deserialize() {
        let json_str = r#"
{
    "loads": [
        {
            "kind": "dlopen",
            "path": "/users/hariomnarang/miniconda3/lib/libpango.so",
            "symlinks": ["pango"]
        }
    ],
    "libs": [
        {
            "path": "some-path"
        }
    ],
    "skip": {
        "prefixes": [
            "/miniconda/pygraphviz"
        ],
        "libs": []
    },
    "python": {
        "sys": {
            "prefix": "/Users/hariomnarang/miniconda3",
            "exec_prefix": "/Users/hariomnarang/miniconda3",
            "platlibdir": "lib",
            "version": {
                "major": 3,
                "minor": 12,
                "abi_thread": ""
            },
            "path": ["/Users/hariomnarang/miniconda3/lib/python3.12/site-packages"],
            "executable": "/Users/hariomnarang/miniconda3/bin/python"
        }
    },
    "env": {
        "PATH": "..."
    }
}
"#;

        let manifest: super::ShenziManifest =
            serde_json::from_str(json_str).expect("Failed to deserialize manifest");

        assert_eq!(manifest.loads.len(), 1);
        assert_eq!(
            manifest.loads[0].path.to_str().unwrap(),
            "/users/hariomnarang/miniconda3/lib/libpango.so"
        );

        assert_eq!(
            manifest.python.sys.prefix.to_str().unwrap(),
            "/Users/hariomnarang/miniconda3"
        );
        assert_eq!(
            manifest.python.sys.exec_prefix.to_str().unwrap(),
            "/Users/hariomnarang/miniconda3"
        );
        assert_eq!(manifest.python.sys.platlibdir.to_str().unwrap(), "lib");
        assert_eq!(manifest.python.sys.version.major, 3);
        assert_eq!(manifest.python.sys.version.minor, 12);
        assert_eq!(manifest.python.sys.version.abi_thread, "");
        assert_eq!(manifest.python.sys.path.len(), 1);
        assert_eq!(
            manifest.python.sys.path[0].to_str().unwrap(),
            "/Users/hariomnarang/miniconda3/lib/python3.12/site-packages"
        );
        assert_eq!(
            manifest.python.sys.executable.to_str().unwrap(),
            "/Users/hariomnarang/miniconda3/bin/python"
        );
    }
}
