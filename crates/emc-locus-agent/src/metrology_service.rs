use crate::metrology_dto::{instrument_dto, InstrumentEnvelopeDto, InstrumentListDto};
use crate::metrology_repository::{
    insert_instrument, load_instrument, load_instruments, load_latest_calibration_record,
    open_metrology_connection, NewInstrumentRecord,
};
use crate::{render_json, AgentError};
use emc_locus_core::{DomainError, InstrumentCode};
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisterInstrumentInput {
    pub asset_id: String,
    pub family: String,
    pub category_code: String,
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub part_number: Option<String>,
    pub calibration_requirement: String,
    pub calibration_period_months: Option<u32>,
    pub serviceability_status: String,
    pub serviceability_reason: String,
    pub capabilities_json: String,
    pub metrology_notes: String,
}

pub fn list_metrology_instruments(storage_root: &Path) -> Result<String, AgentError> {
    let connection = open_metrology_connection(storage_root)?;
    let instruments = load_instruments(&connection)?;
    let mut instrument_dtos = Vec::with_capacity(instruments.len());
    for instrument in &instruments {
        let calibration = load_latest_calibration_record(&connection, &instrument.asset_id)?;
        instrument_dtos.push(instrument_dto(instrument, calibration.as_ref()));
    }
    Ok(render_json(&InstrumentListDto {
        instruments: instrument_dtos,
    }))
}

pub fn get_metrology_instrument(storage_root: &Path, asset_id: &str) -> Result<String, AgentError> {
    let connection = open_metrology_connection(storage_root)?;
    let instrument = load_instrument(&connection, asset_id)?.ok_or_else(|| {
        AgentError::new(
            "metrology_instrument_not_found",
            format!("instrument does not exist: {asset_id}"),
        )
    })?;
    let calibration = load_latest_calibration_record(&connection, &instrument.asset_id)?;
    Ok(render_json(&InstrumentEnvelopeDto {
        instrument: instrument_dto(&instrument, calibration.as_ref()),
    }))
}

pub fn register_metrology_instrument(
    storage_root: &Path,
    input: RegisterInstrumentInput,
) -> Result<String, AgentError> {
    let asset_id = InstrumentCode::parse(input.asset_id.clone()).map_err(domain_error)?;
    require_non_empty(&input.family, "family")?;
    require_non_empty(&input.category_code, "category_code")?;
    require_non_empty(&input.manufacturer, "manufacturer")?;
    require_non_empty(&input.model, "model")?;
    require_non_empty(&input.serial_number, "serial_number")?;
    validate_calibration_requirement(&input.calibration_requirement)?;
    validate_serviceability_status(&input.serviceability_status)?;
    validate_capabilities_json(&input.capabilities_json)?;
    if input.calibration_period_months == Some(0) {
        return Err(AgentError::new(
            "invalid_metrology_instrument",
            "calibration_period_months must be positive",
        ));
    }

    let connection = open_metrology_connection(storage_root)?;
    if load_instrument(&connection, asset_id.as_str())?.is_some() {
        return Err(AgentError::new(
            "metrology_instrument_already_exists",
            format!("instrument already exists: {}", asset_id.as_str()),
        ));
    }
    let now = utc_timestamp()?;
    insert_instrument(
        &connection,
        NewInstrumentRecord {
            asset_id: asset_id.as_str(),
            family: input.family.trim(),
            manufacturer: input.manufacturer.trim(),
            model: input.model.trim(),
            serial_number: input.serial_number.trim(),
            category_code: input.category_code.trim(),
            part_number: input
                .part_number
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
            calibration_requirement: input.calibration_requirement.trim(),
            calibration_period_months: input.calibration_period_months,
            capabilities_json: input.capabilities_json.trim(),
            metrology_notes: input.metrology_notes.trim(),
            serviceability_status: input.serviceability_status.trim(),
            serviceability_reason: input.serviceability_reason.trim(),
            timestamp: &now,
        },
    )?;
    get_metrology_instrument(storage_root, asset_id.as_str())
}

fn validate_calibration_requirement(value: &str) -> Result<(), AgentError> {
    match value.trim() {
        "required" | "conditional" | "not_required" => Ok(()),
        other => Err(AgentError::new(
            "invalid_metrology_instrument",
            format!("unknown calibration requirement: {other}"),
        )),
    }
}

fn validate_serviceability_status(value: &str) -> Result<(), AgentError> {
    match value.trim() {
        "usable" | "restricted" | "out_of_service" | "retired" => Ok(()),
        other => Err(AgentError::new(
            "invalid_metrology_instrument",
            format!("unknown serviceability status: {other}"),
        )),
    }
}

fn validate_capabilities_json(value: &str) -> Result<(), AgentError> {
    let parsed = serde_json::from_str::<serde_json::Value>(value)
        .map_err(|error| AgentError::new("invalid_metrology_instrument", error.to_string()))?;
    if !(parsed.is_array() || parsed.is_object()) {
        return Err(AgentError::new(
            "invalid_metrology_instrument",
            "capabilities_json must be a JSON object or array",
        ));
    }
    Ok(())
}

fn require_non_empty(value: &str, field: &'static str) -> Result<(), AgentError> {
    if value.trim().is_empty() {
        return Err(AgentError::new(
            "invalid_metrology_instrument",
            format!("{field} is required"),
        ));
    }
    Ok(())
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_failed", error.to_string()))
}

