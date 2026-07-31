use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const LEGACY_STATION_SETUP_DEFINITION_SCHEMA_VERSION: &str =
    "emc-locus.station-measurement-setup-definition.v1";
pub const STATION_SETUP_V2_DEFINITION_SCHEMA_VERSION: &str =
    "emc-locus.station-measurement-setup-definition.v2";
pub const STATION_SETUP_DEFINITION_SCHEMA_VERSION: &str =
    "emc-locus.station-measurement-setup-definition.v3";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationSetupRevisionStatus {
    Draft,
    Qualified,
    Ready,
    Superseded,
}

impl StationSetupRevisionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Qualified => "qualified",
            Self::Ready => "ready",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationMaterialSelectionPolicy {
    CategoryPool,
    CapabilityMatch,
    ExactAsset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationMaterialAssignmentStage {
    SetupDefinition,
    PlannedTestPreparation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationMaterialSubstitutionPolicy {
    NoSubstitution,
    SameExactModel,
    SameCategory,
    SameCapabilities,
    ApprovedEquivalent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationCalibrationRequirement {
    Required,
    IfUsed,
    NotRequired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationCategoryRequirementDefinition {
    pub category_id: String,
    #[serde(default)]
    pub accept_descendants: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StationRangeConstraint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StationCapabilityRequirementDefinition {
    pub capability_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_capability_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_capability_equipment_model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_range: Option<StationRangeConstraint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voltage_range: Option<StationRangeConstraint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_range: Option<StationRangeConstraint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_range: Option<StationRangeConstraint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub detector_modes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal_domain: Option<crate::SignalDomain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_directionality: Option<crate::PortDirectionality>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector_requirement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impedance_ohm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub communication_capability: Option<String>,
    #[serde(default)]
    pub automated_control_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_driver_action: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StationLogicalPortRequirementDefinition {
    pub logical_port_id: String,
    pub label: String,
    pub directionality: crate::PortDirectionality,
    pub signal_domain: crate::SignalDomain,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector_requirement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impedance_ohm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_range: Option<StationRangeConstraint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voltage_range: Option<StationRangeConstraint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_range: Option<StationRangeConstraint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_range: Option<StationRangeConstraint>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StationMaterialRequirementDefinition {
    pub requirement_id: String,
    pub role_label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub required: bool,
    pub selection_policy: StationMaterialSelectionPolicy,
    pub assignment_stage: StationMaterialAssignmentStage,
    pub substitution_policy: StationMaterialSubstitutionPolicy,
    pub calibration_requirement: StationCalibrationRequirement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_requirement: Option<StationCategoryRequirementDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_requirement: Option<StationCapabilityRequirementDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exact_asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logical_ports: Vec<StationLogicalPortRequirementDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationPhysicalPortMappingDefinition {
    pub logical_port_id: String,
    pub actual_port_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationMaterialAssignmentDefinition {
    pub requirement_id: String,
    pub asset_id: String,
    pub asset_revision: String,
    pub inventory_code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial_number: Option<String>,
    pub equipment_model_id: String,
    pub equipment_model_revision_id: String,
    pub equipment_model_checksum: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_ports: Vec<StationPhysicalPortMappingDefinition>,
    pub assignment_context: String,
    pub assigned_on: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StationLogicalPortEndpoint {
    pub requirement_id: String,
    pub logical_port_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationLogicalConnectionDefinition {
    pub connection_id: String,
    pub label: String,
    pub from: StationLogicalPortEndpoint,
    pub to: StationLogicalPortEndpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationCompatibilityState {
    Compatible,
    Incompatible,
    Indeterminate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationCompatibilityReason {
    pub code: String,
    pub dimension: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_action: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationRequirementCompatibility {
    pub state: StationCompatibilityState,
    pub requirement_compatible: bool,
    pub reasons: Vec<StationCompatibilityReason>,
    #[serde(default)]
    pub logical_port_candidates: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationCorrectionKind {
    TimeConversion,
    FrequencyResponse,
}

impl StationCorrectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TimeConversion => "time_conversion",
            Self::FrequencyResponse => "frequency_response",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationReadinessSeverity {
    Blocking,
    Warning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationReadinessDimension {
    Structure,
    AssetIdentity,
    Serviceability,
    CalibrationValidity,
    MissingEvidence,
    Nonconformance,
    PortCompatibility,
    CorrectionValidity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationReadinessIssue {
    pub code: String,
    pub severity: StationReadinessSeverity,
    pub dimension: StationReadinessDimension,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binding_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connection_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationSetupReadiness {
    pub ready: bool,
    pub checked_on: String,
    pub issues: Vec<StationReadinessIssue>,
}

impl StationSetupReadiness {
    pub fn from_issues(checked_on: impl Into<String>, issues: Vec<StationReadinessIssue>) -> Self {
        let ready = !issues
            .iter()
            .any(|issue| issue.severity == StationReadinessSeverity::Blocking);
        Self {
            ready,
            checked_on: checked_on.into(),
            issues,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationAssetBindingDefinition {
    pub binding_id: String,
    pub role_label: String,
    pub asset_id: String,
    pub asset_revision: String,
    pub equipment_model_id: String,
    pub equipment_model_revision_id: String,
    pub equipment_model_checksum: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StationPortEndpoint {
    pub binding_id: String,
    pub port_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationConnectionDefinition {
    pub connection_id: String,
    pub label: String,
    pub from: StationPortEndpoint,
    pub to: StationPortEndpoint,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationCorrectionSelectionDefinition {
    pub selection_id: String,
    pub binding_id: String,
    pub correction_kind: StationCorrectionKind,
    pub characterization_id: String,
    pub characterization_checksum: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StationMeasurementSetupDefinition {
    pub definition_schema_version: String,
    pub setup_id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laboratory_location_id: Option<String>,
    #[serde(alias = "station_label")]
    pub laboratory_location_label: String,
    pub planned_use_on: String,
    pub execution_mode: String,
    #[serde(default)]
    pub asset_bindings: Vec<StationAssetBindingDefinition>,
    #[serde(default)]
    pub connections: Vec<StationConnectionDefinition>,
    #[serde(default)]
    pub correction_selections: Vec<StationCorrectionSelectionDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub material_requirements: Vec<StationMaterialRequirementDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub material_assignments: Vec<StationMaterialAssignmentDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logical_connections: Vec<StationLogicalConnectionDefinition>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub notes: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalStationMeasurementSetupDefinition {
    pub setup_id: String,
    pub label: String,
    pub definition_schema_version: String,
    pub canonical_json: String,
    pub definition_checksum: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationSetupValidationIssue {
    pub code: String,
    pub path: String,
    pub message: String,
}

impl StationMeasurementSetupDefinition {
    pub fn from_json_str(value: &str) -> Result<Self, StationSetupValidationIssue> {
        serde_json::from_str(value).map_err(|error| StationSetupValidationIssue {
            code: "invalid_station_setup_json".to_owned(),
            path: "$".to_owned(),
            message: format!("invalid station measurement setup definition: {error}"),
        })
    }

    pub fn validate_integrity(&self) -> Vec<StationSetupValidationIssue> {
        validate_station_setup_integrity(self)
    }

    pub fn structural_readiness_issues(&self) -> Vec<StationReadinessIssue> {
        station_setup_structural_readiness_issues(self)
    }

    pub fn canonicalize(
        &self,
    ) -> Result<CanonicalStationMeasurementSetupDefinition, Vec<StationSetupValidationIssue>> {
        let issues = self.validate_integrity();
        if !issues.is_empty() {
            return Err(issues);
        }

        let mut normalized = self.clone();
        normalized.label = normalized.label.trim().to_owned();
        normalized.laboratory_location_label =
            normalized.laboratory_location_label.trim().to_owned();
        for binding in &mut normalized.asset_bindings {
            binding.role_label = binding.role_label.trim().to_owned();
        }
        for connection in &mut normalized.connections {
            connection.label = connection.label.trim().to_owned();
        }
        for selection in &mut normalized.correction_selections {
            selection.label = selection.label.trim().to_owned();
        }
        for requirement in &mut normalized.material_requirements {
            requirement.role_label = requirement.role_label.trim().to_owned();
            requirement.description = requirement.description.trim().to_owned();
            requirement
                .logical_ports
                .sort_by(|left, right| left.logical_port_id.cmp(&right.logical_port_id));
        }
        for assignment in &mut normalized.material_assignments {
            assignment.inventory_code = assignment.inventory_code.trim().to_owned();
            assignment.assignment_context = assignment.assignment_context.trim().to_owned();
            assignment
                .selected_ports
                .sort_by(|left, right| left.logical_port_id.cmp(&right.logical_port_id));
        }
        for connection in &mut normalized.logical_connections {
            connection.label = connection.label.trim().to_owned();
        }
        normalized
            .asset_bindings
            .sort_by(|left, right| left.binding_id.cmp(&right.binding_id));
        normalized
            .connections
            .sort_by(|left, right| left.connection_id.cmp(&right.connection_id));
        normalized
            .correction_selections
            .sort_by(|left, right| left.selection_id.cmp(&right.selection_id));
        normalized
            .material_requirements
            .sort_by(|left, right| left.requirement_id.cmp(&right.requirement_id));
        normalized
            .material_assignments
            .sort_by(|left, right| left.requirement_id.cmp(&right.requirement_id));
        normalized
            .logical_connections
            .sort_by(|left, right| left.connection_id.cmp(&right.connection_id));

        let mut value = serde_json::to_value(&normalized).map_err(|error| {
            vec![StationSetupValidationIssue {
                code: "station_setup_serialization_failed".to_owned(),
                path: "$".to_owned(),
                message: error.to_string(),
            }]
        })?;
        if normalized.definition_schema_version == LEGACY_STATION_SETUP_DEFINITION_SCHEMA_VERSION {
            let object = value
                .as_object_mut()
                .expect("station definition is an object");
            object.remove("laboratory_location_id");
            if let Some(label) = object.remove("laboratory_location_label") {
                object.insert("station_label".to_owned(), label);
            }
        }
        canonicalize_json_value(&mut value);
        let canonical_json = serde_json::to_string(&value).map_err(|error| {
            vec![StationSetupValidationIssue {
                code: "station_setup_serialization_failed".to_owned(),
                path: "$".to_owned(),
                message: error.to_string(),
            }]
        })?;
        let digest = Sha256::digest(canonical_json.as_bytes());
        Ok(CanonicalStationMeasurementSetupDefinition {
            setup_id: normalized.setup_id,
            label: normalized.label,
            definition_schema_version: normalized.definition_schema_version,
            canonical_json,
            definition_checksum: format!("sha256:{digest:x}"),
        })
    }
}

pub fn validate_station_setup_integrity(
    definition: &StationMeasurementSetupDefinition,
) -> Vec<StationSetupValidationIssue> {
    let mut issues = Vec::new();
    if !matches!(
        definition.definition_schema_version.as_str(),
        STATION_SETUP_DEFINITION_SCHEMA_VERSION
            | STATION_SETUP_V2_DEFINITION_SCHEMA_VERSION
            | LEGACY_STATION_SETUP_DEFINITION_SCHEMA_VERSION
    ) {
        push_validation(
            &mut issues,
            "unsupported_station_setup_schema",
            "definition_schema_version",
            "unsupported station measurement setup schema",
        );
    }
    require_id(&mut issues, &definition.setup_id, "setup_id");
    require_text(
        &mut issues,
        &definition.label,
        "label",
        "a setup name is required",
    );
    if matches!(
        definition.definition_schema_version.as_str(),
        STATION_SETUP_DEFINITION_SCHEMA_VERSION | STATION_SETUP_V2_DEFINITION_SCHEMA_VERSION
    ) {
        match definition.laboratory_location_id.as_deref() {
            Some(location_id) => require_id(&mut issues, location_id, "laboratory_location_id"),
            None => push_validation(
                &mut issues,
                "station_setup_location_identity_required",
                "laboratory_location_id",
                "a stable laboratory location identifier is required",
            ),
        }
    } else if definition.laboratory_location_id.is_some() {
        push_validation(
            &mut issues,
            "legacy_station_setup_location_identity_forbidden",
            "laboratory_location_id",
            "legacy station definitions cannot carry the v2 location identity",
        );
    }

    if definition.definition_schema_version == STATION_SETUP_DEFINITION_SCHEMA_VERSION {
        if !definition.asset_bindings.is_empty()
            || !definition.connections.is_empty()
            || !definition.correction_selections.is_empty()
        {
            push_validation(
                &mut issues,
                "station_v3_legacy_material_contract_forbidden",
                "$",
                "station v3 uses material requirements, assignments, and logical connections",
            );
        }
        validate_material_requirements(definition, &mut issues);
    } else if !definition.material_requirements.is_empty()
        || !definition.material_assignments.is_empty()
        || !definition.logical_connections.is_empty()
    {
        push_validation(
            &mut issues,
            "legacy_station_v3_contract_forbidden",
            "$",
            "station v1 and v2 definitions cannot carry v3 material requirements",
        );
    }
    require_text(
        &mut issues,
        &definition.laboratory_location_label,
        "laboratory_location_label",
        "a laboratory location label is required",
    );
    if !valid_date(&definition.planned_use_on) {
        push_validation(
            &mut issues,
            "invalid_station_setup_date",
            "planned_use_on",
            "planned use date must be a valid YYYY-MM-DD date",
        );
    }
    if !matches!(
        definition.execution_mode.as_str(),
        "accredited" | "non_accredited" | "investigation"
    ) {
        push_validation(
            &mut issues,
            "invalid_station_setup_execution_mode",
            "execution_mode",
            "execution mode must be accredited, non_accredited, or investigation",
        );
    }

    let mut binding_ids = BTreeSet::new();
    let mut asset_ids = BTreeSet::new();
    for (index, binding) in definition.asset_bindings.iter().enumerate() {
        let path = format!("asset_bindings[{index}]");
        require_id(
            &mut issues,
            &binding.binding_id,
            &format!("{path}.binding_id"),
        );
        require_text(
            &mut issues,
            &binding.role_label,
            &format!("{path}.role_label"),
            "a laboratory role is required for each material",
        );
        require_id(&mut issues, &binding.asset_id, &format!("{path}.asset_id"));
        require_id(
            &mut issues,
            &binding.asset_revision,
            &format!("{path}.asset_revision"),
        );
        require_id(
            &mut issues,
            &binding.equipment_model_id,
            &format!("{path}.equipment_model_id"),
        );
        require_id(
            &mut issues,
            &binding.equipment_model_revision_id,
            &format!("{path}.equipment_model_revision_id"),
        );
        require_checksum(
            &mut issues,
            &binding.equipment_model_checksum,
            &format!("{path}.equipment_model_checksum"),
        );
        if !binding_ids.insert(binding.binding_id.clone()) {
            push_validation(
                &mut issues,
                "duplicate_station_binding_id",
                &format!("{path}.binding_id"),
                "material binding identifiers must be unique",
            );
        }
        if !asset_ids.insert(binding.asset_id.clone()) {
            push_validation(
                &mut issues,
                "duplicate_station_asset",
                &format!("{path}.asset_id"),
                "the same physical material cannot be added twice",
            );
        }
    }

    let mut connection_ids = BTreeSet::new();
    let mut connection_pairs = BTreeSet::new();
    let mut occupied_inputs = BTreeSet::new();
    for (index, connection) in definition.connections.iter().enumerate() {
        let path = format!("connections[{index}]");
        require_id(
            &mut issues,
            &connection.connection_id,
            &format!("{path}.connection_id"),
        );
        require_text(
            &mut issues,
            &connection.label,
            &format!("{path}.label"),
            "a connection name is required",
        );
        validate_endpoint(&mut issues, &connection.from, &format!("{path}.from"));
        validate_endpoint(&mut issues, &connection.to, &format!("{path}.to"));
        if !connection_ids.insert(connection.connection_id.clone()) {
            push_validation(
                &mut issues,
                "duplicate_station_connection_id",
                &format!("{path}.connection_id"),
                "connection identifiers must be unique",
            );
        }
        if !binding_ids.contains(&connection.from.binding_id) {
            push_validation(
                &mut issues,
                "station_connection_unknown_source_material",
                &format!("{path}.from.binding_id"),
                "the source material is not part of this setup",
            );
        }
        if !binding_ids.contains(&connection.to.binding_id) {
            push_validation(
                &mut issues,
                "station_connection_unknown_destination_material",
                &format!("{path}.to.binding_id"),
                "the destination material is not part of this setup",
            );
        }
        if connection.from.binding_id == connection.to.binding_id {
            push_validation(
                &mut issues,
                "station_connection_same_material",
                &path,
                "a physical connection must link two different materials",
            );
        }
        let pair = (connection.from.clone(), connection.to.clone());
        if !connection_pairs.insert(pair) {
            push_validation(
                &mut issues,
                "duplicate_station_connection",
                &path,
                "the same two ports are already connected",
            );
        }
        if !occupied_inputs.insert(connection.to.clone()) {
            push_validation(
                &mut issues,
                "station_input_connected_twice",
                &format!("{path}.to"),
                "a destination port cannot receive two physical connections",
            );
        }
    }

    let mut selection_ids = BTreeSet::new();
    let mut binding_kinds = BTreeSet::new();
    for (index, selection) in definition.correction_selections.iter().enumerate() {
        let path = format!("correction_selections[{index}]");
        require_id(
            &mut issues,
            &selection.selection_id,
            &format!("{path}.selection_id"),
        );
        require_id(
            &mut issues,
            &selection.binding_id,
            &format!("{path}.binding_id"),
        );
        require_id(
            &mut issues,
            &selection.characterization_id,
            &format!("{path}.characterization_id"),
        );
        require_checksum(
            &mut issues,
            &selection.characterization_checksum,
            &format!("{path}.characterization_checksum"),
        );
        require_text(
            &mut issues,
            &selection.label,
            &format!("{path}.label"),
            "a correction label is required",
        );
        if !selection_ids.insert(selection.selection_id.clone()) {
            push_validation(
                &mut issues,
                "duplicate_station_correction_selection_id",
                &format!("{path}.selection_id"),
                "correction selection identifiers must be unique",
            );
        }
        if !binding_ids.contains(&selection.binding_id) {
            push_validation(
                &mut issues,
                "station_correction_unknown_material",
                &format!("{path}.binding_id"),
                "the corrected material is not part of this setup",
            );
        }
        if !binding_kinds.insert((selection.binding_id.clone(), selection.correction_kind)) {
            push_validation(
                &mut issues,
                "duplicate_station_correction_kind",
                &path,
                "only one correction of each kind can be selected for a material",
            );
        }
    }
    issues
}

fn validate_material_requirements(
    definition: &StationMeasurementSetupDefinition,
    issues: &mut Vec<StationSetupValidationIssue>,
) {
    let mut requirement_ids = BTreeSet::new();
    let mut requirement_ports: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (index, requirement) in definition.material_requirements.iter().enumerate() {
        let path = format!("material_requirements[{index}]");
        require_id(
            issues,
            &requirement.requirement_id,
            &format!("{path}.requirement_id"),
        );
        require_text(
            issues,
            &requirement.role_label,
            &format!("{path}.role_label"),
            "a material role name is required",
        );
        if !requirement_ids.insert(requirement.requirement_id.as_str()) {
            push_validation(
                issues,
                "duplicate_station_material_requirement_id",
                &format!("{path}.requirement_id"),
                "material requirement identifiers must be unique",
            );
        }
        match requirement.selection_policy {
            StationMaterialSelectionPolicy::CategoryPool => {
                if requirement.category_requirement.is_none()
                    || requirement.capability_requirement.is_some()
                    || requirement.exact_asset_id.is_some()
                {
                    push_validation(
                        issues,
                        "invalid_station_category_requirement",
                        &path,
                        "a category-pool role requires only a category requirement",
                    );
                }
            }
            StationMaterialSelectionPolicy::CapabilityMatch => {
                if requirement.capability_requirement.is_none()
                    || requirement.exact_asset_id.is_some()
                {
                    push_validation(
                        issues,
                        "invalid_station_capability_requirement",
                        &path,
                        "a capability role requires a capability contract and no exact asset",
                    );
                }
            }
            StationMaterialSelectionPolicy::ExactAsset => {
                if requirement.exact_asset_id.is_none()
                    || requirement.substitution_policy
                        != StationMaterialSubstitutionPolicy::NoSubstitution
                {
                    push_validation(
                        issues,
                        "invalid_station_exact_asset_requirement",
                        &path,
                        "an exact-asset role requires an asset and forbids substitution",
                    );
                }
            }
        }
        if let Some(category) = &requirement.category_requirement {
            require_id(
                issues,
                &category.category_id,
                &format!("{path}.category_requirement.category_id"),
            );
        }
        if let Some(capability) = &requirement.capability_requirement {
            require_id(
                issues,
                &capability.capability_kind,
                &format!("{path}.capability_requirement.capability_kind"),
            );
            if let Some(capability_id) = &capability.model_capability_id {
                require_id(
                    issues,
                    capability_id,
                    &format!("{path}.capability_requirement.model_capability_id"),
                );
            }
            if capability.model_capability_id.is_some()
                != capability.model_capability_equipment_model_id.is_some()
            {
                push_validation(
                    issues,
                    "unscoped_model_capability_id",
                    &format!("{path}.capability_requirement.model_capability_id"),
                    "a model-local capability ID must name its originating equipment model",
                );
            }
            if let Some(model_id) = &capability.model_capability_equipment_model_id {
                require_id(
                    issues,
                    model_id,
                    &format!("{path}.capability_requirement.model_capability_equipment_model_id"),
                );
            }
            validate_range(
                issues,
                capability.frequency_range.as_ref(),
                &format!("{path}.capability_requirement.frequency_range"),
            );
            validate_range(
                issues,
                capability.voltage_range.as_ref(),
                &format!("{path}.capability_requirement.voltage_range"),
            );
            validate_range(
                issues,
                capability.current_range.as_ref(),
                &format!("{path}.capability_requirement.current_range"),
            );
            validate_range(
                issues,
                capability.power_range.as_ref(),
                &format!("{path}.capability_requirement.power_range"),
            );
            if capability
                .impedance_ohm
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
            {
                push_validation(
                    issues,
                    "invalid_station_impedance_requirement",
                    &format!("{path}.capability_requirement.impedance_ohm"),
                    "impedance must be a finite positive value",
                );
            }
        }
        let mut logical_port_ids = BTreeSet::new();
        for (port_index, port) in requirement.logical_ports.iter().enumerate() {
            let port_path = format!("{path}.logical_ports[{port_index}]");
            require_id(
                issues,
                &port.logical_port_id,
                &format!("{port_path}.logical_port_id"),
            );
            require_text(
                issues,
                &port.label,
                &format!("{port_path}.label"),
                "a logical port name is required",
            );
            if !logical_port_ids.insert(port.logical_port_id.as_str()) {
                push_validation(
                    issues,
                    "duplicate_station_logical_port_id",
                    &format!("{port_path}.logical_port_id"),
                    "logical port identifiers must be unique within a material role",
                );
            }
            validate_range(
                issues,
                port.frequency_range.as_ref(),
                &format!("{port_path}.frequency_range"),
            );
            validate_range(
                issues,
                port.voltage_range.as_ref(),
                &format!("{port_path}.voltage_range"),
            );
            validate_range(
                issues,
                port.current_range.as_ref(),
                &format!("{port_path}.current_range"),
            );
            validate_range(
                issues,
                port.power_range.as_ref(),
                &format!("{port_path}.power_range"),
            );
        }
        requirement_ports.insert(requirement.requirement_id.as_str(), logical_port_ids);
    }

    let mut assigned_requirements = BTreeSet::new();
    for (index, assignment) in definition.material_assignments.iter().enumerate() {
        let path = format!("material_assignments[{index}]");
        require_id(
            issues,
            &assignment.requirement_id,
            &format!("{path}.requirement_id"),
        );
        require_id(issues, &assignment.asset_id, &format!("{path}.asset_id"));
        require_id(
            issues,
            &assignment.asset_revision,
            &format!("{path}.asset_revision"),
        );
        require_text(
            issues,
            &assignment.inventory_code,
            &format!("{path}.inventory_code"),
            "an inventory code snapshot is required",
        );
        require_id(
            issues,
            &assignment.equipment_model_id,
            &format!("{path}.equipment_model_id"),
        );
        require_id(
            issues,
            &assignment.equipment_model_revision_id,
            &format!("{path}.equipment_model_revision_id"),
        );
        require_checksum(
            issues,
            &assignment.equipment_model_checksum,
            &format!("{path}.equipment_model_checksum"),
        );
        require_text(
            issues,
            &assignment.assignment_context,
            &format!("{path}.assignment_context"),
            "an assignment context is required",
        );
        if !valid_date(&assignment.assigned_on) {
            push_validation(
                issues,
                "invalid_station_assignment_date",
                &format!("{path}.assigned_on"),
                "assigned_on must be a valid YYYY-MM-DD date",
            );
        }
        if !requirement_ids.contains(assignment.requirement_id.as_str()) {
            push_validation(
                issues,
                "station_assignment_unknown_requirement",
                &format!("{path}.requirement_id"),
                "the material assignment references an unknown requirement",
            );
        }
        if !assigned_requirements.insert(assignment.requirement_id.as_str()) {
            push_validation(
                issues,
                "duplicate_station_material_assignment",
                &format!("{path}.requirement_id"),
                "a material requirement can have only one physical assignment",
            );
        }
        let mut mapped_logical_ports = BTreeSet::new();
        let mut actual_ports = BTreeSet::new();
        for (mapping_index, mapping) in assignment.selected_ports.iter().enumerate() {
            let mapping_path = format!("{path}.selected_ports[{mapping_index}]");
            require_id(
                issues,
                &mapping.logical_port_id,
                &format!("{mapping_path}.logical_port_id"),
            );
            require_id(
                issues,
                &mapping.actual_port_id,
                &format!("{mapping_path}.actual_port_id"),
            );
            if !mapped_logical_ports.insert(mapping.logical_port_id.as_str()) {
                push_validation(
                    issues,
                    "duplicate_station_logical_port_mapping",
                    &mapping_path,
                    "a logical port can be mapped only once",
                );
            }
            if !actual_ports.insert(mapping.actual_port_id.as_str()) {
                push_validation(
                    issues,
                    "duplicate_station_actual_port_mapping",
                    &mapping_path,
                    "an actual port cannot satisfy two logical ports in one assignment",
                );
            }
            if !requirement_ports
                .get(assignment.requirement_id.as_str())
                .is_some_and(|ports| ports.contains(mapping.logical_port_id.as_str()))
            {
                push_validation(
                    issues,
                    "station_assignment_unknown_logical_port",
                    &format!("{mapping_path}.logical_port_id"),
                    "the assignment maps a logical port not declared by the requirement",
                );
            }
        }
    }

    let mut connection_ids = BTreeSet::new();
    for (index, connection) in definition.logical_connections.iter().enumerate() {
        let path = format!("logical_connections[{index}]");
        require_id(
            issues,
            &connection.connection_id,
            &format!("{path}.connection_id"),
        );
        require_text(
            issues,
            &connection.label,
            &format!("{path}.label"),
            "a logical connection name is required",
        );
        if !connection_ids.insert(connection.connection_id.as_str()) {
            push_validation(
                issues,
                "duplicate_station_logical_connection_id",
                &format!("{path}.connection_id"),
                "logical connection identifiers must be unique",
            );
        }
        validate_logical_endpoint(
            issues,
            &connection.from,
            &format!("{path}.from"),
            &requirement_ports,
        );
        validate_logical_endpoint(
            issues,
            &connection.to,
            &format!("{path}.to"),
            &requirement_ports,
        );
        if connection.from == connection.to {
            push_validation(
                issues,
                "station_logical_connection_same_port",
                &path,
                "a logical connection must link two different ports",
            );
        }
    }
}

fn validate_logical_endpoint(
    issues: &mut Vec<StationSetupValidationIssue>,
    endpoint: &StationLogicalPortEndpoint,
    path: &str,
    requirement_ports: &BTreeMap<&str, BTreeSet<&str>>,
) {
    require_id(
        issues,
        &endpoint.requirement_id,
        &format!("{path}.requirement_id"),
    );
    require_id(
        issues,
        &endpoint.logical_port_id,
        &format!("{path}.logical_port_id"),
    );
    match requirement_ports.get(endpoint.requirement_id.as_str()) {
        None => push_validation(
            issues,
            "station_logical_connection_unknown_requirement",
            &format!("{path}.requirement_id"),
            "the logical connection references an unknown requirement",
        ),
        Some(ports) if !ports.contains(endpoint.logical_port_id.as_str()) => push_validation(
            issues,
            "station_logical_connection_unknown_port",
            &format!("{path}.logical_port_id"),
            "the logical connection references an unknown logical port",
        ),
        Some(_) => {}
    }
}

fn validate_range(
    issues: &mut Vec<StationSetupValidationIssue>,
    range: Option<&StationRangeConstraint>,
    path: &str,
) {
    let Some(range) = range else { return };
    if range.unit.trim().is_empty()
        || range.minimum.is_some_and(|value| !value.is_finite())
        || range.maximum.is_some_and(|value| !value.is_finite())
        || range
            .minimum
            .zip(range.maximum)
            .is_some_and(|(minimum, maximum)| minimum > maximum)
        || (range.minimum.is_none() && range.maximum.is_none())
    {
        push_validation(
            issues,
            "invalid_station_range_constraint",
            path,
            "a range needs a unit and coherent finite bounds",
        );
    }
}

pub fn evaluate_station_material_requirement(
    requirement: &StationMaterialRequirementDefinition,
    candidate_asset_id: &str,
    candidate_model_id: &str,
    candidate_category_path: &[String],
    model: &crate::EquipmentModelDefinition,
    driver_action_capabilities: &BTreeSet<String>,
) -> StationRequirementCompatibility {
    let mut reasons = Vec::new();
    let mut indeterminate = false;
    match requirement.selection_policy {
        StationMaterialSelectionPolicy::ExactAsset => {
            if requirement.exact_asset_id.as_deref() != Some(candidate_asset_id) {
                incompatible_reason(
                    &mut reasons,
                    "exact_asset_mismatch",
                    "model_pin",
                    "L'exemplaire ne correspond pas à l'exemplaire imposé.",
                );
            }
        }
        StationMaterialSelectionPolicy::CategoryPool => {
            if let Some(category) = &requirement.category_requirement {
                let category_matches = candidate_category_path
                    .iter()
                    .any(|candidate| candidate == &category.category_id);
                let exact_category = candidate_category_path
                    .last()
                    .is_some_and(|candidate| candidate == &category.category_id);
                if !(exact_category || (category.accept_descendants && category_matches)) {
                    incompatible_reason(
                        &mut reasons,
                        "category_mismatch",
                        "category",
                        "La catégorie de l'exemplaire ne satisfait pas le rôle matériel.",
                    );
                }
            }
        }
        StationMaterialSelectionPolicy::CapabilityMatch => {
            if let Some(expected) = &requirement.capability_requirement {
                let capabilities: Vec<_> = model
                    .capabilities
                    .iter()
                    .filter(|candidate| candidate.capability_kind == expected.capability_kind)
                    .collect();
                if capabilities.is_empty() {
                    incompatible_reason(
                        &mut reasons,
                        "capability_kind_missing",
                        "capability",
                        "Le modèle ne déclare pas l'aptitude technique requise.",
                    );
                } else if expected.model_capability_equipment_model_id.as_deref()
                    == Some(candidate_model_id)
                    && expected
                        .model_capability_id
                        .as_ref()
                        .is_some_and(|expected_id| {
                            capabilities
                                .iter()
                                .all(|candidate| &candidate.capability_id != expected_id)
                        })
                {
                    incompatible_reason(
                        &mut reasons,
                        "model_capability_id_missing",
                        "capability",
                        "L'aptitude locale attendue n'existe pas dans cette version du modèle.",
                    );
                }
                if let Some(domain) = expected.signal_domain {
                    if !model.signal_domains.contains(&domain) {
                        incompatible_reason(
                            &mut reasons,
                            "signal_domain_mismatch",
                            "mode",
                            "Le domaine de signal requis n'est pas déclaré par le modèle.",
                        );
                    }
                }
                for mode in &expected.detector_modes {
                    let present = capabilities.iter().any(|capability| {
                        capability
                            .inputs
                            .iter()
                            .chain(capability.outputs.iter())
                            .flat_map(|value| value.enum_values.iter())
                            .any(|candidate| candidate.eq_ignore_ascii_case(mode))
                    });
                    if !present {
                        incompatible_reason(
                            &mut reasons,
                            "detector_mode_missing",
                            "mode",
                            format!("Le mode ou détecteur « {mode} » n'est pas déclaré."),
                        );
                    }
                }
                evaluate_candidate_range(
                    expected.frequency_range.as_ref(),
                    crate::PhysicalQuantity::Frequency,
                    model,
                    &mut reasons,
                    &mut indeterminate,
                );
                evaluate_candidate_range(
                    expected.voltage_range.as_ref(),
                    crate::PhysicalQuantity::Voltage,
                    model,
                    &mut reasons,
                    &mut indeterminate,
                );
                evaluate_candidate_range(
                    expected.current_range.as_ref(),
                    crate::PhysicalQuantity::Current,
                    model,
                    &mut reasons,
                    &mut indeterminate,
                );
                evaluate_candidate_range(
                    expected.power_range.as_ref(),
                    crate::PhysicalQuantity::Power,
                    model,
                    &mut reasons,
                    &mut indeterminate,
                );
                if expected.automated_control_required
                    && (model.communication_interfaces.is_empty()
                        || driver_action_capabilities.is_empty())
                {
                    incompatible_reason(
                        &mut reasons,
                        "automated_control_missing",
                        "driver",
                        "Le pilotage automatisé requis n'est pas disponible.",
                    );
                }
                if expected
                    .required_driver_action
                    .as_ref()
                    .is_some_and(|action| !driver_action_capabilities.contains(action))
                {
                    incompatible_reason(
                        &mut reasons,
                        "driver_action_missing",
                        "driver",
                        "L'action de driver requise n'est pas disponible.",
                    );
                }
            }
        }
    }

    let mut logical_port_candidates = BTreeMap::new();
    for logical_port in &requirement.logical_ports {
        let candidates: Vec<String> = model
            .signal_ports
            .iter()
            .filter(|port| logical_port_matches(logical_port, port))
            .map(|port| port.port_id.clone())
            .collect();
        if candidates.is_empty() {
            incompatible_reason(
                &mut reasons,
                "logical_port_unresolved",
                "port",
                format!(
                    "Aucun port physique ne satisfait le port logique « {} ».",
                    logical_port.label
                ),
            );
        }
        logical_port_candidates.insert(logical_port.logical_port_id.clone(), candidates);
    }

    let incompatible = reasons.iter().any(|reason| reason.next_action.is_none());
    let state = if incompatible {
        StationCompatibilityState::Incompatible
    } else if indeterminate {
        StationCompatibilityState::Indeterminate
    } else {
        StationCompatibilityState::Compatible
    };
    StationRequirementCompatibility {
        state,
        requirement_compatible: state == StationCompatibilityState::Compatible,
        reasons,
        logical_port_candidates,
    }
}

fn evaluate_candidate_range(
    required: Option<&StationRangeConstraint>,
    quantity: crate::PhysicalQuantity,
    model: &crate::EquipmentModelDefinition,
    reasons: &mut Vec<StationCompatibilityReason>,
    indeterminate: &mut bool,
) {
    let Some(required) = required else { return };
    let Some((required_quantity, factor)) = trusted_unit_scale(&required.unit) else {
        *indeterminate = true;
        reasons.push(StationCompatibilityReason {
            code: "unit_conversion_indeterminate".to_owned(),
            dimension: "range".to_owned(),
            message: format!(
                "L'unité {} ne dispose pas d'une conversion de confiance.",
                required.unit
            ),
            next_action: Some("Harmoniser les unités dans le référentiel du modèle.".to_owned()),
        });
        return;
    };
    if required_quantity != quantity {
        incompatible_reason(
            reasons,
            "range_unit_incompatible",
            "range",
            format!(
                "L'unité {} ne correspond pas à la grandeur requise.",
                required.unit
            ),
        );
        return;
    }
    let expected_min = required.minimum.map(|value| value * factor);
    let expected_max = required.maximum.map(|value| value * factor);
    let mut known_ranges = Vec::new();
    for specification in &model.specifications {
        if specification.quantity != quantity {
            continue;
        }
        let Some((candidate_quantity, candidate_factor)) = trusted_unit_scale(&specification.unit)
        else {
            continue;
        };
        if candidate_quantity == quantity {
            known_ranges.push((
                specification.minimum.map(|value| value * candidate_factor),
                specification.maximum.map(|value| value * candidate_factor),
            ));
        }
    }
    for port in &model.signal_ports {
        let range = match quantity {
            crate::PhysicalQuantity::Frequency => (port.frequency_min, port.frequency_max),
            crate::PhysicalQuantity::Voltage => (None, port.voltage_max),
            crate::PhysicalQuantity::Current => (None, port.current_max),
            crate::PhysicalQuantity::Power => (None, port.power_max),
            _ => (None, None),
        };
        if range.0.is_some() || range.1.is_some() {
            known_ranges.push(range);
        }
    }
    if known_ranges.is_empty() {
        *indeterminate = true;
        reasons.push(StationCompatibilityReason {
            code: "range_evidence_missing".to_owned(),
            dimension: "range".to_owned(),
            message: "La version du modèle ne fournit pas la plage nécessaire.".to_owned(),
            next_action: Some("Compléter les caractéristiques de la version du modèle.".to_owned()),
        });
        return;
    }
    let covered = known_ranges.iter().any(|(minimum, maximum)| {
        expected_min.is_none_or(|expected| minimum.is_some_and(|actual| actual <= expected))
            && expected_max.is_none_or(|expected| maximum.is_some_and(|actual| actual >= expected))
    });
    if !covered {
        incompatible_reason(
            reasons,
            "technical_range_not_covered",
            "range",
            "Les plages déclarées par le modèle ne couvrent pas le besoin.",
        );
    }
}

fn logical_port_matches(
    required: &StationLogicalPortRequirementDefinition,
    actual: &crate::SignalPortDefinition,
) -> bool {
    directionality_matches(required.directionality, actual.directionality)
        && required.signal_domain == actual.signal_domain
        && required
            .connector_requirement
            .as_ref()
            .is_none_or(|connector| {
                actual
                    .connector_type
                    .as_ref()
                    .is_some_and(|actual| actual.eq_ignore_ascii_case(connector))
            })
        && required.impedance_ohm.is_none_or(|impedance| {
            actual
                .impedance
                .is_some_and(|actual| (actual - impedance).abs() <= 0.001)
        })
        && port_range_matches(
            required.frequency_range.as_ref(),
            actual.frequency_min,
            actual.frequency_max,
            crate::PhysicalQuantity::Frequency,
        )
        && port_range_matches(
            required.voltage_range.as_ref(),
            None,
            actual.voltage_max,
            crate::PhysicalQuantity::Voltage,
        )
        && port_range_matches(
            required.current_range.as_ref(),
            None,
            actual.current_max,
            crate::PhysicalQuantity::Current,
        )
        && port_range_matches(
            required.power_range.as_ref(),
            None,
            actual.power_max,
            crate::PhysicalQuantity::Power,
        )
}

fn directionality_matches(
    required: crate::PortDirectionality,
    actual: crate::PortDirectionality,
) -> bool {
    required == actual
        || actual == crate::PortDirectionality::Bidirectional
        || (required == crate::PortDirectionality::Input
            && actual == crate::PortDirectionality::Through)
        || (required == crate::PortDirectionality::Output
            && actual == crate::PortDirectionality::Through)
}

fn port_range_matches(
    required: Option<&StationRangeConstraint>,
    actual_minimum: Option<f64>,
    actual_maximum: Option<f64>,
    quantity: crate::PhysicalQuantity,
) -> bool {
    let Some(required) = required else {
        return true;
    };
    let Some((actual_quantity, factor)) = trusted_unit_scale(&required.unit) else {
        return false;
    };
    actual_quantity == quantity
        && required
            .minimum
            .is_none_or(|expected| actual_minimum.is_some_and(|actual| actual <= expected * factor))
        && required
            .maximum
            .is_none_or(|expected| actual_maximum.is_some_and(|actual| actual >= expected * factor))
}

fn trusted_unit_scale(unit: &str) -> Option<(crate::PhysicalQuantity, f64)> {
    match unit {
        "Hz" => Some((crate::PhysicalQuantity::Frequency, 1.0)),
        "kHz" => Some((crate::PhysicalQuantity::Frequency, 1_000.0)),
        "MHz" => Some((crate::PhysicalQuantity::Frequency, 1_000_000.0)),
        "GHz" => Some((crate::PhysicalQuantity::Frequency, 1_000_000_000.0)),
        "V" => Some((crate::PhysicalQuantity::Voltage, 1.0)),
        "mV" => Some((crate::PhysicalQuantity::Voltage, 0.001)),
        "uV" => Some((crate::PhysicalQuantity::Voltage, 0.000_001)),
        "A" => Some((crate::PhysicalQuantity::Current, 1.0)),
        "mA" => Some((crate::PhysicalQuantity::Current, 0.001)),
        "uA" => Some((crate::PhysicalQuantity::Current, 0.000_001)),
        "W" => Some((crate::PhysicalQuantity::Power, 1.0)),
        "mW" => Some((crate::PhysicalQuantity::Power, 0.001)),
        _ => None,
    }
}

fn incompatible_reason(
    reasons: &mut Vec<StationCompatibilityReason>,
    code: &str,
    dimension: &str,
    message: impl Into<String>,
) {
    reasons.push(StationCompatibilityReason {
        code: code.to_owned(),
        dimension: dimension.to_owned(),
        message: message.into(),
        next_action: None,
    });
}

pub fn station_setup_structural_readiness_issues(
    definition: &StationMeasurementSetupDefinition,
) -> Vec<StationReadinessIssue> {
    if definition.definition_schema_version == STATION_SETUP_DEFINITION_SCHEMA_VERSION {
        return station_v3_structural_readiness_issues(definition);
    }
    let mut issues = Vec::new();
    if definition.asset_bindings.len() < 2 {
        issues.push(blocking_structure(
            "station_setup_requires_two_materials",
            "Ajoutez au moins deux matériels réels au montage.",
            Vec::new(),
            Vec::new(),
        ));
    }
    if definition.connections.is_empty() {
        issues.push(blocking_structure(
            "station_setup_requires_connection",
            "Reliez au moins deux ports pour former un chemin de signal.",
            Vec::new(),
            Vec::new(),
        ));
    }

    let connected: BTreeSet<&str> = definition
        .connections
        .iter()
        .flat_map(|connection| {
            [
                connection.from.binding_id.as_str(),
                connection.to.binding_id.as_str(),
            ]
        })
        .collect();
    for binding in &definition.asset_bindings {
        if !connected.contains(binding.binding_id.as_str()) {
            issues.push(blocking_structure(
                "station_material_not_connected",
                format!(
                    "Le matériel « {} » n'est relié à aucun autre matériel.",
                    binding.role_label
                ),
                vec![binding.binding_id.clone()],
                Vec::new(),
            ));
        }
    }
    if has_binding_cycle(definition) {
        issues.push(blocking_structure(
            "station_setup_cycle_detected",
            "Le chemin de signal forme une boucle. Corrigez les connexions avant de continuer.",
            definition
                .asset_bindings
                .iter()
                .map(|binding| binding.binding_id.clone())
                .collect(),
            definition
                .connections
                .iter()
                .map(|connection| connection.connection_id.clone())
                .collect(),
        ));
    }
    issues
}

fn station_v3_structural_readiness_issues(
    definition: &StationMeasurementSetupDefinition,
) -> Vec<StationReadinessIssue> {
    let mut issues = Vec::new();
    if definition.material_requirements.len() < 2 {
        issues.push(blocking_structure(
            "station_setup_requires_two_material_roles",
            "Ajoutez au moins deux rôles matériels au montage.",
            Vec::new(),
            Vec::new(),
        ));
    }
    if definition.logical_connections.is_empty() {
        issues.push(blocking_structure(
            "station_setup_requires_logical_connection",
            "Reliez les ports logiques pour former un chemin de signal.",
            Vec::new(),
            Vec::new(),
        ));
    }
    let connected_requirements: BTreeSet<&str> = definition
        .logical_connections
        .iter()
        .flat_map(|connection| {
            [
                connection.from.requirement_id.as_str(),
                connection.to.requirement_id.as_str(),
            ]
        })
        .collect();
    for requirement in &definition.material_requirements {
        if requirement.required
            && !connected_requirements.contains(requirement.requirement_id.as_str())
        {
            issues.push(blocking_structure(
                "station_required_material_role_not_connected",
                format!(
                    "Le rôle obligatoire « {} » n'est relié à aucun autre rôle.",
                    requirement.role_label
                ),
                vec![requirement.requirement_id.clone()],
                Vec::new(),
            ));
        }
        let assignment = definition
            .material_assignments
            .iter()
            .find(|assignment| assignment.requirement_id == requirement.requirement_id);
        if assignment.is_none() {
            let (severity, message) = if requirement.required {
                (
                    StationReadinessSeverity::Blocking,
                    format!(
                        "Le rôle obligatoire « {} » n'a pas encore d'exemplaire affecté.",
                        requirement.role_label
                    ),
                )
            } else {
                (
                    StationReadinessSeverity::Warning,
                    format!(
                        "Le rôle optionnel « {} » reste sans exemplaire affecté.",
                        requirement.role_label
                    ),
                )
            };
            issues.push(StationReadinessIssue {
                code: if requirement.required {
                    "station_required_material_unassigned".to_owned()
                } else {
                    "station_optional_material_unassigned".to_owned()
                },
                severity,
                dimension: StationReadinessDimension::AssetIdentity,
                message,
                binding_ids: vec![requirement.requirement_id.clone()],
                connection_ids: Vec::new(),
            });
        }
    }
    issues
}

pub fn station_setup_qualification_issues(
    definition: &StationMeasurementSetupDefinition,
) -> Vec<StationReadinessIssue> {
    if definition.definition_schema_version != STATION_SETUP_DEFINITION_SCHEMA_VERSION {
        return definition.structural_readiness_issues();
    }
    station_v3_structural_readiness_issues(definition)
        .into_iter()
        .filter(|issue| {
            !matches!(
                issue.code.as_str(),
                "station_required_material_unassigned" | "station_optional_material_unassigned"
            )
        })
        .collect()
}

fn has_binding_cycle(definition: &StationMeasurementSetupDefinition) -> bool {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for connection in &definition.connections {
        adjacency
            .entry(connection.from.binding_id.as_str())
            .or_default()
            .push(connection.to.binding_id.as_str());
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    definition.asset_bindings.iter().any(|binding| {
        visit_cycle(
            binding.binding_id.as_str(),
            &adjacency,
            &mut visiting,
            &mut visited,
        )
    })
}

fn visit_cycle<'a>(
    node: &'a str,
    adjacency: &BTreeMap<&'a str, Vec<&'a str>>,
    visiting: &mut BTreeSet<&'a str>,
    visited: &mut BTreeSet<&'a str>,
) -> bool {
    if visited.contains(node) {
        return false;
    }
    if !visiting.insert(node) {
        return true;
    }
    if adjacency.get(node).is_some_and(|next| {
        next.iter()
            .any(|child| visit_cycle(child, adjacency, visiting, visited))
    }) {
        return true;
    }
    visiting.remove(node);
    visited.insert(node);
    false
}

fn validate_endpoint(
    issues: &mut Vec<StationSetupValidationIssue>,
    endpoint: &StationPortEndpoint,
    path: &str,
) {
    require_id(issues, &endpoint.binding_id, &format!("{path}.binding_id"));
    require_id(issues, &endpoint.port_id, &format!("{path}.port_id"));
}

fn require_id(issues: &mut Vec<StationSetupValidationIssue>, value: &str, path: &str) {
    let valid = !value.trim().is_empty()
        && value == value.trim()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        });
    if !valid {
        push_validation(
            issues,
            "invalid_station_setup_identifier",
            path,
            "identifier must use letters, digits, hyphen, underscore, or dot",
        );
    }
}

fn require_text(
    issues: &mut Vec<StationSetupValidationIssue>,
    value: &str,
    path: &str,
    message: &str,
) {
    if value.trim().is_empty() || value != value.trim() {
        push_validation(issues, "invalid_station_setup_text", path, message);
    }
}

fn require_checksum(issues: &mut Vec<StationSetupValidationIssue>, value: &str, path: &str) {
    let valid = value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    });
    if !valid {
        push_validation(
            issues,
            "invalid_station_setup_checksum",
            path,
            "reference checksum must use canonical lowercase sha256 form",
        );
    }
}

fn valid_date(value: &str) -> bool {
    let mut parts = value.split('-');
    let Some(year) = parts.next().and_then(|part| part.parse::<u16>().ok()) else {
        return false;
    };
    let Some(month) = parts.next().and_then(|part| part.parse::<u8>().ok()) else {
        return false;
    };
    let Some(day) = parts.next().and_then(|part| part.parse::<u8>().ok()) else {
        return false;
    };
    if parts.next().is_some() || value.len() != 10 || year < 1900 || !(1..=12).contains(&month) {
        return false;
    }
    let leap = (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max_day).contains(&day)
}

fn blocking_structure(
    code: impl Into<String>,
    message: impl Into<String>,
    binding_ids: Vec<String>,
    connection_ids: Vec<String>,
) -> StationReadinessIssue {
    StationReadinessIssue {
        code: code.into(),
        severity: StationReadinessSeverity::Blocking,
        dimension: StationReadinessDimension::Structure,
        message: message.into(),
        binding_ids,
        connection_ids,
    }
}

fn push_validation(
    issues: &mut Vec<StationSetupValidationIssue>,
    code: &str,
    path: &str,
    message: &str,
) {
    issues.push(StationSetupValidationIssue {
        code: code.to_owned(),
        path: path.to_owned(),
        message: message.to_owned(),
    });
}

fn canonicalize_json_value(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for nested in object.values_mut() {
                canonicalize_json_value(nested);
            }
            let mut ordered = BTreeMap::new();
            for (key, nested) in std::mem::take(object) {
                ordered.insert(key, nested);
            }
            object.extend(ordered);
        }
        Value::Array(values) => {
            for nested in values {
                canonicalize_json_value(nested);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(id: &str, asset: &str) -> StationAssetBindingDefinition {
        StationAssetBindingDefinition {
            binding_id: id.to_owned(),
            role_label: id.to_owned(),
            asset_id: asset.to_owned(),
            asset_revision: format!("rev-{asset}"),
            equipment_model_id: format!("model-{asset}"),
            equipment_model_revision_id: format!("model-{asset}-rev-0001"),
            equipment_model_checksum: format!("sha256:{}", "a".repeat(64)),
        }
    }

    fn definition() -> StationMeasurementSetupDefinition {
        StationMeasurementSetupDefinition {
            definition_schema_version: STATION_SETUP_V2_DEFINITION_SCHEMA_VERSION.to_owned(),
            setup_id: "setup-rf-001".to_owned(),
            label: "Chaîne RF réception".to_owned(),
            laboratory_location_id: Some("LAB-LOCATION-CEM-1".to_owned()),
            laboratory_location_label: "Salle CEM 1".to_owned(),
            planned_use_on: "2026-07-15".to_owned(),
            execution_mode: "accredited".to_owned(),
            asset_bindings: vec![
                binding("antenna", "SA-ANT-001"),
                binding("receiver", "SA-RX-001"),
            ],
            connections: vec![StationConnectionDefinition {
                connection_id: "link-001".to_owned(),
                label: "Antenne vers récepteur".to_owned(),
                from: StationPortEndpoint {
                    binding_id: "antenna".to_owned(),
                    port_id: "RF_OUT".to_owned(),
                },
                to: StationPortEndpoint {
                    binding_id: "receiver".to_owned(),
                    port_id: "RF_IN".to_owned(),
                },
            }],
            correction_selections: Vec::new(),
            material_requirements: Vec::new(),
            material_assignments: Vec::new(),
            logical_connections: Vec::new(),
            notes: BTreeMap::new(),
        }
    }

    fn v3_requirement(
        id: &str,
        role: &str,
        policy: StationMaterialSelectionPolicy,
    ) -> StationMaterialRequirementDefinition {
        StationMaterialRequirementDefinition {
            requirement_id: id.to_owned(),
            role_label: role.to_owned(),
            description: String::new(),
            required: true,
            selection_policy: policy,
            assignment_stage: StationMaterialAssignmentStage::PlannedTestPreparation,
            substitution_policy: if policy == StationMaterialSelectionPolicy::ExactAsset {
                StationMaterialSubstitutionPolicy::NoSubstitution
            } else {
                StationMaterialSubstitutionPolicy::SameCapabilities
            },
            calibration_requirement: StationCalibrationRequirement::Required,
            category_requirement: (policy == StationMaterialSelectionPolicy::CategoryPool).then(
                || StationCategoryRequirementDefinition {
                    category_id: "receivers".to_owned(),
                    accept_descendants: true,
                },
            ),
            capability_requirement: (policy == StationMaterialSelectionPolicy::CapabilityMatch)
                .then(|| StationCapabilityRequirementDefinition {
                    capability_kind: "measure_emission".to_owned(),
                    model_capability_id: None,
                    model_capability_equipment_model_id: None,
                    frequency_range: Some(StationRangeConstraint {
                        minimum: Some(9.0),
                        maximum: Some(1_000.0),
                        unit: "MHz".to_owned(),
                    }),
                    voltage_range: None,
                    current_range: None,
                    power_range: None,
                    detector_modes: vec!["quasi_peak".to_owned()],
                    signal_domain: Some(crate::SignalDomain::Rf),
                    port_directionality: Some(crate::PortDirectionality::Input),
                    connector_requirement: Some("N".to_owned()),
                    impedance_ohm: Some(50.0),
                    communication_capability: Some("scpi".to_owned()),
                    automated_control_required: true,
                    required_driver_action: Some("measure_emission".to_owned()),
                }),
            exact_asset_id: (policy == StationMaterialSelectionPolicy::ExactAsset)
                .then(|| "CA-001".to_owned()),
            logical_ports: vec![StationLogicalPortRequirementDefinition {
                logical_port_id: "rf".to_owned(),
                label: "RF".to_owned(),
                directionality: crate::PortDirectionality::Input,
                signal_domain: crate::SignalDomain::Rf,
                connector_requirement: Some("N".to_owned()),
                impedance_ohm: Some(50.0),
                frequency_range: Some(StationRangeConstraint {
                    minimum: Some(9.0),
                    maximum: Some(1_000.0),
                    unit: "MHz".to_owned(),
                }),
                voltage_range: None,
                current_range: None,
                power_range: None,
            }],
        }
    }

    fn v3_definition() -> StationMeasurementSetupDefinition {
        let source = StationMaterialRequirementDefinition {
            logical_ports: vec![StationLogicalPortRequirementDefinition {
                logical_port_id: "rf".to_owned(),
                label: "RF".to_owned(),
                directionality: crate::PortDirectionality::Output,
                signal_domain: crate::SignalDomain::Rf,
                connector_requirement: Some("N".to_owned()),
                impedance_ohm: Some(50.0),
                frequency_range: None,
                voltage_range: None,
                current_range: None,
                power_range: None,
            }],
            ..v3_requirement(
                "source",
                "Source RF",
                StationMaterialSelectionPolicy::CategoryPool,
            )
        };
        let receiver = v3_requirement(
            "receiver",
            "Récepteur EMI",
            StationMaterialSelectionPolicy::CapabilityMatch,
        );
        StationMeasurementSetupDefinition {
            definition_schema_version: STATION_SETUP_DEFINITION_SCHEMA_VERSION.to_owned(),
            setup_id: "SETUP-V3-001".to_owned(),
            label: "Chaîne RF logique".to_owned(),
            laboratory_location_id: Some("LAB-CEM".to_owned()),
            laboratory_location_label: "Labo CEM".to_owned(),
            planned_use_on: "2026-07-31".to_owned(),
            execution_mode: "accredited".to_owned(),
            asset_bindings: Vec::new(),
            connections: Vec::new(),
            correction_selections: Vec::new(),
            material_requirements: vec![source, receiver],
            material_assignments: Vec::new(),
            logical_connections: vec![StationLogicalConnectionDefinition {
                connection_id: "rf-path".to_owned(),
                label: "Source vers récepteur".to_owned(),
                from: StationLogicalPortEndpoint {
                    requirement_id: "source".to_owned(),
                    logical_port_id: "rf".to_owned(),
                },
                to: StationLogicalPortEndpoint {
                    requirement_id: "receiver".to_owned(),
                    logical_port_id: "rf".to_owned(),
                },
            }],
            notes: BTreeMap::new(),
        }
    }

    fn receiver_model() -> crate::EquipmentModelDefinition {
        serde_json::from_value(serde_json::json!({
            "definition_schema_version": "emc-locus.equipment-model-definition.v2",
            "manufacturer": "Demo",
            "model_name": "Receiver",
            "equipment_class": "controllable_instrument",
            "functional_role": "measurement_instrument",
            "category_code": "emi_receivers",
            "signal_domains": ["rf"],
            "specifications": [{
                "specification_id": "frequency_range",
                "label": "Frequency range",
                "quantity": "frequency",
                "unit": "Hz",
                "minimum": 9000000.0,
                "maximum": 1000000000.0,
                "conditions": []
            }],
            "signal_ports": [{
                "port_id": "rf_input",
                "label": "RF input",
                "directionality": "input",
                "flow_role": "measurement_port",
                "signal_domain": "rf",
                "required": true,
                "connector_type": "N",
                "quantity": "voltage",
                "unit": "V",
                "impedance": 50.0,
                "frequency_min": 9000000.0,
                "frequency_max": 1000000000.0
            }],
            "communication_interfaces": [{
                "interface_id": "lan",
                "label": "LAN",
                "transport_kind": "ethernet_tcp",
                "access_provider_kind": "native_tcp",
                "protocol_kind": "scpi",
                "required": true,
                "default_interface": true,
                "configuration_schema": {},
                "default_configuration": {},
                "firmware_compatibility": []
            }],
            "capabilities": [{
                "capability_id": "measure_emission_1",
                "label": "Measure emission",
                "description": "EMI receiver",
                "capability_kind": "measure_emission",
                "inputs": [{
                    "name": "detector",
                    "value_type": "text",
                    "quantity": "text",
                    "unit": "1",
                    "required": true,
                    "enum_values": ["peak", "quasi_peak"]
                }],
                "outputs": [],
                "constraints": [],
                "required_signal_ports": ["rf_input"],
                "safety_class": "read_only"
            }],
            "custom_field_values": {},
            "metadata": {}
        }))
        .unwrap()
    }

    #[test]
    fn canonical_checksum_is_stable_across_collection_order() {
        let mut first = definition();
        first.asset_bindings.push(binding("cable", "SA-CABLE-001"));
        first.connections.push(StationConnectionDefinition {
            connection_id: "link-002".to_owned(),
            label: "Câble vers récepteur".to_owned(),
            from: StationPortEndpoint {
                binding_id: "cable".to_owned(),
                port_id: "RF_B".to_owned(),
            },
            to: StationPortEndpoint {
                binding_id: "receiver".to_owned(),
                port_id: "RF_IN_2".to_owned(),
            },
        });
        let mut second = first.clone();
        second.asset_bindings.reverse();
        second.connections.reverse();

        let first = first.canonicalize().unwrap();
        let second = second.canonicalize().unwrap();
        assert_eq!(first.canonical_json, second.canonical_json);
        assert_eq!(first.definition_checksum, second.definition_checksum);
    }

    #[test]
    fn legacy_definition_keeps_its_original_canonical_shape() {
        let legacy_json = concat!(
            "{\"asset_bindings\":[],\"connections\":[],\"correction_selections\":[],",
            "\"definition_schema_version\":\"emc-locus.station-measurement-setup-definition.v1\",",
            "\"execution_mode\":\"investigation\",\"label\":\"Montage historique\",",
            "\"planned_use_on\":\"2026-07-16\",\"setup_id\":\"SETUP-LEGACY-001\",",
            "\"station_label\":\"Ancien poste\"}"
        );
        let definition = StationMeasurementSetupDefinition::from_json_str(legacy_json).unwrap();
        let canonical = definition.canonicalize().unwrap();

        assert_eq!(canonical.canonical_json, legacy_json);
        assert!(definition.laboratory_location_id.is_none());
        assert_eq!(definition.laboratory_location_label, "Ancien poste");
    }

    #[test]
    fn incomplete_draft_is_canonical_but_not_ready() {
        let mut definition = definition();
        definition.asset_bindings.clear();
        definition.connections.clear();

        assert!(definition.canonicalize().is_ok());
        let issues = definition.structural_readiness_issues();
        assert!(issues
            .iter()
            .any(|issue| issue.code == "station_setup_requires_two_materials"));
        assert!(issues
            .iter()
            .any(|issue| issue.code == "station_setup_requires_connection"));
    }

    #[test]
    fn rejects_duplicate_asset_and_destination_port() {
        let mut definition = definition();
        definition
            .asset_bindings
            .push(binding("receiver-2", "SA-RX-001"));
        definition.connections.push(StationConnectionDefinition {
            connection_id: "link-002".to_owned(),
            label: "Deuxième liaison".to_owned(),
            from: StationPortEndpoint {
                binding_id: "receiver-2".to_owned(),
                port_id: "RF_OUT".to_owned(),
            },
            to: StationPortEndpoint {
                binding_id: "receiver".to_owned(),
                port_id: "RF_IN".to_owned(),
            },
        });

        let issues = definition.validate_integrity();
        assert!(issues
            .iter()
            .any(|issue| issue.code == "duplicate_station_asset"));
        assert!(issues
            .iter()
            .any(|issue| issue.code == "station_input_connected_twice"));
    }

    #[test]
    fn detects_signal_loop() {
        let mut definition = definition();
        definition.connections.push(StationConnectionDefinition {
            connection_id: "link-return".to_owned(),
            label: "Retour interdit".to_owned(),
            from: StationPortEndpoint {
                binding_id: "receiver".to_owned(),
                port_id: "OUT".to_owned(),
            },
            to: StationPortEndpoint {
                binding_id: "antenna".to_owned(),
                port_id: "IN".to_owned(),
            },
        });

        assert!(definition
            .structural_readiness_issues()
            .iter()
            .any(|issue| issue.code == "station_setup_cycle_detected"));
    }

    #[test]
    fn rejects_invalid_reference_checksum() {
        let mut definition = definition();
        definition.asset_bindings[0].equipment_model_checksum = "SHA256:ABC".to_owned();
        assert!(definition
            .validate_integrity()
            .iter()
            .any(|issue| issue.code == "invalid_station_setup_checksum"));
    }

    #[test]
    fn v3_canonicalization_is_stable_and_rejects_duplicate_requirements() {
        let first = v3_definition();
        let mut reordered = first.clone();
        reordered.material_requirements.reverse();
        assert_eq!(
            first.canonicalize().unwrap().definition_checksum,
            reordered.canonicalize().unwrap().definition_checksum
        );

        let mut duplicated = first;
        duplicated
            .material_requirements
            .push(duplicated.material_requirements[0].clone());
        assert!(duplicated
            .validate_integrity()
            .iter()
            .any(|issue| issue.code == "duplicate_station_material_requirement_id"));
    }

    #[test]
    fn category_matching_honors_descendants() {
        let requirement = v3_requirement(
            "receiver",
            "Receiver",
            StationMaterialSelectionPolicy::CategoryPool,
        );
        let compatible = evaluate_station_material_requirement(
            &requirement,
            "RX-001",
            "MODEL-RX",
            &[
                "instruments".to_owned(),
                "receivers".to_owned(),
                "emi_receivers".to_owned(),
            ],
            &receiver_model(),
            &BTreeSet::new(),
        );
        assert_eq!(compatible.state, StationCompatibilityState::Compatible);
    }

    #[test]
    fn capability_matching_checks_range_detector_domain_driver_and_ports() {
        let requirement = v3_requirement(
            "receiver",
            "Receiver",
            StationMaterialSelectionPolicy::CapabilityMatch,
        );
        let compatible = evaluate_station_material_requirement(
            &requirement,
            "RX-001",
            "MODEL-RX",
            &["emi_receivers".to_owned()],
            &receiver_model(),
            &BTreeSet::from(["measure_emission".to_owned()]),
        );
        assert_eq!(compatible.state, StationCompatibilityState::Compatible);
        assert_eq!(
            compatible.logical_port_candidates["rf"],
            vec!["rf_input".to_owned()]
        );

        let mut invalid = requirement;
        invalid
            .capability_requirement
            .as_mut()
            .unwrap()
            .detector_modes = vec!["average".to_owned()];
        let rejected = evaluate_station_material_requirement(
            &invalid,
            "RX-001",
            "MODEL-RX",
            &["emi_receivers".to_owned()],
            &receiver_model(),
            &BTreeSet::new(),
        );
        assert_eq!(rejected.state, StationCompatibilityState::Incompatible);
        assert!(rejected
            .reasons
            .iter()
            .any(|reason| reason.code == "detector_mode_missing"));
        assert!(rejected
            .reasons
            .iter()
            .any(|reason| reason.code == "automated_control_missing"));
    }

    #[test]
    fn unknown_units_are_indeterminate_and_never_guessed() {
        let mut requirement = v3_requirement(
            "receiver",
            "Receiver",
            StationMaterialSelectionPolicy::CapabilityMatch,
        );
        requirement
            .capability_requirement
            .as_mut()
            .unwrap()
            .frequency_range
            .as_mut()
            .unwrap()
            .unit = "furlong_per_fortnight".to_owned();
        requirement.logical_ports.clear();
        let result = evaluate_station_material_requirement(
            &requirement,
            "RX-001",
            "MODEL-RX",
            &[],
            &receiver_model(),
            &BTreeSet::from(["measure_emission".to_owned()]),
        );
        assert_eq!(result.state, StationCompatibilityState::Indeterminate);
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.code == "unit_conversion_indeterminate"));
    }

    #[test]
    fn exact_asset_requirement_is_distinct_from_assignment_and_readiness() {
        let requirement = v3_requirement(
            "receiver",
            "Cable imposed",
            StationMaterialSelectionPolicy::ExactAsset,
        );
        let mismatch = evaluate_station_material_requirement(
            &requirement,
            "CA-002",
            "MODEL-CABLE",
            &[],
            &receiver_model(),
            &BTreeSet::new(),
        );
        assert_eq!(mismatch.state, StationCompatibilityState::Incompatible);

        let mut setup = v3_definition();
        setup.material_requirements[1] = requirement;
        assert!(setup.material_assignments.is_empty());
        assert!(station_setup_qualification_issues(&setup).is_empty());
        assert!(setup
            .structural_readiness_issues()
            .iter()
            .any(|issue| issue.code == "station_required_material_unassigned"));
    }

    #[test]
    fn assignment_snapshots_must_reference_declared_logical_ports() {
        let mut setup = v3_definition();
        setup
            .material_assignments
            .push(StationMaterialAssignmentDefinition {
                requirement_id: "receiver".to_owned(),
                asset_id: "RX-001".to_owned(),
                asset_revision: "1".to_owned(),
                inventory_code: "RX-001".to_owned(),
                serial_number: Some("SN-1".to_owned()),
                equipment_model_id: "MODEL-RX".to_owned(),
                equipment_model_revision_id: "MODEL-RX-rev-0001".to_owned(),
                equipment_model_checksum: format!("sha256:{}", "a".repeat(64)),
                selected_ports: vec![StationPhysicalPortMappingDefinition {
                    logical_port_id: "missing".to_owned(),
                    actual_port_id: "rf_input".to_owned(),
                }],
                assignment_context: "setup_definition".to_owned(),
                assigned_on: "2026-07-31".to_owned(),
            });
        assert!(setup
            .validate_integrity()
            .iter()
            .any(|issue| issue.code == "station_assignment_unknown_logical_port"));
    }
}
