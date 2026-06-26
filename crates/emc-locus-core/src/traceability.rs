use crate::identifiers::ProjectCode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceabilityRequirement {
    CustomerRequest,
    ContractReview,
    TestMethod,
    InstrumentIdentity,
    CalibrationStatus,
    EnvironmentalConditions,
    RawDataRetention,
    DataProcessingRecord,
    TechnicalReview,
    ReportApproval,
    AuditTrail,
}

pub fn baseline_traceability_requirements() -> Vec<TraceabilityRequirement> {
    use TraceabilityRequirement::*;

    vec![
        CustomerRequest,
        ContractReview,
        TestMethod,
        InstrumentIdentity,
        CalibrationStatus,
        EnvironmentalConditions,
        RawDataRetention,
        DataProcessingRecord,
        TechnicalReview,
        ReportApproval,
        AuditTrail,
    ]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CampaignTrace {
    project: ProjectCode,
    requirements: Vec<TraceabilityRequirement>,
}

impl CampaignTrace {
    pub fn new(project: ProjectCode) -> Self {
        Self {
            project,
            requirements: baseline_traceability_requirements(),
        }
    }

    pub fn project(&self) -> &ProjectCode {
        &self.project
    }

    pub fn requirements(&self) -> &[TraceabilityRequirement] {
        &self.requirements
    }

    pub fn is_baseline_complete(&self) -> bool {
        let baseline = baseline_traceability_requirements();
        baseline
            .iter()
            .all(|requirement| self.requirements.contains(requirement))
    }
}
