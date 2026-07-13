# Driver Profile

Release `0.11.0` introduces revisioned driver profiles as a separate aggregate
from equipment models and metrology records.

## Concepts

- `DriverProfileIdentity`: stable driver id, linked equipment model id, display
  label, creator and current approved revision pointer.
- `DriverProfileRevision`: the coherent approval unit for all actions and
  scripts of a driver profile.
- `DriverProfileDefinition`: typed Rust core definition referencing an approved
  equipment model revision and checksum.

## Model Link

A driver revision must reference:

- `equipment_model_id`;
- approved `supported_model_revision_id`;
- `supported_model_definition_checksum`;
- communication profile ids declared by that approved model revision.

This prevents a driver from silently drifting away from the equipment model it
claims to implement.
The checksum must use canonical `sha256:<64 lowercase hex characters>` syntax,
matching the digest format produced by core canonicalization.

Release `0.12.0` does not change driver execution semantics, but it strengthens
the equipment model side of that link. Driver authors now select against
models whose functional role, signal domains, technology tags, and port
topology are visible through indexed catalog filters and backend presets.

## Driver Actions

Each action declares:

- stable `action_id`;
- implemented capability;
- typed inputs and outputs;
- safety class;
- bounded timeout and optional retry policy;
- structured `DriverScriptDefinition` AST.

Actions are not independently approved. The whole driver revision is approved
together so scripts, actions and compatibility metadata remain coherent.

## Runtime Boundary

`0.11.0` contains deterministic simulation of the structured AST. It is not yet
a certified real instrument runtime. Real providers can be added later behind
the communication adapter contract without changing the approved driver
definition shape.

`0.12.0` remains in that boundary: classification presets do not create driver
profiles, station bindings, or real hardware sessions.
