# Offline-First Architecture

EMC Locus must support measurement campaigns performed away from the laboratory
network. Internet or remote-reference access must improve the workflow, not be a
hard dependency for acquisition.

## Core Principle

A field station must be able to execute a planned campaign with local
references:

- metrology snapshot;
- test-definition snapshot;
- instrument-driver snapshot;
- project package;
- report templates;
- update catalog metadata.

The station then synchronizes results when connectivity returns.

## Repository Split

EMC Locus should use multiple domain repositories instead of one opaque
database.

### Metrology Repository

Owns:

- instrument identities;
- calibration records;
- out-of-service status;
- uncertainty references;
- metrology audit events.

Offline use requires a signed local snapshot.

### Test Definition Repository

Owns:

- standards and method references;
- test templates;
- limit lines;
- acceptance criteria;
- default acquisition parameters.

Offline use requires a versioned local snapshot.

### Instrument Driver Repository

Owns:

- driver packages;
- transport bindings;
- capability declarations;
- simulator fixtures;
- validation records.

Offline use requires installed, validated driver packages.

### Project Repository

Owns:

- customer project records;
- contract review;
- campaign planning;
- audit trail;
- report workflow state.

Offline use creates local changes that synchronize later.

### Measurement Data Repository

Owns:

- immutable raw files;
- checksums;
- processed datasets;
- command and observation logs;
- acquisition environment records.

This repository must be local-first. Raw data should never require a remote
write to be considered acquired.

### Report Template Repository

Owns:

- report templates;
- export profiles;
- disclaimers for accredited, non-accredited, and investigation modes.

### Update Catalog Repository

Owns:

- available versions;
- package signatures;
- compatibility metadata;
- rollback metadata.

## Synchronization

Synchronization should be explicit and reviewable:

- pull signed reference snapshots;
- run compatibility checks;
- push project and measurement changes;
- detect conflicts before merging;
- record sync audit events;
- never rewrite immutable raw data.

The Rust core now includes synchronization conflict records with:

- conflict id;
- repository domain;
- conflict kind;
- local snapshot id;
- reference snapshot id;
- status;
- resolution.

Supported first conflict kinds include concurrent update, local/reference
deletion, checksum mismatch, and schema mismatch. A conflict can be resolved or
deferred for later review, but a resolved conflict cannot be resolved again.

## Field Workflow

1. Prepare a signed field package in the laboratory.
2. Verify local references before departure.
3. Execute acquisition without remote login.
4. Mark outputs according to execution mode.
5. Synchronize when connectivity returns.
6. Resolve conflicts and produce final report packages.

## Field Repository Package

The Rust core now models a field package as one signed snapshot per repository
domain. Offline-field requirements currently expect:

- metrology;
- test definitions;
- instrument drivers;
- project records;
- measurement data;
- report templates;
- update catalog.

Each snapshot carries:

- repository domain;
- snapshot id;
- schema version;
- checksum;
- signature flag.

Validation rejects missing domains, duplicate domains, unsigned snapshots, and
snapshots below the required schema version. This is the first domain-level
control for switching from a remote reference to a local field package.

## Technology Direction

Early implementation can use SQLite per domain plus exported signed bundles.
Later implementations can add PostgreSQL, object storage, content-addressed raw
data, or peer-to-peer synchronization where needed.

The initial SQLite migration split is now represented in `storage/sqlite/` for
metrology, projects, test definitions, measurement data, and update catalog
repositories.
