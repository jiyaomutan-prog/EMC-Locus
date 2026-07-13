# Equipment API

Release `0.13.0` extends the revisioned equipment catalog with
measurement-engineering definitions: sensors/transducers, scaling profiles,
engineering correction curves, DAQ channel profiles, and logical acquisition
channel recipes. These routes complement the 0.12 physical-classification
catalog; they do not run hardware or acquire data.

## Equipment Models

```text
GET    /api/v1/equipment-models
POST   /api/v1/equipment-models
POST   /api/v1/equipment-models/from-preset
POST   /api/v1/equipment-model-definitions/validate
GET    /api/v1/equipment-models/{equipment_model_id}
POST   /api/v1/equipment-models/{equipment_model_id}/clone
GET    /api/v1/equipment-models/{equipment_model_id}/revisions
GET    /api/v1/equipment-models/{equipment_model_id}/revisions/{revision_id}
POST   /api/v1/equipment-models/{equipment_model_id}/revisions
PUT    /api/v1/equipment-models/{equipment_model_id}/revisions/{revision_id}/definition
POST   /api/v1/equipment-models/{equipment_model_id}/revisions/{revision_id}/transitions/submit-for-review
POST   /api/v1/equipment-models/{equipment_model_id}/revisions/{revision_id}/transitions/approve
GET    /api/v1/equipment-models/{equipment_model_id}/audit-events
```

Draft replacement requires `expected_definition_checksum`, `actor`, `reason`
and `operation_id`. The expected checksum must use canonical
`sha256:<64 lowercase hex characters>` syntax, matching the server-generated
definition checksum exactly.

`GET /api/v1/equipment-models` accepts indexed filters:

- `functional_role`;
- `signal_domain`;
- `technology_tag`;
- `equipment_class`;
- `manufacturer`;
- `status`;
- `q` or `search`.

Role, domain and tag filters use normalized summary tables, not JSON parsing.

`POST /api/v1/equipment-models/from-preset` requires `preset_id`,
`equipment_model_id`, `manufacturer`, `model_name`, `actor`, `reason`, and
`operation_id`, with optional `variant`, `correlation_id`, and `device_id`. It
creates a draft model revision from the selected backend preset and records
audit/outbox evidence with operation kind `equipment_model_created_from_preset`.

## Classification Registries And Presets

```text
GET /api/v1/equipment/registries
GET /api/v1/equipment/classification-presets
GET /api/v1/equipment/classification-presets/{preset_id}
```

Registries expose functional roles, signal domains, port directionalities, flow
roles, and technology tags. Classification presets provide default model class,
role, domains, tags, and port topology. They are catalog authoring aids, not a
driver runtime, acquisition engine, or permission system.

## Driver Profiles

```text
GET    /api/v1/driver-profiles
POST   /api/v1/driver-profiles
POST   /api/v1/driver-profile-definitions/validate
GET    /api/v1/driver-profiles/{driver_profile_id}
GET    /api/v1/driver-profiles/{driver_profile_id}/revisions
GET    /api/v1/driver-profiles/{driver_profile_id}/revisions/{revision_id}
POST   /api/v1/driver-profiles/{driver_profile_id}/revisions
PUT    /api/v1/driver-profiles/{driver_profile_id}/revisions/{revision_id}/definition
POST   /api/v1/driver-profiles/{driver_profile_id}/revisions/{revision_id}/transitions/submit-for-review
POST   /api/v1/driver-profiles/{driver_profile_id}/revisions/{revision_id}/transitions/approve
GET    /api/v1/driver-profiles/{driver_profile_id}/audit-events
POST   /api/v1/driver-profile-simulations
```

A driver revision must reference an approved equipment model revision and its
definition checksum.

## Provider Status

```text
GET /api/v1/equipment/communication-providers
```

The endpoint reports installed or modeled communication providers. Unavailable
VISA, vendor CAN bus and USB providers are not hidden behind simulation.

## Measurement Engineering Definitions

The following aggregate families share the same operation discipline:

- `actor`, `reason`, `operation_id`, optional `device_id` and
  `correlation_id`;
- draft save with `expected_definition_checksum`;
- immutable submitted and approved revisions;
- approval supersedes the previous approved revision for the same identity;
- audit and outbox evidence for every mutating operation.

Write requests are idempotent by `operation_id`. Replaying the same
measurement-engineering create, draft replacement, clone/revision, submit, or
approve request returns the original operation result with `replayed: true`;
reusing the same `operation_id` for a different payload remains a conflict.
Draft replacement checksums must use canonical
`sha256:<64 lowercase hex characters>` syntax before compare-and-swap matching.

### Sensor Definitions

```text
GET    /api/v1/sensor-definitions
POST   /api/v1/sensor-definitions
POST   /api/v1/sensor-definitions/{id}/clone
POST   /api/v1/sensor-definition-definitions/validate
GET    /api/v1/sensor-definitions/{id}
GET    /api/v1/sensor-definitions/{id}/revisions
GET    /api/v1/sensor-definitions/{id}/revisions/{revision_id}
POST   /api/v1/sensor-definitions/{id}/revisions
PUT    /api/v1/sensor-definitions/{id}/revisions/{revision_id}/definition
POST   /api/v1/sensor-definitions/{id}/revisions/{revision_id}/transitions/submit-for-review
POST   /api/v1/sensor-definitions/{id}/revisions/{revision_id}/transitions/approve
GET    /api/v1/sensor-definitions/{id}/audit-events
```

