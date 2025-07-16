/*
the main module for dealing with `packaging` property

{
    "packaging": {
        "kind": "poetry",
        "config_file": "relative_path_from_root",
        "groups": ["main"],
    }
}


during shenzi init, we ask the user what the packaging is
later we can do auto discover but its not needed right now
*/

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Write},
    path::PathBuf,
};

use crate::workspace::pylock;

#[derive(Serialize, Deserialize, Debug)]
pub struct PoetryPackaging {
    pub config_file: String,
    pub groups: Vec<String>,
}

impl PoetryPackaging {
    pub fn get_required_dependencies(&self) -> Result<Vec<String>> {
        let config_file = PathBuf::from(&self.config_file);
        if !config_file.exists() {
            bail!("passed lock file for poetry dependency analysis does not exist, path={}", config_file.display());
        }
        pylock::poetry::get_required_dependencies(&config_file, &self.groups)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum Packaging {
    #[serde(rename = "poetry")]
    Poetry(PoetryPackaging),
}

pub fn ask_user() -> Result<Packaging> {
    let tool_type: &str = &crate::ask::ask_user(
        "What type of packaging tool do you use? (poetry)",
        &Some(String::from("poetry")),
    )?
    .to_lowercase();
    // TODO: add this to individual module
    let lock_path = crate::ask::ask_user(
        "provide the lock file path relative to current directory (e.g., poetry.lock), default=poetry.lock",
        &Some(String::from("poetry.lock")),
    )?;
    let lock_path_buf = PathBuf::from(&lock_path);
    if !lock_path_buf.exists() {
        bail!(
            "provided lock file path={} does not exist",
            lock_path_buf.display()
        );
    }
    match tool_type {
        "poetry" => {
            let groups = pylock::poetry::ask_user_for_groups()?;
            Ok(Packaging::Poetry(PoetryPackaging {
                config_file: lock_path,
                groups: groups,
            }))
        }
        _ => {
            bail!("invalid tool type, only `poetry` is supported right now");
        }
    }
}
