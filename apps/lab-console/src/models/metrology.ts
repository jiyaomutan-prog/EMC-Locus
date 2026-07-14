import type { EquipmentFileReference, SignalTransformationKind } from "./equipment";

export interface CalibrationRecordSummary {
  calibrated_at: string;
  due_at: string;
  certificate_reference: string;
  revision: string;
}

export interface MetrologyInstrument {
  asset_id: string;
  family: string;
  category_code: string | null;
  equipment_model_id: string | null;
  equipment_model_revision_id: string | null;
  equipment_model_checksum: string | null;
  manufacturer: string;
  model: string;
  serial_number: string;
  part_number: string | null;
  serviceability_status: "usable" | "restricted" | "out_of_service" | "retired";
  serviceability_reason: string;
  calibration_requirement: "required" | "conditional" | "not_required";
  calibration_period_months: number | null;
  calibration_due_warning_days: number;
  metrology_notes: string;
  created_at: string;
  updated_at: string;
  revision: string;
  latest_calibration: CalibrationRecordSummary | null;
  latest_calibration_event: CalibrationRecordSummary | null;
}

export interface RegisterMetrologyInstrumentInput {
  asset_id: string;
  family: string;
  category_code?: string;
  equipment_model_id?: string;
  equipment_model_revision_id?: string;
  equipment_model_checksum?: string;
  manufacturer: string;
  model: string;
  serial_number: string;
  part_number?: string;
  calibration_requirement: MetrologyInstrument["calibration_requirement"];
  calibration_period_months?: number;
  calibration_due_warning_days?: number;
  serviceability_status: MetrologyInstrument["serviceability_status"];
  serviceability_reason?: string;
  capabilities: unknown;
  metrology_notes?: string;
  actor: string;
  reason: string;
}

export interface CharacterizationUncertainty {
  expanded_uncertainty?: number;
  unit?: string;
  coverage_factor?: number;
  confidence_level_percent?: number;
  statement?: string;
}

export interface AssetCharacterizationDefinition {
  definition_schema_version: "emc-locus.asset-characterization-definition.v1";
  characterization_id: string;
  asset_id: string;
  label: string;
  correction:
    | {
        correction_kind: "time_conversion";
        correction: Record<string, unknown>;
      }
    | {
        correction_kind: "frequency_response";
        correction: Record<string, unknown>;
      };
  model_correction_reference?: {
    transformation_kind: SignalTransformationKind;
    entity_id: string;
    revision_id: string;
    definition_checksum: string;
  };
  uncertainty?: CharacterizationUncertainty;
  conditions?: Record<string, unknown>;
}

export interface AssetCharacterization {
  characterization_id: string;
  asset_id: string;
  characterization_kind: "time_conversion" | "frequency_response";
  label: string;
  performed_on: string;
  valid_until: string;
  provider: string;
  method_reference: string;
  decision: "conforming" | "nonconforming" | "indeterminate" | "not_assessed";
  definition_schema_version: string;
  definition: AssetCharacterizationDefinition;
  definition_checksum: string;
  certificate_reference: string | null;
  document_manifest: EquipmentFileReference | null;
  comment: string;
  recorded_at: string;
  recorded_by: string;
  revision: string;
}

export interface RecordAssetCharacterizationInput {
  characterization_id: string;
  performed_on: string;
  valid_until: string;
  provider: string;
  method_reference: string;
  decision: AssetCharacterization["decision"];
  definition: AssetCharacterizationDefinition;
  certificate_reference?: string;
  document_manifest?: EquipmentFileReference;
  comment?: string;
  recorded_by: string;
  actor: string;
  reason: string;
}

export interface MetrologyAuditEvent {
  sequence: number;
  actor: string;
  action: string;
  reason: string;
  operation_id: string;
  correlation_id: string;
  device_id: string;
  base_revision: string;
  resulting_revision: string;
  payload_json: string;
  occurred_at: string;
}
