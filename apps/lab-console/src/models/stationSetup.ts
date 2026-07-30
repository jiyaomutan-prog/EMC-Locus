export type StationSetupRevisionStatus = "draft" | "ready" | "superseded";

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
  notes: Record<string, unknown>;
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
}

export interface StationSetupAggregate {
  identity: {
    setup_id: string;
    label: string;
    current_ready_revision_id: string | null;
    created_by: string;
    created_at: string;
    updated_at: string;
  };
  active_draft_revision: StationSetupRevision | null;
  current_ready_revision: StationSetupRevision | null;
  latest_revision: StationSetupRevision;
}

export interface StationSetupOperationResult {
  operation: string;
  operation_id: string;
  replayed: boolean;
  station_setup: StationSetupAggregate;
}
