// given a macho to parse
// we return the rpaths if they exist
// loader-path would be simply the current path
// we also want executable-path as an input

use std::{collections::HashMap, ffi::OsStr, path::PathBuf, str::FromStr};

use anyhow::{Context, Error, Result, anyhow, bail};
use lief::macho::{
    Binary, Commands, FatBinary,
    commands::{Command, LoadCommandTypes},
    header::CpuType,
};
use log::{debug, warn};

use crate::{parse::error::ErrDidNotFindDependency, paths::{is_sys_lib, normalize_path, split_colon_separated_into_valid_search_paths}};

use crate::parse::core::{BinaryParseError, Macho};

#[derive(Debug)]
struct PathResolverCtx<'a> {
    rpaths: Vec<PathBuf>,
    loader_path: &'a PathBuf,
    shared_lib_ctx: &'a SharedLibCtx<'a>,
}

#[derive(Debug)]
struct SharedLibCtx<'a> {
    executable_path: &'a PathBuf,
    cwd: &'a PathBuf,
    dyld_library_path: &'a Vec<PathBuf>,
}

/// parse a macho file and get its dependencies
/// Parsing logic depends on three kinds of paths
/// First is an actual path, denoted by Path/PathBuf
/// Second is a string path that needs resolution
pub fn parse(
    macho: FatBinary,
    macho_path: &PathBuf,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    env: &HashMap<String, String>,
    known_libs: &HashMap<String, PathBuf>,
) -> Result<Macho> {
    let dyld_library_path =
        &split_colon_separated_into_valid_search_paths(env.get("DYLD_LIBRARY_PATH"));
    let ctx = SharedLibCtx {
        executable_path,
        cwd,
        dyld_library_path,
    };
    _parse(macho, macho_path, &ctx, known_libs)
        .with_context(|| anyhow!("failed in parsing macho={} context={:?}", macho_path.display(), ctx))
}

fn _parse(
    fat: FatBinary,
    macho_path: &PathBuf,
    ctx: &SharedLibCtx,
    known_libs: &HashMap<String, PathBuf>,
) -> Result<Macho> {
    let host_cpu_type = get_host_cpu_type()?;

    for macho in fat.iter() {
        // im going through all binaries inside the fat binary and creating everyone's Macho struct
        // this is extra work, we could just check the host header first, and if its not of our arch, move on
        // only problem is, if I'm calling `header()` before `load_commands()` for binaries, its randomly segfaulting
        // if we call `header()` later, it does not happen
        let (parsed, cpu_type) = _parse_single_macho(&macho_path, macho, ctx, known_libs)?;
        if cpu_type == host_cpu_type {
            return Ok(parsed);
        }
    }
    warn!(
        "No binary found inside FAT Macho Binary for the host architecture, ignoring. path={} arch={:?}",
        macho_path.display(), host_cpu_type
    );
    return Err(Error::new(BinaryParseError::UnsupportedArchitecture));
}

fn _parse_single_macho(
    macho_path: &PathBuf,
    macho: Binary,
    ctx: &SharedLibCtx,
    known_libs: &HashMap<String, PathBuf>,
) -> Result<(Macho, CpuType)> {
    let loader_path = macho_path
        .parent()
        .ok_or(anyhow!(
            "fatal: could not find directory of path={}",
            macho_path.display()
        ))?
        .to_path_buf();

    let (rpaths, all_rpaths) = get_rpaths(
        &macho,
        ctx.executable_path,
        ctx.cwd,
        &loader_path,
        &ctx.dyld_library_path,
    )
    .context(anyhow!("failed in parsing rpath"))?;
    let resolver_ctx = PathResolverCtx {
        loader_path: &loader_path,
        rpaths: rpaths.iter().map(|(_, rpath)| rpath.clone()).collect(),
        shared_lib_ctx: ctx,
    };
    let (id_dylib, load_cmds) = get_load_commands(&macho, &macho_path, &resolver_ctx, known_libs)
        .context(anyhow!(
        "failed in parsing load commands for {}",
        macho_path.display()
    ))?;

    // NOTE: make sure to always call `header` in the end, if called before `load_commands`, we get random segfaults
    Ok((
        Macho {
            load_cmds,
            rpaths,
            id_dylib,
            path: macho_path.clone(),
            all_rpaths,
        },
        macho.header().cpu_type(),
    ))
}

fn get_host_cpu_type() -> Result<CpuType> {
    let current_arch = std::env::consts::ARCH;
    match current_arch {
        "x86_64" => Ok(CpuType::X86_64),
        "aarch64" => Ok(CpuType::ARM64),
        _ => bail!(
            "unsupported host architecture {}, only x86_64 and aarch64 are supported",
            current_arch
        ),
    }
}

fn get_rpaths(
    macho: &lief::macho::Binary,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    loader_path: &PathBuf,
    dyld_library_path: &Vec<PathBuf>,
) -> Result<(HashMap<String, PathBuf>, Vec<String>)> {
    let mut all_rpaths = Vec::new();
    let mut rpaths = HashMap::new();
    for cmd in macho.commands() {
        match cmd {
            Commands::RPath(rpath) => {
                let val = rpath.path();
                all_rpaths.push(val.clone());
                let p = resolve_rpath(&val, executable_path, cwd, loader_path, dyld_library_path)
                    .context(anyhow!("failed in resolving rpath={}", val))?;
                if let Some(inner) = p {
                    rpaths.insert(val, inner);
                }
            }
            _ => {}
        };
    }
    Ok((rpaths, all_rpaths))
}

