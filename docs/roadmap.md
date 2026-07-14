# Roadmap

## Phase 0: Foundations

Goal: agree on vocabulary, boundaries, and quality principles.

Deliverables:

- product README;
- consolidated product objectives;
- architecture notes;
- core crate structure notes;
- domain model;
- EN ISO/IEC 17025 alignment notes;
- minimal Rust core crate;
- minimal Python helper package.

## Phase 1: Campaign Core

Goal: model the complete project lifecycle.

Deliverables:

- project and campaign entities;
- lifecycle state machine; initial Rust model added;
- audit events; initial Rust project audit log added;
- contract-review checklist; initial Rust checklist added;
- contract-review stage gate; initial Rust gate added;
- SQLite project stage/audit and contract-review write APIs; initial Python
  updates added;
- public BAT-EMC feature baseline; initial analysis added;
- quality modes and offline policy primitives; initial Rust model added;
- DewesoftX/openDAQ concept baseline; initial analysis added;
- signal acquisition and analysis primitives; initial Rust model added;
- first storage schema draft; initial SQLite sketch added.
- versioned split SQLite migrations; initial domain migrations added.
- Rust core module split; initial structure added.
- Rust application-service boundary; initial project stage advancement service
  added as first migration path away from direct Python write rules.
- SQLite test-definition adapter for standards, test methods, approved
  revisions, processing graph metadata, and evidence steps; initial Python
  adapter added.
- Adjustable test-category taxonomy seeded with emission/immunity and
  conducted/radiated branches; initial SQLite/Python/Qt/browser support added.

## Phase 2: Metrology Database

Goal: manage instruments and calibration records.

Deliverables:

- instrument registry; initial Rust model added;
- calibration status model; initial Rust model added;
- out-of-service workflow; initial Rust blocking rule added;
- uncertainty references;
- pre-run equipment validity checks; initial Rust readiness report added.
- SQLite metrology adapter; initial Python smoke adapter added.
- SQLite metrology query APIs; initial Python lookups added.
- SQLite metrology write APIs; initial Python updates added.
- Revisioned instrument category taxonomy for electronics, EMC, thermal,
  acoustic, shock/vibration, radio/RF, and data-monitoring equipment; initial
  SQLite/Python/Qt support added.
- Instrument registration fields for manufacturer, model, serial, part number,
  calibration periodicity, calculated next calibration, notes, and attached
  certificates, datasheets, transducer sheets, and scripts; initial
  SQLite/Python/Qt/browser support added.
- Agent-backed metrology readiness vertical slice; instrument registration,
  calibration events with certificate manifests, computed status, serviceability
  changes, readiness assessment, audit/outbox evidence, Qt/Python agent path,
  historical calibration migration, and real HTTP restart E2E coverage delivered
  as the `0.7.0` baseline.
- First simulated EMC execution workflow delivered as the `0.8.0` baseline:
  operator launch attempt, test-context metrology preflight, structured refusal,
  persisted completed result, instrumentation snapshot, project audit/outbox,
  local API routes, and a minimal Qt operator form.
- Simulated EMC execution launch now blocks known agent-owned test templates
  unless the referenced template is approved, and persists the selected
  approved revision id plus definition checksum on the execution attempt while
  the full execution-package binding remains a future slice.
- Revisioned test-template aggregate delivered as the `0.9.0` baseline:
  typed core definition, canonical JSON and checksum, draft replacement with
  optimistic concurrency, immutable submitted/approved revisions, derivation
  from approved revisions, audit/outbox evidence, Python client routes, and real
  HTTP restart E2E coverage. This is not a LAB CONSOLE editor and not a runtime
  execution engine.
- `0.9.1` hardens that baseline for future editor work: SQL-level CAS for draft
  edits and transitions, one active draft per template, explicit aggregate DTO
  pointers for current approved/latest/active draft revisions, approval-time
  supersession of older approved revisions, strict GUI bootstrap JSON, and
  Windows launchers for the prototype and Qt console.
