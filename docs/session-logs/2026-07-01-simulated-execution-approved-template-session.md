# 2026-07-01 - Simulated Execution Approved Template Session

## Objective

Add a small execution-control guard between the agent-owned test-template
lifecycle and the existing simulated EMC launch route.

## Changes

- Simulated EMC execution now checks whether `test_method_reference` matches a
  stored test template.
- Known templates must have status `approved`; draft or under-review templates
  are rejected with `test_execution_template_not_approved`.
- Added a local API regression covering draft-template rejection and approved
  template acceptance through the existing metrology preflight path.
- Updated the changelog, local-agent notes, test-template API documentation,
  and roadmap traceability.

## Boundaries

- Free-form method references continue to work.
- No project/campaign template instantiation, resolved variable snapshot,
  method-parameter persistence, measurement-data evidence, version bump, or tag
  was added.

## Validation

- Targeted Rust local API regression:
  `cargo test -p emc-locus-agent local_api_requires_approved_test_template_for_simulated_execution -- --nocapture`
  passed.
- `cargo fmt --check`: passed.
- `py -m compileall python\emc_locus`: passed.
- `cargo test`: passed with `37` agent tests and `155` core tests.