A sensor definition describes a reusable type of sensor or transducer, such as
a current probe, receiving antenna, IEPE accelerometer, microphone, or generic
transducer. It is not a physical asset with a serial number and not a
calibration event.

### Scaling Profiles

```text
GET    /api/v1/scaling-profiles
POST   /api/v1/scaling-profiles
POST   /api/v1/scaling-profiles/{id}/clone
POST   /api/v1/scaling-profile-definitions/validate
GET    /api/v1/scaling-profiles/{id}
GET    /api/v1/scaling-profiles/{id}/revisions
GET    /api/v1/scaling-profiles/{id}/revisions/{revision_id}
POST   /api/v1/scaling-profiles/{id}/revisions
PUT    /api/v1/scaling-profiles/{id}/revisions/{revision_id}/definition
POST   /api/v1/scaling-profiles/{id}/revisions/{revision_id}/transitions/submit-for-review
POST   /api/v1/scaling-profiles/{id}/revisions/{revision_id}/transitions/approve
GET    /api/v1/scaling-profiles/{id}/audit-events
```

Scaling profiles transform a DAQ or sensor electrical signal into an
engineering quantity. Supported definitions include identity, linear,
two-point, polynomial, lookup-table, piecewise-linear, and a limited expression
DSL. Scaling evaluation rejects non-finite inputs and non-finite computed
outputs before they can become traceability evidence. Scaling profiles are
reusable transformation definitions, not calibration certificates.

### Engineering Curves

```text
GET    /api/v1/engineering-curves
POST   /api/v1/engineering-curves
POST   /api/v1/engineering-curves/{id}/clone
POST   /api/v1/engineering-curve-definitions/validate
GET    /api/v1/engineering-curves/{id}
GET    /api/v1/engineering-curves/{id}/revisions
GET    /api/v1/engineering-curves/{id}/revisions/{revision_id}
POST   /api/v1/engineering-curves/{id}/revisions
PUT    /api/v1/engineering-curves/{id}/revisions/{revision_id}/definition
POST   /api/v1/engineering-curves/{id}/revisions/{revision_id}/transitions/submit-for-review
POST   /api/v1/engineering-curves/{id}/revisions/{revision_id}/transitions/approve
POST   /api/v1/engineering-curves/{id}/revisions/{revision_id}/evaluate
GET    /api/v1/engineering-curves/{id}/audit-events
```

Curve definitions cover antenna factor, cable loss, amplifier gain, attenuator
loss, current-probe transfer, voltage-probe transfer, sensor frequency
response, phase response, uncertainty, VSWR, S-parameter magnitude, site
characterization, and generic correction artifacts. Evaluation is deterministic
for 1D curves and returns values, axis values, interpolation mode,
extrapolation flag, optional warning, source revision id, and source checksum.
Validation rejects logarithmic interpolation inputs that cannot be evaluated:
`log_x_linear_y` requires positive x values and `linear_x_log_y` requires
positive dependent values. Evaluation requests for `log_x_linear_y` curves also
reject non-positive axis values before extrapolation is applied. Interpolated
and extrapolated dependent results must remain finite; non-finite results are
returned as controlled validation errors.

Example evaluation request:

```json
{
  "axis_values": {
    "frequency": 100000000.0
  }
}
```

### DAQ Channel Profiles

```text
GET    /api/v1/daq-channel-profiles
POST   /api/v1/daq-channel-profiles
POST   /api/v1/daq-channel-profiles/{id}/clone
POST   /api/v1/daq-channel-profile-definitions/validate
GET    /api/v1/daq-channel-profiles/{id}
GET    /api/v1/daq-channel-profiles/{id}/revisions
GET    /api/v1/daq-channel-profiles/{id}/revisions/{revision_id}
POST   /api/v1/daq-channel-profiles/{id}/revisions
PUT    /api/v1/daq-channel-profiles/{id}/revisions/{revision_id}/definition
POST   /api/v1/daq-channel-profiles/{id}/revisions/{revision_id}/transitions/submit-for-review
POST   /api/v1/daq-channel-profiles/{id}/revisions/{revision_id}/transitions/approve
GET    /api/v1/daq-channel-profiles/{id}/audit-events
```

A DAQ channel profile describes what a class of channel can accept or produce:
kind, signal domain, input quantity/unit, ranges, sampling limits, coupling,
input modes, IEPE, bridge, isolation, synchronization, and triggering. It is
not a runtime channel instance bound to a station.

### Acquisition Channel Recipes

```text
GET    /api/v1/acquisition-channel-recipes
POST   /api/v1/acquisition-channel-recipes
POST   /api/v1/acquisition-channel-recipes/{id}/clone
POST   /api/v1/acquisition-channel-recipe-definitions/validate
GET    /api/v1/acquisition-channel-recipes/{id}
GET    /api/v1/acquisition-channel-recipes/{id}/revisions
GET    /api/v1/acquisition-channel-recipes/{id}/revisions/{revision_id}
POST   /api/v1/acquisition-channel-recipes/{id}/revisions
PUT    /api/v1/acquisition-channel-recipes/{id}/revisions/{revision_id}/definition
POST   /api/v1/acquisition-channel-recipes/{id}/revisions/{revision_id}/transitions/submit-for-review
POST   /api/v1/acquisition-channel-recipes/{id}/revisions/{revision_id}/transitions/approve
GET    /api/v1/acquisition-channel-recipes/{id}/audit-events
```

An acquisition channel recipe links approved DAQ, optional sensor, scaling, and
correction definitions into a reusable logical output channel such as
`current_A`. It is not a campaign execution package and does not start a
measurement.