- `0.10.0` delivers LAB CONSOLE Template Studio v1: a React/TypeScript
  application served by the local agent under `/lab/`, backed only by public API
  routes, with template library, create/clone, structured draft editing,
  validation, checksum save, submit, approve, derive, history, audit, demo seed,
  launchers, CI, unit tests, and Playwright E2E coverage.
- `0.11.0` delivers the Equipment Definition Catalog and Driver Script Studio
  vertical: separate `equipment.sqlite`, typed Rust core aggregates for
  equipment models and driver profiles, physical ports, communication
  interfaces, measurement capabilities, structured driver actions/scripts,
  deterministic simulation, provider status, LAB CONSOLE Equipment workspace,
  API seed, audit/outbox, and Python client coverage. It is not a physical
  fleet redesign or a certified real hardware runtime.
- `0.12.0` productizes physics-based equipment classification: backend-owned
  registries for functional roles, signal domains, port directions, flow roles
  and technology tags; classification presets with port topology; indexed
  model summaries for catalog filters; `from-preset` API creation; Python
  client methods; LAB CONSOLE catalog filters, preset creation and port
  topology editing; and public-API demo seed expansion. It is not a new driver
  runtime or acquisition engine.
- `0.12.1` repairs CI/release parity after the 0.12.0 push: LAB CONSOLE source
  and generated text artifacts are pinned to LF line endings, npm is the single
  active frontend validation path, `scripts/validate-ci.ps1` mirrors GitHub
  Actions, and CI emits tool versions plus failure diagnostics. It adds no
  product feature.
- `0.13.0` delivers Sensors, DAQ Channels, Scaling And Engineering Curves:
  typed revisioned sensor/transducer definitions, scaling profiles, engineering
  correction curves, DAQ channel profiles, and logical acquisition channel
  recipes in `equipment.sqlite`; public local-agent routes; Python client
  helpers; seed/demo data; and LAB CONSOLE editors with CSV curve/table support
  and deterministic 1D curve evaluation. It is not real acquisition, station
  wiring, physical fleet deployment, FFT, reporting, RBAC, or synchronization.
- `0.13.1` refounds the Equipment Repository UX around a business taxonomy and
  configurable field templates: seven system root categories, subcategories,
  field dictionary, inherited category field rules, effective templates,
  model field snapshots, category-driven creation wizard, demo filtering, API
  routes, tests, and docs. It preserves the 0.11-0.13 technical domains under
  the hood and does not implement physical assets, station wiring, acquisition,
  FFT, reporting, RBAC, or synchronization.
- `0.13.2` polishes that Equipment Repository experience: grouped navigation,
  a real expandable tree with contextual actions, explicit wizard choices,
  label-first category and field creation with generated internal codes, form
  preview separated from advanced diagnostics, multi-level subcategory use,
  and category-subtree filtering. It remains a usability patch and does not
  implement physical assets, station wiring, acquisition, FFT, reporting,
  RBAC, or synchronization.
- `0.13.3` refounds the LAB CONSOLE shell and operator hierarchy: only active
  workspaces are actionable, Equipment commands follow the selected context,
  technical evidence is progressively disclosed, the equipment wizard is a
  focused modal task, and desktop/mobile layouts preserve usable editor width.
  It also carries the post-0.13.2 planning overlap guards and adds no new
  acquisition or laboratory domain vertical.

## Phase 3: Measurement Runtime

Goal: run repeatable test sequences with simulated hardware.

Deliverables:

- measurement-run planning model; initial Rust pre-run gate added;
- simulated instrument driver; initial Rust runtime added;
- command and observation log; initial Rust observation log added;
- measurement-run execution model; initial Rust execution binding added;
- raw dataset checksum; initial Rust dataset evidence model added;
- SQLite measurement-data adapter; initial Python adapter added;
- data-retention policy hooks; initial Rust workflow, SQLite evidence, and
  Python adapter APIs added.
