use crate::metrology_dto::{
    calibration_event_dto, instrument_dto, CalibrationEventEnvelopeDto, CalibrationEventListDto,
    CalibrationStatusDto, InstrumentEnvelopeDto, InstrumentListDto,
};
use crate::metrology_repository::{
    insert_calibration_event, insert_instrument, load_calibration_event, load_calibration_events,
    load_instrument, load_instruments, load_latest_calibration_event,
    load_latest_calibration_record, open_metrology_connection, NewCalibrationEventRecord,
    NewInstrumentRecord, StoredCalibrationEvent, StoredInstrument,
};
use crate::{render_json, AgentError};
use emc_locus_core::{
    DomainError, InstrumentCode, MetrologyDate, DEFAULT_CALIBRATION_DUE_SOON_WARNING_DAYS,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
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
    pub calibration_due_warning_days: Option<u32>,
    pub serviceability_status: String,
    pub serviceability_reason: String,
    pub capabilities_json: String,
    pub metrology_notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordCalibrationInput {
    pub event_id: String,
    pub asset_id: String,
    pub certificate_reference: String,
    pub calibrated_at: String,
    pub due_at: String,
    pub provider: String,
    pub decision: String,
    pub as_found_status: Option<String>,
    pub as_left_status: Option<String>,
    pub adjustment_performed: bool,
    pub uncertainty_summary_json: String,
    pub traceability_reference: Option<String>,
    pub comment: String,
    pub document_manifest_json: Option<String>,
    pub recorded_by: String,
}

pub fn list_metrology_instruments(storage_root: &Path) -> Result<String, AgentError> {
    let connection = open_metrology_connection(storage_root)?;
    let instruments = load_instruments(&connection)?;
    let mut instrument_dtos = Vec::with_capacity(instruments.len());
    for instrument in &instruments {
        let calibration = load_latest_calibration_record(&connection, &instrument.asset_id)?;
        let calibration_event = load_latest_calibration_event(&connection, &instrument.asset_id)?;
        instrument_dtos.push(instrument_dto(
            instrument,
            calibration.as_ref(),
            calibration_event.as_ref(),
        ));
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
    let calibration_event = load_latest_calibration_event(&connection, &instrument.asset_id)?;
    Ok(render_json(&InstrumentEnvelopeDto {
        instrument: instrument_dto(
            &instrument,
            calibration.as_ref(),
            calibration_event.as_ref(),
        ),
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
    if input.calibration_due_warning_days == Some(0) {
        return Err(AgentError::new(
            "invalid_metrology_instrument",
            "calibration_due_warning_days must be positive",
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
            calibration_due_warning_days: input
                .calibration_due_warning_days
                .unwrap_or(DEFAULT_CALIBRATION_DUE_SOON_WARNING_DAYS),
            capabilities_json: input.capabilities_json.trim(),
            metrology_notes: input.metrology_notes.trim(),
            serviceability_status: input.serviceability_status.trim(),
            serviceability_reason: input.serviceability_reason.trim(),
            timestamp: &now,
        },
    )?;
    get_metrology_instrument(storage_root, asset_id.as_str())
}

pub fn record_metrology_calibration(
    storage_root: &Path,
    input: RecordCalibrationInput,
) -> Result<String, AgentError> {
    let event_id = safe_identifier(&input.event_id, "event_id")?;
    let asset_id = InstrumentCode::parse(input.asset_id.clone()).map_err(domain_error)?;
    require_non_empty(&input.certificate_reference, "certificate_reference")?;
    require_non_empty(&input.provider, "provider")?;
    require_non_empty(&input.recorded_by, "recorded_by")?;
    validate_calibration_decision(&input.decision)?;
    validate_optional_calibration_decision(input.as_found_status.as_deref(), "as_found_status")?;
    validate_optional_calibration_decision(input.as_left_status.as_deref(), "as_left_status")?;
    validate_json_object(&input.uncertainty_summary_json, "uncertainty_summary_json")?;
    validate_document_manifest(input.document_manifest_json.as_deref())?;
    let calibrated_at = parse_metrology_date(&input.calibrated_at, "calibrated_at")?;
    let due_at = parse_metrology_date(&input.due_at, "due_at")?;
    if due_at < calibrated_at {
        return Err(AgentError::new(
            "invalid_metrology_calibration",
            "due_at must be on or after calibrated_at",
        ));
    }

    let connection = open_metrology_connection(storage_root)?;
    if load_instrument(&connection, asset_id.as_str())?.is_none() {
        return Err(AgentError::new(
            "metrology_instrument_not_found",
            format!("instrument does not exist: {}", asset_id.as_str()),
        ));
    }
    if load_calibration_event(&connection, event_id.as_str())?.is_some() {
        return Err(AgentError::new(
            "metrology_calibration_already_exists",
            format!("calibration event already exists: {}", event_id.as_str()),
        ));
    }
    if load_calibration_events(&connection, asset_id.as_str())?
        .iter()
        .any(|event| event.certificate_reference == input.certificate_reference.trim())
    {
        return Err(AgentError::new(
            "metrology_calibration_already_exists",
            format!(
                "calibration certificate already exists for {}: {}",
                asset_id.as_str(),
                input.certificate_reference.trim()
            ),
        ));
    }

    let recorded_at = utc_timestamp()?;
    let revision = revision_for("calibration_event", event_id.as_str(), &recorded_at);
    insert_calibration_event(
        &connection,
        NewCalibrationEventRecord {
            event_id: event_id.as_str(),
            asset_id: asset_id.as_str(),
            certificate_reference: input.certificate_reference.trim(),
            calibrated_at: input.calibrated_at.trim(),
            due_at: input.due_at.trim(),
            provider: input.provider.trim(),
            decision: input.decision.trim(),
            as_found_status: trimmed_optional(input.as_found_status.as_deref()),
            as_left_status: trimmed_optional(input.as_left_status.as_deref()),
            adjustment_performed: input.adjustment_performed,
            uncertainty_summary_json: input.uncertainty_summary_json.trim(),
            traceability_reference: trimmed_optional(input.traceability_reference.as_deref()),
            comment: input.comment.trim(),
            document_manifest_json: trimmed_optional(input.document_manifest_json.as_deref()),
            recorded_at: &recorded_at,
            recorded_by: input.recorded_by.trim(),
            revision: &revision,
        },
    )?;
    let event = load_calibration_event(&connection, event_id.as_str())?.ok_or_else(|| {
        AgentError::new(
            "metrology_calibration_query_failed",
            "calibration event was not readable after insert",
        )
    })?;
    Ok(render_json(&CalibrationEventEnvelopeDto {
        calibration_event: calibration_event_dto(&event),
    }))
}

pub fn list_metrology_calibrations(
    storage_root: &Path,
    asset_id: &str,
) -> Result<String, AgentError> {
    let connection = open_metrology_connection(storage_root)?;
    let instrument = load_instrument(&connection, asset_id)?.ok_or_else(|| {
        AgentError::new(
            "metrology_instrument_not_found",
            format!("instrument does not exist: {asset_id}"),
        )
    })?;
    let events = load_calibration_events(&connection, &instrument.asset_id)?;
    Ok(render_json(&CalibrationEventListDto {
        asset_id: instrument.asset_id,
        calibration_events: events.iter().map(calibration_event_dto).collect(),
    }))
}

pub fn get_metrology_calibration_status(
    storage_root: &Path,
    asset_id: &str,
    checked_on: &str,
) -> Result<String, AgentError> {
    let checked_on_date = parse_metrology_date(checked_on, "checked_on")?;
    let connection = open_metrology_connection(storage_root)?;
    let instrument = load_instrument(&connection, asset_id)?.ok_or_else(|| {
        AgentError::new(
            "metrology_instrument_not_found",
            format!("instrument does not exist: {asset_id}"),
        )
    })?;
    let latest = load_latest_calibration_event(&connection, &instrument.asset_id)?;
    let status = computed_status(&instrument, latest.as_ref(), checked_on_date)?;
    Ok(render_json(&status))
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

fn validate_calibration_decision(value: &str) -> Result<(), AgentError> {
    match value.trim() {
        "conforming" | "nonconforming" | "indeterminate" | "not_assessed" => Ok(()),
        other => Err(AgentError::new(
            "invalid_metrology_calibration",
            format!("unknown calibration decision: {other}"),
        )),
    }
}

fn validate_optional_calibration_decision(
    value: Option<&str>,
    field: &'static str,
) -> Result<(), AgentError> {
    if let Some(value) = trimmed_optional(value) {
        validate_calibration_decision(value)
            .map_err(|error| AgentError::new(error.code, format!("{field}: {}", error.message)))?;
    }
    Ok(())
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

fn validate_json_object(value: &str, field: &'static str) -> Result<Value, AgentError> {
    let parsed = serde_json::from_str::<Value>(value)
        .map_err(|error| AgentError::new("invalid_metrology_calibration", error.to_string()))?;
    if !parsed.is_object() {
        return Err(AgentError::new(
            "invalid_metrology_calibration",
            format!("{field} must be a JSON object"),
        ));
    }
    Ok(parsed)
}

fn validate_document_manifest(value: Option<&str>) -> Result<(), AgentError> {
    let Some(value) = trimmed_optional(value) else {
        return Ok(());
    };
    let parsed = validate_json_object(value, "document_manifest_json")?;
    if let Some(sha256) = parsed.get("sha256").and_then(Value::as_str) {
        validate_sha256(sha256)?;
    }
    Ok(())
}

fn validate_sha256(value: &str) -> Result<(), AgentError> {
    if value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(AgentError::new(
        "invalid_metrology_calibration",
        "document_manifest_json.sha256 must be a 64-character hexadecimal SHA-256",
    ))
}

fn safe_identifier(value: &str, field: &'static str) -> Result<String, AgentError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AgentError::new(
            "invalid_metrology_calibration",
            format!("{field} is required"),
        ));
    }
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Ok(value.to_owned());
    }
    Err(AgentError::new(
        "invalid_metrology_calibration",
        format!("{field} contains unsupported characters"),
    ))
}

fn parse_metrology_date(value: &str, field: &'static str) -> Result<MetrologyDate, AgentError> {
    let parts = value.trim().split('-').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(AgentError::new(
            "invalid_metrology_date",
            format!("{field} must use YYYY-MM-DD"),
        ));
    }
    let year = parts[0].parse::<u16>().map_err(|_| {
        AgentError::new(
            "invalid_metrology_date",
            format!("{field} must use YYYY-MM-DD"),
        )
    })?;
    let month = parts[1].parse::<u8>().map_err(|_| {
        AgentError::new(
            "invalid_metrology_date",
            format!("{field} must use YYYY-MM-DD"),
        )
    })?;
    let day = parts[2].parse::<u8>().map_err(|_| {
        AgentError::new(
            "invalid_metrology_date",
            format!("{field} must use YYYY-MM-DD"),
        )
    })?;
    MetrologyDate::new(year, month, day).map_err(domain_error)
}

fn computed_status(
    instrument: &StoredInstrument,
    latest: Option<&StoredCalibrationEvent>,
    checked_on: MetrologyDate,
) -> Result<CalibrationStatusDto, AgentError> {
    let mut reasons = Vec::new();
    if instrument.calibration_requirement == "not_required" {
        reasons.push("calibration_not_required".to_owned());
        return Ok(status_dto(
            instrument,
            checked_on,
            "not_required",
            None,
            None,
            reasons,
        ));
    }

    let Some(latest) = latest else {
        reasons.push("calibration_missing".to_owned());
        return Ok(status_dto(
            instrument, checked_on, "missing", None, None, reasons,
        ));
    };

    if latest.decision != "conforming" {
        reasons.push(format!("calibration_decision_{}", latest.decision));
        return Ok(status_dto(
            instrument,
            checked_on,
            "nonconforming",
            Some(latest),
            Some(latest.due_at.clone()),
            reasons,
        ));
    }

    let due_at = parse_metrology_date(&latest.due_at, "due_at")?;
    let days_until_due = checked_on.days_until(due_at);
    if days_until_due < 0 {
        reasons.push("calibration_expired".to_owned());
        return Ok(status_dto(
            instrument,
            checked_on,
            "expired",
            Some(latest),
            Some(latest.due_at.clone()),
            reasons,
        ));
    }
    if days_until_due <= instrument.calibration_due_warning_days as i32 {
        reasons.push("calibration_due_soon".to_owned());
        return Ok(status_dto(
            instrument,
            checked_on,
            "due_soon",
            Some(latest),
            Some(latest.due_at.clone()),
            reasons,
        ));
    }
    reasons.push("calibration_valid".to_owned());
    Ok(status_dto(
        instrument,
        checked_on,
        "valid",
        Some(latest),
        Some(latest.due_at.clone()),
        reasons,
    ))
}

fn status_dto(
    instrument: &StoredInstrument,
    checked_on: MetrologyDate,
    calibration_status: &str,
    latest: Option<&StoredCalibrationEvent>,
    due_at: Option<String>,
    reasons: Vec<String>,
) -> CalibrationStatusDto {
    CalibrationStatusDto {
        asset_id: instrument.asset_id.clone(),
        checked_on: format!(
            "{:04}-{:02}-{:02}",
            checked_on.year(),
            checked_on.month(),
            checked_on.day()
        ),
        calibration_status: calibration_status.to_owned(),
        serviceability_status: instrument.serviceability_status.clone(),
        calibration_requirement: instrument.calibration_requirement.clone(),
        calibration_due_warning_days: instrument.calibration_due_warning_days,
        due_at,
        decision: latest.map(|event| event.decision.clone()),
        latest_calibration_event_id: latest.map(|event| event.event_id.clone()),
        latest_calibration_revision: latest.map(|event| event.revision.clone()),
        instrument_revision: instrument.revision.clone(),
        reasons,
    }
}

fn trimmed_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
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

fn revision_for(entity_type: &str, entity_id: &str, updated_at: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"emc-locus-agent:");
    hasher.update(entity_type.as_bytes());
    hasher.update(b":");
    hasher.update(entity_id.as_bytes());
    hasher.update(b":");
    hasher.update(updated_at.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    format!("rev-{}", &digest[..12])
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
                    calibration_due_warning_days: Some(45),
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
        assert_eq!(value["instrument"]["calibration_due_warning_days"], 45);
        assert_eq!(value["instrument"]["latest_calibration"], Value::Null);
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn record_calibration_event_and_compute_status() {
        let storage_root = initialized_storage_root("metrology-service-calibration");
        let connection = open_metrology_connection(&storage_root).unwrap();
        insert_instrument_with_calibration(&connection);
        drop(connection);

        let missing: Value = serde_json::from_str(
            &get_metrology_calibration_status(&storage_root, "SA-001", "2026-06-30").unwrap(),
        )
        .unwrap();
        assert_eq!(missing["calibration_status"], "missing");
        assert_eq!(missing["reasons"][0], "calibration_missing");

        let created: Value = serde_json::from_str(
            &record_metrology_calibration(
                &storage_root,
                RecordCalibrationInput {
                    event_id: "CAL-SA-001-2026".to_owned(),
                    asset_id: "SA-001".to_owned(),
                    certificate_reference: "CERT-SA-001-2026".to_owned(),
                    calibrated_at: "2026-06-30".to_owned(),
                    due_at: "2027-06-30".to_owned(),
                    provider: "Accredited Lab".to_owned(),
                    decision: "conforming".to_owned(),
                    as_found_status: Some("conforming".to_owned()),
                    as_left_status: Some("conforming".to_owned()),
                    adjustment_performed: false,
                    uncertainty_summary_json: "{\"level_db\":0.6}".to_owned(),
                    traceability_reference: Some("SI-chain-001".to_owned()),
                    comment: "Annual calibration".to_owned(),
                    document_manifest_json: Some(format!(
                        "{{\"object_id\":\"obj-cert\",\"original_filename\":\"cert.pdf\",\"mime_type\":\"application/pdf\",\"size_bytes\":12,\"sha256\":\"{}\",\"storage_key\":\"metrology/SA-001/cert.pdf\",\"revision\":\"A\"}}",
                        "a".repeat(64)
                    )),
                    recorded_by: "metrology.admin".to_owned(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(created["calibration_event"]["event_id"], "CAL-SA-001-2026");
        assert_eq!(created["calibration_event"]["decision"], "conforming");

        let valid: Value = serde_json::from_str(
            &get_metrology_calibration_status(&storage_root, "SA-001", "2026-07-01").unwrap(),
        )
        .unwrap();
        assert_eq!(valid["calibration_status"], "valid");

        let due_soon: Value = serde_json::from_str(
            &get_metrology_calibration_status(&storage_root, "SA-001", "2027-06-01").unwrap(),
        )
        .unwrap();
        assert_eq!(due_soon["calibration_status"], "due_soon");

        let events: Value =
            serde_json::from_str(&list_metrology_calibrations(&storage_root, "SA-001").unwrap())
                .unwrap();
        assert_eq!(events["calibration_events"].as_array().unwrap().len(), 1);

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn nonconforming_calibration_event_drives_computed_status() {
        let storage_root = initialized_storage_root("metrology-service-nonconforming");
        let connection = open_metrology_connection(&storage_root).unwrap();
        insert_instrument_with_calibration(&connection);
        drop(connection);

        record_metrology_calibration(
            &storage_root,
            RecordCalibrationInput {
                event_id: "CAL-SA-001-NC".to_owned(),
                asset_id: "SA-001".to_owned(),
                certificate_reference: "CERT-SA-001-NC".to_owned(),
                calibrated_at: "2026-06-30".to_owned(),
                due_at: "2027-06-30".to_owned(),
                provider: "Accredited Lab".to_owned(),
                decision: "nonconforming".to_owned(),
                as_found_status: Some("nonconforming".to_owned()),
                as_left_status: Some("nonconforming".to_owned()),
                adjustment_performed: false,
                uncertainty_summary_json: "{}".to_owned(),
                traceability_reference: None,
                comment: "Out of tolerance".to_owned(),
                document_manifest_json: None,
                recorded_by: "metrology.admin".to_owned(),
            },
        )
        .unwrap();

        let status: Value = serde_json::from_str(
            &get_metrology_calibration_status(&storage_root, "SA-001", "2026-07-01").unwrap(),
        )
        .unwrap();
        assert_eq!(status["calibration_status"], "nonconforming");
        assert_eq!(status["reasons"][0], "calibration_decision_nonconforming");

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
