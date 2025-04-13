use std::{path::PathBuf, process::Command};

mod output_dir;

#[test]
fn test_add_gh() {
    let output = output_dir::new();

    let program = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/chug");
    let status = Command::new(program).args(["add", "gh"]).status().unwrap();
    assert!(status.success());

    let program = output.bin_dir().join("gh");
    let status = Command::new(program).arg("--version").status().unwrap();
    assert!(status.success());
}

#[test]
fn test_add_python() {
    let output = output_dir::new();

    let program = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/chug");
    let status = Command::new(program)
        .args(["add", "python@3.12"])
        .status()
        .unwrap();
    assert!(status.success());

    let program = output.bin_dir().join("python3.12");
    let status = Command::new(program).arg("--version").status().unwrap();
    assert!(status.success());
}

#[test]
fn test_add_certs() {
    let output = output_dir::new();

    let program = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/chug");
    let status = Command::new(program)
        .args(["add", "httpie"])
        .status()
        .unwrap();
    assert!(status.success());

    let program = output.bin_dir().join("https");
    let status = Command::new(program)
        .arg("get")
        .arg("api.github.com")
        .status()
        .unwrap();
    assert!(status.success());
}
