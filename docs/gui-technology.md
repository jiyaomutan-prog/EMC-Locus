# GUI Technology Direction

EMC Locus needs two different UI surfaces:

- a local measurement-station operator console;
- possible project-tracking or review screens for non-acquisition work.

The local operator console should be Qt desktop. This is the right direction for
advanced EMC measurement software because it can support dense controls,
instrument panels, long-running acquisition state, local/offline operation,
future plotting, dockable workspaces, and native workstation packaging.

The existing static browser shell remains useful as a workflow prototype. It can
quickly shape dashboard vocabulary and bootstrap data contracts, but it should
not become the primary technology for instrument-facing acquisition screens.

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