fn domain_error(error: DomainError) -> AgentError {
    AgentError::new("domain_error", format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};
    use serde_json::Value;
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn list_metrology_instruments_exposes_serviceability_dto() {
        let storage_root = initialized_storage_root("metrology-service-list");
        let connection = open_metrology_connection(&storage_root).unwrap();
        insert_instrument_with_calibration(&connection);

        let value: Value =
            serde_json::from_str(&list_metrology_instruments(&storage_root).unwrap()).unwrap();
        let instruments = value["instruments"].as_array().unwrap();

        assert_eq!(instruments.len(), 1);
        assert_eq!(instruments[0]["asset_id"], "SA-001");
        assert_eq!(instruments[0]["serviceability_status"], "usable");
        assert_eq!(instruments[0]["legacy_availability"], "reserved");
        assert_eq!(
            instruments[0]["latest_calibration"]["certificate_reference"],
            "CERT-SA-001"
        );
        assert!(instruments[0]["revision"]
            .as_str()
            .unwrap()
            .starts_with("rev-"));

        drop(connection);
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn register_metrology_instrument_persists_agent_owned_asset() {
        let storage_root = initialized_storage_root("metrology-service-register");

        let value: Value = serde_json::from_str(
            &register_metrology_instrument(
                &storage_root,
                RegisterInstrumentInput {
                    asset_id: "SA-REG-001".to_owned(),
                    family: "SpectrumAnalyzer".to_owned(),
                    category_code: "spectrum_analyzer".to_owned(),
                    manufacturer: "Rohde Schwarz".to_owned(),
                    model: "FSW".to_owned(),
                    serial_number: "REG-001".to_owned(),
                    part_number: Some("FSW44".to_owned()),
                    calibration_requirement: "required".to_owned(),
                    calibration_period_months: Some(12),
                    serviceability_status: "usable".to_owned(),
                    serviceability_reason: String::new(),
                    capabilities_json: "{\"frequency_max_hz\":44000000000}".to_owned(),
                    metrology_notes: "Agent registration".to_owned(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(value["instrument"]["asset_id"], "SA-REG-001");
        assert_eq!(value["instrument"]["category_code"], "spectrum_analyzer");
        assert_eq!(value["instrument"]["part_number"], "FSW44");
        assert_eq!(value["instrument"]["serviceability_status"], "usable");
        assert_eq!(value["instrument"]["latest_calibration"], Value::Null);
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn get_metrology_instrument_reports_missing_assets_structurally() {
        let storage_root = initialized_storage_root("metrology-service-get-missing");

        let error = get_metrology_instrument(&storage_root, "MISSING").unwrap_err();

        assert_eq!(error.code, "metrology_instrument_not_found");
        remove_temporary_storage_root(&storage_root);
    }

    fn insert_instrument_with_calibration(connection: &rusqlite::Connection) {
        connection
            .execute(
                concat!(
                    "INSERT INTO instruments (asset_id, family, manufacturer, model, ",
                    "serial_number, availability, calibration_requirement, capabilities_json, ",
                    "category_code, part_number, calibration_period_months, metrology_notes, ",
                    "serviceability_status, serviceability_reason, serviceability_updated_at, ",
                    "legacy_availability, created_at, updated_at) ",
                    "VALUES (?1, 'SpectrumAnalyzer', 'Rohde Schwarz', 'FSW', '100001', ",
                    "'reserved', 'required', '{\"frequency_max_hz\":44000000000}', ",
                    "'spectrum_analyzer', 'FSW44', 12, 'Field unit', 'usable', ",
                    "'Migrated reservation', '2026-06-30T00:00:00Z', 'reserved', ",
                    "'2026-06-30T00:00:00Z', '2026-06-30T00:00:00Z')"
                ),
                params!["SA-001"],
            )
            .unwrap();
        connection
            .execute(
                concat!(
                    "INSERT INTO calibration_records (asset_id, certificate_reference, ",
                    "calibrated_at, due_at, provider, status_at_import, uncertainty_json, ",
                    "file_reference, checksum, created_at) ",
                    "VALUES ('SA-001', 'CERT-SA-001', '2026-06-30', '2027-06-30', ",
                    "'Accredited Lab', 'valid', '{\"level_db\":0.6}', ",
                    "'metrology/SA-001/cert.pdf', 'sha256:cert', '2026-06-30T00:00:00Z')"
                ),
                [],
            )
            .unwrap();
    }

    fn initialized_storage_root(name: &str) -> PathBuf {
        let storage_root = temporary_storage_root(name);
        fs::create_dir_all(&storage_root).unwrap();
        let connection = Connection::open(storage_root.join("metrology.sqlite")).unwrap();
        let migrations_root = repo_root().join("storage/sqlite/metrology");
        let mut migrations = fs::read_dir(migrations_root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "sql"))
            .collect::<Vec<_>>();
        migrations.sort();
        for migration in migrations {
            let sql = fs::read_to_string(migration).unwrap();
            connection.execute_batch(&sql).unwrap();
        }
        storage_root
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("agent crate should be inside crates/")
            .to_path_buf()
    }

    fn temporary_storage_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("emc-locus-{name}-{}-{suffix}", std::process::id()));
        if root.exists() {
            remove_temporary_storage_root(&root);
        }
        root
    }

    fn remove_temporary_storage_root(root: &Path) {
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }
}
