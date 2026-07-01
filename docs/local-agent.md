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
prepare the metrology and test-definition agent databases. They manage:

- `projects.sqlite`;
- `sync.sqlite`;
- `metrology.sqlite`;
- `test_definitions.sqlite`.

Use an explicit storage root and migration root:

```text
cargo run -q -p emc-locus-agent -- storage init --storage-root data\agent --migrations-root storage\sqlite
cargo run -q -p emc-locus-agent -- storage status --storage-root data\agent --migrations-root storage\sqlite
cargo run -q -p emc-locus-agent -- storage verify --storage-root data\agent --migrations-root storage\sqlite
```

`storage init` creates the storage directory if needed and applies missing
project, sync, metrology, and test-definition migrations. `storage status`
reports whether the databases are missing, current, invalid, or need migration.
`storage verify` fails when a database is not current or fails SQLite integrity
checks.

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

The server also serves the prebuilt LAB CONSOLE bundle under `/lab/`:

```text
cargo run -q -p emc-locus-agent -- serve --storage-root data\agent --migrations-root storage\sqlite --bind 127.0.0.1:8765 --lab-console-dist apps\lab-console\dist
```

`GET /` redirects to `/lab/`. If `index.html` is missing, LAB CONSOLE requests
return a structured `lab_console_build_missing` error while `/api/v1/...`
routes remain available. The same Rust process serves LAB CONSOLE and the API;
there is no production Node server.

The normal release launcher is:

```powershell
.\scripts\start-lab.ps1
.\scripts\start-lab.ps1 -SeedDemo
.\scripts\start-full-demo.ps1
```

`start-lab` does not require Node when the committed `apps/lab-console/dist`
bundle is present. Developers can pass `-Rebuild` or run
`.\scripts\build-lab.ps1` on a machine with npm or pnpm.

The implemented routes are:

