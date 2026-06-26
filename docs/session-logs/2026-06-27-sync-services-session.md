# 2026-06-27 Sync Services Session

## Intent

Turn synchronization conflicts from passive records into reviewable action
plans that can drive later persistence, audit events, and merge tooling.

## Changes

- Added stable slugs for synchronization conflict kinds and resolutions.
- Added synchronization actions for pushing local snapshots, pulling reference
  snapshots, manual merge, deletion application, and deferral.
- Added sync conflict action plans that preserve conflict id, domain, kind,
  selected resolution, action, and local/reference snapshot ids.
- Added a sync conflict service that can plan and apply resolutions.
- Kept deferred conflicts pending and resolved conflicts out of the pending
  queue.
- Rejected unknown conflict ids.
- Rejected invalid resolution choices for a conflict kind.
- Added tests for action-plan mapping, resolution application, deferral,
  unknown conflicts, and invalid resolutions.
- Updated offline-first architecture, roadmap, changelog, README, objectives,
  core-structure notes, and backlog.

## Validation

- `py -m compileall python\emc_locus` passed.
- SQLite migration validation passed for all five repository domains.
- `cargo fmt --check` passed.
- `cargo test` passed with 110 tests.
- `git diff --check` passed.

## Next

- Add broader SQLite write/update APIs for existing repositories.
- Add update-catalog persistence APIs for signed bundles and install records.
- Add sync persistence adapters around conflict action plans.
