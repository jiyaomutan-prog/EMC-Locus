# Domain Model

This document names the first business objects for EMC Locus. The goal is to
make the laboratory workflow explicit before choosing a database schema or UI.

## Main Entities

### Customer

Represents the organization requesting tests.

Important fields:

- legal name;
- contacts;
- billing references;
- confidentiality constraints.

### Project

A project is the complete commercial and technical envelope of a measurement
campaign, from quotation to report delivery.

Lifecycle:

1. Quotation
2. Contract review
3. Test planning
4. Measuring
5. Technical review
6. Report issued
7. Archived

### Campaign

A campaign groups one or more EMC test sequences for a project.

Important fields:

- tested equipment;
- applicable standards;
- test site or bench;
- planned measurement sequences;
- actual measurement runs.

### Instrument

Represents a physical or simulated measurement instrument.

Important fields:

- asset identifier;
- category code and metrology domain;
- manufacturer;
- model;
- serial number;
- supported capabilities;
- current serviceability status and legacy availability/planning status.

### Instrument Category

Represents a revisioned metrology taxonomy entry used to group instruments by
measurement domain before driver, calibration, and readiness rules become
specific to a model.

Initial domains:

- electronics;
- EMC;
- thermal;
- acoustic;
- shock and vibration;
- radio/RF;
- data monitoring.

Each category records typical instrument types, measured quantities, likely
communication transports, a calibration profile, a default calibration
requirement, and public source provenance.

### Calibration Record

Defines the metrological validity of an instrument.

Important fields:

- certificate reference;
- calibration date;
- due date;
- uncertainty information;
- accredited provider when applicable;
- linked instrument identity.

### Measurement Run

Represents one controlled execution of a test method.

Important fields:

- campaign reference;
- operator;
- instrument set;
- software version;
- environmental conditions;
- raw dataset references;
- command and observation log.

### Dataset

Represents stored measurement data.

Important fields:

- immutable raw file reference;
- checksum;
- acquisition timestamp;
- originating measurement run;
- processing lineage;
- retention status.

Immutable raw datasets require a reviewed retention workflow before deletion.
The current core model records deletion requests, approvals, rejections, and
executed deletion events with actor and reason evidence.
Python measurement-data writes require dataset checksums to use canonical
`sha256:<64 lowercase hex characters>` evidence before persistence.

### Signal Processing Graph

Represents a revisioned numerical processing definition applied to acquired
signals.

Important fields:

- graph reference;
- graph revision;
- source dataset reference and checksum;
- input and output signal references;
- processing operations;
- graph definition checksum;
- creator identity;
- software version used to define or execute the graph.
- linked result artifacts with output signal references, file references,
  checksums, and raw lineage.

The Rust core now provides a persistable processing-graph instance model. The
SQLite measurement-data repository can store and retrieve revisioned graph
instances with source dataset checksum verification and link result artifacts
back to graph revisions.
Python processing-graph and result-artifact writes use the same canonical
`sha256:<64 lowercase hex characters>` checksum format.

### Report

Represents the customer-facing deliverable.

Important fields:

- report identifier;
- revision;
- reviewed datasets;
- approver;
- issue date;
- exported files.

### Update Bundle

Represents a controlled software, driver, template, signal-engine, or database
migration update.

Important fields:

- package name;
- package version;
- component;
- compatibility range for the currently installed version;
- checksum;
- signature evidence;
- offline-install permission;
- rollback reference.

### Audit Event

Records a meaningful action or decision.

Important fields:

- actor;
- action;
- target entity;
- timestamp;
- reason when required;
- previous and new values for controlled changes.

## Current Core Invariants

The Rust core now models the project lifecycle as a controlled sequence. A
project can move through the approved campaign stages only in order:

```text
Quotation -> Contract review -> Test planning -> Measuring
  -> Technical review -> Report issued -> Archived
```

A `ProjectRecord` wraps the current `Project` and its audit events. Opening a
record creates a first `ProjectCreated` audit event. Advancing the project stage
requires:

- a valid next stage;
- an audit actor;
- a non-empty reason.

If a transition is rejected, the project stage is unchanged and no audit event is
added.

## Audit Event Shape

The first audit event model contains:

- sequence number inside the project record;
- actor;
- project code;
- action;
- optional reason.

## Contract Review Checklist

The first contract-review checklist names the minimum information that should be
controlled before detailed planning and acquisition work:

- customer request defined;
- test method selected;
- laboratory capability confirmed;
- equipment availability checked;
- calibration status reviewed;
- impartiality risks reviewed;
- data retention agreed;
- report requirements agreed;
- deviations recorded.

