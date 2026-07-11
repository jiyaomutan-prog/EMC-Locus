use crate::{
    render_json,
    sqlite_policy::{enforce_project_slice_journal_mode, AttachedDatabase},
    AgentError,
};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredEquipmentModelIdentity {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredEquipmentModelRevision {
    pub(crate) revision_id: String,
    pub(crate) equipment_model_id: String,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<String>,
    pub(crate) status: String,
    pub(crate) definition_schema_version: String,
    pub(crate) definition_json: String,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredDriverProfileIdentity {
    pub(crate) driver_profile_id: String,
    pub(crate) equipment_model_id: String,
    pub(crate) label: String,
    pub(crate) current_approved_revision_id: Option<String>,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredDriverProfileRevision {
    pub(crate) revision_id: String,
    pub(crate) driver_profile_id: String,
    pub(crate) equipment_model_id: String,
    pub(crate) supported_model_revision_id: String,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<String>,
    pub(crate) status: String,
    pub(crate) definition_schema_version: String,
    pub(crate) definition_json: String,
    pub(crate) definition_checksum: String,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) submitted_at: Option<String>,
    pub(crate) approved_at: Option<String>,
    pub(crate) action_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredEquipmentAuditEvent {
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
pub(crate) struct StoredEquipmentOperation {
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StoredEquipmentRegistryItem {
    pub(crate) code: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) recommended_equipment_classes: Option<String>,
    pub(crate) recommended_functional_roles: Option<String>,
    pub(crate) compatible_signal_domains: Option<String>,
    pub(crate) compatible_directionalities: Option<String>,
    pub(crate) deprecated: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StoredEquipmentClassificationPreset {
    pub(crate) preset_id: String,
    pub(crate) category_label: String,
    pub(crate) function_description: String,
    pub(crate) example_label: String,
    pub(crate) default_equipment_class: String,
    pub(crate) default_functional_role: String,
    pub(crate) default_signal_domains: String,
    pub(crate) default_technology_tags: String,
    pub(crate) notes: String,
    pub(crate) deprecated: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StoredEquipmentClassificationPresetPort {
    pub(crate) port_order: u32,
    pub(crate) port_id: String,
    pub(crate) label: String,
    pub(crate) directionality: String,
    pub(crate) flow_role: String,
    pub(crate) signal_domain: String,
    pub(crate) connector_type: Option<String>,
    pub(crate) technology_tags: String,
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

#[derive(Default)]
pub(crate) struct EquipmentModelListFilter<'a> {
    pub(crate) manufacturer: Option<&'a str>,
    pub(crate) equipment_class: Option<&'a str>,
    pub(crate) category_code: Option<&'a str>,
    pub(crate) functional_role: Option<&'a str>,
    pub(crate) signal_domain: Option<&'a str>,
    pub(crate) technology_tag: Option<&'a str>,
    pub(crate) status: Option<&'a str>,
    pub(crate) search: Option<&'a str>,
}

#[derive(Default)]
pub(crate) struct DriverProfileListFilter<'a> {
    pub(crate) equipment_model_id: Option<&'a str>,
    pub(crate) status: Option<&'a str>,
    pub(crate) search: Option<&'a str>,
}

pub(crate) struct NewEquipmentModelIdentityRecord<'a> {
    pub(crate) equipment_model_id: &'a str,
    pub(crate) manufacturer: &'a str,
    pub(crate) model_name: &'a str,
    pub(crate) variant: Option<&'a str>,
    pub(crate) equipment_class: &'a str,
    pub(crate) category_code: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct NewEquipmentModelRevisionRecord<'a> {
    pub(crate) revision_id: &'a str,
    pub(crate) equipment_model_id: &'a str,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<&'a str>,
    pub(crate) status: &'a str,
    pub(crate) definition_schema_version: &'a str,
    pub(crate) definition_json: &'a str,
    pub(crate) definition_checksum: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) timestamp: &'a str,
    pub(crate) capability_count: u32,
    pub(crate) interface_count: u32,
    pub(crate) signal_port_count: u32,
}

pub(crate) struct EquipmentClassificationSummaryRecord<'a> {
    pub(crate) equipment_model_id: &'a str,
    pub(crate) revision_id: &'a str,
    pub(crate) revision_number: u32,
    pub(crate) status: &'a str,
    pub(crate) manufacturer: &'a str,
    pub(crate) equipment_class: &'a str,
    pub(crate) category_code: &'a str,
    pub(crate) functional_role: &'a str,
    pub(crate) definition_checksum: &'a str,
    pub(crate) signal_domains_json: &'a str,
    pub(crate) technology_tags_json: &'a str,
    pub(crate) signal_domains: &'a [String],
    pub(crate) technology_tags: &'a [String],
    pub(crate) timestamp: &'a str,
}

pub(crate) struct NewDriverProfileIdentityRecord<'a> {
    pub(crate) driver_profile_id: &'a str,
    pub(crate) equipment_model_id: &'a str,
    pub(crate) label: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct NewDriverProfileRevisionRecord<'a> {
    pub(crate) revision_id: &'a str,
    pub(crate) driver_profile_id: &'a str,
    pub(crate) equipment_model_id: &'a str,
    pub(crate) supported_model_revision_id: &'a str,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<&'a str>,
    pub(crate) status: &'a str,
    pub(crate) definition_schema_version: &'a str,
    pub(crate) definition_json: &'a str,
    pub(crate) definition_checksum: &'a str,
    pub(crate) created_by: &'a str,
    pub(crate) timestamp: &'a str,
    pub(crate) action_count: u32,
}

pub(crate) struct UpdateDefinitionInput<'a> {
    pub(crate) revision_id: &'a str,
    pub(crate) expected_definition_checksum: &'a str,
    pub(crate) definition_schema_version: &'a str,
    pub(crate) definition_json: &'a str,
    pub(crate) definition_checksum: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct UpdateModelDefinitionCounts {
    pub(crate) capability_count: u32,
    pub(crate) interface_count: u32,
    pub(crate) signal_port_count: u32,
}

pub(crate) struct UpdateDriverDefinitionCounts {
    pub(crate) action_count: u32,
}

pub(crate) struct UpdateStatusInput<'a> {
    pub(crate) revision_id: &'a str,
    pub(crate) expected_status: &'a str,
    pub(crate) status: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct EquipmentAuditEventInput<'a> {
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

pub(crate) struct EquipmentOperationFingerprintInput<'a> {
    pub(crate) aggregate_kind: &'a str,
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

pub(crate) struct EquipmentSyncOperationInput<'a> {
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

pub(crate) fn open_equipment_connection(storage_root: &Path) -> Result<Connection, AgentError> {
    let database = storage_root.join("equipment.sqlite");
    if !database.exists() {
        return Err(AgentError::new(
            "storage_not_initialized",
            "equipment reads require initialized equipment.sqlite",
        ));
    }
    let connection = Connection::open(&database).map_err(|error| {
        AgentError::new(
            "database_open_error",
            format!("cannot open {}: {error}", database.display()),
        )
    })?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| AgentError::new("database_pragma_error", error.to_string()))?;
    ensure_equipment_tables(&connection)?;
    Ok(connection)
}

