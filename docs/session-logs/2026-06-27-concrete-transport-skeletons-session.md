# 2026-06-27 Concrete Transport Skeletons Session

## Intent

Add the first concrete transport adapter types without pretending that hardware
IO exists in the core crate.

## Changes

- Added transport timeout policy with validation.
- Added VISA transport adapter skeleton.
- Added TCP/IP transport adapter skeleton.
- Added serial transport adapter skeleton.
- Added endpoint transport validation for concrete adapters.
- Added explicit unavailable-IO errors for concrete adapters.
- Kept simulated adapter as the only deterministic fake exchange.
- Added tests for timeout validation, adapter endpoint mismatch, unavailable
  concrete exchange, and runtime behavior when concrete IO is unavailable.
- Updated instrument-control docs, roadmap, changelog, README, objectives, and
  backlog.

## Validation

- `py -m compileall python\emc_locus` passed.
- SQLite migration validation passed for all five repository domains.
- `cargo fmt --check` passed.
- `cargo test` passed with 116 tests.
- `git diff --check` passed.

## Next

- Add sync persistence adapters around conflict action plans.
- Add SQLite adapters for measurement data and test-definition domains.
- Add IO-backed VISA, TCP/IP, or serial implementations behind the skeletons.
