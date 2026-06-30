# GUI Product Surface Architecture

## Purpose

EMC Locus has three user-facing products, not one generic dashboard:

- **Locus Metrology** is the metrology and asset aptitude surface.
- **Locus Lab Management** is the laboratory management surface.
- **Locus Test Station** is the local Qt execution surface for tests.

The three surfaces share the same domain language, audit trail, repository
strategy, identity model, attached document registry, templates, methods,
metrology evidence, and synchronization backbone. They differ in cadence, risk,
density, and primary operator context.

The current static web shell under `apps/gui-shell` is only an information
architecture prototype for the product-family navigation and Locus Lab
Management object hierarchy. It is not the long-term implementation
architecture for the web products, and it must not become the
instrument-facing execution console.

## Locus Metrology

Locus Metrology covers the controlled life of measurement means. It is
optimized for asset identity, calibration evidence, restrictions, service
state, traceability, external reservation contacts, and metrological aptitude.

Locus Metrology owns:

- instrument assets and categories;
- calibration events, certificates, providers, uncertainty summaries, and
  due-date policy;
- serviceability, restrictions, nonconformities, and quarantine decisions;
- datasheets, certificates, worksheets, transducer files, and scripts attached
  to assets;
- external equipment contact and reservation evidence;
- source records used by readiness calculations.

## Locus Lab Management

Locus Lab Management covers laboratory coordination before and after
acquisition. It is optimized for structure, traceability, review, planning,
controlled changes, and multi-person work.

Locus Lab Management owns:

- customers, contacts, confidentiality constraints, and commercial context;
- requests, quotations, communications, and contract review;
- tested products, product versions, and customer product evidence;
- projects from quotation to report delivery;
- campaigns and campaign plans;
- project templates, campaign templates, tested-product templates, and test
  templates;
- method revisions and approval workflows;
- standards, client documents, PDFs, drawings, datasheets, contracts, and
  project attachments as first-class objects;
- laboratory personnel, configurable roles, competences, authorizations, and
  approval delegation;
- resource assignment using metrology-owned asset state;
- service planning, equipment reservations, room reservations, and operator
  assignments;
- reports, publications, customer delivery packages, and revision history;
- synchronization status, audit review, repository health, and software update
  evidence.

Locus Lab Management must make controlled business relationships visible. A
user should be able to answer which customer product version belongs to which
project, which campaign uses which method revision, which template produced
which test instance, which documents were applicable, who approved the method,
and what changed after approval.

## Locus Test Station

Locus Test Station covers execution on a laboratory or field station. It is
optimized for dense local operation, offline work, readiness checks, instrument
control, data capture, monitoring, and immediate evidence publication back to
the local repositories.

Locus Test Station owns:

- selecting an assigned campaign and test instance for execution;
- presenting the active quality mode and any approved relaxation;
- checking metrology readiness for the exact execution context;
- presenting the required and substituted instrumentation chain;
- guiding sequence steps, branches, limits, operator prompts, and safety
  interlocks;
- driving simulated and real instrument adapters through the local agent;
- displaying acquisition state, storage state, data rates, alerts, and command
  observations;
- collecting raw data, corrected data, calculated results, validated results,
  deviations, and operator comments;
- recording reruns, supersession links, substitutions, and refusal evidence;
- publishing executed evidence into the local repositories and outbox.

Qt is the direction for Locus Test Station. The execution console must be able
to support native windows, dockable panes, keyboard-heavy operation,
long-running measurement state, plotting, dense status bars, and field/offline
station packaging.

## Shared Backbone

The three products share these contracts:

- local agent write boundary for controlled operations;
- split SQLite repositories and future sync/merge packages;
- audit events and outbox operations;
- identity, role, competence, and approval evidence;
- attached document identity, ownership, storage reference, checksum, revision,
  confidentiality, and applicability;
- method revisions and template revision identity;
- standards and attached documents;
- metrology registry, calibration evidence, and readiness verdict model;
- project, campaign, tested product, and execution references;
- raw/corrected/calculated/validated/published data lineage;
- software version, repository schema version, and update evidence.

Shared does not mean one screen. It means one domain contract and one evidence
model consumed by different UI products.

## What Belongs Where

| Concern | Locus Metrology | Locus Lab Management | Locus Test Station |
| --- | --- | --- | --- |
| Customer and contract context | Reads when needed | Owns | Reads selected context |
| Product and product version | Reads when needed | Owns | Reads for execution identity |
| Project lifecycle | Reads assignments | Owns | Reads stage and active gates |
| Campaign plan | Supplies asset constraints | Owns | Executes assigned campaign slice |
| Template authoring | Owns metrology templates only | Owns project/campaign/test templates | Executes instances only |
| Method authoring and approval | Contributes calibration/readiness rules | Owns approval workflow | Executes approved method revision |
| Instrument source records | Owns | Consumes for planning | Consumes for readiness |
| Instrument readiness | Owns source evidence | Reviews risk and reservations | Calculates execution verdict in context |
| Instrument control | No | No | Owns runtime operation |
| Data acquisition | No | No | Owns local capture and immediate evidence |
| Reports and publication | Supplies metrology evidence | Owns final review/publication | Publishes execution evidence only |
| Offline field work | Supplies signed asset snapshots | Prepares packages and reconciles | Operates from local package |
| Audit and sync | Produces controlled asset events | Reviews and resolves | Produces local execution events |

## Static Web Shell Boundary

The current static shell may show:

- the three-product surface map;
- Locus Lab Management navigation and object hierarchy;
- sample relationship maps between customers, products, projects, campaigns,
  templates, methods, documents, personnel, metrology, planning, reports, sync,
  audit, and updates;
- read-only prototype cards and tables that explain information architecture;
- harmless selection state used to demonstrate navigation.

The static shell must no longer receive:

- fake instrument control;
- fake acquisition state presented as if it were executable;
- workflow buttons that imply backend writes when no backend is connected;
- ad hoc CRUD screens that bypass the future Locus information model;
- Locus Test Station execution panels;
- simulated result execution flows already owned by the agent and Qt direction;
- legacy vocabulary used as the source of truth for new contracts.

Any future implementation of Locus Metrology or Locus Lab Management must be
designed as a real application over the agent/services backbone, not as the
continuation of this static prototype.
