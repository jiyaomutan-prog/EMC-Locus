use crate::{
    equipment_repository::{open_equipment_connection, open_equipment_connection_with_sync},
    render_json,
    sqlite_policy::{enforce_project_slice_journal_mode, AttachedDatabase},
    AgentError,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredPhysicalAsset {
    pub(crate) asset_id: String,
    pub(crate) inventory_code: String,
    pub(crate) serial_number: Option<String>,
    pub(crate) part_number: Option<String>,
    pub(crate) equipment_model_id: Option<String>,
    pub(crate) equipment_model_revision_id: Option<String>,
    pub(crate) equipment_model_checksum: Option<String>,
    pub(crate) manufacturer_snapshot: String,
    pub(crate) model_name_snapshot: String,
    pub(crate) variant_snapshot: Option<String>,
    pub(crate) category_code_snapshot: String,
    pub(crate) category_path_json: String,
    pub(crate) laboratory_location_id: Option<String>,
    pub(crate) laboratory_location_label_snapshot: Option<String>,
    pub(crate) ownership_source: String,
    pub(crate) service_state: String,
    pub(crate) availability_state: String,
    pub(crate) service_state_reason: String,
    pub(crate) notes: String,
    pub(crate) revision: u64,
    pub(crate) model_link_state: String,
    pub(crate) migrated_from_metrology: bool,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredLaboratoryLocation {
    pub(crate) location_id: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) status: String,
    pub(crate) revision: u64,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredFleetOperation {
    pub(crate) operation_id: String,
    pub(crate) entity_id: String,
    pub(crate) action: String,
    pub(crate) request_checksum: String,
    pub(crate) resulting_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredFleetAuditEvent {
    pub(crate) sequence: u64,
    pub(crate) action: String,
    pub(crate) actor: String,
    pub(crate) reason: String,
    pub(crate) old_revision: Option<u64>,
    pub(crate) new_revision: u64,
    pub(crate) operation_id: String,
    pub(crate) device_id: String,
    pub(crate) correlation_id: String,
    pub(crate) payload_json: String,
    pub(crate) occurred_at: String,
}

pub(crate) struct NewPhysicalAssetRecord<'a> {
    pub(crate) asset_id: &'a str,
    pub(crate) inventory_code: &'a str,
    pub(crate) serial_number: Option<&'a str>,
    pub(crate) part_number: Option<&'a str>,
    pub(crate) equipment_model_id: &'a str,
    pub(crate) equipment_model_revision_id: &'a str,
    pub(crate) equipment_model_checksum: &'a str,
    pub(crate) manufacturer_snapshot: &'a str,
    pub(crate) model_name_snapshot: &'a str,
    pub(crate) variant_snapshot: Option<&'a str>,
    pub(crate) category_code_snapshot: &'a str,
    pub(crate) category_path_json: &'a str,
    pub(crate) laboratory_location_id: Option<&'a str>,
    pub(crate) laboratory_location_label_snapshot: Option<&'a str>,
    pub(crate) ownership_source: &'a str,
    pub(crate) service_state: &'a str,
    pub(crate) availability_state: &'a str,
    pub(crate) service_state_reason: &'a str,
    pub(crate) notes: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct UpdatePhysicalAssetIdentityInput<'a> {
    pub(crate) asset_id: &'a str,
    pub(crate) expected_revision: u64,
    pub(crate) inventory_code: &'a str,
    pub(crate) serial_number: Option<&'a str>,
    pub(crate) part_number: Option<&'a str>,
    pub(crate) laboratory_location_id: Option<&'a str>,
    pub(crate) laboratory_location_label_snapshot: Option<&'a str>,
    pub(crate) ownership_source: &'a str,
    pub(crate) notes: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct FleetEvidenceInput<'a> {
    pub(crate) entity_kind: &'a str,
    pub(crate) entity_id: &'a str,
    pub(crate) action: &'a str,
    pub(crate) actor: &'a str,
    pub(crate) reason: &'a str,
    pub(crate) operation_id: &'a str,
    pub(crate) device_id: &'a str,
    pub(crate) correlation_id: &'a str,
    pub(crate) old_revision: Option<u64>,
    pub(crate) new_revision: u64,
    pub(crate) request_checksum: &'a str,
    pub(crate) payload_json: &'a str,
    pub(crate) timestamp: &'a str,
}

pub(crate) fn open_fleet_connection(storage_root: &Path) -> Result<Connection, AgentError> {
    let connection = open_equipment_connection(storage_root)?;
    attach_metrology(&connection, storage_root)?;
    Ok(connection)
}

pub(crate) fn open_fleet_connection_with_sync(
    storage_root: &Path,
) -> Result<Connection, AgentError> {
    let connection = open_equipment_connection_with_sync(storage_root)?;
    attach_metrology(&connection, storage_root)?;
    enforce_project_slice_journal_mode(
        &connection,
        AttachedDatabase::MetrologyDb,
        "metrology.sqlite",
    )?;
    Ok(connection)
}

fn attach_metrology(connection: &Connection, storage_root: &Path) -> Result<(), AgentError> {
    let database = storage_root.join("metrology.sqlite");
    if !database.exists() {
        return Err(AgentError::new(
            "storage_not_initialized",
            "fleet commands require initialized metrology.sqlite",
        ));
    }
    connection
        .execute(
            "ATTACH DATABASE ?1 AS metrology_db",
            params![database.to_string_lossy().to_string()],
        )
        .map_err(|error| AgentError::new("database_attach_error", error.to_string()))?;
    let dossier_exists: u64 = connection
        .query_row(
            "SELECT COUNT(*) FROM metrology_db.sqlite_master
             WHERE type = 'table' AND name = 'metrology_asset_dossiers'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| AgentError::new("database_invalid", error.to_string()))?;
    if dossier_exists != 1 {
        return Err(AgentError::new(
            "storage_not_initialized",
            "missing required table metrology_db.metrology_asset_dossiers",
        ));
    }
    Ok(())
}

pub(crate) fn load_physical_asset(
    connection: &Connection,
    asset_id: &str,
) -> Result<Option<StoredPhysicalAsset>, AgentError> {
    connection
        .query_row(
            &format!("{} WHERE asset_id = ?1", physical_asset_select()),
            params![asset_id],
            physical_asset_row,
        )
        .optional()
        .map_err(|error| AgentError::new("physical_asset_query_failed", error.to_string()))
}

pub(crate) fn load_physical_asset_by_inventory_code(
    connection: &Connection,
    inventory_code: &str,
) -> Result<Option<StoredPhysicalAsset>, AgentError> {
    connection
        .query_row(
            &format!(
                "{} WHERE lower(inventory_code) = lower(?1)",
                physical_asset_select()
            ),
            params![inventory_code],
            physical_asset_row,
        )
        .optional()
        .map_err(|error| AgentError::new("physical_asset_query_failed", error.to_string()))
}

pub(crate) fn list_physical_assets(
    connection: &Connection,
) -> Result<Vec<StoredPhysicalAsset>, AgentError> {
    let sql = format!(
        "{} ORDER BY category_code_snapshot, manufacturer_snapshot, model_name_snapshot, inventory_code",
        physical_asset_select()
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AgentError::new("physical_asset_query_failed", error.to_string()))?;
    let rows = statement
        .query_map([], physical_asset_row)
        .map_err(|error| AgentError::new("physical_asset_query_failed", error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AgentError::new("physical_asset_query_failed", error.to_string()))
}

pub(crate) fn insert_physical_asset(
    transaction: &Transaction<'_>,
    input: NewPhysicalAssetRecord<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "INSERT INTO physical_assets (
                asset_id, inventory_code, serial_number, part_number,
                equipment_model_id, equipment_model_revision_id, equipment_model_checksum,
                manufacturer_snapshot, model_name_snapshot, variant_snapshot,
                category_code_snapshot, category_path_json, laboratory_location_id,
                laboratory_location_label_snapshot, ownership_source, service_state,
                availability_state, service_state_reason, notes, revision,
                model_link_state, migrated_from_metrology, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17, ?18, ?19, 1, 'resolved', 0, ?20, ?20
            )",
            params![
                input.asset_id,
                input.inventory_code,
                input.serial_number,
                input.part_number,
                input.equipment_model_id,
                input.equipment_model_revision_id,
                input.equipment_model_checksum,
                input.manufacturer_snapshot,
                input.model_name_snapshot,
                input.variant_snapshot,
                input.category_code_snapshot,
                input.category_path_json,
                input.laboratory_location_id,
                input.laboratory_location_label_snapshot,
                input.ownership_source,
                input.service_state,
                input.availability_state,
                input.service_state_reason,
                input.notes,
                input.timestamp,
            ],
        )
        .map_err(map_asset_write_error)?;
    Ok(())
}

pub(crate) fn update_physical_asset_identity(
    transaction: &Transaction<'_>,
    input: UpdatePhysicalAssetIdentityInput<'_>,
) -> Result<u64, AgentError> {
    let changed = transaction
        .execute(
            "UPDATE physical_assets SET
                inventory_code = ?3, serial_number = ?4, part_number = ?5,
                laboratory_location_id = ?6, laboratory_location_label_snapshot = ?7,
                ownership_source = ?8, notes = ?9, revision = revision + 1, updated_at = ?10
             WHERE asset_id = ?1 AND revision = ?2",
            params![
                input.asset_id,
                input.expected_revision,
                input.inventory_code,
                input.serial_number,
                input.part_number,
                input.laboratory_location_id,
                input.laboratory_location_label_snapshot,
                input.ownership_source,
                input.notes,
                input.timestamp,
            ],
        )
        .map_err(map_asset_write_error)?;
    if changed == 0 {
        return Err(AgentError::with_details(
            "physical_asset_revision_conflict",
            "L'exemplaire a été modifié ailleurs. Rechargez sa fiche avant de recommencer.",
            json!({ "asset_id": input.asset_id, "expected_revision": input.expected_revision }),
        ));
    }
    Ok(input.expected_revision + 1)
}

pub(crate) fn update_physical_asset_service_state(
    transaction: &Transaction<'_>,
    asset_id: &str,
    expected_revision: u64,
    service_state: &str,
    availability_state: &str,
    reason: &str,
    timestamp: &str,
) -> Result<u64, AgentError> {
    let changed = transaction
        .execute(
            "UPDATE physical_assets SET service_state = ?3, availability_state = ?4,
                service_state_reason = ?5, revision = revision + 1, updated_at = ?6
             WHERE asset_id = ?1 AND revision = ?2",
            params![
                asset_id,
                expected_revision,
                service_state,
                availability_state,
                reason,
                timestamp
            ],
        )
        .map_err(map_asset_write_error)?;
    require_asset_update(changed, asset_id, expected_revision)?;
    Ok(expected_revision + 1)
}

pub(crate) fn update_physical_asset_availability(
    transaction: &Transaction<'_>,
    asset_id: &str,
    expected_revision: u64,
    availability_state: &str,
    timestamp: &str,
) -> Result<u64, AgentError> {
    let changed = transaction
        .execute(
            "UPDATE physical_assets SET availability_state = ?3,
                revision = revision + 1, updated_at = ?4
             WHERE asset_id = ?1 AND revision = ?2",
            params![asset_id, expected_revision, availability_state, timestamp],
        )
        .map_err(map_asset_write_error)?;
    require_asset_update(changed, asset_id, expected_revision)?;
    Ok(expected_revision + 1)
}

pub(crate) fn reconcile_physical_asset_model(
    transaction: &Transaction<'_>,
    asset_id: &str,
    expected_revision: u64,
    model: &emc_locus_core::PinnedEquipmentModel,
    category_path_json: &str,
    timestamp: &str,
) -> Result<u64, AgentError> {
    let changed = transaction
        .execute(
            "UPDATE physical_assets SET
                equipment_model_id = ?3,
                equipment_model_revision_id = ?4,
                equipment_model_checksum = ?5,
                manufacturer_snapshot = ?6,
                model_name_snapshot = ?7,
                variant_snapshot = ?8,
                category_code_snapshot = ?9,
                category_path_json = ?10,
                model_link_state = 'resolved',
                revision = revision + 1,
                updated_at = ?11
             WHERE asset_id = ?1 AND revision = ?2
               AND model_link_state = 'migration_review_required'",
            params![
                asset_id,
                expected_revision,
                model.equipment_model_id,
                model.equipment_model_revision_id,
                model.equipment_model_checksum,
                model.manufacturer,
                model.model_name,
                model.variant,
                model.category_code,
                category_path_json,
                timestamp,
            ],
        )
        .map_err(map_asset_write_error)?;
    require_asset_update(changed, asset_id, expected_revision)?;
    Ok(expected_revision + 1)
}

pub(crate) fn load_laboratory_location(
    connection: &Connection,
    location_id: &str,
) -> Result<Option<StoredLaboratoryLocation>, AgentError> {
    connection
        .query_row(
            "SELECT location_id, label, description, status, revision, created_at, updated_at
             FROM laboratory_locations WHERE location_id = ?1",
            params![location_id],
            |row| {
                Ok(StoredLaboratoryLocation {
                    location_id: row.get(0)?,
                    label: row.get(1)?,
                    description: row.get(2)?,
                    status: row.get(3)?,
                    revision: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(|error| AgentError::new("laboratory_location_query_failed", error.to_string()))
}

pub(crate) fn list_laboratory_locations(
    connection: &Connection,
    include_archived: bool,
) -> Result<Vec<StoredLaboratoryLocation>, AgentError> {
    let sql = if include_archived {
        "SELECT location_id, label, description, status, revision, created_at, updated_at
         FROM laboratory_locations ORDER BY status, label"
    } else {
        "SELECT location_id, label, description, status, revision, created_at, updated_at
         FROM laboratory_locations WHERE status = 'active' ORDER BY label"
    };
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| AgentError::new("laboratory_location_query_failed", error.to_string()))?;
    let rows = statement
        .query_map([], |row| {
            Ok(StoredLaboratoryLocation {
                location_id: row.get(0)?,
                label: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                revision: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|error| AgentError::new("laboratory_location_query_failed", error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AgentError::new("laboratory_location_query_failed", error.to_string()))
}

pub(crate) fn insert_laboratory_location(
    transaction: &Transaction<'_>,
    location_id: &str,
    label: &str,
    description: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    transaction
        .execute(
            "INSERT INTO laboratory_locations (
                location_id, label, description, status, revision, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 'active', 1, ?4, ?4)",
            params![location_id, label, description, timestamp],
        )
        .map_err(map_location_write_error)?;
    Ok(())
}

pub(crate) fn update_laboratory_location(
    transaction: &Transaction<'_>,
    location_id: &str,
    expected_revision: u64,
    label: &str,
    description: &str,
    timestamp: &str,
) -> Result<u64, AgentError> {
    let changed = transaction
        .execute(
            "UPDATE laboratory_locations SET label = ?3, description = ?4,
                revision = revision + 1, updated_at = ?5
             WHERE location_id = ?1 AND revision = ?2 AND status = 'active'",
            params![
                location_id,
                expected_revision,
                label,
                description,
                timestamp
            ],
        )
        .map_err(map_location_write_error)?;
    require_location_update(changed, location_id, expected_revision)?;
    Ok(expected_revision + 1)
}

pub(crate) fn archive_laboratory_location(
    transaction: &Transaction<'_>,
    location_id: &str,
    expected_revision: u64,
    timestamp: &str,
) -> Result<u64, AgentError> {
    let changed = transaction
        .execute(
            "UPDATE laboratory_locations SET status = 'archived',
                revision = revision + 1, updated_at = ?3
             WHERE location_id = ?1 AND revision = ?2 AND status = 'active'",
            params![location_id, expected_revision, timestamp],
        )
        .map_err(map_location_write_error)?;
    require_location_update(changed, location_id, expected_revision)?;
    Ok(expected_revision + 1)
}

pub(crate) fn existing_fleet_operation(
    connection: &Connection,
    entity_kind: &str,
    operation_id: &str,
) -> Result<Option<StoredFleetOperation>, AgentError> {
    let (table, id_column) = evidence_table(entity_kind, "operations")?;
    let sql = format!(
        "SELECT operation_id, {id_column}, action, request_checksum, resulting_revision
         FROM {table} WHERE operation_id = ?1"
    );
    connection
        .query_row(&sql, params![operation_id], |row| {
            Ok(StoredFleetOperation {
                operation_id: row.get(0)?,
                entity_id: row.get(1)?,
                action: row.get(2)?,
                request_checksum: row.get(3)?,
                resulting_revision: row.get(4)?,
            })
        })
        .optional()
        .map_err(|error| AgentError::new("fleet_operation_query_failed", error.to_string()))
}

pub(crate) fn write_fleet_evidence(
    transaction: &Transaction<'_>,
    input: FleetEvidenceInput<'_>,
) -> Result<(), AgentError> {
    let (operation_table, id_column) = evidence_table(input.entity_kind, "operations")?;
    let (audit_table, _) = evidence_table(input.entity_kind, "audit")?;
    let sequence = next_audit_sequence(transaction, input.entity_kind, input.entity_id)?;
    let operation_sql = format!(
        "INSERT INTO {operation_table} (
            operation_id, {id_column}, action, request_checksum, resulting_revision, occurred_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    );
    transaction
        .execute(
            &operation_sql,
            params![
                input.operation_id,
                input.entity_id,
                input.action,
                input.request_checksum,
                input.new_revision,
                input.timestamp,
            ],
        )
        .map_err(|error| AgentError::new("fleet_operation_write_failed", error.to_string()))?;
    let payload_checksum = sha256_text(input.payload_json);
    let audit_sql = format!(
        "INSERT INTO {audit_table} (
            {id_column}, sequence, action, actor, reason, old_revision, new_revision,
            operation_id, device_id, correlation_id, payload_json, payload_checksum, occurred_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"
    );
    transaction
        .execute(
            &audit_sql,
            params![
                input.entity_id,
                sequence,
                input.action,
                input.actor,
                input.reason,
                input.old_revision,
                input.new_revision,
                input.operation_id,
                input.device_id,
                input.correlation_id,
                input.payload_json,
                payload_checksum,
                input.timestamp,
            ],
        )
        .map_err(|error| AgentError::new("fleet_audit_write_failed", error.to_string()))?;
    let base_revision = input
        .old_revision
        .map(|revision| format!("rev-{revision:04}"))
        .unwrap_or_else(|| "rev-0000".to_owned());
    let resulting_revision = format!("rev-{:04}", input.new_revision);
    let outbox_payload = render_json(&json!({
        "domain": "equipment",
        "entity_type": input.entity_kind,
        "entity_id": input.entity_id,
        "operation_kind": input.action,
        "payload": serde_json::from_str::<serde_json::Value>(input.payload_json)
            .unwrap_or_else(|_| json!({ "raw": input.payload_json }))
    }));
    let outbox_checksum = sha256_text(&outbox_payload);
    transaction
        .execute(
            "INSERT INTO sync_db.sync_operations (
                operation_id, domain, entity_type, entity_id, operation_kind,
                base_revision, resulting_revision, actor_id, device_id, correlation_id,
                payload_json, payload_checksum, status, occurred_at, recorded_at
             ) VALUES (?1, 'equipment', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                'pending', ?12, ?12)",
            params![
                input.operation_id,
                input.entity_kind,
                input.entity_id,
                input.action,
                base_revision,
                resulting_revision,
                input.actor,
                input.device_id,
                input.correlation_id,
                outbox_payload,
                outbox_checksum,
                input.timestamp,
            ],
        )
        .map_err(|error| AgentError::new("fleet_outbox_write_failed", error.to_string()))?;
    Ok(())
}

pub(crate) fn load_fleet_audit_events(
    connection: &Connection,
    entity_kind: &str,
    entity_id: &str,
) -> Result<Vec<StoredFleetAuditEvent>, AgentError> {
    let (table, id_column) = evidence_table(entity_kind, "audit")?;
    let sql = format!(
        "SELECT sequence, action, actor, reason, old_revision, new_revision,
            operation_id, device_id, correlation_id, payload_json, occurred_at
         FROM {table} WHERE {id_column} = ?1 ORDER BY sequence"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AgentError::new("fleet_audit_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![entity_id], |row| {
            Ok(StoredFleetAuditEvent {
                sequence: row.get(0)?,
                action: row.get(1)?,
                actor: row.get(2)?,
                reason: row.get(3)?,
                old_revision: row.get(4)?,
                new_revision: row.get(5)?,
                operation_id: row.get(6)?,
                device_id: row.get(7)?,
                correlation_id: row.get(8)?,
                payload_json: row.get(9)?,
                occurred_at: row.get(10)?,
            })
        })
        .map_err(|error| AgentError::new("fleet_audit_query_failed", error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AgentError::new("fleet_audit_query_failed", error.to_string()))
}

pub(crate) fn request_checksum(payload_json: &str) -> String {
    sha256_text(payload_json)
}

fn physical_asset_select() -> &'static str {
    "SELECT asset_id, inventory_code, serial_number, part_number,
        equipment_model_id, equipment_model_revision_id, equipment_model_checksum,
        manufacturer_snapshot, model_name_snapshot, variant_snapshot,
        category_code_snapshot, category_path_json, laboratory_location_id,
        laboratory_location_label_snapshot, ownership_source, service_state,
        availability_state, service_state_reason, notes, revision, model_link_state,
        migrated_from_metrology, created_at, updated_at FROM physical_assets"
}

fn physical_asset_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredPhysicalAsset> {
    Ok(StoredPhysicalAsset {
        asset_id: row.get(0)?,
        inventory_code: row.get(1)?,
        serial_number: row.get(2)?,
        part_number: row.get(3)?,
        equipment_model_id: row.get(4)?,
        equipment_model_revision_id: row.get(5)?,
        equipment_model_checksum: row.get(6)?,
        manufacturer_snapshot: row.get(7)?,
        model_name_snapshot: row.get(8)?,
        variant_snapshot: row.get(9)?,
        category_code_snapshot: row.get(10)?,
        category_path_json: row.get(11)?,
        laboratory_location_id: row.get(12)?,
        laboratory_location_label_snapshot: row.get(13)?,
        ownership_source: row.get(14)?,
        service_state: row.get(15)?,
        availability_state: row.get(16)?,
        service_state_reason: row.get(17)?,
        notes: row.get(18)?,
        revision: row.get(19)?,
        model_link_state: row.get(20)?,
        migrated_from_metrology: row.get::<_, i64>(21)? != 0,
        created_at: row.get(22)?,
        updated_at: row.get(23)?,
    })
}

fn next_audit_sequence(
    transaction: &Transaction<'_>,
    entity_kind: &str,
    entity_id: &str,
) -> Result<u64, AgentError> {
    let (table, id_column) = evidence_table(entity_kind, "audit")?;
    let sql = format!("SELECT COALESCE(MAX(sequence), 0) + 1 FROM {table} WHERE {id_column} = ?1");
    transaction
        .query_row(&sql, params![entity_id], |row| row.get(0))
        .map_err(|error| AgentError::new("fleet_audit_query_failed", error.to_string()))
}

fn evidence_table(
    entity_kind: &str,
    table_kind: &str,
) -> Result<(&'static str, &'static str), AgentError> {
    match (entity_kind, table_kind) {
        ("physical_asset", "operations") => Ok(("physical_asset_operations", "asset_id")),
        ("physical_asset", "audit") => Ok(("physical_asset_audit_events", "asset_id")),
        ("laboratory_location", "operations") => {
            Ok(("laboratory_location_operations", "location_id"))
        }
        ("laboratory_location", "audit") => Ok(("laboratory_location_audit_events", "location_id")),
        _ => Err(AgentError::new(
            "invalid_fleet_entity_kind",
            "unsupported fleet evidence entity kind",
        )),
    }
}

fn require_asset_update(
    changed: usize,
    asset_id: &str,
    expected_revision: u64,
) -> Result<(), AgentError> {
    if changed == 0 {
        return Err(AgentError::with_details(
            "physical_asset_revision_conflict",
            "L'exemplaire a été modifié ailleurs. Rechargez sa fiche avant de recommencer.",
            json!({ "asset_id": asset_id, "expected_revision": expected_revision }),
        ));
    }
    Ok(())
}

fn require_location_update(
    changed: usize,
    location_id: &str,
    expected_revision: u64,
) -> Result<(), AgentError> {
    if changed == 0 {
        return Err(AgentError::with_details(
            "laboratory_location_revision_conflict",
            "Le lieu a été modifié ou archivé. Rechargez le registre avant de recommencer.",
            json!({ "location_id": location_id, "expected_revision": expected_revision }),
        ));
    }
    Ok(())
}

fn map_asset_write_error(error: rusqlite::Error) -> AgentError {
    let message = error.to_string();
    if message.contains("physical_assets_inventory_code_unique_idx")
        || message.contains("physical_assets.inventory_code")
    {
        AgentError::new(
            "physical_asset_inventory_code_conflict",
            "Ce code inventaire est déjà utilisé par un autre exemplaire.",
        )
    } else {
        AgentError::new("physical_asset_write_failed", message)
    }
}

fn map_location_write_error(error: rusqlite::Error) -> AgentError {
    let message = error.to_string();
    if message.contains("laboratory_locations_label_unique_idx") {
        AgentError::new(
            "laboratory_location_label_conflict",
            "Un lieu porte déjà ce libellé.",
        )
    } else {
        AgentError::new("laboratory_location_write_failed", message)
    }
}

fn sha256_text(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("sha256:{digest:x}")
}
