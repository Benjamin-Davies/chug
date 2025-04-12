use std::{
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::Context;

use crate::{dirs, formulae::Formula, magic};

pub fn extract_and_patch(archive: impl io::Read, formula: &Formula) -> anyhow::Result<PathBuf> {
    let bottles_dir = dirs::bottles_dir()?;

    let mut tar = tar::Archive::new(archive);
    tar.unpack(bottles_dir)
        .context("Failed to unpack bottle archive")?;

    let path = formula
        .bottle_path()?
        .context("Failed to find where bottle was extracted to")?;

    patch(&path).context("Failed to patch bottle")?;

    Ok(path)
}

fn patch(path: &Path) -> anyhow::Result<()> {
    let stat = path
        .symlink_metadata()
        .with_context(|| format!("Failed to get metadata for {}", path.display()))?;
    if stat.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            patch(&entry.path())?;
        }
    } else if stat.is_file() {
        match magic::detect(path).unwrap_or(magic::Magic::Unknown) {
            #[cfg(target_os = "macos")]
            magic::Magic::MachO => crate::macho::patch(path)?,
            #[cfg(target_os = "linux")]
            magic::Magic::Elf => todo!(),
            _ => (),
        }
    }

    Ok(())
}
