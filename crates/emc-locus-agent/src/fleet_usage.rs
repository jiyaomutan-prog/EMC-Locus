use crate::fleet_dto::{OperationalUsageEvidenceDto, OperationalUsageSummaryDto};
use crate::fleet_repository::StoredPhysicalAsset;
use emc_locus_core::{operational_usage_state_code, OperationalUsageState};
use rusqlite::{params, Connection};
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub(crate) fn compute_operational_usage(
    connection: &Connection,
    asset: &StoredPhysicalAsset,
    assessed_at: OffsetDateTime,
) -> OperationalUsageSummaryDto {
    let assessed_at_text = assessed_at
        .format(&Rfc3339)
        .unwrap_or_else(|_| assessed_at.unix_timestamp().to_string());
    let mut evidence = administrative_and_service_evidence(asset);
    append_active_test_evidence(connection, asset, assessed_at, &mut evidence);
    append_schedule_evidence(connection, asset, assessed_at, &mut evidence);
    append_setup_evidence(connection, asset, &mut evidence);

    let state = if evidence.iter().any(|item| {
        item.blocks_selection
            && matches!(
                item.source_kind.as_str(),
                "administrative_availability" | "service_state"
            )
    }) {
        OperationalUsageState::Unavailable
    } else if evidence
        .iter()
        .any(|item| item.source_kind == "active_test" && item.blocks_selection)
    {
        OperationalUsageState::InTest
    } else if evidence
        .iter()
        .any(|item| item.source_kind == "planned_test_reservation" && item.blocks_selection)
    {
        OperationalUsageState::Reserved
    } else if evidence
        .iter()
        .any(|item| item.source_kind == "station_setup_reference")
    {
        OperationalUsageState::AssignedToSetup
    } else {
        OperationalUsageState::Available
    };

    OperationalUsageSummaryDto {
        state: operational_usage_state_code(state).to_owned(),
        assessed_at: assessed_at_text,
        evidence,
    }
}

fn administrative_and_service_evidence(
    asset: &StoredPhysicalAsset,
) -> Vec<OperationalUsageEvidenceDto> {
    let mut evidence = Vec::new();
    if asset.administrative_availability == "unavailable" {
        evidence.push(OperationalUsageEvidenceDto {
            source_kind: "administrative_availability".to_owned(),
            source_identifier: asset.asset_id.clone(),
            source_label: "Décision du parc matériel".to_owned(),
            relevant_start_at: None,
            relevant_end_at: None,
            reason: asset.administrative_unavailability_reason.clone(),
            blocks_selection: true,
        });
    }
    let service_blocks = matches!(
        asset.service_state.as_str(),
        "in_maintenance" | "out_of_service" | "retired"
    );
    if service_blocks || asset.service_state == "restricted" {
        evidence.push(OperationalUsageEvidenceDto {
            source_kind: "service_state".to_owned(),
            source_identifier: asset.asset_id.clone(),
            source_label: "État technique de l'exemplaire".to_owned(),
            relevant_start_at: None,
            relevant_end_at: None,
            reason: if asset.service_state_reason.trim().is_empty() {
                format!("État de service : {}", asset.service_state)
            } else {
                asset.service_state_reason.clone()
            },
            blocks_selection: service_blocks,
        });
    }
    evidence
}

fn append_active_test_evidence(
    connection: &Connection,
    asset: &StoredPhysicalAsset,
    assessed_at: OffsetDateTime,
    evidence: &mut Vec<OperationalUsageEvidenceDto>,
) {
    let query = connection.prepare(
        "SELECT CAST(run.id AS TEXT), project.code, campaign.name,
                run.started_at, run.completed_at
         FROM projects_db.measurement_run_instruments instrument
         JOIN projects_db.measurement_runs run ON run.id = instrument.measurement_run_id
         JOIN projects_db.campaigns campaign ON campaign.id = run.campaign_id
         JOIN projects_db.projects project ON project.code = campaign.project_code
         WHERE instrument.asset_id = ?1",
    );
    let Ok(mut statement) = query else {
        push_source_unavailable(evidence, "test_execution_source", "Essais en cours");
        return;
    };
    let rows = statement.query_map(params![asset.asset_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    });
    let Ok(rows) = rows else {
        push_source_unavailable(evidence, "test_execution_source", "Essais en cours");
        return;
    };
    for row in rows.flatten() {
        if interval_contains(assessed_at, &row.3, row.4.as_deref()) {
            evidence.push(OperationalUsageEvidenceDto {
                source_kind: "active_test".to_owned(),
                source_identifier: row.0,
                source_label: format!("{} · {}", row.1, row.2),
                relevant_start_at: Some(row.3),
                relevant_end_at: row.4,
                reason: "Cet exemplaire est utilisé par une mesure démarrée.".to_owned(),
                blocks_selection: true,
            });
        }
    }
}

