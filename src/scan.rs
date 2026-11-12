//! Filesystem walk and measurement.
//!
//! The walk is iterative (no recursion on the call stack) and prunes hard: once
//! a directory is recognised as disposable output it is never descended into,
//! which keeps `moltdisk` fast on machines with a hundred `node_modules` trees.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rayon::prelude::*;

use crate::kinds::{rule_for_dir, rule_for_file, Rule, SKIP_DIRS};

#[derive(Clone)]
pub struct ScanOpts {
    pub max_depth: usize,
    /// `None` means "the kinds enabled by default".
    pub kinds: Option<Vec<&'static str>>,
    pub min_size: u64,
    pub older_than: Option<Duration>,
}

impl Default for ScanOpts {
    fn default() -> Self {
        ScanOpts {
            max_depth: 8,
            kinds: None,
            min_size: 0,
            older_than: None,
        }
    }
}

#[derive(Clone)]
pub struct Target {
    pub path: PathBuf,
    pub kind: &'static str,
    pub size: u64,
    pub files: u64,
    pub modified: Option<SystemTime>,
}

impl Target {
    pub fn label(&self) -> String {
        self.path.to_string_lossy().to_string()
    }
}

/// Rule-level opt-in: an explicit `--kind` overrides the default, so asking for
/// `-k node` also reaches entries such as `dist/` that stay off by default.
fn enabled(rule: &Rule, opts: &ScanOpts) -> bool {
    match &opts.kinds {
        Some(list) => list.contains(&rule.kind),
        None => rule.default,
    }
}

/// Walk `roots` and return every disposable target, largest first.
pub fn scan(roots: &[PathBuf], opts: &ScanOpts) -> Vec<Target> {
    let mut raw: Vec<Target> = Vec::new();
    let mut stack: Vec<(PathBuf, usize)> = Vec::new();

    for root in roots {
        if let Some(target) = root_itself(root, opts) {
            raw.push(target);
            continue;
        }
        stack.push((root.clone(), 0));
    }

    while let Some((dir, depth)) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue, // permission denied, racing deletion, …
        };

        let mut names: Vec<String> = Vec::new();
        let mut child_dirs: Vec<PathBuf> = Vec::new();
        let mut child_files: Vec<(PathBuf, u64, Option<SystemTime>)> = Vec::new();

        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            names.push(entry.file_name().to_string_lossy().to_string());
            if file_type.is_dir() {
                child_dirs.push(entry.path());
            } else if file_type.is_file() {
                let meta = entry.metadata().ok();
                child_files.push((
                    entry.path(),
                    meta.as_ref().map(|m| m.len()).unwrap_or(0),
                    meta.as_ref().and_then(|m| m.modified().ok()),
                ));
            }
        }

        for child in child_dirs {
            let name = child
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }

            if let Some(rule) = rule_for_dir(&name, &names) {
                if enabled(rule, opts) {
                    raw.push(Target {
                        path: child,
                        kind: rule.kind,
                        size: 0,
                        files: 0,
                        modified: None,
                    });
                }
                // Never descend into recognised output, enabled or not.
                continue;
            }

            if depth + 1 < opts.max_depth {
                stack.push((child, depth + 1));
            }
        }

        for (path, len, modified) in child_files {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if let Some(rule) = rule_for_file(&name, &names) {
                if enabled(rule, opts) {
                    raw.push(Target {
                        path,
                        kind: rule.kind,
                        size: len,
                        files: 1,
                        modified,
                    });
                }
            }
        }
    }

    // Measuring is I/O bound, so fan the roots out across threads.
    raw.par_iter_mut().for_each(|target| {
        if target.size == 0 && target.files == 0 {
            let (bytes, files) = measure(&target.path);
            target.size = bytes;
            target.files = files;
        }
        if target.modified.is_none() {
            target.modified = fs::metadata(&target.path).and_then(|m| m.modified()).ok();
        }
    });

    let now = SystemTime::now();
    let mut results: Vec<Target> = raw
        .into_iter()
        .filter(|t| t.size >= opts.min_size)
        .filter(|t| match (opts.older_than, t.modified) {
            (Some(limit), Some(modified)) => now
                .duration_since(modified)
                .map(|age| age >= limit)
                .unwrap_or(false),
            (Some(_), None) => false,
            (None, _) => true,
        })
        .collect();

    results.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
    results
}

/// `moltdisk ./node_modules` should do the obvious thing.
fn root_itself(root: &Path, opts: &ScanOpts) -> Option<Target> {
    let name = root.file_name()?.to_string_lossy().to_string();
    let parent = root.parent()?;
    let siblings: Vec<String> = fs::read_dir(parent)
        .ok()?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    let rule = if root.is_dir() {
        rule_for_dir(&name, &siblings)?
    } else {
        rule_for_file(&name, &siblings)?
    };
    if !enabled(rule, opts) {
        return None;
    }
    let (bytes, files) = if root.is_dir() {
        measure(root)
    } else {
        (fs::metadata(root).map(|m| m.len()).unwrap_or(0), 1)
    };
    Some(Target {
        path: root.to_path_buf(),
        kind: rule.kind,
        size: bytes,
        files,
        modified: fs::metadata(root).and_then(|m| m.modified()).ok(),
    })
}

/// Recursively sum file sizes. Symlinks are counted but not followed.
pub fn measure(path: &Path) -> (u64, u64) {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return (0, 0),
    };
    if meta.file_type().is_symlink() {
        return (0, 0);
    }
    if meta.is_file() {
        return (meta.len(), 1);
    }
    if !meta.is_dir() {
        return (0, 0);
    }

    let mut bytes = 0u64;
    let mut files = 0u64;
    let mut stack: Vec<PathBuf> = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            // `file_type` does not follow symlinks, which keeps us out of
            // symlink cycles and out of directories that merely look ours.
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                stack.push(entry.path());
            } else if file_type.is_file() {
                bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
                files += 1;
            }
        }
    }
    (bytes, files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn touch(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = fs::File::create(path).unwrap();
        file.write_all(&vec![0u8; bytes]).unwrap();
    }

    #[test]
    fn finds_node_modules_and_ignores_plain_dirs() {
        let root = std::env::temp_dir().join(format!("moltdisk-scan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        touch(&root.join("app/node_modules/left-pad/index.js"), 2048);
        touch(&root.join("app/src/main.js"), 64);
        touch(&root.join("app/target/keep.rs"), 64);

        let found = scan(std::slice::from_ref(&root), &ScanOpts::default());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, "node");
        assert_eq!(found[0].size, 2048);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn measure_counts_nested_files() {
        let root = std::env::temp_dir().join(format!("moltdisk-measure-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        touch(&root.join("a/one.txt"), 10);
        touch(&root.join("a/b/two.txt"), 32);
        let (bytes, files) = measure(&root);
        assert_eq!(bytes, 42);
        assert_eq!(files, 2);
        let _ = fs::remove_dir_all(&root);
    }
}
