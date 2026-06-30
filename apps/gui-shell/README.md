# EMC Locus GUI Shell

This folder contains a static information-architecture prototype for the Locus
product family. It is not the execution console and not the final web
application architecture.

Open `index.html` directly in a browser.

Current intent:

- show the future Locus Lab Management navigation hierarchy;
- show Locus Metrology and Locus Test Station as separate companion products;
- make clients, products, product versions, projects, campaigns, templates,
  methods, documents, people, metrology, planning, reports, sync, audit, and
  updates visible as related laboratory objects;
- keep the visual atmosphere of the earlier shell while removing fake runtime
  behavior;
- avoid backend writes, fake acquisition, fake instrument control, or ad hoc
  CRUD promises.

Locus Test Station remains the Qt direction for local/offline execution,
readiness, instrument control, acquisition monitoring, deviations,
substitutions, reruns, and execution evidence publication. Locus Metrology owns
the detailed instrument park, calibration, restriction, document, traceability,
and metrological aptitude workflows.

`bootstrap.js` can provide small static overrides for this prototype when it
sets `lab_console_version` to `ia-0.1`. Older bootstrap files generated for the
previous dashboard shape are intentionally ignored by `app.js`.
