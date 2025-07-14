use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Macho {
    // all load commands, along with the resolved path of the dependency
    pub load_cmds: HashMap<String, PathBuf>,

    // all rpaths, along with resolved rpath
    pub _rpaths: HashMap<String, PathBuf>,

    // the current id of the dylib
    pub _id_dylib: Option<String>,

    // path to the lib
    pub _path: PathBuf,

    // all rpaths, unresolved, raw rpaths
    pub all_rpaths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Elf {
    // parsed and found libraries that the elf file needs, equivalent to load_commands
    pub dt_needed: HashMap<String, PathBuf>,

    // all resolvable rpaths
    pub dt_rpaths: HashMap<String, PathBuf>,

    // all resolvable runpaths
    pub _dt_runpaths: HashMap<String, PathBuf>,

    // path to lib
    pub _path: PathBuf,

    // all rpath entries (resolved and unresolved)
    pub all_dt_rpaths: Vec<String>,

    // all runpath entries (resolved and unresolved)
    pub all_dt_runpaths: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum BinaryParseError {
    UnsupportedArchitecture,
    NotBinary,
}

impl fmt::Display for BinaryParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryParseError::UnsupportedArchitecture => {
                write!(f, "Unsupported architecture")
            }
            BinaryParseError::NotBinary => {
                write!(f, "Not a binary")
            }
        }
    }
}

impl std::error::Error for BinaryParseError {}

#[derive(Debug, Clone)]
pub enum Binary {
    Macho(Macho),
    Elf(Elf),
}

impl Binary {
    pub fn dependencies(&self) -> Vec<PathBuf> {
        match self {
            Binary::Macho(macho) => macho.load_cmds.values().cloned().collect(),
            Binary::Elf(elf) => elf.dt_needed.values().cloned().collect(),
        }
    }

    pub fn paths_to_add_for_next_search(&self) -> Vec<PathBuf> {
        // DT_RPATH in linux requires us to use the current RPATH the dependencies search space too
        match self {
            Binary::Macho(_) => Vec::new(),
            Binary::Elf(elf) => elf.dt_rpaths.values().cloned().collect(),
        }
    }
}
