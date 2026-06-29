use crate::{
    identifiers::{AuditActor, AuditReason, ProjectCode},
    project::{ProjectRecord, ProjectStage},
    quality::{AuthorizedDeviation, ContractReviewChecklist, ExecutionMode},
    DomainError,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdvanceProjectStageCommand {
    project_code: ProjectCode,
    actor: AuditActor,
    reason: AuditReason,
    target_stage: ProjectStage,
    execution_mode: ExecutionMode,
    deviation: Option<AuthorizedDeviation>,
}

impl AdvanceProjectStageCommand {
    pub fn new(
        project_code: ProjectCode,
        actor: AuditActor,
        reason: AuditReason,
        target_stage: ProjectStage,
        execution_mode: ExecutionMode,
    ) -> Self {
        Self {
            project_code,
            actor,
            reason,
            target_stage,
            execution_mode,
            deviation: None,
        }
    }

    pub fn with_deviation(mut self, deviation: AuthorizedDeviation) -> Self {
        self.deviation = Some(deviation);
        self
    }

    pub fn project_code(&self) -> &ProjectCode {
        &self.project_code
    }

    pub fn actor(&self) -> &AuditActor {
        &self.actor
    }

    pub fn reason(&self) -> &AuditReason {
        &self.reason
    }

    pub fn target_stage(&self) -> ProjectStage {
        self.target_stage
    }

    pub fn execution_mode(&self) -> ExecutionMode {
        self.execution_mode
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationWriteKind {
    ProjectStageAdvance,
    ContractReviewDeviation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplicationWriteReceipt {
    project_code: ProjectCode,
    resulting_stage: ProjectStage,
    audit_event_count: usize,
    writes: Vec<ApplicationWriteKind>,
}

impl ApplicationWriteReceipt {
    pub fn project_code(&self) -> &ProjectCode {
        &self.project_code
    }

    pub fn resulting_stage(&self) -> ProjectStage {
        self.resulting_stage
    }

    pub fn audit_event_count(&self) -> usize {
        self.audit_event_count
    }

    pub fn writes(&self) -> &[ApplicationWriteKind] {
        &self.writes
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectApplicationService;

impl ProjectApplicationService {
    pub fn new() -> Self {
        Self
    }

    pub fn advance_stage(
        &self,
        record: &mut ProjectRecord,
        command: AdvanceProjectStageCommand,
        checklist: Option<&ContractReviewChecklist>,
    ) -> Result<ApplicationWriteReceipt, DomainError> {
        if record.project().code() != command.project_code() {
            return Err(DomainError::ApplicationProjectMismatch {
                expected: record.project().code().clone(),
                actual: command.project_code().clone(),
            });
        }

        let audit_count_before = record.audit_events().len();
        if command.target_stage() == ProjectStage::TestPlanning {
            let checklist = checklist.ok_or_else(|| DomainError::MissingApplicationChecklist {
                project: command.project_code().clone(),
            })?;
            let deviation = command.deviation.clone();
            record.advance_to_test_planning_for_mode(
                checklist,
                command.execution_mode,
                command.actor.clone(),
                command.reason.clone(),
                deviation,
            )?;
        } else {
            record.advance_to(
                command.target_stage(),
                command.actor.clone(),
                command.reason.clone(),
            )?;
        }

        let audit_event_count = record.audit_events().len();
        let mut writes = vec![ApplicationWriteKind::ProjectStageAdvance];
        if audit_event_count > audit_count_before + 1 {
            writes.insert(0, ApplicationWriteKind::ContractReviewDeviation);
        }

        Ok(ApplicationWriteReceipt {
            project_code: command.project_code,
            resulting_stage: record.project().stage(),
            audit_event_count,
            writes,
        })
    }
}
