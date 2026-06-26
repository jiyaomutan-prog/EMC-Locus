# 2026-06-27 Update Bundles Session

## Intent

Add the first core workflow for a controlled update manager that can support
offline field stations without weakening laboratory traceability.

## Changes

- Added update package names and semantic software versions.
- Added update signatures and rollback references.
- Added update components for core application, instrument drivers, signal
  processing, report templates, and database migrations.
- Added compatibility ranges for installed-version checks.
- Added update bundles with checksum, signature, offline-install flag, and
  rollback metadata.
- Added install planning that rejects unsigned packages, incompatible installed
  versions, disallowed offline bundles, and updates during active acquisition.
- Updated roadmap, objectives, storage notes, offline architecture, changelog,
  README, core-structure notes, and backlog.

## Validation

- `py -m compileall python\emc_locus` passed.
- SQLite migration validation passed for all five repository domains.
- `cargo fmt --check` passed.
- `cargo test` passed with 100 tests.
- `git diff --check` passed.

## Next

- Add the first real transport adapter spike behind the simulated runtime.
- Add sync application services around split-repository conflict records.
- Add update-catalog persistence APIs for signed bundles and install records.
