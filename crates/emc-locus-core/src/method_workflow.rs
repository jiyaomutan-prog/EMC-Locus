use crate::test_definitions::{
    CalibrationRequirement, ExecutionStepKind, InstrumentSubstitutionPolicy,
    PostProcessingOperationType, TestTemplateDefinition, VariableDefaultValue, VariableLockPolicy,
    VariableValueType,
};
use crate::{PortDirectionality, SignalDomain};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const TEST_METHOD_DEFINITION_SCHEMA_VERSION: &str = "emc-locus.test-method-definition.v2";
pub const MEASUREMENT_SYSTEM_TEMPLATE_SCHEMA_VERSION: &str =
    "emc-locus.measurement-system-template-definition.v1";
pub const REGULATION_PROFILE_SCHEMA_VERSION: &str = "emc-locus.regulation-profile-definition.v1";
pub const EXECUTION_CONFIGURATION_SCHEMA_VERSION: &str = "emc-locus.execution-configuration.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MethodHierarchyNodeKind {
    Domain,
    TestFamily,
    SourceDocument,
    ApplicableEdition,
    Procedure,
    MethodVariant,
    ParameterProfile,
    SubRangeProfile,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodHierarchyNode {
    pub node_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_node_id: Option<String>,
    pub node_kind: MethodHierarchyNodeKind,
    pub label: String,
    pub position: u32,
    #[serde(default)]
    pub archived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MethodVariableSemantic {
    MethodParameter,
    ProjectEutInput,
    OperatorInput,
    DerivedValue,
    Setpoint,
    ObservedSignal,
    MonitoringSignal,
    IntermediateResult,
    FinalResult,
    VerdictOutput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityPhase {
    Definition,
    Preparation,
    Preflight,
    Execution,
    PostProcessing,
    Verdict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityDimension {
    Dimensionless,
    Frequency,
    Time,
    Voltage,
    Current,
    Power,
    ElectricFieldStrength,
    MagneticFieldStrength,
    Ratio,
    Text,
    Boolean,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantityValue {
    pub value: f64,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeterministicExpression {
    Literal {
        value: f64,
        unit: String,
    },
    Variable {
        variable_id: String,
    },
    Add {
        operands: Vec<DeterministicExpression>,
    },
    Subtract {
        left: Box<DeterministicExpression>,
        right: Box<DeterministicExpression>,
    },
    Multiply {
        left: Box<DeterministicExpression>,
        right: Box<DeterministicExpression>,
    },
    Divide {
        numerator: Box<DeterministicExpression>,
        denominator: Box<DeterministicExpression>,
    },
    Minimum {
        operands: Vec<DeterministicExpression>,
    },
    Maximum {
        operands: Vec<DeterministicExpression>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MethodVariableDefinition {
    pub variable_id: String,
    pub label: String,
    pub description: String,
    pub semantic: MethodVariableSemantic,
    pub value_type: VariableValueType,
    pub dimension: QuantityDimension,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<VariableDefaultValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
    pub required: bool,
    pub source: String,
    pub availability_phase: AvailabilityPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<DeterministicExpression>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consumers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TechnicalRangeRequirement {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MethodCapabilityRequirement {
    pub capability_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_range: Option<TechnicalRangeRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voltage_range: Option<TechnicalRangeRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_range: Option<TechnicalRangeRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_range: Option<TechnicalRangeRequirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operating_modes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_driver_action: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MethodLogicalPort {
    pub port_id: String,
    pub label: String,
    pub directionality: PortDirectionality,
    pub signal_domain: SignalDomain,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impedance_ohm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity_dimension: Option<QuantityDimension>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MethodCalibrationPolicy {
    Required,
    IfUsed,
    NotRequired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MethodSubstitutionPolicy {
    NoSubstitution,
    SameExactModel,
    SameCategory,
    SameCapabilities,
    ApprovedEquivalent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleAssignmentStage {
    MethodDefinition,
    SystemTemplate,
    PlannedTestPreparation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MethodFunctionalRole {
    pub role_id: String,
    pub label: String,
    pub purpose: String,
    pub required: bool,
    pub functional_category: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<MethodCapabilityRequirement>,
    pub calibration_policy: MethodCalibrationPolicy,
    pub substitution_policy: MethodSubstitutionPolicy,
    pub assignment_stage: RoleAssignmentStage,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logical_ports: Vec<MethodLogicalPort>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consumes_variables: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub produces_variables: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyEdgeKind {
    PhysicalSignal,
    ExcitationOrPower,
    ControlCommand,
    FeedbackMeasurement,
    Monitoring,
    Trigger,
    Synchronization,
    DataStream,
    EutState,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TopologyPortEndpoint {
    pub node_id: String,
    pub port_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeasurementSystemRoleNode {
    pub node_id: String,
    pub label: String,
    pub role_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method_role_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<MethodLogicalPort>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementSystemEdge {
    pub edge_id: String,
    pub label: String,
    pub edge_kind: TopologyEdgeKind,
    pub from: TopologyPortEndpoint,
    pub to: TopologyPortEndpoint,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrectionPointDefinition {
    pub correction_point_id: String,
    pub label: String,
    pub node_id: String,
    pub signal_variable_id: String,
    pub correction_kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegulationLoopMapping {
    pub loop_id: String,
    pub label: String,
    pub regulation_profile_id: String,
    pub actuator_node_id: String,
    pub feedback_node_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub monitoring_node_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementSystemTemplateStatus {
    Draft,
    Validated,
    Approved,
    Superseded,
    Archived,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeasurementSystemTemplateDefinition {
    pub definition_schema_version: String,
    pub template_id: String,
    pub label: String,
    pub classification: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<MeasurementSystemRoleNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<MeasurementSystemEdge>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub correction_points: Vec<CorrectionPointDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regulation_loops: Vec<RegulationLoopMapping>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegulationControlMode {
    OpenLoop,
    ClosedLoop,
    MonitorOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub maximum_attempts: u32,
    pub on_exhaustion: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegulationProfileDefinition {
    pub definition_schema_version: String,
    pub profile_id: String,
    pub label: String,
    pub regulated_quantity: String,
    pub regulated_unit: String,
    pub target_expression: DeterministicExpression,
    pub tolerance_band: QuantityValue,
    pub control_mode: RegulationControlMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actuator_role_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_driver_action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_role_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_signal_variable_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub monitoring_role_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration_reference_profile: Option<String>,
    pub start_threshold: QuantityValue,
    pub fast_increasing_step: QuantityValue,
    pub slow_increasing_step: QuantityValue,
    pub decreasing_step: QuantityValue,
    pub dwell_t1_seconds: f64,
    pub dwell_t2_seconds: f64,
    pub dwell_t3_seconds: f64,
    pub regulation_start_criterion: String,
    pub regulation_end_criterion: String,
    pub regulation_factor: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_output: Option<QuantityValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_forward_power: Option<QuantityValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_reflected_power: Option<QuantityValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overshoot_limit: Option<QuantityValue>,
    pub retry_policy: RetryPolicy,
    pub abort_policy: String,
    pub safe_state_policy: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub operator_notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcedureNodeKind {
    Preparation,
    Verification,
    Calibration,
    Phase,
    SubRange,
    Loop,
    Sweep,
    SetEutState,
    SetOrientation,
    SetPolarization,
    ApplySetpoint,
    Regulate,
    Dwell,
    Acquire,
    Monitor,
    OperatorAction,
    Decision,
    Repeat,
    Finalize,
    SafeShutdown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcedureFailurePolicy {
    Stop,
    SafeShutdown,
    ContinueWithWarning,
    OperatorDecision,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcedureNode {
    pub node_id: String,
    pub label: String,
    pub purpose: String,
    pub node_kind: ProcedureNodeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled_condition: Option<DeterministicExpression>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_variables: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_variables: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<f64>,
    pub failure_policy: ProcedureFailurePolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_iterations: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ProcedureNode>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub audit_notes: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FrequencyProgression {
    FixedStep { step: QuantityValue },
    Percentage { percentage: f64 },
    PointsPerDecade { points: u32 },
    ExplicitList { points: Vec<QuantityValue> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SweepDirection {
    Increasing,
    Decreasing,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubRangeDefinition {
    pub sub_range_id: String,
    pub label: String,
    pub start_frequency: QuantityValue,
    pub stop_frequency: QuantityValue,
    #[serde(default = "default_true")]
    pub include_start: bool,
    #[serde(default = "default_true")]
    pub include_stop: bool,
    pub progression: FrequencyProgression,
    pub direction: SweepDirection,
    pub sweep_mode: String,
    pub dwell_seconds: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special_dwell_seconds: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include_frequencies: Vec<QuantityValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_frequencies: Vec<QuantityValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_template_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regulation_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modulation_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eut_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polarization: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comments: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModulationDefinition {
    None {
        profile_id: String,
        label: String,
    },
    Am {
        profile_id: String,
        label: String,
        modulation_frequency: QuantityValue,
        depth_percent: f64,
    },
    Fm {
        profile_id: String,
        label: String,
        modulation_frequency: QuantityValue,
        deviation: QuantityValue,
    },
    Pulse {
        profile_id: String,
        label: String,
        width: QuantityValue,
        repetition_frequency: QuantityValue,
    },
    LaboratoryExtension {
        profile_id: String,
        label: String,
        extension_kind: String,
        parameters: BTreeMap<String, Value>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitClassification {
    Safety,
    InstrumentProtection,
    ProcedureAcceptance,
    EutPerformance,
    UncertaintyHandling,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitComparison {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    InsideBand,
    OutsideBand,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitAction {
    Warn,
    Abort,
    SafeShutdown,
    MarkNonconforming,
    RequestOperatorDecision,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MethodLimitRule {
    pub limit_id: String,
    pub label: String,
    pub classification: LimitClassification,
    pub evaluated_variable_id: String,
    pub comparison: LimitComparison,
    pub threshold: QuantityValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_threshold: Option<QuantityValue>,
    pub phase: AvailabilityPhase,
    pub severity: String,
    pub action: LimitAction,
    pub contributes_to_verdict: bool,
    pub explanation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostProcessingNodeKind {
    CorrectionApplication,
    UnitConversion,
    Interpolation,
    Detector,
    Window,
    FftRequest,
    Smoothing,
    Aggregation,
    MaximumSearch,
    Average,
    PeakHold,
    LimitComparison,
    UncertaintyContribution,
    ResultExport,
    ReportField,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PostProcessingNode {
    pub node_id: String,
    pub label: String,
    pub node_kind: PostProcessingNodeKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_reference_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParameterProfileDefinition {
    pub profile_id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, VariableDefaultValue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionReference {
    pub identity_id: String,
    pub revision_id: String,
    pub definition_checksum: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestMethodDefinitionV2 {
    pub definition_schema_version: String,
    pub title: String,
    pub objective: String,
    pub scope: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classification_path: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub standard_references: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<MethodVariableDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lock_policy: Vec<VariableLockPolicy>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameter_profiles: Vec<ParameterProfileDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub functional_roles: Vec<MethodFunctionalRole>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub measurement_system_templates: Vec<RevisionReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regulation_profiles: Vec<RevisionReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modulation_profiles: Vec<ModulationDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sub_ranges: Vec<SubRangeDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub procedure: Vec<ProcedureNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limits: Vec<MethodLimitRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_processing: Vec<PostProcessingNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_output_variables: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub migration_evidence: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalMethodDefinition {
    pub definition_schema_version: String,
    pub canonical_json: String,
    pub definition_checksum: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodWorkflowValidationIssue {
    pub code: String,
    pub path: String,
    pub message: String,
}

impl MethodWorkflowValidationIssue {
    fn new(code: impl Into<String>, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            path: path.into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum VersionedMethodDefinition {
    V1(TestTemplateDefinition),
    V2(TestMethodDefinitionV2),
}

impl VersionedMethodDefinition {
    pub fn from_json_str(value: &str) -> Result<Self, MethodWorkflowValidationIssue> {
        let parsed: Value = serde_json::from_str(value).map_err(|error| {
            MethodWorkflowValidationIssue::new(
                "invalid_test_method_json",
                "$",
                format!("invalid method definition JSON: {error}"),
            )
        })?;
        match parsed
            .get("definition_schema_version")
            .and_then(Value::as_str)
        {
            Some(crate::test_definitions::TEST_TEMPLATE_DEFINITION_SCHEMA_VERSION) => {
                serde_json::from_value(parsed)
                    .map(Self::V1)
                    .map_err(|error| {
                        MethodWorkflowValidationIssue::new(
                            "invalid_legacy_method_definition",
                            "$",
                            error.to_string(),
                        )
                    })
            }
            Some(TEST_METHOD_DEFINITION_SCHEMA_VERSION) => serde_json::from_value(parsed)
                .map(Self::V2)
                .map_err(|error| {
                    MethodWorkflowValidationIssue::new(
                        "invalid_test_method_v2_definition",
                        "$",
                        error.to_string(),
                    )
                }),
            Some(schema) => Err(MethodWorkflowValidationIssue::new(
                "unsupported_test_method_schema",
                "definition_schema_version",
                format!("unsupported test method schema: {schema}"),
            )),
            None => Err(MethodWorkflowValidationIssue::new(
                "missing_test_method_schema",
                "definition_schema_version",
                "definition_schema_version is required",
            )),
        }
    }

    pub fn canonicalize(&self) -> Result<CanonicalMethodDefinition, MethodWorkflowValidationIssue> {
        match self {
            Self::V1(definition) => definition
                .canonicalize()
                .map(|canonical| CanonicalMethodDefinition {
                    definition_schema_version: canonical.definition_schema_version,
                    canonical_json: canonical.canonical_json,
                    definition_checksum: canonical.definition_checksum,
                })
                .map_err(|error| {
                    MethodWorkflowValidationIssue::new(error.code, "$", error.message)
                }),
            Self::V2(definition) => definition.canonicalize(),
        }
    }

    pub fn with_title(self, title: String) -> Self {
        match self {
            Self::V1(mut definition) => {
                definition.title = title;
                Self::V1(definition)
            }
            Self::V2(mut definition) => {
                definition.title = title;
                Self::V2(definition)
            }
        }
    }
}

pub fn derive_method_v2_successor(
    source: &TestTemplateDefinition,
) -> Result<TestMethodDefinitionV2, MethodWorkflowValidationIssue> {
    source
        .canonicalize()
        .map_err(|error| MethodWorkflowValidationIssue::new(error.code, "$", error.message))?;

    let mut migration_evidence = vec![
        "Converted explicitly from a test-template v1 revision; topology remains to be reviewed."
            .to_owned(),
    ];
    if !source.limits.is_empty() {
        migration_evidence.push(format!(
            "{} legacy limit definition(s) retained in the immutable source revision and require classified v2 rules.",
            source.limits.len()
        ));
    }
    if !source.post_processing.is_empty() {
        let kinds = source
            .post_processing
            .iter()
            .map(|operation| legacy_post_processing_kind(&operation.operation_type))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        migration_evidence.push(format!(
            "Legacy post-processing contracts ({kinds}) require explicit signal ownership review."
        ));
    }
    if source.sequence.iter().any(|step| !step.branches.is_empty()) {
        migration_evidence.push(
            "Legacy branch conditions remain in the immutable source revision and require deterministic expression review."
                .to_owned(),
        );
    }

    let variables = source
        .variables
        .iter()
        .map(|variable| {
            let (dimension, unit) = migrated_dimension(
                variable.constraints.dimensionless,
                variable.constraints.unit.as_deref(),
            );
            MethodVariableDefinition {
                variable_id: variable.variable_id.clone(),
                label: variable.label.clone(),
                description: variable
                    .description
                    .clone()
                    .unwrap_or_else(|| "Parametre migre depuis la definition v1.".to_owned()),
                semantic: MethodVariableSemantic::MethodParameter,
                value_type: variable.value_type.clone(),
                dimension,
                unit,
                default_value: variable.default_value.clone(),
                minimum: variable.constraints.minimum,
                maximum: variable.constraints.maximum,
                enum_values: variable.constraints.enum_values.clone(),
                required: variable.constraints.required,
                source: "legacy_method_v1".to_owned(),
                availability_phase: AvailabilityPhase::Definition,
                expression: None,
                consumers: Vec::new(),
            }
        })
        .collect();
    let functional_roles = source
        .instrumentation_chain
        .iter()
        .map(|slot| MethodFunctionalRole {
            role_id: slot.slot_id.clone(),
            label: slot.label.clone(),
            purpose: format!(
                "Besoin fonctionnel migre depuis l'emplacement {} de la definition v1.",
                slot.slot_id
            ),
            required: slot.required,
            functional_category: slot
                .required_category
                .clone()
                .unwrap_or_else(|| "review_required".to_owned()),
            capabilities: vec![MethodCapabilityRequirement {
                capability_kind: slot
                    .required_capability
                    .clone()
                    .unwrap_or_else(|| "legacy_category_membership".to_owned()),
                frequency_range: None,
                voltage_range: None,
                current_range: None,
                power_range: None,
                operating_modes: Vec::new(),
                required_driver_action: None,
            }],
            calibration_policy: match slot.calibration_requirement {
                CalibrationRequirement::Required => MethodCalibrationPolicy::Required,
                CalibrationRequirement::IfUsed => MethodCalibrationPolicy::IfUsed,
                CalibrationRequirement::NotRequired => MethodCalibrationPolicy::NotRequired,
            },
            substitution_policy: match slot.substitution_policy {
                InstrumentSubstitutionPolicy::NoSubstitution => {
                    MethodSubstitutionPolicy::NoSubstitution
                }
                InstrumentSubstitutionPolicy::SameCategory => {
                    MethodSubstitutionPolicy::SameCategory
                }
                InstrumentSubstitutionPolicy::SameCapability => {
                    MethodSubstitutionPolicy::SameCapabilities
                }
                InstrumentSubstitutionPolicy::ApprovedEquivalent => {
                    MethodSubstitutionPolicy::ApprovedEquivalent
                }
            },
            assignment_stage: RoleAssignmentStage::PlannedTestPreparation,
            logical_ports: Vec::new(),
            consumes_variables: Vec::new(),
            produces_variables: Vec::new(),
        })
        .collect();
    let procedure = source
        .sequence
        .iter()
        .map(|step| ProcedureNode {
            node_id: step.step_id.clone(),
            label: step.label.clone(),
            purpose: step
                .instruction
                .clone()
                .unwrap_or_else(|| "Etape migree depuis la definition v1.".to_owned()),
            node_kind: match step.kind {
                ExecutionStepKind::Prepare => ProcedureNodeKind::Preparation,
                ExecutionStepKind::ConfigureInstrument => ProcedureNodeKind::Verification,
                ExecutionStepKind::Acquire => ProcedureNodeKind::Acquire,
                ExecutionStepKind::OperatorDecision => ProcedureNodeKind::OperatorAction,
                ExecutionStepKind::PostProcess => ProcedureNodeKind::Phase,
                ExecutionStepKind::Verify => ProcedureNodeKind::Verification,
                ExecutionStepKind::Finish => ProcedureNodeKind::Finalize,
            },
            enabled_condition: None,
            input_variables: Vec::new(),
            output_variables: Vec::new(),
            timeout_seconds: None,
            failure_policy: ProcedureFailurePolicy::Stop,
            maximum_iterations: None,
            children: Vec::new(),
            audit_notes: if step.branches.is_empty() {
                String::new()
            } else {
                format!(
                    "{} branche(s) legacy a revoir dans le workflow hierarchique.",
                    step.branches.len()
                )
            },
        })
        .collect();

    let definition = TestMethodDefinitionV2 {
        definition_schema_version: TEST_METHOD_DEFINITION_SCHEMA_VERSION.to_owned(),
        title: source.title.clone(),
        objective: source.description.clone(),
        scope: format!(
            "Successor workflow for {:?} measurements.",
            source.measurement_axis
        ),
        classification_path: vec![source
            .method_code
            .clone()
            .unwrap_or_else(|| "legacy_method".to_owned())],
        standard_references: source.standard_references.clone(),
        variables,
        lock_policy: source.lock_policy.clone(),
        parameter_profiles: Vec::new(),
        functional_roles,
        measurement_system_templates: Vec::new(),
        regulation_profiles: Vec::new(),
        modulation_profiles: Vec::new(),
        sub_ranges: Vec::new(),
        procedure,
        limits: Vec::new(),
        post_processing: Vec::new(),
        expected_output_variables: Vec::new(),
        migration_evidence,
    };
    if let Some(issue) = definition.validate().into_iter().next() {
        return Err(issue);
    }
    Ok(definition)
}

fn migrated_dimension(
    dimensionless: bool,
    unit: Option<&str>,
) -> (QuantityDimension, Option<String>) {
    if dimensionless {
        return (QuantityDimension::Dimensionless, None);
    }
    unit.and_then(unit_dimension)
        .map(|dimension| (dimension, unit.map(str::to_owned)))
        .unwrap_or((QuantityDimension::Dimensionless, None))
}

fn legacy_post_processing_kind(kind: &PostProcessingOperationType) -> &'static str {
    match kind {
        PostProcessingOperationType::Correction => "correction",
        PostProcessingOperationType::Fft => "fft",
        PostProcessingOperationType::Windowing => "windowing",
        PostProcessingOperationType::Resampling => "resampling",
        PostProcessingOperationType::HarmonicCalculation => "harmonic_calculation",
        PostProcessingOperationType::EventCounting => "event_counting",
        PostProcessingOperationType::ChannelMath => "channel_math",
        PostProcessingOperationType::Peak => "peak",
        PostProcessingOperationType::Custom => "custom",
    }
}

impl TestMethodDefinitionV2 {
    pub fn validate(&self) -> Vec<MethodWorkflowValidationIssue> {
        validate_test_method_v2(self)
    }

    pub fn canonicalize(&self) -> Result<CanonicalMethodDefinition, MethodWorkflowValidationIssue> {
        let issues = self.validate();
        if let Some(issue) = issues.into_iter().next() {
            return Err(issue);
        }
        canonicalize(self, &self.definition_schema_version)
    }
}

impl MeasurementSystemTemplateDefinition {
    pub fn validate(
        &self,
        method_roles: &[MethodFunctionalRole],
        regulation_profiles: &[RegulationProfileDefinition],
    ) -> Vec<MethodWorkflowValidationIssue> {
        validate_measurement_system_template(self, method_roles, regulation_profiles)
    }

    pub fn canonicalize(
        &self,
        method_roles: &[MethodFunctionalRole],
        regulation_profiles: &[RegulationProfileDefinition],
    ) -> Result<CanonicalMethodDefinition, MethodWorkflowValidationIssue> {
        let issues = self.validate(method_roles, regulation_profiles);
        if let Some(issue) = issues.into_iter().next() {
            return Err(issue);
        }
        canonicalize(self, &self.definition_schema_version)
    }
}

impl RegulationProfileDefinition {
    pub fn validate(
        &self,
        variables: &[MethodVariableDefinition],
        roles: &[MethodFunctionalRole],
    ) -> Vec<MethodWorkflowValidationIssue> {
        validate_regulation_profile(self, variables, roles)
    }

    pub fn canonicalize(
        &self,
        variables: &[MethodVariableDefinition],
        roles: &[MethodFunctionalRole],
    ) -> Result<CanonicalMethodDefinition, MethodWorkflowValidationIssue> {
        let issues = self.validate(variables, roles);
        if let Some(issue) = issues.into_iter().next() {
            return Err(issue);
        }
        canonicalize(self, &self.definition_schema_version)
    }
}

impl ExecutionConfigurationDefinition {
    pub fn validate(
        &self,
        method: &TestMethodDefinitionV2,
        system: &MeasurementSystemTemplateDefinition,
    ) -> Vec<MethodWorkflowValidationIssue> {
        let mut issues = Vec::new();
        if self.definition_schema_version != EXECUTION_CONFIGURATION_SCHEMA_VERSION {
            issues.push(MethodWorkflowValidationIssue::new(
                "unsupported_execution_configuration_schema",
                "definition_schema_version",
                "execution configuration must use v1",
            ));
        }
        for (value, path) in [
            (&self.configuration_id, "configuration_id"),
            (&self.parameter_profile_id, "parameter_profile_id"),
            (&self.laboratory_location_id, "laboratory_location_id"),
            (&self.planned_use_on, "planned_use_on"),
            (&self.eut_context, "eut_context"),
        ] {
            require_text(&mut issues, value, path);
        }
        if !valid_iso_date(&self.planned_use_on) {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_execution_planned_date",
                "planned_use_on",
                "planned_use_on must be an ISO civil date",
            ));
        }
        validate_revision_references(
            &[
                self.method_revision.clone(),
                self.measurement_system_template_revision.clone(),
                self.station_setup_revision.clone(),
            ],
            "execution_configuration.revision_references",
            &mut issues,
        );
        if self.measurement_system_template_revision.identity_id != system.template_id {
            issues.push(MethodWorkflowValidationIssue::new(
                "execution_system_identity_mismatch",
                "measurement_system_template_revision.identity_id",
                "the pinned system identity does not match the selected topology",
            ));
        }
        let sub_range_ids: BTreeSet<_> = method
            .sub_ranges
            .iter()
            .map(|range| range.sub_range_id.as_str())
            .collect();
        for sub_range_id in &self.selected_sub_range_ids {
            if !sub_range_ids.contains(sub_range_id.as_str()) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "unknown_execution_sub_range",
                    "selected_sub_range_ids",
                    format!("selected sub-range {sub_range_id} does not exist in the method"),
                ));
            }
        }
        let profile = method
            .parameter_profiles
            .iter()
            .find(|profile| profile.profile_id == self.parameter_profile_id);
        if !method.parameter_profiles.is_empty() && profile.is_none() {
            issues.push(MethodWorkflowValidationIssue::new(
                "unknown_execution_parameter_profile",
                "parameter_profile_id",
                "the selected parameter profile does not exist in the method",
            ));
        }
        let variable_ids: BTreeSet<_> = method
            .variables
            .iter()
            .map(|variable| variable.variable_id.as_str())
            .collect();
        for variable_id in self.parameter_values.keys() {
            if !variable_ids.contains(variable_id.as_str()) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "unknown_execution_parameter",
                    "parameter_values",
                    format!("execution parameter {variable_id} does not exist in the method"),
                ));
            }
        }
        let role_ids: BTreeSet<_> = method
            .functional_roles
            .iter()
            .map(|role| role.role_id.as_str())
            .collect();
        let mut assigned_roles = BTreeSet::new();
        let mut assigned_assets = BTreeSet::new();
        for assignment in &self.assignments {
            if !role_ids.contains(assignment.role_id.as_str()) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "unknown_execution_role_assignment",
                    "assignments[].role_id",
                    format!("assignment references unknown role {}", assignment.role_id),
                ));
            }
            if !assigned_roles.insert(assignment.role_id.as_str()) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "duplicate_execution_role_assignment",
                    "assignments",
                    format!("role {} is assigned more than once", assignment.role_id),
                ));
            }
            if !assigned_assets.insert(assignment.asset_id.as_str()) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "duplicate_execution_asset_assignment",
                    "assignments",
                    format!("asset {} is assigned more than once", assignment.asset_id),
                ));
            }
            if !valid_checksum(&assignment.equipment_model_checksum) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "invalid_execution_model_checksum",
                    "assignments[].equipment_model_checksum",
                    "assignment model snapshots require canonical SHA-256 checksums",
                ));
            }
        }
        issues
    }

    pub fn canonicalize(
        &self,
        method: &TestMethodDefinitionV2,
        system: &MeasurementSystemTemplateDefinition,
    ) -> Result<CanonicalMethodDefinition, MethodWorkflowValidationIssue> {
        if let Some(issue) = self.validate(method, system).into_iter().next() {
            return Err(issue);
        }
        canonicalize(self, &self.definition_schema_version)
    }
}

pub fn validate_method_hierarchy(
    nodes: &[MethodHierarchyNode],
) -> Vec<MethodWorkflowValidationIssue> {
    let mut issues = Vec::new();
    let mut ids = BTreeSet::new();
    let mut positions = BTreeSet::new();
    for node in nodes {
        if !valid_token(&node.node_id) {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_hierarchy_node_id",
                "hierarchy[].node_id",
                "hierarchy node IDs must be stable ASCII tokens",
            ));
        }
        if !ids.insert(node.node_id.clone()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "duplicate_hierarchy_node_id",
                "hierarchy",
                format!("duplicate hierarchy node: {}", node.node_id),
            ));
        }
        if node.label.trim().is_empty() {
            issues.push(MethodWorkflowValidationIssue::new(
                "missing_hierarchy_label",
                "hierarchy[].label",
                "hierarchy node label is required",
            ));
        }
        if !positions.insert((node.parent_node_id.clone(), node.position)) {
            issues.push(MethodWorkflowValidationIssue::new(
                "duplicate_hierarchy_position",
                "hierarchy[].position",
                "siblings must have deterministic positions",
            ));
        }
    }
    for node in nodes {
        if let Some(parent) = node.parent_node_id.as_ref() {
            if !ids.contains(parent) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "unknown_hierarchy_parent",
                    "hierarchy[].parent_node_id",
                    format!("unknown hierarchy parent: {parent}"),
                ));
            }
        }
    }
    let parents: BTreeMap<_, _> = nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node.parent_node_id.as_deref()))
        .collect();
    for node in nodes {
        let mut seen = BTreeSet::new();
        let mut current = Some(node.node_id.as_str());
        while let Some(id) = current {
            if !seen.insert(id) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "method_hierarchy_cycle",
                    "hierarchy",
                    format!("hierarchy cycle includes {id}"),
                ));
                break;
            }
            current = parents.get(id).copied().flatten();
        }
    }
    issues
}

fn validate_test_method_v2(
    definition: &TestMethodDefinitionV2,
) -> Vec<MethodWorkflowValidationIssue> {
    let mut issues = Vec::new();
    if definition.definition_schema_version != TEST_METHOD_DEFINITION_SCHEMA_VERSION {
        issues.push(MethodWorkflowValidationIssue::new(
            "unsupported_test_method_schema",
            "definition_schema_version",
            "method v2 must use the 0.22.2 schema",
        ));
    }
    require_text(&mut issues, &definition.title, "title");
    require_text(&mut issues, &definition.objective, "objective");
    require_text(&mut issues, &definition.scope, "scope");
    if definition.classification_path.is_empty() {
        issues.push(MethodWorkflowValidationIssue::new(
            "missing_method_classification",
            "classification_path",
            "select a method classification",
        ));
    }
    let variable_ids = validate_variables(&definition.variables, &mut issues);
    for policy in &definition.lock_policy {
        if !variable_ids.contains(&policy.variable_id) {
            issues.push(MethodWorkflowValidationIssue::new(
                "unknown_lock_variable",
                "lock_policy",
                format!(
                    "lock policy references unknown variable {}",
                    policy.variable_id
                ),
            ));
        }
    }
    validate_parameter_profiles(&definition.parameter_profiles, &variable_ids, &mut issues);
    let role_ids = validate_roles(&definition.functional_roles, &variable_ids, &mut issues);
    validate_revision_references(
        &definition.measurement_system_templates,
        "measurement_system_templates",
        &mut issues,
    );
    validate_revision_references(
        &definition.regulation_profiles,
        "regulation_profiles",
        &mut issues,
    );
    validate_modulations(&definition.modulation_profiles, &mut issues);
    validate_sub_ranges(&definition.sub_ranges, &mut issues);
    validate_procedure(&definition.procedure, &variable_ids, &mut issues);
    validate_limits(&definition.limits, &variable_ids, &mut issues);
    validate_post_processing(
        &definition.post_processing,
        &variable_ids,
        &definition.limits,
        &mut issues,
    );
    for output in &definition.expected_output_variables {
        if !variable_ids.contains(output) {
            issues.push(MethodWorkflowValidationIssue::new(
                "unknown_expected_output",
                "expected_output_variables",
                format!("expected output references unknown variable {output}"),
            ));
        }
    }
    if role_ids.is_empty() {
        issues.push(MethodWorkflowValidationIssue::new(
            "missing_functional_roles",
            "functional_roles",
            "define at least one functional instrumentation role",
        ));
    }
    issues
}

fn validate_variables(
    variables: &[MethodVariableDefinition],
    issues: &mut Vec<MethodWorkflowValidationIssue>,
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for variable in variables {
        if !valid_token(&variable.variable_id) || !ids.insert(variable.variable_id.clone()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_or_duplicate_variable_id",
                "variables[].variable_id",
                format!("invalid or duplicate variable ID: {}", variable.variable_id),
            ));
        }
        require_text(issues, &variable.label, "variables[].label");
        require_text(issues, &variable.description, "variables[].description");
        if variable
            .minimum
            .zip(variable.maximum)
            .is_some_and(|(min, max)| min > max)
        {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_variable_range",
                "variables",
                format!("minimum exceeds maximum for {}", variable.variable_id),
            ));
        }
        if requires_unit(variable.dimension) && variable.unit.as_deref().is_none_or(str::is_empty) {
            issues.push(MethodWorkflowValidationIssue::new(
                "missing_variable_unit",
                "variables[].unit",
                format!("{} requires an explicit unit", variable.variable_id),
            ));
        }
        if let Some(unit) = variable.unit.as_deref() {
            if unit_dimension(unit) != Some(variable.dimension) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "incompatible_variable_unit",
                    "variables[].unit",
                    format!("unit {unit} is incompatible with {}", variable.variable_id),
                ));
            }
        }
        if matches!(variable.semantic, MethodVariableSemantic::DerivedValue)
            && variable.expression.is_none()
        {
            issues.push(MethodWorkflowValidationIssue::new(
                "missing_derived_expression",
                "variables[].expression",
                format!(
                    "derived variable {} requires an expression",
                    variable.variable_id
                ),
            ));
        }
        validate_default(variable, issues);
    }
    let definitions: BTreeMap<_, _> = variables
        .iter()
        .map(|variable| (variable.variable_id.as_str(), variable))
        .collect();
    let mut dependencies: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for variable in variables {
        if let Some(expression) = variable.expression.as_ref() {
            let refs = expression_references(expression);
            for reference in &refs {
                let Some(source) = definitions.get(reference.as_str()) else {
                    issues.push(MethodWorkflowValidationIssue::new(
                        "unknown_expression_variable",
                        "variables[].expression",
                        format!(
                            "{} references unknown variable {reference}",
                            variable.variable_id
                        ),
                    ));
                    continue;
                };
                if source.availability_phase > variable.availability_phase {
                    issues.push(MethodWorkflowValidationIssue::new(
                        "variable_not_available_in_phase",
                        "variables[].availability_phase",
                        format!(
                            "{reference} is not available when {} is evaluated",
                            variable.variable_id
                        ),
                    ));
                }
            }
            if let Ok(dimension) = expression_dimension(expression, &definitions) {
                if dimension != variable.dimension {
                    issues.push(MethodWorkflowValidationIssue::new(
                        "expression_unit_mismatch",
                        "variables[].expression",
                        format!(
                            "expression dimension does not match {}",
                            variable.variable_id
                        ),
                    ));
                }
            }
            if contains_literal_zero_denominator(expression) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "unsafe_division_by_zero",
                    "variables[].expression",
                    format!(
                        "{} contains a literal division by zero",
                        variable.variable_id
                    ),
                ));
            }
            dependencies.insert(variable.variable_id.clone(), refs);
        }
    }
    for variable_id in dependencies.keys() {
        if dependency_cycle(variable_id, &dependencies) {
            issues.push(MethodWorkflowValidationIssue::new(
                "variable_dependency_cycle",
                "variables",
                format!("variable dependency cycle includes {variable_id}"),
            ));
        }
    }
    ids
}

fn validate_default(
    variable: &MethodVariableDefinition,
    issues: &mut Vec<MethodWorkflowValidationIssue>,
) {
    let Some(value) = variable.default_value.as_ref() else {
        return;
    };
    let type_matches = matches!(
        (&variable.value_type, value),
        (VariableValueType::Integer, VariableDefaultValue::Integer(_))
            | (VariableValueType::Number, VariableDefaultValue::Integer(_))
            | (VariableValueType::Number, VariableDefaultValue::Number(_))
            | (VariableValueType::Boolean, VariableDefaultValue::Boolean(_))
            | (VariableValueType::Text, VariableDefaultValue::Text(_))
            | (VariableValueType::Enum, VariableDefaultValue::Text(_))
    );
    if !type_matches {
        issues.push(MethodWorkflowValidationIssue::new(
            "variable_default_type_mismatch",
            "variables[].default_value",
            format!("default value type does not match {}", variable.variable_id),
        ));
    }
    if let VariableDefaultValue::Text(default) = value {
        if matches!(variable.value_type, VariableValueType::Enum)
            && !variable.enum_values.iter().any(|value| value == default)
        {
            issues.push(MethodWorkflowValidationIssue::new(
                "enum_default_not_allowed",
                "variables[].default_value",
                format!("enum default is not allowed for {}", variable.variable_id),
            ));
        }
    }
}

fn validate_parameter_profiles(
    profiles: &[ParameterProfileDefinition],
    variable_ids: &BTreeSet<String>,
    issues: &mut Vec<MethodWorkflowValidationIssue>,
) {
    let mut ids = BTreeSet::new();
    for profile in profiles {
        if !valid_token(&profile.profile_id) || !ids.insert(profile.profile_id.clone()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_or_duplicate_parameter_profile",
                "parameter_profiles",
                format!("invalid or duplicate profile {}", profile.profile_id),
            ));
        }
        for variable in profile.values.keys() {
            if !variable_ids.contains(variable) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "unknown_profile_variable",
                    "parameter_profiles[].values",
                    format!("parameter profile references unknown variable {variable}"),
                ));
            }
        }
    }
}

