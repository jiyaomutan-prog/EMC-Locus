use crate::{render_json, AgentError};
use emc_locus_core::measurement_engineering::MeasurementEngineeringAggregateKind;
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use serde_json::json;
use sha2::{Digest, Sha256};

pub(crate) const MEASUREMENT_ENGINEERING_OPERATION_ENTITY_TYPE: &str =
    "measurement_engineering_revision";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MeasurementEngineeringStorageKind {
    pub(crate) aggregate_kind: MeasurementEngineeringAggregateKind,
    pub(crate) identity_table: &'static str,
    pub(crate) revision_table: &'static str,
    pub(crate) id_column: &'static str,
    pub(crate) route_collection_key: &'static str,
    pub(crate) entity_type: &'static str,
}

impl MeasurementEngineeringStorageKind {
    pub(crate) fn from_core(kind: MeasurementEngineeringAggregateKind) -> Self {
        match kind {
            MeasurementEngineeringAggregateKind::SensorDefinition => Self {
                aggregate_kind: kind,
                identity_table: "sensor_definition_identities",
                revision_table: "sensor_definition_revisions",
                id_column: "sensor_definition_id",
                route_collection_key: "sensor_definitions",
                entity_type: "sensor_definition_revision",
            },
            MeasurementEngineeringAggregateKind::ScalingProfile => Self {
                aggregate_kind: kind,
                identity_table: "scaling_profile_identities",
                revision_table: "scaling_profile_revisions",
                id_column: "scaling_profile_id",
                route_collection_key: "scaling_profiles",
                entity_type: "scaling_profile_revision",
            },
            MeasurementEngineeringAggregateKind::EngineeringCurve => Self {
                aggregate_kind: kind,
                identity_table: "engineering_curve_identities",
                revision_table: "engineering_curve_revisions",
                id_column: "engineering_curve_id",
                route_collection_key: "engineering_curves",
                entity_type: "engineering_curve_revision",
            },
            MeasurementEngineeringAggregateKind::DaqChannelProfile => Self {
                aggregate_kind: kind,
                identity_table: "daq_channel_profile_identities",
                revision_table: "daq_channel_profile_revisions",
                id_column: "daq_channel_profile_id",
                route_collection_key: "daq_channel_profiles",
                entity_type: "daq_channel_profile_revision",
            },
            MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe => Self {
                aggregate_kind: kind,
                identity_table: "acquisition_channel_recipe_identities",
                revision_table: "acquisition_channel_recipe_revisions",
                id_column: "acquisition_channel_recipe_id",
                route_collection_key: "acquisition_channel_recipes",
                entity_type: "acquisition_channel_recipe_revision",
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredMeasurementEngineeringIdentity {
    pub(crate) aggregate_kind: MeasurementEngineeringAggregateKind,
    pub(crate) entity_id: String,
    pub(crate) label: String,
    pub(crate) summary_kind: String,
    pub(crate) current_approved_revision_id: Option<String>,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredMeasurementEngineeringRevision {
    pub(crate) aggregate_kind: MeasurementEngineeringAggregateKind,
    pub(crate) revision_id: String,
    pub(crate) entity_id: String,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<String>,
    pub(crate) status: String,
    pub(crate) definition_schema_version: String,
    pub(crate) definition_json: String,
    pub(crate) definition_checksum: String,
    pub(crate) label: String,
    pub(crate) summary_kind: String,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) submitted_at: Option<String>,
    pub(crate) approved_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredMeasurementEngineeringAggregate {
    pub(crate) identity: StoredMeasurementEngineeringIdentity,
    pub(crate) current_approved_revision: Option<StoredMeasurementEngineeringRevision>,
    pub(crate) latest_revision: Option<StoredMeasurementEngineeringRevision>,
    pub(crate) active_draft_revision: Option<StoredMeasurementEngineeringRevision>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredMeasurementEngineeringAuditEvent {
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
    pub(crate) payload_json: String,
    pub(crate) occurred_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredMeasurementEngineeringOperation {
    pub(crate) operation_id: String,
    pub(crate) aggregate_kind: String,
    pub(crate) entity_id: String,
    pub(crate) revision_id: Option<String>,
    pub(crate) action: String,
    pub(crate) actor: String,
    pub(crate) device_id: String,
    pub(crate) correlation_id: String,
    pub(crate) old_revision_id: Option<String>,
    pub(crate) new_revision_id: Option<String>,
    pub(crate) old_definition_checksum: Option<String>,
    pub(crate) new_definition_checksum: Option<String>,
    pub(crate) payload_checksum: String,
}

pub(crate) struct NewMeasurementEngineeringIdentityRecord<'a> {
    pub(crate) entity_id: &'a str,
    pub(crate) label: &'a str,
    pub(crate) summary_kind: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct NewMeasurementEngineeringRevisionRecord<'a> {
    pub(crate) revision_id: &'a str,
    pub(crate) entity_id: &'a str,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<&'a str>,
    pub(crate) status: &'a str,
    pub(crate) definition_schema_version: &'a str,
    pub(crate) definition_json: &'a str,
    pub(crate) definition_checksum: &'a str,
    pub(crate) label: &'a str,
    pub(crate) summary_kind: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct UpdateMeasurementEngineeringRevisionDefinitionInput<'a> {
    pub(crate) entity_id: &'a str,
    pub(crate) revision_id: &'a str,
    pub(crate) expected_definition_checksum: &'a str,
    pub(crate) definition_schema_version: &'a str,
    pub(crate) definition_json: &'a str,
    pub(crate) definition_checksum: &'a str,
    pub(crate) label: &'a str,
    pub(crate) summary_kind: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct UpdateMeasurementEngineeringRevisionStatusInput<'a> {
    pub(crate) entity_id: &'a str,
    pub(crate) revision_id: &'a str,
    pub(crate) expected_current_status: &'a str,
    pub(crate) status: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct MeasurementEngineeringAuditEventInput<'a> {
    pub(crate) aggregate_kind: &'a str,
    pub(crate) entity_id: &'a str,
    pub(crate) revision_id: Option<&'a str>,
    pub(crate) action: &'a str,
    pub(crate) actor: &'a str,
    pub(crate) reason: &'a str,
    pub(crate) operation_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) old_revision_id: Option<&'a str>,
    pub(crate) new_revision_id: Option<&'a str>,
    pub(crate) old_definition_checksum: Option<&'a str>,
    pub(crate) new_definition_checksum: Option<&'a str>,
    pub(crate) payload_json: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct MeasurementEngineeringOperationFingerprintInput<'a> {
    pub(crate) aggregate_kind: &'a str,
    pub(crate) entity_type: &'a str,
    pub(crate) entity_id: &'a str,
    pub(crate) revision_id: Option<&'a str>,
    pub(crate) action: &'a str,
    pub(crate) actor: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) old_revision_id: Option<&'a str>,
    pub(crate) new_revision_id: Option<&'a str>,
    pub(crate) old_definition_checksum: Option<&'a str>,
    pub(crate) new_definition_checksum: Option<&'a str>,
    pub(crate) payload_json: &'a str,
}

pub(crate) struct MeasurementEngineeringSyncOperationInput<'a> {
    pub(crate) operation_id: &'a str,
    pub(crate) entity_type: &'a str,
    pub(crate) entity_id: &'a str,
    pub(crate) operation_kind: &'a str,
    pub(crate) base_revision: &'a str,
    pub(crate) resulting_revision: &'a str,
    pub(crate) actor_id: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) payload_json: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) fn required_measurement_engineering_tables() -> [&'static str; 11] {
    [
        "sensor_definition_identities",
        "sensor_definition_revisions",
        "scaling_profile_identities",
        "scaling_profile_revisions",
        "engineering_curve_identities",
        "engineering_curve_revisions",
        "daq_channel_profile_identities",
        "daq_channel_profile_revisions",
        "acquisition_channel_recipe_identities",
        "acquisition_channel_recipe_revisions",
        "measurement_engineering_audit_events",
    ]
}

pub(crate) fn load_measurement_engineering_identity(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    entity_id: &str,
) -> Result<Option<StoredMeasurementEngineeringIdentity>, AgentError> {
    let sql = format!(
        "SELECT {id}, label, summary_kind, current_approved_revision_id, created_by, created_at, updated_at \
         FROM {table} WHERE {id} = ?1",
        id = kind.id_column,
        table = kind.identity_table
    );
    connection
        .query_row(&sql, params![entity_id], |row| identity_from_row(row, kind))
        .optional()
        .map_err(|error| AgentError::new("measurement_engineering_query_failed", error.to_string()))
}

pub(crate) fn list_measurement_engineering_identities(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
) -> Result<Vec<StoredMeasurementEngineeringIdentity>, AgentError> {
    let sql = format!(
        "SELECT {id}, label, summary_kind, current_approved_revision_id, created_by, created_at, updated_at \
         FROM {table} ORDER BY summary_kind, label, {id}",
        id = kind.id_column,
        table = kind.identity_table
    );
    let mut statement = connection.prepare(&sql).map_err(|error| {
        AgentError::new("measurement_engineering_query_failed", error.to_string())
    })?;
    let rows = statement
        .query_map([], |row| identity_from_row(row, kind))
        .map_err(|error| {
            AgentError::new("measurement_engineering_query_failed", error.to_string())
        })?;
    let mut identities = Vec::new();
    for row in rows {
        identities.push(row.map_err(|error| {
            AgentError::new("measurement_engineering_query_failed", error.to_string())
        })?);
    }
    Ok(identities)
}

fn identity_from_row(
    row: &Row<'_>,
    kind: MeasurementEngineeringStorageKind,
) -> rusqlite::Result<StoredMeasurementEngineeringIdentity> {
    Ok(StoredMeasurementEngineeringIdentity {
        aggregate_kind: kind.aggregate_kind,
        entity_id: row.get(0)?,
        label: row.get(1)?,
        summary_kind: row.get(2)?,
        current_approved_revision_id: row.get(3)?,
        created_by: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub(crate) fn load_measurement_engineering_revision(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    entity_id: &str,
    revision_id: &str,
) -> Result<Option<StoredMeasurementEngineeringRevision>, AgentError> {
    let sql = revision_select_sql(
        kind,
        &format!("WHERE {} = ?1 AND revision_id = ?2", kind.id_column),
    );
    connection
        .query_row(&sql, params![entity_id, revision_id], |row| {
            revision_from_row(row, kind)
        })
        .optional()
        .map_err(|error| {
            AgentError::new(
                "measurement_engineering_revision_query_failed",
                error.to_string(),
            )
        })
}

pub(crate) fn load_current_approved_measurement_engineering_revision(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    identity: &StoredMeasurementEngineeringIdentity,
) -> Result<Option<StoredMeasurementEngineeringRevision>, AgentError> {
    if let Some(revision_id) = identity.current_approved_revision_id.as_deref() {
        return load_measurement_engineering_revision(
            connection,
            kind,
            &identity.entity_id,
            revision_id,
        );
    }
    Ok(None)
}

pub(crate) fn load_latest_measurement_engineering_revision(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    entity_id: &str,
) -> Result<Option<StoredMeasurementEngineeringRevision>, AgentError> {
    let sql = revision_select_sql(
        kind,
        &format!(
            "WHERE {} = ?1 ORDER BY revision_number DESC LIMIT 1",
            kind.id_column
        ),
    );
    connection
        .query_row(&sql, params![entity_id], |row| revision_from_row(row, kind))
        .optional()
        .map_err(|error| {
            AgentError::new(
                "measurement_engineering_revision_query_failed",
                error.to_string(),
            )
        })
}

pub(crate) fn load_active_draft_measurement_engineering_revision(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    entity_id: &str,
) -> Result<Option<StoredMeasurementEngineeringRevision>, AgentError> {
    let sql = revision_select_sql(
        kind,
        &format!("WHERE {} = ?1 AND status = 'draft' LIMIT 1", kind.id_column),
    );
    connection
        .query_row(&sql, params![entity_id], |row| revision_from_row(row, kind))
        .optional()
        .map_err(|error| {
            AgentError::new(
                "measurement_engineering_revision_query_failed",
                error.to_string(),
            )
        })
}

pub(crate) fn list_approved_measurement_engineering_revisions(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    entity_id: &str,
) -> Result<Vec<StoredMeasurementEngineeringRevision>, AgentError> {
    let sql = revision_select_sql(
        kind,
        &format!(
            "WHERE {} = ?1 AND status = 'approved' ORDER BY revision_number",
            kind.id_column
        ),
    );
    query_revisions(connection, kind, &sql, &[entity_id])
}

pub(crate) fn list_measurement_engineering_revisions(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    entity_id: &str,
) -> Result<Vec<StoredMeasurementEngineeringRevision>, AgentError> {
    let sql = revision_select_sql(
        kind,
        &format!("WHERE {} = ?1 ORDER BY revision_number", kind.id_column),
    );
    query_revisions(connection, kind, &sql, &[entity_id])
}

fn query_revisions(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    sql: &str,
    values: &[&str],
) -> Result<Vec<StoredMeasurementEngineeringRevision>, AgentError> {
    let mut statement = connection.prepare(sql).map_err(|error| {
        AgentError::new(
            "measurement_engineering_revision_query_failed",
            error.to_string(),
        )
    })?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values), |row| {
            revision_from_row(row, kind)
        })
        .map_err(|error| {
            AgentError::new(
                "measurement_engineering_revision_query_failed",
                error.to_string(),
            )
        })?;
    let mut revisions = Vec::new();
    for row in rows {
        revisions.push(row.map_err(|error| {
            AgentError::new(
                "measurement_engineering_revision_query_failed",
                error.to_string(),
            )
        })?);
    }
    Ok(revisions)
}

fn revision_select_sql(kind: MeasurementEngineeringStorageKind, suffix: &str) -> String {
    format!(
        "SELECT revision_id, {id}, revision_number, parent_revision_id, status, \
         definition_schema_version, definition_json, definition_checksum, label, summary_kind, \
         created_by, created_at, updated_at, submitted_at, approved_at FROM {table} {suffix}",
        id = kind.id_column,
        table = kind.revision_table
    )
}

fn revision_from_row(
    row: &Row<'_>,
    kind: MeasurementEngineeringStorageKind,
) -> rusqlite::Result<StoredMeasurementEngineeringRevision> {
    Ok(StoredMeasurementEngineeringRevision {
        aggregate_kind: kind.aggregate_kind,
        revision_id: row.get(0)?,
        entity_id: row.get(1)?,
        revision_number: row.get(2)?,
        parent_revision_id: row.get(3)?,
        status: row.get(4)?,
        definition_schema_version: row.get(5)?,
        definition_json: row.get(6)?,
        definition_checksum: row.get(7)?,
        label: row.get(8)?,
        summary_kind: row.get(9)?,
        created_by: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        submitted_at: row.get(13)?,
        approved_at: row.get(14)?,
    })
}

pub(crate) fn next_measurement_engineering_revision_number(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    entity_id: &str,
) -> Result<u32, AgentError> {
    let sql = format!(
        "SELECT COALESCE(MAX(revision_number), 0) + 1 FROM {table} WHERE {id} = ?1",
        table = kind.revision_table,
        id = kind.id_column
    );
    connection
        .query_row(&sql, params![entity_id], |row| row.get::<_, u32>(0))
        .map_err(|error| {
            AgentError::new(
                "measurement_engineering_revision_query_failed",
                error.to_string(),
            )
        })
}

pub(crate) fn insert_measurement_engineering_identity(
    transaction: &Transaction<'_>,
    kind: MeasurementEngineeringStorageKind,
    input: NewMeasurementEngineeringIdentityRecord<'_>,
) -> Result<(), AgentError> {
    let sql = format!(
        "INSERT INTO {table} ({id}, label, summary_kind, created_by, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        table = kind.identity_table,
        id = kind.id_column
    );
    transaction
        .execute(
            &sql,
            params![
                input.entity_id,
                input.label,
                input.summary_kind,
                input.created_by,
                input.timestamp
            ],
        )
        .map_err(|error| {
            AgentError::new("measurement_engineering_write_failed", error.to_string())
        })?;
    Ok(())
}

pub(crate) fn insert_measurement_engineering_revision(
    transaction: &Transaction<'_>,
    kind: MeasurementEngineeringStorageKind,
    input: NewMeasurementEngineeringRevisionRecord<'_>,
) -> Result<(), AgentError> {
    let sql = format!(
        "INSERT INTO {table} (revision_id, {id}, revision_number, parent_revision_id, status, \
         definition_schema_version, definition_json, definition_checksum, label, summary_kind, created_by, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
        table = kind.revision_table,
        id = kind.id_column
    );
    transaction
        .execute(
            &sql,
            params![
                input.revision_id,
                input.entity_id,
                input.revision_number,
                input.parent_revision_id,
                input.status,
                input.definition_schema_version,
                input.definition_json,
                input.definition_checksum,
                input.label,
                input.summary_kind,
                input.created_by,
                input.timestamp
            ],
        )
        .map_err(|error| {
            AgentError::new(
                "measurement_engineering_revision_write_failed",
                error.to_string(),
            )
        })?;
    Ok(())
}

pub(crate) fn update_measurement_engineering_revision_definition(
    transaction: &Transaction<'_>,
    kind: MeasurementEngineeringStorageKind,
    input: UpdateMeasurementEngineeringRevisionDefinitionInput<'_>,
) -> Result<usize, AgentError> {
    let sql = format!(
        "UPDATE {table} SET definition_schema_version = ?4, definition_json = ?5, \
         definition_checksum = ?6, label = ?7, summary_kind = ?8, updated_at = ?9 \
         WHERE {id} = ?1 AND revision_id = ?2 AND status = 'draft' AND definition_checksum = ?3",
        table = kind.revision_table,
        id = kind.id_column
    );
    transaction
        .execute(
            &sql,
            params![
                input.entity_id,
                input.revision_id,
                input.expected_definition_checksum,
                input.definition_schema_version,
                input.definition_json,
                input.definition_checksum,
                input.label,
                input.summary_kind,
                input.timestamp
            ],
        )
        .map_err(|error| {
            AgentError::new(
                "measurement_engineering_revision_write_failed",
                error.to_string(),
            )
        })
}

pub(crate) fn update_measurement_engineering_revision_status(
    transaction: &Transaction<'_>,
    kind: MeasurementEngineeringStorageKind,
    input: UpdateMeasurementEngineeringRevisionStatusInput<'_>,
) -> Result<usize, AgentError> {
    let (submitted_at, approved_at): (Option<&str>, Option<&str>) = match input.status {
        "under_review" => (Some(input.timestamp), None),
        "approved" => (Some(input.timestamp), Some(input.timestamp)),
        _ => (None, None),
    };
    let sql = format!(
        "UPDATE {table} SET status = ?3, updated_at = ?4, \
         submitted_at = COALESCE(submitted_at, ?5), approved_at = COALESCE(approved_at, ?6) \
         WHERE {id} = ?1 AND revision_id = ?2 AND status = ?7",
        table = kind.revision_table,
        id = kind.id_column
    );
    let updated = transaction
        .execute(
            &sql,
            params![
                input.entity_id,
                input.revision_id,
                input.status,
                input.timestamp,
                submitted_at,
                approved_at,
                input.expected_current_status
            ],
        )
        .map_err(|error| {
            AgentError::new(
                "measurement_engineering_revision_write_failed",
                error.to_string(),
            )
        })?;
    if updated == 0 {
        return Ok(0);
    }
    if input.status == "approved" {
        let identity_sql = format!(
            "UPDATE {table} SET current_approved_revision_id = ?2, updated_at = ?3 WHERE {id} = ?1",
            table = kind.identity_table,
            id = kind.id_column
        );
        transaction
            .execute(
                &identity_sql,
                params![input.entity_id, input.revision_id, input.timestamp],
            )
            .map_err(|error| {
                AgentError::new("measurement_engineering_write_failed", error.to_string())
            })?;
    } else {
        touch_measurement_engineering_identity(
            transaction,
            kind,
            input.entity_id,
            input.timestamp,
        )?;
    }
    Ok(updated)
}

pub(crate) fn supersede_approved_measurement_engineering_revision(
    transaction: &Transaction<'_>,
    kind: MeasurementEngineeringStorageKind,
    entity_id: &str,
    revision_id: &str,
    timestamp: &str,
) -> Result<usize, AgentError> {
    let sql = format!(
        "UPDATE {table} SET status = 'superseded', updated_at = ?3 \
         WHERE {id} = ?1 AND revision_id = ?2 AND status = 'approved'",
        table = kind.revision_table,
        id = kind.id_column
    );
    transaction
        .execute(&sql, params![entity_id, revision_id, timestamp])
        .map_err(|error| {
            AgentError::new(
                "measurement_engineering_revision_write_failed",
                error.to_string(),
            )
        })
}

pub(crate) fn touch_measurement_engineering_identity(
    transaction: &Transaction<'_>,
    kind: MeasurementEngineeringStorageKind,
    entity_id: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    let sql = format!(
        "UPDATE {table} SET updated_at = ?2 WHERE {id} = ?1",
        table = kind.identity_table,
        id = kind.id_column
    );
    transaction
        .execute(&sql, params![entity_id, timestamp])
        .map_err(|error| {
            AgentError::new("measurement_engineering_write_failed", error.to_string())
        })?;
    Ok(())
}

pub(crate) fn existing_measurement_engineering_operation(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<StoredMeasurementEngineeringOperation>, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT operation_id, aggregate_kind, entity_id, revision_id, action, actor, device_id, ",
                "correlation_id, old_revision_id, new_revision_id, old_definition_checksum, ",
                "new_definition_checksum, payload_checksum ",
                "FROM measurement_engineering_audit_events WHERE operation_id = ?1"
            ),
            params![operation_id],
            |row| {
                Ok(StoredMeasurementEngineeringOperation {
                    operation_id: row.get(0)?,
                    aggregate_kind: row.get(1)?,
                    entity_id: row.get(2)?,
                    revision_id: row.get(3)?,
                    action: row.get(4)?,
                    actor: row.get(5)?,
                    device_id: row.get(6)?,
                    correlation_id: row.get(7)?,
                    old_revision_id: row.get(8)?,
                    new_revision_id: row.get(9)?,
                    old_definition_checksum: row.get(10)?,
                    new_definition_checksum: row.get(11)?,
                    payload_checksum: row.get(12)?,
                })
            },
        )
        .optional()
        .map_err(|error| {
            AgentError::new(
                "measurement_engineering_audit_query_failed",
                error.to_string(),
            )
        })
}

pub(crate) fn ensure_measurement_engineering_operation_replay(
    operation: &StoredMeasurementEngineeringOperation,
    operation_id: &str,
    expected: MeasurementEngineeringOperationFingerprintInput<'_>,
) -> Result<(), AgentError> {
    let expected_fingerprint = measurement_engineering_operation_fingerprint(&expected);
    if operation.aggregate_kind == expected.aggregate_kind
        && operation.entity_id == expected.entity_id
        && operation.revision_id.as_deref() == expected.revision_id
        && operation.action == expected.action
        && operation.actor == expected.actor
        && operation.device_id == expected.device_id
        && operation.correlation_id == expected.correlation_id
        && operation.old_revision_id.as_deref() == expected.old_revision_id
        && operation.new_revision_id.as_deref() == expected.new_revision_id
        && operation.old_definition_checksum.as_deref() == expected.old_definition_checksum
        && operation.new_definition_checksum.as_deref() == expected.new_definition_checksum
        && operation.payload_checksum == expected_fingerprint
    {
        return Ok(());
    }
    Err(AgentError::with_details(
        "operation_replay_mismatch",
        "operation_id is already used for a different canonical measurement-engineering operation fingerprint",
        json!({
            "operation_id": operation_id,
            "existing_aggregate_kind": operation.aggregate_kind,
            "existing_entity_id": operation.entity_id,
            "existing_revision_id": operation.revision_id,
            "existing_action": operation.action,
            "expected_action": expected.action,
            "expected_fingerprint": expected_fingerprint,
            "stored_fingerprint": operation.payload_checksum,
        }),
    ))
}

pub(crate) fn insert_measurement_engineering_audit_event(
    transaction: &Transaction<'_>,
    input: MeasurementEngineeringAuditEventInput<'_>,
) -> Result<(), AgentError> {
    let checksum = measurement_engineering_operation_fingerprint(
        &MeasurementEngineeringOperationFingerprintInput {
            aggregate_kind: input.aggregate_kind,
            entity_type: MEASUREMENT_ENGINEERING_OPERATION_ENTITY_TYPE,
            entity_id: input.entity_id,
            revision_id: input.revision_id,
            action: input.action,
            actor: input.actor,
            device_id: input.device_id,
            correlation_id: input.correlation_id,
            old_revision_id: input.old_revision_id,
            new_revision_id: input.new_revision_id,
            old_definition_checksum: input.old_definition_checksum,
            new_definition_checksum: input.new_definition_checksum,
            payload_json: input.payload_json,
        },
    );
    transaction
        .execute(
            concat!(
                "INSERT INTO measurement_engineering_audit_events ",
                "(aggregate_kind, entity_id, revision_id, action, actor, reason, old_revision_id, ",
                "new_revision_id, old_definition_checksum, new_definition_checksum, ",
                "operation_id, device_id, correlation_id, payload_json, payload_checksum, occurred_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
            ),
            params![
                input.aggregate_kind,
                input.entity_id,
                input.revision_id,
                input.action,
                input.actor,
                input.reason,
                input.old_revision_id,
                input.new_revision_id,
                input.old_definition_checksum,
                input.new_definition_checksum,
                input.operation_id,
                input.device_id,
                input.correlation_id,
                input.payload_json,
                checksum,
                input.timestamp,
            ],
        )
        .map_err(|error| {
            AgentError::new(
                "measurement_engineering_audit_write_failed",
                error.to_string(),
            )
        })?;
    Ok(())
}

pub(crate) fn insert_measurement_engineering_sync_operation(
    transaction: &Transaction<'_>,
    input: MeasurementEngineeringSyncOperationInput<'_>,
) -> Result<(), AgentError> {
    let checksum = payload_checksum(&render_json(&json!({
        "domain": "equipment",
        "entity_type": input.entity_type,
        "entity_id": input.entity_id,
        "operation_kind": input.operation_kind,
        "base_revision": input.base_revision,
        "resulting_revision": input.resulting_revision,
        "actor_id": input.actor_id,
        "device_id": input.device_id,
        "correlation_id": input.correlation_id,
        "payload": serde_json::from_str::<serde_json::Value>(input.payload_json)
            .expect("canonical measurement-engineering operation payload must be valid JSON"),
    })));
    transaction
        .execute(
            concat!(
                "INSERT INTO sync_db.sync_operations ",
                "(operation_id, domain, entity_type, entity_id, operation_kind, ",
                "base_revision, resulting_revision, actor_id, device_id, correlation_id, ",
                "payload_json, payload_checksum, status, occurred_at, recorded_at) ",
                "VALUES (?1, 'equipment', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'pending', ?12, ?12)"
            ),
            params![
                input.operation_id,
                input.entity_type,
                input.entity_id,
                input.operation_kind,
                input.base_revision,
                input.resulting_revision,
                input.actor_id,
                input.device_id,
                input.correlation_id,
                input.payload_json,
                checksum,
                input.timestamp,
            ],
        )
        .map_err(|error| AgentError::new("sync_outbox_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn load_measurement_engineering_audit_events(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    entity_id: &str,
) -> Result<Vec<StoredMeasurementEngineeringAuditEvent>, AgentError> {
    let mut statement = connection
        .prepare(concat!(
            "SELECT audit_id, aggregate_kind, entity_id, revision_id, action, actor, reason, ",
            "old_revision_id, new_revision_id, old_definition_checksum, ",
            "new_definition_checksum, operation_id, device_id, correlation_id, ",
            "payload_json, occurred_at FROM measurement_engineering_audit_events ",
            "WHERE aggregate_kind = ?1 AND entity_id = ?2 ORDER BY audit_id"
        ))
        .map_err(|error| {
            AgentError::new(
                "measurement_engineering_audit_query_failed",
                error.to_string(),
            )
        })?;
    let rows = statement
        .query_map(params![kind.aggregate_kind.as_str(), entity_id], |row| {
            Ok(StoredMeasurementEngineeringAuditEvent {
                audit_id: row.get(0)?,
                aggregate_kind: row.get(1)?,
                entity_id: row.get(2)?,
                revision_id: row.get(3)?,
                action: row.get(4)?,
                actor: row.get(5)?,
                reason: row.get(6)?,
                old_revision_id: row.get(7)?,
                new_revision_id: row.get(8)?,
                old_definition_checksum: row.get(9)?,
                new_definition_checksum: row.get(10)?,
                operation_id: row.get(11)?,
                device_id: row.get(12)?,
                correlation_id: row.get(13)?,
                payload_json: row.get(14)?,
                occurred_at: row.get(15)?,
            })
        })
        .map_err(|error| {
            AgentError::new(
                "measurement_engineering_audit_query_failed",
                error.to_string(),
            )
        })?;
    let mut events = Vec::new();
    for row in rows {
        events.push(row.map_err(|error| {
            AgentError::new(
                "measurement_engineering_audit_query_failed",
                error.to_string(),
            )
        })?);
    }
    Ok(events)
}

fn measurement_engineering_operation_fingerprint(
    input: &MeasurementEngineeringOperationFingerprintInput<'_>,
) -> String {
    payload_checksum(&render_json(&json!({
        "domain": "equipment",
        "aggregate_kind": input.aggregate_kind,
        "entity_type": input.entity_type,
        "entity_id": input.entity_id,
        "revision_id": input.revision_id,
        "action": input.action,
        "actor": input.actor,
        "device_id": input.device_id,
        "correlation_id": input.correlation_id,
        "old_revision_id": input.old_revision_id,
        "new_revision_id": input.new_revision_id,
        "old_definition_checksum": input.old_definition_checksum,
        "new_definition_checksum": input.new_definition_checksum,
        "payload": serde_json::from_str::<serde_json::Value>(input.payload_json)
            .expect("canonical measurement-engineering operation payload must be valid JSON"),
    })))
}

fn payload_checksum(payload_json: &str) -> String {
    let digest = Sha256::digest(payload_json.as_bytes());
    format!("sha256:{digest:x}")
}
