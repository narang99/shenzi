use anyhow::{Context, Result, anyhow, bail};
use log::info;
use std::{fs, io::Read};

use crate::{
    gather::build_graph_from_manifest,
    manifest::{Bin, ShenziManifest},
    paths::marker_file_path,
    pkg::{bootstrap::write_bootstrap_script, move_all_nodes, write_warnings},
    warnings::validate_warnings,
    workspace::InitializedShenziWorkspace,
};

pub fn run(manifest: &str, skip_warning_checks: bool) -> Result<()> {
    let dist = std::env::current_dir().unwrap().join("dist");
    if dist.exists() {
        bail!(
            "the folder `dist` already exists in the current directory, delete it and call `shenzi` again"
        );
    }
    fs::create_dir_all(&dist).context(anyhow!("failed in creating dist at {}", dist.display()))?;
    let shenzi_marker = marker_file_path(&dist);
    fs::File::create(&shenzi_marker)
        .context(anyhow!("failed to create SHENZI marker file in dist"))?;

    let manifest = get_manifest(&manifest)?;
    let (graph, path_components, mut warnings) =
        build_graph_from_manifest(&manifest, &manifest.python.cwd)
            .context("failed in building graph")?;
    let main_destination = move_all_nodes(&graph, &dist, &manifest.python.main)?;
    write_bootstrap_script(
        &dist,
        &path_components,
        &manifest.python.sys.version,
        &main_destination,
    )
    .context("failed in writing bootstrap script")?;

    if !skip_warning_checks {
        println!(
            "shenzi will now validate if any of your warnings are errors, this can take time (it will scan your whole file system). You can skip this by passing --skip-warning-checks, number of warnings: {}",
            warnings.len(),
        );
        warnings = validate_warnings(warnings).context("Warning validation found some errors")?;
        println!("warning validation done: all warnings can be ignored");
    }
    let (warnings_file, wrote_warnings) =
        write_warnings(warnings, &dist).context("failed in writing warnings")?;
    if wrote_warnings {
        println!("warnings written to {}", warnings_file.display());
        println!(
            "you would need to test the application to see if any of the warnings have affected the final distribution"
        );
    }

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

fn get_manifest(manifest: &str) -> Result<ShenziManifest> {
    let shenzi_workspace = InitializedShenziWorkspace::search()?;
    let manifest = read_manifest_from_path_or_stdio(manifest)
        .context(anyhow!("failed in reading manifest file at {}", manifest))?;
    let mut manifest = ShenziManifest::from_str(&manifest)?;
    if let Some(workspace) = shenzi_workspace {
        merge_manifest_and_shenzi_workspace_manifest(&mut manifest, &workspace)?;
    }
    Ok(manifest)
}

fn merge_manifest_and_shenzi_workspace_manifest(
    manifest: &mut ShenziManifest,
    workspace: &InitializedShenziWorkspace,
) -> Result<()> {
    manifest.python.main = workspace.main_path();
    if !manifest.python.main.exists() {
        bail!(
            "main file in shenzi workspace file does not exist, path={}",
            manifest.python.main.display()
        );
    }
    let deps = workspace.get_required_dependencies()?;
    manifest.python.allowed_packages = Some(deps);
    let extra_binaries: Vec<Bin> = workspace
        .workspace
        .binaries
        .iter()
        .map(|b| Bin {
            path: b.to_string(),
        })
        .collect();
    manifest.bins.extend(extra_binaries);
    info!(
        "merging manifest file with workspace, main-file={} required-dependencies={:?} binaries={:?}",
        manifest.python.main.display(),
        manifest.python.allowed_packages,
        manifest.bins,
    );
    Ok(())
}
