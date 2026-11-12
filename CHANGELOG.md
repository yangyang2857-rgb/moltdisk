# Changelog

All notable changes to `moltdisk` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[semantic versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] — 2026-11-13

The auditing release. `moltdisk` no longer deletes anything, and says so loudly.

### Removed

- **Deletion, entirely.** The `remove()` primitive, `delete_targets()`, the
  interactive picker, the `clean` command's real behaviour, and the `-y` /
  `--yes` / `--dry-run` flags. `clean` still parses, then refuses with exit
  code 2 and an explanation. `-y` is a hard error.
- The `crossterm` dependency. The picker is gone, so raw-mode terminal handling
  is gone with it; the binary now depends only on `rayon`.

### Added

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
- A README section documenting how to verify the absence of delete code
  (`strings target/release/moltdisk | grep -c remove_dir_all`).

### Changed

- `moltdisk` with no subcommand always scans. It no longer opens a picker when
  attached to a TTY.
- Argument parsing drops the `--sort` flag from v0.1.0; results are always
  largest-first.
- Terminal width comes from `COLUMNS` instead of a crossterm ioctl, and falls
  back to 100 columns for a TTY, 160 for a pipe.
- Test suite re-framed around the read-only contract: every end-to-end test now
  also asserts the scanned tree still exists afterwards.

## [0.1.0] — 2026-11-11

First release. A single binary that finds regenerable build output and, on
request, deletes it.

### Added

- `moltdisk scan` — walk one or more roots, report every artefact with its size,
  file count and idle time. Read-only by construction.
- `moltdisk clean` — the same report, but with a view you can act on: the bare
  `moltdisk` command opens an interactive picker when stdout is a TTY, and falls
  back to scanning when piped.
- `moltdisk kinds` — list the rulebook, including which kinds are on by default.
- Rule table covering Node/web, Rust, JVM, Python, .NET, PHP, Swift, Xcode
  tooling, editor caches and OS junk. Every ambiguous name requires a sibling
  marker file, and `dist/`, `build/`, `bin/` and `venv/` are off by default.
- `--json` output for scripting, `--no-color`, `--depth`, `--min-size`,
  `--kind/-k`, `--sort`, `--quiet`, `--yes`, `--dry-run`.
- Safety rails: a bare `moltdisk clean` cannot delete into `$HOME`, the picker
  refuses to act without a selection, and failed deletions are reported per
  path instead of aborting the run.
- Parallel scanning with a bounded worker pool, sized from the machine's
  parallelism.
- 28 tests: unit coverage of the rule table, human-readable sizes, ANSI-safe
  table alignment, plus end-to-end tests that build a decoy-filled tree and
  assert the decoys survive `clean`.

[0.1.0]: https://github.com/moltdisk-cli/moltdisk/releases/tag/v0.1.0
