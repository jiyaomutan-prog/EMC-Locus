# Metrology API

The first local metrology API is served by `emc-locus-agent` on loopback only.

```text
cargo run -q -p emc-locus-agent -- serve --storage-root data\agent --migrations-root storage\sqlite --bind 127.0.0.1:8765
```

## Routes

```text
GET  /api/v1/metrology/instruments
POST /api/v1/metrology/instruments
GET  /api/v1/metrology/instruments/{asset_id}
GET  /api/v1/metrology/instruments/{asset_id}/calibrations
POST /api/v1/metrology/instruments/{asset_id}/calibrations
GET  /api/v1/metrology/instruments/{asset_id}/characterizations
POST /api/v1/metrology/instruments/{asset_id}/characterizations
GET  /api/v1/metrology/instruments/{asset_id}/characterizations/{characterization_id}
GET  /api/v1/metrology/instruments/{asset_id}/characterizations/{characterization_id}/audit-events
POST /api/v1/metrology/files
GET  /api/v1/metrology/instruments/{asset_id}/corrections
POST /api/v1/metrology/instruments/{asset_id}/corrections
GET  /api/v1/metrology/instruments/{asset_id}/corrections/{assignment_id}
GET  /api/v1/metrology/instruments/{asset_id}/corrections/{assignment_id}/audit-events
GET  /api/v1/metrology/corrections/review-queue
POST /api/v1/metrology/instruments/{asset_id}/corrections/{assignment_id}/transitions/submit-for-review
POST /api/v1/metrology/instruments/{asset_id}/corrections/{assignment_id}/transitions/approve-and-activate
POST /api/v1/metrology/instruments/{asset_id}/corrections/{assignment_id}/transitions/reject
POST /api/v1/metrology/instruments/{asset_id}/corrections/{assignment_id}/transitions/request-changes
POST /api/v1/metrology/instruments/{asset_id}/corrections/resolve
GET  /api/v1/metrology/instruments/{asset_id}/status?checked_on=YYYY-MM-DD
POST /api/v1/metrology/instruments/{asset_id}/serviceability
POST /api/v1/metrology/readiness
GET  /api/v1/metrology/instruments/{asset_id}/audit-events
```

## Register Instrument

```json
{
  "asset_id": "SA-001",
  "family": "receiver",
  "category_code": "RF-SPECTRUM-ANALYZER",
  "manufacturer": "Example",
  "model": "SA9000",
  "serial_number": "SN-001",
  "part_number": "PN-SA9000",
  "calibration_requirement": "required",
  "calibration_period_months": 12,
  "calibration_due_warning_days": 45,
  "serviceability_status": "usable",
  "serviceability_reason": "Initial entry",
  "capabilities": {
    "frequency_hz": {
      "min": 9000,
      "max": 3000000000
    }
  },
  "metrology_notes": "Reference spectrum analyzer",
  "actor": "metrology.admin",
  "reason": "Initial registration",
  "operation_id": "op-register-SA-001"
}
```

Required fields are `asset_id`, `family`, `manufacturer`, `model`,
`serial_number`, `calibration_requirement`, `actor`, `reason`, and
`operation_id`. A direct metrology registration also requires `category_code`.
An asset created from Equipment Repository instead supplies the complete typed
catalog reference:

```json
{
  "equipment_model_id": "EQM-RF-LNA-001",
  "equipment_model_revision_id": "EQM-RF-LNA-001-rev-0003",
  "equipment_model_checksum": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}
```

These three values are indivisible. The checksum is the canonical
`sha256:<64 lowercase hex>` checksum of the approved model definition. The metrology taxonomy and the
Equipment Repository hierarchy deliberately remain separate: no equipment
category identifier is inserted into the `instrument_categories` foreign key.

Since `0.14.0`, LAB CONSOLE exposes this registration through `Matériels réels`
and pre-fills model identity, family, capabilities, and the immutable approved
revision reference. The serial number and part number remain properties of the
physical metrology asset, not of the reusable model.

Accepted calibration requirements are:

