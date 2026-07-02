# Changelog

All notable changes to EMC Locus are recorded here.

The project follows Semantic Versioning once public releases begin. During the
foundation phase, `0.x.y` versions may still evolve quickly, but every committed
change should remain traceable through Git history, session logs, and this file.

## [Unreleased]

### Fixed

- Normalized optional service-schedule notes in the Python action/repository
  path so callers cannot leak `NULL` into the non-null planning notes column.
- Added regression coverage and documentation for service-schedule positive
  duration validation so repository callers cannot rely on zero-length planning
  blocks being accepted.
- Validated and normalized service-schedule read filters in the Python project
  repository so blank project filters and unknown status filters no longer
  fail as silent empty schedule lists.
- Tightened local service-planning timestamp validation so schedule rows must
  use canonical `YYYY-MM-DDTHH:MM` local date-times instead of Python-accepted
  ISO variants such as week-date or compact forms.
- Moved service-schedule status validation into the project repository so
  direct Python callers cannot bypass the allowed planning status vocabulary.
- Moved required service-schedule field validation into the project repository
  so direct Python callers cannot persist blank operator, location, title, or
  equipment context.
- Normalized optional service-schedule category and method references in the
  Python action/repository path so blank values persist as absent references
  instead of empty traceability fields.
- Added an explicit project-existence guard to repository service-schedule
  inserts so direct Python callers receive a controlled planning error before
  any SQLite foreign-key failure.
- Rejected blank service-schedule item codes on repository status updates so
  direct Python callers cannot turn an operator mistake into a silent no-op.

## [0.10.0] - 2026-07-01

### Added

- Added `apps/lab-console`, a React/TypeScript/Vite LAB CONSOLE application
  for the first real Template Studio workflow: template library, create/clone,
  structured draft editing, server validation, checksum save, submit, approve,
  derive, revision history, audit, and system status.
- Added a versioned production build under `apps/lab-console/dist` so release
  launch does not require Node on the user machine.
- Added Vitest/React Testing Library unit coverage and Playwright E2E coverage
  for the template workflow against a real local agent.
- Added optional local-agent serving for a built LAB CONSOLE under `/lab/`,
  with root redirect, asset serving, SPA fallback, explicit missing-build
  errors, and traversal-safe asset paths.
- Added `POST /api/v1/test-template-definitions/validate` so clients can
  validate and canonicalize typed template definitions before a draft write.
- Added controlled cloning of an approved test-template revision into a new
  draft template identity with audit/outbox evidence and idempotent replay.
- Added `dimensionless=true` for numeric template variables that intentionally
  have no engineering unit.
- Added `scripts/start-lab`, `scripts/start-full-demo`, `scripts/build-lab`,
  and `scripts/seed-lab-demo` with BAT wrappers.
- Added a strict JSON Qt demo fixture under `apps/qt-console/demo/bootstrap.json`.

### Changed

- Replaced the provisional `apps/gui-shell` static prototype with LAB CONSOLE.
- Decoupled TEST CONSOLE Qt from LAB web bootstrap JavaScript; static Qt mode
  now loads strict JSON directly.
- Reworked launcher smoke coverage around `/api/v1/health`, `/`, `/lab/`,
  built assets, API seeding, and Qt static mode.
- Updated CI to run npm install/build/test, Playwright E2E, `dist` consistency,
  and the LAB launcher smoke.
- Bumped synchronized Rust, Python, and frontend versions to `0.10.0`.

### Fixed

- Fixed test-template clone idempotent replay so the agent checks the existing
  operation fingerprint before rejecting the already-created clone identity.

## [0.9.2] - 2026-07-01

### Added

- Added shared PowerShell launcher utilities for command checks, HTTP readiness
  waits, agent storage validation, PID state, and safe stop behavior.
- Added stop scripts for prototype, agent, and all EMC Locus launcher-owned
  processes.
- Added `scripts/smoke-launchers.ps1` to execute real launcher smoke tests,
  including paths with spaces.
- Added `logs/` to `.gitignore` because launcher logs and runtime PID state are
  local machine artifacts.

### Changed

- Fixed launcher process argument handling so repository paths with spaces are
  not split by `Start-Process -ArgumentList`.
- `start-proto.ps1` now verifies `py`, validates the static shell entry point,
  waits for HTTP 200, and opens the browser only after readiness.
- `start-agent-qt.ps1` now builds/starts the agent executable directly, verifies
  `/api/v1/health`, validates the returned storage root, refuses incompatible
  existing agents, and never launches Qt before positive agent health.
- `start-qt-demo.ps1` now supports explicit `-Mode Static`, `-Mode Agent`, and
  `-Mode Auto` behavior.
