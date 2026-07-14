use crate::equipment::{
    is_logarithmic_unit, unit_quantity, DefinitionValidationIssue, PhysicalQuantity, SignalDomain,
    TechnologyTag,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SENSOR_DEFINITION_SCHEMA_VERSION: &str = "emc-locus.sensor-definition.v1";
pub const SCALING_PROFILE_DEFINITION_SCHEMA_VERSION: &str =
    "emc-locus.scaling-profile-definition.v1";
pub const ENGINEERING_CURVE_DEFINITION_SCHEMA_VERSION: &str =
    "emc-locus.engineering-curve-definition.v1";
pub const DAQ_CHANNEL_PROFILE_DEFINITION_SCHEMA_VERSION: &str =
    "emc-locus.daq-channel-profile-definition.v1";
pub const ACQUISITION_CHANNEL_RECIPE_DEFINITION_SCHEMA_VERSION: &str =
    "emc-locus.acquisition-channel-recipe-definition.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementEngineeringRevisionStatus {
    Draft,
    UnderReview,
    Approved,
    Superseded,
    Suspended,
    Retired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementEngineeringAggregateKind {
    SensorDefinition,
    ScalingProfile,
    EngineeringCurve,
    DaqChannelProfile,
    AcquisitionChannelRecipe,
}

impl MeasurementEngineeringAggregateKind {
    pub fn schema_version(self) -> &'static str {
        match self {
            Self::SensorDefinition => SENSOR_DEFINITION_SCHEMA_VERSION,
            Self::ScalingProfile => SCALING_PROFILE_DEFINITION_SCHEMA_VERSION,
            Self::EngineeringCurve => ENGINEERING_CURVE_DEFINITION_SCHEMA_VERSION,
            Self::DaqChannelProfile => DAQ_CHANNEL_PROFILE_DEFINITION_SCHEMA_VERSION,
            Self::AcquisitionChannelRecipe => ACQUISITION_CHANNEL_RECIPE_DEFINITION_SCHEMA_VERSION,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SensorDefinition => "sensor_definition",
            Self::ScalingProfile => "scaling_profile",
            Self::EngineeringCurve => "engineering_curve",
            Self::DaqChannelProfile => "daq_channel_profile",
            Self::AcquisitionChannelRecipe => "acquisition_channel_recipe",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalMeasurementEngineeringDefinition {
    pub aggregate_kind: MeasurementEngineeringAggregateKind,
    pub entity_id: String,
    pub label: String,
    pub summary_kind: String,
    pub definition_schema_version: String,
    pub canonical_json: String,
    pub definition_checksum: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MeasurementEngineeringDefinition {
    Sensor(SensorDefinition),
    Scaling(ScalingProfileDefinition),
    Curve(EngineeringCurveDefinition),
    Daq(DaqChannelProfileDefinition),
    Recipe(AcquisitionChannelRecipeDefinition),
}

impl MeasurementEngineeringDefinition {
    pub fn from_json_str(
        kind: MeasurementEngineeringAggregateKind,
        value: &str,
    ) -> Result<Self, DefinitionValidationIssue> {
        match kind {
            MeasurementEngineeringAggregateKind::SensorDefinition => {
                serde_json::from_str::<SensorDefinition>(value)
                    .map(Self::Sensor)
                    .map_err(|error| parse_issue(kind, error.to_string()))
            }
            MeasurementEngineeringAggregateKind::ScalingProfile => {
                serde_json::from_str::<ScalingProfileDefinition>(value)
                    .map(Self::Scaling)
                    .map_err(|error| parse_issue(kind, error.to_string()))
            }
            MeasurementEngineeringAggregateKind::EngineeringCurve => {
                serde_json::from_str::<EngineeringCurveDefinition>(value)
                    .map(Self::Curve)
                    .map_err(|error| parse_issue(kind, error.to_string()))
            }
            MeasurementEngineeringAggregateKind::DaqChannelProfile => {
                serde_json::from_str::<DaqChannelProfileDefinition>(value)
                    .map(Self::Daq)
                    .map_err(|error| parse_issue(kind, error.to_string()))
            }
            MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe => {
                serde_json::from_str::<AcquisitionChannelRecipeDefinition>(value)
                    .map(Self::Recipe)
                    .map_err(|error| parse_issue(kind, error.to_string()))
            }
        }
    }

    pub fn aggregate_kind(&self) -> MeasurementEngineeringAggregateKind {
        match self {
            Self::Sensor(_) => MeasurementEngineeringAggregateKind::SensorDefinition,
            Self::Scaling(_) => MeasurementEngineeringAggregateKind::ScalingProfile,
            Self::Curve(_) => MeasurementEngineeringAggregateKind::EngineeringCurve,
            Self::Daq(_) => MeasurementEngineeringAggregateKind::DaqChannelProfile,
            Self::Recipe(_) => MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe,
        }
    }

    pub fn entity_id(&self) -> &str {
        match self {
            Self::Sensor(definition) => &definition.sensor_definition_id,
            Self::Scaling(definition) => &definition.scaling_profile_id,
            Self::Curve(definition) => &definition.curve_id,
            Self::Daq(definition) => &definition.daq_channel_profile_id,
            Self::Recipe(definition) => &definition.recipe_id,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Sensor(definition) => format!(
                "{} {}",
                definition.manufacturer.trim(),
                definition.model_name.trim()
            )
            .trim()
            .to_owned(),
            Self::Scaling(definition) => definition.label.clone(),
            Self::Curve(definition) => definition.label.clone(),
            Self::Daq(definition) => definition.label.clone(),
            Self::Recipe(definition) => definition.label.clone(),
        }
    }

    pub fn summary_kind(&self) -> String {
        match self {
            Self::Sensor(definition) => format!("{:?}", definition.sensor_family),
            Self::Scaling(definition) => format!("{:?}", definition.scaling_kind),
            Self::Curve(definition) => format!("{:?}", definition.curve_type),
            Self::Daq(definition) => format!("{:?}", definition.channel_kind),
            Self::Recipe(_) => "logical_channel_recipe".to_owned(),
        }
    }

    pub fn validate_all(&self) -> Vec<DefinitionValidationIssue> {
        match self {
            Self::Sensor(definition) => validate_sensor_definition(definition),
            Self::Scaling(definition) => validate_scaling_profile_definition(definition),
            Self::Curve(definition) => validate_engineering_curve_definition(definition),
            Self::Daq(definition) => validate_daq_channel_profile_definition(definition),
            Self::Recipe(definition) => validate_acquisition_channel_recipe_definition(definition),
        }
    }

    pub fn canonicalize(
        &self,
    ) -> Result<CanonicalMeasurementEngineeringDefinition, Vec<DefinitionValidationIssue>> {
        let issues = self.validate_all();
        if issues.iter().any(|item| item.severity == "error") {
            return Err(issues);
        }
        let mut value = match self {
            Self::Sensor(definition) => serde_json::to_value(definition),
            Self::Scaling(definition) => serde_json::to_value(definition),
            Self::Curve(definition) => serde_json::to_value(definition),
            Self::Daq(definition) => serde_json::to_value(definition),
            Self::Recipe(definition) => serde_json::to_value(definition),
        }
        .map_err(|error| {
            vec![issue(
                "error",
                "measurement_engineering_definition_serialization_failed",
                "$",
                error.to_string(),
                None::<String>,
            )]
        })?;
        canonicalize_json_value(&mut value);
        let canonical_json = serde_json::to_string(&value).map_err(|error| {
            vec![issue(
                "error",
                "measurement_engineering_definition_serialization_failed",
                "$",
                error.to_string(),
                None::<String>,
            )]
        })?;
        let digest = Sha256::digest(canonical_json.as_bytes());
        Ok(CanonicalMeasurementEngineeringDefinition {
            aggregate_kind: self.aggregate_kind(),
            entity_id: self.entity_id().to_owned(),
            label: self.label(),
            summary_kind: self.summary_kind(),
            definition_schema_version: self.aggregate_kind().schema_version().to_owned(),
            canonical_json,
            definition_checksum: format!("sha256:{digest:x}"),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DefinitionReference {
    pub entity_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_id: Option<String>,
    #[serde(default = "default_true")]
    pub require_approved: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NumericRange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrequencyRange {
    pub minimum_hz: f64,
    pub maximum_hz: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalRepresentation {
    #[default]
    TimeDomainSamples,
    FrequencyDomainSpectrum,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleLimitHandling {
    #[default]
    Warn,
    Reject,
    MarkClipped,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampleInputLimits {
    pub minimum: f64,
    pub maximum: f64,
    #[serde(default)]
    pub handling: SampleLimitHandling,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExcitationRequirement {
    pub excitation_kind: ExcitationKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nominal_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default)]
    pub external_allowed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExcitationKind {
    None,
    External,
    Voltage,
    Current,
    Iepe,
    Bridge,
    Charge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorFamily {
    CurrentProbe,
    VoltageProbe,
    FieldProbe,
    ReceivingAntenna,
    TransmittingAntenna,
    Accelerometer,
    Microphone,
    Thermocouple,
    PressureSensor,
    Photodiode,
    StrainGauge,
    GenericTransducer,
    ManualTransducer,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SensorDefinition {
    pub definition_schema_version: String,
    pub sensor_definition_id: String,
    pub manufacturer: String,
    pub model_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    pub sensor_family: SensorFamily,
    pub physical_input_quantity: PhysicalQuantity,
    pub engineering_output_quantity: PhysicalQuantity,
    pub engineering_output_unit: String,
    pub electrical_output_quantity: PhysicalQuantity,
    pub electrical_output_unit: String,
    pub signal_domain: SignalDomain,
    #[serde(default)]
    pub technology_tags: Vec<TechnologyTag>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_excitation: Option<ExcitationRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_mode_requirement: Option<DaqInputMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nominal_range: Option<NumericRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safe_range: Option<NumericRange>,
    #[serde(default)]
    pub orientation_axes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settling_time_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_range: Option<FrequencyRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature_range: Option<NumericRange>,
    #[serde(default)]
    pub scaling_profile_refs: Vec<DefinitionReference>,
    #[serde(default)]
    pub correction_curve_refs: Vec<DefinitionReference>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScalingKind {
    Identity,
    Linear,
    TwoPoint,
    Polynomial,
    LookupTable,
    PiecewiseLinear,
    Expression,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScalingInterpolation {
    Linear,
    Nearest,
    StepPrevious,
    StepNext,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtrapolationPolicy {
    Forbidden,
    Clamp,
    Warn,
    Allow,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScalingPoint {
    pub input: f64,
    pub output: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScalingParameters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_point_1: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_point_1: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_point_2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_point_2: Option<f64>,
    #[serde(default)]
    pub coefficients: Vec<f64>,
    #[serde(default)]
    pub points: Vec<ScalingPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpolation: Option<ScalingInterpolation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extrapolation_policy: Option<ExtrapolationPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
}

impl Default for ScalingParameters {
    fn default() -> Self {
        Self {
            scale: None,
            offset: Some(0.0),
            input_point_1: None,
            output_point_1: None,
            input_point_2: None,
            output_point_2: None,
            coefficients: Vec::new(),
            points: Vec::new(),
            interpolation: None,
            extrapolation_policy: None,
            expression: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScalingProfileDefinition {
    pub definition_schema_version: String,
    pub scaling_profile_id: String,
    pub label: String,
    pub input_quantity: PhysicalQuantity,
    pub input_unit: String,
    pub output_quantity: PhysicalQuantity,
    pub output_unit: String,
    #[serde(default)]
    pub signal_representation: SignalRepresentation,
    pub scaling_kind: ScalingKind,
    #[serde(default)]
    pub parameters: ScalingParameters,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_limits: Option<SampleInputLimits>,
    #[serde(default)]
    pub validity_domain: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineeringCurveType {
    AntennaFactor,
    CableLoss,
    AmplifierGain,
    AttenuatorLoss,
    CurrentProbeTransfer,
    VoltageProbeTransfer,
    SensorFrequencyResponse,
    PhaseResponse,
    LinearityCorrection,
    Uncertainty,
    Vswr,
    SParameterMagnitude,
    SiteCharacterization,
    GenericCorrection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveAxis {
    Frequency,
    Amplitude,
    Temperature,
    Distance,
    Polarization,
    Orientation,
    Time,
    ChannelIndex,
}

impl CurveAxis {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Frequency => "frequency",
            Self::Amplitude => "amplitude",
            Self::Temperature => "temperature",
            Self::Distance => "distance",
            Self::Polarization => "polarization",
            Self::Orientation => "orientation",
            Self::Time => "time",
            Self::ChannelIndex => "channel_index",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveInterpolation {
    LinearXLinearY,
    LogXLinearY,
    LinearXLogY,
    Nearest,
    StepPrevious,
    StepNext,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveAxisDefinition {
    pub axis: CurveAxis,
    pub quantity: PhysicalQuantity,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveValueDefinition {
    pub value_id: String,
    pub quantity: PhysicalQuantity,
    pub unit: String,
    #[serde(default)]
    pub component: FrequencyResponseComponent,
    #[serde(default)]
    pub operation: CorrectionOperation,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrequencyResponseComponent {
    #[default]
    Amplitude,
    Phase,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionOperation {
    #[default]
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngineeringCurvePoint {
    pub axis_values: BTreeMap<String, f64>,
    pub values: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngineeringCurveDefinition {
    pub definition_schema_version: String,
    pub curve_id: String,
    pub curve_type: EngineeringCurveType,
    pub label: String,
    #[serde(default = "default_frequency_domain_spectrum")]
    pub signal_representation: SignalRepresentation,
    #[serde(default)]
    pub independent_axes: Vec<CurveAxisDefinition>,
    #[serde(default)]
    pub dependent_values: Vec<CurveValueDefinition>,
    #[serde(default)]
    pub units: BTreeMap<String, String>,
    #[serde(default)]
    pub points: Vec<EngineeringCurvePoint>,
    pub interpolation: CurveInterpolation,
    pub extrapolation_policy: ExtrapolationPolicy,
    #[serde(default)]
    pub validity_domain: BTreeMap<String, Value>,
    #[serde(default)]
    pub conditions: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_document_reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_checksum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaqChannelKind {
    AnalogInput,
    AnalogOutput,
    DigitalInput,
    DigitalOutput,
    DigitalBidirectional,
    CounterInput,
    FrequencyInput,
    TriggerInput,
    TriggerOutput,
    CanBusChannel,
    SoftwareChannel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaqInputMode {
    SingleEnded,
    Differential,
    PseudoDifferential,
    CurrentLoop,
    Charge,
    Iepe,
    BridgeQuarter,
    BridgeHalf,
    BridgeFull,
    Thermocouple,
    Rtd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouplingMode {
    Dc,
    Ac,
    Gnd,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SupportedRange {
    pub minimum: f64,
    pub maximum: f64,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DaqChannelProfileDefinition {
    pub definition_schema_version: String,
    pub daq_channel_profile_id: String,
    pub label: String,
    pub channel_kind: DaqChannelKind,
    pub signal_domain: SignalDomain,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_quantity: Option<PhysicalQuantity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_unit: Option<String>,
    #[serde(default)]
    pub supported_ranges: Vec<SupportedRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_bits: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_sampling_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_sampling_rate: Option<f64>,
    #[serde(default)]
    pub coupling_modes: Vec<CouplingMode>,
    #[serde(default)]
    pub input_modes: Vec<DaqInputMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anti_alias_filter: Option<String>,
    #[serde(default)]
    pub excitation_capabilities: Vec<ExcitationRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_completion: Option<String>,
    #[serde(default)]
    pub iepe_support: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isolation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synchronization: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggering: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AcquisitionChannelRecipeDefinition {
    pub definition_schema_version: String,
    pub recipe_id: String,
    pub label: String,
    pub output_channel_name: String,
    pub output_quantity: PhysicalQuantity,
    pub output_unit: String,
    pub daq_channel_profile_ref: DefinitionReference,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensor_definition_ref: Option<DefinitionReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scaling_profile_ref: Option<DefinitionReference>,
    #[serde(default)]
    pub correction_curve_refs: Vec<DefinitionReference>,
    pub sample_rate: f64,
    pub range: SupportedRange,
    pub coupling: CouplingMode,
    pub input_mode: DaqInputMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excitation: Option<ExcitationRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filtering: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggering: Option<String>,
    #[serde(default)]
    pub validation_rules: Vec<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

pub struct ResolvedAcquisitionRecipeContext<'a> {
    pub daq_channel_profile: Option<&'a DaqChannelProfileDefinition>,
    pub sensor_definition: Option<&'a SensorDefinition>,
    pub scaling_profile: Option<&'a ScalingProfileDefinition>,
    pub correction_curves: Vec<&'a EngineeringCurveDefinition>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngineeringCurveEvaluation {
    pub values: BTreeMap<String, f64>,
    pub axis_values: BTreeMap<String, f64>,
    pub interpolation: CurveInterpolation,
    pub extrapolated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    pub source_revision_id: String,
    pub source_checksum: String,
}

pub fn validate_sensor_definition(definition: &SensorDefinition) -> Vec<DefinitionValidationIssue> {
    let mut issues = Vec::new();
    require_schema(
        &mut issues,
        &definition.definition_schema_version,
        SENSOR_DEFINITION_SCHEMA_VERSION,
        "definition_schema_version",
    );
    require_id(
        &mut issues,
        &definition.sensor_definition_id,
        "sensor_definition_id",
    );
    require_text(&mut issues, &definition.manufacturer, "manufacturer");
    require_text(&mut issues, &definition.model_name, "model_name");
    validate_quantity_unit(
        &mut issues,
        definition.engineering_output_quantity,
        &definition.engineering_output_unit,
        "engineering_output_unit",
    );
    validate_quantity_unit(
        &mut issues,
        definition.electrical_output_quantity,
        &definition.electrical_output_unit,
        "electrical_output_unit",
    );
    validate_range(
        &mut issues,
        definition.nominal_range.as_ref(),
        definition.physical_input_quantity,
        "nominal_range",
    );
    validate_range(
        &mut issues,
        definition.safe_range.as_ref(),
        definition.physical_input_quantity,
        "safe_range",
    );
    validate_range(
        &mut issues,
        definition.temperature_range.as_ref(),
        PhysicalQuantity::Temperature,
        "temperature_range",
    );
    if let Some(range) = &definition.frequency_range {
        if !range.minimum_hz.is_finite()
            || !range.maximum_hz.is_finite()
            || range.minimum_hz < 0.0
            || range.maximum_hz <= range.minimum_hz
        {
            issues.push(issue(
                "error",
                "invalid_frequency_range",
                "frequency_range",
                "frequency_range must use finite positive values with maximum above minimum",
                Some("Use values in Hz, for example 10.0 to 100000000.0."),
            ));
        }
    }
    if let Some(settling) = definition.settling_time_ms {
        if !settling.is_finite() || settling < 0.0 {
            issues.push(issue(
                "error",
                "invalid_settling_time",
                "settling_time_ms",
                "settling_time_ms must be a finite non-negative number",
                None::<String>,
            ));
        }
    }
    validate_refs(
        &mut issues,
        &definition.scaling_profile_refs,
        "scaling_profile_refs",
    );
    validate_refs(
        &mut issues,
        &definition.correction_curve_refs,
        "correction_curve_refs",
    );
    match definition.sensor_family {
        SensorFamily::CurrentProbe
            if definition.physical_input_quantity != PhysicalQuantity::Current =>
        {
            issues.push(issue(
                "error",
                "current_probe_quantity_mismatch",
                "physical_input_quantity",
                "current_probe sensors must measure current",
                Some("Set physical_input_quantity to current."),
            ));
        }
        SensorFamily::Accelerometer
            if definition.physical_input_quantity != PhysicalQuantity::Acceleration =>
        {
            issues.push(issue(
                "error",
                "accelerometer_quantity_mismatch",
                "physical_input_quantity",
                "accelerometers must measure acceleration",
                Some("Set physical_input_quantity to acceleration."),
            ));
        }
        SensorFamily::Microphone
            if definition.physical_input_quantity != PhysicalQuantity::SoundPressure =>
        {
            issues.push(issue(
                "error",
                "microphone_quantity_mismatch",
                "physical_input_quantity",
                "microphones must measure sound_pressure",
                Some("Set physical_input_quantity to sound_pressure."),
            ));
        }
        SensorFamily::ReceivingAntenna
            if definition.physical_input_quantity != PhysicalQuantity::ElectricField =>
        {
            issues.push(issue(
                "error",
                "receiving_antenna_quantity_mismatch",
                "physical_input_quantity",
                "receiving antennas must measure electric_field",
                Some("Set physical_input_quantity to electric_field."),
            ));
        }
        _ => {}
    }
    if matches!(definition.sensor_family, SensorFamily::Accelerometer)
        && matches!(
            definition
                .required_excitation
                .as_ref()
                .map(|item| item.excitation_kind),
            Some(ExcitationKind::Iepe)
        )
        && !definition.technology_tags.contains(&TechnologyTag::Iepe)
    {
        issues.push(issue(
            "warning",
            "iepe_tag_missing",
            "technology_tags",
            "IEPE accelerometers should declare the iepe technology tag",
            Some("Add technology_tags: [\"iepe\"] when the sensor requires IEPE excitation."),
        ));
    }
    validate_excitation(
        &mut issues,
        definition.required_excitation.as_ref(),
        "required_excitation",
    );
    issues
}

pub fn validate_scaling_profile_definition(
    definition: &ScalingProfileDefinition,
) -> Vec<DefinitionValidationIssue> {
    let mut issues = Vec::new();
    require_schema(
        &mut issues,
        &definition.definition_schema_version,
        SCALING_PROFILE_DEFINITION_SCHEMA_VERSION,
        "definition_schema_version",
    );
    require_id(
        &mut issues,
        &definition.scaling_profile_id,
        "scaling_profile_id",
    );
    require_text(&mut issues, &definition.label, "label");
    validate_quantity_unit(
        &mut issues,
        definition.input_quantity,
        &definition.input_unit,
        "input_unit",
    );
    validate_quantity_unit(
        &mut issues,
        definition.output_quantity,
        &definition.output_unit,
        "output_unit",
    );
    if definition.signal_representation != SignalRepresentation::TimeDomainSamples {
        issues.push(issue(
            "error",
            "scaling_requires_time_domain_samples",
            "signal_representation",
            "sample conversion profiles operate on time-domain samples",
            Some("Use frequency-response correction for spectrum amplitude or phase compensation."),
        ));
    }
    if let Some(limits) = &definition.input_limits {
        if !limits.minimum.is_finite()
            || !limits.maximum.is_finite()
            || limits.maximum <= limits.minimum
        {
            issues.push(issue(
                "error",
                "invalid_sample_input_limits",
                "input_limits",
                "sample input limits require finite values with maximum above minimum",
                Some("Declare the usable input range before overload or clipping."),
            ));
        }
    }
    match definition.scaling_kind {
        ScalingKind::Identity => {
            if definition.input_quantity != definition.output_quantity
                || definition.input_unit != definition.output_unit
            {
                issues.push(issue(
                    "error",
                    "identity_scaling_unit_mismatch",
                    "scaling_kind",
                    "identity scaling requires identical input and output quantity/unit",
                    Some("Use linear scaling when a unit conversion or sensitivity is required."),
                ));
            }
        }
        ScalingKind::Linear => {
            require_finite(
                &mut issues,
                definition.parameters.scale,
                "parameters.scale",
                "missing_linear_scale",
            );
            if let Some(offset) = definition.parameters.offset {
                if !offset.is_finite() {
                    issues.push(issue(
                        "error",
                        "invalid_linear_offset",
                        "parameters.offset",
                        "linear offset must be finite",
                        None::<String>,
                    ));
                }
            }
        }
        ScalingKind::TwoPoint => {
            let x1 = require_finite(
                &mut issues,
                definition.parameters.input_point_1,
                "parameters.input_point_1",
                "missing_two_point_input_1",
            );
            require_finite(
                &mut issues,
                definition.parameters.output_point_1,
                "parameters.output_point_1",
                "missing_two_point_output_1",
            );
            let x2 = require_finite(
                &mut issues,
                definition.parameters.input_point_2,
                "parameters.input_point_2",
                "missing_two_point_input_2",
            );
            require_finite(
                &mut issues,
                definition.parameters.output_point_2,
                "parameters.output_point_2",
                "missing_two_point_output_2",
            );
            if let (Some(x1), Some(x2)) = (x1, x2) {
                if (x1 - x2).abs() < f64::EPSILON {
                    issues.push(issue(
                        "error",
                        "two_point_inputs_identical",
                        "parameters.input_point_2",
                        "two-point scaling requires distinct input points",
                        Some("Change one input point so the slope is defined."),
                    ));
                }
            }
        }
        ScalingKind::Polynomial => {
            if definition.parameters.coefficients.is_empty() {
                issues.push(issue(
                    "error",
                    "polynomial_coefficients_empty",
                    "parameters.coefficients",
                    "polynomial scaling requires at least one coefficient",
                    None::<String>,
                ));
            }
            for (index, coefficient) in definition.parameters.coefficients.iter().enumerate() {
                if !coefficient.is_finite() {
                    issues.push(issue(
                        "error",
                        "invalid_polynomial_coefficient",
                        format!("parameters.coefficients[{index}]"),
                        "polynomial coefficients must be finite",
                        None::<String>,
                    ));
                }
            }
        }
        ScalingKind::LookupTable | ScalingKind::PiecewiseLinear => {
            validate_scaling_points(&mut issues, &definition.parameters);
        }
        ScalingKind::Expression => match definition.parameters.expression.as_deref() {
            Some(expression) if !expression.trim().is_empty() => {
                validate_expression(&mut issues, expression, "parameters.expression")
            }
            _ => issues.push(issue(
                "error",
                "missing_expression",
                "parameters.expression",
                "expression scaling requires an expression",
                Some(
                    "Use variables x, input, temperature or frequency with the allowed functions.",
                ),
            )),
        },
    }
    issues
}

pub fn evaluate_scaling_profile(
    definition: &ScalingProfileDefinition,
    input: f64,
) -> Result<f64, Vec<DefinitionValidationIssue>> {
    let issues = validate_scaling_profile_definition(definition);
    if issues.iter().any(|item| item.severity == "error") {
        return Err(issues);
    }
    if !input.is_finite() {
        return Err(vec![issue(
            "error",
            "invalid_scaling_input",
            "input",
            "scaling input must be finite",
            None::<String>,
        )]);
    }
    let value = match definition.scaling_kind {
        ScalingKind::Identity => input,
        ScalingKind::Linear => {
            definition.parameters.scale.unwrap_or(1.0) * input
                + definition.parameters.offset.unwrap_or(0.0)
        }
        ScalingKind::TwoPoint => {
            let x1 = definition.parameters.input_point_1.unwrap();
            let y1 = definition.parameters.output_point_1.unwrap();
            let x2 = definition.parameters.input_point_2.unwrap();
            let y2 = definition.parameters.output_point_2.unwrap();
            y1 + (input - x1) * (y2 - y1) / (x2 - x1)
        }
        ScalingKind::Polynomial => definition
            .parameters
            .coefficients
            .iter()
            .enumerate()
            .map(|(power, coefficient)| coefficient * input.powi(power as i32))
            .sum(),
        ScalingKind::LookupTable | ScalingKind::PiecewiseLinear => {
            interpolate_scaling_points(definition, input)?
        }
        ScalingKind::Expression => {
            return Err(vec![issue(
            "error",
            "expression_evaluation_not_implemented",
            "parameters.expression",
            "0.13.0 validates the expression DSL but does not execute expression scaling",
            Some(
                "Use linear, two_point, polynomial or lookup_table for executable 0.13.0 scaling.",
            ),
        )])
        }
    };
    if !value.is_finite() {
        return Err(vec![issue(
            "error",
            "invalid_scaling_result",
            "output",
            "scaling results must be finite",
            Some("Use a bounded input range or scaling parameters that cannot overflow."),
        )]);
    }
    Ok(value)
}

pub fn validate_engineering_curve_definition(
    definition: &EngineeringCurveDefinition,
) -> Vec<DefinitionValidationIssue> {
    let mut issues = Vec::new();
    require_schema(
        &mut issues,
        &definition.definition_schema_version,
        ENGINEERING_CURVE_DEFINITION_SCHEMA_VERSION,
        "definition_schema_version",
    );
    require_id(&mut issues, &definition.curve_id, "curve_id");
    require_text(&mut issues, &definition.label, "label");
    if definition.signal_representation != SignalRepresentation::FrequencyDomainSpectrum {
        issues.push(issue(
            "error",
            "engineering_curve_requires_frequency_spectrum",
            "signal_representation",
            "frequency-response corrections operate on frequency-domain spectra",
            Some("Use a sample conversion profile for gain, offset, overload, or clipping in the time domain."),
        ));
    }
    if definition.independent_axes.is_empty() {
        issues.push(issue(
            "error",
            "missing_curve_axis",
            "independent_axes",
            "engineering curves require at least one independent axis",
            None::<String>,
        ));
    }
    if definition.dependent_values.is_empty() {
        issues.push(issue(
            "error",
            "missing_curve_value",
            "dependent_values",
            "engineering curves require at least one dependent value",
            None::<String>,
        ));
    }
    let mut axis_ids = BTreeSet::new();
    for (index, axis) in definition.independent_axes.iter().enumerate() {
        let axis_id = axis.axis.as_str();
        if !axis_ids.insert(axis_id) {
            issues.push(issue(
                "error",
                "duplicate_curve_axis",
                format!("independent_axes[{index}].axis"),
                "curve axes must be unique",
                None::<String>,
            ));
        }
        validate_quantity_unit(
            &mut issues,
            axis.quantity,
            &axis.unit,
            format!("independent_axes[{index}].unit"),
        );
    }
    let mut value_ids = BTreeSet::new();
    let mut components = BTreeSet::new();
    for (index, dependent) in definition.dependent_values.iter().enumerate() {
        require_id(
            &mut issues,
            &dependent.value_id,
            format!("dependent_values[{index}].value_id"),
        );
        if !value_ids.insert(dependent.value_id.as_str()) {
            issues.push(issue(
                "error",
                "duplicate_curve_value",
                format!("dependent_values[{index}].value_id"),
                "curve dependent value ids must be unique",
                None::<String>,
            ));
        }
        validate_quantity_unit(
            &mut issues,
            dependent.quantity,
            &dependent.unit,
            format!("dependent_values[{index}].unit"),
        );
        if !components.insert(dependent.component) {
            issues.push(issue(
                "error",
                "duplicate_frequency_response_component",
                format!("dependent_values[{index}].component"),
                "a frequency response can define at most one amplitude and one phase component",
                None::<String>,
            ));
        }
        match dependent.component {
            FrequencyResponseComponent::Amplitude => {
                if dependent.quantity != PhysicalQuantity::Dimensionless {
                    issues.push(issue(
                        "error",
                        "amplitude_correction_quantity_mismatch",
                        format!("dependent_values[{index}].quantity"),
                        "amplitude response corrections must use the dimensionless quantity with dB or a linear ratio unit",
                        Some("Use quantity=dimensionless and unit=dB for RF loss or gain."),
                    ));
                }
            }
            FrequencyResponseComponent::Phase => {
                if dependent.quantity != PhysicalQuantity::Angle
                    || !matches!(dependent.unit.as_str(), "deg" | "rad")
                {
                    issues.push(issue(
                        "error",
                        "phase_correction_unit_mismatch",
                        format!("dependent_values[{index}].unit"),
                        "phase response corrections must use an angle in deg or rad",
                        Some("Use quantity=angle with unit=deg or rad."),
                    ));
                }
            }
        }
    }
    if !components.contains(&FrequencyResponseComponent::Amplitude) {
        issues.push(issue(
            "error",
            "missing_amplitude_frequency_response",
            "dependent_values",
            "frequency-response corrections require an amplitude component",
            Some("Add an amplitude correction in dB or as a linear ratio."),
        ));
    }
    if !definition
        .independent_axes
        .iter()
        .any(|axis| axis.axis == CurveAxis::Frequency)
    {
        issues.push(issue(
            "error",
            "frequency_response_axis_missing",
            "independent_axes",
            "frequency-response corrections require a frequency axis",
            Some("Use axis=frequency, quantity=frequency, unit=Hz."),
        ));
    }
    if definition.points.is_empty() {
        issues.push(issue(
            "error",
            "missing_curve_points",
            "points",
            "engineering curves require at least one point",
            Some("Provide table rows with axis_values and values."),
        ));
    }
    if definition.independent_axes.len() == 1 {
        validate_curve_1d_points(&mut issues, definition);
    }
    if matches!(definition.interpolation, CurveInterpolation::LogXLinearY) {
        for (index, point) in definition.points.iter().enumerate() {
            if let Some(x) = first_axis_value(definition, point) {
                if x <= 0.0 {
                    issues.push(issue(
                        "error",
                        "log_interpolation_non_positive_x",
                        format!("points[{index}].axis_values"),
                        "log-x interpolation requires strictly positive x values",
                        None::<String>,
                    ));
                }
            }
        }
    }
    validate_curve_type_units(&mut issues, definition);
    if let Some(checksum) = &definition.source_checksum {
        if !is_sha256_checksum(checksum) {
            issues.push(issue(
                "error",
                "invalid_source_checksum",
                "source_checksum",
                "source checksum must use sha256:<64 lowercase hex characters>",
                None::<String>,
            ));
        }
    }
    issues
}

pub fn evaluate_engineering_curve(
    definition: &EngineeringCurveDefinition,
    axis_values: BTreeMap<String, f64>,
    source_revision_id: &str,
    source_checksum: &str,
) -> Result<EngineeringCurveEvaluation, Vec<DefinitionValidationIssue>> {
    let issues = validate_engineering_curve_definition(definition);
    if issues.iter().any(|item| item.severity == "error") {
        return Err(issues);
    }
    if definition.independent_axes.len() != 1 {
        return Err(vec![issue(
            "error",
            "curve_evaluation_requires_1d",
            "independent_axes",
            "0.13.0 curve evaluation supports one independent axis",
            None::<String>,
        )]);
    }
    let axis_id = definition.independent_axes[0].axis.as_str();
    let requested_x = *axis_values.get(axis_id).ok_or_else(|| {
        vec![issue(
            "error",
            "missing_curve_evaluation_axis",
            "axis_values",
            format!("missing axis value for {axis_id}"),
            None::<String>,
        )]
    })?;
    if !requested_x.is_finite() {
        return Err(vec![issue(
            "error",
            "invalid_curve_evaluation_axis",
            "axis_values",
            "axis values must be finite",
            None::<String>,
        )]);
    }
    if matches!(definition.interpolation, CurveInterpolation::LogXLinearY) && requested_x <= 0.0 {
        return Err(vec![issue(
            "error",
            "log_interpolation_non_positive_x",
            "axis_values",
            "log-x interpolation requires strictly positive x values",
            None::<String>,
        )]);
    }
    let mut points = definition.points.clone();
    points.sort_by(|left, right| {
        first_axis_value(definition, left)
            .unwrap_or(0.0)
            .total_cmp(&first_axis_value(definition, right).unwrap_or(0.0))
    });
    let first_x = first_axis_value(definition, &points[0]).unwrap();
    let last_x = first_axis_value(definition, points.last().unwrap()).unwrap();
    let mut x = requested_x;
    let mut extrapolated = requested_x < first_x || requested_x > last_x;
    let mut warning = None;
    if extrapolated {
        match definition.extrapolation_policy {
            ExtrapolationPolicy::Forbidden => {
                return Err(vec![issue(
                    "error",
                    "curve_extrapolation_forbidden",
                    "axis_values",
                    format!(
                        "axis value {requested_x} is outside the curve domain [{first_x}, {last_x}]"
                    ),
                    Some("Use a value inside the curve domain or change the extrapolation policy."),
                )]);
            }
            ExtrapolationPolicy::Clamp => {
                x = x.clamp(first_x, last_x);
                warning = Some("axis value was clamped to the curve domain".to_owned());
            }
            ExtrapolationPolicy::Warn => {
                warning = Some("axis value was extrapolated outside the curve domain".to_owned());
            }
            ExtrapolationPolicy::Allow => {}
        }
    }
    if (x - requested_x).abs() > f64::EPSILON {
        extrapolated = true;
    }
    let mut result_values = BTreeMap::new();
    for dependent in &definition.dependent_values {
        let value = interpolate_curve_value(definition, &points, x, &dependent.value_id)?;
        if !value.is_finite() {
            return Err(vec![issue(
                "error",
                "invalid_curve_evaluation_result",
                format!("values.{}", dependent.value_id),
                "curve evaluation results must be finite",
                Some("Use a value inside the curve domain or a stricter extrapolation policy."),
            )]);
        }
        result_values.insert(dependent.value_id.clone(), value);
    }
    Ok(EngineeringCurveEvaluation {
        values: result_values,
        axis_values,
        interpolation: definition.interpolation,
        extrapolated,
        warning,
        source_revision_id: source_revision_id.to_owned(),
        source_checksum: source_checksum.to_owned(),
    })
}

pub fn validate_daq_channel_profile_definition(
    definition: &DaqChannelProfileDefinition,
) -> Vec<DefinitionValidationIssue> {
    let mut issues = Vec::new();
    require_schema(
        &mut issues,
        &definition.definition_schema_version,
        DAQ_CHANNEL_PROFILE_DEFINITION_SCHEMA_VERSION,
        "definition_schema_version",
    );
    require_id(
        &mut issues,
        &definition.daq_channel_profile_id,
        "daq_channel_profile_id",
    );
    require_text(&mut issues, &definition.label, "label");
    if matches!(
        definition.channel_kind,
        DaqChannelKind::AnalogInput | DaqChannelKind::AnalogOutput
    ) {
        match (definition.input_quantity, definition.input_unit.as_deref()) {
            (Some(quantity), Some(unit)) => {
                validate_quantity_unit(&mut issues, quantity, unit, "input_unit")
            }
            _ => issues.push(issue(
                "error",
                "analog_channel_missing_quantity_unit",
                "input_quantity",
                "analog channels require input_quantity and input_unit",
                None::<String>,
            )),
        }
    }
    for (index, range) in definition.supported_ranges.iter().enumerate() {
        validate_supported_range(&mut issues, range, format!("supported_ranges[{index}]"));
        if let Some(quantity) = definition.input_quantity {
            validate_quantity_unit(
                &mut issues,
                quantity,
                &range.unit,
                format!("supported_ranges[{index}].unit"),
            );
        }
    }
    if let Some(bits) = definition.resolution_bits {
        if bits == 0 {
            issues.push(issue(
                "error",
                "invalid_resolution_bits",
                "resolution_bits",
                "resolution_bits must be positive",
                None::<String>,
            ));
        }
    }
    if let Some(max_rate) = definition.max_sampling_rate {
        if !max_rate.is_finite() || max_rate <= 0.0 {
            issues.push(issue(
                "error",
                "invalid_max_sampling_rate",
                "max_sampling_rate",
                "max_sampling_rate must be positive",
                None::<String>,
            ));
        }
    }
    if let Some(min_rate) = definition.min_sampling_rate {
        if !min_rate.is_finite() || min_rate < 0.0 {
            issues.push(issue(
                "error",
                "invalid_min_sampling_rate",
                "min_sampling_rate",
                "min_sampling_rate must be non-negative",
                None::<String>,
            ));
        }
    }
    if let (Some(min_rate), Some(max_rate)) =
        (definition.min_sampling_rate, definition.max_sampling_rate)
    {
        if min_rate > max_rate {
            issues.push(issue(
                "error",
                "sampling_rate_order_invalid",
                "min_sampling_rate",
                "min_sampling_rate must not exceed max_sampling_rate",
                None::<String>,
            ));
        }
    }
    if definition.iepe_support && !definition.input_modes.contains(&DaqInputMode::Iepe) {
        issues.push(issue(
            "error",
            "iepe_support_without_input_mode",
            "iepe_support",
            "IEPE support requires input_modes to include iepe",
            None::<String>,
        ));
    }
    if matches!(definition.channel_kind, DaqChannelKind::CanBusChannel)
        && definition.signal_domain != SignalDomain::CanBus
    {
        issues.push(issue(
            "error",
            "can_channel_domain_mismatch",
            "signal_domain",
            "CAN bus channels must use signal_domain=can_bus",
            Some("Use a DAQ analog input profile for ADC voltage/current channels."),
        ));
    }
    if matches!(
        definition.channel_kind,
        DaqChannelKind::TriggerInput | DaqChannelKind::TriggerOutput
    ) && definition.signal_domain != SignalDomain::Trigger
    {
        issues.push(issue(
            "error",
            "trigger_channel_domain_mismatch",
            "signal_domain",
            "trigger channels must use signal_domain=trigger",
            None::<String>,
        ));
    }
    if matches!(
        definition.channel_kind,
        DaqChannelKind::DigitalInput
            | DaqChannelKind::DigitalOutput
            | DaqChannelKind::DigitalBidirectional
    ) && matches!(
        definition.signal_domain,
        SignalDomain::AnalogVoltage | SignalDomain::AnalogCurrent
    ) {
        issues.push(issue(
            "error",
            "digital_channel_analog_domain",
            "signal_domain",
            "digital channels must not be modelled as analog ADC/DAC channels",
            None::<String>,
        ));
    }
    for (index, excitation) in definition.excitation_capabilities.iter().enumerate() {
        validate_excitation(
            &mut issues,
            Some(excitation),
            format!("excitation_capabilities[{index}]"),
        );
    }
    issues
}

pub fn validate_acquisition_channel_recipe_definition(
    definition: &AcquisitionChannelRecipeDefinition,
) -> Vec<DefinitionValidationIssue> {
    let mut issues = Vec::new();
    require_schema(
        &mut issues,
        &definition.definition_schema_version,
        ACQUISITION_CHANNEL_RECIPE_DEFINITION_SCHEMA_VERSION,
        "definition_schema_version",
    );
    require_id(&mut issues, &definition.recipe_id, "recipe_id");
    require_text(&mut issues, &definition.label, "label");
    require_id(
        &mut issues,
        &definition.output_channel_name,
        "output_channel_name",
    );
    validate_quantity_unit(
        &mut issues,
        definition.output_quantity,
        &definition.output_unit,
        "output_unit",
    );
    validate_ref(
        &mut issues,
        &definition.daq_channel_profile_ref,
        "daq_channel_profile_ref",
    );
    if let Some(reference) = &definition.sensor_definition_ref {
        validate_ref(&mut issues, reference, "sensor_definition_ref");
    }
    if let Some(reference) = &definition.scaling_profile_ref {
        validate_ref(&mut issues, reference, "scaling_profile_ref");
    }
    validate_refs(
        &mut issues,
        &definition.correction_curve_refs,
        "correction_curve_refs",
    );
    if !definition.sample_rate.is_finite() || definition.sample_rate <= 0.0 {
        issues.push(issue(
            "error",
            "invalid_recipe_sample_rate",
            "sample_rate",
            "sample_rate must be positive",
            None::<String>,
        ));
    }
    validate_supported_range(&mut issues, &definition.range, "range");
    validate_excitation(&mut issues, definition.excitation.as_ref(), "excitation");
    issues
}

pub fn validate_acquisition_channel_recipe_with_context(
    definition: &AcquisitionChannelRecipeDefinition,
    context: ResolvedAcquisitionRecipeContext<'_>,
) -> Vec<DefinitionValidationIssue> {
    let mut issues = validate_acquisition_channel_recipe_definition(definition);
    let Some(daq) = context.daq_channel_profile else {
        issues.push(issue(
            "error",
            "missing_resolved_daq_channel_profile",
            "daq_channel_profile_ref",
            "DAQ channel profile reference must resolve to an approved definition",
            None::<String>,
        ));
        return issues;
    };
    let daq_issues = validate_daq_channel_profile_definition(daq);
    if daq_issues.iter().any(|item| item.severity == "error") {
        issues.push(issue(
            "error",
            "resolved_daq_channel_profile_invalid",
            "daq_channel_profile_ref",
            "resolved DAQ channel profile is not valid",
            None::<String>,
        ));
    }
    if !daq.input_modes.is_empty() && !daq.input_modes.contains(&definition.input_mode) {
        issues.push(issue(
            "error",
            "recipe_input_mode_not_supported",
            "input_mode",
            "recipe input_mode is not supported by the DAQ channel profile",
            None::<String>,
        ));
    }
    if !daq.coupling_modes.is_empty() && !daq.coupling_modes.contains(&definition.coupling) {
        issues.push(issue(
            "error",
            "recipe_coupling_not_supported",
            "coupling",
            "recipe coupling is not supported by the DAQ channel profile",
            None::<String>,
        ));
    }
    if let Some(max_rate) = daq.max_sampling_rate {
        if definition.sample_rate > max_rate {
            issues.push(issue(
                "error",
                "recipe_sample_rate_exceeds_daq",
                "sample_rate",
                format!(
                    "recipe sample_rate {} exceeds DAQ max_sampling_rate {}",
                    definition.sample_rate, max_rate
                ),
                None::<String>,
            ));
        }
    }
    if let Some(quantity) = daq.input_quantity {
        validate_quantity_unit(&mut issues, quantity, &definition.range.unit, "range.unit");
    }
    if !daq.supported_ranges.iter().any(|range| {
        range.unit == definition.range.unit
            && range.minimum <= definition.range.minimum
            && range.maximum >= definition.range.maximum
    }) {
        issues.push(issue(
            "error",
            "recipe_range_not_supported",
            "range",
            "recipe range is not covered by the DAQ supported ranges",
            Some("Use a range inside one declared supported_ranges entry."),
        ));
    }
    if let Some(sensor) = context.sensor_definition {
        if let Some(required) = &sensor.required_excitation {
            let satisfied = matches!(required.excitation_kind, ExcitationKind::None)
                || required.external_allowed
                || daq
                    .excitation_capabilities
                    .iter()
                    .any(|capability| capability.excitation_kind == required.excitation_kind);
            if !satisfied {
                issues.push(issue(
                    "error",
                    "recipe_sensor_excitation_not_available",
                    "excitation",
                    "sensor excitation cannot be supplied by the DAQ profile and is not marked external",
                    None::<String>,
                ));
            }
        }
        if sensor.electrical_output_quantity
            != daq
                .input_quantity
                .unwrap_or(sensor.electrical_output_quantity)
        {
            issues.push(issue(
                "error",
                "recipe_sensor_daq_quantity_mismatch",
                "sensor_definition_ref",
                "sensor electrical output quantity does not match the DAQ input quantity",
                None::<String>,
            ));
        }
    }
    if let Some(scaling) = context.scaling_profile {
        if scaling.output_quantity != definition.output_quantity
            || scaling.output_unit != definition.output_unit
        {
            issues.push(issue(
                "error",
                "recipe_scaling_output_mismatch",
                "scaling_profile_ref",
                "scaling profile output quantity/unit must match the recipe output",
                None::<String>,
            ));
        }
        if let Some(sensor) = context.sensor_definition {
            if scaling.input_quantity != sensor.electrical_output_quantity
                || scaling.input_unit != sensor.electrical_output_unit
            {
                issues.push(issue(
                    "error",
                    "recipe_scaling_sensor_input_mismatch",
                    "scaling_profile_ref",
                    "scaling input must match the sensor electrical output",
                    None::<String>,
                ));
            }
        }
    }
    for curve in context.correction_curves {
        if curve.dependent_values.is_empty() {
            issues.push(issue(
                "error",
                "recipe_correction_curve_invalid",
                "correction_curve_refs",
                "correction curve must expose at least one dependent value",
                None::<String>,
            ));
        }
    }
    issues
}

fn validate_range(
    issues: &mut Vec<DefinitionValidationIssue>,
    range: Option<&NumericRange>,
    quantity: PhysicalQuantity,
    path: impl AsRef<str>,
) {
    let Some(range) = range else { return };
    let path = path.as_ref();
    validate_quantity_unit(issues, quantity, &range.unit, format!("{path}.unit"));
    if let (Some(minimum), Some(maximum)) = (range.minimum, range.maximum) {
        if !minimum.is_finite() || !maximum.is_finite() || maximum <= minimum {
            issues.push(issue(
                "error",
                "invalid_numeric_range",
                path,
                "range values must be finite with maximum above minimum",
                None::<String>,
            ));
        }
    }
}

fn validate_supported_range(
    issues: &mut Vec<DefinitionValidationIssue>,
    range: &SupportedRange,
    path: impl AsRef<str>,
) {
    let path = path.as_ref();
    if !range.minimum.is_finite() || !range.maximum.is_finite() || range.maximum <= range.minimum {
        issues.push(issue(
            "error",
            "invalid_supported_range",
            path,
            "range minimum/maximum must be finite with maximum above minimum",
            None::<String>,
        ));
    }
    if unit_quantity(&range.unit).is_none() {
        issues.push(issue(
            "error",
            "unknown_unit",
            format!("{path}.unit"),
            format!("unit is not in the registry: {}", range.unit),
            None::<String>,
        ));
    }
}

fn validate_scaling_points(
    issues: &mut Vec<DefinitionValidationIssue>,
    parameters: &ScalingParameters,
) {
    if parameters.points.len() < 2 {
        issues.push(issue(
            "error",
            "lookup_points_too_short",
            "parameters.points",
            "lookup and piecewise scaling require at least two points",
            None::<String>,
        ));
    }
    validate_scaling_interpolation_policy(issues, parameters);
    let mut previous = None;
    let mut seen = BTreeSet::new();
    for (index, point) in parameters.points.iter().enumerate() {
        if !point.input.is_finite() || !point.output.is_finite() {
            issues.push(issue(
                "error",
                "invalid_lookup_point",
                format!("parameters.points[{index}]"),
                "lookup point input and output must be finite",
                None::<String>,
            ));
        }
        let normalized = point.input.to_bits();
        if !seen.insert(normalized) {
            issues.push(issue(
                "error",
                "duplicate_lookup_input",
                format!("parameters.points[{index}].input"),
                "lookup table inputs must be unique",
                None::<String>,
            ));
        }
        if let Some(previous) = previous {
            if point.input <= previous {
                issues.push(issue(
                    "error",
                    "lookup_inputs_not_monotonic",
                    format!("parameters.points[{index}].input"),
                    "lookup inputs must be strictly increasing",
                    None::<String>,
                ));
            }
        }
        previous = Some(point.input);
    }
}

fn validate_scaling_interpolation_policy(
    issues: &mut Vec<DefinitionValidationIssue>,
    parameters: &ScalingParameters,
) {
    if parameters.interpolation.is_none() {
        issues.push(issue(
            "error",
            "missing_interpolation",
            "parameters.interpolation",
            "lookup and piecewise scaling require an explicit interpolation mode",
            None::<String>,
        ));
    }
    if parameters.extrapolation_policy.is_none() {
        issues.push(issue(
            "error",
            "missing_extrapolation_policy",
            "parameters.extrapolation_policy",
            "lookup and piecewise scaling require an explicit extrapolation policy",
            None::<String>,
        ));
    }
}

fn interpolate_scaling_points(
    definition: &ScalingProfileDefinition,
    input: f64,
) -> Result<f64, Vec<DefinitionValidationIssue>> {
    let mut points = definition.parameters.points.clone();
    points.sort_by(|left, right| left.input.total_cmp(&right.input));
    let first = points.first().unwrap();
    let last = points.last().unwrap();
    let mut x = input;
    if x < first.input || x > last.input {
        match definition
            .parameters
            .extrapolation_policy
            .unwrap_or(ExtrapolationPolicy::Forbidden)
        {
            ExtrapolationPolicy::Forbidden => {
                return Err(vec![issue(
                    "error",
                    "scaling_extrapolation_forbidden",
                    "input",
                    "input is outside the scaling lookup domain",
                    None::<String>,
                )])
            }
            ExtrapolationPolicy::Clamp => x = x.clamp(first.input, last.input),
            ExtrapolationPolicy::Warn | ExtrapolationPolicy::Allow => {}
        }
    }
    let pair = points
        .windows(2)
        .find(|window| x >= window[0].input && x <= window[1].input)
        .unwrap_or_else(|| {
            if x < first.input {
                &points[0..2]
            } else {
                &points[points.len() - 2..]
            }
        });
    let left = &pair[0];
    let right = &pair[1];
    let value = match definition
        .parameters
        .interpolation
        .unwrap_or(ScalingInterpolation::Linear)
    {
        ScalingInterpolation::Linear => {
            left.output
                + (x - left.input) * (right.output - left.output) / (right.input - left.input)
        }
        ScalingInterpolation::Nearest => {
            if (x - left.input).abs() <= (right.input - x).abs() {
                left.output
            } else {
                right.output
            }
        }
        ScalingInterpolation::StepPrevious => left.output,
        ScalingInterpolation::StepNext => right.output,
    };
    Ok(value)
}

fn validate_expression(issues: &mut Vec<DefinitionValidationIssue>, expression: &str, path: &str) {
    let allowed_variables = ["x", "input", "temperature", "frequency"];
    let allowed_functions = ["pow", "sqrt", "log10", "ln", "abs", "min", "max"];
    let mut token = String::new();
    for character in expression.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            token.push(character);
            continue;
        }
        if !token.is_empty() {
            validate_expression_identifier(
                issues,
                &token,
                &allowed_variables,
                &allowed_functions,
                path,
            );
            token.clear();
        }
        if !(character.is_ascii_whitespace()
            || matches!(
                character,
                '+' | '-' | '*' | '/' | '^' | '(' | ')' | ',' | '.'
            ))
        {
            issues.push(issue(
                "error",
                "unsafe_expression_character",
                path,
                format!("unsupported character in expression: {character}"),
                Some("Use only numeric operators and the allowed function names."),
            ));
        }
    }
    if !token.is_empty() {
        validate_expression_identifier(
            issues,
            &token,
            &allowed_variables,
            &allowed_functions,
            path,
        );
    }
}

fn validate_expression_identifier(
    issues: &mut Vec<DefinitionValidationIssue>,
    token: &str,
    allowed_variables: &[&str],
    allowed_functions: &[&str],
    path: &str,
) {
    if token.chars().all(|character| character.is_ascii_digit()) {
        return;
    }
    if allowed_variables.contains(&token) || allowed_functions.contains(&token) {
        return;
    }
    issues.push(issue(
        "error",
        "unknown_expression_identifier",
        path,
        format!("unknown expression identifier: {token}"),
        Some("Allowed variables are x, input, temperature, frequency; allowed functions are pow, sqrt, log10, ln, abs, min and max."),
    ));
}

fn validate_curve_1d_points(
    issues: &mut Vec<DefinitionValidationIssue>,
    definition: &EngineeringCurveDefinition,
) {
    let axis_id = definition.independent_axes[0].axis.as_str();
    let mut seen = BTreeSet::new();
    for (index, point) in definition.points.iter().enumerate() {
        let Some(x) = point.axis_values.get(axis_id).copied() else {
            issues.push(issue(
                "error",
                "missing_curve_point_axis",
                format!("points[{index}].axis_values"),
                format!("missing {axis_id} axis value"),
                None::<String>,
            ));
            continue;
        };
        if !x.is_finite() {
            issues.push(issue(
                "error",
                "invalid_curve_point_axis",
                format!("points[{index}].axis_values.{axis_id}"),
                "curve axis values must be finite",
                None::<String>,
            ));
        }
        if definition.independent_axes[0].axis == CurveAxis::Frequency && x <= 0.0 {
            issues.push(issue(
                "error",
                "frequency_curve_non_positive_frequency",
                format!("points[{index}].axis_values.{axis_id}"),
                "frequency curves require positive frequencies",
                None::<String>,
            ));
        }
        if !matches!(
            definition.interpolation,
            CurveInterpolation::StepPrevious | CurveInterpolation::StepNext
        ) && !seen.insert(x.to_bits())
        {
            issues.push(issue(
                "error",
                "duplicate_curve_x",
                format!("points[{index}].axis_values.{axis_id}"),
                "curve x values must be unique unless a step interpolation explicitly allows duplicates",
                None::<String>,
            ));
        }
        for dependent in &definition.dependent_values {
            match point.values.get(&dependent.value_id) {
                Some(value) if value.is_finite() => {
                    if matches!(definition.interpolation, CurveInterpolation::LinearXLogY)
                        && *value <= 0.0
                    {
                        issues.push(issue(
                            "error",
                            "log_y_interpolation_non_positive_value",
                            format!("points[{index}].values.{}", dependent.value_id),
                            "linear_x_log_y interpolation requires positive dependent values",
                            None::<String>,
                        ));
                    }
                }
                Some(_) => issues.push(issue(
                    "error",
                    "invalid_curve_point_value",
                    format!("points[{index}].values.{}", dependent.value_id),
                    "curve dependent values must be finite",
                    None::<String>,
                )),
                None => issues.push(issue(
                    "error",
                    "missing_curve_point_value",
                    format!("points[{index}].values"),
                    format!("missing dependent value {}", dependent.value_id),
                    None::<String>,
                )),
            }
        }
    }
}

fn validate_curve_type_units(
    issues: &mut Vec<DefinitionValidationIssue>,
    definition: &EngineeringCurveDefinition,
) {
    for dependent in &definition.dependent_values {
        if dependent.component == FrequencyResponseComponent::Phase {
            continue;
        }
        match definition.curve_type {
            EngineeringCurveType::AntennaFactor
                if !matches!(dependent.unit.as_str(), "dB_per_meter" | "dBuV_per_m") =>
            {
                issues.push(issue(
                    "error",
                    "antenna_factor_unit_mismatch",
                    "dependent_values",
                    "antenna factor curves must use dB_per_meter or dBuV_per_m",
                    None::<String>,
                ));
            }
            EngineeringCurveType::CableLoss
            | EngineeringCurveType::AmplifierGain
            | EngineeringCurveType::AttenuatorLoss
                if !matches!(dependent.unit.as_str(), "dB" | "dB_per_meter") =>
            {
                issues.push(issue(
                    "error",
                    "rf_gain_loss_unit_mismatch",
                    "dependent_values",
                    "RF gain/loss curves must use dB-like units",
                    None::<String>,
                ));
            }
            EngineeringCurveType::Uncertainty
                if !matches!(dependent.unit.as_str(), "dB" | "percent" | "dimensionless") =>
            {
                issues.push(issue(
                    "error",
                    "uncertainty_unit_mismatch",
                    "dependent_values",
                    "uncertainty curves must use dB, percent or dimensionless units",
                    None::<String>,
                ));
            }
            _ => {}
        }
        if matches!(definition.interpolation, CurveInterpolation::LinearXLogY)
            && !is_logarithmic_unit(&dependent.unit)
        {
            issues.push(issue(
                "warning",
                "linear_x_log_y_with_linear_unit",
                "interpolation",
                "linear_x_log_y is unusual for a non-logarithmic dependent unit",
                None::<String>,
            ));
        }
    }
}

fn first_axis_value(
    definition: &EngineeringCurveDefinition,
    point: &EngineeringCurvePoint,
) -> Option<f64> {
    definition
        .independent_axes
        .first()
        .and_then(|axis| point.axis_values.get(axis.axis.as_str()))
        .copied()
}

fn interpolate_curve_value(
    definition: &EngineeringCurveDefinition,
    points: &[EngineeringCurvePoint],
    x: f64,
    value_id: &str,
) -> Result<f64, Vec<DefinitionValidationIssue>> {
    if points.len() == 1 {
        return Ok(points[0].values[value_id]);
    }
    let pair = points
        .windows(2)
        .find(|window| {
            let left_x = first_axis_value(definition, &window[0]).unwrap();
            let right_x = first_axis_value(definition, &window[1]).unwrap();
            x >= left_x && x <= right_x
        })
        .unwrap_or_else(|| {
            let first_x = first_axis_value(definition, &points[0]).unwrap();
            if x < first_x {
                &points[0..2]
            } else {
                &points[points.len() - 2..]
            }
        });
    let left_x = first_axis_value(definition, &pair[0]).unwrap();
    let right_x = first_axis_value(definition, &pair[1]).unwrap();
    let left_y = pair[0].values[value_id];
    let right_y = pair[1].values[value_id];
    let value = match definition.interpolation {
        CurveInterpolation::LinearXLinearY => {
            left_y + (x - left_x) * (right_y - left_y) / (right_x - left_x)
        }
        CurveInterpolation::LogXLinearY => {
            let lx = left_x.log10();
            let rx = right_x.log10();
            left_y + (x.log10() - lx) * (right_y - left_y) / (rx - lx)
        }
        CurveInterpolation::LinearXLogY => {
            if left_y <= 0.0 || right_y <= 0.0 {
                return Err(vec![issue(
                    "error",
                    "log_y_interpolation_non_positive_value",
                    "points",
                    "linear_x_log_y interpolation requires positive y values",
                    None::<String>,
                )]);
            }
            let ly = left_y.log10();
            let ry = right_y.log10();
            10f64.powf(ly + (x - left_x) * (ry - ly) / (right_x - left_x))
        }
        CurveInterpolation::Nearest => {
            if (x - left_x).abs() <= (right_x - x).abs() {
                left_y
            } else {
                right_y
            }
        }
        CurveInterpolation::StepPrevious => left_y,
        CurveInterpolation::StepNext => right_y,
    };
    Ok(value)
}

fn validate_refs(
    issues: &mut Vec<DefinitionValidationIssue>,
    refs: &[DefinitionReference],
    path: &str,
) {
    let mut seen = BTreeSet::new();
    for (index, reference) in refs.iter().enumerate() {
        validate_ref(issues, reference, format!("{path}[{index}]"));
        if !seen.insert((
            reference.entity_id.as_str(),
            reference.revision_id.as_deref().unwrap_or(""),
        )) {
            issues.push(issue(
                "error",
                "duplicate_definition_reference",
                format!("{path}[{index}]"),
                "definition references must not be duplicated",
                None::<String>,
            ));
        }
    }
}

fn validate_ref(
    issues: &mut Vec<DefinitionValidationIssue>,
    reference: &DefinitionReference,
    path: impl AsRef<str>,
) {
    let path = path.as_ref();
    require_id(issues, &reference.entity_id, format!("{path}.entity_id"));
    if let Some(revision_id) = &reference.revision_id {
        require_id(issues, revision_id, format!("{path}.revision_id"));
    }
}

fn validate_excitation(
    issues: &mut Vec<DefinitionValidationIssue>,
    excitation: Option<&ExcitationRequirement>,
    path: impl AsRef<str>,
) {
    let Some(excitation) = excitation else { return };
    let path = path.as_ref();
    if matches!(excitation.excitation_kind, ExcitationKind::None)
        && (excitation.nominal_value.is_some() || excitation.unit.is_some())
    {
        issues.push(issue(
            "warning",
            "none_excitation_has_value",
            path,
            "excitation_kind=none should not carry a nominal value or unit",
            None::<String>,
        ));
    }
    if let Some(value) = excitation.nominal_value {
        if !value.is_finite() || value < 0.0 {
            issues.push(issue(
                "error",
                "invalid_excitation_value",
                format!("{path}.nominal_value"),
                "excitation nominal value must be finite and non-negative",
                None::<String>,
            ));
        }
    }
    if let Some(unit) = &excitation.unit {
        match excitation.excitation_kind {
            ExcitationKind::Voltage | ExcitationKind::Bridge => {
                validate_quantity_unit(
                    issues,
                    PhysicalQuantity::Voltage,
                    unit,
                    format!("{path}.unit"),
                );
            }
            ExcitationKind::Current | ExcitationKind::Iepe => {
                validate_quantity_unit(
                    issues,
                    PhysicalQuantity::Current,
                    unit,
                    format!("{path}.unit"),
                );
            }
            ExcitationKind::Charge => {
                validate_quantity_unit(
                    issues,
                    PhysicalQuantity::ElectricCharge,
                    unit,
                    format!("{path}.unit"),
                );
            }
            ExcitationKind::None | ExcitationKind::External => {}
        }
    }
}

fn validate_quantity_unit(
    issues: &mut Vec<DefinitionValidationIssue>,
    quantity: PhysicalQuantity,
    unit: &str,
    path: impl AsRef<str>,
) {
    let path = path.as_ref();
    let unit = unit.trim();
    if unit.is_empty() {
        issues.push(issue(
            "error",
            "missing_unit",
            path,
            "unit is required",
            Some("Use an explicit unit from the measurement engineering unit registry."),
        ));
        return;
    }
    match unit_quantity(unit) {
        Some(unit_family) if unit_family == quantity => {}
        Some(PhysicalQuantity::Pressure)
            if quantity == PhysicalQuantity::SoundPressure && unit == "Pa" => {}
        Some(PhysicalQuantity::ElectricCharge)
            if quantity == PhysicalQuantity::Charge && matches!(unit, "C" | "pC") => {}
        Some(PhysicalQuantity::Dimensionless)
            if matches!(
                unit,
                "mV_per_g"
                    | "V_per_g"
                    | "pC_per_N"
                    | "V_per_A"
                    | "mV_per_A"
                    | "A_per_V"
                    | "V_per_V"
                    | "dB_ohm"
                    | "dB_per_meter"
                    | "dB_per_microampere"
            ) => {}
        Some(unit_family) => issues.push(issue(
            "error",
            "quantity_unit_mismatch",
            path,
            format!("unit {unit} belongs to {unit_family:?}, not {quantity:?}"),
            Some(
                "Choose a unit from the same physical quantity family or a declared transfer unit.",
            ),
        )),
        None => issues.push(issue(
            "error",
            "unknown_unit",
            path,
            format!("unit is not in the registry: {unit}"),
            Some(
                "Add the unit deliberately to the equipment measurement registry before using it.",
            ),
        )),
    }
}

fn require_schema(
    issues: &mut Vec<DefinitionValidationIssue>,
    observed: &str,
    expected: &'static str,
    path: &str,
) {
    if observed != expected {
        issues.push(issue(
            "error",
            "unsupported_measurement_engineering_schema",
            path,
            format!("unsupported schema version: {observed}"),
            Some(expected),
        ));
    }
}

fn require_finite(
    issues: &mut Vec<DefinitionValidationIssue>,
    value: Option<f64>,
    path: &str,
    missing_code: &'static str,
) -> Option<f64> {
    match value {
        Some(value) if value.is_finite() => Some(value),
        Some(_) => {
            issues.push(issue(
                "error",
                "invalid_numeric_value",
                path,
                "numeric value must be finite",
                None::<String>,
            ));
            None
        }
        None => {
            issues.push(issue(
                "error",
                missing_code,
                path,
                "numeric value is required",
                None::<String>,
            ));
            None
        }
    }
}

fn require_text(issues: &mut Vec<DefinitionValidationIssue>, value: &str, path: impl AsRef<str>) {
    let path = path.as_ref();
    if value.trim().is_empty() {
        issues.push(issue(
            "error",
            "blank_text",
            path,
            "text value must not be blank",
            None::<String>,
        ));
    }
}

fn require_id(issues: &mut Vec<DefinitionValidationIssue>, value: &str, path: impl AsRef<str>) {
    let path = path.as_ref();
    if value.trim().is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        issues.push(issue(
            "error",
            "invalid_identifier",
            path,
            "identifier must use ASCII letters, digits, hyphen or underscore",
            None::<String>,
        ));
    }
}

fn is_sha256_checksum(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("sha256:") else {
        return false;
    };
    rest.len() == 64
        && rest
            .chars()
            .all(|character| matches!(character, '0'..='9' | 'a'..='f'))
}

fn parse_issue(
    kind: MeasurementEngineeringAggregateKind,
    message: String,
) -> DefinitionValidationIssue {
    issue(
        "error",
        "invalid_measurement_engineering_definition_json",
        "$",
        format!("invalid {} definition JSON: {message}", kind.as_str()),
        Some("Send a JSON object matching the requested measurement engineering schema."),
    )
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

fn canonicalize_json_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let mut sorted = Map::new();
            for (key, mut value) in std::mem::take(map) {
                canonicalize_json_value(&mut value);
                sorted.insert(key, value);
            }
            *map = sorted;
            map.sort_keys();
        }
        Value::Array(values) => {
            for value in values {
                canonicalize_json_value(value);
            }
        }
        _ => {}
    }
}

fn default_true() -> bool {
    true
}

fn default_frequency_domain_spectrum() -> SignalRepresentation {
    SignalRepresentation::FrequencyDomainSpectrum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalization_is_stable_for_map_order() {
        let mut first = demo_curve();
        let mut second = demo_curve();
        first.metadata.insert("b".to_owned(), Value::from(2));
        first.metadata.insert("a".to_owned(), Value::from(1));
        second.metadata.insert("a".to_owned(), Value::from(1));
        second.metadata.insert("b".to_owned(), Value::from(2));

        let first = MeasurementEngineeringDefinition::Curve(first)
            .canonicalize()
            .unwrap();
        let second = MeasurementEngineeringDefinition::Curve(second)
            .canonicalize()
            .unwrap();

        assert_eq!(first.canonical_json, second.canonical_json);
        assert_eq!(first.definition_checksum, second.definition_checksum);
    }

    #[test]
    fn checksum_changes_when_curve_points_change() {
        let original = MeasurementEngineeringDefinition::Curve(demo_curve())
            .canonicalize()
            .unwrap();
        let mut changed = demo_curve();
        changed.points[1]
            .values
            .insert("correction_db".to_owned(), 2.0);

        let changed = MeasurementEngineeringDefinition::Curve(changed)
            .canonicalize()
            .unwrap();

        assert_ne!(original.definition_checksum, changed.definition_checksum);
    }

    #[test]
    fn evaluates_log_frequency_curve() {
        let curve = demo_curve();
        let result = evaluate_engineering_curve(
            &curve,
            BTreeMap::from([("frequency".to_owned(), 100_000_000.0)]),
            "curve-rev-0001",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap();

        assert!((result.values["correction_db"] - 1.0).abs() < 1e-9);
        assert!(!result.extrapolated);
    }

    #[test]
    fn rejects_forbidden_curve_extrapolation() {
        let curve = demo_curve();
        let error = evaluate_engineering_curve(
            &curve,
            BTreeMap::from([("frequency".to_owned(), 1_000.0)]),
            "curve-rev-0001",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap_err();

        assert!(error
            .iter()
            .any(|issue| issue.code == "curve_extrapolation_forbidden"));
    }

    #[test]
    fn rejects_linear_x_log_y_with_non_positive_curve_values() {
        let mut curve = demo_curve();
        curve.interpolation = CurveInterpolation::LinearXLogY;

        let issues = validate_engineering_curve_definition(&curve);

        assert!(issues
            .iter()
            .any(|issue| issue.code == "log_y_interpolation_non_positive_value"));
    }

    #[test]
    fn rejects_log_x_curve_evaluation_with_non_positive_axis_request() {
        let mut curve = demo_curve();
        curve.extrapolation_policy = ExtrapolationPolicy::Allow;

        let error = evaluate_engineering_curve(
            &curve,
            BTreeMap::from([("frequency".to_owned(), 0.0)]),
            "curve-rev-0001",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap_err();

        assert!(error
            .iter()
            .any(|issue| issue.code == "log_interpolation_non_positive_x"));
    }

    #[test]
    fn rejects_non_finite_curve_evaluation_result() {
        let mut curve = demo_curve();
        curve.interpolation = CurveInterpolation::LinearXLogY;
        curve.extrapolation_policy = ExtrapolationPolicy::Allow;
        for (index, point) in curve.points.iter_mut().enumerate() {
            point
                .values
                .insert("correction_db".to_owned(), 10f64.powi(index as i32));
        }

        let error = evaluate_engineering_curve(
            &curve,
            BTreeMap::from([("frequency".to_owned(), f64::MAX)]),
            "curve-rev-0001",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap_err();

        assert!(error
            .iter()
            .any(|issue| issue.code == "invalid_curve_evaluation_result"));
    }

    #[test]
    fn rejects_uppercase_curve_source_checksum() {
        let mut curve = demo_curve();
        curve.source_checksum = Some(
            "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_owned(),
        );

        let issues = validate_engineering_curve_definition(&curve);

        assert!(issues
            .iter()
            .any(|issue| issue.code == "invalid_source_checksum"));
    }

    #[test]
    fn rejects_unknown_expression_identifier() {
        let mut scaling = demo_scaling();
        scaling.scaling_kind = ScalingKind::Expression;
        scaling.parameters.expression = Some("system(input)".to_owned());

        let issues = validate_scaling_profile_definition(&scaling);

        assert!(issues
            .iter()
            .any(|issue| issue.code == "unknown_expression_identifier"));
    }

    #[test]
    fn evaluates_linear_two_point_polynomial_and_lookup_scaling() {
        let mut scaling = demo_scaling();
        assert_eq!(evaluate_scaling_profile(&scaling, 2.0).unwrap(), 200.0);

        scaling.scaling_kind = ScalingKind::TwoPoint;
        scaling.parameters = ScalingParameters {
            input_point_1: Some(0.0),
            output_point_1: Some(0.0),
            input_point_2: Some(10.0),
            output_point_2: Some(5.0),
            ..ScalingParameters::default()
        };
        assert_eq!(evaluate_scaling_profile(&scaling, 4.0).unwrap(), 2.0);

        scaling.scaling_kind = ScalingKind::Polynomial;
        scaling.parameters = ScalingParameters {
            coefficients: vec![1.0, 2.0, 3.0],
            ..ScalingParameters::default()
        };
        assert_eq!(evaluate_scaling_profile(&scaling, 2.0).unwrap(), 17.0);

        scaling.scaling_kind = ScalingKind::LookupTable;
        scaling.parameters = ScalingParameters {
            points: vec![
                ScalingPoint {
                    input: 0.0,
                    output: 0.0,
                },
                ScalingPoint {
                    input: 10.0,
                    output: 100.0,
                },
            ],
            interpolation: Some(ScalingInterpolation::Linear),
            extrapolation_policy: Some(ExtrapolationPolicy::Forbidden),
            ..ScalingParameters::default()
        };
        assert_eq!(evaluate_scaling_profile(&scaling, 2.5).unwrap(), 25.0);
    }

    #[test]
    fn rejects_non_finite_scaling_result() {
        let mut scaling = demo_scaling();
        scaling.scaling_kind = ScalingKind::Polynomial;
        scaling.parameters = ScalingParameters {
            coefficients: vec![0.0, 0.0, 1.0],
            ..ScalingParameters::default()
        };

        let error = evaluate_scaling_profile(&scaling, f64::MAX).unwrap_err();

        assert!(error
            .iter()
            .any(|issue| issue.code == "invalid_scaling_result"));
    }

    #[test]
    fn validates_sensor_daq_and_recipe_compatibility() {
        let sensor = demo_sensor();
        assert!(validate_sensor_definition(&sensor).is_empty());

        let daq = demo_daq();
        assert!(validate_daq_channel_profile_definition(&daq).is_empty());

        let scaling = demo_scaling();
        let recipe = demo_recipe();
        let issues = validate_acquisition_channel_recipe_with_context(
            &recipe,
            ResolvedAcquisitionRecipeContext {
                daq_channel_profile: Some(&daq),
                sensor_definition: Some(&sensor),
                scaling_profile: Some(&scaling),
                correction_curves: vec![&demo_curve()],
            },
        );

        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn detects_sample_rate_and_range_violations() {
        let sensor = demo_sensor();
        let daq = demo_daq();
        let scaling = demo_scaling();
        let mut recipe = demo_recipe();
        recipe.sample_rate = 2_000_000.0;
        recipe.range.maximum = 20.0;

        let issues = validate_acquisition_channel_recipe_with_context(
            &recipe,
            ResolvedAcquisitionRecipeContext {
                daq_channel_profile: Some(&daq),
                sensor_definition: Some(&sensor),
                scaling_profile: Some(&scaling),
                correction_curves: Vec::new(),
            },
        );

        assert!(issues
            .iter()
            .any(|issue| issue.code == "recipe_sample_rate_exceeds_daq"));
        assert!(issues
            .iter()
            .any(|issue| issue.code == "recipe_range_not_supported"));
    }

    #[test]
    fn rejects_frequency_domain_scaling_and_invalid_sample_limits() {
        let mut definition = demo_scaling();
        definition.signal_representation = SignalRepresentation::FrequencyDomainSpectrum;
        definition.input_limits = Some(SampleInputLimits {
            minimum: 10.0,
            maximum: -10.0,
            handling: SampleLimitHandling::MarkClipped,
        });

        let issues = validate_scaling_profile_definition(&definition);

        assert!(issues
            .iter()
            .any(|issue| issue.code == "scaling_requires_time_domain_samples"));
        assert!(issues
            .iter()
            .any(|issue| issue.code == "invalid_sample_input_limits"));
    }

    #[test]
    fn validates_amplitude_and_optional_phase_frequency_response() {
        let mut definition = demo_curve();
        definition.dependent_values.push(CurveValueDefinition {
            value_id: "phase_correction_deg".to_owned(),
            quantity: PhysicalQuantity::Angle,
            unit: "deg".to_owned(),
            component: FrequencyResponseComponent::Phase,
            operation: CorrectionOperation::Add,
        });
        for point in &mut definition.points {
            point.values.insert("phase_correction_deg".to_owned(), 0.0);
        }

        let issues = validate_engineering_curve_definition(&definition);

        assert!(
            !issues.iter().any(|item| item.severity == "error"),
            "{issues:?}"
        );
    }

    #[test]
    fn rejects_frequency_response_without_frequency_axis() {
        let mut definition = demo_curve();
        definition.independent_axes[0] = CurveAxisDefinition {
            axis: CurveAxis::Time,
            quantity: PhysicalQuantity::Time,
            unit: "s".to_owned(),
        };
        for point in &mut definition.points {
            let frequency = point.axis_values.remove("frequency").unwrap();
            point.axis_values.insert("time".to_owned(), frequency);
        }

        let issues = validate_engineering_curve_definition(&definition);

        assert!(issues
            .iter()
            .any(|issue| issue.code == "frequency_response_axis_missing"));
    }

    fn demo_scaling() -> ScalingProfileDefinition {
        ScalingProfileDefinition {
            definition_schema_version: SCALING_PROFILE_DEFINITION_SCHEMA_VERSION.to_owned(),
            scaling_profile_id: "demo-current-probe-10mv-a".to_owned(),
            label: "Demo current probe 10mV/A".to_owned(),
            input_quantity: PhysicalQuantity::Voltage,
            input_unit: "V".to_owned(),
            output_quantity: PhysicalQuantity::Current,
            output_unit: "A".to_owned(),
            signal_representation: SignalRepresentation::TimeDomainSamples,
            scaling_kind: ScalingKind::Linear,
            parameters: ScalingParameters {
                scale: Some(100.0),
                offset: Some(0.0),
                ..ScalingParameters::default()
            },
            input_limits: None,
            validity_domain: BTreeMap::new(),
            uncertainty: None,
            source_reference: None,
            metadata: BTreeMap::new(),
        }
    }

    fn demo_curve() -> EngineeringCurveDefinition {
        EngineeringCurveDefinition {
            definition_schema_version: ENGINEERING_CURVE_DEFINITION_SCHEMA_VERSION.to_owned(),
            curve_id: "demo-cable-loss".to_owned(),
            curve_type: EngineeringCurveType::CableLoss,
            label: "Demo cable loss".to_owned(),
            signal_representation: SignalRepresentation::FrequencyDomainSpectrum,
            independent_axes: vec![CurveAxisDefinition {
                axis: CurveAxis::Frequency,
                quantity: PhysicalQuantity::Frequency,
                unit: "Hz".to_owned(),
            }],
            dependent_values: vec![CurveValueDefinition {
                value_id: "correction_db".to_owned(),
                quantity: PhysicalQuantity::Dimensionless,
                unit: "dB".to_owned(),
                component: FrequencyResponseComponent::Amplitude,
                operation: CorrectionOperation::Add,
            }],
            units: BTreeMap::new(),
            points: vec![
                EngineeringCurvePoint {
                    axis_values: BTreeMap::from([("frequency".to_owned(), 10_000_000.0)]),
                    values: BTreeMap::from([("correction_db".to_owned(), 0.0)]),
                },
                EngineeringCurvePoint {
                    axis_values: BTreeMap::from([("frequency".to_owned(), 100_000_000.0)]),
                    values: BTreeMap::from([("correction_db".to_owned(), 1.0)]),
                },
                EngineeringCurvePoint {
                    axis_values: BTreeMap::from([("frequency".to_owned(), 1_000_000_000.0)]),
                    values: BTreeMap::from([("correction_db".to_owned(), 3.0)]),
                },
            ],
            interpolation: CurveInterpolation::LogXLinearY,
            extrapolation_policy: ExtrapolationPolicy::Forbidden,
            validity_domain: BTreeMap::new(),
            conditions: BTreeMap::new(),
            source_document_reference: None,
            source_checksum: Some(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_owned(),
            ),
            status: None,
            metadata: BTreeMap::new(),
        }
    }

    fn demo_sensor() -> SensorDefinition {
        SensorDefinition {
            definition_schema_version: SENSOR_DEFINITION_SCHEMA_VERSION.to_owned(),
            sensor_definition_id: "demo-current-probe".to_owned(),
            manufacturer: "EMC Locus".to_owned(),
            model_name: "Demo Current Probe 10mV/A".to_owned(),
            variant: None,
            sensor_family: SensorFamily::CurrentProbe,
            physical_input_quantity: PhysicalQuantity::Current,
            engineering_output_quantity: PhysicalQuantity::Current,
            engineering_output_unit: "A".to_owned(),
            electrical_output_quantity: PhysicalQuantity::Voltage,
            electrical_output_unit: "V".to_owned(),
            signal_domain: SignalDomain::AnalogVoltage,
            technology_tags: vec![TechnologyTag::VoltageInput],
            required_excitation: None,
            input_mode_requirement: Some(DaqInputMode::SingleEnded),
            nominal_range: Some(NumericRange {
                minimum: Some(-100.0),
                maximum: Some(100.0),
                unit: "A".to_owned(),
            }),
            safe_range: None,
            orientation_axes: Vec::new(),
            settling_time_ms: Some(1.0),
            frequency_range: Some(FrequencyRange {
                minimum_hz: 10.0,
                maximum_hz: 100_000_000.0,
            }),
            temperature_range: None,
            scaling_profile_refs: vec![DefinitionReference {
                entity_id: "demo-current-probe-10mv-a".to_owned(),
                revision_id: None,
                require_approved: true,
            }],
            correction_curve_refs: vec![DefinitionReference {
                entity_id: "demo-cable-loss".to_owned(),
                revision_id: None,
                require_approved: true,
            }],
            metadata: BTreeMap::new(),
        }
    }

    fn demo_daq() -> DaqChannelProfileDefinition {
        DaqChannelProfileDefinition {
            definition_schema_version: DAQ_CHANNEL_PROFILE_DEFINITION_SCHEMA_VERSION.to_owned(),
            daq_channel_profile_id: "demo-daq-ai-10v".to_owned(),
            label: "Demo DAQ AI +/-10V".to_owned(),
            channel_kind: DaqChannelKind::AnalogInput,
            signal_domain: SignalDomain::AnalogVoltage,
            input_quantity: Some(PhysicalQuantity::Voltage),
            input_unit: Some("V".to_owned()),
            supported_ranges: vec![SupportedRange {
                minimum: -10.0,
                maximum: 10.0,
                unit: "V".to_owned(),
            }],
            resolution_bits: Some(16),
            max_sampling_rate: Some(1_000_000.0),
            min_sampling_rate: Some(1.0),
            coupling_modes: vec![CouplingMode::Dc],
            input_modes: vec![DaqInputMode::SingleEnded, DaqInputMode::Differential],
            anti_alias_filter: Some("optional".to_owned()),
            excitation_capabilities: Vec::new(),
            bridge_completion: None,
            iepe_support: false,
            isolation: None,
            synchronization: Some("shared_clock_ready".to_owned()),
            triggering: Some("software_or_external".to_owned()),
            metadata: BTreeMap::new(),
        }
    }

    fn demo_recipe() -> AcquisitionChannelRecipeDefinition {
        AcquisitionChannelRecipeDefinition {
            definition_schema_version: ACQUISITION_CHANNEL_RECIPE_DEFINITION_SCHEMA_VERSION
                .to_owned(),
            recipe_id: "current-a".to_owned(),
            label: "current_A".to_owned(),
            output_channel_name: "current_A".to_owned(),
            output_quantity: PhysicalQuantity::Current,
            output_unit: "A".to_owned(),
            daq_channel_profile_ref: DefinitionReference {
                entity_id: "demo-daq-ai-10v".to_owned(),
                revision_id: None,
                require_approved: true,
            },
            sensor_definition_ref: Some(DefinitionReference {
                entity_id: "demo-current-probe".to_owned(),
                revision_id: None,
                require_approved: true,
            }),
            scaling_profile_ref: Some(DefinitionReference {
                entity_id: "demo-current-probe-10mv-a".to_owned(),
                revision_id: None,
                require_approved: true,
            }),
            correction_curve_refs: vec![DefinitionReference {
                entity_id: "demo-cable-loss".to_owned(),
                revision_id: None,
                require_approved: true,
            }],
            sample_rate: 1_000_000.0,
            range: SupportedRange {
                minimum: -10.0,
                maximum: 10.0,
                unit: "V".to_owned(),
            },
            coupling: CouplingMode::Dc,
            input_mode: DaqInputMode::SingleEnded,
            excitation: None,
            filtering: None,
            triggering: None,
            validation_rules: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}
