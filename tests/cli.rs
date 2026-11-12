//! End-to-end checks against the real binary. The contract these tests defend:
//! moltdisk reports, and nothing on disk changes.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_moltdisk"))
}

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("moltdisk-cli-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

/// A web project with node_modules, and a Rust project with a real target/.
fn build_tree(root: &Path) {
    fs::create_dir_all(root.join("web-shop/node_modules/left-pad")).unwrap();
    fs::write(root.join("web-shop/package.json"), "{}").unwrap();
    fs::write(
        root.join("web-shop/node_modules/left-pad/index.js"),
        vec![b'x'; 4096],
    )
    .unwrap();

    fs::create_dir_all(root.join("rusty/target/debug")).unwrap();
    fs::write(root.join("rusty/Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
    fs::write(root.join("rusty/target/debug/x"), vec![b'y'; 8192]).unwrap();
}

fn run(args: &[&str]) -> (String, String, i32) {
    let out = Command::new(bin())
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn scan_finds_targets_and_deletes_nothing() {
    let root = scratch("scan");
    build_tree(&root);

    let (stdout, _, code) = run(&[root.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("node_modules"), "stdout: {stdout}");
    assert!(stdout.contains("target"), "stdout: {stdout}");
    assert!(stdout.contains("rebuilt from source"), "stdout: {stdout}");

    // The whole point: the tree is untouched.
    assert!(root
        .join("web-shop/node_modules/left-pad/index.js")
        .exists());
    assert!(root.join("rusty/target/debug/x").exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn clean_is_refused() {
    let root = scratch("clean");
    build_tree(&root);

    let (_, stderr, code) = run(&["clean", root.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stderr.contains("never deletes"), "stderr: {stderr}");
    assert!(root.join("web-shop/node_modules").exists());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn why_explains_the_marker() {
    let root = scratch("why");
    build_tree(&root);

    let target = root.join("rusty/target");
    let (stdout, _, code) = run(&["why", target.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("reclaimable"), "stdout: {stdout}");
    assert!(stdout.contains("Cargo.toml"), "stdout: {stdout}");
    assert!(stdout.contains("cargo build"), "stdout: {stdout}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn why_rejects_a_lookalike() {
    let root = scratch("why-lookalike");
    fs::create_dir_all(root.join("target")).unwrap();
    fs::write(root.join("target/notes.txt"), "not a build").unwrap();

    let (stdout, _, code) = run(&["why", root.join("target").to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("not build output"), "stdout: {stdout}");
    assert!(stdout.contains("Cargo.toml"), "stdout: {stdout}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn why_missing_path_exits_one() {
    let root = scratch("why-missing");
    let (_, stderr, code) = run(&["why", root.join("nope").to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(stderr.contains("no such path"), "stderr: {stderr}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn json_is_machine_readable() {
    let root = scratch("json");
    build_tree(&root);

    let (stdout, _, code) = run(&["--json", root.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with('{'), "stdout: {stdout}");
    assert!(stdout.contains("\"total_size\":"));
    assert!(stdout.contains("\"kind\": \"node\""));
    // Cheap shape check without pulling in serde: braces must balance.
    let opens = stdout.matches('{').count();
    let closes = stdout.matches('}').count();
    assert_eq!(opens, closes, "stdout: {stdout}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn destructive_flags_are_gone() {
    for flag in ["-y", "--yes", "--dry-run"] {
        let (_, stderr, code) = run(&[flag]);
        assert_eq!(code, 2, "{flag} should be rejected");
        assert!(
            stderr.contains("this build") || stderr.contains("read-only"),
            "{flag} stderr: {stderr}"
        );
    }
}

#[test]
fn min_size_filters_small_targets() {
    let root = scratch("min-size");
    build_tree(&root);

    let (stdout, _, _) = run(&["--min-size", "1gb", root.to_str().unwrap()]);
    assert!(
        stdout.contains("Nothing to reclaim"),
        "expected empty result, got: {stdout}"
    );
    let _ = fs::remove_dir_all(&root);
}
