use std::path::Path;

use anyhow::{Context, Result};

pub fn get_all_valid_package_names_in_path(directory: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(directory)
        .with_context(|| format!("Failed to read directory: {}", directory.display()))?
    {
        let entry = entry.context("Failed to read a directory entry")?;
        let path = entry.path();
        if path.is_dir() && path.join("__init__.py").is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                names.push(name.to_string());
            }
        }
    }
    Ok(names)
}
