# Equipment API

Release `0.11.0` adds local-agent routes for revisioned equipment model and
driver profile definitions.

## Equipment Models

```text
GET    /api/v1/equipment-models
POST   /api/v1/equipment-models
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
VISA, vendor CAN and USB providers are not hidden behind simulation.