pub(crate) fn open_equipment_connection_with_sync(
    storage_root: &Path,
) -> Result<Connection, AgentError> {
    let equipment_database = storage_root.join("equipment.sqlite");
    let sync_database = storage_root.join("sync.sqlite");
    if !equipment_database.exists() || !sync_database.exists() {
        return Err(AgentError::new(
            "storage_not_initialized",
            "equipment writes require initialized equipment.sqlite and sync.sqlite",
        ));
    }
    let connection = Connection::open(&equipment_database).map_err(|error| {
        AgentError::new(
            "database_open_error",
            format!("cannot open {}: {error}", equipment_database.display()),
        )
    })?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| AgentError::new("database_pragma_error", error.to_string()))?;
    let sync_path = sync_database.to_string_lossy().to_string();
    connection
        .execute("ATTACH DATABASE ?1 AS sync_db", params![sync_path])
        .map_err(|error| AgentError::new("database_attach_error", error.to_string()))?;
    enforce_project_slice_journal_mode(&connection, AttachedDatabase::Main, "equipment.sqlite")?;
    enforce_project_slice_journal_mode(&connection, AttachedDatabase::SyncDb, "sync.sqlite")?;
    ensure_equipment_tables(&connection)?;
    ensure_sync_tables(&connection)?;
    Ok(connection)
}

fn ensure_equipment_tables(connection: &Connection) -> Result<(), AgentError> {
    for table in [
        "schema_migrations",
        "repository_metadata",
        "equipment_class_registry",
        "equipment_unit_registry",
        "equipment_model_identities",
        "equipment_model_revisions",
        "equipment_functional_role_registry",
        "equipment_signal_domain_registry",
        "equipment_port_directionality_registry",
        "equipment_flow_role_registry",
        "equipment_technology_tag_registry",
        "equipment_classification_presets",
        "equipment_classification_preset_ports",
        "equipment_model_classification_summaries",
        "equipment_model_signal_domain_summaries",
        "equipment_model_technology_tag_summaries",
        "driver_profile_identities",
        "driver_profile_revisions",
        "equipment_audit_events",
    ] {
        if !table_exists_in_schema(connection, "main", table)? {
            return Err(AgentError::new(
                "storage_not_initialized",
                format!("missing required equipment table {table}"),
            ));
        }
    }
    Ok(())
}

fn ensure_sync_tables(connection: &Connection) -> Result<(), AgentError> {
    if !table_exists_in_schema(connection, "sync_db", "sync_operations")? {
        return Err(AgentError::new(
            "storage_not_initialized",
            "missing required table sync_db.sync_operations",
        ));
    }
    Ok(())
}

pub(crate) fn insert_equipment_model_identity(
    transaction: &Transaction<'_>,
    input: NewEquipmentModelIdentityRecord<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "INSERT INTO equipment_model_identities (
                equipment_model_id, manufacturer, model_name, variant, equipment_class,
                category_code, created_by, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![
                input.equipment_model_id,
                input.manufacturer,
                input.model_name,
                input.variant,
                input.equipment_class,
                input.category_code,
                input.created_by,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("equipment_model_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn insert_equipment_model_revision(
    transaction: &Transaction<'_>,
    input: NewEquipmentModelRevisionRecord<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "INSERT INTO equipment_model_revisions (
                revision_id, equipment_model_id, revision_number, parent_revision_id, status,
                definition_schema_version, definition_json, definition_checksum,
                created_by, created_at, updated_at, capability_count, interface_count,
                signal_port_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?11, ?12, ?13)",
            params![
                input.revision_id,
                input.equipment_model_id,
                input.revision_number,
                input.parent_revision_id,
                input.status,
                input.definition_schema_version,
                input.definition_json,
                input.definition_checksum,
                input.created_by,
                input.timestamp,
                input.capability_count,
                input.interface_count,
                input.signal_port_count
            ],
        )
        .map_err(|error| {
            AgentError::new("equipment_model_revision_write_failed", error.to_string())
        })?;
    Ok(())
}

pub(crate) fn replace_equipment_model_classification_summary(
    transaction: &Transaction<'_>,
    input: EquipmentClassificationSummaryRecord<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "DELETE FROM equipment_model_classification_summaries WHERE equipment_model_id = ?1",
            params![input.equipment_model_id],
        )
        .map_err(|error| {
            AgentError::new(
                "equipment_model_classification_summary_write_failed",
                error.to_string(),
            )
        })?;
    transaction
        .execute(
            "INSERT INTO equipment_model_classification_summaries (
                equipment_model_id, revision_id, revision_number, status, manufacturer,
                equipment_class, category_code, functional_role, definition_checksum,
                signal_domains_json, technology_tags_json, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                input.equipment_model_id,
                input.revision_id,
                input.revision_number,
                input.status,
                input.manufacturer,
                input.equipment_class,
                input.category_code,
                input.functional_role,
                input.definition_checksum,
                input.signal_domains_json,
                input.technology_tags_json,
                input.timestamp
            ],
        )
        .map_err(|error| {
            AgentError::new(
                "equipment_model_classification_summary_write_failed",
                error.to_string(),
            )
        })?;
    for domain in input.signal_domains {
        transaction
            .execute(
                "INSERT INTO equipment_model_signal_domain_summaries (
                    equipment_model_id, signal_domain, revision_id
                ) VALUES (?1, ?2, ?3)",
                params![input.equipment_model_id, domain, input.revision_id],
            )
            .map_err(|error| {
                AgentError::new(
                    "equipment_model_classification_summary_write_failed",
                    error.to_string(),
                )
            })?;
    }
    for tag in input.technology_tags {
        transaction
            .execute(
                "INSERT INTO equipment_model_technology_tag_summaries (
                    equipment_model_id, technology_tag, revision_id
                ) VALUES (?1, ?2, ?3)",
                params![input.equipment_model_id, tag, input.revision_id],
            )
            .map_err(|error| {
                AgentError::new(
                    "equipment_model_classification_summary_write_failed",
                    error.to_string(),
                )
            })?;
    }
    Ok(())
}

