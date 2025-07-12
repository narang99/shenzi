use std::{env, path::PathBuf, process::exit};

use clap::Parser;
use log::info;

use crate::{
    cli::Cli,
    gather::{NodeFactory, build_graph_from_manifest},
    graph::FileGraph,
    manifest::ShenziManifest,
    paths::normalize_path,
    pkg::{bootstrap::write_bootstrap_script, move_all_nodes, move_to_dist},
};

mod cli;
mod digest;
mod factory;
mod gather;
mod graph;
mod manifest;
mod node;
mod parse;
mod paths;
mod pkg;
mod site_pkgs;

fn main() {
    env_logger::init();
    match cli::run() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{:#}", e);
            exit(1);
        }
    }
}
