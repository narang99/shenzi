use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone)]
pub enum PythonPathComponent {
    RelativeToLibDynLoad {
        rel_path: PathBuf,
    },
    RelativeToStdlib {
        rel_path: PathBuf,
    },
    TopLevel {
        alias: String,
    },
    RelativeToSitePkg {
        top_level_alias: String,
        rel_path: PathBuf,
    },
}

pub fn get_python_path_mapping(
    site_pkg_by_alias: &HashMap<PathBuf, String>,
    stdlib: &PathBuf,
    lib_dynload: &PathBuf,
    all_site_pkgs: &Vec<PathBuf>,
) -> Vec<PythonPathComponent> {
    // given a potentially nested site packages
    // we need to find provide a map or something of each site-package, its alias, and how it relates to some other site-package
    let top_level_site_pkgs: Vec<&PathBuf> = site_pkg_by_alias.keys().collect();
    let mut res = Vec::new();
    for site_pkg in all_site_pkgs {
        if site_pkg == lib_dynload {
            continue
        } else if site_pkg == stdlib {
            continue
        }
        let maybe_alias = site_pkg_by_alias.get(site_pkg);
        match maybe_alias {
            Some(alias) => {
                res.push(PythonPathComponent::TopLevel {
                    alias: alias.to_string(),
                });
            }
            None => {
                // if its not in site_pkg_by_alias, its a nested path, can be one of any
                // if this returns `Some`, do push
                // else if it returns `None`, go to next
                let comp = get_nested_pkg_component(
                    site_pkg,
                    stdlib,
                    lib_dynload,
                    site_pkg_by_alias,
                    &top_level_site_pkgs,
                ).unwrap_or_else( || {
                    panic!(
                        "fatal error, impossible condition: failed in finding nested component for site_package even though it was not top level site_pkg={} stdlib={} lib_dynload={} site_pkg_by_alias={:?} top_level_site_pkgs={:?}",
                        site_pkg.display(),
                        stdlib.display(),
                        lib_dynload.display(),
                        site_pkg_by_alias,
                        top_level_site_pkgs,
                    );
                });
                res.push(comp);
            }
        }
    }
    res
}

fn get_nested_pkg_component(
    site_pkg: &PathBuf,
    stdlib: &PathBuf,
    lib_dynload: &PathBuf,
    site_pkg_by_alias: &HashMap<PathBuf, String>,
    top_level_pkgs: &Vec<&PathBuf>,
) -> Option<PythonPathComponent> {
    let from_stdlib = get_relative_site_pkg_from(site_pkg, stdlib).map(|rel_path| {
        PythonPathComponent::RelativeToStdlib {
            rel_path: rel_path.clone(),
        }
    });
    from_stdlib
        .or_else(|| {
            get_relative_site_pkg_from(site_pkg, lib_dynload).map(|rel_path| {
                PythonPathComponent::RelativeToLibDynLoad {
                    rel_path: rel_path.clone(),
                }
            })
        })
        .or_else(|| {
            get_relative_from_top_level_site_pkgs(site_pkg, site_pkg_by_alias, top_level_pkgs)
        })
}

fn get_relative_from_top_level_site_pkgs(
    site_pkg: &PathBuf,
    site_pkg_by_alias: &HashMap<PathBuf, String>,
    top_level_pkgs: &Vec<&PathBuf>,
) -> Option<PythonPathComponent> {
    for candidate in top_level_pkgs {
        match get_relative_site_pkg_from(site_pkg, candidate) {
            Some(rel_path) => {
                let top_level_alias = site_pkg_by_alias.get(*candidate).expect(
                    &format!(
                        "fatal: impossible error. alias was not found for site_pkg candidate: candidate={} site_pkg_by_alias={:?}",
                        candidate.display(),
                        site_pkg_by_alias
                    )
                ).to_string();
                return Some(PythonPathComponent::RelativeToSitePkg {
                    top_level_alias,
                    rel_path: rel_path.to_path_buf(),
                });
            }
            None => {}
        }
    }
    None
}

fn get_relative_site_pkg_from(site_pkg: &PathBuf, base: &PathBuf) -> Option<PathBuf> {
    if site_pkg.starts_with(base) {
        let rel_path = site_pkg.strip_prefix(base).expect(
            &format!(
                "fatal: impossible error, `strip_prefix` returned error even though site_pkg starts_with base, site_pkg={} base={}",
                site_pkg.display(),
                base.display(),
            )
        );
        Some(rel_path.to_path_buf())
    } else {
        None
    }
}
