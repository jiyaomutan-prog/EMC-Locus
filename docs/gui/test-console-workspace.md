# TEST CONSOLE Workspace

## Direction

TEST CONSOLE is the Qt desktop workspace for local and field execution. It
is not a project dashboard and not a metrology registry editor. It is the
operator station used when a test is being prepared, run, monitored, corrected,
rerun, or published as execution evidence.

The design must support offline execution from local repositories, dense
instrument state, long-running measurements, live warnings, multiple data
streams, and controlled publication back to the local agent.

The normal handoff is: LAB CONSOLE prepares and freezes a campaign/test package;
TEST CONSOLE imports or opens that package locally; TEST CONSOLE executes and
publishes evidence back to the local repositories; LAB CONSOLE reviews,
reconciles, validates, reports, and publishes.

## Implemented Preparation Slice

Version `0.17.0` implements the first narrow part of this direction as
`Préparation du poste`. It is deliberately a physical setup workflow rather
than a dashboard:

- select real serial-numbered materials from metrology;
- retain the approved equipment-model revision that defines their ports;
- assign a laboratory role to each material;
- connect a compatible output or through port to an input or through port;
- select the applicable conversion or frequency response measured for that
  serial number;
- inspect the structured readiness verdict;
- freeze an immutable revision as `Prêt à câbler`.

This slice does not yet open a campaign package, control hardware, acquire
samples, calculate an FFT, apply a correction, or publish a result.

## One-Screen Layout

The ideal single-screen layout uses a dense docked workstation pattern:

- top status bar for context that must never disappear;
- left navigation rail for assigned campaign, test list, sequence steps, and
  branches;
- central workspace for the active test procedure, acquisition/plot area, and
  operator prompts;
- right dock for instrumentation chain, readiness verdict, alerts, and manual
  substitutions;
- bottom event strip for command log, storage state, acquisition state, and
  audit notes.

The operator must not have to leave this screen to answer:

- which campaign and test are active;
- which quality mode applies;
- whether the station is offline;
- whether readiness is blocking;
- which instruments are required, bound, substituted, or unavailable;
- whether acquisition and storage are healthy;
- whether deviations or nonconformities exist;
- whether the current result can be published.

The operator may drill into source documents or metrology evidence, but source
record maintenance belongs to LAB CONSOLE unless a station-local emergency
policy explicitly grants a controlled, audited exception.

## Always-Visible States

The following states stay visible at all times:

- campaign code and project code;
- active test instance and source template revision;
- method revision and approval status;
- quality mode: accredited, non-accredited, or investigation;
- readiness verdict: ready, warning, blocked, refused, or expired;
- instrumentation chain state;
- offline/local/reference repository state;
- disk target, free space, and write permission;
- acquisition state: idle, armed, running, paused, stopped, failed;
- active data source count and synchronization state;
- alert count grouped by blocking, warning, and information;
- current operator and role context;
- software version and local agent connection state.

## Panels And Tabs

### Campaign Panel

Shows the assigned campaign package, test queue, execution order, already
executed tests, blocked tests, reruns, and superseded runs.

### Procedure Panel

Shows the selected test instance with sequence steps, prerequisites, operator
prompts, branching decisions, limits, and editable parameters allowed by the
template lock policy.

### Instrumentation Panel

Shows the required chain slots and bound instruments:

- slot name and required category/capability;
- selected asset;
- serviceability;
- calibration validity;
- missing evidence;
- nonconformity;
- communication endpoint;
- safety limits;
- substitution status.

Source metrology records are read-only here. The panel can request or record an
execution substitution, but it must not silently edit calibration, service state,
reservation contact, or certificate records.

### Readiness Panel

Displays the derived pre-flight verdict for this exact execution context. The
panel must show structured refusal details:

- refusal code;
- human message;
- issue list;
- affected instruments;
- affected dimensions such as serviceability, calibration validity, missing
  evidence, nonconformance, authorization, storage, or offline package;
- who may override or relax the issue when policy allows it.

### Acquisition Panel

Displays the active measurement view. Future layouts can include frequency
sweeps, time traces, FFT views, harmonic tables, event counters, inrush
captures, and synchronized DAQ channel views.

### Data And Result Panel

Separates:

- raw data;
- corrected data;
- calculated result;
- validated result;
- published result.

The operator must see which state exists and which state is still missing.

### Event And Audit Panel

Shows command observations, manual events, substitutions, deviations, refusal
records, rerun reasons, and publication attempts. It must be append-only from
the operator point of view, with corrections represented as new records.

## Operator Actions

An operator may:

- select an assigned campaign/test package;
- prepare the station;
- run pre-flight readiness;
- bind allowed instruments to chain slots;
- launch an approved test instance when readiness permits;
- pause, stop, and annotate execution;
- record a deviation when permitted;
- request material substitution;
- create a rerun request with reason;
- publish execution evidence to the local repositories.

An operator may not:

- approve method revisions;
- edit source templates;
- silently remove readiness evidence;
- publish final customer reports;
- bypass audit creation.

## Admin Actions

An admin may:

- configure station endpoints and local storage paths;
- manage local package imports;
- troubleshoot local agent connectivity;
- mark station maintenance state;
- install signed updates when policy permits;
- grant station-local emergency access according to policy.

Admin actions remain audited and do not imply technical or quality approval.

## Investigation Actions

Investigation mode may:

- loosen non-safety constraints;
- run exploratory sequences;
- allow provisional instruments when documented;
- mark results as investigation-only;
- block direct report publication until reviewed.

Investigation mode must not hide the relaxed constraints. It must preserve why
the run was exploratory and which rules were not applied.

## Deviations And Substitutions

Deviations and substitutions are first-class execution records.

A deviation record captures:

- execution context;
- affected step or result;
- rule or expectation being deviated from;
- reason;
- actor;
- required approver when applicable;
- effect on reportability.

A material substitution record captures:

- original required chain slot;
- planned instrument when one existed;
- substituted instrument;
- readiness verdict for the substitute;
- reason for substitution;
- reservation/contact evidence for external equipment if applicable;
- approval requirement and approval status.

The execution can continue only if the quality mode and method policy allow the
deviation or substitution state. Otherwise TEST CONSOLE records a structured
refusal rather than presenting a fake green workflow.
