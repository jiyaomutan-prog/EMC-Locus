use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub(crate) struct TestTemplateIdentityDto {
    pub(crate) template_id: String,
    pub(crate) title: String,
    pub(crate) category_code: String,
    pub(crate) current_approved_revision_id: Option<String>,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateRevisionDto {
    pub(crate) revision_id: String,
    pub(crate) template_id: String,
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
}

#[derive(Serialize)]
pub(crate) struct TestTemplateAggregateDto {
    pub(crate) identity: TestTemplateIdentityDto,
    pub(crate) current_approved_revision: Option<TestTemplateRevisionDto>,
    pub(crate) latest_revision: Option<TestTemplateRevisionDto>,
    pub(crate) active_draft_revision: Option<TestTemplateRevisionDto>,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateEnvelopeDto {
    pub(crate) test_template: TestTemplateAggregateDto,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateListDto {
    pub(crate) test_templates: Vec<TestTemplateAggregateDto>,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateRevisionEnvelopeDto {
    pub(crate) revision: TestTemplateRevisionDto,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateRevisionListDto {
    pub(crate) template_id: String,
    pub(crate) revisions: Vec<TestTemplateRevisionDto>,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateOperationResultDto {
    pub(crate) operation: String,
    pub(crate) operation_id: String,
    pub(crate) replayed: bool,
    pub(crate) test_template: TestTemplateAggregateDto,
    pub(crate) revision: TestTemplateRevisionDto,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateAuditEventsDto {
    pub(crate) template_id: String,
    pub(crate) audit_events: Vec<TestTemplateAuditEventDto>,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateAuditEventDto {
    pub(crate) audit_id: u64,
    pub(crate) template_id: String,
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
pub(crate) struct TestTemplateDefinitionValidationDto {
    pub(crate) valid: bool,
    pub(crate) issues: Vec<TestTemplateDefinitionValidationIssueDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) definition_schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) definition_checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) canonical_json: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateDefinitionValidationIssueDto {
    pub(crate) severity: String,
    pub(crate) code: String,
    pub(crate) path: String,
    pub(crate) message: String,
}
