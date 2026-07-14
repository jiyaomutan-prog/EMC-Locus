use crate::file_store::{store_content_addressed_file, FileStorePolicy, StoreLocalFileInput};
use crate::metrology_dto::{
    asset_characterization_dto, calibration_event_dto, instrument_dto, AssetCharacterizationDto,
    AssetCharacterizationEnvelopeDto, AssetCharacterizationListDto, CalibrationEventEnvelopeDto,
    CalibrationEventListDto, CalibrationStatusDto, InstrumentEnvelopeDto, InstrumentListDto,
    MetrologyAuditEventDto, MetrologyAuditEventsDto, ReadinessInstrumentResultDto,
    ReadinessIssueDto, ReadinessReportDto,
};
use crate::metrology_repository::{
    ensure_metrology_operation_replay, existing_metrology_operation, insert_asset_characterization,
    insert_calibration_event, insert_instrument, insert_metrology_audit_event,
    insert_metrology_sync_operation, load_asset_characterization, load_asset_characterizations,
    load_calibration_event, load_calibration_events, load_instrument, load_instruments,
    load_latest_calibration_event, load_latest_calibration_record, load_metrology_audit_events,
    next_metrology_audit_sequence, open_metrology_connection, open_metrology_connection_with_sync,
    update_instrument_serviceability, MetrologyAuditEventInput, MetrologyOperationFingerprintInput,
    MetrologySyncOperationInput, NewAssetCharacterizationRecord, NewCalibrationEventRecord,
    NewInstrumentRecord, StoredAssetCharacterization, StoredCalibrationEvent, StoredInstrument,
};
use crate::{render_json, AgentError};
use emc_locus_core::{
    AssetCharacterizationDefinition, DomainError, InstrumentCode, MetrologyDate,
    DEFAULT_CALIBRATION_DUE_SOON_WARNING_DAYS,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisterInstrumentInput {
    pub asset_id: String,
    pub family: String,
    pub category_code: Option<String>,
    pub equipment_model_id: Option<String>,
    pub equipment_model_revision_id: Option<String>,
    pub equipment_model_checksum: Option<String>,
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
    pub context: MetrologyOperationContext,
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
    pub context: MetrologyOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordAssetCharacterizationInput {
    pub characterization_id: String,
    pub asset_id: String,
    pub performed_on: String,
    pub valid_from: String,
    pub valid_until: String,
    pub source_kind: String,
    pub provider: String,
    pub method_reference: String,
    pub decision: String,
    pub definition_json: String,
    pub certificate_reference: Option<String>,
    pub document_manifest_json: Option<String>,
    pub comment: String,
    pub environmental_conditions_json: String,
    pub as_found_json: Option<String>,
    pub as_left_json: Option<String>,
    pub adjustment_performed: bool,
    pub recorded_by: String,
    pub context: MetrologyOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreMetrologyFileInput {
    pub original_filename: String,
    pub mime_type: String,
    pub content_base64: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetServiceabilityInput {
    pub asset_id: String,
    pub serviceability_status: String,
    pub serviceability_reason: String,
    pub context: MetrologyOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssessReadinessInput {
    pub asset_ids: Vec<String>,
    pub execution_mode: String,
    pub checked_on: String,
    pub context: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetrologyOperationContext {
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

pub(crate) struct MetrologyAuditWrite<'a> {
    pub(crate) entity_type: &'a str,
    pub(crate) entity_id: &'a str,
    pub(crate) sequence: u64,
    pub(crate) action: &'a str,
    pub(crate) base_revision: &'a str,
    pub(crate) resulting_revision: &'a str,
    pub(crate) context: &'a MetrologyOperationContext,
    pub(crate) payload_json: &'a str,
    pub(crate) timestamp: &'a str,
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
    validate_operation_context(&input.context)?;
    require_non_empty(&input.family, "family")?;
    validate_equipment_model_reference(&input)?;
    if input
        .category_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
        && input.equipment_model_id.is_none()
    {
        return Err(AgentError::new(
            "invalid_metrology_instrument",
            "category_code or a complete equipment model reference is required",
        ));
    }
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
    let warning_days = input
        .calibration_due_warning_days
        .unwrap_or(DEFAULT_CALIBRATION_DUE_SOON_WARNING_DAYS);
    let payload_json = register_instrument_payload_json(&input, warning_days);

    let mut connection = open_metrology_connection_with_sync(storage_root)?;
    if let Some(operation) = existing_metrology_operation(&connection, &input.context.operation_id)?
    {
        ensure_metrology_operation_replay(
            &operation,
            &input.context.operation_id,
            MetrologyOperationFingerprintInput {
                entity_type: "instrument",
                entity_id: asset_id.as_str(),
                operation_kind: "instrument_registered",
                base_revision: "rev-0000",
                actor_id: &input.context.actor,
                device_id: &input.context.device_id,
                correlation_id: &input.context.correlation_id,
                payload_json: &payload_json,
            },
        )?;
        return get_metrology_instrument(storage_root, asset_id.as_str());
    }
    if load_instrument(&connection, asset_id.as_str())?.is_some() {
        return Err(AgentError::new(
            "metrology_instrument_already_exists",
            format!("instrument already exists: {}", asset_id.as_str()),
        ));
    }
    let now = utc_timestamp()?;
    let resulting_revision = revision_for("instrument", asset_id.as_str(), &now);
    let sequence = next_metrology_audit_sequence(&connection, "instrument", asset_id.as_str())?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_instrument(
        &transaction,
        NewInstrumentRecord {
            asset_id: asset_id.as_str(),
            family: input.family.trim(),
            manufacturer: input.manufacturer.trim(),
            model: input.model.trim(),
            serial_number: input.serial_number.trim(),
            category_code: trimmed_optional(input.category_code.as_deref()),
            equipment_model_id: trimmed_optional(input.equipment_model_id.as_deref()),
            equipment_model_revision_id: trimmed_optional(
                input.equipment_model_revision_id.as_deref(),
            ),
            equipment_model_checksum: trimmed_optional(input.equipment_model_checksum.as_deref()),
            part_number: input
                .part_number
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
            calibration_requirement: input.calibration_requirement.trim(),
            calibration_period_months: input.calibration_period_months,
            calibration_due_warning_days: warning_days,
            capabilities_json: input.capabilities_json.trim(),
            metrology_notes: input.metrology_notes.trim(),
            serviceability_status: input.serviceability_status.trim(),
            serviceability_reason: input.serviceability_reason.trim(),
            timestamp: &now,
        },
    )?;
    write_metrology_audit_and_outbox(
        &transaction,
        MetrologyAuditWrite {
            entity_type: "instrument",
            entity_id: asset_id.as_str(),
            sequence,
            action: "instrument_registered",
            base_revision: "rev-0000",
            resulting_revision: &resulting_revision,
            context: &input.context,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    get_metrology_instrument(storage_root, asset_id.as_str())
}

pub fn record_metrology_calibration(
    storage_root: &Path,
    input: RecordCalibrationInput,
) -> Result<String, AgentError> {
    let event_id = safe_identifier(&input.event_id, "event_id")?;
    let asset_id = InstrumentCode::parse(input.asset_id.clone()).map_err(domain_error)?;
    validate_operation_context(&input.context)?;
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

    let payload_json = calibration_event_payload_json(&input);
    let mut connection = open_metrology_connection_with_sync(storage_root)?;
    if load_instrument(&connection, asset_id.as_str())?.is_none() {
        return Err(AgentError::new(
            "metrology_instrument_not_found",
            format!("instrument does not exist: {}", asset_id.as_str()),
        ));
    }
    if let Some(operation) = existing_metrology_operation(&connection, &input.context.operation_id)?
    {
        ensure_metrology_operation_replay(
            &operation,
            &input.context.operation_id,
            MetrologyOperationFingerprintInput {
                entity_type: "calibration_event",
                entity_id: event_id.as_str(),
                operation_kind: "calibration_recorded",
                base_revision: "rev-0000",
                actor_id: &input.context.actor,
                device_id: &input.context.device_id,
                correlation_id: &input.context.correlation_id,
                payload_json: &payload_json,
            },
        )?;
        let event = load_calibration_event(&connection, event_id.as_str())?.ok_or_else(|| {
            AgentError::new(
                "operation_replay_missing_entity",
                "operation exists but calibration event is missing",
            )
        })?;
        return Ok(render_json(&CalibrationEventEnvelopeDto {
            calibration_event: calibration_event_dto(&event),
        }));
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
    let sequence =
        next_metrology_audit_sequence(&connection, "calibration_event", event_id.as_str())?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_calibration_event(
        &transaction,
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
    write_metrology_audit_and_outbox(
        &transaction,
        MetrologyAuditWrite {
            entity_type: "calibration_event",
            entity_id: event_id.as_str(),
            sequence,
            action: "calibration_recorded",
            base_revision: "rev-0000",
            resulting_revision: &revision,
            context: &input.context,
            payload_json: &payload_json,
            timestamp: &recorded_at,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    let connection = open_metrology_connection(storage_root)?;
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

pub fn record_asset_characterization(
    storage_root: &Path,
    input: RecordAssetCharacterizationInput,
) -> Result<String, AgentError> {
    validate_operation_context(&input.context)?;
    require_non_empty(&input.provider, "provider")?;
    require_non_empty(&input.method_reference, "method_reference")?;
    require_non_empty(&input.recorded_by, "recorded_by")?;
    validate_characterization_decision(&input.decision)?;
    validate_characterization_source_kind(&input.source_kind)?;
    validate_characterization_document_manifest(input.document_manifest_json.as_deref())?;
    validate_characterization_json_object(
        &input.environmental_conditions_json,
        "environmental_conditions",
    )?;
    if let Some(value) = trimmed_optional(input.as_found_json.as_deref()) {
        validate_characterization_json_object(value, "as_found")?;
    }
    if let Some(value) = trimmed_optional(input.as_left_json.as_deref()) {
        validate_characterization_json_object(value, "as_left")?;
    }

    let performed_on = parse_metrology_date(&input.performed_on, "performed_on")?;
    let valid_from = parse_metrology_date(&input.valid_from, "valid_from")?;
    let valid_until = parse_metrology_date(&input.valid_until, "valid_until")?;
    if valid_until < valid_from || valid_from < performed_on {
        return Err(AgentError::new(
            "invalid_asset_characterization",
            "validity must start on or after the measurement and end on or after valid_from",
        ));
    }

    let definition = AssetCharacterizationDefinition::from_json_str(&input.definition_json)
        .map_err(|issue| invalid_asset_characterization(vec![issue]))?;
    let canonical = definition
        .canonicalize()
        .map_err(invalid_asset_characterization)?;
    if canonical.characterization_id != input.characterization_id.trim() {
        return Err(AgentError::new(
            "invalid_asset_characterization",
            "characterization_id must match the definition",
        ));
    }
    if canonical.asset_id != input.asset_id.trim() {
        return Err(AgentError::new(
            "invalid_asset_characterization",
            "asset_id must match the definition",
        ));
    }

    let payload_json = asset_characterization_payload_json(&input, &canonical);
    let mut connection = open_metrology_connection_with_sync(storage_root)?;
    if load_instrument(&connection, &canonical.asset_id)?.is_none() {
        return Err(AgentError::new(
            "metrology_instrument_not_found",
            format!("instrument does not exist: {}", canonical.asset_id),
        ));
    }
    if let Some(operation) = existing_metrology_operation(&connection, &input.context.operation_id)?
    {
        ensure_metrology_operation_replay(
            &operation,
            &input.context.operation_id,
            MetrologyOperationFingerprintInput {
                entity_type: "asset_characterization",
                entity_id: &canonical.characterization_id,
                operation_kind: "asset_characterization_recorded",
                base_revision: "rev-0000",
                actor_id: &input.context.actor,
                device_id: &input.context.device_id,
                correlation_id: &input.context.correlation_id,
                payload_json: &payload_json,
            },
        )?;
        let stored = load_asset_characterization(&connection, &canonical.characterization_id)?
            .ok_or_else(|| {
                AgentError::new(
                    "operation_replay_missing_entity",
                    "operation exists but asset characterization is missing",
                )
            })?;
        return render_asset_characterization(&stored);
    }
    if load_asset_characterization(&connection, &canonical.characterization_id)?.is_some() {
        return Err(AgentError::new(
            "asset_characterization_already_exists",
            format!(
                "asset characterization already exists: {}",
                canonical.characterization_id
            ),
        ));
    }

    let recorded_at = utc_timestamp()?;
    let revision = revision_for(
        "asset_characterization",
        &canonical.characterization_id,
        &recorded_at,
    );
    let sequence = next_metrology_audit_sequence(
        &connection,
        "asset_characterization",
        &canonical.characterization_id,
    )?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_asset_characterization(
        &transaction,
        NewAssetCharacterizationRecord {
            characterization_id: &canonical.characterization_id,
            asset_id: &canonical.asset_id,
            characterization_kind: canonical.kind.as_str(),
            label: &canonical.label,
            performed_on: input.performed_on.trim(),
            valid_from: input.valid_from.trim(),
            valid_until: input.valid_until.trim(),
            source_kind: input.source_kind.trim(),
            provider: input.provider.trim(),
            method_reference: input.method_reference.trim(),
            decision: input.decision.trim(),
            definition_schema_version: &canonical.definition_schema_version,
            definition_json: &canonical.canonical_json,
            definition_checksum: &canonical.definition_checksum,
            certificate_reference: trimmed_optional(input.certificate_reference.as_deref()),
            document_manifest_json: trimmed_optional(input.document_manifest_json.as_deref()),
            comment: input.comment.trim(),
            recorded_at: &recorded_at,
            recorded_by: input.recorded_by.trim(),
            revision: &revision,
            environmental_conditions_json: input.environmental_conditions_json.trim(),
            as_found_json: trimmed_optional(input.as_found_json.as_deref()),
            as_left_json: trimmed_optional(input.as_left_json.as_deref()),
            adjustment_performed: input.adjustment_performed,
        },
    )?;
    write_metrology_audit_and_outbox(
        &transaction,
        MetrologyAuditWrite {
            entity_type: "asset_characterization",
            entity_id: &canonical.characterization_id,
            sequence,
            action: "asset_characterization_recorded",
            base_revision: "rev-0000",
            resulting_revision: &revision,
            context: &input.context,
            payload_json: &payload_json,
            timestamp: &recorded_at,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    let connection = open_metrology_connection(storage_root)?;
    let stored = load_asset_characterization(&connection, &canonical.characterization_id)?
        .ok_or_else(|| {
            AgentError::new(
                "metrology_characterization_query_failed",
                "asset characterization was not readable after insert",
            )
        })?;
    render_asset_characterization(&stored)
}

pub fn store_metrology_file(
    storage_root: &Path,
    input: StoreMetrologyFileInput,
) -> Result<String, AgentError> {
    store_content_addressed_file(
        storage_root,
        StoreLocalFileInput {
            original_filename: input.original_filename,
            mime_type: input.mime_type,
            content_base64: input.content_base64,
        },
        FileStorePolicy {
            namespace: "metrology",
            invalid_code: "invalid_metrology_file",
            too_large_code: "metrology_file_too_large",
            store_failed_code: "metrology_file_store_failed",
        },
    )
}

pub fn list_asset_characterizations(
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
    let records = load_asset_characterizations(&connection, &instrument.asset_id)?;
    let characterizations = records
        .iter()
        .map(asset_characterization_dto_from_stored)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(render_json(&AssetCharacterizationListDto {
        asset_id: instrument.asset_id,
        characterizations,
    }))
}

pub fn get_asset_characterization(
    storage_root: &Path,
    asset_id: &str,
    characterization_id: &str,
) -> Result<String, AgentError> {
    let connection = open_metrology_connection(storage_root)?;
    let stored =
        load_asset_characterization(&connection, characterization_id)?.ok_or_else(|| {
            AgentError::new(
                "asset_characterization_not_found",
                format!("asset characterization does not exist: {characterization_id}"),
            )
        })?;
    if stored.asset_id != asset_id {
        return Err(AgentError::new(
            "asset_characterization_not_found",
            format!("asset characterization does not belong to instrument: {asset_id}"),
        ));
    }
    render_asset_characterization(&stored)
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

pub fn set_metrology_serviceability(
    storage_root: &Path,
    input: SetServiceabilityInput,
) -> Result<String, AgentError> {
    let asset_id = InstrumentCode::parse(input.asset_id.clone()).map_err(domain_error)?;
    validate_operation_context(&input.context)?;
    validate_serviceability_status(&input.serviceability_status)?;
    require_non_empty(&input.serviceability_reason, "serviceability_reason")?;

    let mut connection = open_metrology_connection_with_sync(storage_root)?;
    let instrument = load_instrument(&connection, asset_id.as_str())?.ok_or_else(|| {
        AgentError::new(
            "metrology_instrument_not_found",
            format!("instrument does not exist: {}", asset_id.as_str()),
        )
    })?;
    let payload_json = serviceability_payload_json(&input);
    if let Some(operation) = existing_metrology_operation(&connection, &input.context.operation_id)?
    {
        ensure_metrology_operation_replay(
            &operation,
            &input.context.operation_id,
            MetrologyOperationFingerprintInput {
                entity_type: "instrument",
                entity_id: asset_id.as_str(),
                operation_kind: "instrument_serviceability_changed",
                base_revision: &instrument.revision,
                actor_id: &input.context.actor,
                device_id: &input.context.device_id,
                correlation_id: &input.context.correlation_id,
                payload_json: &payload_json,
            },
        )?;
        return get_metrology_instrument(storage_root, asset_id.as_str());
    }

    let now = utc_timestamp()?;
    let resulting_revision = revision_for("instrument", asset_id.as_str(), &now);
    let sequence = next_metrology_audit_sequence(&connection, "instrument", asset_id.as_str())?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    update_instrument_serviceability(
        &transaction,
        asset_id.as_str(),
        input.serviceability_status.trim(),
        input.serviceability_reason.trim(),
        &now,
    )?;
    write_metrology_audit_and_outbox(
        &transaction,
        MetrologyAuditWrite {
            entity_type: "instrument",
            entity_id: asset_id.as_str(),
            sequence,
            action: "instrument_serviceability_changed",
            base_revision: &instrument.revision,
            resulting_revision: &resulting_revision,
            context: &input.context,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    get_metrology_instrument(storage_root, asset_id.as_str())
}

pub fn assess_metrology_readiness(
    storage_root: &Path,
    input: AssessReadinessInput,
) -> Result<String, AgentError> {
    let report = assess_metrology_readiness_report(storage_root, input)?;
    Ok(render_json(&report))
}

pub(crate) fn assess_metrology_readiness_report(
    storage_root: &Path,
    input: AssessReadinessInput,
) -> Result<ReadinessReportDto, AgentError> {
    if input.asset_ids.is_empty() {
        return Err(AgentError::new(
            "invalid_metrology_readiness",
            "asset_ids must not be empty",
        ));
    }
    let checked_on = parse_metrology_date(&input.checked_on, "checked_on")?;
    validate_execution_mode(&input.execution_mode)?;
    let connection = open_metrology_connection(storage_root)?;
    let mut instrument_results = Vec::new();
    let mut blocking_issues = Vec::new();
    let mut warnings = Vec::new();

    for asset_id in &input.asset_ids {
        let parsed_asset = InstrumentCode::parse(asset_id.clone()).map_err(domain_error)?;
        let Some(instrument) = load_instrument(&connection, parsed_asset.as_str())? else {
            let issue = ReadinessIssueDto {
                asset_id: parsed_asset.as_str().to_owned(),
                code: "instrument_unknown".to_owned(),
                dimension: "missing_evidence".to_owned(),
                message: "instrument is not registered".to_owned(),
            };
            blocking_issues.push(issue);
            instrument_results.push(ReadinessInstrumentResultDto {
                asset_id: parsed_asset.as_str().to_owned(),
                manufacturer: None,
                model: None,
                serial_number: None,
                serviceability_status: None,
                calibration_requirement: None,
                calibration_status: "unknown".to_owned(),
                due_at: None,
                reasons: vec!["instrument_unknown".to_owned()],
                blocking: true,
                instrument_revision: None,
                calibration_revision: None,
            });
            continue;
        };
        let latest = load_latest_calibration_event(&connection, &instrument.asset_id)?;
        let status = computed_status(&instrument, latest.as_ref(), checked_on)?;
        let mut reasons = status.reasons.clone();
        let mut blocking = false;

        if matches!(
            instrument.serviceability_status.as_str(),
            "out_of_service" | "retired"
        ) {
            blocking = true;
            reasons.push(instrument.serviceability_status.clone());
            blocking_issues.push(ReadinessIssueDto {
                asset_id: instrument.asset_id.clone(),
                code: instrument.serviceability_status.clone(),
                dimension: "serviceability".to_owned(),
                message: "instrument is not operationally usable".to_owned(),
            });
        }
        if instrument.serviceability_status == "restricted" {
            warnings.push(ReadinessIssueDto {
                asset_id: instrument.asset_id.clone(),
                code: "restricted".to_owned(),
                dimension: "serviceability".to_owned(),
                message: "instrument has serviceability restrictions".to_owned(),
            });
        }

        let calibration_blocks = input.execution_mode == "accredited"
            && instrument.calibration_requirement == "required";
        match status.calibration_status.as_str() {
            "missing" | "expired" if calibration_blocks => {
                blocking = true;
                blocking_issues.push(ReadinessIssueDto {
                    asset_id: instrument.asset_id.clone(),
                    code: format!("calibration_{}", status.calibration_status),
                    dimension: calibration_issue_dimension(&status.calibration_status).to_owned(),
                    message: "required calibration is not valid".to_owned(),
                });
            }
            "nonconforming" => {
                blocking = true;
                blocking_issues.push(ReadinessIssueDto {
                    asset_id: instrument.asset_id.clone(),
                    code: "calibration_nonconforming".to_owned(),
                    dimension: "nonconformance".to_owned(),
                    message: "latest calibration decision is not conforming".to_owned(),
                });
            }
            "due_soon" => warnings.push(ReadinessIssueDto {
                asset_id: instrument.asset_id.clone(),
                code: "calibration_due_soon".to_owned(),
                dimension: "calibration_validity".to_owned(),
                message: "calibration due date is near".to_owned(),
            }),
            "missing" | "expired" => warnings.push(ReadinessIssueDto {
                asset_id: instrument.asset_id.clone(),
                code: format!("calibration_{}", status.calibration_status),
                dimension: calibration_issue_dimension(&status.calibration_status).to_owned(),
                message: "calibration status requires attention".to_owned(),
            }),
            _ => {}
        }

        instrument_results.push(ReadinessInstrumentResultDto {
            asset_id: instrument.asset_id.clone(),
            manufacturer: Some(instrument.manufacturer.clone()),
            model: Some(instrument.model.clone()),
            serial_number: Some(instrument.serial_number.clone()),
            serviceability_status: Some(instrument.serviceability_status.clone()),
            calibration_requirement: Some(instrument.calibration_requirement.clone()),
            calibration_status: status.calibration_status,
            due_at: status.due_at,
            reasons,
            blocking,
            instrument_revision: Some(instrument.revision),
            calibration_revision: status.latest_calibration_revision,
        });
    }

    Ok(ReadinessReportDto {
        ready: blocking_issues.is_empty(),
        checked_on: format_metrology_date(checked_on),
        execution_mode: input.execution_mode,
        context: input.context,
        instrument_results,
        blocking_issues,
        warnings,
    })
}

pub fn list_metrology_audit_events(
    storage_root: &Path,
    entity_type: &str,
    entity_id: &str,
) -> Result<String, AgentError> {
    let connection = open_metrology_connection(storage_root)?;
    let events = load_metrology_audit_events(&connection, entity_type, entity_id)?;
    Ok(render_json(&MetrologyAuditEventsDto {
        entity_type: entity_type.to_owned(),
        entity_id: entity_id.to_owned(),
        audit_events: events
            .into_iter()
            .map(|event| MetrologyAuditEventDto {
                sequence: event.sequence,
                actor: event.actor,
                action: event.action,
                reason: event.reason,
                operation_id: event.operation_id,
                correlation_id: event.correlation_id,
                device_id: event.device_id,
                base_revision: event.base_revision,
                resulting_revision: event.resulting_revision,
                payload_json: event.payload_json,
                occurred_at: event.occurred_at,
            })
            .collect(),
    }))
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

fn validate_characterization_decision(value: &str) -> Result<(), AgentError> {
    match value.trim() {
        "conforming" | "nonconforming" | "indeterminate" | "not_assessed" => Ok(()),
        other => Err(AgentError::new(
            "invalid_asset_characterization",
            format!("unknown characterization decision: {other}"),
        )),
    }
}

fn validate_characterization_source_kind(value: &str) -> Result<(), AgentError> {
    match value.trim() {
        "calibration"
        | "characterization"
        | "verification"
        | "manufacturer_certificate"
        | "internal_measurement" => Ok(()),
        other => Err(AgentError::new(
            "invalid_asset_characterization",
            format!("unknown characterization source kind: {other}"),
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

fn validate_execution_mode(value: &str) -> Result<(), AgentError> {
    match value.trim() {
        "accredited" | "non_accredited" | "investigation" => Ok(()),
        other => Err(AgentError::new(
            "invalid_metrology_readiness",
            format!("unknown execution mode: {other}"),
        )),
    }
}

fn calibration_issue_dimension(status: &str) -> &'static str {
    match status {
        "missing" => "missing_evidence",
        "nonconforming" => "nonconformance",
        _ => "calibration_validity",
    }
}

pub(crate) fn validate_operation_context(
    context: &MetrologyOperationContext,
) -> Result<(), AgentError> {
    safe_identifier(&context.operation_id, "operation_id")?;
    safe_identifier(&context.correlation_id, "correlation_id")?;
    safe_identifier(&context.device_id, "device_id")?;
    require_non_empty(&context.actor, "actor")?;
    require_non_empty(&context.reason, "reason")?;
    Ok(())
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

fn validate_equipment_model_reference(input: &RegisterInstrumentInput) -> Result<(), AgentError> {
    let values = [
        trimmed_optional(input.equipment_model_id.as_deref()),
        trimmed_optional(input.equipment_model_revision_id.as_deref()),
        trimmed_optional(input.equipment_model_checksum.as_deref()),
    ];
    let present = values.iter().filter(|value| value.is_some()).count();
    if present != 0 && present != values.len() {
        return Err(AgentError::new(
            "invalid_metrology_instrument",
            "equipment model id, revision id and checksum must be provided together",
        ));
    }
    if let Some(checksum) = values[2] {
        let digest = checksum.strip_prefix("sha256:").unwrap_or_default();
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(AgentError::new(
                "invalid_metrology_instrument",
                "equipment_model_checksum must use canonical sha256:<64 lowercase hex> form",
            ));
        }
    }
    Ok(())
}

fn register_instrument_payload_json(input: &RegisterInstrumentInput, warning_days: u32) -> String {
    render_json(&serde_json::json!({
        "asset_id": input.asset_id.trim(),
        "family": input.family.trim(),
        "category_code": trimmed_optional(input.category_code.as_deref()),
        "equipment_model_id": trimmed_optional(input.equipment_model_id.as_deref()),
        "equipment_model_revision_id": trimmed_optional(input.equipment_model_revision_id.as_deref()),
        "equipment_model_checksum": trimmed_optional(input.equipment_model_checksum.as_deref()),
        "manufacturer": input.manufacturer.trim(),
        "model": input.model.trim(),
        "serial_number": input.serial_number.trim(),
        "part_number": trimmed_optional(input.part_number.as_deref()),
        "calibration_requirement": input.calibration_requirement.trim(),
        "calibration_period_months": input.calibration_period_months,
        "calibration_due_warning_days": warning_days,
        "serviceability_status": input.serviceability_status.trim(),
        "serviceability_reason": input.serviceability_reason.trim(),
        "capabilities_json": input.capabilities_json.trim(),
        "metrology_notes": input.metrology_notes.trim(),
    }))
}

fn calibration_event_payload_json(input: &RecordCalibrationInput) -> String {
    render_json(&serde_json::json!({
        "event_id": input.event_id.trim(),
        "asset_id": input.asset_id.trim(),
        "certificate_reference": input.certificate_reference.trim(),
        "calibrated_at": input.calibrated_at.trim(),
        "due_at": input.due_at.trim(),
        "provider": input.provider.trim(),
        "decision": input.decision.trim(),
        "as_found_status": trimmed_optional(input.as_found_status.as_deref()),
        "as_left_status": trimmed_optional(input.as_left_status.as_deref()),
        "adjustment_performed": input.adjustment_performed,
        "uncertainty_summary_json": input.uncertainty_summary_json.trim(),
        "traceability_reference": trimmed_optional(input.traceability_reference.as_deref()),
        "comment": input.comment.trim(),
        "document_manifest_json": trimmed_optional(input.document_manifest_json.as_deref()),
        "recorded_by": input.recorded_by.trim(),
    }))
}

fn asset_characterization_payload_json(
    input: &RecordAssetCharacterizationInput,
    canonical: &emc_locus_core::CanonicalAssetCharacterizationDefinition,
) -> String {
    let definition = serde_json::from_str::<Value>(&canonical.canonical_json)
        .expect("canonical asset characterization must be valid JSON");
    let document_manifest =
        trimmed_optional(input.document_manifest_json.as_deref()).map(|value| {
            serde_json::from_str::<Value>(value)
                .expect("validated asset characterization manifest must be valid JSON")
        });
    render_json(&serde_json::json!({
        "characterization_id": canonical.characterization_id,
        "asset_id": canonical.asset_id,
        "characterization_kind": canonical.kind.as_str(),
        "label": canonical.label,
        "performed_on": input.performed_on.trim(),
        "valid_from": input.valid_from.trim(),
        "valid_until": input.valid_until.trim(),
        "source_kind": input.source_kind.trim(),
        "provider": input.provider.trim(),
        "method_reference": input.method_reference.trim(),
        "decision": input.decision.trim(),
        "definition": definition,
        "definition_checksum": canonical.definition_checksum,
        "certificate_reference": trimmed_optional(input.certificate_reference.as_deref()),
        "document_manifest": document_manifest,
        "comment": input.comment.trim(),
        "environmental_conditions": serde_json::from_str::<Value>(input.environmental_conditions_json.trim())
            .expect("validated characterization environment must be JSON"),
        "as_found": trimmed_optional(input.as_found_json.as_deref())
            .map(|value| serde_json::from_str::<Value>(value).expect("validated as-found data must be JSON")),
        "as_left": trimmed_optional(input.as_left_json.as_deref())
            .map(|value| serde_json::from_str::<Value>(value).expect("validated as-left data must be JSON")),
        "adjustment_performed": input.adjustment_performed,
        "recorded_by": input.recorded_by.trim(),
    }))
}

fn render_asset_characterization(
    record: &StoredAssetCharacterization,
) -> Result<String, AgentError> {
    Ok(render_json(&AssetCharacterizationEnvelopeDto {
        characterization: asset_characterization_dto_from_stored(record)?,
    }))
}

fn asset_characterization_dto_from_stored(
    record: &StoredAssetCharacterization,
) -> Result<AssetCharacterizationDto, AgentError> {
    let definition = AssetCharacterizationDefinition::from_json_str(&record.definition_json)
        .map_err(|issue| characterization_storage_invalid(issue.message))?;
    let canonical = definition.canonicalize().map_err(|issues| {
        characterization_storage_invalid(format!(
            "stored characterization definition violates {} invariant(s)",
            issues.len()
        ))
    })?;
    if canonical.characterization_id != record.characterization_id
        || canonical.asset_id != record.asset_id
        || canonical.kind.as_str() != record.characterization_kind
        || canonical.label != record.label
        || canonical.definition_schema_version != record.definition_schema_version
        || canonical.canonical_json != record.definition_json
        || canonical.definition_checksum != record.definition_checksum
    {
        return Err(characterization_storage_invalid(
            "stored characterization columns do not match the canonical definition",
        ));
    }
    let performed_on = parse_metrology_date(&record.performed_on, "performed_on")
        .map_err(|error| characterization_storage_invalid(error.message))?;
    let valid_from = parse_metrology_date(&record.valid_from, "valid_from")
        .map_err(|error| characterization_storage_invalid(error.message))?;
    let valid_until = parse_metrology_date(&record.valid_until, "valid_until")
        .map_err(|error| characterization_storage_invalid(error.message))?;
    if valid_until < valid_from || valid_from < performed_on {
        return Err(characterization_storage_invalid(
            "stored characterization validity precedes its measurement date",
        ));
    }
    if !matches!(
        record.decision.as_str(),
        "conforming" | "nonconforming" | "indeterminate" | "not_assessed"
    ) || !matches!(
        record.source_kind.as_str(),
        "calibration"
            | "characterization"
            | "verification"
            | "manufacturer_certificate"
            | "internal_measurement"
    ) || [
        record.provider.as_str(),
        record.method_reference.as_str(),
        record.recorded_by.as_str(),
        record.revision.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty())
    {
        return Err(characterization_storage_invalid(
            "stored characterization traceability fields are invalid",
        ));
    }
    validate_characterization_document_manifest(record.document_manifest_json.as_deref())
        .map_err(|error| characterization_storage_invalid(error.message))?;
    let definition = serde_json::from_str::<Value>(&canonical.canonical_json)
        .expect("canonical asset characterization must be valid JSON");
    let document_manifest = record
        .document_manifest_json
        .as_deref()
        .map(|value| {
            serde_json::from_str::<Value>(value).map_err(|error| {
                AgentError::new(
                    "metrology_characterization_storage_invalid",
                    format!("stored characterization document manifest is invalid: {error}"),
                )
            })
        })
        .transpose()?;
    let environmental_conditions = parse_stored_characterization_object(
        &record.environmental_conditions_json,
        "environmental conditions",
    )?;
    let as_found = record
        .as_found_json
        .as_deref()
        .map(|value| parse_stored_characterization_object(value, "as-found data"))
        .transpose()?;
    let as_left = record
        .as_left_json
        .as_deref()
        .map(|value| parse_stored_characterization_object(value, "as-left data"))
        .transpose()?;
    Ok(asset_characterization_dto(
        record,
        definition,
        document_manifest,
        environmental_conditions,
        as_found,
        as_left,
    ))
}

fn parse_stored_characterization_object(value: &str, label: &str) -> Result<Value, AgentError> {
    let parsed = serde_json::from_str::<Value>(value).map_err(|error| {
        characterization_storage_invalid(format!("stored {label} is invalid: {error}"))
    })?;
    if !parsed.is_object() {
        return Err(characterization_storage_invalid(format!(
            "stored {label} must be a JSON object"
        )));
    }
    Ok(parsed)
}

fn characterization_storage_invalid(message: impl Into<String>) -> AgentError {
    AgentError::new("metrology_characterization_storage_invalid", message)
}

fn invalid_asset_characterization(
    issues: Vec<emc_locus_core::DefinitionValidationIssue>,
) -> AgentError {
    AgentError::with_details(
        "invalid_asset_characterization",
        "asset characterization definition is invalid",
        serde_json::json!({ "issues": issues }),
    )
}

fn serviceability_payload_json(input: &SetServiceabilityInput) -> String {
    render_json(&serde_json::json!({
        "asset_id": input.asset_id.trim(),
        "serviceability_status": input.serviceability_status.trim(),
        "serviceability_reason": input.serviceability_reason.trim(),
    }))
}

pub(crate) fn write_metrology_audit_and_outbox(
    transaction: &rusqlite::Transaction<'_>,
    input: MetrologyAuditWrite<'_>,
) -> Result<(), AgentError> {
    insert_metrology_audit_event(
        transaction,
        MetrologyAuditEventInput {
            entity_type: input.entity_type,
            entity_id: input.entity_id,
            sequence: input.sequence,
            actor: input.context.actor.trim(),
            action: input.action,
            reason: input.context.reason.trim(),
            operation_id: input.context.operation_id.trim(),
            correlation_id: input.context.correlation_id.trim(),
            device_id: input.context.device_id.trim(),
            base_revision: input.base_revision,
            resulting_revision: input.resulting_revision,
            payload_json: input.payload_json,
            timestamp: input.timestamp,
        },
    )?;
    insert_metrology_sync_operation(
        transaction,
        MetrologySyncOperationInput {
            operation_id: input.context.operation_id.trim(),
            entity_type: input.entity_type,
            entity_id: input.entity_id,
            operation_kind: input.action,
            base_revision: input.base_revision,
            resulting_revision: input.resulting_revision,
            actor_id: input.context.actor.trim(),
            device_id: input.context.device_id.trim(),
            correlation_id: input.context.correlation_id.trim(),
            payload_json: input.payload_json,
            timestamp: input.timestamp,
        },
    )
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

fn validate_characterization_json_object(
    value: &str,
    field: &'static str,
) -> Result<Value, AgentError> {
    let parsed = serde_json::from_str::<Value>(value)
        .map_err(|error| AgentError::new("invalid_asset_characterization", error.to_string()))?;
    if !parsed.is_object() {
        return Err(AgentError::new(
            "invalid_asset_characterization",
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

fn validate_characterization_document_manifest(value: Option<&str>) -> Result<(), AgentError> {
    let Some(value) = trimmed_optional(value) else {
        return Ok(());
    };
    let parsed = serde_json::from_str::<Value>(value)
        .map_err(|error| AgentError::new("invalid_asset_characterization", error.to_string()))?;
    let object = parsed.as_object().ok_or_else(|| {
        AgentError::new(
            "invalid_asset_characterization",
            "document_manifest must be a JSON object",
        )
    })?;
    let required_text = |field: &str| -> Result<&str, AgentError> {
        object
            .get(field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AgentError::new(
                    "invalid_asset_characterization",
                    format!("document_manifest.{field} is required"),
                )
            })
    };
    let object_id = required_text("object_id")?;
    let filename = required_text("original_filename")?;
    let mime_type = required_text("mime_type")?;
    let checksum = required_text("sha256")?;
    let storage_key = required_text("storage_key")?;
    if filename.contains(['/', '\\', '\0']) || filename == "." || filename == ".." {
        return Err(AgentError::new(
            "invalid_asset_characterization",
            "document_manifest.original_filename must be a safe file name",
        ));
    }
    if !mime_type.contains('/') || mime_type.chars().any(char::is_whitespace) {
        return Err(AgentError::new(
            "invalid_asset_characterization",
            "document_manifest.mime_type must be a valid media type",
        ));
    }
    if object
        .get("size_bytes")
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .is_none()
    {
        return Err(AgentError::new(
            "invalid_asset_characterization",
            "document_manifest.size_bytes must be a positive integer",
        ));
    }
    if validate_sha256(checksum).is_err() {
        return Err(AgentError::new(
            "invalid_asset_characterization",
            "document_manifest.sha256 must be a 64-character lowercase hexadecimal SHA-256",
        ));
    }
    if object_id != format!("sha256:{checksum}") {
        return Err(AgentError::new(
            "invalid_asset_characterization",
            "document_manifest.object_id must match document_manifest.sha256",
        ));
    }
    if !storage_key.starts_with("objects/")
        || storage_key.contains("..")
        || storage_key.contains('\\')
        || !storage_key.ends_with(checksum)
    {
        return Err(AgentError::new(
            "invalid_asset_characterization",
            "document_manifest.storage_key must be a content-addressed local object key",
        ));
    }
    Ok(())
}

fn validate_sha256(value: &str) -> Result<(), AgentError> {
    if value.len() == 64
        && value
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, 'a'..='f'))
    {
        return Ok(());
    }
    Err(AgentError::new(
        "invalid_metrology_calibration",
        "document_manifest_json.sha256 must be a 64-character lowercase hexadecimal SHA-256",
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
        checked_on: format_metrology_date(checked_on),
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

fn format_metrology_date(date: MetrologyDate) -> String {
    format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
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

pub(crate) fn revision_for(entity_type: &str, entity_id: &str, updated_at: &str) -> String {
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

pub(crate) fn utc_timestamp() -> Result<String, AgentError> {
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
                    category_code: Some("spectrum_analyzer".to_owned()),
                    equipment_model_id: None,
                    equipment_model_revision_id: None,
                    equipment_model_checksum: None,
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
                    context: test_context("op-register-SA-REG-001"),
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
                    context: test_context("op-record-CAL-SA-001-2026"),
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
                context: test_context("op-record-CAL-SA-001-NC"),
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

    #[test]
    fn rejects_characterization_storage_with_mismatched_checksum() {
        let definition_json = serde_json::json!({
            "definition_schema_version": "emc-locus.asset-characterization-definition.v1",
            "characterization_id": "CHAR-STORAGE-001",
            "asset_id": "SA-001",
            "label": "Measured cable loss",
            "correction": {
                "correction_kind": "frequency_response",
                "correction": {
                    "definition_schema_version": "emc-locus.engineering-curve-definition.v1",
                    "curve_id": "CHAR-STORAGE-001",
                    "curve_type": "cable_loss",
                    "label": "Measured cable loss",
                    "signal_representation": "frequency_domain_spectrum",
                    "independent_axes": [{"axis":"frequency","quantity":"frequency","unit":"Hz"}],
                    "dependent_values": [{
                        "value_id":"amplitude",
                        "quantity":"dimensionless",
                        "unit":"dB",
                        "component":"amplitude",
                        "operation":"add"
                    }],
                    "points": [
                        {"axis_values":{"frequency":1000000.0},"values":{"amplitude":0.2}},
                        {"axis_values":{"frequency":1000000000.0},"values":{"amplitude":3.0}}
                    ],
                    "interpolation":"log_x_linear_y",
                    "extrapolation_policy":"forbidden"
                }
            }
        })
        .to_string();
        let canonical = AssetCharacterizationDefinition::from_json_str(&definition_json)
            .unwrap()
            .canonicalize()
            .unwrap();
        let mut stored = StoredAssetCharacterization {
            characterization_id: canonical.characterization_id.clone(),
            asset_id: canonical.asset_id.clone(),
            characterization_kind: canonical.kind.as_str().to_owned(),
            label: canonical.label.clone(),
            performed_on: "2026-07-01".to_owned(),
            valid_from: "2026-07-01".to_owned(),
            valid_until: "2027-07-01".to_owned(),
            source_kind: "characterization".to_owned(),
            provider: "Internal laboratory".to_owned(),
            method_reference: "MET-RF-CABLE-001".to_owned(),
            decision: "conforming".to_owned(),
            definition_schema_version: canonical.definition_schema_version.clone(),
            definition_json: canonical.canonical_json,
            definition_checksum: canonical.definition_checksum,
            certificate_reference: None,
            document_manifest_json: None,
            comment: String::new(),
            recorded_at: "2026-07-14T00:00:00Z".to_owned(),
            recorded_by: "metrology.admin".to_owned(),
            revision: "rev-0001".to_owned(),
            environmental_conditions_json: "{}".to_owned(),
            as_found_json: None,
            as_left_json: None,
            adjustment_performed: false,
        };
        assert!(asset_characterization_dto_from_stored(&stored).is_ok());

        stored.definition_checksum = format!("sha256:{}", "f".repeat(64));
        let error = asset_characterization_dto_from_stored(&stored).unwrap_err();

        assert_eq!(error.code, "metrology_characterization_storage_invalid");
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
        apply_migrations(
            &storage_root.join("metrology.sqlite"),
            &repo_root().join("storage/sqlite/metrology"),
        );
        apply_migrations(
            &storage_root.join("sync.sqlite"),
            &repo_root().join("storage/sqlite/sync"),
        );
        storage_root
    }

    fn apply_migrations(database: &Path, migrations_root: &Path) {
        let connection = Connection::open(database).unwrap();
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

    fn test_context(operation_id: &str) -> MetrologyOperationContext {
        MetrologyOperationContext {
            actor: "metrology.admin".to_owned(),
            reason: "test operation".to_owned(),
            operation_id: operation_id.to_owned(),
            correlation_id: operation_id.to_owned(),
            device_id: "station-test".to_owned(),
        }
    }
}
