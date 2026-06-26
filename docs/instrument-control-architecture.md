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

1. Define `InstrumentTransport`.
2. Define an instrument identity and capability model.
3. Add a simulated SCPI-like instrument.
4. Record command and observation events.
5. Validate one measurement procedure against the simulator.
