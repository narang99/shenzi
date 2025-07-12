use anyhow::{Context, Result, anyhow, bail};
use std::io::Read;

use crate::{
    gather::build_graph_from_manifest,
    manifest::ShenziManifest,
    pkg::{bootstrap::write_bootstrap_script, move_all_nodes},
};

pub fn run(manifest: &str) -> Result<()> {
    let manifest = read_manifest_from_path_or_stdio(manifest)
        .context(anyhow!("failed in reading manifest file at {}", manifest))?;
    let cwd = std::env::current_dir().unwrap();
    let dist = cwd.join("dist");
    if dist.exists() {
        bail!(
            "the folder `dist` already exists in the current directory, delete it and call `shenzi` again"
        );
    }
    let manifest = ShenziManifest::from_str(&manifest)?;
    let (graph, path_components) =
        build_graph_from_manifest(&manifest, &cwd).context("failed in building graph")?;
    let main_destination = move_all_nodes(&graph, &dist, &manifest.python.main)?;
    write_bootstrap_script(
        &dist,
        &path_components,
        &manifest.python.sys.version,
        &main_destination,
    )
    .context("failed in writing bootstrap script")?;
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
