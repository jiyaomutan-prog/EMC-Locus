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
- `SignalPortDefinition` for RF, electrical, digital, mechanical,
  environmental and logical ports;
- `CommunicationInterfaceDefinition` separating transport, access provider and
  application protocol;
- `MeasurementCapabilityDefinition` for requirements that future test templates
  can request without naming a vendor model.

Canonical JSON and checksum are computed in `emc-locus-core`; SQLite, HTTP and
Qt do not define the business invariants.
