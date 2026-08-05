# Measurement-System Template

`emc-locus.measurement-system-template-definition.v1` is a reusable,
revisioned logical topology. It explains how functions are connected without
choosing a current physical asset, laboratory location or test date.

## Structure

A definition owns:

- functional nodes, optionally mapped to method role IDs;
- named logical ports with direction and signal domain;
- typed edges;
- correction points;
- explicit regulation-loop mappings;
- classification and laboratory notes.

Supported edge meanings are `physical_signal`, `excitation_or_power`,
`control_command`, `feedback_measurement`, `monitoring`, `trigger`,
`synchronization`, `data_stream` and `eut_state`. LAB CONSOLE presents French
labels and four visual layers while keeping the stable values out of normal
operator vocabulary.

## Validation

The Rust core is authoritative for node and port uniqueness, endpoint
existence, direction compatibility, physical signal-domain compatibility,
undeclared cycles, method-role references and regulation-loop reachability. A
closed-loop mapping requires an explicit feedback edge from the feedback node
to the actuator node. The accessible connection table is equivalent to the
visual graph; it is not a second validator.

## Lifecycle

The revision lifecycle is `draft -> validated -> approved`; a previously
approved revision becomes `superseded` when a newer revision is approved.
Draft replacement requires the expected canonical checksum. Every mutation is
idempotent, audited and paired atomically with an outbox event.

The template may be used by many planned tests. Its roles are resolved to real
equipment only in dated station preparation. A justified exact-asset
requirement continues to be represented by the existing station/material
engine, not embedded in this template.
