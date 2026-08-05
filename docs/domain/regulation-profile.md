# Regulation Profile

`emc-locus.regulation-profile-definition.v1` defines a reusable laboratory
control strategy. It is separate from modulation and from the topology that
maps the strategy to nodes.

The typed definition includes regulated quantity and unit, target expression,
tolerance, open-loop/closed-loop/monitor-only mode, actuator and required
driver action, feedback and monitoring roles, start threshold, fast/slow up
steps, down step, dwell T1/T2/T3, start/end criteria, regulation factor,
protection ceilings, retry/abort/safe-state policies and operator notes.

The Rust validator blocks a closed-loop profile without actuator, driver
action, feedback role or feedback variable. When combined with a method and a
measurement-system template, it also requires compatible variables and a
reachable explicit feedback path.

LAB CONSOLE presents the same draft through a stepped chart, a structured
parameter table, a French language summary and a validation result. Profiles
use the same revision, checksum, optimistic-concurrency, audit and outbox
contract as measurement-system templates.

A profile is a laboratory configuration aid. Its existence does not prove
compliance with a standard and 0.22.2 does not execute its driver actions.
