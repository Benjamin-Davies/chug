use std::{
    env, fs,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

static LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
pub struct OutputDir {
    path: PathBuf,
    _guard: MutexGuard<'static, ()>,
}

pub fn new() -> OutputDir {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_output");
    let guard = LOCK.lock().unwrap();
    let dir = OutputDir {
        path,
        _guard: guard,
    };
    unsafe {
        env::set_var("XDG_BIN_HOME", dir.bin_dir());
        env::set_var("XDG_CACHE_HOME", dir.cache_dir());
        env::set_var("XDG_DATA_HOME", dir.data_dir());
    }
    dir
}

impl OutputDir {
    pub fn bin_dir(&self) -> PathBuf {
        self.path.join("bin")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.path.join("cache")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.path.join("data")
    }
}

impl Drop for OutputDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
