use std::{fs, path::Path, process::Command};

use anyhow::Context;

use crate::dirs;

use super::{HOMEBREW_CELLAR_PLACEHOLDER, HOMEBREW_PREFIX_PLACEHOLDER};

pub fn patch_and_write(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    let macho = goblin::mach::MachO::parse(contents, 0)?;

    let homebrew_prefix = dirs::data_dir()?
        .to_str()
        .context("Data dir path is non-utf8")?;
    let homebrew_cellar = dirs::bottles_dir()?
        .to_str()
        .context("Bottles dir path is non-utf8")?;

    let mut replacements = Vec::new();
    for (index, lib) in macho.libs.iter().enumerate() {
        // HACK: arwen has a bug where it will error if we try and replace the first lib
        if index == 0 {
            continue;
        }

        if lib.starts_with(HOMEBREW_PREFIX_PLACEHOLDER) {
            replacements.push((
                lib,
                lib.replace(HOMEBREW_PREFIX_PLACEHOLDER, homebrew_prefix),
            ));
        } else if lib.starts_with(HOMEBREW_CELLAR_PLACEHOLDER) {
            replacements.push((
                lib,
                lib.replace(HOMEBREW_CELLAR_PLACEHOLDER, homebrew_cellar),
            ));
        }
    }

    if replacements.is_empty() {
        fs::write(path, contents)?;
        return Ok(());
    }

    let mut macho = arwen::macho::MachoContainer::parse(contents)?;
    for (old, new) in &replacements {
        macho.change_install_name(old, new)?;
    }

    fs::write(path, macho.data)?;

    Command::new("codesign")
        .arg("--force")
        .arg("--sign")
        .arg("-")
        .arg(path)
        .output()
        .context("Failed to codesign patched binary")?;

    Ok(())
}
