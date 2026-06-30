use crate::metrology_dto::{instrument_dto, InstrumentEnvelopeDto, InstrumentListDto};
use crate::metrology_repository::{
    load_instrument, load_instruments, load_latest_calibration_record, open_metrology_connection,
};
use crate::{render_json, AgentError};
use std::path::Path;

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
