use std::{fs, path::PathBuf, process::Command};

mod output_dir;

#[test]
fn test_remove_python() {
    let output = output_dir::new();

    let program = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/chug");
    let status = Command::new(&program)
        .args(["add", "python@3.12"])
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new(&program)
        .args(["remove", "python@3.12"])
        .status()
        .unwrap();
    assert!(status.success());

    let program = output.bin_dir().join("python3.12");
    assert!(!program.try_exists().unwrap());

    let bottles_dir = output.data_dir().join("chug/bottles");
    assert_eq!(fs::read_dir(&bottles_dir).unwrap().count(), 0);
}
