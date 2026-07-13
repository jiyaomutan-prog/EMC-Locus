# Equipment Field Dictionary

Release `0.13.1` adds `EquipmentFieldDefinition` as the configurable field
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

In `0.13.1`, only the `equipment_model` scope is wired into the creation UX.
Other scopes are reserved for physical assets, metrology records, station
connections, driver profiles, sensor definitions, DAQ channel profiles, and
acquisition recipes.

The Rust core validates basic field contracts: identifiers and labels are
required, choice fields need non-empty options, unit fields need allowed units,
and default values must match the declared field type.
