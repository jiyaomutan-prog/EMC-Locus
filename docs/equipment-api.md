# Equipment API

Release `0.12.0` extends the revisioned equipment catalog with backend-owned
physical classification registries, classification presets, preset-based model
creation, and indexed catalog filters.

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
and `operation_id`.

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
