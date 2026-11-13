# Contributing to moltdisk

Thanks for looking. The most valuable contributions here are **new rules** and
**new ways for a rule to be wrong**.

## The rule I want to add is missing an artefact

Everything the scanner knows lives in one table: `RULES` in
[`src/kinds.rs`](src/kinds.rs). Adding an artefact is usually three lines.

```rust
Rule { name: ".cargo-cache", kind: "cache", marker: Marker::Any, default: true, is_dir: true },
```

Then answer these questions in the pull request:

1. **What marker proves the folder belongs to a project?** If a hand-written
   folder could plausibly carry the same name, the marker is not optional —
   use `Marker::Files` or `Marker::Suffix`. `target/` next to `Cargo.toml` is
   safe; `target/` anywhere is not.
2. **Can it be regenerated with one command?** If it costs a network round trip
   or a long build, say so — that is fine, but it belongs in the pull request
   description.
3. **Is it on by default?** Anything whose name is ambiguous (`dist`, `build`,
   `bin`, `venv`) ships as `default: false`, so `moltdisk` alone never touches it.

## What a change must not break

`moltdisk` deletes directories for a living, so the bar is higher than "it compiled":

- `cargo test` — unit tests cover the rule table, the size parser, and the
  output layout; `tests/cli.rs` drives the real binary end to end.
- `cargo clippy --all-targets -- -D warnings` — CI treats warnings as errors.
- `cargo fmt --all -- --check`.
- Every deletion path in the tests is paired with an assertion that a **decoy**
  with the same name but no marker survived. Please keep that habit.

## Reporting a false positive

A false positive is the only kind of bug that matters here: a report gets
copy-pasted into `rm`, so a wrong verdict is not a cosmetic defect. Use the
[false-positive form](https://github.com/yangyang2857-rgb/moltdisk/issues/new?template=false-positive.yml),
which asks for the three things such a report needs — the path that was matched,
the sibling files in its parent directory, and what `moltdisk why <path>` printed.
The form is the contract; if a field in it turns out to be useless for triage,
that is a bug in the form and worth a pull request.

If you can, add a failing test to `tests/cli.rs` first. That file builds its
fixtures from scratch, so a reproduction is about ten lines, and the fixture
doubles as the specification for the fix.

## Style

Small modules, no `unsafe`, no dependencies unless they remove a lot of code.
Output goes through `src/term.rs`, so it stays aligned in pipes, in TTYs, and
in `NO_COLOR` mode.