fn validate_roles(
    roles: &[MethodFunctionalRole],
    variable_ids: &BTreeSet<String>,
    issues: &mut Vec<MethodWorkflowValidationIssue>,
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for role in roles {
        if !valid_token(&role.role_id) || !ids.insert(role.role_id.clone()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_or_duplicate_functional_role",
                "functional_roles[].role_id",
                format!("invalid or duplicate role {}", role.role_id),
            ));
        }
        require_text(issues, &role.label, "functional_roles[].label");
        require_text(issues, &role.purpose, "functional_roles[].purpose");
        require_text(
            issues,
            &role.functional_category,
            "functional_roles[].functional_category",
        );
        if role.capabilities.is_empty() {
            issues.push(MethodWorkflowValidationIssue::new(
                "missing_role_capability",
                "functional_roles[].capabilities",
                format!("role {} needs a stable capability", role.role_id),
            ));
        }
        let mut port_ids = BTreeSet::new();
        for port in &role.logical_ports {
            if !valid_token(&port.port_id) || !port_ids.insert(port.port_id.clone()) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "invalid_or_duplicate_role_port",
                    "functional_roles[].logical_ports",
                    format!(
                        "invalid or duplicate port {} on {}",
                        port.port_id, role.role_id
                    ),
                ));
            }
        }
        for variable in role
            .consumes_variables
            .iter()
            .chain(role.produces_variables.iter())
        {
            if !variable_ids.contains(variable) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "unknown_role_variable",
                    "functional_roles",
                    format!(
                        "role {} references unknown variable {variable}",
                        role.role_id
                    ),
                ));
            }
        }
        for capability in &role.capabilities {
            require_text(
                issues,
                &capability.capability_kind,
                "functional_roles[].capabilities[].capability_kind",
            );
            for range in [
                capability.frequency_range.as_ref(),
                capability.voltage_range.as_ref(),
                capability.current_range.as_ref(),
                capability.power_range.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                if range
                    .minimum
                    .zip(range.maximum)
                    .is_some_and(|(min, max)| min > max)
                {
                    issues.push(MethodWorkflowValidationIssue::new(
                        "invalid_role_range",
                        "functional_roles[].capabilities",
                        "technical range minimum exceeds maximum",
                    ));
                }
                if unit_dimension(&range.unit).is_none() {
                    issues.push(MethodWorkflowValidationIssue::new(
                        "unsupported_role_unit",
                        "functional_roles[].capabilities",
                        format!("unsupported or indeterminate unit {}", range.unit),
                    ));
                }
            }
        }
    }
    ids
}

