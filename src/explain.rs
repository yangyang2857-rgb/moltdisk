//! `moltdisk why <path>` — print the reasoning behind a verdict.
//!
//! A scanner that only prints a number is asking for trust it has not earned.
//! This module shows the working: which rules matched the name, which marker
//! each of them demanded, and which one (if any) was actually satisfied.

use std::fs;
use std::path::{Path, PathBuf};

use crate::kinds::{kind, marker_evidence, marker_wants, rules_for, Rule};
use crate::scan::measure;
use crate::size::{describe_age, human};
use crate::term::{tidy_path, Style};

pub struct Candidate {
    pub rule: &'static Rule,
    /// Name of the sibling that satisfied the marker; empty string when the
    /// name alone was enough; `None` when the marker was not satisfied.
    pub evidence: Option<String>,
}

pub struct Explanation {
    pub path: PathBuf,
    pub name: String,
    pub exists: bool,
    pub is_dir: bool,
    pub candidates: Vec<Candidate>,
}

impl Explanation {
    /// The first rule whose name matched *and* whose marker held. Same order as
    /// `scan`, so `why` can never disagree with what a scan reported.
    pub fn verdict(&self) -> Option<&Candidate> {
        self.candidates.iter().find(|c| c.evidence.is_some())
    }
}

pub fn explain(path: &Path) -> Explanation {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    let exists = path.exists();
    let is_dir = path.is_dir();
    let siblings = siblings_of(path);

    let candidates = if exists {
        rules_for(&name, is_dir)
            .into_iter()
            .map(|rule| Candidate {
                rule,
                evidence: marker_evidence(&rule.marker, &siblings),
            })
            .collect()
    } else {
        Vec::new()
    };

    Explanation {
        path: path.to_path_buf(),
        name,
        exists,
        is_dir,
        candidates,
    }
}

fn siblings_of(path: &Path) -> Vec<String> {
    path.parent()
        .and_then(|parent| fs::read_dir(parent).ok())
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default()
}

pub fn print(exp: &Explanation, style: &Style) {
    println!("{}", style.bold(&tidy_path(&exp.path)));
    println!();

    match exp.verdict() {
        Some(candidate) => {
            let kind_label = kind(candidate.rule.kind)
                .map(|k| k.label)
                .unwrap_or(candidate.rule.kind);
            println!(
                "  {}   {} — {}",
                style.dim("verdict"),
                style.green("reclaimable"),
                kind_label
            );
            println!(
                "  {}   {}  ·  marker: {}",
                style.dim("matched"),
                candidate.rule.name,
                marker_note(candidate, style)
            );
            if let Some(k) = kind(candidate.rule.kind) {
                println!("  {}   {}", style.dim("rebuild"), k.rebuild);
            }
            let (bytes, files) = if exp.is_dir {
                measure(&exp.path)
            } else {
                (fs::metadata(&exp.path).map(|m| m.len()).unwrap_or(0), 1)
            };
            println!(
                "  {}      {}  ·  {} {}",
                style.dim("size"),
                human(bytes),
                files,
                if files == 1 { "file" } else { "files" }
            );
            if let Ok(modified) = fs::metadata(&exp.path).and_then(|m| m.modified()) {
                if let Ok(age) = std::time::SystemTime::now().duration_since(modified) {
                    println!(
                        "  {}  {}",
                        style.dim("modified"),
                        describe_age(age.as_secs())
                    );
                }
            }
            println!();
            println!(
                "{}",
                style.dim(
                    "This is a report, not a permission slip. Check the directory yourself before removing it."
                )
            );
        }
        None => {
            println!(
                "  {}   {}",
                style.dim("verdict"),
                style.yellow("not build output")
            );
            if !exp.exists {
                println!("  {}   path does not exist", style.dim("reason"));
                return;
            }
            if exp.candidates.is_empty() {
                println!(
                    "  {}   no rule matches the name `{}`",
                    style.dim("reason"),
                    exp.name
                );
                println!(
                    "  {}   run `moltdisk kinds` for the full rulebook",
                    style.dim("known")
                );
            } else {
                println!(
                    "  {}   the name matched, but no marker was satisfied:",
                    style.dim("reason")
                );
                for candidate in &exp.candidates {
                    let kind_label = kind(candidate.rule.kind)
                        .map(|k| k.label)
                        .unwrap_or(candidate.rule.kind);
                    println!(
                        "    {} {} needs {} — not found",
                        style.dim("·"),
                        kind_label,
                        marker_wants(&candidate.rule.marker)
                    );
                }
            }
            println!();
            println!(
                "{}",
                style.dim(
                    "Looks like an ordinary directory or file. Without the marker, the name alone is not evidence."
                )
            );
        }
    }
}

fn marker_note(candidate: &Candidate, style: &Style) -> String {
    match candidate.evidence.as_deref() {
        Some("") => style.dim("name is self-evident"),
        Some(file) => style.dim(&format!("found `{file}`")),
        None => style.dim("not satisfied"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn scratch(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("moltdisk-why-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn accepts_rust_target_with_marker() {
        let root = scratch("rust");
        fs::File::create(root.join("Cargo.toml")).unwrap();
        fs::create_dir_all(root.join("target/debug")).unwrap();
        let exp = explain(&root.join("target"));
        assert_eq!(exp.verdict().unwrap().rule.kind, "rust");
        assert_eq!(
            exp.verdict().unwrap().evidence.as_deref(),
            Some("Cargo.toml")
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_lookalike_target() {
        let root = scratch("lookalike");
        fs::create_dir_all(root.join("target")).unwrap();
        let mut f = fs::File::create(root.join("target/main.rs")).unwrap();
        f.write_all(b"fn main() {}").unwrap();
        let exp = explain(&root.join("target"));
        assert!(exp.verdict().is_none());
        assert_eq!(
            exp.candidates.len(),
            2,
            "both Rust and JVM rules should be considered"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn explains_self_evident_names() {
        let root = scratch("node");
        fs::create_dir_all(root.join("node_modules/left-pad")).unwrap();
        let exp = explain(&root.join("node_modules"));
        let verdict = exp.verdict().unwrap();
        assert_eq!(verdict.rule.kind, "node");
        assert_eq!(verdict.evidence.as_deref(), Some(""));
        let _ = fs::remove_dir_all(&root);
    }
}
