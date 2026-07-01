export type RevisionStatus =
  | "draft"
  | "under_review"
  | "approved"
  | "superseded"
  | "suspended"
  | "retired";

export type MeasurementAxis =
  | "frequency_sweep"
  | "time_series"
  | "event_triggered"
  | "mixed_time_frequency";

export type VariableValueType = "number" | "integer" | "boolean" | "text" | "enum";

export interface VariableConstraints {
  required: boolean;
  dimensionless?: boolean;
  unit?: string;
  minimum?: number;
  maximum?: number;
  enum_values?: string[];
}

export interface VariableDefinition {
  variable_id: string;
  label: string;
  value_type: VariableValueType;
  default_value?: number | string | boolean;
  constraints: VariableConstraints;
  description?: string;
}

export interface VariableLockPolicy {
  variable_id: string;
  policy:
    | "editable_until_campaign_freeze"
    | "editable_until_execution"
    | "admin_only"
    | "investigation_only"
    | "immutable";
}

export interface InstrumentationChainSlot {
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

export interface BranchRule {
  rule_id: string;
  condition: string;
  destination_step_id: string;
  allow_cycle?: boolean;
}

export interface ExecutionSequenceStep {
  step_id: string;
  order: number;
  kind:
    | "prepare"
    | "configure_instrument"
    | "acquire"
    | "operator_decision"
    | "post_process"
    | "verify"
    | "finish";
  label: string;
  instruction?: string;
  required_slots?: string[];
  branches?: BranchRule[];
}

export interface LimitDefinition {
  limit_id: string;
  kind: "time_limit" | "frequency_limit" | "scalar_threshold";
  axis: MeasurementAxis;
  unit: string;
  application_domain: string;
  source_reference: string;
  threshold?: number;
  attention_rule?: string;
  variable_refs?: string[];
}

export interface PostProcessingDefinition {
  operation_id: string;
  order: number;
  operation_type:
    | "correction"
    | "fft"
    | "windowing"
    | "resampling"
    | "harmonic_calculation"
    | "event_counting"
    | "channel_math"
    | "peak"
    | "custom";
  inputs: string[];
  outputs: string[];
  parameters: Record<string, unknown>;
}

export interface TestTemplateDefinition {
  definition_schema_version: string;
  title: string;
  description: string;
  measurement_axis: MeasurementAxis;
  method_code?: string;
  method_revision?: string;
  standard_references: string[];
  variables: VariableDefinition[];
  lock_policy: VariableLockPolicy[];
  instrumentation_chain: InstrumentationChainSlot[];
  entry_step_id: string;
  sequence: ExecutionSequenceStep[];
  limits: LimitDefinition[];
  post_processing: PostProcessingDefinition[];
  method_parameters: Record<string, unknown>;
}

export interface TestTemplateIdentity {
  template_id: string;
  title: string;
  category_code: string;
  current_approved_revision_id: string | null;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface TestTemplateRevision {
  revision_id: string;
  template_id: string;
  revision_number: number;
  parent_revision_id: string | null;
  status: RevisionStatus;
  definition_schema_version: string;
  definition: TestTemplateDefinition;
  definition_checksum: string;
  created_by: string;
  created_at: string;
  updated_at: string;
  submitted_at: string | null;
  approved_at: string | null;
}

export interface TestTemplateAggregate {
  identity: TestTemplateIdentity;
  current_approved_revision: TestTemplateRevision | null;
  latest_revision: TestTemplateRevision | null;
  active_draft_revision: TestTemplateRevision | null;
}

export interface AuditEvent {
  audit_id: number;
  template_id: string;
  revision_id: string | null;
  action: string;
  actor: string;
  reason: string;
  old_revision_id: string | null;
  new_revision_id: string | null;
  old_definition_checksum: string | null;
  new_definition_checksum: string | null;
  operation_id: string;
  correlation_id: string;
  device_id: string;
  payload_json: string;
  occurred_at: string;
}

export interface ValidationIssue {
  severity: "error" | "warning" | "info";
  code: string;
  path: string;
  message: string;
}

export interface ValidationResult {
  valid: boolean;
  issues: ValidationIssue[];
  definition_schema_version?: string;
  definition_checksum?: string;
  canonical_json?: string;
}

export interface HealthReport {
  agent: string;
  version: string;
  storage_root: string;
  storage_root_exists: boolean;
  domains: string[];
}

export interface StorageStatus {
  action: string;
  storage_root: string;
  migrations_root: string;
  domains: Array<{
    domain: string;
    database_path: string;
    exists: boolean;
    schema_version: number | null;
    latest_migration: number;
    status: string;
    integrity_check?: string | null;
    journal_mode?: string | null;
    atomicity_compatible?: boolean | null;
  }>;
}

export interface ApiErrorBody {
  error: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
  };
}

export type SaveState =
  | "clean"
  | "dirty"
  | "saving"
  | "saved"
  | "conflict"
  | "error";
