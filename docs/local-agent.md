# Local Agent

`emc-locus-agent` is the future local runtime boundary for EMC Locus. It should
eventually own local SQLite lifecycle, offline synchronization, health checks,
local API hosting, and object-cache coordination.

The first committed command is deliberately read-only:

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