fn validate_revision_references(
    references: &[RevisionReference],
    path: &str,
    issues: &mut Vec<MethodWorkflowValidationIssue>,
) {
    let mut ids = BTreeSet::new();
    for reference in references {
        if !ids.insert(reference.identity_id.clone()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "duplicate_revision_reference",
                path,
                format!("duplicate revision reference {}", reference.identity_id),
            ));
        }
        if !valid_checksum(&reference.definition_checksum) {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_revision_checksum",
                path,
                "revision references require canonical SHA-256 checksums",
            ));
        }
    }
}

fn validate_measurement_system_template(
    definition: &MeasurementSystemTemplateDefinition,
    method_roles: &[MethodFunctionalRole],
    regulation_profiles: &[RegulationProfileDefinition],
) -> Vec<MethodWorkflowValidationIssue> {
    let mut issues = Vec::new();
    if definition.definition_schema_version != MEASUREMENT_SYSTEM_TEMPLATE_SCHEMA_VERSION {
        issues.push(MethodWorkflowValidationIssue::new(
            "unsupported_measurement_system_schema",
            "definition_schema_version",
            "measurement-system template must use v1",
        ));
    }
    require_text(&mut issues, &definition.label, "label");
    require_text(&mut issues, &definition.classification, "classification");
    let method_role_ids: BTreeSet<_> = method_roles
        .iter()
        .map(|role| role.role_id.as_str())
        .collect();
    let mut nodes = BTreeMap::new();
    for node in &definition.nodes {
        if !valid_token(&node.node_id) || nodes.insert(node.node_id.as_str(), node).is_some() {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_or_duplicate_topology_node",
                "nodes[].node_id",
                format!("invalid or duplicate node {}", node.node_id),
            ));
        }
        if let Some(role_id) = node.method_role_id.as_deref() {
            if !method_role_ids.contains(role_id) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "unknown_method_role_mapping",
                    "nodes[].method_role_id",
                    format!("node {} maps unknown method role {role_id}", node.node_id),
                ));
            }
        }
        let mut ports = BTreeSet::new();
        for port in &node.ports {
            if !ports.insert(port.port_id.as_str()) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "duplicate_topology_port",
                    "nodes[].ports",
                    format!("duplicate port {} on node {}", port.port_id, node.node_id),
                ));
            }
        }
    }
    let mut edge_ids = BTreeSet::new();
    for edge in &definition.edges {
        if !valid_token(&edge.edge_id) || !edge_ids.insert(edge.edge_id.clone()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_or_duplicate_topology_edge",
                "edges[].edge_id",
                format!("invalid or duplicate edge {}", edge.edge_id),
            ));
        }
        let from = endpoint_port(&nodes, &edge.from);
        let to = endpoint_port(&nodes, &edge.to);
        if from.is_none() || to.is_none() {
            issues.push(MethodWorkflowValidationIssue::new(
                "unknown_topology_endpoint",
                "edges",
                format!("edge {} references an unknown node or port", edge.edge_id),
            ));
            continue;
        }
        let (from, to) = (from.unwrap(), to.unwrap());
        if !directions_connect(&from.directionality, &to.directionality) {
            issues.push(MethodWorkflowValidationIssue::new(
                "incompatible_topology_direction",
                "edges",
                format!("edge {} connects incompatible directions", edge.edge_id),
            ));
        }
        if edge_uses_physical_domain(&edge.edge_kind) && from.signal_domain != to.signal_domain {
            issues.push(MethodWorkflowValidationIssue::new(
                "incompatible_topology_signal_domain",
                "edges",
                format!("edge {} crosses incompatible signal domains", edge.edge_id),
            ));
        }
    }
    validate_non_feedback_cycles(definition, &mut issues);
    let regulation_ids: BTreeSet<_> = regulation_profiles
        .iter()
        .map(|profile| profile.profile_id.as_str())
        .collect();
    let mut loop_ids = BTreeSet::new();
    for mapping in &definition.regulation_loops {
        if !loop_ids.insert(mapping.loop_id.as_str()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "duplicate_regulation_loop",
                "regulation_loops",
                format!("duplicate regulation loop {}", mapping.loop_id),
            ));
        }
        if !regulation_ids.contains(mapping.regulation_profile_id.as_str()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "unknown_regulation_profile",
                "regulation_loops[].regulation_profile_id",
                format!(
                    "unknown regulation profile {}",
                    mapping.regulation_profile_id
                ),
            ));
        }
        if !nodes.contains_key(mapping.actuator_node_id.as_str())
            || !nodes.contains_key(mapping.feedback_node_id.as_str())
        {
            issues.push(MethodWorkflowValidationIssue::new(
                "unknown_regulation_loop_node",
                "regulation_loops",
                format!(
                    "regulation loop {} references an unknown node",
                    mapping.loop_id
                ),
            ));
        }
        let has_feedback_edge = definition.edges.iter().any(|edge| {
            edge.edge_kind == TopologyEdgeKind::FeedbackMeasurement
                && edge.from.node_id == mapping.feedback_node_id
                && edge.to.node_id == mapping.actuator_node_id
        });
        if !has_feedback_edge {
            issues.push(MethodWorkflowValidationIssue::new(
                "unreachable_regulation_feedback",
                "regulation_loops",
                format!("loop {} has no explicit feedback path", mapping.loop_id),
            ));
        }
    }
    for correction in &definition.correction_points {
        if !nodes.contains_key(correction.node_id.as_str()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "unknown_correction_point_node",
                "correction_points",
                format!(
                    "correction point {} references an unknown node",
                    correction.correction_point_id
                ),
            ));
        }
    }
    issues
}

