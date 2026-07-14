# 2026-07-14 - Measurement Data Checksum Session

## Goal

Close a remaining Python-side checksum validation gap in measurement-data
lineage before adding new `0.14.0` physical-asset scope.

## Work Completed

- Tightened `MeasurementDataRepository` so dataset, processing-graph,
  processing-graph instance, processing-graph artifact, and result-artifact
  writes require canonical `sha256:<64 lowercase hex characters>` evidence.
- Rejected caller-provided source-dataset checksums unless they are canonical
  and still match the persisted dataset checksum.
- Updated measurement-data and Qt/bootstrap fixtures to avoid short
  fixture-like checksum evidence.
- Added regression coverage proving shortened and uppercase measurement-data
  checksums are rejected before related rows are persisted.
- Updated `CHANGELOG.md`, `docs/roadmap.md`, `docs/storage-migrations.md`, and
  `docs/domain-model.md`.

## Validation Notes

Passed during implementation:

```text
$env:PYTHONPATH='python'; py -m unittest python.tests.test_sqlite_repositories.MeasurementDataRepositoryTests
$env:PYTHONPATH='python'; py -m unittest python.tests.test_qt_console
py -m compileall python/emc_locus
$env:PYTHONPATH='python'; py -m unittest discover -s python\tests
cargo test
```

## Limits

This session only fixes Python measurement-data checksum validation. It does
not change SQLite migrations, validate historical direct-SQL rows, bump
`VERSION`, create a release tag, implement physical assets, station wiring,
acquisition, FFT, reports, RBAC, or central sync.
