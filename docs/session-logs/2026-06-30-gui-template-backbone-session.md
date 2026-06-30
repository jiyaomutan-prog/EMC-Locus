# 2026-06-30 GUI And Template Backbone Session

## Goal

Freeze the product-level GUI split and define the template/execution backbone
before adding more runtime slices.

## Decisions Recorded

- LAB CONSOLE and TEST CONSOLE are separate consoles sharing a domain backbone.
- Metrology is a controlled LAB domain and TEST readiness dependency, not a
  third GUI product.
- The static web shell is a LAB CONSOLE information architecture prototype, not
  the final web architecture and not an execution console.
- TEST CONSOLE remains Qt-directed for dense local/offline execution.
- Legacy behavior may be imported, archived, or migrated, but it must not drive
  new domain vocabulary or UI structure.
- No major new runtime vertical should be added before the template and
  information architecture backbone is stabilized.

## Documentation Added

- `docs/gui/gui-dual-surface-architecture.md`
- `docs/gui/lab-console-information-architecture.md`
- `docs/gui/test-console-workspace.md`
- `docs/domain/template-and-execution-definition.md`
- `docs/domain/people-roles-competence.md`
- `docs/adr/0002-gui-and-template-backbone.md`

## Prototype Update

`apps/gui-shell` was refocused on LAB CONSOLE information architecture. The
prototype now shows hierarchical LAB navigation, LAB-to-TEST handoff points,
object relationships, and guardrails instead of suggesting that the static web
shell is an execution dashboard.

## Validation

- `node --check apps\gui-shell\app.js`: passed.
- `py -m py_compile apps\qt-console\main.py`: passed.
- Static selector check: passed, every `querySelector("#...")` target exists
  in `apps/gui-shell/index.html`.
- In-app browser inspection at `http://127.0.0.1:8765/`: blocked by the local
  browser security policy for `127.0.0.1`; no workaround was attempted.
- `git diff --check`: passed before staging.
- Final staged validation is recorded in
  `docs/session-logs/2026-06-30-release-0.8.2-session.md`.