fn validate_regulation_profile(
    profile: &RegulationProfileDefinition,
    variables: &[MethodVariableDefinition],
    roles: &[MethodFunctionalRole],
) -> Vec<MethodWorkflowValidationIssue> {
    let mut issues = Vec::new();
    if profile.definition_schema_version != REGULATION_PROFILE_SCHEMA_VERSION {
        issues.push(MethodWorkflowValidationIssue::new(
            "unsupported_regulation_profile_schema",
            "definition_schema_version",
            "regulation profile must use v1",
        ));
    }
    let variable_map: BTreeMap<_, _> = variables
        .iter()
        .map(|variable| (variable.variable_id.as_str(), variable))
        .collect();
    let role_ids: BTreeSet<_> = roles.iter().map(|role| role.role_id.as_str()).collect();
    if unit_dimension(&profile.regulated_unit).is_none() {
        issues.push(MethodWorkflowValidationIssue::new(
            "unsupported_regulated_unit",
            "regulated_unit",
            "regulated quantity unit is unsupported or indeterminate",
        ));
    }
    if let Err(message) = expression_dimension(&profile.target_expression, &variable_map) {
        issues.push(MethodWorkflowValidationIssue::new(
            "invalid_regulation_target_expression",
            "target_expression",
            message,
        ));
    }
    if matches!(profile.control_mode, RegulationControlMode::ClosedLoop) {
        for (value, code, path) in [
            (
                profile.actuator_role_id.as_deref(),
                "missing_regulation_actuator",
                "actuator_role_id",
            ),
            (
                profile.feedback_role_id.as_deref(),
                "missing_regulation_feedback",
                "feedback_role_id",
            ),
            (
                profile.required_driver_action.as_deref(),
                "missing_regulation_driver_action",
                "required_driver_action",
            ),
            (
                profile.feedback_signal_variable_id.as_deref(),
                "missing_regulation_feedback_signal",
                "feedback_signal_variable_id",
            ),
        ] {
            if value.is_none_or(str::is_empty) {
                issues.push(MethodWorkflowValidationIssue::new(
                    code,
                    path,
                    "closed-loop regulation requires this field",
                ));
            }
        }
    }
    for role in profile
        .actuator_role_id
        .iter()
        .chain(profile.feedback_role_id.iter())
        .chain(profile.monitoring_role_ids.iter())
    {
        if !role_ids.contains(role.as_str()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "unknown_regulation_role",
                "regulation_profile",
                format!("regulation profile references unknown role {role}"),
            ));
        }
    }
    if let Some(variable) = profile.feedback_signal_variable_id.as_deref() {
        if !variable_map.contains_key(variable) {
            issues.push(MethodWorkflowValidationIssue::new(
                "unknown_feedback_variable",
                "feedback_signal_variable_id",
                format!("unknown feedback variable {variable}"),
            ));
        }
    }
    for (value, field) in [
        (profile.fast_increasing_step.value, "fast_increasing_step"),
        (profile.slow_increasing_step.value, "slow_increasing_step"),
        (profile.decreasing_step.value, "decreasing_step"),
        (profile.dwell_t1_seconds, "dwell_t1_seconds"),
        (profile.dwell_t2_seconds, "dwell_t2_seconds"),
        (profile.dwell_t3_seconds, "dwell_t3_seconds"),
    ] {
        if !value.is_finite() || value <= 0.0 {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_regulation_step",
                field,
                "regulation steps and dwell periods must be positive and finite",
            ));
        }
    }
    if profile.retry_policy.maximum_attempts > 100 {
        issues.push(MethodWorkflowValidationIssue::new(
            "unbounded_regulation_retry",
            "retry_policy.maximum_attempts",
            "regulation retry count must be at most 100",
        ));
    }
    issues
}

