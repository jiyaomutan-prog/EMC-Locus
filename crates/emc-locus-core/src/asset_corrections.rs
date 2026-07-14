use crate::equipment::{
    AssetSpecificCorrectionPolicy, CorrectionRequirementDefinition,
    ModelDefaultCorrectionReference, NominalCorrectionQuality,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionSourceKind {
    Calibration,
    Characterization,
    Verification,
    ManufacturerCertificate,
    InternalMeasurement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetCorrectionAssignmentStatus {
    Draft,
    WaitingForReview,
    Approved,
    Active,
    Expired,
    Superseded,
    Rejected,
}

impl AssetCorrectionAssignmentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::WaitingForReview => "waiting_for_review",
            Self::Approved => "approved",
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Superseded => "superseded",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetCorrectionAssignment {
    pub assignment_id: String,
    pub asset_id: String,
    pub equipment_model_id: String,
    pub equipment_model_revision_id: String,
    pub equipment_model_checksum: String,
    pub signal_path_id: String,
    pub requirement_id: String,
    pub correction_definition_id: String,
    pub correction_revision_id: String,
    pub correction_checksum: String,
    pub source_event_id: String,
    pub source_kind: CorrectionSourceKind,
    pub valid_from: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<String>,
    pub status: AssetCorrectionAssignmentStatus,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub conditions: BTreeMap<String, String>,
    pub assigned_at: String,
    pub assigned_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetCorrectionValidationIssue {
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedCorrectionSource {
    AssetSpecific,
    ModelNominal,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrectionResolution {
    pub requirement_id: String,
    pub display_name: String,
    pub signal_path_id: String,
    pub selected_source: ResolvedCorrectionSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_definition_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_revision_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_checksum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<String>,
    pub reason: String,
    pub fallback_used: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    pub blocking: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetCorrectionResolutionReport {
    pub asset_id: String,
    pub intended_use_on: String,
    pub execution_context: String,
    pub ready: bool,
    pub resolutions: Vec<CorrectionResolution>,
}

pub fn validate_asset_correction_assignment(
    assignment: &AssetCorrectionAssignment,
) -> Vec<AssetCorrectionValidationIssue> {
    let mut issues = Vec::new();
    for (path, value) in [
        ("assignment_id", assignment.assignment_id.as_str()),
        ("asset_id", assignment.asset_id.as_str()),
        ("equipment_model_id", assignment.equipment_model_id.as_str()),
        (
            "equipment_model_revision_id",
            assignment.equipment_model_revision_id.as_str(),
        ),
        ("signal_path_id", assignment.signal_path_id.as_str()),
        ("requirement_id", assignment.requirement_id.as_str()),
        (
            "correction_definition_id",
            assignment.correction_definition_id.as_str(),
        ),
        (
            "correction_revision_id",
            assignment.correction_revision_id.as_str(),
        ),
        ("source_event_id", assignment.source_event_id.as_str()),
        ("assigned_by", assignment.assigned_by.as_str()),
    ] {
        if !valid_token(value) {
            issues.push(validation_issue(
                "invalid_asset_correction_identifier",
                path,
                "a non-empty machine-safe identifier is required",
            ));
        }
    }
    for (path, value) in [
        (
            "equipment_model_checksum",
            assignment.equipment_model_checksum.as_str(),
        ),
        (
            "correction_checksum",
            assignment.correction_checksum.as_str(),
        ),
    ] {
        if !valid_checksum(value) {
            issues.push(validation_issue(
                "invalid_asset_correction_checksum",
                path,
                "a canonical sha256 checksum is required",
            ));
        }
    }
    if !valid_date(&assignment.valid_from) {
        issues.push(validation_issue(
            "invalid_asset_correction_date",
            "valid_from",
            "valid_from must be a valid YYYY-MM-DD date",
        ));
    }
    if assignment
        .valid_until
        .as_deref()
        .is_some_and(|value| !valid_date(value))
    {
        issues.push(validation_issue(
            "invalid_asset_correction_date",
            "valid_until",
            "valid_until must be a valid YYYY-MM-DD date",
        ));
    }
    if assignment
        .valid_until
        .as_deref()
        .is_some_and(|value| value < assignment.valid_from.as_str())
    {
        issues.push(validation_issue(
            "invalid_asset_correction_validity",
            "valid_until",
            "valid_until must be on or after valid_from",
        ));
    }
    if assignment.status == AssetCorrectionAssignmentStatus::Draft
        && (assignment.submitted_at.is_some()
            || assignment.approved_at.is_some()
            || assignment.approved_by.is_some())
    {
        issues.push(validation_issue(
            "invalid_asset_correction_lifecycle",
            "status",
            "a draft correction cannot carry review or approval evidence",
        ));
    }
    if assignment.status == AssetCorrectionAssignmentStatus::WaitingForReview
        && assignment.submitted_at.is_none()
    {
        issues.push(validation_issue(
            "invalid_asset_correction_lifecycle",
            "submitted_at",
            "a correction waiting for review requires submitted_at",
        ));
    }
    if matches!(
        assignment.status,
        AssetCorrectionAssignmentStatus::Approved
            | AssetCorrectionAssignmentStatus::Active
            | AssetCorrectionAssignmentStatus::Expired
            | AssetCorrectionAssignmentStatus::Superseded
    ) && (assignment.approved_at.is_none() || assignment.approved_by.is_none())
    {
        issues.push(validation_issue(
            "invalid_asset_correction_lifecycle",
            "approved_at",
            "an approved or formerly active correction requires approval evidence",
        ));
    }
    if assignment.status == AssetCorrectionAssignmentStatus::Superseded
        && assignment.superseded_by.is_none()
    {
        issues.push(validation_issue(
            "invalid_asset_correction_lifecycle",
            "superseded_by",
            "a superseded correction must identify its replacement",
        ));
    }
    for (key, value) in &assignment.conditions {
        if !valid_token(key) || value.trim().is_empty() {
            issues.push(validation_issue(
                "invalid_asset_correction_condition",
                &format!("conditions.{key}"),
                "condition names and values must be explicit",
            ));
        }
    }
    issues
}

pub fn resolve_asset_corrections(
    asset_id: impl Into<String>,
    requirements: &[CorrectionRequirementDefinition],
    assignments: &[AssetCorrectionAssignment],
    intended_use_on: &str,
    execution_context: &str,
    requested_conditions: &BTreeMap<String, String>,
) -> AssetCorrectionResolutionReport {
    let mut applicable = requirements
        .iter()
        .filter(|requirement| requirement_applies(requirement, requested_conditions))
        .collect::<Vec<_>>();
    applicable.sort_by(|left, right| {
        left.signal_path_id
            .cmp(&right.signal_path_id)
            .then_with(|| left.requirement_id.cmp(&right.requirement_id))
    });
    let resolutions = applicable
        .into_iter()
        .map(|requirement| {
            resolve_requirement(
                requirement,
                assignments,
                intended_use_on,
                execution_context,
                requested_conditions,
            )
        })
        .collect::<Vec<_>>();
    AssetCorrectionResolutionReport {
        asset_id: asset_id.into(),
        intended_use_on: intended_use_on.to_owned(),
        execution_context: execution_context.to_owned(),
        ready: resolutions.iter().all(|resolution| !resolution.blocking),
        resolutions,
    }
}

fn resolve_requirement(
    requirement: &CorrectionRequirementDefinition,
    assignments: &[AssetCorrectionAssignment],
    intended_use_on: &str,
    execution_context: &str,
    requested_conditions: &BTreeMap<String, String>,
) -> CorrectionResolution {
    let mut matching = assignments
        .iter()
        .filter(|assignment| {
            assignment.requirement_id == requirement.requirement_id
                && assignment.signal_path_id == requirement.signal_path_id
                && assignment_conditions_match(assignment, requirement, requested_conditions)
        })
        .collect::<Vec<_>>();
    matching.sort_by(|left, right| {
        right
            .assigned_at
            .cmp(&left.assigned_at)
            .then_with(|| left.assignment_id.cmp(&right.assignment_id))
    });

    if requirement.asset_specific_policy != AssetSpecificCorrectionPolicy::ModelValueOnly {
        if let Some(assignment) = matching.iter().copied().find(|assignment| {
            assignment.status == AssetCorrectionAssignmentStatus::Active
                && assignment.valid_from.as_str() <= intended_use_on
                && assignment
                    .valid_until
                    .as_deref()
                    .is_none_or(|value| intended_use_on <= value)
        }) {
            return asset_resolution(requirement, assignment, "active_asset_correction");
        }
        if let Some(assignment) = matching.iter().copied().find(|assignment| {
            assignment.status == AssetCorrectionAssignmentStatus::Approved
                && assignment.valid_until.is_none()
                && assignment.valid_from.as_str() <= intended_use_on
        }) {
            return asset_resolution(
                requirement,
                assignment,
                "approved_asset_correction_without_expiry",
            );
        }
    }

    if let Some(reference) = requirement
        .model_default_reference
        .as_ref()
        .filter(|reference| model_reference_allowed(requirement, reference, execution_context))
    {
        let fallback_used =
            requirement.asset_specific_policy != AssetSpecificCorrectionPolicy::ModelValueOnly;
        return CorrectionResolution {
            requirement_id: requirement.requirement_id.clone(),
            display_name: requirement.display_name.clone(),
            signal_path_id: requirement.signal_path_id.clone(),
            selected_source: ResolvedCorrectionSource::ModelNominal,
            selected_definition_id: Some(reference.definition_id.clone()),
            selected_revision_id: Some(reference.revision_id.clone()),
            selected_checksum: Some(reference.definition_checksum.clone()),
            assignment_id: None,
            valid_from: None,
            valid_until: None,
            reason: if fallback_used {
                "model_nominal_fallback_allowed".to_owned()
            } else {
                "model_value_required_by_policy".to_owned()
            },
            fallback_used,
            warning: fallback_used.then(|| {
                "La valeur nominale du modèle sera utilisée avec avertissement.".to_owned()
            }),
            blocking: false,
        };
    }

    let reason = unavailable_reason(&matching, intended_use_on);
    CorrectionResolution {
        requirement_id: requirement.requirement_id.clone(),
        display_name: requirement.display_name.clone(),
        signal_path_id: requirement.signal_path_id.clone(),
        selected_source: ResolvedCorrectionSource::None,
        selected_definition_id: None,
        selected_revision_id: None,
        selected_checksum: None,
        assignment_id: None,
        valid_from: None,
        valid_until: None,
        reason: reason.to_owned(),
        fallback_used: false,
        warning: (!requirement.required_for_use).then(|| {
            format!(
                "La correction facultative « {} » n'est pas disponible.",
                requirement.display_name
            )
        }),
        blocking: requirement.required_for_use,
    }
}

fn asset_resolution(
    requirement: &CorrectionRequirementDefinition,
    assignment: &AssetCorrectionAssignment,
    reason: &str,
) -> CorrectionResolution {
    CorrectionResolution {
        requirement_id: requirement.requirement_id.clone(),
        display_name: requirement.display_name.clone(),
        signal_path_id: requirement.signal_path_id.clone(),
        selected_source: ResolvedCorrectionSource::AssetSpecific,
        selected_definition_id: Some(assignment.correction_definition_id.clone()),
        selected_revision_id: Some(assignment.correction_revision_id.clone()),
        selected_checksum: Some(assignment.correction_checksum.clone()),
        assignment_id: Some(assignment.assignment_id.clone()),
        valid_from: Some(assignment.valid_from.clone()),
        valid_until: assignment.valid_until.clone(),
        reason: reason.to_owned(),
        fallback_used: false,
        warning: None,
        blocking: false,
    }
}

fn requirement_applies(
    requirement: &CorrectionRequirementDefinition,
    requested_conditions: &BTreeMap<String, String>,
) -> bool {
    requested_conditions.is_empty()
        || requirement.conditions.iter().all(|(key, value)| {
            requested_conditions
                .get(key)
                .is_none_or(|requested| requested == value)
        })
}

fn assignment_conditions_match(
    assignment: &AssetCorrectionAssignment,
    requirement: &CorrectionRequirementDefinition,
    requested_conditions: &BTreeMap<String, String>,
) -> bool {
    requirement
        .conditions
        .iter()
        .all(|(key, value)| assignment.conditions.get(key) == Some(value))
        && assignment.conditions.iter().all(|(key, value)| {
            requirement.conditions.get(key) == Some(value)
                || requested_conditions.get(key) == Some(value)
        })
}

fn model_reference_allowed(
    requirement: &CorrectionRequirementDefinition,
    reference: &ModelDefaultCorrectionReference,
    execution_context: &str,
) -> bool {
    if requirement.asset_specific_policy == AssetSpecificCorrectionPolicy::AssetRequired {
        return false;
    }
    match reference.quality {
        NominalCorrectionQuality::SimulationOnly => execution_context == "simulation",
        _ if execution_context == "accredited" => {
            requirement.asset_specific_policy == AssetSpecificCorrectionPolicy::ModelValueOnly
        }
        _ => true,
    }
}

fn unavailable_reason(
    matching: &[&AssetCorrectionAssignment],
    intended_use_on: &str,
) -> &'static str {
    if matching.iter().any(|assignment| {
        assignment.status == AssetCorrectionAssignmentStatus::Active
            && assignment
                .valid_until
                .as_deref()
                .is_some_and(|value| value < intended_use_on)
    }) {
        "asset_correction_expired"
    } else if matching
        .iter()
        .any(|assignment| assignment.status == AssetCorrectionAssignmentStatus::WaitingForReview)
    {
        "asset_correction_waiting_for_review"
    } else if matching
        .iter()
        .any(|assignment| assignment.status == AssetCorrectionAssignmentStatus::Draft)
    {
        "asset_correction_draft"
    } else {
        "asset_correction_missing"
    }
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_checksum(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_date(value: &str) -> bool {
    if value.len() != 10 || &value[4..5] != "-" || &value[7..8] != "-" {
        return false;
    }
    let Ok(year) = value[0..4].parse::<u32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u32>() else {
        return false;
    };
    year > 0 && (1..=12).contains(&month) && (1..=31).contains(&day)
}

fn validation_issue(code: &str, path: &str, message: &str) -> AssetCorrectionValidationIssue {
    AssetCorrectionValidationIssue {
        code: code.to_owned(),
        path: path.to_owned(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equipment::{
        CorrectionApplicationOperation, CorrectionRequirementKind, PhysicalQuantity,
    };

    const CHECKSUM: &str =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn cable_requires_an_active_serial_specific_loss_curve() {
        let requirement = requirement(AssetSpecificCorrectionPolicy::AssetRequired, None);
        let report = resolve_asset_corrections(
            "CBL-014",
            std::slice::from_ref(&requirement),
            &[],
            "2026-07-15",
            "accredited",
            &BTreeMap::new(),
        );
        assert!(!report.ready);
        assert_eq!(report.resolutions[0].reason, "asset_correction_missing");

        let report = resolve_asset_corrections(
            "CBL-014",
            &[requirement],
            &[assignment("CBL-014", "loss", BTreeMap::new())],
            "2026-07-15",
            "accredited",
            &BTreeMap::new(),
        );
        assert!(report.ready);
        assert_eq!(
            report.resolutions[0].selected_source,
            ResolvedCorrectionSource::AssetSpecific
        );
    }

    #[test]
    fn nominal_value_is_only_a_visible_fallback_when_policy_allows() {
        let nominal = ModelDefaultCorrectionReference {
            correction_kind: CorrectionRequirementKind::RawSignalConversion,
            definition_id: "CURRENT-PROBE-10MV-A".to_owned(),
            revision_id: "CURRENT-PROBE-10MV-A-rev-0001".to_owned(),
            definition_checksum: CHECKSUM.to_owned(),
            quality: NominalCorrectionQuality::ManufacturerNominal,
        };
        let requirement = requirement(AssetSpecificCorrectionPolicy::AssetPreferred, Some(nominal));
        let report = resolve_asset_corrections(
            "CP-003",
            &[requirement],
            &[],
            "2026-07-15",
            "non_accredited",
            &BTreeMap::new(),
        );
        assert!(report.ready);
        assert!(report.resolutions[0].fallback_used);
        assert_eq!(
            report.resolutions[0].selected_source,
            ResolvedCorrectionSource::ModelNominal
        );
    }

    #[test]
    fn antenna_polarization_selects_only_the_matching_requirement() {
        let mut horizontal = requirement(AssetSpecificCorrectionPolicy::AssetRequired, None);
        horizontal.requirement_id = "antenna_factor_h".to_owned();
        horizontal
            .conditions
            .insert("polarization".to_owned(), "horizontal".to_owned());
        let mut vertical = horizontal.clone();
        vertical.requirement_id = "antenna_factor_v".to_owned();
        vertical
            .conditions
            .insert("polarization".to_owned(), "vertical".to_owned());
        let mut context = BTreeMap::new();
        context.insert("polarization".to_owned(), "vertical".to_owned());
        let report = resolve_asset_corrections(
            "ANT-003",
            &[horizontal, vertical],
            &[],
            "2026-07-15",
            "accredited",
            &context,
        );
        assert_eq!(report.resolutions.len(), 1);
        assert_eq!(report.resolutions[0].requirement_id, "antenna_factor_v");
    }

    #[test]
    fn expired_asset_correction_does_not_override_nominal_fallback() {
        let nominal = ModelDefaultCorrectionReference {
            correction_kind: CorrectionRequirementKind::RawSignalConversion,
            definition_id: "ACC-100MV-G".to_owned(),
            revision_id: "ACC-100MV-G-rev-0001".to_owned(),
            definition_checksum: CHECKSUM.to_owned(),
            quality: NominalCorrectionQuality::ManufacturerNominal,
        };
        let requirement = requirement(AssetSpecificCorrectionPolicy::AssetPreferred, Some(nominal));
        let mut expired = assignment("ACC-003", "sensitivity", BTreeMap::new());
        expired.valid_until = Some("2026-06-30".to_owned());
        let report = resolve_asset_corrections(
            "ACC-003",
            &[requirement],
            &[expired],
            "2026-07-15",
            "non_accredited",
            &BTreeMap::new(),
        );
        assert!(report.ready);
        assert!(report.resolutions[0].fallback_used);
    }

    #[test]
    fn assignment_validation_rejects_lifecycle_and_checksum_mismatches() {
        let mut assignment = assignment("CBL-014", "loss", BTreeMap::new());
        assignment.correction_checksum = "sha256:ABC".to_owned();
        assignment.approved_at = None;
        let issues = validate_asset_correction_assignment(&assignment);
        assert!(issues
            .iter()
            .any(|issue| issue.code == "invalid_asset_correction_checksum"));
        assert!(issues
            .iter()
            .any(|issue| issue.code == "invalid_asset_correction_lifecycle"));
    }

    #[test]
    fn calibrated_accelerometer_sensitivity_overrides_the_nominal_model_value() {
        let nominal = ModelDefaultCorrectionReference {
            correction_kind: CorrectionRequirementKind::RawSignalConversion,
            definition_id: "ACC-NOMINAL-100MV-G".to_owned(),
            revision_id: "ACC-NOMINAL-100MV-G-rev-0001".to_owned(),
            definition_checksum: CHECKSUM.to_owned(),
            quality: NominalCorrectionQuality::ManufacturerNominal,
        };
        let mut requirement =
            requirement(AssetSpecificCorrectionPolicy::AssetPreferred, Some(nominal));
        requirement.requirement_id = "sensitivity".to_owned();
        requirement.display_name = "Sensibilite de l'accelerometre".to_owned();
        let report = resolve_asset_corrections(
            "ACC-003",
            &[requirement],
            &[assignment("ACC-003", "sensitivity", BTreeMap::new())],
            "2026-07-15",
            "non_accredited",
            &BTreeMap::new(),
        );
        assert!(report.ready);
        assert_eq!(
            report.resolutions[0].selected_source,
            ResolvedCorrectionSource::AssetSpecific
        );
        assert!(!report.resolutions[0].fallback_used);
    }

    #[test]
    fn approved_correction_without_expiry_is_selected_deterministically() {
        let requirement = requirement(AssetSpecificCorrectionPolicy::AssetRequired, None);
        let mut older = assignment("CBL-014", "loss", BTreeMap::new());
        older.assignment_id = "ASSIGN-CBL-014-OLDER".to_owned();
        older.status = AssetCorrectionAssignmentStatus::Approved;
        older.valid_until = None;
        older.assigned_at = "2026-01-01T10:00:00Z".to_owned();
        let mut newer = older.clone();
        newer.assignment_id = "ASSIGN-CBL-014-NEWER".to_owned();
        newer.correction_definition_id = "CHAR-CBL-014-NEWER".to_owned();
        newer.assigned_at = "2026-06-01T10:00:00Z".to_owned();

        let report = resolve_asset_corrections(
            "CBL-014",
            &[requirement],
            &[older, newer],
            "2026-07-15",
            "accredited",
            &BTreeMap::new(),
        );
        assert!(report.ready);
        assert_eq!(
            report.resolutions[0].assignment_id.as_deref(),
            Some("ASSIGN-CBL-014-NEWER")
        );
        assert_eq!(
            report.resolutions[0].reason,
            "approved_asset_correction_without_expiry"
        );
    }

    #[test]
    fn antenna_correction_is_selected_for_the_requested_polarization() {
        let mut horizontal = requirement(AssetSpecificCorrectionPolicy::AssetRequired, None);
        horizontal.requirement_id = "antenna_factor_h".to_owned();
        horizontal
            .conditions
            .insert("polarization".to_owned(), "horizontal".to_owned());
        let mut vertical = horizontal.clone();
        vertical.requirement_id = "antenna_factor_v".to_owned();
        vertical
            .conditions
            .insert("polarization".to_owned(), "vertical".to_owned());
        let mut vertical_conditions = BTreeMap::new();
        vertical_conditions.insert("polarization".to_owned(), "vertical".to_owned());
        let mut vertical_assignment =
            assignment("ANT-003", "antenna_factor_v", vertical_conditions.clone());
        vertical_assignment.assignment_id = "ASSIGN-ANT-003-V".to_owned();

        let report = resolve_asset_corrections(
            "ANT-003",
            &[horizontal, vertical],
            &[vertical_assignment],
            "2026-07-15",
            "accredited",
            &vertical_conditions,
        );
        assert!(report.ready);
        assert_eq!(report.resolutions.len(), 1);
        assert_eq!(report.resolutions[0].requirement_id, "antenna_factor_v");
        assert_eq!(
            report.resolutions[0].selected_source,
            ResolvedCorrectionSource::AssetSpecific
        );
    }

    #[test]
    fn missing_optional_correction_does_not_block_readiness() {
        let mut requirement = requirement(AssetSpecificCorrectionPolicy::AssetRequired, None);
        requirement.required_for_use = false;
        let report = resolve_asset_corrections(
            "CBL-014",
            &[requirement],
            &[],
            "2026-07-15",
            "accredited",
            &BTreeMap::new(),
        );
        assert!(report.ready);
        assert!(!report.resolutions[0].blocking);
        assert!(report.resolutions[0].warning.is_some());
    }

    fn requirement(
        policy: AssetSpecificCorrectionPolicy,
        nominal: Option<ModelDefaultCorrectionReference>,
    ) -> CorrectionRequirementDefinition {
        CorrectionRequirementDefinition {
            requirement_id: "loss".to_owned(),
            display_name: "Pertes du câble".to_owned(),
            description: "Compense les pertes du câble réel".to_owned(),
            signal_path_id: "rf_through".to_owned(),
            correction_kind: nominal.as_ref().map_or(
                CorrectionRequirementKind::FrequencyDependentCorrection,
                |value| value.correction_kind,
            ),
            physical_purpose: "cable_loss".to_owned(),
            operation: CorrectionApplicationOperation::Subtract,
            input_quantity: PhysicalQuantity::Power,
            output_quantity: PhysicalQuantity::Power,
            expected_unit: "dB".to_owned(),
            required_for_use: true,
            asset_specific_policy: policy,
            model_default_reference: nominal,
            conditions: BTreeMap::new(),
        }
    }

    fn assignment(
        asset_id: &str,
        requirement_id: &str,
        conditions: BTreeMap<String, String>,
    ) -> AssetCorrectionAssignment {
        AssetCorrectionAssignment {
            assignment_id: format!("ASSIGN-{asset_id}-{requirement_id}"),
            asset_id: asset_id.to_owned(),
            equipment_model_id: "EQM-DEMO".to_owned(),
            equipment_model_revision_id: "EQM-DEMO-rev-0001".to_owned(),
            equipment_model_checksum: CHECKSUM.to_owned(),
            signal_path_id: "rf_through".to_owned(),
            requirement_id: requirement_id.to_owned(),
            correction_definition_id: format!("CHAR-{asset_id}"),
            correction_revision_id: "immutable:1".to_owned(),
            correction_checksum: CHECKSUM.to_owned(),
            source_event_id: format!("CHAR-{asset_id}"),
            source_kind: CorrectionSourceKind::Characterization,
            valid_from: "2026-07-01".to_owned(),
            valid_until: Some("2027-07-01".to_owned()),
            status: AssetCorrectionAssignmentStatus::Active,
            conditions,
            assigned_at: "2026-07-01T12:00:00Z".to_owned(),
            assigned_by: "metrology.admin".to_owned(),
            submitted_at: Some("2026-07-01T12:01:00Z".to_owned()),
            approved_at: Some("2026-07-01T12:02:00Z".to_owned()),
            approved_by: Some("metrology.reviewer".to_owned()),
            superseded_by: None,
        }
    }
}
