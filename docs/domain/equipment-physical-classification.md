# Equipment Physical Classification

Release `0.12.0` turns equipment classification into backend-owned product
data instead of UI-only metadata.

## Concepts

- **Equipment class** remains the catalog family used for lifecycle and driver
  eligibility: controllable instrument, DAQ device, converter, sensor,
  transducer, passive component, software adapter, manual equipment, etc.
- **Functional role** describes what the equipment does in a measurement chain:
  source, RF network element, sensor, actuator, measurement instrument,
  acquisition device, converter, control system, software system, facility, or
  manual accessory. A `daq_device` class can therefore be a `converter` when it
  is a pure ADC, or an `acquisition_device` when it acquires and controls
  channels.
- **Signal domain** describes the physical or communication domain carried by a
  model or port: RF, analog voltage/current/charge, power AC/DC, trigger,
  pulse, relay/contact, CAN bus, serial, Ethernet, USB, GPIB, optical,
  mechanical, environmental, or software. This is distinct from an access
  provider such as VISA, SocketCAN, PCAN, native TCP, or simulation.
- **Technology tag** refines a model or port with implementation details such
  as RF 50 ohm, ADC converter, voltage input, IEPE, SCPI, raw TCP, VISA, or CAN
  bus. A tag is searchable classification, not a substitute for structured
  fields such as `transport_kind`, `protocol_kind`, impedance, or unit.
- **Directionality** describes the physical direction at a connector: input,
  output, bidirectional, through, control, or communication.
- **Flow role** describes why the port exists in a chain. `source_port` emits
  energy or signal, `sink_port` terminates or consumes it, `through_port` is one
  side of a pass-through network element, `measurement_port` is used to measure,
  `communication_port` carries data/control, `field_side_port` is the physical
  field side of a transducer, and `transducer_output_port` is the converted
  output of a sensor.
- **Port topology** records directionality, flow role, domain, connector,
  required/optional status, quantity, unit, impedance and bounds.
- **Classification preset** is a backend seed for creating a draft equipment
  model with coherent class, role, domains, tags and port topology.

## Important Distinctions

- **Sensor vs actuator**: a receiving antenna, field probe, current probe,
  microphone or accelerometer is a sensor because it converts a field or
  physical quantity into an output. A transmitting antenna, speaker, heater,
  motor, relay or valve is an actuator because it emits or changes the setup.
- **Measurement instrument vs acquisition device**: an oscilloscope or EMI
  receiver is a measurement instrument. A DAQ card is an acquisition device
  when it has measurement inputs plus a communication/control path. A pure ADC
  converter can be modeled as `equipment_class=daq_device` and
  `functional_role=converter` without inventing a CAN bus port.
- **ADC vs DAC vs CAN bus**: `adc_converter` and `dac_converter` are conversion
  technologies. `can_bus` is Controller Area Network communication. The release
  rejects ambiguous `category_code` values `adc`, `dac`, or `can`.
- **Signal domain vs access provider**: `can_bus`, `ethernet`, `usb`, `gpib`,
  `rs232` and `rs485` are domains/transports. `visa`, `socketcan`, `pcan`,
  `vector_can`, `native_tcp`, `native_serial` and `simulation` are access
  providers or runtime availability concepts.

## Boundaries

Classification presets are authoring aids. They do not create driver profiles,
communication sessions, physical fleet assets, acquisition channels, or runtime
execution behavior. A model created from a preset remains a draft revision that
must pass core validation and normal submit/approve workflow.

## Indexed Summaries

The local agent writes `equipment_model_classification_summaries` plus
normalized domain/tag summary tables in the same transaction as equipment model
creation, draft replacement, revision derivation, submission and approval.
LAB CONSOLE and API filters use these summaries for `functional_role`,
`signal_domain`, `technology_tag`, `equipment_class`, `manufacturer`, `status`
and text search.

This is intentionally separate from future sensor/DAQ channel scaling,
engineering curves, calibration transfer functions, and acquisition runtime
execution.

## Preset Examples

### RF Generator

- class: `controllable_instrument`
- role: `signal_source`
- domains: `rf`, plus communication domains when controlled
- tags: `rf_50_ohm`, optional `scpi`, `ethernet`, `usb`, or `visa`
- topology: `RF_OUT` as `source_port` on RF with 50 ohm impedance

### RF Cable

- class: `passive_component`
- role: `rf_network_element`
- domains/tags: `rf`, `rf_50_ohm`
- topology: `RF_A` and `RF_B` as `through_port` with 50 ohm impedance

### RF Load

- class: `passive_component`
- role: `rf_network_element`
- topology: one RF `sink_port` is valid because the function is termination,
  not a through path.

### Receiving Antenna

- class: `sensor`
- role: `sensor`
- topology: `FIELD` as environmental `field_side_port` input and `RF_OUT` as
  RF `transducer_output_port` with 50 ohm impedance.

### Transmitting Antenna

- class: `transducer`
- role: `actuator`
- topology: `RF_IN` as RF sink input and `FIELD` as environmental field-side
  output.

### Oscilloscope

- class: `controllable_instrument`
- role: `measurement_instrument`
- topology: analog voltage `measurement_port`, trigger `control_port`, and LAN
  or USB `communication_port`.

### ADC Converter

- class: `daq_device`
- role: `converter`
- domains/tags: `analog_voltage`, `digital_logic`, `adc_converter`,
  `voltage_input`
- topology: analog measurement input and digital transducer output. No CAN bus
  port is created by default.

### DAQ Card

- class: `daq_device`
- role: `acquisition_device`
- topology: at least one measurement input and at least one communication or
  control path such as USB, Ethernet, trigger, or internal control.

### CAN Bus Controlled Unit

- class: `controllable_instrument`
- role: `control_system`
- domains/tags: explicit `can_bus`
- topology: `CAN_BUS` communication port plus the physical input or output the
  controlled unit actually provides.

### Software Controller

- class: `software_adapter`
- role: `software_system`
- domains: `software` and optional communication domains
- topology: communication/software ports only unless
  `metadata.allow_physical_ports=true` documents a hybrid adapter.
