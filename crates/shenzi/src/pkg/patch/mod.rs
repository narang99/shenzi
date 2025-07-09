// patching libraries to work with the new symlink tree
// basically all install_name_tool operations

use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use anyhow::{Result, anyhow, bail};
use pathdiff::diff_paths;

use crate::{parse::Macho, pkg::patch::elf::{patch_elf, patch_elf_for_destination}};
use crate::{node::deps::Deps, parse::Binary, pkg::patch::macho::patch_macho};

mod macho;
mod elf;

pub trait LibPatch {
    fn patch(&self, real_path: &PathBuf, symlink_farm_path: &PathBuf) -> Result<()>;

    fn patch_for_destination(&self, dest_path: &PathBuf, real_path: &PathBuf, symlink_farm_path: &PathBuf) -> Result<()>;
}

impl LibPatch for Deps {
    fn patch(&self, real_path: &PathBuf, symlink_farm_path: &PathBuf) -> Result<()> {
        match self {
            Deps::Plain => Ok(()),
            Deps::Binary(binary) => {
                patch_lib(real_path, &binary, symlink_farm_path)?;
                Ok(())
            }
            #[cfg(test)]
            Deps::Mock { paths: _ } => Ok(()),
        }
    }


    fn patch_for_destination(&self, dest_path: &PathBuf, real_path: &PathBuf, symlink_farm_path: &PathBuf) -> Result<()> {
        match self {
            Deps::Plain => Ok(()),
            Deps::Binary(binary) => {
                match binary {
                    Binary::Elf(_) =>  {
                        patch_elf_for_destination(dest_path, real_path, symlink_farm_path)?;
                    },
                    _=> {}
                };
                Ok(())
            }
            #[cfg(test)]
            Deps::Mock { paths: _ } => Ok(()),
        }
    }
}

pub fn patch_lib(reals_path: &PathBuf, binary: &Binary, symlink_farm_path: &PathBuf) -> Result<()> {
    // deps is a vector of shared library names, generated from the graph
    // im assuming that symlink farm location is hardcoded here
    // TODO: make this less hardcoded, we should simply find the relative path of symlink farm from reals
    // rpaths etc should use that string instead of hardcoding everything

    match binary {
        Binary::Macho(mach) => {
            patch_macho(mach, reals_path, symlink_farm_path)?;
        }
        Binary::Elf(elf) => {
            // nothing for elfs
            // we put the reals path in the ld_library_path
            // TODO: make symlinks from reals to actual library path instead of this
            // this is a hack
            patch_elf(elf, reals_path, symlink_farm_path)?;
        }
    };
    Ok(())
}
