# Stage Gate Session - 2026-06-26

## Objective

Resolve the next existing workflow gap before adding metrology features: the
contract-review checklist existed, but it was not yet connected to the project
stage transition into test planning.

## Changes

- Added `ProjectRecord::advance_to_test_planning`.
- Added `AuthorizedDeviation` for controlled incomplete contract reviews.
- Added a dedicated `ContractReviewDeviationAuthorized` audit action.
- Added domain errors for checklist/project mismatch and incomplete contract
  review.
- Added tests for complete checklist, incomplete checklist, authorized
  deviation, checklist/project mismatch, and invalid source stage.
- Updated `CHANGELOG.md`, `docs/domain-model.md`, and `docs/roadmap.md`.

## Product Rationale

The project lifecycle should not rely on UI discipline to enforce laboratory
quality gates. The Rust core now prevents a project from entering test planning
unless contract review evidence is complete or an authorized deviation is
recorded in the audit trail.

## Validation Notes

Run before commit:

```text
py -m compileall python\emc_locus
cargo test
```

Result: Python compilation passed, and 18 Rust tests passed.

## Next Recommended Step

Start the metrology registry:

1. instrument identity;
2. instrument status;
3. calibration record validity;
4. pre-run equipment checks;
5. storage schema alignment.
