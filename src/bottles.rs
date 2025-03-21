use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::Context;
use data_encoding::HEXLOWER;
use flate2::read::GzDecoder;
use reqwest::blocking::Response;
use serde::Deserialize;

use crate::{cache::http_client, dirs, formulae::Formula, validate::Validate};

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
    pub fn download_bottle(&self) -> anyhow::Result<PathBuf> {
        if let Some(path) = self.bottle_path()? {
            println!("Bottle {:?} already downloaded", self.name);
            return Ok(path);
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
    fn download_bottle_inner(&self) -> anyhow::Result<PathBuf> {
        let bottles_dir = dirs::bottles_dir()?;
        let file_metadata = self.bottle.stable.current_target()?;

        let mut raw_data = file_metadata.fetch()?;
        let unzip = GzDecoder::new(&mut raw_data);
        let mut tar = tar::Archive::new(unzip);
        tar.unpack(bottles_dir)?;

        let path = self
            .bottle_path()?
            .context("Failed to find where bottle was extracted to")?;

        raw_data.validate()?;

        Ok(path)
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