fn append_schedule_evidence(
    connection: &Connection,
    asset: &StoredPhysicalAsset,
    assessed_at: OffsetDateTime,
    evidence: &mut Vec<OperationalUsageEvidenceDto>,
) {
    let query = connection.prepare(
        "SELECT schedule.item_code, schedule.title, schedule.planned_start_at,
                schedule.planned_end_at, schedule.status, revision.definition_json
         FROM projects_db.service_schedule_items schedule
         JOIN projects_db.planned_test_preparation_identities identity
           ON identity.schedule_item_code = schedule.item_code
         JOIN projects_db.planned_test_preparation_revisions revision
           ON revision.revision_id = identity.current_revision_id
         WHERE schedule.status IN ('planned', 'confirmed', 'in_progress')",
    );
    let Ok(mut statement) = query else {
        push_source_unavailable(evidence, "planning_source", "Réservations planifiées");
        return;
    };
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
        ))
    });
    let Ok(rows) = rows else {
        push_source_unavailable(evidence, "planning_source", "Réservations planifiées");
        return;
    };
    for row in rows.flatten() {
        if !interval_contains(assessed_at, &row.2, Some(&row.3)) {
            continue;
        }
        let Ok(definition) = serde_json::from_str::<Value>(&row.5) else {
            continue;
        };
        if !json_array_references_asset(&definition, "/station_setup/assets", &asset.asset_id) {
            continue;
        }
        let in_progress = row.4 == "in_progress";
        evidence.push(OperationalUsageEvidenceDto {
            source_kind: if in_progress {
                "active_test".to_owned()
            } else {
                "planned_test_reservation".to_owned()
            },
            source_identifier: row.0,
            source_label: row.1,
            relevant_start_at: Some(row.2),
            relevant_end_at: Some(row.3),
            reason: if in_progress {
                "L'essai planifié utilisant cet exemplaire a démarré.".to_owned()
            } else {
                "Cet exemplaire est réservé par une préparation d'essai sur ce créneau.".to_owned()
            },
            blocks_selection: true,
        });
    }
}

fn append_setup_evidence(
    connection: &Connection,
    asset: &StoredPhysicalAsset,
    evidence: &mut Vec<OperationalUsageEvidenceDto>,
) {
    let query = connection.prepare(
        "SELECT identity.setup_id, identity.label, revision.definition_json
         FROM station_db.station_setup_identities identity
         JOIN station_db.station_setup_revisions revision
           ON revision.revision_id = identity.current_ready_revision_id",
    );
    let Ok(mut statement) = query else {
        push_source_unavailable(evidence, "station_setup_source", "Montages de mesure");
        return;
    };
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    });
    let Ok(rows) = rows else {
        push_source_unavailable(evidence, "station_setup_source", "Montages de mesure");
        return;
    };
    for row in rows.flatten() {
        let Ok(definition) = serde_json::from_str::<Value>(&row.2) else {
            continue;
        };
        if json_array_references_asset(&definition, "/asset_bindings", &asset.asset_id) {
            evidence.push(OperationalUsageEvidenceDto {
                source_kind: "station_setup_reference".to_owned(),
                source_identifier: row.0,
                source_label: row.1,
                relevant_start_at: None,
                relevant_end_at: None,
                reason: "Cet exemplaire est référencé par un montage prêt. Cette référence n'est pas exclusive à elle seule.".to_owned(),
                blocks_selection: false,
            });
        }
    }
}

fn json_array_references_asset(definition: &Value, pointer: &str, asset_id: &str) -> bool {
    definition
        .pointer(pointer)
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|item| {
                item.get("asset_id")
                    .and_then(Value::as_str)
                    .is_some_and(|candidate| candidate == asset_id)
            })
        })
}

fn interval_contains(assessed_at: OffsetDateTime, start_at: &str, end_at: Option<&str>) -> bool {
    let Ok(start) = OffsetDateTime::parse(start_at, &Rfc3339) else {
        return false;
    };
    let end = end_at.and_then(|value| OffsetDateTime::parse(value, &Rfc3339).ok());
    start <= assessed_at && end.is_none_or(|value| assessed_at < value)
}

fn push_source_unavailable(
    evidence: &mut Vec<OperationalUsageEvidenceDto>,
    source_kind: &str,
    source_label: &str,
) {
    if evidence.iter().any(|item| item.source_kind == source_kind) {
        return;
    }
    evidence.push(OperationalUsageEvidenceDto {
        source_kind: source_kind.to_owned(),
        source_identifier: String::new(),
        source_label: source_label.to_owned(),
        relevant_start_at: None,
        relevant_end_at: None,
        reason: "Cette source d'usage est temporairement indisponible.".to_owned(),
        blocks_selection: false,
    });
}
