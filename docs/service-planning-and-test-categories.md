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
and `cancelled`.

Schedule rows must use canonical `YYYY-MM-DDTHH:MM` local date-times without
timezone offsets. A single row must have a planned end after its planned start
and remain inside one business day, and the project repository enforces that
rule, the allowed status vocabulary, and required planning context fields even
when callers bypass the GUI/CLI action layer. Optional category and method
references are trimmed when present; blank optional references are stored as
absent values rather than empty strings.
Repository inserts also check that the referenced project exists before the
schedule row is written, so direct Python callers get the same controlled
planning error as the GUI/CLI action path. Repository status updates also
reject blank planning item codes before attempting the write, avoiding silent
no-op updates for malformed operator input. Repository list filters trim
project and status values when present, reject blank project filters, and reject
unknown status filters before returning planning rows.

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