- BAT wrappers now preserve the window on errors and return the PowerShell exit
  code.
- Bumped the synchronized Rust/Python software version to `0.9.2`.

## [0.9.1] - 2026-07-01

### Added

- Added `storage/sqlite/projects/0005_simulated_execution_template_revision.sql`
  so simulated EMC execution attempts can persist the approved test-template
  revision selected at launch.
- Simulated EMC execution responses now expose an optional
  `test_template_revision` object with template id, revision id, and definition
  checksum when `test_method_reference` matches an approved stored template.
- Added `storage/sqlite/test_definitions/0005_single_active_draft.sql` with a
  partial unique index enforcing one active draft revision per template.
- Added Windows launchers for the static prototype, Qt demo, and local
  agent-plus-Qt workflow under `scripts/`, with logs in `logs/launchers`.
- Added a Python smoke test for the GUI shell bootstrap and relative static
  asset paths.

### Changed

- Test-template aggregate DTOs now expose `current_approved_revision`,
  `latest_revision`, and `active_draft_revision` instead of the ambiguous
  `current_revision`.
- Draft definition replacement and lifecycle transitions now use SQL-level
  compare-and-swap guards for checksum/status concurrency.
- Approving a newer template revision now supersedes older approved revisions
  for the same template in the same transaction, with audit and outbox records.
- `apps/gui-shell/bootstrap.js` remains a browser script but now carries a
  strict JSON payload parseable by the Qt Python loader.
- Bumped the synchronized Rust/Python software version to `0.9.1`.

## [0.9.0] - 2026-07-01

### Added

- Added a typed `emc-locus-core` test-template definition aggregate with
  explicit variables, constraints, lock policies, instrumentation slots,
  calibration requirements, sequence steps, branch rules, limits,
  post-processing definitions, revision statuses, canonical JSON, and
  SHA-256 definition checksums.
- Added `storage/sqlite/test_definitions/0004_template_revision_aggregate.sql`
  with `test_template_identities`, `test_template_revisions`, and
  `test_template_audit_events`.
- Added local-agent routes for revision history, revision detail, draft
  definition replacement with `expected_definition_checksum`, draft derivation
  from an approved source revision, and transitions on explicit revision ids.
- Added Python local-agent client methods for the 0.9.0 revisioned template API.
- Added real HTTP E2E coverage for create, edit, submit, approve, derive,
  audit/outbox, restart, and re-read.

### Changed

- Replaced the 0.8.x one-row `test_templates` runtime model. No dual-read,
  dual-write, or legacy DTO remains in the new runtime.
- Test-template creation now accepts method links only when the referenced
  method revision is approved.
- Simulated EMC executions now reject references to stored test-template
  identities unless a current approved revision exists.
- Bumped the synchronized Rust software version to `0.9.0`.

### Fixed

- Draft updates now use optimistic concurrency based on definition checksum, so
  stale edits are refused with `test_template_definition_checksum_mismatch`.
- Submitted and approved template revisions are immutable and reject definition
  replacement with `test_template_revision_immutable`.

## [0.8.4] - 2026-06-30

### Added

- Added agent-owned test-template lifecycle transitions for submitting draft
  templates to review and approving under-review templates.
- Added local API routes
  `/api/v1/test-templates/{template_id}/transitions/submit-for-review` and
  `/api/v1/test-templates/{template_id}/transitions/approve`.
- Added audit and sync outbox evidence for
  `test_template_submitted_for_review` and `test_template_approved`
  operations.
- Added Python `LocalAgentClient` methods for test-template submit and approve
  transitions.

### Changed

- Bumped the synchronized Rust/Python software version to `0.8.4`.

## [0.8.3] - 2026-06-30

### Added

- Added test-definition migration `0003_test_templates.sql` for controlled draft
  test templates and `test_template_audit_events`.
- Added agent-owned test-template draft creation, list, detail, and audit routes
  under `/api/v1/test-templates`.
- Added audit and sync outbox evidence for `test_definitions` operations with
  entity type `test_template`.
- Added Python `LocalAgentClient` methods for test-template creation and reads.
- Added `docs/test-template-api.md`.

### Changed

- `emc-locus-agent storage init/status/verify` now includes
  `test_definitions.sqlite` alongside projects, sync, and metrology.
- Bumped the synchronized Rust/Python software version to `0.8.3`.

## [0.8.2] - 2026-06-30

### Changed

- Froze the GUI direction around two product consoles: LAB CONSOLE for web
  laboratory management and TEST CONSOLE for Qt local/offline execution.
