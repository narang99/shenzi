// generating the bootstrap script

use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use log::info;
use pathdiff::diff_paths;

use crate::{
    gather::PythonPathComponent,
    manifest::Version,
    pkg::paths::{lib_dynload_relative_path, site_pkgs_relative_path, stdlib_relative_path},
};

const SCRIPT_DIR_SH: &str = r#"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
echo "bootstrap directory: $SCRIPT_DIR"
"#;

const RUN_PY_INTERPRETER_SH: &str = r#"
exec "$SCRIPT_DIR/python/bin/python" "$@"
"#;

const RUN_MAIN_SH: &str = r#"
cd $SCRIPT_DIR/{{MAIN_SCRIPT_DIR}}
exec "$SCRIPT_DIR/python/bin/python" {{MAIN_SCRIPT_NAME}} "$@"
"#;

const MAC_ENV_EXPORT_SH: &str = r#"
ORIGINAL_DYLD_LIBRARY_PATH="${DYLD_LIBRARY_PATH:-}"
export DYLD_LIBRARY_PATH="$SCRIPT_DIR/lib/l:$ORIGINAL_DYLD_LIBRARY_PATH"
echo "DYLD_LIBRARY_PATH: $DYLD_LIBRARY_PATH"

SITE_PKG_REL_PATHS={{SITE_PKGS_REPLACEMENT}}

export PYTHONPATH=""
for path in "${SITE_PKG_REL_PATHS[@]}"; do 
    export PYTHONPATH="$PYTHONPATH:$SCRIPT_DIR/$path"
done

echo "PYTHONPATH=$PYTHONPATH"
"#;

const PY_SH_TEMPLATE: &str = r#"
#!/bin/bash
set -euo pipefail

{{SCRIPT_DIR_SH}}

{{ENV_EXPORT_SH}}

{{RUN_PY_INTERPRETER_SH}}
"#;

const BOOTSTRAP_SCRIPT_TEMPLATE: &str = r#"
#!/bin/bash
set -euo pipefail

{{SCRIPT_DIR_SH}}

{{ENV_EXPORT_SH}}

{{RUN_MAIN_SH}}
"#;

const LINUX_ENV_EXPORT_SH: &str = r#"
ORIGINAL_LD_LIBRARY_PATH="${LD_LIBRARY_PATH:-}"
export LD_LIBRARY_PATH="$SCRIPT_DIR/lib/l:$ORIGINAL_LD_LIBRARY_PATH"
echo "LD_LIBRARY_PATH: $LD_LIBRARY_PATH"

SITE_PKG_REL_PATHS={{SITE_PKGS_REPLACEMENT}}

export PYTHONPATH=""
for path in "${SITE_PKG_REL_PATHS[@]}"; do 
    export PYTHONPATH="$PYTHONPATH:$SCRIPT_DIR/$path"
done

echo "PYTHONPATH=$PYTHONPATH"
"#;

const PY_SCRIPT_IN_BIN_SH: &str = r#"
#!/bin/bash
set -euo pipefail

{{SCRIPT_DIR_SH}}

exec "$SCRIPT_DIR/../../py.sh" $SCRIPT_DIR/{{REL_PY_SCRIPT_PATH}}
"#;

pub fn write_bootstrap_script(
    dist: &PathBuf,
    comps: &Vec<PythonPathComponent>,
    version: &Version,
    main_destination: &PathBuf,
) -> Result<()> {
    let script_path = dist.join("bootstrap.sh");
    info!("writing bootstrap script at {}", script_path.display());
    info!("python path components: {:?}", comps);
    let comps_array = python_path_from_components(comps, version).with_context(|| {
        format!(
            "failed in generated PYTHONPATH, components={:?} version={:?}",
            comps, version
        )
    })?;
    let (main_parent_dir, main_filename) = get_main_script_paths(main_destination, dist)?;
    let os = std::env::consts::OS;
    let gen_bootstrap = |template: &str| {
        template
            .replace("{{SITE_PKGS_REPLACEMENT}}", &comps_array)
            .replace("{{MAIN_SCRIPT_DIR}}", &main_parent_dir)
            .replace("{{MAIN_SCRIPT_NAME}}", &main_filename)
    };
    let script = match os {
        "macos" => gen_bootstrap(&get_basic_bootstrap_template(BootstrapOS::Mac)),
        "linux" => gen_bootstrap(&get_basic_bootstrap_template(BootstrapOS::Linux)),

        _ => {
            bail!("unsupported OS: {}", os);
        }
    };

    fs::write(script_path, script)?;
    info!("bootstrap script written");
    Ok(())
}

