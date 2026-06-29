# Local Agent

`emc-locus-agent` is the future local runtime boundary for EMC Locus. It should
eventually own local SQLite lifecycle, offline synchronization, health checks,
local API hosting, and object-cache coordination.

The first committed health command is read-only:

```text
cargo run -q -p emc-locus-agent -- health --storage-root storage
```

It returns JSON with:

- agent name;
- package version;
- configured storage root;
- whether the storage root exists;
- repository domains known by the Rust core.

This command is not the final service API. It is the first executable boundary
that lets the project move Python and Qt workflows behind local Rust services
one capability at a time.

## Project Slice Storage Commands

The first storage commands are scoped to the project vertical slice. They manage
only:

- `projects.sqlite`;
- `sync.sqlite`.

Use an explicit storage root and migration root:

```text
cargo run -q -p emc-locus-agent -- storage init --storage-root data\agent --migrations-root storage\sqlite
cargo run -q -p emc-locus-agent -- storage status --storage-root data\agent --migrations-root storage\sqlite
cargo run -q -p emc-locus-agent -- storage verify --storage-root data\agent --migrations-root storage\sqlite
```

`storage init` creates the storage directory if needed and applies missing
project/sync migrations. `storage status` reports whether the databases are
missing, current, invalid, or need migration. `storage verify` fails when a
database is not current or fails SQLite integrity checks.
