# Service Planning and Test Categories

EMC Locus now separates two needs that were missing from the early GUI:

- planning when tests will be performed;
- maintaining an adjustable taxonomy of tests.
- keeping that planning tied to metrology and method references without forcing
  a permanent online repository connection.

## Service Planning

The project repository owns `service_schedule_items`. A planning row records:

- planning code;
- project code;
- test title;
- optional test category and method references;
- planned start and end timestamps;
- assigned operator;
- location;
- equipment under test;
- status.

Allowed status values are `planned`, `confirmed`, `in_progress`, `completed`,
and `cancelled`. Repository and GUI/CLI paths trim surrounding whitespace from
status text before validation, persistence, filtering, and audit payloads.
New planning rows must start as `planned`. Confirmation, start, completion, and
cancellation are recorded afterward through the audited status-update path, so
callers cannot create rows that have already skipped workflow evidence.

Schedule rows must use canonical `YYYY-MM-DDTHH:MM` local date-times without
timezone offsets. A single row must have a planned end after its planned start
and remain inside one business day, and the project repository enforces that
rule, the allowed status vocabulary, the initial `planned` status, and required
planning context fields even when callers bypass the GUI/CLI action layer.
Optional category and method references are trimmed when present; blank optional
references are stored as absent values rather than empty strings.
When the local action is given a test-definition repository, non-empty category
and method references must already exist in that repository before the planning
row is written.
Notes are optional operator context; missing notes are normalized to an empty
non-null text value before the planning row is written.
Repository inserts also reject blank planning item codes and check that the
referenced project exists before the schedule row is written, so direct Python
callers get the same controlled planning errors as the GUI/CLI action path.
Duplicate planning item codes are rejected through the repository path before
SQLite uniqueness constraints are reached.
Missing required planning text is handled through the same validation path
instead of surfacing raw Python attribute errors.
Repository regression coverage also proves weekend-only planning blocks are
rejected before a row can be persisted, independently from the GUI/CLI action
path.
Repository inserts also require the referenced project to already be in
`test_planning`, so direct Python callers cannot create execution planning
blocks before the contract-review gate has advanced the campaign.
Repository status updates also reject blank planning item codes before
attempting the write, avoiding silent no-op updates for malformed operator
input. Status updates also reject unknown planning item codes and orphan rows
whose project reference no longer resolves instead of mutating schedule state
without a controlled campaign context. Status updates also reject unchanged
statuses before mutating `updated_at` or appending audit evidence, so duplicate
operator submissions remain side-effect free. Once a row reaches `completed` or
`cancelled`, repository status updates reject further changes so closed
laboratory blocks cannot be reopened through direct or audited Python calls.
Status updates also enforce the sequential workflow `planned -> confirmed ->
in_progress -> completed`, with `cancelled` allowed from any non-terminal
state, so direct and audited callers cannot move planning rows backward or skip
the confirmation/start states.
Repository list filters trim project and status values when present, reject
blank project filters, and reject unknown status filters before returning
planning rows. Project-filtered list reads also reject unknown project codes
instead of returning an ambiguous empty schedule. Repository list reads also
reject orphan planning rows whose project reference no longer resolves, so a
corrupted import cannot surface schedule blocks without campaign context.
The GUI/CLI service-planning action uses the audited repository path: creating
a planning row also appends a project audit event with the operator, planning
window, EUT, status, and optional category/method references in the payload.
Repository callers that change planning status can use the audited status
update path to append a project audit event with the previous and new status in
the payload. The local Python/CLI action and Qt form use that audited path for
operator status changes, and can refresh the bootstrap after a confirmation,
start, completion, or cancellation. The Qt status form offers only actionable
update targets (`confirmed`, `in_progress`, `completed`, `cancelled`) and hides
already terminal planning rows, so completed or cancelled blocks are not
presented as editable status targets.

Example local action:

```text
$env:PYTHONPATH='python'
python -m emc_locus.actions_cli schedule-service-item `
  --projects-db local/projects.sqlite `
  --item-code PLAN-001 `
  --project-code CEM-2026-001 `
  --title "Emission conduite" `
  --test-category-code emission_conducted `
  --planned-start-at 2026-07-01T09:00 `
  --planned-end-at 2026-07-01T12:00 `
  --assigned-operator operator.one `
  --location "Lab A" `
  --equipment-under-test "EUT rail" `
  --bootstrap-output local/bootstrap.js
```

Example status update:

```text
$env:PYTHONPATH='python'
python -m emc_locus.actions_cli update-service-schedule-status `
  --projects-db local/projects.sqlite `
  --item-code PLAN-001 `
  --status confirmed `
  --actor operator.one `
  --reason "Lab slot confirmed" `
  --bootstrap-output local/bootstrap.js
```

The browser and Qt bootstrap now include a `schedule` table.

Before a project moves from contract review to test planning, EMC Locus checks
the completed contract-review items. Accredited projects require a stricter
set of items than non-accredited and investigation projects, so the workflow can
stay controlled without forcing Cofrac-style constraints on exploratory work.

## Test Categories

The test-definition repository owns `test_categories`. Categories are
hierarchical and adjustable. The default seed is:

- `emission`;
- `emission_conducted`;
- `emission_radiated`;
- `immunity`;
- `immunity_conducted`;
- `immunity_radiated`;
- additional default subfamilies for harmonics/flicker, transient time-domain
  emission, ESD, fast transients, and power-quality immunity.

Example local action:

```text
$env:PYTHONPATH='python'
python -m emc_locus.actions_cli create-test-category `
  --test-definitions-db local/test_definitions.sqlite `
  --code immunity_magnetic_field `
  --parent-code immunity_radiated `
  --label "Champ magnetique" `
  --description "Essais d immunite au champ magnetique basse frequence" `
  --sort-order 30 `
  --bootstrap-output local/bootstrap.js
```

The browser and Qt bootstrap now include a `test_categories` table so the
operator can see the taxonomy used by planning and method definitions.
