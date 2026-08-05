export type MethodHierarchyNodeKind =
  | "domain"
  | "test_family"
  | "source_document"
  | "applicable_edition"
  | "procedure"
  | "method_variant"
  | "parameter_profile"
  | "sub_range_profile";

export interface MethodHierarchyNode {
  node_id: string;
  parent_node_id?: string;
  node_kind: MethodHierarchyNodeKind;
  label: string;
  position: number;
  archived: boolean;
  revision: number;
}

export interface QuantityValue {
  value: number;
  unit: string;
}

export type DeterministicExpression =
  | { kind: "literal"; value: number; unit: string }
  | { kind: "variable"; variable_id: string }
  | { kind: "add" | "minimum" | "maximum"; operands: DeterministicExpression[] }
  | { kind: "subtract"; left: DeterministicExpression; right: DeterministicExpression }
  | { kind: "multiply"; left: DeterministicExpression; right: DeterministicExpression }
  | { kind: "divide"; numerator: DeterministicExpression; denominator: DeterministicExpression };

export interface MethodVariableV2 {
  variable_id: string;
  label: string;
  description: string;
  semantic: string;
  value_type: string;
  dimension: string;
  unit?: string;
  default_value?: number | string | boolean;
  minimum?: number;
  maximum?: number;
  enum_values?: string[];
  required: boolean;
  source: string;
  availability_phase: string;
  expression?: DeterministicExpression;
  consumers?: string[];
}

export interface LogicalPort {
  port_id: string;
  label: string;
  directionality: string;
  signal_domain: string;
  connector?: string;
  impedance_ohm?: number;
  quantity_dimension?: string;
}

export interface FunctionalRole {
  role_id: string;
  label: string;
  purpose: string;
  required: boolean;
  functional_category: string;
  capabilities: Array<Record<string, unknown>>;
  calibration_policy: string;
  substitution_policy: string;
  assignment_stage: string;
  logical_ports: LogicalPort[];
  consumes_variables: string[];
  produces_variables: string[];
}

export interface TopologyNode {
  node_id: string;
  label: string;
  role_type: string;
  method_role_id?: string;
  ports: LogicalPort[];
  notes: string;
}

export interface TopologyEdge {
  edge_id: string;
  label: string;
  edge_kind: string;
  from: { node_id: string; port_id: string };
  to: { node_id: string; port_id: string };
}

export interface MeasurementSystemDefinition {
  definition_schema_version: "emc-locus.measurement-system-template-definition.v1";
  template_id: string;
  label: string;
  classification: string;
  nodes: TopologyNode[];
  edges: TopologyEdge[];
  correction_points: Array<{
    correction_point_id: string;
    label: string;
    node_id: string;
    signal_variable_id: string;
    correction_kind: string;
  }>;
  regulation_loops: Array<{
    loop_id: string;
    label: string;
    regulation_profile_id: string;
    actuator_node_id: string;
    feedback_node_id: string;
    monitoring_node_ids: string[];
  }>;
  notes: string;
}

export interface RegulationProfileDefinition {
  definition_schema_version: "emc-locus.regulation-profile-definition.v1";
  profile_id: string;
  label: string;
  regulated_quantity: string;
  regulated_unit: string;
  target_expression: DeterministicExpression;
  tolerance_band: QuantityValue;
  control_mode: string;
  actuator_role_id?: string;
  required_driver_action?: string;
  feedback_role_id?: string;
  feedback_signal_variable_id?: string;
  monitoring_role_ids: string[];
  calibration_reference_profile?: string;
  start_threshold: QuantityValue;
  fast_increasing_step: QuantityValue;
  slow_increasing_step: QuantityValue;
  decreasing_step: QuantityValue;
  dwell_t1_seconds: number;
  dwell_t2_seconds: number;
  dwell_t3_seconds: number;
  regulation_start_criterion: string;
  regulation_end_criterion: string;
  regulation_factor: number;
  maximum_output?: QuantityValue;
  maximum_forward_power?: QuantityValue;
  maximum_reflected_power?: QuantityValue;
  overshoot_limit?: QuantityValue;
  retry_policy: { maximum_attempts: number; on_exhaustion: string };
  abort_policy: string;
  safe_state_policy: string;
  operator_notes: string;
}

