# Template And Execution Definition Backbone

## Purpose

EMC Locus needs a stable backbone for templates, methods, execution instances,
and results before adding more runtime slices. This document defines the target
business vocabulary that should guide future schemas, routes, Qt models, and
Locus Lab Management screens.

The key rule is simple: generic definitions, project instances, executed
instances, and results are different objects. They may reference each other, but
they must not be collapsed into one mutable record.

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
by Locus Test Station or another controlled execution path.

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
