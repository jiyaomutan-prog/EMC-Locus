# Locus Lab Management Information Architecture

## Navigation Principle

Locus Lab Management is organized around laboratory objects and their
controlled relationships. It is not organized around database tables. Each
space should answer a laboratory question and expose the related objects needed
to make a traceable decision.

Locus Metrology owns the detailed asset, calibration, restriction, and
metrological aptitude domain. Locus Lab Management consumes metrology readiness,
reservations, and asset summaries for planning and project decisions, but it
should not become a duplicate metrology application.

The first Locus Lab Management navigation tree should be:

1. Clients
2. Products
3. Product Versions
4. Projects
5. Campaigns
6. Project Templates
7. Campaign Templates
8. Tested Product Templates
9. Test Templates
10. Methods
11. Standards and Documents
12. Personnel, Competences, and Roles
13. Metrology summary and resource fit
14. Planning
15. Reports and Publications
16. Synchronization, Audit, and Updates

## Clients

Objective: maintain the requesting organizations and contractual contacts used
by projects, reports, billing references, confidentiality handling, and customer
delivery.

Main objects:

- customer organization;
- site;
- contact;
- confidentiality profile;
- billing or quotation reference;
- customer access profile.

Main actions:

- create or update customer records;
- link contacts to sites and projects;
- mark confidentiality constraints;
- prepare customer-facing access without exposing laboratory internals.

Relations:

- customers own products and product versions;
- customers request projects;
- customer contacts approve scope, receive reports, or supply documents.

## Products

Objective: represent the product family or equipment type tested for a
customer.

Main objects:

- product;
- product family;
- technical owner;
- expected standards;
- reusable product evidence.

Main actions:

- create a product shell for a customer;
- link reusable documents and expected standards;
- derive product versions.

Relations:

- products belong to clients;
- products group product versions;
- project scope normally references a product version, not a loose product
  name.

## Product Versions

Objective: freeze the tested article identity used during a project or
campaign.

Main objects:

- product version;
- serial or configuration identity;
- hardware/software revision;
- representative sample;
- customer-provided documents;
- tested-product template instance.

Main actions:

- create a new version from a product or tested-product template;
- attach drawings, datasheets, firmware notes, photos, and declarations;
- mark the tested configuration as approved for a campaign.

Relations:

- product versions belong to products;
- projects and campaigns reference product versions;
- execution evidence must preserve the product version actually tested.

## Projects

Objective: manage the commercial and technical envelope from quotation to final
report delivery.

Main objects:

- project;
- quotation or contract review;
- execution mode;
- quality gates;
- deviations;
- linked campaigns;
- report package.

Main actions:

- open a project from a project template;
- complete contract review;
- select quality mode;
- approve deviations;
- create campaigns;
- review project readiness before execution;
- close or archive the project.

Relations:

- projects belong to clients;
- projects reference product versions;
- projects own campaigns and reports;
- project templates supply default structure but do not replace project
  instances.

## Campaigns

Objective: group the tests, methods, resources, and execution evidence for one
measurement campaign.

Main objects:

- campaign;
- campaign template instance;
- test instances;
- environmental conditions;
- assigned rooms, benches, operators, and instruments;
- execution package for Locus Test Station.

Main actions:

- create a campaign from a template;
- add or remove test instances before execution;
- schedule tests;
- freeze campaign scope for field/offline operation;
- receive execution evidence from Locus Test Station.

Relations:

- campaigns belong to projects;
- campaigns consume campaign templates;
- campaigns contain test instances derived from test templates;
- campaigns publish execution packages and receive executed evidence.

## Project Templates

Objective: define reusable project structures for common laboratory offerings.

Main objects:

- project template;
- template revision;
- default contract-review checklist;
- default document set;
- default campaign template links;
- variable definitions and lock policies.

Main actions:

- create, duplicate, derive, approve, retire, or supersede a template;
- instantiate a project from a template;
- update non-executed project elements under controlled rules.

Relations:

- project templates create project instances;
- project templates can include campaign templates and document requirements;
- updates must preserve executed project history.

## Campaign Templates

Objective: define reusable campaign structures without binding them to one
customer article.

Main objects:

- campaign template;
- campaign revision;
- default test template list;
- sequencing policy;
- planning assumptions;
- required roles and competence rules.

Main actions:

- create, duplicate, derive, approve, retire, or supersede campaign templates;
- instantiate campaign plans in projects;
- control updates to unexecuted test instances.

Relations:

- campaign templates are used by projects;
- campaign templates reference test templates and method revisions;
- execution instances preserve the source template revision.

## Tested Product Templates

Objective: standardize the description of equipment under test without
pretending every product is identical.

Main objects:

- tested-product template;
- identification fields;
- configuration variables;
- required customer documents;
- sample tracking rules;
- photos or inspection requirements.

Main actions:

- define reusable product identity structures;
- instantiate product version records;
- lock fields that must not drift during a campaign.

