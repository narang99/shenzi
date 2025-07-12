use anyhow::{Context, Result, anyhow};
use log::info;
use std::io::Read;

use crate::{
    gather::build_graph_from_manifest,
    manifest::ShenziManifest,
    pkg::{bootstrap::write_bootstrap_script, move_all_nodes},
};

pub fn run(manifest: &str) -> Result<()> {
    let manifest = read_manifest_from_path_or_stdio(manifest)?;
    let manifest = ShenziManifest::from_str(&manifest)?;
    let cwd = std::env::current_dir().unwrap();
    let (graph, path_components) =
        build_graph_from_manifest(&manifest, &cwd).context("failed in building graph")?;
    let dist = cwd.join("dist");
    if dist.exists() {
        info!("found existing dist, removing. path={}", dist.display());
        std::fs::remove_dir_all(&dist).context(anyhow!(
            "Failed to remove existing dist directory at {}",
            dist.display()
        ))?;
    }
    move_all_nodes(&graph, &dist);
    write_bootstrap_script(&dist, &path_components, &manifest.python.sys.version)
        .expect("failed in writing bootstrap script");
    Ok(())
}

fn read_manifest_from_path_or_stdio(manifest: &str) -> Result<String> {
    let mut contents = String::new();
    if manifest == "-" {
        std::io::stdin().read_to_string(&mut contents)?;
    } else {
        contents = std::fs::read_to_string(manifest)?;
    }
    Ok(contents)
}

// pub fn export_files() {
//     let args: Vec<String> = std::env::args().collect();
//     let shenzi_manifest_path = args
//         .get(1)
//         .expect("Expected a single argument, the path the shenzi manifest");
//     let manifest_contents = std::fs::read_to_string(shenzi_manifest_path).expect(&format!(
//         "Failed to read shenzi manifest file {}",
//         shenzi_manifest_path
//     ));
//     let manifest = ShenziManifest::from_str(&manifest_contents).unwrap();
//     // let manifest = get_manifest(&manifest_contents);
//     let cwd = env::current_dir().unwrap();

//     let (graph, path_components) =
//         build_graph_from_manifest(&manifest, &cwd).expect("failed in building graph");
//     let dist = cwd.join("dist");
//     if dist.exists() {
//         info!("found existing dist, removing. path={}", dist.display());
//         std::fs::remove_dir_all(&dist).expect(&format!(
//             "Failed to remove existing dist directory at {}",
//             dist.display()
//         ));
//     }
//     move_all_nodes(&graph, &dist);
//     write_bootstrap_script(&dist, &path_components, &manifest.python.sys.version)
//         .expect("failed in writing bootstrap script");
// }
