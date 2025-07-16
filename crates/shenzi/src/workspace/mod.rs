use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::workspace::{packaging::Packaging, pylock::poetry};

mod packaging;
mod pylock;

/*
    how does the workspace file look?
    {
        "packaging": {
            "kind": "poetry",
            "config": "./poetry.lock",
            "groups": ["main", "dev"],
        },
        "execution": {
            "main": "./hello.py",
        },
        // not added right now, will be added later
        "skip": {
            "package": ["graphviz"],
            "shared_libraries": ["libhello.so.2"],
        }
    }

*/
#[derive(Serialize, Deserialize, Debug)]
pub struct ShenziWorkspace {
    pub packaging: Packaging,
    pub execution: Execution,

    #[serde(skip)]
    pub workspace_file: PathBuf,
}

impl ShenziWorkspace {
    pub fn get_required_dependencies(&self) -> Result<Vec<String>> {
        match self.packaging {
            Packaging::Poetry(ref pkg) => {
                pkg.get_required_dependencies()
            }
        }
    }
}

pub struct InitializedShenziWorkspace {
    pub workspace: ShenziWorkspace,
    file: PathBuf
}

impl InitializedShenziWorkspace {
    pub fn from_path(file: PathBuf) -> Result<Option<InitializedShenziWorkspace>> {
        let workspace = get_shenzi_workspace(&file)?;
        match workspace {
            Some(workspace) => Ok(Some(Self {workspace, file})),
            None => Ok(None)
        }
    }

    pub fn search() -> Result<Option<InitializedShenziWorkspace>> {
        Self::from_path(workspace_file_path())
    }

    pub fn path(&self) -> &Path {
        &self.file
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Execution {
    pub main: String,
}

pub fn init_workspace() -> Result<()> {
    // currently we override everything
    // later need to add support to read existing file, and add defaults from that file if the user has asked for it
    let pkg = packaging::ask_user()?;
    let main_file = crate::ask::ask_user("Path to the main file that should run in the generated application?", &None)?;
    if !PathBuf::from(&main_file).exists() {
        bail!("passed main file does not exist, path={}", main_file);
    }


    let file_path = workspace_file_path();
    let workspace = ShenziWorkspace {
        packaging: pkg,
        execution: Execution { main: main_file },
        workspace_file: file_path,
    };


    let content = toml::to_string(&workspace)?;
    std::fs::write(workspace.workspace_file, content)?;
    Ok(())
}


fn get_shenzi_workspace(config_file: &Path) -> Result<Option<ShenziWorkspace>> {
    // if the workspace file does not exist, we return None
    // let poetry_lock: PoetryLock = toml::from_str(&contents)?;

    if !config_file.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(config_file)?;
    let shenzi_workspace: ShenziWorkspace = toml::from_str(&contents)?;
    Ok(Some(shenzi_workspace))
}

pub fn workspace_file_path() -> PathBuf {
    PathBuf::from("shenzi_workspace.toml")
}
