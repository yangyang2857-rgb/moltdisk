# Changelog

All notable changes to `moltdisk` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[semantic versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-11-13

First public release. `moltdisk` (蜕冗) audits a dev machine: it finds build
output that can be rebuilt from source, shows the evidence, and never deletes
anything.

Earlier internal iterations carried a deletion path; it was never published, and
the first public version ships without it by construction.

### Added

- `moltdisk scan` — walk one or more roots, report every artefact with its size,
  file count and idle time. Read-only by construction.
- `moltdisk kinds` — list the rulebook, including which kinds are on by default
  and how each one is rebuilt.
- `moltdisk why <path>` — the reasoning behind a verdict, on demand: which rules
  matched the name, what marker each rule demanded, which marker was actually
  found, how the category is rebuilt, size, file count and idle time. For a
  rejected path it lists every rule that was considered and what each wanted.
- A `rebuild` line per kind in the rulebook, surfaced by `moltdisk kinds` and by
  `moltdisk why`. The rebuild path is the actual argument for disposability, so it
  belongs in the tool's output rather than only in the docs.
- `marker_evidence()`, `marker_wants()` and `rules_for()` in the rule engine, so
  the explanation is derived from the same functions that make the verdict —
  `why` cannot drift from `scan`.
- Conclusion-first output: the total, then the three largest piles, then the
  evidence table.
- Rule table covering Node/web, Rust, JVM, Python, .NET, PHP, Swift, Xcode
  tooling, editor caches and OS junk. Every ambiguous name requires a sibling
  marker file, and `dist/`, `build/`, `bin/` and `venv/` are off by default.
- `--json` output for scripting, plus `--no-color`, `--depth`, `--min-size`,
  `--kind/-k`, `--quiet`.
- Parallel scanning with a bounded worker pool, sized from the machine's
  parallelism.
- A README section documenting how to verify the absence of delete code
  (`strings target/release/moltdisk | grep -c remove_dir_all`).

### Changed

- `moltdisk` with no subcommand always scans. It never opens a picker.
- Results are always largest-first; there is no `--sort` flag.
- Terminal width comes from `COLUMNS` instead of a crossterm ioctl, and falls
  back to 100 columns for a TTY, 160 for a pipe.
- Test suite re-framed around the read-only contract: every end-to-end test also
  asserts the scanned tree still exists afterwards.

### Not included

- **Deletion, of any kind.** No `remove()` primitive, no picker, no `-y` /
  `--yes` / `--dry-run`. `clean` still parses, then refuses with exit code 2 and
  an explanation; `-y` is a hard error. The binary depends only on `rayon`.

[0.1.0]: https://github.com/yangyang2857-rgb/moltdisk/releases/tag/v0.1.0
