use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub(crate) struct MeasurementEngineeringIdentityDto {
    pub(crate) aggregate_kind: String,
    pub(crate) entity_id: String,
    pub(crate) label: String,
    pub(crate) summary_kind: String,
    pub(crate) current_approved_revision_id: Option<String>,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize)]
pub(crate) struct MeasurementEngineeringRevisionDto {
    pub(crate) aggregate_kind: String,
    pub(crate) revision_id: String,
    pub(crate) entity_id: String,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<String>,
    pub(crate) status: String,
    pub(crate) definition_schema_version: String,
    pub(crate) definition: Value,
    pub(crate) definition_checksum: String,
    pub(crate) label: String,
    pub(crate) summary_kind: String,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) submitted_at: Option<String>,
    pub(crate) approved_at: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct MeasurementEngineeringAggregateDto {
    pub(crate) identity: MeasurementEngineeringIdentityDto,
    pub(crate) current_approved_revision: Option<MeasurementEngineeringRevisionDto>,
    pub(crate) latest_revision: Option<MeasurementEngineeringRevisionDto>,
    pub(crate) active_draft_revision: Option<MeasurementEngineeringRevisionDto>,
}

#[derive(Serialize)]
pub(crate) struct MeasurementEngineeringListDto {
    pub(crate) aggregate_kind: String,
    pub(crate) collection_key: String,
    pub(crate) items: Vec<MeasurementEngineeringAggregateDto>,
}

#[derive(Serialize)]
pub(crate) struct MeasurementEngineeringEnvelopeDto {
    pub(crate) aggregate_kind: String,
    pub(crate) item: MeasurementEngineeringAggregateDto,
}

#[derive(Serialize)]
pub(crate) struct MeasurementEngineeringRevisionListDto {
    pub(crate) aggregate_kind: String,
    pub(crate) entity_id: String,
    pub(crate) revisions: Vec<MeasurementEngineeringRevisionDto>,
}

#[derive(Serialize)]
pub(crate) struct MeasurementEngineeringRevisionEnvelopeDto {
    pub(crate) aggregate_kind: String,
    pub(crate) revision: MeasurementEngineeringRevisionDto,
}

#[derive(Serialize)]
pub(crate) struct MeasurementEngineeringValidationIssueDto {
    pub(crate) severity: String,
    pub(crate) code: String,
    pub(crate) path: String,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) suggestion: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct MeasurementEngineeringValidationDto {
    pub(crate) valid: bool,
    pub(crate) issues: Vec<MeasurementEngineeringValidationIssueDto>,
    pub(crate) definition_schema_version: Option<String>,
    pub(crate) definition_checksum: Option<String>,
    pub(crate) canonical_json: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct MeasurementEngineeringAuditEventDto {
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
    pub(crate) device_id: String,
    pub(crate) correlation_id: String,
    pub(crate) payload_json: Value,
    pub(crate) occurred_at: String,
}

#[derive(Serialize)]
pub(crate) struct MeasurementEngineeringAuditEventsDto {
    pub(crate) aggregate_kind: String,
    pub(crate) entity_id: String,
    pub(crate) audit_events: Vec<MeasurementEngineeringAuditEventDto>,
}

#[derive(Serialize)]
pub(crate) struct MeasurementEngineeringOperationResultDto {
    pub(crate) operation: String,
    pub(crate) operation_id: String,
    pub(crate) replayed: bool,
    pub(crate) aggregate_kind: String,
    pub(crate) item: MeasurementEngineeringAggregateDto,
    pub(crate) revision: MeasurementEngineeringRevisionDto,
}

#[derive(Serialize)]
pub(crate) struct EngineeringCurveEvaluationDto {
    pub(crate) values: std::collections::BTreeMap<String, f64>,
    pub(crate) axis_values: std::collections::BTreeMap<String, f64>,
    pub(crate) interpolation: String,
    pub(crate) extrapolated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) warning: Option<String>,
    pub(crate) source_revision_id: String,
    pub(crate) source_checksum: String,
}

#[derive(Serialize)]
pub(crate) struct EngineeringCurveEvaluationEnvelopeDto {
    pub(crate) evaluation: EngineeringCurveEvaluationDto,
}
