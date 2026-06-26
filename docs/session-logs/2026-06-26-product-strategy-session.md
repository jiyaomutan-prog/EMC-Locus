# Product Strategy Session - 2026-06-26

## Objective

Capture the product direction needed for EMC Locus to compete with BAT-EMC
while addressing known pain points: remote-reference dependency, overly rigid
quality constraints, weak instrument control, monolithic database design, and
missing update management.

## Changes

- Added a public BAT-EMC/Nexio feature baseline under `docs/competitive-analysis/`.
- Added offline-first architecture notes.
- Added transport-neutral instrument-control architecture notes.
- Updated the main architecture and roadmap.
- Added Rust domain primitives for:
  - accredited, non-accredited, and investigation execution modes;
  - connectivity mode;
  - split repository domains and synchronization direction;
  - instrument transport coverage;
  - update package policy.
- Added Rust tests for the new policy primitives.
- Updated `CHANGELOG.md`.

## Public Research Notes

The feature baseline uses public Nexio and distributor pages only. No
proprietary BAT-EMC artifact, UI, schema, code, or confidential document was
used.

## Product Decisions

- EMC Locus must be offline-first for field acquisition.
- EMC Locus must separate repository domains instead of using one opaque
  database.
- EMC Locus must support distinct execution modes so non-COFRAC and
  investigation work can be controlled without pretending to be accredited.
- EMC Locus must treat instrument control as a transport-neutral runtime with
  simulations, capability declarations, command logs, and safety checks.
- EMC Locus must include a signed, offline-capable update manager.

## Validation Notes

Run before commit:

```text
py -m compileall python\emc_locus
cargo fmt --check
cargo test
```

Result: Python compilation passed, `cargo fmt --check` passed after installing
rustfmt, and 25 Rust tests passed.

## Next Recommended Step

Start the metrology registry with instrument identity, calibration validity, and
out-of-service status, while aligning persistence with the split metrology
repository direction.
