use crate::equipment_repository::{
    list_equipment_categories, list_equipment_model_identities, list_equipment_model_revisions,
    load_equipment_model_identity, load_equipment_model_revision, EquipmentModelListFilter,
};
use crate::fleet_dto::{
    AssetSelectionReasonDto, ExecutablePhysicalAssetOptionDto, FleetAuditEventDto,
    FleetAuditEventListDto, LaboratoryLocationDto, LaboratoryLocationEnvelopeDto,
    LaboratoryLocationListDto, ModelReconciliationCandidateDto,
    ModelReconciliationCandidateListDto, PhysicalAssetDto, PhysicalAssetEnvelopeDto,
    PhysicalAssetListDto,
};
use crate::fleet_repository::{
    archive_laboratory_location, existing_fleet_operation, insert_laboratory_location,
    insert_physical_asset, list_laboratory_locations, list_physical_assets,
    load_fleet_audit_events, load_laboratory_location, load_physical_asset,
    load_physical_asset_by_inventory_code, move_physical_asset as persist_physical_asset_move,
    open_fleet_connection, open_fleet_connection_with_sync, reconcile_physical_asset_model,
    request_checksum, update_laboratory_location,
    update_physical_asset_administrative_availability, update_physical_asset_identity,
    update_physical_asset_service_state, write_fleet_evidence, FleetEvidenceInput,
    MovePhysicalAssetRecord, NewPhysicalAssetRecord, PhysicalAssetServiceStateUpdate,
    StoredLaboratoryLocation, StoredPhysicalAsset, UpdatePhysicalAssetIdentityInput,
};
use crate::fleet_usage::compute_operational_usage_for_context;
use crate::metrology_assessment::{
    assess_metrology_source, MetrologyAssessmentSource, MetrologyStatusSummaryDto,
};
use crate::{render_json, AgentError};
use emc_locus_core::metrology::MetrologyDate;
use emc_locus_core::{
    administrative_availability_code, ownership_source_code, service_state_code,
    validate_administrative_availability_transition, validate_service_state_transition,
    AdministrativeAvailability, EquipmentModelDefinition, LaboratoryLocationDefinition,
    LaboratoryLocationStatus, MetrologyAssessmentStatus, OwnershipSource, PhysicalAssetDefinition,
    PinnedEquipmentModel, ServiceState, LABORATORY_LOCATION_DEFINITION_SCHEMA_VERSION,
    PHYSICAL_ASSET_DEFINITION_SCHEMA_VERSION,
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

#[derive(Clone, Debug)]
pub(crate) struct PhysicalAssetSelectionContext {
    pub(crate) assessed_at: OffsetDateTime,
    pub(crate) checked_on: MetrologyDate,
    pub(crate) execution_mode: String,
    pub(crate) laboratory_location_id: Option<String>,
    pub(crate) excluded_schedule_item_code: Option<String>,
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
    pub administrative_availability: String,
    pub administrative_unavailability_reason: String,
    pub service_state_reason: String,
    pub notes: String,
    pub calibration_requirement: String,
    pub calibration_period_months: Option<u32>,
    pub calibration_due_warning_days: u32,
    pub metrology_notes: String,
    pub context: FleetOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdatePhysicalAssetIdentificationInput {
    pub asset_id: String,
    pub expected_revision: u64,
    pub inventory_code: String,
    pub serial_number: Option<String>,
    pub part_number: Option<String>,
    pub ownership_source: String,
    pub notes: String,
    pub context: FleetOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovePhysicalAssetInput {
    pub asset_id: String,
    pub expected_revision: u64,
    pub destination_location_id: Option<String>,
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
pub struct TransitionPhysicalAssetAdministrativeAvailabilityInput {
    pub asset_id: String,
    pub expected_revision: u64,
    pub administrative_availability: String,
    pub administrative_unavailability_reason: String,
    pub context: FleetOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconcilePhysicalAssetModelInput {
    pub asset_id: String,
    pub expected_revision: u64,
    pub equipment_model_id: String,
    pub equipment_model_revision_id: String,
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
    list_physical_assets_json_at(storage_root, None)
}

pub fn list_physical_assets_json_at(
    storage_root: &Path,
    assessed_at: Option<&str>,
) -> Result<String, AgentError> {
    list_physical_assets_json_for_context(storage_root, assessed_at, None)
}

pub fn list_physical_assets_json_for_context(
    storage_root: &Path,
    assessed_at: Option<&str>,
    checked_on: Option<&str>,
) -> Result<String, AgentError> {
    let connection = open_fleet_connection(storage_root)?;
    let assets = list_physical_assets(&connection)?;
    let assessed_at = parse_assessed_at(assessed_at)?;
    let checked_on = match checked_on {
        Some(value) => crate::metrology_assessment::parse_checked_on(value, "checked_on")?,
        None => metrology_date_from_instant(assessed_at),
    };
    let dtos = assets
        .iter()
        .map(|asset| physical_asset_dto_at(&connection, asset, assessed_at, checked_on))
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

pub fn list_model_reconciliation_candidates_json(
    storage_root: &Path,
) -> Result<String, AgentError> {
    let connection = open_fleet_connection(storage_root)?;
    let identities =
        list_equipment_model_identities(&connection, EquipmentModelListFilter::default())?;
    let mut candidates = Vec::new();
    for identity in identities {
        for revision in list_equipment_model_revisions(&connection, &identity.equipment_model_id)? {
            if !matches!(revision.status.as_str(), "approved" | "superseded") {
                continue;
            }
            let model = resolve_immutable_model_revision(
                &connection,
                &identity.equipment_model_id,
                &revision.revision_id,
            )?;
            candidates.push(ModelReconciliationCandidateDto {
                equipment_model_id: model.equipment_model_id,
                equipment_model_revision_id: model.equipment_model_revision_id,
                revision_number: revision.revision_number,
                lifecycle_status: revision.status,
                approved_at: revision.approved_at,
                manufacturer: model.manufacturer,
                model_name: model.model_name,
                variant: model.variant,
                category_path: model.category_path,
            });
        }
    }
    candidates.sort_by(|left, right| {
        left.manufacturer
            .cmp(&right.manufacturer)
            .then(left.model_name.cmp(&right.model_name))
            .then(left.variant.cmp(&right.variant))
            .then(left.revision_number.cmp(&right.revision_number))
    });
    Ok(render_json(&ModelReconciliationCandidateListDto {
        candidates,
    }))
}

pub fn reconcile_physical_asset_model_json(
    storage_root: &Path,
    input: ReconcilePhysicalAssetModelInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let mut connection = open_fleet_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let current = required_asset(&transaction, &input.asset_id)?;
    let request_json = render_json(&json!({
        "asset_id": input.asset_id,
        "expected_revision": input.expected_revision,
        "equipment_model_id": input.equipment_model_id.trim(),
        "equipment_model_revision_id": input.equipment_model_revision_id.trim()
    }));
    let checksum = request_checksum(&request_json);
    if let Some(replay) = replay_asset_operation(
        &transaction,
        &input.context,
        &input.asset_id,
        "physical_asset_model_reconciled",
        &checksum,
    )? {
        return Ok(replay);
    }
    require_expected_revision(&current.asset_id, current.revision, input.expected_revision)?;
    if current.model_link_state != "migration_review_required" {
        return Err(AgentError::new(
            "physical_asset_model_already_resolved",
            "Le modÃ¨le constructeur de cet exemplaire est dÃ©jÃ  rapprochÃ©. Aucune nouvelle affectation silencieuse n'est autorisÃ©e.",
        ));
    }
    let model = resolve_immutable_model_revision(
        &transaction,
        input.equipment_model_id.trim(),
        input.equipment_model_revision_id.trim(),
    )?;
    let category_path_json = render_json(&model.category_path);
    let now = utc_timestamp()?;
    let new_revision = reconcile_physical_asset_model(
        &transaction,
        &input.asset_id,
        input.expected_revision,
        &model,
        &category_path_json,
        &now,
    )?;
    let payload_json = render_json(&json!({
        "asset_id": input.asset_id,
        "expected_revision": input.expected_revision,
        "model": model,
        "migration_evidence_preserved": true
    }));
    write_fleet_evidence(
        &transaction,
        evidence(
            "physical_asset",
            &input.asset_id,
            "physical_asset_model_reconciled",
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

pub fn create_physical_asset(
    storage_root: &Path,
    input: CreatePhysicalAssetInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let ownership_source = parse_ownership_source(&input.ownership_source)?;
    let service_state = parse_service_state(&input.service_state)?;
    let requested_administrative_availability =
        parse_administrative_availability(&input.administrative_availability)?;
    let service_state_reason = input.service_state_reason.trim();
    let service_forces_unavailability = matches!(
        service_state,
        ServiceState::InMaintenance | ServiceState::OutOfService | ServiceState::Retired
    );
    let administrative_availability = if service_forces_unavailability {
        AdministrativeAvailability::Unavailable
    } else {
        requested_administrative_availability
    };
    let administrative_unavailability_reason = if service_forces_unavailability
        && input.administrative_unavailability_reason.trim().is_empty()
    {
        service_state_reason
    } else {
        input.administrative_unavailability_reason.trim()
    };
    validate_administrative_unavailability_reason(
        administrative_availability,
        administrative_unavailability_reason,
    )?;
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
        "administrative_availability": administrative_availability_code(administrative_availability),
        "administrative_unavailability_reason": administrative_unavailability_reason,
        "service_state_reason": service_state_reason,
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
        administrative_availability,
        administrative_unavailability_reason: administrative_unavailability_reason.to_owned(),
        service_state_reason: service_state_reason.to_owned(),
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
            administrative_availability: administrative_availability_code(
                definition.administrative_availability,
            ),
            administrative_unavailability_reason: &definition.administrative_unavailability_reason,
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

pub fn update_physical_asset_identification(
    storage_root: &Path,
    input: UpdatePhysicalAssetIdentificationInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let ownership_source = parse_ownership_source(&input.ownership_source)?;
    let mut connection = open_fleet_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let current = required_asset(&transaction, &input.asset_id)?;
    let inventory_code = input.inventory_code.trim().to_owned();
    let serial_number = trimmed_optional(input.serial_number.as_deref());
    let part_number = trimmed_optional(input.part_number.as_deref());
    validate_asset_identity_fields(
        &inventory_code,
        serial_number.as_deref(),
        part_number.as_deref(),
    )?;
    let payload_json = render_json(&json!({
        "asset_id": input.asset_id,
        "expected_revision": input.expected_revision,
        "identity": {
            "inventory_code": inventory_code,
            "serial_number": serial_number,
            "part_number": part_number,
            "ownership_source": ownership_source_code(ownership_source),
            "notes": input.notes.trim()
        }
    }));
    let checksum = request_checksum(&payload_json);
    if let Some(replay) = replay_asset_operation(
        &transaction,
        &input.context,
        &input.asset_id,
        "physical_asset_identification_updated",
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
            inventory_code: &inventory_code,
            serial_number: serial_number.as_deref(),
            part_number: part_number.as_deref(),
            ownership_source: ownership_source_code(ownership_source),
            notes: input.notes.trim(),
            timestamp: &now,
        },
    )?;
    write_fleet_evidence(
        &transaction,
        evidence(
            "physical_asset",
            &input.asset_id,
            "physical_asset_identification_updated",
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

pub fn move_physical_asset(
    storage_root: &Path,
    input: MovePhysicalAssetInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let mut connection = open_fleet_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let current = required_asset(&transaction, &input.asset_id)?;
    let requested_destination_id = trimmed_optional(input.destination_location_id.as_deref());
    let request_json = render_json(&json!({
        "asset_id": input.asset_id,
        "expected_revision": input.expected_revision,
        "destination_location_id": requested_destination_id
    }));
    let checksum = request_checksum(&request_json);
    if let Some(replay) = replay_asset_operation(
        &transaction,
        &input.context,
        &input.asset_id,
        "physical_asset_moved",
        &checksum,
    )? {
        return Ok(replay);
    }
    require_expected_revision(&current.asset_id, current.revision, input.expected_revision)?;
    let destination = resolve_active_location(&transaction, requested_destination_id.as_deref())?;
    let destination_id = destination.as_ref().map(|item| item.location_id.as_str());
    if current.laboratory_location_id.as_deref() == destination_id {
        return Err(AgentError::new(
            "physical_asset_location_unchanged",
            "L'exemplaire est deja affecte a cet emplacement.",
        ));
    }
    let current_location = match current.laboratory_location_id.as_deref() {
        Some(location_id) => load_laboratory_location(&transaction, location_id)?,
        None => None,
    };
    let payload_json = render_json(&json!({
        "asset_id": input.asset_id,
        "expected_revision": input.expected_revision,
        "from": {
            "laboratory_location_id": current.laboratory_location_id,
            "laboratory_location_label": current_location
                .as_ref()
                .map(|item| item.label.clone())
                .or_else(|| current.laboratory_location_label_snapshot.clone())
        },
        "to": {
            "laboratory_location_id": destination.as_ref().map(|item| item.location_id.clone()),
            "laboratory_location_label": destination.as_ref().map(|item| item.label.clone())
        }
    }));
    let now = utc_timestamp()?;
    let new_revision = persist_physical_asset_move(
        &transaction,
        MovePhysicalAssetRecord {
            asset_id: &input.asset_id,
            expected_revision: input.expected_revision,
            laboratory_location_id: destination_id,
            laboratory_location_label_snapshot: destination
                .as_ref()
                .map(|item| item.label.as_str()),
            timestamp: &now,
        },
    )?;
    write_fleet_evidence(
        &transaction,
        evidence(
            "physical_asset",
            &input.asset_id,
            "physical_asset_moved",
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
    let next_administrative_availability = if matches!(
        requested,
        ServiceState::InMaintenance | ServiceState::OutOfService | ServiceState::Retired
    ) {
        AdministrativeAvailability::Unavailable
    } else {
        parse_administrative_availability(&current.administrative_availability)?
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
        "administrative_availability": administrative_availability_code(next_administrative_availability),
        "administrative_unavailability_reason": if next_administrative_availability == AdministrativeAvailability::Unavailable {
            reason
        } else {
            current.administrative_unavailability_reason.as_str()
        },
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
    let administrative_reason =
        if next_administrative_availability == AdministrativeAvailability::Unavailable {
            if current.administrative_availability == "unavailable"
                && !current
                    .administrative_unavailability_reason
                    .trim()
                    .is_empty()
            {
                current.administrative_unavailability_reason.as_str()
            } else {
                reason
            }
        } else {
            ""
        };
    let now = utc_timestamp()?;
    let new_revision = update_physical_asset_service_state(
        &transaction,
        PhysicalAssetServiceStateUpdate {
            asset_id: &input.asset_id,
            expected_revision: input.expected_revision,
            service_state: service_state_code(requested),
            administrative_availability: administrative_availability_code(
                next_administrative_availability,
            ),
            administrative_unavailability_reason: administrative_reason,
            reason,
            timestamp: &now,
        },
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

pub fn transition_physical_asset_administrative_availability(
    storage_root: &Path,
    input: TransitionPhysicalAssetAdministrativeAvailabilityInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let requested = parse_administrative_availability(&input.administrative_availability)?;
    let reason = input.administrative_unavailability_reason.trim();
    validate_administrative_unavailability_reason(requested, reason)?;
    let mut connection = open_fleet_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let current = required_asset(&transaction, &input.asset_id)?;
    let service_state = parse_service_state(&current.service_state)?;
    let current_availability =
        parse_administrative_availability(&current.administrative_availability)?;
    let payload_json = render_json(&json!({
        "asset_id": input.asset_id,
        "expected_revision": input.expected_revision,
        "to": administrative_availability_code(requested),
        "administrative_unavailability_reason": reason
    }));
    let checksum = request_checksum(&payload_json);
    if let Some(replay) = replay_asset_operation(
        &transaction,
        &input.context,
        &input.asset_id,
        "physical_asset_administrative_availability_changed",
        &checksum,
    )? {
        return Ok(replay);
    }
    require_expected_revision(&current.asset_id, current.revision, input.expected_revision)?;
    validate_administrative_availability_transition(service_state, current_availability, requested)
        .map_err(transition_error)?;
    let now = utc_timestamp()?;
    let new_revision = update_physical_asset_administrative_availability(
        &transaction,
        &input.asset_id,
        input.expected_revision,
        administrative_availability_code(requested),
        reason,
        &now,
    )?;
    write_fleet_evidence(
        &transaction,
        evidence(
            "physical_asset",
            &input.asset_id,
            "physical_asset_administrative_availability_changed",
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
    resolve_immutable_model_revision(connection, equipment_model_id, revision_id)
}

fn resolve_immutable_model_revision(
    connection: &rusqlite::Connection,
    equipment_model_id: &str,
    revision_id: &str,
) -> Result<PinnedEquipmentModel, AgentError> {
    let identity =
        load_equipment_model_identity(connection, equipment_model_id)?.ok_or_else(|| {
            AgentError::new(
                "equipment_model_not_found",
                "Le modèle constructeur sélectionné n'existe pas.",
            )
        })?;
    let revision = load_equipment_model_revision(connection, equipment_model_id, revision_id)?
        .ok_or_else(|| {
            AgentError::new(
                "equipment_model_revision_not_found",
                "La version sélectionnée du modèle constructeur est introuvable.",
            )
        })?;
    if !matches!(revision.status.as_str(), "approved" | "superseded") {
        return Err(AgentError::with_details(
            "equipment_model_revision_not_immutable",
            "Sélectionnez une version approuvée ou remplacée, donc immuable.",
            json!({ "revision_status": revision.status }),
        ));
    }
    let definition =
        EquipmentModelDefinition::from_json_str(&revision.definition_json).map_err(|issue| {
            AgentError::with_details(
                "invalid_equipment_model_definition",
                "La version du modèle constructeur est illisible.",
                json!({ "issue_code": issue.code, "message": issue.message }),
            )
        })?;
    let canonical = definition.canonicalize().map_err(|issues| {
        AgentError::with_details(
            "invalid_equipment_model_definition",
            "La version du modèle constructeur ne respecte pas son contrat métier.",
            json!({ "issues": issues }),
        )
    })?;
    if canonical.definition_schema_version != revision.definition_schema_version
        || canonical.definition_checksum != revision.definition_checksum
    {
        return Err(AgentError::with_details(
            "equipment_model_revision_checksum_mismatch",
            "L'empreinte de la version constructeur ne correspond pas à sa définition canonique.",
            json!({
                "equipment_model_id": equipment_model_id,
                "equipment_model_revision_id": revision_id,
                "stored_checksum": revision.definition_checksum,
                "computed_checksum": canonical.definition_checksum
            }),
        ));
    }
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
        equipment_model_checksum: canonical.definition_checksum,
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

pub(crate) fn executable_physical_asset_options(
    storage_root: &Path,
    context: &PhysicalAssetSelectionContext,
) -> Result<Vec<ExecutablePhysicalAssetOptionDto>, AgentError> {
    let connection = open_fleet_connection(storage_root)?;
    let mut options = list_physical_assets(&connection)?
        .iter()
        .map(|asset| executable_physical_asset_option(&connection, asset, context))
        .collect::<Result<Vec<_>, _>>()?;
    options.sort_by(|left, right| {
        right
            .eligible
            .cmp(&left.eligible)
            .then_with(|| left.asset.category_path.cmp(&right.asset.category_path))
            .then_with(|| left.asset.manufacturer.cmp(&right.asset.manufacturer))
            .then_with(|| left.asset.model_name.cmp(&right.asset.model_name))
            .then_with(|| left.asset.inventory_code.cmp(&right.asset.inventory_code))
    });
    Ok(options)
}

fn executable_physical_asset_option(
    connection: &rusqlite::Connection,
    asset: &StoredPhysicalAsset,
    context: &PhysicalAssetSelectionContext,
) -> Result<ExecutablePhysicalAssetOptionDto, AgentError> {
    let asset_dto = physical_asset_dto_at_for_context(
        connection,
        asset,
        context.assessed_at,
        context.checked_on,
        context.excluded_schedule_item_code.as_deref(),
    )?;
    let mut blocking_reasons = Vec::new();
    let mut warnings = Vec::new();

    if let Some(reason) = model_pin_selection_issue(connection, asset)? {
        blocking_reasons.push(reason);
    }
    match asset.service_state.as_str() {
        "restricted" => warnings.push(selection_reason(
            "service_restricted",
            "Cet exemplaire comporte une restriction d'utilisation.",
            if asset.service_state_reason.trim().is_empty() {
                "Consultez son dossier avant de confirmer son utilisation."
            } else {
                asset.service_state_reason.trim()
            },
        )),
        "in_maintenance" => blocking_reasons.push(selection_reason(
            "service_in_maintenance",
            "Cet exemplaire est en maintenance.",
            "Attendez sa remise en service ou choisissez un autre exemplaire.",
        )),
        "out_of_service" => blocking_reasons.push(selection_reason(
            "service_out_of_service",
            "Cet exemplaire est hors service.",
            "Faites rétablir son état de service ou choisissez un autre exemplaire.",
        )),
        "retired" => blocking_reasons.push(selection_reason(
            "service_retired",
            "Cet exemplaire est retiré du parc.",
            "Choisissez un exemplaire actif du parc.",
        )),
        _ => {}
    }
    if asset.administrative_availability == "unavailable" {
        blocking_reasons.push(selection_reason(
            "administratively_unavailable",
            "Cet exemplaire est déclaré indisponible par le parc matériel.",
            if asset.administrative_unavailability_reason.trim().is_empty() {
                "Rendez-le disponible dans son dossier ou choisissez un autre exemplaire."
            } else {
                asset.administrative_unavailability_reason.trim()
            },
        ));
    }

    for evidence in &asset_dto.operational_usage.evidence {
        match evidence.source_kind.as_str() {
            "active_test" if evidence.blocks_selection => blocking_reasons.push(selection_reason(
                "active_test_conflict",
                "Cet exemplaire est déjà utilisé par un essai en cours.",
                "Attendez la fin de l'essai ou choisissez un autre exemplaire.",
            )),
            "planned_test_reservation" if evidence.blocks_selection => {
                blocking_reasons.push(selection_reason(
                    "planned_test_reservation_conflict",
                    "Cet exemplaire est réservé sur ce créneau.",
                    "Choisissez un autre exemplaire ou modifiez la réservation concernée.",
                ))
            }
            "station_setup_reference" => warnings.push(selection_reason(
                "referenced_by_ready_setup",
                "Cet exemplaire est déjà référencé par un montage prêt.",
                "Cette référence n'est pas exclusive ; vérifiez néanmoins son usage prévu.",
            )),
            kind if kind.ends_with("_source") && !evidence.blocks_selection => {
                warnings.push(selection_reason(
                    format!("{kind}_unavailable"),
                    &evidence.reason,
                    "Actualisez la liste avant l'utilisation effective.",
                ));
            }
            _ => {}
        }
    }

    match asset_dto.laboratory_location_id.as_deref() {
        None => blocking_reasons.push(selection_reason(
            "location_missing",
            "L'emplacement de cet exemplaire n'est pas défini.",
            "Déplacez l'exemplaire vers un lieu actif du laboratoire.",
        )),
        Some(_) if asset_dto.laboratory_location_status.as_deref() != Some("active") => {
            blocking_reasons.push(selection_reason(
                "location_archived",
                "L'emplacement actuel de cet exemplaire est archivé.",
                "Déplacez l'exemplaire vers un lieu actif du laboratoire.",
            ));
        }
        Some(location_id)
            if context
                .laboratory_location_id
                .as_deref()
                .is_some_and(|required| required != location_id) =>
        {
            blocking_reasons.push(selection_reason(
                "location_mismatch",
                "Cet exemplaire se trouve dans un autre lieu.",
                "Déplacez-le vers le lieu prévu ou choisissez un exemplaire déjà présent.",
            ));
        }
        _ => {}
    }

    append_metrology_selection_reasons(
        &asset_dto.metrology,
        &context.execution_mode,
        &mut blocking_reasons,
        &mut warnings,
    );
    blocking_reasons.sort_by(|left, right| left.code.cmp(&right.code));
    blocking_reasons.dedup_by(|left, right| left.code == right.code);
    warnings.sort_by(|left, right| left.code.cmp(&right.code));
    warnings.dedup_by(|left, right| left.code == right.code);

    Ok(ExecutablePhysicalAssetOptionDto {
        eligible: blocking_reasons.is_empty(),
        asset: asset_dto,
        blocking_reasons,
        warnings,
    })
}

fn model_pin_selection_issue(
    connection: &rusqlite::Connection,
    asset: &StoredPhysicalAsset,
) -> Result<Option<AssetSelectionReasonDto>, AgentError> {
    let unresolved = || {
        selection_reason(
            "model_reconciliation_required",
            "Le modèle constructeur doit être rapproché.",
            "Rapprochez cet exemplaire avec la version exacte de son modèle constructeur.",
        )
    };
    if asset.model_link_state != "resolved" {
        return Ok(Some(unresolved()));
    }
    let (Some(model_id), Some(revision_id), Some(checksum)) = (
        asset.equipment_model_id.as_deref(),
        asset.equipment_model_revision_id.as_deref(),
        asset.equipment_model_checksum.as_deref(),
    ) else {
        return Ok(Some(unresolved()));
    };
    let Some(revision) = load_equipment_model_revision(connection, model_id, revision_id)? else {
        return Ok(Some(selection_reason(
            "model_revision_unavailable",
            "La version exacte du modèle constructeur n'est plus disponible.",
            "Restaurez le référentiel local avant d'utiliser cet exemplaire.",
        )));
    };
    let definition_valid = EquipmentModelDefinition::from_json_str(&revision.definition_json)
        .ok()
        .and_then(|definition| definition.canonicalize().ok())
        .is_some_and(|canonical| canonical.definition_checksum == revision.definition_checksum);
    if !matches!(revision.status.as_str(), "approved" | "superseded")
        || revision.definition_checksum != checksum
        || !definition_valid
    {
        return Ok(Some(selection_reason(
            "model_revision_not_trusted",
            "La version du modèle constructeur n'est pas une référence immuable valide.",
            "Faites contrôler le lien au modèle avant d'utiliser cet exemplaire.",
        )));
    }
    Ok(None)
}

fn append_metrology_selection_reasons(
    metrology: &MetrologyStatusSummaryDto,
    execution_mode: &str,
    blocking_reasons: &mut Vec<AssetSelectionReasonDto>,
    warnings: &mut Vec<AssetSelectionReasonDto>,
) {
    let accredited = execution_mode == "accredited";
    let (code, message, next_action) = match metrology.assessment.status {
        MetrologyAssessmentStatus::Valid | MetrologyAssessmentStatus::NotRequired => return,
        MetrologyAssessmentStatus::DueSoon => (
            "calibration_due_soon",
            "L'échéance d'étalonnage de cet exemplaire est proche.",
            "Planifiez son étalonnage avant l'échéance.",
        ),
        MetrologyAssessmentStatus::Expired => (
            "calibration_expired",
            "L'étalonnage requis est expiré à la date prévue.",
            "Enregistrez un étalonnage conforme ou choisissez un autre exemplaire.",
        ),
        MetrologyAssessmentStatus::Missing => (
            "calibration_missing",
            "Aucun étalonnage valide n'est disponible à la date prévue.",
            "Enregistrez la preuve métrologique requise ou choisissez un autre exemplaire.",
        ),
        MetrologyAssessmentStatus::Nonconforming => (
            "calibration_nonconforming",
            "Le dernier étalonnage de cet exemplaire est non conforme.",
            "Traitez la non-conformité avant toute utilisation.",
        ),
        MetrologyAssessmentStatus::Indeterminate => (
            "calibration_indeterminate",
            "La décision métrologique de cet exemplaire est indéterminée.",
            "Faites statuer le métrologue avant l'utilisation.",
        ),
        MetrologyAssessmentStatus::Unavailable => (
            "metrology_unavailable",
            "La métrologie de cet exemplaire est temporairement indisponible.",
            "Rétablissez l'accès au dossier métrologique avant l'utilisation.",
        ),
    };
    let reason = selection_reason(code, message, next_action);
    let always_blocking = matches!(
        metrology.assessment.status,
        MetrologyAssessmentStatus::Nonconforming | MetrologyAssessmentStatus::Unavailable
    );
    if always_blocking
        || (accredited
            && matches!(
                metrology.assessment.status,
                MetrologyAssessmentStatus::Expired
                    | MetrologyAssessmentStatus::Missing
                    | MetrologyAssessmentStatus::Indeterminate
            ))
    {
        blocking_reasons.push(reason);
    } else {
        warnings.push(reason);
    }
}

fn selection_reason(
    code: impl Into<String>,
    message: impl Into<String>,
    next_action: impl Into<String>,
) -> AssetSelectionReasonDto {
    AssetSelectionReasonDto {
        code: code.into(),
        message: message.into(),
        next_action: next_action.into(),
    }
}

fn physical_asset_dto(
    connection: &rusqlite::Connection,
    asset: &StoredPhysicalAsset,
) -> Result<PhysicalAssetDto, AgentError> {
    let assessed_at = OffsetDateTime::now_utc();
    physical_asset_dto_at(
        connection,
        asset,
        assessed_at,
        metrology_date_from_instant(assessed_at),
    )
}

fn physical_asset_dto_at(
    connection: &rusqlite::Connection,
    asset: &StoredPhysicalAsset,
    assessed_at: OffsetDateTime,
    metrology_checked_on: MetrologyDate,
) -> Result<PhysicalAssetDto, AgentError> {
    physical_asset_dto_at_for_context(connection, asset, assessed_at, metrology_checked_on, None)
}

fn physical_asset_dto_at_for_context(
    connection: &rusqlite::Connection,
    asset: &StoredPhysicalAsset,
    assessed_at: OffsetDateTime,
    metrology_checked_on: MetrologyDate,
    excluded_schedule_item_code: Option<&str>,
) -> Result<PhysicalAssetDto, AgentError> {
    let current_location = match asset.laboratory_location_id.as_deref() {
        Some(location_id) => load_laboratory_location(connection, location_id)?,
        None => None,
    };
    let current_location_label = current_location
        .as_ref()
        .map(|location| location.label.clone())
        .or_else(|| asset.laboratory_location_label_snapshot.clone());
    let operational_usage = compute_operational_usage_for_context(
        connection,
        asset,
        assessed_at,
        excluded_schedule_item_code,
    );
    let availability_state = operational_usage.state.clone();
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
        laboratory_location_status: current_location.map(|location| location.status),
        ownership_source: asset.ownership_source.clone(),
        service_state: asset.service_state.clone(),
        administrative_availability: asset.administrative_availability.clone(),
        administrative_unavailability_reason: asset.administrative_unavailability_reason.clone(),
        operational_usage,
        availability_state,
        service_state_reason: asset.service_state_reason.clone(),
        notes: asset.notes.clone(),
        revision: asset.revision,
        model_link_state: asset.model_link_state.clone(),
        migrated_from_metrology: asset.migrated_from_metrology,
        metrology: load_metrology_summary(connection, &asset.asset_id, metrology_checked_on),
        created_at: asset.created_at.clone(),
        updated_at: asset.updated_at.clone(),
    })
}

fn metrology_date_from_instant(value: OffsetDateTime) -> MetrologyDate {
    MetrologyDate::new(value.year() as u16, value.month() as u8, value.day())
        .expect("an OffsetDateTime always contains a valid civil date")
}

fn parse_assessed_at(value: Option<&str>) -> Result<OffsetDateTime, AgentError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(OffsetDateTime::now_utc());
    };
    OffsetDateTime::parse(value, &Rfc3339).map_err(|error| {
        AgentError::with_details(
            "invalid_operational_usage_assessment_time",
            "La date d'évaluation de l'usage doit être un instant RFC 3339.",
            json!({ "assessed_at": value, "error": error.to_string() }),
        )
    })
}

pub(crate) fn load_metrology_summary(
    connection: &rusqlite::Connection,
    asset_id: &str,
    checked_on: MetrologyDate,
) -> MetrologyStatusSummaryDto {
    let source = connection
        .query_row(
            "SELECT dossier.calibration_requirement, dossier.calibration_period_months,
                dossier.calibration_due_warning_days, latest.calibrated_at, latest.due_at,
                latest.decision, latest.event_id, latest.revision
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
                Ok(MetrologyAssessmentSource {
                    calibration_requirement: row.get(0)?,
                    calibration_period_months: row.get(1)?,
                    calibration_due_warning_days: row.get(2)?,
                    calibrated_at: row.get(3)?,
                    due_at: row.get(4)?,
                    decision: row.get(5)?,
                    latest_calibration_event_id: row.get(6)?,
                    latest_calibration_revision: row.get(7)?,
                })
            },
        )
        .optional();
    match source {
        Ok(Some(source)) => assess_metrology_source(checked_on, source),
        Ok(None) | Err(_) => MetrologyStatusSummaryDto::unavailable(checked_on),
    }
}

pub(crate) fn category_path(asset: &StoredPhysicalAsset) -> Vec<String> {
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

fn validate_asset_identity_fields(
    inventory_code: &str,
    serial_number: Option<&str>,
    part_number: Option<&str>,
) -> Result<(), AgentError> {
    let inventory_is_valid = !inventory_code.is_empty()
        && inventory_code.chars().count() <= 80
        && inventory_code.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '/')
        });
    let optional_is_valid = |value: Option<&str>| {
        value.is_none_or(|text| !text.trim().is_empty() && text.trim().chars().count() <= 200)
    };
    if inventory_is_valid && optional_is_valid(serial_number) && optional_is_valid(part_number) {
        return Ok(());
    }
    Err(AgentError::with_details(
        "invalid_physical_asset",
        "L'identification de l'exemplaire contient une valeur invalide.",
        json!({
            "inventory_code_valid": inventory_is_valid,
            "serial_number_valid": optional_is_valid(serial_number),
            "part_number_valid": optional_is_valid(part_number)
        }),
    ))
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

fn parse_administrative_availability(
    value: &str,
) -> Result<AdministrativeAvailability, AgentError> {
    match value.trim() {
        "available" => Ok(AdministrativeAvailability::Available),
        "unavailable" => Ok(AdministrativeAvailability::Unavailable),
        "reserved" | "assigned_to_setup" | "in_test" => Err(AgentError::with_details(
            "operational_usage_cannot_be_set_manually",
            "Une réservation, une affectation à un montage ou un essai en cours doit provenir du workflow correspondant.",
            json!({ "requested_operational_usage": value.trim() }),
        )),
        _ => Err(AgentError::new(
            "invalid_administrative_availability",
            "La disponibilité administrative doit être 'available' ou 'unavailable'.",
        )),
    }
}

fn validate_administrative_unavailability_reason(
    availability: AdministrativeAvailability,
    reason: &str,
) -> Result<(), AgentError> {
    match availability {
        AdministrativeAvailability::Unavailable if reason.trim().is_empty() => {
            Err(AgentError::new(
                "administrative_unavailability_reason_required",
                "Indiquez pourquoi l'exemplaire est administrativement indisponible.",
            ))
        }
        AdministrativeAvailability::Available if !reason.trim().is_empty() => Err(AgentError::new(
            "unexpected_administrative_unavailability_reason",
            "Supprimez le motif d'indisponibilité avant de rendre l'exemplaire disponible.",
        )),
        _ => Ok(()),
    }
}

fn transition_error(error: emc_locus_core::FleetTransitionError) -> AgentError {
    let code = match error.code.as_str() {
        "service_state_unchanged" => "service_state_unchanged",
        "retired_asset_service_state_is_terminal" => "retired_asset_service_state_is_terminal",
        "administrative_availability_unchanged" => "administrative_availability_unchanged",
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
    fn metrology_storage_failure_preserves_physical_asset_identity() {
        let storage_root = initialized_storage("fleet-metrology-unavailable");
        seed_approved_model(&storage_root, 1, "Scope 1", 'a');
        let asset_id = generated_id("ASSET", "op-unavailable", "INV-UNAVAILABLE");
        create_physical_asset(
            &storage_root,
            asset_input("INV-UNAVAILABLE", None, None, "op-unavailable"),
        )
        .unwrap();

        let metrology_path = storage_root.join("metrology.sqlite");
        let unavailable_path = storage_root.join("metrology.unavailable");
        std::fs::rename(&metrology_path, &unavailable_path).unwrap();

        let response = json_value(&get_physical_asset_json(&storage_root, &asset_id).unwrap());
        assert_eq!(response["asset"]["inventory_code"], "INV-UNAVAILABLE");
        assert_eq!(response["asset"]["metrology"]["status"], "unavailable");
        assert_eq!(response["asset"]["metrology"]["blocking"], true);

        let _ = std::fs::remove_dir_all(storage_root);
    }

    #[test]
    fn fleet_metrology_is_assessed_on_an_explicit_civil_date() {
        let storage_root = initialized_storage("fleet-metrology-dated");
        seed_approved_model(&storage_root, 1, "Scope 1", 'a');
        let asset_id = generated_id("ASSET", "op-dated", "INV-DATED");
        let mut input = asset_input("INV-DATED", None, None, "op-dated");
        input.calibration_requirement = "required".to_owned();
        input.calibration_period_months = Some(12);
        create_physical_asset(&storage_root, input).unwrap();
        crate::metrology_service::record_metrology_calibration(
            &storage_root,
            crate::metrology_service::RecordCalibrationInput {
                event_id: "CAL-DATED-1".to_owned(),
                asset_id,
                certificate_reference: "CERT-DATED-1".to_owned(),
                calibrated_at: "2025-07-27".to_owned(),
                due_at: "2026-07-27".to_owned(),
                provider: "Laboratoire accrédité".to_owned(),
                decision: "conforming".to_owned(),
                as_found_status: Some("conforming".to_owned()),
                as_left_status: Some("conforming".to_owned()),
                adjustment_performed: false,
                uncertainty_summary_json: "{}".to_owned(),
                traceability_reference: Some("TRACE-DATED-1".to_owned()),
                comment: "Étalonnage de référence".to_owned(),
                document_manifest_json: None,
                recorded_by: "metrologist".to_owned(),
                context: crate::metrology_service::MetrologyOperationContext {
                    actor: "metrologist".to_owned(),
                    reason: "Test du statut daté".to_owned(),
                    operation_id: "op-cal-dated".to_owned(),
                    correlation_id: "corr-cal-dated".to_owned(),
                    device_id: "test-device".to_owned(),
                },
            },
        )
        .unwrap();

        for (checked_on, expected) in [
            ("2026-06-01", "valid"),
            ("2026-07-27", "due_soon"),
            ("2026-07-28", "expired"),
        ] {
            let response = json_value(
                &list_physical_assets_json_for_context(
                    &storage_root,
                    Some("2026-07-27T23:30:00-11:00"),
                    Some(checked_on),
                )
                .unwrap(),
            );
            assert_eq!(response["assets"][0]["metrology"]["status"], expected);
            assert_eq!(response["assets"][0]["metrology"]["checked_on"], checked_on);
        }

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
        move_physical_asset(
            &storage_root,
            MovePhysicalAssetInput {
                asset_id: asset_one_id.clone(),
                expected_revision: 1,
                destination_location_id: Some(location_b_id),
                context: context("op-asset-1-move"),
            },
        )
        .unwrap();
        let evidence_before_stale_write = fleet_evidence_counts(&storage_root);
        let stale_write = update_physical_asset_identification(
            &storage_root,
            UpdatePhysicalAssetIdentificationInput {
                asset_id: asset_one_id.clone(),
                expected_revision: 1,
                inventory_code: "INV-0001".to_owned(),
                serial_number: Some("SN-001".to_owned()),
                part_number: None,
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
        let invalid_transition = transition_physical_asset_administrative_availability(
            &storage_root,
            TransitionPhysicalAssetAdministrativeAvailabilityInput {
                asset_id: asset_one_id.clone(),
                expected_revision: 3,
                administrative_availability: "available".to_owned(),
                administrative_unavailability_reason: String::new(),
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

    #[test]
    fn migrated_asset_reconciliation_is_exact_idempotent_and_atomic() {
        let storage_root = initialized_storage("fleet-model-reconciliation");
        seed_approved_model(&storage_root, 1, "Scope historique", 'a');
        seed_approved_model(&storage_root, 2, "Scope courant", 'b');
        seed_unresolved_migrated_asset(&storage_root, "LEGACY-SCOPE-001");

        let candidates =
            json_value(&list_model_reconciliation_candidates_json(&storage_root).unwrap());
        let candidates = candidates["candidates"].as_array().unwrap();
        assert_eq!(candidates.len(), 2);
        let historical = candidates
            .iter()
            .find(|candidate| candidate["revision_number"] == 1)
            .unwrap();
        let current = candidates
            .iter()
            .find(|candidate| candidate["revision_number"] == 2)
            .unwrap();
        assert_eq!(historical["lifecycle_status"], "superseded");
        assert_eq!(current["lifecycle_status"], "approved");

        let evidence_before_rejections = fleet_evidence_counts(&storage_root);
        let stale = reconcile_physical_asset_model_json(
            &storage_root,
            reconciliation_input("LEGACY-SCOPE-001", 2, 1, "op-reconcile-stale"),
        )
        .unwrap_err();
        assert_eq!(stale.code, "fleet_revision_conflict");

        seed_revision_copy(&storage_root, 3, "draft", None);
        let draft = reconcile_physical_asset_model_json(
            &storage_root,
            reconciliation_input("LEGACY-SCOPE-001", 1, 3, "op-reconcile-draft"),
        )
        .unwrap_err();
        assert_eq!(draft.code, "equipment_model_revision_not_immutable");
        assert_eq!(
            fleet_evidence_counts(&storage_root),
            evidence_before_rejections
        );

        let migration_evidence_before = migration_evidence(&storage_root, "LEGACY-SCOPE-001");
        let first_input =
            reconciliation_input("LEGACY-SCOPE-001", 1, 1, "op-reconcile-legacy-scope");
        let first = json_value(
            &reconcile_physical_asset_model_json(&storage_root, first_input.clone()).unwrap(),
        );
        assert_eq!(first["asset"]["model_link_state"], "resolved");
        assert_eq!(first["asset"]["revision"], 2);
        assert_eq!(first["asset"]["model_name"], "Scope historique");
        assert_eq!(
            first["asset"]["equipment_model_revision_id"],
            "EQM-SCOPE-REV-0001"
        );
        assert_eq!(
            first["asset"]["equipment_model_checksum"],
            model_revision_checksum(&storage_root, 1)
        );
        assert_eq!(
            migration_evidence(&storage_root, "LEGACY-SCOPE-001"),
            migration_evidence_before
        );

        let replay =
            json_value(&reconcile_physical_asset_model_json(&storage_root, first_input).unwrap());
        assert_eq!(replay["replayed"], true);
        assert_eq!(replay["asset"]["revision"], 2);
        let evidence_after_success = fleet_evidence_counts(&storage_root);

        let mismatch = reconcile_physical_asset_model_json(
            &storage_root,
            reconciliation_input("LEGACY-SCOPE-001", 1, 2, "op-reconcile-legacy-scope"),
        )
        .unwrap_err();
        assert_eq!(mismatch.code, "operation_replay_mismatch");
        let repoint = reconcile_physical_asset_model_json(
            &storage_root,
            reconciliation_input("LEGACY-SCOPE-001", 2, 2, "op-reconcile-repoint"),
        )
        .unwrap_err();
        assert_eq!(repoint.code, "physical_asset_model_already_resolved");
        assert_eq!(fleet_evidence_counts(&storage_root), evidence_after_success);

        seed_unresolved_migrated_asset(&storage_root, "LEGACY-SCOPE-CORRUPT");
        seed_revision_copy(
            &storage_root,
            4,
            "approved",
            Some("sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
        );
        let before_corrupt = fleet_evidence_counts(&storage_root);
        let corrupt = reconcile_physical_asset_model_json(
            &storage_root,
            reconciliation_input("LEGACY-SCOPE-CORRUPT", 1, 4, "op-reconcile-corrupt"),
        )
        .unwrap_err();
        assert_eq!(corrupt.code, "equipment_model_revision_checksum_mismatch");
        assert_eq!(fleet_evidence_counts(&storage_root), before_corrupt);

        seed_unresolved_migrated_asset(&storage_root, "LEGACY-SCOPE-ATOMIC");
        seed_conflicting_outbox_operation(&storage_root, "op-reconcile-atomic");
        let before_atomic = fleet_evidence_counts(&storage_root);
        let atomic = reconcile_physical_asset_model_json(
            &storage_root,
            reconciliation_input("LEGACY-SCOPE-ATOMIC", 1, 1, "op-reconcile-atomic"),
        )
        .unwrap_err();
        assert_eq!(atomic.code, "fleet_outbox_write_failed");
        assert_eq!(fleet_evidence_counts(&storage_root), before_atomic);
        let unchanged =
            json_value(&get_physical_asset_json(&storage_root, "LEGACY-SCOPE-ATOMIC").unwrap());
        assert_eq!(
            unchanged["asset"]["model_link_state"],
            "migration_review_required"
        );
        assert_eq!(unchanged["asset"]["revision"], 1);

        let restarted =
            json_value(&get_physical_asset_json(&storage_root, "LEGACY-SCOPE-001").unwrap());
        assert_eq!(restarted["asset"]["model_link_state"], "resolved");
        assert_eq!(restarted["asset"]["model_name"], "Scope historique");
        let audit =
            json_value(&list_physical_asset_audit_json(&storage_root, "LEGACY-SCOPE-001").unwrap());
        assert_eq!(audit["audit_events"].as_array().unwrap().len(), 1);
        assert_eq!(
            audit["audit_events"][0]["action"],
            "physical_asset_model_reconciled"
        );

        let _ = std::fs::remove_dir_all(storage_root);
    }

    #[test]
    fn asset_creation_canonicalizes_unusable_states_and_keeps_optional_identity_fields() {
        let storage_root = initialized_storage("fleet-create-combinations");
        seed_approved_model(&storage_root, 1, "Scope creation", 'a');

        let without_location = json_value(
            &create_physical_asset(
                &storage_root,
                asset_input("INV-NO-LOCATION", None, None, "op-no-location"),
            )
            .unwrap(),
        );
        assert_eq!(without_location["asset"]["serial_number"], Value::Null);
        assert_eq!(
            without_location["asset"]["laboratory_location_id"],
            Value::Null
        );

        let evidence_before_invalid = fleet_evidence_counts(&storage_root);
        let mut restricted_without_reason = asset_input(
            "INV-RESTRICTED-INVALID",
            None,
            None,
            "op-restricted-invalid",
        );
        restricted_without_reason.service_state = "restricted".to_owned();
        let invalid = create_physical_asset(&storage_root, restricted_without_reason).unwrap_err();
        assert_eq!(invalid.code, "invalid_physical_asset");
        assert_eq!(
            fleet_evidence_counts(&storage_root),
            evidence_before_invalid
        );

        let mut restricted = asset_input("INV-RESTRICTED", None, None, "op-restricted-valid");
        restricted.service_state = "restricted".to_owned();
        restricted.service_state_reason = "Utilisation sous surveillance".to_owned();
        let restricted = json_value(&create_physical_asset(&storage_root, restricted).unwrap());
        assert_eq!(restricted["asset"]["service_state"], "restricted");
        assert_eq!(
            restricted["asset"]["administrative_availability"],
            "available"
        );

        for (index, service_state) in ["in_maintenance", "out_of_service", "retired"]
            .into_iter()
            .enumerate()
        {
            let inventory_code = format!("INV-FORCED-{index}");
            let operation_id = format!("op-forced-{index}");
            let mut input = asset_input(&inventory_code, None, None, &operation_id);
            input.ownership_source = if index == 0 {
                "software_license".to_owned()
            } else {
                "laboratory_owned".to_owned()
            };
            input.service_state = service_state.to_owned();
            input.service_state_reason = "État technique déclaré à la création".to_owned();
            input.administrative_availability = "available".to_owned();
            let created = json_value(&create_physical_asset(&storage_root, input).unwrap());
            assert_eq!(created["asset"]["service_state"], service_state);
            assert_eq!(
                created["asset"]["administrative_availability"],
                "unavailable"
            );
            assert_eq!(
                created["asset"]["administrative_unavailability_reason"],
                "État technique déclaré à la création"
            );
            assert_eq!(created["asset"]["laboratory_location_id"], Value::Null);
        }

        let _ = std::fs::remove_dir_all(storage_root);
    }

    #[test]
    fn archived_location_does_not_block_identity_edit_and_moves_are_explicit() {
        let storage_root = initialized_storage("fleet-identity-move-boundary");
        seed_approved_model(&storage_root, 1, "Scope mouvement", 'a');
        let location_a_id = generated_id("LOC", "op-move-location-a", "Salle historique");
        let location_b_id = generated_id("LOC", "op-move-location-b", "Salle active");
        for (location_id, label, operation_id) in [
            (&location_a_id, "Salle historique", "op-move-location-a"),
            (&location_b_id, "Salle active", "op-move-location-b"),
        ] {
            let created = json_value(
                &create_laboratory_location(
                    &storage_root,
                    CreateLaboratoryLocationInput {
                        label: label.to_owned(),
                        description: String::new(),
                        context: context(operation_id),
                    },
                )
                .unwrap(),
            );
            assert_eq!(created["location"]["location_id"], location_id.as_str());
        }
        let asset_id = generated_id("ASSET", "op-move-asset", "INV-MOVE-001");
        create_physical_asset(
            &storage_root,
            asset_input(
                "INV-MOVE-001",
                Some("SN-MOVE"),
                Some(&location_a_id),
                "op-move-asset",
            ),
        )
        .unwrap();
        archive_laboratory_location_json(
            &storage_root,
            ArchiveLaboratoryLocationInput {
                location_id: location_a_id.clone(),
                expected_revision: 1,
                context: context("op-archive-current-location"),
            },
        )
        .unwrap();

        let archived_current =
            json_value(&get_physical_asset_json(&storage_root, &asset_id).unwrap());
        assert_eq!(
            archived_current["asset"]["laboratory_location_status"],
            "archived"
        );
        assert_eq!(
            archived_current["asset"]["laboratory_location_label"],
            "Salle historique"
        );

        let edited = json_value(
            &update_physical_asset_identification(
                &storage_root,
                UpdatePhysicalAssetIdentificationInput {
                    asset_id: asset_id.clone(),
                    expected_revision: 1,
                    inventory_code: "INV-MOVE-RENAMED".to_owned(),
                    serial_number: Some("SN-MOVE".to_owned()),
                    part_number: Some("PN-MOVE".to_owned()),
                    ownership_source: "laboratory_owned".to_owned(),
                    notes: "Identification modifiée au lieu archivé".to_owned(),
                    context: context("op-edit-at-archived-location"),
                },
            )
            .unwrap(),
        );
        assert_eq!(edited["asset"]["revision"], 2);
        assert_eq!(edited["asset"]["laboratory_location_status"], "archived");

        let evidence_before_archived_move = fleet_evidence_counts(&storage_root);
        let archived_destination = move_physical_asset(
            &storage_root,
            MovePhysicalAssetInput {
                asset_id: asset_id.clone(),
                expected_revision: 2,
                destination_location_id: Some(location_a_id.clone()),
                context: context("op-move-to-archived"),
            },
        )
        .unwrap_err();
        assert_eq!(archived_destination.code, "laboratory_location_archived");
        assert_eq!(
            fleet_evidence_counts(&storage_root),
            evidence_before_archived_move
        );

        let move_input = MovePhysicalAssetInput {
            asset_id: asset_id.clone(),
            expected_revision: 2,
            destination_location_id: Some(location_b_id.clone()),
            context: context("op-move-to-active"),
        };
        let moved = json_value(&move_physical_asset(&storage_root, move_input.clone()).unwrap());
        assert_eq!(moved["asset"]["laboratory_location_id"], location_b_id);
        assert_eq!(moved["asset"]["laboratory_location_status"], "active");
        let replay = json_value(&move_physical_asset(&storage_root, move_input.clone()).unwrap());
        assert_eq!(replay["replayed"], true);

        update_laboratory_location_json(
            &storage_root,
            UpdateLaboratoryLocationInput {
                location_id: location_b_id.clone(),
                expected_revision: 1,
                label: "Salle active renommée".to_owned(),
                description: String::new(),
                context: context("op-rename-active-location"),
            },
        )
        .unwrap();
        let renamed = json_value(&get_physical_asset_json(&storage_root, &asset_id).unwrap());
        assert_eq!(renamed["asset"]["laboratory_location_id"], location_b_id);
        assert_eq!(
            renamed["asset"]["laboratory_location_label"],
            "Salle active renommée"
        );

        let audit = json_value(&list_physical_asset_audit_json(&storage_root, &asset_id).unwrap());
        let move_event = audit["audit_events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["action"] == "physical_asset_moved")
            .unwrap();
        assert_eq!(
            move_event["payload"]["to"]["laboratory_location_label"],
            "Salle active"
        );
        assert_eq!(
            move_event["payload"]["from"]["laboratory_location_label"],
            "Salle historique"
        );

        archive_laboratory_location_json(
            &storage_root,
            ArchiveLaboratoryLocationInput {
                location_id: location_b_id.clone(),
                expected_revision: 2,
                context: context("op-archive-move-destination"),
            },
        )
        .unwrap();
        let replay_after_archive =
            json_value(&move_physical_asset(&storage_root, move_input).unwrap());
        assert_eq!(replay_after_archive["replayed"], true);

        let removed = json_value(
            &move_physical_asset(
                &storage_root,
                MovePhysicalAssetInput {
                    asset_id: asset_id.clone(),
                    expected_revision: 3,
                    destination_location_id: None,
                    context: context("op-remove-location"),
                },
            )
            .unwrap(),
        );
        assert_eq!(removed["asset"]["laboratory_location_id"], Value::Null);
        let evidence_before_stale_move = fleet_evidence_counts(&storage_root);
        let stale = move_physical_asset(
            &storage_root,
            MovePhysicalAssetInput {
                asset_id: asset_id.clone(),
                expected_revision: 3,
                destination_location_id: Some(location_b_id),
                context: context("op-stale-concurrent-move"),
            },
        )
        .unwrap_err();
        assert_eq!(stale.code, "fleet_revision_conflict");
        assert_eq!(
            fleet_evidence_counts(&storage_root),
            evidence_before_stale_move
        );
        let restarted = json_value(&get_physical_asset_json(&storage_root, &asset_id).unwrap());
        assert_eq!(restarted["asset"]["inventory_code"], "INV-MOVE-RENAMED");
        assert_eq!(restarted["asset"]["laboratory_location_id"], Value::Null);

        let _ = std::fs::remove_dir_all(storage_root);
    }

    #[test]
    fn unresolved_migrated_asset_keeps_safe_administrative_edits() {
        let storage_root = initialized_storage("fleet-unresolved-safe-edits");
        seed_approved_model(&storage_root, 1, "Scope fiable", 'a');
        seed_unresolved_migrated_asset(&storage_root, "LEGACY-EDIT-001");
        let location_id = generated_id("LOC", "op-safe-location", "Zone attente");
        create_laboratory_location(
            &storage_root,
            CreateLaboratoryLocationInput {
                label: "Zone attente".to_owned(),
                description: "Matériels à identifier".to_owned(),
                context: context("op-safe-location"),
            },
        )
        .unwrap();

        let updated = json_value(
            &update_physical_asset_identification(
                &storage_root,
                UpdatePhysicalAssetIdentificationInput {
                    asset_id: "LEGACY-EDIT-001".to_owned(),
                    expected_revision: 1,
                    inventory_code: "LEGACY-EDIT-RENAMED".to_owned(),
                    serial_number: Some("SN-RETROUVE".to_owned()),
                    part_number: None,
                    ownership_source: "laboratory_owned".to_owned(),
                    notes: "Identification complétée avant rapprochement".to_owned(),
                    context: context("op-safe-identity"),
                },
            )
            .unwrap(),
        );
        assert_eq!(
            updated["asset"]["model_link_state"],
            "migration_review_required"
        );
        assert_eq!(updated["asset"]["equipment_model_id"], Value::Null);
        assert_eq!(updated["asset"]["laboratory_location_id"], Value::Null);
        assert_eq!(updated["asset"]["revision"], 2);

        let moved = json_value(
            &move_physical_asset(
                &storage_root,
                MovePhysicalAssetInput {
                    asset_id: "LEGACY-EDIT-001".to_owned(),
                    expected_revision: 2,
                    destination_location_id: Some(location_id.clone()),
                    context: context("op-safe-move"),
                },
            )
            .unwrap(),
        );
        assert_eq!(moved["asset"]["laboratory_location_id"], location_id);
        assert_eq!(moved["asset"]["revision"], 3);

        let restricted = json_value(
            &transition_physical_asset_service_state(
                &storage_root,
                TransitionPhysicalAssetServiceStateInput {
                    asset_id: "LEGACY-EDIT-001".to_owned(),
                    expected_revision: 3,
                    service_state: "restricted".to_owned(),
                    service_state_reason: "Utilisation sous surveillance".to_owned(),
                    context: context("op-safe-service-state"),
                },
            )
            .unwrap(),
        );
        assert_eq!(restricted["asset"]["service_state"], "restricted");
        assert_eq!(
            restricted["asset"]["model_link_state"],
            "migration_review_required"
        );

        let reconciled = json_value(
            &reconcile_physical_asset_model_json(
                &storage_root,
                reconciliation_input("LEGACY-EDIT-001", 4, 1, "op-safe-reconciliation"),
            )
            .unwrap(),
        );
        assert_eq!(reconciled["asset"]["model_link_state"], "resolved");
        assert_eq!(reconciled["asset"]["inventory_code"], "LEGACY-EDIT-RENAMED");
        assert_eq!(reconciled["asset"]["laboratory_location_id"], location_id);

        let _ = std::fs::remove_dir_all(storage_root);
    }

    #[test]
    fn administrative_availability_and_operational_usage_remain_separate() {
        let storage_root = initialized_storage("fleet-derived-usage");
        seed_approved_model(&storage_root, 1, "Scope usage", 'a');
        let asset_id = generated_id("ASSET", "op-usage-asset", "INV-USAGE-001");
        create_physical_asset(
            &storage_root,
            asset_input("INV-USAGE-001", None, None, "op-usage-asset"),
        )
        .unwrap();

        let initial = asset_at(&storage_root, &asset_id, "2026-07-27T08:00:00Z");
        assert_eq!(initial["administrative_availability"], "available");
        assert_eq!(initial["operational_usage"]["state"], "available");
        assert!(initial["operational_usage"]["evidence"]
            .as_array()
            .unwrap()
            .is_empty());

        seed_station_setup_reference(&storage_root, &asset_id);
        let assigned = asset_at(&storage_root, &asset_id, "2026-07-27T08:00:00Z");
        assert_eq!(assigned["operational_usage"]["state"], "assigned_to_setup");
        assert_eq!(
            assigned["operational_usage"]["evidence"][0]["blocks_selection"],
            false
        );

        seed_planned_test_reservation(&storage_root, &asset_id);
        let reserved = asset_at(&storage_root, &asset_id, "2026-07-27T10:00:00Z");
        assert_eq!(reserved["operational_usage"]["state"], "reserved");
        assert!(reserved["operational_usage"]["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["source_kind"] == "planned_test_reservation"
                && item["source_identifier"] == "SCHED-USAGE-001"));

        seed_active_measurement_run(&storage_root, &asset_id);
        let in_test = asset_at(&storage_root, &asset_id, "2026-07-27T10:00:00Z");
        assert_eq!(in_test["operational_usage"]["state"], "in_test");
        assert!(in_test["operational_usage"]["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["source_kind"] == "active_test"));

        let evidence_before_rejections = fleet_evidence_counts(&storage_root);
        for requested in ["reserved", "assigned_to_setup", "in_test"] {
            let invented = transition_physical_asset_administrative_availability(
                &storage_root,
                TransitionPhysicalAssetAdministrativeAvailabilityInput {
                    asset_id: asset_id.clone(),
                    expected_revision: 1,
                    administrative_availability: requested.to_owned(),
                    administrative_unavailability_reason: String::new(),
                    context: context(&format!("op-invented-{requested}")),
                },
            )
            .unwrap_err();
            assert_eq!(invented.code, "operational_usage_cannot_be_set_manually");
        }
        let missing_reason = transition_physical_asset_administrative_availability(
            &storage_root,
            TransitionPhysicalAssetAdministrativeAvailabilityInput {
                asset_id: asset_id.clone(),
                expected_revision: 1,
                administrative_availability: "unavailable".to_owned(),
                administrative_unavailability_reason: String::new(),
                context: context("op-unavailable-without-reason"),
            },
        )
        .unwrap_err();
        assert_eq!(
            missing_reason.code,
            "administrative_unavailability_reason_required"
        );
        assert_eq!(
            fleet_evidence_counts(&storage_root),
            evidence_before_rejections
        );

        transition_physical_asset_administrative_availability(
            &storage_root,
            TransitionPhysicalAssetAdministrativeAvailabilityInput {
                asset_id: asset_id.clone(),
                expected_revision: 1,
                administrative_availability: "unavailable".to_owned(),
                administrative_unavailability_reason: "Prêt à un autre laboratoire".to_owned(),
                context: context("op-administrative-unavailable"),
            },
        )
        .unwrap();
        let unavailable = asset_at(&storage_root, &asset_id, "2026-07-27T10:00:00Z");
        assert_eq!(unavailable["administrative_availability"], "unavailable");
        assert_eq!(unavailable["operational_usage"]["state"], "unavailable");
        assert_eq!(
            unavailable["operational_usage"]["evidence"][0]["reason"],
            "Prêt à un autre laboratoire"
        );

        transition_physical_asset_administrative_availability(
            &storage_root,
            TransitionPhysicalAssetAdministrativeAvailabilityInput {
                asset_id: asset_id.clone(),
                expected_revision: 2,
                administrative_availability: "available".to_owned(),
                administrative_unavailability_reason: String::new(),
                context: context("op-administrative-available"),
            },
        )
        .unwrap();
        complete_measurement_run(&storage_root);
        let after_interval = asset_at(&storage_root, &asset_id, "2026-07-27T12:00:00Z");
        assert_eq!(after_interval["administrative_availability"], "available");
        assert_eq!(
            after_interval["operational_usage"]["state"],
            "assigned_to_setup"
        );
        assert_eq!(after_interval["revision"], 3);

        let _ = std::fs::remove_dir_all(storage_root);
    }

    #[test]
    fn executable_options_explain_contextual_eligibility_and_real_usage() {
        let storage_root = initialized_storage("fleet-executable-options");
        seed_approved_model(&storage_root, 1, "Scope sélection", 'a');
        let location_a = generated_id("LOC", "op-options-location-a", "Salle A");
        let location_b = generated_id("LOC", "op-options-location-b", "Salle B");
        for (label, operation_id) in [
            ("Salle A", "op-options-location-a"),
            ("Salle B", "op-options-location-b"),
        ] {
            create_laboratory_location(
                &storage_root,
                CreateLaboratoryLocationInput {
                    label: label.to_owned(),
                    description: String::new(),
                    context: context(operation_id),
                },
            )
            .unwrap();
        }

        let eligible_id = generated_id("ASSET", "op-options-eligible", "INV-OPTIONS-OK");
        create_physical_asset(
            &storage_root,
            asset_input(
                "INV-OPTIONS-OK",
                Some("SN-OK"),
                Some(&location_a),
                "op-options-eligible",
            ),
        )
        .unwrap();

        let restricted_id =
            generated_id("ASSET", "op-options-restricted", "INV-OPTIONS-RESTRICTED");
        create_physical_asset(
            &storage_root,
            asset_input(
                "INV-OPTIONS-RESTRICTED",
                None,
                Some(&location_a),
                "op-options-restricted",
            ),
        )
        .unwrap();
        transition_physical_asset_service_state(
            &storage_root,
            TransitionPhysicalAssetServiceStateInput {
                asset_id: restricted_id,
                expected_revision: 1,
                service_state: "restricted".to_owned(),
                service_state_reason: "Usage sous surveillance".to_owned(),
                context: context("op-options-restricted-state"),
            },
        )
        .unwrap();

        let out_of_service_id =
            generated_id("ASSET", "op-options-out-of-service", "INV-OPTIONS-OOS");
        create_physical_asset(
            &storage_root,
            asset_input(
                "INV-OPTIONS-OOS",
                None,
                Some(&location_a),
                "op-options-out-of-service",
            ),
        )
        .unwrap();
        transition_physical_asset_service_state(
            &storage_root,
            TransitionPhysicalAssetServiceStateInput {
                asset_id: out_of_service_id,
                expected_revision: 1,
                service_state: "out_of_service".to_owned(),
                service_state_reason: "Panne confirmée".to_owned(),
                context: context("op-options-out-of-service-state"),
            },
        )
        .unwrap();

        let unavailable_id =
            generated_id("ASSET", "op-options-unavailable", "INV-OPTIONS-UNAVAILABLE");
        create_physical_asset(
            &storage_root,
            asset_input(
                "INV-OPTIONS-UNAVAILABLE",
                None,
                Some(&location_a),
                "op-options-unavailable",
            ),
        )
        .unwrap();
        transition_physical_asset_administrative_availability(
            &storage_root,
            TransitionPhysicalAssetAdministrativeAvailabilityInput {
                asset_id: unavailable_id,
                expected_revision: 1,
                administrative_availability: "unavailable".to_owned(),
                administrative_unavailability_reason: "Prêt externe".to_owned(),
                context: context("op-options-unavailable-state"),
            },
        )
        .unwrap();

        create_physical_asset(
            &storage_root,
            asset_input("INV-OPTIONS-NO-LOC", None, None, "op-options-no-location"),
        )
        .unwrap();
        create_physical_asset(
            &storage_root,
            asset_input(
                "INV-OPTIONS-WRONG-LOC",
                None,
                Some(&location_b),
                "op-options-wrong-location",
            ),
        )
        .unwrap();

        seed_unresolved_migrated_asset(&storage_root, "LEGACY-OPTIONS-001");
        move_physical_asset(
            &storage_root,
            MovePhysicalAssetInput {
                asset_id: "LEGACY-OPTIONS-001".to_owned(),
                expected_revision: 1,
                destination_location_id: Some(location_a.clone()),
                context: context("op-options-move-legacy"),
            },
        )
        .unwrap();

        let expired_id = generated_id("ASSET", "op-options-expired", "INV-OPTIONS-EXPIRED");
        let mut expired_input = asset_input(
            "INV-OPTIONS-EXPIRED",
            None,
            Some(&location_a),
            "op-options-expired",
        );
        expired_input.calibration_requirement = "required".to_owned();
        expired_input.calibration_period_months = Some(12);
        create_physical_asset(&storage_root, expired_input).unwrap();
        seed_calibration(
            &storage_root,
            &expired_id,
            "CAL-OPTIONS-EXPIRED",
            "2025-07-26",
            "2026-07-26",
            "conforming",
        );

        let context = PhysicalAssetSelectionContext {
            assessed_at: OffsetDateTime::parse("2026-07-27T10:00:00Z", &Rfc3339).unwrap(),
            checked_on: MetrologyDate::parse_iso("2026-07-27").unwrap(),
            execution_mode: "accredited".to_owned(),
            laboratory_location_id: Some(location_a),
            excluded_schedule_item_code: None,
        };
        let initial = executable_physical_asset_options(&storage_root, &context).unwrap();
        assert!(option_by_inventory(&initial, "INV-OPTIONS-OK").eligible);
        assert!(option_by_inventory(&initial, "INV-OPTIONS-RESTRICTED").eligible);
        assert_reason(
            option_by_inventory(&initial, "INV-OPTIONS-RESTRICTED"),
            "service_restricted",
            false,
        );
        assert_reason(
            option_by_inventory(&initial, "INV-OPTIONS-OOS"),
            "service_out_of_service",
            true,
        );
        assert_reason(
            option_by_inventory(&initial, "INV-OPTIONS-UNAVAILABLE"),
            "administratively_unavailable",
            true,
        );
        assert_reason(
            option_by_inventory(&initial, "INV-OPTIONS-NO-LOC"),
            "location_missing",
            true,
        );
        assert_reason(
            option_by_inventory(&initial, "INV-OPTIONS-WRONG-LOC"),
            "location_mismatch",
            true,
        );
        assert_reason(
            option_by_inventory(&initial, "LEGACY-OPTIONS-001"),
            "model_reconciliation_required",
            true,
        );
        assert_reason(
            option_by_inventory(&initial, "INV-OPTIONS-EXPIRED"),
            "calibration_expired",
            true,
        );
        assert!(initial
            .iter()
            .all(|option| option.asset.asset_id != "EQM-SCOPE"));

        seed_planned_test_reservation(&storage_root, &eligible_id);
        let reserved = executable_physical_asset_options(&storage_root, &context).unwrap();
        assert_reason(
            option_by_inventory(&reserved, "INV-OPTIONS-OK"),
            "planned_test_reservation_conflict",
            true,
        );
        let mut own_schedule_context = context.clone();
        own_schedule_context.excluded_schedule_item_code = Some("SCHED-USAGE-001".to_owned());
        let own_schedule =
            executable_physical_asset_options(&storage_root, &own_schedule_context).unwrap();
        assert!(option_by_inventory(&own_schedule, "INV-OPTIONS-OK").eligible);

        seed_active_measurement_run(&storage_root, &eligible_id);
        let active =
            executable_physical_asset_options(&storage_root, &own_schedule_context).unwrap();
        assert_reason(
            option_by_inventory(&active, "INV-OPTIONS-OK"),
            "active_test_conflict",
            true,
        );
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

    fn seed_approved_model(storage_root: &Path, revision: u64, model_name: &str, _hash: char) {
        let connection = open_fleet_connection(storage_root).unwrap();
        let revision_id = format!("EQM-SCOPE-REV-{revision:04}");
        let definition = render_json(&json!({
            "definition_schema_version": "emc-locus.equipment-model-definition.v2",
            "manufacturer": "Acme Test",
            "model_name": model_name,
            "equipment_class": "controllable_instrument",
            "functional_role": "measurement_instrument",
            "category_code": "oscilloscope",
            "signal_domains": ["rf"],
            "technology_tags": [],
            "specifications": [],
            "signal_ports": [{
                "port_id": "rf_input",
                "label": "Entrée RF",
                "directionality": "input",
                "flow_role": "measurement_port",
                "signal_domain": "rf",
                "required": true,
                "technology_tags": [],
                "quantity": "voltage",
                "unit": "V",
                "impedance": 50.0,
                "differential": false,
                "isolated": false
            }],
            "communication_interfaces": [],
            "capabilities": [],
            "metadata": {}
        }));
        let canonical = EquipmentModelDefinition::from_json_str(&definition)
            .unwrap()
            .canonicalize()
            .unwrap();
        let definition = canonical.canonical_json;
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
        let checksum = canonical.definition_checksum;
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

    fn seed_unresolved_migrated_asset(storage_root: &Path, asset_id: &str) {
        let equipment = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        let migration_evidence = render_json(&json!({
            "source": "metrology.sqlite/legacy_instruments_0_21_1",
            "legacy_asset_id": asset_id,
            "legacy_model_reference": null
        }));
        equipment
            .execute(
                "INSERT INTO physical_assets (
                    asset_id, inventory_code, serial_number, part_number,
                    equipment_model_id, equipment_model_revision_id, equipment_model_checksum,
                    manufacturer_snapshot, model_name_snapshot, variant_snapshot,
                    category_code_snapshot, category_path_json, laboratory_location_id,
                    laboratory_location_label_snapshot, ownership_source, service_state,
                    availability_state, service_state_reason, notes, revision,
                    model_link_state, migrated_from_metrology, created_at, updated_at,
                    migration_evidence_json
                 ) VALUES (?1, ?1, NULL, NULL, NULL, NULL, NULL,
                    'Ancien fabricant', 'Modèle à rapprocher', NULL, 'legacy_metrology',
                    '[\"Ancien registre\"]', NULL, NULL, 'laboratory_owned', 'usable',
                    'available', '', 'Import historique', 1, 'migration_review_required', 1,
                    '2026-07-01T00:00:00Z', '2026-07-01T00:00:00Z', ?2)",
                params![asset_id, migration_evidence],
            )
            .unwrap();
        let metrology = Connection::open(storage_root.join("metrology.sqlite")).unwrap();
        metrology
            .execute(
                "INSERT INTO metrology_asset_dossiers (
                    asset_id, calibration_requirement, calibration_period_months,
                    calibration_due_warning_days, metrology_notes, legacy_capabilities_json,
                    revision, created_at, updated_at
                 ) VALUES (?1, 'not_required', NULL, 30, '', '[]', 1,
                    '2026-07-01T00:00:00Z', '2026-07-01T00:00:00Z')",
                params![asset_id],
            )
            .unwrap();
    }

    fn seed_revision_copy(
        storage_root: &Path,
        revision: u64,
        status: &str,
        checksum_override: Option<&str>,
    ) {
        let connection = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        let revision_id = format!("EQM-SCOPE-REV-{revision:04}");
        let source: (String, String, String) = connection
            .query_row(
                "SELECT definition_schema_version, definition_json, definition_checksum
                 FROM equipment_model_revisions WHERE revision_id = 'EQM-SCOPE-REV-0002'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO equipment_model_revisions (
                    revision_id, equipment_model_id, revision_number, parent_revision_id, status,
                    definition_schema_version, definition_json, definition_checksum, created_by,
                    created_at, updated_at, submitted_at, approved_at
                 ) VALUES (?1, 'EQM-SCOPE', ?2, 'EQM-SCOPE-REV-0002', ?3, ?4, ?5, ?6,
                    'test', '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z',
                    ?7, ?8)",
                params![
                    revision_id,
                    revision,
                    status,
                    source.0,
                    source.1,
                    checksum_override.unwrap_or(&source.2),
                    if status == "draft" {
                        None
                    } else {
                        Some("2026-07-26T00:00:00Z")
                    },
                    if status == "approved" {
                        Some("2026-07-26T00:00:00Z")
                    } else {
                        None
                    }
                ],
            )
            .unwrap();
    }

    fn reconciliation_input(
        asset_id: &str,
        expected_revision: u64,
        model_revision: u64,
        operation_id: &str,
    ) -> ReconcilePhysicalAssetModelInput {
        ReconcilePhysicalAssetModelInput {
            asset_id: asset_id.to_owned(),
            expected_revision,
            equipment_model_id: "EQM-SCOPE".to_owned(),
            equipment_model_revision_id: format!("EQM-SCOPE-REV-{model_revision:04}"),
            context: context(operation_id),
        }
    }

    fn model_revision_checksum(storage_root: &Path, revision: u64) -> String {
        let connection = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        connection
            .query_row(
                "SELECT definition_checksum FROM equipment_model_revisions WHERE revision_id = ?1",
                params![format!("EQM-SCOPE-REV-{revision:04}")],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn migration_evidence(storage_root: &Path, asset_id: &str) -> String {
        let connection = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        connection
            .query_row(
                "SELECT migration_evidence_json FROM physical_assets WHERE asset_id = ?1",
                params![asset_id],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn seed_conflicting_outbox_operation(storage_root: &Path, operation_id: &str) {
        let connection = Connection::open(storage_root.join("sync.sqlite")).unwrap();
        connection
            .execute(
                "INSERT INTO sync_operations (
                    operation_id, domain, entity_type, entity_id, operation_kind,
                    base_revision, resulting_revision, actor_id, device_id, correlation_id,
                    payload_json, payload_checksum, status, occurred_at, recorded_at
                 ) VALUES (?1, 'equipment', 'fixture', 'fixture', 'fixture', 'rev-0000',
                    'rev-0001', 'fixture', 'fixture', 'fixture', '{}',
                    'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
                    'pending', '2026-07-01T00:00:00Z', '2026-07-01T00:00:00Z')",
                params![operation_id],
            )
            .unwrap();
    }

    fn asset_at(storage_root: &Path, asset_id: &str, assessed_at: &str) -> Value {
        let response =
            json_value(&list_physical_assets_json_at(storage_root, Some(assessed_at)).unwrap());
        response["assets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|asset| asset["asset_id"] == asset_id)
            .unwrap()
            .clone()
    }

    fn seed_station_setup_reference(storage_root: &Path, asset_id: &str) {
        let mut connection = Connection::open(storage_root.join("station.sqlite")).unwrap();
        let transaction = connection.transaction().unwrap();
        transaction
            .execute(
                "INSERT INTO station_setup_identities (
                    setup_id, label, current_ready_revision_id, created_by, created_at, updated_at
                 ) VALUES ('SETUP-USAGE-001', 'Montage émission conduite', NULL, 'fixture',
                    '2026-07-27T07:00:00Z', '2026-07-27T07:00:00Z')",
                [],
            )
            .unwrap();
        let definition = render_json(&json!({
            "asset_bindings": [{ "asset_id": asset_id }]
        }));
        transaction
            .execute(
                "INSERT INTO station_setup_revisions (
                    revision_id, setup_id, revision_number, parent_revision_id, status,
                    definition_schema_version, definition_json, definition_checksum,
                    readiness_json, created_by, created_at, updated_at, ready_at
                 ) VALUES ('SETUP-USAGE-001-REV-0001', 'SETUP-USAGE-001', 1, NULL, 'ready',
                    'fixture.v1', ?1,
                    'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
                    '{}', 'fixture', '2026-07-27T07:00:00Z', '2026-07-27T07:00:00Z',
                    '2026-07-27T07:00:00Z')",
                params![definition],
            )
            .unwrap();
        transaction
            .execute(
                "UPDATE station_setup_identities SET current_ready_revision_id =
                    'SETUP-USAGE-001-REV-0001' WHERE setup_id = 'SETUP-USAGE-001'",
                [],
            )
            .unwrap();
        transaction.commit().unwrap();
    }

    fn seed_planned_test_reservation(storage_root: &Path, asset_id: &str) {
        let mut connection = Connection::open(storage_root.join("projects.sqlite")).unwrap();
        let transaction = connection.transaction().unwrap();
        transaction
            .execute(
                "INSERT INTO projects (code, customer_name, stage, execution_mode, created_at)
                 VALUES ('PRJ-USAGE', 'Client usage', 'test_planning', 'non_accredited',
                    '2026-07-27T07:00:00Z')",
                [],
            )
            .unwrap();
        transaction
            .execute(
                "INSERT INTO service_schedule_items (
                    item_code, project_code, title, planned_start_at, planned_end_at,
                    assigned_operator, location, equipment_under_test, status, notes,
                    created_at, updated_at, revision, created_by, updated_by
                 ) VALUES ('SCHED-USAGE-001', 'PRJ-USAGE', 'Essai réservé',
                    '2026-07-27T09:00:00Z', '2026-07-27T11:00:00Z', 'operator',
                    'Salle CEM', 'Objet client', 'confirmed', '',
                    '2026-07-27T07:00:00Z', '2026-07-27T07:00:00Z', 1, 'fixture', 'fixture')",
                [],
            )
            .unwrap();
        transaction
            .execute(
                "INSERT INTO planned_test_preparation_identities (
                    project_code, schedule_item_code, current_revision_id,
                    created_by, created_at, updated_at
                 ) VALUES ('PRJ-USAGE', 'SCHED-USAGE-001', NULL, 'fixture',
                    '2026-07-27T07:00:00Z', '2026-07-27T07:00:00Z')",
                [],
            )
            .unwrap();
        let definition = render_json(&json!({
            "station_setup": { "assets": [{ "asset_id": asset_id }] }
        }));
        transaction
            .execute(
                "INSERT INTO planned_test_preparation_revisions (
                    revision_id, project_code, schedule_item_code, revision_number,
                    parent_revision_id, schedule_revision, method_template_id,
                    method_revision_id, method_definition_checksum, station_setup_id,
                    station_setup_revision_id, station_setup_definition_checksum,
                    verdict_state, definition_schema_version, definition_json,
                    definition_checksum, operation_id, request_checksum, actor, reason,
                    device_id, correlation_id, created_at
                 ) VALUES ('PREP-USAGE-001-REV-0001', 'PRJ-USAGE', 'SCHED-USAGE-001', 1,
                    NULL, 1, 'METHOD-USAGE', 'METHOD-USAGE-REV-1',
                    'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
                    'SETUP-USAGE-001', 'SETUP-USAGE-001-REV-0001',
                    'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
                    'ready', 'fixture.v1', ?1,
                    'sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc',
                    'op-prep-usage',
                    'sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd',
                    'fixture', 'fixture', 'fixture', 'fixture', '2026-07-27T07:00:00Z')",
                params![definition],
            )
            .unwrap();
        transaction
            .execute(
                "UPDATE planned_test_preparation_identities SET current_revision_id =
                    'PREP-USAGE-001-REV-0001' WHERE schedule_item_code = 'SCHED-USAGE-001'",
                [],
            )
            .unwrap();
        transaction.commit().unwrap();
    }

    fn seed_active_measurement_run(storage_root: &Path, asset_id: &str) {
        let connection = Connection::open(storage_root.join("projects.sqlite")).unwrap();
        connection
            .execute(
                "INSERT INTO campaigns (
                    project_code, name, standard_reference, equipment_under_test, started_at
                 ) VALUES ('PRJ-USAGE', 'Campagne CEM', 'Méthode interne', 'Objet client',
                    '2026-07-27T09:30:00Z')",
                [],
            )
            .unwrap();
        let campaign_id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO measurement_runs (
                    campaign_id, operator, method_reference, software_version, started_at
                 ) VALUES (?1, 'operator', 'Méthode interne', '0.22.0-dev',
                    '2026-07-27T09:30:00Z')",
                params![campaign_id],
            )
            .unwrap();
        let run_id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO measurement_run_instruments (
                    measurement_run_id, asset_id, role, readiness_status
                 ) VALUES (?1, ?2, 'Mesure', 'ready')",
                params![run_id, asset_id],
            )
            .unwrap();
    }

    fn complete_measurement_run(storage_root: &Path) {
        let connection = Connection::open(storage_root.join("projects.sqlite")).unwrap();
        connection
            .execute(
                "UPDATE measurement_runs SET completed_at = '2026-07-27T10:30:00Z'",
                [],
            )
            .unwrap();
    }

    fn seed_calibration(
        storage_root: &Path,
        asset_id: &str,
        event_id: &str,
        calibrated_at: &str,
        due_at: &str,
        decision: &str,
    ) {
        let connection = Connection::open(storage_root.join("metrology.sqlite")).unwrap();
        connection
            .execute(
                "INSERT INTO calibration_events (
                    event_id, asset_id, certificate_reference, calibrated_at, due_at,
                    provider, decision, adjustment_performed, uncertainty_summary_json,
                    comment, recorded_at, recorded_by, revision
                 ) VALUES (?1, ?2, ?1, ?3, ?4, 'Laboratoire étalon', ?5, 0, '{}', '',
                    '2026-07-27T08:00:00Z', 'fixture', 'rev-0001')",
                params![event_id, asset_id, calibrated_at, due_at, decision],
            )
            .unwrap();
    }

    fn option_by_inventory<'a>(
        options: &'a [ExecutablePhysicalAssetOptionDto],
        inventory_code: &str,
    ) -> &'a ExecutablePhysicalAssetOptionDto {
        options
            .iter()
            .find(|option| option.asset.inventory_code == inventory_code)
            .unwrap()
    }

    fn assert_reason(option: &ExecutablePhysicalAssetOptionDto, code: &str, blocking: bool) {
        let reasons = if blocking {
            &option.blocking_reasons
        } else {
            &option.warnings
        };
        assert!(reasons.iter().any(|reason| reason.code == code), "{code}");
        if blocking {
            assert!(!option.eligible);
        }
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
            administrative_availability: "available".to_owned(),
            administrative_unavailability_reason: String::new(),
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
