import type { PhysicalAsset, AssetSelectionReason } from "./fleet";

export type StationSetupRevisionStatus = "draft" | "qualified" | "ready" | "superseded";
export type StationMaterialSelectionPolicy = "category_pool" | "capability_match" | "exact_asset";
export type StationMaterialAssignmentStage = "setup_definition" | "planned_test_preparation";
export type StationMaterialSubstitutionPolicy =
  | "no_substitution"
  | "same_exact_model"
  | "same_category"
  | "same_capabilities"
  | "approved_equivalent";

export interface StationRangeConstraint {
  minimum?: number;
  maximum?: number;
  unit: string;
}

export interface StationLogicalPortRequirement {
  logical_port_id: string;
  label: string;
  directionality: "input" | "output" | "bidirectional" | "through" | "control" | "communication";
  signal_domain: string;
  connector_requirement?: string;
  impedance_ohm?: number;
  frequency_range?: StationRangeConstraint;
  voltage_range?: StationRangeConstraint;
  current_range?: StationRangeConstraint;
  power_range?: StationRangeConstraint;
}

export interface StationMaterialRequirement {
  requirement_id: string;
  role_label: string;
  description?: string;
  required: boolean;
  selection_policy: StationMaterialSelectionPolicy;
  assignment_stage: StationMaterialAssignmentStage;
  substitution_policy: StationMaterialSubstitutionPolicy;
  calibration_requirement: "required" | "if_used" | "not_required";
  category_requirement?: { category_id: string; accept_descendants: boolean };
  capability_requirement?: {
    capability_kind: string;
    model_capability_id?: string;
    model_capability_equipment_model_id?: string;
    frequency_range?: StationRangeConstraint;
    voltage_range?: StationRangeConstraint;
    current_range?: StationRangeConstraint;
    power_range?: StationRangeConstraint;
    detector_modes?: string[];
    signal_domain?: string;
    port_directionality?: StationLogicalPortRequirement["directionality"];
    connector_requirement?: string;
    impedance_ohm?: number;
    communication_capability?: string;
    automated_control_required?: boolean;
    required_driver_action?: string;
  };
  exact_asset_id?: string;
  logical_ports: StationLogicalPortRequirement[];
}

export interface StationMaterialAssignment {
  requirement_id: string;
  asset_id: string;
  asset_revision: string;
  inventory_code: string;
  serial_number?: string;
  equipment_model_id: string;
  equipment_model_revision_id: string;
  equipment_model_checksum: string;
  selected_ports: Array<{ logical_port_id: string; actual_port_id: string }>;
  assignment_context: string;
  assigned_on: string;
}

export interface StationAssetBindingDefinition {
  binding_id: string;
  role_label: string;
  asset_id: string;
  asset_revision: string;
  equipment_model_id: string;
  equipment_model_revision_id: string;
  equipment_model_checksum: string;
}

export interface StationMeasurementSetupDefinition {
  definition_schema_version: string;
  setup_id: string;
  label: string;
  laboratory_location_id: string | null;
  laboratory_location_label: string;
  planned_use_on: string;
  execution_mode: "accredited" | "non_accredited" | "investigation";
  asset_bindings: StationAssetBindingDefinition[];
  connections: Array<{
    connection_id: string;
    label: string;
    from: { binding_id: string; port_id: string };
    to: { binding_id: string; port_id: string };
  }>;
  correction_selections: Array<{
    selection_id: string;
    binding_id: string;
    correction_kind: "time_conversion" | "frequency_response";
    characterization_id: string;
    characterization_checksum: string;
    label: string;
  }>;
  material_requirements?: StationMaterialRequirement[];
  material_assignments?: StationMaterialAssignment[];
  logical_connections?: Array<{
    connection_id: string;
    label: string;
    from: { requirement_id: string; logical_port_id: string };
    to: { requirement_id: string; logical_port_id: string };
  }>;
  notes: Record<string, unknown>;
}

export interface StationCompatibilityReason {
  code: string;
  dimension: string;
  message: string;
  next_action?: string;
}

export interface StationMaterialCandidate {
  asset: PhysicalAsset;
  requirement_compatible: boolean;
  compatibility_state: "compatible" | "incompatible" | "indeterminate";
  operationally_eligible: boolean;
  assignable: boolean;
  exact_asset_required: boolean;
  category_evidence: string[];
  capability_evidence: string[];
  technical_constraint_results: StationCompatibilityReason[];
  driver_evidence: string[];
  logical_port_resolution_candidates: Record<string, string[]>;
  compatibility_blockers: StationCompatibilityReason[];
  operational_blockers: AssetSelectionReason[];
  warnings: AssetSelectionReason[];
  next_actions: string[];
}

export interface StationMaterialCandidates {
  setup_id: string;
  revision_id: string;
  requirement_id: string;
  request_context_key: string;
  planned_use_on: string;
  execution_mode: string;
  laboratory_location_id: string;
  candidates: StationMaterialCandidate[];
}

export interface StationSetupReadinessIssue {
  code: string;
  severity: "blocking" | "warning";
  dimension: string;
  message: string;
  binding_ids?: string[];
  connection_ids?: string[];
}

export interface StationSetupReadiness {
  ready: boolean;
  checked_on: string;
  issues: StationSetupReadinessIssue[];
}

export interface StationSetupRevision {
  revision_id: string;
  setup_id: string;
  revision_number: number;
  parent_revision_id: string | null;
  status: StationSetupRevisionStatus;
  definition_schema_version: string;
  definition: StationMeasurementSetupDefinition;
  definition_checksum: string;
  readiness: StationSetupReadiness;
  created_by: string;
  created_at: string;
  updated_at: string;
  ready_at: string | null;
  qualified_at?: string | null;
}

export interface StationSetupAggregate {
  identity: {
    setup_id: string;
    label: string;
    current_ready_revision_id: string | null;
    current_qualified_revision_id?: string | null;
    created_by: string;
    created_at: string;
    updated_at: string;
  };
  active_draft_revision: StationSetupRevision | null;
  current_qualified_revision?: StationSetupRevision | null;
  current_ready_revision: StationSetupRevision | null;
  latest_revision: StationSetupRevision;
}

export interface StationSetupOperationResult {
  operation: string;
  operation_id: string;
  replayed: boolean;
  station_setup: StationSetupAggregate;
}
