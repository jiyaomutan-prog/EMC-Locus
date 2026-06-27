# EMC Locus Qt Console

This is the intended desktop direction for a real measurement-station operator
console.

The static `apps/gui-shell` screen remains useful as a fast workflow prototype,
but advanced measurement software needs a desktop UI stack that can handle:

- instrument-control panels;
- long-running acquisitions;
- live status and alarms;
- dense metrology and project data;
- local/offline operation;
- future plots, traces, and multi-dock workspaces.

## Runtime

The first skeleton uses PySide6 so the team can shape Qt screens quickly while
the Rust core and Python adapters mature.

```text
py -m pip install PySide6
py apps\qt-console\main.py
```

The console can read the same `bootstrap.js` file generated for the static
prototype:

```text
$env:PYTHONPATH='python'; py -m emc_locus.gui_actions refresh-bootstrap --output apps\gui-shell\bootstrap.js
py apps\qt-console\main.py --bootstrap apps\gui-shell\bootstrap.js
```

## Direction

The first implementation already separates Qt rendering from testable Python
view models in `emc_locus.qt_console_models`. This keeps the GUI bridge small
while future screens move toward real Qt model/view widgets backed by stable
application services.

The Rust core remains responsible for domain invariants, while Python adapters
bridge local repositories, scripts, and early instrument workflows.
