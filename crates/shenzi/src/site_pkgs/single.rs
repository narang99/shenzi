// describing a single package inside site-packages

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use configparser::ini::Ini;
use log::{error, info, warn};

use crate::paths::{file_name_as_str, normalize_path};

pub struct PyPackage {
    dist_info: PathBuf,
    normalized_name: String,
    site_package_path: PathBuf,
}

impl PyPackage {
    pub fn new(dist_info: PathBuf) -> Result<Self> {
        let normalized_name = Self::normalized_name_for_dist_info(&dist_info)?;
        let site_package_path = dist_info.parent().map(|p| p.to_path_buf()).ok_or_else(|| {
            anyhow!(
                "dist_info should always have a parent, path={}",
                dist_info.display()
            )
        })?;
        Ok(Self {
            dist_info,
            normalized_name,
            site_package_path,
        })
    }

    pub fn dist_info(&self) -> &Path {
        &self.dist_info
    }

    pub fn get_dist_infos_in_dir(directory: &Path) -> Result<Vec<PathBuf>> {
        let mut result = Vec::new();

        for entry in std::fs::read_dir(directory)
            .with_context(|| format!("Failed to read directory: {}", directory.display()))?
        {
            let entry = entry.context("Failed to read a directory entry")?;
            let path = entry.path();

            if path.is_dir() && Self::is_dist_info(&path) {
                result.push(path);
            }
        }

        Ok(result)
    }

    pub fn is_dist_info(path: &PathBuf) -> bool {
        match file_name_as_str(path) {
            Err(_) => false,
            Ok(file_name) => {
                // format: {name}-{version}.dist-info
                // the first name should be exactly two components when split using -
                match file_name.strip_suffix(".dist-info") {
                    None => false,
                    Some(file_name) => {
                        let comps: Vec<&str> = file_name.split("-").collect();
                        comps.len() == 2
                    }
                }
            }
        }
    }

    fn normalized_name_for_dist_info(dist_info: &PathBuf) -> Result<String> {
        let file_name = file_name_as_str(dist_info)?;
        match file_name.split("-").next() {
            None => Err(anyhow!(
                "dist-info folder has invalid name, could not split at `-` and get the normalized name of the package, please raise this issue with the developer, path={}",
                dist_info.display()
            )),
            Some(pkg_name) => Ok(normalize_package_name(pkg_name)),
        }
    }

    pub fn should_include_in_dist(&self, allowed_packages_normalized: &HashSet<String>) -> bool {
        allowed_packages_normalized.contains(&self.normalized_name)
    }

    pub fn normalized_name(&self) -> &str {
        &self.normalized_name
    }

    fn read_record(&self) -> Result<Option<String>> {
        let record = self.dist_info.join("RECORD");
        if !record.exists() {
            return Ok(None);
        }
        Ok(Some(std::fs::read_to_string(&record)?))
    }

    pub fn get_installed_files(&self) -> Result<Vec<PathBuf>> {
        match self.read_record() {
            Err(e) => Err(e),
            Ok(None) => {
                error!(
                    "RECORD file does not exist inside dist-info folder, corrupt python package, dist-info={}. shenzi will skip this folder",
                    self.dist_info.display()
                );
                return Ok(Vec::new());
            },
            Ok(Some(contents)) => {
                Ok(self.files_from_record(&contents))
            },
        }
    }

    pub fn get_binaries(&self) -> Result<Vec<PathBuf>> {
        match self.read_record() {
            Err(e) => Err(e),
            Ok(None) => {
                Ok(Vec::new())
            },
            Ok(Some(contents)) => {
                let names = get_binary_names_from_entry_point(&self.dist_info)?;
                info!("{} binary namessssssssss: {:?}", self.dist_info.display(), names);
                Ok(self.binaries_from_record(&contents, names))
            },
        }
    }

    fn binaries_from_record(&self, record_contents: &str, binary_names: Vec<String>) -> Vec<PathBuf> {
        let binary_names: HashSet<String> = binary_names.into_iter().collect();
        self.raw_files_from_record(record_contents)
            .iter()
            .filter_map(|s| {
                let f = self.site_package_path.join(s);
                if f.exists() {
                    match file_name_as_str(&f) {
                        Ok(file_name) => {
                            if binary_names.contains(&file_name) {
                                Some(normalize_path(&f))
                            } else {
                                None
                            }
                        },
                        Err(_) => None,
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    fn files_from_record(&self, record_contents: &str) -> Vec<PathBuf> {
        self.raw_files_from_record(record_contents)
            .iter()
            .filter_map(|s| {
                let f = self.site_package_path.join(s);
                if f.exists() {
                    let f = normalize_path(&f);
                    if f.starts_with(&self.site_package_path) {
                        Some(f)
                    } else {
                        warn!(
                            "found a file in dist-info record which is outside site-packages, skipping. site_package_path={} dist_info={} file_path={}",
                            self.site_package_path.display(), self.dist_info.display(), f.display()
                        );
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    fn raw_files_from_record(&self, record_contents: &str) -> Vec<String> {
        record_contents
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(2, ',');
                parts.next().map(|s| s.to_string())
            })
            .collect()
    }
}

pub fn normalize_package_name(name: &str) -> String {
    name.replace(['-', '_', '.'], "_").to_lowercase()
}

fn get_binary_names_from_entry_point(dist_info: &Path) -> Result<Vec<String>> {
    let mut res = Vec::new();
    let entry_points = dist_info.join("entry_points.txt");
    if !entry_points.exists() {
        return Ok(Vec::new());
    }
    let mut config = Ini::new();
    match config.load(&entry_points) {
        Err(e) => {
            error!(
                "failed in reading entry_points.txt for dist={}, skipping entry points, e={}",
                dist_info.display(),
                e
            );
            return Ok(Vec::new());
        }
        Ok(config) => {
            if let Some(scripts) = config.get("console_scripts") {
                for (cmd, target) in scripts.iter() {
                    if target.is_some() {
                        res.push(cmd.clone());
                    }
                }
            }
        }
    };
    Ok(res)
}
