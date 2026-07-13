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
    pub(crate) root_category_id: Option<String>,
    pub(crate) is_demo: bool,
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
pub(crate) struct EquipmentCategoryDto {
    pub(crate) category_id: String,
    pub(crate) parent_category_id: Option<String>,
    pub(crate) root_category_id: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) sort_order: i64,
    pub(crate) active: bool,
    pub(crate) system_defined: bool,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) children: Vec<EquipmentCategoryDto>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentCategoryListDto {
    pub(crate) categories: Vec<EquipmentCategoryDto>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentCategoryEnvelopeDto {
    pub(crate) category: EquipmentCategoryDto,
}

#[derive(Serialize)]
pub(crate) struct EquipmentFieldDefinitionDto {
    pub(crate) field_id: String,
    pub(crate) field_code: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) data_type: String,
    pub(crate) scope: String,
    pub(crate) required_by_default: bool,
    pub(crate) visible_by_default: bool,
    pub(crate) unique_value: bool,
    pub(crate) unit_quantity: Option<String>,
    pub(crate) allowed_units: Vec<String>,
    pub(crate) option_values: Vec<String>,
    pub(crate) validation_regex: Option<String>,
    pub(crate) default_value: Option<Value>,
    pub(crate) display_group: String,
    pub(crate) display_order: i64,
    pub(crate) active: bool,
    pub(crate) system_defined: bool,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize)]
pub(crate) struct EquipmentFieldDefinitionListDto {
    pub(crate) field_definitions: Vec<EquipmentFieldDefinitionDto>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentFieldDefinitionEnvelopeDto {
    pub(crate) field_definition: EquipmentFieldDefinitionDto,
}

#[derive(Serialize)]
pub(crate) struct EquipmentCategoryFieldRuleDto {
    pub(crate) category_id: String,
    pub(crate) field_id: String,
    pub(crate) required: Option<bool>,
    pub(crate) visible: Option<bool>,
    pub(crate) display_group: Option<String>,
    pub(crate) display_order: Option<i64>,
    pub(crate) default_value: Option<Value>,
    pub(crate) help_text_override: Option<String>,
    pub(crate) updated_at: String,
}

#[derive(Serialize)]
pub(crate) struct EquipmentCategoryFieldRuleListDto {
    pub(crate) category_id: String,
    pub(crate) rules: Vec<EquipmentCategoryFieldRuleDto>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentEffectiveFieldDto {
    pub(crate) field: EquipmentFieldDefinitionDto,
    pub(crate) required: bool,
    pub(crate) visible: bool,
    pub(crate) display_group: String,
    pub(crate) display_order: i64,
    pub(crate) default_value: Option<Value>,
    pub(crate) help_text: Option<String>,
    pub(crate) inherited_from_category_ids: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct EquipmentEffectiveTemplateDto {
    pub(crate) category: EquipmentCategoryDto,
    pub(crate) root_category: EquipmentCategoryDto,
    pub(crate) category_path: Vec<EquipmentCategoryDto>,
    pub(crate) fields: Vec<EquipmentEffectiveFieldDto>,
    pub(crate) template_checksum: String,
}

#[derive(Serialize)]
pub(crate) struct EquipmentEffectiveTemplateEnvelopeDto {
    pub(crate) effective_template: EquipmentEffectiveTemplateDto,
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