- Reframed metrology as a controlled LAB domain and TEST readiness dependency,
  not a third GUI product.
- Strengthened GUI, template, execution-definition, people/roles, and
  competence documentation before adding another runtime vertical.
- Reworked `apps/gui-shell` as a static LAB CONSOLE information-architecture
  prototype with hierarchical navigation, LAB-to-TEST handoff points, and
  explicit guardrails against fake runtime behavior.

### Added

- Added release documentation for the GUI/template backbone clarification slice.

## [0.8.1] - 2026-06-30

### Added

- Added the first shared attached-document registry behind the Rust local
  agent, with document metadata, owner surface/entity, storage URI, checksum,
  revision, applicability, confidentiality, audit, and sync outbox evidence.
- Added project migration `0004_attached_documents.sql` for
  `attached_documents` and `document_audit_events`.
- Added local API routes for registering documents, listing documents,
  filtering by owner, reading document detail, and reading document audit
  events.
- Added Python local-agent client methods for document registration, document
  reads, owner-filtered lists, and document audit reads.
- Added ADR 0003 for the three Locus surfaces and the agent-owned document
  registry.

### Changed

- Bumped the synchronized Rust/Python software version to `0.8.1`.
- Updated the static GUI shell and architecture documentation from a two-surface
  console split to the three-product target: Locus Metrology, Locus Lab
  Management, and Locus Test Station.

## [0.8.0] - 2026-06-30

### Added

- Added the first simulated EMC test execution workflow through the local Rust
  agent: launch attempt, metrology preflight, structured refusal, persisted
  completed result, project audit, and sync outbox evidence.
- Added project migration `0003_simulated_test_executions.sql` for execution
  attempts and per-instrument readiness snapshots.
- Added dedicated local API routes for simulated EMC execution creation,
  execution detail reads, and project execution history.
- Added Python local-agent client, GUI action, Qt form contract, and Qt table
  support for the single operator-centered simulated EMC workflow.
- Added real HTTP E2E coverage for refused preflight, calibration correction,
  authorized execution, persisted result, replay, conflict detection, restart
  persistence, audit, and outbox visibility.

### Changed

- Bumped the synchronized Rust/Python software version to `0.8.0`.
- Extended metrology readiness issues with a business `dimension` so execution
  refusals can explain serviceability, missing evidence, calibration validity,
  and nonconformance separately.

### Fixed

- Local service-planning actions now reject multi-day schedule items even when
  every date is a business day, keeping each planned execution item as a single
  intra-day laboratory block.

## [0.7.0] - 2026-06-30

### Added

- Consolidated the first agent-backed metrology readiness vertical slice as the
  new baseline release.
- Added release documentation summarizing the migrated routes, Qt/Python agent
  surface, SQLite structures, validation commands, remaining limits, and next
  recommended vertical slice.

### Changed

- Bumped the synchronized Rust/Python software version to `0.7.0`.
- Marked the next recommended implementation slice as a first simulated EMC
  test that consumes metrology readiness before execution.

## [0.6.7] - 2026-06-30

### Added

- Added metrology migration `0007_legacy_calibration_events.sql` to backfill
  legacy `calibration_records` into `calibration_events` without deleting the
  original records.
- Added migration coverage for the legacy calibration backfill, reserved
  availability conversion, idempotent migration re-runs, and certificate
  manifest preservation.
- Added a real HTTP server metrology E2E test covering storage initialization,
  instrument registration, readiness refusal without calibration, calibration
  with certificate manifest, due-soon warning, out-of-service blocking,
  return-to-service, idempotent replay, replay conflict, restart persistence,
  audit reads, and sync outbox reads.

### Changed

- Bumped the synchronized Rust/Python software version to `0.6.7`.

## [0.6.6] - 2026-06-30

### Added

- Added Python local-agent client methods for metrology instrument reads,
  calibration reads, computed status, readiness, and metrology audit events.
- Added agent-backed Qt/Python metrology bootstrap loading when `agent_url` is
  configured, including instrument table rows, certificate manifest rows, and a
  simple readiness table sourced from `POST /api/v1/metrology/readiness`.
- Added agent-backed Python/Qt write paths for instrument registration,
  calibration-event recording, and serviceability changes without opening
  `metrology.sqlite` directly when `agent_url` is configured.
- Added certificate document-manifest forwarding for agent-backed calibration
  events from existing Qt/Python certificate reference fields.

### Changed

- Qt treats the local agent as the writable metrology surface for migrated
  forms while keeping legacy document attachment disabled unless a metrology
  SQLite repository is explicitly configured.
- Bumped the synchronized Rust/Python software version to `0.6.6`.

