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
  location: string;
  equipment_under_test: string;
  status: ServiceScheduleStatus;
  notes: string;
  revision: number;
  created_by: string;
  updated_by: string;
  created_at: string;
  updated_at: string;
  available_transitions: ServiceScheduleStatus[];
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
