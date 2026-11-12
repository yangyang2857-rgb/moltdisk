//! Human tables and machine JSON. No rendering framework, just columns.

use std::time::SystemTime;

use crate::scan::Target;
use crate::size::{describe_age, human};
use crate::term::{display_width, ellipsis_left, pad, tidy_path, Style};

fn paint_kind(style: &Style, kind: &str, text: &str) -> String {
    match kind {
        "node" => style.green(text),
        "rust" => style.yellow(text),
        "jvm" => style.red(text),
        "python" => style.blue(text),
        "dotnet" => style.magenta(text),
        "cache" | "junk" => style.dim(text),
        _ => style.cyan(text),
    }
}

fn count(n: u64) -> String {
    if n < 1_000 {
        format!("{n}")
    } else if n < 1_000_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    }
}

fn age_of(modified: Option<SystemTime>) -> String {
    match modified.and_then(|m| SystemTime::now().duration_since(m).ok()) {
        Some(d) => describe_age(d.as_secs()),
        None => "—".to_string(),
    }
}

pub fn total_size(targets: &[Target]) -> u64 {
    targets.iter().map(|t| t.size).sum()
}

pub fn total_files(targets: &[Target]) -> u64 {
    targets.iter().map(|t| t.files).sum()
}

/// One row per target, sorted by size (already sorted by the scanner).
pub fn print_table(targets: &[Target], style: &Style, limit: Option<usize>) {
    if targets.is_empty() {
        println!(
            "{}",
            style.dim("nothing to reclaim — no build output matched the rulebook")
        );
        return;
    }

    let width = terminal_width();
    let path_budget = width.saturating_sub(42).max(24);
    let shown = match limit {
        Some(n) if targets.len() > n => n,
        _ => targets.len(),
    };

    // Pad the plain text first, then colour it: escape sequences occupy no
    // columns and would otherwise push every later column out of alignment.
    println!(
        "{} {}{}  {}  {}",
        style.dim(&format!("{:>10}", "SIZE")),
        style.dim(&pad("KIND", 9)),
        style.dim(&format!("{:>7}", "FILES")),
        style.dim(&pad("IDLE", 9)),
        style.dim("PATH")
    );

    for target in targets.iter().take(shown) {
        let size = style.bold(&format!("{:>10}", human(target.size)));
        let kind = paint_kind(style, target.kind, &pad(target.kind, 9));
        let files = style.dim(&format!("{:>7}", count(target.files)));
        let age = style.dim(&pad(&age_of(target.modified), 9));
        let path = ellipsis_left(
            &tidy_path(std::path::Path::new(&target.label())),
            path_budget,
        );
        println!("{size} {kind}{files}  {age}  {path}");
    }

    if shown < targets.len() {
        println!(
            "{}",
            style.dim(&format!(
                "… and {} more (raise --depth or lower --min-size)",
                targets.len() - shown
            ))
        );
    }
}

/// `12 targets · 8.42 GB · 486k files`, plus a per-kind breakdown.
pub fn print_summary(targets: &[Target], style: &Style) {
    if targets.is_empty() {
        return;
    }
    let size = total_size(targets);
    let files = total_files(targets);
    println!();
    println!(
        "{} {}  ·  {}  ·  {} files",
        style.bold(&targets.len().to_string()),
        style.dim(if targets.len() == 1 {
            "target"
        } else {
            "targets"
        }),
        style.green(&human(size)),
        count(files)
    );

    let mut by_kind: Vec<(&str, u64)> = Vec::new();
    for target in targets {
        match by_kind.iter_mut().find(|(k, _)| *k == target.kind) {
            Some(entry) => entry.1 += target.size,
            None => by_kind.push((target.kind, target.size)),
        }
    }
    by_kind.sort_by_key(|&(_, size)| std::cmp::Reverse(size));
    let breakdown = by_kind
        .iter()
        .take(4)
        .map(|(kind, bytes)| format!("{} {}", paint_kind(style, kind, kind), human(*bytes)))
        .collect::<Vec<_>>()
        .join("  ·  ");
    println!("{} {}", style.dim("by kind:"), breakdown);
}

pub fn print_hint(style: &Style) {
    println!();
    println!(
        "{}",
        style.dim(
            "`moltdisk why <path>` shows the evidence for any entry. This build never deletes — that call is yours."
        )
    );
}

