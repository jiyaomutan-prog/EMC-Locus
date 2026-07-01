# EMC Locus LAB CONSOLE

LAB CONSOLE is the browser application for laboratory management workflows. In
0.10.0 it provides Template Studio v1 for revisioned test-template authoring.

## Runtime

Normal release launch does not require Node:

```powershell
.\scripts\start-lab.ps1
.\scripts\start-lab.ps1 -SeedDemo
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

## Scope In 0.10.0

Functional:

- template library from the API;
- create and clone template identities;
- structured draft editing;
- server validation;
- checksum-based draft saving;
- submit, approve, and derive revision workflow;
- revision history and audit;
- system status.

Not implemented:

- campaign instantiation;
- execution package freezing;
- instrument control;
- acquisition;
- FFT or signal processing;
- reporting;
- authentication/RBAC;
- central synchronization.
