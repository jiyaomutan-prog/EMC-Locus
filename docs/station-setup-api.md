# Physical Station Setup API

Version `0.22.1` adds the typed v3 requirement/assignment contract described
below. Historical v1/v2 sections remain as compatibility documentation.

Version `0.22.2` keeps this API as the sole physical-assignment and operational
readiness authority. Reusable logical topology now lives in
`emc-locus.measurement-system-template-definition.v1`; it does not replace
station v3. A dated execution configuration pins an approved/validated system
template and the exact station/preparation revision that resolved its roles.
The derivation route is:

```text
POST /api/v1/projects/{project_code}/schedule-items/{item_code}/execution-configuration
GET  /api/v1/execution-configurations/{configuration_id}
```

The POST requires `planned_preparation_revision_id`, exact method/system/station
revision checksums, parameter values and the normal operation context. The
agent rejects assignments that did not come from the pinned preparation. It
does not silently substitute an asset or weaken service, reservation,
metrology, location, driver, port or correction blockers.

Version `0.17.0` exposes one local-agent workflow for preparing a real
measurement setup. The API is revisioned and local-first. It prepares the
physical chain; it does not control instruments or process measurement data.

## Routes

```text
POST /api/v1/station-setups
GET  /api/v1/station-setups
GET  /api/v1/station-setups/asset-options
GET  /api/v1/station-setups/{setup_id}

GET  /api/v1/station-setups/{setup_id}/revisions
POST /api/v1/station-setups/{setup_id}/revisions
GET  /api/v1/station-setups/{setup_id}/revisions/{revision_id}
PUT  /api/v1/station-setups/{setup_id}/revisions/{revision_id}/definition
GET  /api/v1/station-setups/{setup_id}/revisions/{revision_id}/readiness
GET  /api/v1/station-setups/{setup_id}/revisions/{revision_id}/material-requirements/{requirement_id}/candidates
POST /api/v1/station-setups/{setup_id}/revisions/{revision_id}/transitions/qualified
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
`emc-locus.station-measurement-setup-definition.v2`. Creation requires the
stable location identity; the agent derives the readable label from the
laboratory registry:

```json
{
  "setup_id": "SETUP-RF-001",
  "label": "Chaîne de mesure RF",
  "laboratory_location_id": "LAB-LOCATION-CEM-1",
  "planned_use_on": "2026-07-16",
  "execution_mode": "accredited",
  "actor": "test.technician",
  "reason": "préparer le montage"
}
```

The location ID is compared with the planned slot's location ID. Labels are
server-owned display snapshots: renaming a location does not break
compatibility, and two locations with the same label remain distinct. A client
label is ignored during draft replacement and cannot override the registry.
Normal application users select a location by label and never type the ID.

## Executable asset options

`GET /api/v1/station-setups/asset-options` requires:

```text
planned_use_on=YYYY-MM-DD
execution_mode=accredited|non_accredited|investigation
laboratory_location_id=<stable location id>
```

Every returned option embeds the readable physical asset, its exact model pin,
current registry location, administrative availability, computed operational
usage, and authoritative metrology assessment for the requested civil date.
`eligible`, `blocking_reasons` and `warnings` are computed by the agent. Each
reason has a stable code, human message and next action.

LAB CONSOLE keys each option request by `planned_use_on`, `execution_mode` and
`laboratory_location_id`. A context change immediately clears the pending
selection and disables assignment until the matching response arrives. Late
responses for an older context are ignored, while bindings already stored in
the draft remain readable. This client guard prevents stale choices; backend
readiness and write validation remain authoritative.

Unresolved migrated assets, invalid model pins, non-usable or administratively
unavailable assets, active tests, conflicting reservations, missing or
incompatible locations, and blocking metrology states are explained rather
than silently hidden. A restricted asset remains selectable with an explicit
warning. `not_required` metrology is accepted; an expired required calibration
blocks accredited use. Catalogue model identities are never returned by this
physical-asset endpoint.

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
warning. Readiness consumes the same backend eligibility rules as the option
endpoint, so a forged draft cannot bypass location, usage, service,
availability, model-link or metrology blocks.

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

These writes use one `BEGIN IMMEDIATE` transaction with `equipment.sqlite` and
`sync.sqlite` attached. Active-location validation and label derivation happen
inside that boundary. A concurrent archive leaves no partial station, audit,
operation or outbox evidence.

Authenticated identity, RBAC, electronic signatures, central synchronization,
real acquisition and correction application are outside this release.

## Version 3 Material Requirements

`emc-locus.station-measurement-setup-definition.v3` replaces immediate asset
bindings with two first-class collections:

- `material_requirements` describes category pools, stable capability matches
  or one imposed exact `asset_id` using logical ports;
- `material_assignments` pins the selected physical asset revision, inventory
  and serial snapshots, exact immutable model revision/checksum, assignment
  context and logical-to-physical port mappings.

`selection_policy` is `category_pool`, `capability_match` or `exact_asset`.
`assignment_stage` is `setup_definition` or `planned_test_preparation`.
Substitution is explicit; `exact_asset` always means no silent substitution.
An exact asset may be recorded while operationally blocked, but it cannot be
assigned or make the setup ready.

The requirement-specific candidate route requires `planned_use_on`,
`execution_mode` and `laboratory_location_id`; it accepts an optional
`excluded_schedule_item_code`. Each result exposes independent
`requirement_compatible`, `operationally_eligible` and `assignable` states,
technical/driver/port evidence, correction readiness, blockers, warnings and
next actions. Rust owns all matching and unit conversion.

`qualified` freezes a logically coherent definition that may retain mandatory
roles deferred to planned-test preparation. `ready` requires every mandatory
assignment and all contextual checks. Deriving from v2 with
`upgrade_to_v3=true` creates an audited child draft and leaves source JSON and
checksum unchanged.
