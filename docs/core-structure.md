# Core Crate Structure

The `emc-locus-core` crate owns domain invariants and stays independent from UI,
database adapters, and hardware drivers.

## Module Map

```text
crates/emc-locus-core/src/
  lib.rs           Public module declarations and re-exports
  identifiers.rs   Value objects such as project codes and audit identities
  audit.rs         Audit actions and audit events
  project.rs       Project lifecycle and project audit record
  quality.rs       Contract review, deviations, and execution modes
  metrology.rs     Instrument registry, calibration records, and readiness checks
  repositories.rs  Connectivity, repository domains, and sync policy
  instrument.rs    Instrument transports and update policy
  signal.rs        DAQ, synchronization, and signal-processing primitives
  traceability.rs  Baseline traceability requirements
  error.rs         Domain errors shared by the modules
  tests.rs         Public behavior tests across module boundaries
```

## Boundary Rules

- Domain modules must not depend on a database, UI framework, hardware SDK, or
  network client.
- Adapters should call core rules; they should not duplicate them.
- New controlled workflow transitions must create audit evidence.
- New relaxed workflows must expose their execution mode and deviation evidence.
- New metrology checks must distinguish blocking safety/quality failures from
  non-blocking attention points.
- New signal-processing outputs must retain lineage to raw data.

## Growth Direction

The next code growth should preserve the current shape:

- storage migrations should live outside `emc-locus-core`;
- instrument drivers should live outside `emc-locus-core` and depend on core
  transport/capability concepts;
- signal-processing implementations can start outside core, while core owns the
  workflow vocabulary and traceability requirements.
