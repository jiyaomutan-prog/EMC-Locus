import type { RevisionStatus, ValidationResult } from "../types";

export type EquipmentClass =
  | "controllable_instrument"
  | "daq_device"
  | "acquisition_device"
  | "converter"
  | "sensor"
  | "transducer"
  | "passive_component"
  | "switching_device"
  | "motion_system"
  | "facility"
  | "software_adapter"
  | "manual_equipment";

export type FunctionalRole =
  | "energy_source"
  | "signal_source"
  | "rf_network_element"
  | "sensor"
  | "actuator"
  | "measurement_instrument"
  | "acquisition_device"
  | "converter"
  | "control_system"
  | "software_system"
  | "facility"
  | "manual_accessory";

export type SignalDomain =
  | "power_dc"
  | "power_ac"
  | "rf"
  | "analog_voltage"
  | "analog_current"
  | "analog_charge"
  | "digital_logic"
  | "trigger"
  | "pulse"
  | "contact_dry"
  | "relay"
  | "can_bus"
  | "rs232"
  | "rs485"
  | "ethernet"
  | "usb"
  | "gpib"
  | "optical"
  | "mechanical"
  | "environmental"
  | "software";

export type PortDirectionality =
  | "input"
  | "output"
  | "bidirectional"
  | "through"
  | "control"
  | "communication";

export type PortFlowRole =
  | "source_port"
  | "sink_port"
  | "through_port"
  | "measurement_port"
  | "control_port"
  | "communication_port"
  | "field_side_port"
  | "transducer_output_port";

export type TechnologyTag =
  | "adc_converter"
  | "dac_converter"
  | "rf_50_ohm"
  | "rf_75_ohm"
  | "ttl"
  | "cmos"
  | "trigger"
  | "dry_contact"
  | "relay_contact"
  | "voltage_input"
  | "current_input"
  | "charge_input"
  | "iepe"
  | "bridge"
  | "usb"
  | "ethernet"
  | "gpib"
  | "rs232"
  | "rs485"
  | "can_bus"
  | "visa"
  | "raw_tcp"
  | "serial_text"
  | "scpi";

export type PhysicalQuantity =
  | "frequency"
  | "time"
  | "voltage"
  | "current"
  | "power"
  | "electric_field"
  | "magnetic_field"
  | "impedance"
  | "resistance"
  | "capacitance"
  | "inductance"
  | "temperature"
  | "distance"
  | "angle"
  | "velocity"
  | "acceleration"
  | "pressure"
  | "dimensionless"
  | "text"
  | "boolean"
  | "binary";

export type AccessProviderKind =
  | "native_serial"
  | "native_tcp"
  | "native_udp"
  | "visa"
  | "socketcan"
  | "pcan"
  | "vector_can"
  | "usbtmc"
  | "hid"
  | "custom_adapter"
  | "simulation";

export type TransportKind =
  | "none"
  | "serial"
  | "gpib"
  | "ethernet_tcp"
  | "ethernet_udp"
  | "can_bus"
  | "usb"
  | "rs485"
  | "lin"
  | "modbus"
  | "bluetooth"
  | "vendor_bus";

export type ProtocolKind =
  | "scpi"
  | "raw_ascii"
  | "raw_binary"
  | "can_bus_frames"
  | "modbus_rtu"
  | "modbus_tcp"
  | "custom_protocol"
  | "manual";

export type SafetyClass =
  | "read_only"
  | "configuration_change"
  | "energizes_output"
  | "deenergizes_output"
  | "moves_mechanism"
  | "changes_routing"
  | "requires_interlock"
  | "potentially_destructive";

export interface EngineeringSpecification {
  specification_id: string;
  label: string;
  quantity: PhysicalQuantity;
  unit: string;
  minimum?: number;
  maximum?: number;
  nominal?: number;
  resolution?: number;
  conditions?: string[];
  comment?: string;
}

export interface SignalPortDefinition {
  port_id: string;
  label: string;
  directionality: PortDirectionality;
  flow_role: PortFlowRole;
  signal_domain: SignalDomain;
  connector_type?: string;
  quantity: PhysicalQuantity;
  unit: string;
  impedance?: number;
  frequency_min?: number;
  frequency_max?: number;
  voltage_max?: number;
  current_max?: number;
  power_max?: number;
  channel_index?: number;
  differential?: boolean;
  isolated?: boolean;
  comment?: string;
}

export interface CommunicationInterfaceDefinition {
  interface_id: string;
  label: string;
  transport_kind: TransportKind;
  access_provider_kind: AccessProviderKind;
  protocol_kind: ProtocolKind;
  required: boolean;
  default_interface: boolean;
  configuration_schema?: Record<string, unknown>;
  default_configuration?: Record<string, unknown>;
  framing?: string;
  identification_strategy?: Record<string, unknown>;
  firmware_compatibility?: string[];
}

export interface ActionValueDefinition {
  name: string;
  value_type: "number" | "integer" | "boolean" | "text" | "bytes" | "list" | "object" | "frame";
  quantity: PhysicalQuantity;
  unit: string;
  required: boolean;
  default_value?: unknown;
  minimum?: number;
  maximum?: number;
  enum_values?: string[];
  description?: string;
}

export interface MeasurementCapabilityDefinition {
  capability_id: string;
  label: string;
  description: string;
  capability_kind: string;
  inputs: ActionValueDefinition[];
  outputs: ActionValueDefinition[];
  constraints?: EngineeringSpecification[];
  required_signal_ports?: string[];
  safety_class: SafetyClass;
}

