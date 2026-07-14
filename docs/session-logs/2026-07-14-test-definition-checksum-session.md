# 2026-07-14 - Test Definition Checksum Session

## Goal

Close the next Python-side checksum validation gap before starting new
`0.14.0` physical-asset scope.

## Work Completed

- Tightened `TestDefinitionRepository.add_method_revision` and
  `approve_method_revision` so optional method revision checksums must use
  canonical `sha256:<64 lowercase hex characters>` evidence.
- Updated bootstrap and test-definition fixtures away from short placeholder
  checksum values.
- Added regression coverage proving shortened creation checksums and uppercase
  approval checksums are rejected before repository state changes.
- Updated `CHANGELOG.md`, `docs/roadmap.md`, and
  `docs/storage-migrations.md`.

## Validation Notes

Passed during implementation:

```text
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.TestDefinitionRepositoryTests
py -m compileall python/emc_locus
$env:PYTHONPATH='python'; py -m unittest discover -s python\tests
cargo test
```

## Limits

This session only fixes Python test-definition method-revision checksum
validation. It does not change SQLite migrations, validate historical direct-SQL
rows, bump `VERSION`, create a release tag, implement physical assets, station
wiring, acquisition, FFT, reports, RBAC, or central sync.