```text
required
conditional
not_required
```

Accepted serviceability states are:

```text
usable
restricted
out_of_service
retired
```

`capabilities` can be a JSON object or array. A legacy `capabilities_json`
string is also accepted by the local API for simple clients.

## Record Calibration

```json
{
  "event_id": "CAL-SA-001-2026",
  "certificate_reference": "CERT-SA-001-2026",
  "calibrated_at": "2026-06-30",
  "due_at": "2027-06-30",
  "provider": "Accredited Lab",
  "decision": "conforming",
  "as_found_status": "conforming",
  "as_left_status": "conforming",
  "adjustment_performed": false,
  "uncertainty_summary": {
    "level_db": 0.6
  },
  "traceability_reference": "SI-chain-001",
  "comment": "Annual calibration",
  "document_manifest": {
    "object_id": "obj-cert",
    "original_filename": "cert.pdf",
    "mime_type": "application/pdf",
    "size_bytes": 12,
    "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "storage_key": "metrology/SA-001/cert.pdf",
    "revision": "A"
  },
  "recorded_by": "metrology.admin",
  "actor": "metrology.admin",
  "reason": "Annual calibration",
  "operation_id": "op-cal-SA-001-2026"
}
```

Accepted decisions are:

```text
conforming
nonconforming
indeterminate
not_assessed
```

The agent stores the document metadata as a manifest only. It does not store the
PDF, image, or spreadsheet payload in SQLite. When a document manifest includes
`sha256`, it must use the canonical unprefixed 64-character lowercase
hexadecimal SHA-256 digest.

## Record An Asset Characterization

An asset characterization is an immutable metrology event for one physical
serial number. It does not edit the approved equipment model or its generic
correction definition.

```json
{
  "characterization_id": "CHAR-RF-CABLE-001-2026",
  "performed_on": "2026-07-01",
  "valid_until": "2027-07-01",
  "provider": "Internal EMC laboratory",
  "method_reference": "MET-RF-CABLE-001",
  "decision": "conforming",
  "definition": {
    "definition_schema_version": "emc-locus.asset-characterization-definition.v1",
    "characterization_id": "CHAR-RF-CABLE-001-2026",
    "asset_id": "RF-CABLE-001",
    "label": "Measured cable loss",
    "correction": {
      "correction_kind": "frequency_response",
      "correction": {
        "definition_schema_version": "emc-locus.engineering-curve-definition.v1",
        "curve_id": "CHAR-RF-CABLE-001-2026",
        "curve_type": "cable_loss",
        "label": "Measured cable loss",
        "signal_representation": "frequency_domain_spectrum",
        "independent_axes": [
          {"axis": "frequency", "quantity": "frequency", "unit": "Hz"}
        ],
        "dependent_values": [
          {
            "value_id": "amplitude",
            "quantity": "dimensionless",
            "unit": "dB",
            "component": "amplitude",
            "operation": "add"
          }
        ],
        "points": [
          {"axis_values": {"frequency": 1000000}, "values": {"amplitude": 0.12}},
          {"axis_values": {"frequency": 1000000000}, "values": {"amplitude": 2.91}}
        ],
        "interpolation": "log_x_linear_y",
        "extrapolation_policy": "forbidden"
      }
    },
    "uncertainty": {
      "expanded_uncertainty": 0.2,
      "unit": "dB",
      "coverage_factor": 2,
      "confidence_level_percent": 95
    }
  },
  "certificate_reference": "CERT-RF-CABLE-001-2026",
  "document_manifest": {
    "object_id": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "original_filename": "certificate.pdf",
    "mime_type": "application/pdf",
    "size_bytes": 1234,
    "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "storage_key": "objects/metrology/aa/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  },
  "recorded_by": "metrology.operator",
  "actor": "metrology.operator",
  "reason": "Record annual cable characterization",
  "operation_id": "op-characterization-RF-CABLE-001-2026"
}
```

The typed definition accepts either:

- `time_conversion`, using the time-sample gain, offset, units, and optional
  overload/clipping limits contract;
