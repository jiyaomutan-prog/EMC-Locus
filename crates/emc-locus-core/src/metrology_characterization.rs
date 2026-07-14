use crate::{
    measurement_engineering::{
        validate_engineering_curve_definition, validate_scaling_profile_definition,
        EngineeringCurveDefinition, ScalingProfileDefinition,
    },
    DefinitionValidationIssue, SignalTransformationKind, SignalTransformationReference,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const ASSET_CHARACTERIZATION_DEFINITION_SCHEMA_VERSION: &str =
    "emc-locus.asset-characterization-definition.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetCharacterizationKind {
    TimeConversion,
    FrequencyResponse,
}

impl AssetCharacterizationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TimeConversion => "time_conversion",
            Self::FrequencyResponse => "frequency_response",
        }
    }

    pub fn transformation_kind(self) -> SignalTransformationKind {
        match self {
            Self::TimeConversion => SignalTransformationKind::SampleConversion,
            Self::FrequencyResponse => SignalTransformationKind::FrequencyResponse,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "correction_kind", rename_all = "snake_case")]
pub enum AssetCorrectionDefinition {
    TimeConversion {
        correction: ScalingProfileDefinition,
    },
    FrequencyResponse {
        correction: EngineeringCurveDefinition,
    },
}

impl AssetCorrectionDefinition {
    pub fn kind(&self) -> AssetCharacterizationKind {
        match self {
            Self::TimeConversion { .. } => AssetCharacterizationKind::TimeConversion,
            Self::FrequencyResponse { .. } => AssetCharacterizationKind::FrequencyResponse,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CharacterizationUncertainty {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expanded_uncertainty: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_level_percent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssetCharacterizationDefinition {
    pub definition_schema_version: String,
    pub characterization_id: String,
    pub asset_id: String,
    pub label: String,
    pub correction: AssetCorrectionDefinition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_correction_reference: Option<SignalTransformationReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty: Option<CharacterizationUncertainty>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub conditions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalAssetCharacterizationDefinition {
    pub characterization_id: String,
    pub asset_id: String,
    pub kind: AssetCharacterizationKind,
    pub label: String,
    pub definition_schema_version: String,
    pub canonical_json: String,
    pub definition_checksum: String,
}

impl AssetCharacterizationDefinition {
    pub fn from_json_str(value: &str) -> Result<Self, DefinitionValidationIssue> {
        serde_json::from_str(value).map_err(|error| {
            issue(
                "invalid_asset_characterization_json",
                "$",
                format!("invalid asset characterization definition: {error}"),
                None,
            )
        })
    }

    pub fn validate_all(&self) -> Vec<DefinitionValidationIssue> {
        validate_asset_characterization_definition(self)
    }

    pub fn canonicalize(
        &self,
    ) -> Result<CanonicalAssetCharacterizationDefinition, Vec<DefinitionValidationIssue>> {
        let issues = self.validate_all();
        if !issues.is_empty() {
            return Err(issues);
        }

        let mut value = serde_json::to_value(self).map_err(|error| {
            vec![issue(
                "asset_characterization_serialization_failed",
                "$",
                error.to_string(),
                None,
            )]
        })?;
        canonicalize_json_value(&mut value);
        let canonical_json = serde_json::to_string(&value).map_err(|error| {
            vec![issue(
                "asset_characterization_serialization_failed",
                "$",
                error.to_string(),
                None,
            )]
        })?;
        let digest = Sha256::digest(canonical_json.as_bytes());
        Ok(CanonicalAssetCharacterizationDefinition {
            characterization_id: self.characterization_id.clone(),
            asset_id: self.asset_id.clone(),
            kind: self.correction.kind(),
            label: self.label.trim().to_owned(),
            definition_schema_version: self.definition_schema_version.clone(),
            canonical_json,
            definition_checksum: format!("sha256:{digest:x}"),
        })
    }
}

pub fn validate_asset_characterization_definition(
    definition: &AssetCharacterizationDefinition,
) -> Vec<DefinitionValidationIssue> {
    let mut issues = Vec::new();
    if definition.definition_schema_version != ASSET_CHARACTERIZATION_DEFINITION_SCHEMA_VERSION {
        issues.push(issue(
            "unsupported_asset_characterization_schema",
            "definition_schema_version",
            "unsupported asset characterization definition schema",
            Some(format!(
                "Use {ASSET_CHARACTERIZATION_DEFINITION_SCHEMA_VERSION}."
            )),
        ));
    }
    require_stable_id(
        &mut issues,
        &definition.characterization_id,
        "characterization_id",
    );
    require_stable_id(&mut issues, &definition.asset_id, "asset_id");
    if definition.label.trim().is_empty() {
        issues.push(issue(
            "missing_asset_characterization_label",
            "label",
            "a characterization name is required",
            Some(
                "Name the measured correction so it can be recognized in the material record."
                    .to_owned(),
            ),
        ));
    }

    match &definition.correction {
        AssetCorrectionDefinition::TimeConversion { correction } => {
            if correction.scaling_profile_id != definition.characterization_id {
                issues.push(issue(
                    "asset_characterization_identity_mismatch",
                    "correction.scaling_profile_id",
                    "the time conversion must use the characterization identity",
                    None,
                ));
            }
            append_nested_issues(
                &mut issues,
                "correction",
                validate_scaling_profile_definition(correction),
            );
        }
        AssetCorrectionDefinition::FrequencyResponse { correction } => {
            if correction.curve_id != definition.characterization_id {
                issues.push(issue(
                    "asset_characterization_identity_mismatch",
                    "correction.curve_id",
                    "the frequency response must use the characterization identity",
                    None,
                ));
            }
            append_nested_issues(
                &mut issues,
                "correction",
                validate_engineering_curve_definition(correction),
            );
            if correction.points.len() < 2 {
                issues.push(issue(
                    "asset_frequency_response_requires_two_points",
                    "correction.points",
                    "a material frequency response requires at least two measured frequencies",
                    Some("Add the lower and upper measured frequencies.".to_owned()),
                ));
            }
        }
    }

    if let Some(reference) = &definition.model_correction_reference {
        if reference.transformation_kind != definition.correction.kind().transformation_kind() {
            issues.push(issue(
                "model_correction_kind_mismatch",
                "model_correction_reference.transformation_kind",
                "the model correction and material characterization must use the same signal representation",
                None,
            ));
        }
        require_stable_id(
            &mut issues,
            &reference.entity_id,
            "model_correction_reference.entity_id",
        );
        require_stable_id(
            &mut issues,
            &reference.revision_id,
            "model_correction_reference.revision_id",
        );
        if !is_canonical_sha256(&reference.definition_checksum) {
            issues.push(issue(
                "invalid_model_correction_checksum",
                "model_correction_reference.definition_checksum",
                "the model correction checksum must use canonical sha256 form",
                None,
            ));
        }
    }

    if let Some(uncertainty) = &definition.uncertainty {
        validate_uncertainty(&mut issues, uncertainty);
    }
    issues
}

fn validate_uncertainty(
    issues: &mut Vec<DefinitionValidationIssue>,
    uncertainty: &CharacterizationUncertainty,
) {
    if uncertainty.expanded_uncertainty.is_none()
        && (uncertainty.unit.is_some()
            || uncertainty.coverage_factor.is_some()
            || uncertainty.confidence_level_percent.is_some())
    {
        issues.push(issue(
            "incomplete_characterization_uncertainty",
            "uncertainty.expanded_uncertainty",
            "expanded uncertainty is required when its unit, coverage factor, or confidence level is provided",
            None,
        ));
    }
    if let Some(value) = uncertainty.expanded_uncertainty {
        if !value.is_finite() || value < 0.0 {
            issues.push(issue(
                "invalid_characterization_uncertainty",
                "uncertainty.expanded_uncertainty",
                "expanded uncertainty must be a finite non-negative value",
                None,
            ));
        }
        if uncertainty
            .unit
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        {
            issues.push(issue(
                "missing_characterization_uncertainty_unit",
                "uncertainty.unit",
                "an uncertainty unit is required when a numeric uncertainty is provided",
                None,
            ));
        }
    }
    if let Some(value) = uncertainty.coverage_factor {
        if !value.is_finite() || value <= 0.0 {
            issues.push(issue(
                "invalid_characterization_coverage_factor",
                "uncertainty.coverage_factor",
                "coverage factor must be a finite positive value",
                None,
            ));
        }
    }
    if let Some(value) = uncertainty.confidence_level_percent {
        if !value.is_finite() || !(0.0..=100.0).contains(&value) {
            issues.push(issue(
                "invalid_characterization_confidence_level",
                "uncertainty.confidence_level_percent",
                "confidence level must be between 0 and 100 percent",
                None,
            ));
        }
    }
}

fn append_nested_issues(
    issues: &mut Vec<DefinitionValidationIssue>,
    prefix: &str,
    nested: Vec<DefinitionValidationIssue>,
) {
    issues.extend(nested.into_iter().map(|mut item| {
        item.path = if item.path == "$" {
            prefix.to_owned()
        } else {
            format!("{prefix}.{}", item.path)
        };
        item
    }));
}

fn require_stable_id(issues: &mut Vec<DefinitionValidationIssue>, value: &str, path: &str) {
    let valid = !value.trim().is_empty()
        && value == value.trim()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'));
    if !valid {
        issues.push(issue(
            "invalid_asset_characterization_identifier",
            path,
            "identifier must use letters, digits, hyphen, underscore, or dot",
            None,
        ));
    }
}

fn is_canonical_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn issue(
    code: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
    suggestion: Option<String>,
) -> DefinitionValidationIssue {
    DefinitionValidationIssue {
        severity: "error".to_owned(),
        code: code.into(),
        path: path.into(),
        message: message.into(),
        suggestion,
    }
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
    use crate::measurement_engineering::{
        CorrectionOperation, CurveAxis, CurveAxisDefinition, CurveInterpolation,
        CurveValueDefinition, EngineeringCurvePoint, EngineeringCurveType, ExtrapolationPolicy,
        FrequencyResponseComponent, ScalingKind, ScalingParameters, SignalRepresentation,
        ENGINEERING_CURVE_DEFINITION_SCHEMA_VERSION,
    };
    use crate::PhysicalQuantity;

    fn frequency_definition() -> AssetCharacterizationDefinition {
        AssetCharacterizationDefinition {
            definition_schema_version: ASSET_CHARACTERIZATION_DEFINITION_SCHEMA_VERSION.to_owned(),
            characterization_id: "char-cable-001".to_owned(),
            asset_id: "SA-CABLE-001".to_owned(),
            label: "Pertes mesurées du câble".to_owned(),
            correction: AssetCorrectionDefinition::FrequencyResponse {
                correction: EngineeringCurveDefinition {
                    definition_schema_version: ENGINEERING_CURVE_DEFINITION_SCHEMA_VERSION
                        .to_owned(),
                    curve_id: "char-cable-001".to_owned(),
                    curve_type: EngineeringCurveType::CableLoss,
                    label: "Pertes mesurées du câble".to_owned(),
                    signal_representation: SignalRepresentation::FrequencyDomainSpectrum,
                    independent_axes: vec![CurveAxisDefinition {
                        axis: CurveAxis::Frequency,
                        quantity: PhysicalQuantity::Frequency,
                        unit: "Hz".to_owned(),
                    }],
                    dependent_values: vec![CurveValueDefinition {
                        value_id: "amplitude".to_owned(),
                        quantity: PhysicalQuantity::Dimensionless,
                        unit: "dB".to_owned(),
                        component: FrequencyResponseComponent::Amplitude,
                        operation: CorrectionOperation::Add,
                    }],
                    units: BTreeMap::new(),
                    points: vec![
                        EngineeringCurvePoint {
                            axis_values: BTreeMap::from([("frequency".to_owned(), 1.0e6)]),
                            values: BTreeMap::from([("amplitude".to_owned(), 0.4)]),
                        },
                        EngineeringCurvePoint {
                            axis_values: BTreeMap::from([("frequency".to_owned(), 1.0e9)]),
                            values: BTreeMap::from([("amplitude".to_owned(), 3.2)]),
                        },
                    ],
                    interpolation: CurveInterpolation::LogXLinearY,
                    extrapolation_policy: ExtrapolationPolicy::Forbidden,
                    validity_domain: BTreeMap::new(),
                    conditions: BTreeMap::new(),
                    source_document_reference: None,
                    source_checksum: None,
                    status: None,
                    metadata: BTreeMap::new(),
                },
            },
            model_correction_reference: None,
            uncertainty: Some(CharacterizationUncertainty {
                expanded_uncertainty: Some(0.2),
                unit: Some("dB".to_owned()),
                coverage_factor: Some(2.0),
                confidence_level_percent: Some(95.0),
                statement: None,
            }),
            conditions: BTreeMap::new(),
        }
    }

    #[test]
    fn canonicalizes_frequency_characterization_deterministically() {
        let first = frequency_definition().canonicalize().unwrap();
        let second = frequency_definition().canonicalize().unwrap();

        assert_eq!(first.canonical_json, second.canonical_json);
        assert_eq!(first.definition_checksum, second.definition_checksum);
        assert_eq!(first.kind, AssetCharacterizationKind::FrequencyResponse);
    }

    #[test]
    fn rejects_mismatched_nested_identity() {
        let mut definition = frequency_definition();
        let AssetCorrectionDefinition::FrequencyResponse { correction } =
            &mut definition.correction
        else {
            unreachable!()
        };
        correction.curve_id = "other".to_owned();

        assert!(definition
            .validate_all()
            .iter()
            .any(|issue| issue.code == "asset_characterization_identity_mismatch"));
    }

    #[test]
    fn rejects_uncertainty_without_unit() {
        let mut definition = frequency_definition();
        definition.uncertainty.as_mut().unwrap().unit = None;

        assert!(definition
            .validate_all()
            .iter()
            .any(|issue| issue.code == "missing_characterization_uncertainty_unit"));
    }

    #[test]
    fn rejects_uncertainty_metadata_without_numeric_value() {
        let mut definition = frequency_definition();
        definition
            .uncertainty
            .as_mut()
            .unwrap()
            .expanded_uncertainty = None;

        assert!(definition
            .validate_all()
            .iter()
            .any(|issue| issue.code == "incomplete_characterization_uncertainty"));
    }

    #[test]
    fn rejects_frequency_response_with_single_point() {
        let mut definition = frequency_definition();
        let AssetCorrectionDefinition::FrequencyResponse { correction } =
            &mut definition.correction
        else {
            unreachable!()
        };
        correction.points.truncate(1);

        assert!(definition
            .validate_all()
            .iter()
            .any(|issue| issue.code == "asset_frequency_response_requires_two_points"));
    }

    #[test]
    fn accepts_linear_time_conversion() {
        use crate::measurement_engineering::SCALING_PROFILE_DEFINITION_SCHEMA_VERSION;

        let definition = AssetCharacterizationDefinition {
            definition_schema_version: ASSET_CHARACTERIZATION_DEFINITION_SCHEMA_VERSION.to_owned(),
            characterization_id: "char-probe-001".to_owned(),
            asset_id: "SA-PROBE-001".to_owned(),
            label: "Sensibilité mesurée".to_owned(),
            correction: AssetCorrectionDefinition::TimeConversion {
                correction: ScalingProfileDefinition {
                    definition_schema_version: SCALING_PROFILE_DEFINITION_SCHEMA_VERSION.to_owned(),
                    scaling_profile_id: "char-probe-001".to_owned(),
                    label: "Sensibilité mesurée".to_owned(),
                    input_quantity: PhysicalQuantity::Voltage,
                    input_unit: "V".to_owned(),
                    output_quantity: PhysicalQuantity::ElectricField,
                    output_unit: "V_per_meter".to_owned(),
                    signal_representation: SignalRepresentation::TimeDomainSamples,
                    scaling_kind: ScalingKind::Linear,
                    parameters: ScalingParameters {
                        scale: Some(2.0),
                        offset: Some(0.0),
                        ..ScalingParameters::default()
                    },
                    input_limits: None,
                    validity_domain: BTreeMap::new(),
                    uncertainty: None,
                    source_reference: None,
                    metadata: BTreeMap::new(),
                },
            },
            model_correction_reference: None,
            uncertainty: None,
            conditions: BTreeMap::new(),
        };

        assert!(definition.validate_all().is_empty());
    }
}
