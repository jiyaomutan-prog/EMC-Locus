# EMC Locus GUI Shell

This folder contains a static LAB CONSOLE information-architecture prototype.
It is not the execution console and not the final web application architecture.

Open `index.html` directly in a browser.

Current intent:

- show the future LAB CONSOLE navigation hierarchy;
- make clients, products, product versions, projects, campaigns, templates,
  methods, documents, people, metrology, planning, reports, sync, audit, and
  updates visible as related laboratory objects;
- keep the visual atmosphere of the earlier shell while removing fake runtime
  behavior;
- avoid backend writes, fake acquisition, fake instrument control, or ad hoc
  CRUD promises.

TEST CONSOLE remains the Qt direction for local/offline execution, readiness,
instrument control, acquisition monitoring, deviations, substitutions, reruns,
and execution evidence publication.

`bootstrap.js` can provide small static overrides for this prototype when it
sets `lab_console_version` to `ia-0.1`. Older bootstrap files generated for the
previous dashboard shape are intentionally ignored by `app.js`.