enum BootstrapOS {
    Linux,
    Mac,
}

fn get_basic_bootstrap_template(os: BootstrapOS) -> String {
    let base_template = BOOTSTRAP_SCRIPT_TEMPLATE
        .replace("{{SCRIPT_DIR_SH}}", SCRIPT_DIR_SH)
        .replace("{{RUN_MAIN_SH}}", RUN_MAIN_SH);
    match os {
        BootstrapOS::Mac => base_template.replace("{{ENV_EXPORT_SH}}", MAC_ENV_EXPORT_SH),
        BootstrapOS::Linux => base_template.replace("{{ENV_EXPORT_SH}}", LINUX_ENV_EXPORT_SH),
    }
}

fn get_basic_py_sh_template(os: BootstrapOS) -> String {
    let base_template = PY_SH_TEMPLATE
        .replace("{{SCRIPT_DIR_SH}}", SCRIPT_DIR_SH)
        .replace("{{RUN_PY_INTERPRETER_SH}}", RUN_PY_INTERPRETER_SH);
    match os {
        BootstrapOS::Mac => base_template.replace("{{ENV_EXPORT_SH}}", MAC_ENV_EXPORT_SH),
        BootstrapOS::Linux => base_template.replace("{{ENV_EXPORT_SH}}", LINUX_ENV_EXPORT_SH),
    }
}

fn get_py_script_in_bin_template() -> String {
    PY_SCRIPT_IN_BIN_SH.replace("{{SCRIPT_DIR_SH}}", SCRIPT_DIR_SH)
}

fn get_main_script_paths(main_destination: &PathBuf, dist: &PathBuf) -> Result<(String, String)> {
    let main_path = diff_paths(main_destination, dist).ok_or_else(|| {
        anyhow!(
            "failed in finding relative path of main script inside dist main={} dist={}",
            main_destination.display(),
            dist.display()
        )
    })?;

    let main_script_dir = main_path
        .parent()
        .ok_or(anyhow!(
            "could not find parent of main script={}",
            main_path.display()
        ))?
        .to_str()
        .ok_or(anyhow!(
            "failed in converting directory name of main script to string, main={}",
            main_path.display()
        ))?
        .to_string();
    let file_name = main_path
        .file_name()
        .ok_or(anyhow!(
            "could not find file_name of main script={}",
            main_path.display()
        ))?
        .to_str()
        .ok_or(anyhow!(
            "failed in converting filename of main script to string, main={}",
            main_path.display()
        ))?
        .to_string();

    Ok((main_script_dir, file_name))
}

fn python_path_from_components(
    comps: &Vec<PythonPathComponent>,
    version: &Version,
) -> Result<String> {
    let mut res = Vec::new();
    let stdlib_rel_path = path_buf_to_str(&stdlib_relative_path(version))?;
    let lib_dynload_rel_path = path_buf_to_str(&lib_dynload_relative_path(version))?;
    for comp in comps {
        match comp {
            PythonPathComponent::RelativeToLibDynLoad { rel_path } => {
                let rel_path = path_buf_to_str(&rel_path)?;
                res.push(format!("{}/{}", lib_dynload_rel_path, rel_path));
            }
            PythonPathComponent::RelativeToStdlib { rel_path } => {
                let rel_path = path_buf_to_str(&rel_path)?;
                res.push(format!("{}/{}", stdlib_rel_path, rel_path));
            }
            PythonPathComponent::TopLevel { alias } => {
                let site_pkgs_path = path_buf_to_str(&site_pkgs_relative_path(alias))?;
                res.push(site_pkgs_path);
            }
            PythonPathComponent::RelativeToSitePkg {
                top_level_alias,
                rel_path,
            } => {
                let site_pkgs_path = path_buf_to_str(&site_pkgs_relative_path(&top_level_alias))?;
                let rel_path = path_buf_to_str(&rel_path)?;
                res.push(format!("{}/{}", site_pkgs_path, rel_path));
            }
        }
    }
    let bash_array_contents = res
        .iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<String>>()
        .join(" ");

    let bash_array = format!("({})", bash_array_contents);
    Ok(bash_array)
}

fn path_buf_to_str(b: &PathBuf) -> Result<String> {
    let p = b.to_str().ok_or_else(|| {
        anyhow!(
            "failed in converting relative path to string {}",
            b.display()
        )
    })?;
    Ok(p.to_string())
}
