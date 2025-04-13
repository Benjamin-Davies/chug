use std::{
    fs,
    io::{self, Read},
    os::unix::{self, ffi::OsStrExt, fs::PermissionsExt},
    path::{Component, Path, PathBuf},
};

use anyhow::Context;
use memchr::memmem;

use crate::{dirs, formulae::Formula};

#[cfg(target_os = "macos")]
mod macho;
mod magic;

pub mod validate;

const HOMEBREW_PREFIX_PLACEHOLDER: &str = "@@HOMEBREW_PREFIX@@";
const HOMEBREW_CELLAR_PLACEHOLDER: &str = "@@HOMEBREW_CELLAR@@";
const PLACEHOLDER_PREFIX: &str = "@@HOMEBREW_";

pub fn extract(archive: impl io::Read, formula: &Formula) -> anyhow::Result<PathBuf> {
    let bottles_dir = dirs::bottles_dir()?;

    let mut tar = tar::Archive::new(archive);

    let mut bottle_path: Option<PathBuf> = None;
    // Defer directory creation
    // See also: https://github.com/alexcrichton/tar-rs/blob/5af52e0651474905f682d68c2ece702797746f80/src/archive.rs#L230
    let mut directories = Vec::new();
    for file in tar.entries()? {
        let file = file?;

        if let Some(prefix) = &bottle_path {
            anyhow::ensure!(
                file.path()?.starts_with(prefix),
                "Attempting to extract file outside of bottle path",
            );
        } else {
            let path = file.path()?;
            let mut components = path.components();
            anyhow::ensure!(
                components
                    .next()
                    .and_then(|c| c.as_os_str().to_str())
                    .context("Invalid path inside bottle")?
                    == formula.name,
                "Bottle path does not match formula name: {path:?}",
            );
            anyhow::ensure!(
                components
                    .next()
                    .and_then(|c| c.as_os_str().to_str())
                    .context("Invalid path inside bottle")?
                    .starts_with(&formula.versions.stable),
                "Bottle path does not match formula version: {path:?}",
            );

            bottle_path = Some(path.into_owned());
        }

        if file.header().entry_type().is_dir() {
            directories.push(file);
        } else {
            extract_file(file, bottles_dir)?;
        }
    }

    directories.sort_by(|a, b| b.path_bytes().cmp(&a.path_bytes()));
    for dir in directories {
        extract_file(dir, bottles_dir)?;
    }

    let path = bottles_dir.join(bottle_path.context("Empty bottle")?);

    Ok(path)
}

fn extract_file(mut file: tar::Entry<impl io::Read>, bottles_dir: &Path) -> anyhow::Result<()> {
    let path = sanitise_path(bottles_dir, &file.path()?).context("Malformed path inside bottle")?;

    let parent = path.parent().context("Path has no parent")?;
    fs::create_dir_all(parent)?;

    let perm = fs::Permissions::from_mode(file.header().mode()?);
    let kind = file.header().entry_type();

    match kind {
        tar::EntryType::Regular => {
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;

            patch_and_write(&path, &contents)?;
            fs::set_permissions(&path, perm)?;
        }
        tar::EntryType::Directory => {
            fs::create_dir_all(&path)?;
            fs::set_permissions(&path, perm)?;
        }
        tar::EntryType::Symlink => {
            let target = file
                .header()
                .link_name()?
                .context("Symlink has no target")?;
            unix::fs::symlink(&target, &path)?;

            // On Unix it's not possible to manipulate the permissions of a symlink
            // See also: https://github.com/rust-lang/rust/issues/75942#issuecomment-2769976820
        }
        _ => anyhow::bail!("Encountered unsupported tar entry type: {kind:?}"),
    }

    Ok(())
}

fn sanitise_path(base_dir: &Path, path: &Path) -> Option<PathBuf> {
    let mut sanitised = base_dir.to_owned();
    for component in path.components() {
        match component {
            Component::Prefix(..) | Component::RootDir | Component::CurDir => continue,
            Component::ParentDir => return None,
            Component::Normal(part) => sanitised.push(part),
        }
    }

    if sanitised == base_dir || path.parent().is_none() {
        return None;
    }

    Some(sanitised)
}

fn patch_and_write(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    match magic::detect(contents).unwrap_or(magic::Magic::Unknown) {
        #[cfg(target_os = "macos")]
        magic::Magic::MachO => macho::patch_and_write(path, contents)?,
        #[cfg(target_os = "linux")]
        magic::Magic::Elf => todo!(),
        _ => patch_and_write_misc(path, contents)?,
    }

    Ok(())
}

fn patch_and_write_misc(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    let homebrew_prefix = dirs::data_dir()?.as_os_str().as_bytes();
    let homebrew_cellar = dirs::bottles_dir()?.as_os_str().as_bytes();

    let mut output = Vec::new();
    let mut last_index = 0;
    for index in memmem::find_iter(contents, PLACEHOLDER_PREFIX) {
        if contents[index..].starts_with(HOMEBREW_CELLAR_PLACEHOLDER.as_bytes()) {
            output.extend_from_slice(&contents[last_index..index]);
            output.extend_from_slice(homebrew_cellar);
            last_index = index + HOMEBREW_CELLAR_PLACEHOLDER.len();
        } else if contents[index..].starts_with(HOMEBREW_PREFIX_PLACEHOLDER.as_bytes()) {
            output.extend_from_slice(&contents[last_index..index]);
            output.extend_from_slice(homebrew_prefix);
            last_index = index + HOMEBREW_PREFIX_PLACEHOLDER.len();
        }
    }

    if output.is_empty() {
        // Output will be empty if no occurrences of the placeholder were found
        fs::write(path, contents)?;
    } else {
        output.extend_from_slice(&contents[last_index..]);
        fs::write(path, output)?;
    }

    Ok(())
}