The Rust core can mark each item complete, report missing items, and avoid
duplicate completions.

## Contract Review Stage Gate

The Rust core now provides a specific transition helper for moving from contract
review to test planning. This gate rejects the transition unless:

- the checklist belongs to the same project;
- the current project stage is `ContractReview`;
- the checklist is complete, or an authorized deviation is recorded.

When an authorized deviation is used, EMC Locus records a dedicated audit event
with the missing checklist items and the deviation reason before recording the
stage transition to `TestPlanning`.

Rejected gate checks are side-effect free: the project stage remains unchanged
and no audit event is appended.

## Metrology Registry

The Rust core now owns the first metrology registry primitives:

- instrument asset code;
- instrument family;
- instrument category reference;
- manufacturer, model, and serial number;
- serviceability status;
- legacy availability status retained for migration and planning compatibility;
- calibration requirement;
- calibration certificate reference;
- issue date, due date, and provider;
- pre-run readiness report.

Accredited work blocks when a required instrument has no valid calibration.
Non-accredited work still reports missing or expired calibration, but those
issues are non-blocking unless another safety or serviceability issue exists.
Investigation mode can run exploratory checks with relaxed calibration
constraints, while out-of-service equipment remains blocking in every mode.

Calibration due soon is treated as an attention point rather than a hard block,
so the operator can continue a valid run while planning renewal.

The SQLite metrology repository now seeds the first revisioned instrument
category taxonomy (`2026-06-27-v1`) with electronics, EMC, thermal, acoustic,
shock/vibration, radio/RF, and data-monitoring categories. Existing v1
metrology databases migrate with a nullable `category_code`, so legacy assets
remain valid while new assets can be linked to controlled categories.

## Measurement Run Planning

The Rust core now models the first pre-run planning gate. A measurement run plan
links:

- project code;
- run reference;
- test method reference;
- execution mode;
- selected instrument asset codes;
- metrology readiness report.

The plan is accepted only when the readiness report has no blocking issue.
Accredited work therefore rejects missing or expired required calibration.
Non-accredited and investigation work can preserve relaxed calibration warnings
as non-blocking evidence. Empty equipment selections are rejected.

This model does not execute acquisition yet. It prepares the controlled handoff
between campaign planning, metrology, and the future instrument runtime.

## Measurement Run Evidence

The Rust core now links an accepted measurement-run plan to evidence generated
around execution:

- instrument command observations;
- raw dataset records;
- dataset references;
- file references;
- SHA-256 checksum references;
- immutable raw-data flag.

A raw dataset record must belong to the same run reference as the plan. This
keeps the evidence chain coherent before persistence or report generation is
introduced.

## Measurement Execution Session

The Rust core now binds accepted measurement-run plans to simulated execution:

- the runtime instrument must belong to the planned equipment selection;
- executed commands are recorded as instrument observations;
- raw datasets are attached to the same run reference;
- finishing execution requires at least one raw dataset.

This creates the first controlled bridge from pre-run validation to executable
runtime evidence.

## Report Package Workflow

The Rust core now models the first result-to-report gate:

- draft report package;
- technical review submission;
- completed technical review with reviewer;
- approval with approver;
- issue.

For accredited work, approval requires completed technical review, and issue
requires approval. For non-accredited work, the report can be issued without the
same formal approval gate, while still retaining the option to review or approve
when the laboratory wants it.

Invalid workflow transitions are rejected without changing the report status.

## Report Export Bundle

The Rust core now models report export evidence. An export bundle can be created
only from an issued report and records:

- project code;
- report number;
- report revision;
- export format;
- exported file reference;
- checksum;
- reviewer identity when available;
- approver identity when available.

This keeps customer-facing files linked to the controlled report workflow rather
than treating exports as loose files.

## Traceability Report View

The Rust core now provides a traceability view for audit and technical review.
It links an issued report export to measurement-run evidence, raw dataset
checksums, command-observation counts, exchange-attempt summaries, equipment,
test method reference, technical reviewer, approver, and the baseline
traceability requirements.

## Update Bundle Workflow

The Rust core now models controlled update bundles. An install plan can be
prepared only when:

- the package is signed when the laboratory policy requires signatures;
- the installed software version is inside the package compatibility range;
- offline installation is allowed by both policy and package metadata;
- no measurement acquisition is active when policy blocks live updates.

The plan preserves rollback metadata so the future updater can retain evidence
for recovery and audit review.
