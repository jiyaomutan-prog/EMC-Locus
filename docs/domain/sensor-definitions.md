# Sensor Definitions

Release `0.13.0` introduces `SensorDefinition` as a reusable engineering
definition for a type of sensor or transducer. It answers: what physical
phenomenon enters the device, what electrical signal leaves it, what sensor
power or conditioning is required, and which approved time-domain conversion
or frequency-response definitions are normally used.

A sensor definition is not a physical asset. A physical asset will later add a
serial number, inventory state, calibration history, station assignment, and
serviceability state. The definition is the controlled technical model that
many physical assets may share.

## Core Fields

- `sensor_definition_id`, manufacturer, model name, variant, family;
- physical input quantity and engineering output quantity/unit;
- electrical output quantity/unit and signal domain;
- technology tags and input mode requirement;
- required sensor power/conditioning, nominal/safe range, frequency and
  temperature range;
- references to approved time-domain sample conversions and frequency
  responses;
- metadata for controlled local extensions.

Supported families include current probes, voltage probes, field probes,
receiving/transmitting antennas, accelerometers, microphones, thermocouples,
pressure sensors, photodiodes, strain gauges, generic transducers, and manual
transducers.

## Examples

- Current probe: physical input `current`, electrical output `voltage`, sample
  conversion such as `10 mV/A`, optional transfer response versus frequency.
- IEPE accelerometer: physical input `acceleration`, electrical output
  `voltage`, required IEPE constant-current supply/conditioning, sample
  conversion such as `100 mV/g`.
- Receiving antenna: physical input `electric_field`, electrical or RF-related
  output, antenna-factor frequency response.

## Validation Boundary

Validation checks family/quantity coherence, unit compatibility, sensor supply
coherence, frequency range order, signal-domain tags, and referenced approved
conversion/response definitions. It does not prove that a specific serialized
asset is calibrated, available, connected, or ready for a campaign.
