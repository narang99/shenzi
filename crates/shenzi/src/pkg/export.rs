use std::{fs, path::{Path, PathBuf}};

use anyhow::{Context, Result, anyhow};
use pathdiff::diff_paths;

use crate::{node::Pkg, paths::make_executable};

pub trait Export {
    fn to_destination(&self, path: &PathBuf, dest: &PathBuf, dist: &PathBuf) -> Result<()>;
}

impl Export for Pkg {
    fn to_destination(&self, path: &PathBuf, dest: &PathBuf, dist: &PathBuf) -> Result<()> {
        mk_parent_dirs(dest)?;
        match self {
            Pkg::SitePackagesPlain { _site_packages: _, alias: _, rel_path: _ }
            | Pkg::Executable
            | Pkg::PrefixPlain(_)
            | Pkg::MainPyScript
            | Pkg::ExecPrefixPlain(_) => {
                fs::copy(path, dest)?;
            },
            Pkg::PlainPyBinaryFile => {
                fs::copy(path, dest)?;
                mk_file_executable(path, true)?;
            },

            Pkg::BinaryInLDPath { symlinks, sha: _ } => {
                let (rel_path, dest_dir) = mk_symlink_in_dest(dest, dist, path)?;

                for symlink in symlinks {
                    let symlink_path = dest_dir.join(symlink);
                    if symlink_path.exists() {
                        fs::remove_file(&symlink_path)?;
                    }
                    std::os::unix::fs::symlink(&rel_path, &symlink_path).with_context(|| {
                        format!(
                            "failed in creating auxiliary symlink to destination, rel_path={} dest={}",
                            rel_path.display(),
                            dest.display()
                        )
                    })?;
                }
            },

            Pkg::SitePackagesBinary { _site_packages: _, alias: _, rel_path: _, sha: _ }
            | Pkg::Binary { sha: _ }
            | Pkg::PrefixBinary(_)
            | Pkg::ExecPrefixBinary(_) => {
                mk_symlink_in_dest(dest, dist, path)?;
            },
            | Pkg::BinaryInPath { sha: _ } => {
                mk_symlink_in_dest(dest, dist, path)?;
                mk_file_executable(path, false)?;
            },
        };
        Ok(())
    }
}


fn mk_file_executable(path: &Path, is_plain: bool) -> Result<()> {
    if is_plain {
        let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
        if ext == "py" {
            prepend_shebang(path, "#!/usr/bin/env python3")?;
        } else if ext == "sh" {
            prepend_shebang(path, "#!/usr/bin/env bash")?;
        }
    }

    make_executable(path)?;
    Ok(())
}


fn prepend_shebang(path: &Path, shebang: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let new_content = format!("{}\n{}", shebang, content);
    std::fs::write(path, new_content)?;
    Ok(())
}


fn mk_symlink_in_dest(dest: &PathBuf, dist: &PathBuf, path: &PathBuf) -> Result<(PathBuf, PathBuf)> {
    if !dest.starts_with(&dist) {
        panic!(
            "failed in moving path to destination, it is mandatory for a shared lib path to be inside dist, dist={} source_path={} destination={}",
            dist.display(),
            path.display(),
            dest.display()
        );
    }
    let parent_dir = dest.parent().expect(&format!("fatal error: tried symlinking file at dest={}, but it does not have any parent", dest.display()));
    let rel_path = diff_paths(&path, &parent_dir).ok_or_else(|| {
        anyhow!(
            "failed in finding relative path for symlinking to destination, destination={} path={}",
            dest.display(),
            path.display()
        )
    })?;
    if dest.exists() {
        fs::remove_file(&dest)?;
    }
    std::os::unix::fs::symlink(&rel_path, &dest).with_context(|| {
        format!(
            "failed in creating symlink to destination, rel_path={} dest={}",
            rel_path.display(),
            dest.display()
        )
    })?;
    Ok((rel_path, parent_dir.to_path_buf()))
}

pub fn mk_parent_dirs(dest: &PathBuf) -> std::io::Result<()> {
    if let Some(parent) = dest.parent() {
        if parent.exists() {
            return Ok(());
        }
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