- Python measurement-data writes now reject non-canonical dataset,
  processing-graph, and result-artifact checksums before lineage evidence is
  persisted.
- Python measurement-data observation lookups now reject non-canonical
  observation checksums before querying runtime traceability records.
- Python test-definition writes now reject non-canonical test-method revision
  checksums before draft creation or approval can persist shortened or uppercase
  `sha256:` evidence.

## Phase 4: Reporting Pipeline

Goal: build a controlled result-to-report path.

Deliverables:

- report package model; initial Rust model added;
- technical review workflow; initial Rust gate added;
- approval workflow; initial Rust gate added;
- export bundle; initial Rust evidence model added;
- traceability report for audit; initial Rust view added;
- exchange-attempt review summaries; total and maximum attempts per run added.

## Phase 4b: Signal Acquisition and Analysis

Goal: support CEM tests based on time-domain acquisition and advanced signal
processing, not only level-versus-frequency sweeps.

Deliverables:

- openDAQ-preferred DAQ integration boundary;
- simulated DAQ source;
- synchronized acquisition dataset model; initial Rust model added;
- FFT and temporal-processing pipeline model; initial graph model added;
- numeric DFT/peak/channel arithmetic fixture; initial Rust execution added;
- windowing and deterministic downsampling; initial Rust execution added, with
  rectangular, Hann, Hamming, Blackman, and flat-top windows;
- FFT backend boundary and interpolation resampling; initial Rust execution
  added;
- optimized radix-2 FFT-compatible backend with DFT fallback; initial Rust
  execution added;
- windowed FFT execution with retained window metadata and optimized-backend
  compatibility; initial Rust execution added;
- channel math and signal lineage; initial Rust lineage model added;
- persisted processing graph instances; initial Rust instance model, SQLite
  migration, and Python repository APIs added;
- result artifacts linked to revisioned processing graph instances; initial
  Rust artifact model, SQLite migration, and Python repository APIs added;
- graph-driven execution records; initial Rust model, SQLite migration, and
  Python repository APIs added;
- execution records now reject artifacts from a different processing graph
  reference or revision, preserving graph-to-result evidence integrity;
- Python measurement-data writes now reject graph execution records whose
  output artifact count does not match the artifacts persisted for that graph
  instance;
- failed processing graph executions are covered by the same persisted-artifact
  count invariant as completed executions;
- Python measurement-data writes now reject processing graph result artifacts
  whose output signal reference or raw-lineage JSON cannot be parsed as
  controlled signal traceability evidence;
- Python measurement-data writes now reject processing graph instances and
  executions whose software-version evidence is blank;
- Python measurement-data writes now reject malformed processing-graph
  operation definitions before persisting legacy or revisioned graph records;
- CEM time-domain test families such as railway harmonics, axle counters, and
  inrush measurements.

## Phase 5: Real Instrument Adapters

Goal: connect selected real instruments safely.

Deliverables:

- transport adapter boundary; initial Rust trait and simulated fixture added;
- first concrete hardware transport adapter skeletons; initial Rust VISA,
  TCP/IP, and serial skeletons added;
- TCP/IP IO-backed exchange; initial Rust standard-library implementation and
  local socket test added;
- transport attempt traceability; adapter-backed observations now retain
  exchange counts, including TCP/IP retry attempts;
- structured serial endpoint settings; initial Rust parser and adapter
  validation added;
- structured VISA resource settings; initial Rust parser and adapter validation
  added;
- VISA TCP/IP resources exchanged through the guarded TCP socket path; initial
  Rust local socket coverage added;
- instrument capability declarations;
- command templates;
- validation against simulated baseline;
- operational safety checklist; initial typed safety-limit model added.

## Phase 6: Offline, Sync, and Updates

Goal: make EMC Locus robust outside the laboratory network.

Deliverables:

