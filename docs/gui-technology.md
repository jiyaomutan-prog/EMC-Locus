# GUI Technology Direction

EMC Locus needs two different UI surfaces:

- LAB CONSOLE for laboratory management, projects, templates, documents,
  methods, people, metrology, planning, reports, sync, audit, and updates;
- TEST CONSOLE for local/offline execution, readiness, instrumentation,
  monitoring, acquisition evidence, deviations, substitutions, and publication
  back to the local repositories.

The detailed split is now defined in
`gui/gui-dual-surface-architecture.md`. The LAB CONSOLE navigation model is
defined in `gui/lab-console-information-architecture.md`, and the Qt execution
workspace is defined in `gui/test-console-workspace.md`.

The local operator console should be Qt desktop. This is the right direction
for advanced EMC measurement software because it can support dense controls,
instrument panels, long-running acquisition state, local/offline operation,
future plotting, dockable workspaces, and native workstation packaging.

The existing static browser shell remains useful only as a LAB CONSOLE
information-architecture prototype. It should not become the primary technology
for instrument-facing acquisition screens and should not accumulate fake
runtime workflows.

## Initial Stack

- PySide6 for fast Qt screen shaping.
- Rust core for domain invariants and critical measurement rules.
- Python adapters for local repositories, scripting, and early service wiring.
- SQLite split repositories for offline station data.

## Hardening Path

1. Replace fixture table columns with explicit Qt models.
2. Bind Qt models to stable application-service APIs.
3. Add dedicated workspaces for projects, metrology, methods, data, updates, and
   instrument runtime.
4. Introduce plotting and live acquisition panels only after the runtime events
   and signal-processing records are stable.
5. Package the station app with controlled dependencies and update evidence.

## Current Implementation Slice

The initial Qt console uses testable Python view models for project,
metrology, method, dataset, and update tables. The PySide6 layer renders those
models but does not own the data shaping rules. This keeps the first UI bridge
thin and prepares the transition to proper Qt model/view classes.

The view-model layer also exposes operator action intents. These describe which
high-level commands can be shown to the operator while write execution remains
centralized in tested Python action services.

The current Qt slice adds a `Saisie` tab backed by testable form contracts. The
forms can register instruments, attach material documents, schedule service
items, create projects, complete contract-review items, and create test
categories when local repository paths are supplied.