pub(crate) fn insert_driver_profile_identity(
    transaction: &Transaction<'_>,
    input: NewDriverProfileIdentityRecord<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "INSERT INTO driver_profile_identities (
                driver_profile_id, equipment_model_id, label, created_by, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![
                input.driver_profile_id,
                input.equipment_model_id,
                input.label,
                input.created_by,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("driver_profile_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn insert_driver_profile_revision(
    transaction: &Transaction<'_>,
    input: NewDriverProfileRevisionRecord<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "INSERT INTO driver_profile_revisions (
                revision_id, driver_profile_id, equipment_model_id, supported_model_revision_id,
                revision_number, parent_revision_id, status, definition_schema_version,
                definition_json, definition_checksum, created_by, created_at, updated_at,
                action_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12, ?13)",
            params![
                input.revision_id,
                input.driver_profile_id,
                input.equipment_model_id,
                input.supported_model_revision_id,
                input.revision_number,
                input.parent_revision_id,
                input.status,
                input.definition_schema_version,
                input.definition_json,
                input.definition_checksum,
                input.created_by,
                input.timestamp,
                input.action_count
            ],
        )
        .map_err(|error| {
            AgentError::new("driver_profile_revision_write_failed", error.to_string())
        })?;
    Ok(())
}

pub(crate) fn list_equipment_model_identities(
    connection: &Connection,
    filter: EquipmentModelListFilter<'_>,
) -> Result<Vec<StoredEquipmentModelIdentity>, AgentError> {
    let mut statement = connection
        .prepare(
            "SELECT DISTINCT i.equipment_model_id, i.manufacturer, i.model_name, i.variant,
                i.equipment_class, i.category_code, i.current_approved_revision_id,
                i.created_by, i.created_at, i.updated_at
             FROM equipment_model_identities i
             LEFT JOIN equipment_model_classification_summaries s
                ON s.equipment_model_id = i.equipment_model_id
             WHERE (?1 IS NULL OR i.manufacturer = ?1)
               AND (?2 IS NULL OR i.equipment_class = ?2)
               AND (?3 IS NULL OR i.category_code = ?3)
               AND (?4 IS NULL OR s.functional_role = ?4)
               AND (
                    ?5 IS NULL OR EXISTS (
                        SELECT 1 FROM equipment_model_signal_domain_summaries sd
                        WHERE sd.equipment_model_id = i.equipment_model_id
                          AND sd.signal_domain = ?5
                    )
               )
               AND (
                    ?6 IS NULL OR EXISTS (
                        SELECT 1 FROM equipment_model_technology_tag_summaries tt
                        WHERE tt.equipment_model_id = i.equipment_model_id
                          AND tt.technology_tag = ?6
                    )
               )
               AND (?7 IS NULL OR s.status = ?7)
               AND (
                    ?8 IS NULL
                    OR lower(
                        i.manufacturer || ' ' || i.model_name || ' ' ||
                        coalesce(i.variant, '') || ' ' || i.category_code || ' ' ||
                        coalesce(s.functional_role, '') || ' ' ||
                        coalesce(s.signal_domains_json, '') || ' ' ||
                        coalesce(s.technology_tags_json, '')
                    ) LIKE '%' || lower(?8) || '%'
               )
             ORDER BY i.manufacturer, i.model_name, i.variant",
        )
        .map_err(|error| AgentError::new("equipment_model_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(
            params![
                filter.manufacturer,
                filter.equipment_class,
                filter.category_code,
                filter.functional_role,
                filter.signal_domain,
                filter.technology_tag,
                filter.status,
                filter.search
            ],
            equipment_model_identity_from_row,
        )
        .map_err(|error| AgentError::new("equipment_model_query_failed", error.to_string()))?;
    collect_rows(rows, "equipment_model_query_failed")
}

pub(crate) fn list_driver_profile_identities(
    connection: &Connection,
    filter: DriverProfileListFilter<'_>,
) -> Result<Vec<StoredDriverProfileIdentity>, AgentError> {
    let mut statement = connection
        .prepare(
            "SELECT DISTINCT i.driver_profile_id, i.equipment_model_id, i.label,
                i.current_approved_revision_id, i.created_by, i.created_at, i.updated_at
             FROM driver_profile_identities i
             LEFT JOIN driver_profile_revisions r
                ON r.driver_profile_id = i.driver_profile_id
             WHERE (?1 IS NULL OR i.equipment_model_id = ?1)
               AND (?2 IS NULL OR r.status = ?2)
               AND (?3 IS NULL OR lower(i.label) LIKE '%' || lower(?3) || '%')
             ORDER BY i.label",
        )
        .map_err(|error| AgentError::new("driver_profile_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(
            params![filter.equipment_model_id, filter.status, filter.search],
            driver_profile_identity_from_row,
        )
        .map_err(|error| AgentError::new("driver_profile_query_failed", error.to_string()))?;
    collect_rows(rows, "driver_profile_query_failed")
}

pub(crate) fn list_equipment_functional_role_registry(
    connection: &Connection,
) -> Result<Vec<StoredEquipmentRegistryItem>, AgentError> {
    list_equipment_registry(
        connection,
        "SELECT role_code, label, description, recommended_equipment_classes,
                NULL, NULL, NULL, deprecated
         FROM equipment_functional_role_registry
         ORDER BY label",
    )
}

pub(crate) fn list_equipment_signal_domain_registry(
    connection: &Connection,
) -> Result<Vec<StoredEquipmentRegistryItem>, AgentError> {
    list_equipment_registry(
        connection,
        "SELECT domain_code, label, description, NULL, recommended_functional_roles,
                NULL, NULL, deprecated
         FROM equipment_signal_domain_registry
         ORDER BY label",
    )
}

pub(crate) fn list_equipment_port_directionality_registry(
    connection: &Connection,
) -> Result<Vec<StoredEquipmentRegistryItem>, AgentError> {
    list_equipment_registry(
        connection,
        "SELECT directionality_code, label, description, NULL, NULL, NULL, NULL, deprecated
         FROM equipment_port_directionality_registry
         ORDER BY label",
    )
}

pub(crate) fn list_equipment_flow_role_registry(
    connection: &Connection,
) -> Result<Vec<StoredEquipmentRegistryItem>, AgentError> {
    list_equipment_registry(
        connection,
        "SELECT flow_role_code, label, description, NULL, NULL,
                compatible_signal_domains, compatible_directionalities, deprecated
         FROM equipment_flow_role_registry
         ORDER BY label",
    )
}

pub(crate) fn list_equipment_technology_tag_registry(
    connection: &Connection,
) -> Result<Vec<StoredEquipmentRegistryItem>, AgentError> {
    list_equipment_registry(
        connection,
        "SELECT tag_code, label, description, NULL, recommended_functional_roles,
                compatible_signal_domains, NULL, deprecated
         FROM equipment_technology_tag_registry
         ORDER BY label",
    )
}

pub(crate) fn list_equipment_classification_presets(
    connection: &Connection,
) -> Result<Vec<StoredEquipmentClassificationPreset>, AgentError> {
    let mut statement = connection
        .prepare(
            "SELECT preset_id, category_label, function_description, example_label,
                default_equipment_class, default_functional_role, default_signal_domains,
                default_technology_tags, notes, deprecated
             FROM equipment_classification_presets
             ORDER BY sort_order, category_label, example_label",
        )
        .map_err(|error| {
            AgentError::new(
                "equipment_classification_preset_query_failed",
                error.to_string(),
            )
        })?;
    let rows = statement
        .query_map([], equipment_classification_preset_from_row)
        .map_err(|error| {
            AgentError::new(
                "equipment_classification_preset_query_failed",
                error.to_string(),
            )
        })?;
    collect_rows(rows, "equipment_classification_preset_query_failed")
}

pub(crate) fn load_equipment_classification_preset(
    connection: &Connection,
    preset_id: &str,
) -> Result<Option<StoredEquipmentClassificationPreset>, AgentError> {
    connection
        .query_row(
            "SELECT preset_id, category_label, function_description, example_label,
                default_equipment_class, default_functional_role, default_signal_domains,
                default_technology_tags, notes, deprecated
             FROM equipment_classification_presets
             WHERE preset_id = ?1",
            params![preset_id],
            equipment_classification_preset_from_row,
        )
        .optional()
        .map_err(|error| {
            AgentError::new(
                "equipment_classification_preset_query_failed",
                error.to_string(),
            )
        })
}

pub(crate) fn list_equipment_classification_preset_ports(
    connection: &Connection,
    preset_id: &str,
) -> Result<Vec<StoredEquipmentClassificationPresetPort>, AgentError> {
    let mut statement = connection
        .prepare(
            "SELECT port_order, port_id, label, directionality, flow_role, signal_domain,
                connector_type, technology_tags, quantity, unit, impedance, frequency_min,
                frequency_max, voltage_max, current_max, power_max, required, comment
             FROM equipment_classification_preset_ports
             WHERE preset_id = ?1
             ORDER BY port_order",
        )
        .map_err(|error| {
            AgentError::new(
                "equipment_classification_preset_port_query_failed",
                error.to_string(),
            )
        })?;
    let rows = statement
        .query_map(
            params![preset_id],
            equipment_classification_preset_port_from_row,
        )
        .map_err(|error| {
            AgentError::new(
                "equipment_classification_preset_port_query_failed",
                error.to_string(),
            )
        })?;
    collect_rows(rows, "equipment_classification_preset_port_query_failed")
}

pub(crate) fn load_equipment_model_identity(
    connection: &Connection,
    equipment_model_id: &str,
) -> Result<Option<StoredEquipmentModelIdentity>, AgentError> {
    connection
        .query_row(
            "SELECT equipment_model_id, manufacturer, model_name, variant, equipment_class,
                category_code, current_approved_revision_id, created_by, created_at, updated_at
             FROM equipment_model_identities WHERE equipment_model_id = ?1",
            params![equipment_model_id],
            equipment_model_identity_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("equipment_model_query_failed", error.to_string()))
}

pub(crate) fn load_driver_profile_identity(
    connection: &Connection,
    driver_profile_id: &str,
) -> Result<Option<StoredDriverProfileIdentity>, AgentError> {
    connection
        .query_row(
            "SELECT driver_profile_id, equipment_model_id, label, current_approved_revision_id,
                created_by, created_at, updated_at
             FROM driver_profile_identities WHERE driver_profile_id = ?1",
            params![driver_profile_id],
            driver_profile_identity_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("driver_profile_query_failed", error.to_string()))
}

pub(crate) fn list_equipment_model_revisions(
    connection: &Connection,
    equipment_model_id: &str,
) -> Result<Vec<StoredEquipmentModelRevision>, AgentError> {
    let mut statement = connection
        .prepare(
            "SELECT revision_id, equipment_model_id, revision_number, parent_revision_id, status,
                definition_schema_version, definition_json, definition_checksum, created_by,
                created_at, updated_at, submitted_at, approved_at, capability_count,
                interface_count, signal_port_count
             FROM equipment_model_revisions
             WHERE equipment_model_id = ?1
             ORDER BY revision_number",
        )
        .map_err(|error| {
            AgentError::new("equipment_model_revision_query_failed", error.to_string())
        })?;
    let rows = statement
        .query_map(
            params![equipment_model_id],
            equipment_model_revision_from_row,
        )
        .map_err(|error| {
            AgentError::new("equipment_model_revision_query_failed", error.to_string())
        })?;
    collect_rows(rows, "equipment_model_revision_query_failed")
}

pub(crate) fn list_driver_profile_revisions(
    connection: &Connection,
    driver_profile_id: &str,
) -> Result<Vec<StoredDriverProfileRevision>, AgentError> {
    let mut statement = connection
        .prepare(
            "SELECT revision_id, driver_profile_id, equipment_model_id, supported_model_revision_id,
                revision_number, parent_revision_id, status, definition_schema_version,
                definition_json, definition_checksum, created_by, created_at, updated_at,
                submitted_at, approved_at, action_count
             FROM driver_profile_revisions
             WHERE driver_profile_id = ?1
             ORDER BY revision_number",
        )
        .map_err(|error| AgentError::new("driver_profile_revision_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![driver_profile_id], driver_profile_revision_from_row)
        .map_err(|error| {
            AgentError::new("driver_profile_revision_query_failed", error.to_string())
        })?;
    collect_rows(rows, "driver_profile_revision_query_failed")
}

pub(crate) fn load_equipment_model_revision(
    connection: &Connection,
    equipment_model_id: &str,
    revision_id: &str,
) -> Result<Option<StoredEquipmentModelRevision>, AgentError> {
    connection
        .query_row(
            "SELECT revision_id, equipment_model_id, revision_number, parent_revision_id, status,
                definition_schema_version, definition_json, definition_checksum, created_by,
                created_at, updated_at, submitted_at, approved_at, capability_count,
                interface_count, signal_port_count
             FROM equipment_model_revisions
             WHERE equipment_model_id = ?1 AND revision_id = ?2",
            params![equipment_model_id, revision_id],
            equipment_model_revision_from_row,
        )
        .optional()
        .map_err(|error| {
            AgentError::new("equipment_model_revision_query_failed", error.to_string())
        })
}

pub(crate) fn load_driver_profile_revision(
    connection: &Connection,
    driver_profile_id: &str,
    revision_id: &str,
) -> Result<Option<StoredDriverProfileRevision>, AgentError> {
    connection
        .query_row(
            "SELECT revision_id, driver_profile_id, equipment_model_id, supported_model_revision_id,
                revision_number, parent_revision_id, status, definition_schema_version,
                definition_json, definition_checksum, created_by, created_at, updated_at,
                submitted_at, approved_at, action_count
             FROM driver_profile_revisions
             WHERE driver_profile_id = ?1 AND revision_id = ?2",
            params![driver_profile_id, revision_id],
            driver_profile_revision_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("driver_profile_revision_query_failed", error.to_string()))
}

pub(crate) fn load_latest_equipment_model_revision(
    connection: &Connection,
    equipment_model_id: &str,
) -> Result<Option<StoredEquipmentModelRevision>, AgentError> {
    connection
        .query_row(
            "SELECT revision_id, equipment_model_id, revision_number, parent_revision_id, status,
                definition_schema_version, definition_json, definition_checksum, created_by,
                created_at, updated_at, submitted_at, approved_at, capability_count,
                interface_count, signal_port_count
             FROM equipment_model_revisions
             WHERE equipment_model_id = ?1
             ORDER BY revision_number DESC LIMIT 1",
            params![equipment_model_id],
            equipment_model_revision_from_row,
        )
        .optional()
        .map_err(|error| {
            AgentError::new("equipment_model_revision_query_failed", error.to_string())
        })
}

pub(crate) fn load_latest_driver_profile_revision(
    connection: &Connection,
    driver_profile_id: &str,
) -> Result<Option<StoredDriverProfileRevision>, AgentError> {
    connection
        .query_row(
            "SELECT revision_id, driver_profile_id, equipment_model_id, supported_model_revision_id,
                revision_number, parent_revision_id, status, definition_schema_version,
                definition_json, definition_checksum, created_by, created_at, updated_at,
                submitted_at, approved_at, action_count
             FROM driver_profile_revisions
             WHERE driver_profile_id = ?1
             ORDER BY revision_number DESC LIMIT 1",
            params![driver_profile_id],
            driver_profile_revision_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("driver_profile_revision_query_failed", error.to_string()))
}

pub(crate) fn load_active_draft_equipment_model_revision(
    connection: &Connection,
    equipment_model_id: &str,
) -> Result<Option<StoredEquipmentModelRevision>, AgentError> {
    load_equipment_model_revision_by_status(connection, equipment_model_id, "draft")
}

pub(crate) fn load_active_draft_driver_profile_revision(
    connection: &Connection,
    driver_profile_id: &str,
) -> Result<Option<StoredDriverProfileRevision>, AgentError> {
    load_driver_profile_revision_by_status(connection, driver_profile_id, "draft")
}

pub(crate) fn load_current_approved_equipment_model_revision(
    connection: &Connection,
    identity: &StoredEquipmentModelIdentity,
) -> Result<Option<StoredEquipmentModelRevision>, AgentError> {
    let Some(revision_id) = identity.current_approved_revision_id.as_deref() else {
        return Ok(None);
    };
    load_equipment_model_revision(connection, &identity.equipment_model_id, revision_id)
}

pub(crate) fn load_current_approved_driver_profile_revision(
    connection: &Connection,
    identity: &StoredDriverProfileIdentity,
) -> Result<Option<StoredDriverProfileRevision>, AgentError> {
    let Some(revision_id) = identity.current_approved_revision_id.as_deref() else {
        return Ok(None);
    };
    load_driver_profile_revision(connection, &identity.driver_profile_id, revision_id)
}

fn load_equipment_model_revision_by_status(
    connection: &Connection,
    equipment_model_id: &str,
    status: &str,
) -> Result<Option<StoredEquipmentModelRevision>, AgentError> {
    connection
        .query_row(
            "SELECT revision_id, equipment_model_id, revision_number, parent_revision_id, status,
                definition_schema_version, definition_json, definition_checksum, created_by,
                created_at, updated_at, submitted_at, approved_at, capability_count,
                interface_count, signal_port_count
             FROM equipment_model_revisions
             WHERE equipment_model_id = ?1 AND status = ?2
             ORDER BY revision_number DESC LIMIT 1",
            params![equipment_model_id, status],
            equipment_model_revision_from_row,
        )
        .optional()
        .map_err(|error| {
            AgentError::new("equipment_model_revision_query_failed", error.to_string())
        })
}

fn load_driver_profile_revision_by_status(
    connection: &Connection,
    driver_profile_id: &str,
    status: &str,
) -> Result<Option<StoredDriverProfileRevision>, AgentError> {
    connection
        .query_row(
            "SELECT revision_id, driver_profile_id, equipment_model_id, supported_model_revision_id,
                revision_number, parent_revision_id, status, definition_schema_version,
                definition_json, definition_checksum, created_by, created_at, updated_at,
                submitted_at, approved_at, action_count
             FROM driver_profile_revisions
             WHERE driver_profile_id = ?1 AND status = ?2
             ORDER BY revision_number DESC LIMIT 1",
            params![driver_profile_id, status],
            driver_profile_revision_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("driver_profile_revision_query_failed", error.to_string()))
}

pub(crate) fn next_equipment_model_revision_number(
    connection: &Connection,
    equipment_model_id: &str,
) -> Result<u32, AgentError> {
    next_revision_number(
        connection,
        "equipment_model_revisions",
        "equipment_model_id",
        equipment_model_id,
        "equipment_model_revision_query_failed",
    )
}

pub(crate) fn next_driver_profile_revision_number(
    connection: &Connection,
    driver_profile_id: &str,
) -> Result<u32, AgentError> {
    next_revision_number(
        connection,
        "driver_profile_revisions",
        "driver_profile_id",
        driver_profile_id,
        "driver_profile_revision_query_failed",
    )
}

fn next_revision_number(
    connection: &Connection,
    table: &str,
    id_column: &str,
    id: &str,
    error_code: &'static str,
) -> Result<u32, AgentError> {
    let sql =
        format!("SELECT COALESCE(MAX(revision_number), 0) + 1 FROM {table} WHERE {id_column} = ?1");
    connection
        .query_row(&sql, params![id], |row| row.get::<_, u32>(0))
        .map_err(|error| AgentError::new(error_code, error.to_string()))
}

pub(crate) fn update_equipment_model_revision_definition(
    transaction: &Transaction<'_>,
    input: UpdateDefinitionInput<'_>,
    counts: UpdateModelDefinitionCounts,
) -> Result<(), AgentError> {
    let updated = transaction
        .execute(
            "UPDATE equipment_model_revisions
             SET definition_schema_version = ?2, definition_json = ?3, definition_checksum = ?4,
                 updated_at = ?5, capability_count = ?6, interface_count = ?7, signal_port_count = ?8
             WHERE revision_id = ?1 AND status = 'draft' AND definition_checksum = ?9",
            params![
                input.revision_id,
                input.definition_schema_version,
                input.definition_json,
                input.definition_checksum,
                input.timestamp,
                counts.capability_count,
                counts.interface_count,
                counts.signal_port_count,
                input.expected_definition_checksum
            ],
        )
        .map_err(|error| AgentError::new("equipment_model_revision_write_failed", error.to_string()))?;
    if updated == 0 {
        return Err(AgentError::new(
            "equipment_definition_checksum_mismatch",
            "draft equipment model revision was changed or is no longer editable",
        ));
    }
    Ok(())
}

pub(crate) fn update_driver_profile_revision_definition(
    transaction: &Transaction<'_>,
    input: UpdateDefinitionInput<'_>,
    counts: UpdateDriverDefinitionCounts,
) -> Result<(), AgentError> {
    let updated = transaction
        .execute(
            "UPDATE driver_profile_revisions
             SET definition_schema_version = ?2, definition_json = ?3, definition_checksum = ?4,
                 updated_at = ?5, action_count = ?6
             WHERE revision_id = ?1 AND status = 'draft' AND definition_checksum = ?7",
            params![
                input.revision_id,
                input.definition_schema_version,
                input.definition_json,
                input.definition_checksum,
                input.timestamp,
                counts.action_count,
                input.expected_definition_checksum
            ],
        )
        .map_err(|error| {
            AgentError::new("driver_profile_revision_write_failed", error.to_string())
        })?;
    if updated == 0 {
        return Err(AgentError::new(
            "equipment_definition_checksum_mismatch",
            "draft driver profile revision was changed or is no longer editable",
        ));
    }
    Ok(())
}

pub(crate) fn update_equipment_model_revision_status(
    transaction: &Transaction<'_>,
    input: UpdateStatusInput<'_>,
) -> Result<(), AgentError> {
    update_revision_status(
        transaction,
        "equipment_model_revisions",
        input,
        "equipment_model_revision_transition_conflict",
    )
}

pub(crate) fn update_driver_profile_revision_status(
    transaction: &Transaction<'_>,
    input: UpdateStatusInput<'_>,
) -> Result<(), AgentError> {
    update_revision_status(
        transaction,
        "driver_profile_revisions",
        input,
        "driver_profile_revision_transition_conflict",
    )
}

fn update_revision_status(
    transaction: &Transaction<'_>,
    table: &str,
    input: UpdateStatusInput<'_>,
    conflict_code: &'static str,
) -> Result<(), AgentError> {
    let submitted_at = (input.status == "under_review").then_some(input.timestamp);
    let approved_at = (input.status == "approved").then_some(input.timestamp);
    let sql = format!(
        "UPDATE {table}
         SET status = ?2, updated_at = ?3,
             submitted_at = COALESCE(submitted_at, ?4),
             approved_at = COALESCE(approved_at, ?5)
         WHERE revision_id = ?1 AND status = ?6"
    );
    let updated = transaction
        .execute(
            &sql,
            params![
                input.revision_id,
                input.status,
                input.timestamp,
                submitted_at,
                approved_at,
                input.expected_status
            ],
        )
        .map_err(|error| AgentError::new("equipment_revision_write_failed", error.to_string()))?;
    if updated == 0 {
        return Err(AgentError::new(
            conflict_code,
            "revision status changed before transition could be applied",
        ));
    }
    Ok(())
}

pub(crate) fn supersede_approved_equipment_model_revision(
    transaction: &Transaction<'_>,
    equipment_model_id: &str,
    revision_id: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "UPDATE equipment_model_revisions
             SET status = 'superseded', updated_at = ?3
             WHERE equipment_model_id = ?1 AND revision_id = ?2 AND status = 'approved'",
            params![equipment_model_id, revision_id, timestamp],
        )
        .map_err(|error| {
            AgentError::new("equipment_model_revision_write_failed", error.to_string())
        })?;
    Ok(())
}

pub(crate) fn supersede_approved_driver_profile_revision(
    transaction: &Transaction<'_>,
    driver_profile_id: &str,
    revision_id: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "UPDATE driver_profile_revisions
             SET status = 'superseded', updated_at = ?3
             WHERE driver_profile_id = ?1 AND revision_id = ?2 AND status = 'approved'",
            params![driver_profile_id, revision_id, timestamp],
        )
        .map_err(|error| {
            AgentError::new("driver_profile_revision_write_failed", error.to_string())
        })?;
    Ok(())
}

pub(crate) fn set_current_approved_equipment_model_revision(
    transaction: &Transaction<'_>,
    equipment_model_id: &str,
    revision_id: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "UPDATE equipment_model_identities
             SET current_approved_revision_id = ?2, updated_at = ?3
             WHERE equipment_model_id = ?1",
            params![equipment_model_id, revision_id, timestamp],
        )
        .map_err(|error| AgentError::new("equipment_model_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn set_current_approved_driver_profile_revision(
    transaction: &Transaction<'_>,
    driver_profile_id: &str,
    revision_id: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "UPDATE driver_profile_identities
             SET current_approved_revision_id = ?2, updated_at = ?3
             WHERE driver_profile_id = ?1",
            params![driver_profile_id, revision_id, timestamp],
        )
        .map_err(|error| AgentError::new("driver_profile_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn touch_equipment_model_identity(
    transaction: &Transaction<'_>,
    equipment_model_id: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "UPDATE equipment_model_identities SET updated_at = ?2 WHERE equipment_model_id = ?1",
            params![equipment_model_id, timestamp],
        )
        .map_err(|error| AgentError::new("equipment_model_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn touch_driver_profile_identity(
    transaction: &Transaction<'_>,
    driver_profile_id: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "UPDATE driver_profile_identities SET updated_at = ?2 WHERE driver_profile_id = ?1",
            params![driver_profile_id, timestamp],
        )
        .map_err(|error| AgentError::new("driver_profile_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn existing_equipment_operation(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<StoredEquipmentOperation>, AgentError> {
    connection
        .query_row(
            "SELECT operation_id, aggregate_kind, entity_id, revision_id, action, actor, device_id,
                correlation_id, old_revision_id, new_revision_id, old_definition_checksum,
                new_definition_checksum, payload_checksum
             FROM equipment_audit_events WHERE operation_id = ?1",
            params![operation_id],
            |row| {
                Ok(StoredEquipmentOperation {
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
        .map_err(|error| AgentError::new("equipment_audit_query_failed", error.to_string()))
}

pub(crate) fn ensure_equipment_operation_replay(
    operation: &StoredEquipmentOperation,
    expected: EquipmentOperationFingerprintInput<'_>,
) -> Result<(), AgentError> {
    let expected_fingerprint = equipment_operation_fingerprint(&expected);
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
        "operation_id is already used for a different canonical equipment operation fingerprint",
        json!({
            "operation_id": operation.operation_id,
            "existing_aggregate_kind": operation.aggregate_kind,
            "existing_entity_id": operation.entity_id,
            "existing_action": operation.action
        }),
    ))
}

pub(crate) fn insert_equipment_audit_event(
    transaction: &Transaction<'_>,
    input: EquipmentAuditEventInput<'_>,
) -> Result<(), AgentError> {
    let checksum = equipment_operation_fingerprint(&EquipmentOperationFingerprintInput {
        aggregate_kind: input.aggregate_kind,
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
    });
    transaction
        .execute(
            "INSERT INTO equipment_audit_events (
                aggregate_kind, entity_id, revision_id, action, actor, reason,
                old_revision_id, new_revision_id, old_definition_checksum,
                new_definition_checksum, operation_id, device_id, correlation_id,
                payload_json, payload_checksum, occurred_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
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
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("equipment_audit_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn insert_equipment_sync_operation(
    transaction: &Transaction<'_>,
    input: EquipmentSyncOperationInput<'_>,
) -> Result<(), AgentError> {
    let payload_value: serde_json::Value = serde_json::from_str(input.payload_json)
        .unwrap_or_else(|_| json!({ "raw": input.payload_json }));
    let payload = render_json(&json!({
        "domain": "equipment",
        "entity_type": input.entity_type,
        "entity_id": input.entity_id,
        "operation_kind": input.operation_kind,
        "payload": payload_value
    }));
    let payload_checksum = sha256_text(&payload);
    transaction
        .execute(
            "INSERT INTO sync_db.sync_operations (
                operation_id, domain, entity_type, entity_id, operation_kind,
                base_revision, resulting_revision, actor_id, device_id, correlation_id,
                payload_json, payload_checksum, status, occurred_at, recorded_at
            ) VALUES (?1, 'equipment', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'pending', ?12, ?12)",
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
                payload,
                payload_checksum,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("equipment_outbox_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn load_equipment_audit_events(
    connection: &Connection,
    aggregate_kind: &str,
    entity_id: &str,
) -> Result<Vec<StoredEquipmentAuditEvent>, AgentError> {
    let mut statement = connection
        .prepare(
            "SELECT audit_id, aggregate_kind, entity_id, revision_id, action, actor, reason,
                old_revision_id, new_revision_id, old_definition_checksum,
                new_definition_checksum, operation_id, device_id, correlation_id,
                payload_json, occurred_at
             FROM equipment_audit_events
             WHERE aggregate_kind = ?1 AND entity_id = ?2
             ORDER BY audit_id",
        )
        .map_err(|error| AgentError::new("equipment_audit_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![aggregate_kind, entity_id], |row| {
            Ok(StoredEquipmentAuditEvent {
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
        .map_err(|error| AgentError::new("equipment_audit_query_failed", error.to_string()))?;
    collect_rows(rows, "equipment_audit_query_failed")
}

pub(crate) fn equipment_model_class_exists(
    connection: &Connection,
    class_code: &str,
) -> Result<bool, AgentError> {
    let count: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM equipment_class_registry WHERE class_code = ?1",
            params![class_code],
            |row| row.get(0),
        )
        .map_err(|error| AgentError::new("equipment_registry_query_failed", error.to_string()))?;
    Ok(count > 0)
}

fn equipment_operation_fingerprint(input: &EquipmentOperationFingerprintInput<'_>) -> String {
    let payload = render_json(&json!({
        "aggregate_kind": input.aggregate_kind,
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
        "payload_json": input.payload_json
    }));
    sha256_text(&payload)
}

fn sha256_text(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    format!("sha256:{digest:x}")
}

fn equipment_model_identity_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<StoredEquipmentModelIdentity> {
    Ok(StoredEquipmentModelIdentity {
        equipment_model_id: row.get(0)?,
        manufacturer: row.get(1)?,
        model_name: row.get(2)?,
        variant: row.get(3)?,
        equipment_class: row.get(4)?,
        category_code: row.get(5)?,
        current_approved_revision_id: row.get(6)?,
        created_by: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn equipment_model_revision_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<StoredEquipmentModelRevision> {
    Ok(StoredEquipmentModelRevision {
        revision_id: row.get(0)?,
        equipment_model_id: row.get(1)?,
        revision_number: row.get(2)?,
        parent_revision_id: row.get(3)?,
        status: row.get(4)?,
        definition_schema_version: row.get(5)?,
        definition_json: row.get(6)?,
        definition_checksum: row.get(7)?,
        created_by: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        submitted_at: row.get(11)?,
        approved_at: row.get(12)?,
        capability_count: row.get(13)?,
        interface_count: row.get(14)?,
        signal_port_count: row.get(15)?,
    })
}

fn driver_profile_identity_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<StoredDriverProfileIdentity> {
    Ok(StoredDriverProfileIdentity {
        driver_profile_id: row.get(0)?,
        equipment_model_id: row.get(1)?,
        label: row.get(2)?,
        current_approved_revision_id: row.get(3)?,
        created_by: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn driver_profile_revision_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<StoredDriverProfileRevision> {
    Ok(StoredDriverProfileRevision {
        revision_id: row.get(0)?,
        driver_profile_id: row.get(1)?,
        equipment_model_id: row.get(2)?,
        supported_model_revision_id: row.get(3)?,
        revision_number: row.get(4)?,
        parent_revision_id: row.get(5)?,
        status: row.get(6)?,
        definition_schema_version: row.get(7)?,
        definition_json: row.get(8)?,
        definition_checksum: row.get(9)?,
        created_by: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        submitted_at: row.get(13)?,
        approved_at: row.get(14)?,
        action_count: row.get(15)?,
    })
}

fn list_equipment_registry(
    connection: &Connection,
    sql: &str,
) -> Result<Vec<StoredEquipmentRegistryItem>, AgentError> {
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| AgentError::new("equipment_registry_query_failed", error.to_string()))?;
    let rows = statement
        .query_map([], equipment_registry_item_from_row)
        .map_err(|error| AgentError::new("equipment_registry_query_failed", error.to_string()))?;
    collect_rows(rows, "equipment_registry_query_failed")
}

fn equipment_registry_item_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<StoredEquipmentRegistryItem> {
    let deprecated: i64 = row.get(7)?;
    Ok(StoredEquipmentRegistryItem {
        code: row.get(0)?,
        label: row.get(1)?,
        description: row.get(2)?,
        recommended_equipment_classes: row.get(3)?,
        recommended_functional_roles: row.get(4)?,
        compatible_signal_domains: row.get(5)?,
        compatible_directionalities: row.get(6)?,
        deprecated: deprecated != 0,
    })
}

fn equipment_classification_preset_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<StoredEquipmentClassificationPreset> {
    let deprecated: i64 = row.get(9)?;
    Ok(StoredEquipmentClassificationPreset {
        preset_id: row.get(0)?,
        category_label: row.get(1)?,
        function_description: row.get(2)?,
        example_label: row.get(3)?,
        default_equipment_class: row.get(4)?,
        default_functional_role: row.get(5)?,
        default_signal_domains: row.get(6)?,
        default_technology_tags: row.get(7)?,
        notes: row.get(8)?,
        deprecated: deprecated != 0,
    })
}

fn equipment_classification_preset_port_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<StoredEquipmentClassificationPresetPort> {
    let required: i64 = row.get(16)?;
    Ok(StoredEquipmentClassificationPresetPort {
        port_order: row.get(0)?,
        port_id: row.get(1)?,
        label: row.get(2)?,
        directionality: row.get(3)?,
        flow_role: row.get(4)?,
        signal_domain: row.get(5)?,
        connector_type: row.get(6)?,
        technology_tags: row.get(7)?,
        quantity: row.get(8)?,
        unit: row.get(9)?,
        impedance: row.get(10)?,
        frequency_min: row.get(11)?,
        frequency_max: row.get(12)?,
        voltage_max: row.get(13)?,
        current_max: row.get(14)?,
        power_max: row.get(15)?,
        required: required != 0,
        comment: row.get(17)?,
    })
}

fn collect_rows<T>(
    rows: impl Iterator<Item = rusqlite::Result<T>>,
    code: &'static str,
) -> Result<Vec<T>, AgentError> {
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(|error| AgentError::new(code, error.to_string()))?);
    }
    Ok(values)
}

fn table_exists_in_schema(
    connection: &Connection,
    schema: &str,
    table: &str,
) -> Result<bool, AgentError> {
    let sql =
        format!("SELECT COUNT(*) FROM {schema}.sqlite_master WHERE type = 'table' AND name = ?1");
    let count: u32 = connection
        .query_row(&sql, params![table], |row| row.get(0))
        .map_err(|error| AgentError::new("database_invalid", error.to_string()))?;
    Ok(count > 0)
}
