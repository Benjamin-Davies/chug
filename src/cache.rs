use std::{
    fs,
    path::Path,
    sync::Mutex,
    time::{Duration, SystemTime},
};

use serde::de::DeserializeOwned;

use crate::dirs::cache_dir;

const DISK_CACHE_TIMEOUT: Duration = Duration::from_secs(24 * 3_600);

pub struct Cache<T: 'static> {
    contents: Mutex<Option<&'static T>>,
}

pub struct DiskCache<'a, T: 'static> {
    filename: &'a str,
    inner: &'a Cache<T>,
}

impl<T: 'static> Cache<T> {
    pub const fn new() -> Self {
        Cache {
            contents: Mutex::new(None),
        }
    }

    pub fn get_or_init(&self, f: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<&'static T> {
        let mut lock = self.contents.lock().unwrap();
        if let Some(contents) = lock.as_ref() {
            Ok(contents)
        } else {
            let value = f()?;

            let contents = Box::leak(Box::new(value));
            *lock = Some(contents);
            Ok(contents)
        }
    }

    pub fn with_file<'a>(&'a self, filename: &'a str) -> DiskCache<'a, T> {
        DiskCache {
            filename,
            inner: self,
        }
    }
}

impl<T: 'static> DiskCache<'_, T>
where
    T: DeserializeOwned,
{
    pub fn get_or_init_json(
        &self,
        f: impl FnOnce() -> anyhow::Result<String>,
    ) -> anyhow::Result<&'static T> {
        self.inner.get_or_init(|| {
            let disk_cache_path = cache_dir()?.join(self.filename);
            if let Ok(data) = load_json(&disk_cache_path) {
                return Ok(data);
            }

            let json = f()?;
            let value = serde_json::from_str(&json)?;

            store(&disk_cache_path, &json)?;

            Ok(value)
        })
    }
}

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

macro_rules! cache {
    ($ty:ty) => {{
        use crate::cache::Cache;
        static CACHE: Cache<$ty> = Cache::new();
        &CACHE
    }};
}