## [0.6.5] - 2026-06-30

### Added

- Added metrology migration `0006_metrology_audit_events.sql` for
  per-entity audit events with operation fingerprint evidence.
- Added operation context, strict replay checks, metrology audit rows, and
  `sync_operations` outbox rows for agent-backed instrument registration and
  calibration-event recording.
- Added agent-backed serviceability changes with audit/outbox evidence.
- Added structured metrology readiness assessment through CLI and
  `POST /api/v1/metrology/readiness`.
- Added metrology audit reads through CLI and
  `GET /api/v1/metrology/instruments/{asset_id}/audit-events`.

### Changed

- Metrology write routes now require `actor`, `reason`, and `operation_id`.
- Bumped the synchronized Rust/Python software version to `0.6.5`.

## [0.6.4] - 2026-06-30

### Added

- Added metrology migration `0005_calibration_events_and_status.sql` with
  `calibration_events`, per-instrument `calibration_due_warning_days`, and
  repository metadata for computed calibration status.
- Added Rust agent/service/repository/DTO support for recording calibration
  events with controlled decisions, uncertainty summaries, optional traceability
  references, and certificate document manifests.
- Added metrology CLI actions for `record-calibration`, `list-calibrations`, and
  `status`.
- Added loopback API routes for
  `POST/GET /api/v1/metrology/instruments/{asset_id}/calibrations` and
  `GET /api/v1/metrology/instruments/{asset_id}/status?checked_on=YYYY-MM-DD`.
- Added computed calibration status results for `valid`, `due_soon`, `expired`,
  `missing`, `not_required`, and `nonconforming`.

### Changed

- Replaced the hard-coded due-soon threshold in the Rust metrology domain with
  an explicit default constant and a configurable status calculation.
- Bumped the synchronized Rust/Python software version to `0.6.4`.

## [0.6.3] - 2026-06-30

### Added

- Added Rust metrology agent actions for registering, listing, and reading
  instruments through the same command-dispatch pattern used by the project
  slice.
- Added loopback API routes for `GET/POST /api/v1/metrology/instruments` and
  `GET /api/v1/metrology/instruments/{asset_id}`.
- Added agent-backed instrument registration validation for stable asset ids,
  required fields, calibration requirement, serviceability state, and structured
  capabilities JSON.
- Added `docs/metrology-api.md` and updated local-agent documentation for the
  first metrology routes and storage lifecycle.

### Changed

- `storage init/status/verify` now includes `metrology.sqlite` alongside the
  project/sync databases so the first metrology API routes work after a normal
  local storage initialization.
- Bumped the synchronized Rust/Python software version to `0.6.3`.

## [0.6.2] - 2026-06-30

### Added

- Added Rust metrology DTOs for instrument lists/details and latest calibration
  records, using Serde instead of manual JSON assembly.
- Added a Rust metrology repository boundary that opens `metrology.sqlite`,
  validates required serviceability schema columns, loads instruments, and reads
  each instrument's latest calibration record.
- Added a thin Rust metrology service that renders the first instrument list/get
  JSON contracts and tests the serviceability DTO contract.

### Changed

- Bumped the synchronized Rust/Python software version to `0.6.2` for the
  metrology DTO/repository tranche.

## [0.6.1] - 2026-06-30

### Added

- Added a non-destructive metrology migration for instrument
  `serviceability_status`, `serviceability_reason`,
  `serviceability_updated_at`, and `legacy_availability`, preserving the legacy
  `availability` column while converting `reserved` to serviceable by default.
- Added Rust `InstrumentServiceability` and updated equipment readiness so
  serviceability, not planning reservation, drives out-of-service blocking.
- Added Python and Qt action support for setting instrument serviceability, plus
  GUI/bootstrap columns that show service state separately from planning
  availability.

### Changed

- Legacy `set-instrument-availability` remains available, but now synchronizes
  serviceability through a compatibility path so older out-of-service actions
  keep their safety effect.
- Static GUI fallback data and metrology tables now use the same service/planning
  instrument row contract as the Qt console.

### Fixed

- Processing graph execution records now reject result artifacts produced from a
  different graph reference or revision, preserving traceability between the
  execution evidence and the revisioned graph instance.
- Python measurement-data writes for processing graph executions now reject
  output artifact counts that do not match the artifacts persisted for the same
  graph instance.
- Failed processing graph executions are now covered by the same Python
  persisted-artifact count invariant as completed executions.
- Python measurement-data writes for processing graph instance artifacts now
  reject malformed output signal references and raw-lineage JSON before
  persisting traceability evidence.
- Python measurement-data writes for processing graph instances and executions
  now reject blank software-version evidence before persistence.

