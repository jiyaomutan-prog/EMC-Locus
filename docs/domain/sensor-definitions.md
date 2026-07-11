# Sensor Definitions

Release `0.13.0` introduces `SensorDefinition` as a reusable engineering
definition for a type of sensor or transducer. It answers: what physical
phenomenon enters the device, what electrical signal leaves it, what excitation
is required, and which approved scaling or correction artifacts are normally
used.

A sensor definition is not a physical asset. A physical asset will later add a
serial number, inventory state, calibration history, station assignment, and
serviceability state. The definition is the controlled technical model that
many physical assets may share.

## Core Fields

- `sensor_definition_id`, manufacturer, model name, variant, family;
- physical input quantity and engineering output quantity/unit;
- electrical output quantity/unit and signal domain;
- technology tags and input mode requirement;
- required excitation, nominal/safe range, frequency and temperature range;
- references to approved scaling profiles and engineering curves;
- metadata for controlled local extensions.

Supported families include current probes, voltage probes, field probes,
receiving/transmitting antennas, accelerometers, microphones, thermocouples,
pressure sensors, photodiodes, strain gauges, generic transducers, and manual
transducers.

## Examples

- Current probe: physical input `current`, electrical output `voltage`, scaling
  such as `10 mV/A`, optional current-probe transfer curve versus frequency.
- IEPE accelerometer: physical input `acceleration`, electrical output
  `voltage`, required IEPE current excitation, scaling such as `100 mV/g`.
- Receiving antenna: physical input `electric_field`, electrical or RF-related
  output, antenna factor curve versus frequency.

## Validation Boundary

Validation checks family/quantity coherence, unit compatibility, excitation
coherence, frequency range order, signal-domain tags, and referenced approved
scaling/curve definitions. It does not prove that a specific serialized asset
is calibrated, available, connected, or ready for a campaign.