- split local repositories;
- signed reference snapshots; initial Rust field-package model added;
- snapshot schema compatibility checks; initial Rust validation added;
- synchronization conflict workflow; initial Rust action-plan service added;
- synchronization conflict records; initial Rust model added;
- synchronization conflict persistence; initial SQLite migration and Python
  adapter added;
- local agent storage initialization/status/verification for the project
  vertical slice; initial Rust CLI support added for project and sync databases;
- project/sync multi-SQLite atomicity policy; rollback journal enforcement and
  status reporting added for the agent-owned vertical slice;
- local agent project write path; initial Rust CLI support added for project
  creation, contract-review completion, transition to test planning, audit
  inspection, and pending outbox inspection;
- local agent versioned loopback API; initial Rust HTTP routes added for the
  project vertical slice and pending outbox inspection;
- Python/Qt project client path; initial thin Python client added so project
  creation, contract-review completion, and planning transition can call the
  local agent when configured;
- Qt/Python project read path; project list, contract-review status, audit
  events, and sync outbox now read through the local agent when configured;
- hardened 0.6.0 project vertical slice; idempotence, DTOs, module split,
  SQLite atomicity, Qt/Python agent reads, and CI consolidated into the release
  baseline;
- real local-agent E2E coverage; initial automated HTTP server test added for
  the project vertical slice, including restart and persistence verification;
- Qt project transition form; initial operator form added for the agent-backed
  transition to `test_planning`;
- offline update bundles; initial Rust model added;
- update-catalog persistence APIs; initial Python adapter added;
- rollback and compatibility checks; initial Rust validation added.
- update-install validation evidence mapped into SQLite; initial Python adapter
  added.

## Phase 7: Operator Console

Goal: give the laboratory user a practical, local-first application surface.

Deliverables:

- dashboard for campaign status, readiness, datasets, and update gates; initial
  static workflow prototype added;
- Qt desktop console for local measurement stations; initial PySide6 bootstrap
  added;
- project workflow view with selected campaign detail and stage movement;
  initial fixture-driven interaction added;
- metrology, test-definition, measurement-data, and update-management views;
  initial static prototype views added;
- local/offline switch visible in the operator workflow; initial UI control
  added;
- service wiring to the Python repository adapters; initial Python bootstrap
  export and `bootstrap.js` loading path added;
- local write actions and refresh workflow; initial audited project stage action
  and dataset-retention and update-management actions added;
- service planning table for scheduling test execution by project, operator,
  category, location, EUT, status, and planned time window; initial
  SQLite/Python/Qt/browser support added;
- local service-planning actions now reject non-canonical local date-time
  blocks and weekend-crossing schedule items before repository writes;
- local service-planning actions now also reject multi-day business-day blocks,
  keeping each scheduled execution item as a single intra-day lab block;
- project repository service-schedule inserts now enforce the same one
  intra-day, positive-duration business-block validation, so lower-level Python
  callers cannot bypass the planning safeguard;
- project repository service-schedule inserts and status updates now also
  enforce the allowed planning status vocabulary before SQLite writes;
- project repository service-schedule status updates now reject blank planning
  item codes before they can become silent no-op updates;
- project repository service-schedule insert coverage now explicitly proves
  blank planning item codes are rejected before SQLite constraints are reached;
- project repository service-schedule inserts now also return controlled
  validation errors for missing required planning text instead of raw Python
  attribute errors;
- project repository service-schedule status updates now also reject unknown
  planning item codes instead of returning a silent no-op result;
- project repository service-schedule list filters now normalize readable
  project/status filters and reject malformed filters instead of returning
  misleading empty schedule lists;
- project repository service-schedule inserts now also enforce required
  planning context fields before SQLite writes, including operator, location,
  title, and equipment under test;
- project repository service-schedule inserts now normalize optional category
  and method references so blank planning references are stored as absent
  values rather than empty traceability fields;
- project repository service-schedule inserts now normalize optional notes so
  missing notes persist as an empty non-null planning note instead of surfacing
  a raw SQLite constraint error;
