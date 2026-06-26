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
- manufacturer;
- model;
- serial number;
- supported capabilities;
- current availability status.

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
- processing lineage.

### Report

Represents the customer-facing deliverable.

Important fields:

- report identifier;
- revision;
- reviewed datasets;
- approver;
- issue date;
- exported files.

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
- manufacturer, model, and serial number;
- availability status;
- calibration requirement;
- calibration certificate reference;
- issue date, due date, and provider;
- pre-run readiness report.

Accredited work blocks when a required instrument has no valid calibration.
Non-accredited work still reports missing or expired calibration, but those
issues are non-blocking unless another safety or availability issue exists.
Investigation mode can run exploratory checks with relaxed calibration
constraints, while out-of-service equipment remains blocking in every mode.

Calibration due soon is treated as an attention point rather than a hard block,
so the operator can continue a valid run while planning renewal.

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
