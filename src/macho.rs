use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

use anyhow::Context;

use crate::dirs;

const HOMEBREW_PREFIX_PLACEHOLDER: &str = "@@HOMEBREW_PREFIX@@";
const HOMEBREW_CELLAR_PLACEHOLDER: &str = "@@HOMEBREW_CELLAR@@";

pub fn patch(path: &Path) -> anyhow::Result<()> {
    let bytes = fs::read(path)?;
    let macho = goblin::mach::MachO::parse(&bytes, 0)?;

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
        return Ok(());
    }

    let mut macho = arwen::macho::MachoContainer::parse(&bytes)?;
    for (old, new) in &replacements {
        macho.change_install_name(old, new)?;
    }

    let old_permissions = path.metadata()?.permissions();
    let mut modified_permissions = false;
    if old_permissions.readonly() {
        let mut new_permissions = old_permissions.clone();
        new_permissions.set_mode(0o600);
        fs::set_permissions(path, new_permissions)?;
        modified_permissions = true;
    }

    fs::write(path, macho.data)?;

    Command::new("codesign")
        .arg("--force")
        .arg("--sign")
        .arg("-")
        .arg(path)
        .output()
        .context("Failed to codesign patched binary")?;

    if modified_permissions {
        fs::set_permissions(path, old_permissions)?;
    }

    Ok(())
}
