# EN ISO/IEC 17025 Alignment Notes

EMC Locus should help a laboratory operate with traceable, reviewable, and
controlled records. The software cannot certify a laboratory by itself, but it
can make compliant work easier to perform and easier to audit.

## Design Principles

### Traceability

Every report result should be traceable to:

- the customer request;
- the contract review;
- the chosen method;
- the instrument identity;
- the calibration status;
- environmental conditions;
- raw data;
- processing steps;
- technical review;
- report approval.

### Data Integrity

The system should protect:

- immutable raw data;
- checksums for stored files;
- controlled metadata changes;
- explicit revision history;
- clear links between raw and processed results.

### Competence and Authorization

The system should model:

- user roles;
- authorized reviewers;
- authorized report approvers;
- training or competence references when required by local quality procedures.

### Method Control

The system should distinguish:

- approved test methods;
- customer-specific deviations;
- instrument setups;
- environmental limits;
- acceptance criteria.

### Equipment Control

Before a measurement run, the system should check:

- instrument identity;
- calibration validity;
- availability;
- required accessories;
- known restrictions or out-of-service status.

## First Compliance Backlog

1. Add immutable audit events to every controlled state transition.
2. Store instrument calibration status before measurement acquisition.
3. Add raw dataset checksums.
4. Require technical review before report issue.
5. Link each report result to a measurement run and dataset.
6. Export a traceability package for internal audit.

## Language to Avoid

Until validated by a quality manager and tested in real procedures, avoid claims
such as:

- "EN ISO/IEC 17025 certified software";
- "guarantees compliance";
- "validated for accreditation".

Prefer:

- "designed to support EN ISO/IEC 17025 workflows";
- "traceability-oriented laboratory platform";
- "audit-ready records and controlled workflows".
