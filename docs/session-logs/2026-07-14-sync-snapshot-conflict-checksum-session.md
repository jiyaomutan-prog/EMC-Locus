# 2026-07-14 - Sync Snapshot Conflict Checksum Session

## Goal

Close a small Python synchronization checksum validation gap before starting
new `0.14.0` physical-asset scope.

## Work Completed

- Tightened `SyncRepository.record_snapshot_conflict` so persisted local and
  reference snapshot checksums are revalidated as canonical
  `sha256:<64 lowercase hex characters>` evidence before comparison.
- Added regression coverage for a constraint-bypassed imported snapshot row
  with an uppercase checksum, proving conflict detection rejects it before
  opening a conflict.
- Updated `CHANGELOG.md`, `docs/storage-migrations.md`,
  `docs/offline-first-architecture.md`, and `docs/roadmap.md`.

## Validation Notes

Passed during implementation:

```text
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.SyncRepositoryTests.test_rejects_corrupted_snapshot_checksums_before_conflict_detection
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.SyncRepositoryTests
py -m compileall python/emc_locus
$env:PYTHONPATH='python'; py -m unittest discover -s python\tests
```

An isolated temporary worktree based on `HEAD` plus only this sync checksum
patch also passed:

```text
py -m compileall python/emc_locus
$env:PYTHONPATH='python'; py -m unittest discover -s python\tests
cargo test
```

The primary worktree's direct `cargo test` run failed on unrelated in-progress
equipment changes present after this session started: one test expected
equipment schema version `4` while the worktree has migration `0005`, and one
category-tree test expected seven root categories while the worktree now nests
families under `general_equipment`.

## Limits

This session only fixes Python sync snapshot checksum validation during
conflict detection. It does not change SQLite migrations, validate historical
databases in place, bump `VERSION`, create a release tag, implement physical
assets, station wiring, acquisition, FFT, reports, RBAC, or central sync.
