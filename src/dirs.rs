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

pub fn data_dir() -> anyhow::Result<&'static Path> {
    let path = cache!(PathBuf).get_or_init(|| {
        let mut path = if let Ok(xdg_dir) = env::var("XDG_DATA_HOME") {
            PathBuf::from(xdg_dir)
        } else {
            home_dir()?.join(".local/share")
        };
        path.push(PROGRAM_NAME);

        fs::create_dir_all(&path).expect("Could not create data dir");

        Ok(path)
    })?;
    Ok(path)
}

pub fn bottles_dir() -> anyhow::Result<&'static Path> {
    let path = cache!(PathBuf).get_or_init(|| {
        let path = data_dir()?.join("bottles");

        fs::create_dir_all(&path).expect("Could not create bottles dir");

        Ok(path)
    })?;
    Ok(path)
}
