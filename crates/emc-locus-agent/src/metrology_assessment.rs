use crate::AgentError;
use emc_locus_core::metrology::{
    assess_metrology, CalibrationDecision, CalibrationRequirement, MetrologyAssessment,
    MetrologyAssessmentReasonCode, MetrologyAssessmentStatus, MetrologyDate,
    DEFAULT_CALIBRATION_DUE_SOON_WARNING_DAYS,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MetrologyAssessmentSource {
    pub(crate) calibration_requirement: String,
    pub(crate) calibration_period_months: Option<u32>,
    pub(crate) calibration_due_warning_days: u32,
    pub(crate) calibrated_at: Option<String>,
    pub(crate) due_at: Option<String>,
    pub(crate) decision: Option<String>,
    pub(crate) latest_calibration_event_id: Option<String>,
    pub(crate) latest_calibration_revision: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MetrologyStatusSummaryDto {
    #[serde(flatten)]
    pub(crate) assessment: MetrologyAssessment,
    pub(crate) calibration_period_months: Option<u32>,
    pub(crate) explanation: String,
    pub(crate) latest_calibration_event_id: Option<String>,
    pub(crate) latest_calibration_revision: Option<String>,
}

impl MetrologyStatusSummaryDto {
    pub(crate) fn unavailable(checked_on: MetrologyDate) -> Self {
        from_assessment(
            MetrologyAssessment::unavailable(checked_on, DEFAULT_CALIBRATION_DUE_SOON_WARNING_DAYS),
            None,
            None,
            None,
        )
    }
}

pub(crate) fn assess_metrology_source(
    checked_on: MetrologyDate,
    source: MetrologyAssessmentSource,
) -> MetrologyStatusSummaryDto {
    let Some(requirement) = calibration_requirement(&source.calibration_requirement) else {
        return MetrologyStatusSummaryDto::unavailable(checked_on);
    };
    let decision = source.decision.as_deref().map(calibration_decision);
    let calibrated_at = source
        .calibrated_at
        .as_deref()
        .and_then(|value| MetrologyDate::parse_iso(value).ok());
    let due_at = source
        .due_at
        .as_deref()
        .and_then(|value| MetrologyDate::parse_iso(value).ok());
    let assessment = assess_metrology(
        checked_on,
        requirement,
        decision,
        calibrated_at,
        due_at,
        source.calibration_due_warning_days,
    );
    from_assessment(
        assessment,
        source.calibration_period_months,
        source.latest_calibration_event_id,
        source.latest_calibration_revision,
    )
}

pub(crate) fn parse_checked_on(
    value: &str,
    field: &'static str,
) -> Result<MetrologyDate, AgentError> {
    MetrologyDate::parse_iso(value).map_err(|_| {
        AgentError::new(
            "invalid_metrology_date",
            format!("{field} must use YYYY-MM-DD"),
        )
    })
}

pub(crate) fn metrology_status_code(status: MetrologyAssessmentStatus) -> &'static str {
    match status {
        MetrologyAssessmentStatus::Valid => "valid",
        MetrologyAssessmentStatus::DueSoon => "due_soon",
        MetrologyAssessmentStatus::Expired => "expired",
        MetrologyAssessmentStatus::Missing => "missing",
        MetrologyAssessmentStatus::NotRequired => "not_required",
        MetrologyAssessmentStatus::Nonconforming => "nonconforming",
        MetrologyAssessmentStatus::Indeterminate => "indeterminate",
        MetrologyAssessmentStatus::Unavailable => "unavailable",
    }
}

pub(crate) fn metrology_reason_code(reason: MetrologyAssessmentReasonCode) -> &'static str {
    match reason {
        MetrologyAssessmentReasonCode::CalibrationValid => "calibration_valid",
        MetrologyAssessmentReasonCode::CalibrationDueSoon => "calibration_due_soon",
        MetrologyAssessmentReasonCode::CalibrationExpired => "calibration_expired",
        MetrologyAssessmentReasonCode::CalibrationMissing => "calibration_missing",
        MetrologyAssessmentReasonCode::CalibrationNotRequired => "calibration_not_required",
        MetrologyAssessmentReasonCode::CalibrationNonconforming => "calibration_nonconforming",
        MetrologyAssessmentReasonCode::CalibrationDecisionIndeterminate => {
            "calibration_decision_indeterminate"
        }
        MetrologyAssessmentReasonCode::MetrologySourceUnavailable => "metrology_source_unavailable",
    }
}

fn from_assessment(
    assessment: MetrologyAssessment,
    calibration_period_months: Option<u32>,
    latest_calibration_event_id: Option<String>,
    latest_calibration_revision: Option<String>,
) -> MetrologyStatusSummaryDto {
    let explanation = assessment_explanation(&assessment);
    MetrologyStatusSummaryDto {
        assessment,
        calibration_period_months,
        explanation,
        latest_calibration_event_id,
        latest_calibration_revision,
    }
}

