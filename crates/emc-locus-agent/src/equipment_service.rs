use crate::{
    equipment_dto::{
        CommunicationProviderStatusDto, CommunicationProviderStatusListDto,
        DriverProfileAggregateDto, DriverProfileEnvelopeDto, DriverProfileIdentityDto,
        DriverProfileListDto, DriverProfileRevisionDto, DriverProfileRevisionEnvelopeDto,
        DriverProfileRevisionListDto, EquipmentAuditEventDto, EquipmentAuditEventsDto,
        EquipmentClassificationPresetDto, EquipmentClassificationPresetEnvelopeDto,
        EquipmentClassificationPresetListDto, EquipmentClassificationPresetPortDto,
        EquipmentDefinitionValidationDto, EquipmentDefinitionValidationIssueDto,
        EquipmentModelAggregateDto, EquipmentModelEnvelopeDto, EquipmentModelIdentityDto,
        EquipmentModelListDto, EquipmentModelRevisionDto, EquipmentModelRevisionEnvelopeDto,
        EquipmentModelRevisionListDto, EquipmentOperationResultDto, EquipmentRegistriesDto,
        EquipmentRegistryItemDto,
    },
    equipment_repository::{
        ensure_equipment_operation_replay, equipment_model_class_exists,
        existing_equipment_operation, insert_driver_profile_identity,
        insert_driver_profile_revision, insert_equipment_audit_event,
        insert_equipment_model_identity, insert_equipment_model_revision,
        insert_equipment_sync_operation, list_driver_profile_identities,
        list_driver_profile_revisions as load_driver_revision_history,
        list_equipment_classification_preset_ports, list_equipment_classification_presets,
        list_equipment_flow_role_registry, list_equipment_functional_role_registry,
        list_equipment_model_identities,
        list_equipment_model_revisions as load_model_revision_history,
        list_equipment_port_directionality_registry, list_equipment_signal_domain_registry,
        list_equipment_technology_tag_registry, load_active_draft_driver_profile_revision,
        load_active_draft_equipment_model_revision, load_current_approved_driver_profile_revision,
        load_current_approved_equipment_model_revision, load_driver_profile_identity,
        load_driver_profile_revision, load_equipment_audit_events,
        load_equipment_classification_preset, load_equipment_model_identity,
        load_equipment_model_revision, load_latest_driver_profile_revision,
        load_latest_equipment_model_revision, next_driver_profile_revision_number,
        next_equipment_model_revision_number, open_equipment_connection,
        open_equipment_connection_with_sync, replace_equipment_model_classification_summary,
        set_current_approved_driver_profile_revision,
        set_current_approved_equipment_model_revision, supersede_approved_driver_profile_revision,
        supersede_approved_equipment_model_revision, touch_driver_profile_identity,
        touch_equipment_model_identity, update_driver_profile_revision_definition,
        update_driver_profile_revision_status, update_equipment_model_revision_definition,
        update_equipment_model_revision_status, DriverProfileListFilter, EquipmentAuditEventInput,
        EquipmentClassificationSummaryRecord, EquipmentModelListFilter,
        EquipmentOperationFingerprintInput, EquipmentSyncOperationInput,
        NewDriverProfileIdentityRecord, NewDriverProfileRevisionRecord,
        NewEquipmentModelIdentityRecord, NewEquipmentModelRevisionRecord,
        StoredDriverProfileIdentity, StoredDriverProfileRevision, StoredEquipmentAuditEvent,
        StoredEquipmentClassificationPreset, StoredEquipmentClassificationPresetPort,
        StoredEquipmentModelIdentity, StoredEquipmentModelRevision, StoredEquipmentOperation,
        StoredEquipmentRegistryItem, UpdateDefinitionInput, UpdateDriverDefinitionCounts,
        UpdateModelDefinitionCounts, UpdateStatusInput,
    },
    render_json, AgentError,
};
use emc_locus_core::equipment::{
    simulate_driver_action, DefinitionValidationIssue, DriverProfileDefinition,
    DriverSimulationScenario, EquipmentClass, EquipmentModelDefinition, EquipmentRevisionStatus,
    FunctionalRole, PhysicalQuantity, SignalDomain, SignalPortDefinition, TechnologyTag,
    EQUIPMENT_MODEL_DEFINITION_SCHEMA_VERSION,
};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateEquipmentModelInput {
    pub equipment_model_id: String,
    pub definition_json: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListEquipmentModelsInput {
    pub manufacturer: Option<String>,
    pub equipment_class: Option<String>,
    pub category_code: Option<String>,
    pub functional_role: Option<String>,
    pub signal_domain: Option<String>,
    pub technology_tag: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateEquipmentModelFromPresetInput {
    pub preset_id: String,
    pub equipment_model_id: String,
    pub manufacturer: String,
    pub model_name: String,
    pub variant: Option<String>,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplaceEquipmentModelDefinitionInput {
    pub equipment_model_id: String,
    pub revision_id: String,
    pub expected_definition_checksum: String,
    pub definition_json: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateEquipmentModelRevisionInput {
    pub equipment_model_id: String,
    pub source_revision_id: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloneEquipmentModelInput {
    pub source_equipment_model_id: String,
    pub source_revision_id: Option<String>,
    pub new_equipment_model_id: String,
    pub manufacturer: Option<String>,
    pub model_name: Option<String>,
    pub variant: Option<String>,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionEquipmentModelRevisionInput {
    pub equipment_model_id: String,
    pub revision_id: String,
    pub target_status: EquipmentRevisionStatus,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateDriverProfileInput {
    pub driver_profile_id: String,
    pub label: String,
    pub definition_json: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListDriverProfilesInput {
    pub equipment_model_id: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplaceDriverProfileDefinitionInput {
    pub driver_profile_id: String,
    pub revision_id: String,
    pub expected_definition_checksum: String,
    pub definition_json: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateDriverProfileRevisionInput {
    pub driver_profile_id: String,
    pub source_revision_id: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionDriverProfileRevisionInput {
    pub driver_profile_id: String,
    pub revision_id: String,
    pub target_status: EquipmentRevisionStatus,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimulateDriverProfileInput {
    pub driver_profile_id: String,
    pub revision_id: Option<String>,
    pub action_id: String,
    pub scenario_json: String,
}

pub fn validate_equipment_model_definition_json(
    definition_json: &str,
) -> Result<String, AgentError> {
    match EquipmentModelDefinition::from_json_str(definition_json) {
        Ok(definition) => match definition.canonicalize() {
            Ok(canonical) => Ok(render_json(&EquipmentDefinitionValidationDto {
                valid: true,
                issues: Vec::new(),
                definition_schema_version: Some(canonical.definition_schema_version),
                definition_checksum: Some(canonical.definition_checksum),
                canonical_json: Some(canonical.canonical_json),
            })),
            Err(issues) => Ok(render_definition_issues(issues)),
        },
        Err(issue) => Ok(render_definition_issues(vec![issue])),
    }
}

pub fn validate_driver_profile_definition_json(
    storage_root: &Path,
    definition_json: &str,
) -> Result<String, AgentError> {
    let definition = match DriverProfileDefinition::from_json_str(definition_json) {
        Ok(definition) => definition,
        Err(issue) => return Ok(render_definition_issues(vec![issue])),
    };
    let model = approved_model_definition_for_driver(storage_root, &definition)?;
    match definition.canonicalize(Some(&model)) {
        Ok(canonical) => Ok(render_json(&EquipmentDefinitionValidationDto {
            valid: true,
            issues: Vec::new(),
            definition_schema_version: Some(canonical.definition_schema_version),
            definition_checksum: Some(canonical.definition_checksum),
            canonical_json: Some(canonical.canonical_json),
        })),
        Err(issues) => Ok(render_definition_issues(issues)),
    }
}

pub fn list_equipment_models(
    storage_root: &Path,
    input: ListEquipmentModelsInput,
) -> Result<String, AgentError> {
    validate_optional_id(input.equipment_class.as_deref(), "equipment_class")?;
    validate_optional_id(input.category_code.as_deref(), "category_code")?;
    validate_optional_id(input.functional_role.as_deref(), "functional_role")?;
    validate_optional_id(input.signal_domain.as_deref(), "signal_domain")?;
    validate_optional_id(input.technology_tag.as_deref(), "technology_tag")?;
    validate_optional_id(input.status.as_deref(), "status")?;
    let connection = open_equipment_connection(storage_root)?;
    let identities = list_equipment_model_identities(
        &connection,
        EquipmentModelListFilter {
            manufacturer: input.manufacturer.as_deref(),
            equipment_class: input.equipment_class.as_deref(),
            category_code: input.category_code.as_deref(),
            functional_role: input.functional_role.as_deref(),
            signal_domain: input.signal_domain.as_deref(),
            technology_tag: input.technology_tag.as_deref(),
            status: input.status.as_deref(),
            search: input.search.as_deref(),
        },
    )?;
    let equipment_models = identities
        .iter()
        .map(|identity| model_aggregate_for_identity(&connection, identity))
        .collect::<Result<Vec<_>, AgentError>>()?;
    Ok(render_json(&EquipmentModelListDto {
        equipment_models: equipment_models
            .iter()
            .map(equipment_model_aggregate_dto)
            .collect(),
    }))
}

pub fn equipment_registries(storage_root: &Path) -> Result<String, AgentError> {
    let connection = open_equipment_connection(storage_root)?;
    Ok(render_json(&EquipmentRegistriesDto {
        functional_roles: list_equipment_functional_role_registry(&connection)?
            .iter()
            .map(registry_item_dto)
            .collect(),
        signal_domains: list_equipment_signal_domain_registry(&connection)?
            .iter()
            .map(registry_item_dto)
            .collect(),
        port_directionalities: list_equipment_port_directionality_registry(&connection)?
            .iter()
            .map(registry_item_dto)
            .collect(),
        flow_roles: list_equipment_flow_role_registry(&connection)?
            .iter()
            .map(registry_item_dto)
            .collect(),
        technology_tags: list_equipment_technology_tag_registry(&connection)?
            .iter()
            .map(registry_item_dto)
            .collect(),
    }))
}

pub fn list_classification_presets(storage_root: &Path) -> Result<String, AgentError> {
    let connection = open_equipment_connection(storage_root)?;
    let presets = list_equipment_classification_presets(&connection)?;
    let presets = presets
        .iter()
        .map(|preset| classification_preset_dto(&connection, preset))
        .collect::<Result<Vec<_>, AgentError>>()?;
    Ok(render_json(&EquipmentClassificationPresetListDto {
        presets,
    }))
}

pub fn get_classification_preset(
    storage_root: &Path,
    preset_id: &str,
) -> Result<String, AgentError> {
    validate_id(preset_id, "preset_id")?;
    let connection = open_equipment_connection(storage_root)?;
    let preset =
        load_equipment_classification_preset(&connection, preset_id)?.ok_or_else(|| {
            AgentError::new(
                "equipment_classification_preset_not_found",
                format!("equipment classification preset not found: {preset_id}"),
            )
        })?;
    Ok(render_json(&EquipmentClassificationPresetEnvelopeDto {
        preset: classification_preset_dto(&connection, &preset)?,
    }))
}

pub fn create_equipment_model_from_preset(
    storage_root: &Path,
    input: CreateEquipmentModelFromPresetInput,
) -> Result<String, AgentError> {
    validate_common_operation(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    validate_id(&input.preset_id, "preset_id")?;
    validate_id(&input.equipment_model_id, "equipment_model_id")?;
    if input.manufacturer.trim().is_empty() {
        return Err(AgentError::new(
            "invalid_manufacturer",
            "manufacturer is required",
        ));
    }
    if input.model_name.trim().is_empty() {
        return Err(AgentError::new(
            "invalid_model_name",
            "model_name is required",
        ));
    }

    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let preset =
        load_equipment_classification_preset(&connection, &input.preset_id)?.ok_or_else(|| {
            AgentError::new(
                "equipment_classification_preset_not_found",
                format!(
                    "equipment classification preset not found: {}",
                    input.preset_id
                ),
            )
        })?;
    if preset.deprecated {
        return Err(AgentError::new(
            "equipment_classification_preset_deprecated",
            "deprecated classification presets cannot create new models",
        ));
    }
    let ports = list_equipment_classification_preset_ports(&connection, &input.preset_id)?;
    let definition = equipment_model_definition_from_preset(&preset, &ports, &input)?;
    let canonical = definition
        .canonicalize()
        .map_err(|issues| invalid_definition_error("invalid_equipment_model_definition", issues))?;
    let parsed =
        EquipmentModelDefinition::from_json_str(&canonical.canonical_json).map_err(|issue| {
            invalid_definition_error("invalid_equipment_model_definition", vec![issue])
        })?;
    let revision_id = model_revision_id_for(&input.equipment_model_id, 1);
    let payload_json = render_json(&json!({
        "equipment_model_id": input.equipment_model_id,
        "preset_id": input.preset_id,
        "definition_checksum": canonical.definition_checksum
    }));
    if let Some(operation) = existing_equipment_operation(&connection, &input.operation_id)? {
        ensure_equipment_operation_replay(
            &operation,
            EquipmentOperationFingerprintInput {
                aggregate_kind: "equipment_model",
                entity_id: &input.equipment_model_id,
                revision_id: Some(&revision_id),
                action: "equipment_model_created_from_preset",
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: None,
                new_revision_id: Some(&revision_id),
                old_definition_checksum: None,
                new_definition_checksum: Some(&canonical.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return model_operation_replay_result(&connection, &operation);
    }
    if load_equipment_model_identity(&connection, &input.equipment_model_id)?.is_some() {
        return Err(AgentError::new(
            "equipment_model_already_exists",
            format!(
                "equipment model already exists: {}",
                input.equipment_model_id
            ),
        ));
    }
    if !equipment_model_class_exists(&connection, equipment_class_text(parsed.equipment_class))? {
        return Err(AgentError::new(
            "equipment_model_class_not_found",
            format!(
                "unknown equipment class: {}",
                equipment_class_text(parsed.equipment_class)
            ),
        ));
    }

    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_equipment_model_identity(
        &transaction,
        NewEquipmentModelIdentityRecord {
            equipment_model_id: &input.equipment_model_id,
            manufacturer: parsed.manufacturer.trim(),
            model_name: parsed.model_name.trim(),
            variant: parsed.variant.as_deref(),
            equipment_class: equipment_class_text(parsed.equipment_class),
            category_code: &parsed.category_code,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_equipment_model_revision(
        &transaction,
        NewEquipmentModelRevisionRecord {
            revision_id: &revision_id,
            equipment_model_id: &input.equipment_model_id,
            revision_number: 1,
            parent_revision_id: None,
            status: revision_status_text(EquipmentRevisionStatus::Draft),
            definition_schema_version: &canonical.definition_schema_version,
            definition_json: &canonical.canonical_json,
            definition_checksum: &canonical.definition_checksum,
            created_by: &input.actor,
            timestamp: &now,
            capability_count: parsed.capabilities.len() as u32,
            interface_count: parsed.communication_interfaces.len() as u32,
            signal_port_count: parsed.signal_ports.len() as u32,
        },
    )?;
    write_equipment_model_classification_summary(
        &transaction,
        EquipmentModelClassificationSummaryInput {
            equipment_model_id: &input.equipment_model_id,
            revision_id: &revision_id,
            revision_number: 1,
            status: revision_status_text(EquipmentRevisionStatus::Draft),
            definition_checksum: &canonical.definition_checksum,
            definition: &parsed,
            timestamp: &now,
        },
    )?;
    write_equipment_audit_and_outbox(
        &transaction,
        EquipmentAuditEventInput {
            aggregate_kind: "equipment_model",
            entity_id: &input.equipment_model_id,
            revision_id: Some(&revision_id),
            action: "equipment_model_created_from_preset",
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: None,
            new_revision_id: Some(&revision_id),
            old_definition_checksum: None,
            new_definition_checksum: Some(&canonical.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
        EquipmentSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "equipment_model_revision",
            entity_id: &revision_id,
            operation_kind: "equipment_model_created_from_preset",
            base_revision: "none",
            resulting_revision: &revision_id,
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    model_operation_result_for_revision(
        &connection,
        "equipment_model_created_from_preset",
        &input.operation_id,
        false,
        &input.equipment_model_id,
        &revision_id,
    )
}

pub fn create_equipment_model(
    storage_root: &Path,
    input: CreateEquipmentModelInput,
) -> Result<String, AgentError> {
    validate_common_operation(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    validate_id(&input.equipment_model_id, "equipment_model_id")?;
    let definition = canonical_equipment_model_definition(&input.definition_json)?;
    let parsed =
        EquipmentModelDefinition::from_json_str(&definition.canonical_json).map_err(|issue| {
            invalid_definition_error("invalid_equipment_model_definition", vec![issue])
        })?;
    let revision_id = model_revision_id_for(&input.equipment_model_id, 1);
    let payload_json = render_json(&json!({
        "equipment_model_id": input.equipment_model_id,
        "definition_checksum": definition.definition_checksum
    }));
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    if let Some(operation) = existing_equipment_operation(&connection, &input.operation_id)? {
        ensure_equipment_operation_replay(
            &operation,
            EquipmentOperationFingerprintInput {
                aggregate_kind: "equipment_model",
                entity_id: &input.equipment_model_id,
                revision_id: Some(&revision_id),
                action: "equipment_model_created",
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: None,
                new_revision_id: Some(&revision_id),
                old_definition_checksum: None,
                new_definition_checksum: Some(&definition.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return model_operation_replay_result(&connection, &operation);
    }
    if load_equipment_model_identity(&connection, &input.equipment_model_id)?.is_some() {
        return Err(AgentError::new(
            "equipment_model_already_exists",
            format!(
                "equipment model already exists: {}",
                input.equipment_model_id
            ),
        ));
    }
    if !equipment_model_class_exists(&connection, equipment_class_text(parsed.equipment_class))? {
        return Err(AgentError::new(
            "equipment_model_class_not_found",
            format!(
                "unknown equipment class: {}",
                equipment_class_text(parsed.equipment_class)
            ),
        ));
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_equipment_model_identity(
        &transaction,
        NewEquipmentModelIdentityRecord {
            equipment_model_id: &input.equipment_model_id,
            manufacturer: parsed.manufacturer.trim(),
            model_name: parsed.model_name.trim(),
            variant: parsed.variant.as_deref(),
            equipment_class: equipment_class_text(parsed.equipment_class),
            category_code: &parsed.category_code,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_equipment_model_revision(
        &transaction,
        NewEquipmentModelRevisionRecord {
            revision_id: &revision_id,
            equipment_model_id: &input.equipment_model_id,
            revision_number: 1,
            parent_revision_id: None,
            status: revision_status_text(EquipmentRevisionStatus::Draft),
            definition_schema_version: &definition.definition_schema_version,
            definition_json: &definition.canonical_json,
            definition_checksum: &definition.definition_checksum,
            created_by: &input.actor,
            timestamp: &now,
            capability_count: parsed.capabilities.len() as u32,
            interface_count: parsed.communication_interfaces.len() as u32,
            signal_port_count: parsed.signal_ports.len() as u32,
        },
    )?;
    write_equipment_model_classification_summary(
        &transaction,
        EquipmentModelClassificationSummaryInput {
            equipment_model_id: &input.equipment_model_id,
            revision_id: &revision_id,
            revision_number: 1,
            status: revision_status_text(EquipmentRevisionStatus::Draft),
            definition_checksum: &definition.definition_checksum,
            definition: &parsed,
            timestamp: &now,
        },
    )?;
    write_equipment_audit_and_outbox(
        &transaction,
        EquipmentAuditEventInput {
            aggregate_kind: "equipment_model",
            entity_id: &input.equipment_model_id,
            revision_id: Some(&revision_id),
            action: "equipment_model_created",
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: None,
            new_revision_id: Some(&revision_id),
            old_definition_checksum: None,
            new_definition_checksum: Some(&definition.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
        EquipmentSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "equipment_model_revision",
            entity_id: &revision_id,
            operation_kind: "equipment_model_created",
            base_revision: "none",
            resulting_revision: &revision_id,
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    model_operation_result_for_revision(
        &connection,
        "equipment_model_created",
        &input.operation_id,
        false,
        &input.equipment_model_id,
        &revision_id,
    )
}

pub fn clone_equipment_model(
    storage_root: &Path,
    input: CloneEquipmentModelInput,
) -> Result<String, AgentError> {
    validate_common_operation(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    validate_id(
        &input.source_equipment_model_id,
        "source_equipment_model_id",
    )?;
    validate_id(&input.new_equipment_model_id, "new_equipment_model_id")?;
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let source_identity = require_model_identity(&connection, &input.source_equipment_model_id)?;
    let source_revision = match input.source_revision_id.as_deref() {
        Some(source_revision_id) => load_equipment_model_revision(
            &connection,
            &input.source_equipment_model_id,
            source_revision_id,
        )?,
        None => load_current_approved_equipment_model_revision(&connection, &source_identity)?
            .or_else(|| {
                load_latest_equipment_model_revision(&connection, &input.source_equipment_model_id)
                    .ok()
                    .flatten()
            }),
    }
    .ok_or_else(|| {
        AgentError::new(
            "equipment_model_revision_not_found",
            "source model revision not found",
        )
    })?;
    if load_equipment_model_identity(&connection, &input.new_equipment_model_id)?.is_some() {
        return Err(AgentError::new(
            "equipment_model_already_exists",
            format!(
                "equipment model already exists: {}",
                input.new_equipment_model_id
            ),
        ));
    }
    let mut cloned_definition = EquipmentModelDefinition::from_json_str(
        &source_revision.definition_json,
    )
    .map_err(|issue| invalid_definition_error("invalid_equipment_model_definition", vec![issue]))?;
    cloned_definition.manufacturer = input
        .manufacturer
        .clone()
        .unwrap_or_else(|| source_identity.manufacturer.clone());
    cloned_definition.model_name = input
        .model_name
        .clone()
        .unwrap_or_else(|| format!("{} copy", source_identity.model_name));
    cloned_definition.variant = input.variant.clone().or(source_identity.variant.clone());
    let canonical = cloned_definition
        .canonicalize()
        .map_err(|issues| invalid_definition_error("invalid_equipment_model_definition", issues))?;
    let revision_id = model_revision_id_for(&input.new_equipment_model_id, 1);
    let payload_json = render_json(&json!({
        "source_equipment_model_id": input.source_equipment_model_id,
        "source_revision_id": source_revision.revision_id,
        "new_equipment_model_id": input.new_equipment_model_id,
        "new_definition_checksum": canonical.definition_checksum
    }));
    if let Some(operation) = existing_equipment_operation(&connection, &input.operation_id)? {
        ensure_equipment_operation_replay(
            &operation,
            EquipmentOperationFingerprintInput {
                aggregate_kind: "equipment_model",
                entity_id: &input.new_equipment_model_id,
                revision_id: Some(&revision_id),
                action: "equipment_model_cloned",
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: Some(&source_revision.revision_id),
                new_revision_id: Some(&revision_id),
                old_definition_checksum: Some(&source_revision.definition_checksum),
                new_definition_checksum: Some(&canonical.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return model_operation_replay_result(&connection, &operation);
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_equipment_model_identity(
        &transaction,
        NewEquipmentModelIdentityRecord {
            equipment_model_id: &input.new_equipment_model_id,
            manufacturer: &cloned_definition.manufacturer,
            model_name: &cloned_definition.model_name,
            variant: cloned_definition.variant.as_deref(),
            equipment_class: equipment_class_text(cloned_definition.equipment_class),
            category_code: &cloned_definition.category_code,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_equipment_model_revision(
        &transaction,
        NewEquipmentModelRevisionRecord {
            revision_id: &revision_id,
            equipment_model_id: &input.new_equipment_model_id,
            revision_number: 1,
            parent_revision_id: Some(&source_revision.revision_id),
            status: "draft",
            definition_schema_version: &canonical.definition_schema_version,
            definition_json: &canonical.canonical_json,
            definition_checksum: &canonical.definition_checksum,
            created_by: &input.actor,
            timestamp: &now,
            capability_count: cloned_definition.capabilities.len() as u32,
            interface_count: cloned_definition.communication_interfaces.len() as u32,
            signal_port_count: cloned_definition.signal_ports.len() as u32,
        },
    )?;
    write_equipment_model_classification_summary(
        &transaction,
        EquipmentModelClassificationSummaryInput {
            equipment_model_id: &input.new_equipment_model_id,
            revision_id: &revision_id,
            revision_number: 1,
            status: revision_status_text(EquipmentRevisionStatus::Draft),
            definition_checksum: &canonical.definition_checksum,
            definition: &cloned_definition,
            timestamp: &now,
        },
    )?;
    write_equipment_audit_and_outbox(
        &transaction,
        EquipmentAuditEventInput {
            aggregate_kind: "equipment_model",
            entity_id: &input.new_equipment_model_id,
            revision_id: Some(&revision_id),
            action: "equipment_model_cloned",
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: Some(&source_revision.revision_id),
            new_revision_id: Some(&revision_id),
            old_definition_checksum: Some(&source_revision.definition_checksum),
            new_definition_checksum: Some(&canonical.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
        EquipmentSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "equipment_model_revision",
            entity_id: &revision_id,
            operation_kind: "equipment_model_cloned",
            base_revision: &source_revision.revision_id,
            resulting_revision: &revision_id,
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    model_operation_result_for_revision(
        &connection,
        "equipment_model_cloned",
        &input.operation_id,
        false,
        &input.new_equipment_model_id,
        &revision_id,
    )
}

pub fn get_equipment_model(
    storage_root: &Path,
    equipment_model_id: &str,
) -> Result<String, AgentError> {
    validate_id(equipment_model_id, "equipment_model_id")?;
    let connection = open_equipment_connection(storage_root)?;
    let identity =
        load_equipment_model_identity(&connection, equipment_model_id)?.ok_or_else(|| {
            AgentError::new(
                "equipment_model_not_found",
                format!("equipment model not found: {equipment_model_id}"),
            )
        })?;
    let aggregate = model_aggregate_for_identity(&connection, &identity)?;
    Ok(render_json(&EquipmentModelEnvelopeDto {
        equipment_model: equipment_model_aggregate_dto(&aggregate),
    }))
}

pub fn list_equipment_model_revisions(
    storage_root: &Path,
    equipment_model_id: &str,
) -> Result<String, AgentError> {
    validate_id(equipment_model_id, "equipment_model_id")?;
    let connection = open_equipment_connection(storage_root)?;
    require_model_identity(&connection, equipment_model_id)?;
    let revisions = load_model_revision_history(&connection, equipment_model_id)?;
    Ok(render_json(&EquipmentModelRevisionListDto {
        equipment_model_id: equipment_model_id.to_owned(),
        revisions: revisions.iter().map(equipment_model_revision_dto).collect(),
    }))
}

pub fn get_equipment_model_revision(
    storage_root: &Path,
    equipment_model_id: &str,
    revision_id: &str,
) -> Result<String, AgentError> {
    validate_id(equipment_model_id, "equipment_model_id")?;
    validate_id(revision_id, "revision_id")?;
    let connection = open_equipment_connection(storage_root)?;
    let revision = load_equipment_model_revision(&connection, equipment_model_id, revision_id)?
        .ok_or_else(|| {
            AgentError::new(
                "equipment_model_revision_not_found",
                "equipment model revision not found",
            )
        })?;
    Ok(render_json(&EquipmentModelRevisionEnvelopeDto {
        revision: equipment_model_revision_dto(&revision),
    }))
}

pub fn replace_equipment_model_revision_definition(
    storage_root: &Path,
    input: ReplaceEquipmentModelDefinitionInput,
) -> Result<String, AgentError> {
    validate_common_operation(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    validate_id(&input.equipment_model_id, "equipment_model_id")?;
    validate_id(&input.revision_id, "revision_id")?;
    let definition = canonical_equipment_model_definition(&input.definition_json)?;
    let parsed =
        EquipmentModelDefinition::from_json_str(&definition.canonical_json).map_err(|issue| {
            invalid_definition_error("invalid_equipment_model_definition", vec![issue])
        })?;
    let payload_json = render_json(&json!({
        "equipment_model_id": input.equipment_model_id,
        "revision_id": input.revision_id,
        "expected_definition_checksum": input.expected_definition_checksum,
        "new_definition_checksum": definition.definition_checksum
    }));
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    if let Some(operation) = existing_equipment_operation(&connection, &input.operation_id)? {
        ensure_equipment_operation_replay(
            &operation,
            EquipmentOperationFingerprintInput {
                aggregate_kind: "equipment_model",
                entity_id: &input.equipment_model_id,
                revision_id: Some(&input.revision_id),
                action: "equipment_model_definition_replaced",
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: Some(&input.revision_id),
                new_revision_id: Some(&input.revision_id),
                old_definition_checksum: Some(&input.expected_definition_checksum),
                new_definition_checksum: Some(&definition.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return model_operation_replay_result(&connection, &operation);
    }
    let existing =
        load_equipment_model_revision(&connection, &input.equipment_model_id, &input.revision_id)?
            .ok_or_else(|| {
                AgentError::new(
                    "equipment_model_revision_not_found",
                    "equipment model revision not found",
                )
            })?;
    if existing.status != "draft" {
        return Err(AgentError::new(
            "equipment_revision_immutable",
            "only draft equipment model revisions can be edited",
        ));
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    update_equipment_model_revision_definition(
        &transaction,
        UpdateDefinitionInput {
            revision_id: &input.revision_id,
            expected_definition_checksum: &input.expected_definition_checksum,
            definition_schema_version: &definition.definition_schema_version,
            definition_json: &definition.canonical_json,
            definition_checksum: &definition.definition_checksum,
            timestamp: &now,
        },
        UpdateModelDefinitionCounts {
            capability_count: parsed.capabilities.len() as u32,
            interface_count: parsed.communication_interfaces.len() as u32,
            signal_port_count: parsed.signal_ports.len() as u32,
        },
    )?;
    touch_equipment_model_identity(&transaction, &input.equipment_model_id, &now)?;
    write_equipment_model_classification_summary(
        &transaction,
        EquipmentModelClassificationSummaryInput {
            equipment_model_id: &input.equipment_model_id,
            revision_id: &input.revision_id,
            revision_number: existing.revision_number,
            status: revision_status_text(EquipmentRevisionStatus::Draft),
            definition_checksum: &definition.definition_checksum,
            definition: &parsed,
            timestamp: &now,
        },
    )?;
    write_equipment_audit_and_outbox(
        &transaction,
        EquipmentAuditEventInput {
            aggregate_kind: "equipment_model",
            entity_id: &input.equipment_model_id,
            revision_id: Some(&input.revision_id),
            action: "equipment_model_definition_replaced",
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: Some(&input.revision_id),
            new_revision_id: Some(&input.revision_id),
            old_definition_checksum: Some(&input.expected_definition_checksum),
            new_definition_checksum: Some(&definition.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
        EquipmentSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "equipment_model_revision",
            entity_id: &input.revision_id,
            operation_kind: "equipment_model_definition_replaced",
            base_revision: &input.expected_definition_checksum,
            resulting_revision: &definition.definition_checksum,
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    model_operation_result_for_revision(
        &connection,
        "equipment_model_definition_replaced",
        &input.operation_id,
        false,
        &input.equipment_model_id,
        &input.revision_id,
    )
}

pub fn create_equipment_model_revision(
    storage_root: &Path,
    input: CreateEquipmentModelRevisionInput,
) -> Result<String, AgentError> {
    validate_common_operation(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    validate_id(&input.equipment_model_id, "equipment_model_id")?;
    validate_id(&input.source_revision_id, "source_revision_id")?;
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let source = load_equipment_model_revision(
        &connection,
        &input.equipment_model_id,
        &input.source_revision_id,
    )?
    .ok_or_else(|| {
        AgentError::new(
            "equipment_model_revision_not_found",
            "source model revision not found",
        )
    })?;
    if source.status != "approved" {
        return Err(AgentError::new(
            "equipment_revision_source_not_approved",
            "new equipment model revisions must be derived from an approved revision",
        ));
    }
    if let Some(draft) =
        load_active_draft_equipment_model_revision(&connection, &input.equipment_model_id)?
    {
        return Err(AgentError::with_details(
            "equipment_active_draft_exists",
            "equipment model already has an active draft revision",
            json!({ "active_draft_revision_id": draft.revision_id }),
        ));
    }
    let next_number = next_equipment_model_revision_number(&connection, &input.equipment_model_id)?;
    let revision_id = model_revision_id_for(&input.equipment_model_id, next_number);
    let payload_json = render_json(&json!({
        "equipment_model_id": input.equipment_model_id,
        "source_revision_id": input.source_revision_id,
        "new_revision_id": revision_id
    }));
    if let Some(operation) = existing_equipment_operation(&connection, &input.operation_id)? {
        ensure_equipment_operation_replay(
            &operation,
            EquipmentOperationFingerprintInput {
                aggregate_kind: "equipment_model",
                entity_id: &input.equipment_model_id,
                revision_id: Some(&revision_id),
                action: "equipment_model_revision_created",
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: Some(&input.source_revision_id),
                new_revision_id: Some(&revision_id),
                old_definition_checksum: Some(&source.definition_checksum),
                new_definition_checksum: Some(&source.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return model_operation_replay_result(&connection, &operation);
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_equipment_model_revision(
        &transaction,
        NewEquipmentModelRevisionRecord {
            revision_id: &revision_id,
            equipment_model_id: &input.equipment_model_id,
            revision_number: next_number,
            parent_revision_id: Some(&input.source_revision_id),
            status: "draft",
            definition_schema_version: &source.definition_schema_version,
            definition_json: &source.definition_json,
            definition_checksum: &source.definition_checksum,
            created_by: &input.actor,
            timestamp: &now,
            capability_count: source.capability_count,
            interface_count: source.interface_count,
            signal_port_count: source.signal_port_count,
        },
    )?;
    touch_equipment_model_identity(&transaction, &input.equipment_model_id, &now)?;
    let parsed_source =
        EquipmentModelDefinition::from_json_str(&source.definition_json).map_err(|issue| {
            invalid_definition_error("invalid_equipment_model_definition", vec![issue])
        })?;
    write_equipment_model_classification_summary(
        &transaction,
        EquipmentModelClassificationSummaryInput {
            equipment_model_id: &input.equipment_model_id,
            revision_id: &revision_id,
            revision_number: next_number,
            status: revision_status_text(EquipmentRevisionStatus::Draft),
            definition_checksum: &source.definition_checksum,
            definition: &parsed_source,
            timestamp: &now,
        },
    )?;
    write_equipment_audit_and_outbox(
        &transaction,
        EquipmentAuditEventInput {
            aggregate_kind: "equipment_model",
            entity_id: &input.equipment_model_id,
            revision_id: Some(&revision_id),
            action: "equipment_model_revision_created",
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: Some(&input.source_revision_id),
            new_revision_id: Some(&revision_id),
            old_definition_checksum: Some(&source.definition_checksum),
            new_definition_checksum: Some(&source.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
        EquipmentSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "equipment_model_revision",
            entity_id: &revision_id,
            operation_kind: "equipment_model_revision_created",
            base_revision: &input.source_revision_id,
            resulting_revision: &revision_id,
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    model_operation_result_for_revision(
        &connection,
        "equipment_model_revision_created",
        &input.operation_id,
        false,
        &input.equipment_model_id,
        &revision_id,
    )
}

pub fn transition_equipment_model_revision(
    storage_root: &Path,
    input: TransitionEquipmentModelRevisionInput,
) -> Result<String, AgentError> {
    transition_model_revision(storage_root, input)
}

pub fn list_driver_profiles(
    storage_root: &Path,
    input: ListDriverProfilesInput,
) -> Result<String, AgentError> {
    validate_optional_id(input.equipment_model_id.as_deref(), "equipment_model_id")?;
    validate_optional_id(input.status.as_deref(), "status")?;
    let connection = open_equipment_connection(storage_root)?;
    let identities = list_driver_profile_identities(
        &connection,
        DriverProfileListFilter {
            equipment_model_id: input.equipment_model_id.as_deref(),
            status: input.status.as_deref(),
            search: input.search.as_deref(),
        },
    )?;
    let driver_profiles = identities
        .iter()
        .map(|identity| driver_aggregate_for_identity(&connection, identity))
        .collect::<Result<Vec<_>, AgentError>>()?;
    Ok(render_json(&DriverProfileListDto {
        driver_profiles: driver_profiles
            .iter()
            .map(driver_profile_aggregate_dto)
            .collect(),
    }))
}

pub fn create_driver_profile(
    storage_root: &Path,
    input: CreateDriverProfileInput,
) -> Result<String, AgentError> {
    validate_common_operation(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    validate_id(&input.driver_profile_id, "driver_profile_id")?;
    let definition =
        DriverProfileDefinition::from_json_str(&input.definition_json).map_err(|issue| {
            invalid_definition_error("invalid_driver_profile_definition", vec![issue])
        })?;
    let model_definition = approved_model_definition_for_driver(storage_root, &definition)?;
    let canonical = definition
        .canonicalize(Some(&model_definition))
        .map_err(|issues| invalid_definition_error("invalid_driver_profile_definition", issues))?;
    let revision_id = driver_revision_id_for(&input.driver_profile_id, 1);
    let payload_json = render_json(&json!({
        "driver_profile_id": input.driver_profile_id,
        "equipment_model_id": definition.equipment_model_id,
        "supported_model_revision_id": definition.supported_model_revision_id,
        "definition_checksum": canonical.definition_checksum
    }));
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    if let Some(operation) = existing_equipment_operation(&connection, &input.operation_id)? {
        ensure_equipment_operation_replay(
            &operation,
            EquipmentOperationFingerprintInput {
                aggregate_kind: "driver_profile",
                entity_id: &input.driver_profile_id,
                revision_id: Some(&revision_id),
                action: "driver_profile_created",
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: None,
                new_revision_id: Some(&revision_id),
                old_definition_checksum: None,
                new_definition_checksum: Some(&canonical.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return driver_operation_replay_result(&connection, &operation);
    }
    if load_driver_profile_identity(&connection, &input.driver_profile_id)?.is_some() {
        return Err(AgentError::new(
            "driver_profile_already_exists",
            format!("driver profile already exists: {}", input.driver_profile_id),
        ));
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_driver_profile_identity(
        &transaction,
        NewDriverProfileIdentityRecord {
            driver_profile_id: &input.driver_profile_id,
            equipment_model_id: &definition.equipment_model_id,
            label: &input.label,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_driver_profile_revision(
        &transaction,
        NewDriverProfileRevisionRecord {
            revision_id: &revision_id,
            driver_profile_id: &input.driver_profile_id,
            equipment_model_id: &definition.equipment_model_id,
            supported_model_revision_id: &definition.supported_model_revision_id,
            revision_number: 1,
            parent_revision_id: None,
            status: "draft",
            definition_schema_version: &canonical.definition_schema_version,
            definition_json: &canonical.canonical_json,
            definition_checksum: &canonical.definition_checksum,
            created_by: &input.actor,
            timestamp: &now,
            action_count: definition.actions.len() as u32,
        },
    )?;
    write_equipment_audit_and_outbox(
        &transaction,
        EquipmentAuditEventInput {
            aggregate_kind: "driver_profile",
            entity_id: &input.driver_profile_id,
            revision_id: Some(&revision_id),
            action: "driver_profile_created",
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: None,
            new_revision_id: Some(&revision_id),
            old_definition_checksum: None,
            new_definition_checksum: Some(&canonical.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
        EquipmentSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "driver_profile_revision",
            entity_id: &revision_id,
            operation_kind: "driver_profile_created",
            base_revision: "none",
            resulting_revision: &revision_id,
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    driver_operation_result_for_revision(
        &connection,
        "driver_profile_created",
        &input.operation_id,
        false,
        &input.driver_profile_id,
        &revision_id,
    )
}

pub fn get_driver_profile(
    storage_root: &Path,
    driver_profile_id: &str,
) -> Result<String, AgentError> {
    validate_id(driver_profile_id, "driver_profile_id")?;
    let connection = open_equipment_connection(storage_root)?;
    let identity =
        load_driver_profile_identity(&connection, driver_profile_id)?.ok_or_else(|| {
            AgentError::new(
                "driver_profile_not_found",
                format!("driver profile not found: {driver_profile_id}"),
            )
        })?;
    let aggregate = driver_aggregate_for_identity(&connection, &identity)?;
    Ok(render_json(&DriverProfileEnvelopeDto {
        driver_profile: driver_profile_aggregate_dto(&aggregate),
    }))
}

pub fn list_driver_profile_revisions(
    storage_root: &Path,
    driver_profile_id: &str,
) -> Result<String, AgentError> {
    validate_id(driver_profile_id, "driver_profile_id")?;
    let connection = open_equipment_connection(storage_root)?;
    require_driver_identity(&connection, driver_profile_id)?;
    let revisions = load_driver_revision_history(&connection, driver_profile_id)?;
    Ok(render_json(&DriverProfileRevisionListDto {
        driver_profile_id: driver_profile_id.to_owned(),
        revisions: revisions.iter().map(driver_profile_revision_dto).collect(),
    }))
}

pub fn get_driver_profile_revision(
    storage_root: &Path,
    driver_profile_id: &str,
    revision_id: &str,
) -> Result<String, AgentError> {
    validate_id(driver_profile_id, "driver_profile_id")?;
    validate_id(revision_id, "revision_id")?;
    let connection = open_equipment_connection(storage_root)?;
    let revision = load_driver_profile_revision(&connection, driver_profile_id, revision_id)?
        .ok_or_else(|| {
            AgentError::new(
                "driver_profile_revision_not_found",
                "driver profile revision not found",
            )
        })?;
    Ok(render_json(&DriverProfileRevisionEnvelopeDto {
        revision: driver_profile_revision_dto(&revision),
    }))
}

pub fn replace_driver_profile_revision_definition(
    storage_root: &Path,
    input: ReplaceDriverProfileDefinitionInput,
) -> Result<String, AgentError> {
    validate_common_operation(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    let definition =
        DriverProfileDefinition::from_json_str(&input.definition_json).map_err(|issue| {
            invalid_definition_error("invalid_driver_profile_definition", vec![issue])
        })?;
    let model_definition = approved_model_definition_for_driver(storage_root, &definition)?;
    let canonical = definition
        .canonicalize(Some(&model_definition))
        .map_err(|issues| invalid_definition_error("invalid_driver_profile_definition", issues))?;
    let payload_json = render_json(&json!({
        "driver_profile_id": input.driver_profile_id,
        "revision_id": input.revision_id,
        "expected_definition_checksum": input.expected_definition_checksum,
        "new_definition_checksum": canonical.definition_checksum
    }));
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    if let Some(operation) = existing_equipment_operation(&connection, &input.operation_id)? {
        ensure_equipment_operation_replay(
            &operation,
            EquipmentOperationFingerprintInput {
                aggregate_kind: "driver_profile",
                entity_id: &input.driver_profile_id,
                revision_id: Some(&input.revision_id),
                action: "driver_profile_definition_replaced",
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: Some(&input.revision_id),
                new_revision_id: Some(&input.revision_id),
                old_definition_checksum: Some(&input.expected_definition_checksum),
                new_definition_checksum: Some(&canonical.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return driver_operation_replay_result(&connection, &operation);
    }
    let existing =
        load_driver_profile_revision(&connection, &input.driver_profile_id, &input.revision_id)?
            .ok_or_else(|| {
                AgentError::new(
                    "driver_profile_revision_not_found",
                    "driver profile revision not found",
                )
            })?;
    if existing.status != "draft" {
        return Err(AgentError::new(
            "equipment_revision_immutable",
            "only draft driver profile revisions can be edited",
        ));
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    update_driver_profile_revision_definition(
        &transaction,
        UpdateDefinitionInput {
            revision_id: &input.revision_id,
            expected_definition_checksum: &input.expected_definition_checksum,
            definition_schema_version: &canonical.definition_schema_version,
            definition_json: &canonical.canonical_json,
            definition_checksum: &canonical.definition_checksum,
            timestamp: &now,
        },
        UpdateDriverDefinitionCounts {
            action_count: definition.actions.len() as u32,
        },
    )?;
    touch_driver_profile_identity(&transaction, &input.driver_profile_id, &now)?;
    write_equipment_audit_and_outbox(
        &transaction,
        EquipmentAuditEventInput {
            aggregate_kind: "driver_profile",
            entity_id: &input.driver_profile_id,
            revision_id: Some(&input.revision_id),
            action: "driver_profile_definition_replaced",
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: Some(&input.revision_id),
            new_revision_id: Some(&input.revision_id),
            old_definition_checksum: Some(&input.expected_definition_checksum),
            new_definition_checksum: Some(&canonical.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
        EquipmentSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "driver_profile_revision",
            entity_id: &input.revision_id,
            operation_kind: "driver_profile_definition_replaced",
            base_revision: &input.expected_definition_checksum,
            resulting_revision: &canonical.definition_checksum,
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    driver_operation_result_for_revision(
        &connection,
        "driver_profile_definition_replaced",
        &input.operation_id,
        false,
        &input.driver_profile_id,
        &input.revision_id,
    )
}

pub fn create_driver_profile_revision(
    storage_root: &Path,
    input: CreateDriverProfileRevisionInput,
) -> Result<String, AgentError> {
    validate_common_operation(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let identity = require_driver_identity(&connection, &input.driver_profile_id)?;
    let source = load_driver_profile_revision(
        &connection,
        &input.driver_profile_id,
        &input.source_revision_id,
    )?
    .ok_or_else(|| {
        AgentError::new(
            "driver_profile_revision_not_found",
            "source driver profile revision not found",
        )
    })?;
    if source.status != "approved" {
        return Err(AgentError::new(
            "equipment_revision_source_not_approved",
            "new driver profile revisions must be derived from an approved revision",
        ));
    }
    if let Some(draft) =
        load_active_draft_driver_profile_revision(&connection, &input.driver_profile_id)?
    {
        return Err(AgentError::with_details(
            "equipment_active_draft_exists",
            "driver profile already has an active draft revision",
            json!({ "active_draft_revision_id": draft.revision_id }),
        ));
    }
    let next_number = next_driver_profile_revision_number(&connection, &input.driver_profile_id)?;
    let revision_id = driver_revision_id_for(&input.driver_profile_id, next_number);
    let payload_json = render_json(&json!({
        "driver_profile_id": input.driver_profile_id,
        "source_revision_id": input.source_revision_id,
        "new_revision_id": revision_id
    }));
    if let Some(operation) = existing_equipment_operation(&connection, &input.operation_id)? {
        ensure_equipment_operation_replay(
            &operation,
            EquipmentOperationFingerprintInput {
                aggregate_kind: "driver_profile",
                entity_id: &input.driver_profile_id,
                revision_id: Some(&revision_id),
                action: "driver_profile_revision_created",
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: Some(&input.source_revision_id),
                new_revision_id: Some(&revision_id),
                old_definition_checksum: Some(&source.definition_checksum),
                new_definition_checksum: Some(&source.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return driver_operation_replay_result(&connection, &operation);
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_driver_profile_revision(
        &transaction,
        NewDriverProfileRevisionRecord {
            revision_id: &revision_id,
            driver_profile_id: &input.driver_profile_id,
            equipment_model_id: &identity.equipment_model_id,
            supported_model_revision_id: &source.supported_model_revision_id,
            revision_number: next_number,
            parent_revision_id: Some(&input.source_revision_id),
            status: "draft",
            definition_schema_version: &source.definition_schema_version,
            definition_json: &source.definition_json,
            definition_checksum: &source.definition_checksum,
            created_by: &input.actor,
            timestamp: &now,
            action_count: source.action_count,
        },
    )?;
    touch_driver_profile_identity(&transaction, &input.driver_profile_id, &now)?;
    write_equipment_audit_and_outbox(
        &transaction,
        EquipmentAuditEventInput {
            aggregate_kind: "driver_profile",
            entity_id: &input.driver_profile_id,
            revision_id: Some(&revision_id),
            action: "driver_profile_revision_created",
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: Some(&input.source_revision_id),
            new_revision_id: Some(&revision_id),
            old_definition_checksum: Some(&source.definition_checksum),
            new_definition_checksum: Some(&source.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
        EquipmentSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "driver_profile_revision",
            entity_id: &revision_id,
            operation_kind: "driver_profile_revision_created",
            base_revision: &input.source_revision_id,
            resulting_revision: &revision_id,
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    driver_operation_result_for_revision(
        &connection,
        "driver_profile_revision_created",
        &input.operation_id,
        false,
        &input.driver_profile_id,
        &revision_id,
    )
}

pub fn transition_driver_profile_revision(
    storage_root: &Path,
    input: TransitionDriverProfileRevisionInput,
) -> Result<String, AgentError> {
    transition_driver_revision(storage_root, input)
}

pub fn list_equipment_audit_events_for_model(
    storage_root: &Path,
    equipment_model_id: &str,
) -> Result<String, AgentError> {
    validate_id(equipment_model_id, "equipment_model_id")?;
    let connection = open_equipment_connection(storage_root)?;
    require_model_identity(&connection, equipment_model_id)?;
    render_audit(&connection, "equipment_model", equipment_model_id)
}

pub fn list_equipment_audit_events_for_driver(
    storage_root: &Path,
    driver_profile_id: &str,
) -> Result<String, AgentError> {
    validate_id(driver_profile_id, "driver_profile_id")?;
    let connection = open_equipment_connection(storage_root)?;
    require_driver_identity(&connection, driver_profile_id)?;
    render_audit(&connection, "driver_profile", driver_profile_id)
}

pub fn communication_provider_status() -> Result<String, AgentError> {
    Ok(render_json(&CommunicationProviderStatusListDto {
        providers: vec![
            CommunicationProviderStatusDto {
                provider: "simulation".to_owned(),
                available: true,
                reason: None,
            },
            CommunicationProviderStatusDto {
                provider: "native_tcp".to_owned(),
                available: true,
                reason: None,
            },
            CommunicationProviderStatusDto {
                provider: "native_udp".to_owned(),
                available: true,
                reason: None,
            },
            CommunicationProviderStatusDto {
                provider: "native_serial".to_owned(),
                available: true,
                reason: Some("Provider contract available; CI uses fake serial provider, no physical COM port required.".to_owned()),
            },
            CommunicationProviderStatusDto {
                provider: "visa".to_owned(),
                available: false,
                reason: Some("No VISA implementation installed.".to_owned()),
            },
            CommunicationProviderStatusDto {
                provider: "socketcan".to_owned(),
                available: false,
                reason: Some("CAN provider modeled and simulated; no vendor SDK installed.".to_owned()),
            },
            CommunicationProviderStatusDto {
                provider: "pcan".to_owned(),
                available: false,
                reason: Some("PEAK PCAN provider not installed.".to_owned()),
            },
            CommunicationProviderStatusDto {
                provider: "vector_can".to_owned(),
                available: false,
                reason: Some("Vector CAN provider not installed.".to_owned()),
            },
            CommunicationProviderStatusDto {
                provider: "usbtmc".to_owned(),
                available: false,
                reason: Some("USBTMC access is modeled but no provider is installed.".to_owned()),
            },
            CommunicationProviderStatusDto {
                provider: "hid".to_owned(),
                available: false,
                reason: Some("HID access is modeled but no provider is installed.".to_owned()),
            },
        ],
    }))
}

pub fn simulate_driver_profile(
    storage_root: &Path,
    input: SimulateDriverProfileInput,
) -> Result<String, AgentError> {
    validate_id(&input.driver_profile_id, "driver_profile_id")?;
    let connection = open_equipment_connection(storage_root)?;
    let identity = require_driver_identity(&connection, &input.driver_profile_id)?;
    let revision = match input.revision_id.as_deref() {
        Some(revision_id) => {
            load_driver_profile_revision(&connection, &input.driver_profile_id, revision_id)?
        }
        None => load_current_approved_driver_profile_revision(&connection, &identity)?,
    }
    .ok_or_else(|| {
        AgentError::new(
            "driver_profile_revision_not_found",
            "driver profile revision not found",
        )
    })?;
    let definition =
        DriverProfileDefinition::from_json_str(&revision.definition_json).map_err(|issue| {
            invalid_definition_error("invalid_driver_profile_definition", vec![issue])
        })?;
    let scenario: DriverSimulationScenario =
        serde_json::from_str(&input.scenario_json).map_err(|error| {
            AgentError::with_details(
                "invalid_driver_simulation_scenario",
                error.to_string(),
                json!({ "scenario_json": input.scenario_json }),
            )
        })?;
    if scenario.driver_revision_id != revision.revision_id {
        return Err(AgentError::with_details(
            "driver_simulation_revision_mismatch",
            "scenario driver_revision_id does not match selected driver profile revision",
            json!({
                "scenario_driver_revision_id": scenario.driver_revision_id,
                "selected_revision_id": revision.revision_id
            }),
        ));
    }
    if scenario.action_id != input.action_id {
        return Err(AgentError::with_details(
            "driver_simulation_action_mismatch",
            "scenario action_id does not match selected action",
            json!({ "scenario_action_id": scenario.action_id, "selected_action_id": input.action_id }),
        ));
    }
    let result = simulate_driver_action(&definition, &scenario)
        .map_err(|issue| invalid_definition_error("invalid_driver_simulation", vec![issue]))?;
    Ok(render_json(&json!({ "simulation": result })))
}

fn transition_model_revision(
    storage_root: &Path,
    input: TransitionEquipmentModelRevisionInput,
) -> Result<String, AgentError> {
    validate_common_operation(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    let target = input.target_status;
    let (expected, action) = match target {
        EquipmentRevisionStatus::UnderReview => ("draft", "equipment_model_submitted_for_review"),
        EquipmentRevisionStatus::Approved => ("under_review", "equipment_model_approved"),
        _ => {
            return Err(AgentError::new(
                "equipment_revision_transition_not_allowed",
                "only submit-for-review and approve transitions are exposed in this release",
            ))
        }
    };
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let revision =
        load_equipment_model_revision(&connection, &input.equipment_model_id, &input.revision_id)?
            .ok_or_else(|| {
                AgentError::new(
                    "equipment_model_revision_not_found",
                    "equipment model revision not found",
                )
            })?;
    let payload_json = render_json(&json!({
        "equipment_model_id": input.equipment_model_id,
        "revision_id": input.revision_id,
        "target_status": revision_status_text(target)
    }));
    if let Some(operation) = existing_equipment_operation(&connection, &input.operation_id)? {
        ensure_equipment_operation_replay(
            &operation,
            EquipmentOperationFingerprintInput {
                aggregate_kind: "equipment_model",
                entity_id: &input.equipment_model_id,
                revision_id: Some(&input.revision_id),
                action,
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: Some(&input.revision_id),
                new_revision_id: Some(&input.revision_id),
                old_definition_checksum: Some(&revision.definition_checksum),
                new_definition_checksum: Some(&revision.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return model_operation_replay_result(&connection, &operation);
    }
    if revision.status != expected {
        return Err(AgentError::new(
            "equipment_revision_transition_conflict",
            format!("revision is {}, expected {expected}", revision.status),
        ));
    }
    let now = utc_timestamp()?;
    let previous_approved = if target == EquipmentRevisionStatus::Approved {
        let identity = require_model_identity(&connection, &input.equipment_model_id)?;
        identity.current_approved_revision_id
    } else {
        None
    };
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    if let Some(previous) = previous_approved.as_deref() {
        if previous != input.revision_id {
            supersede_approved_equipment_model_revision(
                &transaction,
                &input.equipment_model_id,
                previous,
                &now,
            )?;
        }
    }
    update_equipment_model_revision_status(
        &transaction,
        UpdateStatusInput {
            revision_id: &input.revision_id,
            expected_status: expected,
            status: revision_status_text(target),
            timestamp: &now,
        },
    )?;
    if target == EquipmentRevisionStatus::Approved {
        set_current_approved_equipment_model_revision(
            &transaction,
            &input.equipment_model_id,
            &input.revision_id,
            &now,
        )?;
    } else {
        touch_equipment_model_identity(&transaction, &input.equipment_model_id, &now)?;
    }
    let parsed_definition = EquipmentModelDefinition::from_json_str(&revision.definition_json)
        .map_err(|issue| {
            invalid_definition_error("invalid_equipment_model_definition", vec![issue])
        })?;
    write_equipment_model_classification_summary(
        &transaction,
        EquipmentModelClassificationSummaryInput {
            equipment_model_id: &input.equipment_model_id,
            revision_id: &input.revision_id,
            revision_number: revision.revision_number,
            status: revision_status_text(target),
            definition_checksum: &revision.definition_checksum,
            definition: &parsed_definition,
            timestamp: &now,
        },
    )?;
    write_equipment_audit_and_outbox(
        &transaction,
        EquipmentAuditEventInput {
            aggregate_kind: "equipment_model",
            entity_id: &input.equipment_model_id,
            revision_id: Some(&input.revision_id),
            action,
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: Some(&input.revision_id),
            new_revision_id: Some(&input.revision_id),
            old_definition_checksum: Some(&revision.definition_checksum),
            new_definition_checksum: Some(&revision.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
        EquipmentSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "equipment_model_revision",
            entity_id: &input.revision_id,
            operation_kind: action,
            base_revision: &revision.status,
            resulting_revision: revision_status_text(target),
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    if let Some(previous) = previous_approved.as_deref() {
        if target == EquipmentRevisionStatus::Approved && previous != input.revision_id {
            let supersede_operation_id = format!("{}:supersede:{previous}", input.operation_id);
            let supersede_payload = render_json(&json!({
                "equipment_model_id": input.equipment_model_id,
                "superseded_revision_id": previous,
                "approved_revision_id": input.revision_id
            }));
            write_equipment_audit_and_outbox(
                &transaction,
                EquipmentAuditEventInput {
                    aggregate_kind: "equipment_model",
                    entity_id: &input.equipment_model_id,
                    revision_id: Some(previous),
                    action: "equipment_model_revision_superseded",
                    actor: &input.actor,
                    reason: &input.reason,
                    operation_id: &supersede_operation_id,
                    correlation_id: &input.correlation_id,
                    device_id: &input.device_id,
                    old_revision_id: Some(previous),
                    new_revision_id: Some(&input.revision_id),
                    old_definition_checksum: None,
                    new_definition_checksum: Some(&revision.definition_checksum),
                    payload_json: &supersede_payload,
                    timestamp: &now,
                },
                EquipmentSyncOperationInput {
                    operation_id: &supersede_operation_id,
                    entity_type: "equipment_model_revision",
                    entity_id: previous,
                    operation_kind: "equipment_model_revision_superseded",
                    base_revision: previous,
                    resulting_revision: &input.revision_id,
                    actor_id: &input.actor,
                    device_id: &input.device_id,
                    correlation_id: &input.correlation_id,
                    payload_json: &supersede_payload,
                    timestamp: &now,
                },
            )?;
        }
    }
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    model_operation_result_for_revision(
        &connection,
        action,
        &input.operation_id,
        false,
        &input.equipment_model_id,
        &input.revision_id,
    )
}

fn transition_driver_revision(
    storage_root: &Path,
    input: TransitionDriverProfileRevisionInput,
) -> Result<String, AgentError> {
    validate_common_operation(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    let target = input.target_status;
    let (expected, action) = match target {
        EquipmentRevisionStatus::UnderReview => ("draft", "driver_profile_submitted_for_review"),
        EquipmentRevisionStatus::Approved => ("under_review", "driver_profile_approved"),
        _ => {
            return Err(AgentError::new(
                "equipment_revision_transition_not_allowed",
                "only submit-for-review and approve transitions are exposed in this release",
            ))
        }
    };
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let revision =
        load_driver_profile_revision(&connection, &input.driver_profile_id, &input.revision_id)?
            .ok_or_else(|| {
                AgentError::new(
                    "driver_profile_revision_not_found",
                    "driver profile revision not found",
                )
            })?;
    let payload_json = render_json(&json!({
        "driver_profile_id": input.driver_profile_id,
        "revision_id": input.revision_id,
        "target_status": revision_status_text(target)
    }));
    if let Some(operation) = existing_equipment_operation(&connection, &input.operation_id)? {
        ensure_equipment_operation_replay(
            &operation,
            EquipmentOperationFingerprintInput {
                aggregate_kind: "driver_profile",
                entity_id: &input.driver_profile_id,
                revision_id: Some(&input.revision_id),
                action,
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: Some(&input.revision_id),
                new_revision_id: Some(&input.revision_id),
                old_definition_checksum: Some(&revision.definition_checksum),
                new_definition_checksum: Some(&revision.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return driver_operation_replay_result(&connection, &operation);
    }
    if revision.status != expected {
        return Err(AgentError::new(
            "equipment_revision_transition_conflict",
            format!("revision is {}, expected {expected}", revision.status),
        ));
    }
    let now = utc_timestamp()?;
    let previous_approved = if target == EquipmentRevisionStatus::Approved {
        let identity = require_driver_identity(&connection, &input.driver_profile_id)?;
        identity.current_approved_revision_id
    } else {
        None
    };
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    if let Some(previous) = previous_approved.as_deref() {
        if previous != input.revision_id {
            supersede_approved_driver_profile_revision(
                &transaction,
                &input.driver_profile_id,
                previous,
                &now,
            )?;
        }
    }
    update_driver_profile_revision_status(
        &transaction,
        UpdateStatusInput {
            revision_id: &input.revision_id,
            expected_status: expected,
            status: revision_status_text(target),
            timestamp: &now,
        },
    )?;
    if target == EquipmentRevisionStatus::Approved {
        set_current_approved_driver_profile_revision(
            &transaction,
            &input.driver_profile_id,
            &input.revision_id,
            &now,
        )?;
    } else {
        touch_driver_profile_identity(&transaction, &input.driver_profile_id, &now)?;
    }
    write_equipment_audit_and_outbox(
        &transaction,
        EquipmentAuditEventInput {
            aggregate_kind: "driver_profile",
            entity_id: &input.driver_profile_id,
            revision_id: Some(&input.revision_id),
            action,
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: Some(&input.revision_id),
            new_revision_id: Some(&input.revision_id),
            old_definition_checksum: Some(&revision.definition_checksum),
            new_definition_checksum: Some(&revision.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
        EquipmentSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "driver_profile_revision",
            entity_id: &input.revision_id,
            operation_kind: action,
            base_revision: &revision.status,
            resulting_revision: revision_status_text(target),
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    driver_operation_result_for_revision(
        &connection,
        action,
        &input.operation_id,
        false,
        &input.driver_profile_id,
        &input.revision_id,
    )
}

fn approved_model_definition_for_driver(
    storage_root: &Path,
    definition: &DriverProfileDefinition,
) -> Result<EquipmentModelDefinition, AgentError> {
    let connection = open_equipment_connection(storage_root)?;
    let revision = load_equipment_model_revision(
        &connection,
        &definition.equipment_model_id,
        &definition.supported_model_revision_id,
    )?
    .ok_or_else(|| {
        AgentError::new(
            "equipment_model_revision_not_found",
            "driver profile references an unknown equipment model revision",
        )
    })?;
    if revision.status != "approved" {
        return Err(AgentError::new(
            "driver_model_revision_not_approved",
            "driver profile must reference an approved equipment model revision",
        ));
    }
    if revision.definition_checksum != definition.supported_model_definition_checksum {
        return Err(AgentError::new(
            "driver_model_definition_checksum_mismatch",
            "driver profile supported_model_definition_checksum does not match the model revision",
        ));
    }
    EquipmentModelDefinition::from_json_str(&revision.definition_json).map_err(|issue| {
        invalid_definition_error("invalid_equipment_model_definition", vec![issue])
    })
}

fn canonical_equipment_model_definition(
    definition_json: &str,
) -> Result<emc_locus_core::equipment::CanonicalEquipmentDefinition, AgentError> {
    let definition = EquipmentModelDefinition::from_json_str(definition_json).map_err(|issue| {
        invalid_definition_error("invalid_equipment_model_definition", vec![issue])
    })?;
    definition
        .canonicalize()
        .map_err(|issues| invalid_definition_error("invalid_equipment_model_definition", issues))
}

fn render_definition_issues(issues: Vec<DefinitionValidationIssue>) -> String {
    render_json(&EquipmentDefinitionValidationDto {
        valid: false,
        issues: issues
            .into_iter()
            .map(|issue| EquipmentDefinitionValidationIssueDto {
                severity: issue.severity,
                code: issue.code,
                path: issue.path,
                message: issue.message,
                suggestion: issue.suggestion,
            })
            .collect(),
        definition_schema_version: None,
        definition_checksum: None,
        canonical_json: None,
    })
}

fn invalid_definition_error(
    code: &'static str,
    issues: Vec<DefinitionValidationIssue>,
) -> AgentError {
    AgentError::with_details(
        code,
        "definition validation failed",
        json!({
            "issues": issues
        }),
    )
}

struct StoredModelAggregate {
    identity: StoredEquipmentModelIdentity,
    current_approved_revision: Option<StoredEquipmentModelRevision>,
    latest_revision: Option<StoredEquipmentModelRevision>,
    active_draft_revision: Option<StoredEquipmentModelRevision>,
}

struct StoredDriverAggregate {
    identity: StoredDriverProfileIdentity,
    current_approved_revision: Option<StoredDriverProfileRevision>,
    latest_revision: Option<StoredDriverProfileRevision>,
    active_draft_revision: Option<StoredDriverProfileRevision>,
}

fn model_aggregate_for_identity(
    connection: &rusqlite::Connection,
    identity: &StoredEquipmentModelIdentity,
) -> Result<StoredModelAggregate, AgentError> {
    Ok(StoredModelAggregate {
        identity: identity.clone(),
        current_approved_revision: load_current_approved_equipment_model_revision(
            connection, identity,
        )?,
        latest_revision: load_latest_equipment_model_revision(
            connection,
            &identity.equipment_model_id,
        )?,
        active_draft_revision: load_active_draft_equipment_model_revision(
            connection,
            &identity.equipment_model_id,
        )?,
    })
}

fn driver_aggregate_for_identity(
    connection: &rusqlite::Connection,
    identity: &StoredDriverProfileIdentity,
) -> Result<StoredDriverAggregate, AgentError> {
    Ok(StoredDriverAggregate {
        identity: identity.clone(),
        current_approved_revision: load_current_approved_driver_profile_revision(
            connection, identity,
        )?,
        latest_revision: load_latest_driver_profile_revision(
            connection,
            &identity.driver_profile_id,
        )?,
        active_draft_revision: load_active_draft_driver_profile_revision(
            connection,
            &identity.driver_profile_id,
        )?,
    })
}

fn model_operation_result_for_revision(
    connection: &rusqlite::Connection,
    operation: &str,
    operation_id: &str,
    replayed: bool,
    equipment_model_id: &str,
    revision_id: &str,
) -> Result<String, AgentError> {
    let identity = require_model_identity(connection, equipment_model_id)?;
    let aggregate = model_aggregate_for_identity(connection, &identity)?;
    let revision = load_equipment_model_revision(connection, equipment_model_id, revision_id)?
        .ok_or_else(|| {
            AgentError::new(
                "equipment_model_revision_not_found",
                "equipment model revision not found",
            )
        })?;
    Ok(render_json(&EquipmentOperationResultDto {
        operation: operation.to_owned(),
        operation_id: operation_id.to_owned(),
        replayed,
        aggregate: equipment_model_aggregate_dto(&aggregate),
        revision: equipment_model_revision_dto(&revision),
    }))
}

fn driver_operation_result_for_revision(
    connection: &rusqlite::Connection,
    operation: &str,
    operation_id: &str,
    replayed: bool,
    driver_profile_id: &str,
    revision_id: &str,
) -> Result<String, AgentError> {
    let identity = require_driver_identity(connection, driver_profile_id)?;
    let aggregate = driver_aggregate_for_identity(connection, &identity)?;
    let revision = load_driver_profile_revision(connection, driver_profile_id, revision_id)?
        .ok_or_else(|| {
            AgentError::new(
                "driver_profile_revision_not_found",
                "driver profile revision not found",
            )
        })?;
    Ok(render_json(&EquipmentOperationResultDto {
        operation: operation.to_owned(),
        operation_id: operation_id.to_owned(),
        replayed,
        aggregate: driver_profile_aggregate_dto(&aggregate),
        revision: driver_profile_revision_dto(&revision),
    }))
}

fn model_operation_replay_result(
    connection: &rusqlite::Connection,
    operation: &StoredEquipmentOperation,
) -> Result<String, AgentError> {
    let revision_id = operation.revision_id.as_deref().ok_or_else(|| {
        AgentError::new(
            "equipment_audit_query_failed",
            "operation replay has no revision id",
        )
    })?;
    model_operation_result_for_revision(
        connection,
        &operation.action,
        &operation.operation_id,
        true,
        &operation.entity_id,
        revision_id,
    )
}

fn driver_operation_replay_result(
    connection: &rusqlite::Connection,
    operation: &StoredEquipmentOperation,
) -> Result<String, AgentError> {
    let revision_id = operation.revision_id.as_deref().ok_or_else(|| {
        AgentError::new(
            "equipment_audit_query_failed",
            "operation replay has no revision id",
        )
    })?;
    driver_operation_result_for_revision(
        connection,
        &operation.action,
        &operation.operation_id,
        true,
        &operation.entity_id,
        revision_id,
    )
}

fn write_equipment_audit_and_outbox(
    transaction: &rusqlite::Transaction<'_>,
    audit: EquipmentAuditEventInput<'_>,
    outbox: EquipmentSyncOperationInput<'_>,
) -> Result<(), AgentError> {
    insert_equipment_audit_event(transaction, audit)?;
    insert_equipment_sync_operation(transaction, outbox)?;
    Ok(())
}

struct EquipmentModelClassificationSummaryInput<'a> {
    equipment_model_id: &'a str,
    revision_id: &'a str,
    revision_number: u32,
    status: &'a str,
    definition_checksum: &'a str,
    definition: &'a EquipmentModelDefinition,
    timestamp: &'a str,
}

fn write_equipment_model_classification_summary(
    transaction: &rusqlite::Transaction<'_>,
    input: EquipmentModelClassificationSummaryInput<'_>,
) -> Result<(), AgentError> {
    let mut signal_domains = input
        .definition
        .signal_domains
        .iter()
        .copied()
        .map(signal_domain_text)
        .collect::<Vec<_>>();
    signal_domains.sort();
    signal_domains.dedup();
    let mut technology_tags = input
        .definition
        .technology_tags
        .iter()
        .copied()
        .map(technology_tag_text)
        .collect::<Vec<_>>();
    technology_tags.sort();
    technology_tags.dedup();
    let signal_domains_json = render_json(&signal_domains);
    let technology_tags_json = render_json(&technology_tags);
    let functional_role = functional_role_text(input.definition.functional_role);
    replace_equipment_model_classification_summary(
        transaction,
        EquipmentClassificationSummaryRecord {
            equipment_model_id: input.equipment_model_id,
            revision_id: input.revision_id,
            revision_number: input.revision_number,
            status: input.status,
            manufacturer: input.definition.manufacturer.trim(),
            equipment_class: equipment_class_text(input.definition.equipment_class),
            category_code: &input.definition.category_code,
            functional_role: &functional_role,
            definition_checksum: input.definition_checksum,
            signal_domains_json: &signal_domains_json,
            technology_tags_json: &technology_tags_json,
            signal_domains: &signal_domains,
            technology_tags: &technology_tags,
            timestamp: input.timestamp,
        },
    )
}

fn require_model_identity(
    connection: &rusqlite::Connection,
    equipment_model_id: &str,
) -> Result<StoredEquipmentModelIdentity, AgentError> {
    load_equipment_model_identity(connection, equipment_model_id)?.ok_or_else(|| {
        AgentError::new(
            "equipment_model_not_found",
            format!("equipment model not found: {equipment_model_id}"),
        )
    })
}

fn require_driver_identity(
    connection: &rusqlite::Connection,
    driver_profile_id: &str,
) -> Result<StoredDriverProfileIdentity, AgentError> {
    load_driver_profile_identity(connection, driver_profile_id)?.ok_or_else(|| {
        AgentError::new(
            "driver_profile_not_found",
            format!("driver profile not found: {driver_profile_id}"),
        )
    })
}

fn render_audit(
    connection: &rusqlite::Connection,
    aggregate_kind: &str,
    entity_id: &str,
) -> Result<String, AgentError> {
    let events = load_equipment_audit_events(connection, aggregate_kind, entity_id)?;
    Ok(render_json(&EquipmentAuditEventsDto {
        aggregate_kind: aggregate_kind.to_owned(),
        entity_id: entity_id.to_owned(),
        audit_events: events.iter().map(audit_event_dto).collect(),
    }))
}

fn equipment_model_aggregate_dto(aggregate: &StoredModelAggregate) -> EquipmentModelAggregateDto {
    EquipmentModelAggregateDto {
        identity: equipment_model_identity_dto(&aggregate.identity),
        current_approved_revision: aggregate
            .current_approved_revision
            .as_ref()
            .map(equipment_model_revision_dto),
        latest_revision: aggregate
            .latest_revision
            .as_ref()
            .map(equipment_model_revision_dto),
        active_draft_revision: aggregate
            .active_draft_revision
            .as_ref()
            .map(equipment_model_revision_dto),
    }
}

fn equipment_model_identity_dto(
    identity: &StoredEquipmentModelIdentity,
) -> EquipmentModelIdentityDto {
    EquipmentModelIdentityDto {
        equipment_model_id: identity.equipment_model_id.clone(),
        manufacturer: identity.manufacturer.clone(),
        model_name: identity.model_name.clone(),
        variant: identity.variant.clone(),
        equipment_class: identity.equipment_class.clone(),
        category_code: identity.category_code.clone(),
        current_approved_revision_id: identity.current_approved_revision_id.clone(),
        created_by: identity.created_by.clone(),
        created_at: identity.created_at.clone(),
        updated_at: identity.updated_at.clone(),
    }
}

fn equipment_model_revision_dto(
    revision: &StoredEquipmentModelRevision,
) -> EquipmentModelRevisionDto {
    EquipmentModelRevisionDto {
        revision_id: revision.revision_id.clone(),
        equipment_model_id: revision.equipment_model_id.clone(),
        revision_number: revision.revision_number,
        parent_revision_id: revision.parent_revision_id.clone(),
        status: revision.status.clone(),
        definition_schema_version: revision.definition_schema_version.clone(),
        definition: parse_json_value(&revision.definition_json),
        definition_checksum: revision.definition_checksum.clone(),
        created_by: revision.created_by.clone(),
        created_at: revision.created_at.clone(),
        updated_at: revision.updated_at.clone(),
        submitted_at: revision.submitted_at.clone(),
        approved_at: revision.approved_at.clone(),
        capability_count: revision.capability_count,
        interface_count: revision.interface_count,
        signal_port_count: revision.signal_port_count,
    }
}

fn driver_profile_aggregate_dto(aggregate: &StoredDriverAggregate) -> DriverProfileAggregateDto {
    DriverProfileAggregateDto {
        identity: driver_profile_identity_dto(&aggregate.identity),
        current_approved_revision: aggregate
            .current_approved_revision
            .as_ref()
            .map(driver_profile_revision_dto),
        latest_revision: aggregate
            .latest_revision
            .as_ref()
            .map(driver_profile_revision_dto),
        active_draft_revision: aggregate
            .active_draft_revision
            .as_ref()
            .map(driver_profile_revision_dto),
    }
}

fn driver_profile_identity_dto(identity: &StoredDriverProfileIdentity) -> DriverProfileIdentityDto {
    DriverProfileIdentityDto {
        driver_profile_id: identity.driver_profile_id.clone(),
        equipment_model_id: identity.equipment_model_id.clone(),
        label: identity.label.clone(),
        current_approved_revision_id: identity.current_approved_revision_id.clone(),
        created_by: identity.created_by.clone(),
        created_at: identity.created_at.clone(),
        updated_at: identity.updated_at.clone(),
    }
}

fn driver_profile_revision_dto(revision: &StoredDriverProfileRevision) -> DriverProfileRevisionDto {
    DriverProfileRevisionDto {
        revision_id: revision.revision_id.clone(),
        driver_profile_id: revision.driver_profile_id.clone(),
        equipment_model_id: revision.equipment_model_id.clone(),
        supported_model_revision_id: revision.supported_model_revision_id.clone(),
        revision_number: revision.revision_number,
        parent_revision_id: revision.parent_revision_id.clone(),
        status: revision.status.clone(),
        definition_schema_version: revision.definition_schema_version.clone(),
        definition: parse_json_value(&revision.definition_json),
        definition_checksum: revision.definition_checksum.clone(),
        created_by: revision.created_by.clone(),
        created_at: revision.created_at.clone(),
        updated_at: revision.updated_at.clone(),
        submitted_at: revision.submitted_at.clone(),
        approved_at: revision.approved_at.clone(),
        action_count: revision.action_count,
    }
}

fn audit_event_dto(event: &StoredEquipmentAuditEvent) -> EquipmentAuditEventDto {
    EquipmentAuditEventDto {
        audit_id: event.audit_id,
        aggregate_kind: event.aggregate_kind.clone(),
        entity_id: event.entity_id.clone(),
        revision_id: event.revision_id.clone(),
        action: event.action.clone(),
        actor: event.actor.clone(),
        reason: event.reason.clone(),
        old_revision_id: event.old_revision_id.clone(),
        new_revision_id: event.new_revision_id.clone(),
        old_definition_checksum: event.old_definition_checksum.clone(),
        new_definition_checksum: event.new_definition_checksum.clone(),
        operation_id: event.operation_id.clone(),
        correlation_id: event.correlation_id.clone(),
        device_id: event.device_id.clone(),
        payload_json: event.payload_json.clone(),
        occurred_at: event.occurred_at.clone(),
    }
}

fn registry_item_dto(item: &StoredEquipmentRegistryItem) -> EquipmentRegistryItemDto {
    EquipmentRegistryItemDto {
        code: item.code.clone(),
        label: item.label.clone(),
        description: item.description.clone(),
        recommended_equipment_classes: item
            .recommended_equipment_classes
            .as_deref()
            .and_then(|value| parse_json_string_array(value, "recommended_equipment_classes").ok())
            .unwrap_or_default(),
        recommended_functional_roles: item
            .recommended_functional_roles
            .as_deref()
            .and_then(|value| parse_json_string_array(value, "recommended_functional_roles").ok())
            .unwrap_or_default(),
        compatible_signal_domains: item
            .compatible_signal_domains
            .as_deref()
            .and_then(|value| parse_json_string_array(value, "compatible_signal_domains").ok())
            .unwrap_or_default(),
        compatible_directionalities: item
            .compatible_directionalities
            .as_deref()
            .and_then(|value| parse_json_string_array(value, "compatible_directionalities").ok())
            .unwrap_or_default(),
        deprecated: item.deprecated,
    }
}

fn classification_preset_dto(
    connection: &rusqlite::Connection,
    preset: &StoredEquipmentClassificationPreset,
) -> Result<EquipmentClassificationPresetDto, AgentError> {
    let ports = list_equipment_classification_preset_ports(connection, &preset.preset_id)?;
    Ok(EquipmentClassificationPresetDto {
        preset_id: preset.preset_id.clone(),
        category_label: preset.category_label.clone(),
        function_description: preset.function_description.clone(),
        example_label: preset.example_label.clone(),
        default_equipment_class: preset.default_equipment_class.clone(),
        default_functional_role: preset.default_functional_role.clone(),
        default_signal_domains: parse_json_string_array(
            &preset.default_signal_domains,
            "default_signal_domains",
        )?,
        default_technology_tags: parse_json_string_array(
            &preset.default_technology_tags,
            "default_technology_tags",
        )?,
        notes: preset.notes.clone(),
        deprecated: preset.deprecated,
        ports: ports.iter().map(classification_preset_port_dto).collect(),
    })
}

fn classification_preset_port_dto(
    port: &StoredEquipmentClassificationPresetPort,
) -> EquipmentClassificationPresetPortDto {
    EquipmentClassificationPresetPortDto {
        port_order: port.port_order,
        port_id: port.port_id.clone(),
        label: port.label.clone(),
        directionality: port.directionality.clone(),
        flow_role: port.flow_role.clone(),
        signal_domain: port.signal_domain.clone(),
        connector_type: port.connector_type.clone(),
        technology_tags: parse_json_string_array(&port.technology_tags, "technology_tags")
            .unwrap_or_default(),
        quantity: port.quantity.clone(),
        unit: port.unit.clone(),
        impedance: port.impedance,
        frequency_min: port.frequency_min,
        frequency_max: port.frequency_max,
        voltage_max: port.voltage_max,
        current_max: port.current_max,
        power_max: port.power_max,
        required: port.required,
        comment: port.comment.clone(),
    }
}

fn equipment_model_definition_from_preset(
    preset: &StoredEquipmentClassificationPreset,
    ports: &[StoredEquipmentClassificationPresetPort],
    input: &CreateEquipmentModelFromPresetInput,
) -> Result<EquipmentModelDefinition, AgentError> {
    let equipment_class: EquipmentClass =
        parse_enum_code(&preset.default_equipment_class, "default_equipment_class")?;
    let functional_role: FunctionalRole =
        parse_enum_code(&preset.default_functional_role, "default_functional_role")?;
    let signal_domains =
        parse_json_enum_array(&preset.default_signal_domains, "default_signal_domains")?;
    let technology_tags =
        parse_json_enum_array(&preset.default_technology_tags, "default_technology_tags")?;
    let signal_ports = ports
        .iter()
        .map(signal_port_from_preset_port)
        .collect::<Result<Vec<_>, AgentError>>()?;
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "classification_preset_id".to_owned(),
        Value::String(preset.preset_id.clone()),
    );
    metadata.insert(
        "classification_preset_label".to_owned(),
        Value::String(preset.example_label.clone()),
    );
    metadata.insert(
        "classification_preset_category".to_owned(),
        Value::String(preset.category_label.clone()),
    );
    metadata.insert(
        "classification_notes".to_owned(),
        Value::String(preset.notes.clone()),
    );
    Ok(EquipmentModelDefinition {
        definition_schema_version: EQUIPMENT_MODEL_DEFINITION_SCHEMA_VERSION.to_owned(),
        manufacturer: input.manufacturer.trim().to_owned(),
        model_name: input.model_name.trim().to_owned(),
        variant: input
            .variant
            .as_ref()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()),
        equipment_class,
        functional_role,
        category_code: preset.preset_id.clone(),
        signal_domains,
        technology_tags,
        specifications: Vec::new(),
        signal_ports,
        communication_interfaces: Vec::new(),
        capabilities: Vec::new(),
        metadata,
    })
}

fn signal_port_from_preset_port(
    port: &StoredEquipmentClassificationPresetPort,
) -> Result<SignalPortDefinition, AgentError> {
    Ok(SignalPortDefinition {
        port_id: port.port_id.clone(),
        label: port.label.clone(),
        directionality: parse_enum_code(&port.directionality, "directionality")?,
        flow_role: parse_enum_code(&port.flow_role, "flow_role")?,
        signal_domain: parse_enum_code(&port.signal_domain, "signal_domain")?,
        required: port.required,
        connector_type: port.connector_type.clone(),
        technology_tags: parse_json_enum_array(&port.technology_tags, "technology_tags")?,
        quantity: parse_enum_code::<PhysicalQuantity>(&port.quantity, "quantity")?,
        unit: port.unit.clone(),
        impedance: port.impedance,
        frequency_min: port.frequency_min,
        frequency_max: port.frequency_max,
        voltage_max: port.voltage_max,
        current_max: port.current_max,
        power_max: port.power_max,
        channel_index: None,
        differential: false,
        isolated: false,
        comment: port.comment.clone(),
    })
}

fn parse_json_value(value: &str) -> Value {
    serde_json::from_str(value).unwrap_or_else(|_| json!({ "raw": value }))
}

fn revision_status_text(status: EquipmentRevisionStatus) -> &'static str {
    match status {
        EquipmentRevisionStatus::Draft => "draft",
        EquipmentRevisionStatus::UnderReview => "under_review",
        EquipmentRevisionStatus::Approved => "approved",
        EquipmentRevisionStatus::Superseded => "superseded",
        EquipmentRevisionStatus::Suspended => "suspended",
        EquipmentRevisionStatus::Retired => "retired",
    }
}

fn equipment_class_text(class: emc_locus_core::equipment::EquipmentClass) -> &'static str {
    match class {
        emc_locus_core::equipment::EquipmentClass::ControllableInstrument => {
            "controllable_instrument"
        }
        emc_locus_core::equipment::EquipmentClass::DaqDevice => "daq_device",
        emc_locus_core::equipment::EquipmentClass::AcquisitionDevice => "acquisition_device",
        emc_locus_core::equipment::EquipmentClass::Converter => "converter",
        emc_locus_core::equipment::EquipmentClass::Sensor => "sensor",
        emc_locus_core::equipment::EquipmentClass::Transducer => "transducer",
        emc_locus_core::equipment::EquipmentClass::PassiveComponent => "passive_component",
        emc_locus_core::equipment::EquipmentClass::SwitchingDevice => "switching_device",
        emc_locus_core::equipment::EquipmentClass::MotionSystem => "motion_system",
        emc_locus_core::equipment::EquipmentClass::Facility => "facility",
        emc_locus_core::equipment::EquipmentClass::SoftwareAdapter => "software_adapter",
        emc_locus_core::equipment::EquipmentClass::ManualEquipment => "manual_equipment",
    }
}

fn functional_role_text(role: FunctionalRole) -> String {
    enum_code(role)
}

fn signal_domain_text(domain: SignalDomain) -> String {
    enum_code(domain)
}

fn technology_tag_text(tag: TechnologyTag) -> String {
    enum_code(tag)
}

fn enum_code<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn parse_enum_code<T: DeserializeOwned>(code: &str, field: &'static str) -> Result<T, AgentError> {
    serde_json::from_value(Value::String(code.to_owned())).map_err(|error| {
        AgentError::with_details(
            "invalid_equipment_registry_value",
            format!("{field} contains a value that is not supported by emc-locus-core"),
            json!({ "field": field, "value": code, "reason": error.to_string() }),
        )
    })
}

fn parse_json_string_array(value: &str, field: &'static str) -> Result<Vec<String>, AgentError> {
    serde_json::from_str::<Vec<String>>(value).map_err(|error| {
        AgentError::with_details(
            "invalid_equipment_registry_json",
            format!("{field} must be a JSON array of strings"),
            json!({ "field": field, "reason": error.to_string() }),
        )
    })
}

fn parse_json_enum_array<T: DeserializeOwned>(
    value: &str,
    field: &'static str,
) -> Result<Vec<T>, AgentError> {
    parse_json_string_array(value, field)?
        .iter()
        .map(|code| parse_enum_code(code, field))
        .collect()
}

fn model_revision_id_for(equipment_model_id: &str, revision_number: u32) -> String {
    format!("{equipment_model_id}-rev-{revision_number:04}")
}

fn driver_revision_id_for(driver_profile_id: &str, revision_number: u32) -> String {
    format!("{driver_profile_id}-rev-{revision_number:04}")
}

fn validate_common_operation(
    actor: &str,
    reason: &str,
    operation_id: &str,
    correlation_id: &str,
    device_id: &str,
) -> Result<(), AgentError> {
    validate_id(operation_id, "operation_id")?;
    validate_id(correlation_id, "correlation_id")?;
    validate_id(device_id, "device_id")?;
    if actor.trim().is_empty() {
        return Err(AgentError::new("invalid_actor", "actor is required"));
    }
    if reason.trim().is_empty() {
        return Err(AgentError::new("invalid_reason", "reason is required"));
    }
    Ok(())
}

fn validate_optional_id(value: Option<&str>, field: &'static str) -> Result<(), AgentError> {
    if let Some(value) = value {
        validate_id(value, field)?;
    }
    Ok(())
}

fn validate_id(value: &str, field: &'static str) -> Result<(), AgentError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(AgentError::with_details(
            "invalid_equipment_identifier",
            format!("{field} contains unsupported characters"),
            json!({ "field": field, "value": value }),
        ));
    }
    Ok(())
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_failed", error.to_string()))
}
