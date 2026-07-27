use crate::fleet_repository::{request_checksum, write_fleet_evidence, FleetEvidenceInput};
use crate::{
    render_json,
    sqlite_policy::{enforce_project_slice_journal_mode, AttachedDatabase},
    AgentError,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const MIGRATION_ID: &str = "metrology-instruments-to-fleet-v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FleetMigrationSummary {
    pub(crate) migration_id: String,
    pub(crate) source_count: u64,
    pub(crate) imported_count: u64,
    pub(crate) resolved_model_count: u64,
    pub(crate) reconciliation_required_count: u64,
    pub(crate) replayed: bool,
}

#[derive(Clone, Debug)]
struct LegacyInstrument {
    asset_id: String,
    family: String,
    manufacturer: String,
    model: String,
    serial_number: String,
    availability: String,
    category_code: Option<String>,
    part_number: Option<String>,
    serviceability_status: String,
    serviceability_reason: String,
    equipment_model_id: Option<String>,
    equipment_model_revision_id: Option<String>,
    equipment_model_checksum: Option<String>,
    created_at: String,
    updated_at: String,
}

pub(crate) fn migrate_legacy_metrology_instruments(
    storage_root: &Path,
) -> Result<FleetMigrationSummary, AgentError> {
    let equipment_path = storage_root.join("equipment.sqlite");
    let metrology_path = storage_root.join("metrology.sqlite");
    let sync_path = storage_root.join("sync.sqlite");
    if !equipment_path.exists() || !metrology_path.exists() || !sync_path.exists() {
        return Err(AgentError::new(
            "storage_not_initialized",
            "the equipment, metrology and sync databases are required for fleet migration",
        ));
    }

    let mut connection = Connection::open(&equipment_path).map_err(|error| {
        AgentError::new(
            "database_open_error",
            format!("cannot open {}: {error}", equipment_path.display()),
        )
    })?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| AgentError::new("database_pragma_error", error.to_string()))?;
    connection
        .execute(
            "ATTACH DATABASE ?1 AS metrology_db",
            params![metrology_path.to_string_lossy().to_string()],
        )
        .map_err(|error| AgentError::new("database_attach_error", error.to_string()))?;
    connection
        .execute(
            "ATTACH DATABASE ?1 AS sync_db",
            params![sync_path.to_string_lossy().to_string()],
        )
        .map_err(|error| AgentError::new("database_attach_error", error.to_string()))?;
    enforce_project_slice_journal_mode(&connection, AttachedDatabase::Main, "equipment.sqlite")?;
    enforce_project_slice_journal_mode(
        &connection,
        AttachedDatabase::MetrologyDb,
        "metrology.sqlite",
    )?;
    enforce_project_slice_journal_mode(&connection, AttachedDatabase::SyncDb, "sync.sqlite")?;

    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    if let Some(evidence_json) = transaction
        .query_row(
            "SELECT evidence_json FROM equipment_cross_domain_migrations WHERE migration_id = ?1",
            params![MIGRATION_ID],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| AgentError::new("fleet_migration_query_failed", error.to_string()))?
    {
        let mut summary: FleetMigrationSummary =
            serde_json::from_str(&evidence_json).map_err(|error| {
                AgentError::new("fleet_migration_evidence_invalid", error.to_string())
            })?;
        summary.replayed = true;
        return Ok(summary);
    }

    let legacy = load_legacy_instruments(&transaction)?;
    let source_count = legacy.len() as u64;
    let mut imported_count = 0_u64;
    let mut resolved_model_count = 0_u64;
    let now = utc_timestamp()?;
    for instrument in legacy {
        ensure_asset_slot_is_free(&transaction, &instrument.asset_id)?;
        let model_is_resolved = legacy_model_reference_is_resolved(&transaction, &instrument)?;
        if model_is_resolved {
            resolved_model_count += 1;
        }
        insert_migrated_asset(&transaction, &instrument, model_is_resolved, &now)?;
        write_migration_evidence(&transaction, &instrument, model_is_resolved, &now)?;
        imported_count += 1;
    }

    let summary = FleetMigrationSummary {
        migration_id: MIGRATION_ID.to_owned(),
        source_count,
        imported_count,
        resolved_model_count,
        reconciliation_required_count: source_count.saturating_sub(resolved_model_count),
        replayed: false,
    };
    let evidence_json = render_json(&summary);
    transaction
        .execute(
            "INSERT INTO equipment_cross_domain_migrations (
                migration_id, source_domain, source_schema_version, imported_record_count,
                evidence_json, completed_at
             ) VALUES (?1, 'metrology', 11, ?2, ?3, ?4)",
            params![MIGRATION_ID, imported_count, evidence_json, now],
        )
        .map_err(|error| AgentError::new("fleet_migration_write_failed", error.to_string()))?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    Ok(summary)
}

