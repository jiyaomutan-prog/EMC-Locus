use crate::{
    project_repository::{
        insert_sync_operation, open_project_connection, table_exists, SyncOperationInput,
    },
    AgentError,
};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredAttachedDocument {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredDocumentAuditEvent {
    pub(crate) sequence: u64,
    pub(crate) actor: String,
    pub(crate) action: String,
    pub(crate) reason: Option<String>,
    pub(crate) payload_json: String,
    pub(crate) occurred_at: String,
}

pub(crate) struct NewAttachedDocumentRecord<'a> {
    pub(crate) document_id: &'a str,
    pub(crate) classification: &'a str,
    pub(crate) title: &'a str,
    pub(crate) owner_domain: &'a str,
    pub(crate) owner_entity_type: &'a str,
    pub(crate) owner_entity_id: &'a str,
    pub(crate) storage_backend: &'a str,
    pub(crate) storage_uri: &'a str,
    pub(crate) original_filename: &'a str,
    pub(crate) mime_type: &'a str,
    pub(crate) size_bytes: u64,
    pub(crate) sha256: &'a str,
    pub(crate) revision: &'a str,
    pub(crate) applicability: &'a str,
    pub(crate) confidentiality: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct DocumentAuditEventInput<'a> {
    pub(crate) document_id: &'a str,
    pub(crate) sequence: u64,
    pub(crate) actor: &'a str,
    pub(crate) action: &'a str,
    pub(crate) reason: Option<&'a str>,
    pub(crate) payload_json: &'a str,
    pub(crate) timestamp: &'a str,
}

#[derive(Default)]
pub(crate) struct DocumentListFilter<'a> {
    pub(crate) owner_domain: Option<&'a str>,
    pub(crate) owner_entity_type: Option<&'a str>,
    pub(crate) owner_entity_id: Option<&'a str>,
}

pub(crate) fn open_document_connection(storage_root: &Path) -> Result<Connection, AgentError> {
    let connection = open_project_connection(storage_root)?;
    ensure_document_tables(&connection)?;
    Ok(connection)
}

fn ensure_document_tables(connection: &Connection) -> Result<(), AgentError> {
    for table in ["attached_documents", "document_audit_events"] {
        if !table_exists(connection, "main", table)? {
            return Err(AgentError::new(
                "storage_not_initialized",
                format!("missing required table main.{table}"),
            ));
        }
    }
    Ok(())
}

pub(crate) fn insert_attached_document(
    transaction: &Transaction<'_>,
    input: NewAttachedDocumentRecord<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "INSERT INTO attached_documents ",
                "(document_id, classification, title, owner_domain, owner_entity_type, owner_entity_id, ",
                "storage_backend, storage_uri, original_filename, mime_type, size_bytes, sha256, revision, ",
                "applicability, confidentiality, created_by, created_at, updated_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?17)"
            ),
            params![
                input.document_id,
                input.classification,
                input.title,
                input.owner_domain,
                input.owner_entity_type,
                input.owner_entity_id,
                input.storage_backend,
                input.storage_uri,
                input.original_filename,
                input.mime_type,
                input.size_bytes as i64,
                input.sha256,
                input.revision,
                input.applicability,
                input.confidentiality,
                input.created_by,
                input.timestamp,
            ],
        )
        .map_err(|error| AgentError::new("document_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn insert_document_audit_event(
    transaction: &Transaction<'_>,
    input: DocumentAuditEventInput<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "INSERT INTO document_audit_events ",
                "(document_id, sequence, actor, action, reason, payload_json, occurred_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
            ),
            params![
                input.document_id,
                input.sequence,
                input.actor,
                input.action,
                input.reason,
                input.payload_json,
                input.timestamp,
            ],
        )
        .map_err(|error| AgentError::new("document_audit_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn insert_document_sync_operation(
    transaction: &Transaction<'_>,
    input: SyncOperationInput<'_>,
) -> Result<(), AgentError> {
    insert_sync_operation(transaction, input)
}

pub(crate) fn load_attached_document(
    connection: &Connection,
    document_id: &str,
) -> Result<Option<StoredAttachedDocument>, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT document_id, classification, title, owner_domain, owner_entity_type, owner_entity_id, ",
                "storage_backend, storage_uri, original_filename, mime_type, size_bytes, sha256, revision, ",
                "applicability, confidentiality, created_by, created_at, updated_at ",
                "FROM attached_documents WHERE document_id = ?1"
            ),
            params![document_id],
            stored_document_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("document_query_failed", error.to_string()))
}

