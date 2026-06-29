# ADR 0001 - Rust Application Services As The Write Boundary

## Status

Accepted.

## Context

EMC Locus currently has strong Rust domain primitives, but several local UI and
Python actions still write directly to SQLite. This was useful for fast
prototype progress, but it creates drift risk for project gates, metrology
rules, update validation, dataset retention, and future synchronization.

## Decision

All critical business writes must converge toward Rust application services.
The services own command validation, query contracts, authorization hooks,
audit intent, idempotency metadata, and sync-operation creation. Python, Qt,
web apps, the local agent, and the station runtime should call this boundary
instead of duplicating invariants.

## Consequences

- Python write actions remain allowed only as transitional adapters.
- New domain rules should be added to Rust first.
- SQLite migrations and adapters remain compatible while Rust storage adapters
  and PyO3/local APIs are introduced.
- UI layers must not become the source of execution, metrology, planning, sync,
  or reporting invariants.
