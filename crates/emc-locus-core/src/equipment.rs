use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const EQUIPMENT_MODEL_DEFINITION_SCHEMA_VERSION: &str =
    "emc-locus.equipment-model-definition.v2";
pub const DRIVER_PROFILE_DEFINITION_SCHEMA_VERSION: &str = "emc-locus.driver-profile-definition.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentRevisionStatus {
    Draft,
    UnderReview,
    Approved,
    Superseded,
    Suspended,
    Retired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentClass {
    ControllableInstrument,
    DaqDevice,
    AcquisitionDevice,
    Converter,
    Sensor,
    Transducer,
    PassiveComponent,
    SwitchingDevice,
    MotionSystem,
    Facility,
    SoftwareAdapter,
    ManualEquipment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionalRole {
    EnergySource,
    SignalSource,
    RfNetworkElement,
    Sensor,
    Actuator,
    MeasurementInstrument,
    AcquisitionDevice,
    Converter,
    ControlSystem,
    SoftwareSystem,
    Facility,
    ManualAccessory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalQuantity {
    Frequency,
    Time,
    Voltage,
    Current,
    Power,
    ElectricField,
    MagneticField,
    Impedance,
    Resistance,
    Capacitance,
    Inductance,
    Temperature,
    Distance,
    Angle,
    Velocity,
    Acceleration,
    Pressure,
    Dimensionless,
    Text,
    Boolean,
    Binary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    Number,
    Integer,
    Boolean,
    Text,
    Bytes,
    List,
    Object,
    Frame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortDirectionality {
    Input,
    Output,
    Bidirectional,
    Through,
    Control,
    Communication,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortFlowRole {
    SourcePort,
    SinkPort,
    ThroughPort,
    MeasurementPort,
    ControlPort,
    CommunicationPort,
    FieldSidePort,
    TransducerOutputPort,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalDomain {
    PowerDc,
    PowerAc,
    Rf,
    AnalogVoltage,
    AnalogCurrent,
    AnalogCharge,
    DigitalLogic,
    Trigger,
    Pulse,
    ContactDry,
    Relay,
    CanBus,
    Rs232,
    Rs485,
    Ethernet,
    Usb,
    Gpib,
    Optical,
    Mechanical,
    Environmental,
    Software,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TechnologyTag {
    AdcConverter,
    DacConverter,
    #[serde(rename = "rf_50_ohm")]
    Rf50Ohm,
    #[serde(rename = "rf_75_ohm")]
    Rf75Ohm,
    Ttl,
    Cmos,
    Trigger,
    DryContact,
    RelayContact,
    VoltageInput,
    CurrentInput,
    ChargeInput,
    Iepe,
    Bridge,
    Usb,
    Ethernet,
    Gpib,
    Rs232,
    Rs485,
    CanBus,
    Visa,
    RawTcp,
    SerialText,
    Scpi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    None,
    Serial,
    Gpib,
    EthernetTcp,
    EthernetUdp,
    CanBus,
    Usb,
    Rs485,
    Lin,
    Modbus,
    Bluetooth,
    VendorBus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessProviderKind {
    NativeSerial,
    NativeTcp,
    NativeUdp,
    Visa,
    Socketcan,
    Pcan,
    VectorCan,
    Usbtmc,
    Hid,
    CustomAdapter,
    Simulation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolKind {
    Scpi,
    RawAscii,
    RawBinary,
    CanBusFrames,
    ModbusRtu,
    ModbusTcp,
    CustomProtocol,
    Manual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyClass {
    ReadOnly,
    ConfigurationChange,
    EnergizesOutput,
    DeenergizesOutput,
    MovesMechanism,
    ChangesRouting,
    RequiresInterlock,
    PotentiallyDestructive,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngineeringSpecification {
    pub specification_id: String,
    pub label: String,
    pub quantity: PhysicalQuantity,
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nominal: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignalPortDefinition {
    pub port_id: String,
    pub label: String,
    pub directionality: PortDirectionality,
    pub flow_role: PortFlowRole,
    pub signal_domain: SignalDomain,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub technology_tags: Vec<TechnologyTag>,
    pub quantity: PhysicalQuantity,
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impedance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voltage_max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_index: Option<u32>,
    #[serde(default)]
    pub differential: bool,
    #[serde(default)]
    pub isolated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IdentificationStrategy {
    pub strategy_id: String,
    pub strategy_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_regex: Option<String>,
    #[serde(default)]
    pub parameters: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommunicationInterfaceDefinition {
    pub interface_id: String,
    pub label: String,
    pub transport_kind: TransportKind,
    pub access_provider_kind: AccessProviderKind,
    pub protocol_kind: ProtocolKind,
    pub required: bool,
    pub default_interface: bool,
    #[serde(default)]
    pub configuration_schema: BTreeMap<String, Value>,
    #[serde(default)]
    pub default_configuration: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identification_strategy: Option<IdentificationStrategy>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub firmware_compatibility: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionValueDefinition {
    pub name: String,
    pub value_type: ValueType,
    pub quantity: PhysicalQuantity,
    pub unit: String,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeasurementCapabilityDefinition {
    pub capability_id: String,
    pub label: String,
    pub description: String,
    pub capability_kind: String,
    #[serde(default)]
    pub inputs: Vec<ActionValueDefinition>,
    #[serde(default)]
    pub outputs: Vec<ActionValueDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<EngineeringSpecification>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_signal_ports: Vec<String>,
    pub safety_class: SafetyClass,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EquipmentModelDefinition {
    pub definition_schema_version: String,
    pub manufacturer: String,
    pub model_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    pub equipment_class: EquipmentClass,
    pub functional_role: FunctionalRole,
    pub category_code: String,
    #[serde(default)]
    pub signal_domains: Vec<SignalDomain>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub technology_tags: Vec<TechnologyTag>,
    #[serde(default)]
    pub specifications: Vec<EngineeringSpecification>,
    #[serde(default)]
    pub signal_ports: Vec<SignalPortDefinition>,
    #[serde(default)]
    pub communication_interfaces: Vec<CommunicationInterfaceDefinition>,
    #[serde(default)]
    pub capabilities: Vec<MeasurementCapabilityDefinition>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadFormat {
    Text,
    Hex,
    Bytes,
    BinaryBlock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptStepType {
    IoWrite,
    IoRead,
    IoQuery,
    CanBusSend,
    CanBusReceive,
    CanBusRequestResponse,
    SetVariable,
    ParseNumber,
    ParseText,
    ParseCsv,
    ParseRegex,
    ConvertUnit,
    Calculate,
    Assert,
    If,
    LoopUntil,
    Repeat,
    CallAction,
    Return,
    OperatorMessage,
    OperatorConfirmation,
    OperatorInput,
    Delay,
    WaitUntil,
    CallRegisteredAdapter,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanFrameDefinition {
    pub arbitration_id: u32,
    pub extended: bool,
    #[serde(default)]
    pub remote_frame: bool,
    #[serde(default)]
    pub data: Vec<u8>,
    pub dlc: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetryPolicyDefinition {
    pub max_attempts: u32,
    pub delay_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backoff: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retry_on: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriverScriptStep {
    pub step_id: String,
    pub step_type: ScriptStepType,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_format: Option<PayloadFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_binding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<CanFrameDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<DriverScriptStep>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub else_steps: Vec<DriverScriptStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_policy: Option<RetryPolicyDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

fn default_enabled() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriverScriptDefinition {
    #[serde(default)]
    pub steps: Vec<DriverScriptStep>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriverActionDefinition {
    pub action_id: String,
    pub label: String,
    pub description: String,
    pub implements_capability_id: String,
    #[serde(default)]
    pub inputs: Vec<ActionValueDefinition>,
    #[serde(default)]
    pub outputs: Vec<ActionValueDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preconditions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub postconditions: Vec<String>,
    pub safety_class: SafetyClass,
    pub default_timeout_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_policy: Option<RetryPolicyDefinition>,
    pub script: DriverScriptDefinition,
    #[serde(default)]
    pub requires_operator_confirmation: bool,
    #[serde(default)]
    pub safe_to_retry: bool,
    #[serde(default)]
    pub idempotent: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback_action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safe_state_action_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriverProfileDefinition {
    pub definition_schema_version: String,
    pub equipment_model_id: String,
    pub supported_model_revision_id: String,
    pub supported_model_definition_checksum: String,
    #[serde(default)]
    pub supported_firmware_ranges: Vec<String>,
    #[serde(default)]
    pub communication_profiles: Vec<String>,
    #[serde(default)]
    pub actions: Vec<DriverActionDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safe_state_action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_check_action_id: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionValidationIssue {
    pub severity: String,
    pub code: String,
    pub path: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalEquipmentDefinition {
    pub definition_schema_version: String,
    pub canonical_json: String,
    pub definition_checksum: String,
}

impl EquipmentModelDefinition {
    pub fn from_json_str(value: &str) -> Result<Self, DefinitionValidationIssue> {
        serde_json::from_str(value).map_err(|error| {
            issue(
                "error",
                "invalid_equipment_model_definition_json",
                "$",
                error.to_string(),
                Some("Send a JSON object matching EquipmentModelDefinition v1."),
            )
        })
    }

    pub fn validate_all(&self) -> Vec<DefinitionValidationIssue> {
        validate_equipment_model_definition(self)
    }

    pub fn canonicalize(
        &self,
    ) -> Result<CanonicalEquipmentDefinition, Vec<DefinitionValidationIssue>> {
        canonicalize_definition(
            self,
            self.definition_schema_version.clone(),
            validate_equipment_model_definition(self),
            "equipment_model_definition_serialization_failed",
        )
    }
}

impl DriverProfileDefinition {
    pub fn from_json_str(value: &str) -> Result<Self, DefinitionValidationIssue> {
        serde_json::from_str(value).map_err(|error| {
            issue(
                "error",
                "invalid_driver_profile_definition_json",
                "$",
                error.to_string(),
                Some("Send a JSON object matching DriverProfileDefinition v1."),
            )
        })
    }

    pub fn validate_all(
        &self,
        model: Option<&EquipmentModelDefinition>,
    ) -> Vec<DefinitionValidationIssue> {
        validate_driver_profile_definition(self, model)
    }

    pub fn canonicalize(
        &self,
        model: Option<&EquipmentModelDefinition>,
    ) -> Result<CanonicalEquipmentDefinition, Vec<DefinitionValidationIssue>> {
        canonicalize_definition(
            self,
            self.definition_schema_version.clone(),
            validate_driver_profile_definition(self, model),
            "driver_profile_definition_serialization_failed",
        )
    }
}

fn canonicalize_definition<T: Serialize>(
    definition: &T,
    schema_version: String,
    issues: Vec<DefinitionValidationIssue>,
    serialization_code: &'static str,
) -> Result<CanonicalEquipmentDefinition, Vec<DefinitionValidationIssue>> {
    if issues.iter().any(|item| item.severity == "error") {
        return Err(issues);
    }
    let mut value = serde_json::to_value(definition).map_err(|error| {
        vec![issue(
            "error",
            serialization_code,
            "$",
            error.to_string(),
            Option::<String>::None,
        )]
    })?;
    canonicalize_json_value(&mut value);
    let canonical_json = serde_json::to_string(&value).map_err(|error| {
        vec![issue(
            "error",
            serialization_code,
            "$",
            error.to_string(),
            Option::<String>::None,
        )]
    })?;
    let digest = Sha256::digest(canonical_json.as_bytes());
    Ok(CanonicalEquipmentDefinition {
        definition_schema_version: schema_version,
        canonical_json,
        definition_checksum: format!("sha256:{digest:x}"),
    })
}

pub fn validate_equipment_model_definition(
    definition: &EquipmentModelDefinition,
) -> Vec<DefinitionValidationIssue> {
    let mut issues = Vec::new();
    if definition.definition_schema_version != EQUIPMENT_MODEL_DEFINITION_SCHEMA_VERSION {
        issues.push(issue(
            "error",
            "unsupported_equipment_model_definition_schema",
            "definition_schema_version",
            format!(
                "unsupported schema version: {}",
                definition.definition_schema_version
            ),
            Some(EQUIPMENT_MODEL_DEFINITION_SCHEMA_VERSION),
        ));
    }
    require_text(&mut issues, &definition.manufacturer, "manufacturer");
    require_text(&mut issues, &definition.model_name, "model_name");
    require_token(&mut issues, &definition.category_code, "category_code");
    validate_equipment_classification(&mut issues, definition);

    let model_signal_domains = definition
        .signal_domains
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let model_technology_tags = definition
        .technology_tags
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let port_ids = validate_signal_ports(
        &mut issues,
        &definition.signal_ports,
        &model_signal_domains,
        &model_technology_tags,
    );
    validate_functional_port_topology(&mut issues, definition);
    let interface_ids = validate_interfaces(&mut issues, &definition.communication_interfaces);
    validate_can_bus_protocol_topology(&mut issues, definition);
    validate_specifications(&mut issues, &definition.specifications);
    validate_capabilities(
        &mut issues,
        &definition.capabilities,
        &port_ids,
        &port_map_for(&definition.signal_ports),
    );

    if definition.equipment_class == EquipmentClass::ManualEquipment
        && definition
            .communication_interfaces
            .iter()
            .any(|interface| interface.transport_kind != TransportKind::None)
    {
        issues.push(issue(
            "warning",
            "manual_equipment_has_transport",
            "communication_interfaces",
            "manual equipment normally has no communication interface",
            Some("Use transport_kind=none and protocol_kind=manual unless a software adapter is required."),
        ));
    }
    if definition.equipment_class == EquipmentClass::ControllableInstrument
        && interface_ids.is_empty()
    {
        issues.push(issue(
            "warning",
            "controllable_equipment_without_interface",
            "communication_interfaces",
            "controllable equipment should define at least one communication interface",
            Some("Add a simulation, serial, TCP, UDP, VISA, CAN bus, or USB interface."),
        ));
    }
    issues
}

fn validate_equipment_classification(
    issues: &mut Vec<DefinitionValidationIssue>,
    definition: &EquipmentModelDefinition,
) {
    let allows_empty_domains = matches!(
        definition.functional_role,
        FunctionalRole::ManualAccessory | FunctionalRole::SoftwareSystem
    ) || matches!(
        definition.equipment_class,
        EquipmentClass::ManualEquipment | EquipmentClass::SoftwareAdapter
    );
    if definition.signal_domains.is_empty() && !allows_empty_domains {
        issues.push(issue(
            "error",
            "missing_signal_domains",
            "signal_domains",
            "equipment model must declare the signal, energy, or communication domains it handles",
            Some("Declare values such as rf, analog_voltage, power_dc, ethernet, or can_bus."),
        ));
    }
    let category_code = definition.category_code.as_str();
    if matches!(category_code, "can" | "adc" | "dac") {
        issues.push(issue(
            "error",
            "ambiguous_equipment_category_code",
            "category_code",
            format!(
                "category_code={} is ambiguous in a French/English laboratory context",
                definition.category_code
            ),
            Some("Use adc_converter, dac_converter, or can_bus with explicit context."),
        ));
    }
    if category_code == "adc_converter"
        && !definition
            .technology_tags
            .contains(&TechnologyTag::AdcConverter)
    {
        issues.push(issue(
            "error",
            "adc_converter_requires_adc_technology_tag",
            "technology_tags",
            "ADC converter models must declare technology_tags=adc_converter",
            Some("Use adc_converter for analog-to-digital conversion; do not use can_bus unless the model really has a CAN bus port."),
        ));
    }
    if category_code == "dac_converter"
        && !definition
            .technology_tags
            .contains(&TechnologyTag::DacConverter)
    {
        issues.push(issue(
            "error",
            "dac_converter_requires_dac_technology_tag",
            "technology_tags",
            "DAC converter models must declare technology_tags=dac_converter",
            Some("Use dac_converter for digital-to-analog conversion; do not use can_bus unless the model really has a CAN bus port."),
        ));
    }
    if category_code == "can_bus"
        && (!definition.signal_domains.contains(&SignalDomain::CanBus)
            || !definition.technology_tags.contains(&TechnologyTag::CanBus))
    {
        issues.push(issue(
            "error",
            "can_bus_category_requires_can_bus_classification",
            "signal_domains",
            "CAN bus models must explicitly declare signal_domains=can_bus and technology_tags=can_bus",
            Some("Model Controller Area Network as can_bus; keep ADC/DAC conversion separate."),
        ));
    }
    if definition.technology_tags.contains(&TechnologyTag::CanBus)
        && !definition.signal_domains.contains(&SignalDomain::CanBus)
    {
        issues.push(issue(
            "error",
            "can_bus_tag_requires_can_bus_domain",
            "technology_tags",
            "technology_tags=can_bus requires signal_domains=can_bus",
            Some("Add a CAN bus communication domain or remove the CAN bus tag."),
        ));
    }
    if matches!(
        definition.functional_role,
        FunctionalRole::AcquisitionDevice | FunctionalRole::Converter
    ) && definition.category_code == "can_bus"
    {
        issues.push(issue(
            "error",
            "adc_dac_can_bus_ambiguity",
            "category_code",
            "an ADC/DAC converter role cannot be categorized as can_bus",
            Some("Use adc_converter or dac_converter for converters; reserve can_bus for Controller Area Network communication."),
        ));
    }
    if definition.functional_role == FunctionalRole::Converter
        && !matches!(
            definition.equipment_class,
            EquipmentClass::Converter
                | EquipmentClass::DaqDevice
                | EquipmentClass::AcquisitionDevice
        )
    {
        issues.push(issue(
            "warning",
            "equipment_class_functional_role_mismatch",
            "equipment_class",
            "converter functional role is normally modeled with converter, daq_device, or acquisition_device equipment class",
            Some("Use equipment_class=converter for a pure converter, or daq_device/acquisition_device when it is also an acquisition device."),
        ));
    }
    if definition.functional_role == FunctionalRole::MeasurementInstrument
        && !matches!(
            definition.equipment_class,
            EquipmentClass::ControllableInstrument
                | EquipmentClass::DaqDevice
                | EquipmentClass::AcquisitionDevice
                | EquipmentClass::ManualEquipment
        )
    {
        issues.push(issue(
            "warning",
            "equipment_class_functional_role_mismatch",
            "equipment_class",
            "measurement instrument role is unusual for this equipment class",
            Some("Use controllable_instrument, daq_device, acquisition_device, or manual_equipment unless this is intentionally hybrid."),
        ));
    }
}

fn validate_can_bus_protocol_topology(
    issues: &mut Vec<DefinitionValidationIssue>,
    definition: &EquipmentModelDefinition,
) {
    let has_can_bus_protocol = definition
        .communication_interfaces
        .iter()
        .any(|interface| interface.protocol_kind == ProtocolKind::CanBusFrames);
    if !has_can_bus_protocol {
        return;
    }
    if !definition.signal_domains.contains(&SignalDomain::CanBus) {
        issues.push(issue(
            "error",
            "can_bus_protocol_requires_can_bus_domain",
            "signal_domains",
            "protocol_kind=can_bus_frames requires signal_domains=can_bus",
            Some("Add the CAN bus domain and a CAN bus communication port, or use a non-CAN protocol."),
        ));
    }
    if !definition.signal_ports.iter().any(|port| {
        port.signal_domain == SignalDomain::CanBus
            && port.flow_role == PortFlowRole::CommunicationPort
            && matches!(
                port.directionality,
                PortDirectionality::Communication | PortDirectionality::Bidirectional
            )
    }) {
        issues.push(issue(
            "error",
            "can_bus_protocol_requires_can_bus_port",
            "signal_ports",
            "protocol_kind=can_bus_frames requires an explicit CAN bus communication port",
            Some("Add a port with signal_domain=can_bus, flow_role=communication_port and directionality=communication."),
        ));
    }
}

pub fn validate_driver_profile_definition(
    definition: &DriverProfileDefinition,
    model: Option<&EquipmentModelDefinition>,
) -> Vec<DefinitionValidationIssue> {
    let mut issues = Vec::new();
    if definition.definition_schema_version != DRIVER_PROFILE_DEFINITION_SCHEMA_VERSION {
        issues.push(issue(
            "error",
            "unsupported_driver_profile_definition_schema",
            "definition_schema_version",
            format!(
                "unsupported schema version: {}",
                definition.definition_schema_version
            ),
            Some(DRIVER_PROFILE_DEFINITION_SCHEMA_VERSION),
        ));
    }
    require_token(
        &mut issues,
        &definition.equipment_model_id,
        "equipment_model_id",
    );
    require_token(
        &mut issues,
        &definition.supported_model_revision_id,
        "supported_model_revision_id",
    );
    require_checksum(
        &mut issues,
        &definition.supported_model_definition_checksum,
        "supported_model_definition_checksum",
    );
    let model_interface_ids = model.map(interface_ids_for).unwrap_or_default();
    let model_capability_ids = model.map(capability_ids_for).unwrap_or_default();
    for (index, interface_id) in definition.communication_profiles.iter().enumerate() {
        require_token(
            &mut issues,
            interface_id,
            &format!("communication_profiles[{index}]"),
        );
        if model.is_some() && !model_interface_ids.contains(interface_id) {
            issues.push(issue(
                "error",
                "unknown_driver_interface",
                format!("communication_profiles[{index}]"),
                format!(
                    "driver references interface not present in approved model: {interface_id}"
                ),
                Some("Reference an interface_id from the supported model revision."),
            ));
        }
    }
    let action_ids = validate_actions(
        &mut issues,
        &definition.actions,
        &model_interface_ids,
        &model_capability_ids,
    );
    for (path, value) in [
        (
            "safe_state_action_id",
            definition.safe_state_action_id.as_deref(),
        ),
        (
            "error_check_action_id",
            definition.error_check_action_id.as_deref(),
        ),
    ] {
        if let Some(action_id) = value {
            if !action_ids.contains(action_id) {
                issues.push(issue(
                    "error",
                    "unknown_driver_action_reference",
                    path,
                    format!("{path} references unknown action: {action_id}"),
                    Some("Use an action_id declared in actions."),
                ));
            }
        }
    }
    issues
}

fn validate_specifications(
    issues: &mut Vec<DefinitionValidationIssue>,
    specifications: &[EngineeringSpecification],
) {
    let mut ids = BTreeSet::new();
    for (index, specification) in specifications.iter().enumerate() {
        let path = format!("specifications[{index}]");
        require_token(
            issues,
            &specification.specification_id,
            &format!("{path}.specification_id"),
        );
        require_text(issues, &specification.label, &format!("{path}.label"));
        if !ids.insert(specification.specification_id.clone()) {
            issues.push(issue(
                "error",
                "duplicate_specification_id",
                format!("{path}.specification_id"),
                format!(
                    "duplicate specification id: {}",
                    specification.specification_id
                ),
                Option::<String>::None,
            ));
        }
        validate_quantity_unit(
            issues,
            specification.quantity,
            &specification.unit,
            &format!("{path}.unit"),
        );
        validate_bounds(
            issues,
            specification.minimum,
            specification.maximum,
            &format!("{path}.minimum"),
            &format!("{path}.maximum"),
        );
    }
}

fn validate_signal_ports(
    issues: &mut Vec<DefinitionValidationIssue>,
    ports: &[SignalPortDefinition],
    model_signal_domains: &BTreeSet<SignalDomain>,
    model_technology_tags: &BTreeSet<TechnologyTag>,
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for (index, port) in ports.iter().enumerate() {
        let path = format!("signal_ports[{index}]");
        require_token(issues, &port.port_id, &format!("{path}.port_id"));
        require_text(issues, &port.label, &format!("{path}.label"));
        if !ids.insert(port.port_id.clone()) {
            issues.push(issue(
                "error",
                "duplicate_signal_port_id",
                format!("{path}.port_id"),
                format!("duplicate signal port id: {}", port.port_id),
                Option::<String>::None,
            ));
        }
        validate_quantity_unit(issues, port.quantity, &port.unit, &format!("{path}.unit"));
        validate_port_directionality(issues, port, &path);
        if !model_signal_domains.contains(&port.signal_domain) {
            issues.push(issue(
                "error",
                "port_signal_domain_not_declared_on_model",
                format!("{path}.signal_domain"),
                format!(
                    "port domain {:?} is not present in model signal_domains",
                    port.signal_domain
                ),
                Some("Add the domain to signal_domains or correct the port classification."),
            ));
        }
        for (tag_index, tag) in port.technology_tags.iter().enumerate() {
            if !technology_tag_compatible_with_domain(*tag, port.signal_domain) {
                issues.push(issue(
                    "error",
                    "port_technology_tag_domain_mismatch",
                    format!("{path}.technology_tags[{tag_index}]"),
                    format!(
                        "technology tag {:?} is not compatible with port domain {:?}",
                        tag, port.signal_domain
                    ),
                    Some("Choose a tag compatible with the port signal_domain or correct the port domain."),
                ));
            }
            if !model_technology_tags.contains(tag) {
                issues.push(issue(
                    "warning",
                    "port_technology_tag_not_declared_on_model",
                    format!("{path}.technology_tags[{tag_index}]"),
                    format!(
                        "port tag {:?} is not declared in model technology_tags",
                        tag
                    ),
                    Some("Add the tag to model technology_tags if it describes this equipment."),
                ));
            }
        }
        if port.signal_domain == SignalDomain::Rf
            && !is_communication_port(port)
            && port.flow_role != PortFlowRole::FieldSidePort
            && port.impedance.unwrap_or(0.0) <= 0.0
        {
            issues.push(issue(
                "error",
                "rf_port_missing_impedance",
                format!("{path}.impedance"),
                "RF signal ports must declare their reference impedance",
                Some("Declare impedance such as 50.0 or 75.0 ohm."),
            ));
        } else if port.signal_domain == SignalDomain::Rf
            && !is_communication_port(port)
            && port.flow_role != PortFlowRole::FieldSidePort
            && port.impedance.is_some_and(|impedance| {
                (impedance - 50.0).abs() > 0.001 && (impedance - 75.0).abs() > 0.001
            })
        {
            issues.push(issue(
                "warning",
                "rf_port_non_standard_impedance",
                format!("{path}.impedance"),
                "RF signal port impedance is neither 50 ohm nor 75 ohm",
                Some("Most EMC conducted RF chains use 50 ohm; keep other values only when explicit."),
            ));
        }
        if is_communication_port(port) && !is_communication_signal_domain(port.signal_domain) {
            issues.push(issue(
                "error",
                "communication_port_has_physical_signal_domain",
                format!("{path}.signal_domain"),
                "communication ports must use a communication signal domain",
                Some("Use ethernet, usb, gpib, rs232, rs485, can_bus, or software."),
            ));
        }
        if !is_communication_port(port) && is_communication_signal_domain(port.signal_domain) {
            issues.push(issue(
                "error",
                "communication_domain_used_as_measurement_port",
                format!("{path}.flow_role"),
                "communication domains cannot be modeled as measurement signal ports",
                Some("Use flow_role=communication_port and directionality=communication, or choose a physical signal domain."),
            ));
        }
        validate_bounds(
            issues,
            port.frequency_min,
            port.frequency_max,
            &format!("{path}.frequency_min"),
            &format!("{path}.frequency_max"),
        );
    }
    ids
}

fn technology_tag_compatible_with_domain(tag: TechnologyTag, domain: SignalDomain) -> bool {
    match tag {
        TechnologyTag::AdcConverter => matches!(
            domain,
            SignalDomain::AnalogVoltage | SignalDomain::AnalogCurrent | SignalDomain::DigitalLogic
        ),
        TechnologyTag::DacConverter => matches!(
            domain,
            SignalDomain::DigitalLogic | SignalDomain::AnalogVoltage | SignalDomain::AnalogCurrent
        ),
        TechnologyTag::Rf50Ohm | TechnologyTag::Rf75Ohm => domain == SignalDomain::Rf,
        TechnologyTag::Ttl | TechnologyTag::Cmos => {
            matches!(domain, SignalDomain::DigitalLogic | SignalDomain::Trigger)
        }
        TechnologyTag::Trigger => matches!(domain, SignalDomain::Trigger | SignalDomain::Pulse),
        TechnologyTag::DryContact => domain == SignalDomain::ContactDry,
        TechnologyTag::RelayContact => domain == SignalDomain::Relay,
        TechnologyTag::VoltageInput | TechnologyTag::Bridge => {
            domain == SignalDomain::AnalogVoltage
        }
        TechnologyTag::CurrentInput => domain == SignalDomain::AnalogCurrent,
        TechnologyTag::ChargeInput => domain == SignalDomain::AnalogCharge,
        TechnologyTag::Iepe => matches!(
            domain,
            SignalDomain::AnalogVoltage | SignalDomain::AnalogCurrent
        ),
        TechnologyTag::Usb => domain == SignalDomain::Usb,
        TechnologyTag::Ethernet | TechnologyTag::RawTcp => domain == SignalDomain::Ethernet,
        TechnologyTag::Gpib => domain == SignalDomain::Gpib,
        TechnologyTag::Rs232 | TechnologyTag::SerialText => domain == SignalDomain::Rs232,
        TechnologyTag::Rs485 => domain == SignalDomain::Rs485,
        TechnologyTag::CanBus => domain == SignalDomain::CanBus,
        TechnologyTag::Visa | TechnologyTag::Scpi => matches!(
            domain,
            SignalDomain::Ethernet
                | SignalDomain::Usb
                | SignalDomain::Gpib
                | SignalDomain::Rs232
                | SignalDomain::Rs485
        ),
    }
}

fn validate_port_directionality(
    issues: &mut Vec<DefinitionValidationIssue>,
    port: &SignalPortDefinition,
    path: &str,
) {
    let compatible = match port.flow_role {
        PortFlowRole::SourcePort => matches!(
            port.directionality,
            PortDirectionality::Output | PortDirectionality::Bidirectional
        ),
        PortFlowRole::SinkPort | PortFlowRole::MeasurementPort => matches!(
            port.directionality,
            PortDirectionality::Input | PortDirectionality::Bidirectional
        ),
        PortFlowRole::ThroughPort => matches!(
            port.directionality,
            PortDirectionality::Through | PortDirectionality::Bidirectional
        ),
        PortFlowRole::ControlPort => matches!(
            port.directionality,
            PortDirectionality::Control
                | PortDirectionality::Input
                | PortDirectionality::Output
                | PortDirectionality::Bidirectional
        ),
        PortFlowRole::CommunicationPort => matches!(
            port.directionality,
            PortDirectionality::Communication | PortDirectionality::Bidirectional
        ),
        PortFlowRole::FieldSidePort => matches!(
            port.directionality,
            PortDirectionality::Input
                | PortDirectionality::Output
                | PortDirectionality::Bidirectional
        ),
        PortFlowRole::TransducerOutputPort => matches!(
            port.directionality,
            PortDirectionality::Output | PortDirectionality::Bidirectional
        ),
    };
    if !compatible {
        issues.push(issue(
            "error",
            "port_flow_role_directionality_mismatch",
            format!("{path}.flow_role"),
            format!(
                "flow_role {:?} is not compatible with directionality {:?}",
                port.flow_role, port.directionality
            ),
            Some("Choose a flow role that matches the physical direction of energy, signal, control, or communication."),
        ));
    }
}

fn validate_interfaces(
    issues: &mut Vec<DefinitionValidationIssue>,
    interfaces: &[CommunicationInterfaceDefinition],
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let mut default_count = 0;
    for (index, interface) in interfaces.iter().enumerate() {
        let path = format!("communication_interfaces[{index}]");
        require_token(
            issues,
            &interface.interface_id,
            &format!("{path}.interface_id"),
        );
        require_text(issues, &interface.label, &format!("{path}.label"));
        if !ids.insert(interface.interface_id.clone()) {
            issues.push(issue(
                "error",
                "duplicate_communication_interface_id",
                format!("{path}.interface_id"),
                format!(
                    "duplicate communication interface id: {}",
                    interface.interface_id
                ),
                Option::<String>::None,
            ));
        }
        if interface.default_interface {
            default_count += 1;
        }
        validate_interface_compatibility(issues, interface, &path);
    }
    if default_count > 1 {
        issues.push(issue(
            "error",
            "multiple_default_interfaces",
            "communication_interfaces",
            "only one communication interface can be marked as the default",
            Some("Keep default_interface=true on a single preferred interface."),
        ));
    }
    ids
}

fn validate_functional_port_topology(
    issues: &mut Vec<DefinitionValidationIssue>,
    definition: &EquipmentModelDefinition,
) {
    let through_count = definition
        .signal_ports
        .iter()
        .filter(|port| {
            port.flow_role == PortFlowRole::ThroughPort
                && matches!(
                    port.directionality,
                    PortDirectionality::Through | PortDirectionality::Bidirectional
                )
        })
        .count();
    if through_count == 1 {
        issues.push(issue(
            "error",
            "through_device_requires_two_ports",
            "signal_ports",
            "through-path devices must expose at least two through-compatible ports",
            Some("Model both sides of a cable, attenuator, filter, coupler, combiner, divider, isolator, or adapter."),
        ));
    }

    if definition.functional_role == FunctionalRole::Sensor {
        let has_physical_input = definition.signal_ports.iter().any(is_physical_input_port);
        let has_output = definition.signal_ports.iter().any(is_physical_output_port);
        if !has_physical_input || !has_output {
            issues.push(issue(
                "error",
                "sensor_requires_input_and_output",
                "signal_ports",
                "sensor models must expose at least one physical input and one output",
                Some("Example: FIELD input plus RF_OUT, voltage, current, charge, digital, or optical output."),
            ));
        }
    }

    if definition.functional_role == FunctionalRole::SignalSource
        && !definition
            .signal_ports
            .iter()
            .any(|port| port.flow_role == PortFlowRole::SourcePort && !is_communication_port(port))
    {
        issues.push(issue(
            "error",
            "signal_source_requires_source_output",
            "signal_ports",
            "signal sources must expose at least one physical source output",
            Some("Add an RF_OUT, analog output, pulse output, field-side output, or other source port."),
        ));
    }

    if definition.functional_role == FunctionalRole::MeasurementInstrument
        && !definition.signal_ports.iter().any(|port| {
            port.flow_role == PortFlowRole::MeasurementPort && !is_communication_port(port)
        })
    {
        issues.push(issue(
            "error",
            "measurement_instrument_requires_measurement_input",
            "signal_ports",
            "measurement instruments must expose at least one physical measurement input",
            Some("Add an RF, voltage, current, field, temperature, vibration, acoustic, or digital measurement input."),
        ));
    }

    if (definition.functional_role == FunctionalRole::AcquisitionDevice
        || definition.equipment_class == EquipmentClass::AcquisitionDevice)
        && (!definition.signal_ports.iter().any(|port| {
            port.flow_role == PortFlowRole::MeasurementPort && !is_communication_port(port)
        }) || !definition.signal_ports.iter().any(|port| {
            port.flow_role == PortFlowRole::CommunicationPort
                || port.flow_role == PortFlowRole::ControlPort
        }))
    {
        issues.push(issue(
            "error",
            "acquisition_device_requires_input_and_control_path",
            "signal_ports",
            "acquisition devices must expose at least one measurement input and one communication/control path",
            Some("Add an analog/digital measurement input and a USB, Ethernet, trigger, or control port."),
        ));
    }

    let communication_only_domains = definition
        .signal_domains
        .iter()
        .all(|domain| is_communication_signal_domain(*domain));
    let allows_physical_ports = definition
        .metadata
        .get("allow_physical_ports")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if definition.functional_role == FunctionalRole::SoftwareSystem
        && communication_only_domains
        && !allows_physical_ports
        && definition
            .signal_ports
            .iter()
            .any(|port| port.signal_domain == SignalDomain::Rf)
    {
        issues.push(issue(
            "error",
            "software_system_declares_physical_rf_port",
            "signal_ports",
            "communication-only software systems cannot declare physical RF ports",
            Some("Set metadata.allow_physical_ports=true only for an explicitly modeled hybrid adapter."),
        ));
    }
}

fn validate_capabilities(
    issues: &mut Vec<DefinitionValidationIssue>,
    capabilities: &[MeasurementCapabilityDefinition],
    port_ids: &BTreeSet<String>,
    ports_by_id: &BTreeMap<String, &SignalPortDefinition>,
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for (index, capability) in capabilities.iter().enumerate() {
        let path = format!("capabilities[{index}]");
        require_token(
            issues,
            &capability.capability_id,
            &format!("{path}.capability_id"),
        );
        require_text(issues, &capability.label, &format!("{path}.label"));
        require_text(
            issues,
            &capability.description,
            &format!("{path}.description"),
        );
        require_token(
            issues,
            &capability.capability_kind,
            &format!("{path}.capability_kind"),
        );
        if !ids.insert(capability.capability_id.clone()) {
            issues.push(issue(
                "error",
                "duplicate_capability_id",
                format!("{path}.capability_id"),
                format!("duplicate capability id: {}", capability.capability_id),
                Option::<String>::None,
            ));
        }
        validate_action_values(issues, &capability.inputs, &format!("{path}.inputs"));
        validate_action_values(issues, &capability.outputs, &format!("{path}.outputs"));
        for (port_index, port_id) in capability.required_signal_ports.iter().enumerate() {
            if !port_ids.contains(port_id) {
                issues.push(issue(
                    "error",
                    "unknown_capability_port",
                    format!("{path}.required_signal_ports[{port_index}]"),
                    format!("capability references unknown signal port: {port_id}"),
                    Some("Use a port_id declared in signal_ports."),
                ));
            } else if ports_by_id
                .get(port_id)
                .is_some_and(|port| is_communication_port(port))
            {
                issues.push(issue(
                    "error",
                    "communication_port_used_as_signal_port",
                    format!("{path}.required_signal_ports[{port_index}]"),
                    format!("capability references communication-only port as a measurement signal: {port_id}"),
                    Some("Reference a physical signal port, not Ethernet, USB, GPIB, RS-232/RS-485, or CAN bus communication."),
                ));
            }
        }
    }
    ids
}

fn port_map_for(ports: &[SignalPortDefinition]) -> BTreeMap<String, &SignalPortDefinition> {
    ports
        .iter()
        .map(|port| (port.port_id.clone(), port))
        .collect()
}

fn is_communication_signal_domain(domain: SignalDomain) -> bool {
    matches!(
        domain,
        SignalDomain::CanBus
            | SignalDomain::Rs232
            | SignalDomain::Rs485
            | SignalDomain::Ethernet
            | SignalDomain::Usb
            | SignalDomain::Gpib
            | SignalDomain::Software
    )
}

fn is_communication_port(port: &SignalPortDefinition) -> bool {
    port.flow_role == PortFlowRole::CommunicationPort
        || port.directionality == PortDirectionality::Communication
        || is_communication_signal_domain(port.signal_domain)
}

fn is_physical_input_port(port: &SignalPortDefinition) -> bool {
    !is_communication_port(port)
        && matches!(
            port.directionality,
            PortDirectionality::Input | PortDirectionality::Bidirectional
        )
        && matches!(
            port.flow_role,
            PortFlowRole::SinkPort | PortFlowRole::MeasurementPort | PortFlowRole::FieldSidePort
        )
}

fn is_physical_output_port(port: &SignalPortDefinition) -> bool {
    !is_communication_port(port)
        && matches!(
            port.directionality,
            PortDirectionality::Output | PortDirectionality::Bidirectional
        )
        && matches!(
            port.flow_role,
            PortFlowRole::SourcePort
                | PortFlowRole::TransducerOutputPort
                | PortFlowRole::FieldSidePort
        )
}

fn validate_actions(
    issues: &mut Vec<DefinitionValidationIssue>,
    actions: &[DriverActionDefinition],
    model_interface_ids: &BTreeSet<String>,
    model_capability_ids: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for (index, action) in actions.iter().enumerate() {
        let path = format!("actions[{index}]");
        require_token(issues, &action.action_id, &format!("{path}.action_id"));
        require_text(issues, &action.label, &format!("{path}.label"));
        require_text(issues, &action.description, &format!("{path}.description"));
        require_token(
            issues,
            &action.implements_capability_id,
            &format!("{path}.implements_capability_id"),
        );
        if !ids.insert(action.action_id.clone()) {
            issues.push(issue(
                "error",
                "duplicate_driver_action_id",
                format!("{path}.action_id"),
                format!("duplicate driver action id: {}", action.action_id),
                Option::<String>::None,
            ));
        }
        if !model_capability_ids.is_empty()
            && !model_capability_ids.contains(&action.implements_capability_id)
        {
            issues.push(issue(
                "error",
                "unknown_implemented_capability",
                format!("{path}.implements_capability_id"),
                format!(
                    "action implements capability absent from model: {}",
                    action.implements_capability_id
                ),
                Some("Reference a capability_id from the supported model revision."),
            ));
        }
        if action.default_timeout_ms == 0 {
            issues.push(issue(
                "error",
                "invalid_action_timeout",
                format!("{path}.default_timeout_ms"),
                "default_timeout_ms must be greater than zero",
                Some("Use a bounded timeout in milliseconds."),
            ));
        }
        validate_action_values(issues, &action.inputs, &format!("{path}.inputs"));
        validate_action_values(issues, &action.outputs, &format!("{path}.outputs"));
        let mut variables = BTreeSet::new();
        for input in &action.inputs {
            variables.insert(format!("input.{}", input.name));
        }
        validate_script_steps(
            issues,
            &action.script.steps,
            &format!("{path}.script.steps"),
            model_interface_ids,
            &ids,
            &mut variables,
        );
        for output in &action.outputs {
            if !variables.contains(&format!("result.{}", output.name)) {
                issues.push(issue(
                    "error",
                    "declared_output_not_produced",
                    format!("{path}.outputs.{}", output.name),
                    format!("declared output is never produced: {}", output.name),
                    Some("Bind a response, calculate, or set result.<output_name>."),
                ));
            }
        }
    }
    for (index, action) in actions.iter().enumerate() {
        for (field, value) in [
            ("rollback_action_id", action.rollback_action_id.as_deref()),
            (
                "safe_state_action_id",
                action.safe_state_action_id.as_deref(),
            ),
        ] {
            if let Some(action_id) = value {
                if !ids.contains(action_id) {
                    issues.push(issue(
                        "error",
                        "unknown_driver_action_reference",
                        format!("actions[{index}].{field}"),
                        format!("{field} references unknown action: {action_id}"),
                        Some("Use an action_id declared in this driver profile."),
                    ));
                }
            }
        }
    }
    ids
}

fn validate_action_values(
    issues: &mut Vec<DefinitionValidationIssue>,
    values: &[ActionValueDefinition],
    path: &str,
) {
    let mut names = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        require_token(issues, &value.name, &format!("{item_path}.name"));
        if !names.insert(value.name.clone()) {
            issues.push(issue(
                "error",
                "duplicate_value_name",
                format!("{item_path}.name"),
                format!("duplicate input/output name: {}", value.name),
                Option::<String>::None,
            ));
        }
        validate_quantity_unit(
            issues,
            value.quantity,
            &value.unit,
            &format!("{item_path}.unit"),
        );
        validate_bounds(
            issues,
            value.minimum,
            value.maximum,
            &format!("{item_path}.minimum"),
            &format!("{item_path}.maximum"),
        );
        if value.value_type == ValueType::Boolean && value.quantity != PhysicalQuantity::Boolean {
            issues.push(issue(
                "warning",
                "boolean_type_quantity_mismatch",
                format!("{item_path}.quantity"),
                "boolean values should normally use quantity=boolean",
                Some("Set quantity to boolean unless a method-specific encoding is intended."),
            ));
        }
    }
}

fn validate_script_steps(
    issues: &mut Vec<DefinitionValidationIssue>,
    steps: &[DriverScriptStep],
    path: &str,
    model_interface_ids: &BTreeSet<String>,
    current_action_ids: &BTreeSet<String>,
    variables: &mut BTreeSet<String>,
) {
    let mut ids = BTreeSet::new();
    for (index, step) in steps.iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        require_token(issues, &step.step_id, &format!("{item_path}.step_id"));
        if !ids.insert(step.step_id.clone()) {
            issues.push(issue(
                "error",
                "duplicate_script_step_id",
                format!("{item_path}.step_id"),
                format!("duplicate script step id: {}", step.step_id),
                Option::<String>::None,
            ));
        }
        if !step.enabled {
            continue;
        }
        validate_step_required_fields(issues, step, &item_path, model_interface_ids);
        validate_expression_variables(issues, step.expression.as_deref(), &item_path, variables);
        if let Some(binding) = step.response_binding.as_deref() {
            require_variable_binding(issues, binding, &format!("{item_path}.response_binding"));
            variables.insert(
                binding
                    .trim_start_matches("${")
                    .trim_end_matches('}')
                    .to_owned(),
            );
        }
        if let Some(variable) = step.variable.as_deref() {
            require_variable_binding(issues, variable, &format!("{item_path}.variable"));
            variables.insert(
                variable
                    .trim_start_matches("${")
                    .trim_end_matches('}')
                    .to_owned(),
            );
        }
        if matches!(
            step.step_type,
            ScriptStepType::LoopUntil | ScriptStepType::Repeat
        ) {
            let bounded = step.max_iterations.unwrap_or(0) > 0 || step.timeout_ms.unwrap_or(0) > 0;
            if !bounded {
                issues.push(issue(
                    "error",
                    "unbounded_loop",
                    item_path.clone(),
                    "loop steps must declare max_iterations and/or timeout_ms",
                    Some("Set max_iterations or timeout_ms so simulation and runtime can stop."),
                ));
            }
        }
        if matches!(step.step_type, ScriptStepType::CallAction) {
            match step.action_id.as_deref() {
                Some(action_id) if current_action_ids.contains(action_id) => {}
                Some(action_id) => issues.push(issue(
                    "error",
                    "unknown_called_action",
                    format!("{item_path}.action_id"),
                    format!("called action does not exist: {action_id}"),
                    Some("Declare the action before approval."),
                )),
                None => issues.push(issue(
                    "error",
                    "missing_called_action",
                    format!("{item_path}.action_id"),
                    "call_action requires action_id",
                    Option::<String>::None,
                )),
            }
        }
        validate_script_steps(
            issues,
            &step.steps,
            &format!("{item_path}.steps"),
            model_interface_ids,
            current_action_ids,
            variables,
        );
        validate_script_steps(
            issues,
            &step.else_steps,
            &format!("{item_path}.else_steps"),
            model_interface_ids,
            current_action_ids,
            variables,
        );
    }
}

fn validate_step_required_fields(
    issues: &mut Vec<DefinitionValidationIssue>,
    step: &DriverScriptStep,
    path: &str,
    model_interface_ids: &BTreeSet<String>,
) {
    let uses_interface = matches!(
        step.step_type,
        ScriptStepType::IoWrite
            | ScriptStepType::IoRead
            | ScriptStepType::IoQuery
            | ScriptStepType::CanBusSend
            | ScriptStepType::CanBusReceive
            | ScriptStepType::CanBusRequestResponse
    );
    if uses_interface {
        match step.interface_id.as_deref() {
            Some(interface_id)
                if model_interface_ids.is_empty() || model_interface_ids.contains(interface_id) => {
            }
            Some(interface_id) => issues.push(issue(
                "error",
                "unknown_script_interface",
                format!("{path}.interface_id"),
                format!("script step references unknown interface: {interface_id}"),
                Some("Use an interface_id from the approved equipment model."),
            )),
            None => issues.push(issue(
                "error",
                "missing_script_interface",
                format!("{path}.interface_id"),
                "I/O steps require interface_id",
                Option::<String>::None,
            )),
        }
    }
    if matches!(
        step.step_type,
        ScriptStepType::IoWrite | ScriptStepType::IoQuery
    ) && step
        .payload
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        issues.push(issue(
            "error",
            "missing_io_payload",
            format!("{path}.payload"),
            "io_write and io_query steps require payload",
            Option::<String>::None,
        ));
    }
    if matches!(
        step.step_type,
        ScriptStepType::CanBusSend | ScriptStepType::CanBusRequestResponse
    ) && step.frame.is_none()
    {
        issues.push(issue(
            "error",
            "missing_can_bus_frame",
            format!("{path}.frame"),
            "CAN bus send/request steps require a structured frame",
            Option::<String>::None,
        ));
    }
    if matches!(
        step.step_type,
        ScriptStepType::Assert | ScriptStepType::If | ScriptStepType::WaitUntil
    ) && step
        .expression
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        issues.push(issue(
            "error",
            "missing_expression",
            format!("{path}.expression"),
            "conditional/assertion steps require expression",
            Option::<String>::None,
        ));
    }
    if matches!(step.step_type, ScriptStepType::Delay) && step.duration_ms.is_none() {
        issues.push(issue(
            "error",
            "missing_delay_duration",
            format!("{path}.duration_ms"),
            "delay steps require duration_ms",
            Option::<String>::None,
        ));
    }
}

fn validate_interface_compatibility(
    issues: &mut Vec<DefinitionValidationIssue>,
    interface: &CommunicationInterfaceDefinition,
    path: &str,
) {
    let compatible = match interface.access_provider_kind {
        AccessProviderKind::NativeSerial => interface.transport_kind == TransportKind::Serial,
        AccessProviderKind::NativeTcp => interface.transport_kind == TransportKind::EthernetTcp,
        AccessProviderKind::NativeUdp => interface.transport_kind == TransportKind::EthernetUdp,
        AccessProviderKind::Socketcan
        | AccessProviderKind::Pcan
        | AccessProviderKind::VectorCan => interface.transport_kind == TransportKind::CanBus,
        AccessProviderKind::Usbtmc | AccessProviderKind::Hid => {
            interface.transport_kind == TransportKind::Usb
        }
        AccessProviderKind::Visa => matches!(
            interface.transport_kind,
            TransportKind::Gpib
                | TransportKind::Usb
                | TransportKind::EthernetTcp
                | TransportKind::Serial
        ),
        AccessProviderKind::Simulation | AccessProviderKind::CustomAdapter => true,
    };
    if !compatible {
        issues.push(issue(
            "error",
            "transport_access_provider_mismatch",
            format!("{path}.access_provider_kind"),
            "access provider is incompatible with the declared physical transport",
            Some("Treat VISA as an access layer and choose a compatible transport."),
        ));
    }
    if interface.protocol_kind == ProtocolKind::CanBusFrames
        && interface.transport_kind != TransportKind::CanBus
    {
        issues.push(issue(
            "error",
            "can_bus_protocol_requires_can_bus_transport",
            format!("{path}.protocol_kind"),
            "CAN bus frame protocol requires transport_kind=can_bus",
            Option::<String>::None,
        ));
    }
    if interface.protocol_kind == ProtocolKind::Manual
        && interface.transport_kind != TransportKind::None
    {
        issues.push(issue(
            "warning",
            "manual_protocol_with_transport",
            format!("{path}.protocol_kind"),
            "manual protocol normally uses transport_kind=none",
            Option::<String>::None,
        ));
    }
}

fn validate_quantity_unit(
    issues: &mut Vec<DefinitionValidationIssue>,
    quantity: PhysicalQuantity,
    unit: &str,
    path: &str,
) {
    let unit = unit.trim();
    if unit.is_empty() {
        issues.push(issue(
            "error",
            "missing_unit",
            path,
            "unit is required",
            Some("Use an explicit unit such as Hz, V, dBm, or dimensionless."),
        ));
        return;
    }
    if matches!(
        quantity,
        PhysicalQuantity::Text | PhysicalQuantity::Boolean | PhysicalQuantity::Binary
    ) && unit == "dimensionless"
    {
        return;
    }
    match unit_quantity(unit) {
        Some(unit_family) if unit_family == quantity => {}
        Some(PhysicalQuantity::Power)
            if quantity == PhysicalQuantity::Voltage && matches!(unit, "dBuV" | "dBuV_per_m") =>
        {
            issues.push(issue(
                "error",
                "quantity_unit_mismatch",
                path,
                format!("unit {unit} is not compatible with quantity voltage"),
                Some("Use V, mV, or uV for voltage; use dBuV for logarithmic EMC levels."),
            ));
        }
        Some(unit_family) => issues.push(issue(
            "error",
            "quantity_unit_mismatch",
            path,
            format!("unit {unit} belongs to {unit_family:?}, not {quantity:?}"),
            Some("Choose a unit from the same physical quantity family."),
        )),
        None => issues.push(issue(
            "error",
            "unknown_unit",
            path,
            format!("unit is not in the registry: {unit}"),
            Some("Extend the unit registry deliberately before using a new unit."),
        )),
    }
}

pub fn unit_quantity(unit: &str) -> Option<PhysicalQuantity> {
    Some(match unit {
        "Hz" | "kHz" | "MHz" | "GHz" => PhysicalQuantity::Frequency,
        "s" | "ms" | "us" | "ns" => PhysicalQuantity::Time,
        "V" | "mV" | "uV" => PhysicalQuantity::Voltage,
        "A" | "mA" | "uA" => PhysicalQuantity::Current,
        "W" | "mW" | "dBm" => PhysicalQuantity::Power,
        "dBuV_per_m" => PhysicalQuantity::ElectricField,
        "dB_per_m" => PhysicalQuantity::MagneticField,
        "dBuV" => PhysicalQuantity::Voltage,
        "dB" | "percent" | "dimensionless" => PhysicalQuantity::Dimensionless,
        "ohm" => PhysicalQuantity::Resistance,
        "m" | "cm" | "mm" => PhysicalQuantity::Distance,
        "deg" | "rad" => PhysicalQuantity::Angle,
        "Celsius" => PhysicalQuantity::Temperature,
        "Pa" => PhysicalQuantity::Pressure,
        _ => return None,
    })
}

pub fn convert_prefixed_value(value: f64, from_unit: &str, to_unit: &str) -> Option<f64> {
    if unit_quantity(from_unit)? != unit_quantity(to_unit)? {
        return None;
    }
    if is_logarithmic_unit(from_unit) || is_logarithmic_unit(to_unit) {
        return (from_unit == to_unit).then_some(value);
    }
    Some(value * unit_scale(from_unit)? / unit_scale(to_unit)?)
}

pub fn is_logarithmic_unit(unit: &str) -> bool {
    matches!(unit, "dBm" | "dBuV" | "dBuV_per_m" | "dB" | "dB_per_m")
}

fn unit_scale(unit: &str) -> Option<f64> {
    Some(match unit {
        "GHz" => 1_000_000_000.0,
        "MHz" => 1_000_000.0,
        "kHz" => 1_000.0,
        "Hz" | "s" | "V" | "A" | "W" | "ohm" | "m" | "rad" | "Celsius" | "Pa" | "dimensionless" => {
            1.0
        }
        "ms" | "mV" | "mA" | "mW" | "mm" => 0.001,
        "us" | "uV" | "uA" => 0.000_001,
        "ns" => 0.000_000_001,
        "cm" => 0.01,
        "deg" => std::f64::consts::PI / 180.0,
        "percent" => 0.01,
        _ => return None,
    })
}

fn validate_bounds(
    issues: &mut Vec<DefinitionValidationIssue>,
    minimum: Option<f64>,
    maximum: Option<f64>,
    minimum_path: &str,
    maximum_path: &str,
) {
    if let (Some(minimum), Some(maximum)) = (minimum, maximum) {
        if minimum > maximum {
            issues.push(issue(
                "error",
                "invalid_numeric_bounds",
                minimum_path,
                format!("minimum {minimum} is greater than maximum {maximum}"),
                Some(&format!("Adjust {minimum_path} or {maximum_path}.")),
            ));
        }
    }
}

fn validate_expression_variables(
    issues: &mut Vec<DefinitionValidationIssue>,
    expression: Option<&str>,
    path: &str,
    variables: &BTreeSet<String>,
) {
    let Some(expression) = expression else {
        return;
    };
    for reference in variable_references(expression) {
        let reference = reference.trim_start_matches("${").trim_end_matches('}');
        if !variables.contains(reference) {
            issues.push(issue(
                "error",
                "unknown_variable_reference",
                format!("{path}.expression"),
                format!("expression references unknown variable: {reference}"),
                Some("Use ${input.name}, ${state.name}, ${result.name}, or assign it before use."),
            ));
        }
    }
    if expression.contains("eval(") || expression.contains("system(") {
        issues.push(issue(
            "error",
            "unsafe_expression",
            format!("{path}.expression"),
            "expressions cannot call arbitrary code",
            Some("Use the limited expression DSL."),
        ));
    }
}

fn require_variable_binding(issues: &mut Vec<DefinitionValidationIssue>, value: &str, path: &str) {
    let trimmed = value.trim();
    if !(trimmed.starts_with("${") && trimmed.ends_with('}')) {
        issues.push(issue(
            "error",
            "invalid_variable_binding",
            path,
            format!("variable binding must use ${{scope.name}} syntax: {trimmed}"),
            Some("Example: ${state.answer} or ${result.forward_power_dbm}."),
        ));
    }
}

fn variable_references(expression: &str) -> Vec<String> {
    let mut references = Vec::new();
    let mut rest = expression;
    while let Some(start) = rest.find("${") {
        let after_start = &rest[start..];
        if let Some(end) = after_start.find('}') {
            references.push(after_start[..=end].to_owned());
            rest = &after_start[end + 1..];
        } else {
            break;
        }
    }
    references
}

fn require_text(issues: &mut Vec<DefinitionValidationIssue>, value: &str, path: &str) {
    if value.trim().is_empty() {
        issues.push(issue(
            "error",
            "missing_text",
            path,
            "text value cannot be empty",
            Option::<String>::None,
        ));
    }
}

fn require_token(issues: &mut Vec<DefinitionValidationIssue>, value: &str, path: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        issues.push(issue(
            "error",
            "invalid_identifier",
            path,
            format!("identifier contains unsupported characters: {value}"),
            Some("Use letters, digits, '-', '_', '.', or ':'."),
        ));
    }
}

fn require_checksum(issues: &mut Vec<DefinitionValidationIssue>, value: &str, path: &str) {
    let valid = value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].chars().all(|ch| ch.is_ascii_hexdigit());
    if !valid {
        issues.push(issue(
            "error",
            "invalid_checksum",
            path,
            "checksum must be sha256:<64 hex characters>",
            Option::<String>::None,
        ));
    }
}

fn issue(
    severity: impl Into<String>,
    code: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
    suggestion: Option<impl Into<String>>,
) -> DefinitionValidationIssue {
    DefinitionValidationIssue {
        severity: severity.into(),
        code: code.into(),
        path: path.into(),
        message: message.into(),
        suggestion: suggestion.map(Into::into),
    }
}

fn default_required() -> bool {
    true
}

fn interface_ids_for(model: &EquipmentModelDefinition) -> BTreeSet<String> {
    model
        .communication_interfaces
        .iter()
        .map(|interface| interface.interface_id.clone())
        .collect()
}

fn capability_ids_for(model: &EquipmentModelDefinition) -> BTreeSet<String> {
    model
        .capabilities
        .iter()
        .map(|capability| capability.capability_id.clone())
        .collect()
}

fn canonicalize_json_value(value: &mut Value) {
    match value {
        Value::Object(object) => {
            let mut sorted = BTreeMap::new();
            for (key, mut value) in std::mem::take(object) {
                canonicalize_json_value(&mut value);
                sorted.insert(key, value);
            }
            *object = sorted.into_iter().collect::<Map<String, Value>>();
        }
        Value::Array(items) => {
            for item in items {
                canonicalize_json_value(item);
            }
        }
        _ => {}
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriverSimulationScenario {
    pub scenario_id: String,
    pub driver_revision_id: String,
    pub action_id: String,
    #[serde(default)]
    pub input_values: BTreeMap<String, Value>,
    #[serde(default)]
    pub simulated_responses: Vec<Value>,
    #[serde(default)]
    pub operator_confirmations: BTreeMap<String, bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriverSimulationTraceEntry {
    pub step_index: usize,
    pub step_id: String,
    pub step_type: String,
    pub timestamp_virtual_ms: u64,
    pub operation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,
    #[serde(default)]
    pub variable_changes: BTreeMap<String, Value>,
    pub assertion_result: Option<bool>,
    pub duration_virtual_ms: u64,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriverSimulationResult {
    pub scenario_id: String,
    pub driver_revision_id: String,
    pub action_id: String,
    pub status: String,
    pub trace: Vec<DriverSimulationTraceEntry>,
    pub outputs: BTreeMap<String, Value>,
    pub final_state: BTreeMap<String, Value>,
}

pub fn simulate_driver_action(
    definition: &DriverProfileDefinition,
    scenario: &DriverSimulationScenario,
) -> Result<DriverSimulationResult, DefinitionValidationIssue> {
    let action = definition
        .actions
        .iter()
        .find(|action| action.action_id == scenario.action_id)
        .ok_or_else(|| {
            issue(
                "error",
                "simulation_action_not_found",
                "action_id",
                format!("action does not exist: {}", scenario.action_id),
                None::<String>,
            )
        })?;
    let mut engine = SimulationEngine {
        scenario,
        virtual_time_ms: 0,
        response_index: 0,
        step_index: 0,
        state: BTreeMap::new(),
        trace: Vec::new(),
    };
    for (key, value) in &scenario.input_values {
        engine.state.insert(format!("input.{key}"), value.clone());
    }
    engine.run_steps(&action.script.steps)?;
    let mut outputs = BTreeMap::new();
    for output in &action.outputs {
        let key = format!("result.{}", output.name);
        if let Some(value) = engine.state.get(&key) {
            outputs.insert(output.name.clone(), value.clone());
        }
    }
    Ok(DriverSimulationResult {
        scenario_id: scenario.scenario_id.clone(),
        driver_revision_id: scenario.driver_revision_id.clone(),
        action_id: scenario.action_id.clone(),
        status: "passed".to_owned(),
        trace: engine.trace,
        outputs,
        final_state: engine.state,
    })
}

struct SimulationEngine<'a> {
    scenario: &'a DriverSimulationScenario,
    virtual_time_ms: u64,
    response_index: usize,
    step_index: usize,
    state: BTreeMap<String, Value>,
    trace: Vec<DriverSimulationTraceEntry>,
}

struct TraceRecord {
    operation: &'static str,
    request: Option<Value>,
    response: Option<Value>,
    variable_changes: BTreeMap<String, Value>,
    assertion_result: Option<bool>,
    duration_virtual_ms: u64,
    status: &'static str,
}

impl TraceRecord {
    fn ok(operation: &'static str) -> Self {
        Self {
            operation,
            request: None,
            response: None,
            variable_changes: BTreeMap::new(),
            assertion_result: None,
            duration_virtual_ms: 0,
            status: "ok",
        }
    }
}

impl SimulationEngine<'_> {
    fn run_steps(&mut self, steps: &[DriverScriptStep]) -> Result<(), DefinitionValidationIssue> {
        for step in steps {
            if !step.enabled {
                continue;
            }
            self.run_step(step)?;
        }
        Ok(())
    }

    fn run_step(&mut self, step: &DriverScriptStep) -> Result<(), DefinitionValidationIssue> {
        match step.step_type {
            ScriptStepType::IoWrite => {
                self.record_io(step, "write", step.payload.clone().map(Value::String), None)
            }
            ScriptStepType::IoRead => {
                let response = self.next_response(step)?;
                self.bind_response(step, response.clone());
                self.record_io(step, "read", None, Some(response));
            }
            ScriptStepType::IoQuery => {
                let response = self.next_response(step)?;
                self.bind_response(step, response.clone());
                self.record_io(
                    step,
                    "query",
                    step.payload.clone().map(Value::String),
                    Some(response),
                );
            }
            ScriptStepType::CanBusSend => self.record_io(
                step,
                "can_bus_send",
                step.frame
                    .as_ref()
                    .and_then(|frame| serde_json::to_value(frame).ok()),
                None,
            ),
            ScriptStepType::CanBusReceive => {
                let response = self.next_response(step)?;
                self.bind_response(step, response.clone());
                self.record_io(step, "can_bus_receive", None, Some(response));
            }
            ScriptStepType::CanBusRequestResponse => {
                let response = self.next_response(step)?;
                self.bind_response(step, response.clone());
                self.record_io(
                    step,
                    "can_bus_request_response",
                    step.frame
                        .as_ref()
                        .and_then(|frame| serde_json::to_value(frame).ok()),
                    Some(response),
                );
            }
            ScriptStepType::SetVariable
            | ScriptStepType::ParseNumber
            | ScriptStepType::ParseText
            | ScriptStepType::ParseCsv
            | ScriptStepType::ParseRegex
            | ScriptStepType::ConvertUnit
            | ScriptStepType::Calculate => {
                let variable = step
                    .variable
                    .as_deref()
                    .unwrap_or("${state.value}")
                    .trim_start_matches("${")
                    .trim_end_matches('}')
                    .to_owned();
                let value = step.value.clone().unwrap_or(Value::Null);
                self.state.insert(variable.clone(), value.clone());
                let mut changes = BTreeMap::new();
                changes.insert(variable, value);
                self.record(
                    step,
                    TraceRecord {
                        variable_changes: changes,
                        ..TraceRecord::ok("assign")
                    },
                );
            }
            ScriptStepType::Assert => {
                let result =
                    evaluate_expression(step.expression.as_deref().unwrap_or(""), &self.state);
                self.record(
                    step,
                    TraceRecord {
                        assertion_result: Some(result),
                        status: if result { "ok" } else { "failed" },
                        ..TraceRecord::ok("assert")
                    },
                );
                if !result {
                    return Err(issue(
                        "error",
                        "simulation_assertion_failed",
                        step.step_id.clone(),
                        "assertion evaluated to false",
                        None::<String>,
                    ));
                }
            }
            ScriptStepType::If => {
                let result =
                    evaluate_expression(step.expression.as_deref().unwrap_or(""), &self.state);
                self.record(
                    step,
                    TraceRecord {
                        assertion_result: Some(result),
                        ..TraceRecord::ok("if")
                    },
                );
                if result {
                    self.run_steps(&step.steps)?;
                } else {
                    self.run_steps(&step.else_steps)?;
                }
            }
            ScriptStepType::LoopUntil | ScriptStepType::Repeat => {
                let iterations = step.max_iterations.unwrap_or(1).max(1);
                for _ in 0..iterations {
                    self.run_steps(&step.steps)?;
                    if step.step_type == ScriptStepType::LoopUntil
                        && evaluate_expression(
                            step.expression.as_deref().unwrap_or(""),
                            &self.state,
                        )
                    {
                        break;
                    }
                }
                self.record(step, TraceRecord::ok("loop"));
            }
            ScriptStepType::OperatorMessage
            | ScriptStepType::OperatorConfirmation
            | ScriptStepType::OperatorInput => {
                self.record(
                    step,
                    TraceRecord {
                        request: step.message.clone().map(Value::String),
                        ..TraceRecord::ok("operator")
                    },
                );
            }
            ScriptStepType::Delay | ScriptStepType::WaitUntil => {
                let duration = step.duration_ms.or(step.timeout_ms).unwrap_or(0);
                self.virtual_time_ms += duration;
                self.record(
                    step,
                    TraceRecord {
                        duration_virtual_ms: duration,
                        ..TraceRecord::ok("delay")
                    },
                );
            }
            ScriptStepType::CallAction
            | ScriptStepType::Return
            | ScriptStepType::CallRegisteredAdapter => {
                self.record(step, TraceRecord::ok("control"));
            }
        }
        Ok(())
    }

    fn next_response(
        &mut self,
        step: &DriverScriptStep,
    ) -> Result<Value, DefinitionValidationIssue> {
        let response = self
            .scenario
            .simulated_responses
            .get(self.response_index)
            .cloned()
            .ok_or_else(|| {
                issue(
                    "error",
                    "simulation_response_missing",
                    step.step_id.clone(),
                    "no simulated response is available for read/query step",
                    Some("Add a simulated response to the scenario."),
                )
            })?;
        self.response_index += 1;
        Ok(response)
    }

    fn bind_response(&mut self, step: &DriverScriptStep, response: Value) {
        if let Some(binding) = step.response_binding.as_deref() {
            let key = binding
                .trim_start_matches("${")
                .trim_end_matches('}')
                .to_owned();
            self.state.insert(key, response);
        }
    }

    fn record_io(
        &mut self,
        step: &DriverScriptStep,
        operation: &str,
        request: Option<Value>,
        response: Option<Value>,
    ) {
        self.record(
            step,
            TraceRecord {
                operation: match operation {
                    "query" => "query",
                    "read" => "read",
                    "write" => "write",
                    "can_bus_send" => "can_bus_send",
                    "can_bus_receive" => "can_bus_receive",
                    "can_bus_request_response" => "can_bus_request_response",
                    _ => "io",
                },
                request,
                response,
                duration_virtual_ms: step.timeout_ms.unwrap_or(0),
                ..TraceRecord::ok("io")
            },
        );
    }

    fn record(&mut self, step: &DriverScriptStep, record: TraceRecord) {
        self.step_index += 1;
        self.trace.push(DriverSimulationTraceEntry {
            step_index: self.step_index,
            step_id: step.step_id.clone(),
            step_type: format!("{:?}", step.step_type).to_lowercase(),
            timestamp_virtual_ms: self.virtual_time_ms,
            operation: record.operation.to_owned(),
            request: record.request,
            response: record.response,
            variable_changes: record.variable_changes,
            assertion_result: record.assertion_result,
            duration_virtual_ms: record.duration_virtual_ms,
            status: record.status.to_owned(),
        });
    }
}

fn evaluate_expression(expression: &str, state: &BTreeMap<String, Value>) -> bool {
    let expression = expression.trim();
    if expression.is_empty() {
        return true;
    }
    if let Some((left, right)) = expression.split_once("!=") {
        return value_for_expression(left.trim(), state)
            != value_for_expression(right.trim(), state);
    }
    if let Some((left, right)) = expression.split_once("==") {
        return value_for_expression(left.trim(), state)
            == value_for_expression(right.trim(), state);
    }
    if let Some((left, right)) = expression.split_once(">") {
        return numeric_expression(left.trim(), state) > numeric_expression(right.trim(), state);
    }
    if let Some(reference) = expression.strip_prefix("not ") {
        return !truthy(&value_for_expression(reference.trim(), state));
    }
    truthy(&value_for_expression(expression, state))
}

fn value_for_expression(token: &str, state: &BTreeMap<String, Value>) -> Value {
    let trimmed = token.trim().trim_matches('"');
    if trimmed.starts_with("${") && trimmed.ends_with('}') {
        return state
            .get(trimmed.trim_start_matches("${").trim_end_matches('}'))
            .cloned()
            .unwrap_or(Value::Null);
    }
    if let Ok(value) = trimmed.parse::<f64>() {
        return Value::from(value);
    }
    Value::String(trimmed.to_owned())
}

fn numeric_expression(token: &str, state: &BTreeMap<String, Value>) -> f64 {
    match value_for_expression(token, state) {
        Value::Number(number) => number.as_f64().unwrap_or(0.0),
        Value::String(text) => text.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Bool(value) => *value,
        Value::Number(number) => number.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(text) => !text.is_empty() && text != "0" && text != "false",
        Value::Array(items) => !items.is_empty(),
        Value::Object(items) => !items.is_empty(),
        Value::Null => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_quantity_unit_compatibility() {
        let mut definition = minimal_model();
        definition.specifications.push(EngineeringSpecification {
            specification_id: "frequency_range".to_owned(),
            label: "Frequency range".to_owned(),
            quantity: PhysicalQuantity::Frequency,
            unit: "V".to_owned(),
            minimum: Some(1.0),
            maximum: Some(2.0),
            nominal: None,
            resolution: None,
            conditions: Vec::new(),
            comment: None,
        });

        let issues = definition.validate_all();
        assert!(issues
            .iter()
            .any(|issue| issue.code == "quantity_unit_mismatch"));
    }

    #[test]
    fn accepts_dimensionless_unit_for_textual_values() {
        let mut driver = minimal_driver();
        driver.actions[0].outputs[0] = ActionValueDefinition {
            name: "error_text".to_owned(),
            value_type: ValueType::Text,
            quantity: PhysicalQuantity::Text,
            unit: "dimensionless".to_owned(),
            required: true,
            default_value: None,
            minimum: None,
            maximum: None,
            enum_values: Vec::new(),
            description: None,
        };
        driver.actions[0].script.steps[1].response_binding =
            Some("${result.error_text}".to_owned());

        let issues = driver.validate_all(Some(&minimal_model()));

        assert!(!issues
            .iter()
            .any(|issue| issue.code == "quantity_unit_mismatch"));
        assert!(!issues.iter().any(|issue| issue.code == "unknown_unit"));
    }

    #[test]
    fn canonical_checksum_is_deterministic() {
        let left = minimal_model().canonicalize().unwrap();
        let right = minimal_model().canonicalize().unwrap();

        assert_eq!(left.canonical_json, right.canonical_json);
        assert_eq!(left.definition_checksum, right.definition_checksum);
    }

    #[test]
    fn accepts_valid_rf_cable_preset_topology() {
        let definition = rf_cable_model();

        assert_no_validation_errors(&definition);
    }

    #[test]
    fn accepts_valid_rf_load_preset_topology() {
        let mut definition = rf_cable_model();
        definition.category_code = "rf_load".to_owned();
        definition.signal_ports = vec![rf_port(
            "rf_in",
            "RF input",
            PortDirectionality::Input,
            PortFlowRole::SinkPort,
        )];

        assert_no_validation_errors(&definition);
    }

    #[test]
    fn accepts_valid_receiving_antenna_preset_topology() {
        let definition = receiving_antenna_model();

        assert_no_validation_errors(&definition);
    }

    #[test]
    fn accepts_valid_transmitting_antenna_preset_topology() {
        let mut definition = receiving_antenna_model();
        definition.model_name = "Transmit antenna".to_owned();
        definition.functional_role = FunctionalRole::Actuator;
        definition.category_code = "transmitting_antenna".to_owned();
        definition.signal_ports = vec![
            rf_port(
                "rf_in",
                "RF input",
                PortDirectionality::Input,
                PortFlowRole::SinkPort,
            ),
            SignalPortDefinition {
                port_id: "field".to_owned(),
                label: "Radiated field".to_owned(),
                directionality: PortDirectionality::Output,
                flow_role: PortFlowRole::FieldSidePort,
                signal_domain: SignalDomain::Environmental,
                required: true,
                connector_type: None,
                technology_tags: Vec::new(),
                quantity: PhysicalQuantity::ElectricField,
                unit: "dBuV_per_m".to_owned(),
                impedance: None,
                frequency_min: None,
                frequency_max: None,
                voltage_max: None,
                current_max: None,
                power_max: None,
                channel_index: None,
                differential: false,
                isolated: false,
                comment: None,
            },
        ];

        assert_no_validation_errors(&definition);
    }

    #[test]
    fn accepts_valid_oscilloscope_preset_topology() {
        let mut definition = minimal_model();
        definition.manufacturer = "Demo".to_owned();
        definition.model_name = "Scope".to_owned();
        definition.category_code = "oscilloscope".to_owned();
        definition.signal_domains = vec![
            SignalDomain::AnalogVoltage,
            SignalDomain::Trigger,
            SignalDomain::Ethernet,
        ];
        definition.technology_tags = vec![
            TechnologyTag::VoltageInput,
            TechnologyTag::Trigger,
            TechnologyTag::Ethernet,
        ];
        definition.signal_ports = vec![
            SignalPortDefinition {
                port_id: "ch1".to_owned(),
                label: "CH1".to_owned(),
                directionality: PortDirectionality::Input,
                flow_role: PortFlowRole::MeasurementPort,
                signal_domain: SignalDomain::AnalogVoltage,
                required: true,
                connector_type: Some("BNC".to_owned()),
                technology_tags: vec![TechnologyTag::VoltageInput],
                quantity: PhysicalQuantity::Voltage,
                unit: "V".to_owned(),
                impedance: None,
                frequency_min: None,
                frequency_max: None,
                voltage_max: Some(400.0),
                current_max: None,
                power_max: None,
                channel_index: Some(1),
                differential: false,
                isolated: false,
                comment: None,
            },
            SignalPortDefinition {
                port_id: "trig_in".to_owned(),
                label: "Trigger input".to_owned(),
                directionality: PortDirectionality::Input,
                flow_role: PortFlowRole::ControlPort,
                signal_domain: SignalDomain::Trigger,
                required: false,
                connector_type: Some("BNC".to_owned()),
                technology_tags: vec![TechnologyTag::Trigger],
                quantity: PhysicalQuantity::Voltage,
                unit: "V".to_owned(),
                impedance: None,
                frequency_min: None,
                frequency_max: None,
                voltage_max: None,
                current_max: None,
                power_max: None,
                channel_index: None,
                differential: false,
                isolated: false,
                comment: None,
            },
            SignalPortDefinition {
                port_id: "lan".to_owned(),
                label: "LAN".to_owned(),
                directionality: PortDirectionality::Communication,
                flow_role: PortFlowRole::CommunicationPort,
                signal_domain: SignalDomain::Ethernet,
                required: false,
                connector_type: Some("RJ45".to_owned()),
                technology_tags: vec![TechnologyTag::Ethernet],
                quantity: PhysicalQuantity::Binary,
                unit: "dimensionless".to_owned(),
                impedance: None,
                frequency_min: None,
                frequency_max: None,
                voltage_max: None,
                current_max: None,
                power_max: None,
                channel_index: None,
                differential: false,
                isolated: false,
                comment: None,
            },
        ];
        definition.communication_interfaces.clear();
        definition.capabilities.clear();

        assert_no_validation_errors(&definition);
    }

    #[test]
    fn accepts_valid_adc_converter_without_can_bus() {
        let definition = adc_converter_model();

        assert_no_validation_errors(&definition);
        assert!(!definition.signal_domains.contains(&SignalDomain::CanBus));
        assert!(!definition.technology_tags.contains(&TechnologyTag::CanBus));
        assert!(!definition
            .signal_ports
            .iter()
            .any(|port| port.signal_domain == SignalDomain::CanBus));
    }

    #[test]
    fn rejects_can_bus_device_without_explicit_can_bus_domain() {
        let mut definition = minimal_model();
        definition.category_code = "can_bus".to_owned();
        definition.technology_tags.push(TechnologyTag::CanBus);

        let issues = definition.validate_all();

        assert!(issues
            .iter()
            .any(|issue| issue.code == "can_bus_category_requires_can_bus_classification"));
    }

    #[test]
    fn rejects_can_bus_frames_interface_without_can_bus_port() {
        let mut definition = minimal_model();
        definition.communication_interfaces[0].transport_kind = TransportKind::CanBus;
        definition.communication_interfaces[0].access_provider_kind =
            AccessProviderKind::Simulation;
        definition.communication_interfaces[0].protocol_kind = ProtocolKind::CanBusFrames;

        let issues = definition.validate_all();

        assert!(issues
            .iter()
            .any(|issue| issue.code == "can_bus_protocol_requires_can_bus_domain"));
        assert!(issues
            .iter()
            .any(|issue| issue.code == "can_bus_protocol_requires_can_bus_port"));
    }

    #[test]
    fn rejects_measurement_instrument_without_measurement_input() {
        let mut definition = minimal_model();
        definition.signal_ports.clear();

        let issues = definition.validate_all();

        assert!(issues
            .iter()
            .any(|issue| issue.code == "measurement_instrument_requires_measurement_input"));
    }

    #[test]
    fn rejects_software_system_with_physical_rf_port_unless_explicitly_allowed() {
        let mut definition = minimal_model();
        definition.equipment_class = EquipmentClass::SoftwareAdapter;
        definition.functional_role = FunctionalRole::SoftwareSystem;
        definition.category_code = "test_acquisition_software".to_owned();
        definition.signal_domains = vec![SignalDomain::Software];
        definition.technology_tags.clear();

        let issues = definition.validate_all();

        assert!(issues
            .iter()
            .any(|issue| issue.code == "software_system_declares_physical_rf_port"));
    }

    #[test]
    fn checksum_changes_when_classification_fields_change() {
        let mut changed = minimal_model();
        changed.signal_domains.push(SignalDomain::Usb);
        changed.technology_tags.push(TechnologyTag::Usb);

        let original = minimal_model().canonicalize().unwrap();
        let changed = changed.canonicalize().unwrap();

        assert_ne!(original.definition_checksum, changed.definition_checksum);
    }

    #[test]
    fn v2_canonicalization_is_stable_for_classification_maps() {
        let mut first = minimal_model();
        first.metadata.insert(
            "classification".to_owned(),
            serde_json::json!({"b": 2, "a": {"z": true, "m": false}}),
        );
        let mut second = minimal_model();
        second.metadata.insert(
            "classification".to_owned(),
            serde_json::json!({"a": {"m": false, "z": true}, "b": 2}),
        );

        let first = first.canonicalize().unwrap();
        let second = second.canonicalize().unwrap();

        assert_eq!(
            first.definition_schema_version,
            EQUIPMENT_MODEL_DEFINITION_SCHEMA_VERSION
        );
        assert_eq!(first.canonical_json, second.canonical_json);
        assert_eq!(first.definition_checksum, second.definition_checksum);
    }

    #[test]
    fn rejects_rf_signal_port_without_impedance() {
        let mut definition = minimal_model();
        definition.signal_ports[0].impedance = None;

        let issues = definition.validate_all();

        assert!(issues
            .iter()
            .any(|issue| issue.code == "rf_port_missing_impedance"));
    }

    #[test]
    fn rejects_signal_source_without_source_output() {
        let mut definition = minimal_model();
        definition.functional_role = FunctionalRole::SignalSource;
        definition.signal_ports[0].flow_role = PortFlowRole::MeasurementPort;

        let issues = definition.validate_all();

        assert!(issues
            .iter()
            .any(|issue| issue.code == "signal_source_requires_source_output"));
    }

    #[test]
    fn rejects_sensor_without_transducer_output() {
        let mut definition = minimal_model();
        definition.equipment_class = EquipmentClass::Sensor;
        definition.functional_role = FunctionalRole::Sensor;
        definition.signal_domains = vec![SignalDomain::Environmental];
        definition.signal_ports = vec![SignalPortDefinition {
            port_id: "temperature_field".to_owned(),
            label: "Temperature field".to_owned(),
            directionality: PortDirectionality::Input,
            flow_role: PortFlowRole::FieldSidePort,
            signal_domain: SignalDomain::Environmental,
            required: true,
            connector_type: None,
            technology_tags: Vec::new(),
            quantity: PhysicalQuantity::Temperature,
            unit: "Celsius".to_owned(),
            impedance: None,
            frequency_min: None,
            frequency_max: None,
            voltage_max: None,
            current_max: None,
            power_max: None,
            channel_index: None,
            differential: false,
            isolated: false,
            comment: None,
        }];

        let issues = definition.validate_all();

        assert!(issues
            .iter()
            .any(|issue| issue.code == "sensor_requires_input_and_output"));
    }

    #[test]
    fn rejects_communication_port_used_as_measurement_signal() {
        let mut definition = minimal_model();
        definition.signal_domains.push(SignalDomain::Usb);
        definition.signal_ports.push(SignalPortDefinition {
            port_id: "usb_control".to_owned(),
            label: "USB control".to_owned(),
            directionality: PortDirectionality::Communication,
            flow_role: PortFlowRole::CommunicationPort,
            signal_domain: SignalDomain::Usb,
            required: false,
            connector_type: Some("USB-C".to_owned()),
            technology_tags: vec![TechnologyTag::Usb],
            quantity: PhysicalQuantity::Binary,
            unit: "dimensionless".to_owned(),
            impedance: None,
            frequency_min: None,
            frequency_max: None,
            voltage_max: None,
            current_max: None,
            power_max: None,
            channel_index: None,
            differential: false,
            isolated: false,
            comment: None,
        });
        definition.capabilities[0]
            .required_signal_ports
            .push("usb_control".to_owned());

        let issues = definition.validate_all();

        assert!(issues
            .iter()
            .any(|issue| issue.code == "communication_port_used_as_signal_port"));
    }

    #[test]
    fn rejects_port_technology_tag_incompatible_with_signal_domain() {
        let mut definition = minimal_model();
        definition.signal_ports[0].technology_tags = vec![TechnologyTag::VoltageInput];

        let issues = definition.validate_all();

        assert!(issues
            .iter()
            .any(|issue| issue.code == "port_technology_tag_domain_mismatch"));
    }

    #[test]
    fn rejects_acquisition_device_without_control_path() {
        let mut definition = minimal_model();
        definition.equipment_class = EquipmentClass::AcquisitionDevice;
        definition.functional_role = FunctionalRole::AcquisitionDevice;
        definition.category_code = "daq_card".to_owned();
        definition.signal_domains = vec![SignalDomain::AnalogVoltage];
        definition.technology_tags = vec![TechnologyTag::VoltageInput];
        definition.signal_ports[0].signal_domain = SignalDomain::AnalogVoltage;
        definition.signal_ports[0].technology_tags = vec![TechnologyTag::VoltageInput];
        definition.signal_ports[0].impedance = None;
        definition.signal_ports[0].quantity = PhysicalQuantity::Voltage;
        definition.signal_ports[0].unit = "V".to_owned();
        definition.communication_interfaces.clear();

        let issues = definition.validate_all();

        assert!(issues
            .iter()
            .any(|issue| issue.code == "acquisition_device_requires_input_and_control_path"));
    }

    #[test]
    fn rejects_bare_can_category_for_converter_context() {
        let mut definition = minimal_model();
        definition.equipment_class = EquipmentClass::Converter;
        definition.functional_role = FunctionalRole::Converter;
        definition.category_code = "can".to_owned();

        let issues = definition.validate_all();

        assert!(issues
            .iter()
            .any(|issue| issue.code == "ambiguous_equipment_category_code"));
    }

    #[test]
    fn rejects_single_through_port_topology() {
        let mut definition = minimal_model();
        definition.functional_role = FunctionalRole::RfNetworkElement;
        definition.signal_ports[0].directionality = PortDirectionality::Through;
        definition.signal_ports[0].flow_role = PortFlowRole::ThroughPort;

        let issues = definition.validate_all();

        assert!(issues
            .iter()
            .any(|issue| issue.code == "through_device_requires_two_ports"));
    }

    #[test]
    fn rejects_unbounded_driver_loop() {
        let model = minimal_model();
        let driver = DriverProfileDefinition {
            definition_schema_version: DRIVER_PROFILE_DEFINITION_SCHEMA_VERSION.to_owned(),
            equipment_model_id: "model.power-meter".to_owned(),
            supported_model_revision_id: "model.power-meter-rev-0001".to_owned(),
            supported_model_definition_checksum:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
            supported_firmware_ranges: Vec::new(),
            communication_profiles: vec!["tcp".to_owned()],
            actions: vec![DriverActionDefinition {
                action_id: "set_frequency".to_owned(),
                label: "Set frequency".to_owned(),
                description: "Set tuned frequency".to_owned(),
                implements_capability_id: "set_frequency".to_owned(),
                inputs: Vec::new(),
                outputs: Vec::new(),
                preconditions: Vec::new(),
                postconditions: Vec::new(),
                safety_class: SafetyClass::ConfigurationChange,
                default_timeout_ms: 1000,
                retry_policy: None,
                script: DriverScriptDefinition {
                    steps: vec![DriverScriptStep {
                        step_id: "wait_forever".to_owned(),
                        step_type: ScriptStepType::LoopUntil,
                        enabled: true,
                        interface_id: None,
                        payload_format: None,
                        payload: None,
                        response_binding: None,
                        timeout_ms: None,
                        variable: None,
                        value: None,
                        expression: Some("${state.ready} == true".to_owned()),
                        action_id: None,
                        max_iterations: None,
                        duration_ms: None,
                        message: None,
                        frame: None,
                        steps: Vec::new(),
                        else_steps: Vec::new(),
                        retry_policy: None,
                        comment: None,
                    }],
                },
                requires_operator_confirmation: false,
                safe_to_retry: true,
                idempotent: true,
                rollback_action_id: None,
                safe_state_action_id: None,
            }],
            safe_state_action_id: None,
            error_check_action_id: None,
            metadata: BTreeMap::new(),
        };

        let issues = driver.validate_all(Some(&model));
        assert!(issues.iter().any(|issue| issue.code == "unbounded_loop"));
    }

    fn assert_no_validation_errors(definition: &EquipmentModelDefinition) {
        let errors = definition
            .validate_all()
            .into_iter()
            .filter(|issue| issue.severity == "error")
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{errors:?}");
    }

    fn rf_cable_model() -> EquipmentModelDefinition {
        let mut definition = minimal_model();
        definition.manufacturer = "Demo".to_owned();
        definition.model_name = "RF cable".to_owned();
        definition.variant = Some("1m".to_owned());
        definition.equipment_class = EquipmentClass::PassiveComponent;
        definition.functional_role = FunctionalRole::RfNetworkElement;
        definition.category_code = "rf_cable".to_owned();
        definition.signal_domains = vec![SignalDomain::Rf];
        definition.technology_tags = vec![TechnologyTag::Rf50Ohm];
        definition.signal_ports = vec![
            rf_port(
                "rf_a",
                "RF A",
                PortDirectionality::Through,
                PortFlowRole::ThroughPort,
            ),
            rf_port(
                "rf_b",
                "RF B",
                PortDirectionality::Through,
                PortFlowRole::ThroughPort,
            ),
        ];
        definition.communication_interfaces.clear();
        definition.capabilities.clear();
        definition
    }

    fn receiving_antenna_model() -> EquipmentModelDefinition {
        let mut definition = minimal_model();
        definition.manufacturer = "Demo".to_owned();
        definition.model_name = "Receive antenna".to_owned();
        definition.variant = None;
        definition.equipment_class = EquipmentClass::Sensor;
        definition.functional_role = FunctionalRole::Sensor;
        definition.category_code = "receiving_antenna".to_owned();
        definition.signal_domains = vec![SignalDomain::Environmental, SignalDomain::Rf];
        definition.technology_tags = vec![TechnologyTag::Rf50Ohm];
        definition.signal_ports = vec![
            SignalPortDefinition {
                port_id: "field".to_owned(),
                label: "Field".to_owned(),
                directionality: PortDirectionality::Input,
                flow_role: PortFlowRole::FieldSidePort,
                signal_domain: SignalDomain::Environmental,
                required: true,
                connector_type: None,
                technology_tags: Vec::new(),
                quantity: PhysicalQuantity::ElectricField,
                unit: "dBuV_per_m".to_owned(),
                impedance: None,
                frequency_min: None,
                frequency_max: None,
                voltage_max: None,
                current_max: None,
                power_max: None,
                channel_index: None,
                differential: false,
                isolated: false,
                comment: None,
            },
            rf_port(
                "rf_out",
                "RF output",
                PortDirectionality::Output,
                PortFlowRole::TransducerOutputPort,
            ),
        ];
        definition.communication_interfaces.clear();
        definition.capabilities.clear();
        definition
    }

    fn adc_converter_model() -> EquipmentModelDefinition {
        let mut definition = minimal_model();
        definition.manufacturer = "Demo".to_owned();
        definition.model_name = "ADC converter".to_owned();
        definition.variant = None;
        definition.equipment_class = EquipmentClass::DaqDevice;
        definition.functional_role = FunctionalRole::Converter;
        definition.category_code = "adc_converter".to_owned();
        definition.signal_domains = vec![SignalDomain::AnalogVoltage, SignalDomain::DigitalLogic];
        definition.technology_tags = vec![TechnologyTag::AdcConverter, TechnologyTag::VoltageInput];
        definition.signal_ports = vec![
            SignalPortDefinition {
                port_id: "analog_in".to_owned(),
                label: "Analog input".to_owned(),
                directionality: PortDirectionality::Input,
                flow_role: PortFlowRole::MeasurementPort,
                signal_domain: SignalDomain::AnalogVoltage,
                required: true,
                connector_type: Some("BNC".to_owned()),
                technology_tags: vec![TechnologyTag::VoltageInput],
                quantity: PhysicalQuantity::Voltage,
                unit: "V".to_owned(),
                impedance: None,
                frequency_min: None,
                frequency_max: None,
                voltage_max: Some(10.0),
                current_max: None,
                power_max: None,
                channel_index: Some(1),
                differential: false,
                isolated: false,
                comment: None,
            },
            SignalPortDefinition {
                port_id: "digital_out".to_owned(),
                label: "Digital output".to_owned(),
                directionality: PortDirectionality::Output,
                flow_role: PortFlowRole::TransducerOutputPort,
                signal_domain: SignalDomain::DigitalLogic,
                required: true,
                connector_type: Some("internal".to_owned()),
                technology_tags: vec![TechnologyTag::AdcConverter],
                quantity: PhysicalQuantity::Binary,
                unit: "dimensionless".to_owned(),
                impedance: None,
                frequency_min: None,
                frequency_max: None,
                voltage_max: None,
                current_max: None,
                power_max: None,
                channel_index: None,
                differential: false,
                isolated: false,
                comment: None,
            },
        ];
        definition.communication_interfaces.clear();
        definition.capabilities.clear();
        definition
    }

    fn rf_port(
        port_id: &str,
        label: &str,
        directionality: PortDirectionality,
        flow_role: PortFlowRole,
    ) -> SignalPortDefinition {
        SignalPortDefinition {
            port_id: port_id.to_owned(),
            label: label.to_owned(),
            directionality,
            flow_role,
            signal_domain: SignalDomain::Rf,
            required: true,
            connector_type: Some("N".to_owned()),
            technology_tags: vec![TechnologyTag::Rf50Ohm],
            quantity: PhysicalQuantity::Power,
            unit: "dBm".to_owned(),
            impedance: Some(50.0),
            frequency_min: Some(9_000.0),
            frequency_max: Some(1_000_000_000.0),
            voltage_max: None,
            current_max: None,
            power_max: Some(30.0),
            channel_index: None,
            differential: false,
            isolated: false,
            comment: None,
        }
    }

    #[test]
    fn simulator_uses_structured_steps_and_virtual_delays() {
        let driver = minimal_driver();
        let scenario = DriverSimulationScenario {
            scenario_id: "success".to_owned(),
            driver_revision_id: "driver-rev-0001".to_owned(),
            action_id: "measure_power".to_owned(),
            input_values: BTreeMap::new(),
            simulated_responses: vec![Value::String("-12.3".to_owned())],
            operator_confirmations: BTreeMap::new(),
        };

        let result = simulate_driver_action(&driver, &scenario).unwrap();

        assert_eq!(result.status, "passed");
        assert_eq!(result.trace.len(), 3);
        assert_eq!(result.trace[1].operation, "query");
        assert_eq!(result.trace[2].duration_virtual_ms, 25);
    }

    fn minimal_model() -> EquipmentModelDefinition {
        EquipmentModelDefinition {
            definition_schema_version: EQUIPMENT_MODEL_DEFINITION_SCHEMA_VERSION.to_owned(),
            manufacturer: "Rohde & Schwarz".to_owned(),
            model_name: "NRP6AN".to_owned(),
            variant: Some("FWD".to_owned()),
            equipment_class: EquipmentClass::ControllableInstrument,
            functional_role: FunctionalRole::MeasurementInstrument,
            category_code: "power_meter".to_owned(),
            signal_domains: vec![SignalDomain::Rf, SignalDomain::Ethernet],
            technology_tags: vec![
                TechnologyTag::Rf50Ohm,
                TechnologyTag::Ethernet,
                TechnologyTag::RawTcp,
                TechnologyTag::Scpi,
            ],
            specifications: Vec::new(),
            signal_ports: vec![SignalPortDefinition {
                port_id: "rf_input".to_owned(),
                label: "RF input".to_owned(),
                directionality: PortDirectionality::Input,
                flow_role: PortFlowRole::MeasurementPort,
                signal_domain: SignalDomain::Rf,
                required: true,
                connector_type: Some("N".to_owned()),
                technology_tags: vec![TechnologyTag::Rf50Ohm],
                quantity: PhysicalQuantity::Power,
                unit: "dBm".to_owned(),
                impedance: Some(50.0),
                frequency_min: Some(9_000.0),
                frequency_max: Some(1_000_000_000.0),
                voltage_max: None,
                current_max: None,
                power_max: Some(30.0),
                channel_index: None,
                differential: false,
                isolated: false,
                comment: None,
            }],
            communication_interfaces: vec![CommunicationInterfaceDefinition {
                interface_id: "tcp".to_owned(),
                label: "Raw TCP SCPI".to_owned(),
                transport_kind: TransportKind::EthernetTcp,
                access_provider_kind: AccessProviderKind::NativeTcp,
                protocol_kind: ProtocolKind::Scpi,
                required: false,
                default_interface: true,
                configuration_schema: BTreeMap::new(),
                default_configuration: BTreeMap::new(),
                framing: Some("lf".to_owned()),
                identification_strategy: None,
                firmware_compatibility: Vec::new(),
            }],
            capabilities: vec![MeasurementCapabilityDefinition {
                capability_id: "set_frequency".to_owned(),
                label: "Set frequency".to_owned(),
                description: "Tune the measurement frequency".to_owned(),
                capability_kind: "set_frequency".to_owned(),
                inputs: Vec::new(),
                outputs: Vec::new(),
                constraints: Vec::new(),
                required_signal_ports: vec!["rf_input".to_owned()],
                safety_class: SafetyClass::ConfigurationChange,
            }],
            metadata: BTreeMap::new(),
        }
    }

    fn minimal_driver() -> DriverProfileDefinition {
        DriverProfileDefinition {
            definition_schema_version: DRIVER_PROFILE_DEFINITION_SCHEMA_VERSION.to_owned(),
            equipment_model_id: "model.power-meter".to_owned(),
            supported_model_revision_id: "model.power-meter-rev-0001".to_owned(),
            supported_model_definition_checksum:
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
            supported_firmware_ranges: Vec::new(),
            communication_profiles: vec!["tcp".to_owned()],
            actions: vec![DriverActionDefinition {
                action_id: "measure_power".to_owned(),
                label: "Measure power".to_owned(),
                description: "Query power".to_owned(),
                implements_capability_id: "set_frequency".to_owned(),
                inputs: Vec::new(),
                outputs: vec![ActionValueDefinition {
                    name: "power_dbm".to_owned(),
                    value_type: ValueType::Text,
                    quantity: PhysicalQuantity::Power,
                    unit: "dBm".to_owned(),
                    required: true,
                    default_value: None,
                    minimum: None,
                    maximum: None,
                    enum_values: Vec::new(),
                    description: None,
                }],
                preconditions: Vec::new(),
                postconditions: Vec::new(),
                safety_class: SafetyClass::ReadOnly,
                default_timeout_ms: 1000,
                retry_policy: None,
                script: DriverScriptDefinition {
                    steps: vec![
                        DriverScriptStep {
                            step_id: "write_config".to_owned(),
                            step_type: ScriptStepType::IoWrite,
                            enabled: true,
                            interface_id: Some("tcp".to_owned()),
                            payload_format: Some(PayloadFormat::Text),
                            payload: Some("SENS:FREQ 1000000".to_owned()),
                            response_binding: None,
                            timeout_ms: Some(1000),
                            variable: None,
                            value: None,
                            expression: None,
                            action_id: None,
                            max_iterations: None,
                            duration_ms: None,
                            message: None,
                            frame: None,
                            steps: Vec::new(),
                            else_steps: Vec::new(),
                            retry_policy: None,
                            comment: None,
                        },
                        DriverScriptStep {
                            step_id: "query_power".to_owned(),
                            step_type: ScriptStepType::IoQuery,
                            enabled: true,
                            interface_id: Some("tcp".to_owned()),
                            payload_format: Some(PayloadFormat::Text),
                            payload: Some("MEAS:POW?".to_owned()),
                            response_binding: Some("${result.power_dbm}".to_owned()),
                            timeout_ms: Some(1000),
                            variable: None,
                            value: None,
                            expression: None,
                            action_id: None,
                            max_iterations: None,
                            duration_ms: None,
                            message: None,
                            frame: None,
                            steps: Vec::new(),
                            else_steps: Vec::new(),
                            retry_policy: None,
                            comment: None,
                        },
                        DriverScriptStep {
                            step_id: "virtual_delay".to_owned(),
                            step_type: ScriptStepType::Delay,
                            enabled: true,
                            interface_id: None,
                            payload_format: None,
                            payload: None,
                            response_binding: None,
                            timeout_ms: None,
                            variable: None,
                            value: None,
                            expression: None,
                            action_id: None,
                            max_iterations: None,
                            duration_ms: Some(25),
                            message: None,
                            frame: None,
                            steps: Vec::new(),
                            else_steps: Vec::new(),
                            retry_policy: None,
                            comment: None,
                        },
                    ],
                },
                requires_operator_confirmation: false,
                safe_to_retry: true,
                idempotent: true,
                rollback_action_id: None,
                safe_state_action_id: None,
            }],
            safe_state_action_id: None,
            error_check_action_id: None,
            metadata: BTreeMap::new(),
        }
    }
}