Relations:

- tested-product templates feed product versions;
- projects and campaigns consume product-version instances;
- test templates can reference product variables when needed.

## Test Templates

Objective: define reusable executable test structures.

Main objects:

- test template;
- sequence steps;
- branching rules;
- instrumentation chain slots;
- limits;
- post-processing definitions;
- editable parameters;
- variable locks.

Main actions:

- author a test template;
- link it to method revisions and standards;
- duplicate, derive, approve, retire, or supersede it;
- instantiate test instances in campaigns;
- instantiate a template from an executed real test when authorized.

Relations:

- test templates produce campaign test instances;
- test templates reference methods, limits, standards, and instrumentation slot
  requirements;
- Locus Test Station executes test instances, not bare templates.

## Methods

Objective: control laboratory methods, revisions, approval requirements, and
technical ownership.

Main objects:

- method;
- method revision;
- approval workflow;
- second-approval requirement;
- competence requirements;
- linked standards;
- validation evidence.

Main actions:

- create method revisions;
- request technical review;
- approve, second-approve, suspend, or retire revisions;
- assign methods to templates and campaigns.

Relations:

- methods reference standards and documents;
- test templates reference approved method revisions;
- people/roles determine who may author, review, approve, or execute.

## Standards And Documents

Objective: make standards, client files, PDFs, worksheets, scripts, contracts,
and project attachments first-class controlled objects.

Main objects:

- standard reference;
- document;
- document revision;
- attachment;
- document classification;
- checksum and storage reference;
- applicability link.

Main actions:

- register a standard;
- attach PDFs and customer files;
- classify documents by use;
- link documents to clients, products, product versions, projects, campaigns,
  methods, instruments, and reports;
- mark a document as applicable, superseded, or archival.

Relations:

- methods and templates consume standard references;
- product versions and projects own customer documents;
- metrology records own certificates and worksheets;
- reports cite documents and standards.

## Personnel, Competences, And Roles

Objective: configure who can do what, under which competence evidence, and how
those rights changed over time.

Main objects:

- person;
- role definition;
- role assignment;
- permission;
- competence record;
- training evidence;
- approval delegation.

Main actions:

- create or rename roles;
- assign cumulative roles to people;
- grant or revoke permissions;
- record competence evidence;
- trace approvals and role changes.

Relations:

- methods require competences for execution and approval;
- projects and campaigns assign people;
- audit events reference actors and role context;
- client-reader access is constrained by project publication rules.

## Metrology Summary And Resource Fit

Objective: consume metrology-owned inventory and evidence required for planning
and execution readiness without duplicating Locus Metrology.

Main objects:

- instrument asset summary;
- instrument category summary;
- serviceability state summary;
- calibration due/validity summary;
- readiness risk;
- external equipment reservation contact;
- attached document links owned by the shared document registry.

Main actions:

- open the source Locus Metrology record;
- reserve internal or external equipment;
- review upcoming due dates and nonconformities for planning impact.

Relations:

- test templates define instrumentation chain slots;
- campaign test instances bind slots to instruments;
- Locus Metrology owns the source evidence;
- Locus Test Station computes readiness against the execution context;
- planning consumes availability and reservation constraints.

## Planning

Objective: schedule tests, people, rooms, benches, instruments, and external
resources.

Main objects:

- planning block;
- resource reservation;
- operator assignment;
- room or bench assignment;
- external equipment reservation;
- schedule conflict.

Main actions:

- plan campaign execution;
- reserve rooms and instruments;
- assign qualified operators;
- identify conflicts;
- freeze the package sent to Locus Test Station.

Relations:

- planning links campaigns, test instances, people, instruments, and sites;
- metrology state influences planning risk;
- Locus Test Station consumes assigned work but records actual execution evidence.

## Reports And Publications

Objective: control customer-facing deliverables and their approval path.

Main objects:

- report;
- report revision;
- technical review;
- approval;
- publication package;
- export file;
- customer delivery event.

Main actions:

- prepare draft report package;
- review technical evidence;
- approve and publish;
- supersede a report revision;
- deliver files to customer-visible scope.

Relations:

- reports belong to projects;
- reports cite campaigns, test results, standards, documents, and approvals;
- publications are fed by validated and published results, not raw execution
  fragments.

## Synchronization, Audit, And Updates

Objective: supervise offline-first repository health, merge status, audit
review, and controlled software or template updates.

Main objects:

- sync operation;
- outbox event;
- repository snapshot;
- merge conflict;
- audit event;
- update package;
- rollback evidence.

Main actions:

- inspect outbox status;
- review conflicts;
- prepare offline packages;
- verify snapshots;
- validate signed updates;
- block updates during active measurement work.

Relations:

- every controlled surface emits audit and sync evidence;
- Locus Lab Management reviews and resolves;
- Locus Test Station produces local execution events that later sync or merge.
