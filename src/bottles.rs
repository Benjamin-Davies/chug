use std::{collections::BTreeMap, fs, path::Path};

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
    pub fn download_bottle(&self) -> anyhow::Result<()> {
        let bottles_path = dirs::bottles_dir()?;
        let bottle_path = bottles_path.join(&self.name).join(&self.versions.stable);
        if bottle_path.exists() {
            println!("Bottle {:?} already downloaded", self.name);
            return Ok(());
        }

        println!("Dowloading {} {}...", self.name, self.versions.stable);
        let result = self.bottle.stable.download_inner(
            &bottles_path,
            &bottle_path,
            &self.name,
            &self.versions.stable,
        );

        if result.is_err() && bottle_path.exists() {
            let _ = fs::remove_dir_all(&bottle_path);
            if let Some(parent) = bottle_path.parent() {
                let _ = fs::remove_dir(parent);
            }
        }

        result
    }
}

impl Bottle {
    /// Expects the bottle to not already be downloaded and will not clean up if
    /// the download fails.
    fn download_inner(
        &self,
        bottles_path: &Path,
        bottle_path: &Path,
        name: &str,
        version: &str,
    ) -> anyhow::Result<()> {
        let mut raw_data = self.current_target()?.fetch()?;

        let unzip = GzDecoder::new(&mut raw_data);
        let mut tar = tar::Archive::new(unzip);
        tar.unpack(bottles_path)?;

        // Sometimes the bottle directory has "_1" appended to the version
        // If it does, we want to rename it to the correct version
        if !bottle_path.exists() {
            let parent = bottles_path.join(name);
            let mut found = false;
            for child in fs::read_dir(parent)? {
                let child = child?;
                let file_name = child.file_name();
                let file_name = file_name.to_str().context("Invalid file name")?;
                if file_name.starts_with(&version) && file_name[version.len()..].starts_with('_') {
                    fs::rename(child.path(), bottle_path)?;
                    found = true;
                    break;
                }
            }

            anyhow::ensure!(
                found,
                "Failed to find where bottle {name:?} was extracted to",
            );
        }

        raw_data.validate()?;

        Ok(())
    }

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
