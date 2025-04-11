use std::{
    collections::BTreeMap,
    fs,
    os::unix,
    path::{Path, PathBuf},
};

use anyhow::Context;
use data_encoding::HEXLOWER;
use flate2::read::GzDecoder;
use reqwest::blocking::Response;
use serde::Deserialize;

use crate::{
    cache::http_client,
    db::models::{DownloadedBottle, LinkedFile},
    dirs,
    formulae::Formula,
    macho, magic,
    validate::Validate,
};

#[derive(Debug, Deserialize)]
pub struct Bottles {
    pub stable: Bottle,
}

#[derive(Debug, Deserialize)]
pub struct Bottle {
    pub files: BTreeMap<String, FileMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct FileMetadata {
    pub url: String,
    pub sha256: String,
}

impl Formula {
    pub fn download_bottle(&self) -> anyhow::Result<DownloadedBottle> {
        if let Some(bottle) = DownloadedBottle::get(&self.name, &self.versions.stable)? {
            return Ok(bottle);
        }

        println!("Dowloading {} {}...", self.name, self.versions.stable);
        let result = self.download_bottle_inner();

        if result.is_err() {
            if let Ok(Some(path)) = self.bottle_path() {
                let _ = fs::remove_dir_all(&path);
                if let Some(parent) = path.parent() {
                    let _ = fs::remove_dir(parent);
                }
            }
        }

        result.with_context(|| format!("Downloading {} {}", self.name, self.versions.stable))
    }

    /// Expects the bottle to not already be downloaded and will not clean up if
    /// the download fails.
    fn download_bottle_inner(&self) -> anyhow::Result<DownloadedBottle> {
        let bottles_dir = dirs::bottles_dir()?;
        let file_metadata = self.bottle.stable.current_target()?;

        let mut raw_data = file_metadata
            .fetch()
            .context("Failed to fetch bottle archive")?;
        let unzip = GzDecoder::new(&mut raw_data);
        let mut tar = tar::Archive::new(unzip);
        tar.unpack(bottles_dir)
            .context("Failed to unpack bottle archive")?;

        let path = self
            .bottle_path()?
            .context("Failed to find where bottle was extracted to")?;

        raw_data
            .validate()
            .context("Failed to validate bottle download")?;

        let bottle = DownloadedBottle::create(&self.name, &self.versions.stable, &path)?;

        bottle.patch().context("Failed to patch bottle")?;

        Ok(bottle)
    }

    pub fn bottle_path(&self) -> anyhow::Result<Option<PathBuf>> {
        let name = self.name.as_str();
        let version = self.versions.stable.as_str();

        let bottles_path = dirs::bottles_dir()?;
        let parent_path = bottles_path.join(name);
        if !parent_path.exists() {
            return Ok(None);
        }

        let path = parent_path.join(version);
        if path.exists() {
            return Ok(Some(path));
        }

        // Sometimes the bottle directory has "_1" appended to the version
        for child in fs::read_dir(parent_path)? {
            let child = child?;
            let file_name = child.file_name();
            let file_name = file_name.to_str().context("Invalid file name")?;
            if file_name.starts_with(&version) && file_name[version.len()..].starts_with('_') {
                return Ok(Some(child.path()));
            }
        }

        Ok(None)
    }
}

impl Bottle {
    pub fn current_target(&self) -> anyhow::Result<&FileMetadata> {
        let target = crate::target::Target::current_str()?;
        if let Some(file) = self.files.get(target) {
            Ok(file)
        } else if let Some(file) = self.files.get("all") {
            Ok(file)
        } else {
            anyhow::bail!("No bottle for target: {target}");
        }
    }
}

impl FileMetadata {
    pub fn fetch(&self) -> anyhow::Result<Validate<Response>> {
        let response = http_client()
            .get(&self.url)
            // https://github.com/orgs/community/discussions/35172#discussioncomment-8738476
            .bearer_auth("QQ==")
            .send()?;
        anyhow::ensure!(
            response.status().is_success(),
            "Failed to fetch bottle. Response code was: {}",
            response.status(),
        );

        let sha256 = HEXLOWER.decode(self.sha256.as_bytes())?;
        let reader = Validate::new(response, sha256);

        Ok(reader)
    }
}

impl DownloadedBottle {
    pub fn patch(&self) -> anyhow::Result<()> {
        fn traverse(path: &Path) -> anyhow::Result<()> {
            let stat = path
                .symlink_metadata()
                .with_context(|| format!("Failed to get metadata for {}", path.display()))?;
            if stat.is_dir() {
                for entry in fs::read_dir(path)? {
                    let entry = entry?;
                    traverse(&entry.path())?;
                }
            } else if stat.is_file() {
                match magic::detect(path).unwrap_or(magic::Magic::Unknown) {
                    #[cfg(target_os = "macos")]
                    magic::Magic::MachO => macho::patch(path)?,
                    #[cfg(target_os = "linux")]
                    magic::Magic::Elf => todo!(),
                    _ => (),
                }
            }

            Ok(())
        }

        println!("Patching {} {}...", self.name(), self.version());

        traverse(self.path())?;

        Ok(())
    }

    pub fn link(&self) -> anyhow::Result<()> {
        println!("Linking {} {}...", self.name(), self.version());

        let bin_dir = dirs::bin_dir()?;
        let bottle_bin_dir = PathBuf::from(self.path()).join("bin");

        if bottle_bin_dir.exists() {
            for entry in fs::read_dir(bottle_bin_dir)? {
                let entry = entry?;
                let entry_path = entry.path();
                let entry_name = entry.file_name();
                let dest = bin_dir.join(entry_name);

                if dest.exists() {
                    let Ok(existing_path) = fs::read_link(&dest) else {
                        continue;
                    };
                    if !existing_path.starts_with(dirs::bottles_dir()?) {
                        continue;
                    }
                    fs::remove_file(&dest)?;
                }

                LinkedFile::create(&dest, self)?;

                unix::fs::symlink(&entry_path, &dest)?;
            }
        }

        Ok(())
    }

    pub fn unlink(&self) -> anyhow::Result<()> {
        println!("Unlinking {} {}...", self.name(), self.version());

        let bottle_dir = self.path();
        for linked_file in self.linked_files()? {
            if let Ok(linked_path) = fs::read_link(linked_file.path()) {
                if linked_path.starts_with(bottle_dir) {
                    fs::remove_file(linked_file.path())?;
                }
            }

            linked_file.delete()?;
        }

        Ok(())
    }

    pub fn remove(&self) -> anyhow::Result<()> {
        println!("Deleting {} {}...", self.name(), self.version());

        let _ = fs::remove_dir_all(self.path());
        if let Some(parent) = self.path().parent() {
            let _ = fs::remove_dir(parent);
        }

        self.delete()?;

        Ok(())
    }
}
