# Equipment Repository UX

Release `0.14.0` adds the editable universal `Général` inheritance root, actual
document upload, explicit required/optional semantics, field editing and
archival, and the first physical-material registration flow. It does not add
station wiring, acquisition runtime, reporting, RBAC, or final sync.

## Normal User Flow

1. Browse equipment by category tree or list.
2. Create a model through the wizard.
3. Choose the equipment family from a radio/list choice, not from action
   buttons.
4. Choose any leaf or intermediate subcategory from the tree.
5. Fill the category-adapted identification form.
6. Review and create a draft.
7. Validate, submit, and approve through the existing revision workflow.

The normal flow does not expose raw JSON, checksums, schema versions, internal
category codes, field codes, checksums, revision IDs, schema versions, or
snake_case technical enums.

## Repository Administration

Administration du referentiel is centered on the selected category:

- `Informations`: category label, description, parent category and active
  state;
- `Sous-categories`: direct children and child creation under any selected
  category;
- `Formulaire`: field creation/editing/archival, visible category fields,
  required/optional flags, grouping, inheritance origin and direct rule
  removal;
- `Aperçu`: the technician form plus repliable technical evidence.

The category tree starts at `Général`; every classification family inherits its
rules. The physical-material tab is separate because a reusable model and a
serial-numbered laboratory asset have different lifecycles.

## Category Tree Behavior

The category tree supports arbitrary depth. Rows are selectable rows rather than
buttons, with indentation, folder icons, expand/collapse controls, visible
selection, hover state, keyboard selection, and a `...` menu for contextual
actions. A user can create a child category under any existing category and can
select intermediate or leaf categories in the model wizard.

## Field Dictionary Workflow

Field creation is label-first:

1. Enter the business label, description/help text, field type, choices or
   units, and display group.
2. The internal field code is generated automatically from the label.
3. Technical identifiers are shown only in expandable advanced options.
4. Choice fields use a visible list editor for values such as `Faible`,
   `Normale`, `Haute`, and `Critique`.

`custom_criticality` is not a normal product UI default.

## Form Preview

Aperçu shows the final technician form using business labels and explicit
`Obligatoire`/`Optionnel` badges. It hides template checksum, internal field
IDs, field codes, category IDs, revision IDs, schema versions, and raw JSON
until `Informations techniques` is expanded.

## Physical Materials

`Matériels réels` starts from an approved equipment model and requires an asset
reference and serial number. The form copies manufacturer, model, family,
category, and capabilities into the metrology registration contract, then asks
for calibration applicability, periodicity, warning window and serviceability.
Calibration events and certificates continue through the metrology domain.

## Demo Data

Fresh storage has no demo models, sensors, drivers or recipes. Demonstration
records are created only by explicit seed actions such as
`.\scripts\start-lab.ps1 -SeedEquipmentDemo` or `-SeedMeasurementDemo`.
Equipment demo records carry `is_demo = true`, and LAB CONSOLE provides hide,
show and demo-only filters.

## Advanced Diagnostics

The technical model is preserved. Equipment class, functional role, signal
domains, technology tags and raw JSON remain available in Advanced /
Diagnostics for developers and method engineers.
