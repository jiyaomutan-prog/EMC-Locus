# People, Roles, Rights, Competences, And Approvals

## Purpose

EMC Locus must treat people, roles, permissions, competences, and approvals as
controlled business objects. A laboratory cannot rely on hard-coded job titles
or hidden UI assumptions when execution, method approval, deviations, and report
publication depend on authorization.

The default role names below are seed examples, not fixed product limits.
Laboratories must be able to rename roles, create new roles, combine roles, and
change role assignments over time while preserving audit history.

Screens may display local role labels, but services should authorize decisions
with stable permissions, scope, competence evidence, and approval policy. A
renamed role must not change the meaning of historical approvals.

## Core Objects

### Person

Represents a human actor.

Fields:

- identity;
- display name;
- email or login;
- active/suspended state;
- organization;
- optional client visibility flag.

### Role Definition

A configurable named bundle of permissions.

Fields:

- role code;
- display name;
- description;
- active state;
- permission set;
- effective date;
- supersession link when renamed or replaced.

### Role Assignment

Links a person to a role.

Fields:

- person;
- role definition;
- scope such as laboratory, project, campaign, method family, or customer;
- effective from/to;
- assigned by;
- reason;
- revocation evidence.

Assignments are cumulative. A person may be operator and project manager on one
campaign, but only reader on another.

### Permission

An atomic authorization used by services and UI policy.

Examples:

- manage customers;
- create projects;
- approve contract review;
- author templates;
- approve method revision;
- second-approve method revision;
- edit metrology records;
- mark instrument out of service;
- schedule service;
- run assigned tests;
- approve deviations;
- validate results;
- publish report;
- manage updates;
- resolve sync conflicts.

### Competence Record

Evidence that a person may perform a technical task.

Fields:

- person;
- competence area;
- method, instrument category, test family, or role scope;
- evidence document;
- issuer or evaluator;
- issue date;
- expiry date when applicable;
- status.

### Approval Record

Evidence that an actor approved a controlled object or decision.

Fields:

- approved entity;
- approval type;
- actor;
- actor role context;
- competence evidence when required;
- timestamp;
- decision;
- reason/comment;
- second-approval relation when applicable.

## Baseline Role Examples

These names are recommended defaults only. The product should store permissions
and role definitions separately from labels.

### Admin

Typical permissions:

- manage users and role definitions;
- configure station and repository settings;
- install signed software packages when policy permits;
- manage local system health.

Limits:

- admin rights do not imply technical method approval;
- admin rights do not imply quality approval.

### Laboratory Manager

Typical permissions:

- supervise projects and planning;
- assign people and resources;
- approve laboratory-level deviations when policy allows;
- review laboratory workload and conflicts.

### Technical Manager

Typical permissions:

- own method families;
- review or approve method revisions;
- approve technical deviations;
- validate technical results;
- define competence requirements.

### Quality Manager

Typical permissions:

- configure quality modes and gate policies;
- approve quality deviations;
- review audit records;
- approve report publication policy;
- review nonconformities.

### Metrology Manager

Typical permissions:

- register instrument categories and assets;
- record or validate calibration evidence;
- mark serviceability;
- review nonconformities and due dates;
- approve external equipment acceptance rules.

### Project Manager

Typical permissions:

- open and maintain projects;
- complete or coordinate contract review;
- create campaigns from templates;
- plan resources;
- communicate with the customer;
- prepare report package context.

### Operator

Typical permissions:

- run assigned tests;
- execute readiness checks;
- bind instruments when allowed;
- record observations, substitutions, and deviations;
- publish execution evidence to local repositories.

Limits:

- cannot approve methods unless another role grants that permission;
- cannot silently bypass blocking readiness;
- cannot publish final customer reports by default.

### Client Reader

Typical permissions:

- view published documents or report packages explicitly shared with the
  customer;
- download approved deliverables;
- view limited project status if configured.

Limits:

- no access to internal audit details, non-published raw data, personnel
  records, or internal deviations unless explicitly shared.

## Configurable And Mutable Roles

The system must support:

- renaming a role without rewriting historical events;
- retiring a role while preserving old assignments;
- creating site-specific or customer-specific roles;
- assigning multiple roles to one person;
- scoping roles to project, campaign, method family, or station;
- temporary delegation with expiry;
- emergency access with strict audit evidence.

Historical records should display the role label that was effective at the time
of the decision, plus the stable role identity.

## Approval Policy

Approvals must be policy-driven. A method may require:

- no approval while draft-only;
- one technical approval;
- one technical approval plus quality approval;
- one technical approval plus configurable second approval;
- reapproval after template or standard changes.

The policy must specify:

- required permission;
- required competence;
- separation-of-duty rule if one exists;
- whether the same person can author and approve;
- second-approval conditions;
- expiry or revalidation rule.

## Console Authorization Boundary

LAB CONSOLE is the normal place for:

- defining and renaming roles;
- assigning permissions and scopes;
- recording competences and evidence;
- approving methods, templates, deviations, reports, and publications;
- reviewing role history and approval traceability.

TEST CONSOLE consumes the resulting authorization context locally. It may allow
an operator to run assigned tests, record observations, request deviations,
request substitutions, and publish execution evidence only when the local package
and current role context allow it. TEST CONSOLE must not invent new role
definitions or silently grant itself method, quality, or metrology approval
power.

## Traceability Requirements

Every controlled role, competence, and approval change must record:

- actor;
- timestamp;
- target person or object;
- previous value;
- new value;
- reason;
- source document when applicable;
- repository operation id;
- audit event id;
- outbox operation when synchronization is required.

Role changes are not retroactive. They affect future decisions and may explain
current authorization, but they must not rewrite the authorization context of a
past execution, method approval, deviation, or report publication.
