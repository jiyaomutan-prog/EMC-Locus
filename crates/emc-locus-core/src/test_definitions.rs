use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub const TEST_TEMPLATE_DEFINITION_SCHEMA_VERSION: &str = "emc-locus.test-template-definition.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementAxis {
    FrequencySweep,
    TimeSeries,
    EventTriggered,
    MixedTimeFrequency,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableValueType {
    Number,
    Integer,
    Boolean,
    Text,
    Enum,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VariableDefaultValue {
    Integer(i64),
    Number(f64),
    Boolean(bool),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VariableConstraints {
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VariableDefinition {
    pub variable_id: String,
    pub label: String,
    pub value_type: VariableValueType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<VariableDefaultValue>,
    pub constraints: VariableConstraints,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableLockPolicyKind {
    EditableUntilCampaignFreeze,
    EditableUntilExecution,
    AdminOnly,
    InvestigationOnly,
    Immutable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariableLockPolicy {
    pub variable_id: String,
    pub policy: VariableLockPolicyKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationRequirement {
    Required,
    NotRequired,
    IfUsed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentSubstitutionPolicy {
    NoSubstitution,
    SameCategory,
    SameCapability,
    ApprovedEquivalent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstrumentationChainSlot {
    pub slot_id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_capability: Option<String>,
    pub required: bool,
    pub calibration_requirement: CalibrationRequirement,
    pub substitution_policy: InstrumentSubstitutionPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on_slots: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStepKind {
    Prepare,
    ConfigureInstrument,
    Acquire,
    OperatorDecision,
    PostProcess,
    Verify,
    Finish,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchRule {
    pub rule_id: String,
    pub condition: String,
    pub destination_step_id: String,
    #[serde(default)]
    pub allow_cycle: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionSequenceStep {
    pub step_id: String,
    pub order: u32,
    pub kind: ExecutionStepKind,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_slots: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branches: Vec<BranchRule>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitKind {
    TimeLimit,
    FrequencyLimit,
    ScalarThreshold,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LimitDefinition {
    pub limit_id: String,
    pub kind: LimitKind,
    pub axis: MeasurementAxis,
    pub unit: String,
    pub application_domain: String,
    pub source_reference: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attention_rule: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variable_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostProcessingOperationType {
    Correction,
    Fft,
    Windowing,
    Resampling,
    HarmonicCalculation,
    EventCounting,
    ChannelMath,
    Peak,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PostProcessingDefinition {
    pub operation_id: String,
    pub order: u32,
    pub operation_type: PostProcessingOperationType,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    #[serde(default)]
    pub parameters: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateRevisionStatus {
    Draft,
    UnderReview,
    Approved,
    Suspended,
    Superseded,
    Retired,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestTemplateDefinition {
    pub definition_schema_version: String,
    pub title: String,
    pub description: String,
    pub measurement_axis: MeasurementAxis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub standard_references: Vec<String>,
    pub variables: Vec<VariableDefinition>,
    pub lock_policy: Vec<VariableLockPolicy>,
    pub instrumentation_chain: Vec<InstrumentationChainSlot>,
    pub entry_step_id: String,
    pub sequence: Vec<ExecutionSequenceStep>,
    pub limits: Vec<LimitDefinition>,
    pub post_processing: Vec<PostProcessingDefinition>,
    #[serde(default)]
    pub method_parameters: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalTestTemplateDefinition {
    pub definition_schema_version: String,
    pub canonical_json: String,
    pub definition_checksum: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestTemplateValidationError {
    pub code: &'static str,
    pub message: String,
}

impl TestTemplateValidationError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl TestTemplateDefinition {
    pub fn from_json_str(value: &str) -> Result<Self, TestTemplateValidationError> {
        serde_json::from_str(value).map_err(|error| {
            TestTemplateValidationError::new(
                "invalid_test_template_definition_json",
                error.to_string(),
            )
        })
    }

    pub fn validate(&self) -> Result<(), TestTemplateValidationError> {
        self.canonicalize().map(|_| ())
    }

    pub fn canonicalize(
        &self,
    ) -> Result<CanonicalTestTemplateDefinition, TestTemplateValidationError> {
        validate_definition(self)?;
        let mut value = serde_json::to_value(self).map_err(|error| {
            TestTemplateValidationError::new(
                "test_template_definition_serialization_failed",
                error.to_string(),
            )
        })?;
        canonicalize_json_value(&mut value);
        let canonical_json = serde_json::to_string(&value).map_err(|error| {
            TestTemplateValidationError::new(
                "test_template_definition_serialization_failed",
                error.to_string(),
            )
        })?;
        let digest = Sha256::digest(canonical_json.as_bytes());
        Ok(CanonicalTestTemplateDefinition {
            definition_schema_version: self.definition_schema_version.clone(),
            canonical_json,
            definition_checksum: format!("sha256:{digest:x}"),
        })
    }
}

pub fn canonicalize_test_template_definition_json(
    value: &str,
) -> Result<CanonicalTestTemplateDefinition, TestTemplateValidationError> {
    TestTemplateDefinition::from_json_str(value)?.canonicalize()
}

fn validate_definition(
    definition: &TestTemplateDefinition,
) -> Result<(), TestTemplateValidationError> {
    if definition.definition_schema_version != TEST_TEMPLATE_DEFINITION_SCHEMA_VERSION {
        return Err(TestTemplateValidationError::new(
            "unsupported_test_template_definition_schema",
            format!(
                "unsupported definition schema version: {}",
                definition.definition_schema_version
            ),
        ));
    }
    require_text(&definition.title, "title")?;
    require_text(&definition.description, "description")?;
    if matches!(
        (&definition.method_code, &definition.method_revision),
        (Some(_), None) | (None, Some(_))
    ) {
        return Err(TestTemplateValidationError::new(
            "invalid_method_reference",
            "method_code and method_revision must be provided together",
        ));
    }
    if let Some(method_code) = definition.method_code.as_deref() {
        require_token(method_code, "method_code")?;
    }
    if let Some(method_revision) = definition.method_revision.as_deref() {
        require_text(method_revision, "method_revision")?;
    }
    let variable_ids = validate_variables(&definition.variables)?;
    validate_lock_policy(&definition.lock_policy, &variable_ids)?;
    let slot_ids = validate_instrumentation_chain(&definition.instrumentation_chain)?;
    validate_sequence(&definition.sequence, &definition.entry_step_id, &slot_ids)?;
    validate_limits(&definition.limits, &variable_ids)?;
    validate_post_processing(&definition.post_processing)?;
    for reference in &definition.standard_references {
        require_text(reference, "standard_reference")?;
    }
    validate_json_map(&definition.method_parameters, "method_parameters")?;
    Ok(())
}

fn validate_variables(
    variables: &[VariableDefinition],
) -> Result<BTreeSet<String>, TestTemplateValidationError> {
    if variables.is_empty() {
        return Err(TestTemplateValidationError::new(
            "missing_variables",
            "at least one variable definition is required",
        ));
    }
    let mut ids = BTreeSet::new();
    for variable in variables {
        require_token(&variable.variable_id, "variable_id")?;
        require_text(&variable.label, "variable_label")?;
        if !ids.insert(variable.variable_id.clone()) {
            return Err(TestTemplateValidationError::new(
                "duplicate_variable_id",
                format!("duplicate variable id: {}", variable.variable_id),
            ));
        }
        if matches!(
            variable.value_type,
            VariableValueType::Number | VariableValueType::Integer
        ) && variable
            .constraints
            .unit
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        {
            return Err(TestTemplateValidationError::new(
                "missing_variable_unit",
                format!("numeric variable requires a unit: {}", variable.variable_id),
            ));
        }
        if let (Some(minimum), Some(maximum)) =
            (variable.constraints.minimum, variable.constraints.maximum)
        {
            if minimum > maximum {
                return Err(TestTemplateValidationError::new(
                    "invalid_numeric_bounds",
                    format!(
                        "minimum is greater than maximum for {}",
                        variable.variable_id
                    ),
                ));
            }
        }
        if variable.value_type == VariableValueType::Enum {
            if variable.constraints.enum_values.is_empty() {
                return Err(TestTemplateValidationError::new(
                    "empty_enum_values",
                    format!("enum variable has no values: {}", variable.variable_id),
                ));
            }
            let mut enum_values = BTreeSet::new();
            for value in &variable.constraints.enum_values {
                require_text(value, "enum_value")?;
                if !enum_values.insert(value.clone()) {
                    return Err(TestTemplateValidationError::new(
                        "duplicate_enum_value",
                        format!("duplicate enum value for {}", variable.variable_id),
                    ));
                }
            }
        }
        validate_default_value(variable)?;
    }
    Ok(ids)
}

fn validate_default_value(
    variable: &VariableDefinition,
) -> Result<(), TestTemplateValidationError> {
    let Some(default_value) = variable.default_value.as_ref() else {
        return Ok(());
    };
    match (&variable.value_type, default_value) {
        (VariableValueType::Number, VariableDefaultValue::Number(value)) => {
            validate_numeric_default(variable, *value)
        }
        (VariableValueType::Integer, VariableDefaultValue::Integer(value)) => {
            validate_numeric_default(variable, *value as f64)
        }
        (VariableValueType::Boolean, VariableDefaultValue::Boolean(_)) => Ok(()),
        (VariableValueType::Text, VariableDefaultValue::Text(value)) => {
            require_text(value, "text_default")
        }
        (VariableValueType::Enum, VariableDefaultValue::Text(value)) => {
            if variable
                .constraints
                .enum_values
                .iter()
                .any(|item| item == value)
            {
                Ok(())
            } else {
                Err(TestTemplateValidationError::new(
                    "enum_default_not_allowed",
                    format!(
                        "enum default is not in allowed values: {}",
                        variable.variable_id
                    ),
                ))
            }
        }
        _ => Err(TestTemplateValidationError::new(
            "default_value_type_mismatch",
            format!(
                "default value does not match variable type: {}",
                variable.variable_id
            ),
        )),
    }
}

fn validate_numeric_default(
    variable: &VariableDefinition,
    value: f64,
) -> Result<(), TestTemplateValidationError> {
    if let Some(minimum) = variable.constraints.minimum {
        if value < minimum {
            return Err(TestTemplateValidationError::new(
                "default_below_minimum",
                format!(
                    "default value is below minimum for {}",
                    variable.variable_id
                ),
            ));
        }
    }
    if let Some(maximum) = variable.constraints.maximum {
        if value > maximum {
            return Err(TestTemplateValidationError::new(
                "default_above_maximum",
                format!(
                    "default value is above maximum for {}",
                    variable.variable_id
                ),
            ));
        }
    }
    Ok(())
}

fn validate_lock_policy(
    policies: &[VariableLockPolicy],
    variable_ids: &BTreeSet<String>,
) -> Result<(), TestTemplateValidationError> {
    let mut seen = BTreeSet::new();
    for policy in policies {
        require_token(&policy.variable_id, "lock_policy.variable_id")?;
        if !variable_ids.contains(&policy.variable_id) {
            return Err(TestTemplateValidationError::new(
                "unknown_lock_variable",
                format!(
                    "lock policy references unknown variable: {}",
                    policy.variable_id
                ),
            ));
        }
        if !seen.insert(policy.variable_id.clone()) {
            return Err(TestTemplateValidationError::new(
                "duplicate_lock_policy",
                format!("duplicate lock policy for variable: {}", policy.variable_id),
            ));
        }
    }
    Ok(())
}

fn validate_instrumentation_chain(
    slots: &[InstrumentationChainSlot],
) -> Result<BTreeSet<String>, TestTemplateValidationError> {
    let mut slot_ids = BTreeSet::new();
    for slot in slots {
        require_token(&slot.slot_id, "slot_id")?;
        require_text(&slot.label, "slot_label")?;
        if !slot_ids.insert(slot.slot_id.clone()) {
            return Err(TestTemplateValidationError::new(
                "duplicate_slot_id",
                format!("duplicate instrumentation slot: {}", slot.slot_id),
            ));
        }
        let category = slot
            .required_category
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let capability = slot
            .required_capability
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if category.is_none() && capability.is_none() {
            return Err(TestTemplateValidationError::new(
                "missing_slot_requirement",
                format!(
                    "instrumentation slot requires category or capability: {}",
                    slot.slot_id
                ),
            ));
        }
    }
    for slot in slots {
        for dependency in &slot.depends_on_slots {
            if !slot_ids.contains(dependency) {
                return Err(TestTemplateValidationError::new(
                    "unknown_slot_reference",
                    format!(
                        "instrumentation slot references unknown slot: {}",
                        dependency
                    ),
                ));
            }
            if dependency == &slot.slot_id {
                return Err(TestTemplateValidationError::new(
                    "self_slot_reference",
                    format!("instrumentation slot depends on itself: {}", slot.slot_id),
                ));
            }
        }
    }
    Ok(slot_ids)
}

fn validate_sequence(
    steps: &[ExecutionSequenceStep],
    entry_step_id: &str,
    slot_ids: &BTreeSet<String>,
) -> Result<(), TestTemplateValidationError> {
    if steps.is_empty() {
        return Err(TestTemplateValidationError::new(
            "missing_sequence_steps",
            "at least one execution sequence step is required",
        ));
    }
    require_token(entry_step_id, "entry_step_id")?;
    let mut step_ids = BTreeSet::new();
    let mut orders = BTreeSet::new();
    for step in steps {
        require_token(&step.step_id, "step_id")?;
        require_text(&step.label, "step_label")?;
        if !step_ids.insert(step.step_id.clone()) {
            return Err(TestTemplateValidationError::new(
                "duplicate_step_id",
                format!("duplicate sequence step: {}", step.step_id),
            ));
        }
        if !orders.insert(step.order) {
            return Err(TestTemplateValidationError::new(
                "duplicate_step_order",
                format!("duplicate sequence order: {}", step.order),
            ));
        }
        for slot in &step.required_slots {
            if !slot_ids.contains(slot) {
                return Err(TestTemplateValidationError::new(
                    "unknown_step_slot_reference",
                    format!("sequence step references unknown slot: {slot}"),
                ));
            }
        }
        let mut branch_ids = BTreeSet::new();
        for branch in &step.branches {
            require_token(&branch.rule_id, "branch_rule_id")?;
            require_text(&branch.condition, "branch_condition")?;
            require_token(&branch.destination_step_id, "branch_destination_step_id")?;
            if !branch_ids.insert(branch.rule_id.clone()) {
                return Err(TestTemplateValidationError::new(
                    "duplicate_branch_rule_id",
                    format!("duplicate branch rule: {}", branch.rule_id),
                ));
            }
        }
    }
    if !step_ids.contains(entry_step_id) {
        return Err(TestTemplateValidationError::new(
            "unknown_entry_step",
            format!("entry step does not exist: {entry_step_id}"),
        ));
    }

    let ordered_steps = ordered_step_ids(steps);
    let order_index = ordered_steps
        .iter()
        .enumerate()
        .map(|(index, id)| (id.clone(), index))
        .collect::<HashMap<_, _>>();
    let mut edges = HashMap::<String, Vec<(String, bool)>>::new();
    for (index, step_id) in ordered_steps.iter().enumerate() {
        if let Some(next_step) = ordered_steps.get(index + 1) {
            edges
                .entry(step_id.clone())
                .or_default()
                .push((next_step.clone(), true));
        }
    }
    for step in steps {
        for branch in &step.branches {
            if !step_ids.contains(&branch.destination_step_id) {
                return Err(TestTemplateValidationError::new(
                    "unknown_branch_destination",
                    format!(
                        "branch references unknown destination: {}",
                        branch.destination_step_id
                    ),
                ));
            }
            let source_order = order_index[&step.step_id];
            let destination_order = order_index[&branch.destination_step_id];
            if destination_order <= source_order && !branch.allow_cycle {
                return Err(TestTemplateValidationError::new(
                    "undeclared_sequence_cycle",
                    format!(
                        "branch creates a cycle without allow_cycle: {}",
                        branch.rule_id
                    ),
                ));
            }
            edges
                .entry(step.step_id.clone())
                .or_default()
                .push((branch.destination_step_id.clone(), branch.allow_cycle));
        }
    }
    let reachable = reachable_steps(entry_step_id, &edges);
    for step_id in step_ids {
        if !reachable.contains(&step_id) {
            return Err(TestTemplateValidationError::new(
                "unreachable_sequence_step",
                format!("sequence step is unreachable: {step_id}"),
            ));
        }
    }
    Ok(())
}

fn ordered_step_ids(steps: &[ExecutionSequenceStep]) -> Vec<String> {
    let mut ordered = steps
        .iter()
        .map(|step| (step.order, step.step_id.clone()))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(order, _)| *order);
    ordered.into_iter().map(|(_, step_id)| step_id).collect()
}

fn reachable_steps(
    entry_step_id: &str,
    edges: &HashMap<String, Vec<(String, bool)>>,
) -> BTreeSet<String> {
    let mut reachable = BTreeSet::new();
    let mut stack = vec![entry_step_id.to_owned()];
    while let Some(step_id) = stack.pop() {
        if !reachable.insert(step_id.clone()) {
            continue;
        }
        if let Some(next_steps) = edges.get(&step_id) {
            for (next_step, _) in next_steps {
                stack.push(next_step.clone());
            }
        }
    }
    reachable
}

fn validate_limits(
    limits: &[LimitDefinition],
    variable_ids: &BTreeSet<String>,
) -> Result<(), TestTemplateValidationError> {
    let mut limit_ids = BTreeSet::new();
    for limit in limits {
        require_token(&limit.limit_id, "limit_id")?;
        require_text(&limit.unit, "limit_unit")?;
        require_text(&limit.application_domain, "limit_application_domain")?;
        require_text(&limit.source_reference, "limit_source_reference")?;
        if !limit_ids.insert(limit.limit_id.clone()) {
            return Err(TestTemplateValidationError::new(
                "duplicate_limit_id",
                format!("duplicate limit id: {}", limit.limit_id),
            ));
        }
        if matches!(limit.kind, LimitKind::ScalarThreshold) && limit.threshold.is_none() {
            return Err(TestTemplateValidationError::new(
                "missing_scalar_threshold",
                format!("scalar limit requires threshold: {}", limit.limit_id),
            ));
        }
        for variable_ref in &limit.variable_refs {
            if !variable_ids.contains(variable_ref) {
                return Err(TestTemplateValidationError::new(
                    "unknown_limit_variable",
                    format!("limit references unknown variable: {variable_ref}"),
                ));
            }
        }
    }
    Ok(())
}

fn validate_post_processing(
    operations: &[PostProcessingDefinition],
) -> Result<(), TestTemplateValidationError> {
    let mut operation_ids = BTreeSet::new();
    let mut operation_orders = BTreeSet::new();
    let mut outputs = BTreeMap::<String, u32>::new();
    for operation in operations {
        require_token(&operation.operation_id, "post_processing.operation_id")?;
        if !operation_ids.insert(operation.operation_id.clone()) {
            return Err(TestTemplateValidationError::new(
                "duplicate_post_processing_operation",
                format!(
                    "duplicate post-processing operation: {}",
                    operation.operation_id
                ),
            ));
        }
        if !operation_orders.insert(operation.order) {
            return Err(TestTemplateValidationError::new(
                "duplicate_post_processing_order",
                format!("duplicate post-processing order: {}", operation.order),
            ));
        }
        if operation.inputs.is_empty() {
            return Err(TestTemplateValidationError::new(
                "missing_post_processing_inputs",
                format!(
                    "post-processing operation has no inputs: {}",
                    operation.operation_id
                ),
            ));
        }
        if operation.outputs.is_empty() {
            return Err(TestTemplateValidationError::new(
                "missing_post_processing_outputs",
                format!(
                    "post-processing operation has no outputs: {}",
                    operation.operation_id
                ),
            ));
        }
        for output in &operation.outputs {
            require_signal_reference(output, "post_processing.output")?;
            if outputs.insert(output.clone(), operation.order).is_some() {
                return Err(TestTemplateValidationError::new(
                    "duplicate_post_processing_output",
                    format!("duplicate post-processing output: {output}"),
                ));
            }
        }
        validate_json_map(&operation.parameters, "post_processing.parameters")?;
    }
    for operation in operations {
        for input in &operation.inputs {
            require_signal_reference(input, "post_processing.input")?;
            if let Some(source_order) = outputs.get(input) {
                if *source_order >= operation.order {
                    return Err(TestTemplateValidationError::new(
                        "invalid_post_processing_dependency",
                        format!(
                            "post-processing input is not produced by an earlier operation: {input}"
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_json_map(
    map: &BTreeMap<String, Value>,
    field: &'static str,
) -> Result<(), TestTemplateValidationError> {
    for (key, value) in map {
        require_token(key, field)?;
        validate_controlled_json_value(value, field)?;
    }
    Ok(())
}

fn validate_controlled_json_value(
    value: &Value,
    field: &'static str,
) -> Result<(), TestTemplateValidationError> {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(()),
        Value::Number(number) if number.is_i64() || number.is_u64() || number.is_f64() => Ok(()),
        Value::Array(values) => {
            for value in values {
                validate_controlled_json_value(value, field)?;
            }
            Ok(())
        }
        Value::Object(values) => {
            for (key, value) in values {
                require_token(key, field)?;
                validate_controlled_json_value(value, field)?;
            }
            Ok(())
        }
        Value::Number(_) => Err(TestTemplateValidationError::new(
            "invalid_json_number",
            format!("unsupported JSON number in {field}"),
        )),
    }
}

fn canonicalize_json_value(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values {
                canonicalize_json_value(value);
            }
        }
        Value::Object(values) => {
            let original = std::mem::take(values);
            let mut sorted = BTreeMap::new();
            for (key, mut value) in original {
                canonicalize_json_value(&mut value);
                sorted.insert(key, value);
            }
            *values = sorted.into_iter().collect::<Map<String, Value>>();
        }
        _ => {}
    }
}

fn require_token(value: &str, field: &'static str) -> Result<(), TestTemplateValidationError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(TestTemplateValidationError::new(
            "empty_token",
            format!("{field} is required"),
        ));
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(TestTemplateValidationError::new(
            "invalid_token",
            format!("{field} must be a stable ASCII token: {trimmed}"),
        ));
    }
    Ok(())
}

fn require_text(value: &str, field: &'static str) -> Result<(), TestTemplateValidationError> {
    if value.trim().is_empty() {
        return Err(TestTemplateValidationError::new(
            "empty_text",
            format!("{field} is required"),
        ));
    }
    Ok(())
}

fn require_signal_reference(
    value: &str,
    field: &'static str,
) -> Result<(), TestTemplateValidationError> {
    require_text(value, field)?;
    if value.contains(char::is_whitespace) {
        return Err(TestTemplateValidationError::new(
            "invalid_signal_reference",
            format!("{field} must not contain whitespace: {value}"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_serialization_and_checksum_are_deterministic() {
        let first = fixture_definition();
        let mut second = fixture_definition();
        second.method_parameters = BTreeMap::from([
            ("zeta".to_owned(), Value::Bool(true)),
            ("alpha".to_owned(), serde_json::json!({"b": 2, "a": [3, 1]})),
        ]);
        let mut first_with_same_params = first.clone();
        first_with_same_params.method_parameters = BTreeMap::from([
            ("alpha".to_owned(), serde_json::json!({"a": [3, 1], "b": 2})),
            ("zeta".to_owned(), Value::Bool(true)),
        ]);

        let first_canonical = first_with_same_params.canonicalize().unwrap();
        let second_canonical = second.canonicalize().unwrap();

        assert_eq!(
            first_canonical.canonical_json,
            second_canonical.canonical_json
        );
        assert_eq!(
            first_canonical.definition_checksum,
            second_canonical.definition_checksum
        );
        assert!(first_canonical.definition_checksum.starts_with("sha256:"));
    }

    #[test]
    fn rejects_invalid_variable_default() {
        let mut definition = fixture_definition();
        definition.variables[0].default_value = Some(VariableDefaultValue::Text("fast".to_owned()));

        let error = definition.validate().unwrap_err();

        assert_eq!(error.code, "default_value_type_mismatch");
    }

    #[test]
    fn rejects_lock_policy_for_missing_variable() {
        let mut definition = fixture_definition();
        definition.lock_policy[0].variable_id = "missing".to_owned();

        let error = definition.validate().unwrap_err();

        assert_eq!(error.code, "unknown_lock_variable");
    }

    #[test]
    fn rejects_duplicate_instrumentation_slots() {
        let mut definition = fixture_definition();
        definition
            .instrumentation_chain
            .push(definition.instrumentation_chain[0].clone());

        let error = definition.validate().unwrap_err();

        assert_eq!(error.code, "duplicate_slot_id");
    }

    #[test]
    fn rejects_branch_to_missing_step() {
        let mut definition = fixture_definition();
        definition.sequence[0].branches[0].destination_step_id = "missing".to_owned();

        let error = definition.validate().unwrap_err();

        assert_eq!(error.code, "unknown_branch_destination");
    }

    #[test]
    fn rejects_invalid_sequence_graph() {
        let mut definition = fixture_definition();
        definition.sequence[1].branches.push(BranchRule {
            rule_id: "retry".to_owned(),
            condition: "operator_retry".to_owned(),
            destination_step_id: "arm".to_owned(),
            allow_cycle: false,
        });

        let error = definition.validate().unwrap_err();

        assert_eq!(error.code, "undeclared_sequence_cycle");
    }

    pub(crate) fn fixture_definition() -> TestTemplateDefinition {
        TestTemplateDefinition {
            definition_schema_version: TEST_TEMPLATE_DEFINITION_SCHEMA_VERSION.to_owned(),
            title: "Inrush current capture".to_owned(),
            description: "Time-domain inrush capture for EMC investigations.".to_owned(),
            measurement_axis: MeasurementAxis::TimeSeries,
            method_code: Some("TD-INRUSH".to_owned()),
            method_revision: Some("A".to_owned()),
            standard_references: vec!["IEC-61000-4-30".to_owned()],
            variables: vec![VariableDefinition {
                variable_id: "sample_rate_hz".to_owned(),
                label: "Sample rate".to_owned(),
                value_type: VariableValueType::Number,
                default_value: Some(VariableDefaultValue::Number(100_000.0)),
                constraints: VariableConstraints {
                    required: true,
                    unit: Some("Hz".to_owned()),
                    minimum: Some(1_000.0),
                    maximum: Some(1_000_000.0),
                    enum_values: Vec::new(),
                },
                description: Some("DAQ sample rate".to_owned()),
            }],
            lock_policy: vec![VariableLockPolicy {
                variable_id: "sample_rate_hz".to_owned(),
                policy: VariableLockPolicyKind::EditableUntilCampaignFreeze,
            }],
            instrumentation_chain: vec![
                InstrumentationChainSlot {
                    slot_id: "current_probe".to_owned(),
                    label: "Current probe".to_owned(),
                    required_category: Some("current_probe".to_owned()),
                    required_capability: None,
                    required: true,
                    calibration_requirement: CalibrationRequirement::Required,
                    substitution_policy: InstrumentSubstitutionPolicy::ApprovedEquivalent,
                    depends_on_slots: Vec::new(),
                },
                InstrumentationChainSlot {
                    slot_id: "daq".to_owned(),
                    label: "DAQ".to_owned(),
                    required_category: Some("daq_chassis".to_owned()),
                    required_capability: Some("time_series_capture".to_owned()),
                    required: true,
                    calibration_requirement: CalibrationRequirement::IfUsed,
                    substitution_policy: InstrumentSubstitutionPolicy::SameCapability,
                    depends_on_slots: vec!["current_probe".to_owned()],
                },
            ],
            entry_step_id: "arm".to_owned(),
            sequence: vec![
                ExecutionSequenceStep {
                    step_id: "arm".to_owned(),
                    order: 10,
                    kind: ExecutionStepKind::ConfigureInstrument,
                    label: "Arm acquisition".to_owned(),
                    instruction: Some("Arm acquisition and wait for trigger.".to_owned()),
                    required_slots: vec!["daq".to_owned()],
                    branches: vec![BranchRule {
                        rule_id: "manual_abort".to_owned(),
                        condition: "operator_abort".to_owned(),
                        destination_step_id: "finish".to_owned(),
                        allow_cycle: false,
                    }],
                },
                ExecutionSequenceStep {
                    step_id: "capture".to_owned(),
                    order: 20,
                    kind: ExecutionStepKind::Acquire,
                    label: "Capture transient".to_owned(),
                    instruction: Some("Capture the inrush event.".to_owned()),
                    required_slots: vec!["current_probe".to_owned(), "daq".to_owned()],
                    branches: Vec::new(),
                },
                ExecutionSequenceStep {
                    step_id: "finish".to_owned(),
                    order: 30,
                    kind: ExecutionStepKind::Finish,
                    label: "Finish".to_owned(),
                    instruction: None,
                    required_slots: Vec::new(),
                    branches: Vec::new(),
                },
            ],
            limits: vec![LimitDefinition {
                limit_id: "peak_current".to_owned(),
                kind: LimitKind::ScalarThreshold,
                axis: MeasurementAxis::TimeSeries,
                unit: "A".to_owned(),
                application_domain: "inrush".to_owned(),
                source_reference: "method:TD-INRUSH:A".to_owned(),
                threshold: Some(30.0),
                attention_rule: Some("warn_above_80_percent".to_owned()),
                variable_refs: vec!["sample_rate_hz".to_owned()],
            }],
            post_processing: vec![PostProcessingDefinition {
                operation_id: "peak".to_owned(),
                order: 10,
                operation_type: PostProcessingOperationType::Peak,
                inputs: vec!["raw.current".to_owned()],
                outputs: vec!["calculated.peak_current".to_owned()],
                parameters: BTreeMap::from([("absolute".to_owned(), serde_json::json!(true))]),
            }],
            method_parameters: BTreeMap::new(),
        }
    }
}
