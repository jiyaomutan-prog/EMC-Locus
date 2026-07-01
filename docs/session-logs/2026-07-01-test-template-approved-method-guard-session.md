# 2026-07-01 - Test Template Approved Method Guard Session

## Objective

Close a lifecycle-control gap in the recent agent-owned test-template workflow:
templates that reference a laboratory method must not use a draft or retired
method revision as an executable source.

## Changes

- Required referenced test-method revisions to have status `approved` during
  agent-backed test-template creation.
- Added a local API regression covering rejection of a draft method revision and
  acceptance of an approved revision.
- Updated the changelog, local-agent notes, test-template API documentation,
  and roadmap traceability.

## Boundaries

- No method-authoring route, method approval workflow, template instantiation,
  execution package generation, or version bump was added.

## Validation

- Targeted Rust local API regression:
  `cargo test -p emc-locus-agent local_api_requires_approved_method_revision_for_test_template -- --nocapture`
  passed.
- `py -m compileall python\emc_locus`: passed.
- `cargo test`: passed with `36` agent tests and `155` core tests.
