# Template And Execution Definition Backbone

## Purpose

EMC Locus needs a stable backbone for templates, methods, execution instances,
and results before adding more runtime slices. This document defines the target
business vocabulary that should guide future schemas, routes, Qt models, and
LAB CONSOLE screens.

The key rule is simple: generic definitions, project instances, executed
instances, and results are different objects. They may reference each other, but
they must not be collapsed into one mutable record.

The second key rule is that a template is never the thing being executed.
Execution always happens against a campaign test instance that preserves its
source template revision and its local parameter values at the time the package
was frozen.

## Object Definitions

### Project Template

A reusable definition for opening a project structure.

Contains:

- template identity and revision;
- project type;
- default contract-review checklist;
- default campaign template links;
- default document requirements;
- variables and lock policies;
- approval and retirement state.

Creates: project instances.

### Campaign Template

A reusable definition for a campaign plan.

Contains:

- campaign purpose;
- default test template list;
- suggested sequence;
- planning assumptions;
- required roles and competences;
- offline package requirements;
- variables and lock policies.

Creates: campaign instances inside projects.

### Tested Product Template

A reusable structure for describing equipment under test.

Contains:

- identification fields;
- configuration variables;
- required customer documents;
- sample tracking rules;
- lock policy for critical identity fields;
- derivation history.

Creates: tested product or product-version instances.

### Test Template

A reusable executable test definition.

Contains:

- stable template identity, independent from content revision numbers;
- identity title used as the business-facing library name;
- revision history with deterministic revision numbers and explicit parent
  revision links;
- definition title used as the technical title of a specific revision;
- revision status, where draft content is editable and submitted/approved
  content is immutable;
- canonical definition JSON and SHA-256 definition checksum;
- source method revision;
- standard references;
- instrumentation chain slots;
- execution sequence steps;
- branching rules;
- editable parameters;
- limits;
- post-processing definitions;
- required raw/corrected/calculated/validated/published data states;
- rerun and supersession policy.

Creates: test instances inside campaign instances.

0.9.0 implements the first real test-template aggregate in `emc-locus-core` and
the local agent. The runtime now persists `test_template_identities`,
`test_template_revisions`, and `test_template_audit_events` instead of one
mutable row per template. The definition is typed in Rust for measurement axis,
variables, constraints, lock policy, instrumentation slots, calibration
requirements, execution steps, branch rules, limits, post-processing operations,
and revision status. The future LAB CONSOLE editor must build on this
aggregate, not on the retired 0.8.x JSON-column shape.

0.9.1 keeps that scope deliberately narrow and hardens the aggregate boundary:
draft edits and status transitions are compare-and-swap writes, each template
can have only one active draft, the aggregate read model distinguishes current
approved, latest, and active draft revisions, and approval of a newer revision
supersedes older approved revisions with audit/outbox evidence. It is still not
a Template Studio editor or an execution runtime.

0.10.0 adds LAB CONSOLE Template Studio v1 on top of this aggregate. The editor
is a React/TypeScript client served by the local Rust agent and uses only the
public `/api/v1/test-templates` routes. It can create or clone template
identities, edit a draft through structured sections, validate definitions on
the server, save with an expected definition checksum, submit, approve, derive a
new draft revision, and inspect revision history and audit. This still does not
instantiate a campaign test or freeze an immutable execution package; that is
the next domain step.

### Method Revision

A controlled laboratory method at a specific revision.

Contains:

- method identity;
- revision;
- author;
- technical reviewer;
- approval status;
- optional second-approval requirement;
- competence requirements;
- validation evidence;
- linked standards and documents;
- retirement or suspension state.

Referenced by: test templates and executed test instances.

### Standard Reference

A controlled reference to a standard, client requirement, internal instruction,
or method family.

Contains:

- reference code;
- title;
- revision or edition;
- issuing body or source;
- applicability notes;
- supersession links;
- attached documents when licensed or allowed.

Referenced by: methods, templates, projects, reports, and deviations.

### Attached Document

A first-class controlled file or document record.

Contains:

- document identity;
- classification such as client document, standard, certificate, datasheet,
  worksheet, script, report, photo, drawing, or contract;
- file reference;
- checksum;
- revision;
- owner domain;
- applicability link;
- confidentiality and publication rules.

Referenced by: almost every major domain object.

### Variable Definition

A named parameter used by templates and instances.

Contains:

- variable name;
- data type;
- unit;
- default value;
- allowed range or enum;
- source template revision;
- required/optional state;
- human label and description.

Referenced by: templates, campaign/test instances, sequence steps, limits, and
post-processing definitions.

### Variable Lock Policy

Defines when a variable can change and who can change it.

Policy examples:

- locked at project creation;
- editable until campaign freeze;
- editable until first execution;
- editable only in investigation mode;
- editable only with technical approval;
- derived from an executed run and no longer editable.

Every variable change after instantiation must preserve previous value, new
value, actor, reason, and affected instances.

### Limit Definition

A controlled expectation used during or after execution.

Contains:

- limit identity;
- expression or threshold;
- unit;
- dimension;
- applicable frequency/time/channel range;
- uncertainty or guard-band policy;
- source standard or method reference;
- pass/fail classification;
- applicability condition.

Referenced by: test templates, execution instances, post-processing, and
validated result decisions.

### Post-Processing Definition

A controlled data transformation definition.

Contains:

- operation type such as correction, FFT, windowing, resampling, harmonic
  calculation, event counting, detector calculation, channel math, or averaging;
- input data state requirements;
- parameters;
- software implementation reference;
- expected output;
- lineage rules;
- checksum or revision identity.

