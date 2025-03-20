use std::collections::BTreeMap;

use data_encoding::HEXLOWER;
use flate2::read::GzDecoder;
use reqwest::blocking::Response;
use serde::Deserialize;

use crate::{cache::http_client, dirs, validate::Validate};

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

    pub fn download(&self) -> anyhow::Result<()> {
        let mut raw_data = self.current_target()?.fetch()?;

        let unzip = GzDecoder::new(&mut raw_data);
        let mut tar = tar::Archive::new(unzip);
        let path = dirs::cellar_dir()?;
        tar.unpack(path)?;

        raw_data.validate()?;

        Ok(())
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
