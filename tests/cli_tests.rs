use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

fn create_test_file(path: &std::path::Path, name: &str, content: &[u8]) {
    let mut file = File::create(path.join(name)).unwrap();
    file.write_all(content).unwrap();
}

#[test]
fn test_pack_unpack_roundtrip() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("input");
    let output = temp.path().join("output");
    let archive = temp.path().join("archive.squish");

    fs::create_dir(&input).unwrap();
    create_test_file(&input, "file1.txt", b"hello");
    create_test_file(&input, "file2.bin", &[0, 1, 2, 3, 4]);

    Command::cargo_bin("squishrs")
        .unwrap()
        .args([
            "pack",
            input.to_str().unwrap(),
            "--output",
            archive.to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("squishrs")
        .unwrap()
        .args([
            "unpack",
            archive.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read(input.join("file1.txt")).unwrap(),
        fs::read(output.join("file1.txt")).unwrap()
    );
    assert_eq!(
        fs::read(input.join("file2.bin")).unwrap(),
        fs::read(output.join("file2.bin")).unwrap()
    );
}

#[test]
fn test_pack_empty_directory() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("empty");
    let archive = temp.path().join("empty.squish");

    fs::create_dir(&input).unwrap();

    Command::cargo_bin("squishrs")
        .unwrap()
        .args([
            "pack",
            input.to_str().unwrap(),
            "--output",
            archive.to_str().unwrap(),
        ])
        .assert()
        .success();

    // List should show 0 files
    Command::cargo_bin("squishrs")
        .unwrap()
        .args(["list", archive.to_str().unwrap(), "--simple"])
        .assert()
        .stdout(predicate::str::contains("number_of_files: 0"));
}

#[test]
fn test_list_invalid_archive() {
    let temp = tempdir().unwrap();
    let bad_file = temp.path().join("corrupt.squish");

    // Write random data to file
    fs::write(&bad_file, b"not an archive").unwrap();

    Command::cargo_bin("squishrs")
        .unwrap()
        .args(["list", bad_file.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Unknown frame descriptor")
                .or(predicate::str::contains("Error")),
        );
}

#[test]
fn test_unpack_nonexistent_archive() {
    let temp = tempdir().unwrap();
    let output = temp.path().join("output");

    Command::cargo_bin("squishrs")
        .unwrap()
        .args([
            "unpack",
            "nonexistent.squish",
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("No such file or directory")
                .or(predicate::str::contains("Error")),
        );
}

#[test]
fn test_pack_nested_directories() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("nested");
    let archive = temp.path().join("nested.squish");
    let output = temp.path().join("output");

    fs::create_dir_all(input.join("subdir")).unwrap();
    create_test_file(&input, "file_root.txt", b"root");
    create_test_file(&input.join("subdir"), "file_sub.txt", b"subdir content");

    Command::cargo_bin("squishrs")
        .unwrap()
        .args([
            "pack",
            input.to_str().unwrap(),
            "--output",
            archive.to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("squishrs")
        .unwrap()
        .args([
            "unpack",
            archive.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read(input.join("subdir").join("file_sub.txt")).unwrap(),
        fs::read(output.join("subdir").join("file_sub.txt")).unwrap()
    );
}
