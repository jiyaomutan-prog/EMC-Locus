# Test Method Workflow

Release 0.22.2 introduces `emc-locus.test-method-definition.v2` as the
authoritative reusable description of what a laboratory test must do. A method
revision contains no date, location, physical asset, serial number, current
calibration state or reservation.

## Aggregate Boundary

The existing test-template identity and immutable revision history remain the
method aggregate. A v2 revision contains:

- objective, scope, classification path and laboratory-owned references;
- typed variables and deterministic expressions;
- functional instrumentation roles and logical ports;
- references to exact measurement-system and regulation-profile revisions;
- modulation profiles and bounded sub-ranges;
- a hierarchical, finite procedure;
- classified safety, protection, acceptance and verdict limits;
- a typed post-processing contract and expected outputs.

The Rust core validates IDs, values, units, dependencies, graph references,
finite loops, limit ownership and post-processing inputs. Expressions are a
closed declarative AST. They never execute JavaScript, Python, shell commands
or arbitrary code.

## Variables And Roles

Variable semantics distinguish method parameters, project/EUT inputs,
operator inputs, derived values, setpoints, observed and monitoring signals,
intermediate and final results, and verdict outputs. Source and availability
phase are explicit. Unknown references, dependency cycles, incompatible units
and unsafe arithmetic are rejected.

A functional role says what capability is required, not which serialized
instrument will be used. It may carry technical ranges, required driver
actions, calibration and substitution policies, logical ports, and produced or
consumed variables. A measurement-system node may map to one role by stable ID.

## Procedure, Limits And Processing

Procedure nodes form a hierarchy of preparation, verification, calibration,
phase, loop, sweep, EUT state, setpoint, regulation, dwell, acquisition,
monitoring, operator action, decision, repeat, finalization and safe-shutdown
steps. Every loop requires a finite maximum iteration count.

Runtime abort limits and final conformity criteria are separate records. Each
limit identifies its variable, comparison, threshold or curve, unit, phase,
severity, action and verdict contribution.

Post-processing nodes define intended corrections, conversions, interpolation,
detectors, windows, FFT requests, smoothing, aggregation, searches,
comparisons, uncertainty contributions and exports. In 0.22.2 the compiler
validates and reports these requests; it does not execute FFT or apply a
correction to acquired data.

## Revision And Compatibility Rules

Draft changes use the existing checksum compare-and-swap contract. Submitted
and approved revisions remain immutable. The explicit
`successor-0.22.2` command derives a v2 draft from a historical v1 revision,
maps known instrumentation slots to functional roles and retains ambiguous
content as migration evidence. It never invents a topology or approves the
draft.

Historical v1 definitions and checksums remain readable and unchanged. There
is no dual-write path and no silent conversion.