```text
GET  /
GET  /lab/
GET  /lab/assets/{asset}
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
GET  /api/v1/projects/{code}/test-executions
GET  /api/v1/sync/outbox
GET  /api/v1/documents
POST /api/v1/documents
GET  /api/v1/documents/{document_id}
GET  /api/v1/documents/{document_id}/audit-events
GET  /api/v1/test-templates
POST /api/v1/test-templates
POST /api/v1/test-template-definitions/validate
GET  /api/v1/test-templates/{template_id}
POST /api/v1/test-templates/{template_id}/clone
GET  /api/v1/test-templates/{template_id}/revisions
GET  /api/v1/test-templates/{template_id}/revisions/{revision_id}
PUT  /api/v1/test-templates/{template_id}/revisions/{revision_id}/definition
POST /api/v1/test-templates/{template_id}/revisions
POST /api/v1/test-templates/{template_id}/revisions/{revision_id}/transitions/submit-for-review
POST /api/v1/test-templates/{template_id}/revisions/{revision_id}/transitions/approve
GET  /api/v1/test-templates/{template_id}/audit-events
POST /api/v1/test-executions/simulated-emc
GET  /api/v1/test-executions/{attempt_id}
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

The API is intentionally local and narrow. It backs LAB CONSOLE Template Studio
v1 for test-template authoring, but it does not expose central synchronization,
PostgreSQL, object storage, instrument control, acquisition runtime features,
campaign instantiation, or reporting.

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

Version `0.7.0` promotes that metrology path to the current vertical-slice
baseline. The remaining direct-SQLite Qt forms are outside this baseline and are
tracked as future slices, starting with standalone metrology documents and
richer execution/method evidence.

Version `0.8.0` adds that first simulated EMC execution workflow. `POST
/api/v1/test-executions/simulated-emc` persists the operator launch attempt,
computes metrology readiness for the test context, stores a structured refusal
when equipment is not ready, or stores a deterministic conducted-emission
simulation result when the preflight passes. The workflow also writes a project
audit event and a pending sync outbox operation for entity type
`simulated_test_execution`. Qt exposes this as a single operator form instead
of a dispersed CRUD surface.

Version `0.8.1` adds the first shared attached-document registry. `POST
/api/v1/documents` registers document metadata with owner surface/entity,
classification, storage URI, checksum, revision, applicability, and
confidentiality. It does not upload file bytes. Successful writes persist an
`attached_documents` row, a `document_audit_events` row, and a pending outbox
operation with entity type `attached_document`.

Version `0.8.3` adds the first agent-owned test-template draft workflow. `POST
/api/v1/test-templates` creates one controlled draft test template in
`test_definitions.sqlite`, validates its category and structured definition
blocks, requires referenced method revisions to be approved, writes
`test_template_audit_events`, and emits a `test_definitions` outbox operation.
The slice does not yet approve templates, instantiate campaign tests, or
execute acquisition/post-processing.

Version `0.8.4` adds the first controlled template lifecycle transitions.
`POST /api/v1/test-templates/{template_id}/transitions/submit-for-review`
moves a draft template to `under_review`, and
`POST /api/v1/test-templates/{template_id}/transitions/approve` moves an
under-review template to `approved`. Direct approval from `draft` is refused
with `test_template_transition_not_allowed`. Successful transitions are
idempotent by `operation_id`, update the template status, append
`test_template_audit_events`, and emit `test_definitions` outbox operations.

Version `0.9.0` replaces the 0.8.x template storage and API. Templates now have
a stable identity plus revisioned content. `POST /api/v1/test-templates`
creates identity plus first draft revision. Draft definitions are replaced with
`PUT /api/v1/test-templates/{template_id}/revisions/{revision_id}/definition`
and require `expected_definition_checksum`. Submitted and approved revisions are
immutable. New work derives from an approved source through
`POST /api/v1/test-templates/{template_id}/revisions`. Audit events now carry
the template id, revision id, actor, reason, old/new revision ids, old/new
definition checksums, operation id, device id, and correlation id. The sync
outbox entity type is `test_template_revision`.

The simulated execution path checks this new model when an operator launch uses
a stored template id as `test_method_reference`: known template identities must
have a `current_approved_revision_id`, otherwise
`POST /api/v1/test-executions/simulated-emc` returns
`test_execution_template_not_approved`. When the launch is accepted, the stored
execution and response include `test_template_revision` with `template_id`,
`revision_id`, and `definition_checksum`. This is still not a campaign
execution-package binding, variable-resolution workflow, or copied definition
snapshot.

Version `0.9.1` hardens this template workflow without adding Template Studio
or a new runtime. Draft definition replacement now uses a SQL compare-and-swap
on `definition_checksum`; lifecycle transitions use a SQL compare-and-swap on
the current status; the API aggregate exposes `current_approved_revision`,
`latest_revision`, and `active_draft_revision`; SQLite enforces one active
draft per template identity; and approving a newer revision supersedes older
approved revisions in the same transaction with audit/outbox evidence.

Version `0.9.2` fixes the Windows launcher layer. `scripts/start-agent-qt.ps1`
now builds the agent executable, initializes `data\local-agent`, starts the
agent with relative arguments from the repository working directory, waits for
`/api/v1/health`, verifies the returned `storage_root`, and refuses to reuse an
agent on the same port when it points at another storage root. Qt is launched
only after positive health. `scripts/start-qt-demo.ps1` has explicit `-Mode
Static`, `-Mode Agent`, and `-Mode Auto` behavior. Launcher-owned processes are
recorded under `logs\launchers\runtime` and can be stopped with
`scripts/stop-agent.ps1` or `scripts/stop-all.ps1` without killing unrelated
Python, Cargo, or agent processes.

Version `0.10.0` adds the normal LAB launcher layer. `scripts/start-lab.ps1`
verifies the committed LAB CONSOLE build, starts or reuses the compatible local
agent, waits for `/api/v1/health` and `/lab/`, then opens the browser unless
`-NoBrowser` is passed. `scripts/seed-lab-demo.ps1` creates demonstration
templates through the public API only. `scripts/start-full-demo.ps1` opens LAB
CONSOLE and starts TEST CONSOLE Qt against the same `data\local-agent` storage.
