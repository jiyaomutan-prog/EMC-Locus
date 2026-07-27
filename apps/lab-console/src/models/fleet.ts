export type OwnershipSource =
  | "laboratory_owned"
  | "customer_supplied"
  | "rented"
  | "borrowed"
  | "external"
  | "software_license"
  | "installed_facility";

export type ServiceState =
  | "usable"
  | "restricted"
  | "in_maintenance"
  | "out_of_service"
  | "retired";

export type AdministrativeAvailability = "available" | "unavailable";

export type OperationalUsageState =
  | "available"
  | "reserved"
  | "assigned_to_setup"
  | "in_test"
  | "unavailable";

export interface PhysicalAssetMetrologySummary {
  calibration_requirement: string;
  calibration_period_months: number | null;
  calibration_due_warning_days: number;
  latest_due_at: string | null;
  latest_decision: string | null;
}

export interface PhysicalAsset {
  asset_id: string;
  inventory_code: string;
  serial_number: string | null;
  part_number: string | null;
  equipment_model_id: string | null;
  equipment_model_revision_id: string | null;
  equipment_model_checksum: string | null;
  manufacturer: string;
  model_name: string;
  variant: string | null;
  category_code: string;
  category_path: string[];
  laboratory_location_id: string | null;
  laboratory_location_label: string | null;
  laboratory_location_status: "active" | "archived" | null;
  ownership_source: OwnershipSource;
  service_state: ServiceState;
  administrative_availability: AdministrativeAvailability;
  administrative_unavailability_reason: string;
  operational_usage: OperationalUsageSummary;
  availability_state: OperationalUsageState;
  service_state_reason: string;
  notes: string;
  revision: number;
  model_link_state: string;
  migrated_from_metrology: boolean;
  metrology: PhysicalAssetMetrologySummary | null;
  created_at: string;
  updated_at: string;
}

export interface OperationalUsageEvidence {
  source_kind: string;
  source_identifier: string;
  source_label: string;
  relevant_start_at: string | null;
  relevant_end_at: string | null;
  reason: string;
  blocks_selection: boolean;
}

export interface OperationalUsageSummary {
  state: OperationalUsageState;
  assessed_at: string;
  evidence: OperationalUsageEvidence[];
}

export interface LaboratoryLocation {
  location_id: string;
  label: string;
  description: string;
  status: "active" | "archived";
  revision: number;
  created_at: string;
  updated_at: string;
}

export interface ModelReconciliationCandidate {
  equipment_model_id: string;
  equipment_model_revision_id: string;
  revision_number: number;
  lifecycle_status: "approved" | "superseded";
  approved_at: string | null;
  manufacturer: string;
  model_name: string;
  variant: string | null;
  category_path: string[];
}

export interface CreatePhysicalAssetInput {
  inventory_code: string;
  serial_number?: string;
  part_number?: string;
  equipment_model_id: string;
  laboratory_location_id?: string;
  ownership_source: OwnershipSource;
  service_state: ServiceState;
  administrative_availability: AdministrativeAvailability;
  administrative_unavailability_reason?: string;
  service_state_reason?: string;
  notes?: string;
  calibration_requirement: string;
  calibration_period_months?: number;
  calibration_due_warning_days?: number;
  metrology_notes?: string;
}

export interface FleetOperationResult<T> {
  replayed: boolean;
  asset?: T;
  location?: T;
}
