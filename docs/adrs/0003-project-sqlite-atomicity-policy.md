# ADR 0003 - Project SQLite Atomicity Policy

## Status

Accepted.

## Context

The project vertical slice writes one logical operation across two local SQLite
files:

- `projects.sqlite` for project records, contract-review state, and audit
  events;
- `sync.sqlite` for the pending `sync_operations` outbox.

The two-file layout keeps local repositories independently inspectable and
exportable, but the project slice still needs all-or-nothing commits for
project/audit/outbox changes. SQLite can make transactions across attached
database files atomic when rollback-journal modes are used. WAL does not provide
the same attached-database atomicity guarantee for this slice.

## Decision

Keep `projects.sqlite` and `sync.sqlite` split for now.

For the project vertical slice, the agent requires rollback-journal modes that
are compatible with attached-database atomic commits:

- `delete`;
- `truncate`;
- `persist`.

`storage init` sets the project and sync databases to `journal_mode=DELETE`.
`storage status` reports each database journal mode and whether it is compatible
with this policy. `storage verify` and project commands reject incompatible
modes such as `wal`, `memory`, or `off`.

## Consequences

- Field/offline repositories remain separate files and remain easy to export or
  inspect.
- WAL is intentionally unavailable for the project/sync vertical slice until the
  outbox is moved into `projects.sqlite` or a different atomic write boundary is
  designed.
- Operators can repair a compatible local store by running `storage init` again,
  which reapplies the required rollback journal mode before migrations.
- Future central synchronization, PostgreSQL, and object storage designs remain
  out of scope for this decision.
