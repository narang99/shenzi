use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use log::info;
use serde::{Deserialize, Serialize};

use crate::{
    paths::normalize_path,
    workspace::{packaging::Packaging, pylock::poetry},
};

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
        },
        // not added right now, will be added later
        "binaries": [
            // all binaries we need
            // if absolute path, used as is
            // else we search it in the path of the user
        ]
    }

*/
#[derive(Serialize, Deserialize, Debug)]
pub struct ShenziWorkspace {
    pub packaging: Packaging,
    pub execution: Execution,
    pub binaries: Vec<String>,

    #[serde(skip)]
    pub workspace_file: PathBuf,
}

impl ShenziWorkspace {
    pub fn get_required_dependencies(&self, workspace_path: &Path) -> Result<Vec<String>> {
        match self.packaging {
            Packaging::Poetry(ref pkg) => pkg.get_required_dependencies(workspace_path),
        }
    }
}

#[derive(Debug)]
pub struct InitializedShenziWorkspace {
    pub workspace: ShenziWorkspace,
    path: PathBuf,
}

impl InitializedShenziWorkspace {
    pub fn from_path(file: PathBuf) -> Result<Option<InitializedShenziWorkspace>> {
        let file = normalize_path(&file);
        if !file.exists() {
            return Ok(None);
        }
        let workspace_path = file.parent().ok_or_else(|| {
            anyhow!(
                "workspace file does not have a parent, file={}",
                file.display()
            )
        })?;
        let workspace = get_shenzi_workspace(&file)?;
        match workspace {
            Some(workspace) => Ok(Some(Self {
                workspace,
                path: workspace_path.to_path_buf(),
            })),
            None => Ok(None),
        }
    }

    pub fn search() -> Result<Option<InitializedShenziWorkspace>> {
        Self::from_path(workspace_file_path())
    }

    pub fn get_required_dependencies(&self) -> Result<Vec<String>> {
        self.workspace.get_required_dependencies(&self.path)
    }

    pub fn main_path(&self) -> PathBuf {
        normalize_path(&self.path.join(&self.workspace.execution.main))
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
    let main_file = crate::ask::ask_user(
        "Path to the main file that should run in the generated application?",
        &None,
    )?;
    if !PathBuf::from(&main_file).exists() {
        bail!("passed main file does not exist, path={}", main_file);
    }

    let binaries = crate::ask::ask_user(
        "Add a comma-separated list of binaries (in PATH) you want in the distribution (example: if you are calling any CLI in your application, add that CLI in this list).",
        &Some(String::from("")),
    )?;

    let file_path = workspace_file_path();
    let workspace = ShenziWorkspace {
        packaging: pkg,
        execution: Execution { main: main_file },
        workspace_file: file_path,
        binaries: binaries.split(",").map(|s| s.to_string()).collect(),
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
    let cwd = std::env::current_dir().unwrap();
    return cwd.join("shenzi_workspace.toml");
}
