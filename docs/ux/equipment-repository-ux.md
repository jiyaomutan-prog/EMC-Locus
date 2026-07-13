# Equipment Repository UX

Release `0.13.1` changes the first operator experience from technical schema
editing to a configurable laboratory repository.

## Normal User Flow

1. Browse equipment by category tree or list.
2. Create a model through the wizard.
3. Choose root category.
4. Choose subcategory.
5. Fill the category-adapted identification form.
6. Review and create a draft.
7. Validate, submit, and approve through the existing revision workflow.

The normal flow does not expose raw JSON, checksums, schema versions, internal
category codes, or snake_case technical enums.

## Repository Administration

Repository Administration contains:

- Categories;
- Field Dictionary;
- Entry Templates;
- Defaults.

This area is for laboratory managers configuring what fields matter in the
equipment repository. It is not a physical fleet or station wiring screen.

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
