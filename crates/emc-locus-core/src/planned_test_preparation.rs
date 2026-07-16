use crate::service_planning::ServiceScheduleStatus;
use crate::station_setup::{
    StationCorrectionKind, StationReadinessDimension, StationReadinessSeverity,
    StationSetupReadiness, StationSetupRevisionStatus,
};
use crate::test_definitions::{
    CalibrationRequirement, InstrumentSubstitutionPolicy, InstrumentationChainSlot,
    MeasurementAxis, TemplateRevisionStatus,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const PLANNED_TEST_PREPARATION_SCHEMA_VERSION: &str = "emc-locus.planned-test-preparation.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannedTestPreparationSeverity {
    Blocking,
    Warning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannedTestPreparationDimension {
    ScheduleContext,
    TestMethod,
    StationSetup,
    InstrumentAssignment,
    Serviceability,
    CalibrationValidity,
    MissingEvidence,
    Nonconformance,
    CorrectionValidity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannedTestPreparationState {
    Blocked,
    Ready,
    Stale,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedTestPreparationIssue {
    pub code: String,
    pub severity: PlannedTestPreparationSeverity,
    pub dimension: PlannedTestPreparationDimension,
    pub message: String,
    pub next_action: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub method_slot_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binding_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asset_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedTestPreparationVerdict {
    pub ready: bool,
    pub checked_on: String,
    pub issues: Vec<PlannedTestPreparationIssue>,
}

impl PlannedTestPreparationVerdict {
    fn from_issues(checked_on: String, issues: Vec<PlannedTestPreparationIssue>) -> Self {
        let ready = !issues
            .iter()
            .any(|issue| issue.severity == PlannedTestPreparationSeverity::Blocking);
        Self {
            ready,
            checked_on,
            issues,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedTestScheduleSnapshot {
    pub project_code: String,
    pub item_code: String,
    pub revision: u64,
    pub title: String,
    pub planned_start_at: String,
    pub planned_end_at: String,
    pub assigned_operator: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laboratory_location_id: Option<String>,
    #[serde(alias = "location")]
    pub laboratory_location_label: String,
    pub equipment_under_test: String,
    pub execution_mode: String,
    pub status: ServiceScheduleStatus,
}

impl PlannedTestScheduleSnapshot {
    pub fn planned_date(&self) -> &str {
        self.planned_start_at.get(0..10).unwrap_or("")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedTestMethodSnapshot {
    pub template_id: String,
    pub revision_id: String,
    pub revision_number: u32,
    pub revision_status: TemplateRevisionStatus,
    pub definition_checksum: String,
    pub title: String,
    pub measurement_axis: MeasurementAxis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub standard_references: Vec<String>,
    pub instrumentation_chain: Vec<InstrumentationChainSlot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedEquipmentCapabilitySnapshot {
    pub capability_id: String,
    pub label: String,
    pub capability_kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedStationAssetSnapshot {
    pub binding_id: String,
    pub role_label: String,
    pub asset_id: String,
    pub asset_revision: String,
    pub inventory_code: String,
    pub serial_number: String,
    pub manufacturer: String,
    pub model_name: String,
    pub equipment_model_id: String,
    pub equipment_model_revision_id: String,
    pub equipment_model_checksum: String,
    pub category_code: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<PreparedEquipmentCapabilitySnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedStationCorrectionSnapshot {
    pub selection_id: String,
    pub binding_id: String,
    pub correction_kind: StationCorrectionKind,
    pub characterization_id: String,
    pub characterization_checksum: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedStationSetupSnapshot {
    pub setup_id: String,
    pub revision_id: String,
    pub revision_number: u32,
    pub revision_status: StationSetupRevisionStatus,
    pub definition_checksum: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laboratory_location_id: Option<String>,
    #[serde(alias = "station_label")]
    pub laboratory_location_label: String,
    pub planned_use_on: String,
    pub execution_mode: String,
    pub assets: Vec<PreparedStationAssetSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corrections: Vec<PreparedStationCorrectionSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedTestInstrumentAssignment {
    pub slot_id: String,
    pub binding_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedTestMaterialCompatibility {
    pub slot_id: String,
    pub binding_id: String,
    pub compatible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_action: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedTestPreparationAssessmentInput {
    pub schedule: PlannedTestScheduleSnapshot,
    pub method: PreparedTestMethodSnapshot,
    pub station_setup: PreparedStationSetupSnapshot,
    pub assignments: Vec<PlannedTestInstrumentAssignment>,
    pub station_readiness: StationSetupReadiness,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedTestPreparationDefinition {
    pub definition_schema_version: String,
    pub schedule: PlannedTestScheduleSnapshot,
    pub method: PreparedTestMethodSnapshot,
    pub station_setup: PreparedStationSetupSnapshot,
    pub assignments: Vec<PlannedTestInstrumentAssignment>,
    pub verdict: PlannedTestPreparationVerdict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalPlannedTestPreparationDefinition {
    pub definition_schema_version: String,
    pub canonical_json: String,
    pub definition_checksum: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedTestPreparationValidationIssue {
    pub code: String,
    pub path: String,
    pub message: String,
}

impl PlannedTestPreparationDefinition {
    pub fn from_json_str(value: &str) -> Result<Self, PlannedTestPreparationValidationIssue> {
        serde_json::from_str(value).map_err(|error| PlannedTestPreparationValidationIssue {
            code: "invalid_planned_test_preparation_json".to_owned(),
            path: "$".to_owned(),
            message: format!("invalid planned test preparation definition: {error}"),
        })
    }

    pub fn validate_integrity(&self) -> Vec<PlannedTestPreparationValidationIssue> {
        validate_definition_integrity(self)
    }

    pub fn effective_state(&self, current_schedule_revision: u64) -> PlannedTestPreparationState {
        if self.schedule.revision != current_schedule_revision {
            PlannedTestPreparationState::Stale
        } else if self.verdict.ready {
            PlannedTestPreparationState::Ready
        } else {
            PlannedTestPreparationState::Blocked
        }
    }

    pub fn permits_start(
        &self,
        current_schedule_revision: u64,
        current_status: ServiceScheduleStatus,
    ) -> bool {
        current_status == ServiceScheduleStatus::Confirmed
            && self.effective_state(current_schedule_revision) == PlannedTestPreparationState::Ready
    }

    pub fn canonicalize(
        &self,
    ) -> Result<CanonicalPlannedTestPreparationDefinition, Vec<PlannedTestPreparationValidationIssue>>
    {
        let issues = self.validate_integrity();
        if !issues.is_empty() {
            return Err(issues);
        }
        let mut normalized = self.clone();
        normalize_definition(&mut normalized);
        let mut value = serde_json::to_value(&normalized).map_err(|error| {
            vec![validation_issue(
                "planned_test_preparation_serialization_failed",
                "$",
                &error.to_string(),
            )]
        })?;
        if normalized.schedule.laboratory_location_id.is_none()
            && normalized.station_setup.laboratory_location_id.is_none()
        {
            let object = value
                .as_object_mut()
                .expect("planned test preparation is an object");
            if let Some(schedule) = object.get_mut("schedule").and_then(Value::as_object_mut) {
                schedule.remove("laboratory_location_id");
                if let Some(label) = schedule.remove("laboratory_location_label") {
                    schedule.insert("location".to_owned(), label);
                }
            }
            if let Some(station) = object
                .get_mut("station_setup")
                .and_then(Value::as_object_mut)
            {
                station.remove("laboratory_location_id");
                if let Some(label) = station.remove("laboratory_location_label") {
                    station.insert("station_label".to_owned(), label);
                }
            }
        }
        canonicalize_json_value(&mut value);
        let canonical_json = serde_json::to_string(&value).map_err(|error| {
            vec![validation_issue(
                "planned_test_preparation_serialization_failed",
                "$",
                &error.to_string(),
            )]
        })?;
        let digest = Sha256::digest(canonical_json.as_bytes());
        Ok(CanonicalPlannedTestPreparationDefinition {
            definition_schema_version: normalized.definition_schema_version,
            canonical_json,
            definition_checksum: format!("sha256:{digest:x}"),
        })
    }
}

pub fn assess_planned_test_preparation(
    input: PlannedTestPreparationAssessmentInput,
) -> Result<PlannedTestPreparationDefinition, Vec<PlannedTestPreparationValidationIssue>> {
    let structural_issues = validate_assessment_input(&input);
    if !structural_issues.is_empty() {
        return Err(structural_issues);
    }

    let mut issues = derive_static_readiness_issues(
        &input.schedule,
        &input.method,
        &input.station_setup,
        &input.assignments,
    );
    issues.extend(
        input
            .station_readiness
            .issues
            .iter()
            .map(|issue| PlannedTestPreparationIssue {
                code: issue.code.clone(),
                severity: match issue.severity {
                    StationReadinessSeverity::Blocking => PlannedTestPreparationSeverity::Blocking,
                    StationReadinessSeverity::Warning => PlannedTestPreparationSeverity::Warning,
                },
                dimension: preparation_dimension(issue.dimension),
                message: issue.message.clone(),
                next_action: station_issue_next_action(issue.dimension).to_owned(),
                method_slot_ids: Vec::new(),
                binding_ids: issue.binding_ids.clone(),
                asset_ids: asset_ids_for_bindings(&input.station_setup, &issue.binding_ids),
            }),
    );
    sort_readiness_issues(&mut issues);

    let definition = PlannedTestPreparationDefinition {
        definition_schema_version: PLANNED_TEST_PREPARATION_SCHEMA_VERSION.to_owned(),
        schedule: input.schedule,
        method: input.method,
        station_setup: input.station_setup,
        assignments: input.assignments,
        verdict: PlannedTestPreparationVerdict::from_issues(
            input.station_readiness.checked_on,
            issues,
        ),
    };
    let integrity_issues = definition.validate_integrity();
    if integrity_issues.is_empty() {
        Ok(definition)
    } else {
        Err(integrity_issues)
    }
}

pub fn assess_planned_test_material_compatibility(
    method: &PreparedTestMethodSnapshot,
    station: &PreparedStationSetupSnapshot,
    station_readiness: &StationSetupReadiness,
) -> Vec<PlannedTestMaterialCompatibility> {
    let mut results = method
        .instrumentation_chain
        .iter()
        .flat_map(|slot| {
            station.assets.iter().map(move |asset| {
                let incompatibility = material_incompatibility(slot, asset, station_readiness);
                PlannedTestMaterialCompatibility {
                    slot_id: slot.slot_id.clone(),
                    binding_id: asset.binding_id.clone(),
                    compatible: incompatibility.is_none(),
                    reason: incompatibility.as_ref().map(|value| value.0.clone()),
                    next_action: incompatibility.map(|value| value.1),
                }
            })
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| {
        left.slot_id
            .cmp(&right.slot_id)
            .then_with(|| left.binding_id.cmp(&right.binding_id))
    });
    results
}

fn material_incompatibility(
    slot: &InstrumentationChainSlot,
    asset: &PreparedStationAssetSnapshot,
    station_readiness: &StationSetupReadiness,
) -> Option<(String, String)> {
    if let Some(required_category) = slot.required_category.as_deref() {
        if !asset
            .category_code
            .trim()
            .eq_ignore_ascii_case(required_category.trim())
        {
            return Some((
                substitution_mismatch_reason(slot, asset, "catégorie"),
                "Choisissez un matériel de la catégorie attendue ou mettez à jour son modèle."
                    .to_owned(),
            ));
        }
    }
    if let Some(required_capability) = slot.required_capability.as_deref() {
        let has_capability = asset.capabilities.iter().any(|capability| {
            capability
                .capability_id
                .trim()
                .eq_ignore_ascii_case(required_capability.trim())
                || capability
                    .capability_kind
                    .trim()
                    .eq_ignore_ascii_case(required_capability.trim())
        });
        if !has_capability {
            return Some((
                substitution_mismatch_reason(slot, asset, "capacité"),
                "Choisissez un matériel dont le modèle déclare la capacité attendue.".to_owned(),
            ));
        }
    }

    station_readiness
        .issues
        .iter()
        .find(|issue| {
            issue.severity == StationReadinessSeverity::Blocking
                && issue
                    .binding_ids
                    .iter()
                    .any(|binding_id| binding_id == &asset.binding_id)
                && material_readiness_dimension_applies(slot, issue.dimension)
        })
        .map(|issue| {
            (
                issue.message.clone(),
                station_issue_next_action(issue.dimension).to_owned(),
            )
        })
}

fn substitution_mismatch_reason(
    slot: &InstrumentationChainSlot,
    asset: &PreparedStationAssetSnapshot,
    requirement: &str,
) -> String {
    let policy = match slot.substitution_policy {
        InstrumentSubstitutionPolicy::NoSubstitution => {
            "La substitution n'est pas autorisée pour ce rôle."
        }
        InstrumentSubstitutionPolicy::SameCategory => "La substitution exige la même catégorie.",
        InstrumentSubstitutionPolicy::SameCapability => "La substitution exige la même capacité.",
        InstrumentSubstitutionPolicy::ApprovedEquivalent => {
            "Aucune équivalence approuvée ne permet de relâcher l'exigence déclarée."
        }
    };
    format!(
        "{} ne respecte pas l'exigence de {requirement} pour « {} ». {policy}",
        material_label(asset),
        slot.label,
    )
}

fn material_readiness_dimension_applies(
    slot: &InstrumentationChainSlot,
    dimension: StationReadinessDimension,
) -> bool {
    match dimension {
        StationReadinessDimension::AssetIdentity
        | StationReadinessDimension::Serviceability
        | StationReadinessDimension::Nonconformance
        | StationReadinessDimension::CorrectionValidity => true,
        StationReadinessDimension::CalibrationValidity
        | StationReadinessDimension::MissingEvidence => {
            slot.calibration_requirement != CalibrationRequirement::NotRequired
        }
        StationReadinessDimension::Structure | StationReadinessDimension::PortCompatibility => {
            false
        }
    }
}

fn validate_assessment_input(
    input: &PlannedTestPreparationAssessmentInput,
) -> Vec<PlannedTestPreparationValidationIssue> {
    let placeholder = PlannedTestPreparationDefinition {
        definition_schema_version: PLANNED_TEST_PREPARATION_SCHEMA_VERSION.to_owned(),
        schedule: input.schedule.clone(),
        method: input.method.clone(),
        station_setup: input.station_setup.clone(),
        assignments: input.assignments.clone(),
        verdict: PlannedTestPreparationVerdict {
            ready: true,
            checked_on: input.station_readiness.checked_on.clone(),
            issues: Vec::new(),
        },
    };
    validate_definition_structure(&placeholder)
}

fn validate_definition_integrity(
    definition: &PlannedTestPreparationDefinition,
) -> Vec<PlannedTestPreparationValidationIssue> {
    let mut issues = validate_definition_structure(definition);
    if definition.verdict.checked_on != definition.schedule.planned_date() {
        issues.push(validation_issue(
            "planned_test_preparation_verdict_date_mismatch",
            "verdict.checked_on",
            "the preparation verdict must be checked for the scheduled date",
        ));
    }
    let expected_ready = !definition
        .verdict
        .issues
        .iter()
        .any(|issue| issue.severity == PlannedTestPreparationSeverity::Blocking);
    if definition.verdict.ready != expected_ready {
        issues.push(validation_issue(
            "planned_test_preparation_verdict_inconsistent",
            "verdict.ready",
            "the preparation readiness flag does not match its blocking issues",
        ));
    }
    for expected in derive_static_readiness_issues(
        &definition.schedule,
        &definition.method,
        &definition.station_setup,
        &definition.assignments,
    ) {
        if !definition.verdict.issues.iter().any(|stored| {
            stored.code == expected.code
                && stored.severity == expected.severity
                && stored.dimension == expected.dimension
                && same_values(&stored.method_slot_ids, &expected.method_slot_ids)
                && same_values(&stored.binding_ids, &expected.binding_ids)
                && same_values(&stored.asset_ids, &expected.asset_ids)
        }) {
            issues.push(validation_issue(
                "planned_test_preparation_static_verdict_incomplete",
                "verdict.issues",
                "the stored preparation verdict omits a derived blocking or warning issue",
            ));
        }
    }
    let slot_ids: BTreeSet<&str> = definition
        .method
        .instrumentation_chain
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect();
    let binding_ids: BTreeSet<&str> = definition
        .station_setup
        .assets
        .iter()
        .map(|asset| asset.binding_id.as_str())
        .collect();
    for (index, issue) in definition.verdict.issues.iter().enumerate() {
        require_text(
            &mut issues,
            &issue.code,
            &format!("verdict.issues[{index}].code"),
        );
        require_text(
            &mut issues,
            &issue.message,
            &format!("verdict.issues[{index}].message"),
        );
        require_text(
            &mut issues,
            &issue.next_action,
            &format!("verdict.issues[{index}].next_action"),
        );
        for slot_id in &issue.method_slot_ids {
            if !slot_ids.contains(slot_id.as_str()) {
                issues.push(validation_issue(
                    "planned_test_preparation_issue_unknown_slot",
                    &format!("verdict.issues[{index}].method_slot_ids"),
                    "a verdict issue references an unknown method role",
                ));
            }
        }
        for binding_id in &issue.binding_ids {
            if !binding_ids.contains(binding_id.as_str()) {
                issues.push(validation_issue(
                    "planned_test_preparation_issue_unknown_binding",
                    &format!("verdict.issues[{index}].binding_ids"),
                    "a verdict issue references an unknown station material",
                ));
            }
        }
    }
    issues
}

fn validate_definition_structure(
    definition: &PlannedTestPreparationDefinition,
) -> Vec<PlannedTestPreparationValidationIssue> {
    let mut issues = Vec::new();
    if definition.definition_schema_version != PLANNED_TEST_PREPARATION_SCHEMA_VERSION {
        issues.push(validation_issue(
            "unsupported_planned_test_preparation_schema",
            "definition_schema_version",
            "unsupported planned test preparation schema",
        ));
    }
    require_id(
        &mut issues,
        &definition.schedule.project_code,
        "schedule.project_code",
    );
    require_id(
        &mut issues,
        &definition.schedule.item_code,
        "schedule.item_code",
    );
    if definition.schedule.revision == 0 {
        issues.push(validation_issue(
            "invalid_planned_test_schedule_revision",
            "schedule.revision",
            "the scheduled test revision must be greater than zero",
        ));
    }
    for (path, value) in [
        ("schedule.title", definition.schedule.title.as_str()),
        (
            "schedule.assigned_operator",
            definition.schedule.assigned_operator.as_str(),
        ),
        (
            "schedule.laboratory_location_label",
            definition.schedule.laboratory_location_label.as_str(),
        ),
        (
            "schedule.equipment_under_test",
            definition.schedule.equipment_under_test.as_str(),
        ),
    ] {
        require_text(&mut issues, value, path);
    }
    if let Some(location_id) = definition.schedule.laboratory_location_id.as_deref() {
        require_id(&mut issues, location_id, "schedule.laboratory_location_id");
    }
    if definition.schedule.planned_date().len() != 10 {
        issues.push(validation_issue(
            "invalid_planned_test_schedule_date",
            "schedule.planned_start_at",
            "the scheduled test must expose a canonical local date",
        ));
    }
    if !matches!(
        definition.schedule.execution_mode.as_str(),
        "accredited" | "non_accredited" | "investigation"
    ) {
        issues.push(validation_issue(
            "invalid_planned_test_execution_mode",
            "schedule.execution_mode",
            "the scheduled test execution mode is invalid",
        ));
    }
    if !matches!(
        definition.schedule.status,
        ServiceScheduleStatus::Planned | ServiceScheduleStatus::Confirmed
    ) {
        issues.push(validation_issue(
            "planned_test_schedule_not_preparable",
            "schedule.status",
            "only a planned or confirmed test can be prepared",
        ));
    }

    require_id(
        &mut issues,
        &definition.method.template_id,
        "method.template_id",
    );
    require_id(
        &mut issues,
        &definition.method.revision_id,
        "method.revision_id",
    );
    require_checksum(
        &mut issues,
        &definition.method.definition_checksum,
        "method.definition_checksum",
    );
    require_text(&mut issues, &definition.method.title, "method.title");
    if definition.method.revision_number == 0 {
        issues.push(validation_issue(
            "invalid_planned_test_method_revision_number",
            "method.revision_number",
            "the method revision number must be greater than zero",
        ));
    }
    let mut slot_ids = BTreeSet::new();
    for (index, slot) in definition.method.instrumentation_chain.iter().enumerate() {
        require_id(
            &mut issues,
            &slot.slot_id,
            &format!("method.instrumentation_chain[{index}].slot_id"),
        );
        require_text(
            &mut issues,
            &slot.label,
            &format!("method.instrumentation_chain[{index}].label"),
        );
        if !slot_ids.insert(slot.slot_id.as_str()) {
            issues.push(validation_issue(
                "duplicate_planned_test_method_slot",
                &format!("method.instrumentation_chain[{index}].slot_id"),
                "method role identifiers must be unique",
            ));
        }
    }

    require_id(
        &mut issues,
        &definition.station_setup.setup_id,
        "station_setup.setup_id",
    );
    require_id(
        &mut issues,
        &definition.station_setup.revision_id,
        "station_setup.revision_id",
    );
    require_checksum(
        &mut issues,
        &definition.station_setup.definition_checksum,
        "station_setup.definition_checksum",
    );
    require_text(
        &mut issues,
        &definition.station_setup.label,
        "station_setup.label",
    );
    require_text(
        &mut issues,
        &definition.station_setup.laboratory_location_label,
        "station_setup.laboratory_location_label",
    );
    if let Some(location_id) = definition.station_setup.laboratory_location_id.as_deref() {
        require_id(
            &mut issues,
            location_id,
            "station_setup.laboratory_location_id",
        );
    }
    if definition.station_setup.revision_number == 0 {
        issues.push(validation_issue(
            "invalid_planned_test_station_revision_number",
            "station_setup.revision_number",
            "the station setup revision number must be greater than zero",
        ));
    }
    let mut binding_ids = BTreeSet::new();
    let mut asset_ids = BTreeSet::new();
    for (index, asset) in definition.station_setup.assets.iter().enumerate() {
        let path = format!("station_setup.assets[{index}]");
        require_id(
            &mut issues,
            &asset.binding_id,
            &format!("{path}.binding_id"),
        );
        require_id(&mut issues, &asset.asset_id, &format!("{path}.asset_id"));
        require_id(
            &mut issues,
            &asset.asset_revision,
            &format!("{path}.asset_revision"),
        );
        require_text(
            &mut issues,
            &asset.role_label,
            &format!("{path}.role_label"),
        );
        require_text(
            &mut issues,
            &asset.inventory_code,
            &format!("{path}.inventory_code"),
        );
        require_text(
            &mut issues,
            &asset.serial_number,
            &format!("{path}.serial_number"),
        );
        require_text(
            &mut issues,
            &asset.manufacturer,
            &format!("{path}.manufacturer"),
        );
        require_text(
            &mut issues,
            &asset.model_name,
            &format!("{path}.model_name"),
        );
        require_text(
            &mut issues,
            &asset.category_code,
            &format!("{path}.category_code"),
        );
        require_checksum(
            &mut issues,
            &asset.equipment_model_checksum,
            &format!("{path}.equipment_model_checksum"),
        );
        if !binding_ids.insert(asset.binding_id.as_str()) {
            issues.push(validation_issue(
                "duplicate_planned_test_station_binding",
                &format!("{path}.binding_id"),
                "station material bindings must be unique",
            ));
        }
        if !asset_ids.insert(asset.asset_id.as_str()) {
            issues.push(validation_issue(
                "duplicate_planned_test_station_asset",
                &format!("{path}.asset_id"),
                "the same physical material cannot appear twice",
            ));
        }
    }

    let mut assignment_slots = BTreeSet::new();
    for (index, assignment) in definition.assignments.iter().enumerate() {
        let path = format!("assignments[{index}]");
        require_id(&mut issues, &assignment.slot_id, &format!("{path}.slot_id"));
        require_id(
            &mut issues,
            &assignment.binding_id,
            &format!("{path}.binding_id"),
        );
        if !assignment_slots.insert(assignment.slot_id.as_str()) {
            issues.push(validation_issue(
                "duplicate_planned_test_assignment",
                &format!("{path}.slot_id"),
                "a method role can only be assigned once",
            ));
        }
        if !slot_ids.contains(assignment.slot_id.as_str()) {
            issues.push(validation_issue(
                "planned_test_assignment_unknown_slot",
                &format!("{path}.slot_id"),
                "the assignment references an unknown method role",
            ));
        }
        if !binding_ids.contains(assignment.binding_id.as_str()) {
            issues.push(validation_issue(
                "planned_test_assignment_unknown_binding",
                &format!("{path}.binding_id"),
                "the assignment references a material outside the selected setup",
            ));
        }
    }
    issues
}

fn derive_static_readiness_issues(
    schedule: &PlannedTestScheduleSnapshot,
    method: &PreparedTestMethodSnapshot,
    station: &PreparedStationSetupSnapshot,
    assignments: &[PlannedTestInstrumentAssignment],
) -> Vec<PlannedTestPreparationIssue> {
    let mut issues = Vec::new();
    if !matches!(
        method.revision_status,
        TemplateRevisionStatus::Approved | TemplateRevisionStatus::Superseded
    ) {
        issues.push(blocking_issue(
            "planned_test_method_not_approved",
            PlannedTestPreparationDimension::TestMethod,
            "La révision de méthode n'est pas approuvée pour cet essai.",
            "Choisissez une révision de méthode approuvée.",
            None,
            None,
            None,
        ));
    }
    if method.revision_status == TemplateRevisionStatus::Superseded {
        issues.push(warning_issue(
            "planned_test_method_superseded",
            PlannedTestPreparationDimension::TestMethod,
            "Une révision plus récente de la méthode existe, mais la révision figée reste identifiable.",
            "Confirmez que la révision figée reste applicable au dossier.",
            None,
            None,
            None,
        ));
    }
    if !matches!(
        station.revision_status,
        StationSetupRevisionStatus::Ready | StationSetupRevisionStatus::Superseded
    ) {
        issues.push(blocking_issue(
            "planned_test_station_not_ready",
            PlannedTestPreparationDimension::StationSetup,
            "Le montage choisi n'est pas prêt à câbler.",
            "Choisissez un montage prêt ou terminez sa préparation dans Test Station.",
            None,
            None,
            None,
        ));
    }
    if station.revision_status == StationSetupRevisionStatus::Superseded {
        issues.push(warning_issue(
            "planned_test_station_superseded",
            PlannedTestPreparationDimension::StationSetup,
            "Une version plus récente du montage existe, mais la version figée reste identifiable.",
            "Confirmez que le montage figé reste celui qui sera câblé.",
            None,
            None,
            None,
        ));
    }
    if station.planned_use_on != schedule.planned_date() {
        issues.push(blocking_issue(
            "planned_test_station_date_mismatch",
            PlannedTestPreparationDimension::ScheduleContext,
            "Le montage n'a pas été vérifié pour la date de ce créneau.",
            "Préparez ou revérifiez le montage pour la date planifiée.",
            None,
            None,
            None,
        ));
    }
    if station.execution_mode != schedule.execution_mode {
        issues.push(blocking_issue(
            "planned_test_station_mode_mismatch",
            PlannedTestPreparationDimension::ScheduleContext,
            "Le mode qualité du montage ne correspond pas au dossier.",
            "Choisissez un montage préparé pour le même mode qualité.",
            None,
            None,
            None,
        ));
    }
    match (
        schedule.laboratory_location_id.as_deref(),
        station.laboratory_location_id.as_deref(),
    ) {
        (Some(schedule_location_id), Some(station_location_id))
            if schedule_location_id != station_location_id =>
        {
            issues.push(blocking_issue(
                "planned_test_station_location_mismatch",
                PlannedTestPreparationDimension::ScheduleContext,
                "Le montage n'est pas rattaché au lieu réservé dans le planning.",
                "Déplacez le créneau vers ce lieu ou choisissez un montage du lieu réservé.",
                None,
                None,
                None,
            ));
        }
        (None, _) | (_, None) => {
            issues.push(blocking_issue(
                "planned_test_location_identity_missing",
                PlannedTestPreparationDimension::ScheduleContext,
                "Le lieu du créneau ou du montage n'a pas d'identifiant stable.",
                "Sélectionnez à nouveau un lieu identifié et un montage rattaché à ce lieu.",
                None,
                None,
                None,
            ));
        }
        _ => {}
    }

    let assignment_by_slot: BTreeMap<&str, &str> = assignments
        .iter()
        .map(|assignment| (assignment.slot_id.as_str(), assignment.binding_id.as_str()))
        .collect();
    let asset_by_binding: BTreeMap<&str, &PreparedStationAssetSnapshot> = station
        .assets
        .iter()
        .map(|asset| (asset.binding_id.as_str(), asset))
        .collect();
    for slot in &method.instrumentation_chain {
        let Some(binding_id) = assignment_by_slot.get(slot.slot_id.as_str()).copied() else {
            if slot.required {
                issues.push(blocking_issue(
                    "planned_test_required_role_unassigned",
                    PlannedTestPreparationDimension::InstrumentAssignment,
                    &format!("Le rôle « {} » n'a pas de matériel affecté.", slot.label),
                    "Choisissez un matériel du montage pour ce rôle.",
                    Some(&slot.slot_id),
                    None,
                    None,
                ));
            }
            continue;
        };
        let Some(asset) = asset_by_binding.get(binding_id).copied() else {
            continue;
        };
        if let Some(category) = slot.required_category.as_deref() {
            if !asset
                .category_code
                .trim()
                .eq_ignore_ascii_case(category.trim())
            {
                issues.push(blocking_issue(
                    "planned_test_role_category_mismatch",
                    PlannedTestPreparationDimension::InstrumentAssignment,
                    &format!(
                        "{} ne correspond pas à la catégorie demandée pour « {} ».",
                        material_label(asset),
                        slot.label
                    ),
                    "Choisissez un matériel de la catégorie attendue.",
                    Some(&slot.slot_id),
                    Some(&asset.binding_id),
                    Some(&asset.asset_id),
                ));
            }
        }
        if let Some(required_capability) = slot.required_capability.as_deref() {
            let has_capability = asset.capabilities.iter().any(|capability| {
                capability
                    .capability_id
                    .trim()
                    .eq_ignore_ascii_case(required_capability.trim())
                    || capability
                        .capability_kind
                        .trim()
                        .eq_ignore_ascii_case(required_capability.trim())
            });
            if !has_capability {
                issues.push(blocking_issue(
                    "planned_test_role_capability_mismatch",
                    PlannedTestPreparationDimension::InstrumentAssignment,
                    &format!(
                        "{} ne fournit pas la capacité demandée pour « {} ».",
                        material_label(asset),
                        slot.label
                    ),
                    "Choisissez un matériel dont le modèle déclare cette capacité.",
                    Some(&slot.slot_id),
                    Some(&asset.binding_id),
                    Some(&asset.asset_id),
                ));
            }
        }
    }
    issues
}

fn blocking_issue(
    code: &str,
    dimension: PlannedTestPreparationDimension,
    message: &str,
    next_action: &str,
    slot_id: Option<&str>,
    binding_id: Option<&str>,
    asset_id: Option<&str>,
) -> PlannedTestPreparationIssue {
    readiness_issue(
        code,
        PlannedTestPreparationSeverity::Blocking,
        dimension,
        message,
        next_action,
        slot_id,
        binding_id,
        asset_id,
    )
}

fn warning_issue(
    code: &str,
    dimension: PlannedTestPreparationDimension,
    message: &str,
    next_action: &str,
    slot_id: Option<&str>,
    binding_id: Option<&str>,
    asset_id: Option<&str>,
) -> PlannedTestPreparationIssue {
    readiness_issue(
        code,
        PlannedTestPreparationSeverity::Warning,
        dimension,
        message,
        next_action,
        slot_id,
        binding_id,
        asset_id,
    )
}

#[allow(clippy::too_many_arguments)]
fn readiness_issue(
    code: &str,
    severity: PlannedTestPreparationSeverity,
    dimension: PlannedTestPreparationDimension,
    message: &str,
    next_action: &str,
    slot_id: Option<&str>,
    binding_id: Option<&str>,
    asset_id: Option<&str>,
) -> PlannedTestPreparationIssue {
    PlannedTestPreparationIssue {
        code: code.to_owned(),
        severity,
        dimension,
        message: message.to_owned(),
        next_action: next_action.to_owned(),
        method_slot_ids: slot_id.into_iter().map(str::to_owned).collect(),
        binding_ids: binding_id.into_iter().map(str::to_owned).collect(),
        asset_ids: asset_id.into_iter().map(str::to_owned).collect(),
    }
}

fn material_label(asset: &PreparedStationAssetSnapshot) -> String {
    format!(
        "Le matériel {} / {} ({})",
        asset.inventory_code, asset.serial_number, asset.model_name
    )
}

fn preparation_dimension(dimension: StationReadinessDimension) -> PlannedTestPreparationDimension {
    match dimension {
        StationReadinessDimension::Structure
        | StationReadinessDimension::AssetIdentity
        | StationReadinessDimension::PortCompatibility => {
            PlannedTestPreparationDimension::StationSetup
        }
        StationReadinessDimension::Serviceability => {
            PlannedTestPreparationDimension::Serviceability
        }
        StationReadinessDimension::CalibrationValidity => {
            PlannedTestPreparationDimension::CalibrationValidity
        }
        StationReadinessDimension::MissingEvidence => {
            PlannedTestPreparationDimension::MissingEvidence
        }
        StationReadinessDimension::Nonconformance => {
            PlannedTestPreparationDimension::Nonconformance
        }
        StationReadinessDimension::CorrectionValidity => {
            PlannedTestPreparationDimension::CorrectionValidity
        }
    }
}

fn station_issue_next_action(dimension: StationReadinessDimension) -> &'static str {
    match dimension {
        StationReadinessDimension::Structure
        | StationReadinessDimension::AssetIdentity
        | StationReadinessDimension::PortCompatibility => {
            "Corrigez le montage dans Test Station puis créez une nouvelle vérification."
        }
        StationReadinessDimension::Serviceability => {
            "Choisissez un matériel utilisable ou faites mettre à jour son état."
        }
        StationReadinessDimension::CalibrationValidity => {
            "Faites vérifier l'étalonnage ou choisissez un autre matériel."
        }
        StationReadinessDimension::MissingEvidence => {
            "Ajoutez la preuve requise dans le dossier du matériel."
        }
        StationReadinessDimension::Nonconformance => {
            "Traitez la non-conformité avant d'utiliser ce matériel."
        }
        StationReadinessDimension::CorrectionValidity => {
            "Choisissez une correction du matériel applicable à la date prévue."
        }
    }
}

fn asset_ids_for_bindings(
    station: &PreparedStationSetupSnapshot,
    binding_ids: &[String],
) -> Vec<String> {
    let by_binding: BTreeMap<&str, &str> = station
        .assets
        .iter()
        .map(|asset| (asset.binding_id.as_str(), asset.asset_id.as_str()))
        .collect();
    binding_ids
        .iter()
        .filter_map(|binding_id| by_binding.get(binding_id.as_str()).copied())
        .map(str::to_owned)
        .collect()
}

fn normalize_definition(definition: &mut PlannedTestPreparationDefinition) {
    definition.schedule.title = definition.schedule.title.trim().to_owned();
    definition.schedule.assigned_operator = definition.schedule.assigned_operator.trim().to_owned();
    definition.schedule.laboratory_location_label = definition
        .schedule
        .laboratory_location_label
        .trim()
        .to_owned();
    definition.schedule.equipment_under_test =
        definition.schedule.equipment_under_test.trim().to_owned();
    definition.method.title = definition.method.title.trim().to_owned();
    definition.method.standard_references.sort();
    definition.method.standard_references.dedup();
    definition
        .method
        .instrumentation_chain
        .sort_by(|left, right| left.slot_id.cmp(&right.slot_id));
    for slot in &mut definition.method.instrumentation_chain {
        slot.label = slot.label.trim().to_owned();
        slot.depends_on_slots.sort();
        slot.depends_on_slots.dedup();
    }
    definition.station_setup.label = definition.station_setup.label.trim().to_owned();
    definition.station_setup.laboratory_location_label = definition
        .station_setup
        .laboratory_location_label
        .trim()
        .to_owned();
    definition
        .station_setup
        .assets
        .sort_by(|left, right| left.binding_id.cmp(&right.binding_id));
    for asset in &mut definition.station_setup.assets {
        asset.role_label = asset.role_label.trim().to_owned();
        asset.inventory_code = asset.inventory_code.trim().to_owned();
        asset.serial_number = asset.serial_number.trim().to_owned();
        asset.manufacturer = asset.manufacturer.trim().to_owned();
        asset.model_name = asset.model_name.trim().to_owned();
        asset.category_code = asset.category_code.trim().to_owned();
        asset.capabilities.sort_by(|left, right| {
            left.capability_id
                .cmp(&right.capability_id)
                .then_with(|| left.capability_kind.cmp(&right.capability_kind))
        });
    }
    definition
        .station_setup
        .corrections
        .sort_by(|left, right| left.selection_id.cmp(&right.selection_id));
    definition
        .assignments
        .sort_by(|left, right| left.slot_id.cmp(&right.slot_id));
    sort_readiness_issues(&mut definition.verdict.issues);
}

fn sort_readiness_issues(issues: &mut [PlannedTestPreparationIssue]) {
    for issue in issues.iter_mut() {
        issue.method_slot_ids.sort();
        issue.method_slot_ids.dedup();
        issue.binding_ids.sort();
        issue.binding_ids.dedup();
        issue.asset_ids.sort();
        issue.asset_ids.dedup();
    }
    issues.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then_with(|| left.dimension.cmp(&right.dimension))
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.method_slot_ids.cmp(&right.method_slot_ids))
            .then_with(|| left.binding_ids.cmp(&right.binding_ids))
            .then_with(|| left.asset_ids.cmp(&right.asset_ids))
    });
}

fn same_values(left: &[String], right: &[String]) -> bool {
    let mut left = left.to_vec();
    let mut right = right.to_vec();
    left.sort();
    right.sort();
    left == right
}

fn require_id(issues: &mut Vec<PlannedTestPreparationValidationIssue>, value: &str, path: &str) {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 160
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    {
        issues.push(validation_issue(
            "invalid_planned_test_preparation_identifier",
            path,
            "the value must be a stable identifier",
        ));
    }
}

fn require_text(issues: &mut Vec<PlannedTestPreparationValidationIssue>, value: &str, path: &str) {
    if value.trim().is_empty() {
        issues.push(validation_issue(
            "missing_planned_test_preparation_text",
            path,
            "a non-empty value is required",
        ));
    }
}

fn require_checksum(
    issues: &mut Vec<PlannedTestPreparationValidationIssue>,
    value: &str,
    path: &str,
) {
    let bytes = value.as_bytes();
    if bytes.len() != 71
        || !value.starts_with("sha256:")
        || !bytes[7..]
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        issues.push(validation_issue(
            "invalid_planned_test_preparation_checksum",
            path,
            "a canonical lowercase SHA-256 checksum is required",
        ));
    }
}

fn validation_issue(
    code: &str,
    path: &str,
    message: &str,
) -> PlannedTestPreparationValidationIssue {
    PlannedTestPreparationValidationIssue {
        code: code.to_owned(),
        path: path.to_owned(),
        message: message.to_owned(),
    }
}

fn canonicalize_json_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let source = std::mem::take(map);
            let mut entries: Vec<_> = source.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut ordered = Map::new();
            for (key, mut child) in entries {
                canonicalize_json_value(&mut child);
                ordered.insert(key, child);
            }
            *map = ordered;
        }
        Value::Array(values) => {
            for child in values {
                canonicalize_json_value(child);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::station_setup::{
        StationReadinessIssue, StationReadinessSeverity, StationSetupReadiness,
    };
    use crate::test_definitions::{CalibrationRequirement, InstrumentSubstitutionPolicy};

    fn checksum(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn input() -> PlannedTestPreparationAssessmentInput {
        PlannedTestPreparationAssessmentInput {
            schedule: PlannedTestScheduleSnapshot {
                project_code: "CEM-2026-001".to_owned(),
                item_code: "PLAN-001".to_owned(),
                revision: 2,
                title: "Émission conduite".to_owned(),
                planned_start_at: "2026-07-16T09:00".to_owned(),
                planned_end_at: "2026-07-16T12:00".to_owned(),
                assigned_operator: "Alice Martin".to_owned(),
                laboratory_location_id: Some("LAB-LOCATION-CEM-1".to_owned()),
                laboratory_location_label: "Poste CEM 1".to_owned(),
                equipment_under_test: "Calculateur Atlas".to_owned(),
                execution_mode: "investigation".to_owned(),
                status: ServiceScheduleStatus::Confirmed,
            },
            method: PreparedTestMethodSnapshot {
                template_id: "METHOD-COND-001".to_owned(),
                revision_id: "METHOD-COND-001-rev-0001".to_owned(),
                revision_number: 1,
                revision_status: TemplateRevisionStatus::Approved,
                definition_checksum: checksum('a'),
                title: "Émission conduite 150 kHz - 30 MHz".to_owned(),
                measurement_axis: MeasurementAxis::FrequencySweep,
                method_code: Some("CISPR-32-CE".to_owned()),
                method_revision: Some("2026".to_owned()),
                standard_references: vec!["CISPR 32".to_owned()],
                instrumentation_chain: vec![InstrumentationChainSlot {
                    slot_id: "receiver".to_owned(),
                    label: "Récepteur de mesure".to_owned(),
                    required_category: Some("emi_receiver".to_owned()),
                    required_capability: Some("spectrum_measurement".to_owned()),
                    required: true,
                    calibration_requirement: CalibrationRequirement::Required,
                    substitution_policy: InstrumentSubstitutionPolicy::SameCapability,
                    depends_on_slots: Vec::new(),
                }],
            },
            station_setup: PreparedStationSetupSnapshot {
                setup_id: "SETUP-COND-001".to_owned(),
                revision_id: "SETUP-COND-001-rev-0001".to_owned(),
                revision_number: 1,
                revision_status: StationSetupRevisionStatus::Ready,
                definition_checksum: checksum('b'),
                label: "Chaîne émission conduite".to_owned(),
                laboratory_location_id: Some("LAB-LOCATION-CEM-1".to_owned()),
                laboratory_location_label: "Poste CEM 1".to_owned(),
                planned_use_on: "2026-07-16".to_owned(),
                execution_mode: "investigation".to_owned(),
                assets: vec![PreparedStationAssetSnapshot {
                    binding_id: "receiver-binding".to_owned(),
                    role_label: "Récepteur".to_owned(),
                    asset_id: "ASSET-RX-001".to_owned(),
                    asset_revision: "rev-0001".to_owned(),
                    inventory_code: "INV-RX-001".to_owned(),
                    serial_number: "SN-RX-001".to_owned(),
                    manufacturer: "Rohde & Schwarz".to_owned(),
                    model_name: "ESW".to_owned(),
                    equipment_model_id: "MODEL-ESW".to_owned(),
                    equipment_model_revision_id: "MODEL-ESW-rev-0001".to_owned(),
                    equipment_model_checksum: checksum('c'),
                    category_code: "emi_receiver".to_owned(),
                    capabilities: vec![PreparedEquipmentCapabilitySnapshot {
                        capability_id: "spectrum_measurement".to_owned(),
                        label: "Mesure spectrale".to_owned(),
                        capability_kind: "frequency_spectrum".to_owned(),
                    }],
                }],
                corrections: Vec::new(),
            },
            assignments: vec![PlannedTestInstrumentAssignment {
                slot_id: "receiver".to_owned(),
                binding_id: "receiver-binding".to_owned(),
            }],
            station_readiness: StationSetupReadiness::from_issues("2026-07-16", Vec::new()),
        }
    }

    #[test]
    fn compatible_method_and_station_are_ready_and_permit_start() {
        let definition = assess_planned_test_preparation(input()).unwrap();

        assert!(definition.verdict.ready);
        assert!(definition.verdict.issues.is_empty());
        assert_eq!(
            definition.effective_state(2),
            PlannedTestPreparationState::Ready
        );
        assert!(definition.permits_start(2, ServiceScheduleStatus::Confirmed));
        assert!(!definition.permits_start(2, ServiceScheduleStatus::Planned));
    }

    #[test]
    fn missing_required_assignment_is_a_persistable_blocking_verdict() {
        let mut assessment = input();
        assessment.assignments.clear();

        let definition = assess_planned_test_preparation(assessment).unwrap();

        assert!(!definition.verdict.ready);
        assert_eq!(definition.verdict.issues.len(), 1);
        assert_eq!(
            definition.verdict.issues[0].code,
            "planned_test_required_role_unassigned"
        );
        assert_eq!(
            definition.verdict.issues[0].method_slot_ids,
            vec!["receiver"]
        );
    }

    #[test]
    fn category_and_capability_mismatches_name_the_material_and_role() {
        let mut assessment = input();
        assessment.station_setup.assets[0].category_code = "oscilloscope".to_owned();
        assessment.station_setup.assets[0].capabilities.clear();

        let definition = assess_planned_test_preparation(assessment).unwrap();

        assert!(!definition.verdict.ready);
        assert_eq!(definition.verdict.issues.len(), 2);
        assert!(definition
            .verdict
            .issues
            .iter()
            .all(|issue| issue.asset_ids == vec!["ASSET-RX-001"]));
        assert!(definition
            .verdict
            .issues
            .iter()
            .any(|issue| issue.code == "planned_test_role_category_mismatch"));
        assert!(definition
            .verdict
            .issues
            .iter()
            .any(|issue| issue.code == "planned_test_role_capability_mismatch"));
    }

    #[test]
    fn material_compatibility_explains_category_and_capability_mismatches() {
        let mut assessment = input();
        let mut wrong_category = assessment.station_setup.assets[0].clone();
        wrong_category.binding_id = "generator-binding".to_owned();
        wrong_category.asset_id = "ASSET-GEN-001".to_owned();
        wrong_category.inventory_code = "INV-GEN-001".to_owned();
        wrong_category.serial_number = "SN-GEN-001".to_owned();
        wrong_category.model_name = "SMW200A".to_owned();
        wrong_category.category_code = "rf_signal_generator".to_owned();
        assessment.station_setup.assets.push(wrong_category);
        let mut missing_capability = assessment.station_setup.assets[0].clone();
        missing_capability.binding_id = "receiver-without-spectrum-binding".to_owned();
        missing_capability.asset_id = "ASSET-RX-002".to_owned();
        missing_capability.inventory_code = "INV-RX-002".to_owned();
        missing_capability.serial_number = "SN-RX-002".to_owned();
        missing_capability.capabilities.clear();
        assessment.station_setup.assets.push(missing_capability);

        let compatibility = assess_planned_test_material_compatibility(
            &assessment.method,
            &assessment.station_setup,
            &assessment.station_readiness,
        );

        assert_eq!(compatibility.len(), 3);
        assert!(compatibility
            .iter()
            .any(|result| result.binding_id == "receiver-binding" && result.compatible));
        let category_rejection = compatibility
            .iter()
            .find(|result| result.binding_id == "generator-binding")
            .unwrap();
        assert!(!category_rejection.compatible);
        assert!(category_rejection
            .reason
            .as_deref()
            .unwrap()
            .contains("catégorie"));
        let capability_rejection = compatibility
            .iter()
            .find(|result| result.binding_id == "receiver-without-spectrum-binding")
            .unwrap();
        assert!(!capability_rejection.compatible);
        assert!(capability_rejection
            .reason
            .as_deref()
            .unwrap()
            .contains("capacité"));
    }

    #[test]
    fn material_compatibility_applies_serviceability_and_calibration_policy() {
        let mut assessment = input();
        assessment.station_readiness = StationSetupReadiness::from_issues(
            "2026-07-16",
            vec![StationReadinessIssue {
                code: "calibration_expired".to_owned(),
                severity: StationReadinessSeverity::Blocking,
                dimension: StationReadinessDimension::CalibrationValidity,
                message: "L'étalonnage requis est expiré à la date prévue.".to_owned(),
                binding_ids: vec!["receiver-binding".to_owned()],
                connection_ids: Vec::new(),
            }],
        );

        let required = assess_planned_test_material_compatibility(
            &assessment.method,
            &assessment.station_setup,
            &assessment.station_readiness,
        );
        assert!(!required[0].compatible);
        assert_eq!(
            required[0].reason.as_deref(),
            Some("L'étalonnage requis est expiré à la date prévue.")
        );

        assessment.method.instrumentation_chain[0].calibration_requirement =
            CalibrationRequirement::NotRequired;
        let not_required = assess_planned_test_material_compatibility(
            &assessment.method,
            &assessment.station_setup,
            &assessment.station_readiness,
        );
        assert!(not_required[0].compatible);

        assessment.station_readiness.issues[0].dimension =
            StationReadinessDimension::Serviceability;
        assessment.station_readiness.issues[0].message = "Le matériel est hors service.".to_owned();
        let out_of_service = assess_planned_test_material_compatibility(
            &assessment.method,
            &assessment.station_setup,
            &assessment.station_readiness,
        );
        assert!(!out_of_service[0].compatible);
    }

    #[test]
    fn optional_role_can_remain_unassigned() {
        let mut assessment = input();
        assessment.method.instrumentation_chain[0].required = false;
        assessment.assignments.clear();

        let definition = assess_planned_test_preparation(assessment).unwrap();

        assert!(definition.verdict.ready);
        assert!(definition.verdict.issues.is_empty());
    }

    #[test]
    fn schedule_context_mismatches_are_explicit() {
        let mut assessment = input();
        assessment.station_setup.planned_use_on = "2026-07-17".to_owned();
        assessment.station_setup.execution_mode = "accredited".to_owned();
        assessment.station_setup.laboratory_location_id = Some("LAB-LOCATION-CEM-2".to_owned());
        assessment.station_setup.laboratory_location_label = "Poste CEM 1".to_owned();
        assessment.station_readiness.checked_on = "2026-07-16".to_owned();

        let definition = assess_planned_test_preparation(assessment).unwrap();
        let codes: BTreeSet<_> = definition
            .verdict
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect();

        assert!(codes.contains("planned_test_station_date_mismatch"));
        assert!(codes.contains("planned_test_station_mode_mismatch"));
        assert!(codes.contains("planned_test_station_location_mismatch"));
    }

    #[test]
    fn location_identity_ignores_label_changes_and_blocks_missing_identity() {
        let mut renamed = input();
        renamed.station_setup.laboratory_location_label = "Poste CEM renommé".to_owned();
        let renamed = assess_planned_test_preparation(renamed).unwrap();
        assert!(renamed.verdict.ready);
        assert!(!renamed
            .verdict
            .issues
            .iter()
            .any(|issue| issue.code == "planned_test_station_location_mismatch"));

        let mut missing = input();
        missing.schedule.laboratory_location_id = None;
        let missing = assess_planned_test_preparation(missing).unwrap();
        assert!(!missing.verdict.ready);
        assert!(missing
            .verdict
            .issues
            .iter()
            .any(|issue| issue.code == "planned_test_location_identity_missing"));
    }

    #[test]
    fn station_metrology_issue_is_preserved_with_asset_context() {
        let mut assessment = input();
        assessment.station_readiness = StationSetupReadiness::from_issues(
            "2026-07-16",
            vec![StationReadinessIssue {
                code: "calibration_expired".to_owned(),
                severity: StationReadinessSeverity::Blocking,
                dimension: StationReadinessDimension::CalibrationValidity,
                message: "L'étalonnage requis est expiré à la date du montage.".to_owned(),
                binding_ids: vec!["receiver-binding".to_owned()],
                connection_ids: Vec::new(),
            }],
        );

        let definition = assess_planned_test_preparation(assessment).unwrap();

        assert!(!definition.verdict.ready);
        assert_eq!(
            definition.verdict.issues[0].dimension,
            PlannedTestPreparationDimension::CalibrationValidity
        );
        assert_eq!(definition.verdict.issues[0].asset_ids, vec!["ASSET-RX-001"]);
    }

    #[test]
    fn moving_the_schedule_makes_a_ready_preparation_stale() {
        let definition = assess_planned_test_preparation(input()).unwrap();

        assert_eq!(
            definition.effective_state(3),
            PlannedTestPreparationState::Stale
        );
        assert!(!definition.permits_start(3, ServiceScheduleStatus::Confirmed));
    }

    #[test]
    fn duplicate_or_unknown_assignments_are_rejected_structurally() {
        let mut duplicate = input();
        duplicate.assignments.push(duplicate.assignments[0].clone());
        assert!(assess_planned_test_preparation(duplicate)
            .unwrap_err()
            .iter()
            .any(|issue| issue.code == "duplicate_planned_test_assignment"));

        let mut unknown = input();
        unknown.assignments[0].binding_id = "outside-setup".to_owned();
        assert!(assess_planned_test_preparation(unknown)
            .unwrap_err()
            .iter()
            .any(|issue| issue.code == "planned_test_assignment_unknown_binding"));
    }

    #[test]
    fn canonical_json_and_checksum_ignore_collection_order() {
        let mut first = assess_planned_test_preparation(input()).unwrap();
        first
            .method
            .standard_references
            .extend(["EN 55032".to_owned(), "CISPR 32".to_owned()]);
        first.station_setup.assets[0]
            .capabilities
            .push(PreparedEquipmentCapabilitySnapshot {
                capability_id: "amplitude_measurement".to_owned(),
                label: "Mesure d'amplitude".to_owned(),
                capability_kind: "scalar".to_owned(),
            });
        let mut second = first.clone();
        second.method.standard_references.reverse();
        second.station_setup.assets[0].capabilities.reverse();

        let first = first.canonicalize().unwrap();
        let second = second.canonicalize().unwrap();

        assert_eq!(first.canonical_json, second.canonical_json);
        assert_eq!(first.definition_checksum, second.definition_checksum);
    }

    #[test]
    fn stored_verdict_must_match_its_blocking_issues_and_date() {
        let mut definition = assess_planned_test_preparation(input()).unwrap();
        definition.verdict.ready = false;
        definition.verdict.checked_on = "2026-07-17".to_owned();

        let issues = definition.canonicalize().unwrap_err();

        assert!(issues
            .iter()
            .any(|issue| issue.code == "planned_test_preparation_verdict_inconsistent"));
        assert!(issues
            .iter()
            .any(|issue| { issue.code == "planned_test_preparation_verdict_date_mismatch" }));
    }

    #[test]
    fn stored_verdict_cannot_omit_a_derived_required_role_issue() {
        let mut assessment = input();
        assessment.assignments.clear();
        let mut definition = assess_planned_test_preparation(assessment).unwrap();
        definition.verdict.issues.clear();
        definition.verdict.ready = true;

        let issues = definition.canonicalize().unwrap_err();

        assert!(issues
            .iter()
            .any(|issue| { issue.code == "planned_test_preparation_static_verdict_incomplete" }));
    }
}