export interface SubRangeDefinition {
  sub_range_id: string;
  label: string;
  start_frequency: QuantityValue;
  stop_frequency: QuantityValue;
  include_start: boolean;
  include_stop: boolean;
  progression:
    | { kind: "fixed_step"; step: QuantityValue }
    | { kind: "percentage"; percentage: number }
    | { kind: "points_per_decade"; points: number }
    | { kind: "explicit_list"; points: QuantityValue[] };
  direction: "increasing" | "decreasing";
  sweep_mode: string;
  dwell_seconds: number;
  special_dwell_seconds?: number;
  include_frequencies: QuantityValue[];
  exclude_frequencies: QuantityValue[];
  system_template_profile_id?: string;
  regulation_profile_id?: string;
  modulation_profile_id?: string;
  eut_state?: string;
  orientation?: string;
  polarization?: string;
  comments: string;
}

export interface ProcedureNode {
  node_id: string;
  label: string;
  purpose: string;
  node_kind: string;
  input_variables: string[];
  output_variables: string[];
  timeout_seconds?: number;
  failure_policy: string;
  maximum_iterations?: number;
  children?: ProcedureNode[];
  audit_notes: string;
}

export interface TestMethodDefinitionV2 {
  definition_schema_version: "emc-locus.test-method-definition.v2";
  title: string;
  objective: string;
  scope: string;
  classification_path: string[];
  standard_references: string[];
  variables: MethodVariableV2[];
  lock_policy: Array<Record<string, unknown>>;
  parameter_profiles: Array<Record<string, unknown>>;
  functional_roles: FunctionalRole[];
  measurement_system_templates: RevisionReference[];
  regulation_profiles: RevisionReference[];
  modulation_profiles: Array<Record<string, unknown>>;
  sub_ranges: SubRangeDefinition[];
  procedure: ProcedureNode[];
  limits: Array<Record<string, unknown>>;
  post_processing: Array<Record<string, unknown>>;
  expected_output_variables: string[];
  migration_evidence: string[];
}

export interface RevisionReference {
  identity_id: string;
  revision_id: string;
  definition_checksum: string;
}

export interface WorkflowRevision<T> {
  revision_id: string;
  entity_id: string;
  revision_number: number;
  parent_revision_id: string | null;
  status: "draft" | "validated" | "approved" | "superseded" | "archived";
  definition_schema_version: string;
  definition: T;
  definition_checksum: string;
  created_by: string;
  created_at: string;
  updated_at: string;
  validated_at: string | null;
  approved_at: string | null;
}

export interface WorkflowAggregate<T> {
  identity: {
    aggregate_kind: string;
    entity_id: string;
    label: string;
    classification: string;
    current_approved_revision_id: string | null;
    created_by: string;
    created_at: string;
    updated_at: string;
  };
  revisions: WorkflowRevision<T>[];
}

export interface MethodWorkflowRevision {
  revision_id: string;
  template_id: string;
  revision_number: number;
  parent_revision_id: string | null;
  status: string;
  definition_schema_version: string;
  definition: unknown;
  definition_checksum: string;
  created_by: string;
  created_at: string;
  updated_at: string;
  submitted_at: string | null;
  approved_at: string | null;
}

export interface MethodWorkflowAggregate {
  identity: {
    template_id: string;
    title: string;
    category_code: string;
    current_approved_revision_id: string | null;
    created_by: string;
    created_at: string;
    updated_at: string;
  };
  current_approved_revision: MethodWorkflowRevision | null;
  latest_revision: MethodWorkflowRevision | null;
  active_draft_revision: MethodWorkflowRevision | null;
}

export interface MethodWorkflowOperationResult {
  operation: string;
  operation_id: string;
  replayed: boolean;
  test_template: MethodWorkflowAggregate;
  revision: MethodWorkflowRevision;
}

export interface SubRangePreview {
  point_count: number;
  truncated: boolean;
  preview_points_hz: number[];
}

export interface ExecutionPlanNotice {
  code: string;
  message: string;
  next_action: string;
}

export interface ExecutionPlanPreview {
  method_title: string;
  system_template_label: string;
  ordered_phases: Array<{
    node_id: string;
    label: string;
    node_kind: string;
    depth: number;
    maximum_iterations?: number;
  }>;
  required_roles: string[];
  unresolved_roles: string[];
  regulation_loops: string[];
  expected_signals: string[];
  post_processing_requests: string[];
  blockers: ExecutionPlanNotice[];
  warnings: ExecutionPlanNotice[];
  unsupported_runtime_operations: string[];
}

export function isMethodV2(definition: unknown): definition is TestMethodDefinitionV2 {
  return Boolean(
    definition &&
      typeof definition === "object" &&
      (definition as { definition_schema_version?: string }).definition_schema_version ===
        "emc-locus.test-method-definition.v2"
  );
}
