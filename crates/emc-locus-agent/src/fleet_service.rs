use crate::equipment_repository::{
    list_equipment_categories, load_equipment_model_identity, load_equipment_model_revision,
};
use crate::fleet_dto::{
    FleetAuditEventDto, FleetAuditEventListDto, LaboratoryLocationDto,
    LaboratoryLocationEnvelopeDto, LaboratoryLocationListDto, PhysicalAssetDto,
    PhysicalAssetEnvelopeDto, PhysicalAssetListDto, PhysicalAssetMetrologySummaryDto,
};
use crate::fleet_repository::{
    archive_laboratory_location, existing_fleet_operation, insert_laboratory_location,
    insert_physical_asset, list_laboratory_locations, list_physical_assets,
    load_fleet_audit_events, load_laboratory_location, load_physical_asset,
    load_physical_asset_by_inventory_code, open_fleet_connection, open_fleet_connection_with_sync,
    request_checksum, update_laboratory_location, update_physical_asset_availability,
    update_physical_asset_identity, update_physical_asset_service_state, write_fleet_evidence,
    FleetEvidenceInput, NewPhysicalAssetRecord, StoredLaboratoryLocation, StoredPhysicalAsset,
    UpdatePhysicalAssetIdentityInput,
};
use crate::{render_json, AgentError};
use emc_locus_core::{
    availability_state_code, ownership_source_code, service_state_code,
    validate_availability_transition, validate_service_state_transition, AvailabilityState,
    EquipmentModelDefinition, LaboratoryLocationDefinition, LaboratoryLocationStatus,
    OwnershipSource, PhysicalAssetDefinition, PinnedEquipmentModel, ServiceState,
    LABORATORY_LOCATION_DEFINITION_SCHEMA_VERSION, PHYSICAL_ASSET_DEFINITION_SCHEMA_VERSION,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FleetOperationContext {
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatePhysicalAssetInput {
    pub inventory_code: String,
    pub serial_number: Option<String>,
    pub part_number: Option<String>,
    pub equipment_model_id: String,
    pub laboratory_location_id: Option<String>,
    pub ownership_source: String,
    pub service_state: String,
    pub availability_state: String,
    pub service_state_reason: String,
    pub notes: String,
    pub calibration_requirement: String,
    pub calibration_period_months: Option<u32>,
    pub calibration_due_warning_days: u32,
    pub metrology_notes: String,
    pub context: FleetOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdatePhysicalAssetInput {
    pub asset_id: String,
    pub expected_revision: u64,
    pub inventory_code: String,
    pub serial_number: Option<String>,
    pub part_number: Option<String>,
    pub laboratory_location_id: Option<String>,
    pub ownership_source: String,
    pub notes: String,
    pub context: FleetOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionPhysicalAssetServiceStateInput {
    pub asset_id: String,
    pub expected_revision: u64,
    pub service_state: String,
    pub service_state_reason: String,
    pub context: FleetOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionPhysicalAssetAvailabilityInput {
    pub asset_id: String,
    pub expected_revision: u64,
    pub availability_state: String,
    pub context: FleetOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateLaboratoryLocationInput {
    pub label: String,
    pub description: String,
    pub context: FleetOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateLaboratoryLocationInput {
    pub location_id: String,
    pub expected_revision: u64,
    pub label: String,
    pub description: String,
    pub context: FleetOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveLaboratoryLocationInput {
    pub location_id: String,
    pub expected_revision: u64,
    pub context: FleetOperationContext,
}

pub fn list_physical_assets_json(storage_root: &Path) -> Result<String, AgentError> {
    let connection = open_fleet_connection(storage_root)?;
    let assets = list_physical_assets(&connection)?;
    let dtos = assets
        .iter()
        .map(|asset| physical_asset_dto(&connection, asset))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(render_json(&PhysicalAssetListDto { assets: dtos }))
}

pub fn get_physical_asset_json(storage_root: &Path, asset_id: &str) -> Result<String, AgentError> {
    let connection = open_fleet_connection(storage_root)?;
    let asset = required_asset(&connection, asset_id)?;
    Ok(render_json(&PhysicalAssetEnvelopeDto {
        asset: physical_asset_dto(&connection, &asset)?,
        replayed: false,
    }))
}

pub fn create_physical_asset(
    storage_root: &Path,
    input: CreatePhysicalAssetInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let ownership_source = parse_ownership_source(&input.ownership_source)?;
    let service_state = parse_service_state(&input.service_state)?;
    let availability_state = parse_availability_state(&input.availability_state)?;
    validate_metrology_dossier_input(
        &input.calibration_requirement,
        input.calibration_period_months,
        input.calibration_due_warning_days,
    )?;
    let asset_id = generated_id(
        "ASSET",
        &input.context.operation_id,
        input.inventory_code.trim(),
    );
    let mut connection = open_fleet_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let request_json = render_json(&json!({
        "asset_id": asset_id,
        "inventory_code": input.inventory_code.trim(),
        "serial_number": trimmed_optional(input.serial_number.as_deref()),
        "part_number": trimmed_optional(input.part_number.as_deref()),
        "equipment_model_id": input.equipment_model_id.trim(),
        "laboratory_location_id": trimmed_optional(input.laboratory_location_id.as_deref()),
        "ownership_source": ownership_source_code(ownership_source),
        "service_state": service_state_code(service_state),
        "availability_state": availability_state_code(availability_state),
        "service_state_reason": input.service_state_reason.trim(),
        "notes": input.notes.trim(),
        "metrology": {
            "calibration_requirement": input.calibration_requirement.trim(),
            "calibration_period_months": input.calibration_period_months,
            "calibration_due_warning_days": input.calibration_due_warning_days,
            "notes": input.metrology_notes.trim()
        }
    }));
    let checksum = request_checksum(&request_json);
    if let Some(operation) =
        existing_fleet_operation(&transaction, "physical_asset", &input.context.operation_id)?
    {
        ensure_replay(
            &operation.entity_id,
            &operation.action,
            &operation.request_checksum,
            &asset_id,
            "physical_asset_created",
            &checksum,
            &input.context.operation_id,
        )?;
        let asset = required_asset(&transaction, &operation.entity_id)?;
        return Ok(render_json(&PhysicalAssetEnvelopeDto {
            asset: physical_asset_dto(&transaction, &asset)?,
            replayed: true,
        }));
    }
    let model = resolve_current_approved_model(&transaction, &input.equipment_model_id)?;
    let location = resolve_active_location(&transaction, input.laboratory_location_id.as_deref())?;
    let definition = PhysicalAssetDefinition {
        definition_schema_version: PHYSICAL_ASSET_DEFINITION_SCHEMA_VERSION.to_owned(),
        inventory_code: input.inventory_code.trim().to_owned(),
        serial_number: trimmed_optional(input.serial_number.as_deref()),
        part_number: trimmed_optional(input.part_number.as_deref()),
        model,
        laboratory_location_id: location.as_ref().map(|item| item.location_id.clone()),
        laboratory_location_label: location.as_ref().map(|item| item.label.clone()),
        ownership_source,
        service_state,
        availability_state,
        service_state_reason: input.service_state_reason.trim().to_owned(),
        notes: input.notes.trim().to_owned(),
    };
    validate_asset_definition(&definition)?;
    let payload_json = render_json(&json!({
        "asset_id": asset_id,
        "definition": definition
    }));
    if load_physical_asset_by_inventory_code(&transaction, &definition.inventory_code)?.is_some() {
        return Err(AgentError::new(
            "physical_asset_inventory_code_conflict",
            "Ce code inventaire est déjà utilisé par un autre exemplaire.",
        ));
    }
    let now = utc_timestamp()?;
    let category_path_json = render_json(&definition.model.category_path);
    insert_physical_asset(
        &transaction,
        NewPhysicalAssetRecord {
            asset_id: &asset_id,
            inventory_code: &definition.inventory_code,
            serial_number: definition.serial_number.as_deref(),
            part_number: definition.part_number.as_deref(),
            equipment_model_id: &definition.model.equipment_model_id,
            equipment_model_revision_id: &definition.model.equipment_model_revision_id,
            equipment_model_checksum: &definition.model.equipment_model_checksum,
            manufacturer_snapshot: &definition.model.manufacturer,
            model_name_snapshot: &definition.model.model_name,
            variant_snapshot: definition.model.variant.as_deref(),
            category_code_snapshot: &definition.model.category_code,
            category_path_json: &category_path_json,
            laboratory_location_id: definition.laboratory_location_id.as_deref(),
            laboratory_location_label_snapshot: definition.laboratory_location_label.as_deref(),
            ownership_source: ownership_source_code(definition.ownership_source),
            service_state: service_state_code(definition.service_state),
            availability_state: availability_state_code(definition.availability_state),
            service_state_reason: &definition.service_state_reason,
            notes: &definition.notes,
            timestamp: &now,
        },
    )?;
    transaction
        .execute(
            "INSERT INTO metrology_db.metrology_asset_dossiers (
                asset_id, calibration_requirement, calibration_period_months,
                calibration_due_warning_days, metrology_notes, legacy_capabilities_json,
                revision, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, '[]', 1, ?6, ?6)",
            params![
                asset_id,
                input.calibration_requirement.trim(),
                input.calibration_period_months,
                input.calibration_due_warning_days,
                input.metrology_notes.trim(),
                now,
            ],
        )
        .map_err(|error| AgentError::new("metrology_dossier_write_failed", error.to_string()))?;
    write_fleet_evidence(
        &transaction,
        evidence(
            "physical_asset",
            &asset_id,
            "physical_asset_created",
            EvidenceRevision::created(),
            EvidencePayload::new(&checksum, &payload_json),
            &input.context,
            &now,
        ),
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    get_physical_asset_json(storage_root, &asset_id)
}

pub fn update_physical_asset(
    storage_root: &Path,
    input: UpdatePhysicalAssetInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let ownership_source = parse_ownership_source(&input.ownership_source)?;
    let mut connection = open_fleet_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let current = required_asset(&transaction, &input.asset_id)?;
    require_resolved_asset(&current)?;
    let location = resolve_active_location(&transaction, input.laboratory_location_id.as_deref())?;
    let definition = definition_from_stored(
        &current,
        input.inventory_code.trim(),
        trimmed_optional(input.serial_number.as_deref()),
        trimmed_optional(input.part_number.as_deref()),
        location.as_ref(),
        ownership_source,
        input.notes.trim(),
    )?;
    validate_asset_definition(&definition)?;
    let payload_json = render_json(&json!({
        "asset_id": input.asset_id,
        "expected_revision": input.expected_revision,
        "identity": {
            "inventory_code": definition.inventory_code,
            "serial_number": definition.serial_number,
            "part_number": definition.part_number,
            "laboratory_location_id": definition.laboratory_location_id,
            "ownership_source": ownership_source_code(definition.ownership_source),
            "notes": definition.notes
        }
    }));
    let checksum = request_checksum(&payload_json);
    if let Some(replay) = replay_asset_operation(
        &transaction,
        &input.context,
        &input.asset_id,
        "physical_asset_updated",
        &checksum,
    )? {
        return Ok(replay);
    }
    require_expected_revision(&current.asset_id, current.revision, input.expected_revision)?;
    let now = utc_timestamp()?;
    let new_revision = update_physical_asset_identity(
        &transaction,
        UpdatePhysicalAssetIdentityInput {
            asset_id: &input.asset_id,
            expected_revision: input.expected_revision,
            inventory_code: &definition.inventory_code,
            serial_number: definition.serial_number.as_deref(),
            part_number: definition.part_number.as_deref(),
            laboratory_location_id: definition.laboratory_location_id.as_deref(),
            laboratory_location_label_snapshot: definition.laboratory_location_label.as_deref(),
            ownership_source: ownership_source_code(definition.ownership_source),
            notes: &definition.notes,
            timestamp: &now,
        },
    )?;
    write_fleet_evidence(
        &transaction,
        evidence(
            "physical_asset",
            &input.asset_id,
            "physical_asset_updated",
            EvidenceRevision::changed(input.expected_revision, new_revision),
            EvidencePayload::new(&checksum, &payload_json),
            &input.context,
            &now,
        ),
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    get_physical_asset_json(storage_root, &input.asset_id)
}

pub fn transition_physical_asset_service_state(
    storage_root: &Path,
    input: TransitionPhysicalAssetServiceStateInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let requested = parse_service_state(&input.service_state)?;
    let mut connection = open_fleet_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let current = required_asset(&transaction, &input.asset_id)?;
    let current_state = parse_service_state(&current.service_state)?;
    let next_availability = if matches!(
        requested,
        ServiceState::InMaintenance | ServiceState::OutOfService | ServiceState::Retired
    ) {
        AvailabilityState::Unavailable
    } else {
        parse_availability_state(&current.availability_state)?
    };
    let reason = input.service_state_reason.trim();
    if requested != ServiceState::Usable && reason.is_empty() {
        return Err(AgentError::new(
            "service_state_reason_required",
            "Indiquez pourquoi l'exemplaire n'est pas pleinement utilisable.",
        ));
    }
    let payload_json = render_json(&json!({
        "asset_id": input.asset_id,
        "expected_revision": input.expected_revision,
        "to": service_state_code(requested),
        "availability_state": availability_state_code(next_availability),
        "service_state_reason": reason
    }));
    let checksum = request_checksum(&payload_json);
    if let Some(replay) = replay_asset_operation(
        &transaction,
        &input.context,
        &input.asset_id,
        "physical_asset_service_state_changed",
        &checksum,
    )? {
        return Ok(replay);
    }
    require_expected_revision(&current.asset_id, current.revision, input.expected_revision)?;
    validate_service_state_transition(current_state, requested).map_err(transition_error)?;
    let now = utc_timestamp()?;
    let new_revision = update_physical_asset_service_state(
        &transaction,
        &input.asset_id,
        input.expected_revision,
        service_state_code(requested),
        availability_state_code(next_availability),
        reason,
        &now,
    )?;
    write_fleet_evidence(
        &transaction,
        evidence(
            "physical_asset",
            &input.asset_id,
            "physical_asset_service_state_changed",
            EvidenceRevision::changed(input.expected_revision, new_revision),
            EvidencePayload::new(&checksum, &payload_json),
            &input.context,
            &now,
        ),
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    get_physical_asset_json(storage_root, &input.asset_id)
}

pub fn transition_physical_asset_availability(
    storage_root: &Path,
    input: TransitionPhysicalAssetAvailabilityInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let requested = parse_availability_state(&input.availability_state)?;
    let mut connection = open_fleet_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let current = required_asset(&transaction, &input.asset_id)?;
    let service_state = parse_service_state(&current.service_state)?;
    let current_availability = parse_availability_state(&current.availability_state)?;
    let payload_json = render_json(&json!({
        "asset_id": input.asset_id,
        "expected_revision": input.expected_revision,
        "to": availability_state_code(requested)
    }));
    let checksum = request_checksum(&payload_json);
    if let Some(replay) = replay_asset_operation(
        &transaction,
        &input.context,
        &input.asset_id,
        "physical_asset_availability_changed",
        &checksum,
    )? {
        return Ok(replay);
    }
    require_expected_revision(&current.asset_id, current.revision, input.expected_revision)?;
    validate_availability_transition(service_state, current_availability, requested)
        .map_err(transition_error)?;
    let now = utc_timestamp()?;
    let new_revision = update_physical_asset_availability(
        &transaction,
        &input.asset_id,
        input.expected_revision,
        availability_state_code(requested),
        &now,
    )?;
    write_fleet_evidence(
        &transaction,
        evidence(
            "physical_asset",
            &input.asset_id,
            "physical_asset_availability_changed",
            EvidenceRevision::changed(input.expected_revision, new_revision),
            EvidencePayload::new(&checksum, &payload_json),
            &input.context,
            &now,
        ),
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    get_physical_asset_json(storage_root, &input.asset_id)
}

pub fn list_laboratory_locations_json(
    storage_root: &Path,
    include_archived: bool,
) -> Result<String, AgentError> {
    let connection = open_fleet_connection(storage_root)?;
    let locations = list_laboratory_locations(&connection, include_archived)?;
    Ok(render_json(&LaboratoryLocationListDto {
        locations: locations.iter().map(location_dto).collect(),
    }))
}

pub fn create_laboratory_location(
    storage_root: &Path,
    input: CreateLaboratoryLocationInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let definition = LaboratoryLocationDefinition {
        definition_schema_version: LABORATORY_LOCATION_DEFINITION_SCHEMA_VERSION.to_owned(),
        label: input.label.trim().to_owned(),
        description: input.description.trim().to_owned(),
        status: LaboratoryLocationStatus::Active,
    };
    validate_location_definition(&definition)?;
    let location_id = generated_id("LOC", &input.context.operation_id, &definition.label);
    let payload_json = render_json(&json!({
        "location_id": location_id,
        "definition": definition
    }));
    let checksum = request_checksum(&payload_json);
    let mut connection = open_fleet_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    if let Some(operation) = existing_fleet_operation(
        &transaction,
        "laboratory_location",
        &input.context.operation_id,
    )? {
        ensure_replay(
            &operation.entity_id,
            &operation.action,
            &operation.request_checksum,
            &location_id,
            "laboratory_location_created",
            &checksum,
            &input.context.operation_id,
        )?;
        let location = required_location(&transaction, &operation.entity_id)?;
        return Ok(render_json(&LaboratoryLocationEnvelopeDto {
            location: location_dto(&location),
            replayed: true,
        }));
    }
    let now = utc_timestamp()?;
    insert_laboratory_location(
        &transaction,
        &location_id,
        &definition.label,
        &definition.description,
        &now,
    )?;
    write_fleet_evidence(
        &transaction,
        evidence(
            "laboratory_location",
            &location_id,
            "laboratory_location_created",
            EvidenceRevision::created(),
            EvidencePayload::new(&checksum, &payload_json),
            &input.context,
            &now,
        ),
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    let connection = open_fleet_connection(storage_root)?;
    let location = required_location(&connection, &location_id)?;
    Ok(render_json(&LaboratoryLocationEnvelopeDto {
        location: location_dto(&location),
        replayed: false,
    }))
}

pub fn update_laboratory_location_json(
    storage_root: &Path,
    input: UpdateLaboratoryLocationInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let definition = LaboratoryLocationDefinition {
        definition_schema_version: LABORATORY_LOCATION_DEFINITION_SCHEMA_VERSION.to_owned(),
        label: input.label.trim().to_owned(),
        description: input.description.trim().to_owned(),
        status: LaboratoryLocationStatus::Active,
    };
    validate_location_definition(&definition)?;
    let mut connection = open_fleet_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let current = required_location(&transaction, &input.location_id)?;
    let payload_json = render_json(&json!({
        "location_id": input.location_id,
        "expected_revision": input.expected_revision,
        "definition": definition
    }));
    let checksum = request_checksum(&payload_json);
    if let Some(replay) = replay_location_operation(
        &transaction,
        &input.context,
        &input.location_id,
        "laboratory_location_updated",
        &checksum,
    )? {
        return Ok(replay);
    }
    if current.status == "archived" {
        return Err(AgentError::new(
            "laboratory_location_archived",
            "Ce lieu est archivé et ne peut plus être modifié.",
        ));
    }
    require_expected_revision(
        &current.location_id,
        current.revision,
        input.expected_revision,
    )?;
    let now = utc_timestamp()?;
    let new_revision = update_laboratory_location(
        &transaction,
        &input.location_id,
        input.expected_revision,
        &definition.label,
        &definition.description,
        &now,
    )?;
    write_fleet_evidence(
        &transaction,
        evidence(
            "laboratory_location",
            &input.location_id,
            "laboratory_location_updated",
            EvidenceRevision::changed(input.expected_revision, new_revision),
            EvidencePayload::new(&checksum, &payload_json),
            &input.context,
            &now,
        ),
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    location_envelope(storage_root, &input.location_id, false)
}

pub fn archive_laboratory_location_json(
    storage_root: &Path,
    input: ArchiveLaboratoryLocationInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let mut connection = open_fleet_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let current = required_location(&transaction, &input.location_id)?;
    let payload_json = render_json(&json!({
        "location_id": input.location_id,
        "expected_revision": input.expected_revision,
        "status": "archived"
    }));
    let checksum = request_checksum(&payload_json);
    if let Some(replay) = replay_location_operation(
        &transaction,
        &input.context,
        &input.location_id,
        "laboratory_location_archived",
        &checksum,
    )? {
        return Ok(replay);
    }
    if current.status == "archived" {
        return Err(AgentError::new(
            "laboratory_location_already_archived",
            "Ce lieu est déjà archivé.",
        ));
    }
    require_expected_revision(
        &current.location_id,
        current.revision,
        input.expected_revision,
    )?;
    let now = utc_timestamp()?;
    let new_revision = archive_laboratory_location(
        &transaction,
        &input.location_id,
        input.expected_revision,
        &now,
    )?;
    write_fleet_evidence(
        &transaction,
        evidence(
            "laboratory_location",
            &input.location_id,
            "laboratory_location_archived",
            EvidenceRevision::changed(input.expected_revision, new_revision),
            EvidencePayload::new(&checksum, &payload_json),
            &input.context,
            &now,
        ),
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    location_envelope(storage_root, &input.location_id, false)
}

pub fn list_physical_asset_audit_json(
    storage_root: &Path,
    asset_id: &str,
) -> Result<String, AgentError> {
    list_audit_json(storage_root, "physical_asset", asset_id)
}

pub fn list_laboratory_location_audit_json(
    storage_root: &Path,
    location_id: &str,
) -> Result<String, AgentError> {
    list_audit_json(storage_root, "laboratory_location", location_id)
}

fn list_audit_json(
    storage_root: &Path,
    entity_kind: &str,
    entity_id: &str,
) -> Result<String, AgentError> {
    let connection = open_fleet_connection(storage_root)?;
    let events = load_fleet_audit_events(&connection, entity_kind, entity_id)?;
    Ok(render_json(&FleetAuditEventListDto {
        entity_id: entity_id.to_owned(),
        audit_events: events
            .into_iter()
            .map(|event| FleetAuditEventDto {
                sequence: event.sequence,
                action: event.action,
                actor: event.actor,
                reason: event.reason,
                old_revision: event.old_revision,
                new_revision: event.new_revision,
                operation_id: event.operation_id,
                device_id: event.device_id,
                correlation_id: event.correlation_id,
                payload: serde_json::from_str(&event.payload_json)
                    .unwrap_or_else(|_| json!({ "raw": event.payload_json })),
                occurred_at: event.occurred_at,
            })
            .collect(),
    }))
}

fn resolve_current_approved_model(
    connection: &rusqlite::Connection,
    equipment_model_id: &str,
) -> Result<PinnedEquipmentModel, AgentError> {
    let identity =
        load_equipment_model_identity(connection, equipment_model_id)?.ok_or_else(|| {
            AgentError::new(
                "equipment_model_not_found",
                "Le modèle constructeur sélectionné n'existe pas.",
            )
        })?;
    let revision_id = identity
        .current_approved_revision_id
        .as_deref()
        .ok_or_else(|| {
            AgentError::new(
                "equipment_model_not_approved",
                "Soumettez puis approuvez une version du modèle avant de créer un exemplaire.",
            )
        })?;
    let revision = load_equipment_model_revision(connection, equipment_model_id, revision_id)?
        .ok_or_else(|| {
            AgentError::new(
                "equipment_model_revision_not_found",
                "La version approuvée du modèle constructeur est introuvable.",
            )
        })?;
    if revision.status != "approved" {
        return Err(AgentError::new(
            "equipment_model_not_approved",
            "La version courante du modèle constructeur n'est pas approuvée.",
        ));
    }
    let definition =
        EquipmentModelDefinition::from_json_str(&revision.definition_json).map_err(|issue| {
            AgentError::with_details(
                "invalid_equipment_model_definition",
                "La version approuvée du modèle constructeur est illisible.",
                json!({ "issue_code": issue.code, "message": issue.message }),
            )
        })?;
    let category_path = if let Some(snapshot) = definition.template_snapshot.as_ref() {
        let mut path = snapshot.category_path.clone();
        if path.first().is_some_and(|label| label == "Général") {
            path.remove(0);
        }
        path
    } else {
        category_path_from_registry(connection, &identity.category_code)?
    };
    Ok(PinnedEquipmentModel {
        equipment_model_id: identity.equipment_model_id,
        equipment_model_revision_id: revision.revision_id,
        equipment_model_checksum: revision.definition_checksum,
        manufacturer: definition.manufacturer,
        model_name: definition.model_name,
        variant: definition.variant,
        category_code: definition.category_code,
        category_path,
    })
}

fn category_path_from_registry(
    connection: &rusqlite::Connection,
    category_id: &str,
) -> Result<Vec<String>, AgentError> {
    let categories = list_equipment_categories(connection, true)?;
    let mut labels = Vec::new();
    let mut current_id = Some(category_id);
    while let Some(id) = current_id {
        let category = categories
            .iter()
            .find(|candidate| candidate.category_id == id)
            .ok_or_else(|| {
                AgentError::new(
                    "equipment_category_not_found",
                    "La catégorie du modèle constructeur est introuvable.",
                )
            })?;
        if category.category_id != "general_equipment" {
            labels.push(category.label.clone());
        }
        current_id = category.parent_category_id.as_deref();
        if labels.len() > categories.len() {
            return Err(AgentError::new(
                "equipment_category_cycle",
                "La hiérarchie des catégories contient un cycle.",
            ));
        }
    }
    labels.reverse();
    Ok(labels)
}

fn resolve_active_location(
    connection: &rusqlite::Connection,
    location_id: Option<&str>,
) -> Result<Option<StoredLaboratoryLocation>, AgentError> {
    let Some(location_id) = location_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let location = required_location(connection, location_id)?;
    if location.status != "active" {
        return Err(AgentError::with_details(
            "laboratory_location_archived",
            "Ce lieu est archivé. Sélectionnez un lieu actif avant d'enregistrer l'exemplaire.",
            json!({ "location_id": location.location_id, "label": location.label }),
        ));
    }
    Ok(Some(location))
}

fn definition_from_stored(
    current: &StoredPhysicalAsset,
    inventory_code: &str,
    serial_number: Option<String>,
    part_number: Option<String>,
    location: Option<&StoredLaboratoryLocation>,
    ownership_source: OwnershipSource,
    notes: &str,
) -> Result<PhysicalAssetDefinition, AgentError> {
    let equipment_model_id = current.equipment_model_id.clone().ok_or_else(|| {
        AgentError::new(
            "physical_asset_model_reconciliation_required",
            "Rattachez cet exemplaire migré à un modèle approuvé avant de modifier sa fiche.",
        )
    })?;
    let equipment_model_revision_id =
        current.equipment_model_revision_id.clone().ok_or_else(|| {
            AgentError::new(
                "physical_asset_model_reconciliation_required",
                "Rattachez cet exemplaire migré à un modèle approuvé avant de modifier sa fiche.",
            )
        })?;
    let equipment_model_checksum = current.equipment_model_checksum.clone().ok_or_else(|| {
        AgentError::new(
            "physical_asset_model_reconciliation_required",
            "Rattachez cet exemplaire migré à un modèle approuvé avant de modifier sa fiche.",
        )
    })?;
    Ok(PhysicalAssetDefinition {
        definition_schema_version: PHYSICAL_ASSET_DEFINITION_SCHEMA_VERSION.to_owned(),
        inventory_code: inventory_code.to_owned(),
        serial_number,
        part_number,
        model: PinnedEquipmentModel {
            equipment_model_id,
            equipment_model_revision_id,
            equipment_model_checksum,
            manufacturer: current.manufacturer_snapshot.clone(),
            model_name: current.model_name_snapshot.clone(),
            variant: current.variant_snapshot.clone(),
            category_code: current.category_code_snapshot.clone(),
            category_path: category_path(current),
        },
        laboratory_location_id: location.map(|item| item.location_id.clone()),
        laboratory_location_label: location.map(|item| item.label.clone()),
        ownership_source,
        service_state: parse_service_state(&current.service_state)?,
        availability_state: parse_availability_state(&current.availability_state)?,
        service_state_reason: current.service_state_reason.clone(),
        notes: notes.to_owned(),
    })
}

fn physical_asset_dto(
    connection: &rusqlite::Connection,
    asset: &StoredPhysicalAsset,
) -> Result<PhysicalAssetDto, AgentError> {
    let current_location_label = match asset.laboratory_location_id.as_deref() {
        Some(location_id) => load_laboratory_location(connection, location_id)?
            .map(|location| location.label)
            .or_else(|| asset.laboratory_location_label_snapshot.clone()),
        None => None,
    };
    Ok(PhysicalAssetDto {
        asset_id: asset.asset_id.clone(),
        inventory_code: asset.inventory_code.clone(),
        serial_number: asset.serial_number.clone(),
        part_number: asset.part_number.clone(),
        equipment_model_id: asset.equipment_model_id.clone(),
        equipment_model_revision_id: asset.equipment_model_revision_id.clone(),
        equipment_model_checksum: asset.equipment_model_checksum.clone(),
        manufacturer: asset.manufacturer_snapshot.clone(),
        model_name: asset.model_name_snapshot.clone(),
        variant: asset.variant_snapshot.clone(),
        category_code: asset.category_code_snapshot.clone(),
        category_path: category_path(asset),
        laboratory_location_id: asset.laboratory_location_id.clone(),
        laboratory_location_label: current_location_label,
        ownership_source: asset.ownership_source.clone(),
        service_state: asset.service_state.clone(),
        availability_state: asset.availability_state.clone(),
        service_state_reason: asset.service_state_reason.clone(),
        notes: asset.notes.clone(),
        revision: asset.revision,
        model_link_state: asset.model_link_state.clone(),
        migrated_from_metrology: asset.migrated_from_metrology,
        metrology: load_metrology_summary(connection, &asset.asset_id)?,
        created_at: asset.created_at.clone(),
        updated_at: asset.updated_at.clone(),
    })
}

fn load_metrology_summary(
    connection: &rusqlite::Connection,
    asset_id: &str,
) -> Result<Option<PhysicalAssetMetrologySummaryDto>, AgentError> {
    connection
        .query_row(
            "SELECT dossier.calibration_requirement, dossier.calibration_period_months,
                dossier.calibration_due_warning_days, latest.due_at, latest.decision
             FROM metrology_db.metrology_asset_dossiers dossier
             LEFT JOIN metrology_db.calibration_events latest
               ON latest.event_id = (
                    SELECT event_id FROM metrology_db.calibration_events
                    WHERE asset_id = dossier.asset_id
                    ORDER BY due_at DESC, calibrated_at DESC, event_id DESC LIMIT 1
               )
             WHERE dossier.asset_id = ?1",
            params![asset_id],
            |row| {
                Ok(PhysicalAssetMetrologySummaryDto {
                    calibration_requirement: row.get(0)?,
                    calibration_period_months: row.get(1)?,
                    calibration_due_warning_days: row.get(2)?,
                    latest_due_at: row.get(3)?,
                    latest_decision: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|error| AgentError::new("metrology_dossier_query_failed", error.to_string()))
}

fn category_path(asset: &StoredPhysicalAsset) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&asset.category_path_json)
        .unwrap_or_else(|_| vec![asset.category_code_snapshot.clone()])
}

fn location_dto(location: &StoredLaboratoryLocation) -> LaboratoryLocationDto {
    LaboratoryLocationDto {
        location_id: location.location_id.clone(),
        label: location.label.clone(),
        description: location.description.clone(),
        status: location.status.clone(),
        revision: location.revision,
        created_at: location.created_at.clone(),
        updated_at: location.updated_at.clone(),
    }
}

fn location_envelope(
    storage_root: &Path,
    location_id: &str,
    replayed: bool,
) -> Result<String, AgentError> {
    let connection = open_fleet_connection(storage_root)?;
    let location = required_location(&connection, location_id)?;
    Ok(render_json(&LaboratoryLocationEnvelopeDto {
        location: location_dto(&location),
        replayed,
    }))
}

fn required_asset(
    connection: &rusqlite::Connection,
    asset_id: &str,
) -> Result<StoredPhysicalAsset, AgentError> {
    load_physical_asset(connection, asset_id)?.ok_or_else(|| {
        AgentError::new(
            "physical_asset_not_found",
            format!("L'exemplaire du parc n'existe pas : {asset_id}"),
        )
    })
}

fn required_location(
    connection: &rusqlite::Connection,
    location_id: &str,
) -> Result<StoredLaboratoryLocation, AgentError> {
    load_laboratory_location(connection, location_id)?.ok_or_else(|| {
        AgentError::new(
            "laboratory_location_not_found",
            format!("Le lieu du laboratoire n'existe pas : {location_id}"),
        )
    })
}

fn replay_asset_operation(
    connection: &rusqlite::Connection,
    context: &FleetOperationContext,
    asset_id: &str,
    action: &str,
    checksum: &str,
) -> Result<Option<String>, AgentError> {
    let Some(operation) =
        existing_fleet_operation(connection, "physical_asset", &context.operation_id)?
    else {
        return Ok(None);
    };
    ensure_replay(
        &operation.entity_id,
        &operation.action,
        &operation.request_checksum,
        asset_id,
        action,
        checksum,
        &context.operation_id,
    )?;
    let asset = required_asset(connection, asset_id)?;
    Ok(Some(render_json(&PhysicalAssetEnvelopeDto {
        asset: physical_asset_dto(connection, &asset)?,
        replayed: true,
    })))
}

fn replay_location_operation(
    connection: &rusqlite::Connection,
    context: &FleetOperationContext,
    location_id: &str,
    action: &str,
    checksum: &str,
) -> Result<Option<String>, AgentError> {
    let Some(operation) =
        existing_fleet_operation(connection, "laboratory_location", &context.operation_id)?
    else {
        return Ok(None);
    };
    ensure_replay(
        &operation.entity_id,
        &operation.action,
        &operation.request_checksum,
        location_id,
        action,
        checksum,
        &context.operation_id,
    )?;
    let location = required_location(connection, &operation.entity_id)?;
    Ok(Some(render_json(&LaboratoryLocationEnvelopeDto {
        location: location_dto(&location),
        replayed: true,
    })))
}

fn ensure_replay(
    actual_entity_id: &str,
    actual_action: &str,
    actual_checksum: &str,
    expected_entity_id: &str,
    expected_action: &str,
    expected_checksum: &str,
    operation_id: &str,
) -> Result<(), AgentError> {
    if actual_entity_id == expected_entity_id
        && actual_action == expected_action
        && actual_checksum == expected_checksum
    {
        return Ok(());
    }
    Err(AgentError::with_details(
        "operation_replay_mismatch",
        "Cet operation_id est déjà associé à une autre opération du parc.",
        json!({
            "operation_id": operation_id,
            "existing_entity_id": actual_entity_id,
            "existing_action": actual_action
        }),
    ))
}

struct EvidenceRevision {
    old: Option<u64>,
    new: u64,
}

impl EvidenceRevision {
    fn created() -> Self {
        Self { old: None, new: 1 }
    }

    fn changed(old: u64, new: u64) -> Self {
        Self {
            old: Some(old),
            new,
        }
    }
}

struct EvidencePayload<'a> {
    checksum: &'a str,
    json: &'a str,
}

impl<'a> EvidencePayload<'a> {
    fn new(checksum: &'a str, json: &'a str) -> Self {
        Self { checksum, json }
    }
}

fn evidence<'a>(
    entity_kind: &'a str,
    entity_id: &'a str,
    action: &'a str,
    revision: EvidenceRevision,
    payload: EvidencePayload<'a>,
    context: &'a FleetOperationContext,
    timestamp: &'a str,
) -> FleetEvidenceInput<'a> {
    FleetEvidenceInput {
        entity_kind,
        entity_id,
        action,
        actor: &context.actor,
        reason: &context.reason,
        operation_id: &context.operation_id,
        device_id: &context.device_id,
        correlation_id: &context.correlation_id,
        old_revision: revision.old,
        new_revision: revision.new,
        request_checksum: payload.checksum,
        payload_json: payload.json,
        timestamp,
    }
}

fn validate_context(context: &FleetOperationContext) -> Result<(), AgentError> {
    for (field, value) in [
        ("actor", context.actor.as_str()),
        ("reason", context.reason.as_str()),
        ("operation_id", context.operation_id.as_str()),
        ("correlation_id", context.correlation_id.as_str()),
        ("device_id", context.device_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(AgentError::new(
                "invalid_fleet_operation_context",
                format!("{field} est obligatoire"),
            ));
        }
    }
    Ok(())
}

fn validate_asset_definition(definition: &PhysicalAssetDefinition) -> Result<(), AgentError> {
    let issues = definition.validate_all();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(AgentError::with_details(
            "invalid_physical_asset",
            "La fiche de l'exemplaire du parc contient des informations invalides.",
            json!({ "issues": issues }),
        ))
    }
}

fn validate_metrology_dossier_input(
    calibration_requirement: &str,
    calibration_period_months: Option<u32>,
    calibration_due_warning_days: u32,
) -> Result<(), AgentError> {
    if !matches!(
        calibration_requirement.trim(),
        "required" | "conditional" | "not_required"
    ) {
        return Err(AgentError::new(
            "invalid_calibration_requirement",
            "Choisissez si l'etalonnage est obligatoire, conditionnel ou non requis.",
        ));
    }
    if calibration_period_months == Some(0) || calibration_due_warning_days == 0 {
        return Err(AgentError::new(
            "invalid_metrology_dossier",
            "La periodicite et le delai d'alerte doivent etre strictement positifs.",
        ));
    }
    Ok(())
}

fn validate_location_definition(
    definition: &LaboratoryLocationDefinition,
) -> Result<(), AgentError> {
    let issues = definition.validate_all();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(AgentError::with_details(
            "invalid_laboratory_location",
            "La fiche du lieu contient des informations invalides.",
            json!({ "issues": issues }),
        ))
    }
}

fn parse_ownership_source(value: &str) -> Result<OwnershipSource, AgentError> {
    match value.trim() {
        "laboratory_owned" => Ok(OwnershipSource::LaboratoryOwned),
        "customer_supplied" => Ok(OwnershipSource::CustomerSupplied),
        "rented" => Ok(OwnershipSource::Rented),
        "borrowed" => Ok(OwnershipSource::Borrowed),
        "external" => Ok(OwnershipSource::External),
        "software_license" => Ok(OwnershipSource::SoftwareLicense),
        "installed_facility" => Ok(OwnershipSource::InstalledFacility),
        _ => Err(AgentError::new(
            "invalid_ownership_source",
            "La propriété ou provenance de l'exemplaire n'est pas reconnue.",
        )),
    }
}

fn parse_service_state(value: &str) -> Result<ServiceState, AgentError> {
    match value.trim() {
        "usable" => Ok(ServiceState::Usable),
        "restricted" => Ok(ServiceState::Restricted),
        "in_maintenance" => Ok(ServiceState::InMaintenance),
        "out_of_service" => Ok(ServiceState::OutOfService),
        "retired" => Ok(ServiceState::Retired),
        _ => Err(AgentError::new(
            "invalid_service_state",
            "L'état de service demandé n'est pas reconnu.",
        )),
    }
}

fn parse_availability_state(value: &str) -> Result<AvailabilityState, AgentError> {
    match value.trim() {
        "available" => Ok(AvailabilityState::Available),
        "reserved" => Ok(AvailabilityState::Reserved),
        "assigned_to_setup" => Ok(AvailabilityState::AssignedToSetup),
        "in_test" => Ok(AvailabilityState::InTest),
        "unavailable" => Ok(AvailabilityState::Unavailable),
        _ => Err(AgentError::new(
            "invalid_availability_state",
            "La disponibilité demandée n'est pas reconnue.",
        )),
    }
}

fn transition_error(error: emc_locus_core::FleetTransitionError) -> AgentError {
    let code = match error.code.as_str() {
        "service_state_unchanged" => "service_state_unchanged",
        "retired_asset_service_state_is_terminal" => "retired_asset_service_state_is_terminal",
        "availability_state_unchanged" => "availability_state_unchanged",
        "unserviceable_asset_cannot_be_available" => "unserviceable_asset_cannot_be_available",
        _ => "invalid_fleet_transition",
    };
    AgentError::with_details(
        code,
        error.message,
        json!({
            "current_state": error.current_state,
            "requested_state": error.requested_state
        }),
    )
}

fn require_expected_revision(
    entity_id: &str,
    current: u64,
    expected: u64,
) -> Result<(), AgentError> {
    if current == expected {
        Ok(())
    } else {
        Err(AgentError::with_details(
            "fleet_revision_conflict",
            "L'objet a été modifié ailleurs. Rechargez-le avant de recommencer.",
            json!({
                "entity_id": entity_id,
                "expected_revision": expected,
                "actual_revision": current
            }),
        ))
    }
}

fn require_resolved_asset(asset: &StoredPhysicalAsset) -> Result<(), AgentError> {
    if asset.model_link_state == "resolved" {
        Ok(())
    } else {
        Err(AgentError::new(
            "physical_asset_model_reconciliation_required",
            "Rattachez cet exemplaire migré à un modèle approuvé avant de poursuivre.",
        ))
    }
}

fn trimmed_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn generated_id(prefix: &str, operation_id: &str, stable_label: &str) -> String {
    let digest = Sha256::digest(format!("{operation_id}\n{stable_label}").as_bytes());
    let hex = format!("{digest:x}");
    format!("{prefix}-{}", hex[..20].to_ascii_uppercase())
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_error", error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{run_storage_action, StorageAction};
    use rusqlite::{params, Connection};
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn generated_ids_are_stable_without_exposing_inventory_codes() {
        let first = generated_id("ASSET", "op-create-1", "INV-0042");
        let second = generated_id("ASSET", "op-create-1", "INV-0042");
        assert_eq!(first, second);
        assert!(!first.contains("INV-0042"));
    }

    #[test]
    fn fleet_migration_creates_authoritative_tables() {
        let storage_root = test_storage_root("fleet-schema");
        run_storage_action(
            StorageAction::Init,
            storage_root.clone(),
            repo_root().join("storage/sqlite"),
        )
        .unwrap();
        let connection = open_fleet_connection(&storage_root).unwrap();
        let count: u64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN (
                    'physical_assets', 'laboratory_locations',
                    'physical_asset_audit_events', 'laboratory_location_audit_events'
                )",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 4);
        let _ = std::fs::remove_dir_all(storage_root);
    }

    #[test]
    fn fleet_workflow_pins_models_and_keeps_evidence_atomic() {
        let storage_root = initialized_storage("fleet-workflow");
        seed_approved_model(&storage_root, 1, "Scope 1", 'a');

        let location_a_id = generated_id("LOC", "op-location-a", "Salle A");
        create_laboratory_location(
            &storage_root,
            CreateLaboratoryLocationInput {
                label: "Salle A".to_owned(),
                description: "Salle principale".to_owned(),
                context: context("op-location-a"),
            },
        )
        .unwrap();

        let asset_one_id = generated_id("ASSET", "op-asset-1", "INV-0001");
        create_physical_asset(
            &storage_root,
            asset_input(
                "INV-0001",
                Some("SN-001"),
                Some(location_a_id.as_str()),
                "op-asset-1",
            ),
        )
        .unwrap();
        let asset_two_id = generated_id("ASSET", "op-asset-2", "INV-0002");
        let second = create_physical_asset(
            &storage_root,
            asset_input("INV-0002", None, Some(location_a_id.as_str()), "op-asset-2"),
        )
        .unwrap();
        assert_eq!(json_value(&second)["asset"]["serial_number"], Value::Null);

        let evidence_before_conflict = fleet_evidence_counts(&storage_root);
        let conflict = create_physical_asset(
            &storage_root,
            asset_input("inv-0001", None, None, "op-duplicate-inventory"),
        )
        .unwrap_err();
        assert_eq!(conflict.code, "physical_asset_inventory_code_conflict");
        assert_eq!(
            fleet_evidence_counts(&storage_root),
            evidence_before_conflict
        );

        seed_approved_model(&storage_root, 2, "Scope 2", 'b');
        let first_after_model_change =
            json_value(&get_physical_asset_json(&storage_root, &asset_one_id).unwrap());
        assert_eq!(
            first_after_model_change["asset"]["equipment_model_revision_id"],
            "EQM-SCOPE-REV-0001"
        );
        assert_eq!(first_after_model_change["asset"]["model_name"], "Scope 1");

        let third = create_physical_asset(
            &storage_root,
            asset_input("INV-0003", None, None, "op-asset-3"),
        )
        .unwrap();
        assert_eq!(
            json_value(&third)["asset"]["equipment_model_revision_id"],
            "EQM-SCOPE-REV-0002"
        );

        update_laboratory_location_json(
            &storage_root,
            UpdateLaboratoryLocationInput {
                location_id: location_a_id.clone(),
                expected_revision: 1,
                label: "Salle CEM A".to_owned(),
                description: "Salle principale renommee".to_owned(),
                context: context("op-location-a-rename"),
            },
        )
        .unwrap();
        let renamed_asset =
            json_value(&get_physical_asset_json(&storage_root, &asset_two_id).unwrap());
        assert_eq!(
            renamed_asset["asset"]["laboratory_location_id"],
            location_a_id
        );
        assert_eq!(
            renamed_asset["asset"]["laboratory_location_label"],
            "Salle CEM A"
        );

        archive_laboratory_location_json(
            &storage_root,
            ArchiveLaboratoryLocationInput {
                location_id: location_a_id.clone(),
                expected_revision: 2,
                context: context("op-location-a-archive"),
            },
        )
        .unwrap();
        let evidence_before_archived_assignment = fleet_evidence_counts(&storage_root);
        let archived_assignment = create_physical_asset(
            &storage_root,
            asset_input(
                "INV-0004",
                None,
                Some(location_a_id.as_str()),
                "op-asset-archived-location",
            ),
        )
        .unwrap_err();
        assert_eq!(archived_assignment.code, "laboratory_location_archived");
        assert_eq!(
            fleet_evidence_counts(&storage_root),
            evidence_before_archived_assignment
        );

        let location_b_id = generated_id("LOC", "op-location-b", "Salle B");
        create_laboratory_location(
            &storage_root,
            CreateLaboratoryLocationInput {
                label: "Salle B".to_owned(),
                description: String::new(),
                context: context("op-location-b"),
            },
        )
        .unwrap();
        update_physical_asset(
            &storage_root,
            UpdatePhysicalAssetInput {
                asset_id: asset_one_id.clone(),
                expected_revision: 1,
                inventory_code: "INV-0001".to_owned(),
                serial_number: Some("SN-001".to_owned()),
                part_number: None,
                laboratory_location_id: Some(location_b_id),
                ownership_source: "laboratory_owned".to_owned(),
                notes: "Deplace en salle B".to_owned(),
                context: context("op-asset-1-move"),
            },
        )
        .unwrap();
        let evidence_before_stale_write = fleet_evidence_counts(&storage_root);
        let stale_write = update_physical_asset(
            &storage_root,
            UpdatePhysicalAssetInput {
                asset_id: asset_one_id.clone(),
                expected_revision: 1,
                inventory_code: "INV-0001".to_owned(),
                serial_number: Some("SN-001".to_owned()),
                part_number: None,
                laboratory_location_id: None,
                ownership_source: "laboratory_owned".to_owned(),
                notes: String::new(),
                context: context("op-asset-1-stale"),
            },
        )
        .unwrap_err();
        assert_eq!(stale_write.code, "fleet_revision_conflict");
        assert_eq!(
            fleet_evidence_counts(&storage_root),
            evidence_before_stale_write
        );

        transition_physical_asset_service_state(
            &storage_root,
            TransitionPhysicalAssetServiceStateInput {
                asset_id: asset_one_id.clone(),
                expected_revision: 2,
                service_state: "out_of_service".to_owned(),
                service_state_reason: "Controle requis".to_owned(),
                context: context("op-asset-1-out-of-service"),
            },
        )
        .unwrap();
        let evidence_before_invalid_transition = fleet_evidence_counts(&storage_root);
        let invalid_transition = transition_physical_asset_availability(
            &storage_root,
            TransitionPhysicalAssetAvailabilityInput {
                asset_id: asset_one_id.clone(),
                expected_revision: 3,
                availability_state: "available".to_owned(),
                context: context("op-asset-1-invalid-availability"),
            },
        )
        .unwrap_err();
        assert_eq!(
            invalid_transition.code,
            "unserviceable_asset_cannot_be_available"
        );
        assert_eq!(
            fleet_evidence_counts(&storage_root),
            evidence_before_invalid_transition
        );

        let restarted = json_value(&list_physical_assets_json(&storage_root).unwrap());
        assert_eq!(restarted["assets"].as_array().unwrap().len(), 3);
        let audit =
            json_value(&list_physical_asset_audit_json(&storage_root, &asset_one_id).unwrap());
        assert_eq!(audit["audit_events"].as_array().unwrap().len(), 3);
        assert!(fleet_evidence_counts(&storage_root).1 >= 9);

        let _ = std::fs::remove_dir_all(storage_root);
    }

    fn initialized_storage(name: &str) -> PathBuf {
        let storage_root = test_storage_root(name);
        run_storage_action(
            StorageAction::Init,
            storage_root.clone(),
            repo_root().join("storage/sqlite"),
        )
        .unwrap();
        storage_root
    }

    fn seed_approved_model(storage_root: &Path, revision: u64, model_name: &str, hash: char) {
        let connection = open_fleet_connection(storage_root).unwrap();
        let revision_id = format!("EQM-SCOPE-REV-{revision:04}");
        let definition = render_json(&json!({
            "definition_schema_version": "emc-locus.equipment-model-definition.v2",
            "manufacturer": "Acme Test",
            "model_name": model_name,
            "equipment_class": "controllable_instrument",
            "functional_role": "measurement_instrument",
            "category_code": "oscilloscope",
            "signal_domains": [],
            "specifications": [],
            "signal_ports": [],
            "communication_interfaces": [],
            "capabilities": [],
            "metadata": {}
        }));
        if revision == 1 {
            connection
                .execute(
                    "INSERT INTO equipment_model_identities (
                        equipment_model_id, manufacturer, model_name, variant, equipment_class,
                        category_code, current_approved_revision_id, created_by, created_at, updated_at
                    ) VALUES ('EQM-SCOPE', 'Acme Test', ?1, NULL, 'controllable_instrument',
                        'oscilloscope', NULL, 'test', '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z')",
                    params![model_name],
                )
                .unwrap();
        } else {
            connection
                .execute(
                    "UPDATE equipment_model_revisions SET status = 'superseded'
                     WHERE equipment_model_id = 'EQM-SCOPE' AND status = 'approved'",
                    [],
                )
                .unwrap();
        }
        let checksum = format!("sha256:{}", hash.to_string().repeat(64));
        connection
            .execute(
                "INSERT INTO equipment_model_revisions (
                    revision_id, equipment_model_id, revision_number, parent_revision_id, status,
                    definition_schema_version, definition_json, definition_checksum, created_by,
                    created_at, updated_at, submitted_at, approved_at
                 ) VALUES (?1, 'EQM-SCOPE', ?2, NULL, 'approved',
                    'emc-locus.equipment-model-definition.v2', ?3, ?4, 'test',
                    '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z',
                    '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z')",
                params![revision_id, revision, definition, checksum],
            )
            .unwrap();
        connection
            .execute(
                "UPDATE equipment_model_identities SET current_approved_revision_id = ?1,
                    model_name = ?2, updated_at = '2026-07-26T00:00:00Z'
                 WHERE equipment_model_id = 'EQM-SCOPE'",
                params![revision_id, model_name],
            )
            .unwrap();
    }

    fn asset_input(
        inventory_code: &str,
        serial_number: Option<&str>,
        location_id: Option<&str>,
        operation_id: &str,
    ) -> CreatePhysicalAssetInput {
        CreatePhysicalAssetInput {
            inventory_code: inventory_code.to_owned(),
            serial_number: serial_number.map(str::to_owned),
            part_number: None,
            equipment_model_id: "EQM-SCOPE".to_owned(),
            laboratory_location_id: location_id.map(str::to_owned),
            ownership_source: "laboratory_owned".to_owned(),
            service_state: "usable".to_owned(),
            availability_state: "available".to_owned(),
            service_state_reason: String::new(),
            notes: String::new(),
            calibration_requirement: "not_required".to_owned(),
            calibration_period_months: None,
            calibration_due_warning_days: 30,
            metrology_notes: String::new(),
            context: context(operation_id),
        }
    }

    fn context(operation_id: &str) -> FleetOperationContext {
        FleetOperationContext {
            actor: "test.operator".to_owned(),
            reason: "Test du parc".to_owned(),
            operation_id: operation_id.to_owned(),
            correlation_id: operation_id.to_owned(),
            device_id: "test-device".to_owned(),
        }
    }

    fn json_value(payload: &str) -> Value {
        serde_json::from_str(payload).unwrap()
    }

    fn fleet_evidence_counts(storage_root: &Path) -> (u64, u64) {
        let equipment = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        let audit_count = equipment
            .query_row(
                "SELECT (SELECT COUNT(*) FROM physical_asset_audit_events)
                    + (SELECT COUNT(*) FROM laboratory_location_audit_events)",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let sync = Connection::open(storage_root.join("sync.sqlite")).unwrap();
        let outbox_count = sync
            .query_row("SELECT COUNT(*) FROM sync_operations", [], |row| row.get(0))
            .unwrap();
        (audit_count, outbox_count)
    }

    fn test_storage_root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "emc-locus-{name}-{}",
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }
}