Referenced by: test templates and executed processing records.

### Execution Sequence Step

One step in the operator or automation sequence.

Contains:

- step identity;
- order;
- instruction;
- required roles or acknowledgement;
- instrument commands or manual actions;
- acquisition trigger;
- expected evidence;
- blocking conditions;
- allowed branches.

Referenced by: test template revisions and executed step records.

### Branching Rule

A rule that chooses the next step or execution path.

Contains:

- condition expression;
- source variable, measurement, readiness issue, or operator decision;
- destination step;
- audit requirement;
- policy for manual override.

Referenced by: sequence steps and executed branch decisions.

### Instrumentation Chain Slot

A required or optional position in the measurement chain.

Contains:

- slot identity;
- required instrument category or capability;
- quantity measured or generated;
- calibration requirement;
- communication requirement;
- safety limit requirements;
- allowed substitutions;
- external equipment reservation requirement when relevant.

Referenced by: test templates, campaign test instances, readiness verdicts, and
executed instrumentation snapshots.

### Rerun And Supersession Model

Rerun and supersession must be explicit.

A rerun records:

- original execution;
- reason;
- requested by;
- authorized by when required;
- changed context;
- link to new execution.

A supersession records:

- superseded result or execution;
- superseding result or execution;
- reason;
- reportability effect;
- actor and approval evidence.

No executed result should be overwritten to look as if it never existed.

## Definition, Instance, Execution, Result

### Generic Definition

A generic definition is a reusable template or method revision. It can be
drafted, reviewed, approved, retired, duplicated, or derived. It is not
execution evidence.

Examples:

- project template;
- campaign template;
- tested product template;
- test template;
- method revision;
- standard reference.

### Project Instance

A project instance is created for real customer work. It preserves its source
template revision but evolves under project controls.

Examples:

- project;
- campaign;
- product version;
- planned test instance;
- planned instrumentation binding;
- planned document requirement.

### Executed Instance

An executed instance is the evidence of what actually happened. It is produced
by TEST CONSOLE or another controlled execution path.

Examples:

- execution attempt;
- readiness verdict;
- instrument snapshot;
- step execution record;
- branch decision;
- command observation;
- raw dataset capture;
- rerun link;
- deviation record.

### Result

A result is a derived business object produced from execution evidence and
review decisions. It must keep lineage to executed instances.

Examples:

- calculated value;
- pass/fail verdict;
- validated result;
- published result;
- report table entry.

## Data State Vocabulary

### Raw Data

Data acquired or imported as original evidence. It is immutable after capture
except through reviewed retention workflows.

Examples:

- receiver trace file;
- openDAQ time stream;
- oscilloscope capture;
- event log;
- instrument command log.

### Corrected Data

Data transformed by traceable corrections.

Examples:

- antenna factor applied;
- transducer worksheet applied;
- cable loss compensated;
- gain correction applied;
- time alignment applied.

### Calculated Result

Computed value derived from raw or corrected data.

Examples:

- peak level;
- harmonic component;
- FFT bin magnitude;
- inrush maximum;
- limit margin;
- event count.

### Validated Result

A calculated result accepted through the applicable technical or quality rule.
Validation may require method conformance, reviewer approval, deviation review,
or second approval.

### Published Result

A validated result included in a customer-facing publication or report export.
Publication creates a separate traceable state. A published result can be
superseded, but must not be silently edited.

## Controlled Update Of Templates

Templates may be duplicated, derived, or revised. Project elements that have not
yet been executed may receive controlled updates if policy allows it.

The update operation must know:

- source template revision;
- target instances;
- executed or unexecuted state of each target;
- fields affected;
- actor and reason;
- whether approval is required;
- result of the update;
- instances intentionally left unchanged.

Executed instances remain historical evidence. They do not move when a template
changes.

## Template Lifecycle

Every template family should support these lifecycle states:

- draft;
- under review;
- approved;
- suspended;
- superseded;
- retired.

The lifecycle must record author, reviewer, approver, approval policy,
effective date, supersession link, and reason. A suspended or retired template
cannot create new instances, but existing project history remains readable.

## Instantiation Rules

Instantiation creates a new business object with a reference to the exact source
revision:

- project template to project;
- campaign template to campaign;
- tested-product template to product version;
- test template to campaign test instance.

The instance receives copied variable values, lock policies, required document
links, method revision references, instrumentation slot requirements, limits,
post-processing definitions, and sequence steps. Later template changes do not
magically rewrite the instance.

## Controlled Propagation To Unexecuted Work

A future propagation operation may update non-executed instances when policy
allows it. The operation must show:

- affected project, campaign, or test instances;
- source template revision and target revision;
- changes to variables, locks, steps, limits, methods, documents, or
  instrumentation slots;
- execution state of every candidate target;
- refusal reason for targets already executed or manually locked;
- actor, reason, approval evidence, audit event, and sync operation.

The UI should present this as a controlled change set, not as a silent bulk edit.

## Execution Package Boundary

Before TEST CONSOLE runs a test, LAB CONSOLE should be able to freeze an
execution package containing:

- campaign and project identity;
- product version identity;
- test instance identity;
- source test template revision;
- method revision and approval state;
- required documents and standards;
- instrumentation chain slots;
- planned or reserved instruments;
- quality mode and allowed relaxations;
- variables and lock policy;
- sequence, branch rules, limits, and post-processing definition references;
- local storage and synchronization expectations.

TEST CONSOLE may record what actually happened, including substitutions and
deviations, but it must not rewrite the package as if it had been planned that
way from the start.
