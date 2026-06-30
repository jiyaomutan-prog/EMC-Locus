use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub(crate) struct TestTemplateDto {
    pub(crate) template_id: String,
    pub(crate) template_revision: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) category_code: String,
    pub(crate) measurement_axis: String,
    pub(crate) method_code: Option<String>,
    pub(crate) method_revision: Option<String>,
    pub(crate) status: String,
    pub(crate) variables: Value,
    pub(crate) lock_policy: Value,
    pub(crate) instrumentation_chain: Value,
    pub(crate) sequence: Value,
    pub(crate) limits: Value,
    pub(crate) post_processing: Value,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateEnvelopeDto {
    pub(crate) test_template: TestTemplateDto,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateListDto {
    pub(crate) test_templates: Vec<TestTemplateDto>,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateOperationResultDto {
    pub(crate) operation: String,
    pub(crate) operation_id: String,
    pub(crate) replayed: bool,
    pub(crate) test_template: TestTemplateDto,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateAuditEventsDto {
    pub(crate) template_id: String,
    pub(crate) audit_events: Vec<TestTemplateAuditEventDto>,
}

#[derive(Serialize)]
pub(crate) struct TestTemplateAuditEventDto {
    pub(crate) sequence: u64,
    pub(crate) actor: String,
    pub(crate) action: String,
    pub(crate) reason: String,
    pub(crate) operation_id: String,
    pub(crate) correlation_id: String,
    pub(crate) device_id: String,
    pub(crate) base_revision: String,
    pub(crate) resulting_revision: String,
    pub(crate) payload_json: String,
    pub(crate) occurred_at: String,
}