fn load_legacy_instruments(
    transaction: &Transaction<'_>,
) -> Result<Vec<LegacyInstrument>, AgentError> {
    let mut statement = transaction
        .prepare(
            "SELECT asset_id, family, manufacturer, model, serial_number, availability,
                category_code, part_number, serviceability_status, serviceability_reason,
                equipment_model_id, equipment_model_revision_id, equipment_model_checksum,
                created_at, updated_at
             FROM metrology_db.legacy_instruments_0_21_1 ORDER BY asset_id",
        )
        .map_err(|error| AgentError::new("fleet_migration_query_failed", error.to_string()))?;
    let rows = statement
        .query_map([], |row| {
            Ok(LegacyInstrument {
                asset_id: row.get(0)?,
                family: row.get(1)?,
                manufacturer: row.get(2)?,
                model: row.get(3)?,
                serial_number: row.get(4)?,
                availability: row.get(5)?,
                category_code: row.get(6)?,
                part_number: row.get(7)?,
                serviceability_status: row.get(8)?,
                serviceability_reason: row.get(9)?,
                equipment_model_id: row.get(10)?,
                equipment_model_revision_id: row.get(11)?,
                equipment_model_checksum: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })
        .map_err(|error| AgentError::new("fleet_migration_query_failed", error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AgentError::new("fleet_migration_query_failed", error.to_string()))
}

fn ensure_asset_slot_is_free(
    transaction: &Transaction<'_>,
    asset_id: &str,
) -> Result<(), AgentError> {
    let existing = transaction
        .query_row(
            "SELECT asset_id FROM physical_assets
             WHERE asset_id = ?1 OR lower(inventory_code) = lower(?1) LIMIT 1",
            params![asset_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| AgentError::new("fleet_migration_query_failed", error.to_string()))?;
    if let Some(existing) = existing {
        return Err(AgentError::with_details(
            "fleet_migration_asset_conflict",
            "Un identifiant metrologique 0.21.1 entre en conflit avec un exemplaire du parc.",
            json!({ "legacy_asset_id": asset_id, "existing_asset_id": existing }),
        ));
    }
    Ok(())
}

fn legacy_model_reference_is_resolved(
    transaction: &Transaction<'_>,
    instrument: &LegacyInstrument,
) -> Result<bool, AgentError> {
    let (Some(model_id), Some(revision_id), Some(checksum)) = (
        instrument.equipment_model_id.as_deref(),
        instrument.equipment_model_revision_id.as_deref(),
        instrument.equipment_model_checksum.as_deref(),
    ) else {
        return Ok(false);
    };
    let count = transaction
        .query_row(
            "SELECT COUNT(*) FROM equipment_model_revisions r
             JOIN equipment_model_identities i ON i.equipment_model_id = r.equipment_model_id
             WHERE i.equipment_model_id = ?1 AND r.revision_id = ?2
               AND r.definition_checksum = ?3
               AND r.status IN ('approved', 'superseded', 'suspended', 'retired')",
            params![model_id, revision_id, checksum],
            |row| row.get::<_, u64>(0),
        )
        .map_err(|error| AgentError::new("fleet_migration_query_failed", error.to_string()))?;
    Ok(count == 1)
}

fn insert_migrated_asset(
    transaction: &Transaction<'_>,
    instrument: &LegacyInstrument,
    model_is_resolved: bool,
    now: &str,
) -> Result<(), AgentError> {
    let category_code = nonempty(instrument.category_code.as_deref())
        .or_else(|| nonempty(Some(instrument.family.as_str())))
        .unwrap_or("legacy_metrology");
    let category_label = nonempty(Some(instrument.family.as_str())).unwrap_or(category_code);
    let category_path_json = render_json(&vec![category_label]);
    let serial_number = nonempty(Some(instrument.serial_number.as_str()));
    let service_state = migrated_service_state(&instrument.serviceability_status);
    let availability_state = migrated_availability(service_state, &instrument.availability);
    let administrative_unavailability_reason = if availability_state == "unavailable" {
        nonempty(Some(instrument.serviceability_reason.as_str()))
            .unwrap_or("Indisponibilité héritée du registre métrologique")
    } else {
        ""
    };
    let migration_evidence_json = render_json(&json!({
        "source": "metrology.sqlite/legacy_instruments_0_21_1",
        "legacy_asset_id": instrument.asset_id,
        "legacy_family": instrument.family,
        "legacy_availability": instrument.availability,
        "legacy_equipment_model_id": instrument.equipment_model_id,
        "legacy_equipment_model_revision_id": instrument.equipment_model_revision_id,
        "legacy_equipment_model_checksum": instrument.equipment_model_checksum,
        "model_reference_resolved": model_is_resolved
    }));
    let (model_id, revision_id, checksum) = if model_is_resolved {
        (
            instrument.equipment_model_id.as_deref(),
            instrument.equipment_model_revision_id.as_deref(),
            instrument.equipment_model_checksum.as_deref(),
        )
    } else {
        (None, None, None)
    };
    transaction
        .execute(
            "INSERT INTO physical_assets (
                asset_id, inventory_code, serial_number, part_number,
                equipment_model_id, equipment_model_revision_id, equipment_model_checksum,
                manufacturer_snapshot, model_name_snapshot, variant_snapshot,
                category_code_snapshot, category_path_json, laboratory_location_id,
                laboratory_location_label_snapshot, ownership_source, service_state,
                availability_state, service_state_reason, notes, revision,
                model_link_state, migrated_from_metrology, created_at, updated_at,
                migration_evidence_json, administrative_availability,
                administrative_unavailability_reason, legacy_availability_evidence_json
             ) VALUES (
                ?1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10,
                NULL, NULL, 'laboratory_owned', ?11, ?12, ?13, '', 1,
                ?14, 1, ?15, ?16, ?17, ?12, ?18, ?19
             )",
            params![
                instrument.asset_id,
                serial_number,
                instrument.part_number.as_deref(),
                model_id,
                revision_id,
                checksum,
                instrument.manufacturer,
                instrument.model,
                category_code,
                category_path_json,
                service_state,
                availability_state,
                instrument.serviceability_reason,
                if model_is_resolved {
                    "resolved"
                } else {
                    "migration_review_required"
                },
                instrument.created_at,
                instrument.updated_at,
                migration_evidence_json,
                administrative_unavailability_reason,
                render_json(&json!({
                    "legacy_availability_state": instrument.availability
                })),
            ],
        )
        .map_err(|error| AgentError::new("fleet_migration_write_failed", error.to_string()))?;
    let _ = now;
    Ok(())
}

fn write_migration_evidence(
    transaction: &Transaction<'_>,
    instrument: &LegacyInstrument,
    model_is_resolved: bool,
    now: &str,
) -> Result<(), AgentError> {
    let payload_json = render_json(&json!({
        "asset_id": instrument.asset_id,
        "inventory_code": instrument.asset_id,
        "source": "metrology.sqlite/legacy_instruments_0_21_1",
        "model_link_state": if model_is_resolved {
            "resolved"
        } else {
            "migration_review_required"
        }
    }));
    let checksum = request_checksum(&payload_json);
    let operation_id = migration_operation_id(&instrument.asset_id);
    write_fleet_evidence(
        transaction,
        FleetEvidenceInput {
            entity_kind: "physical_asset",
            entity_id: &instrument.asset_id,
            action: "physical_asset_migrated_from_metrology",
            actor: "system.migration",
            reason: "Migration 0.22.0 vers la source d'identite du parc",
            operation_id: &operation_id,
            device_id: "local-agent-migration",
            correlation_id: MIGRATION_ID,
            old_revision: None,
            new_revision: 1,
            request_checksum: &checksum,
            payload_json: &payload_json,
            timestamp: now,
        },
    )
}

fn migrated_service_state(value: &str) -> &'static str {
    match value {
        "restricted" => "restricted",
        "out_of_service" => "out_of_service",
        "retired" => "retired",
        _ => "usable",
    }
}

fn migrated_availability(service_state: &str, legacy_availability: &str) -> &'static str {
    if matches!(service_state, "out_of_service" | "retired") {
        "unavailable"
    } else {
        let _ = legacy_availability;
        "available"
    }
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn migration_operation_id(asset_id: &str) -> String {
    let digest = Sha256::digest(format!("{MIGRATION_ID}\n{asset_id}").as_bytes());
    format!("migration-0.22.0-{}", &format!("{digest:x}")[..24])
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_error", error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrology_repository::{load_asset_characterizations, load_calibration_events};
    use crate::{
        fleet_repository::open_fleet_connection,
        fleet_service::{
            reconcile_physical_asset_model_json, FleetOperationContext,
            ReconcilePhysicalAssetModelInput,
        },
        metrology_repository::open_metrology_connection,
    };
    use emc_locus_core::EquipmentModelDefinition;
    use rusqlite::Connection;
    use std::{fs, path::PathBuf};

    #[test]
    fn migration_preserves_real_0_21_1_metrology_fixture_and_is_idempotent() {
        let storage_root = temporary_storage_root("legacy-metrology-fixture");
        fs::create_dir_all(&storage_root).unwrap();
        apply_migrations_through(
            &storage_root.join("sync.sqlite"),
            &repo_root().join("storage/sqlite/sync"),
            u32::MAX,
        );
        apply_migrations_through(
            &storage_root.join("equipment.sqlite"),
            &repo_root().join("storage/sqlite/equipment"),
            6,
        );
        apply_migrations_through(
            &storage_root.join("metrology.sqlite"),
            &repo_root().join("storage/sqlite/metrology"),
            10,
        );
        seed_0_21_1_fixture(&storage_root);
        apply_migrations_from(
            &storage_root.join("equipment.sqlite"),
            &repo_root().join("storage/sqlite/equipment"),
            7,
        );
        apply_migrations_from(
            &storage_root.join("metrology.sqlite"),
            &repo_root().join("storage/sqlite/metrology"),
            11,
        );

        let first = migrate_legacy_metrology_instruments(&storage_root).unwrap();
        assert_eq!(first.source_count, 2);
        assert_eq!(first.imported_count, 2);
        assert_eq!(first.resolved_model_count, 1);
        assert_eq!(first.reconciliation_required_count, 1);
        assert!(!first.replayed);

        let equipment = open_fleet_connection(&storage_root).unwrap();
        let counts = equipment
            .query_row(
                "SELECT COUNT(*), SUM(migrated_from_metrology),
                    SUM(model_link_state = 'resolved') FROM physical_assets",
                [],
                |row| {
                    Ok((
                        row.get::<_, u64>(0)?,
                        row.get::<_, u64>(1)?,
                        row.get::<_, u64>(2)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(counts, (2, 2, 1));
        let preserved: (String, String, String, String) = equipment
            .query_row(
                "SELECT inventory_code, serial_number, manufacturer_snapshot, model_name_snapshot
                 FROM physical_assets WHERE asset_id = 'LEGACY-SA-001'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            preserved,
            (
                "LEGACY-SA-001".to_owned(),
                "SN-LEGACY-001".to_owned(),
                "Acme Test".to_owned(),
                "Scope 1".to_owned(),
            )
        );
        drop(equipment);

        let metrology = open_metrology_connection(&storage_root).unwrap();
        assert_eq!(
            load_calibration_events(&metrology, "LEGACY-SA-001")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            load_asset_characterizations(&metrology, "LEGACY-SA-001")
                .unwrap()
                .len(),
            1
        );
        let correction_count: u64 = metrology
            .query_row(
                "SELECT COUNT(*) FROM asset_correction_assignments
                 WHERE asset_id = 'LEGACY-SA-001'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(correction_count, 1);
        let dossier_count: u64 = metrology
            .query_row("SELECT COUNT(*) FROM metrology_asset_dossiers", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(dossier_count, 2);
        let read_only = metrology.execute(
            "UPDATE legacy_instruments_0_21_1 SET model = 'forbidden' WHERE asset_id = 'LEGACY-SA-001'",
            [],
        );
        assert!(read_only.is_err());
        drop(metrology);

        let replay = migrate_legacy_metrology_instruments(&storage_root).unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.imported_count, 2);
        let equipment = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        let evidence: (u64, u64, u64) = equipment
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM physical_assets),
                    (SELECT COUNT(*) FROM physical_asset_audit_events),
                    (SELECT COUNT(*) FROM equipment_cross_domain_migrations)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(evidence, (2, 2, 1));
        let sync = Connection::open(storage_root.join("sync.sqlite")).unwrap();
        let outbox_count: u64 = sync
            .query_row(
                "SELECT COUNT(*) FROM sync_operations WHERE domain = 'equipment'
                 AND operation_kind = 'physical_asset_migrated_from_metrology'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(outbox_count, 2);

        let reconciled: serde_json::Value = serde_json::from_str(
            &reconcile_physical_asset_model_json(
                &storage_root,
                ReconcilePhysicalAssetModelInput {
                    asset_id: "LEGACY-CABLE-002".to_owned(),
                    expected_revision: 1,
                    equipment_model_id: "EQM-LEGACY-SCOPE".to_owned(),
                    equipment_model_revision_id: "EQM-LEGACY-SCOPE-REV-0001".to_owned(),
                    context: FleetOperationContext {
                        actor: "fixture.metrologue".to_owned(),
                        reason: "Rapprochement contrôlé du fixture migré".to_owned(),
                        operation_id: "op-fixture-migrated-reconciliation".to_owned(),
                        correlation_id: "corr-fixture-migrated-reconciliation".to_owned(),
                        device_id: "fixture-device".to_owned(),
                    },
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reconciled["asset"]["model_link_state"], "resolved");
        assert_eq!(
            reconciled["asset"]["equipment_model_revision_id"],
            "EQM-LEGACY-SCOPE-REV-0001"
        );
        let equipment = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        let migration_evidence: String = equipment
            .query_row(
                "SELECT migration_evidence_json FROM physical_assets
                 WHERE asset_id = 'LEGACY-CABLE-002'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(migration_evidence.contains("legacy_instruments_0_21_1"));

        let _ = fs::remove_dir_all(storage_root);
    }

    #[test]
    fn administrative_availability_migration_preserves_every_legacy_state() {
        let storage_root = temporary_storage_root("legacy-availability-fixture");
        fs::create_dir_all(&storage_root).unwrap();
        let equipment_path = storage_root.join("equipment.sqlite");
        apply_migrations_through(
            &equipment_path,
            &repo_root().join("storage/sqlite/equipment"),
            8,
        );
        let equipment = Connection::open(&equipment_path).unwrap();
        for state in [
            "available",
            "reserved",
            "assigned_to_setup",
            "in_test",
            "unavailable",
        ] {
            let asset_id = format!("LEGACY-{}", state.to_ascii_uppercase());
            equipment
                .execute(
                    "INSERT INTO physical_assets (
                        asset_id, inventory_code, manufacturer_snapshot, model_name_snapshot,
                        category_code_snapshot, category_path_json, ownership_source,
                        service_state, availability_state, service_state_reason, notes, revision,
                        model_link_state, migrated_from_metrology, created_at, updated_at,
                        migration_evidence_json
                     ) VALUES (?1, ?1, 'Legacy', 'Legacy model', 'legacy', '[\"Legacy\"]',
                        'laboratory_owned', 'usable', ?2, '', '', 1,
                        'migration_review_required', 1, '2026-07-01T00:00:00Z',
                        '2026-07-01T00:00:00Z', '{}')",
                    params![asset_id, state],
                )
                .unwrap();
        }
        drop(equipment);

        apply_migrations_from(
            &equipment_path,
            &repo_root().join("storage/sqlite/equipment"),
            9,
        );
        let equipment = Connection::open(&equipment_path).unwrap();
        for legacy_state in [
            "available",
            "reserved",
            "assigned_to_setup",
            "in_test",
            "unavailable",
        ] {
            let asset_id = format!("LEGACY-{}", legacy_state.to_ascii_uppercase());
            let migrated: (String, String, String, String) = equipment
                .query_row(
                    "SELECT availability_state, administrative_availability,
                        administrative_unavailability_reason,
                        legacy_availability_evidence_json
                     FROM physical_assets WHERE asset_id = ?1",
                    params![asset_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .unwrap();
            let expected = if legacy_state == "unavailable" {
                "unavailable"
            } else {
                "available"
            };
            assert_eq!(migrated.0, expected);
            assert_eq!(migrated.1, expected);
            assert_eq!(
                migrated.2.is_empty(),
                legacy_state != "unavailable",
                "unexpected reason migration for {legacy_state}"
            );
            assert!(migrated.3.contains(legacy_state));
        }
        let migration_count: u64 = equipment
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 9",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(migration_count, 1);

        let _ = fs::remove_dir_all(storage_root);
    }

    fn seed_0_21_1_fixture(storage_root: &Path) {
        let equipment = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        equipment
            .execute(
                "INSERT INTO equipment_model_identities (
                    equipment_model_id, manufacturer, model_name, variant, equipment_class,
                    category_code, current_approved_revision_id, created_by, created_at, updated_at
                 ) VALUES ('EQM-LEGACY-SCOPE', 'Acme Test', 'Scope 1', NULL,
                    'controllable_instrument', 'oscilloscope', NULL, 'fixture',
                    '2026-07-01T00:00:00Z', '2026-07-01T00:00:00Z')",
                [],
            )
            .unwrap();
        let definition = render_json(&json!({
            "definition_schema_version": "emc-locus.equipment-model-definition.v2",
            "manufacturer": "Acme Test",
            "model_name": "Scope 1",
            "equipment_class": "controllable_instrument",
            "functional_role": "measurement_instrument",
            "category_code": "oscilloscope",
            "signal_domains": ["rf"],
            "technology_tags": [],
            "specifications": [],
            "signal_ports": [{
                "port_id": "rf_input",
                "label": "Entrée RF",
                "directionality": "input",
                "flow_role": "measurement_port",
                "signal_domain": "rf",
                "required": true,
                "technology_tags": [],
                "quantity": "voltage",
                "unit": "V",
                "impedance": 50.0,
                "differential": false,
                "isolated": false
            }],
            "communication_interfaces": [],
            "capabilities": [],
            "metadata": {}
        }));
        let canonical = EquipmentModelDefinition::from_json_str(&definition)
            .unwrap()
            .canonicalize()
            .unwrap();
        let checksum = canonical.definition_checksum;
        equipment
            .execute(
                "INSERT INTO equipment_model_revisions (
                    revision_id, equipment_model_id, revision_number, status,
                    definition_schema_version, definition_json, definition_checksum,
                    created_by, created_at, updated_at, submitted_at, approved_at
                 ) VALUES ('EQM-LEGACY-SCOPE-REV-0001', 'EQM-LEGACY-SCOPE', 1, 'approved',
                    'emc-locus.equipment-model-definition.v2', ?1, ?2, 'fixture',
                    '2026-07-01T00:00:00Z', '2026-07-01T00:00:00Z',
                    '2026-07-01T00:00:00Z', '2026-07-01T00:00:00Z')",
                params![canonical.canonical_json, checksum],
            )
            .unwrap();
        equipment
            .execute(
                "UPDATE equipment_model_identities SET current_approved_revision_id =
                    'EQM-LEGACY-SCOPE-REV-0001' WHERE equipment_model_id = 'EQM-LEGACY-SCOPE'",
                [],
            )
            .unwrap();

        let metrology = Connection::open(storage_root.join("metrology.sqlite")).unwrap();
        metrology
            .execute_batch("PRAGMA foreign_keys = ON;")
            .unwrap();
        metrology
            .execute(
                "INSERT INTO instruments (
                    asset_id, family, manufacturer, model, serial_number, availability,
                    calibration_requirement, capabilities_json, category_code, part_number,
                    calibration_period_months, metrology_notes, serviceability_status,
                    serviceability_reason, serviceability_updated_at, legacy_availability,
                    calibration_due_warning_days, equipment_model_id,
                    equipment_model_revision_id, equipment_model_checksum, created_at, updated_at
                 ) VALUES (
                    'LEGACY-SA-001', 'Oscilloscope', 'Acme Test', 'Scope 1', 'SN-LEGACY-001',
                    'reserved', 'required', '{}', 'oscilloscope', 'PN-SCOPE', 12,
                    'Dossier historique', 'usable', '', '2026-07-01T00:00:00Z', 'reserved', 30,
                    'EQM-LEGACY-SCOPE', 'EQM-LEGACY-SCOPE-REV-0001', ?1,
                    '2026-07-01T00:00:00Z', '2026-07-02T00:00:00Z'
                 )",
                params![checksum],
            )
            .unwrap();
        metrology
            .execute(
                "INSERT INTO instruments (
                    asset_id, family, manufacturer, model, serial_number, availability,
                    calibration_requirement, capabilities_json, category_code,
                    calibration_period_months, metrology_notes, serviceability_status,
                    serviceability_reason, serviceability_updated_at, legacy_availability,
                    calibration_due_warning_days, created_at, updated_at
                 ) VALUES (
                    'LEGACY-CABLE-002', 'Cable RF', 'Legacy Cables', 'RF-2M', 'CABLE-002',
                    'available', 'conditional', '{}', 'emc_antennas_probes', 24,
                    'A rattacher au catalogue', 'restricted', 'Usage jusque 1 GHz',
                    '2026-07-01T00:00:00Z', 'available', 45,
                    '2026-07-01T00:00:00Z', '2026-07-02T00:00:00Z'
                 )",
                [],
            )
            .unwrap();
        metrology
            .execute(
                "INSERT INTO calibration_events (
                    event_id, asset_id, certificate_reference, calibrated_at, due_at,
                    provider, decision, uncertainty_summary_json, recorded_at, recorded_by, revision
                 ) VALUES ('CAL-LEGACY-001', 'LEGACY-SA-001', 'CERT-LEGACY-001',
                    '2026-01-01', '2027-01-01', 'Accredited Lab', 'conforming', '{}',
                    '2026-01-02T00:00:00Z', 'fixture', 'rev-cal-001')",
                [],
            )
            .unwrap();
        let correction_checksum = format!("sha256:{}", "b".repeat(64));
        metrology
            .execute(
                "INSERT INTO asset_characterization_events (
                    characterization_id, asset_id, characterization_kind, label, performed_on,
                    valid_until, provider, method_reference, decision, definition_schema_version,
                    definition_json, definition_checksum, comment, recorded_at, recorded_by,
                    revision, source_kind, valid_from, environmental_conditions_json
                 ) VALUES ('CHAR-LEGACY-001', 'LEGACY-SA-001', 'time_conversion',
                    'Facteur historique', '2026-01-01', '2027-01-01', 'Internal Lab',
                    'MET-001', 'conforming', 'emc-locus.asset-characterization-definition.v1',
                    '{}', ?1, '', '2026-01-02T00:00:00Z', 'fixture', 'rev-char-001',
                    'characterization', '2026-01-01', '{}')",
                params![correction_checksum],
            )
            .unwrap();
        metrology
            .execute(
                "INSERT INTO asset_correction_assignments (
                    assignment_id, asset_id, equipment_model_id, equipment_model_revision_id,
                    equipment_model_checksum, signal_path_id, requirement_id,
                    correction_definition_id, correction_revision_id, correction_checksum,
                    source_event_id, source_kind, valid_from, status, conditions_json,
                    assigned_at, assigned_by, updated_at, revision
                 ) VALUES ('ASSIGN-LEGACY-001', 'LEGACY-SA-001', 'EQM-LEGACY-SCOPE',
                    'EQM-LEGACY-SCOPE-REV-0001', ?1, 'input', 'gain', 'CORR-001',
                    'CORR-001-REV-1', ?2, 'CHAR-LEGACY-001', 'characterization',
                    '2026-01-01', 'draft', '{}', '2026-01-02T00:00:00Z', 'fixture',
                    '2026-01-02T00:00:00Z', 'rev-assign-001')",
                params![checksum, correction_checksum],
            )
            .unwrap();
    }

    fn apply_migrations_through(database: &Path, root: &Path, maximum: u32) {
        apply_migration_range(database, root, 1, maximum);
    }

    fn apply_migrations_from(database: &Path, root: &Path, minimum: u32) {
        apply_migration_range(database, root, minimum, u32::MAX);
    }

    fn apply_migration_range(database: &Path, root: &Path, minimum: u32, maximum: u32) {
        let connection = Connection::open(database).unwrap();
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .unwrap();
        let mut migrations = fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "sql"))
            .filter(|path| {
                let version = path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .split('_')
                    .next()
                    .unwrap()
                    .parse::<u32>()
                    .unwrap();
                (minimum..=maximum).contains(&version)
            })
            .collect::<Vec<_>>();
        migrations.sort();
        for migration in migrations {
            connection
                .execute_batch(&fs::read_to_string(migration).unwrap())
                .unwrap();
        }
    }

    fn temporary_storage_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "emc-locus-{name}-{}",
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ))
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }
}