fn validate_modulations(
    profiles: &[ModulationDefinition],
    issues: &mut Vec<MethodWorkflowValidationIssue>,
) {
    let mut ids = BTreeSet::new();
    for profile in profiles {
        let id = match profile {
            ModulationDefinition::None { profile_id, .. }
            | ModulationDefinition::Am { profile_id, .. }
            | ModulationDefinition::Fm { profile_id, .. }
            | ModulationDefinition::Pulse { profile_id, .. }
            | ModulationDefinition::LaboratoryExtension { profile_id, .. } => profile_id,
        };
        if !valid_token(id) || !ids.insert(id.clone()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_or_duplicate_modulation_profile",
                "modulation_profiles",
                format!("invalid or duplicate modulation profile {id}"),
            ));
        }
        match profile {
            ModulationDefinition::Am { depth_percent, .. }
                if !(0.0..=100.0).contains(depth_percent) =>
            {
                issues.push(MethodWorkflowValidationIssue::new(
                    "invalid_am_depth",
                    "modulation_profiles",
                    "AM depth must be between 0 and 100 percent",
                ));
            }
            _ => {}
        }
    }
}

fn validate_sub_ranges(
    ranges: &[SubRangeDefinition],
    issues: &mut Vec<MethodWorkflowValidationIssue>,
) {
    let mut ids = BTreeSet::new();
    for range in ranges {
        if !valid_token(&range.sub_range_id) || !ids.insert(range.sub_range_id.clone()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_or_duplicate_sub_range",
                "sub_ranges",
                format!("invalid or duplicate sub-range {}", range.sub_range_id),
            ));
        }
        if let Err(issue) = preview_sub_range(range, 10_000) {
            issues.push(issue);
        }
    }
}

fn validate_procedure(
    roots: &[ProcedureNode],
    variable_ids: &BTreeSet<String>,
    issues: &mut Vec<MethodWorkflowValidationIssue>,
) {
    if roots.is_empty() {
        issues.push(MethodWorkflowValidationIssue::new(
            "missing_procedure",
            "procedure",
            "define at least one procedure phase",
        ));
        return;
    }
    let mut ids = BTreeSet::new();
    fn visit(
        node: &ProcedureNode,
        ids: &mut BTreeSet<String>,
        variable_ids: &BTreeSet<String>,
        issues: &mut Vec<MethodWorkflowValidationIssue>,
    ) {
        if !valid_token(&node.node_id) || !ids.insert(node.node_id.clone()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_or_duplicate_procedure_node",
                "procedure",
                format!("invalid or duplicate procedure node {}", node.node_id),
            ));
        }
        if matches!(
            node.node_kind,
            ProcedureNodeKind::Loop | ProcedureNodeKind::Repeat
        ) && node
            .maximum_iterations
            .is_none_or(|value| value == 0 || value > 1_000_000)
        {
            issues.push(MethodWorkflowValidationIssue::new(
                "unbounded_procedure_loop",
                "procedure[].maximum_iterations",
                format!("loop {} needs an explicit finite bound", node.node_id),
            ));
        }
        if node
            .timeout_seconds
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_procedure_timeout",
                "procedure[].timeout_seconds",
                "procedure timeout must be positive and finite",
            ));
        }
        for variable in node
            .input_variables
            .iter()
            .chain(node.output_variables.iter())
        {
            if !variable_ids.contains(variable) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "unknown_procedure_variable",
                    "procedure",
                    format!(
                        "procedure node {} references unknown variable {variable}",
                        node.node_id
                    ),
                ));
            }
        }
        for child in &node.children {
            visit(child, ids, variable_ids, issues);
        }
    }
    for root in roots {
        visit(root, &mut ids, variable_ids, issues);
    }
}

fn validate_limits(
    limits: &[MethodLimitRule],
    variable_ids: &BTreeSet<String>,
    issues: &mut Vec<MethodWorkflowValidationIssue>,
) {
    let mut ids = BTreeSet::new();
    for limit in limits {
        if !valid_token(&limit.limit_id) || !ids.insert(limit.limit_id.clone()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_or_duplicate_limit",
                "limits",
                format!("invalid or duplicate limit {}", limit.limit_id),
            ));
        }
        if !variable_ids.contains(&limit.evaluated_variable_id) {
            issues.push(MethodWorkflowValidationIssue::new(
                "unknown_limit_variable",
                "limits[].evaluated_variable_id",
                format!("limit {} references an unknown variable", limit.limit_id),
            ));
        }
        if matches!(
            limit.classification,
            LimitClassification::Safety | LimitClassification::InstrumentProtection
        ) && limit.contributes_to_verdict
        {
            issues.push(MethodWorkflowValidationIssue::new(
                "runtime_limit_contributes_to_verdict",
                "limits[].contributes_to_verdict",
                "runtime protection limits must remain distinct from final conformity verdicts",
            ));
        }
    }
}

