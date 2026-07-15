# Storage Migrations

EMC Locus uses versioned SQLite migrations for the first local/offline storage
target.

The storage layout intentionally follows the repository split:

```text
storage/sqlite/
  metrology/
  projects/
  test_definitions/
  equipment/
  measurement_data/
  update_catalog/
  sync/
```

Each folder is applied to a separate SQLite database file in a future runtime.
This allows a field station to refresh reference repositories without rewriting
project records or raw measurement data.

## Domain Boundaries

### Metrology

Owns instrument identities, instrument categories, families, availability,
calibration requirements, calibration certificates, and category source
provenance. Version 3 adds part numbers, calibration periodicity, metrology
notes, and controlled instrument document attachments. Version 4 adds
`serviceability_status`, `serviceability_reason`,
`serviceability_updated_at`, and `legacy_availability` so planning reservations
remain separate from operational service state. Version 5 adds revisioned
calibration events, per-instrument due-soon warning thresholds, and computed
calibration-status policy metadata while preserving the legacy
`calibration_records` table. Version 6 adds `metrology_audit_events` so
agent-owned metrology writes can produce local audit evidence and sync outbox
operations atomically. Version 7 backfills legacy `calibration_records` into
`calibration_events` for the agent-backed computed-status and readiness paths
without deleting the original records. Version 8 adds an optional, indivisible
Equipment Repository model identity, revision, and canonical checksum reference
to physical instruments. No cross-database foreign key or taxonomy alias is
invented between equipment categories and metrology instrument categories.
Version 9 adds immutable typed characterization events for one physical item.
Version 10 extends their source and validity evidence, then adds reviewed
`asset_correction_assignments` with pinned model/source checksums, lifecycle,
conditions and a unique active correction per material/requirement context.

### Projects

Owns customer projects, audit events, contract review, campaigns, measurement
run references, equipment selections, report workflow state, and service
planning items used to schedule test execution. Version 3 adds
`simulated_test_executions` and `simulated_test_execution_instruments` so the
first simulated EMC launch workflow can persist refused and completed attempts,
metrology-readiness verdicts, instrumentation snapshots, and deterministic
simulated results. Version 4 adds `attached_documents` and
`document_audit_events` for the first shared document registry. The table stores
metadata, owner references, storage URI, checksum, revision, applicability, and
confidentiality only; file bytes remain outside SQLite.

### Test Definitions

Owns standards, methods, method revisions, approved parameters, acceptance
criteria, processing graph definitions, step-by-step evidence expectations, and
the adjustable test-category taxonomy used by planning and methods. Version 3
adds `test_templates` and `test_template_audit_events` for the first
agent-owned draft template workflow. Template records keep structured variables,
lock policy, instrumentation chain slots, sequence steps, limits, and
post-processing definitions as metadata JSON; execution data remains outside
this repository. The 0.8.4 lifecycle slice reuses this schema to move templates
from `draft` to `under_review` and from `under_review` to `approved` with audit
and sync outbox evidence; no additional migration is required for that behavior.
Version 4 is a deliberate 0.9.0 reset of that provisional shape. It drops the
0.8.x `test_templates` table and recreates the runtime model as
`test_template_identities`, `test_template_revisions`, and
`test_template_audit_events`. The new model separates stable identity from
revisioned content, stores canonical typed definition JSON and SHA-256
definition checksums, tracks parent revisions, and records audit events with
explicit old/new revision and checksum fields. There is no dual-read or
dual-write compatibility path after migration. Version 5 adds a partial unique
index on `test_template_revisions(template_id) WHERE status = 'draft'` so the
database enforces one active draft per template identity.

The Python adapter requires optional `test_method_revisions.checksum` evidence
to use canonical `sha256:<64 lowercase hex characters>` form before method
revision creation or approval can persist it.

### Equipment

