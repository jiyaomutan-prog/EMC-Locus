# ADR 0002 - Agent-Owned Local Project Writes

## Status

Accepted.

## Context

The legacy Qt/Python path can still write project records directly into
`projects.sqlite`. That path duplicates business rules around project stage
transitions, contract review, audit evidence, and future synchronization.

The target architecture needs one local write boundary that works offline,
protects ISO 17025 traceability, and can later expose the same behavior to Qt,
HTTP loopback clients, and future web/intranet modules.

## Decision

Project lifecycle writes are owned by `emc-locus-agent`.

For the first vertical slice, the agent writes to local SQLite files only:

- `projects.sqlite` for project records, contract-review items, and audit
  events;
- `sync.sqlite` for pending `sync_operations`.

Write commands must be routed through Rust logic that uses the Rust core project
and quality rules. Each project write must either commit all local effects
needed by the slice or leave no partial project/audit/outbox result.

The initial committed boundary is the CLI surface. The future HTTP loopback API
and Qt client must call the same Rust service path instead of adding new direct
SQLite project writes in Python.

## Consequences

- Python direct-SQLite project writes remain legacy until the Qt migration slice
  removes them from the project workflow.
- Project creation, contract-review item completion, and transition to
  `test_planning` now have a concrete agent-owned path.
- Idempotent `operation_id` values are required for write commands so retries
  can avoid duplicate project, audit, or outbox rows.
- Central synchronization, PostgreSQL, object storage, and conflict merge remain
  out of scope for this decision.
