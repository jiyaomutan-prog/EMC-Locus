# EMC Locus LAB CONSOLE

LAB CONSOLE is the browser application for laboratory management workflows. In
0.12.0 it provides Template Studio v1 plus the Equipment workspace with
revisioned equipment models, driver profiles, communication-provider status,
backend-owned physical classification registries, preset-backed model creation,
catalog filters, and structured port topology editing.

## Runtime

Normal release launch does not require Node:

```powershell
.\scripts\start-lab.ps1
.\scripts\start-lab.ps1 -SeedDemo
.\scripts\start-lab.ps1 -SeedEquipmentDemo
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

Vite proxies `/api` to `http://127.0.0.1:8765` during development. Production
does not use a Node server.

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
