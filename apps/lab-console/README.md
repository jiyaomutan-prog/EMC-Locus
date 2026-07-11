# EMC Locus LAB CONSOLE

LAB CONSOLE is the browser application for laboratory management workflows. In
0.13.0 it provides Template Studio v1 plus the Equipment workspace with
revisioned equipment models, driver profiles, communication-provider status,
backend-owned physical classification registries, preset-backed model creation,
catalog filters, structured port topology editing, and measurement-engineering
editors for sensors, scaling profiles, engineering curves, DAQ channel
profiles, and acquisition channel recipes.

## Runtime

Normal release launch does not require Node:

```powershell
.\scripts\start-lab.ps1
.\scripts\start-lab.ps1 -SeedDemo
.\scripts\start-lab.ps1 -SeedEquipmentDemo
.\scripts\start-lab.ps1 -SeedMeasurementDemo
.\scripts\start-equipment-demo.ps1
```

The Rust local agent serves the production build from:

```text
apps/lab-console/dist
```

Routes:

```text
GET /      -> /lab/
GET /lab/  -> LAB CONSOLE
/api/v1/*  -> local agent API
```

## Development

```powershell
cd apps\lab-console
npm ci
npm run typecheck
npm run lint
npm run test
npm run build
npm run test:e2e
```

The release validation path is npm-only. `package-lock.json` is committed and
GitHub Actions runs `npm ci`; do not use another package manager for release
evidence unless the workflow and lockfile policy are deliberately migrated.

Vite proxies `/api` to `http://127.0.0.1:8765` during development. Production
does not use a Node server.

## Scope In 0.13.0

Functional:

- Sensors & Transducers studio with structured physical input, electrical
  output, excitation, scaling references, correction references, revision
  history, audit, and advanced JSON sections;
- Scaling Profiles studio with linear, two-point, polynomial, lookup-table,
  piecewise, and expression definitions, including lookup CSV paste/export;
- Engineering Curves studio with curve type, axes, table editor, CSV
  paste/import/export, simple 1D SVG plot, validation, and deterministic
  evaluation through the agent;
- DAQ Channels studio with channel kind, signal domain, input modes, ranges,
  sampling, coupling, triggering, synchronization, and excitation fields;
- Acquisition Recipes studio with a readable DAQ -> sensor electrical signal
  -> scaling -> correction -> engineering output chain summary;
- public-API demo seed support through `-SeedMeasurementDemo`.

Still not implemented:

- real DAQ, oscilloscope, or spectrum acquisition;
- live VISA/CAN/USB binding for measurement channels;
- physical serial-numbered asset fleet redesign;
- station connection mapping;
- graphical measurement-chain builder;
- campaign execution package freezing;
- FFT or signal-processing runtime;
- report generation;
- authentication/RBAC;
- central synchronization.

## Scope In 0.12.0

Functional:

- template library from the API;
- create and clone template identities;
- structured draft editing;
- server validation;
- checksum-based draft saving;
- submit, approve, and derive revision workflow;
- revision history and audit;
- system status;
- Equipment navigation with Model Catalog and Drivers and Actions;
- creation, editing, validation, CAS save, submit, approve, derive, clone,
  revision history and audit for equipment models;
- driver creation linked to approved equipment models, typed actions, structured
  script steps, validation, approval and deterministic simulation;
- communication provider status with unavailable VISA/CAN/USB providers shown
  honestly;
- equipment model catalog filters by equipment class, manufacturer, status,
  functional role, signal domain, and technology tag using backend summaries;
- classification preset creation flow for blank, preset-backed, and cloned
  models;
- Model Studio Classification and Port Topology sections, including explicit
  CAN bus wording and ADC/DAC/CAN bus separation.

Not implemented:

- campaign instantiation;
- execution package freezing;
- instrument control;
- acquisition;
- complete sensor scaling and engineering curves;
- measurement-chain builder;
- physical asset fleet redesign;
- certified hardware providers;
- FFT or signal processing;
- reporting;
- authentication/RBAC;
- central synchronization.