- `frequency_response`, using at least two frequency points, one amplitude
  component, and optional phase.

The nested correction identity must equal `characterization_id`. The asset id,
dates, provider, method, decision, definition, recorded-by identity, and
operation context are validated before the transaction starts. A replay with
the same `operation_id` and payload is idempotent; a different payload is a
structured conflict. Successful writes insert the characterization, audit
event, and metrology outbox operation atomically.

Proof files are uploaded first through `POST /api/v1/metrology/files` with
`original_filename`, `mime_type`, and `content_base64`. The local agent stores
at most 20 MiB below `objects/metrology/<sha256-prefix>/<sha256>` and returns a
content-addressed manifest. File bytes are not stored in SQLite.

## Computed Status

```text
GET /api/v1/metrology/instruments/SA-001/status?checked_on=2026-07-01
```

The status is computed from the instrument calibration requirement, the latest
calibration event, its decision, the due date, the requested check date, and the
instrument due-soon warning threshold. Returned statuses are:

```text
valid
due_soon
expired
missing
not_required
nonconforming
```

## Serviceability And Readiness

```json
{
  "serviceability_status": "out_of_service",
  "serviceability_reason": "Damaged input connector",
  "actor": "metrology.admin",
  "reason": "Asset quarantine",
  "operation_id": "op-service-SA-001"
}
```

```json
{
  "asset_ids": ["SA-001"],
  "execution_mode": "accredited",
  "checked_on": "2026-07-01",
  "context": "pre-run check"
}
```

Readiness responses include `ready`, `instrument_results`, `blocking_issues`,
and `warnings`.

## Reviewed Material Corrections

Creating an assignment requires `assignment_id`, `signal_path_id`,
`requirement_id`, `source_event_id`, optional validity/conditions and the normal
operation context. The agent obtains the model and correction revisions plus
checksums from the pinned instrument and immutable source; clients cannot
substitute those values.

Lifecycle transition bodies require `expected_revision`, `actor`, `reason` and
`operation_id` (plus optional device/correlation ids). Stale revisions return a
structured conflict. Approval and activation are one transaction: any previous
active correction for the same material, requirement and conditions becomes
`superseded` and remains traceable.

Resolution request:

```json
{
  "intended_use_on": "2026-07-14",
  "execution_context": "accredited",
  "conditions": { "polarization": "horizontal" }
}
```

The response is context-derived and explains the selected source, pinned
revision/checksum, validity, fallback warning and blocking state. It is not a
runtime signal-processing operation.

## Current Boundary

Version `0.18.0` keeps Rust as the source of truth for the migrated metrology
vertical slice. LAB CONSOLE and the Python client use the local agent for
instrument registration, calibration events, serviceability, readiness, and
serial-specific time or frequency characterizations.

Migration `0010_asset_correction_assignments.sql` extends source evidence and
adds the reviewed assignment lifecycle used by material readiness and
resolution. It never converts a generic model correction into evidence for a
serial number.

Migration `0007_legacy_calibration_events.sql` backfills legacy
`calibration_records` into `calibration_events` so historical certificates are
visible to the agent-backed computed-status and readiness paths while preserving
the original rows.

Migration `0008_equipment_model_traceability.sql` adds the optional typed
Equipment Repository identity, revision, and checksum link to each physical
instrument. The Rust service requires either a metrology category or this
complete model reference when registering an instrument.

Migration `0009_asset_characterizations.sql` adds immutable characterization
events with canonical typed definitions, checksums, validity, uncertainty,
evidence manifests, and revision evidence. The current execution runtime does
not yet select or apply these corrections; a future test-preparation workflow
must pin the chosen characterization id and checksum.

Document attachment remains partly split. Characterization evidence can upload
bytes through the content-addressed metrology file route. Calibration events
still accept a pre-existing certificate manifest, and standalone instrument
documents remain a legacy SQLite form until a dedicated agent route is added.
Optional certificate or instrument-document checksums must be unprefixed
64-character lowercase hexadecimal SHA-256 digests before storage.
