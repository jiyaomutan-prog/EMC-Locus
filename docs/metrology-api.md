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

Required fields are `asset_id`, `family`, `category_code`, `manufacturer`,
`model`, `serial_number`, `calibration_requirement`, `actor`, `reason`, and
`operation_id`.

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
PDF, image, or spreadsheet payload in SQLite.

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

## Current Boundary

Version `0.6.5` covers instrument registration, instrument reads,
calibration-event recording, calibration-event reads, computed calibration
status, serviceability changes, readiness assessment, audit-event reads, and
strict operation-id idempotence for migrated write paths. Qt migration and the
full HTTP E2E scenario are delivered in later `0.6.x` tranches on the path to
`0.7.0`.
