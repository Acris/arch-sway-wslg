// Shared by the broker and agent build scripts through `include!`.
//
// Both release binaries embed one digest of every workspace input that shapes
// them, and `--probe` prints it. A checked-in payload built from older source
// then shows a different digest than a build of the current tree. Carriage
// returns are dropped so that a Windows checkout with CRLF line endings hashes
// like the Linux one.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

fn collect_sources(directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot list {}: {error}", directory.display()))
        .map(|entry| entry.expect("readable directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if path.is_dir() {
            if name != "target" && !name.starts_with('.') {
                collect_sources(&path, files);
            }
        } else if name.ends_with(".rs") || name.ends_with(".toml") || name == "Cargo.lock" {
            files.push(path);
        }
    }
}

fn emit_source_digest() {
    let manifest =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .expect("crate lives in <workspace>/crates/<name>")
        .to_path_buf();

    let mut files = Vec::new();
    collect_sources(&root, &mut files);
    let mut hasher = Sha256::new();
    for path in &files {
        let relative = path
            .strip_prefix(&root)
            .expect("source inside the workspace")
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let content: Vec<u8> = fs::read(path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
            .into_iter()
            .filter(|byte| *byte != b'\r')
            .collect();
        hasher.update((relative.len() as u64).to_le_bytes());
        hasher.update(relative.as_bytes());
        hasher.update((content.len() as u64).to_le_bytes());
        hasher.update(&content);
    }
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();

    // Directories are scanned recursively, which also catches added files.
    for input in [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "crates",
        "build-support",
    ] {
        println!("cargo::rerun-if-changed={}", root.join(input).display());
    }
    println!("cargo::rustc-env=ARCH_SWAY_WSLG_SOURCE_DIGEST={digest}");
}
