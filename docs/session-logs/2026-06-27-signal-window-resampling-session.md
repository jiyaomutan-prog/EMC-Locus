# 2026-06-27 Signal Window and Resampling Session

## Intent

Extend deterministic signal execution with basic windowing and resampling
operations before adding external DSP dependencies.

## Changes

- Added floating-point signal series results.
- Added window function vocabulary.
- Added rectangular and Hann window coefficients.
- Added window execution over acquired signal channels.
- Added deterministic downsampling execution.
- Added invalid resampling-factor error.
- Added tests for Hann window edges, deterministic downsampling, and invalid
  resampling factor.
- Updated signal architecture, roadmap, changelog, README, and backlog.

## Validation

- `py -m compileall python\emc_locus` passed.
- SQLite migration validation passed for all five repository domains.
- `cargo fmt --check` passed.
- `cargo test` passed with 93 tests.
- `git diff --check` passed.

## Next

- Add signed update bundle workflow.
- Add real transport adapter spikes behind the simulated baseline.
- Add optimized FFT and interpolation-based resampling.