## [0.6.0] - 2026-06-29

### Changed

- Finalized the hardened agent-backed project vertical slice by consolidating
  the 0.5.1 through 0.5.6 hardening tranches into the 0.6.0 baseline:
  strict idempotency fingerprints, Serde DTO response rendering, split Rust
  project modules, enforced project/sync SQLite atomicity policy, Qt/Python
  project reads through the local agent, and GitHub Actions CI.
- Added the 0.6.0 release note under `docs/releases/`.

## [0.5.6] - 2026-06-29

### Added

- Added a minimal GitHub Actions CI workflow for `push` to `main` and
  `pull_request`, with read-only repository permissions and the same Rust,
  Python, SQLite migration, JavaScript, release-consistency, and whitespace
  checks used locally.

## [0.5.5] - 2026-06-29

### Changed

- Added Python local-agent client read methods for project list/detail,
  contract-review status, project audit events, and sync outbox.
- Migrated Qt/Python project bootstrap reads to use the local agent whenever
  `agent_url` is configured, while keeping non-project repositories on their
  existing legacy SQLite paths.
- Refreshed agent-backed project forms from agent data instead of requiring a
  local `projects.sqlite` path.

## [0.5.4] - 2026-06-29

### Changed

- Enforced the project/sync multi-SQLite atomicity policy by initializing the
  project slice with rollback journal mode and rejecting incompatible journal
  modes such as WAL before project commands use attached databases.
- Extended storage status JSON with `journal_mode` and
  `atomicity_compatible` fields for the project and sync databases.
- Documented the SQLite atomicity decision in ADR 0003.

## [0.5.3] - 2026-06-29

### Changed

- Split the project agent internals into explicit modules: CLI/API orchestration
  in `project_agent.rs`, service workflow logic in `project_service.rs`, Serde
  response DTOs in `project_dto.rs`, and SQLite project/sync persistence in
  `project_repository.rs`.
- Preserved the existing project vertical-slice behavior and tests while
  reducing `project_agent.rs` to argument parsing and command dispatch.

## [0.5.2] - 2026-06-29

### Changed

- Replaced hand-built agent response JSON with explicit Serde DTO rendering for
  health, storage reports, structured errors, project results, contract-review
  status, audit events, and sync outbox listings.
- Replaced canonical project-slice payload JSON builders with `serde_json`
  values so escaping and null handling no longer depend on custom string
  assembly.

## [0.5.1] - 2026-06-29

### Changed

- Hardened agent project idempotence so replay is accepted only when the
  existing operation and incoming command share the same canonical fingerprint:
  project entity, operation kind, base revision, actor/device/correlation
  metadata, and canonical payload.

### Added

- Rust tests for identical operation replay and mismatched-payload replay
  rejection.
- Python client test proving idempotency conflicts map to structured
  `LocalAgentError` details.

## [0.5.0] - 2026-06-29

### Added

- First complete agent-backed project vertical slice:
  - local project/sync storage initialization;
  - versioned loopback API;
  - storage status route for Qt/automation health display;
  - project create/list/read;
  - contract-review status and item completion;
  - transition to `test_planning`;
  - audit event inspection;
  - pending sync outbox inspection;
  - real HTTP E2E idempotence and restart/persistence coverage;
  - Python local-agent client;
  - Qt project forms for creation, contract review, and planning transition when
    `--agent-url` is configured;
  - visible Qt local-agent status and worker-backed project form submission.

### Added

- Qt operator form for project transition to test planning, routed through the
  Python action layer and local agent when `--agent-url` is configured.
- Automated real HTTP server E2E test for the project vertical slice, including
  storage initialization, contract-review refusal, review completion, transition
  to planning, audit/outbox checks, restart, and persistence verification.
- Thin Python `LocalAgentClient` for the loopback API, with structured error
  handling and UUID-backed operation identifiers.
- Optional `agent_url` project action path for Python/Qt project creation,
  contract-review item completion, and transition to test planning.
- Versioned loopback HTTP API for the local agent project slice, served by
  `emc-locus-agent serve` on `127.0.0.1` by default.
- API routes for health, storage initialization, project creation/list/read,
  contract-review status, review item completion, transition to test planning,
  project audit events, and pending sync outbox inspection.
- Agent-backed project CLI commands for the first vertical slice: create/list/get
  projects, read contract review, complete review items, transition to test
  planning, inspect project audit events, and inspect pending sync outbox
  operations.
- Transactional Rust writes from `emc-locus-agent` across `projects.sqlite` and
  attached `sync.sqlite`, producing project audit events, deterministic
  revisions, idempotent operation replay, and local outbox records.