Owns revisioned equipment model definitions and driver profiles. This domain is
separate from metrology: it describes what a model and driver are, not which
physical asset is calibrated or assigned to a station. Version 1 creates
equipment class and unit registries, `equipment_model_identities`,
`equipment_model_revisions`, `driver_profile_identities`,
`driver_profile_revisions`, `equipment_audit_events`, and single-active-draft
indexes. Complex definitions are stored as canonical JSON with SHA-256
checksums, while identities, statuses, model links, revision ids and approved
pointers remain indexed columns. Version 2 adds first-class physical
classification registries, backend-owned classification presets with port
topology, and transactionally maintained model summary tables so catalog
filters by class, manufacturer, status, functional role, signal domain, and
technology tag do not scan canonical definition JSON. Version 3 adds the
measurement-engineering definition layer in the same repository:
`sensor_definition_*`, `scaling_profile_*`, `engineering_curve_*`,
`daq_channel_profile_*`, `acquisition_channel_recipe_*`, and
`measurement_engineering_audit_events`. These aggregates follow the same
revision discipline as models and driver profiles: one active draft, immutable
approved revisions, approval supersession, checksum CAS for draft saves, audit,
and outbox evidence. They describe reusable measurement-chain definitions and
recipes; they do not create physical assets or runtime acquisitions. Version 4
adds the Equipment Repository UX foundation: hierarchical categories, field
definitions, category field rules, model field values, model template
snapshots, `root_category_id` summaries, and `is_demo` summaries. This
migration seeds structural taxonomy/template defaults only and does not seed
demo models, demo sensors, demo drivers, or demo acquisition recipes.
Version 5 inserts `general_equipment` above the seven classification families,
moves common rules to that universal inheritance root, archives the provisional
`variant` and documentation-URL defaults, and introduces a typed
`file_reference` documentation field. File payloads are stored outside SQLite
under the local storage root; immutable model JSON stores their SHA-256
manifest.
Version 6 records the correction-requirement contract used in canonical model
definitions. Existing revision JSON is not rewritten in place: a legacy
transformation can only become a correction requirement through a new,
controlled model revision. No existing generic curve is migrated into a
physical item's metrology history.

### Measurement Data

Owns immutable raw and processed dataset records, signal channel metadata,
processing graph lineage, result artifacts, and dataset retention evidence.
The Python adapter requires dataset, processing-graph, source-dataset, and
result-artifact checksums to use canonical
`sha256:<64 lowercase hex characters>` evidence before insertion.
Instrument-observation checksum lookups require the same canonical prefixed
lowercase form before runtime traceability rows are queried.

### Update Catalog

Owns signed package metadata, compatibility ranges, offline install permission,
install-plan validation evidence, and installation records.

### Sync

Owns synchronization conflict records and action-plan evidence for resolving or
deferring conflicts between local and reference repository snapshots. Version 2
adds a durable operation journal for idempotent local-first changes, with
actor, device, correlation, base revision, resulting revision, normalized JSON
payload, SHA-256 payload checksum, and pending/applied/failed statuses. Version
3 adds entity snapshots and sync checkpoints so stations can keep local replay
baselines and peer/domain/direction cursors before central merge is introduced.
Version 4 extends the sync domain vocabulary with `equipment` so model/driver
catalog operations can enter the same local outbox as project, metrology and
template operations.
The Python adapter validates payload and snapshot checksum evidence as
canonical `sha256:<64 lowercase hex characters>` before insert or replay.
Snapshot conflict detection revalidates the persisted local and reference
snapshot checksums before comparison, so imported or constraint-bypassed rows
cannot open conflicts with non-canonical digest evidence.

The Rust core now mirrors these storage concepts with update bundles, semantic
software versions, package signatures, compatibility-range validation,
rollback references, and install-plan gates. Python persistence APIs map package
metadata into `update_packages`, store gate results in
`update_install_validation_evidence`, and can link accepted evidence to
`update_install_records`.
The Python update-catalog adapter requires signed package checksums to use
canonical `sha256:<64 lowercase hex characters>` evidence before inserting
package metadata.

## Project Planning Ownership Migration

