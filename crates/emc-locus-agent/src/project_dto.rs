use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProjectDto {
    pub(crate) code: String,
    pub(crate) customer_name: String,
    pub(crate) stage: String,
    pub(crate) execution_mode: String,
    pub(crate) created_at: String,
    pub(crate) archived_at: Option<String>,
    pub(crate) revision: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProjectEnvelopeDto {
    pub(crate) project: ProjectDto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProjectListDto {
    pub(crate) projects: Vec<ProjectDto>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProjectOperationResultDto {
    pub(crate) operation: String,
    pub(crate) operation_id: String,
    pub(crate) replayed: bool,
    pub(crate) project: ProjectDto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ReviewItemOperationResultDto {
    pub(crate) operation: String,
    pub(crate) operation_id: String,
    pub(crate) replayed: bool,
    pub(crate) already_completed: bool,
    pub(crate) resulting_revision: String,
    pub(crate) contract_review: ContractReviewDto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ContractReviewEnvelopeDto {
    pub(crate) contract_review: ContractReviewDto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ContractReviewDto {
    pub(crate) project_code: String,
    pub(crate) execution_mode: String,
    pub(crate) required_items: Vec<String>,
    pub(crate) completed_items: Vec<CompletedReviewItemDto>,
    pub(crate) missing_items: Vec<String>,
    pub(crate) complete: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CompletedReviewItemDto {
    pub(crate) item: String,
    pub(crate) completed_by: Option<String>,
    pub(crate) completed_at: Option<String>,
    pub(crate) comment: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AuditEventsDto {
    pub(crate) project_code: String,
    pub(crate) audit_events: Vec<AuditEventDto>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AuditEventDto {
    pub(crate) sequence: u64,
    pub(crate) actor: String,
    pub(crate) action: String,
    pub(crate) reason: Option<String>,
    pub(crate) payload_json: String,
    pub(crate) occurred_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SyncOutboxDto {
    pub(crate) sync_outbox: Vec<SyncOperationDto>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SyncOperationDto {
    pub(crate) operation_id: String,
    pub(crate) domain: String,
    pub(crate) entity_type: String,
    pub(crate) entity_id: String,
    pub(crate) operation_kind: String,
    pub(crate) base_revision: String,
    pub(crate) resulting_revision: String,
    pub(crate) actor_id: String,
    pub(crate) device_id: String,
    pub(crate) correlation_id: String,
    pub(crate) payload_json: String,
    pub(crate) payload_checksum: String,
    pub(crate) status: String,
    pub(crate) occurred_at: String,
    pub(crate) recorded_at: String,
}
