// download and provide external dependencies

use std::{
    fs::File,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
use flate2::read::GzDecoder;
use log::info;
use tar::Archive;

use crate::paths::{cache_loc, make_executable};

pub fn patchelf_path() -> Result<PathBuf> {
    cache_loc()
        .map(|p| p.join("patchelf"))
        .context("failed in finding patchelf path")
}

fn patchelf_url() -> Result<String> {
    let arch = std::env::consts::ARCH;
    if arch == "x86_64" {
        Ok(String::from(
            "https://github.com/NixOS/patchelf/releases/download/0.18.0/patchelf-0.18.0-x86_64.tar.gz",
        ))
    } else if arch == "aarch64" {
        Ok(String::from(
            "https://github.com/NixOS/patchelf/releases/download/0.18.0/patchelf-0.18.0-aarch64.tar.gz",
        ))
    } else {
        Err(anyhow!(
            "unsupported host architecture to download patchelf = {}",
            arch
        ))
    }
}

pub fn download_patchelf() -> Result<()> {
    let loc = patchelf_path()?;
    if loc.exists() {
        Ok(())
    } else {
        info!("patchelf does not exist, downloading at: {}", loc.display());
        let tmp_dir =
            tempfile::tempdir().context("failed to create temp dir for patchelf download")?;
        let tar_loc = tmp_dir.path().join("patchelf.tar.gz");
        download_at(&patchelf_url()?, &tar_loc)?;
        from_tarball(&tar_loc, &PathBuf::from("./bin/patchelf"), &loc)?;
        make_executable(&loc)?;
        Ok(())
    }
}

fn download_at(url: &str, path: &Path) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let response = client.get(url).send()?;
    let mut content = response;
    let mut dest = std::io::BufWriter::new(std::fs::File::create(path)?);
    std::io::copy(&mut content, &mut dest)?;

    Ok(())
}

fn from_tarball(tarball: &Path, file_to_extract: &Path, dest: &Path) -> Result<()> {
    let tar_gz = File::open(tarball)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    for entry in archive.entries().context("failed to read tar entries")? {
        let mut entry = entry.context("failed to read tar entry")?;
        let path = entry.path()?;
        println!("entryyyy {}", path.display());
        if path == file_to_extract {
            let mut out = File::create(dest).context("failed to create patchelf in cache")?;
            std::io::copy(&mut entry, &mut out).context("failed to extract patchelf binary")?;
            return Ok(());
        }
    }
    bail!(
        "failed to find file {} in archive {}",
        file_to_extract.display(),
        tarball.display()
    );
}