fn get_load_commands(
    macho: &lief::macho::Binary,
    macho_path: &PathBuf,
    ctx: &PathResolverCtx,
    known_libs: &HashMap<String, PathBuf>,
) -> Result<(Option<String>, HashMap<String, PathBuf>)> {
    let mut id_dylib = None;
    let mut load_cmds = HashMap::new();
    for cmd in macho.commands() {
        match cmd {
            Commands::Dylib(dylib) => match dylib.command_type() {
                LoadCommandTypes::IdDylib => id_dylib = Some(dylib.name()),
                LoadCommandTypes::LoadDylib => {
                    let val = dylib.name();
                    if is_sys_lib(&val) {
                        debug!(
                            "skipping system library {} in macho parsing, dependency of {}",
                            val, macho_path.display()
                        );
                        continue;
                    }
                    let p = resolve_load_cmd_path_with_dyld_fallback(&val, ctx, known_libs)
                        .with_context(|| {
                            format!("failed in resolving load command={} ctx={:?}", val, ctx)
                        })?;
                    match p {
                        Some(p) => {
                            let p = normalize_path(&p);
                            load_cmds.insert(val, p);
                        }
                        None => match known_libs.get(&val) {
                            None => {
                                bail!(ErrDidNotFindDependency { lib: macho_path.clone(), name: val});
                            }
                            Some(lib_path) => {
                                load_cmds.insert(val, lib_path.clone());
                            }
                        },
                    }
                }
                _ => {}
            },
            _ => {}
        };
    }
    Ok((id_dylib, load_cmds))
}

fn resolve_rpath(
    load_cmd_rpath: &str,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    loader_path: &PathBuf,
    dyld_library_path: &Vec<PathBuf>,
) -> Result<Option<PathBuf>> {
    if load_cmd_rpath.starts_with("@rpath/") {
        bail!(
            "could not resolve RPath={}, it is invalid for RPaths to start with `@rpath/`, most likely the shared library is corrupted. Raise an issue if you think this is a bug",
            load_cmd_rpath,
        );
    }
    let ctx = &PathResolverCtx {
        rpaths: Vec::new(),
        loader_path: loader_path,
        shared_lib_ctx: &SharedLibCtx {
            executable_path: executable_path,
            cwd: cwd,
            dyld_library_path,
        },
    };
    resolve_load_cmd_path(load_cmd_rpath, ctx)
}

fn resolve_load_cmd_path_with_dyld_fallback(
    load_cmd_path: &str,
    ctx: &PathResolverCtx,
    known_libs: &HashMap<String, PathBuf>,
) -> Result<Option<PathBuf>> {
    let resolved = resolve_load_cmd_path(load_cmd_path, ctx)?;
    match resolved {
        Some(resolved) => Ok(Some(resolved)),
        None => {
            let p = PathBuf::from_str(load_cmd_path).context(format!(
                "failure in converting load_cmd to PathBuf, cmd={}",
                load_cmd_path
            ))?;
            match p.file_name() {
                None => Ok(None),
                Some(file_name) => {
                    let resolved = find_in_dirs(file_name, &ctx.shared_lib_ctx.dyld_library_path)?;
                    match resolved {
                        Some(resolved) => Ok(Some(resolved)),
                        None => {
                            let file_name = file_name.to_str().with_context(|| {
                                anyhow!(
                                    "failed in converting file_name to str, file_name={}",
                                    file_name.display()
                                )
                            })?;
                            match known_libs.get(file_name) {
                                Some(lib_path) => Ok(Some(lib_path.clone())),
                                None => Ok(None),
                            }
                        }
                    }
                }
            }
        }
    }
}

fn find_in_dirs(file_name: &OsStr, dirs: &Vec<PathBuf>) -> Result<Option<PathBuf>> {
    for dir in dirs {
        let candidate = dir.join(file_name);
        if candidate.exists() {
            return Ok(Some(candidate.clone()));
        }
    }
    Ok(None)
}

/// given a load command, this function would try to resolve it
/// if it does not find any path associated with the load command, we send an `Ok(None)`
/// on every other kind of error, we send Err
fn resolve_load_cmd_path(load_cmd_path: &str, ctx: &PathResolverCtx) -> Result<Option<PathBuf>> {
    let load_path = PathBuf::from(load_cmd_path);
    if load_path.starts_with("@rpath") {
        let p = load_path.strip_prefix("@rpath").expect(&format!(
            "fatal: load_cmd_path={} should start with @rpath/",
            load_cmd_path
        ));
        for rpath in &ctx.rpaths {
            let resolved = rpath.join(p);
            if resolved.exists() {
                return Ok(Some(resolved));
            }
        }
        return Ok(None);
    } else if load_path.starts_with("@loader_path") {
        let p = load_path.strip_prefix("@loader_path").expect(&format!(
            "fatal: load_cmd_path={} should start with @loader_path/",
            load_cmd_path
        ));
        let resolved = ctx.loader_path.join(p);
        if resolved.exists() {
            return Ok(Some(resolved));
        } else {
            return Ok(None);
        }
    } else if load_path.starts_with("@executable_path") {
        let p = load_path.strip_prefix("@executable_path").expect(&format!(
            "fatal: load_cmd_path={} should start with @executable_path/",
            load_cmd_path
        ));
        let resolved = ctx.shared_lib_ctx.executable_path.join(p);
        if resolved.exists() {
            return Ok(Some(resolved));
        } else {
            return Ok(None);
        }
    } else if load_path.starts_with("/") {
        let resolved = load_path;
        if resolved.exists() {
            return Ok(Some(resolved));
        } else {
            return Ok(None);
        }
    } else {
        let resolved = ctx.shared_lib_ctx.cwd.join(load_cmd_path);
        if resolved.exists() {
            return Ok(Some(resolved));
        } else {
            return Ok(None);
        }
    }
}
