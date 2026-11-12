//! moltdisk · 蜕冗 — shed your dev bulk.
//!
//! This build is an auditor, not a deleter. It walks your projects, recognises
//! build output that can be rebuilt from source, explains the evidence behind
//! every verdict, and then stops. There is no delete code path in this binary:
//! no recursive removal call is left in `src/`, and `-y` is rejected with an error.
//!
//! That is a deliberate design decision, not a missing feature. See README.

mod cli;
mod explain;
mod kinds;
mod report;
mod scan;
mod size;
mod term;

use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use cli::{Cli, Command};
use scan::ScanOpts;
use term::Style;

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("moltdisk: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<u8, Box<dyn std::error::Error>> {
    let cli = match cli::parse(std::env::args()) {
        Ok(cli) => cli,
        Err(message) => {
            eprintln!("moltdisk: {message}");
            return Ok(2);
        }
    };

    let style = Style::new(term::colour_allowed(cli.no_color));

    match &cli.command {
        Command::Kinds => {
            report::print_kinds(&style);
            return Ok(0);
        }
        Command::Why(path) => {
            let explanation = explain::explain(path);
            if !explanation.exists {
                eprintln!("moltdisk: no such path — {}", path.display());
                return Ok(1);
            }
            explain::print(&explanation, &style);
            return Ok(0);
        }
        Command::Clean => {
            eprintln!("moltdisk: this build never deletes anything — it is an auditor.");
            eprintln!(
                "      run `moltdisk` to see what is reclaimable, or `moltdisk why <path>` for the evidence."
            );
            eprintln!(
                "      if you want the deleting build, it is tagged `v0.1.0` and carries the old safety guards."
            );
            return Ok(2);
        }
        Command::Scan => {}
    }

    if let Some(jobs) = cli.jobs {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build_global();
    }

    let roots = resolve_roots(&cli, &style);
    if roots.is_empty() {
        eprintln!("moltdisk: no usable paths to scan");
        return Ok(1);
    }

    let opts = ScanOpts {
        max_depth: cli.depth,
        kinds: cli.kinds.clone(),
        min_size: cli.min_size,
        older_than: cli.older_than,
    };

    let started = Instant::now();
    if io::stderr().is_terminal() && !cli.quiet && !cli.json {
        eprint!(
            "{}",
            style.dim(&format!("scanning {} … ", describe_roots(&roots)))
        );
        let _ = io::Write::flush(&mut io::stderr());
    }
    let targets = scan::scan(&roots, &opts);
    let elapsed = started.elapsed();
    if io::stderr().is_terminal() && !cli.quiet && !cli.json {
        eprintln!(
            "{}",
            style.dim(&format!("done in {}ms", elapsed.as_millis()))
        );
    }

    if cli.json {
        report::print_json(&targets, &roots);
        return Ok(0);
    }

    report::print_headline(&targets, &style);

    if !targets.is_empty() {
        let limit = if io::stdout().is_terminal() {
            Some(20)
        } else {
            None
        };
        report::print_table(&targets, &style, limit);
        report::print_summary(&targets, &style);
    }

    report::print_hint(&style);
    Ok(0)
}

fn resolve_roots(cli: &Cli, style: &Style) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for root in &cli.roots {
        if root.exists() {
            roots.push(root.clone());
        } else {
            eprintln!(
                "moltdisk: skipping {} — {}",
                root.display(),
                style.dim("no such file or directory")
            );
        }
    }
    roots
}

fn describe_roots(roots: &[PathBuf]) -> String {
    if roots.len() == 1 {
        term::tidy_path(roots[0].as_path())
    } else {
        format!("{} roots", roots.len())
    }
}