`storage/sqlite/projects/0006_service_schedule_agent_ownership.sql` upgrades
the early planning table without rewriting historical migrations. It adds a
monotonic row revision plus creator and last-editor evidence, and records the
Local Agent as the application writer. Existing development rows receive the
explicit `legacy-import` actor until they are changed through a controlled
agent operation.

The migration does not move planning to another database and does not create a
dual-write path. The Local Agent attaches `sync.sqlite` so each schedule write,
project audit event, and pending outbox operation commits atomically.

## Cross-Domain Links

SQLite foreign keys are used inside a domain. Links across domains are stored as
stable references such as project code, asset id, certificate reference, method
code, dataset checksum, or package version.

That rule is deliberate: cross-domain references must survive export, offline
snapshot restore, and delayed synchronization.

## Validation

Python exposes a small validation helper:

```text
$env:PYTHONPATH='python'; py -c "from pathlib import Path; from emc_locus.migrations import validate_sqlite_migrations; print(validate_sqlite_migrations(Path('storage/sqlite')))"
```

The helper checks migration filenames, detects duplicate versions per domain,
and executes each domain's SQL in a fresh in-memory SQLite database.
Repository initialization also applies missing domain migrations to an existing
database that already has a `schema_migrations` table. The Rust local agent now
initializes `projects.sqlite`, `sync.sqlite`, `metrology.sqlite`,
`test_definitions.sqlite`, and `equipment.sqlite`.

## Early Python Adapters

The Python package now exposes first SQLite-backed adapters:

- `MetrologyRepository`;
- `MeasurementDataRepository`;
- `ProjectRepository`;
- `TestDefinitionRepository`;
- `UpdateCatalogRepository`;
- `SyncRepository`.

They can initialize a local database from the matching migration domain and
perform minimal insert/count/query operations for smoke testing:

- instrument records;
- calibration records;
- instrument category records;
- instrument category source records;
- project records;
- project audit events.
- instrument lookup/listing;
- instrument category lookup/listing and domain filtering;
- latest calibration lookup;
- project lookup/listing;
- ordered project audit-event listing.
- instrument availability and capability updates;
- instrument serviceability updates separate from legacy availability;
- calibration attachment updates;
- instrument document insert/list APIs;
- project stage changes with an audit event in the same transaction;
- contract-review item completion/upsert;
- service schedule insert/list/status APIs;
- per-connection SQLite foreign-key enforcement.
- update package insert/count/get/list APIs;
- update-install validation evidence insert/count/get APIs;
- update install record insert/count/list APIs with optional accepted evidence
  linkage.
- immutable dataset insert/count/get/list-by-run APIs;
- dataset retention event insert/list APIs and retention-status filtering;
- signal channel insert/list APIs;
- processing graph insert/list APIs;
- result artifact insert/list APIs.
- standard insert/get/list APIs;
- test category insert/get/list APIs with seeded emission/immunity taxonomy;
- test method insert/get/list APIs;
- method revision insert/approval/list APIs;
- ordered test-step insert/list APIs with duplicate-sequence rejection.
- agent-owned test-template draft creation and read/list/audit API through
  `emc-locus-agent`.
- agent-owned test-template submit-for-review and approve transitions through
  `emc-locus-agent`.
- agent-owned sensor definitions, scaling profiles, engineering curves, DAQ
  channel profiles, and acquisition channel recipes through `emc-locus-agent`.
- synchronization conflict insert/count/get/list APIs;
- synchronization action-plan insert/list APIs;
- transactional conflict resolution/defer APIs with optional audit-event
  references.
- synchronization operation-journal insert/count/get/list APIs;
- synchronization operation status transitions for applied and failed replay
  outcomes.
- synchronization entity snapshot insert/count/get/latest APIs;
- synchronization checkpoint upsert/get/list APIs for push, pull, and
  bidirectional cursors.
- transactional local replay from a pending operation into an entity snapshot.
- deterministic conflict creation from divergent local/reference snapshots.
- idempotent conflict action-plan suggestion without automatic resolution.

These adapters are intentionally small. They prove that the migration domains
are usable from application code before broader query APIs, synchronization, or
Rust storage adapters are introduced.
