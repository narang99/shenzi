use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, anyhow, bail};
use pathdiff::diff_paths;

use crate::{external::patchelf_path, parse::Elf, paths::file_name_as_str};

pub fn patch_elf_for_destination(
    dest_path: &PathBuf,
    real_path: &PathBuf,
    symlink_farm_path: &PathBuf,
) -> Result<()> {
    let rpath = get_new_rpath(dest_path, symlink_farm_path)?;
    add_rpath(&rpath, real_path)?;
    Ok(())
}

pub fn patch_elf(elf: &Elf, reals_path: &PathBuf, symlink_farm_path: &PathBuf) -> Result<()> {
    // TODO: add support to download patchelf if it does not exist
    // TODO: linux does not need a symlink farm, the reals path would simply be the libname
    // and we just add everything in the same folder, the final rpath would also simply be $ORIGIN
    // for now doing the mac structure
    // why this matters is that the rpath can be bigger than what is there originally in the binary
    // this might result in us not being able to patch it
    if elf.dt_needed.len() == 0 {
        // has no dependency, no need to patch
        return Ok(());
    }
    if elf.all_dt_rpaths.len() + elf.all_dt_runpaths.len() > 0 {
        // only remove rpath if there is any
        rm_rpath(reals_path).with_context(|| {
            anyhow!(
                "failed in removing RPATH for {}, all_dt_rpaths={:?} all_dt_runpaths={:?}",
                reals_path.display(),
                elf.all_dt_rpaths,
                elf.all_dt_runpaths
            )
        })?;
    }
    add_rpath(&get_new_rpath(reals_path, symlink_farm_path)?, reals_path)?;
    modify_all_dt_needed(reals_path, symlink_farm_path, elf)?;
    Ok(())
}

fn rm_rpath(path: &PathBuf) -> Result<()> {
    let output = Command::new(patchelf()?)
        .stderr(Stdio::null())
        .arg("--remove-rpath")
        .arg(path)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        bail!(
            "failed in running patchelf to delete rpath path={} status={:?} stdout={} stderr={}",
            path.display(),
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )
    }
}

fn add_rpath(rpath: &str, path: &PathBuf) -> Result<()> {
    let status = Command::new(patchelf()?)
        .stderr(Stdio::null())
        .arg("--add-rpath")
        .arg(rpath)
        .arg(path)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        bail!(
            "failed in running patchelf to set rpath path={} rpath={} status={:?}",
            path.display(),
            rpath,
            status
        )
    }
}

// TODO: remove this duplication from mac
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
    Ok(format!("$ORIGIN/{}/", rel_path))
}

fn modify_all_dt_needed(
    reals_path: &PathBuf,
    symlink_farm_path: &PathBuf,
    elf: &Elf,
) -> Result<()> {
    for (old, parent_path) in &elf.dt_needed {
        let lib_name = file_name_as_str(&parent_path)?;
        let lib_in_farm = symlink_farm_path.join(&lib_name);
        if !lib_in_farm.exists() {
            bail!(
                "fatal: did not find dependency in symlink farm, path={} symlink-farm={} lib_name={}",
                reals_path.display(),
                symlink_farm_path.display(),
                lib_name
            );
        }
        modify_dt_needed(&old, &lib_name, reals_path)?;
    }
    Ok(())
}

fn modify_dt_needed(old: &str, new: &str, path: &PathBuf) -> Result<()> {
    let status = Command::new(patchelf()?)
        .stderr(Stdio::null())
        .arg("--replace-needed")
        .arg(old)
        .arg(new)
        .arg(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        bail!(
            "failed in running patchelf to modifying DT_NEEDED path={} old={} new={} status={:?}",
            old,
            new,
            path.display(),
            status
        )
    }
}

fn patchelf() -> Result<String> {
    let p = patchelf_path()?;
    p.to_str().map(|p| p.to_string()).ok_or(anyhow!("failed in converting patchelf path to string"))
}