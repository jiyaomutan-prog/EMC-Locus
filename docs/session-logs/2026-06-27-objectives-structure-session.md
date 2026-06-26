# Objectives and Structure Session - 2026-06-27

## Objective

Re-run a complete pass over EMC Locus objectives and update the repository
structure where the current shape no longer matched the project scope.

## Findings

- The product objectives had become spread across README, roadmap, architecture
  notes, competitive analysis, and session logs.
- The Rust core was still behaviorally healthy, but `lib.rs` had become too
  broad for the growing domain.
- The next development stream needed clearer prioritization between metrology,
  signal acquisition, offline repositories, instrument runtime, and migrations.

## Changes

- Added `docs/product-objectives.md` as the consolidated product direction.
- Added `docs/core-structure.md` to document the Rust core module map.
- Split `emc-locus-core` into modules:
  - `identifiers`;
  - `audit`;
  - `project`;
  - `quality`;
  - `repositories`;
  - `instrument`;
  - `signal`;
  - `traceability`;
  - `error`;
  - `tests`.
- Kept the public root exports intact through `lib.rs`.
- Updated README, architecture, roadmap, changelog, and Python session backlog.

## Validation Notes

Run before commit:

```text
py -m compileall python\emc_locus
cargo fmt --check
cargo test
git diff --check
```

Result: Python compilation passed, `cargo fmt --check` passed, and 29 Rust
tests passed. `git diff --check` passed.

## Next Recommended Step

Start the metrology registry module with instrument identity, status,
calibration validity, and pre-run checks.
