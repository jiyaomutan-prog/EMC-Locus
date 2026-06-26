use crate::{
    identifiers::{AuditActor, AuditReason, ProjectCode},
    project::ProjectStage,
    quality::ContractReviewItem,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuditAction {
    ProjectCreated,
    ProjectStageAdvanced {
        from: ProjectStage,
        to: ProjectStage,
    },
    ContractReviewDeviationAuthorized {
        missing_items: Vec<ContractReviewItem>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEvent {
    sequence: u64,
    actor: AuditActor,
    project: ProjectCode,
    action: AuditAction,
    reason: Option<AuditReason>,
}

impl AuditEvent {
    pub(crate) fn new(
        sequence: u64,
        actor: AuditActor,
        project: ProjectCode,
        action: AuditAction,
        reason: Option<AuditReason>,
    ) -> Self {
        Self {
            sequence,
            actor,
            project,
            action,
            reason,
        }
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn actor(&self) -> &AuditActor {
        &self.actor
    }

    pub fn project(&self) -> &ProjectCode {
        &self.project
    }

    pub fn action(&self) -> &AuditAction {
        &self.action
    }

    pub fn reason(&self) -> Option<&AuditReason> {
        self.reason.as_ref()
    }
}
