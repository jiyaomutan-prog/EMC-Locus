export type ProjectExecutionMode = "accredited" | "non_accredited" | "investigation";

export type ProjectStage =
  | "quotation"
  | "contract_review"
  | "test_planning"
  | "measuring"
  | "technical_review"
  | "report_issued"
  | "archived";

export interface ProjectRecord {
  code: string;
  customer_name: string;
  stage: ProjectStage;
  execution_mode: ProjectExecutionMode;
  created_at: string;
  archived_at: string | null;
  revision: string;
}

export interface CompletedContractReviewItem {
  item: string;
  completed_by: string | null;
  completed_at: string | null;
  comment: string | null;
}

export interface ContractReviewStatus {
  project_code: string;
  execution_mode: ProjectExecutionMode;
  required_items: string[];
  completed_items: CompletedContractReviewItem[];
  missing_items: string[];
  complete: boolean;
}

export type ServiceScheduleStatus =
  | "planned"
  | "confirmed"
  | "in_progress"
  | "completed"
  | "cancelled";

export interface ServiceScheduleItem {
  item_code: string;
  project_code: string;
  title: string;
  test_category_code: string | null;
  test_method_code: string | null;
  planned_start_at: string;
  planned_end_at: string;
  assigned_operator: string;
  laboratory_location_id: string | null;
  laboratory_location_label: string;
  equipment_under_test: string;
  status: ServiceScheduleStatus;
  notes: string;
  revision: number;
  created_by: string;
  updated_by: string;
  created_at: string;
  updated_at: string;
  available_transitions: ServiceScheduleStatus[];
  can_reschedule: boolean;
}

export interface LaboratoryScheduleItem extends ServiceScheduleItem {
  customer_name: string;
  project_stage: ProjectStage;
}

export interface LaboratoryWeekSchedule {
  week_start: string;
  week_end: string;
  schedule_items: LaboratoryScheduleItem[];
}

export interface LaboratoryLocationOption {
  laboratory_location_id: string;
  laboratory_location_label: string;
}

export interface StationSetupLocationSource {
  current_ready_revision: {
    definition: {
      laboratory_location_id?: string | null;
      laboratory_location_label?: string;
    };
  } | null;
}

export interface ProjectAuditEvent {
  sequence: number;
  actor: string;
  action: string;
  reason: string | null;
  payload_json: string;
  occurred_at: string;
}

export interface ProjectOperationResult {
  operation: string;
  operation_id: string;
  replayed: boolean;
  project: ProjectRecord;
}

export interface ContractReviewOperationResult {
  operation: string;
  operation_id: string;
  replayed: boolean;
  already_completed: boolean;
  resulting_revision: string;
  contract_review: ContractReviewStatus;
}

export interface ServiceScheduleOperationResult {
  operation: string;
  operation_id: string;
  replayed: boolean;
  schedule_item: ServiceScheduleItem;
}

export type PlannedTestPreparationState =
  | "missing"
  | "blocked"
  | "ready"
  | "stale"
  | "inapplicable";

export type PlannedTestPreparationDimension =
  | "schedule_context"
  | "test_method"
  | "station_setup"
  | "instrument_assignment"
  | "serviceability"
  | "calibration_validity"
  | "missing_evidence"
  | "nonconformance"
  | "correction_validity";

export interface PlannedTestPreparationIssue {
  code: string;
  severity: "blocking" | "warning";
  dimension: PlannedTestPreparationDimension;
  message: string;
  next_action: string;
  method_slot_ids?: string[];
  binding_ids?: string[];
  asset_ids?: string[];
}

export interface PlannedTestMethodSlot {
  slot_id: string;
  label: string;
  required_category?: string;
  required_capability?: string;
  required: boolean;
  calibration_requirement: "required" | "not_required" | "if_used";
  substitution_policy:
    | "no_substitution"
    | "same_category"
    | "same_capability"
    | "approved_equivalent";
  depends_on_slots?: string[];
}

export interface PlannedTestMethodSnapshot {
  template_id: string;
  revision_id: string;
  revision_number: number;
  revision_status: "approved" | "superseded";
  definition_checksum: string;
  title: string;
  measurement_axis: string;
  method_code?: string;
  method_revision?: string;
  standard_references?: string[];
  instrumentation_chain: PlannedTestMethodSlot[];
}

export interface PlannedStationAssetSnapshot {
  binding_id: string;
  role_label: string;
  asset_id: string;
  asset_revision: string;
  inventory_code: string;
  serial_number: string;
  manufacturer: string;
  model_name: string;
  equipment_model_id: string;
  equipment_model_revision_id: string;
  equipment_model_checksum: string;
  category_code: string;
  capabilities?: Array<{
    capability_id: string;
    label: string;
    capability_kind: string;
  }>;
}

export interface PlannedStationSetupSnapshot {
  setup_id: string;
  revision_id: string;
  revision_number: number;
  revision_status: "ready" | "superseded";
  definition_checksum: string;
  label: string;
  laboratory_location_id: string | null;
  laboratory_location_label: string;
  planned_use_on: string;
  execution_mode: ProjectExecutionMode;
  assets: PlannedStationAssetSnapshot[];
  corrections?: Array<{
    selection_id: string;
    binding_id: string;
    correction_kind: string;
    characterization_id: string;
    characterization_checksum: string;
    label: string;
  }>;
}

export interface PlannedTestScheduleSnapshot {
  project_code: string;
  item_code: string;
  revision: number;
  title: string;
  planned_start_at: string;
  planned_end_at: string;
  assigned_operator: string;
  laboratory_location_id: string | null;
  laboratory_location_label: string;
  equipment_under_test: string;
  execution_mode: ProjectExecutionMode;
  status: ServiceScheduleStatus;
}

export interface PlannedTestPreparationDefinition {
  definition_schema_version: string;
  schedule: PlannedTestScheduleSnapshot;
  method: PlannedTestMethodSnapshot;
  station_setup: PlannedStationSetupSnapshot;
  assignments: Array<{ slot_id: string; binding_id: string }>;
  verdict: {
    ready: boolean;
    checked_on: string;
    issues: PlannedTestPreparationIssue[];
  };
}

export interface PlannedTestPreparationRevision {
  revision_id: string;
  revision_number: number;
  parent_revision_id: string | null;
  recorded_state: "blocked" | "ready";
  effective_state: "blocked" | "ready" | "stale" | "historical" | "inapplicable";
  is_current: boolean;
  definition: PlannedTestPreparationDefinition;
  definition_checksum: string;
  actor: string;
  reason: string;
  operation_id: string;
  device_id: string;
  correlation_id: string;
  created_at: string;
}

export interface PlannedTestPreparationAggregate {
  project_code: string;
  schedule_item_code: string;
  current_state: PlannedTestPreparationState;
  can_start: boolean;
  current_revision: PlannedTestPreparationRevision | null;
  revision_count: number;
}

export interface PlannedTestPreparationOptions {
  project_code: string;
  schedule_item_code: string;
  methods: PlannedTestMethodSnapshot[];
  station_setups: Array<{
    station_setup: PlannedStationSetupSnapshot;
    readiness: {
      ready: boolean;
      checked_on: string;
      issues: Array<{
        code: string;
        severity: "blocking" | "warning";
        dimension: string;
        message: string;
        binding_ids?: string[];
        connection_ids?: string[];
      }>;
    };
  }>;
}

export interface PlannedTestPreparationOperationResult {
  operation: string;
  operation_id: string;
  replayed: boolean;
  preparation: PlannedTestPreparationAggregate;
}