pub(crate) fn list_attached_documents(
    connection: &Connection,
    filter: DocumentListFilter<'_>,
) -> Result<Vec<StoredAttachedDocument>, AgentError> {
    let mut sql = concat!(
        "SELECT document_id, classification, title, owner_domain, owner_entity_type, owner_entity_id, ",
        "storage_backend, storage_uri, original_filename, mime_type, size_bytes, sha256, revision, ",
        "applicability, confidentiality, created_by, created_at, updated_at ",
        "FROM attached_documents"
    )
    .to_owned();
    let mut conditions = Vec::new();
    let mut values = Vec::new();
    if let Some(owner_domain) = filter.owner_domain {
        conditions.push("owner_domain = ?");
        values.push(owner_domain);
    }
    if let Some(owner_entity_type) = filter.owner_entity_type {
        conditions.push("owner_entity_type = ?");
        values.push(owner_entity_type);
    }
    if let Some(owner_entity_id) = filter.owner_entity_id {
        conditions.push("owner_entity_id = ?");
        values.push(owner_entity_id);
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(" ORDER BY owner_domain, owner_entity_type, owner_entity_id, classification, title, revision");

    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AgentError::new("document_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values), stored_document_from_row)
        .map_err(|error| AgentError::new("document_query_failed", error.to_string()))?;
    let mut documents = Vec::new();
    for row in rows {
        documents.push(
            row.map_err(|error| AgentError::new("document_query_failed", error.to_string()))?,
        );
    }
    Ok(documents)
}

pub(crate) fn load_document_audit_events(
    connection: &Connection,
    document_id: &str,
) -> Result<Vec<StoredDocumentAuditEvent>, AgentError> {
    let mut statement = connection
        .prepare(concat!(
            "SELECT sequence, actor, action, reason, payload_json, occurred_at ",
            "FROM document_audit_events WHERE document_id = ?1 ORDER BY sequence"
        ))
        .map_err(|error| AgentError::new("document_audit_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![document_id], |row| {
            Ok(StoredDocumentAuditEvent {
                sequence: row.get(0)?,
                actor: row.get(1)?,
                action: row.get(2)?,
                reason: row.get(3)?,
                payload_json: row.get(4)?,
                occurred_at: row.get(5)?,
            })
        })
        .map_err(|error| AgentError::new("document_audit_query_failed", error.to_string()))?;
    let mut events = Vec::new();
    for row in rows {
        events.push(
            row.map_err(|error| AgentError::new("document_audit_query_failed", error.to_string()))?,
        );
    }
    Ok(events)
}

fn stored_document_from_row(row: &Row<'_>) -> rusqlite::Result<StoredAttachedDocument> {
    let size_bytes: i64 = row.get(10)?;
    Ok(StoredAttachedDocument {
        document_id: row.get(0)?,
        classification: row.get(1)?,
        title: row.get(2)?,
        owner_domain: row.get(3)?,
        owner_entity_type: row.get(4)?,
        owner_entity_id: row.get(5)?,
        storage_backend: row.get(6)?,
        storage_uri: row.get(7)?,
        original_filename: row.get(8)?,
        mime_type: row.get(9)?,
        size_bytes: size_bytes.max(0) as u64,
        sha256: row.get(11)?,
        revision: row.get(12)?,
        applicability: row.get(13)?,
        confidentiality: row.get(14)?,
        created_by: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}
