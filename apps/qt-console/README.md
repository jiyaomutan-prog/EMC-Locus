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

It can also load local SQLite repositories directly. When repository paths are
provided, the `Saisie` tab enables local write forms for material registration,
material documents, project creation, service planning, test categories, and
contract-review checklist completion.

```text
py apps\qt-console\main.py --projects-db data\projects.sqlite --metrology-db data\metrology.sqlite --test-definitions-db data\test_definitions.sqlite --measurement-data-db data\measurement_data.sqlite
```

Project creation, contract-review item completion, and transition to planning
can be routed through the local Rust agent while the console continues to read
repository data for the current prototype:

```text
cargo run -q -p emc-locus-agent -- serve --storage-root data\agent --migrations-root storage\sqlite --bind 127.0.0.1:8765
py apps\qt-console\main.py --projects-db data\agent\projects.sqlite --metrology-db data\metrology.sqlite --test-definitions-db data\test_definitions.sqlite --agent-url http://127.0.0.1:8765
```

When `--agent-url` is configured, the header displays the local-agent state:
connected, unavailable, storage not initialized, migration required, or local
integrity error. Project form submissions run through a Qt worker so agent calls
do not block the main UI thread.

Agent-backed:

- project creation;
- contract-review item completion;
- transition to `test_planning` through the Qt `Passage planning` form or the
  Python action layer.

Legacy direct SQLite:

- metrology entry and documents;
- service scheduling;
- test-category maintenance;
- measurement-data, update, and runtime actions not yet migrated.

## Direction

The first implementation already separates Qt rendering from testable Python
view models in `emc_locus.qt_console_models`. This keeps the GUI bridge small
while future screens move toward real Qt model/view widgets backed by stable
application services.

The same view-model layer exposes initial operator action intents for project
advancement, dataset-retention requests, and update validation. The first Qt
write path now covers metrology material creation, document attachment,
project creation, service scheduling, and test-category creation by calling the
same Python action layer used by the CLI. Contract-review checklist items now
use that same action path and refresh into Qt/browser tables.

The console also exposes first status metrics for active projects, metrology
alerts, retained datasets, and updates requiring attention.

A first runtime table contract exists for future instrument workspaces. It
expects instrument, transport, endpoint, state, and last-observation fields when
runtime data becomes available.

The Rust core remains responsible for domain invariants, while Python adapters
bridge local repositories, scripts, and early instrument workflows.
