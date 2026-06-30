# GUI Dual Surface Architecture

## Purpose

EMC Locus now distinguishes two product consoles:

- **LAB CONSOLE** is the web-oriented laboratory management console.
- **TEST CONSOLE** is the Qt desktop execution console.

Both consoles share the same domain backbone through the Locus Local Agent,
SQLite repositories, audit trail, outbox, future synchronization contracts, and
controlled document model. They must not become two competing implementations of
the same screens.

The current static web shell under `apps/gui-shell` is only an information
architecture prototype for LAB CONSOLE. It is not the final web application
architecture, and it must never become an instrument-facing execution console.

## LAB CONSOLE

LAB CONSOLE covers the laboratory management work before, around, and after test
execution. It is optimized for structure, traceability, planning, review,
configuration, document control, and multi-person work.

LAB CONSOLE owns or coordinates:

- clients, contacts, confidentiality, and customer visibility;
- products, product versions, tested article identity, and customer evidence;
- requests, quotations, contract review, and project lifecycle;
- projects, campaigns, campaign scope, and report delivery state;
- project templates, campaign templates, tested-product templates, and test
  templates;
- method revisions, standards, validation evidence, and approval workflows;
- documents as first-class controlled objects: client files, standards, PDFs,
  datasheets, certificates, worksheets, scripts, drawings, contracts, reports,
  and publications;
- personnel, configurable roles, permissions, competences, delegations, and
  approval records;
- metrology management: instrument assets, categories, calibrations,
  restrictions, serviceability, certificates, due dates, reservations, external
  equipment contacts, and metrological aptitude evidence;
- planning: rooms, benches, operators, internal instruments, external
  equipment, schedule conflicts, and station packages;
- reports, technical review, publication, customer delivery, archive, and
  supersession;
- synchronization, audit review, repository health, and controlled software or
  template updates.

LAB CONSOLE may display execution evidence produced by TEST CONSOLE, but it does
not drive instruments, acquire data, or pretend to run tests.

## TEST CONSOLE

TEST CONSOLE covers local/offline execution on a laboratory or field station. It
is optimized for dense operator work, readiness, instrumentation, acquisition,
monitoring, deviations, substitutions, reruns, and immediate evidence
publication to the local repositories.

TEST CONSOLE owns:

- selecting an assigned campaign package and test instance;
- displaying the active project, campaign, test, method revision, source
  template revision, and quality mode;
- checking readiness for the exact execution context;
- binding instruments to instrumentation chain slots and showing substitutions;
- presenting sequence steps, branches, operator prompts, limits, safety checks,
  and editable parameters allowed by lock policy;
- driving instrument adapters through the local agent and runtime boundary;
- displaying acquisition, storage, data-rate, synchronization, and alert state;
- recording raw data, corrected data, calculated results, validated results,
  deviation records, substitution records, comments, and rerun reasons;
- refusing execution with structured evidence when policy blocks the run;
- publishing execution evidence into local repositories and outbox.

Qt is the direction for TEST CONSOLE because this surface needs native desktop
behavior: dockable panes, keyboard-heavy operation, long-running state, dense
tables, plotting, device status, offline packages, and controlled station
updates.

## Shared Backbone

The two consoles share these contracts:

- Locus Local Agent write boundary for controlled operations;
- split SQLite repositories and future sync/merge packages;
- audit events and outbox operations;
- identity, role, competence, and approval evidence;
- attached document identity, ownership, storage reference, checksum, revision,
  confidentiality, and applicability;
- method revisions, template revisions, and standard references;
- metrology registry, calibration evidence, restrictions, and readiness verdict
  model;
- project, campaign, product version, test instance, and execution references;
- raw, corrected, calculated, validated, and published data lineage;
- software version, repository schema version, station package version, and
  update evidence.

Shared means one domain contract and one evidence model. It does not mean one
screen, one navigation, or one technology stack.

## What Belongs Where

| Concern | LAB CONSOLE | TEST CONSOLE |
| --- | --- | --- |
| Clients and commercial context | Owns | Reads selected package context |
| Product and product version | Owns | Reads the tested article identity |
| Project lifecycle | Owns | Reads active gates and execution mode |
| Campaign plan | Owns and freezes station package | Executes assigned package slice |
| Template authoring | Owns | Executes instances only |
| Method authoring and approval | Owns | Executes approved method revision |
| Documents and standards | Owns and controls | Reads applicable execution documents |
| Metrology records | Owns source records and reservations | Consumes readiness evidence in context |
| Instrument control | No | Owns runtime operation |
| Data acquisition | No | Owns local capture and immediate evidence |
| Deviations and substitutions | Reviews and approves by policy | Records during execution |
| Reports and publication | Owns final review and publication | Publishes execution evidence only |
| Offline field work | Prepares and reconciles packages | Operates from local package |
| Audit and sync | Reviews and resolves | Produces local execution events |
| Software updates | Defines policy and approves packages | Applies station-safe signed packages |

## Static Web Shell Boundary

The static shell may show:

- the LAB CONSOLE information architecture;
- the TEST CONSOLE boundary and handoff points;
- read-only relationship maps between clients, products, projects, campaigns,
  templates, methods, documents, personnel, metrology, planning, reports, sync,
  audit, and updates;
- harmless selection state used to demonstrate navigation;
- sample data used only to explain object hierarchy.

The static shell must not receive:

- fake instrument control;
- fake acquisition or live plotting;
- fake readiness execution flows;
- workflow buttons that imply backend writes when no backend is connected;
- ad hoc CRUD screens that bypass the target information model;
- TEST CONSOLE execution panels;
- local station configuration;
- simulated result execution flows already owned by the agent and Qt direction;
- legacy vocabulary used as the source of truth for new contracts.

Future LAB CONSOLE implementation may reuse the information architecture, but it
must be built as a real application over the agent/service backbone. Future TEST
CONSOLE implementation must remain Qt-oriented and execution-centered.
