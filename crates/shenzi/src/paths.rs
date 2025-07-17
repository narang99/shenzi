// basic path operations
// given a macho to parse
// we return the rpaths if they exist
// loader-path would be simply the current path
// we also want executable-path as an input

use anyhow::{Context, Result, anyhow};
use log::warn;

use lazy_static::lazy_static;
use regex::Regex;
use std::{
    path::{Component, Path, PathBuf},
    str::FromStr,
};

pub fn marker_file_name() -> String {
    "SHENZI_MARKER".to_string()
}

pub fn marker_file_path(dist: &Path) -> PathBuf {
    dist.join(marker_file_name())
}

pub fn normalize_path(path: &Path) -> PathBuf {
    // copied from cargo
    // https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
    // basically `canonicalize`, but does not require the path to exist
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

pub fn is_sys_lib_mac(path: &str) -> bool {
    path.starts_with("/usr/lib/")
        || path.starts_with("/System/Library/Frameworks/")
        || path.starts_with("/System/Library/PrivateFrameworks/")
}

pub fn is_sys_lib_linux(path: &str) -> bool {
    if let Ok(fname) = file_name_as_str(&PathBuf::from(path)) {
        if fname.starts_with("libc.so") || fname.starts_with("libpthread.so") {
            return true;
        }
    }
    return false;
}


pub fn to_string_path(path: &Path) -> Result<String> {
    path.to_str().map(|s| s.to_string()).with_context(|| {
        anyhow!(
            "failed in getting string representation of file path={}",
            path.display()
        )
    })
}

pub fn to_path_buf(path: &str) -> Result<PathBuf> {
    PathBuf::from_str(path)
        .with_context(|| anyhow!("failed in getting path from string path={}", path))
}

pub fn get_valid_paths(ps: &Vec<String>) -> Vec<PathBuf> {
    let mut res = Vec::new();
    for p in ps {
        let path = PathBuf::from_str(p);
        match path {
            Ok(path) => {
                if path.exists() && path.is_dir() {
                    res.push(path);
                }
            }
            Err(e) => {
                warn!("path parse failure: {p}: {e}");
            }
        }
    }
    res
}

pub fn split_colon_separated_into_valid_search_paths(term: Option<&String>) -> Vec<PathBuf> {
    match term {
        None => Vec::new(),
        Some(term) => {
            let paths: Vec<String> = term.split(":").map(|s| s.to_string()).collect();
            get_valid_paths(&paths)
        }
    }
}

pub fn file_name_as_str(path: &PathBuf) -> Result<String> {
    let lib_name = path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .ok_or_else(|| {
            anyhow!(
                "failed in getting real file name for path={}",
                path.display()
            )
        })?;
    Ok(lib_name.to_string())
}

lazy_static! {
    static ref SONAME_RE: Regex = Regex::new(r"\.so([.\da-zA-Z]*)$")
        .expect("failed to compiled regex for detecting shared library names");
}
pub fn is_maybe_object_file(path: &Path) -> bool {
    // this only checks the path extensions
    // they are not very reliable
    // you should try parsing after this to see if they really are a shared library
    match path.to_str() {
        None => false,
        Some(path) => {
            if path.ends_with(".dylib") {
                true
            } else if SONAME_RE.is_match(path) {
                true
            } else {
                false
            }
        }
    }
}

pub fn get_root_dirs() -> Vec<PathBuf> {
    // TODO: when adding windows support, change this
    return vec![PathBuf::from("/")];
}

pub fn cache_loc() -> Result<PathBuf> {
    let loc = if let Ok(xdg_cache_home) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(xdg_cache_home).join("shenzi")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cache").join("shenzi")
    } else {
        PathBuf::from("/tmp/shenzi")
    };

    std::fs::create_dir_all(&loc)
        .context(anyhow!("failed in creating cache at {}", loc.display()))?;
    Ok(loc)
}

pub fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&path)
        .context("failed to get patchelf permissions")?
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).context("failed to set patchelf permissions")?;
    Ok(())
}