fn calibration_requirement(value: &str) -> Option<CalibrationRequirement> {
    match value {
        "required" => Some(CalibrationRequirement::Required),
        "conditional" => Some(CalibrationRequirement::Conditional),
        "not_required" => Some(CalibrationRequirement::NotRequired),
        _ => None,
    }
}

fn calibration_decision(value: &str) -> CalibrationDecision {
    match value {
        "conforming" => CalibrationDecision::Conforming,
        "nonconforming" => CalibrationDecision::Nonconforming,
        "indeterminate" | "not_assessed" => CalibrationDecision::Indeterminate,
        _ => CalibrationDecision::Indeterminate,
    }
}

fn assessment_explanation(assessment: &MetrologyAssessment) -> String {
    match assessment.status {
        MetrologyAssessmentStatus::Valid => format!(
            "Étalonnage valide jusqu’au {}.",
            assessment
                .due_at
                .expect("a valid assessment has a due date")
        ),
        MetrologyAssessmentStatus::DueSoon => format!(
            "Échéance d’étalonnage proche : {}.",
            assessment
                .due_at
                .expect("a due-soon assessment has a due date")
        ),
        MetrologyAssessmentStatus::Expired => format!(
            "Étalonnage expiré depuis le {}.",
            assessment
                .due_at
                .expect("an expired assessment has a due date")
        ),
        MetrologyAssessmentStatus::Missing => "Aucun étalonnage valide.".to_owned(),
        MetrologyAssessmentStatus::NotRequired => "Étalonnage non requis.".to_owned(),
        MetrologyAssessmentStatus::Nonconforming => "Dernier étalonnage non conforme.".to_owned(),
        MetrologyAssessmentStatus::Indeterminate => {
            "Décision métrologique indéterminée.".to_owned()
        }
        MetrologyAssessmentStatus::Unavailable => {
            "Métrologie temporairement indisponible.".to_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(
        decision: Option<&str>,
        calibrated_at: Option<&str>,
        due_at: Option<&str>,
    ) -> MetrologyAssessmentSource {
        MetrologyAssessmentSource {
            calibration_requirement: "required".to_owned(),
            calibration_period_months: Some(12),
            calibration_due_warning_days: 30,
            calibrated_at: calibrated_at.map(str::to_owned),
            due_at: due_at.map(str::to_owned),
            decision: decision.map(str::to_owned),
            latest_calibration_event_id: Some("CAL-1".to_owned()),
            latest_calibration_revision: Some("rev-1".to_owned()),
        }
    }

    #[test]
    fn dated_assessment_covers_boundary_and_decision_states() {
        let checked_on = MetrologyDate::parse_iso("2026-07-27").unwrap();
        for (input, expected) in [
            (
                source(Some("conforming"), Some("2025-08-31"), Some("2026-08-31")),
                MetrologyAssessmentStatus::Valid,
            ),
            (
                source(Some("conforming"), Some("2025-08-01"), Some("2026-08-01")),
                MetrologyAssessmentStatus::DueSoon,
            ),
            (
                source(Some("conforming"), Some("2025-07-27"), Some("2026-07-27")),
                MetrologyAssessmentStatus::DueSoon,
            ),
            (
                source(Some("conforming"), Some("2025-07-26"), Some("2026-07-26")),
                MetrologyAssessmentStatus::Expired,
            ),
            (source(None, None, None), MetrologyAssessmentStatus::Missing),
            (
                source(
                    Some("nonconforming"),
                    Some("2026-01-01"),
                    Some("2027-01-01"),
                ),
                MetrologyAssessmentStatus::Nonconforming,
            ),
            (
                source(
                    Some("indeterminate"),
                    Some("2026-01-01"),
                    Some("2027-01-01"),
                ),
                MetrologyAssessmentStatus::Indeterminate,
            ),
        ] {
            assert_eq!(
                assess_metrology_source(checked_on, input).assessment.status,
                expected
            );
        }
    }

    #[test]
    fn invalid_calibration_dates_are_indeterminate() {
        let summary = assess_metrology_source(
            MetrologyDate::parse_iso("2026-07-27").unwrap(),
            source(Some("conforming"), Some("not-a-date"), Some("2027-01-01")),
        );
        assert_eq!(
            summary.assessment.status,
            MetrologyAssessmentStatus::Indeterminate
        );
        assert!(summary.assessment.blocking);
    }

    #[test]
    fn not_required_and_unavailable_are_distinct_non_temporal_states() {
        let checked_on = MetrologyDate::parse_iso("2026-07-27").unwrap();
        let mut not_required = source(None, None, None);
        not_required.calibration_requirement = "not_required".to_owned();
        let not_required = assess_metrology_source(checked_on, not_required);
        assert_eq!(
            not_required.assessment.status,
            MetrologyAssessmentStatus::NotRequired
        );
        assert!(!not_required.assessment.blocking);

        let unavailable = MetrologyStatusSummaryDto::unavailable(checked_on);
        assert_eq!(
            unavailable.assessment.status,
            MetrologyAssessmentStatus::Unavailable
        );
        assert!(unavailable.assessment.blocking);
    }
}
