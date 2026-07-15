# 2026-07-15 - Dataset Checksum Hardening Session

## Objective

Close the next checksum-evidence cleanup item by making Rust dataset checksum
evidence reject shortened or uppercase `sha256:` fixtures before records can be
constructed.

## Work Completed

- Tightened `DatasetChecksum::parse` to require `sha256:<64 lowercase hex>`.
- Added rejection coverage for shortened and uppercase dataset checksum values.
- Replaced legacy short Rust core dataset, processing-graph and report-export
  fixtures with canonical checksum constants.
- Updated the changelog and roadmap to record the core invariant.

## Validation Evidence

- `cargo test -p emc-locus-core`: pass, 232 tests.
- `py -m compileall python\emc_locus`: pass.
- `cargo test`: pass, 56 agent tests and 232 core tests.
- `git diff --check`: pass, with only the expected CRLF working-copy
  warnings on Windows.

## Remaining Limits

- This session hardens the Rust core dataset checksum type. It does not change
  older snapshot checksum semantics, which are still used as opaque repository
  snapshot identifiers in synchronization and update tests.
