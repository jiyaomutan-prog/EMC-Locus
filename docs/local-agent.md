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

## Agent Storage Commands

The first storage commands started with the project vertical slice and now also
prepare the first metrology agent database. They manage:

- `projects.sqlite`;
- `sync.sqlite`;
- `metrology.sqlite`.

Use an explicit storage root and migration root:

```text
cargo run -q -p emc-locus-agent -- storage init --storage-root data\agent --migrations-root storage\sqlite
cargo run -q -p emc-locus-agent -- storage status --storage-root data\agent --migrations-root storage\sqlite
cargo run -q -p emc-locus-agent -- storage verify --storage-root data\agent --migrations-root storage\sqlite
```

`storage init` creates the storage directory if needed and applies missing
project, sync, and metrology migrations. `storage status` reports whether the
databases are missing, current, invalid, or need migration. `storage verify`
fails when a database is not current or fails SQLite integrity checks.

For the project vertical slice, `projects.sqlite` and `sync.sqlite` stay as two
files but must use rollback journal modes so attached-database commits remain
atomic. `storage init` sets `journal_mode=DELETE`; `storage status` reports
`journal_mode` and `atomicity_compatible`; `storage verify` and project commands
refuse incompatible modes such as `wal`.

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

## Metrology Registry Commands

Version `0.6.3` adds the first agent-backed metrology registry commands. They
operate on initialized `metrology.sqlite` storage:

```text
cargo run -q -p emc-locus-agent -- metrology register-instrument --storage-root data\agent --asset-id SA-001 --family receiver --category-code RF-SPECTRUM-ANALYZER --manufacturer Example --model SA9000 --serial-number SN-001 --part-number PN-SA9000 --calibration-requirement required --calibration-period-months 12 --serviceability-status usable --capabilities-json "{\"frequency_hz\":{\"min\":9000,\"max\":3000000000}}" --metrology-notes "Reference spectrum analyzer" --actor metrology.admin --reason "Initial registration" --operation-id op-register-SA-001
cargo run -q -p emc-locus-agent -- metrology list-instruments --storage-root data\agent
cargo run -q -p emc-locus-agent -- metrology get-instrument --storage-root data\agent --asset-id SA-001
cargo run -q -p emc-locus-agent -- metrology record-calibration --storage-root data\agent --asset-id SA-001 --event-id CAL-SA-001-2026 --certificate-reference CERT-SA-001-2026 --calibrated-at 2026-06-30 --due-at 2027-06-30 --provider "Accredited Lab" --decision conforming --uncertainty-summary-json "{\"level_db\":0.6}" --recorded-by metrology.admin --actor metrology.admin --reason "Annual calibration" --operation-id op-cal-SA-001-2026
cargo run -q -p emc-locus-agent -- metrology list-calibrations --storage-root data\agent --asset-id SA-001
cargo run -q -p emc-locus-agent -- metrology status --storage-root data\agent --asset-id SA-001 --checked-on 2026-07-01
cargo run -q -p emc-locus-agent -- metrology readiness --storage-root data\agent --asset-ids SA-001 --execution-mode accredited --checked-on 2026-07-01
cargo run -q -p emc-locus-agent -- metrology set-serviceability --storage-root data\agent --asset-id SA-001 --serviceability-status out_of_service --serviceability-reason "Damaged connector" --actor metrology.admin --reason "Quarantine" --operation-id op-service-SA-001
cargo run -q -p emc-locus-agent -- metrology audit-events --storage-root data\agent --entity-id SA-001
```

The registry validates stable asset identifiers, required identity fields,
calibration requirement, optional calibration period, serviceability state, and
structured capabilities JSON. The calibration-event path validates controlled
decisions (`conforming`, `nonconforming`, `indeterminate`, `not_assessed`),
strict `YYYY-MM-DD` dates, JSON uncertainty summaries, and optional certificate
document manifests. It does not yet write metrology audit or sync outbox rows;
Version `0.6.5` adds audit/outbox/idempotence for the migrated metrology write
paths and a readiness assessment command.

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

## Versioned Loopback API

Version `0.4.5` adds a first local HTTP boundary over the same Rust service
path. The server binds to loopback by default:

```text
cargo run -q -p emc-locus-agent -- serve --storage-root data\agent --migrations-root storage\sqlite --bind 127.0.0.1:8765
```

