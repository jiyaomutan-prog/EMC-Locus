# Equipment Repository UX

Release `0.13.2` polishes the first operator experience from technical schema
editing to a usable configurable laboratory repository. It does not add a
physical asset fleet, station wiring, acquisition runtime, reporting, RBAC, or
sync.

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
- `Formulaire`: visible category fields, required/optional flags, grouping and
  direct rule removal;
- `Previsualisation`: the form a technician will see when creating equipment in
  that category;
- `Diagnostic avance`: internal IDs, checksums and inherited rule details.

This area is for laboratory managers configuring what fields matter in the
equipment repository. It is not a physical fleet or station wiring screen.

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
3. Technical identifiers are shown only in expandable advanced options and in
   Diagnostic avance.
4. Choice fields use a visible list editor for values such as `Faible`,
   `Normale`, `Haute`, and `Critique`.

`custom_criticality` is not a normal product UI default.

## Form Preview

Previsualisation shows the final technician form using business labels and
required markers. It intentionally hides template checksum, internal field IDs,
field codes, category IDs, revision IDs, schema versions, and raw JSON. Those
details remain available in Diagnostic avance.

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
