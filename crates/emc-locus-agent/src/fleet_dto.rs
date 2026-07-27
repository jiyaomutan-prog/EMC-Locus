use crate::metrology_assessment::MetrologyStatusSummaryDto;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct PhysicalAssetDto {
    pub(crate) asset_id: String,
    pub(crate) inventory_code: String,
    pub(crate) serial_number: Option<String>,
    pub(crate) part_number: Option<String>,
    pub(crate) equipment_model_id: Option<String>,
    pub(crate) equipment_model_revision_id: Option<String>,
    pub(crate) equipment_model_checksum: Option<String>,
    pub(crate) manufacturer: String,
    pub(crate) model_name: String,
    pub(crate) variant: Option<String>,
    pub(crate) category_code: String,
    pub(crate) category_path: Vec<String>,
    pub(crate) laboratory_location_id: Option<String>,
    pub(crate) laboratory_location_label: Option<String>,
    pub(crate) laboratory_location_status: Option<String>,
    pub(crate) ownership_source: String,
    pub(crate) service_state: String,
    pub(crate) administrative_availability: String,
    pub(crate) administrative_unavailability_reason: String,
    pub(crate) operational_usage: OperationalUsageSummaryDto,
    pub(crate) availability_state: String,
    pub(crate) service_state_reason: String,
    pub(crate) notes: String,
    pub(crate) revision: u64,
    pub(crate) model_link_state: String,
    pub(crate) migrated_from_metrology: bool,
    pub(crate) metrology: MetrologyStatusSummaryDto,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct OperationalUsageEvidenceDto {
    pub(crate) source_kind: String,
    pub(crate) source_identifier: String,
    pub(crate) source_label: String,
    pub(crate) relevant_start_at: Option<String>,
    pub(crate) relevant_end_at: Option<String>,
    pub(crate) reason: String,
    pub(crate) blocks_selection: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct OperationalUsageSummaryDto {
    pub(crate) state: String,
    pub(crate) assessed_at: String,
    pub(crate) evidence: Vec<OperationalUsageEvidenceDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct AssetSelectionReasonDto {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ExecutablePhysicalAssetOptionDto {
    pub(crate) asset: PhysicalAssetDto,
    pub(crate) eligible: bool,
    pub(crate) blocking_reasons: Vec<AssetSelectionReasonDto>,
    pub(crate) warnings: Vec<AssetSelectionReasonDto>,
}

#[derive(Serialize)]
pub(crate) struct ExecutablePhysicalAssetOptionListDto {
    pub(crate) assessed_at: String,
    pub(crate) checked_on: String,
    pub(crate) execution_mode: String,
    pub(crate) laboratory_location_id: Option<String>,
    pub(crate) assets: Vec<ExecutablePhysicalAssetOptionDto>,
}

#[derive(Serialize)]
pub(crate) struct PhysicalAssetEnvelopeDto {
    pub(crate) asset: PhysicalAssetDto,
    pub(crate) replayed: bool,
}

#[derive(Serialize)]
pub(crate) struct PhysicalAssetListDto {
    pub(crate) assets: Vec<PhysicalAssetDto>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ModelReconciliationCandidateDto {
    pub(crate) equipment_model_id: String,
    pub(crate) equipment_model_revision_id: String,
    pub(crate) revision_number: u32,
    pub(crate) lifecycle_status: String,
    pub(crate) approved_at: Option<String>,
    pub(crate) manufacturer: String,
    pub(crate) model_name: String,
    pub(crate) variant: Option<String>,
    pub(crate) category_path: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct ModelReconciliationCandidateListDto {
    pub(crate) candidates: Vec<ModelReconciliationCandidateDto>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct LaboratoryLocationDto {
    pub(crate) location_id: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) status: String,
    pub(crate) revision: u64,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize)]
pub(crate) struct LaboratoryLocationEnvelopeDto {
    pub(crate) location: LaboratoryLocationDto,
    pub(crate) replayed: bool,
}

#[derive(Serialize)]
pub(crate) struct LaboratoryLocationListDto {
    pub(crate) locations: Vec<LaboratoryLocationDto>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct FleetAuditEventDto {
    pub(crate) sequence: u64,
    pub(crate) action: String,
    pub(crate) actor: String,
    pub(crate) reason: String,
    pub(crate) old_revision: Option<u64>,
    pub(crate) new_revision: u64,
    pub(crate) operation_id: String,
    pub(crate) device_id: String,
    pub(crate) correlation_id: String,
    pub(crate) payload: serde_json::Value,
    pub(crate) occurred_at: String,
}

#[derive(Serialize)]
pub(crate) struct FleetAuditEventListDto {
    pub(crate) entity_id: String,
    pub(crate) audit_events: Vec<FleetAuditEventDto>,
}
