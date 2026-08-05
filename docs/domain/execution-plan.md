# Execution Plan And Dated Configuration

Release 0.22.2 distinguishes a read-only compiled plan from a persisted dated
execution configuration.

## Execution-Plan Preview

`POST /api/v1/execution-plans/preview` compiles an exact method revision,
measurement-system definition, regulation profiles and optional dated context.
The result explains:

- ordered procedure phases and finite loops;
- resolved parameter values and sub-ranges;
- required and unresolved roles;
- regulation loops and expected signals;
- requested post-processing;
- blockers, warnings and next actions;
- runtime operations intentionally unsupported in 0.22.2.

This is a deterministic dry run. It does not reserve equipment, command an
instrument, acquire samples, apply corrections or determine a final report.

## Execution Configuration

`emc-locus.execution-configuration.v1` is the dated realization for one
project schedule item. It pins exact checksums and revision IDs for the method,
measurement-system template and station setup. It also stores selected
parameters/sub-ranges, location, planned date, EUT context and the physical
assignment snapshots produced by planned-test preparation.

The derivation route accepts the exact planned-preparation revision and checks
it again. Physical assignments cannot be supplied by a competing matcher. A
blocked exact asset remains a visible requirement and cannot become an
assignment or a ready verdict through this aggregate.

Each immutable configuration revision stores its canonical definition,
checksum, readiness evidence and operation context. Persistence, project audit
and sync outbox are committed atomically. Reload after agent restart returns
the same exact revision pins.
