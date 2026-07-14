# Equipment Ports And Signal Paths

An equipment model declares what enters the equipment, what leaves it, and how
a measurement result is derived. This is separate from communication ports
used to control the equipment.

## Ports

`SignalPortDefinition` describes a physical or logical port with:

- stable port identifier and laboratory label;
- input, output, bidirectional, or through direction;
- signal domain and physical quantity/unit;
- connector and impedance when relevant;
- optional voltage, current, power, and frequency bounds.

A spectrum analyser can therefore have an RF input and a logical software
output containing the measured spectrum. An RF cable has two through ports,
both with `Fmin`, `Fmax`, connector, and impedance.

## Paths

`EquipmentSignalPathDefinition` connects one declared input port to one
declared output port. A path may reference controlled transformations:

- `sample_conversion` for time-domain samples;
- `frequency_response` for spectral amplitude and optional phase.

Every reference pins `entity_id`, `revision_id`, and `definition_checksum`.
Only approved or historically superseded revisions are accepted. Missing,
draft, or checksum-mismatched references block model creation, editing,
submission, or approval with a structured error.

## Sensor Power And Conditioning

The former generic operator label `Excitation` is replaced by **alimentation et
conditionnement du capteur**. It describes current or voltage supplied to the
measurement sensor or its conditioning chain, such as IEPE current, bridge
voltage, or a charge amplifier. It does not describe the test stimulus applied
to the equipment under test; DUT stimulus belongs to the future test method and
execution domains.

## Physical Assets

A serialized material already references its approved equipment-model identity,
revision, and checksum through metrology. This makes the model’s ports and paths
traceable from the asset. Direct per-asset correction override and station
wiring are intentionally not implemented in `0.15.0`; they require their own
revision and metrology policy rather than mutable free text on the asset.
