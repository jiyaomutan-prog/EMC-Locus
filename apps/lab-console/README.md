# EMC Locus LAB CONSOLE

LAB CONSOLE is the browser application for laboratory management workflows. In
0.17.0 it provides Template Studio v1 plus the Equipment workspace in a focused,
responsive application shell. Active navigation is limited to implemented
workflows, Equipment commands are contextual, the model wizard is presented as
a modal task, and normal revision views favor laboratory labels and lifecycle
states over technical identifiers. Category administration, revisioned
equipment models, driver profiles, communication-provider status, and
signal and correction editors remain available.

The Equipment workspace uses signal-facing language: time-domain sample
conversion, frequency response, sensor power/conditioning, equipment ports, and
signal paths. `Matériels réels` also opens the selected asset's metrology dossier
and its serial-specific characterizations. Internal API collection names remain
stable machine identifiers; they are not used as operator terminology.

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

## Scope In 0.17.0

The LAB CONSOLE feature surface is unchanged in 0.17.0. It remains the owner of
equipment models and serial-specific metrology characterizations. The new
physical station-setup workflow lives in the Qt Locus Test Station and consumes
those controlled definitions through the local agent; it does not duplicate
their maintenance in this browser application.

## Scope In 0.16.0

Physical-asset metrology changes:

- one asset-centered dossier with serviceability, calibration, model reference,
  and characterization history;
- guided choice between a time conversion and a frequency response;
- validity, decision, method, uncertainty, certificate reference, and uploaded
  evidence recorded with the serial-specific result;
- readable applicability and correction summaries with audit/checksum evidence
  progressively disclosed;
- end-to-end persistence, audit, outbox, reload, and exact desktop viewport
  acceptance.

Still not implemented: runtime correction selection or application, real
acquisition, station wiring, FFT, reporting, authentication/RBAC, and central
sync.

## Scope In 0.15.1

Experience corrections:

- a first decision between time-sampled conversion and spectrum-frequency
  compensation;
- descriptive names required at creation while internal identities are
  generated automatically;
- readable approved-reference labels, revision history, and change journal;
- French CSV headings for values handled by laboratory users;
- exact 1440 x 900 and 1280 x 720 Playwright acceptance screenshots.

The typed 0.15.0 domain contracts and API routes remain unchanged.

## Scope In 0.15.0

Signal and correction changes:

- explicit equipment input and output port cards;
- revision-pinned signal paths from input to result output;
- time-domain conversions with gain, offset, and overload/clipping limits;
- frequency responses with explicit amplitude operation and optional phase;
- sensor power and conditioning clearly separated from DUT excitation;
- controlled references resolved by identity, revision, and checksum.

Still not implemented: runtime correction application, real acquisition,
station wiring, direct correction assignment to a serialized asset, FFT,
reporting, authentication/RBAC, and central sync.

## Scope In 0.14.0

Equipment repository changes:

- universal editable `Général` category inherited by all equipment families;
- visible required/optional semantics in category and model creation;
- create, edit, and archive field definitions from Repository Administration;
- actual local file upload for equipment-model documentation;
- serial-numbered material registration in the metrology domain from an
  approved equipment model;
- one form-preview workflow with technical evidence under disclosure.

Still not implemented: station wiring, a graphical measurement-chain builder,
real acquisition, FFT, reporting, authentication/RBAC, and central sync.

## Scope In 0.13.3

Experience changes:

- active navigation limited to Methods, Equipment, and local System status;
- future modules shown as roadmap context rather than clickable dead screens;
- collapsible desktop sidebar and compact responsive navigation;
- contextual catalog, engineering, driver, and repository-administration
  controls;
- focused modal equipment wizard with stable actions;
- human revision labels and business titles in normal model and engineering
  views;
- raw IDs, checksums, JSON, and traceability fields retained behind explicit
  advanced disclosures;
- two-column editor layouts at normal desktop widths, with validation below the
  working form and single-column mobile reflow.

Still not implemented:

- physical serial-numbered asset fleet;
- station connection mapping;
- graphical measurement-chain builder;
- real acquisition, FFT, reporting, authentication/RBAC, and central sync.

## Scope In 0.13.2

Functional:

- grouped Equipment navigation for the equipment catalog, signal corrections,
  drivers/actions, and repository administration;
- category tree rows with indentation, expand/collapse, folder icons,
  selection state, hover state, keyboard selection, and contextual `...`
  actions;
- Repository Administration centered on the selected category with
  Informations, Sous-categories, Formulaire, Previsualisation, and Diagnostic
  avance tabs;
- label-first category and field creation with generated internal codes hidden
  in advanced options;
- choice field list editor for visible option editing without comma-separated
  text as the primary UI;
- explicit wizard steps for Categorie, Sous-categorie, Identification, and
  Verification;
- catalog filtering by arbitrary-depth category subtrees.

Normal user mode hides internal category IDs, field codes, checksums, revision
IDs, schema versions, and raw JSON. Diagnostic avancé remains available for
method engineers and developers when technical evidence is needed.

Still not implemented:

- physical serial-numbered asset fleet;
- station connection mapping;
- graphical measurement-chain builder;
- real acquisition, FFT, reporting, authentication/RBAC, and central sync.

## Scope In 0.13.1

Functional:

- Equipment Repository Administration for categories, field dictionary, entry
  templates, and structural defaults;
- tree/list browsing by user-facing root category and subcategory;
- model creation wizard: root category, subcategory, category-adapted fields,
  review, and draft creation;
- demo-data filter (`hide`, `show`, `only`) with explicit demo marking;
- technical classification and raw JSON moved behind Advanced / Diagnostics.

Still not implemented:

- physical serial-numbered asset fleet;
- station connection mapping;
- graphical measurement-chain builder;
- real acquisition, FFT, reporting, authentication/RBAC, and central sync.

## Scope In 0.13.0

Functional:

- Capteurs / transducteurs studio with structured physical input, electrical
  output, sensor power/conditioning, conversion references, frequency-response
  references, revision history, audit, and advanced JSON sections;
- Conversions temporelles studio with linear, two-point, polynomial,
  lookup-table, piecewise, and expression definitions, including lookup CSV
  paste/export;
- Reponses frequentielles studio with response type, axes, table editor, CSV
  paste/import/export, simple 1D SVG plot, validation, and deterministic
  evaluation through the agent;
- Voies DAQ studio with channel kind, signal domain, input modes, ranges,
  sampling, coupling, triggering, synchronization, and sensor-power fields;
- Chaines d'acquisition studio with a readable DAQ -> sensor electrical signal
  -> time conversion -> frequency response -> result chain summary;
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
- richer time-conversion and frequency-response definitions;
- measurement-chain builder;
- physical asset fleet redesign;
- certified hardware providers;
- FFT or signal processing;
- reporting;
- authentication/RBAC;
- central synchronization.
