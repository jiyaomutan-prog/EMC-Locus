# Physical Station Setup API

Version `0.17.0` exposes one local-agent workflow for preparing a real
measurement setup. The API is revisioned and local-first. It prepares the
physical chain; it does not control instruments or process measurement data.

## Routes

```text
POST /api/v1/station-setups
GET  /api/v1/station-setups
GET  /api/v1/station-setups/{setup_id}

GET  /api/v1/station-setups/{setup_id}/revisions
POST /api/v1/station-setups/{setup_id}/revisions
GET  /api/v1/station-setups/{setup_id}/revisions/{revision_id}
PUT  /api/v1/station-setups/{setup_id}/revisions/{revision_id}/definition
GET  /api/v1/station-setups/{setup_id}/revisions/{revision_id}/readiness
POST /api/v1/station-setups/{setup_id}/revisions/{revision_id}/transitions/ready

GET  /api/v1/station-setups/{setup_id}/audit-events
```

## Definition

A station definition contains:

- stable setup identity and readable label;
- stable `laboratory_location_id` plus its readable
  `laboratory_location_label` snapshot;
- planned use date and quality mode;
- real asset bindings with the asset revision and pinned approved model
  revision/checksum;
- physical connections from one typed model port to another;
- selected serial-specific time conversions or frequency responses, pinned by
  characterization id and checksum.

The agent canonicalizes the complete definition and returns a prefixed SHA-256
checksum. Collection order does not change the checksum.

New definitions use
`emc-locus.station-measurement-setup-definition.v2`. Creation requires both
location fields:

```json
{
  "setup_id": "SETUP-RF-001",
  "label": "Chaîne de mesure RF",
  "laboratory_location_id": "LAB-LOCATION-CEM-1",
  "laboratory_location_label": "Poste CEM 1",
  "planned_use_on": "2026-07-16",
  "execution_mode": "accredited",
  "actor": "test.technician",
  "reason": "préparer le montage"
}
```

The location ID is compared with the planned slot's location ID. Labels are
display snapshots only: renaming a location does not break compatibility, and
two locations with the same label remain distinct. Normal application users
select a location by label and never type the ID.

Historical v1 definitions remain readable with their original checksum. Their
missing stable identity is explicit and blocks their use in a new ready
planned-test preparation until a v2 draft is created.

## Draft Replacement

`PUT .../definition` requires:

```json
{
  "expected_definition_checksum": "sha256:...",
  "definition": {},
  "actor": "test.technician",
  "reason": "select materials, ports and correction",
  "operation_id": "op-station-save-001"
}
```

Only a draft can be replaced. A stale checksum returns
`station_setup_concurrent_update`. Reusing an operation id with another payload
returns `operation_replay_mismatch`.

## Readiness

Readiness is derived for the complete setup context, including planned date and
quality mode. It reports `ready` and a structured issue list. Dimensions are:

- `structure`;
- `asset_identity`;
- `serviceability`;
- `calibration_validity`;
- `missing_evidence`;
- `nonconformance`;
- `port_compatibility`;
- `correction_validity`.

Issues identify affected material bindings or physical connections. Known
incompatibilities are blocking; absent optional physical information may be a
warning.

## Ready Revision

The `ready` transition repeats readiness evaluation and requires the current
definition checksum. A blocked setup returns `station_setup_not_ready` with the
structured readiness payload. A successful transition makes the revision
immutable. Future changes use `POST .../revisions` with a ready source revision
and create a deterministic child draft.

## Evidence

Create, draft replacement, ready transition and derivation persist atomically:

- the station revision change in `station.sqlite`;
- an explicit station audit event;
- an operation replay record;
- a pending `station_configurations` outbox operation in `sync.sqlite`.

Authenticated identity, RBAC, electronic signatures, central synchronization,
real acquisition and correction application are outside this release.