- Rust toolchain validation now includes the Clippy component, and existing
  Clippy warnings in the core crates were cleared.
- Contract-review stage gate before test planning.
- Authorized contract-review deviation event for incomplete checklists.
- Rust tests for complete checklist, incomplete checklist, authorized deviation,
  project mismatch, and invalid-stage behavior.
- Public BAT-EMC/Nexio feature baseline for product positioning.
- Offline-first architecture direction with split, synchronizable repositories.
- Instrument-control architecture direction for transport-neutral drivers.
- Rust policy models for execution mode, connectivity, repository domains,
  instrument transports, and update policy.
- Public DewesoftX/openDAQ concept baseline for CEM-oriented time-domain
  acquisition and signal processing.
- Signal acquisition architecture for time series, FFT, temporal processing,
  multi-signal math, and synchronized multi-DAQ setups.
- Rust primitives for measurement axes, DAQ interfaces, synchronization methods,
  signal-processing operations, and CEM time-domain test families.
- Consolidated product objectives and non-objectives.
- Rust core crate module split with documented module boundaries.
- Updated recurring development backlog around metrology, simulated DAQ, local
  repository snapshots, and migration work.
- Rust metrology registry primitives for instrument identity, availability,
  calibration records, calibration status, and pre-run readiness checks.
- Tests for accredited, non-accredited, and investigation equipment readiness
  behavior.
- Versioned SQLite migration domains for metrology, projects, test definitions,
  measurement data, and update catalog repositories.
- Python migration validation helper for filename, version, and executable SQL
  checks.
- Rust simulated DAQ source with deterministic inrush time-series fixture.
- Rust signal dataset, sample-rate, channel metadata, processing graph, and
  raw-lineage primitives for FFT/channel-math workflows.
- Rust measurement-run planning model that consumes metrology readiness reports
  and blocks only on blocking pre-run issues.
- Rust field repository package model for offline snapshots, signatures,
  schema-version compatibility, and complete domain coverage.
- Rust simulated instrument runtime with command messages, deterministic
  responses, supported-transport checks, target checks, and ordered observation
  logs.
- Rust dataset evidence model linking accepted measurement-run plans to command
  observations, immutable raw dataset records, file references, and checksums.
- Rust signal execution primitives for deterministic channel arithmetic, peak
  extraction, and DFT magnitude fixtures.
- Rust report package workflow with accredited technical review, approval, and
  issue gates.
- Rust typed instrument setpoints and safety limits, with simulated-runtime
  blocking for commands outside known ranges.
- Python SQLite repository adapters for metrology and project records, backed by
  the versioned migration domains.
- Rust measurement execution session binding accepted plans to simulated runtime
  observations and raw dataset evidence.
- Rust synchronization conflict records for split repository snapshots,
  including conflict status and resolution.
- Rust report export bundle evidence linking issued reports to exported files,
  checksums, and review/approval identities.
- Python SQLite adapter query APIs for instrument, calibration, project, and
  audit-event records.
- Rust signal windowing and deterministic downsampling execution primitives.
- Rust update bundle workflow with signed-package checks, compatibility ranges,
  rollback references, offline-install rules, and live-measurement blocking.
- Rust transport adapter boundary with endpoints, simulated adapter fixture,
  adapter-backed runtime observations, transport mismatch checks, and shared
  safety-limit enforcement.
- Rust synchronization conflict service that turns conflict resolutions into
  audit-required action plans and applies/deferred resolutions safely.
- Python SQLite write APIs for instrument availability/capabilities,
  calibration attachments, project stage changes with audit events, and
  contract-review item completion.
- Python SQLite update-catalog adapter for signed package metadata and install
  records.
- Rust signal interpolation resampling and FFT backend boundary with backend
  traceability on spectrum results.
- Rust VISA, TCP/IP, and serial transport adapter skeletons with endpoint
  validation, timeout policy, and explicit unavailable-IO errors.
- Python SQLite measurement-data adapter for immutable datasets, signal
  channels, processing graphs, and result artifacts.
- Python SQLite test-definition adapter for standards, test methods, approved
  method revisions, processing graph metadata, and ordered evidence steps.
- SQLite synchronization domain and Python adapter for conflict records,
  action plans, resolution outcomes, and audit-event references.
- SQLite update-install validation evidence for signature, compatibility,
  offline-install, and active-measurement gates.
- Incremental Python migration application for existing domain databases.
- Static operator-facing GUI shell for dashboard, project, metrology, test
  definition, measurement-data, and update workflows.
- Rust dataset retention primitives with reviewed deletion transitions for
  immutable raw data.
