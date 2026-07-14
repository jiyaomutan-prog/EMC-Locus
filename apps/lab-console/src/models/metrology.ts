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
