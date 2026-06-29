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

## Project Vertical Slice Commands

Version `0.4.4` adds the first write path owned by the local agent. The commands
below operate on initialized `projects.sqlite` and `sync.sqlite` files:

```text
cargo run -q -p emc-locus-agent -- projects create --storage-root data\agent --code CEM-2026-001 --customer-name "Example Customer" --execution-mode accredited --actor quality.lead --reason "Contract accepted" --operation-id op-create-CEM-2026-001 --correlation-id corr-CEM-2026-001 --device-id station-a
cargo run -q -p emc-locus-agent -- projects list --storage-root data\agent
cargo run -q -p emc-locus-agent -- projects get --storage-root data\agent --code CEM-2026-001
cargo run -q -p emc-locus-agent -- projects contract-review --storage-root data\agent --code CEM-2026-001
cargo run -q -p emc-locus-agent -- projects complete-review-item --storage-root data\agent --code CEM-2026-001 --item customer_request_defined --actor quality.lead --comment "Customer request reviewed" --operation-id op-review-001
cargo run -q -p emc-locus-agent -- projects to-test-planning --storage-root data\agent --code CEM-2026-001 --actor quality.lead --reason "Contract review complete" --operation-id op-plan-CEM-2026-001
cargo run -q -p emc-locus-agent -- projects audit-events --storage-root data\agent --code CEM-2026-001
cargo run -q -p emc-locus-agent -- sync outbox --storage-root data\agent
```

Project write commands require an `operation_id`; replaying the same operation
returns the stored result instead of duplicating project, audit, or outbox rows.
The initial agent-created project stage defaults to `contract_review` so the
vertical slice can exercise the contract-review gate and the transition to
`test_planning`.

The accredited review checklist uses the Rust core item slugs:

```text
customer_request_defined
test_method_selected
laboratory_capability_confirmed
equipment_availability_checked
calibration_status_reviewed
impartiality_risks_reviewed
data_retention_agreed
report_requirements_agreed
deviations_recorded
```

This remains a CLI boundary. The HTTP loopback API and Qt migration are the next
steps; they should call the same Rust service path rather than reintroducing
direct Python SQLite writes for project creation, contract review, or project
stage transition.
