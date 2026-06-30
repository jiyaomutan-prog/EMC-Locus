use crate::{
    document_dto::{
        AttachedDocumentDto, AttachedDocumentEnvelopeDto, AttachedDocumentListDto,
        AttachedDocumentOperationResultDto, DocumentAuditEventDto, DocumentAuditEventsDto,
    },
    document_repository::{
        insert_attached_document, insert_document_audit_event, insert_document_sync_operation,
        list_attached_documents, load_attached_document, load_document_audit_events,
        open_document_connection, DocumentAuditEventInput, DocumentListFilter,
        NewAttachedDocumentRecord, StoredAttachedDocument,
    },
    project_repository::{
        ensure_operation_replay, existing_operation, load_project, OperationFingerprintInput,
        SyncOperationInput,
    },
    render_json, AgentError,
};
use emc_locus_core::{AuditActor, AuditReason, DomainError, ProjectCode, StableId};
use serde_json::json;
use std::path::{Path, PathBuf};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisterAttachedDocumentInput {
    pub document_id: String,
    pub classification: String,
    pub title: String,
    pub owner_domain: String,
    pub owner_entity_type: String,
    pub owner_entity_id: String,
    pub storage_backend: String,
    pub storage_uri: String,
    pub original_filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub revision: String,
    pub applicability: String,
    pub confidentiality: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListAttachedDocumentsInput {
    pub owner_domain: Option<String>,
    pub owner_entity_type: Option<String>,
    pub owner_entity_id: Option<String>,
}

pub fn register_attached_document(
    storage_root: impl Into<PathBuf>,
    input: RegisterAttachedDocumentInput,
) -> Result<String, AgentError> {
    let storage_root = storage_root.into();
    validate_register_input(&input)?;
    let payload_json = document_payload_json(&input);

    let mut connection = open_document_connection(&storage_root)?;
    if let Some(operation) = existing_operation(&connection, &input.operation_id)? {
        ensure_operation_replay(
            &operation,
            &input.operation_id,
            OperationFingerprintInput {
                domain: "project_records",
                entity_type: "attached_document",
                entity_id: &input.document_id,
                operation_kind: "attached_document_registered",
                base_revision: "rev-0000",
                actor_id: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                payload_json: &payload_json,
            },
        )?;
        let document =
            load_attached_document(&connection, &input.document_id)?.ok_or_else(|| {
                AgentError::new(
                    "operation_replay_missing_entity",
                    "operation exists but attached document is missing",
                )
            })?;
        return Ok(document_result_json(
            "attached_document_registered",
            &input.operation_id,
            true,
            &document,
        ));
    }
    if load_attached_document(&connection, &input.document_id)?.is_some() {
        return Err(AgentError::new(
            "attached_document_already_exists",
            format!("attached document already exists: {}", input.document_id),
        ));
    }
    validate_owner_reference(&connection, &input)?;

    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_attached_document(
        &transaction,
        NewAttachedDocumentRecord {
            document_id: &input.document_id,
            classification: &input.classification,
            title: input.title.trim(),
            owner_domain: &input.owner_domain,
            owner_entity_type: &input.owner_entity_type,
            owner_entity_id: &input.owner_entity_id,
            storage_backend: &input.storage_backend,
            storage_uri: input.storage_uri.trim(),
            original_filename: input.original_filename.trim(),
            mime_type: input.mime_type.trim(),
            size_bytes: input.size_bytes,
            sha256: &input.sha256,
            revision: input.revision.trim(),
            applicability: &input.applicability,
            confidentiality: &input.confidentiality,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_document_audit_event(
        &transaction,
        DocumentAuditEventInput {
            document_id: &input.document_id,
            sequence: 1,
            actor: &input.actor,
            action: "attached_document_registered",
            reason: Some(&input.reason),
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    insert_document_sync_operation(
        &transaction,
        SyncOperationInput {
            domain: "project_records",
            entity_type: "attached_document",
            operation_id: &input.operation_id,
            entity_id: &input.document_id,
            operation_kind: "attached_document_registered",
            base_revision: "rev-0000",
            resulting_revision: "rev-0001",
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

    let document = load_attached_document(&connection, &input.document_id)?
        .ok_or_else(|| AgentError::new("document_read_failed", "document could not be reloaded"))?;
    Ok(document_result_json(
        "attached_document_registered",
        &input.operation_id,
        false,
        &document,
    ))
}

pub fn list_documents(
    storage_root: impl AsRef<Path>,
    input: ListAttachedDocumentsInput,
) -> Result<String, AgentError> {
    validate_optional_filter(input.owner_domain.as_deref(), "owner_domain")?;
    validate_optional_filter(input.owner_entity_type.as_deref(), "owner_entity_type")?;
    validate_optional_filter(input.owner_entity_id.as_deref(), "owner_entity_id")?;
    let connection = open_document_connection(storage_root.as_ref())?;
    let documents = list_attached_documents(
        &connection,
        DocumentListFilter {
            owner_domain: input.owner_domain.as_deref(),
            owner_entity_type: input.owner_entity_type.as_deref(),
            owner_entity_id: input.owner_entity_id.as_deref(),
        },
    )?;
    Ok(render_json(&AttachedDocumentListDto {
        documents: documents.iter().map(document_dto).collect(),
    }))
}

pub fn get_document(
    storage_root: impl AsRef<Path>,
    document_id: &str,
) -> Result<String, AgentError> {
    validate_stable_id(document_id, "document_id")?;
    let connection = open_document_connection(storage_root.as_ref())?;
    let document = load_attached_document(&connection, document_id)?
        .ok_or_else(|| AgentError::new("attached_document_not_found", "document does not exist"))?;
    Ok(render_json(&AttachedDocumentEnvelopeDto {
        document: document_dto(&document),
    }))
}

pub fn list_document_audit_events(
    storage_root: impl AsRef<Path>,
    document_id: &str,
) -> Result<String, AgentError> {
    validate_stable_id(document_id, "document_id")?;
    let connection = open_document_connection(storage_root.as_ref())?;
    load_attached_document(&connection, document_id)?
        .ok_or_else(|| AgentError::new("attached_document_not_found", "document does not exist"))?;
    let audit_events = load_document_audit_events(&connection, document_id)?;
    Ok(render_json(&DocumentAuditEventsDto {
        document_id: document_id.to_owned(),
        audit_events: audit_events
            .iter()
            .map(|event| DocumentAuditEventDto {
                sequence: event.sequence,
                actor: event.actor.clone(),
                action: event.action.clone(),
                reason: event.reason.clone(),
                payload_json: event.payload_json.clone(),
                occurred_at: event.occurred_at.clone(),
            })
            .collect(),
    }))
}

fn validate_register_input(input: &RegisterAttachedDocumentInput) -> Result<(), AgentError> {
    validate_stable_id(&input.document_id, "document_id")?;
    validate_enum(
        &input.classification,
        "classification",
        &[
            "client_document",
            "standard_reference",
            "calibration_certificate",
            "datasheet",
            "worksheet",
            "script",
            "report",
            "photo",
            "drawing",
            "contract",
            "dataset_manifest",
            "other",
        ],
    )?;
    validate_non_empty(&input.title, "title")?;
    validate_enum(
        &input.owner_domain,
        "owner_domain",
        &[
            "locus_metrology",
            "locus_lab_management",
            "locus_test_station",
            "shared",
        ],
    )?;
    validate_stable_id(&input.owner_entity_type, "owner_entity_type")?;
    validate_stable_id(&input.owner_entity_id, "owner_entity_id")?;
    validate_enum(
        &input.storage_backend,
        "storage_backend",
        &["object_store", "local_path", "external_reference"],
    )?;
    validate_non_empty(&input.storage_uri, "storage_uri")?;
    validate_non_empty(&input.original_filename, "original_filename")?;
    validate_non_empty(&input.mime_type, "mime_type")?;
    validate_sha256(&input.sha256)?;
    validate_non_empty(&input.revision, "revision")?;
    validate_enum(
        &input.applicability,
        "applicability",
        &["draft", "applicable", "superseded", "archival"],
    )?;
    validate_enum(
        &input.confidentiality,
        "confidentiality",
        &["internal", "customer_visible", "restricted"],
    )?;
    AuditActor::parse(input.actor.clone()).map_err(domain_error)?;
    AuditReason::parse(input.reason.clone()).map_err(domain_error)?;
    validate_stable_id(&input.operation_id, "operation_id")?;
    validate_stable_id(&input.correlation_id, "correlation_id")?;
    validate_stable_id(&input.device_id, "device_id")?;
    Ok(())
}

fn validate_owner_reference(
    connection: &rusqlite::Connection,
    input: &RegisterAttachedDocumentInput,
) -> Result<(), AgentError> {
    if input.owner_domain == "locus_lab_management" && input.owner_entity_type == "project" {
        ProjectCode::parse(input.owner_entity_id.clone()).map_err(domain_error)?;
        load_project(connection, &input.owner_entity_id)?.ok_or_else(|| {
            AgentError::new(
                "document_owner_not_found",
                "project owner does not exist for attached document",
            )
        })?;
    }
    Ok(())
}

fn validate_optional_filter(value: Option<&str>, field: &'static str) -> Result<(), AgentError> {
    if let Some(value) = value {
        validate_non_empty(value, field)?;
    }
    Ok(())
}

fn validate_stable_id(value: &str, field: &'static str) -> Result<(), AgentError> {
    StableId::parse(value.to_owned()).map_err(|error| {
        let mut agent_error = domain_error(error);
        agent_error.message = format!("{field} must be a non-empty stable ASCII token");
        agent_error
    })?;
    Ok(())
}

fn validate_non_empty(value: &str, field: &'static str) -> Result<(), AgentError> {
    if value.trim().is_empty() {
        return Err(AgentError::with_details(
            "invalid_attached_document",
            format!("{field} is required"),
            json!({ "field": field }),
        ));
    }
    Ok(())
}

fn validate_enum(value: &str, field: &'static str, allowed: &[&str]) -> Result<(), AgentError> {
    if allowed.contains(&value) {
        return Ok(());
    }
    Err(AgentError::with_details(
        "invalid_attached_document",
        format!("{field} has an unsupported value"),
        json!({ "field": field, "value": value, "allowed": allowed }),
    ))
}

fn validate_sha256(value: &str) -> Result<(), AgentError> {
    if value.len() == 64 && value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(AgentError::with_details(
        "invalid_attached_document",
        "sha256 must be 64 hexadecimal characters without prefix",
        json!({ "field": "sha256" }),
    ))
}

fn domain_error(error: DomainError) -> AgentError {
    match error {
        DomainError::EmptyAuditActor => AgentError::new("invalid_actor", "actor is required"),
        DomainError::EmptyAuditReason => AgentError::new("invalid_reason", "reason is required"),
        DomainError::EmptyProjectCode => {
            AgentError::new("invalid_project_code", "project code is required")
        }
        DomainError::InvalidProjectCode(value) => AgentError::new(
            "invalid_project_code",
            format!("invalid project code: {value}"),
        ),
        other => AgentError::new("domain_error", format!("{other:?}")),
    }
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_error", error.to_string()))
}

fn document_payload_json(input: &RegisterAttachedDocumentInput) -> String {
    render_json(&json!({
        "document_id": input.document_id,
        "classification": input.classification,
        "title": input.title.trim(),
        "owner_domain": input.owner_domain,
        "owner_entity_type": input.owner_entity_type,
        "owner_entity_id": input.owner_entity_id,
        "storage_backend": input.storage_backend,
        "storage_uri": input.storage_uri.trim(),
        "original_filename": input.original_filename.trim(),
        "mime_type": input.mime_type.trim(),
        "size_bytes": input.size_bytes,
        "sha256": input.sha256,
        "revision": input.revision.trim(),
        "applicability": input.applicability,
        "confidentiality": input.confidentiality,
    }))
}

fn document_result_json(
    operation: &str,
    operation_id: &str,
    replayed: bool,
    document: &StoredAttachedDocument,
) -> String {
    render_json(&AttachedDocumentOperationResultDto {
        operation: operation.to_owned(),
        operation_id: operation_id.to_owned(),
        replayed,
        document: document_dto(document),
    })
}

fn document_dto(document: &StoredAttachedDocument) -> AttachedDocumentDto {
    AttachedDocumentDto {
        document_id: document.document_id.clone(),
        classification: document.classification.clone(),
        title: document.title.clone(),
        owner_domain: document.owner_domain.clone(),
        owner_entity_type: document.owner_entity_type.clone(),
        owner_entity_id: document.owner_entity_id.clone(),
        storage_backend: document.storage_backend.clone(),
        storage_uri: document.storage_uri.clone(),
        original_filename: document.original_filename.clone(),
        mime_type: document.mime_type.clone(),
        size_bytes: document.size_bytes,
        sha256: document.sha256.clone(),
        revision: document.revision.clone(),
        applicability: document.applicability.clone(),
        confidentiality: document.confidentiality.clone(),
        created_by: document.created_by.clone(),
        created_at: document.created_at.clone(),
        updated_at: document.updated_at.clone(),
    }
}
