use std::{collections::HashSet, path::{Path, PathBuf}};

use anyhow::{Result};
use walkdir::WalkDir;

use crate::{
    parse::{ErrDidNotFindDependencies, ErrDidNotFindDependency},
    paths::{get_root_dirs, is_maybe_object_file},
};

pub enum Warning {
    // the dependency `dependency` for shared library at path `path` was not found
    W001DependencyNotFound { dependency: String, path: PathBuf },
}

use std::fmt;

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Warning::W001DependencyNotFound { dependency, path } => {
                write!(
                    f,
                    "Warning(W001): Dependency not found: '{}' <- '{}'",
                    dependency,
                    path.display()
                )
            }
        }
    }
}

/// validate all warnings
/// return warnings which we still want written later
/// this will filter warnings
pub fn validate_warnings(warnings: Vec<Warning>) -> Result<Vec<Warning>> {
    let all_shared_libs = get_all_shared_libraries_in_system();
    let all_lib_names: HashSet<String> = all_shared_libs
        .iter()
        .filter_map(|p| p.file_name())
        .filter_map(|f| f.to_str())
        .map(|s| s.to_string())
        .collect();

    // we handle classes or warnings together, so split each variant and then send to correct handler
    // this looks a bit awkward but theres no better way it seems
    let dependency_not_found_warnings = split_warnings(&warnings);
    validate_deps_not_found(dependency_not_found_warnings, &all_lib_names)?;

    // currently return empty vector as we error out on every warning type that we have
    Ok(vec![])
}

fn split_warnings(warnings: &Vec<Warning>) -> Vec<(&String, &PathBuf)> {
    let mut dependency_not_found_warnings = Vec::new();
    for warning in warnings {
        match warning {
            Warning::W001DependencyNotFound { dependency, path } => {
                dependency_not_found_warnings.push((dependency, path));
            }
        }
    }

    dependency_not_found_warnings
}

fn validate_deps_not_found(
    warnings: Vec<(&String, &PathBuf)>,
    all_lib_names: &HashSet<String>,
) -> Result<()> {
    let mut all_errs = Vec::new();
    for (dependency, path) in warnings {
        if let Err(e) = validate_dep_not_found(dependency, path, all_lib_names) {
            all_errs.push(e);
        }
    }
    if all_errs.len() == 0 {
        Ok(())
    } else {
        Err(anyhow::Error::from(ErrDidNotFindDependencies {
            causes: all_errs,
        }))
    }
}

fn validate_dep_not_found(
    dependency: &str,
    path: &Path,
    all_lib_names: &HashSet<String>,
) -> std::result::Result<(), ErrDidNotFindDependency> {
    if all_lib_names.contains(dependency) {
        // if the name was found in the system, shenzi screwed up, it couldn't find it
        Err(ErrDidNotFindDependency {
            name: dependency.to_string(),
            lib: path.to_path_buf(),
        })
    } else {
        Ok(())
    }
}

fn get_all_shared_libraries_in_system() -> HashSet<PathBuf> {
    let mut res = HashSet::new();
    let roots = get_root_dirs();
    for root in roots {
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| is_maybe_object_file(e.path()))
        {
            res.insert(entry.path().to_path_buf());
        }
    }
    res
}
