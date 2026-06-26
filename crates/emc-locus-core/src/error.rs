use crate::{identifiers::ProjectCode, project::ProjectStage, quality::ContractReviewItem};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainError {
    EmptyProjectCode,
    InvalidProjectCode(String),
    EmptyCustomerName,
    EmptyAuditActor,
    EmptyAuditReason,
    EmptyInstrumentCode,
    InvalidInstrumentCode(String),
    EmptyInstrumentManufacturer,
    EmptyInstrumentModel,
    EmptyInstrumentSerialNumber,
    EmptyCalibrationCertificate,
    EmptyCalibrationProvider,
    InvalidMetrologyDate {
        year: u16,
        month: u8,
        day: u8,
    },
    InvalidCalibrationPeriod,
    DuplicateInstrumentCode(String),
    UnknownInstrumentCode(String),
    InvalidProjectTransition {
        from: ProjectStage,
        to: ProjectStage,
    },
    ChecklistProjectMismatch {
        project: ProjectCode,
        checklist_project: ProjectCode,
    },
    IncompleteContractReview {
        missing_items: Vec<ContractReviewItem>,
    },
}
