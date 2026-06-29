use crate::{
    audit::{AuditAction, AuditEvent},
    identifiers::{AuditActor, AuditReason, ProjectCode},
    quality::ExecutionMode,
    quality::{AuthorizedDeviation, ContractReviewChecklist},
    DomainError,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    code: ProjectCode,
    customer: String,
    stage: ProjectStage,
}

impl Project {
    pub fn new(code: ProjectCode, customer: impl Into<String>) -> Result<Self, DomainError> {
        let customer = customer.into();
        let customer = customer.trim();

        if customer.is_empty() {
            return Err(DomainError::EmptyCustomerName);
        }

        Ok(Self {
            code,
            customer: customer.to_owned(),
            stage: ProjectStage::Quotation,
        })
    }

    pub fn code(&self) -> &ProjectCode {
        &self.code
    }

    pub fn customer(&self) -> &str {
        &self.customer
    }

    pub fn stage(&self) -> ProjectStage {
        self.stage
    }

    pub fn advance_to(&mut self, next: ProjectStage) -> Result<(), DomainError> {
        if can_transition(self.stage, next) {
            self.stage = next;
            Ok(())
        } else {
            Err(DomainError::InvalidProjectTransition {
                from: self.stage,
                to: next,
            })
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectRecord {
    project: Project,
    audit_events: Vec<AuditEvent>,
    next_audit_sequence: u64,
}

impl ProjectRecord {
    pub fn open(project: Project, actor: AuditActor) -> Self {
        let project_code = project.code.clone();
        let audit_events = vec![AuditEvent::new(
            1,
            actor,
            project_code,
            AuditAction::ProjectCreated,
            None,
        )];

        Self {
            project,
            audit_events,
            next_audit_sequence: 2,
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn audit_events(&self) -> &[AuditEvent] {
        &self.audit_events
    }

    pub fn advance_to(
        &mut self,
        next: ProjectStage,
        actor: AuditActor,
        reason: AuditReason,
    ) -> Result<&AuditEvent, DomainError> {
        let from = self.project.stage();
        self.project.advance_to(next)?;

        Ok(self.append_audit_event(
            actor,
            AuditAction::ProjectStageAdvanced { from, to: next },
            Some(reason),
        ))
    }

    pub fn advance_to_test_planning(
        &mut self,
        checklist: &ContractReviewChecklist,
        actor: AuditActor,
        reason: AuditReason,
        deviation: Option<AuthorizedDeviation>,
    ) -> Result<&AuditEvent, DomainError> {
        self.advance_to_test_planning_for_mode(
            checklist,
            ExecutionMode::Accredited,
            actor,
            reason,
            deviation,
        )
    }

    pub fn advance_to_test_planning_for_mode(
        &mut self,
        checklist: &ContractReviewChecklist,
        execution_mode: ExecutionMode,
        actor: AuditActor,
        reason: AuditReason,
        deviation: Option<AuthorizedDeviation>,
    ) -> Result<&AuditEvent, DomainError> {
        if checklist.project() != self.project.code() {
            return Err(DomainError::ChecklistProjectMismatch {
                project: self.project.code().clone(),
                checklist_project: checklist.project().clone(),
            });
        }

        let from = self.project.stage();
        if !can_transition(from, ProjectStage::TestPlanning) {
            return Err(DomainError::InvalidProjectTransition {
                from,
                to: ProjectStage::TestPlanning,
            });
        }

        let missing_items = checklist.missing_items_for_mode(execution_mode);
        if !missing_items.is_empty() {
            let deviation = deviation.ok_or_else(|| DomainError::IncompleteContractReview {
                missing_items: missing_items.clone(),
            })?;

            self.append_audit_event(
                deviation.authorized_by().clone(),
                AuditAction::ContractReviewDeviationAuthorized { missing_items },
                Some(deviation.reason().clone()),
            );
        }

        self.advance_to(ProjectStage::TestPlanning, actor, reason)
    }

    fn append_audit_event(
        &mut self,
        actor: AuditActor,
        action: AuditAction,
        reason: Option<AuditReason>,
    ) -> &AuditEvent {
        let event = AuditEvent::new(
            self.next_audit_sequence,
            actor,
            self.project.code.clone(),
            action,
            reason,
        );
        self.next_audit_sequence += 1;
        self.audit_events.push(event);

        self.audit_events
            .last()
            .expect("audit event was just appended")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectStage {
    Quotation,
    ContractReview,
    TestPlanning,
    Measuring,
    TechnicalReview,
    ReportIssued,
    Archived,
}

pub fn can_transition(from: ProjectStage, to: ProjectStage) -> bool {
    use ProjectStage::*;

    matches!(
        (from, to),
        (Quotation, ContractReview)
            | (ContractReview, TestPlanning)
            | (TestPlanning, Measuring)
            | (Measuring, TechnicalReview)
            | (TechnicalReview, ReportIssued)
            | (ReportIssued, Archived)
    )
}
