# Product Objectives

This document consolidates the EMC Locus objectives after the initial BAT-EMC,
DewesoftX, openDAQ, offline, metrology, and quality-system analysis.

## Product Mission

EMC Locus is an original, audit-ready platform for EMC laboratories. It should
cover the complete campaign lifecycle from quote to report while supporting both
classic EMC frequency sweeps and time-domain signal-analysis workflows.

The product must remain independent from proprietary BAT EMC internals. Public
product concepts can inform competitive positioning, but implementation,
schemas, UI, assets, and workflows must be original.

## Primary Objectives

### 1. Campaign and Quality Backbone

EMC Locus must model the laboratory campaign lifecycle:

1. quotation;
2. contract review;
3. test planning;
4. measuring;
5. technical review;
6. report issue;
7. archive.

Every controlled transition must produce audit evidence. Accredited work must
stay strict, while non-accredited and investigation work must be possible with
explicit labeling and traceable deviations.

### 2. Offline-First Field Execution

Measurement acquisition must not depend on a remote reference database or live
internet connection. A field station must run with local signed snapshots for
metrology, test definitions, drivers, project data, report templates, and update
metadata.

Synchronization must happen later with conflict detection and audit events.

### 3. Split Repositories

The product must avoid a single opaque database. Domain repositories should be
separated so each can have its own validation, export, import, signature, and
synchronization policy:

- metrology;
- test definitions;
- instrument drivers;
- project records;
- measurement data;
- report templates;
- update catalog.

### 4. Serious Instrument Runtime

Instrument control must be reliable, observable, and testable:

- transport-neutral;
- capability-driven;
- simulator-first;
- command and observation logs;
- safety checks before risky commands;
- explicit manual fallback steps.

The runtime must support practical lab communications: VISA, GPIB, serial,
TCP/IP, UDP, USBTMC, CAN, LIN, Modbus, REST, vendor SDKs, manual operation, and
simulation.

### 5. Native Signal Acquisition and Analysis

EMC Locus must not be limited to level-versus-frequency sweeps. It must also
support time-domain CEM workflows:

- railway harmonics;
- axle-counter measurements;
- inrush current;
- transient capture;
- pulsed disturbance analysis;
- custom investigations.

The signal layer must support DAQ integration, openDAQ-preferred acquisition,
multi-DAQ synchronization, FFT and temporal processing, channel math, event
timing, and lineage from processed results back to raw signals.

### 6. Metrology as First-Class Data

Metrology records must be first-class objects, not side notes:

- instrument identity;
- calibration records;
- validity checks;
- out-of-service status;
- uncertainty references;
- measurement-run equipment snapshots.

### 7. Update Manager

Updates must be controlled like laboratory evidence:

- signed packages;
- offline bundles;
- compatibility checks;
- rollback metadata;
- no updates during active measurement acquisition;
- changelog and validation evidence tied to releases.

### 8. Operator Console

The GUI must be a working laboratory surface, not a marketing layer. It should
let operators see campaign state, readiness, metrology status, test-definition
approval, dataset lineage, update gates, and local/offline operating mode from
one place.

The first GUI shell is static and can load a Python-generated local bootstrap
file from split SQLite repositories. First audited local project-stage,
dataset-retention, and update-management actions exist. The next step is to
connect real instrument IO behind the existing runtime boundary.

## Near-Term Implementation Objectives

1. Add IO-backed VISA, TCP/IP, and serial implementations behind the adapter
   skeletons.
2. Add a real optimized FFT implementation behind the backend boundary.
3. Add traceability report views for audit and technical review.

## Non-Objectives

- Do not copy BAT EMC code, screens, database schemas, assets, private
  procedures, or confidential behavior.
- Do not claim EN ISO/IEC 17025 certification for the software itself.
- Do not require a central server for acquisition.
- Do not hide relaxed, non-accredited, or investigation mode constraints.
- Do not treat processed signal results as sufficient without raw-data lineage.
