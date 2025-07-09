use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;

use crate::parse::BinaryParseError;

use crate::parse::Binary;

#[derive(Debug, Clone)]
pub enum Deps {
    Plain,
    Binary(Binary),

    #[cfg(test)]
    Mock {
        paths: Vec<PathBuf>,
    },
}

impl Deps {
    pub fn new_binary(
        path: &PathBuf,
        executable_path: &PathBuf,
        cwd: &PathBuf,
        env: &HashMap<String, String>,
        known_libs: &HashMap<String, PathBuf>,
    ) -> Result<Deps> {
        let bin = crate::parse::parse_and_search(
            path,
            executable_path,
            cwd,
            env,
            known_libs,
            &Vec::new(),
        );
        match bin {
            Ok(bin) => Ok(Deps::Binary(bin)),
            Err(e) => {
                if let Some(parse_err) = e.downcast_ref::<BinaryParseError>() {
                    match parse_err {
                        BinaryParseError::UnsupportedArchitecture | BinaryParseError::NotBinary => {
                            Ok(Deps::Plain)
                        }
                    }
                } else {
                    Err(e)
                }
            }
        }
    }

    pub fn paths_to_add_for_next_search(&self) -> Vec<PathBuf> {
        match self {
            Deps::Binary(binary) => binary.paths_to_add_for_next_search(),
            _ => Vec::new(),
        }
    }

    pub fn find(&self) -> Result<Vec<PathBuf>> {
        match &self {
            Deps::Plain => Ok(Vec::new()),
            Deps::Binary(binary) => Ok(binary.dependencies()),
            #[cfg(test)]
            Deps::Mock { paths } => Ok(paths.clone()),
        }
    }

    pub fn from_path(
        path: &PathBuf,
        executable_path: &PathBuf,
        cwd: &PathBuf,
        env: &HashMap<String, String>,
        known_libs: &HashMap<String, PathBuf>,
    ) -> Result<Deps> {
        let ext = path.extension();
        match ext {
            None => Ok(Deps::Plain),
            Some(ext) => {
                if ext == "so" || ext == "dylib" {
                    Ok(Deps::new_binary(
                        path,
                        executable_path,
                        cwd,
                        env,
                        known_libs,
                    )?)
                } else {
                    Ok(Deps::Plain)
                }
            }
        }
    }

    #[cfg(test)]
    pub fn mock(deps: Vec<PathBuf>) -> Deps {
        Deps::Mock { paths: deps }
    }

    pub fn is_shared_library(&self) -> bool {
        match self {
            Deps::Binary(_) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::PathBuf};

    use crate::node::deps::Deps;

    // todo: this only works on my machine
    #[test]
    fn test_local() {
        let path =
            PathBuf::from("/Users/hariomnarang/miniconda3/envs/platform/lib/libpango-1.0.0.dylib");
        let executable_path = PathBuf::from("/Users/hariomnarang/miniconda3/bin/python");
        let env = HashMap::new();
        let known_libs = HashMap::new();
        let cwd = PathBuf::from(".");
        let dylib = Deps::new_binary(&path, &executable_path, &cwd, &env, &known_libs).unwrap();
        let dylib = dylib.find().unwrap();
        dbg!(dylib);
    }
}
