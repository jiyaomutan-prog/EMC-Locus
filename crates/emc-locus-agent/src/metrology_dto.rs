use crate::metrology_repository::{
    StoredCalibrationEvent, StoredCalibrationRecord, StoredInstrument,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstrumentDto {
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
    pub availability: String,
    pub legacy_availability: Option<String>,
    pub serviceability_status: String,
    pub serviceability_reason: String,
    pub serviceability_updated_at: Option<String>,
    pub calibration_requirement: String,
    pub calibration_period_months: Option<u32>,
    pub calibration_due_warning_days: u32,
    pub capabilities_json: String,
    pub metrology_notes: String,
    pub created_at: String,
    pub updated_at: String,
    pub revision: String,
    pub latest_calibration: Option<CalibrationRecordDto>,
    pub latest_calibration_event: Option<CalibrationEventDto>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstrumentEnvelopeDto {
    pub instrument: InstrumentDto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstrumentListDto {
    pub instruments: Vec<InstrumentDto>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CalibrationEventListDto {
    pub asset_id: String,
    pub calibration_events: Vec<CalibrationEventDto>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CalibrationEventEnvelopeDto {
    pub calibration_event: CalibrationEventDto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CalibrationRecordDto {
    pub id: i64,
    pub asset_id: String,
    pub certificate_reference: String,
    pub calibrated_at: String,
    pub due_at: String,
    pub provider: String,
    pub status_at_import: String,
    pub uncertainty_json: String,
    pub file_reference: Option<String>,
    pub checksum: Option<String>,
    pub created_at: String,
    pub revision: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CalibrationEventDto {
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
    pub recorded_at: String,
    pub recorded_by: String,
    pub revision: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CalibrationStatusDto {
    pub asset_id: String,
    pub checked_on: String,
    pub calibration_status: String,
    pub serviceability_status: String,
    pub calibration_requirement: String,
    pub calibration_due_warning_days: u32,
    pub due_at: Option<String>,
    pub decision: Option<String>,
    pub latest_calibration_event_id: Option<String>,
    pub latest_calibration_revision: Option<String>,
    pub instrument_revision: String,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReadinessReportDto {
    pub ready: bool,
    pub checked_on: String,
    pub execution_mode: String,
    pub context: Option<String>,
    pub instrument_results: Vec<ReadinessInstrumentResultDto>,
    pub blocking_issues: Vec<ReadinessIssueDto>,
    pub warnings: Vec<ReadinessIssueDto>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReadinessInstrumentResultDto {
    pub asset_id: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub serviceability_status: Option<String>,
    pub calibration_requirement: Option<String>,
    pub calibration_status: String,
    pub due_at: Option<String>,
    pub reasons: Vec<String>,
    pub blocking: bool,
    pub instrument_revision: Option<String>,
    pub calibration_revision: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReadinessIssueDto {
    pub asset_id: String,
    pub code: String,
    pub dimension: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MetrologyAuditEventsDto {
    pub entity_type: String,
    pub entity_id: String,
    pub audit_events: Vec<MetrologyAuditEventDto>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MetrologyAuditEventDto {
    pub sequence: u64,
    pub actor: String,
    pub action: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
    pub base_revision: String,
    pub resulting_revision: String,
    pub payload_json: String,
    pub occurred_at: String,
}

pub(crate) fn instrument_dto(
    instrument: &StoredInstrument,
    latest_calibration: Option<&StoredCalibrationRecord>,
    latest_calibration_event: Option<&StoredCalibrationEvent>,
) -> InstrumentDto {
    InstrumentDto {
        asset_id: instrument.asset_id.clone(),
        family: instrument.family.clone(),
        category_code: instrument.category_code.clone(),
        equipment_model_id: instrument.equipment_model_id.clone(),
        equipment_model_revision_id: instrument.equipment_model_revision_id.clone(),
        equipment_model_checksum: instrument.equipment_model_checksum.clone(),
        manufacturer: instrument.manufacturer.clone(),
        model: instrument.model.clone(),
        serial_number: instrument.serial_number.clone(),
        part_number: instrument.part_number.clone(),
        availability: instrument.availability.clone(),
        legacy_availability: instrument.legacy_availability.clone(),
        serviceability_status: instrument.serviceability_status.clone(),
        serviceability_reason: instrument.serviceability_reason.clone(),
        serviceability_updated_at: instrument.serviceability_updated_at.clone(),
        calibration_requirement: instrument.calibration_requirement.clone(),
        calibration_period_months: instrument.calibration_period_months,
        calibration_due_warning_days: instrument.calibration_due_warning_days,
        capabilities_json: instrument.capabilities_json.clone(),
        metrology_notes: instrument.metrology_notes.clone(),
        created_at: instrument.created_at.clone(),
        updated_at: instrument.updated_at.clone(),
        revision: instrument.revision.clone(),
        latest_calibration: latest_calibration.map(calibration_record_dto),
        latest_calibration_event: latest_calibration_event.map(calibration_event_dto),
    }
}

fn calibration_record_dto(record: &StoredCalibrationRecord) -> CalibrationRecordDto {
    CalibrationRecordDto {
        id: record.id,
        asset_id: record.asset_id.clone(),
        certificate_reference: record.certificate_reference.clone(),
        calibrated_at: record.calibrated_at.clone(),
        due_at: record.due_at.clone(),
        provider: record.provider.clone(),
        status_at_import: record.status_at_import.clone(),
        uncertainty_json: record.uncertainty_json.clone(),
        file_reference: record.file_reference.clone(),
        checksum: record.checksum.clone(),
        created_at: record.created_at.clone(),
        revision: format!("calibration-{:04}", record.id),
    }
}

pub(crate) fn calibration_event_dto(record: &StoredCalibrationEvent) -> CalibrationEventDto {
    CalibrationEventDto {
        event_id: record.event_id.clone(),
        asset_id: record.asset_id.clone(),
        certificate_reference: record.certificate_reference.clone(),
        calibrated_at: record.calibrated_at.clone(),
        due_at: record.due_at.clone(),
        provider: record.provider.clone(),
        decision: record.decision.clone(),
        as_found_status: record.as_found_status.clone(),
        as_left_status: record.as_left_status.clone(),
        adjustment_performed: record.adjustment_performed,
        uncertainty_summary_json: record.uncertainty_summary_json.clone(),
        traceability_reference: record.traceability_reference.clone(),
        comment: record.comment.clone(),
        document_manifest_json: record.document_manifest_json.clone(),
        recorded_at: record.recorded_at.clone(),
        recorded_by: record.recorded_by.clone(),
        revision: record.revision.clone(),
    }
}
