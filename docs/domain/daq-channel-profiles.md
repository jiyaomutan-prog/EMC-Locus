# DAQ Channel Profiles

Release `0.13.0` introduces `DaqChannelProfileDefinition` for reusable channel
capabilities. It describes what a class of DAQ channel can accept or produce,
not a concrete runtime channel on a connected station.

## Core Fields

- `daq_channel_profile_id` and label;
- channel kind and signal domain;
- input quantity/unit for input channels;
- supported ranges, resolution bits, min/max sampling rate;
- coupling modes and input modes;
- anti-alias filter, sensor power/conditioning capabilities, bridge completion,
  and IEPE support;
- isolation, synchronization, triggering, and metadata.

Supported channel kinds include analog input/output, digital input/output,
counter input, frequency input, trigger input/output, CAN bus channel, and
software channel. Supported input modes include single-ended, differential,
pseudo-differential, current loop, charge, IEPE, bridge modes, thermocouple,
and RTD.

## Validation Boundary

Validation checks that analog inputs declare compatible input quantity/unit,
ranges are finite and ordered, sampling limits are positive, IEPE and bridge
settings are coherent, trigger channels use trigger domains, and CAN bus
channels are not confused with ADC channels.

The profile does not bind a channel number, device serial number, driver
connection, clock topology, or running acquisition. Those belong to later
physical fleet and station connection releases.
