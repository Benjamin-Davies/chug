use std::{env, fs, path::PathBuf, process::Command};

#[test]
fn test_install_gh() {
    let output = output_dir();

    let program = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/chug");
    let status = Command::new(program)
        .args(["install", "gh"])
        .status()
        .unwrap();
    assert!(status.success());

    let program = output.bin_dir().join("gh");
    let status = Command::new(program).arg("--version").status().unwrap();
    assert!(status.success());
}

#[derive(Debug)]
struct OutputDir {
    path: PathBuf,
}

fn output_dir() -> OutputDir {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_output");
    let dir = OutputDir { path };
    unsafe {
        env::set_var("XDG_BIN_HOME", dir.bin_dir());
        env::set_var("XDG_CACHE_HOME", dir.cache_dir());
        env::set_var("XDG_DATA_HOME", dir.data_dir());
    }
    dir
}

impl OutputDir {
    fn bin_dir(&self) -> PathBuf {
        self.path.join("bin")
    }

    fn cache_dir(&self) -> PathBuf {
        self.path.join("cache")
    }

    fn data_dir(&self) -> PathBuf {
        self.path.join("data")
    }
}

impl Drop for OutputDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
