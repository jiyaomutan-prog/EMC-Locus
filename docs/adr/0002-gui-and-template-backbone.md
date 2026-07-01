# ADR 0002 - LAB/TEST Console Split And Template Backbone Freeze

## Status

Accepted.

## Context

EMC Locus now has a stable enough `0.8.0` base for agent-owned local writes,
audit/outbox evidence, metrology readiness, and a simulated EMC execution
workflow.

The immediate product risk is no longer only technical execution. The risk is
accumulating provisional UI surfaces, shallow CRUD lists, and temporary
vocabulary that do not match a serious EMC laboratory platform.

The product must not be designed for a legacy internal shape. Legacy concepts
may be imported, archived, migrated, or used as comparison material, but they
must not drive new screens, DTOs, routes, domain names, or workflow contracts.

## Decision

EMC Locus adopts a two-console GUI direction:

- **LAB CONSOLE** is the web-oriented laboratory management console for clients,
  products, projects, campaigns, templates, methods, standards, documents,
  people, competences, roles, metrology, planning, reports, synchronization,
  audit, and updates.
- **TEST CONSOLE** is the Qt desktop console for local/offline execution,
  instrumentation, readiness, acquisition, monitoring, deviations,
  substitutions, reruns, and evidence publication.

Metrology is a first-class controlled LAB CONSOLE domain and a TEST CONSOLE
readiness dependency. It is not a reason to add a third fake dashboard or to
let TEST CONSOLE become the source editor for calibration and serviceability
records.

The former static shell under `apps/gui-shell` was explicitly a LAB CONSOLE
information architecture prototype. In `0.10.0` it is replaced by
`apps/lab-console`, a real React/TypeScript application served by the Rust local
agent. LAB CONSOLE still must not host TEST CONSOLE runtime concepts as fake web
behavior.

Before adding another major runtime vertical, the project must stabilize the
template and execution-definition backbone:

- project template;
- campaign template;
- tested product template;
- test template;
- method revision;
- standard reference;
- attached document;
- variable definition;
- variable lock policy;
- limit definition;
- post-processing definition;
- execution sequence step;
- branching rule;
- instrumentation chain slot;
- rerun and supersession model.

The design must preserve a clear distinction between:

- generic definition;
- project instance;
- executed instance;
- result.

It must also preserve the result-state chain:

- raw data;
- corrected data;
- calculated result;
- validated result;
- published result.

## Consequences

- New LAB CONSOLE screens must be designed around the information architecture,
  not around temporary repository tables.
- New TEST CONSOLE work remains Qt-oriented and execution-centered.
- The static web shell must not gain fake backend writes, fake instrument
  control, fake readiness execution, or fake acquisition.
- Legacy support is limited to import, archive, migration, and traceability
  context.
- Method approval, second approval, roles, and competence rules become part of
  the design backbone before further UI expansion.
- Documents and standards are first-class domain objects, not loose file paths.
- Templates are revisioned, derivable, duplicable, instantiable, and
  update-controlled objects.
- Runtime slices should wait unless they consume this clarified model rather
  than adding another temporary screen or schema.

## Non-Goals

This decision does not implement:

- new Rust runtime slices;
- new SQLite schemas;
- real acquisition;
- drivers;
- FFT or signal processing;
- report generation;
- final synchronization or merge algorithms;
- final web framework selection for LAB CONSOLE.

## Follow-Up

The next implementation tranche should use these documents to design real LAB
CONSOLE screens and domain contracts for templates, documents, people, roles,
competences, method approvals, and execution packages before extending runtime
behavior.
