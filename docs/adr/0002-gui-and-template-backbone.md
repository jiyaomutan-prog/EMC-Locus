# ADR 0002 - GUI Split And Template Backbone Freeze

## Status

Accepted.

## Context

EMC Locus now has a stable enough 0.8.0 base for agent-owned project writes,
metrology readiness, simulated EMC execution, audit, and outbox evidence.

The risk is no longer only technical execution. The immediate product risk is
accumulating provisional UI surfaces and shallow CRUD lists that do not match a
serious EMC laboratory product.

The current static web shell helped expose vocabulary, but it is not a final UI
architecture. Qt has already been selected as the direction for dense local
execution. LAB CONSOLE and TEST CONSOLE must now be treated as different
products sharing a domain backbone, not as two skins over one dashboard.

Legacy concepts may be imported, archived, or migrated, but they must not drive
new vocabulary, screens, routes, DTOs, or domain contracts.

## Decision

EMC Locus adopts a two-surface GUI direction:

- LAB CONSOLE is the laboratory management product for customers, products,
  projects, campaigns, templates, methods, standards, documents, personnel,
  competences, metrology, planning, reports, audit, sync, and updates.
- TEST CONSOLE is the Qt desktop product for local/offline execution,
  instrumentation, readiness, acquisition, monitoring, deviations,
  substitutions, reruns, and evidence publication.

The static shell under `apps/gui-shell` is explicitly a LAB CONSOLE information
architecture prototype. It may show navigation, object relationships, and
read-only prototype views. It must not become the final LAB CONSOLE
implementation architecture and must not host TEST CONSOLE runtime concepts as
fake web behaviors.

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

- New LAB CONSOLE screens should be designed around the information
  architecture, not around temporary repository tables.
- New TEST CONSOLE work should remain Qt-oriented and execution-centered.
- The static web shell should not gain fake backend writes, fake instrument
  control, or fake acquisition.
- Legacy support is limited to import, archive, migration, and traceability
  context.
- Method approval, second approval, roles, and competence rules become part of
  the design backbone before further UI expansion.
- Documents and standards become first-class domain objects rather than loose
  file references.
- Templates become revisioned, derivable, duplicable, instantiable, and
  update-controlled objects.

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
competences, and method approvals before extending execution runtime behavior.
