// basic parser for poetry.lock files

use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use toml;

#[derive(Debug, Deserialize, Serialize)]
pub struct PoetryLock {
    pub package: Vec<PoetryPackage>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PoetryPackage {
    pub name: String,
    #[serde(default)]
    pub groups: Vec<String>,
}

pub fn get_required_dependencies(
    config_file: &Path,
    allowed_groups: &Vec<String>,
) -> Result<Vec<String>> {
    let contents = std::fs::read_to_string(config_file)?;
    get_required_deps_from_string(&contents, allowed_groups)
}

pub fn ask_user_for_groups() -> Result<Vec<String>> {
    let comma_separated = crate::ask::ask_user(
        "Which dependency groups should be kept in the final distribution? Generally you only want the main group (all dependencies in project.dependencies). You can add `dev` for development dependencies. For custom groups, just pass them as is (comma separated, default: main)",
        &Some(String::from("main"))
    )?;
    let res: Vec<String> = comma_separated
        .split(",")
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    Ok(res)
}

fn get_required_deps_from_string(
    contents: &str,
    allowed_groups: &Vec<String>,
) -> Result<Vec<String>> {
    let poetry_lock: PoetryLock = toml::from_str(&contents)?;

    let dependencies: Vec<String> = poetry_lock
        .package
        .iter()
        .filter(|pkg| any_in_groups(&pkg.groups, allowed_groups))
        .map(|pkg| pkg.name.clone())
        .collect();

    Ok(dependencies)
}

fn any_in_groups(pkg_groups: &Vec<String>, allowed: &Vec<String>) -> bool {
    for g in pkg_groups {
        if allowed.contains(g) {
            return true;
        }
    }
    return false;
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse() {
        let poetry_lock_content = r#"
# This file is automatically @generated by Poetry 2.1.3 and should not be changed by hand.

[[package]]
name = "annotated-types"
version = "0.7.0"
description = "Reusable constraint types to use with typing.Annotated"
optional = false
python-versions = ">=3.8"
groups = ["main"]
files = [
    {file = "annotated_types-0.7.0-py3-none-any.whl", hash = "sha256:7e2b1c3a4d5f6e8b9a0c1d2e3f4b5a6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a"},
    {file = "annotated_types-0.7.0.tar.gz", hash = "sha256:3f2e1d0c9b8a7f6e5d4c3b2a1f0e9d8c7b6a5f4e3d2c1b0a9e8f7d6c5b4a3e2d"},
]

[[package]]
name = "cachetools"
version = "6.1.0"
description = "Extensible memoizing collections and decorators"
optional = false
python-versions = ">=3.9"
groups = ["dev"]
files = [
    {file = "cachetools-6.1.0-py3-none-any.whl", hash = "sha256:abc123def4567890fedcba0987654321abcdef1234567890fedcba0987654321"},
    {file = "cachetools-6.1.0.tar.gz", hash = "sha256:123abc456def7890cba0987654321fedcba0987654321abcdef1234567890fed"},
]

[[package]]
name = "chardet"
version = "5.2.0"
description = "Universal encoding detector for Python 3"
optional = false
python-versions = ">=3.7"
groups = ["dev"]
files = [
    {file = "chardet-5.2.0-py3-none-any.whl", hash = "sha256:deadbeefcafebabe1234567890abcdef1234567890abcdef1234567890abcdef"},
    {file = "chardet-5.2.0.tar.gz", hash = "sha256:beefdeadbabe1234567890abcdef1234567890abcdef1234567890abcdef1234"},
]

[[package]]
name = "colorama"
version = "0.4.6"
description = "Cross-platform colored terminal text."
optional = false
python-versions = "!=3.0.*,!=3.1.*,!=3.2.*,!=3.3.*,!=3.4.*,!=3.5.*,!=3.6.*,>=2.7"
groups = ["dev"]
files = [
    {file = "colorama-0.4.6-py2.py3-none-any.whl", hash = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"},
    {file = "colorama-0.4.6.tar.gz", hash = "sha256:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"},
]

[[package]]
name = "coverage"
version = "7.9.2"
description = "Code coverage measurement for Python"
optional = false
python-versions = ">=3.9"
groups = ["dev"]
files = [
    {file = "coverage-7.9.2-cp310-cp310-macosx_10_9_x86_64.whl", hash = "sha256:11223344556677889900aabbccddeeff00112233445566778899aabbccddeeff"},
    {file = "coverage-7.9.2-cp310-cp310-macosx_11_0_arm64.whl", hash = "sha256:ffeeddccbbaa0099887766554433221100ffeeddccbbaa009988776655443322"},
]

[package.extras]
toml = ["tomli ; python_full_version <= \"3.11.0a6\""]

[[package]]
name = "distlib"
version = "0.3.9"
description = "Distribution utilities"
optional = false
python-versions = "*"
groups = ["dev"]
files = [
    {file = "distlib-0.3.9-py2.py3-none-any.whl", hash = "sha256:99887766554433221100ffeeddccbbaa99887766554433221100ffeeddccbbaa"},
    {file = "distlib-0.3.9.tar.gz", hash = "sha256:aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899"},
]

[[package]]
name = "execnet"
version = "2.1.1"
description = "execnet: rapid multi-Python deployment"
optional = false
python-versions = ">=3.8"
groups = ["dev"]
files = [
    {file = "execnet-2.1.1-py3-none-any.whl", hash = "sha256:0f1e2d3c4b5a69788796a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4"},
    {file = "execnet-2.1.1.tar.gz", hash = "sha256:4d5c6b7a8f9e0d1c2b3a4e5f6d7c8b9a0e1f2d3c4b5a69788796a5b4c3d2e1f0"},
]

[package.extras]
testing = ["hatch", "pre-commit", "pytest", "tox"]

[[package]]
name = "filelock"
version = "3.18.0"
description = "A platform independent file lock."
optional = false
python-versions = ">=3.9"
groups = ["dev"]
files = [
    {file = "filelock-3.18.0-py3-none-any.whl", hash = "sha256:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"},
    {file = "filelock-3.18.0.tar.gz", hash = "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcd"},
]

[package.extras]
docs = ["furo (>=2024.8.6)", "sphinx (>=8.1.3)", "sphinx-autodoc-typehints (>=3)"]
testing = ["covdefaults (>=2.3)", "coverage (>=7.6.10)", "diff-cover (>=9.2.1)", "pytest (>=8.3.4)", "pytest-asyncio (>=0.25.2)", "pytest-cov (>=6)", "pytest-mock (>=3.14)", "pytest-timeout (>=2.3.1)", "virtualenv (>=20.28.1)"]
typing = ["typing-extensions (>=4.12.2) ; python_version < \"3.11\""]

[[package]]
name = "iniconfig"
version = "2.1.0"
description = "brain-dead simple config-ini parsing"
optional = false
python-versions = ">=3.8"
groups = ["dev"]
files = [
    {file = "iniconfig-2.1.0-py3-none-any.whl", hash = "sha256:fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"},
    {file = "iniconfig-2.1.0.tar.gz", hash = "sha256:0fedcba9876543210fedcba9876543210fedcba9876543210fedcba987654321"},
]

[[package]]
name = "packaging"
version = "25.0"
description = "Core utilities for Python packages"
optional = false
python-versions = ">=3.8"
groups = ["main", "dev"]
files = [
    {file = "packaging-25.0-py3-none-any.whl", hash = "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"},
    {file = "packaging-25.0.tar.gz", hash = "sha256:7890abcdef1234567890abcdef1234567890abcdef1234567890abcdef123456"},
]
"#;

        let allowed_groups = vec!["main".to_string()];
        let result =
            super::get_required_deps_from_string(poetry_lock_content, &allowed_groups).unwrap();

        // Only packages with "main" in their groups should be included
        // "annotated-types" has ["main"]
        // "packaging" has ["main", "dev"]
        // All others have only ["dev"]
        let mut expected = vec!["annotated-types".to_string(), "packaging".to_string()];
        expected.sort();
        let mut result_sorted = result.clone();
        result_sorted.sort();
        assert_eq!(result_sorted, expected);
    }
}
