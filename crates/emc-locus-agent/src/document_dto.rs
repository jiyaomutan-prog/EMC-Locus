use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct AttachedDocumentDto {
    pub(crate) document_id: String,
    pub(crate) classification: String,
    pub(crate) title: String,
    pub(crate) owner_domain: String,
    pub(crate) owner_entity_type: String,
    pub(crate) owner_entity_id: String,
    pub(crate) storage_backend: String,
    pub(crate) storage_uri: String,
    pub(crate) original_filename: String,
    pub(crate) mime_type: String,
    pub(crate) size_bytes: u64,
    pub(crate) sha256: String,
    pub(crate) revision: String,
    pub(crate) applicability: String,
    pub(crate) confidentiality: String,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize)]
pub(crate) struct AttachedDocumentEnvelopeDto {
    pub(crate) document: AttachedDocumentDto,
}

#[derive(Serialize)]
pub(crate) struct AttachedDocumentListDto {
    pub(crate) documents: Vec<AttachedDocumentDto>,
}

#[derive(Serialize)]
pub(crate) struct AttachedDocumentOperationResultDto {
    pub(crate) operation: String,
    pub(crate) operation_id: String,
    pub(crate) replayed: bool,
    pub(crate) document: AttachedDocumentDto,
}

#[derive(Serialize)]
pub(crate) struct DocumentAuditEventsDto {
    pub(crate) document_id: String,
    pub(crate) audit_events: Vec<DocumentAuditEventDto>,
}

#[derive(Serialize)]
pub(crate) struct DocumentAuditEventDto {
    pub(crate) sequence: u64,
    pub(crate) actor: String,
    pub(crate) action: String,
    pub(crate) reason: Option<String>,
    pub(crate) payload_json: String,
    pub(crate) occurred_at: String,
}
