use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};

use crate::{
    parse::search::linux::auxval::get_at_platform,
    paths::{to_path_buf, to_string_path},
};

pub fn parse_rpath(rpath: &str, object_path: &PathBuf) -> Result<Option<PathBuf>> {
    let parent_path_str = object_path
        .parent()
        .with_context(|| anyhow!("fatal: failed to get parent of {:}", object_path.display()))
        .and_then(|parent| to_string_path(&parent))?;
    let is_64_bit = cfg!(target_pointer_width = "64");
    let lib = if is_64_bit { "lib64" } else { "lib" };
    let at_platform = get_at_platform();

    let rpath = rpath
        .replace("$ORIGIN", &parent_path_str)
        .replace("${ORIGIN}", &parent_path_str)
        .replace("$LIB", lib)
        .replace("${LIB}", lib)
        .replace("$PLATFORM", &at_platform)
        .replace("${PLATFORM}", &at_platform);

    let path = to_path_buf(&rpath)?;
    if path.exists() {
        Ok(Some(path))
    } else {
        Ok(None)
    }
}
