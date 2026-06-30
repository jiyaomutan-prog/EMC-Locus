use crate::metrology_repository::{StoredCalibrationRecord, StoredInstrument};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstrumentDto {
    pub asset_id: String,
    pub family: String,
    pub category_code: Option<String>,
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
    pub capabilities_json: String,
    pub metrology_notes: String,
    pub created_at: String,
    pub updated_at: String,
    pub revision: String,
    pub latest_calibration: Option<CalibrationRecordDto>,
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

pub(crate) fn instrument_dto(
    instrument: &StoredInstrument,
    latest_calibration: Option<&StoredCalibrationRecord>,
) -> InstrumentDto {
    InstrumentDto {
        asset_id: instrument.asset_id.clone(),
        family: instrument.family.clone(),
        category_code: instrument.category_code.clone(),
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
        capabilities_json: instrument.capabilities_json.clone(),
        metrology_notes: instrument.metrology_notes.clone(),
        created_at: instrument.created_at.clone(),
        updated_at: instrument.updated_at.clone(),
        revision: instrument.revision.clone(),
        latest_calibration: latest_calibration.map(calibration_record_dto),
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
