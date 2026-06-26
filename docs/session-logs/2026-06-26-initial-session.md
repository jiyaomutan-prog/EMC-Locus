# Initial Session - 2026-06-26

## Context

The repository was created as `jiyaomutan-prog/EMC-Locus` and cloned locally at:

```text
E:\projets codex\EMC-Locus
```

The product target is an original Rust/Python platform for EMC test control,
metrology records, project data management, and traceability-oriented laboratory
workflows.

## Work Completed

- Expanded the README into a product and technical starting point.
- Added architecture notes.
- Added an initial domain model.
- Added EN ISO/IEC 17025 alignment notes.
- Added a phased roadmap.
- Added a minimal Rust workspace with an `emc-locus-core` crate.
- Added a minimal Python helper package for planning future Codex sessions.

## Validation Notes

Rust/Cargo were not available in the current PATH during this session.
Python was available through the Windows `py` launcher.

Checks performed:

```text
py -m compileall python/emc_locus
```

Expected future Rust check once the toolchain is installed: `cargo test`.

## Next Recommended Step

Implement the first real vertical slice:

1. create project;
2. perform contract review;
3. plan campaign;
4. generate audit events for each transition;
5. persist a local SQLite draft schema.

This slice is small enough to test and valuable enough to shape the rest of the
platform.
