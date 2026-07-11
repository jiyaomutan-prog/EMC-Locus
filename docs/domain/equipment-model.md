# Equipment Model Definition

Release `0.11.0` introduces a revisioned equipment model catalog. An equipment
model describes an engineering type of laboratory equipment, not a physical
asset in the fleet and not its metrology record.

## Concepts

- `EquipmentModelIdentity`: stable identity with `equipment_model_id`,
  manufacturer, model name, optional variant, equipment class, category, creator
  and current approved revision pointer.
- `EquipmentModelRevision`: immutable content revision after submission,
  carrying `revision_id`, deterministic revision number, optional parent
  revision, lifecycle status, canonical definition JSON, SHA-256 checksum,
  timestamps and creator.
- `EquipmentModelDefinition`: typed Rust core aggregate with engineering
  specifications, signal ports, communication interfaces, capabilities and
  controlled metadata.
- `functional_role`: physics/chain role such as `energy_source`,
  `signal_source`, `rf_network_element`, `sensor`, `actuator`,
  `measurement_instrument`, `acquisition_device`, `converter`,
  `control_system`, `software_system`, `facility` or `manual_accessory`.
  This is distinct from `equipment_class`.

## Scope Boundary

An equipment model says what a type of equipment is and what it can do. It does
not represent:

- a physical asset with a serial number;
- a calibration event or certificate;
- a station connection binding;
- a campaign-time readiness verdict.

Those links remain future vertical slices.

## Revision Rules

- only one active draft revision is allowed per model identity;
- draft definition replacement uses `expected_definition_checksum`;
- submitted and approved revisions are immutable;
- approving a new revision supersedes the previous approved revision;
- cloning creates a new identity and an initial draft revision;
- deriving from an approved revision creates a new draft with explicit parent.

## Typed Definition

The model definition includes:

- `EngineeringSpecification` with physical quantity, unit, bounds and
  conditions;
- `signal_domains` for the signal, energy or communication domains handled by
  the model, including `power_dc`, `power_ac`, `rf`, analog voltage/current/
  charge, digital logic, trigger/pulse/contact/relay, `can_bus`, RS-232,
  RS-485, Ethernet, USB, GPIB, optical, mechanical, environmental and software
  domains;
- `technology_tags` for searchable filtering such as `rf_50_ohm`, `ttl`,
  `voltage_input`, `iepe`, `usb`, `ethernet`, `can_bus`, `visa`, `raw_tcp` and
  `scpi`;
- `SignalPortDefinition` with `directionality` and `flow_role`, so future
  chain validation can distinguish source, sink, through, measurement,
  control, communication, field-side and transducer-output ports;
- `CommunicationInterfaceDefinition` separating transport, access provider and
  application protocol;
- `MeasurementCapabilityDefinition` for requirements that future test templates
  can request without naming a vendor model.

Canonical JSON and checksum are computed in `emc-locus-core`; SQLite, HTTP and
Qt do not define the business invariants.

## Static Topology Guards

The core now rejects ambiguous or physically incomplete model definitions:

- bare `can`, `adc` or `dac` category codes are refused; use
  `can_bus`, `adc_converter` or `dac_converter` with explicit context;
- communication domains cannot be used as measurement signal ports;
- RF connector ports must declare impedance, while field-side antenna ports are
  modeled separately;
- through-path elements need at least two through-compatible ports;
- sensors require at least one physical input and one output;
- signal sources require a source output;
- measurement instruments require a measurement input;
- communication-only software systems cannot declare physical RF ports unless
  the model explicitly opts into that hybrid behavior.
