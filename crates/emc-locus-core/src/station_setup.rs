use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const STATION_SETUP_DEFINITION_SCHEMA_VERSION: &str =
    "emc-locus.station-measurement-setup-definition.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationSetupRevisionStatus {
    Draft,
    Ready,
    Superseded,
}

impl StationSetupRevisionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Ready => "ready",
            Self::Superseded => "superseded",
        }
    }
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
    pub station_label: String,
    pub planned_use_on: String,
    pub execution_mode: String,
    #[serde(default)]
    pub asset_bindings: Vec<StationAssetBindingDefinition>,
    #[serde(default)]
    pub connections: Vec<StationConnectionDefinition>,
    #[serde(default)]
    pub correction_selections: Vec<StationCorrectionSelectionDefinition>,
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
        normalized.station_label = normalized.station_label.trim().to_owned();
        for binding in &mut normalized.asset_bindings {
            binding.role_label = binding.role_label.trim().to_owned();
        }
        for connection in &mut normalized.connections {
            connection.label = connection.label.trim().to_owned();
        }
        for selection in &mut normalized.correction_selections {
            selection.label = selection.label.trim().to_owned();
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

        let mut value = serde_json::to_value(&normalized).map_err(|error| {
            vec![StationSetupValidationIssue {
                code: "station_setup_serialization_failed".to_owned(),
                path: "$".to_owned(),
                message: error.to_string(),
            }]
        })?;
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
    if definition.definition_schema_version != STATION_SETUP_DEFINITION_SCHEMA_VERSION {
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
    require_text(
        &mut issues,
        &definition.station_label,
        "station_label",
        "a station name is required",
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

pub fn station_setup_structural_readiness_issues(
    definition: &StationMeasurementSetupDefinition,
) -> Vec<StationReadinessIssue> {
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
            definition_schema_version: STATION_SETUP_DEFINITION_SCHEMA_VERSION.to_owned(),
            setup_id: "setup-rf-001".to_owned(),
            label: "Chaîne RF réception".to_owned(),
            station_label: "Salle CEM 1".to_owned(),
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
            notes: BTreeMap::new(),
        }
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
}
