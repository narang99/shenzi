mod core;
mod elf;
mod macho;
mod search;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
pub use core::{Binary, BinaryParseError, Elf, Macho};
use std::collections::HashSet;
use log::warn;
use std::collections::HashMap;
use std::path::PathBuf;

use elf::parse as parse_linux;
use macho::parse as parse_macho;
// pub use macho::get_deps_from_macho;

pub fn parse_and_search(
    path: &PathBuf,
    executable_path: &PathBuf,
    cwd: &PathBuf,
    env: &HashMap<String, String>,
    known_libs: &HashMap<String, PathBuf>,
    extra_rpaths: &Vec<PathBuf>,
) -> Result<Binary> {
    // TODO: take a set instead of doing this, this is very inefficient way of doing this
    let extra_rpaths = &deduplicate_paths(extra_rpaths);
    let mut file =
        std::fs::File::open(path).context(anyhow!("Can't open the file={}", path.display()))?;
    let os = std::env::consts::OS;
    let binary = match lief::Binary::from(&mut file) {
        Some(lief::Binary::ELF(elf)) => {
            if os != "linux" {
                warn!("found an ELF file in non-linux system, path={}", path.display());
                return Err(Error::new(BinaryParseError::UnsupportedArchitecture));
            }
            let elf = parse_linux(elf, path, cwd, env, extra_rpaths, known_libs)?;
            Binary::Elf(elf)
        }
        Some(lief::Binary::MachO(macho)) => {
            if os != "macos" {
                warn!("found a MACHO file in non-macos system, path={}", path.display());
                return Err(Error::new(BinaryParseError::UnsupportedArchitecture));
            }
            let macho = parse_macho(macho, path, executable_path, cwd, env, known_libs)?;
            Binary::Macho(macho)
        }
        Some(lief::Binary::PE(_)) => {
            warn!(
                "windows PE object files are not supported: {}",
                path.display()
            );
            return Err(Error::new(BinaryParseError::UnsupportedArchitecture));
        }
        None => {
            return Err(Error::new(BinaryParseError::NotBinary));
        }
    };

    Ok(binary)
}


fn deduplicate_paths(paths: &Vec<PathBuf>) -> Vec<PathBuf> {
    let mut set = HashSet::new();
    for path in paths {
        set.insert(path.clone());
    }
    set.into_iter().collect()
}