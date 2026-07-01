# EMC Locus TEST CONSOLE Qt

TEST CONSOLE is the desktop direction for measurement-station operation:
instrument panels, long-running acquisitions, local/offline behavior, dense
status views, and future plots or docked workspaces.

LAB CONSOLE is separate. It is the React/TypeScript laboratory-management app
served by the Rust agent under `/lab/`. TEST CONSOLE Qt no longer reads a LAB
web bootstrap script.

## Static Demo

Static mode uses a strict JSON fixture owned by Qt:

```powershell
py apps\qt-console\main.py --bootstrap apps\qt-console\demo\bootstrap.json
.\scripts\start-qt-demo.ps1 -Mode Static
```

The fixture is only for local UI demonstration. It is not the LAB CONSOLE data
source and it is not a simulated acquisition runtime.

## Agent Mode

Qt can connect to the local Rust agent:

```powershell
.\scripts\start-lab.ps1 -NoBrowser
.\scripts\start-qt-demo.ps1 -Mode Agent
```

`start-full-demo.ps1` starts the agent, opens LAB CONSOLE, then launches Qt
against the same `data\local-agent` storage root.

## Direction

The first implementation keeps rendering separated from testable Python view
models in `emc_locus.qt_console_models`. Existing forms are still transitional:
the long-term path is Rust-owned application services and Qt model/view widgets
for measurement-station workflows.
