use crate::{parse::Macho, paths::get_lib_name};
// patching libraries to work with the new symlink tree
// basically all install_name_tool operations

use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use anyhow::{Result, anyhow, bail};
use pathdiff::diff_paths;

pub fn patch_macho(mach: &Macho, reals_path: &PathBuf, symlink_farm_path: &PathBuf) -> Result<()> {
    if mach.load_cmds.len() == 0 {
        return Ok(());
    }
    // the order of these operations are important
    // for many dylibs, if you simply modify a load command, and if the size of the new load command is bigger than the older one
    // install_name_tool will fail, saying that it cant fit more commands in it
    // if you simply add a new rpath, this same error can occur
    // so first we remove all rpaths to make space
    // then we modify load commands
    // and finally we add the new rpath
    // generally our load_commands would be smaller than the older ones
    // because we simply use @rpath/libname, this is smaller than almost every other prefix based path system
    // only libname as a relative path is generally smaller
    // it is working well in practice

    // TODO: create a fallback which would create symlink farms in every place the original dylib expects data to be in
    // if this whole procedure fails (we can't fit at all), we have to simply make a structure that works with existing dylib
    // the simple solution is to go through all load commands, find the directory where dyld will look for each load command, and make a symlink farm there
    // absolute paths need to be handled separately though, same with relative paths that may go out of dist
    // hopefully i won't have to do this
    // although i believe absolute paths should be easily replaceable by relative paths, a relative path is strictly smaller than the absolute path
    // its almost impossible to make relative paths work though (our directory of calling the binary is not fixed, although the bootstrap script can do this for us)
    // the first hope is to replace with rpaths, and then see what can be done
    // in any case, the operation is basically a map on load commands at a top level (which changes them to smaller variants) given an rpath
    for rpath in &mach.all_rpaths {
        rm_rpath(rpath, reals_path)?;
    }
    let lib_name = get_lib_name(reals_path)?;
    let rpath = get_new_rpath(reals_path, symlink_farm_path)?;
    modify_load_cmds(reals_path, symlink_farm_path, mach)?;
    add_rpath(&rpath, reals_path)?;
    set_dylib_id(dylib_id(&lib_name), &reals_path)?;
    sign_dylib(&reals_path)?;

    Ok(())
}

fn modify_load_cmds(reals_path: &PathBuf, symlink_farm_path: &PathBuf, mach: &Macho) -> Result<()> {
    for (load_cmd, parent_path) in &mach.load_cmds {
        let lib_name = get_lib_name(&parent_path)?;
        let lib_in_farm = symlink_farm_path.join(&lib_name);
        if !lib_in_farm.exists() {
            bail!(
                "fatal: did not find dependency in symlink farm, path={} symlink-farm={} lib_name={}",
                reals_path.display(),
                symlink_farm_path.display(),
                lib_name
            );
        }
        modify_load_command(&load_cmd, &dylib_id(&lib_name), reals_path)?;
        sign_dylib(reals_path)?;
    }
    Ok(())
}

fn dylib_id(lib_name: &str) -> String {
    format!("@rpath/{}", lib_name)
}


fn get_new_rpath(real_path: &PathBuf, symlink_farm: &PathBuf) -> Result<String> {
    let real_path_dir = real_path.parent().ok_or_else(|| {
        anyhow!(
            "failed in getting parent of real_path while patching it, path={}",
            real_path.display()
        )
    })?;
    let rel_path = diff_paths(symlink_farm, real_path_dir).ok_or_else(|| {
        anyhow!(
            "failed in finding relative path of symlink farm during patching, symlink_farm={} real_path={}",
            symlink_farm.display(),
            real_path.display()
        )
    })?;
    let rel_path = rel_path.to_str().map(|s| s.to_string()).ok_or_else(|| {
        anyhow!(
            "failed in converting path to string for adding as rpath, path={}",
            rel_path.display()
        )
    })?;
    Ok(format!("@loader_path/{}/", rel_path))
}

fn rm_rpath(rpath: &str, path: &PathBuf) -> Result<()> {
    let status = Command::new("install_name_tool")
        .stderr(Stdio::null())
        .arg("-delete_rpath")
        .arg(rpath)
        .arg(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        bail!(
            "failed in running install_name_tool to delete rpath path={} rpath={} status={:?}",
            rpath,
            path.display(),
            status
        )
    }
}

fn add_rpath(rpath: &str, path: &PathBuf) -> Result<()> {
    let status = Command::new("install_name_tool")
        .stderr(Stdio::null())
        .arg("-add_rpath")
        .arg(rpath)
        .arg(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        bail!(
            "failed in running install_name_tool to add rpath path={} rpath={} status={:?}",
            path.display(),
            rpath,
            status
        )
    }
}

fn modify_load_command(old: &str, new: &str, path: &PathBuf) -> Result<()> {
    let status = Command::new("install_name_tool")
        .stderr(Stdio::null())
        .arg("-change")
        .arg(old)
        .arg(new)
        .arg(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        bail!(
            "failed in running install_name_tool to modifying load command path={} old={} new={} status={:?}",
            old,
            new,
            path.display(),
            status
        )
    }
}

fn set_dylib_id(id: String, path: &PathBuf) -> Result<()> {
    let status = Command::new("install_name_tool")
        .stderr(Stdio::null())
        .arg("-id")
        .arg(&id)
        .arg(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        bail!(
            "failed in running install_name_tool to set dylib_id path={} id={} status={:?}",
            id,
            path.display(),
            status
        )
    }
}

fn sign_dylib(path: &PathBuf) -> Result<()> {
    let status = Command::new("codesign")
        .stderr(Stdio::null())
        .arg("-s")
        .arg("-")
        .arg("-f")
        .arg(&path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        bail!(
            "failed in running signing dylib_id path={} status={:?}",
            path.display(),
            status
        )
    }
}
