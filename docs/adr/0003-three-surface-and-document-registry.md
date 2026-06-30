# ADR 0003 - Agent-Owned Document Registry

## Status

Accepted for the document registry.

The earlier three-surface GUI wording is superseded by
`0002-gui-and-template-backbone.md`: EMC Locus now distinguishes LAB CONSOLE web
and TEST CONSOLE Qt. Metrology remains a first-class controlled laboratory
domain and readiness dependency, not a third GUI product.

## Context

The product target is a local-first laboratory platform made of two consoles
sharing one data and evidence model.

- LAB CONSOLE manages clients, products, projects, communications, templates,
  documents, metrology records, planning, technical review, reports, delivery,
  sync, audit, and updates.
- TEST CONSOLE is the Qt desktop surface for fast, reliable local/offline test
  preparation, instrument control, acquisition, processing, visualization, and
  technical evidence creation.

The consoles must not introduce competing local writes. The Locus Local Agent
remains the owner of local SQLite writes, audit, outbox, and future
synchronization.

Documents and scientific datasets must not be stored as opaque columns inside
relational tables. SQLite and PostgreSQL hold metadata, relations, revisions,
decisions, states, and audit. Files and large scientific payloads are stored
separately through object references and content checksums.

## Decision

The first shared document capability is an agent-owned attached-document
registry:

- metadata is stored in `projects.sqlite` for the current local slice;
- file content is not uploaded or embedded in this slice;
- each document records owner surface/domain, owner entity, classification,
  title, storage backend, storage URI, original filename, MIME type, byte size,
  SHA-256 checksum, revision, applicability, confidentiality, creator, and
  timestamps;
- each write produces a document audit event and a pending outbox operation;
- idempotent `operation_id` replay follows the same rules as project,
  metrology, and simulated execution writes.

The initial outbox domain remains `project_records` with entity type
`attached_document`. A future repository split may promote documents to their
own local repository domain when synchronization and central PostgreSQL/object
storage contracts are introduced.

## Consequences

- LAB CONSOLE and TEST CONSOLE can refer to the same document object shape
  instead of inventing console-specific file records.
- Metrology certificates, worksheets, datasheets, and scripts converge on the
  same attached-document metadata contract used by projects, methods, standards,
  reports, and execution evidence.
- The static web shell must present the LAB/TEST split and document/metrology
  relationships without pretending to be an execution console.
- The current slice proves document metadata, audit, and outbox behavior
  without pretending to implement object upload, PDF parsing, report
  generation, central PostgreSQL, or cloud synchronization.
- Existing metrology certificate manifests remain valid local evidence, but
  future UI and API work should converge on the attached-document registry.

## Non-Goals

This decision does not implement:

- central PostgreSQL;
- object storage service integration;
- file upload or download;
- PDF, Word, Excel, image, Parquet, or HDF5 parsing;
- final document permissions;
- document version merge;
- final LAB CONSOLE or TEST CONSOLE implementation.