export interface EquipmentModelDefinition {
  definition_schema_version: string;
  manufacturer: string;
  model_name: string;
  variant?: string;
  equipment_class: EquipmentClass;
  functional_role: FunctionalRole;
  category_code: string;
  signal_domains: SignalDomain[];
  technology_tags?: TechnologyTag[];
  specifications: EngineeringSpecification[];
  signal_ports: SignalPortDefinition[];
  communication_interfaces: CommunicationInterfaceDefinition[];
  capabilities: MeasurementCapabilityDefinition[];
  metadata?: Record<string, unknown>;
}

export interface EquipmentModelIdentity {
  equipment_model_id: string;
  manufacturer: string;
  model_name: string;
  variant: string | null;
  equipment_class: EquipmentClass;
  category_code: string;
  current_approved_revision_id: string | null;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface EquipmentModelRevision {
  revision_id: string;
  equipment_model_id: string;
  revision_number: number;
  parent_revision_id: string | null;
  status: RevisionStatus;
  definition_schema_version: string;
  definition: EquipmentModelDefinition;
  definition_checksum: string;
  created_by: string;
  created_at: string;
  updated_at: string;
  submitted_at: string | null;
  approved_at: string | null;
  capability_count: number;
  interface_count: number;
  signal_port_count: number;
}

export interface EquipmentModelAggregate {
  identity: EquipmentModelIdentity;
  current_approved_revision: EquipmentModelRevision | null;
  latest_revision: EquipmentModelRevision | null;
  active_draft_revision: EquipmentModelRevision | null;
}

export interface DriverScriptStep {
  step_id: string;
  step_type:
    | "io_write"
    | "io_read"
    | "io_query"
    | "can_bus_send"
    | "can_bus_receive"
    | "can_bus_request_response"
    | "set_variable"
    | "parse_number"
    | "parse_text"
    | "parse_csv"
    | "parse_regex"
    | "convert_unit"
    | "calculate"
    | "assert"
    | "if"
    | "loop_until"
    | "repeat"
    | "call_action"
    | "return"
    | "operator_message"
    | "operator_confirmation"
    | "operator_input"
    | "delay"
    | "wait_until"
    | "call_registered_adapter";
  enabled?: boolean;
  interface_id?: string;
  payload_format?: "text" | "hex" | "bytes" | "binary_block";
  payload?: string;
  response_binding?: string;
  timeout_ms?: number;
  variable?: string;
  value?: unknown;
  expression?: string;
  action_id?: string;
  max_iterations?: number;
  duration_ms?: number;
  message?: string;
  frame?: {
    arbitration_id: number;
    extended: boolean;
    remote_frame?: boolean;
    data: number[];
    dlc: number;
  };
  steps?: DriverScriptStep[];
  else_steps?: DriverScriptStep[];
  comment?: string;
}

export interface DriverActionDefinition {
  action_id: string;
  label: string;
  description: string;
  implements_capability_id: string;
  inputs: ActionValueDefinition[];
  outputs: ActionValueDefinition[];
  preconditions?: string[];
  postconditions?: string[];
  safety_class: SafetyClass;
  default_timeout_ms: number;
  script: { steps: DriverScriptStep[] };
  requires_operator_confirmation?: boolean;
  safe_to_retry?: boolean;
  idempotent?: boolean;
  rollback_action_id?: string;
  safe_state_action_id?: string;
}

export interface DriverProfileDefinition {
  definition_schema_version: string;
  equipment_model_id: string;
  supported_model_revision_id: string;
  supported_model_definition_checksum: string;
  supported_firmware_ranges: string[];
  communication_profiles: string[];
  actions: DriverActionDefinition[];
  safe_state_action_id?: string;
  error_check_action_id?: string;
  metadata?: Record<string, unknown>;
}

export interface DriverProfileIdentity {
  driver_profile_id: string;
  equipment_model_id: string;
  label: string;
  current_approved_revision_id: string | null;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface DriverProfileRevision {
  revision_id: string;
  driver_profile_id: string;
  equipment_model_id: string;
  supported_model_revision_id: string;
  revision_number: number;
  parent_revision_id: string | null;
  status: RevisionStatus;
  definition_schema_version: string;
  definition: DriverProfileDefinition;
  definition_checksum: string;
  created_by: string;
  created_at: string;
  updated_at: string;
  submitted_at: string | null;
  approved_at: string | null;
  action_count: number;
}

export interface DriverProfileAggregate {
  identity: DriverProfileIdentity;
  current_approved_revision: DriverProfileRevision | null;
  latest_revision: DriverProfileRevision | null;
  active_draft_revision: DriverProfileRevision | null;
}

export interface EquipmentAuditEvent {
  audit_id: number;
  aggregate_kind: string;
  entity_id: string;
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

export interface EquipmentOperationResult<TAggregate, TRevision> {
  operation: string;
  operation_id: string;
  replayed: boolean;
  aggregate: TAggregate;
  revision: TRevision;
}

export interface CommunicationProviderStatus {
  provider: string;
  available: boolean;
  reason?: string;
}

export interface DriverSimulationScenario {
  scenario_id: string;
  driver_revision_id: string;
  action_id: string;
  input_values: Record<string, unknown>;
  expected_transport_operations: string[];
  simulated_responses: unknown[];
  expected_outputs: Record<string, unknown>;
  expected_messages: string[];
  expected_final_state: Record<string, unknown>;
}

export interface DriverSimulationResult {
  scenario_id: string;
  driver_revision_id: string;
  action_id: string;
  status: string;
  trace: Array<Record<string, unknown>>;
  outputs: Record<string, unknown>;
  final_state: Record<string, unknown>;
  messages: string[];
  virtual_duration_ms: number;
}

export type EquipmentValidationResult = ValidationResult;
