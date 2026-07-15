use emc_locus_core::{
    PlannedTestPreparationDefinition, PreparedStationSetupSnapshot, PreparedTestMethodSnapshot,
    StationSetupReadiness,
};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct PlannedTestPreparationRevisionDto {
    pub(crate) revision_id: String,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<String>,
    pub(crate) recorded_state: String,
    pub(crate) effective_state: String,
    pub(crate) is_current: bool,
    pub(crate) definition: PlannedTestPreparationDefinition,
    pub(crate) definition_checksum: String,
    pub(crate) actor: String,
    pub(crate) reason: String,
    pub(crate) operation_id: String,
    pub(crate) device_id: String,
    pub(crate) correlation_id: String,
    pub(crate) created_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct PlannedTestPreparationAggregateDto {
    pub(crate) project_code: String,
    pub(crate) schedule_item_code: String,
    pub(crate) current_state: String,
    pub(crate) can_start: bool,
    pub(crate) current_revision: Option<PlannedTestPreparationRevisionDto>,
    pub(crate) revision_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct PlannedTestPreparationEnvelopeDto {
    pub(crate) preparation: PlannedTestPreparationAggregateDto,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct PlannedTestPreparationRevisionEnvelopeDto {
    pub(crate) revision: PlannedTestPreparationRevisionDto,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct PlannedTestPreparationRevisionListDto {
    pub(crate) project_code: String,
    pub(crate) schedule_item_code: String,
    pub(crate) revisions: Vec<PlannedTestPreparationRevisionDto>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct PlannedTestPreparationOperationResultDto {
    pub(crate) operation: String,
    pub(crate) operation_id: String,
    pub(crate) replayed: bool,
    pub(crate) preparation: PlannedTestPreparationAggregateDto,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct PlannedTestPreparationStationOptionDto {
    pub(crate) station_setup: PreparedStationSetupSnapshot,
    pub(crate) readiness: StationSetupReadiness,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct PlannedTestPreparationOptionsDto {
    pub(crate) project_code: String,
    pub(crate) schedule_item_code: String,
    pub(crate) methods: Vec<PreparedTestMethodSnapshot>,
    pub(crate) station_setups: Vec<PlannedTestPreparationStationOptionDto>,
}
