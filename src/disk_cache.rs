use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};

use serde::de::DeserializeOwned;

const DISK_CACHE_TIMEOUT: Duration = Duration::from_secs(24 * 3_600);

pub fn load_json<T: DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let metadata = path.metadata()?;
    anyhow::ensure!(metadata.is_file());
    anyhow::ensure!(
        SystemTime::now().duration_since(metadata.modified()?)? < DISK_CACHE_TIMEOUT,
        "Disk cache has expired",
    );

    let json = fs::read_to_string(path)?;
    let formulae = serde_json::from_str(&json)?;
    Ok(formulae)
}

pub fn store(path: &Path, contents: &str) -> anyhow::Result<()> {
    fs::write(path, contents)?;
    Ok(())
}
