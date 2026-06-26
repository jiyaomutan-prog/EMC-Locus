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
