# Instrument Control Architecture

Instrument control is a critical differentiator for EMC Locus. The runtime must
be more rigorous than an accumulation of ad hoc scripts.

## Goals

- Support any practical laboratory communication path.
- Make driver behavior testable without hardware.
- Preserve a command and observation log for every measurement run.
- Keep safety interlocks explicit.
- Allow manual fallback steps without hiding them from the audit trail.

## Layers

### Transport

Transports move bytes or messages. They do not know EMC semantics.

Initial transport targets:

- VISA;
- GPIB;
- serial;
- TCP/IP;
- UDP;
- USBTMC;
- CAN;
- LIN;
- Modbus TCP;
- Modbus RTU;
- REST;
- vendor SDK bridge;
- manual operator step;
- simulator.

### Protocol

Protocols turn raw communication into typed exchanges:

- SCPI;
- binary vendor protocols;
- Modbus registers;
- CAN frames;
- REST resources;
- camera/image events;
- manual confirmation forms.

### Driver

Drivers declare capabilities, not just commands.

Examples:

- spectrum analyzer frequency span;
- receiver detector modes;
- amplifier level range;
- antenna mast position range;
- turntable angular range;
- generator modulation modes;
- probe calibration factors;
- EUT monitoring channels.

### Procedure Runtime

The procedure runtime coordinates instruments while preserving:

- command sequence;
- observed values;
- setpoints;
- tolerances;
- retries;
- operator interventions;
- safety events.

## Driver Quality Requirements

Each driver should have:

- simulated fixture;
- capability declaration;
- command mapping;
- error mapping;
- timeout policy;
- validation notes;
- minimum hardware or simulator test evidence.

## Design Rules

- Never let a driver write raw measurement data without a measurement-run
  context.
- Never hide retries from the command log.
- Never apply a range, power, frequency, or level command without a safety check
  when limits are known.
- Keep manual steps explicit and auditable.
- Prefer typed commands over stringly command assembly.

## First Implementation Slice

Implemented in the Rust core:

1. transport vocabulary with stable log/adapter slugs;
2. instrument command messages;
3. simulated instrument runtime;
4. deterministic responses for query and set commands;
5. ordered command and observation log;
6. target-instrument and supported-transport validation;
7. typed setpoints and safety limits;
8. blocking of commands outside known limits;
9. transport endpoint model;
10. transport adapter trait;
11. simulated adapter conformance fixture;
12. adapter-backed runtime that preserves observation logs and safety checks.

Not yet implemented:

- typed SCPI command model;
- timeout/retry policy;
- automatic parsing of string commands into typed setpoints;
- concrete VISA, TCP/IP, serial, and vendor SDK adapters.
