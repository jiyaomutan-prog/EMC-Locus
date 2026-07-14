# Equipment Entry Templates

An entry template is the effective form shown for an equipment category.

Templates are derived from:

- field definitions;
- category field rules;
- inherited rules from parent categories;
- local overrides on the selected category.

Since `0.14.0`, `general_equipment` is the universal inheritance root. It owns
the common manufacturer, model, internal reference, description, and document
rules. Classification families remain distinct children and keep their own
stable `root_category_id`; this separates form inheritance from measurement
physics and catalog filtering.

`GET /api/v1/equipment/categories/{category_id}/effective-template` returns the
resolved template. LAB CONSOLE uses it in the model creation wizard and in the
Identification section of a model.

## Field Rules

`EquipmentCategoryFieldRule` controls whether a field is required, visible,
where it appears, its default value, and optional help text for a category.
Child categories inherit parent rules and can override visibility, required
state, order, group, default and help text.

`general_equipment` cannot create an equipment model directly. A model must
select a classification family or one of its descendants.

## Revision Snapshots

When a model is created from a category template, the draft revision stores:

- `custom_field_values`;
- a `template_snapshot` with category id, root id, category path, template
  checksum, captured time, and effective fields;
- normalized searchable field values in SQLite.

This keeps old approved revisions understandable after an administrator renames
a field or changes category rules.

The template snapshot is traceability metadata, not a runtime measurement
engine.
