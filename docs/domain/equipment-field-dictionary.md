# Equipment Field Dictionary

Release `0.14.0` exposes `EquipmentFieldDefinition` as an editable field
dictionary for repository forms.

Each field definition stores:

- `field_id` and stable `field_code`;
- label and description;
- data type;
- scope;
- required/visible defaults;
- uniqueness flag;
- unit quantity and allowed units;
- choice values;
- optional validation regex and default value;
- display group/order;
- active and system-defined flags.

Supported data types are `short_text`, `long_text`, `number`,
`number_with_unit`, `date`, `boolean`, `choice`, `multi_choice`, `url`,
`file_reference`, and `object_reference`.

The `equipment_model` scope is wired into the category-driven creation UX.
Other scopes are reserved for physical assets, metrology records, station
connections, driver profiles, sensor definitions, DAQ channel profiles, and
acquisition recipes.

The Rust core validates basic field contracts: identifiers and labels are
required, choice fields need non-empty options, unit fields need allowed units,
and default values must match the declared field type.

`file_reference` uses a typed content-addressed manifest containing
`object_id`, `original_filename`, `mime_type`, `size_bytes`, lowercase SHA-256,
and `storage_key`. The local agent stores the actual payload and the equipment
model revision stores the manifest. An arbitrary URL is not valid evidence.

Field definitions can be created, edited, and archived. Archival is used
instead of destructive deletion so approved revision snapshots remain
readable. `manufacturer` and `model_name` are structural identifiers required
by the equipment-model aggregate and cannot be archived; their labels and help
text remain editable.
