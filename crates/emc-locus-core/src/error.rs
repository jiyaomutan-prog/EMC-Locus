use crate::{identifiers::ProjectCode, project::ProjectStage, quality::ContractReviewItem};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainError {
    EmptyProjectCode,
    InvalidProjectCode(String),
    EmptyCustomerName,
    EmptyAuditActor,
    EmptyAuditReason,
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