The implemented routes are:

```text
GET  /api/v1/health
GET  /api/v1/storage/status
POST /api/v1/storage/initialize
POST /api/v1/projects
GET  /api/v1/projects
GET  /api/v1/projects/{code}
GET  /api/v1/projects/{code}/contract-review
POST /api/v1/projects/{code}/contract-review/items/{item}/complete
POST /api/v1/projects/{code}/transitions/to-test-planning
GET  /api/v1/projects/{code}/audit-events
GET  /api/v1/sync/outbox
GET  /api/v1/metrology/instruments
POST /api/v1/metrology/instruments
GET  /api/v1/metrology/instruments/{asset_id}
GET  /api/v1/metrology/instruments/{asset_id}/calibrations
POST /api/v1/metrology/instruments/{asset_id}/calibrations
GET  /api/v1/metrology/instruments/{asset_id}/status?checked_on=YYYY-MM-DD
POST /api/v1/metrology/instruments/{asset_id}/serviceability
POST /api/v1/metrology/readiness
GET  /api/v1/metrology/instruments/{asset_id}/audit-events
```

`GET /api/v1/storage/status` returns the project/sync/metrology storage status
used by Qt to display connected, unavailable, storage-not-initialized,
migration-required, and integrity-error states without opening SQLite directly.

The API is intentionally local and narrow. It does not expose central
synchronization, PostgreSQL, object storage, instrument control, or acquisition
runtime features.

For metrology, `POST /api/v1/metrology/instruments` accepts the same fields as
the `metrology register-instrument` command, with `capabilities` accepted as a
structured JSON object or array. `GET /api/v1/metrology/instruments` returns the
registry list, and `GET /api/v1/metrology/instruments/{asset_id}` returns one
instrument detail with its latest calibration summary when present.

Version `0.6.4` adds calibration-event routes. `POST
/api/v1/metrology/instruments/{asset_id}/calibrations` records one calibration
event and optional certificate document manifest. `GET
/api/v1/metrology/instruments/{asset_id}/calibrations` returns the event
history. `GET /api/v1/metrology/instruments/{asset_id}/status` computes the
status for a required `checked_on=YYYY-MM-DD` query date instead of trusting a
stored import status.

Version `0.6.5` makes migrated metrology write routes require operation context
(`actor`, `reason`, and `operation_id`, with optional `correlation_id` and
`device_id`). Successful writes add a metrology audit row and a pending
`sync_operations` outbox row atomically. `POST /api/v1/metrology/readiness`
returns ready/not-ready, per-instrument statuses, blocking issues, and warnings.

## Python And Qt Client Path

Version `0.4.6` adds `emc_locus.local_agent_client.LocalAgentClient`, a thin
standard-library HTTP client for the loopback API. Python/Qt project actions can
now pass `agent_url` to route these writes through the agent:

- project creation;
- contract-review item completion;
- transition to `test_planning` through `advance_project_stage`.

Version `0.5.5` also routes migrated project reads through the agent when
`agent_url` is configured:

- project list and detail;
- contract-review status;
- project audit events;
- pending sync outbox.

The service-planning table is still a legacy SQLite-backed surface until a
dedicated agent route exists.

The Qt console accepts:

```text
py apps\qt-console\main.py --projects-db data\agent\projects.sqlite --agent-url http://127.0.0.1:8765
```

With `--agent-url`, the console header shows the local-agent state and the
agent-backed project forms are submitted through a Qt worker so the main UI
thread remains responsive.

Version `0.6.6` also routes the temporary Qt/Python metrology surface through
the agent when `agent_url` is configured:

- instrument list rows;
- computed calibration status per instrument;
- a simple readiness table using `POST /api/v1/metrology/readiness`;
- instrument registration;
- calibration-event recording, including certificate document manifests;
- serviceability changes.

The remaining standalone instrument-document form, service planning, test
categories, measurement data, updates, and runtime actions remain legacy direct
SQLite until their own migration slices.

Version `0.6.7` adds the metrology historical migration and E2E validation
layer: legacy calibration rows are backfilled into calibration events, and a
real loopback HTTP test verifies readiness, serviceability, idempotence,
restart persistence, audit, and outbox for the migrated metrology slice.