fn validate_post_processing(
    nodes: &[PostProcessingNode],
    variable_ids: &BTreeSet<String>,
    limits: &[MethodLimitRule],
    issues: &mut Vec<MethodWorkflowValidationIssue>,
) {
    let limit_ids: BTreeSet<_> = limits.iter().map(|limit| limit.limit_id.as_str()).collect();
    let mut node_ids = BTreeSet::new();
    let mut outputs = BTreeMap::new();
    for node in nodes {
        if !valid_token(&node.node_id) || !node_ids.insert(node.node_id.clone()) {
            issues.push(MethodWorkflowValidationIssue::new(
                "invalid_or_duplicate_post_processing_node",
                "post_processing",
                format!("invalid or duplicate post-processing node {}", node.node_id),
            ));
        }
        for output in &node.outputs {
            if outputs
                .insert(output.as_str(), node.node_id.as_str())
                .is_some()
            {
                issues.push(MethodWorkflowValidationIssue::new(
                    "duplicate_post_processing_output",
                    "post_processing[].outputs",
                    format!("post-processing output {output} has multiple owners"),
                ));
            }
        }
        if matches!(node.node_kind, PostProcessingNodeKind::LimitComparison)
            && node
                .limit_reference_id
                .as_deref()
                .is_none_or(|reference| !limit_ids.contains(reference))
        {
            issues.push(MethodWorkflowValidationIssue::new(
                "missing_post_processing_limit",
                "post_processing[].limit_reference_id",
                format!(
                    "limit comparison {} needs a valid limit reference",
                    node.node_id
                ),
            ));
        }
    }
    let mut dependencies: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for node in nodes {
        for input in &node.inputs {
            if !variable_ids.contains(input) && !outputs.contains_key(input.as_str()) {
                issues.push(MethodWorkflowValidationIssue::new(
                    "unknown_post_processing_input",
                    "post_processing[].inputs",
                    format!("{} references unknown input {input}", node.node_id),
                ));
            }
            if let Some(producer) = outputs.get(input.as_str()) {
                dependencies
                    .entry(node.node_id.clone())
                    .or_default()
                    .insert((*producer).to_owned());
            }
        }
    }
    for node in dependencies.keys() {
        if dependency_cycle(node, &dependencies) {
            issues.push(MethodWorkflowValidationIssue::new(
                "post_processing_cycle",
                "post_processing",
                format!("post-processing cycle includes {node}"),
            ));
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubRangePreview {
    pub point_count: u64,
    pub preview_points_hz: Vec<f64>,
    pub truncated: bool,
}

pub fn preview_sub_range(
    range: &SubRangeDefinition,
    maximum_points: u64,
) -> Result<SubRangePreview, MethodWorkflowValidationIssue> {
    let start = to_base_unit(&range.start_frequency, QuantityDimension::Frequency)?;
    let stop = to_base_unit(&range.stop_frequency, QuantityDimension::Frequency)?;
    if start <= 0.0 || stop <= 0.0 || start >= stop {
        return Err(MethodWorkflowValidationIssue::new(
            "invalid_sub_range_bounds",
            "sub_ranges",
            "frequency sub-range requires positive start below stop",
        ));
    }
    if range.dwell_seconds < 0.0 || !range.dwell_seconds.is_finite() {
        return Err(MethodWorkflowValidationIssue::new(
            "invalid_sub_range_dwell",
            "sub_ranges[].dwell_seconds",
            "dwell must be finite and non-negative",
        ));
    }
    let (count, preview) = match &range.progression {
        FrequencyProgression::FixedStep { step } => {
            let step = to_base_unit(step, QuantityDimension::Frequency)?;
            if step <= 0.0 {
                return Err(MethodWorkflowValidationIssue::new(
                    "invalid_sub_range_step",
                    "sub_ranges[].progression",
                    "fixed frequency step must be positive",
                ));
            }
            let count = ((stop - start) / step).floor() as u64 + 1;
            (count, bounded_linear_preview(start, step, count))
        }
        FrequencyProgression::Percentage { percentage } => {
            if !percentage.is_finite() || *percentage <= 0.0 {
                return Err(MethodWorkflowValidationIssue::new(
                    "invalid_sub_range_percentage",
                    "sub_ranges[].progression",
                    "percentage progression must be positive",
                ));
            }
            let ratio = 1.0 + percentage / 100.0;
            let count = ((stop / start).ln() / ratio.ln()).floor() as u64 + 1;
            let preview = (0..count.min(12))
                .map(|index| start * ratio.powi(index as i32))
                .collect();
            (count, preview)
        }
        FrequencyProgression::PointsPerDecade { points } => {
            if *points == 0 || *points > 100_000 {
                return Err(MethodWorkflowValidationIssue::new(
                    "invalid_points_per_decade",
                    "sub_ranges[].progression",
                    "points per decade must be between 1 and 100000",
                ));
            }
            let count = ((stop.log10() - start.log10()) * f64::from(*points)).floor() as u64 + 1;
            let ratio = 10_f64.powf(1.0 / f64::from(*points));
            let preview = (0..count.min(12))
                .map(|index| start * ratio.powi(index as i32))
                .collect();
            (count, preview)
        }
        FrequencyProgression::ExplicitList { points } => {
            if points.is_empty() {
                return Err(MethodWorkflowValidationIssue::new(
                    "empty_explicit_frequency_list",
                    "sub_ranges[].progression",
                    "explicit frequency list cannot be empty",
                ));
            }
            let mut converted = Vec::with_capacity(points.len().min(12));
            for point in points.iter().take(12) {
                converted.push(to_base_unit(point, QuantityDimension::Frequency)?);
            }
            (points.len() as u64, converted)
        }
    };
    if count > maximum_points {
        return Err(MethodWorkflowValidationIssue::new(
            "sub_range_preview_limit_exceeded",
            "sub_ranges[].progression",
            format!("range generates {count} points; maximum preview is {maximum_points}"),
        ));
    }
    Ok(SubRangePreview {
        point_count: count,
        preview_points_hz: preview,
        truncated: count > 12,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionAssignmentPin {
    pub role_id: String,
    pub requirement_id: String,
    pub asset_id: String,
    pub asset_revision: String,
    pub equipment_model_revision_id: String,
    pub equipment_model_checksum: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecutionConfigurationDefinition {
    pub definition_schema_version: String,
    pub configuration_id: String,
    pub method_revision: RevisionReference,
    pub measurement_system_template_revision: RevisionReference,
    pub parameter_profile_id: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameter_values: BTreeMap<String, VariableDefaultValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_sub_range_ids: Vec<String>,
    pub laboratory_location_id: String,
    pub planned_use_on: String,
    pub eut_context: String,
    pub station_setup_revision: RevisionReference,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assignments: Vec<ExecutionAssignmentPin>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPlanPhase {
    pub node_id: String,
    pub label: String,
    pub node_kind: ProcedureNodeKind,
    pub depth: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_iterations: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPlanNotice {
    pub code: String,
    pub message: String,
    pub next_action: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPlanPreview {
    pub method_title: String,
    pub system_template_label: String,
    pub ordered_phases: Vec<ExecutionPlanPhase>,
    pub required_roles: Vec<String>,
    pub unresolved_roles: Vec<String>,
    pub regulation_loops: Vec<String>,
    pub expected_signals: Vec<String>,
    pub post_processing_requests: Vec<String>,
    pub blockers: Vec<ExecutionPlanNotice>,
    pub warnings: Vec<ExecutionPlanNotice>,
    pub unsupported_runtime_operations: Vec<String>,
}

pub fn compile_execution_plan(
    method: &TestMethodDefinitionV2,
    system: &MeasurementSystemTemplateDefinition,
    regulation_profiles: &[RegulationProfileDefinition],
    configuration: Option<&ExecutionConfigurationDefinition>,
) -> ExecutionPlanPreview {
    let mut blockers = method
        .validate()
        .into_iter()
        .map(|issue| ExecutionPlanNotice {
            code: issue.code,
            message: issue.message,
            next_action: "Corriger la définition de méthode.".to_owned(),
        })
        .collect::<Vec<_>>();
    blockers.extend(
        system
            .validate(&method.functional_roles, regulation_profiles)
            .into_iter()
            .map(|issue| ExecutionPlanNotice {
                code: issue.code,
                message: issue.message,
                next_action: "Corriger le système de mesure logique.".to_owned(),
            }),
    );
    for profile in regulation_profiles {
        blockers.extend(
            profile
                .validate(&method.variables, &method.functional_roles)
                .into_iter()
                .map(|issue| ExecutionPlanNotice {
                    code: issue.code,
                    message: issue.message,
                    next_action: "Corriger le profil de régulation.".to_owned(),
                }),
        );
    }
    let assigned: BTreeSet<_> = configuration
        .map(|configuration| {
            configuration
                .assignments
                .iter()
                .map(|assignment| assignment.role_id.as_str())
                .collect()
        })
        .unwrap_or_default();
    let unresolved_roles = method
        .functional_roles
        .iter()
        .filter(|role| role.required && !assigned.contains(role.role_id.as_str()))
        .map(|role| role.label.clone())
        .collect::<Vec<_>>();
    if configuration.is_some() {
        blockers.extend(unresolved_roles.iter().map(|role| ExecutionPlanNotice {
            code: "unresolved_execution_role".to_owned(),
            message: format!("Le rôle obligatoire « {role} » n'est pas affecté."),
            next_action: "Affecter un exemplaire apte dans la préparation datée.".to_owned(),
        }));
    }
    let unsupported_runtime_operations = method
        .post_processing
        .iter()
        .filter(|node| !post_processing_runtime_supported(&node.node_kind))
        .map(|node| node.label.clone())
        .collect::<Vec<_>>();
    let warnings = unsupported_runtime_operations
        .iter()
        .map(|operation| ExecutionPlanNotice {
            code: "runtime_operation_not_implemented".to_owned(),
            message: format!("{operation} : définition disponible, exécution prévue dans une version ultérieure."),
            next_action: "Conserver cette opération comme contrat de traitement.".to_owned(),
        })
        .collect();
    let mut ordered_phases = Vec::new();
    flatten_procedure(&method.procedure, 0, &mut ordered_phases);
    ExecutionPlanPreview {
        method_title: method.title.clone(),
        system_template_label: system.label.clone(),
        ordered_phases,
        required_roles: method
            .functional_roles
            .iter()
            .map(|role| role.label.clone())
            .collect(),
        unresolved_roles,
        regulation_loops: system
            .regulation_loops
            .iter()
            .map(|mapping| mapping.label.clone())
            .collect(),
        expected_signals: method
            .variables
            .iter()
            .filter(|variable| {
                matches!(
                    variable.semantic,
                    MethodVariableSemantic::ObservedSignal
                        | MethodVariableSemantic::MonitoringSignal
                        | MethodVariableSemantic::FinalResult
                )
            })
            .map(|variable| variable.label.clone())
            .collect(),
        post_processing_requests: method
            .post_processing
            .iter()
            .map(|node| node.label.clone())
            .collect(),
        blockers,
        warnings,
        unsupported_runtime_operations,
    }
}

fn flatten_procedure(nodes: &[ProcedureNode], depth: u32, output: &mut Vec<ExecutionPlanPhase>) {
    for node in nodes {
        output.push(ExecutionPlanPhase {
            node_id: node.node_id.clone(),
            label: node.label.clone(),
            node_kind: node.node_kind.clone(),
            depth,
            maximum_iterations: node.maximum_iterations,
        });
        flatten_procedure(&node.children, depth + 1, output);
    }
}

fn post_processing_runtime_supported(kind: &PostProcessingNodeKind) -> bool {
    matches!(
        kind,
        PostProcessingNodeKind::UnitConversion
            | PostProcessingNodeKind::Aggregation
            | PostProcessingNodeKind::MaximumSearch
            | PostProcessingNodeKind::Average
            | PostProcessingNodeKind::LimitComparison
            | PostProcessingNodeKind::ReportField
    )
}

fn endpoint_port<'a>(
    nodes: &BTreeMap<&str, &'a MeasurementSystemRoleNode>,
    endpoint: &TopologyPortEndpoint,
) -> Option<&'a MethodLogicalPort> {
    nodes
        .get(endpoint.node_id.as_str())?
        .ports
        .iter()
        .find(|port| port.port_id == endpoint.port_id)
}

fn directions_connect(from: &PortDirectionality, to: &PortDirectionality) -> bool {
    matches!(
        from,
        PortDirectionality::Output
            | PortDirectionality::Bidirectional
            | PortDirectionality::Through
    ) && matches!(
        to,
        PortDirectionality::Input | PortDirectionality::Bidirectional | PortDirectionality::Through
    )
}

fn edge_uses_physical_domain(kind: &TopologyEdgeKind) -> bool {
    matches!(
        kind,
        TopologyEdgeKind::PhysicalSignal
            | TopologyEdgeKind::ExcitationOrPower
            | TopologyEdgeKind::FeedbackMeasurement
            | TopologyEdgeKind::Monitoring
    )
}

fn validate_non_feedback_cycles(
    definition: &MeasurementSystemTemplateDefinition,
    issues: &mut Vec<MethodWorkflowValidationIssue>,
) {
    let node_ids: BTreeSet<_> = definition
        .nodes
        .iter()
        .map(|node| node.node_id.as_str())
        .collect();
    let mut graph: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut incoming: BTreeMap<&str, usize> = node_ids
        .iter()
        .copied()
        .map(|node_id| (node_id, 0))
        .collect();
    for edge in &definition.edges {
        if !matches!(edge.edge_kind, TopologyEdgeKind::FeedbackMeasurement)
            && node_ids.contains(edge.from.node_id.as_str())
            && node_ids.contains(edge.to.node_id.as_str())
        {
            graph
                .entry(&edge.from.node_id)
                .or_default()
                .push(&edge.to.node_id);
            *incoming.entry(&edge.to.node_id).or_default() += 1;
        }
    }
    let mut queue: VecDeque<_> = incoming
        .iter()
        .filter_map(|(node_id, count)| (*count == 0).then_some(*node_id))
        .collect();
    let mut visited = 0;
    while let Some(current) = queue.pop_front() {
        visited += 1;
        if let Some(next) = graph.get(current) {
            for target in next {
                if let Some(count) = incoming.get_mut(target) {
                    *count -= 1;
                    if *count == 0 {
                        queue.push_back(target);
                    }
                }
            }
        }
    }
    if visited != node_ids.len() {
        issues.push(MethodWorkflowValidationIssue::new(
            "undeclared_topology_cycle",
            "edges",
            "topology contains a cycle that is not modeled as a feedback measurement",
        ));
    }
}

fn expression_references(expression: &DeterministicExpression) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    fn visit(expression: &DeterministicExpression, refs: &mut BTreeSet<String>) {
        match expression {
            DeterministicExpression::Variable { variable_id } => {
                refs.insert(variable_id.clone());
            }
            DeterministicExpression::Add { operands }
            | DeterministicExpression::Minimum { operands }
            | DeterministicExpression::Maximum { operands } => {
                for operand in operands {
                    visit(operand, refs);
                }
            }
            DeterministicExpression::Subtract { left, right }
            | DeterministicExpression::Multiply { left, right } => {
                visit(left, refs);
                visit(right, refs);
            }
            DeterministicExpression::Divide {
                numerator,
                denominator,
            } => {
                visit(numerator, refs);
                visit(denominator, refs);
            }
            DeterministicExpression::Literal { .. } => {}
        }
    }
    visit(expression, &mut refs);
    refs
}

fn expression_dimension(
    expression: &DeterministicExpression,
    variables: &BTreeMap<&str, &MethodVariableDefinition>,
) -> Result<QuantityDimension, String> {
    match expression {
        DeterministicExpression::Literal { unit, .. } => {
            unit_dimension(unit).ok_or_else(|| format!("unsupported expression unit {unit}"))
        }
        DeterministicExpression::Variable { variable_id } => variables
            .get(variable_id.as_str())
            .map(|variable| variable.dimension)
            .ok_or_else(|| format!("unknown expression variable {variable_id}")),
        DeterministicExpression::Add { operands }
        | DeterministicExpression::Minimum { operands }
        | DeterministicExpression::Maximum { operands } => {
            let Some(first) = operands.first() else {
                return Err("expression requires at least one operand".to_owned());
            };
            let dimension = expression_dimension(first, variables)?;
            if operands
                .iter()
                .skip(1)
                .any(|operand| expression_dimension(operand, variables).ok() != Some(dimension))
            {
                return Err("expression combines incompatible dimensions".to_owned());
            }
            Ok(dimension)
        }
        DeterministicExpression::Subtract { left, right } => {
            let left = expression_dimension(left, variables)?;
            let right = expression_dimension(right, variables)?;
            if left != right {
                return Err("subtraction requires compatible dimensions".to_owned());
            }
            Ok(left)
        }
        DeterministicExpression::Multiply { left, right } => {
            let left = expression_dimension(left, variables)?;
            let right = expression_dimension(right, variables)?;
            match (left, right) {
                (QuantityDimension::Dimensionless | QuantityDimension::Ratio, other)
                | (other, QuantityDimension::Dimensionless | QuantityDimension::Ratio) => Ok(other),
                _ => Err(
                    "multiplication is limited to a scalar and one physical quantity".to_owned(),
                ),
            }
        }
        DeterministicExpression::Divide {
            numerator,
            denominator,
        } => {
            let numerator = expression_dimension(numerator, variables)?;
            let denominator = expression_dimension(denominator, variables)?;
            if numerator == denominator {
                Ok(QuantityDimension::Dimensionless)
            } else if matches!(
                denominator,
                QuantityDimension::Dimensionless | QuantityDimension::Ratio
            ) {
                Ok(numerator)
            } else {
                Err("division uses incompatible dimensions".to_owned())
            }
        }
    }
}

fn contains_literal_zero_denominator(expression: &DeterministicExpression) -> bool {
    match expression {
        DeterministicExpression::Divide {
            numerator,
            denominator,
        } => {
            matches!(denominator.as_ref(), DeterministicExpression::Literal { value, .. } if *value == 0.0)
                || contains_literal_zero_denominator(numerator)
                || contains_literal_zero_denominator(denominator)
        }
        DeterministicExpression::Add { operands }
        | DeterministicExpression::Minimum { operands }
        | DeterministicExpression::Maximum { operands } => {
            operands.iter().any(contains_literal_zero_denominator)
        }
        DeterministicExpression::Subtract { left, right }
        | DeterministicExpression::Multiply { left, right } => {
            contains_literal_zero_denominator(left) || contains_literal_zero_denominator(right)
        }
        DeterministicExpression::Literal { .. } | DeterministicExpression::Variable { .. } => false,
    }
}

fn dependency_cycle(start: &str, dependencies: &BTreeMap<String, BTreeSet<String>>) -> bool {
    fn visit(
        current: &str,
        start: &str,
        dependencies: &BTreeMap<String, BTreeSet<String>>,
        seen: &mut BTreeSet<String>,
    ) -> bool {
        if !seen.insert(current.to_owned()) {
            return current == start;
        }
        dependencies.get(current).is_some_and(|next| {
            next.iter().any(|dependency| {
                dependency == start || visit(dependency, start, dependencies, &mut seen.clone())
            })
        })
    }
    visit(start, start, dependencies, &mut BTreeSet::new())
}

fn unit_dimension(unit: &str) -> Option<QuantityDimension> {
    match unit.trim() {
        "" | "1" | "%" => Some(QuantityDimension::Dimensionless),
        "Hz" | "kHz" | "MHz" | "GHz" => Some(QuantityDimension::Frequency),
        "s" | "ms" | "us" | "ns" => Some(QuantityDimension::Time),
        "V" | "mV" | "uV" | "dBuV" => Some(QuantityDimension::Voltage),
        "A" | "mA" | "uA" => Some(QuantityDimension::Current),
        "W" | "mW" | "dBm" => Some(QuantityDimension::Power),
        "V/m" | "dBuV/m" => Some(QuantityDimension::ElectricFieldStrength),
        "A/m" => Some(QuantityDimension::MagneticFieldStrength),
        "dB" => Some(QuantityDimension::Ratio),
        _ => None,
    }
}

fn to_base_unit(
    quantity: &QuantityValue,
    expected: QuantityDimension,
) -> Result<f64, MethodWorkflowValidationIssue> {
    if unit_dimension(&quantity.unit) != Some(expected) || !quantity.value.is_finite() {
        return Err(MethodWorkflowValidationIssue::new(
            "incompatible_or_invalid_unit",
            "quantity",
            format!(
                "{} is incompatible with the required dimension",
                quantity.unit
            ),
        ));
    }
    let factor = match quantity.unit.as_str() {
        "Hz" | "s" | "V" | "A" | "W" | "V/m" | "A/m" | "dB" | "1" | "%" | "" => 1.0,
        "kHz" => 1e3,
        "MHz" => 1e6,
        "GHz" => 1e9,
        "ms" | "mV" | "mA" | "mW" => 1e-3,
        "us" | "uV" | "uA" => 1e-6,
        "ns" => 1e-9,
        "dBuV" | "dBm" | "dBuV/m" => 1.0,
        _ => 1.0,
    };
    Ok(quantity.value * factor)
}

fn bounded_linear_preview(start: f64, step: f64, count: u64) -> Vec<f64> {
    (0..count.min(12))
        .map(|index| start + step * index as f64)
        .collect()
}

fn requires_unit(dimension: QuantityDimension) -> bool {
    !matches!(
        dimension,
        QuantityDimension::Dimensionless | QuantityDimension::Text | QuantityDimension::Boolean
    )
}

fn require_text(issues: &mut Vec<MethodWorkflowValidationIssue>, value: &str, path: &str) {
    if value.trim().is_empty() {
        issues.push(MethodWorkflowValidationIssue::new(
            "missing_required_text",
            path,
            format!("{path} is required"),
        ));
    }
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
}

fn valid_checksum(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, 'a'..='f'))
}

fn valid_iso_date(value: &str) -> bool {
    if !value.is_ascii() || value.len() != 10 || &value[4..5] != "-" || &value[7..8] != "-" {
        return false;
    }
    let year = value[0..4].parse::<u32>().ok();
    let month = value[5..7].parse::<u32>().ok();
    let day = value[8..10].parse::<u32>().ok();
    year.is_some_and(|year| year >= 2000)
        && month.is_some_and(|month| (1..=12).contains(&month))
        && day.is_some_and(|day| (1..=31).contains(&day))
}

fn canonicalize<T: Serialize>(
    value: &T,
    schema: &str,
) -> Result<CanonicalMethodDefinition, MethodWorkflowValidationIssue> {
    let mut json = serde_json::to_value(value).map_err(|error| {
        MethodWorkflowValidationIssue::new("method_serialization_failed", "$", error.to_string())
    })?;
    canonicalize_json(&mut json);
    let canonical_json = serde_json::to_string(&json).map_err(|error| {
        MethodWorkflowValidationIssue::new("method_serialization_failed", "$", error.to_string())
    })?;
    let digest = Sha256::digest(canonical_json.as_bytes());
    Ok(CanonicalMethodDefinition {
        definition_schema_version: schema.to_owned(),
        canonical_json,
        definition_checksum: format!("sha256:{digest:x}"),
    })
}

fn canonicalize_json(value: &mut Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                canonicalize_json(item);
            }
        }
        Value::Object(map) => {
            let previous = std::mem::take(map);
            let mut sorted = BTreeMap::new();
            for (key, mut value) in previous {
                canonicalize_json(&mut value);
                sorted.insert(key, value);
            }
            map.extend(sorted);
        }
        _ => {}
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variable(
        id: &str,
        semantic: MethodVariableSemantic,
        unit: &str,
    ) -> MethodVariableDefinition {
        MethodVariableDefinition {
            variable_id: id.to_owned(),
            label: id.replace('_', " "),
            description: format!("Variable {id}"),
            semantic,
            value_type: VariableValueType::Number,
            dimension: if unit == "dB" {
                QuantityDimension::Ratio
            } else {
                QuantityDimension::Frequency
            },
            unit: Some(unit.to_owned()),
            default_value: Some(VariableDefaultValue::Number(1.0)),
            minimum: Some(0.0),
            maximum: Some(1e9),
            enum_values: Vec::new(),
            required: true,
            source: "laboratory_method".to_owned(),
            availability_phase: AvailabilityPhase::Preparation,
            expression: None,
            consumers: Vec::new(),
        }
    }

    fn port(id: &str, directionality: PortDirectionality) -> MethodLogicalPort {
        MethodLogicalPort {
            port_id: id.to_owned(),
            label: id.to_owned(),
            directionality,
            signal_domain: SignalDomain::Rf,
            connector: Some("N".to_owned()),
            impedance_ohm: Some(50.0),
            quantity_dimension: Some(QuantityDimension::Power),
        }
    }

    fn role(id: &str) -> MethodFunctionalRole {
        MethodFunctionalRole {
            role_id: id.to_owned(),
            label: id.replace('_', " "),
            purpose: format!("Fonction {id}"),
            required: true,
            functional_category: "rf_chain".to_owned(),
            capabilities: vec![MethodCapabilityRequirement {
                capability_kind: "controlled_rf_output".to_owned(),
                frequency_range: Some(TechnicalRangeRequirement {
                    minimum: Some(150.0),
                    maximum: Some(230.0),
                    unit: "MHz".to_owned(),
                }),
                voltage_range: None,
                current_range: None,
                power_range: None,
                operating_modes: vec!["cw".to_owned()],
                required_driver_action: Some("set_frequency".to_owned()),
            }],
            calibration_policy: MethodCalibrationPolicy::IfUsed,
            substitution_policy: MethodSubstitutionPolicy::SameCapabilities,
            assignment_stage: RoleAssignmentStage::PlannedTestPreparation,
            logical_ports: vec![
                port("input", PortDirectionality::Input),
                port("output", PortDirectionality::Output),
            ],
            consumes_variables: Vec::new(),
            produces_variables: Vec::new(),
        }
    }

    fn procedure() -> Vec<ProcedureNode> {
        vec![ProcedureNode {
            node_id: "phase_main".to_owned(),
            label: "Phase principale".to_owned(),
            purpose: "Parcourir les sous-plages".to_owned(),
            node_kind: ProcedureNodeKind::Phase,
            enabled_condition: None,
            input_variables: Vec::new(),
            output_variables: Vec::new(),
            timeout_seconds: Some(600.0),
            failure_policy: ProcedureFailurePolicy::SafeShutdown,
            maximum_iterations: None,
            children: vec![ProcedureNode {
                node_id: "frequency_loop".to_owned(),
                label: "Boucle fréquence".to_owned(),
                purpose: "Parcourir les points".to_owned(),
                node_kind: ProcedureNodeKind::Loop,
                enabled_condition: None,
                input_variables: Vec::new(),
                output_variables: Vec::new(),
                timeout_seconds: Some(300.0),
                failure_policy: ProcedureFailurePolicy::SafeShutdown,
                maximum_iterations: Some(1000),
                children: Vec::new(),
                audit_notes: String::new(),
            }],
            audit_notes: String::new(),
        }]
    }

    fn method() -> TestMethodDefinitionV2 {
        let frequency = variable("frequency", MethodVariableSemantic::MethodParameter, "MHz");
        TestMethodDefinitionV2 {
            definition_schema_version: TEST_METHOD_DEFINITION_SCHEMA_VERSION.to_owned(),
            title: "Immunité conduite démonstrative".to_owned(),
            objective: "Définir une procédure de laboratoire sans revendiquer une norme."
                .to_owned(),
            scope: "Préparation et compilation à blanc.".to_owned(),
            classification_path: vec!["Immunité".to_owned(), "Immunité conduite".to_owned()],
            standard_references: Vec::new(),
            variables: vec![frequency],
            lock_policy: Vec::new(),
            parameter_profiles: vec![ParameterProfileDefinition {
                profile_id: "default".to_owned(),
                label: "Profil nominal".to_owned(),
                values: BTreeMap::new(),
            }],
            functional_roles: vec![role("generator"), role("monitor")],
            measurement_system_templates: Vec::new(),
            regulation_profiles: Vec::new(),
            modulation_profiles: vec![ModulationDefinition::None {
                profile_id: "none".to_owned(),
                label: "Sans modulation".to_owned(),
            }],
            sub_ranges: vec![SubRangeDefinition {
                sub_range_id: "band_150_230".to_owned(),
                label: "150 à 230 MHz".to_owned(),
                start_frequency: QuantityValue {
                    value: 150.0,
                    unit: "MHz".to_owned(),
                },
                stop_frequency: QuantityValue {
                    value: 230.0,
                    unit: "MHz".to_owned(),
                },
                include_start: true,
                include_stop: true,
                progression: FrequencyProgression::FixedStep {
                    step: QuantityValue {
                        value: 1.0,
                        unit: "MHz".to_owned(),
                    },
                },
                direction: SweepDirection::Increasing,
                sweep_mode: "stepped".to_owned(),
                dwell_seconds: 1.0,
                special_dwell_seconds: None,
                include_frequencies: Vec::new(),
                exclude_frequencies: Vec::new(),
                system_template_profile_id: None,
                regulation_profile_id: None,
                modulation_profile_id: Some("none".to_owned()),
                eut_state: Some("nominal".to_owned()),
                orientation: None,
                polarization: None,
                comments: String::new(),
            }],
            procedure: procedure(),
            limits: Vec::new(),
            post_processing: Vec::new(),
            expected_output_variables: Vec::new(),
            migration_evidence: Vec::new(),
        }
    }

    fn regulation() -> RegulationProfileDefinition {
        RegulationProfileDefinition {
            definition_schema_version: REGULATION_PROFILE_SCHEMA_VERSION.to_owned(),
            profile_id: "closed_loop".to_owned(),
            label: "Régulation fermée".to_owned(),
            regulated_quantity: "Niveau injecté".to_owned(),
            regulated_unit: "dB".to_owned(),
            target_expression: DeterministicExpression::Literal {
                value: 10.0,
                unit: "dB".to_owned(),
            },
            tolerance_band: QuantityValue {
                value: 1.0,
                unit: "dB".to_owned(),
            },
            control_mode: RegulationControlMode::ClosedLoop,
            actuator_role_id: Some("generator".to_owned()),
            required_driver_action: Some("set_level".to_owned()),
            feedback_role_id: Some("monitor".to_owned()),
            feedback_signal_variable_id: None,
            monitoring_role_ids: Vec::new(),
            calibration_reference_profile: None,
            start_threshold: QuantityValue {
                value: -20.0,
                unit: "dB".to_owned(),
            },
            fast_increasing_step: QuantityValue {
                value: 3.0,
                unit: "dB".to_owned(),
            },
            slow_increasing_step: QuantityValue {
                value: 0.5,
                unit: "dB".to_owned(),
            },
            decreasing_step: QuantityValue {
                value: 1.0,
                unit: "dB".to_owned(),
            },
            dwell_t1_seconds: 0.1,
            dwell_t2_seconds: 0.2,
            dwell_t3_seconds: 1.0,
            regulation_start_criterion: "start_below_target".to_owned(),
            regulation_end_criterion: "inside_tolerance".to_owned(),
            regulation_factor: 1.0,
            maximum_output: None,
            maximum_forward_power: None,
            maximum_reflected_power: None,
            overshoot_limit: Some(QuantityValue {
                value: 2.0,
                unit: "dB".to_owned(),
            }),
            retry_policy: RetryPolicy {
                maximum_attempts: 3,
                on_exhaustion: "abort".to_owned(),
            },
            abort_policy: "safe_shutdown".to_owned(),
            safe_state_policy: "generator_output_off".to_owned(),
            operator_notes: String::new(),
        }
    }

    fn system(profile: &RegulationProfileDefinition) -> MeasurementSystemTemplateDefinition {
        MeasurementSystemTemplateDefinition {
            definition_schema_version: MEASUREMENT_SYSTEM_TEMPLATE_SCHEMA_VERSION.to_owned(),
            template_id: "system_conducted_immunity".to_owned(),
            label: "Chaîne d'immunité conduite".to_owned(),
            classification: "conducted_immunity".to_owned(),
            nodes: vec![
                MeasurementSystemRoleNode {
                    node_id: "generator".to_owned(),
                    label: "Générateur".to_owned(),
                    role_type: "generator".to_owned(),
                    method_role_id: Some("generator".to_owned()),
                    ports: vec![
                        port("output", PortDirectionality::Output),
                        port("feedback", PortDirectionality::Input),
                    ],
                    notes: String::new(),
                },
                MeasurementSystemRoleNode {
                    node_id: "monitor".to_owned(),
                    label: "Mesure de perturbation".to_owned(),
                    role_type: "monitor".to_owned(),
                    method_role_id: Some("monitor".to_owned()),
                    ports: vec![
                        port("input", PortDirectionality::Input),
                        port("feedback", PortDirectionality::Output),
                    ],
                    notes: String::new(),
                },
            ],
            edges: vec![
                MeasurementSystemEdge {
                    edge_id: "signal".to_owned(),
                    label: "Signal appliqué".to_owned(),
                    edge_kind: TopologyEdgeKind::PhysicalSignal,
                    from: TopologyPortEndpoint {
                        node_id: "generator".to_owned(),
                        port_id: "output".to_owned(),
                    },
                    to: TopologyPortEndpoint {
                        node_id: "monitor".to_owned(),
                        port_id: "input".to_owned(),
                    },
                },
                MeasurementSystemEdge {
                    edge_id: "feedback".to_owned(),
                    label: "Retour".to_owned(),
                    edge_kind: TopologyEdgeKind::FeedbackMeasurement,
                    from: TopologyPortEndpoint {
                        node_id: "monitor".to_owned(),
                        port_id: "feedback".to_owned(),
                    },
                    to: TopologyPortEndpoint {
                        node_id: "generator".to_owned(),
                        port_id: "feedback".to_owned(),
                    },
                },
            ],
            correction_points: Vec::new(),
            regulation_loops: vec![RegulationLoopMapping {
                loop_id: "loop_main".to_owned(),
                label: "Boucle principale".to_owned(),
                regulation_profile_id: profile.profile_id.clone(),
                actuator_node_id: "generator".to_owned(),
                feedback_node_id: "monitor".to_owned(),
                monitoring_node_ids: Vec::new(),
            }],
            notes: String::new(),
        }
    }

    #[test]
    fn hierarchy_rejects_cycles_and_accepts_stable_tree() {
        let mut nodes = vec![
            MethodHierarchyNode {
                node_id: "immunity".to_owned(),
                parent_node_id: None,
                node_kind: MethodHierarchyNodeKind::Domain,
                label: "Immunité".to_owned(),
                position: 10,
                archived: false,
            },
            MethodHierarchyNode {
                node_id: "conducted".to_owned(),
                parent_node_id: Some("immunity".to_owned()),
                node_kind: MethodHierarchyNodeKind::TestFamily,
                label: "Conduite".to_owned(),
                position: 10,
                archived: false,
            },
        ];
        assert!(validate_method_hierarchy(&nodes).is_empty());
        nodes[0].parent_node_id = Some("conducted".to_owned());
        assert!(validate_method_hierarchy(&nodes)
            .iter()
            .any(|issue| issue.code == "method_hierarchy_cycle"));
    }

    #[test]
    fn method_v2_is_canonical_and_checksum_stable() {
        let method = method();
        let first = method.canonicalize().unwrap();
        let second = method.canonicalize().unwrap();
        assert_eq!(first.canonical_json, second.canonical_json);
        assert_eq!(first.definition_checksum, second.definition_checksum);
        assert!(first.definition_checksum.starts_with("sha256:"));
    }

    #[test]
    fn versioned_method_keeps_v1_compatibility() {
        let legacy = crate::test_definitions::tests::fixture_definition();
        let json = serde_json::to_string(&legacy).unwrap();
        let versioned = VersionedMethodDefinition::from_json_str(&json).unwrap();
        assert!(matches!(versioned, VersionedMethodDefinition::V1(_)));
        assert_eq!(
            versioned.canonicalize().unwrap().definition_schema_version,
            crate::test_definitions::TEST_TEMPLATE_DEFINITION_SCHEMA_VERSION
        );
    }

    #[test]
    fn legacy_successor_maps_roles_without_guessing_topology() {
        let legacy = crate::test_definitions::tests::fixture_definition();
        let successor = derive_method_v2_successor(&legacy).unwrap();
        assert_eq!(successor.functional_roles.len(), 2);
        assert!(successor.measurement_system_templates.is_empty());
        assert!(successor.limits.is_empty());
        assert!(successor
            .migration_evidence
            .iter()
            .any(|evidence| evidence.contains("topology")));
        successor.canonicalize().unwrap();
    }

    #[test]
    fn variables_reject_dependency_cycles_and_unsafe_division() {
        let mut method = method();
        let mut left = variable("left", MethodVariableSemantic::DerivedValue, "MHz");
        left.expression = Some(DeterministicExpression::Variable {
            variable_id: "right".to_owned(),
        });
        let mut right = variable("right", MethodVariableSemantic::DerivedValue, "MHz");
        right.expression = Some(DeterministicExpression::Divide {
            numerator: Box::new(DeterministicExpression::Variable {
                variable_id: "left".to_owned(),
            }),
            denominator: Box::new(DeterministicExpression::Literal {
                value: 0.0,
                unit: "1".to_owned(),
            }),
        });
        method.variables.extend([left, right]);
        let issues = method.validate();
        assert!(issues
            .iter()
            .any(|issue| issue.code == "variable_dependency_cycle"));
        assert!(issues
            .iter()
            .any(|issue| issue.code == "unsafe_division_by_zero"));
    }

    #[test]
    fn closed_loop_requires_feedback_and_driver_action() {
        let method = method();
        let mut profile = regulation();
        profile.feedback_role_id = None;
        profile.required_driver_action = None;
        let issues = profile.validate(&method.variables, &method.functional_roles);
        assert!(issues
            .iter()
            .any(|issue| issue.code == "missing_regulation_feedback"));
        assert!(issues
            .iter()
            .any(|issue| issue.code == "missing_regulation_driver_action"));
    }

    #[test]
    fn topology_accepts_declared_feedback_but_rejects_unreachable_feedback() {
        let method = method();
        let profile = regulation();
        let valid = system(&profile);
        assert!(valid
            .validate(&method.functional_roles, std::slice::from_ref(&profile))
            .is_empty());
        let mut invalid = valid;
        invalid
            .edges
            .retain(|edge| edge.edge_kind != TopologyEdgeKind::FeedbackMeasurement);
        assert!(invalid
            .validate(&method.functional_roles, &[profile])
            .iter()
            .any(|issue| issue.code == "unreachable_regulation_feedback"));
    }

    #[test]
    fn procedure_rejects_unbounded_loop() {
        let mut method = method();
        method.procedure[0].children[0].maximum_iterations = None;
        assert!(method
            .validate()
            .iter()
            .any(|issue| issue.code == "unbounded_procedure_loop"));
    }

    #[test]
    fn sub_range_preview_is_bounded_and_unit_aware() {
        let range = &method().sub_ranges[0];
        let preview = preview_sub_range(range, 1000).unwrap();
        assert_eq!(preview.point_count, 81);
        assert!(preview.truncated);
        assert_eq!(preview.preview_points_hz[0], 150_000_000.0);
        assert!(
            preview_sub_range(range, 10).unwrap_err().code == "sub_range_preview_limit_exceeded"
        );
    }

    #[test]
    fn runtime_abort_is_distinct_from_final_verdict() {
        let mut method = method();
        method.limits.push(MethodLimitRule {
            limit_id: "power_abort".to_owned(),
            label: "Protection puissance".to_owned(),
            classification: LimitClassification::InstrumentProtection,
            evaluated_variable_id: "frequency".to_owned(),
            comparison: LimitComparison::GreaterThan,
            threshold: QuantityValue {
                value: 1.0,
                unit: "MHz".to_owned(),
            },
            secondary_threshold: None,
            phase: AvailabilityPhase::Execution,
            severity: "blocking".to_owned(),
            action: LimitAction::Abort,
            contributes_to_verdict: true,
            explanation: "Protection".to_owned(),
        });
        assert!(method
            .validate()
            .iter()
            .any(|issue| issue.code == "runtime_limit_contributes_to_verdict"));
    }

    #[test]
    fn post_processing_rejects_cycles_and_missing_limit() {
        let mut method = method();
        method.post_processing = vec![
            PostProcessingNode {
                node_id: "a".to_owned(),
                label: "A".to_owned(),
                node_kind: PostProcessingNodeKind::Average,
                inputs: vec!["b_out".to_owned()],
                outputs: vec!["a_out".to_owned()],
                parameters: BTreeMap::new(),
                limit_reference_id: None,
            },
            PostProcessingNode {
                node_id: "b".to_owned(),
                label: "B".to_owned(),
                node_kind: PostProcessingNodeKind::LimitComparison,
                inputs: vec!["a_out".to_owned()],
                outputs: vec!["b_out".to_owned()],
                parameters: BTreeMap::new(),
                limit_reference_id: None,
            },
        ];
        let issues = method.validate();
        assert!(issues
            .iter()
            .any(|issue| issue.code == "post_processing_cycle"));
        assert!(issues
            .iter()
            .any(|issue| issue.code == "missing_post_processing_limit"));
    }

    #[test]
    fn execution_preview_explains_unresolved_roles_and_unsupported_runtime() {
        let mut method = method();
        method.post_processing.push(PostProcessingNode {
            node_id: "fft".to_owned(),
            label: "FFT".to_owned(),
            node_kind: PostProcessingNodeKind::FftRequest,
            inputs: vec!["frequency".to_owned()],
            outputs: vec!["spectrum".to_owned()],
            parameters: BTreeMap::new(),
            limit_reference_id: None,
        });
        let profile = regulation();
        let system = system(&profile);
        let configuration = ExecutionConfigurationDefinition {
            definition_schema_version: EXECUTION_CONFIGURATION_SCHEMA_VERSION.to_owned(),
            configuration_id: "execution_001".to_owned(),
            method_revision: reference("method"),
            measurement_system_template_revision: reference("system"),
            parameter_profile_id: "default".to_owned(),
            parameter_values: BTreeMap::new(),
            selected_sub_range_ids: vec!["band_150_230".to_owned()],
            laboratory_location_id: "lab_1".to_owned(),
            planned_use_on: "2026-08-05".to_owned(),
            eut_context: "Prototype client".to_owned(),
            station_setup_revision: reference("station"),
            assignments: Vec::new(),
        };
        let preview = compile_execution_plan(&method, &system, &[profile], Some(&configuration));
        assert_eq!(preview.unresolved_roles.len(), 2);
        assert_eq!(preview.unsupported_runtime_operations, vec!["FFT"]);
        assert!(preview
            .blockers
            .iter()
            .any(|issue| issue.code == "unresolved_execution_role"));
    }

    #[test]
    fn execution_configuration_pins_exact_revisions_and_rejects_unknown_ranges() {
        let method = method();
        let profile = regulation();
        let system = system(&profile);
        let mut configuration = ExecutionConfigurationDefinition {
            definition_schema_version: EXECUTION_CONFIGURATION_SCHEMA_VERSION.to_owned(),
            configuration_id: "execution_002".to_owned(),
            method_revision: reference("method"),
            measurement_system_template_revision: reference(&system.template_id),
            parameter_profile_id: "default".to_owned(),
            parameter_values: BTreeMap::new(),
            selected_sub_range_ids: vec!["band_150_230".to_owned()],
            laboratory_location_id: "lab_1".to_owned(),
            planned_use_on: "2026-08-05".to_owned(),
            eut_context: "Prototype client".to_owned(),
            station_setup_revision: reference("station"),
            assignments: Vec::new(),
        };
        assert!(configuration.validate(&method, &system).is_empty());
        configuration.canonicalize(&method, &system).unwrap();
        configuration
            .selected_sub_range_ids
            .push("missing_band".to_owned());
        assert!(configuration
            .validate(&method, &system)
            .iter()
            .any(|issue| issue.code == "unknown_execution_sub_range"));
    }

    fn reference(id: &str) -> RevisionReference {
        RevisionReference {
            identity_id: id.to_owned(),
            revision_id: format!("{id}-rev-0001"),
            definition_checksum: format!("sha256:{}", "a".repeat(64)),
        }
    }
}
