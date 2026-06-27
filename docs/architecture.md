# EMC Locus Architecture

## Design Intent

EMC Locus should become a laboratory platform, not a single monolithic script.
The architecture should keep regulated laboratory records stable while allowing
instrument drivers, data analysis, and UI workflows to evolve independently.

## Proposed Layers

### 1. Domain Core

The Rust core owns business invariants:

- project and campaign lifecycle states;
- traceability requirements;
- metrology records and calibration validity;
- audit-event creation rules;
- immutable dataset references;
- report approval gates.
- update package compatibility and installation gates.

This layer should not depend on a database, UI framework, or hardware driver.
The current module map is documented in `core-structure.md`.

### 2. Storage and Audit

The storage layer should preserve:

- immutable raw measurement data;
- versioned metadata;
- instrument identity and calibration records;
- user actions and system actions;
- report package history.

EMC Locus should not depend on a single remote repository during acquisition.
The first architecture target is a set of local SQLite repositories split by
domain, with signed snapshots and synchronization flows. Synchronization
coordination has its own local repository for conflict and action-plan evidence.
The first versioned SQLite migrations live under `storage/sqlite/`. See
`offline-first-architecture.md` for the repository split and
`storage-migrations.md` for the migration layout.

### 3. Instrument Runtime

Instrument control should be built around explicit commands and observations:

- simulated driver first;
- transport adapter boundary first, then concrete VISA, serial, TCP/IP, or
  vendor SDK implementations;
- command logs linked to measurement runs;
- safety interlocks and manual validation steps where needed.

The runtime must be transport-neutral and support common laboratory links such
as VISA, GPIB, serial, TCP/IP, UDP, USBTMC, CAN/LIN, Modbus, REST, vendor SDKs,
manual steps, and simulation. See `instrument-control-architecture.md` for the
driver boundary.

### 4. Signal Acquisition and Analysis

EMC Locus must also support tests that are not naturally modeled as a simple
level-versus-frequency sweep. Time-domain CEM work can require synchronized DAQ
streams, FFT or windowed FFT, temporal processing, event counting, harmonic
analysis, inrush analysis, and mathematical operations between signals.

This layer owns:

- DAQ-neutral acquisition profiles, with openDAQ as the preferred generic API;
- synchronization policies for multi-DAQ acquisitions;
- signal-processing pipeline definitions;
- links between processed signals, raw data, and reports.

### 5. Python Automation

Python is useful for:

- quick laboratory scripts;
- data import/export;
- numeric processing;
- report preparation;
- driver prototyping before hardening critical paths.

Python code should call stable APIs rather than duplicate domain rules.

### 6. Application API and UI

The measurement-station UI target is a Qt desktop application. Advanced EMC
measurement work needs dense operator screens, long-running acquisition state,
instrument panels, live warnings, local/offline operation, and future plotting
or dockable analysis workspaces.

The static browser shell is a workflow prototype and dashboard mockup. It is
useful for shaping information architecture, but it should not become the
primary technology for instrument-facing measurement software.

Future web UI work can still be useful for multi-user project tracking,
report-review portals, or fleet monitoring, but acquisition should remain
local-first and desktop-capable.

## Key Boundaries

```text
User/UI
  -> Application services
    -> Rust domain core
    -> Storage adapters
    -> Instrument runtime
    -> Signal acquisition and processing runtime
    -> Python automation pipelines
```

## Early Technical Decisions

- Start with no external runtime dependencies in the Rust core.
- Treat audit events as a core concept, not a later logging feature.
- Treat raw data as immutable once acquired.
- Support simulation before real hardware.
- Support offline field execution from local reference snapshots.
- Support separate quality modes for accredited, non-accredited, and
  investigation work.
- Require signed update packages and block live updates during measurement.
- Preserve update rollback metadata and installed-version compatibility checks.
- Support time-domain DAQ workflows beside frequency-domain sweep workflows.
- Prefer openDAQ for generic DAQ integration while allowing vendor SDK bridges.
- Use Qt as the primary desktop operator-console technology for local
  measurement stations.
- Keep EN ISO/IEC 17025 alignment as a design checklist, not as a legal claim.

## Open Questions

- Which instruments must be supported first?
- Which Qt binding and packaging path should be hardened first for field
  stations?
- What data formats are already used by the lab?
- What report template and approval process should be modeled first?
