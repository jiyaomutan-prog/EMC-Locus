# EMC Locus GUI Shell

This folder contains a static information-architecture prototype for EMC Locus
LAB CONSOLE. It is not the TEST CONSOLE execution surface and not the final web
application architecture.

Open `index.html` directly in a browser, or serve it from the repository root:

```powershell
py -m http.server 8000 --directory ./apps/gui-shell
```

Then open `http://127.0.0.1:8000/`.

When using Git Bash, keep the POSIX-style path `./apps/gui-shell`; do not use
`apps\gui-shell`.

Current intent:

- show the future LAB CONSOLE navigation hierarchy;
- show the TEST CONSOLE Qt boundary and LAB-to-TEST handoff;
- make clients, products, product versions, projects, campaigns, templates,
  methods, documents, people, metrology, planning, reports, sync, audit, and
  updates visible as related laboratory objects;
- keep the visual atmosphere of the earlier shell while removing fake runtime
  behavior;
- avoid backend writes, fake acquisition, fake instrument control, or ad hoc
  CRUD promises.

TEST CONSOLE remains the Qt direction for local/offline execution, readiness,
instrument control, acquisition monitoring, deviations, substitutions, reruns,
and execution evidence publication. Metrology is a first-class LAB CONSOLE
domain and a TEST CONSOLE readiness dependency; source metrology records should
not be edited from the execution prototype.

`bootstrap.js` can provide small static overrides for this prototype when it
sets `lab_console_version` to `ia-0.2`. Older bootstrap files generated for the
previous dashboard shape are intentionally ignored by `app.js`.

Manual smoke test:

1. Open `index.html` directly and confirm the LAB Console IA shell renders.
2. Run `py -m http.server 8000 --directory ./apps/gui-shell` from the repository root.
3. Open `http://127.0.0.1:8000/` and confirm the same shell renders.
4. Run `py -m unittest python.tests.test_gui_shell_smoke`.