- SQLite measurement-data retention evidence migration and Python APIs for
  retention events, current status, and reviewed deletion workflows.
- Python GUI bootstrap exporter that maps local SQLite repositories into the
  static console data contract, with a browser-loadable `bootstrap.js` file.
- Python GUI action command for audited local project stage advancement with
  optional `bootstrap.js` regeneration.
- Python GUI dataset-retention action command for request, approval, rejection,
  deletion marking, and optional `bootstrap.js` regeneration.
- Python GUI update validation and install action commands with optional
  `bootstrap.js` regeneration.
- IO-backed TCP/IP transport adapter with timeout policy, newline-terminated
  command exchange, response readback, and local socket test coverage.
- Transport exchange-attempt traceability on instrument observations, including
  TCP/IP retry attempt counts for failed and successful exchanges.
- Pure Rust radix-2 optimized FFT-compatible backend with deterministic DFT
  fallback and reference-matching test coverage.
- Rust traceability report view linking issued report exports to measurement
  run evidence, raw dataset checksums, observations, review, and approval.
- Traceability report run summaries for total and maximum instrument exchange
  attempts, so unstable communications remain visible during review.
- SQLite/Python measurement-data observation log for instrument commands,
  responses, endpoints, success state, and exchange-attempt evidence, with Qt
  runtime bootstrap rows derived from the latest local observations.
- Qt console runtime contract split into run, sequence, observation, and
  exchange-attempt columns, with runtime error and maximum-attempt metrics.
- Deterministic SHA-256 checksums for persisted instrument observations, with a
  lookup API for offline synchronization and audit comparisons.
- Rust, SQLite, and Python support for revisioned processing graph instances
  bound to source dataset checksums, graph checksums, creator identity, and
  software version.
- Rust, SQLite, and Python links from revisioned processing graph instances to
  result artifacts with output signal references and raw-lineage evidence.
- Rust signal window family expansion with Hamming, Blackman, and flat-top
  coefficients alongside rectangular and Hann windows.
- Rust windowed FFT execution that records the selected window and remains
  compatible with the optimized FFT backend.
- Qt desktop operator-console direction with an initial PySide6 bootstrap that
  reuses local GUI bootstrap data from the static workflow prototype.
- Testable Qt console view models for project, metrology, method, dataset, and
  update tables with explicit business columns.
- Qt console data loader that can read split SQLite repository paths directly
  while preserving `bootstrap.js` compatibility.
- Qt console operator action intents for project advancement, dataset-retention
  requests, and update validation.
- Structured Rust serial endpoint settings for port, baud rate, data bits,
  parity, and stop bits, validated by the serial transport adapter.
- Structured Rust VISA resource parsing for TCPIP, USB, GPIB, and ASRL resource
  strings, validated by the VISA transport adapter.
- IO-backed VISA TCP/IP resource exchange using the guarded TCP socket path,
  with exchange-attempt traceability and local socket test coverage.
- Rust, SQLite, and Python processing graph execution records with execution
  reference, actor, software version, status, and output artifact count.
- Qt console status metrics for active projects, metrology alerts, retained
  datasets, and updates requiring attention.
- Qt console runtime table contract for future instrument transport endpoints,
  runtime state, and last-observation display.
- Revisioned SQLite metrology taxonomy with 34 instrument categories across
  electronics, EMC, thermal, acoustic, shock/vibration, radio/RF, and
  data-monitoring domains, plus Python repository APIs and Qt/browser category
  tables.
- Local metrology registration action and CLI for creating category-linked
  instruments with optional initial calibration records and bootstrap refresh.
- Local metrology calibration action and CLI for adding renewal certificates to
  existing instruments while keeping prior calibration history.
- Local metrology availability action and CLI for marking instruments available,
  reserved, or out of service with bootstrap refresh.
- Local metrology capability action and CLI for replacing controlled instrument
  capability JSON on existing assets.
- Metrology inventory tables now surface instrument category and capability
  previews in the Qt and browser bootstrap views.
- Metrology instrument records now include part number, calibration periodicity,
  notes, automatic next-calibration due-date calculation, and attached document
  records for certificates, datasheets, transducer calculation sheets, scripts,
  manuals, photos, and other evidence.
- Project service-planning records for scheduling test execution with planned
  start/end, operator, location, equipment under test, status, category, and
  method references.
- Adjustable hierarchical test categories seeded with emission/immunity,
  conducted/radiated branches, and additional CEM families for harmonics,
  transient time-domain measurements, ESD, fast transients, and power quality.
- Qt and browser bootstrap views for metrology documents, service planning, and
  test-category taxonomy.
