use std::collections::BTreeMap;

use bytes::Bytes;
use data_encoding::HEXLOWER;
use ring::digest::{SHA256, digest};
use serde::Deserialize;

use crate::cache::http_client;

#[derive(Debug, Deserialize)]
pub struct Bottles {
    pub stable: Bottle,
}

#[derive(Debug, Deserialize)]
pub struct Bottle {
    pub files: BTreeMap<String, FileData>,
}

#[derive(Debug, Deserialize)]
pub struct FileData {
    pub url: String,
    pub sha256: String,
}

impl Bottle {
    pub fn current_target(&self) -> anyhow::Result<&FileData> {
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

impl FileData {
    pub fn fetch(&self) -> anyhow::Result<Bytes> {
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
        let data = response.bytes()?;

        let checksum = digest(&SHA256, &data);
        let expected = HEXLOWER.decode(self.sha256.as_bytes())?;
        anyhow::ensure!(
            checksum.as_ref() == expected.as_slice(),
            "Checksum mismatch when fetching {}",
            self.url,
        );

        Ok(data)
    }
}
