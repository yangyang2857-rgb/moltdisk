//! Argument parsing. Hand-rolled so `moltdisk` stays a one-dependency binary.

use std::path::PathBuf;
use std::time::Duration;

use crate::kinds::{kind, KINDS};
use crate::size::{parse_age, parse_size};

#[derive(Debug, PartialEq, Clone)]
pub enum Command {
    Scan,
    /// Kept only so `moltdisk clean` can refuse with a clear explanation.
    Clean,
    Kinds,
    Why(PathBuf),
}

pub struct Cli {
    pub command: Command,
    pub roots: Vec<PathBuf>,
    /// Whether the user actually named a path, or we defaulted to `.`.
    pub roots_explicit: bool,
    pub kinds: Option<Vec<&'static str>>,
    pub min_size: u64,
    pub older_than: Option<Duration>,
    pub depth: usize,
    pub jobs: Option<usize>,
    pub json: bool,
    pub no_color: bool,
    pub quiet: bool,
}

pub const HELP: &str = "\
moltdisk · 蜕冗 — shed your dev bulk

moltdisk audits disks for build output that can be rebuilt from source, and shows
the evidence behind every verdict. It never deletes anything.
蜕冗：只报告可重建的冗余产物，从不删除。

Usage
  moltdisk [paths...] [options]
  moltdisk why <path>
  moltdisk kinds

Commands
  (none)    Report reclaimable build output under the given paths
  why       Explain the verdict for one directory or file
  kinds     List the categories moltdisk knows about

Options
  -k, --kind <list>      Only these kinds, comma separated (node,rust,jvm,python,
                         dotnet,php,swift,apple,cache,junk) or `all`
  -s, --min-size <size>  Ignore targets smaller than this (e.g. 500mb)
  -t, --older-than <age> Only targets untouched for this long (e.g. 30d, 12h)
  -d, --depth <n>        How deep to walk from each root (default: 8)
      --jobs <n>         Worker threads for measuring (default: cores)
      --json             Machine-readable output on stdout
      --no-color         Disable ANSI colour
  -q, --quiet            No progress chatter
  -h, --help             This text
  -V, --version          Print version

Removed in 0.1.0
  -y, --yes, --dry-run, clean
                         This build cannot delete, so the deleting flags and the
                         `clean` command were removed. `clean` still answers, and
                         answers with a refusal.

Examples
  moltdisk                                  # audit the current directory
  moltdisk ~/code -t 30d -s 1gb             # only big, stale trees
  moltdisk why web-shop/node_modules        # why does it think that is disposable?
  moltdisk --json | jq .total               # machine-readable
";

pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Cli, String> {
    let mut cli = Cli {
        command: Command::Scan,
        roots_explicit: false,
        roots: Vec::new(),
        kinds: None,
        min_size: 0,
        older_than: None,
        depth: 8,
        jobs: None,
        json: false,
        no_color: false,
        quiet: false,
    };

    let mut command_set = false;
    let mut why = false;
    let mut iter = args.into_iter().skip(1).peekable();

    while let Some(arg) = iter.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };

        let mut value = |name: &str| -> Result<String, String> {
            if let Some(v) = inline.clone() {
                return Ok(v);
            }
            iter.next().ok_or_else(|| format!("{name} needs a value"))
        };

        match flag.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("moltdisk {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-y" | "--yes" | "--delete" => {
                return Err(
                    "this build never deletes — `-y` was removed in 0.1.0 (see `moltdisk --help`)"
                        .into(),
                );
            }
            "-n" | "--dry-run" => {
                return Err("moltdisk is always read-only now, so `--dry-run` is redundant".into());
            }
            "--json" => cli.json = true,
            "--no-color" => cli.no_color = true,
            "-q" | "--quiet" => cli.quiet = true,
            "-k" | "--kind" => {
                let raw = value("--kind")?;
                cli.kinds = Some(parse_kinds(&raw)?);
            }
            "-s" | "--min-size" => {
                cli.min_size = parse_size(&value("--min-size")?)?;
            }
            "-t" | "--older-than" => {
                cli.older_than = Some(parse_age(&value("--older-than")?)?);
            }
            "-d" | "--depth" => {
                let raw = value("--depth")?;
                cli.depth = raw.parse().map_err(|_| format!("`{raw}` is not a depth"))?;
                if cli.depth == 0 {
                    return Err("--depth must be at least 1".into());
                }
            }
            "--jobs" => {
                let raw = value("--jobs")?;
                let jobs: usize = raw
                    .parse()
                    .map_err(|_| format!("`{raw}` is not a job count"))?;
                if jobs == 0 {
                    return Err("--jobs must be at least 1".into());
                }
                cli.jobs = Some(jobs);
            }
            "-" => {}
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}` (try `moltdisk --help`)"));
            }
            other => {
                if !command_set {
                    match other {
                        "scan" => {
                            cli.command = Command::Scan;
                            command_set = true;
                            continue;
                        }
                        "clean" => {
                            cli.command = Command::Clean;
                            command_set = true;
                            continue;
                        }
                        "kinds" => {
                            cli.command = Command::Kinds;
                            command_set = true;
                            continue;
                        }
                        "why" => {
                            why = true;
                            command_set = true;
                            continue;
                        }
                        _ => {}
                    }
                }
                cli.roots.push(PathBuf::from(other));
                cli.roots_explicit = true;
            }
        }
    }

    if why {
        if cli.roots.is_empty() {
            return Err("`moltdisk why` needs a path".into());
        }
        let path = cli.roots.remove(0);
        cli.command = Command::Why(path);
        return Ok(cli);
    }

    if cli.command != Command::Kinds && cli.roots.is_empty() {
        cli.roots.push(PathBuf::from("."));
    }
    Ok(cli)
}

fn parse_kinds(raw: &str) -> Result<Vec<&'static str>, String> {
    let mut out = Vec::new();
    for piece in raw.split(',') {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        if piece == "all" {
            return Ok(KINDS.iter().map(|k| k.id).collect());
        }
        match kind(piece) {
            Some(found) => out.push(found.id),
            None => {
                let ids: Vec<&str> = KINDS.iter().map(|k| k.id).collect();
                return Err(format!(
                    "unknown kind `{piece}` — known: {}",
                    ids.join(", ")
                ));
            }
        }
    }
    if out.is_empty() {
        return Err("--kind needs at least one kind".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Cli, String> {
        parse(args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn defaults_to_scanning_cwd() {
        let cli = parse_args(&["moltdisk"]).unwrap();
        assert!(cli.command == Command::Scan);
        assert_eq!(cli.roots, vec![PathBuf::from(".")]);
    }

    #[test]
    fn understands_inline_values() {
        let cli = parse_args(&["moltdisk", "scan", "--min-size=512mb", "-t", "30d"]).unwrap();
        assert_eq!(cli.min_size, 512 * 1024 * 1024);
        assert_eq!(cli.older_than.unwrap().as_secs(), 30 * 86_400);
    }

    #[test]
    fn rejects_nonsense() {
        assert!(parse_args(&["moltdisk", "--min-size", "lots"]).is_err());
        assert!(parse_args(&["moltdisk", "-k", "cobol"]).is_err());
        assert!(parse_args(&["moltdisk", "--wat"]).is_err());
    }

    #[test]
    fn refuses_destructive_flags() {
        assert!(parse_args(&["moltdisk", "-y"]).is_err());
        assert!(parse_args(&["moltdisk", "--dry-run"]).is_err());
    }

    #[test]
    fn why_needs_a_path() {
        assert!(parse_args(&["moltdisk", "why"]).is_err());
        let cli = parse_args(&["moltdisk", "why", "./node_modules"]).unwrap();
        assert_eq!(cli.command, Command::Why(PathBuf::from("./node_modules")));
    }

    #[test]
    fn kind_all_expands() {
        let cli = parse_args(&["moltdisk", "-k", "all"]).unwrap();
        assert_eq!(cli.kinds.unwrap().len(), KINDS.len());
    }
}