- Qt operator entry forms for registering instruments, attaching material
  documents, scheduling service items, and creating test categories against
  local SQLite repositories.
- Audited local project creation action, CLI, and Qt form so service planning
  can start from a new campaign record.
- Audited contract-review item completion action, CLI, Qt form, bootstrap
  export, and Qt/browser tables.
- Contract-review advancement gate requiring a complete checklist before
  planning, with reduced required items for non-accredited and investigation
  projects.
- Architecture transformation audit covering the current repo, target
  local-first architecture, P0-P3 migration plan, risk register, Mermaid
  diagrams, sync/object strategies, and initial JSON package contracts.
- ADR 0001 establishing Rust application services as the future critical
  business write boundary for Python, Qt, web apps, local agent, and station
  runtime.
- Rust `application_services` module with an initial project stage advancement
  command service, command receipt, and application-level errors.
- Mode-specific contract-review requirements in the Rust quality domain for
  accredited, non-accredited, and investigation workflows.
- Rust sync/data contract value objects for schema versions, stable ids, entity
  revisions, full SHA-256 content checksums, object manifests, entity snapshots,
  and idempotent change operations.
- SQLite sync operation journal migration for local-first operations with
  actor/device/correlation evidence, base/resulting revisions, normalized JSON
  payloads, full SHA-256 payload checksums, and replay statuses.
- Python `SyncRepository` operation-journal APIs for record/count/get/list and
  applied/failed status transitions.
- SQLite sync entity snapshot and checkpoint migration for local-first replay
  baselines, latest-entity views, and peer/domain/direction cursors.
- Python `SyncRepository` APIs for recording entity snapshots, querying latest
  snapshots, and upserting/listing sync checkpoints.
- Rust `EntitySnapshot` contract coverage for revisioned local-first entity
  baselines.
- Python local replay API that applies a pending sync operation as an entity
  snapshot and marks the operation applied in the same SQLite transaction.
- Python snapshot divergence API that records synchronization conflicts from
  mismatching entity snapshots without applying an automatic merge policy.
- Python conflict action-plan suggestion API that proposes an idempotent manual
  merge/defer plan while keeping the conflict unresolved for audit review.
- Rust `emc-locus-agent` binary crate with a testable `health` command that
  reports agent version, storage-root availability, and supported repository
  domains as JSON.
- Automated release-consistency test covering `VERSION`, Cargo workspace
  version, crate lockfile versions, Python package version, README, and revision
  baseline.
- `emc-locus-agent` storage commands for the first project vertical slice:
  initializing, inspecting, and verifying `projects.sqlite` and `sync.sqlite`
  from versioned SQLite migrations with stable JSON reports.

### Fixed

- Local service-planning actions now validate ISO local date-time blocks with
  parsed datetimes and reject schedule items crossing weekends before writing
  them to the project repository.
- Serial endpoint parsing now rejects whitespace-bearing port names and
  transport-reserved prefixes such as TCPIP, GPIB, USB, and ASRL, so serial
  adapters cannot silently accept bus or VISA aliases as native serial ports.
- TCP/IP instrument endpoints now resolve VISA-style `TCPIP0::host::port::SOCKET`
  resources and `TCPIP0::host::inst::INSTR` resources without misreading the
  interface or resource class as the socket target.
- Malformed VISA-style TCP/IP `SOCKET` resources with missing hosts, nonnumeric
  ports, or unknown resource classes are now rejected instead of silently
  falling back to the default SCPI port.
- VISA resource validation now rejects incomplete or interface-incompatible
  descriptors, including TCP/IP `SOCKET` resources without numeric ports,
  non-TCP/IP `SOCKET` resources, nonnumeric GPIB addresses, and missing ASRL
  port indexes.
- VISA and TCP/IP endpoint validation now rejects zero-valued TCP socket ports,
  plus GPIB primary or secondary addresses outside the valid 0-30 range.

### Planned

- Expand guarded IO-backed serial and VISA implementations behind the adapter
  skeletons.

## [0.1.0] - 2026-06-26

### Added

- Initial product README for EMC Locus.
- Rust workspace with the `emc-locus-core` crate.
- Core project lifecycle model from quotation to archive.
- Project audit trail model with actors, reasons, actions, and ordered events.
- Contract-review checklist model with baseline EN ISO/IEC 17025-oriented items.
- Baseline traceability requirements.
- Python helper package for recurring development session planning.
- Architecture, domain model, EN ISO/IEC 17025 alignment notes, roadmap, and
  storage schema draft.
- Session logs for the initial and autonomous development work.

### Validated

- `py -m compileall python\emc_locus`
- `cargo test`
