# 2026-07-14 - Update Catalog Checksum Session

## Goal

Close a remaining Python-side checksum validation gap in update package
metadata before adding new `0.14.0` physical-asset scope.

## Work Completed

- Tightened `UpdateCatalogRepository.add_update_package` so signed package
  checksums must use canonical `sha256:<64 lowercase hex characters>` evidence.
- Updated update-catalog and GUI/bootstrap test fixtures to use full canonical
  SHA-256 checksums.
- Added regression coverage proving uppercase and shortened signed package
  checksums are rejected before package metadata is persisted.
- Updated `CHANGELOG.md`, `docs/roadmap.md`, and
  `docs/storage-migrations.md`.

## Validation Notes

Passed during implementation:

```text
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.UpdateCatalogRepositoryTests
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.GuiBootstrapTests.test_builds_bootstrap_from_local_repositories python.tests.test_sqlite_repositories.GuiActionTests.test_update_actions_record_validation_install_and_refresh_bootstrap
py -m compileall python/emc_locus
$env:PYTHONPATH='python'; py -m unittest discover -s python\tests
cargo test
```

## Limits

This session only fixes Python update-catalog package checksum validation. It
does not change the SQLite schema, bump `VERSION`, create a release tag,
implement physical assets, station wiring, acquisition, FFT, reports, RBAC, or
central sync.
