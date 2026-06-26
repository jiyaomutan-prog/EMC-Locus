# BAT-EMC Public Feature Baseline

This note records public information about Nexio BAT-EMC so EMC Locus can be
positioned against the functional expectations of an established EMC laboratory
tool.

This is not reverse engineering. EMC Locus must not copy proprietary BAT-EMC
code, UI screens, database schemas, binary protocols, licensed assets, or
confidential documentation. The purpose is to understand visible product
capabilities and build an original, auditable architecture.

## Public Sources Reviewed

- Nexio EMC measurement software:
  https://nexiogroup.com/en/emc-measurement-software/
- Nexio BAT-EMC project management:
  https://nexiogroup.com/en/emc-measurement-software/emc-measurement-software-project-management/
- Nexio BAT-EMC monitoring:
  https://nexiogroup.com/en/emc-measurement-software/emc-measurement-software-monitoring/
- Nexio BAT-EMC immunity:
  https://nexiogroup.com/en/emc-measurement-software/emc-measurement-software-immunity/
- Nexio new BAT-EMC version announcement:
  https://nexiogroup.com/en/new-version-of-bat-emc/
- Reliant EMC Nexio product catalog mirror:
  https://reliantemc.com/download/NEXIO/NEXIO-Product-Catalog.pdf
- RA Mayes Nexio BAT-EMC overview:
  https://www.ramayes.com/EMC_Test_Software.htm
- Amitronic Nexio products catalog:
  https://amitronic.fi/wp-content/uploads/2023/04/nexio-products-catalogue.pdf

## Observed BAT-EMC Capability Areas

### Test Automation

Public Nexio pages describe BAT-EMC modules for:

- conducted emission;
- radiated emission;
- conducted immunity;
- radiated immunity;
- automotive emissions;
- reverberation-oriented emission and immunity workflows in catalog material.

The public pages also mention test configuration dimensions such as limits,
antenna mast and turntable use, detectors, RBW, sweep time, setpoints,
modulation, regulation, BCI, conducted voltage immunity, and field-zone
calibration.

### Project and Laboratory Management

The public project-management page emphasizes centralized data, collaboration,
requirements integration, compliance support, and adaptable storage. EMC Locus
should treat these as baseline expectations but improve the weak point reported
by users: acquisition work must not depend on a permanent connection to a remote
reference system.

### EUT Monitoring

The monitoring page describes scenario-based fault detection, EUT monitoring
during EMC and environmental tests, integration into BAT-EMC, independent or
synchronized operation with external systems, and compatibility with devices
such as oscilloscopes, NI acquisition cards, and GPIB equipment.

Catalog and distributor material also mentions image-based monitoring,
reference-image comparison, fault images, audio prompts, fault messages, and
information transmission back to the EMC test system.

### Instrument Independence

Public catalog and distributor material presents BAT-EMC as independent from
instrument manufacturers, with hundreds of supported drivers. EMC Locus should
therefore make driver coverage a first-class product goal, but with a more
explicit and testable transport/driver architecture.

### Reporting and Standards

Public material emphasizes automatic reporting, standards coverage, and use by
accredited or full-compliance laboratories. EMC Locus should provide equivalent
traceability while allowing explicitly marked non-accredited and investigation
modes.

## EMC Locus Product Response

### 1. Offline-First, Not Remote-Reference-Dependent

EMC Locus must support field measurements without internet access. A mobile
station must be able to:

- acquire with local metrology snapshots;
- use local test definitions;
- use local instrument drivers;
- store raw data locally;
- generate provisional or final packages according to the selected quality mode;
- synchronize later with conflict detection.

### 2. Quality Modes Instead of One Rigid Workflow

The system must support at least:

- accredited mode: strict gates, valid calibration, controlled methods, report
  approval;
- non-accredited service mode: controlled work with relaxed metrology/report
  constraints where the contract allows it;
- investigation mode: exploratory work with audit-visible deviations and clear
  report labeling.

Relaxed modes must never silently pretend to be accredited work.

### 3. Serious Instrument Control

Instrument control must be designed around:

- transport abstraction;
- capability declarations;
- typed commands and observations;
- command logs;
- simulation;
- driver validation fixtures;
- safety interlocks;
- manual fallback steps.

The baseline transport list includes VISA, GPIB, serial, TCP/IP, UDP, USBTMC,
CAN, LIN, Modbus TCP/RTU, REST, vendor SDKs, manual steps, and simulation.

### 4. Split Repositories

EMC Locus should not use one opaque database for everything. Repository domains
should be separated at least into:

- metrology;
- test definitions;
- instrument drivers;
- project records;
- measurement data;
- report templates;
- update catalog.

Each domain can have its own export, import, validation, signature, and
synchronization policy.

### 5. Update Manager

The update manager must support:

- signed update packages;
- offline installation bundles;
- compatibility checks against local repositories;
- rollback points;
- no update application during active measurement acquisition;
- changelog and validation evidence tied to releases.
