use std::{env, path::PathBuf};

use log::info;

use crate::{
    gather::{build_graph_from_manifest, NodeFactory}, graph::FileGraph, manifest::ShenziManifest, paths::normalize_path, pkg::{bootstrap::write_bootstrap_script, move_to_dist}
};

pub mod digest;
pub mod gather;
pub mod graph;
pub mod manifest;
pub mod node;
pub mod paths;
pub mod pkg;
pub mod parse;
pub mod factory;
pub mod site_pkgs;

/**
 * Algorithm:
 * - enumerate all files that are part of this python package
 * - create a node for each of these that can be inserted in the graph
 * - insert all nodes in the graph, without doing dependency analysis
 * - add all nodes in the second pass, but with dependency analysis this time, creating edges
 *   - this operation can add new nodes that are not present in the graph too
 * - Topologically sort the graph, call to_destination for each node
 * - create bootstrap scripts
 *
 * All file types
 * - python file inside a site-packages folder
 * - unknown file inside a site-packages folder
 *   - both have same handling (plain file basically)
 * - plain file inside prefix
 *   - the handling is similar to site-packages, other than destination in final directory
 * - dylib inside prefix
 *   - handling same as dylib, but destination different
 * - dylib inside exec_prefix
 *   - again, handling same as dylib, but destination different
 * - dylib in site-packages
 *   - again, handling same as dylib, but destination different
 * - might be useful to have a destination object in these files i think
 * - finally we have pythonexe, which has same handling as dylib, but destination is different
 *
 * We call all of these binary file
 *
 *
 * - binary file (with destination)
 * - plain file (with destination)
 *
 * reals is same for both, how they are symlinked to correct location is different
 */

fn main() {
    env_logger::init();
    let start_time = std::time::Instant::now();
    export_files();
    let duration = start_time.elapsed();
    info!("Time to finish: {} seconds", duration.as_secs());
}

fn export_files() {
    let args: Vec<String> = std::env::args().collect();
    let shenzi_manifest_path = args
        .get(1)
        .expect("Expected a single argument, the path the shenzi manifest");
    let manifest_contents = std::fs::read_to_string(shenzi_manifest_path).expect(&format!(
        "Failed to read shenzi manifest file {}",
        shenzi_manifest_path
    ));
    let manifest = get_manifest(&manifest_contents);
    let cwd = env::current_dir().unwrap();

    let (graph, path_components) =
        build_graph_from_manifest(&manifest, &cwd).expect("failed in building graph");
    let dist = cwd.join("dist");
    if dist.exists() {
        info!("found existing dist, removing. path={}", dist.display());
        std::fs::remove_dir_all(&dist).expect(&format!(
            "Failed to remove existing dist directory at {}",
            dist.display()
        ));
    }
    move_all_nodes(&graph, &dist);
    write_bootstrap_script(&dist, &path_components, &manifest.python.sys.version)
        .expect("failed in writing bootstrap script");
}

fn get_manifest(manifest_contents: &str) -> Box<ShenziManifest> {
    let mut manifest: ShenziManifest =
        serde_json::from_str(manifest_contents).expect("Failed to parse shenzi manifest as JSON");
    manifest.python.sys.path = manifest
        .python
        .sys
        .path
        .iter()
        .map(|p| normalize_path(p))
        .collect();
    Box::new(manifest)
}

fn move_all_nodes(graph: &FileGraph<NodeFactory>, dist: &PathBuf) {
    info!("exporting files to dist");
    let total = graph.len();
    let mut i = 0;
    // TODO: parallelize this (we need custom toposort implementation)
    for node in graph.toposort().unwrap() {
        let deps = graph.get_node_dependencies(&node);
        move_to_dist(&node, &deps, dist).unwrap();
        i += 1;
        if total / 10 != 0 && i % (total / 10) == 0 {
            info!("exported {}/{} files", i, total);
        }
    }
}
