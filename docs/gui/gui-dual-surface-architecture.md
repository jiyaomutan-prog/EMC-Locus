# GUI Dual Surface Architecture

## Purpose

EMC Locus has two user-facing products, not one generic dashboard:

- **LAB CONSOLE** is the laboratory management surface.
- **TEST CONSOLE** is the local execution surface for tests.

The two surfaces share the same domain language, audit trail, repository
strategy, identity model, templates, methods, metrology evidence, and
synchronization backbone. They differ in cadence, risk, density, and primary
operator context.

The current static web shell under `apps/gui-shell` is only an information
architecture prototype for LAB CONSOLE. It is not the long-term implementation
architecture for the laboratory web product, and it must not become the
instrument-facing execution console.

## LAB CONSOLE

LAB CONSOLE covers laboratory coordination before and after acquisition. It is
optimized for structure, traceability, review, planning, controlled changes,
and multi-person work.

LAB CONSOLE owns:

- customers, contacts, confidentiality constraints, and commercial context;
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
- metrology inventory, calibration evidence, serviceability, external
  equipment reservation contacts, and instrument document packages;
- service planning, equipment reservations, room reservations, and operator
  assignments;
- reports, publications, customer delivery packages, and revision history;
- synchronization status, audit review, repository health, and software update
  evidence.

LAB CONSOLE must make controlled business relationships visible. A user should
be able to answer which customer product version belongs to which project,
which campaign uses which method revision, which template produced which test
instance, which documents were applicable, who approved the method, and what
changed after approval.

## TEST CONSOLE

TEST CONSOLE covers execution on a laboratory or field station. It is optimized
for dense local operation, offline work, readiness checks, instrument control,
data capture, monitoring, and immediate evidence publication back to the local
repositories.

TEST CONSOLE owns:

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

Qt is the direction for TEST CONSOLE. The execution console must be able to
support native windows, dockable panes, keyboard-heavy operation, long-running
measurement state, plotting, dense status bars, and field/offline station
packaging.

## Shared Backbone

The two products share these contracts:

- local agent write boundary for controlled operations;
- split SQLite repositories and future sync/merge packages;
- audit events and outbox operations;
- identity, role, competence, and approval evidence;
- method revisions and template revision identity;
- standards and attached documents;
- metrology registry, calibration evidence, and readiness verdict model;
- project, campaign, tested product, and execution references;
- raw/corrected/calculated/validated/published data lineage;
- software version, repository schema version, and update evidence.

Shared does not mean one screen. It means one domain contract and one evidence
model consumed by different UI products.

## What Belongs Where

| Concern | LAB CONSOLE | TEST CONSOLE |
| --- | --- | --- |
| Customer and contract context | Owns | Reads selected context |
| Product and product version | Owns | Reads for execution identity |
| Project lifecycle | Owns | Reads stage and active gates |
| Campaign plan | Owns | Executes assigned campaign slice |
| Template authoring | Owns | Instantiates only through approved commands |
| Method authoring and approval | Owns | Executes approved method revision |
| Instrument readiness | Reviews and maintains source records | Calculates execution verdict in context |
| Instrument control | No | Owns runtime operation |
| Data acquisition | No | Owns local capture and immediate evidence |
| Reports and publication | Owns final review/publication | Publishes execution evidence only |
| Offline field work | Prepares packages and reconciles | Operates from local package |
| Audit and sync | Reviews and resolves | Produces local events and outbox |

## Static Web Shell Boundary

The current static shell may show:

- LAB CONSOLE navigation and object hierarchy;
- sample relationship maps between customers, products, projects, campaigns,
  templates, methods, documents, personnel, metrology, planning, reports, sync,
  audit, and updates;
- read-only prototype cards and tables that explain information architecture;
- harmless selection state used to demonstrate navigation.

The static shell must no longer receive:

- fake instrument control;
- fake acquisition state presented as if it were executable;
- workflow buttons that imply backend writes when no backend is connected;
- ad hoc CRUD screens that bypass the future LAB CONSOLE information model;
- TEST CONSOLE execution panels;
- simulated result execution flows already owned by the agent and Qt direction;
- legacy vocabulary used as the source of truth for new contracts.

Any future web implementation of LAB CONSOLE must be designed as a real
application over the agent/services backbone, not as the continuation of this
static prototype.
