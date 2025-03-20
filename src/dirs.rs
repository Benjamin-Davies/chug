use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const PROGRAM_NAME: &str = "chug";

fn home_dir() -> &'static Path {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();
    CACHE.get_or_init(|| env::var("HOME").expect("$HOME not set").into())
}

pub fn cache_dir() -> &'static Path {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut path = if let Ok(xdg_dir) = env::var("XDG_CACHE_DIR") {
            PathBuf::from(xdg_dir)
        } else {
            home_dir().join(".cache")
        };
        path.push(PROGRAM_NAME);

        fs::create_dir_all(&path).expect("Could not create cache dir");

        path
    })
}
