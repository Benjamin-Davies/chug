use std::{collections::BTreeMap, fs, os::unix, path::PathBuf};

use anyhow::Context;
use data_encoding::HEXLOWER;
use flate2::read::GzDecoder;
use reqwest::blocking::Response;
use serde::Deserialize;

use crate::{
    cache::http_client,
    db::models::{DownloadedBottle, LinkedFile},
    dirs,
    extract::{extract, validate::Validate},
    formulae::Formula,
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
        let file_metadata = self.bottle.stable.current_target()?;

        let mut raw_data = file_metadata
            .fetch()
            .context("Failed to fetch bottle archive")?;
        let unzip = GzDecoder::new(&mut raw_data);
        let path = extract(unzip, self)?;

        raw_data
            .validate()
            .context("Failed to validate bottle download")?;

        let bottle = DownloadedBottle::create(&self.name, &self.versions.stable, &path)?;

        Ok(bottle)
    }

    fn bottle_path(&self) -> anyhow::Result<Option<PathBuf>> {
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
            if file_name.starts_with(version) && file_name[version.len()..].starts_with('_') {
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
    pub fn link(&self) -> anyhow::Result<()> {
        println!("Linking {} {}...", self.name(), self.version());

        let opt_dir = dirs::opt_dir()?.join(self.name());
        if opt_dir.exists() {
            fs::remove_file(&opt_dir)?;
        }
        unix::fs::symlink(self.path(), &opt_dir)?;

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

        // Replicate some important post-install scripts
        if self.name() == "ca-certificates" {
            // See also: https://github.com/Homebrew/homebrew-core/blob/39c62e85e1aafaaf438741b7d0d37b35bdbb9b32/Formula/c/ca-certificates.rb#L123
            let source = self.path().join("share/ca-certificates/cacert.pem");
            let destination_dir = dirs::etc_dir()?.join("ca-certificates");
            let destination = destination_dir.join("cert.pem");
            fs::create_dir_all(&destination_dir)?;
            fs::copy(&source, &destination)?;
        }

        Ok(())
    }

    pub fn unlink(&self) -> anyhow::Result<()> {
        println!("Unlinking {} {}...", self.name(), self.version());

        let opt_dir = dirs::opt_dir()?.join(self.name());
        if fs::read_link(&opt_dir)? == self.path() {
            fs::remove_file(&opt_dir)?;
        }

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
