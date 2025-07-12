use std::path::PathBuf;

use anyhow::{Result, anyhow};
use pathdiff::diff_paths;

use crate::{
    digest::make_digest,
    manifest::Version,
    node::{Pkg, PrefixBinary, PrefixPlain},
};

pub fn get_exec_prefix_pkg(
    path: &PathBuf,
    original_prefix: &PathBuf,
    version: &Version,
    is_shared_library: bool,
) -> Result<Pkg> {
    let rel_path = diff_paths(&path, &original_prefix).ok_or_else(|| {
        anyhow!(
            "failed in finding relative path of file inside prefix file={} prefix={}",
            path.display(),
            original_prefix.display()
        )
    })?;
    let original_prefix = original_prefix.clone();
    let version = version.clone();
    if is_shared_library {
        Ok(Pkg::ExecPrefixBinary(PrefixBinary {
            _original_prefix: original_prefix,
            version,
            rel_path,
            sha: make_digest(path)?,
        }))
    } else {
        Ok(Pkg::ExecPrefixPlain(PrefixPlain {
            _original_prefix: original_prefix,
            rel_path,
            version,
        }))
    }
}

pub fn get_prefix_pkg(path: &PathBuf, original_prefix: &PathBuf, version: &Version, is_shared_library: bool) -> Result<Pkg> {
    let rel_path = diff_paths(&path, &original_prefix).ok_or_else(|| {
        anyhow!(
            "failed in finding relative path of file inside prefix file={} prefix={}",
            path.display(),
            original_prefix.display()
        )
    })?;
    let original_prefix = original_prefix.clone();
    let version = version.clone();
    if is_shared_library {
        Ok(Pkg::PrefixBinary(PrefixBinary {
            _original_prefix: original_prefix,
            version,
            rel_path,
            sha: make_digest(path)?,
        }))
    } else {
        Ok(Pkg::PrefixPlain(PrefixPlain {
            _original_prefix: original_prefix,
            rel_path,
            version,
        }))
    }
}

pub fn get_site_packages_pkg(
    path: &PathBuf,
    site_pkg_path: &PathBuf,
    alias: &str,
    _version: &Version,
    is_shared_library: bool,
) -> Result<Pkg> {
    let rel_path = diff_paths(&path, &site_pkg_path).ok_or_else(|| {
        anyhow!(
            "failed in finding relative path of file inside site-packages, file={} site-packages={}",
            path.display(),
            site_pkg_path.display()
        )
    })?;
    let site_packages = site_pkg_path.clone();
    let alias = alias.to_string();
    if is_shared_library {
        Ok(Pkg::SitePackagesBinary {
            _site_packages: site_packages,
            alias,
            rel_path,
            sha: make_digest(path)?,
        })
    } else {
        Ok(Pkg::SitePackagesPlain {
            _site_packages: site_packages,
            alias,
            rel_path,
        })
    }
}
