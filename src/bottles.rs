use std::{collections::BTreeMap, fs, path::Path};

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
        let cellar_path = dirs::cellar_dir()?;
        let bottle_path = cellar_path.join(&self.name).join(&self.versions.stable);
        if bottle_path.exists() {
            println!("Bottle {:?} already downloaded", self.name);
            return Ok(());
        }

        println!("Dowloading {} {}...", self.name, self.versions.stable);
        let result = self.bottle.stable.download_inner(&cellar_path);

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
    fn download_inner(&self, cellar_path: &Path) -> anyhow::Result<()> {
        let mut raw_data = self.current_target()?.fetch()?;

        let unzip = GzDecoder::new(&mut raw_data);
        let mut tar = tar::Archive::new(unzip);
        tar.unpack(cellar_path)?;

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
