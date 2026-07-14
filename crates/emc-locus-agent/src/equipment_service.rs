use crate::{
    equipment_dto::{
        CommunicationProviderStatusDto, CommunicationProviderStatusListDto,
        DriverProfileAggregateDto, DriverProfileEnvelopeDto, DriverProfileIdentityDto,
        DriverProfileListDto, DriverProfileRevisionDto, DriverProfileRevisionEnvelopeDto,
        DriverProfileRevisionListDto, EquipmentAuditEventDto, EquipmentAuditEventsDto,
        EquipmentCategoryDto, EquipmentCategoryEnvelopeDto, EquipmentCategoryFieldRuleDto,
        EquipmentCategoryFieldRuleListDto, EquipmentCategoryListDto,
        EquipmentClassificationPresetDto, EquipmentClassificationPresetEnvelopeDto,
        EquipmentClassificationPresetListDto, EquipmentClassificationPresetPortDto,
        EquipmentDefinitionValidationDto, EquipmentDefinitionValidationIssueDto,
        EquipmentEffectiveFieldDto, EquipmentEffectiveTemplateDto,
        EquipmentEffectiveTemplateEnvelopeDto, EquipmentFieldDefinitionDto,
        EquipmentFieldDefinitionEnvelopeDto, EquipmentFieldDefinitionListDto,
        EquipmentModelAggregateDto, EquipmentModelEnvelopeDto, EquipmentModelIdentityDto,
        EquipmentModelListDto, EquipmentModelRevisionDto, EquipmentModelRevisionEnvelopeDto,
        EquipmentModelRevisionListDto, EquipmentOperationResultDto, EquipmentRegistriesDto,
        EquipmentRegistryItemDto,
    },
    equipment_repository::{
        archive_equipment_category, archive_equipment_field_definition,
        count_equipment_models_in_category, ensure_equipment_operation_replay,
        equipment_model_class_exists, existing_equipment_operation, insert_driver_profile_identity,
        insert_driver_profile_revision, insert_equipment_audit_event, insert_equipment_category,
        insert_equipment_field_definition, insert_equipment_model_identity,
        insert_equipment_model_revision, insert_equipment_sync_operation,
        list_driver_profile_identities,
        list_driver_profile_revisions as load_driver_revision_history, list_equipment_categories,
        list_equipment_category_field_rules, list_equipment_classification_preset_ports,
        list_equipment_classification_presets, list_equipment_field_definitions,
        list_equipment_flow_role_registry, list_equipment_functional_role_registry,
        list_equipment_model_identities,
        list_equipment_model_revisions as load_model_revision_history,
        list_equipment_port_directionality_registry, list_equipment_signal_domain_registry,
        list_equipment_technology_tag_registry, load_active_draft_driver_profile_revision,
        load_active_draft_equipment_model_revision, load_current_approved_driver_profile_revision,
        load_current_approved_equipment_model_revision, load_driver_profile_identity,
        load_driver_profile_revision, load_equipment_audit_events, load_equipment_category,
        load_equipment_classification_preset, load_equipment_field_definition,
        load_equipment_field_definition_by_code, load_equipment_model_identity,
        load_equipment_model_revision, load_latest_driver_profile_revision,
        load_latest_equipment_model_revision, move_equipment_category,
        next_driver_profile_revision_number, next_equipment_model_revision_number,
        open_equipment_connection, open_equipment_connection_with_sync,
        replace_equipment_category_field_rules, replace_equipment_model_classification_summary,
        replace_equipment_model_template_snapshot, set_current_approved_driver_profile_revision,
        set_current_approved_equipment_model_revision, supersede_approved_driver_profile_revision,
        supersede_approved_equipment_model_revision, touch_driver_profile_identity,
        touch_equipment_model_identity, update_driver_profile_revision_definition,
        update_driver_profile_revision_status, update_equipment_category,
        update_equipment_field_definition, update_equipment_model_revision_definition,
        update_equipment_model_revision_status, DriverProfileListFilter, EquipmentAuditEventInput,
        EquipmentClassificationSummaryRecord, EquipmentModelFieldValueRecord,
        EquipmentModelListFilter, EquipmentModelTemplateSnapshotRecord,
        EquipmentOperationFingerprintInput, EquipmentSyncOperationInput,
        MoveEquipmentCategoryRecord, NewDriverProfileIdentityRecord,
        NewDriverProfileRevisionRecord, NewEquipmentCategoryFieldRuleRecord,
        NewEquipmentCategoryRecord, NewEquipmentFieldDefinitionRecord,
        NewEquipmentModelIdentityRecord, NewEquipmentModelRevisionRecord,
        StoredDriverProfileIdentity, StoredDriverProfileRevision, StoredEquipmentAuditEvent,
        StoredEquipmentCategory, StoredEquipmentCategoryFieldRule,
        StoredEquipmentClassificationPreset, StoredEquipmentClassificationPresetPort,
        StoredEquipmentFieldDefinition, StoredEquipmentModelIdentity, StoredEquipmentModelRevision,
        StoredEquipmentOperation, StoredEquipmentRegistryItem, UpdateDefinitionInput,
        UpdateDriverDefinitionCounts, UpdateEquipmentCategoryRecord,
        UpdateEquipmentFieldDefinitionRecord, UpdateModelDefinitionCounts, UpdateStatusInput,
    },
    file_store::{store_content_addressed_file, FileStorePolicy, StoreLocalFileInput},
    measurement_engineering_repository::{
        load_measurement_engineering_revision, MeasurementEngineeringStorageKind,
    },
    render_json, AgentError,
};
use emc_locus_core::equipment::{
    simulate_driver_action, AccessProviderKind, CommunicationInterfaceDefinition,
    CorrectionRequirementKind, DefinitionValidationIssue, DriverProfileDefinition,
    DriverSimulationScenario, EquipmentClass, EquipmentEffectiveFieldRule, EquipmentFieldDataType,
    EquipmentFieldDefinition, EquipmentFieldScope, EquipmentModelDefinition,
    EquipmentModelTemplateSnapshot, EquipmentRevisionStatus, FunctionalRole, PhysicalQuantity,
    PortDirectionality, PortFlowRole, ProtocolKind, SignalDomain, SignalPortDefinition,
    SignalTransformationKind, TechnologyTag, TransportKind,
    EQUIPMENT_MODEL_DEFINITION_SCHEMA_VERSION,
};
use emc_locus_core::measurement_engineering::MeasurementEngineeringAggregateKind;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
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
    pub root_category_id: Option<String>,
    pub demo_mode: Option<String>,
    pub functional_role: Option<String>,
    pub signal_domain: Option<String>,
    pub technology_tag: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListEquipmentCategoriesInput {
    pub include_inactive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateEquipmentCategoryInput {
    pub category_id: String,
    pub parent_category_id: String,
    pub label: String,
    pub description: String,
    pub sort_order: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateEquipmentCategoryInput {
    pub category_id: String,
    pub label: String,
    pub description: String,
    pub sort_order: i64,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveEquipmentCategoryInput {
    pub category_id: String,
    pub parent_category_id: String,
    pub sort_order: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListEquipmentFieldDefinitionsInput {
    pub scope: Option<String>,
    pub include_inactive: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpsertEquipmentFieldDefinitionInput {
    pub field_id: Option<String>,
    pub field_code: Option<String>,
    pub label: String,
    pub description: String,
    pub data_type: String,
    pub scope: String,
    pub required_by_default: bool,
    pub visible_by_default: bool,
    pub unique_value: bool,
    pub unit_quantity: Option<String>,
    pub allowed_units: Vec<String>,
    pub option_values: Vec<String>,
    pub validation_regex: Option<String>,
    pub default_value: Option<Value>,
    pub display_group: String,
    pub display_order: i64,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplaceEquipmentCategoryFieldRulesInput {
    pub category_id: String,
    pub rules: Vec<EquipmentCategoryFieldRuleInput>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EquipmentCategoryFieldRuleInput {
    pub field_id: String,
    pub required: Option<bool>,
    pub visible: Option<bool>,
    pub display_group: Option<String>,
    pub display_order: Option<i64>,
    pub default_value: Option<Value>,
    pub help_text_override: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateEquipmentModelFromCategoryTemplateInput {
    pub category_id: String,
    pub equipment_model_id: Option<String>,
    pub field_values: BTreeMap<String, Value>,
    pub is_demo: bool,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreEquipmentFileInput {
    pub original_filename: String,
    pub mime_type: String,
    pub content_base64: String,
}

pub fn store_equipment_file(
    storage_root: &Path,
    input: StoreEquipmentFileInput,
) -> Result<String, AgentError> {
    store_content_addressed_file(
        storage_root,
        StoreLocalFileInput {
            original_filename: input.original_filename,
            mime_type: input.mime_type,
            content_base64: input.content_base64,
        },
        FileStorePolicy {
            namespace: "equipment",
            invalid_code: "invalid_equipment_file",
            too_large_code: "equipment_file_too_large",
            store_failed_code: "equipment_file_store_failed",
        },
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateEquipmentModelFromPresetInput {
    pub preset_id: String,
    pub equipment_model_id: String,
    pub manufacturer: String,
    pub model_name: String,
    pub variant: Option<String>,
    pub is_demo: bool,
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
    validate_optional_id(input.root_category_id.as_deref(), "root_category_id")?;
    validate_optional_id(input.functional_role.as_deref(), "functional_role")?;
    validate_optional_id(input.signal_domain.as_deref(), "signal_domain")?;
    validate_optional_id(input.technology_tag.as_deref(), "technology_tag")?;
    validate_optional_id(input.status.as_deref(), "status")?;
    let is_demo = demo_mode_filter(input.demo_mode.as_deref())?;
    let connection = open_equipment_connection(storage_root)?;
    let identities = list_equipment_model_identities(
        &connection,
        EquipmentModelListFilter {
            manufacturer: input.manufacturer.as_deref(),
            equipment_class: input.equipment_class.as_deref(),
            category_code: input.category_code.as_deref(),
            root_category_id: input.root_category_id.as_deref(),
            is_demo,
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

pub fn list_equipment_categories_json(
    storage_root: &Path,
    input: ListEquipmentCategoriesInput,
) -> Result<String, AgentError> {
    let connection = open_equipment_connection(storage_root)?;
    let categories = list_equipment_categories(&connection, input.include_inactive)?;
    Ok(render_json(&EquipmentCategoryListDto {
        categories: categories
            .iter()
            .map(|category| equipment_category_dto(category, Vec::new()))
            .collect(),
    }))
}

pub fn equipment_category_tree_json(
    storage_root: &Path,
    input: ListEquipmentCategoriesInput,
) -> Result<String, AgentError> {
    let connection = open_equipment_connection(storage_root)?;
    let categories = list_equipment_categories(&connection, input.include_inactive)?;
    Ok(render_json(&EquipmentCategoryListDto {
        categories: category_tree_dtos(&categories, None),
    }))
}

pub fn create_equipment_category_json(
    storage_root: &Path,
    input: CreateEquipmentCategoryInput,
) -> Result<String, AgentError> {
    validate_id(&input.category_id, "category_id")?;
    validate_id(&input.parent_category_id, "parent_category_id")?;
    if input.label.trim().is_empty() {
        return Err(AgentError::new(
            "invalid_equipment_category",
            "category label is required",
        ));
    }
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    if load_equipment_category(&connection, &input.category_id)?.is_some() {
        return Err(AgentError::new(
            "equipment_category_already_exists",
            "equipment category already exists",
        ));
    }
    let parent =
        load_equipment_category(&connection, &input.parent_category_id)?.ok_or_else(|| {
            AgentError::new(
                "equipment_category_not_found",
                "parent equipment category not found",
            )
        })?;
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_equipment_category(
        &transaction,
        NewEquipmentCategoryRecord {
            category_id: &input.category_id,
            parent_category_id: Some(&input.parent_category_id),
            root_category_id: &parent.root_category_id,
            label: input.label.trim(),
            description: input.description.trim(),
            sort_order: input.sort_order,
            active: true,
            system_defined: false,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    let category = load_equipment_category(&connection, &input.category_id)?.ok_or_else(|| {
        AgentError::new(
            "equipment_category_not_found",
            "created equipment category not found",
        )
    })?;
    Ok(render_json(&EquipmentCategoryEnvelopeDto {
        category: equipment_category_dto(&category, Vec::new()),
    }))
}

pub fn update_equipment_category_json(
    storage_root: &Path,
    input: UpdateEquipmentCategoryInput,
) -> Result<String, AgentError> {
    validate_id(&input.category_id, "category_id")?;
    if input.label.trim().is_empty() {
        return Err(AgentError::new(
            "invalid_equipment_category",
            "category label is required",
        ));
    }
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    require_equipment_category(&connection, &input.category_id)?;
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    update_equipment_category(
        &transaction,
        UpdateEquipmentCategoryRecord {
            category_id: &input.category_id,
            label: input.label.trim(),
            description: input.description.trim(),
            sort_order: input.sort_order,
            active: input.active,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    let category = require_equipment_category(&connection, &input.category_id)?;
    Ok(render_json(&EquipmentCategoryEnvelopeDto {
        category: equipment_category_dto(&category, Vec::new()),
    }))
}

pub fn archive_equipment_category_json(
    storage_root: &Path,
    category_id: &str,
) -> Result<String, AgentError> {
    validate_id(category_id, "category_id")?;
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let category = require_equipment_category(&connection, category_id)?;
    if category.parent_category_id.is_none() || category.system_defined {
        return Err(AgentError::new(
            "equipment_category_system_root_immutable",
            "system root categories cannot be archived",
        ));
    }
    let in_use = count_equipment_models_in_category(&connection, category_id)?;
    if in_use > 0 {
        return Err(AgentError::with_details(
            "equipment_category_in_use",
            "equipment category is used by equipment models and cannot be archived",
            json!({ "category_id": category_id, "equipment_model_count": in_use }),
        ));
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    archive_equipment_category(&transaction, category_id, &now)?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    let category = require_equipment_category(&connection, category_id)?;
    Ok(render_json(&EquipmentCategoryEnvelopeDto {
        category: equipment_category_dto(&category, Vec::new()),
    }))
}

pub fn move_equipment_category_json(
    storage_root: &Path,
    input: MoveEquipmentCategoryInput,
) -> Result<String, AgentError> {
    validate_id(&input.category_id, "category_id")?;
    validate_id(&input.parent_category_id, "parent_category_id")?;
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let category = require_equipment_category(&connection, &input.category_id)?;
    if category.parent_category_id.is_none() || category.system_defined {
        return Err(AgentError::new(
            "equipment_category_system_root_immutable",
            "system root categories cannot be moved",
        ));
    }
    let parent = require_equipment_category(&connection, &input.parent_category_id)?;
    if parent.category_id == input.category_id {
        return Err(AgentError::new(
            "equipment_category_cycle",
            "category cannot be moved under itself",
        ));
    }
    let categories = list_equipment_categories(&connection, true)?;
    if is_descendant_category(&categories, &parent.category_id, &category.category_id) {
        return Err(AgentError::new(
            "equipment_category_cycle",
            "category cannot be moved under one of its descendants",
        ));
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    move_equipment_category(
        &transaction,
        MoveEquipmentCategoryRecord {
            category_id: &input.category_id,
            parent_category_id: Some(&input.parent_category_id),
            root_category_id: &parent.root_category_id,
            sort_order: input.sort_order,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    let category = require_equipment_category(&connection, &input.category_id)?;
    Ok(render_json(&EquipmentCategoryEnvelopeDto {
        category: equipment_category_dto(&category, Vec::new()),
    }))
}

pub fn list_equipment_field_definitions_json(
    storage_root: &Path,
    input: ListEquipmentFieldDefinitionsInput,
) -> Result<String, AgentError> {
    validate_optional_id(input.scope.as_deref(), "scope")?;
    let connection = open_equipment_connection(storage_root)?;
    let fields = list_equipment_field_definitions(
        &connection,
        input.scope.as_deref(),
        input.include_inactive,
    )?;
    Ok(render_json(&EquipmentFieldDefinitionListDto {
        field_definitions: fields.iter().map(field_definition_dto).collect(),
    }))
}

pub fn create_equipment_field_definition_json(
    storage_root: &Path,
    input: UpsertEquipmentFieldDefinitionInput,
) -> Result<String, AgentError> {
    let field_code = input.field_code.as_deref().ok_or_else(|| {
        AgentError::new(
            "invalid_equipment_field",
            "field_code is required when creating a field",
        )
    })?;
    validate_id(field_code, "field_code")?;
    let field_id = input
        .field_id
        .clone()
        .unwrap_or_else(|| format!("field_{field_code}"));
    validate_id(&field_id, "field_id")?;
    let field = core_field_definition_from_input(&field_id, field_code, &input)?;
    validate_field_contract(&field)?;
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    if load_equipment_field_definition(&connection, &field_id)?.is_some()
        || load_equipment_field_definition_by_code(&connection, field_code)?.is_some()
    {
        return Err(AgentError::new(
            "equipment_field_already_exists",
            "equipment field definition already exists",
        ));
    }
    let now = utc_timestamp()?;
    let allowed_units_json = render_json(&input.allowed_units);
    let option_values_json = render_json(&input.option_values);
    let default_value_json = input.default_value.as_ref().map(render_json);
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_equipment_field_definition(
        &transaction,
        NewEquipmentFieldDefinitionRecord {
            field_id: &field_id,
            field_code,
            label: input.label.trim(),
            description: input.description.trim(),
            data_type: &input.data_type,
            scope: &input.scope,
            required_by_default: input.required_by_default,
            visible_by_default: input.visible_by_default,
            unique_value: input.unique_value,
            unit_quantity: input.unit_quantity.as_deref(),
            allowed_units_json: &allowed_units_json,
            option_values_json: &option_values_json,
            validation_regex: input.validation_regex.as_deref(),
            default_value_json: default_value_json.as_deref(),
            display_group: input.display_group.trim(),
            display_order: input.display_order,
            active: input.active,
            system_defined: false,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    let stored = require_equipment_field_definition(&connection, &field_id)?;
    Ok(render_json(&EquipmentFieldDefinitionEnvelopeDto {
        field_definition: field_definition_dto(&stored),
    }))
}

pub fn update_equipment_field_definition_json(
    storage_root: &Path,
    field_id: &str,
    input: UpsertEquipmentFieldDefinitionInput,
) -> Result<String, AgentError> {
    validate_id(field_id, "field_id")?;
    let existing = {
        let connection = open_equipment_connection(storage_root)?;
        require_equipment_field_definition(&connection, field_id)?
    };
    let field = core_field_definition_from_input(field_id, &existing.field_code, &input)?;
    validate_field_contract(&field)?;
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let now = utc_timestamp()?;
    let allowed_units_json = render_json(&input.allowed_units);
    let option_values_json = render_json(&input.option_values);
    let default_value_json = input.default_value.as_ref().map(render_json);
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    update_equipment_field_definition(
        &transaction,
        UpdateEquipmentFieldDefinitionRecord {
            field_id,
            label: input.label.trim(),
            description: input.description.trim(),
            data_type: &input.data_type,
            required_by_default: input.required_by_default,
            visible_by_default: input.visible_by_default,
            unique_value: input.unique_value,
            unit_quantity: input.unit_quantity.as_deref(),
            allowed_units_json: &allowed_units_json,
            option_values_json: &option_values_json,
            validation_regex: input.validation_regex.as_deref(),
            default_value_json: default_value_json.as_deref(),
            display_group: input.display_group.trim(),
            display_order: input.display_order,
            active: input.active,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    let stored = require_equipment_field_definition(&connection, field_id)?;
    Ok(render_json(&EquipmentFieldDefinitionEnvelopeDto {
        field_definition: field_definition_dto(&stored),
    }))
}

pub fn archive_equipment_field_definition_json(
    storage_root: &Path,
    field_id: &str,
) -> Result<String, AgentError> {
    validate_id(field_id, "field_id")?;
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let field = require_equipment_field_definition(&connection, field_id)?;
    if matches!(field.field_code.as_str(), "manufacturer" | "model_name") {
        return Err(AgentError::new(
            "equipment_structural_field_immutable",
            "manufacturer and model_name are structural equipment model identifiers and cannot be archived",
        ));
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    archive_equipment_field_definition(&transaction, field_id, &now)?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    let stored = require_equipment_field_definition(&connection, field_id)?;
    Ok(render_json(&EquipmentFieldDefinitionEnvelopeDto {
        field_definition: field_definition_dto(&stored),
    }))
}

pub fn list_equipment_category_field_rules_json(
    storage_root: &Path,
    category_id: &str,
) -> Result<String, AgentError> {
    validate_id(category_id, "category_id")?;
    let connection = open_equipment_connection(storage_root)?;
    require_equipment_category(&connection, category_id)?;
    let rules = list_equipment_category_field_rules(&connection, category_id)?;
    Ok(render_json(&EquipmentCategoryFieldRuleListDto {
        category_id: category_id.to_owned(),
        rules: rules.iter().map(category_field_rule_dto).collect(),
    }))
}

pub fn replace_equipment_category_field_rules_json(
    storage_root: &Path,
    input: ReplaceEquipmentCategoryFieldRulesInput,
) -> Result<String, AgentError> {
    validate_id(&input.category_id, "category_id")?;
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    require_equipment_category(&connection, &input.category_id)?;
    let mut default_value_json = Vec::new();
    for rule in &input.rules {
        validate_id(&rule.field_id, "field_id")?;
        let field = require_equipment_field_definition(&connection, &rule.field_id)?;
        if let Some(default_value) = rule.default_value.as_ref() {
            validate_equipment_template_value(&field, default_value)?;
        }
        default_value_json.push(rule.default_value.as_ref().map(render_json));
    }
    let now = utc_timestamp()?;
    let records = input
        .rules
        .iter()
        .zip(default_value_json.iter())
        .map(
            |(rule, default_value_json)| NewEquipmentCategoryFieldRuleRecord {
                category_id: &input.category_id,
                field_id: &rule.field_id,
                required: rule.required,
                visible: rule.visible,
                display_group: rule.display_group.as_deref(),
                display_order: rule.display_order,
                default_value_json: default_value_json.as_deref(),
                help_text_override: rule.help_text_override.as_deref(),
                timestamp: &now,
            },
        )
        .collect::<Vec<_>>();
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    replace_equipment_category_field_rules(&transaction, &input.category_id, &records)?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    let rules = list_equipment_category_field_rules(&connection, &input.category_id)?;
    Ok(render_json(&EquipmentCategoryFieldRuleListDto {
        category_id: input.category_id,
        rules: rules.iter().map(category_field_rule_dto).collect(),
    }))
}

pub fn equipment_effective_template_json(
    storage_root: &Path,
    category_id: &str,
) -> Result<String, AgentError> {
    validate_id(category_id, "category_id")?;
    let connection = open_equipment_connection(storage_root)?;
    let template = effective_template_for_category(&connection, category_id)?;
    Ok(render_json(&EquipmentEffectiveTemplateEnvelopeDto {
        effective_template: effective_template_dto(&template),
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
    validate_signal_transformation_references(&connection, &parsed)?;
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
    write_equipment_model_template_snapshot(
        &transaction,
        &input.equipment_model_id,
        &revision_id,
        &parsed,
        &now,
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

pub fn create_equipment_model_from_category_template(
    storage_root: &Path,
    input: CreateEquipmentModelFromCategoryTemplateInput,
) -> Result<String, AgentError> {
    validate_common_operation(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    validate_id(&input.category_id, "category_id")?;
    if input.category_id == "general_equipment" {
        return Err(AgentError::new(
            "equipment_general_category_not_instantiable",
            "la catégorie Général définit les champs communs et ne peut pas créer directement un modèle",
        ));
    }
    if let Some(equipment_model_id) = input.equipment_model_id.as_deref() {
        validate_id(equipment_model_id, "equipment_model_id")?;
    }
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let template = effective_template_for_category(&connection, &input.category_id)?;
    let manufacturer = required_form_text(&input.field_values, "manufacturer", "Fabricant")?;
    let model_name = required_form_text(&input.field_values, "model_name", "Modèle")?;
    let variant = input
        .field_values
        .get("variant")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let equipment_model_id = input.equipment_model_id.clone().unwrap_or_else(|| {
        generated_equipment_model_id(
            &input.category_id,
            manufacturer,
            model_name,
            variant.as_deref(),
        )
    });
    validate_id(&equipment_model_id, "equipment_model_id")?;
    if load_equipment_model_identity(&connection, &equipment_model_id)?.is_some() {
        return Err(AgentError::new(
            "equipment_model_already_exists",
            format!("equipment model already exists: {equipment_model_id}"),
        ));
    }
    let now = utc_timestamp()?;
    let definition = equipment_model_definition_from_category_template(&template, &input, &now)?;
    let canonical = definition
        .canonicalize()
        .map_err(|issues| invalid_definition_error("invalid_equipment_model_definition", issues))?;
    let parsed =
        EquipmentModelDefinition::from_json_str(&canonical.canonical_json).map_err(|issue| {
            invalid_definition_error("invalid_equipment_model_definition", vec![issue])
        })?;
    if !equipment_model_class_exists(&connection, equipment_class_text(parsed.equipment_class))? {
        return Err(AgentError::new(
            "equipment_model_class_not_found",
            format!(
                "unknown equipment class: {}",
                equipment_class_text(parsed.equipment_class)
            ),
        ));
    }
    let revision_id = model_revision_id_for(&equipment_model_id, 1);
    let payload_json = render_json(&json!({
        "equipment_model_id": equipment_model_id,
        "category_id": input.category_id,
        "definition_checksum": canonical.definition_checksum
    }));
    if let Some(operation) = existing_equipment_operation(&connection, &input.operation_id)? {
        ensure_equipment_operation_replay(
            &operation,
            EquipmentOperationFingerprintInput {
                aggregate_kind: "equipment_model",
                entity_id: &equipment_model_id,
                revision_id: Some(&revision_id),
                action: "equipment_model_created_from_category_template",
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
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_equipment_model_identity(
        &transaction,
        NewEquipmentModelIdentityRecord {
            equipment_model_id: &equipment_model_id,
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
            equipment_model_id: &equipment_model_id,
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
            equipment_model_id: &equipment_model_id,
            revision_id: &revision_id,
            revision_number: 1,
            status: revision_status_text(EquipmentRevisionStatus::Draft),
            definition_checksum: &canonical.definition_checksum,
            definition: &parsed,
            timestamp: &now,
        },
    )?;
    write_equipment_model_template_snapshot(
        &transaction,
        &equipment_model_id,
        &revision_id,
        &parsed,
        &now,
    )?;
    write_equipment_audit_and_outbox(
        &transaction,
        EquipmentAuditEventInput {
            aggregate_kind: "equipment_model",
            entity_id: &equipment_model_id,
            revision_id: Some(&revision_id),
            action: "equipment_model_created_from_category_template",
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
            operation_kind: "equipment_model_created_from_category_template",
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
        "equipment_model_created_from_category_template",
        &input.operation_id,
        false,
        &equipment_model_id,
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
    validate_signal_transformation_references(&connection, &parsed)?;
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
    write_equipment_model_template_snapshot(
        &transaction,
        &input.equipment_model_id,
        &revision_id,
        &parsed,
        &now,
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
    validate_signal_transformation_references(&connection, &cloned_definition)?;
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
    write_equipment_model_template_snapshot(
        &transaction,
        &input.new_equipment_model_id,
        &revision_id,
        &cloned_definition,
        &now,
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
    validate_checksum(
        &input.expected_definition_checksum,
        "expected_definition_checksum",
    )?;
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
    validate_signal_transformation_references(&connection, &parsed)?;
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
    write_equipment_model_template_snapshot(
        &transaction,
        &input.equipment_model_id,
        &input.revision_id,
        &parsed,
        &now,
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
    write_equipment_model_template_snapshot(
        &transaction,
        &input.equipment_model_id,
        &revision_id,
        &parsed_source,
        &now,
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
    validate_id(&input.driver_profile_id, "driver_profile_id")?;
    validate_id(&input.revision_id, "revision_id")?;
    validate_checksum(
        &input.expected_definition_checksum,
        "expected_definition_checksum",
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
    let parsed_definition = EquipmentModelDefinition::from_json_str(&revision.definition_json)
        .map_err(|issue| {
            invalid_definition_error("invalid_equipment_model_definition", vec![issue])
        })?;
    validate_signal_transformation_references(&connection, &parsed_definition)?;
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

#[derive(Clone, Debug)]
struct EquipmentEffectiveTemplateInternal {
    category: StoredEquipmentCategory,
    root_category: StoredEquipmentCategory,
    category_path: Vec<StoredEquipmentCategory>,
    fields: Vec<EquipmentEffectiveFieldRule>,
    template_checksum: String,
}

fn require_equipment_category(
    connection: &rusqlite::Connection,
    category_id: &str,
) -> Result<StoredEquipmentCategory, AgentError> {
    load_equipment_category(connection, category_id)?.ok_or_else(|| {
        AgentError::new(
            "equipment_category_not_found",
            format!("equipment category not found: {category_id}"),
        )
    })
}

fn require_equipment_field_definition(
    connection: &rusqlite::Connection,
    field_id: &str,
) -> Result<StoredEquipmentFieldDefinition, AgentError> {
    load_equipment_field_definition(connection, field_id)?.ok_or_else(|| {
        AgentError::new(
            "equipment_field_not_found",
            format!("equipment field definition not found: {field_id}"),
        )
    })
}

fn category_tree_dtos(
    categories: &[StoredEquipmentCategory],
    parent_category_id: Option<&str>,
) -> Vec<EquipmentCategoryDto> {
    let mut children = categories
        .iter()
        .filter(|category| category.parent_category_id.as_deref() == parent_category_id)
        .collect::<Vec<_>>();
    children.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| left.label.cmp(&right.label))
    });
    children
        .into_iter()
        .map(|category| {
            let child_dtos = category_tree_dtos(categories, Some(&category.category_id));
            equipment_category_dto(category, child_dtos)
        })
        .collect()
}

fn equipment_category_dto(
    category: &StoredEquipmentCategory,
    children: Vec<EquipmentCategoryDto>,
) -> EquipmentCategoryDto {
    EquipmentCategoryDto {
        category_id: category.category_id.clone(),
        parent_category_id: category.parent_category_id.clone(),
        root_category_id: category.root_category_id.clone(),
        label: category.label.clone(),
        description: category.description.clone(),
        sort_order: category.sort_order,
        active: category.active,
        system_defined: category.system_defined,
        created_at: category.created_at.clone(),
        updated_at: category.updated_at.clone(),
        children,
    }
}

fn field_definition_dto(field: &StoredEquipmentFieldDefinition) -> EquipmentFieldDefinitionDto {
    EquipmentFieldDefinitionDto {
        field_id: field.field_id.clone(),
        field_code: field.field_code.clone(),
        label: field.label.clone(),
        description: field.description.clone(),
        data_type: field.data_type.clone(),
        scope: field.scope.clone(),
        required_by_default: field.required_by_default,
        visible_by_default: field.visible_by_default,
        unique_value: field.unique_value,
        unit_quantity: field.unit_quantity.clone(),
        allowed_units: parse_json_string_array(&field.allowed_units_json, "allowed_units_json")
            .unwrap_or_default(),
        option_values: parse_json_string_array(&field.option_values_json, "option_values_json")
            .unwrap_or_default(),
        validation_regex: field.validation_regex.clone(),
        default_value: field.default_value_json.as_deref().map(parse_json_value),
        display_group: field.display_group.clone(),
        display_order: field.display_order,
        active: field.active,
        system_defined: field.system_defined,
        created_at: field.created_at.clone(),
        updated_at: field.updated_at.clone(),
    }
}

fn category_field_rule_dto(
    rule: &StoredEquipmentCategoryFieldRule,
) -> EquipmentCategoryFieldRuleDto {
    EquipmentCategoryFieldRuleDto {
        category_id: rule.category_id.clone(),
        field_id: rule.field_id.clone(),
        required: rule.required,
        visible: rule.visible,
        display_group: rule.display_group.clone(),
        display_order: rule.display_order,
        default_value: rule.default_value_json.as_deref().map(parse_json_value),
        help_text_override: rule.help_text_override.clone(),
        updated_at: rule.updated_at.clone(),
    }
}

fn effective_template_for_category(
    connection: &rusqlite::Connection,
    category_id: &str,
) -> Result<EquipmentEffectiveTemplateInternal, AgentError> {
    let categories = list_equipment_categories(connection, true)?;
    let category = categories
        .iter()
        .find(|item| item.category_id == category_id)
        .cloned()
        .ok_or_else(|| {
            AgentError::new(
                "equipment_category_not_found",
                format!("equipment category not found: {category_id}"),
            )
        })?;
    if !category.active {
        return Err(AgentError::new(
            "equipment_category_inactive",
            "inactive equipment categories cannot create new model templates",
        ));
    }
    let root_category = categories
        .iter()
        .find(|item| item.category_id == category.root_category_id)
        .cloned()
        .ok_or_else(|| {
            AgentError::new(
                "equipment_category_not_found",
                "root equipment category not found",
            )
        })?;
    let category_path = category_path_for(&categories, &category.category_id)?;
    let stored_fields =
        list_equipment_field_definitions(connection, Some("equipment_model"), false)?;
    let fields_by_id = stored_fields
        .iter()
        .map(|field| (field.field_id.clone(), field))
        .collect::<BTreeMap<_, _>>();
    let mut effective = BTreeMap::<String, EquipmentEffectiveFieldRule>::new();
    for category in &category_path {
        for rule in list_equipment_category_field_rules(connection, &category.category_id)? {
            let Some(stored_field) = fields_by_id.get(&rule.field_id) else {
                continue;
            };
            let field = core_field_definition_from_stored(stored_field)?;
            let entry = effective
                .entry(field.field_code.clone())
                .or_insert_with(|| EquipmentEffectiveFieldRule {
                    field: field.clone(),
                    required: field.required_by_default,
                    visible: field.visible_by_default,
                    display_group: stored_field.display_group.clone(),
                    display_order: stored_field.display_order,
                    default_value: field.default_value.clone(),
                    help_text: None,
                    inherited_from_category_ids: Vec::new(),
                });
            entry.field = field;
            if let Some(required) = rule.required {
                entry.required = required;
            }
            if let Some(visible) = rule.visible {
                entry.visible = visible;
            }
            if let Some(display_group) = rule.display_group.as_ref() {
                entry.display_group = display_group.clone();
            }
            if let Some(display_order) = rule.display_order {
                entry.display_order = display_order;
            }
            if let Some(default_value) = rule.default_value_json.as_deref() {
                entry.default_value = Some(parse_json_value(default_value));
            }
            if let Some(help) = rule.help_text_override.as_ref() {
                entry.help_text = Some(help.clone());
            }
            entry
                .inherited_from_category_ids
                .push(category.category_id.clone());
        }
    }
    let mut fields = effective.into_values().collect::<Vec<_>>();
    fields.sort_by(|left, right| {
        left.display_order
            .cmp(&right.display_order)
            .then_with(|| left.field.label.cmp(&right.field.label))
    });
    let template_value = json!({
        "category_id": &category.category_id,
        "root_category_id": &root_category.category_id,
        "path": category_path.iter().map(|item| &item.category_id).collect::<Vec<_>>(),
        "fields": &fields,
    });
    let template_checksum = sha256_rendered(&render_json(&template_value));
    Ok(EquipmentEffectiveTemplateInternal {
        category,
        root_category,
        category_path,
        fields,
        template_checksum,
    })
}

fn effective_template_dto(
    template: &EquipmentEffectiveTemplateInternal,
) -> EquipmentEffectiveTemplateDto {
    EquipmentEffectiveTemplateDto {
        category: equipment_category_dto(&template.category, Vec::new()),
        root_category: equipment_category_dto(&template.root_category, Vec::new()),
        category_path: template
            .category_path
            .iter()
            .map(|category| equipment_category_dto(category, Vec::new()))
            .collect(),
        fields: template
            .fields
            .iter()
            .map(|field| EquipmentEffectiveFieldDto {
                field: core_field_definition_dto(&field.field),
                required: field.required,
                visible: field.visible,
                display_group: field.display_group.clone(),
                display_order: field.display_order,
                default_value: field.default_value.clone(),
                help_text: field.help_text.clone(),
                inherited_from_category_ids: field.inherited_from_category_ids.clone(),
            })
            .collect(),
        template_checksum: template.template_checksum.clone(),
    }
}

fn core_field_definition_dto(field: &EquipmentFieldDefinition) -> EquipmentFieldDefinitionDto {
    EquipmentFieldDefinitionDto {
        field_id: field.field_id.clone(),
        field_code: field.field_code.clone(),
        label: field.label.clone(),
        description: field.description.clone(),
        data_type: enum_code(field.data_type),
        scope: enum_code(field.scope),
        required_by_default: field.required_by_default,
        visible_by_default: field.visible_by_default,
        unique_value: field.unique_value,
        unit_quantity: field.unit_quantity.clone(),
        allowed_units: field.allowed_units.clone(),
        option_values: field.option_values.clone(),
        validation_regex: field.validation_regex.clone(),
        default_value: field.default_value.clone(),
        display_group: "Identification".to_owned(),
        display_order: 0,
        active: field.active,
        system_defined: field.system_defined,
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn category_path_for(
    categories: &[StoredEquipmentCategory],
    category_id: &str,
) -> Result<Vec<StoredEquipmentCategory>, AgentError> {
    let mut path = Vec::new();
    let mut current = categories
        .iter()
        .find(|category| category.category_id == category_id)
        .cloned()
        .ok_or_else(|| AgentError::new("equipment_category_not_found", "category not found"))?;
    loop {
        path.push(current.clone());
        let Some(parent_id) = current.parent_category_id.as_deref() else {
            break;
        };
        current = categories
            .iter()
            .find(|category| category.category_id == parent_id)
            .cloned()
            .ok_or_else(|| {
                AgentError::new("equipment_category_not_found", "category parent not found")
            })?;
        if path.len() > categories.len() {
            return Err(AgentError::new(
                "equipment_category_cycle",
                "category hierarchy contains a cycle",
            ));
        }
    }
    path.reverse();
    Ok(path)
}

fn is_descendant_category(
    categories: &[StoredEquipmentCategory],
    candidate_category_id: &str,
    ancestor_category_id: &str,
) -> bool {
    let mut current = categories
        .iter()
        .find(|category| category.category_id == candidate_category_id);
    while let Some(category) = current {
        if category.parent_category_id.as_deref() == Some(ancestor_category_id) {
            return true;
        }
        current = category
            .parent_category_id
            .as_deref()
            .and_then(|parent_id| categories.iter().find(|item| item.category_id == parent_id));
    }
    false
}

fn core_field_definition_from_stored(
    field: &StoredEquipmentFieldDefinition,
) -> Result<EquipmentFieldDefinition, AgentError> {
    Ok(EquipmentFieldDefinition {
        field_id: field.field_id.clone(),
        field_code: field.field_code.clone(),
        label: field.label.clone(),
        description: field.description.clone(),
        data_type: parse_enum_code(&field.data_type, "data_type")?,
        scope: parse_enum_code(&field.scope, "scope")?,
        required_by_default: field.required_by_default,
        visible_by_default: field.visible_by_default,
        unique_value: field.unique_value,
        unit_quantity: field.unit_quantity.clone(),
        allowed_units: parse_json_string_array(&field.allowed_units_json, "allowed_units_json")?,
        option_values: parse_json_string_array(&field.option_values_json, "option_values_json")?,
        validation_regex: field.validation_regex.clone(),
        default_value: field.default_value_json.as_deref().map(parse_json_value),
        active: field.active,
        system_defined: field.system_defined,
    })
}

fn core_field_definition_from_input(
    field_id: &str,
    field_code: &str,
    input: &UpsertEquipmentFieldDefinitionInput,
) -> Result<EquipmentFieldDefinition, AgentError> {
    Ok(EquipmentFieldDefinition {
        field_id: field_id.to_owned(),
        field_code: field_code.to_owned(),
        label: input.label.trim().to_owned(),
        description: input.description.trim().to_owned(),
        data_type: parse_enum_code::<EquipmentFieldDataType>(&input.data_type, "data_type")?,
        scope: parse_enum_code::<EquipmentFieldScope>(&input.scope, "scope")?,
        required_by_default: input.required_by_default,
        visible_by_default: input.visible_by_default,
        unique_value: input.unique_value,
        unit_quantity: input.unit_quantity.clone(),
        allowed_units: input.allowed_units.clone(),
        option_values: input.option_values.clone(),
        validation_regex: input.validation_regex.clone(),
        default_value: input.default_value.clone(),
        active: input.active,
        system_defined: false,
    })
}

fn validate_field_contract(field: &EquipmentFieldDefinition) -> Result<(), AgentError> {
    let issues = emc_locus_core::equipment::validate_equipment_field_definition(field);
    if issues.iter().any(|issue| issue.severity == "error") {
        return Err(invalid_definition_error(
            "invalid_equipment_field_definition",
            issues,
        ));
    }
    Ok(())
}

fn validate_equipment_template_value(
    field: &StoredEquipmentFieldDefinition,
    value: &Value,
) -> Result<(), AgentError> {
    let mut core_field = core_field_definition_from_stored(field)?;
    core_field.default_value = Some(value.clone());
    validate_field_contract(&core_field)
}

fn required_form_text<'a>(
    values: &'a BTreeMap<String, Value>,
    field_code: &str,
    label: &str,
) -> Result<&'a str, AgentError> {
    values
        .get(field_code)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AgentError::with_details(
                "equipment_template_required_field_missing",
                format!("Le champ \"{label}\" est obligatoire."),
                json!({ "field_code": field_code, "label": label }),
            )
        })
}

fn equipment_model_definition_from_category_template(
    template: &EquipmentEffectiveTemplateInternal,
    input: &CreateEquipmentModelFromCategoryTemplateInput,
    captured_at: &str,
) -> Result<EquipmentModelDefinition, AgentError> {
    let manufacturer = required_form_text(&input.field_values, "manufacturer", "Fabricant")?;
    let model_name = required_form_text(&input.field_values, "model_name", "Modèle")?;
    let variant = input
        .field_values
        .get("variant")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let technical = technical_defaults_for_category(
        &template.category.category_id,
        &template.root_category.category_id,
    );
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "entry_template_category_id".to_owned(),
        Value::String(template.category.category_id.clone()),
    );
    metadata.insert(
        "entry_template_category_label".to_owned(),
        Value::String(template.category.label.clone()),
    );
    metadata.insert(
        "entry_template_path".to_owned(),
        Value::Array(
            template
                .category_path
                .iter()
                .map(|category| Value::String(category.label.clone()))
                .collect(),
        ),
    );
    metadata.insert("is_demo".to_owned(), Value::Bool(input.is_demo));
    let custom_field_values = normalized_template_field_values(template, &input.field_values);
    Ok(EquipmentModelDefinition {
        definition_schema_version: EQUIPMENT_MODEL_DEFINITION_SCHEMA_VERSION.to_owned(),
        manufacturer: manufacturer.to_owned(),
        model_name: model_name.to_owned(),
        variant,
        equipment_class: technical.equipment_class,
        functional_role: technical.functional_role,
        category_code: template.category.category_id.clone(),
        signal_domains: technical.signal_domains,
        technology_tags: technical.technology_tags,
        specifications: Vec::new(),
        signal_ports: technical.signal_ports,
        signal_paths: Vec::new(),
        communication_interfaces: technical.communication_interfaces,
        capabilities: Vec::new(),
        custom_field_values,
        template_snapshot: Some(EquipmentModelTemplateSnapshot {
            category_id: template.category.category_id.clone(),
            root_category_id: template.root_category.category_id.clone(),
            category_path: template
                .category_path
                .iter()
                .map(|category| category.label.clone())
                .collect(),
            captured_at: captured_at.to_owned(),
            template_checksum: template.template_checksum.clone(),
            fields: template.fields.clone(),
        }),
        is_demo: input.is_demo,
        metadata,
    })
}

fn normalized_template_field_values(
    template: &EquipmentEffectiveTemplateInternal,
    values: &BTreeMap<String, Value>,
) -> BTreeMap<String, Value> {
    let visible_field_codes = template
        .fields
        .iter()
        .filter(|field| field.visible)
        .map(|field| field.field.field_code.as_str())
        .collect::<BTreeSet<_>>();
    values
        .iter()
        .filter(|(field_code, value)| {
            visible_field_codes.contains(field_code.as_str()) && !is_blank_template_value(value)
        })
        .map(|(field_code, value)| (field_code.clone(), value.clone()))
        .collect()
}

fn is_blank_template_value(value: &Value) -> bool {
    if value.is_null() {
        return true;
    }
    if value.as_str().is_some_and(|text| text.trim().is_empty()) {
        return true;
    }
    if value.as_array().is_some_and(Vec::is_empty) {
        return true;
    }
    if let Some(object) = value.as_object() {
        if object.contains_key("object_id") || object.contains_key("original_filename") {
            return false;
        }
        let number_missing = object
            .get("value")
            .is_none_or(|candidate| candidate.as_f64().is_none());
        let unit_missing = object
            .get("unit")
            .and_then(Value::as_str)
            .is_none_or(|unit| unit.trim().is_empty());
        return number_missing && unit_missing;
    }
    false
}

struct TechnicalCategoryDefaults {
    equipment_class: EquipmentClass,
    functional_role: FunctionalRole,
    signal_domains: Vec<SignalDomain>,
    technology_tags: Vec<TechnologyTag>,
    signal_ports: Vec<SignalPortDefinition>,
    communication_interfaces: Vec<CommunicationInterfaceDefinition>,
}

fn technical_defaults_for_category(
    category_id: &str,
    root_category_id: &str,
) -> TechnicalCategoryDefaults {
    match category_id {
        "rf_cable" | "rf_attenuator" | "rf_coupler" | "rf_filter" | "rf_load" => {
            TechnicalCategoryDefaults {
                equipment_class: EquipmentClass::PassiveComponent,
                functional_role: FunctionalRole::RfNetworkElement,
                signal_domains: vec![SignalDomain::Rf],
                technology_tags: vec![TechnologyTag::Rf50Ohm],
                signal_ports: vec![
                    template_rf_port("rf_a", "RF A", PortDirectionality::Through),
                    template_rf_port("rf_b", "RF B", PortDirectionality::Through),
                ],
                communication_interfaces: Vec::new(),
            }
        }
        "receiving_antenna" | "field_probe" | "current_probe" => TechnicalCategoryDefaults {
            equipment_class: EquipmentClass::Sensor,
            functional_role: FunctionalRole::Sensor,
            signal_domains: vec![SignalDomain::Environmental, SignalDomain::Rf],
            technology_tags: vec![TechnologyTag::Rf50Ohm],
            signal_ports: vec![
                SignalPortDefinition {
                    port_id: "field".to_owned(),
                    label: "Champ".to_owned(),
                    directionality: PortDirectionality::Input,
                    flow_role: PortFlowRole::FieldSidePort,
                    signal_domain: SignalDomain::Environmental,
                    required: true,
                    connector_type: None,
                    technology_tags: Vec::new(),
                    quantity: PhysicalQuantity::ElectricField,
                    unit: "dBuV_per_m".to_owned(),
                    impedance: None,
                    frequency_min: None,
                    frequency_max: None,
                    voltage_max: None,
                    current_max: None,
                    power_max: None,
                    channel_index: None,
                    differential: false,
                    isolated: false,
                    comment: None,
                },
                template_rf_port("rf_out", "Sortie RF", PortDirectionality::Output),
            ],
            communication_interfaces: Vec::new(),
        },
        "oscilloscope" | "daq" => TechnicalCategoryDefaults {
            equipment_class: EquipmentClass::DaqDevice,
            functional_role: FunctionalRole::AcquisitionDevice,
            signal_domains: vec![
                SignalDomain::AnalogVoltage,
                SignalDomain::Trigger,
                SignalDomain::Ethernet,
            ],
            technology_tags: vec![
                TechnologyTag::VoltageInput,
                TechnologyTag::Trigger,
                TechnologyTag::Ethernet,
            ],
            signal_ports: vec![
                SignalPortDefinition {
                    port_id: "ch1".to_owned(),
                    label: "CH1".to_owned(),
                    directionality: PortDirectionality::Input,
                    flow_role: PortFlowRole::MeasurementPort,
                    signal_domain: SignalDomain::AnalogVoltage,
                    required: true,
                    connector_type: Some("BNC".to_owned()),
                    technology_tags: vec![TechnologyTag::VoltageInput],
                    quantity: PhysicalQuantity::Voltage,
                    unit: "V".to_owned(),
                    impedance: None,
                    frequency_min: None,
                    frequency_max: None,
                    voltage_max: None,
                    current_max: None,
                    power_max: None,
                    channel_index: Some(1),
                    differential: false,
                    isolated: false,
                    comment: None,
                },
                template_communication_port(),
            ],
            communication_interfaces: vec![template_simulated_scpi_interface()],
        },
        "emc_receiver" | "spectrum_analyzer" | "rf_power_meter" => TechnicalCategoryDefaults {
            equipment_class: EquipmentClass::ControllableInstrument,
            functional_role: FunctionalRole::MeasurementInstrument,
            signal_domains: vec![SignalDomain::Rf, SignalDomain::Ethernet],
            technology_tags: vec![
                TechnologyTag::Rf50Ohm,
                TechnologyTag::Ethernet,
                TechnologyTag::Scpi,
            ],
            signal_ports: vec![
                SignalPortDefinition {
                    port_id: "rf_input".to_owned(),
                    label: "Entrée RF".to_owned(),
                    directionality: PortDirectionality::Input,
                    flow_role: PortFlowRole::MeasurementPort,
                    signal_domain: SignalDomain::Rf,
                    required: true,
                    connector_type: Some("N".to_owned()),
                    technology_tags: vec![TechnologyTag::Rf50Ohm],
                    quantity: PhysicalQuantity::Power,
                    unit: "dBm".to_owned(),
                    impedance: Some(50.0),
                    frequency_min: None,
                    frequency_max: None,
                    voltage_max: None,
                    current_max: None,
                    power_max: None,
                    channel_index: None,
                    differential: false,
                    isolated: false,
                    comment: None,
                },
                template_communication_port(),
            ],
            communication_interfaces: vec![template_simulated_scpi_interface()],
        },
        _ => match root_category_id {
            "energy_sources" => TechnicalCategoryDefaults {
                equipment_class: EquipmentClass::ControllableInstrument,
                functional_role: FunctionalRole::EnergySource,
                signal_domains: vec![
                    SignalDomain::PowerAc,
                    SignalDomain::PowerDc,
                    SignalDomain::Ethernet,
                ],
                technology_tags: vec![TechnologyTag::Ethernet],
                signal_ports: vec![
                    SignalPortDefinition {
                        port_id: "output".to_owned(),
                        label: "Sortie".to_owned(),
                        directionality: PortDirectionality::Output,
                        flow_role: PortFlowRole::SourcePort,
                        signal_domain: SignalDomain::PowerAc,
                        required: true,
                        connector_type: None,
                        technology_tags: Vec::new(),
                        quantity: PhysicalQuantity::Voltage,
                        unit: "V".to_owned(),
                        impedance: None,
                        frequency_min: None,
                        frequency_max: None,
                        voltage_max: None,
                        current_max: None,
                        power_max: None,
                        channel_index: None,
                        differential: false,
                        isolated: false,
                        comment: None,
                    },
                    template_communication_port(),
                ],
                communication_interfaces: vec![template_simulated_scpi_interface()],
            },
            "signal_sources" | "actuators_emitters" => TechnicalCategoryDefaults {
                equipment_class: EquipmentClass::ControllableInstrument,
                functional_role: if root_category_id == "signal_sources" {
                    FunctionalRole::SignalSource
                } else {
                    FunctionalRole::Actuator
                },
                signal_domains: vec![SignalDomain::Rf, SignalDomain::Ethernet],
                technology_tags: vec![TechnologyTag::Rf50Ohm, TechnologyTag::Ethernet],
                signal_ports: vec![
                    template_rf_port("rf_output", "Sortie RF", PortDirectionality::Output),
                    template_communication_port(),
                ],
                communication_interfaces: vec![template_simulated_scpi_interface()],
            },
            "processing_control_systems" => TechnicalCategoryDefaults {
                equipment_class: EquipmentClass::SoftwareAdapter,
                functional_role: FunctionalRole::SoftwareSystem,
                signal_domains: vec![SignalDomain::Software],
                technology_tags: Vec::new(),
                signal_ports: Vec::new(),
                communication_interfaces: Vec::new(),
            },
            _ => TechnicalCategoryDefaults {
                equipment_class: EquipmentClass::ManualEquipment,
                functional_role: FunctionalRole::ManualAccessory,
                signal_domains: Vec::new(),
                technology_tags: Vec::new(),
                signal_ports: Vec::new(),
                communication_interfaces: Vec::new(),
            },
        },
    }
}

fn template_rf_port(
    port_id: &str,
    label: &str,
    directionality: PortDirectionality,
) -> SignalPortDefinition {
    SignalPortDefinition {
        port_id: port_id.to_owned(),
        label: label.to_owned(),
        directionality,
        flow_role: match directionality {
            PortDirectionality::Output => PortFlowRole::SourcePort,
            PortDirectionality::Input => PortFlowRole::MeasurementPort,
            _ => PortFlowRole::ThroughPort,
        },
        signal_domain: SignalDomain::Rf,
        required: true,
        connector_type: Some("N".to_owned()),
        technology_tags: vec![TechnologyTag::Rf50Ohm],
        quantity: PhysicalQuantity::Power,
        unit: "dBm".to_owned(),
        impedance: Some(50.0),
        frequency_min: None,
        frequency_max: None,
        voltage_max: None,
        current_max: None,
        power_max: None,
        channel_index: None,
        differential: false,
        isolated: false,
        comment: None,
    }
}

fn template_communication_port() -> SignalPortDefinition {
    SignalPortDefinition {
        port_id: "lan".to_owned(),
        label: "LAN".to_owned(),
        directionality: PortDirectionality::Communication,
        flow_role: PortFlowRole::CommunicationPort,
        signal_domain: SignalDomain::Ethernet,
        required: false,
        connector_type: Some("RJ45".to_owned()),
        technology_tags: vec![TechnologyTag::Ethernet],
        quantity: PhysicalQuantity::Binary,
        unit: "dimensionless".to_owned(),
        impedance: None,
        frequency_min: None,
        frequency_max: None,
        voltage_max: None,
        current_max: None,
        power_max: None,
        channel_index: None,
        differential: false,
        isolated: false,
        comment: None,
    }
}

fn template_simulated_scpi_interface() -> CommunicationInterfaceDefinition {
    CommunicationInterfaceDefinition {
        interface_id: "simulation".to_owned(),
        label: "Simulation SCPI".to_owned(),
        transport_kind: TransportKind::EthernetTcp,
        access_provider_kind: AccessProviderKind::Simulation,
        protocol_kind: ProtocolKind::Scpi,
        required: false,
        default_interface: true,
        configuration_schema: BTreeMap::new(),
        default_configuration: BTreeMap::new(),
        framing: Some("lf".to_owned()),
        identification_strategy: None,
        firmware_compatibility: Vec::new(),
    }
}

fn write_equipment_model_template_snapshot(
    transaction: &rusqlite::Transaction<'_>,
    equipment_model_id: &str,
    revision_id: &str,
    definition: &EquipmentModelDefinition,
    timestamp: &str,
) -> Result<(), AgentError> {
    let snapshot = definition.template_snapshot.as_ref().map(|snapshot| {
        let snapshot_json = render_json(snapshot);
        (
            snapshot_json,
            EquipmentModelTemplateSnapshotRecord {
                equipment_model_id,
                revision_id,
                category_id: &snapshot.category_id,
                root_category_id: &snapshot.root_category_id,
                snapshot_json: "",
                snapshot_checksum: &snapshot.template_checksum,
                timestamp,
            },
        )
    });
    let mut snapshot_record = None;
    let snapshot_json_storage;
    if let Some((snapshot_json, mut record)) = snapshot {
        snapshot_json_storage = snapshot_json;
        record.snapshot_json = &snapshot_json_storage;
        snapshot_record = Some(record);
    }
    let fields_by_code = definition
        .template_snapshot
        .as_ref()
        .map(|snapshot| {
            snapshot
                .fields
                .iter()
                .map(|field| (field.field.field_code.clone(), field.field.field_id.clone()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let value_jsons = definition
        .custom_field_values
        .iter()
        .filter_map(|(field_code, value)| {
            fields_by_code.get(field_code).map(|field_id| {
                (
                    field_id.clone(),
                    render_json(value),
                    display_value_for(value),
                )
            })
        })
        .collect::<Vec<_>>();
    let field_values = value_jsons
        .iter()
        .map(
            |(field_id, value_json, display_value)| EquipmentModelFieldValueRecord {
                equipment_model_id,
                revision_id,
                field_id,
                value_json,
                display_value,
            },
        )
        .collect::<Vec<_>>();
    replace_equipment_model_template_snapshot(
        transaction,
        equipment_model_id,
        revision_id,
        snapshot_record,
        &field_values,
    )
}

fn display_value_for(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Array(items) => items
            .iter()
            .map(display_value_for)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Object(object) => {
            if let (Some(value), Some(unit)) = (object.get("value"), object.get("unit")) {
                format!("{} {}", display_value_for(value), display_value_for(unit))
            } else {
                render_json(value)
            }
        }
        Value::Null => String::new(),
    }
}

fn generated_equipment_model_id(
    category_id: &str,
    manufacturer: &str,
    model_name: &str,
    variant: Option<&str>,
) -> String {
    let mut raw = format!("{category_id}-{manufacturer}-{model_name}");
    if let Some(variant) = variant {
        raw.push('-');
        raw.push_str(variant);
    }
    let slug = raw
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    format!("EQM-{slug}")
}

fn demo_mode_filter(value: Option<&str>) -> Result<Option<bool>, AgentError> {
    match value.unwrap_or("hide") {
        "hide" => Ok(Some(false)),
        "show" | "all" => Ok(None),
        "only" => Ok(Some(true)),
        other => Err(AgentError::with_details(
            "invalid_demo_mode",
            "demo_mode must be hide, show, or only",
            json!({ "demo_mode": other }),
        )),
    }
}

fn root_category_for_category_code(category_code: &str) -> &'static str {
    match category_code {
        "energy_sources" => "energy_sources",
        "signal_sources" => "signal_sources",
        "rf_equipment" | "rf_cable" | "rf_attenuator" | "rf_coupler" | "rf_amplifier"
        | "rf_filter" | "rf_load" => "rf_equipment",
        "sensors_transducers"
        | "receiving_antenna"
        | "field_probe"
        | "current_probe"
        | "accelerometer"
        | "microphone" => "sensors_transducers",
        "actuators_emitters" => "actuators_emitters",
        "measurement_instruments_digitizers"
        | "emc_receiver"
        | "spectrum_analyzer"
        | "oscilloscope"
        | "rf_power_meter"
        | "multimeter"
        | "daq" => "measurement_instruments_digitizers",
        "processing_control_systems" => "processing_control_systems",
        _ => "",
    }
}

fn sha256_rendered(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    format!("sha256:{digest:x}")
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
    let root_category_id = input
        .definition
        .template_snapshot
        .as_ref()
        .map(|snapshot| snapshot.root_category_id.as_str())
        .unwrap_or_else(|| root_category_for_category_code(&input.definition.category_code));
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
            root_category_id,
            is_demo: input.definition.is_demo
                || input
                    .definition
                    .metadata
                    .get("is_demo")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
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
        root_category_id: identity.root_category_id.clone(),
        is_demo: identity.is_demo,
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
        signal_paths: Vec::new(),
        communication_interfaces: Vec::new(),
        capabilities: Vec::new(),
        custom_field_values: BTreeMap::new(),
        template_snapshot: None,
        is_demo: input.is_demo,
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

fn validate_checksum(value: &str, field: &'static str) -> Result<(), AgentError> {
    let Some(rest) = value.strip_prefix("sha256:") else {
        return Err(AgentError::with_details(
            "invalid_checksum",
            format!("{field} must be sha256:<64 lowercase hex characters>"),
            json!({ "field": field }),
        ));
    };
    if rest.len() != 64
        || !rest
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, 'a'..='f'))
    {
        return Err(AgentError::with_details(
            "invalid_checksum",
            format!("{field} must be sha256:<64 lowercase hex characters>"),
            json!({ "field": field }),
        ));
    }
    Ok(())
}

fn validate_signal_transformation_references(
    connection: &rusqlite::Connection,
    definition: &EquipmentModelDefinition,
) -> Result<(), AgentError> {
    for signal_path in &definition.signal_paths {
        for reference in &signal_path.transformations {
            let aggregate_kind = match reference.transformation_kind {
                SignalTransformationKind::SampleConversion => {
                    MeasurementEngineeringAggregateKind::ScalingProfile
                }
                SignalTransformationKind::FrequencyResponse => {
                    MeasurementEngineeringAggregateKind::EngineeringCurve
                }
            };
            let storage_kind = MeasurementEngineeringStorageKind::from_core(aggregate_kind);
            let revision = load_measurement_engineering_revision(
                connection,
                storage_kind,
                &reference.entity_id,
                &reference.revision_id,
            )?
            .ok_or_else(|| {
                AgentError::with_details(
                    "signal_transformation_reference_not_found",
                    "signal path references an unknown conversion or frequency response",
                    json!({
                        "path_id": signal_path.path_id,
                        "transformation_kind": enum_code(reference.transformation_kind),
                        "entity_id": reference.entity_id,
                        "revision_id": reference.revision_id
                    }),
                )
            })?;
            if !matches!(revision.status.as_str(), "approved" | "superseded") {
                return Err(AgentError::with_details(
                    "signal_transformation_reference_not_controlled",
                    "signal paths may only reference approved or historically superseded definitions",
                    json!({
                        "path_id": signal_path.path_id,
                        "entity_id": reference.entity_id,
                        "revision_id": reference.revision_id,
                        "status": revision.status
                    }),
                ));
            }
            if revision.definition_checksum != reference.definition_checksum {
                return Err(AgentError::with_details(
                    "signal_transformation_checksum_mismatch",
                    "signal path transformation checksum does not match the controlled revision",
                    json!({
                        "path_id": signal_path.path_id,
                        "entity_id": reference.entity_id,
                        "revision_id": reference.revision_id,
                        "expected_checksum": revision.definition_checksum,
                        "provided_checksum": reference.definition_checksum
                    }),
                ));
            }
        }
        for requirement in &signal_path.correction_requirements {
            let Some(reference) = &requirement.model_default_reference else {
                continue;
            };
            let aggregate_kind = match requirement.correction_kind {
                CorrectionRequirementKind::RawSignalConversion => {
                    MeasurementEngineeringAggregateKind::ScalingProfile
                }
                CorrectionRequirementKind::FrequencyDependentCorrection => {
                    MeasurementEngineeringAggregateKind::EngineeringCurve
                }
            };
            let storage_kind = MeasurementEngineeringStorageKind::from_core(aggregate_kind);
            let revision = load_measurement_engineering_revision(
                connection,
                storage_kind,
                &reference.definition_id,
                &reference.revision_id,
            )?
            .ok_or_else(|| {
                AgentError::with_details(
                    "model_default_correction_not_found",
                    "correction requirement references an unknown nominal model value",
                    json!({
                        "path_id": signal_path.path_id,
                        "requirement_id": requirement.requirement_id,
                        "definition_id": reference.definition_id,
                        "revision_id": reference.revision_id
                    }),
                )
            })?;
            if !matches!(revision.status.as_str(), "approved" | "superseded") {
                return Err(AgentError::with_details(
                    "model_default_correction_not_controlled",
                    "a nominal model value must reference an approved or historically superseded definition",
                    json!({
                        "path_id": signal_path.path_id,
                        "requirement_id": requirement.requirement_id,
                        "definition_id": reference.definition_id,
                        "revision_id": reference.revision_id,
                        "status": revision.status
                    }),
                ));
            }
            if revision.definition_checksum != reference.definition_checksum {
                return Err(AgentError::with_details(
                    "model_default_correction_checksum_mismatch",
                    "nominal model correction checksum does not match the controlled revision",
                    json!({
                        "path_id": signal_path.path_id,
                        "requirement_id": requirement.requirement_id,
                        "definition_id": reference.definition_id,
                        "revision_id": reference.revision_id,
                        "expected_checksum": revision.definition_checksum,
                        "provided_checksum": reference.definition_checksum
                    }),
                ));
            }
        }
    }
    Ok(())
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_failed", error.to_string()))
}