- service-schedule inserts and audited inserts now reject non-text optional
  category, method, and notes values before writing planning rows or audit
  evidence;
- project repository service-schedule inserts now explicitly reject unknown
  project references with a controlled planning error before SQLite write
  attempts;
- project repository service-schedule inserts now explicitly reject duplicate
  planning item codes with a controlled planning error before SQLite uniqueness
  constraints are reached;
- local service-planning actions now reject unknown category and method
  references when a test-definition repository is available, preventing
  operator planning rows from pointing at absent taxonomy records;
- project repository service-schedule insert coverage now directly proves
  weekend-only planning blocks are rejected by the lower-level business-day
  guard, independently from the GUI/CLI action path;
- project repository service-schedule inserts now reject projects that have
  not entered `test_planning`, preserving the contract-review gate even for
  direct Python callers;
- project repository service-schedule status updates now reject planning rows
  whose project reference no longer resolves, preserving campaign context even
  for imported or corrupted local data;
- project repository service-schedule list reads now reject planning rows whose
  project reference no longer resolves, preventing corrupted imports from
  appearing in planning views without campaign context;
- project repository service-schedule list reads filtered by project now reject
  unknown project codes instead of returning an ambiguous empty planning list;
- GUI/CLI service-planning actions now create project audit evidence in the
  same transaction as the schedule row, preserving traceability for planned
  laboratory blocks.
- project repository service-schedule status changes now have an audited update
  path that records previous/new status context as project audit evidence.
- local service-planning actions now expose audited status changes through the
  Python/CLI and Qt form paths, so planned blocks can be confirmed, started,
  completed, or cancelled without bypassing project audit evidence.
- service-schedule status updates now reject unchanged statuses before mutating
  planning rows or creating audit evidence, keeping duplicate operator
  submissions side-effect free.
- service-schedule status updates now reject changes after a row reaches
  `completed` or `cancelled`, keeping closed laboratory blocks terminal for
  direct and audited repository callers.
- service-schedule status updates now reject non-sequential transitions, so
  direct and audited repository callers cannot move planning rows backward or
  skip the confirmation/start workflow states.
- service-schedule status updates now normalize the persisted current status
  before transition validation and audit payload creation, so padded known
  states imported outside repository guards can still advance canonically.
- service-schedule status updates now match persisted project references
  against normalized project codes before mutation or audit creation, so padded
  imported project references can still advance under canonical project
  evidence.
- service-schedule status updates now match persisted planning item codes
  against normalized item references before mutation or audit creation, so
  padded imported planning codes can still advance under canonical audit
  evidence while ambiguous normalized duplicates are rejected.
- service-schedule inserts now reject new planning item codes that would
  duplicate an already-persisted padded planning code after normalization, so
  repository-created rows cannot introduce later read/update ambiguity.
- service-schedule list reads now reject persisted planning rows whose item
  codes normalize to the same value, so operator views cannot surface ambiguous
  planning identifiers that later status updates would refuse.
- service-schedule list reads and status updates now reject blank or non-text
  persisted `created_at`/`updated_at` evidence, preventing corrupted planning
  timestamp evidence from surfacing or being advanced.
- service-schedule list reads and status updates now also require persisted
  `created_at`/`updated_at` evidence to use canonical `YYYY-MM-DDTHH:MM:SSZ`
  UTC timestamps, preventing malformed textual import evidence from surfacing,
  mutating, or creating audit records.
- service-schedule inserts, list reads, and status updates now reject non-text
  `planned_start_at`/`planned_end_at` evidence before business-day validation,
  preventing corrupted planning blocks from surfacing, advancing, or creating
  audit records.
- service-schedule inserts, list filters, and direct or audited status updates
  now reject non-text requested statuses before normalization, preventing raw
  Python type errors and audit evidence from malformed status input.
- service-schedule inserts and status updates now reject non-text required
  planning text, update item codes, and audit actors before any planning row
  mutation or project audit event can be created, preventing raw required-text
  errors in direct repository and traceability paths.
