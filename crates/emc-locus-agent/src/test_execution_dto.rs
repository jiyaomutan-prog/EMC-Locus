use crate::metrology_dto::ReadinessReportDto;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SimulatedTestExecutionEnvelopeDto {
    pub(crate) execution: SimulatedTestExecutionDto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SimulatedTestExecutionListDto {
    pub(crate) executions: Vec<SimulatedTestExecutionSummaryDto>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SimulatedTestExecutionResultDto {
    pub(crate) operation: String,
    pub(crate) operation_id: String,
    pub(crate) replayed: bool,
    pub(crate) execution: SimulatedTestExecutionDto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SimulatedTestExecutionDto {
    pub(crate) attempt_id: String,
    pub(crate) project_code: String,
    pub(crate) test_type: String,
    pub(crate) test_method_reference: String,
    pub(crate) execution_mode: String,
    pub(crate) operator: String,
    pub(crate) checked_on: String,
    pub(crate) status: String,
    pub(crate) readiness: ReadinessReportDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) refusal: Option<ExecutionRefusalDto>,
    pub(crate) instrumentation_snapshot: Vec<ExecutionInstrumentSnapshotDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) simulation_result: Option<SimulatedEmcResultDto>,
    pub(crate) software_version: String,
    pub(crate) started_at: String,
    pub(crate) completed_at: String,
    pub(crate) revision: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SimulatedTestExecutionSummaryDto {
    pub(crate) attempt_id: String,
    pub(crate) project_code: String,
    pub(crate) test_method_reference: String,
    pub(crate) execution_mode: String,
    pub(crate) operator: String,
    pub(crate) status: String,
    pub(crate) ready: bool,
    pub(crate) checked_on: String,
    pub(crate) completed_at: String,
    pub(crate) revision: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ExecutionRefusalDto {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) causes: Vec<ExecutionRefusalCauseDto>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ExecutionRefusalCauseDto {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) asset_id: String,
    pub(crate) dimension: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ExecutionInstrumentSnapshotDto {
    pub(crate) asset_id: String,
    pub(crate) role: String,
    pub(crate) serviceability_status: Option<String>,
    pub(crate) calibration_requirement: Option<String>,
    pub(crate) calibration_status: String,
    pub(crate) due_at: Option<String>,
    pub(crate) blocking: bool,
    pub(crate) reasons: Vec<String>,
    pub(crate) instrument_revision: Option<String>,
    pub(crate) calibration_revision: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SimulatedEmcResultDto {
    pub(crate) strategy: String,
    pub(crate) verdict: String,
    pub(crate) measurement_axis: String,
    pub(crate) start_frequency_hz: u64,
    pub(crate) stop_frequency_hz: u64,
    pub(crate) points: u32,
    pub(crate) peak_frequency_hz: u64,
    pub(crate) peak_level_dbuv: f64,
    pub(crate) limit_dbuv: f64,
    pub(crate) margin_db: f64,
}
