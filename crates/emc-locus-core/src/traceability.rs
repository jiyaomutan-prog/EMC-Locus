use crate::{
    datasets::{
        DatasetChecksum, DatasetFileReference, DatasetKind, DatasetReference,
        MeasurementRunEvidence,
    },
    identifiers::{AuditActor, ProjectCode},
    measurement::{MeasurementRunReference, TestMethodReference},
    metrology::InstrumentCode,
    reporting::{ReportExportBundle, ReportNumber, ReportRevision},
    DomainError,
};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceabilityDatasetView {
    run: MeasurementRunReference,
    reference: DatasetReference,
    kind: DatasetKind,
    checksum: DatasetChecksum,
    immutable: bool,
}

impl TraceabilityDatasetView {
    fn from_evidence(dataset: &crate::datasets::RawDatasetRecord) -> Self {
        Self {
            run: dataset.run().clone(),
            reference: dataset.reference().clone(),
            kind: dataset.kind(),
            checksum: dataset.checksum().clone(),
            immutable: dataset.immutable(),
        }
    }

    pub fn run(&self) -> &MeasurementRunReference {
        &self.run
    }

    pub fn reference(&self) -> &DatasetReference {
        &self.reference
    }

    pub fn kind(&self) -> DatasetKind {
        self.kind
    }

    pub fn checksum(&self) -> &DatasetChecksum {
        &self.checksum
    }

    pub fn immutable(&self) -> bool {
        self.immutable
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceabilityRunView {
    run: MeasurementRunReference,
    method: TestMethodReference,
    equipment: Vec<InstrumentCode>,
    observation_count: usize,
    raw_datasets: Vec<TraceabilityDatasetView>,
}

impl TraceabilityRunView {
    fn from_evidence(evidence: &MeasurementRunEvidence) -> Self {
        Self {
            run: evidence.plan().reference().clone(),
            method: evidence.plan().method().clone(),
            equipment: evidence.plan().equipment().to_vec(),
            observation_count: evidence.observations().len(),
            raw_datasets: evidence
                .raw_datasets()
                .iter()
                .map(TraceabilityDatasetView::from_evidence)
                .collect(),
        }
    }

    pub fn run(&self) -> &MeasurementRunReference {
        &self.run
    }

    pub fn method(&self) -> &TestMethodReference {
        &self.method
    }

    pub fn equipment(&self) -> &[InstrumentCode] {
        &self.equipment
    }

    pub fn observation_count(&self) -> usize {
        self.observation_count
    }

    pub fn raw_datasets(&self) -> &[TraceabilityDatasetView] {
        &self.raw_datasets
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceabilityReportView {
    project: ProjectCode,
    report_number: ReportNumber,
    report_revision: ReportRevision,
    export_file_reference: DatasetFileReference,
    export_checksum: DatasetChecksum,
    reviewed_by: Option<AuditActor>,
    approved_by: Option<AuditActor>,
    runs: Vec<TraceabilityRunView>,
    requirements: Vec<TraceabilityRequirement>,
}

impl TraceabilityReportView {
    pub fn from_export_bundle(
        bundle: &ReportExportBundle,
        evidence: &[MeasurementRunEvidence],
    ) -> Result<Self, DomainError> {
        for run_evidence in evidence {
            if run_evidence.plan().project() != bundle.project() {
                return Err(DomainError::TraceabilityProjectMismatch {
                    expected: bundle.project().as_str().to_owned(),
                    actual: run_evidence.plan().project().as_str().to_owned(),
                });
            }
        }

        Ok(Self {
            project: bundle.project().clone(),
            report_number: bundle.number().clone(),
            report_revision: bundle.revision().clone(),
            export_file_reference: bundle.file_reference().clone(),
            export_checksum: bundle.checksum().clone(),
            reviewed_by: bundle.reviewed_by().cloned(),
            approved_by: bundle.approved_by().cloned(),
            runs: evidence
                .iter()
                .map(TraceabilityRunView::from_evidence)
                .collect(),
            requirements: baseline_traceability_requirements(),
        })
    }

    pub fn project(&self) -> &ProjectCode {
        &self.project
    }

    pub fn report_number(&self) -> &ReportNumber {
        &self.report_number
    }

    pub fn report_revision(&self) -> &ReportRevision {
        &self.report_revision
    }

    pub fn export_file_reference(&self) -> &DatasetFileReference {
        &self.export_file_reference
    }

    pub fn export_checksum(&self) -> &DatasetChecksum {
        &self.export_checksum
    }

    pub fn reviewed_by(&self) -> Option<&AuditActor> {
        self.reviewed_by.as_ref()
    }

    pub fn approved_by(&self) -> Option<&AuditActor> {
        self.approved_by.as_ref()
    }

    pub fn runs(&self) -> &[TraceabilityRunView] {
        &self.runs
    }

    pub fn requirements(&self) -> &[TraceabilityRequirement] {
        &self.requirements
    }

    pub fn has_raw_data_lineage(&self) -> bool {
        !self.runs.is_empty() && self.runs.iter().all(|run| !run.raw_datasets().is_empty())
    }

    pub fn has_technical_review(&self) -> bool {
        self.reviewed_by.is_some()
    }

    pub fn has_report_approval(&self) -> bool {
        self.approved_by.is_some()
    }
}
