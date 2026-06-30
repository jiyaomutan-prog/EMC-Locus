# Changelog

All notable changes to EMC Locus are recorded here.

The project follows Semantic Versioning once public releases begin. During the
foundation phase, `0.x.y` versions may still evolve quickly, but every committed
change should remain traceable through Git history, session logs, and this file.

## [Unreleased]

### Fixed

- Processing graph execution records now reject result artifacts produced from a
  different graph reference or revision, preserving traceability between the
  execution evidence and the revisioned graph instance.
- Python measurement-data writes for processing graph executions now reject
  output artifact counts that do not match the artifacts persisted for the same
  graph instance.

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
