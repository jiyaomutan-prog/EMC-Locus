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
12. adapter-backed runtime that preserves observation logs and safety checks;
13. timeout policy model;
14. VISA, TCP/IP, and serial adapter skeletons;
15. explicit unavailable-IO errors so hardware communication is not faked;
16. IO-backed TCP/IP command exchange with local socket coverage;
17. structured serial endpoint parsing for port, baud rate, data bits, parity,
    and stop bits;
18. structured VISA resource parsing for TCPIP, USB, GPIB, and ASRL resources;
19. exchange-attempt traceability on adapter-backed observations and TCP/IP
    retry attempts.

Not yet implemented:

- typed SCPI command model;
- broader retry classification for connected write/read failures;
- automatic parsing of string commands into typed setpoints;
- IO-backed VISA, serial, and vendor SDK implementations.

TCP/IP currently supports `TCPIP::host::port`, `TCPIP::host`, `host:port`, and
VISA-style `TCPIP0::host::port::SOCKET` endpoints, writes newline-terminated
commands, and reads responses until a newline or closed socket under the
configured timeout policy. `TCPIP0::host::inst::INSTR` resources resolve to the
default SCPI port until a full VISA implementation is selected. Adapter-backed
observations retain the exchange attempt count, so retry behavior remains
visible in run evidence. Malformed VISA-style TCP/IP resources are rejected when
the host is missing, a `SOCKET` port is not numeric, or the resource class is
unknown, so invalid endpoints do not silently fall back to the default SCPI
port.

Serial endpoints currently support `PORT:baud` with default 8N1 framing, or
`PORT:baud:framing` for explicit values such as `COM4:9600:7E2`. Native serial
IO is still intentionally unavailable until a guarded implementation and device
test strategy are added.

VISA resources currently validate common resource strings such as
`TCPIP0::host::inst0::INSTR`, `GPIB0::12::INSTR`,
`USB0::vendor::product::serial::INSTR`, and `ASRL3::INSTR`. Native VISA IO is
still intentionally unavailable until a binding, packaging, and device-test
strategy are selected. Validation is interface-aware: `SOCKET` resources are
limited to TCP/IP resources with numeric ports, GPIB addresses must be numeric,
and ASRL resources must include a serial port index.