- service-schedule list reads and status updates now reject imported planning
  rows whose project has not reached `test_planning`, preventing
  constraint-bypassed rows from surfacing or advancing before the
  contract-review gate.
- service-schedule status updates now reject persisted current statuses that
  are not text before direct or audited mutation, preventing corrupted imports
  from producing ambiguous transition evidence.
- service-schedule status updates now revalidate the persisted planning row
  before direct or audited mutation, preventing corrupted imports with invalid
  business-day blocks or malformed planning context from advancing.
- service-schedule inserts now require the initial status to be `planned`, so
  direct Python, CLI, and Qt callers cannot create planning rows that bypass
  the controlled confirmation/start/completion transitions.
- service-schedule status text is now normalized before inserts, updates,
  filters, and GUI/CLI actions, so whitespace in operator input cannot create
  false unknown-status rejections or non-canonical audit evidence.
- service-schedule list filters and project joins now compare against
  normalized persisted project/status text, so constraint-bypassed imports with
  padded known values remain visible to canonical filtered reads.
- service-schedule list reads now normalize persisted known status text,
  keeping imported padded planning states aligned with repository-written rows.
- service-schedule list reads now reject persisted rows with unknown status
  values, preventing corrupted imports or constraint-bypassed rows from
  reaching repository callers or operator views.
- service-schedule list reads now also revalidate persisted planning windows,
  preventing corrupted imports from surfacing weekend, multi-day, or
  non-positive laboratory blocks.
- service-schedule list reads now also reject persisted rows with blank
  required planning text, preventing corrupted imports from reaching repository
  callers or operator views without usable planning context.
- service-schedule list reads now normalize required planning text from
  persisted rows, keeping imported padded planning codes, titles, operators,
  locations, and EUT context aligned with repository-written rows.
- service-schedule list reads now normalize optional category, method, and
  notes text from persisted rows, keeping imported blank optional planning
  fields aligned with repository-written rows.
- the Qt service-schedule status form now exposes only actionable update
  targets and hides completed/cancelled planning rows, keeping the operator
  form aligned with repository transition guards.
- the Qt service-schedule status form now derives target choices from the
  current non-terminal planning rows, so a planned block does not present
  direct start or completion jumps that repository transition guards reject.
- service-schedule inserts now reject overlapping active planning blocks for
  the same operator or the same location, while adjacent blocks and reuse after
  completion or cancellation remain allowed.
- service-schedule inserts now compare overlap windows against normalized
  persisted timestamps, so whitespace-padded imported rows still reserve their
  operator and location consistently with list-read normalization.
- Python local metrology actions and direct repository writes now reject
  non-canonical certificate and instrument-document checksum evidence, keeping
  legacy SQLite document attachments aligned with the agent's lowercase
  SHA-256 digest contract.
- Python update-catalog writes now reject signed package checksums unless they
  use canonical `sha256:<64 lowercase hex characters>` evidence before install
  validation or bootstrap views consume the package metadata.
- Python synchronization conflict detection now revalidates stored local and
  reference snapshot checksums before comparing them, so imported snapshot rows
  cannot create conflicts with non-canonical digest evidence.
- agent-owned test templates now require any referenced method revision to be
  approved before the template can be created and store content as explicit
  immutable revisions after review;
- future Qt model/view screens backed by application services;
- future Rust-backed command execution bridge for instrument runtime actions.

## Near-Term Next Session

Recommended next vertical: `0.14.0 - Physical Asset Fleet, Station Connections
And Measurement Chain Drafting`. It should connect approved equipment models,
sensors, curves, DAQ profiles, and recipes to physical serial-numbered assets
and station-level connection bindings. It should not become a real acquisition
runtime, FFT engine, report generator, RBAC implementation, or central sync.

The parallel runtime stream should continue guarded serial or VISA IO behind the
adapter skeletons.

The UI stream should keep migrating Qt actions toward Rust-owned application
services instead of direct SQLite writes.
