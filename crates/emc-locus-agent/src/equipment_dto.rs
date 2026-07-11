use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub(crate) struct EquipmentModelIdentityDto {
    pub(crate) equipment_model_id: String,
    pub(crate) manufacturer: String,
    pub(crate) model_name: String,
    pub(crate) variant: Option<String>,
    pub(crate) equipment_class: String,
    pub(crate) category_code: String,
    pub(crate) current_approved_revision_id: Option<String>,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize)]
pub(crate) struct EquipmentModelRevisionDto {
    pub(crate) revision_id: String,
    pub(crate) equipment_model_id: String,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<String>,
    pub(crate) status: String,
    pub(crate) definition_schema_version: String,
    pub(crate) definition: Value,
    pub(crate) definition_checksum: String,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) submitted_at: Option<String>,
    pub(crate) approved_at: Option<String>,
    pub(crate) capability_count: u32,
    pub(crate) interface_count: u32,
    pub(crate) signal_port_count: u32,
}

#[derive(Serialize)]
pub(crate) struct EquipmentModelAggregateDto {
    pub(crate) identity: EquipmentModelIdentityDto,
    pub(crate) current_approved_revision: Option<EquipmentModelRevisionDto>,
    pub(crate) latest_revision: Option<EquipmentModelRevisionDto>,
    pub(crate) active_draft_revision: Option<EquipmentModelRevisionDto>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentModelEnvelopeDto {
    pub(crate) equipment_model: EquipmentModelAggregateDto,
}

#[derive(Serialize)]
pub(crate) struct EquipmentModelListDto {
    pub(crate) equipment_models: Vec<EquipmentModelAggregateDto>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentModelRevisionEnvelopeDto {
    pub(crate) revision: EquipmentModelRevisionDto,
}

#[derive(Serialize)]
pub(crate) struct EquipmentModelRevisionListDto {
    pub(crate) equipment_model_id: String,
    pub(crate) revisions: Vec<EquipmentModelRevisionDto>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentRegistryItemDto {
    pub(crate) code: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) recommended_equipment_classes: Vec<String>,
    pub(crate) recommended_functional_roles: Vec<String>,
    pub(crate) compatible_signal_domains: Vec<String>,
    pub(crate) compatible_directionalities: Vec<String>,
    pub(crate) deprecated: bool,
}

#[derive(Serialize)]
pub(crate) struct EquipmentRegistriesDto {
    pub(crate) functional_roles: Vec<EquipmentRegistryItemDto>,
    pub(crate) signal_domains: Vec<EquipmentRegistryItemDto>,
    pub(crate) port_directionalities: Vec<EquipmentRegistryItemDto>,
    pub(crate) flow_roles: Vec<EquipmentRegistryItemDto>,
    pub(crate) technology_tags: Vec<EquipmentRegistryItemDto>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentClassificationPresetPortDto {
    pub(crate) port_order: u32,
    pub(crate) port_id: String,
    pub(crate) label: String,
    pub(crate) directionality: String,
    pub(crate) flow_role: String,
    pub(crate) signal_domain: String,
    pub(crate) connector_type: Option<String>,
    pub(crate) technology_tags: Vec<String>,
    pub(crate) quantity: String,
    pub(crate) unit: String,
    pub(crate) impedance: Option<f64>,
    pub(crate) frequency_min: Option<f64>,
    pub(crate) frequency_max: Option<f64>,
    pub(crate) voltage_max: Option<f64>,
    pub(crate) current_max: Option<f64>,
    pub(crate) power_max: Option<f64>,
    pub(crate) required: bool,
    pub(crate) comment: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentClassificationPresetDto {
    pub(crate) preset_id: String,
    pub(crate) category_label: String,
    pub(crate) function_description: String,
    pub(crate) example_label: String,
    pub(crate) default_equipment_class: String,
    pub(crate) default_functional_role: String,
    pub(crate) default_signal_domains: Vec<String>,
    pub(crate) default_technology_tags: Vec<String>,
    pub(crate) notes: String,
    pub(crate) deprecated: bool,
    pub(crate) ports: Vec<EquipmentClassificationPresetPortDto>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentClassificationPresetListDto {
    pub(crate) presets: Vec<EquipmentClassificationPresetDto>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentClassificationPresetEnvelopeDto {
    pub(crate) preset: EquipmentClassificationPresetDto,
}

#[derive(Serialize)]
pub(crate) struct DriverProfileIdentityDto {
    pub(crate) driver_profile_id: String,
    pub(crate) equipment_model_id: String,
    pub(crate) label: String,
    pub(crate) current_approved_revision_id: Option<String>,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize)]
pub(crate) struct DriverProfileRevisionDto {
    pub(crate) revision_id: String,
    pub(crate) driver_profile_id: String,
    pub(crate) equipment_model_id: String,
    pub(crate) supported_model_revision_id: String,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<String>,
    pub(crate) status: String,
    pub(crate) definition_schema_version: String,
    pub(crate) definition: Value,
    pub(crate) definition_checksum: String,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) submitted_at: Option<String>,
    pub(crate) approved_at: Option<String>,
    pub(crate) action_count: u32,
}

#[derive(Serialize)]
pub(crate) struct DriverProfileAggregateDto {
    pub(crate) identity: DriverProfileIdentityDto,
    pub(crate) current_approved_revision: Option<DriverProfileRevisionDto>,
    pub(crate) latest_revision: Option<DriverProfileRevisionDto>,
    pub(crate) active_draft_revision: Option<DriverProfileRevisionDto>,
}

#[derive(Serialize)]
pub(crate) struct DriverProfileEnvelopeDto {
    pub(crate) driver_profile: DriverProfileAggregateDto,
}

#[derive(Serialize)]
pub(crate) struct DriverProfileListDto {
    pub(crate) driver_profiles: Vec<DriverProfileAggregateDto>,
}

#[derive(Serialize)]
pub(crate) struct DriverProfileRevisionEnvelopeDto {
    pub(crate) revision: DriverProfileRevisionDto,
}

#[derive(Serialize)]
pub(crate) struct DriverProfileRevisionListDto {
    pub(crate) driver_profile_id: String,
    pub(crate) revisions: Vec<DriverProfileRevisionDto>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentOperationResultDto<TAggregate, TRevision> {
    pub(crate) operation: String,
    pub(crate) operation_id: String,
    pub(crate) replayed: bool,
    pub(crate) aggregate: TAggregate,
    pub(crate) revision: TRevision,
}

#[derive(Serialize)]
pub(crate) struct EquipmentAuditEventsDto {
    pub(crate) aggregate_kind: String,
    pub(crate) entity_id: String,
    pub(crate) audit_events: Vec<EquipmentAuditEventDto>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentAuditEventDto {
    pub(crate) audit_id: u64,
    pub(crate) aggregate_kind: String,
    pub(crate) entity_id: String,
    pub(crate) revision_id: Option<String>,
    pub(crate) action: String,
    pub(crate) actor: String,
    pub(crate) reason: String,
    pub(crate) old_revision_id: Option<String>,
    pub(crate) new_revision_id: Option<String>,
    pub(crate) old_definition_checksum: Option<String>,
    pub(crate) new_definition_checksum: Option<String>,
    pub(crate) operation_id: String,
    pub(crate) correlation_id: String,
    pub(crate) device_id: String,
    pub(crate) payload_json: String,
    pub(crate) occurred_at: String,
}

#[derive(Serialize)]
pub(crate) struct EquipmentDefinitionValidationDto {
    pub(crate) valid: bool,
    pub(crate) issues: Vec<EquipmentDefinitionValidationIssueDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) definition_schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) definition_checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) canonical_json: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentDefinitionValidationIssueDto {
    pub(crate) severity: String,
    pub(crate) code: String,
    pub(crate) path: String,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) suggestion: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct CommunicationProviderStatusDto {
    pub(crate) provider: String,
    pub(crate) available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct CommunicationProviderStatusListDto {
    pub(crate) providers: Vec<CommunicationProviderStatusDto>,
}
