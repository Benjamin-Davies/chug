use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::Context;

const PROGRAM_NAME: &str = "chug";

fn home_dir() -> anyhow::Result<&'static Path> {
    let path = cache!(PathBuf).get_or_init(|| {
        let path = env::var("HOME").context("$HOME not set")?;
        Ok(path.into())
    })?;
    Ok(path)
}

pub fn cache_dir() -> anyhow::Result<&'static Path> {
    let path = cache!(PathBuf).get_or_init(|| {
        let mut path = if let Ok(xdg_dir) = env::var("XDG_CACHE_DIR") {
            PathBuf::from(xdg_dir)
        } else {
            home_dir()?.join(".cache")
        };
        path.push(PROGRAM_NAME);

        fs::create_dir_all(&path).expect("Could not create cache dir");

        Ok(path)
    })?;
    Ok(path)
}
