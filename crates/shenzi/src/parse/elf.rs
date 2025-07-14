use std::{collections::HashMap, path::PathBuf};

use anyhow::{Result, anyhow, bail};
use lief::elf::{Binary, DynamicEntries};

use crate::{
    parse::{error::ErrDidNotFindDependency, search::linux::parse_linux_rpath, Elf},
    paths::split_colon_separated_into_valid_search_paths,
};

pub fn parse(
    binary: Binary,
    object_path: &PathBuf,
    cwd: &PathBuf,
    env: &HashMap<String, String>,
    extra_rpaths: &Vec<PathBuf>,
    known_libs: &HashMap<String, PathBuf>,
) -> Result<Elf> {
    let ld_preload = split_colon_separated_into_valid_search_paths(env.get("LD_PRELOAD"));
    let ld_library_path = split_colon_separated_into_valid_search_paths(env.get("LD_LIBRARY_PATH"));
    let (rpaths, runpaths, libs_needed, _soname) = get_dynamic_entries(&binary, object_path)?;
    do_parse(
        rpaths,
        runpaths,
        libs_needed,
        object_path,
        cwd,
        &ld_preload,
        &ld_library_path,
        extra_rpaths,
        known_libs,
    )
}

fn do_parse(
    rpaths: Vec<String>,
    runpaths: Vec<String>,
    libs_needed: Vec<String>,
    object_path: &PathBuf,
    cwd: &PathBuf,
    ld_preload: &Vec<PathBuf>,
    ld_library_path: &Vec<PathBuf>,
    extra_rpaths: &Vec<PathBuf>,
    known_libs: &HashMap<String, PathBuf>,
) -> Result<Elf> {
    let dt_rpaths = resolve_rpaths(&rpaths, object_path)?;
    let dt_runpaths = resolve_rpaths(&runpaths, object_path)?;

    let dt_rpath_bufs: Vec<PathBuf> = dt_rpaths.values().cloned().collect();
    let dt_runpath_bufs: Vec<PathBuf> = dt_runpaths.values().cloned().collect();

    let mut dt_needed: HashMap<String, PathBuf> = HashMap::new();

    for lib in &libs_needed {
        match crate::parse::search::linux::search(
            lib,
            &dt_rpath_bufs,
            extra_rpaths,
            &dt_runpath_bufs,
            &ld_preload,
            &ld_library_path,
            cwd,
            object_path,
        ) {
            None => {
                if let Some(known_path) = known_libs.get(lib) {
                    dt_needed.insert(lib.to_string(), known_path.clone());
                } else {
                    bail!(ErrDidNotFindDependency {
                        name: lib.clone(),
                        lib: object_path.clone()
                    });
                }
            }
            Some(path) => {
                dt_needed.insert(lib.to_string(), path);
            }
        }
    }

    let elf = Elf {
        dt_needed,
        dt_rpaths,
        _dt_runpaths: dt_runpaths,
        _path: object_path.clone(),
        all_dt_rpaths: rpaths,
        all_dt_runpaths: runpaths,
    };

    Ok(elf)
}

fn resolve_rpaths(rpaths: &Vec<String>, object_path: &PathBuf) -> Result<HashMap<String, PathBuf>> {
    let mut res = HashMap::new();
    for rpath in rpaths {
        match parse_linux_rpath(rpath, object_path)? {
            None => {}
            Some(path) => {
                res.insert(rpath.clone(), path);
            }
        }
    }
    Ok(res)
}

fn get_dynamic_entries(
    binary: &Binary,
    object_path: &PathBuf,
) -> Result<(Vec<String>, Vec<String>, Vec<String>, String)> {
    let mut dt_needed = Vec::new();
    let mut rpaths = Vec::new();
    let mut runpaths = Vec::new();
    let mut soname = None;

    for entry in binary.dynamic_entries() {
        match entry {
            DynamicEntries::Library(e) => {
                dt_needed.push(e.name());
            }
            DynamicEntries::Rpath(e) => {
                let new_rpaths: Vec<String> = e.rpath().split(":").map(|s| s.to_string()).collect();
                rpaths.extend(new_rpaths);
            }
            DynamicEntries::RunPath(e) => {
                let new_runpaths: Vec<String> =
                    e.runpath().split(":").map(|s| s.to_string()).collect();
                runpaths.extend(new_runpaths);
            }
            DynamicEntries::SharedObject(e) => {
                soname = Some(e.name());
            }
            _ => {}
        }
    }

    let soname = match soname {
        Some(soname) => soname,
        None => {
            let filename = object_path
                .file_name()
                .ok_or(anyhow!(
                    "failed in getting filename of {}",
                    object_path.display()
                ))
                .and_then(|f| {
                    f.to_str().ok_or(anyhow!(
                        "failed in converting filename to string, value={}",
                        f.display()
                    ))
                })?;
            filename.to_string()
        }
    };

    Ok((rpaths, runpaths, dt_needed, soname))
}
