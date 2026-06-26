# Storage Migrations

EMC Locus uses versioned SQLite migrations for the first local/offline storage
target.

The storage layout intentionally follows the repository split:

```text
storage/sqlite/
  metrology/
  projects/
  test_definitions/
  measurement_data/
  update_catalog/
```

Each folder is applied to a separate SQLite database file in a future runtime.
This allows a field station to refresh reference repositories without rewriting
project records or raw measurement data.

## Domain Boundaries

### Metrology

Owns instrument identities, families, availability, calibration requirements,
and calibration certificates.

### Projects

Owns customer projects, audit events, contract review, campaigns, measurement
run references, equipment selections, and report workflow state.

### Test Definitions

Owns standards, methods, method revisions, approved parameters, acceptance
criteria, processing graph definitions, and step-by-step evidence expectations.

### Measurement Data

Owns immutable raw and processed dataset records, signal channel metadata,
processing graph lineage, and result artifacts.

### Update Catalog

Owns signed package metadata, compatibility ranges, offline install permission,
and installation records.

The Rust core now mirrors these storage concepts with update bundles, semantic
software versions, package signatures, compatibility-range validation,
rollback references, and install-plan gates. Persistence APIs still need to map
these domain objects to `update_packages` and `update_install_records`.

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

## Early Python Adapters

The Python package now exposes first SQLite-backed adapters:

- `MetrologyRepository`;
- `MeasurementDataRepository`;
- `ProjectRepository`;
- `UpdateCatalogRepository`.

They can initialize a local database from the matching migration domain and
perform minimal insert/count/query operations for smoke testing:

- instrument records;
- calibration records;
- project records;
- project audit events.
- instrument lookup/listing;
- latest calibration lookup;
- project lookup/listing;
- ordered project audit-event listing.
- instrument availability and capability updates;
- calibration attachment updates;
- project stage changes with an audit event in the same transaction;
- contract-review item completion/upsert;
- per-connection SQLite foreign-key enforcement.
- update package insert/count/get/list APIs;
- update install record insert/count/list APIs.
- immutable dataset insert/count/get/list-by-run APIs;
- signal channel insert/list APIs;
- processing graph insert/list APIs;
- result artifact insert/list APIs.

These adapters are intentionally small. They prove that the migration domains
are usable from application code before broader query APIs, synchronization, or
Rust storage adapters are introduced.