/// Conclusion first. A reader who stops after one line should still learn the
/// answer, so the number and the three biggest piles go above the table.
pub fn print_headline(targets: &[Target], style: &Style) {
    if targets.is_empty() {
        println!(
            "{}",
            style.bold("Nothing to reclaim: no build output matched the rulebook.")
        );
        println!();
        return;
    }

    println!(
        "{}",
        style.bold(&format!(
            "{} of build output can be rebuilt from source.",
            human(total_size(targets))
        ))
    );
    println!();

    let mut biggest: Vec<&Target> = targets.iter().collect();
    biggest.sort_by_key(|t| std::cmp::Reverse(t.size));
    let width = biggest
        .iter()
        .take(3)
        .map(|t| human(t.size).chars().count())
        .max()
        .unwrap_or(0);
    for (rank, target) in biggest.iter().take(3).enumerate() {
        println!(
            "  {}. {}  {}  {}",
            rank + 1,
            style.green(&format!("{:>width$}", human(target.size))),
            paint_kind(style, target.kind, &pad(target.kind, 9)),
            style.dim(&tidy_path(std::path::Path::new(&target.label())))
        );
    }
    if targets.len() > 3 {
        println!(
            "  {} more below",
            style.dim(&format!("… {} targets", targets.len() - 3))
        );
    }
    println!();
}

pub fn print_json(targets: &[Target], roots: &[std::path::PathBuf]) {
    let mut out = String::new();
    out.push_str("{\n  \"tool\": \"moltdisk\",\n");
    out.push_str(&format!(
        "  \"version\": \"{}\",\n",
        env!("CARGO_PKG_VERSION")
    ));
    out.push_str(&format!("  \"total_size\": {},\n", total_size(targets)));
    out.push_str(&format!("  \"total_files\": {},\n", total_files(targets)));
    out.push_str("  \"roots\": [");
    for (i, root) in roots.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push('"');
        out.push_str(&escape(&root.to_string_lossy()));
        out.push('"');
    }
    out.push_str("],\n  \"targets\": [\n");

    for (i, target) in targets.iter().enumerate() {
        out.push_str(&format!(
            "    {{\"path\": \"{}\", \"kind\": \"{}\", \"size\": {}, \"files\": {}, \"modified\": {}}}",
            escape(&target.label()),
            target.kind,
            target.size,
            target.files,
            target
                .modified
                .and_then(|m| m.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs().to_string())
                .unwrap_or_else(|| "null".to_string())
        ));
        if i + 1 < targets.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    print!("{out}");
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn terminal_width() -> usize {
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        // Piped into a file or a pager: no reason to be stingy with columns.
        return 160;
    }
    // `COLUMNS` is exported by most shells; without it 100 is a safe guess that
    // keeps the path column readable on a narrow terminal.
    std::env::var("COLUMNS")
        .ok()
        .and_then(|c| c.parse::<usize>().ok())
        .unwrap_or(100)
        .clamp(60, 240)
}

/// Boxed banner used by `--help`-free first runs and `moltdisk kinds`.
pub fn print_kinds(style: &Style) {
    println!(
        "{}",
        style.bold("moltdisk knows these kinds of disposable output:")
    );
    println!();
    let widest = crate::kinds::KINDS
        .iter()
        .map(|k| display_width(k.id))
        .max()
        .unwrap_or(6);
    for kind in crate::kinds::KINDS {
        let marker = if kind.default {
            style.dim("on ")
        } else {
            style.dim("off")
        };
        println!(
            "  {}  {}  {}  {}",
            marker,
            paint_kind(style, kind.id, &pad(kind.id, widest)),
            style.bold(&pad(kind.label, 12)),
            style.dim(kind.blurb)
        );
        println!(
            "      {} {}",
            style.dim("rebuild:"),
            style.dim(kind.rebuild)
        );
    }
    println!();
    println!(
        "{}",
        style.dim(
            "`off` kinds stay out of a plain run; opt in with `-k <kind>`. `rebuild` is why each one is on the list."
        )
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_get_short() {
        assert_eq!(count(999), "999");
        assert_eq!(count(1_500), "1.5k");
        assert_eq!(count(2_400_000), "2.4M");
    }

    #[test]
    fn json_escapes_paths() {
        assert_eq!(escape("a\"b"), "a\\\"b");
        assert_eq!(escape("a\\b"), "a\\\\b");
    }

    #[test]
    fn every_kind_is_paintable() {
        // No kind may fall into an unknown-code path at runtime.
        for kind in crate::kinds::KINDS {
            assert_eq!(paint_kind(&Style::new(false), kind.id, "x"), "x");
        }
    }
}
