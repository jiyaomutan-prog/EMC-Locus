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

Persisted instrument observations carry deterministic SHA-256 checksums over
the command, response, endpoint, run, sequence, exchange-attempt count, and raw
payload evidence. Synchronization and audit tooling can compare observations by
content without relying on local SQLite row ids or station-specific timestamps.

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
- validation evidence for install gates.

The Rust core now models update bundles with package identity, semantic package
version, component, signed checksum, optional signature evidence, compatibility
range, offline-install permission, and rollback reference. Install planning
rejects unsigned packages when the laboratory policy requires signatures,
rejects incompatible installed versions, rejects offline bundles when either the
policy or package disallows them, and blocks update application while a
measurement acquisition is active.

The SQLite update catalog now records validation evidence for these gates before
an install record can reference it: installed version, source, signature status,
compatibility bounds, offline-install permissions, measurement-active status,
the resulting accepted/rejected decision, and the validating actor.

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

The Rust core now also provides a synchronization conflict service. It can
produce action plans from operator resolutions, apply those resolutions to
conflict records, keep deferred conflicts pending, reject unknown conflict ids,
and reject invalid resolution choices for the conflict kind. Every generated
plan requires an audit event and records the local/reference snapshot ids that
will be pushed, pulled, merged, deleted, or deferred.

The SQLite storage layer now includes a synchronization coordination repository
for conflict records and action plans. The Python adapter can persist detected
conflicts, append planned actions, resolve or defer a conflict in the same
transaction as its plan, and retain an optional audit-event reference for later
traceability.

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
metrology, projects, test definitions, measurement data, update catalog, and
synchronization coordination repositories.
