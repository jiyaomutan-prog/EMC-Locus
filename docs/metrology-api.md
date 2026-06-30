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
  "serviceability_status": "usable",
  "serviceability_reason": "Initial entry",
  "capabilities": {
    "frequency_hz": {
      "min": 9000,
      "max": 3000000000
    }
  },
  "metrology_notes": "Reference spectrum analyzer"
}
```

Required fields are `asset_id`, `family`, `category_code`, `manufacturer`,
`model`, `serial_number`, and `calibration_requirement`.

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

## Current Boundary

Version `0.6.3` intentionally limits this API to instrument registration and
instrument reads. Calibration event recording, serviceability changes,
readiness assessment, metrology audit events, and strict operation-id
idempotence are delivered in later `0.6.x` tranches on the path to `0.7.0`.
