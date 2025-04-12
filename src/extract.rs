use std::{
    fs,
    io::{self, Read},
    os::unix::{self, fs::PermissionsExt},
    path::{Component, Path, PathBuf},
};

use anyhow::Context;

use crate::{dirs, formulae::Formula, magic};

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
        magic::Magic::MachO => crate::macho::patch_and_write(path, contents)?,
        #[cfg(target_os = "linux")]
        magic::Magic::Elf => todo!(),
        _ => fs::write(path, contents)?,
    }

    Ok(())
}
