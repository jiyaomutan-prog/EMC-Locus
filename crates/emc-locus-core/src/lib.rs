//! Core domain primitives for EMC Locus.
//!
//! This crate should stay independent from UI, database, and instrument-driver
//! details. It captures business rules that must remain stable across adapters.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectCode(String);

impl ProjectCode {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyProjectCode);
        }

        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(DomainError::InvalidProjectCode(trimmed.to_owned()));
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditActor(String);

impl AuditActor {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyAuditActor);
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditReason(String);

impl AuditReason {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyAuditReason);
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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

        let missing_items = checklist.missing_items();
        if !missing_items.is_empty() {
            let deviation =
                deviation.ok_or_else(|| DomainError::IncompleteContractReview {
                    missing_items: missing_items.clone(),
                })?;

            self.append_audit_event(
                deviation.authorized_by,
                AuditAction::ContractReviewDeviationAuthorized {
                    missing_items,
                },
                Some(deviation.reason),
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
    fn new(
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractReviewItem {
    CustomerRequestDefined,
    TestMethodSelected,
    LaboratoryCapabilityConfirmed,
    EquipmentAvailabilityChecked,
    CalibrationStatusReviewed,
    ImpartialityRisksReviewed,
    DataRetentionAgreed,
    ReportRequirementsAgreed,
    DeviationsRecorded,
}

pub fn baseline_contract_review_items() -> Vec<ContractReviewItem> {
    use ContractReviewItem::*;

    vec![
        CustomerRequestDefined,
        TestMethodSelected,
        LaboratoryCapabilityConfirmed,
        EquipmentAvailabilityChecked,
        CalibrationStatusReviewed,
        ImpartialityRisksReviewed,
        DataRetentionAgreed,
        ReportRequirementsAgreed,
        DeviationsRecorded,
    ]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractReviewChecklist {
    project: ProjectCode,
    completed_items: Vec<ContractReviewItem>,
}

impl ContractReviewChecklist {
    pub fn new(project: ProjectCode) -> Self {
        Self {
            project,
            completed_items: Vec::new(),
        }
    }

    pub fn project(&self) -> &ProjectCode {
        &self.project
    }

    pub fn completed_items(&self) -> &[ContractReviewItem] {
        &self.completed_items
    }

    pub fn mark_complete(&mut self, item: ContractReviewItem) {
        if !self.completed_items.contains(&item) {
            self.completed_items.push(item);
        }
    }

    pub fn missing_items(&self) -> Vec<ContractReviewItem> {
        baseline_contract_review_items()
            .into_iter()
            .filter(|item| !self.completed_items.contains(item))
            .collect()
    }

    pub fn is_complete(&self) -> bool {
        baseline_contract_review_items()
            .iter()
            .all(|item| self.completed_items.contains(item))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizedDeviation {
    authorized_by: AuditActor,
    reason: AuditReason,
}

impl AuthorizedDeviation {
    pub fn new(authorized_by: AuditActor, reason: AuditReason) -> Self {
        Self {
            authorized_by,
            reason,
        }
    }

    pub fn authorized_by(&self) -> &AuditActor {
        &self.authorized_by
    }

    pub fn reason(&self) -> &AuditReason {
        &self.reason
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_code_rejects_empty_values() {
        let error = ProjectCode::parse("   ").unwrap_err();
        assert_eq!(error, DomainError::EmptyProjectCode);
    }

    #[test]
    fn project_code_accepts_lab_friendly_identifiers() {
        let code = ProjectCode::parse("CEM-2026_001.A").unwrap();
        assert_eq!(code.as_str(), "CEM-2026_001.A");
    }

    #[test]
    fn audit_actor_rejects_empty_values() {
        let error = AuditActor::parse(" ").unwrap_err();
        assert_eq!(error, DomainError::EmptyAuditActor);
    }

    #[test]
    fn audit_reason_rejects_empty_values() {
        let error = AuditReason::parse("\t").unwrap_err();
        assert_eq!(error, DomainError::EmptyAuditReason);
    }

    #[test]
    fn project_stages_follow_the_campaign_lifecycle() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let mut project = Project::new(code, "Example Customer").unwrap();

        assert_eq!(project.stage(), ProjectStage::Quotation);
        project.advance_to(ProjectStage::ContractReview).unwrap();
        project.advance_to(ProjectStage::TestPlanning).unwrap();
        project.advance_to(ProjectStage::Measuring).unwrap();
        project.advance_to(ProjectStage::TechnicalReview).unwrap();
        project.advance_to(ProjectStage::ReportIssued).unwrap();
        project.advance_to(ProjectStage::Archived).unwrap();
    }

    #[test]
    fn project_stages_reject_skipped_review_points() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let mut project = Project::new(code, "Example Customer").unwrap();

        let error = project.advance_to(ProjectStage::Measuring).unwrap_err();
        assert_eq!(
            error,
            DomainError::InvalidProjectTransition {
                from: ProjectStage::Quotation,
                to: ProjectStage::Measuring,
            }
        );
    }

    #[test]
    fn project_record_opens_with_a_creation_audit_event() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let project = Project::new(code.clone(), "Example Customer").unwrap();
        let actor = AuditActor::parse("quality.manager").unwrap();

        let record = ProjectRecord::open(project, actor.clone());
        let event = &record.audit_events()[0];

        assert_eq!(record.audit_events().len(), 1);
        assert_eq!(event.sequence(), 1);
        assert_eq!(event.actor(), &actor);
        assert_eq!(event.project(), &code);
        assert_eq!(event.action(), &AuditAction::ProjectCreated);
        assert_eq!(event.reason(), None);
    }

    #[test]
    fn project_record_records_stage_transition_audit_events() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let project = Project::new(code.clone(), "Example Customer").unwrap();
        let actor = AuditActor::parse("quality.manager").unwrap();
        let reason = AuditReason::parse("Contract review approved").unwrap();
        let mut record = ProjectRecord::open(project, actor.clone());

        let event = record
            .advance_to(ProjectStage::ContractReview, actor.clone(), reason.clone())
            .unwrap()
            .clone();

        assert_eq!(record.project().stage(), ProjectStage::ContractReview);
        assert_eq!(record.audit_events().len(), 2);
        assert_eq!(event.sequence(), 2);
        assert_eq!(event.actor(), &actor);
        assert_eq!(event.project(), &code);
        assert_eq!(
            event.action(),
            &AuditAction::ProjectStageAdvanced {
                from: ProjectStage::Quotation,
                to: ProjectStage::ContractReview,
            }
        );
        assert_eq!(event.reason(), Some(&reason));
    }

    #[test]
    fn project_record_rejects_skipped_stages_without_audit_side_effects() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let project = Project::new(code, "Example Customer").unwrap();
        let actor = AuditActor::parse("quality.manager").unwrap();
        let reason = AuditReason::parse("Operator tried to skip planning").unwrap();
        let mut record = ProjectRecord::open(project, actor.clone());

        let error = record.advance_to(ProjectStage::Measuring, actor, reason).unwrap_err();

        assert_eq!(
            error,
            DomainError::InvalidProjectTransition {
                from: ProjectStage::Quotation,
                to: ProjectStage::Measuring,
            }
        );
        assert_eq!(record.project().stage(), ProjectStage::Quotation);
        assert_eq!(record.audit_events().len(), 1);
    }

    #[test]
    fn contract_review_checklist_starts_with_all_items_missing() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let checklist = ContractReviewChecklist::new(code.clone());

        assert_eq!(checklist.project(), &code);
        assert!(!checklist.is_complete());
        assert_eq!(
            checklist.missing_items(),
            baseline_contract_review_items()
        );
    }

    #[test]
    fn contract_review_checklist_does_not_duplicate_completed_items() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let mut checklist = ContractReviewChecklist::new(code);

        checklist.mark_complete(ContractReviewItem::CustomerRequestDefined);
        checklist.mark_complete(ContractReviewItem::CustomerRequestDefined);

        assert_eq!(checklist.completed_items().len(), 1);
    }

    #[test]
    fn contract_review_checklist_can_be_completed() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let mut checklist = ContractReviewChecklist::new(code);

        for item in baseline_contract_review_items() {
            checklist.mark_complete(item);
        }

        assert!(checklist.is_complete());
        assert!(checklist.missing_items().is_empty());
    }

    #[test]
    fn test_planning_requires_complete_contract_review() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let project = Project::new(code.clone(), "Example Customer").unwrap();
        let actor = AuditActor::parse("quality.manager").unwrap();
        let mut record = ProjectRecord::open(project, actor.clone());
        record
            .advance_to(
                ProjectStage::ContractReview,
                actor.clone(),
                AuditReason::parse("Quote accepted").unwrap(),
            )
            .unwrap();
        let checklist = ContractReviewChecklist::new(code);
        let audit_count_before = record.audit_events().len();

        let error = record
            .advance_to_test_planning(
                &checklist,
                actor,
                AuditReason::parse("Planning requested").unwrap(),
                None,
            )
            .unwrap_err();

        assert_eq!(
            error,
            DomainError::IncompleteContractReview {
                missing_items: baseline_contract_review_items(),
            }
        );
        assert_eq!(record.project().stage(), ProjectStage::ContractReview);
        assert_eq!(record.audit_events().len(), audit_count_before);
    }

    #[test]
    fn complete_contract_review_allows_test_planning() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let project = Project::new(code.clone(), "Example Customer").unwrap();
        let actor = AuditActor::parse("quality.manager").unwrap();
        let mut record = ProjectRecord::open(project, actor.clone());
        record
            .advance_to(
                ProjectStage::ContractReview,
                actor.clone(),
                AuditReason::parse("Quote accepted").unwrap(),
            )
            .unwrap();
        let mut checklist = ContractReviewChecklist::new(code.clone());
        for item in baseline_contract_review_items() {
            checklist.mark_complete(item);
        }

        let event = record
            .advance_to_test_planning(
                &checklist,
                actor.clone(),
                AuditReason::parse("Contract review complete").unwrap(),
                None,
            )
            .unwrap()
            .clone();

        assert_eq!(record.project().stage(), ProjectStage::TestPlanning);
        assert_eq!(record.audit_events().len(), 3);
        assert_eq!(
            event.action(),
            &AuditAction::ProjectStageAdvanced {
                from: ProjectStage::ContractReview,
                to: ProjectStage::TestPlanning,
            }
        );
        assert_eq!(event.project(), &code);
        assert_eq!(event.actor(), &actor);
    }

    #[test]
    fn authorized_deviation_allows_incomplete_contract_review_to_reach_planning() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let project = Project::new(code.clone(), "Example Customer").unwrap();
        let actor = AuditActor::parse("quality.manager").unwrap();
        let mut record = ProjectRecord::open(project, actor.clone());
        record
            .advance_to(
                ProjectStage::ContractReview,
                actor.clone(),
                AuditReason::parse("Quote accepted").unwrap(),
            )
            .unwrap();
        let checklist = ContractReviewChecklist::new(code.clone());
        let deviation_reason =
            AuditReason::parse("Quality manager accepted documented planning risk").unwrap();
        let deviation = AuthorizedDeviation::new(actor.clone(), deviation_reason.clone());

        let event = record
            .advance_to_test_planning(
                &checklist,
                actor.clone(),
                AuditReason::parse("Planning authorized with deviation").unwrap(),
                Some(deviation),
            )
            .unwrap()
            .clone();

        assert_eq!(record.project().stage(), ProjectStage::TestPlanning);
        assert_eq!(record.audit_events().len(), 4);

        let deviation_event = &record.audit_events()[2];
        assert_eq!(deviation_event.actor(), &actor);
        assert_eq!(deviation_event.reason(), Some(&deviation_reason));
        assert_eq!(
            deviation_event.action(),
            &AuditAction::ContractReviewDeviationAuthorized {
                missing_items: baseline_contract_review_items(),
            }
        );
        assert_eq!(
            event.action(),
            &AuditAction::ProjectStageAdvanced {
                from: ProjectStage::ContractReview,
                to: ProjectStage::TestPlanning,
            }
        );
    }

    #[test]
    fn contract_review_gate_rejects_checklists_for_another_project() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let other_code = ProjectCode::parse("CEM-2026-002").unwrap();
        let project = Project::new(code.clone(), "Example Customer").unwrap();
        let actor = AuditActor::parse("quality.manager").unwrap();
        let mut record = ProjectRecord::open(project, actor.clone());
        record
            .advance_to(
                ProjectStage::ContractReview,
                actor.clone(),
                AuditReason::parse("Quote accepted").unwrap(),
            )
            .unwrap();
        let checklist = ContractReviewChecklist::new(other_code.clone());

        let error = record
            .advance_to_test_planning(
                &checklist,
                actor,
                AuditReason::parse("Planning requested").unwrap(),
                None,
            )
            .unwrap_err();

        assert_eq!(
            error,
            DomainError::ChecklistProjectMismatch {
                project: code,
                checklist_project: other_code,
            }
        );
    }

    #[test]
    fn contract_review_gate_rejects_invalid_source_stage_before_checklist_checks() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let project = Project::new(code.clone(), "Example Customer").unwrap();
        let actor = AuditActor::parse("quality.manager").unwrap();
        let mut record = ProjectRecord::open(project, actor.clone());
        let checklist = ContractReviewChecklist::new(code);

        let error = record
            .advance_to_test_planning(
                &checklist,
                actor,
                AuditReason::parse("Planning requested").unwrap(),
                None,
            )
            .unwrap_err();

        assert_eq!(
            error,
            DomainError::InvalidProjectTransition {
                from: ProjectStage::Quotation,
                to: ProjectStage::TestPlanning,
            }
        );
        assert_eq!(record.project().stage(), ProjectStage::Quotation);
        assert_eq!(record.audit_events().len(), 1);
    }

    #[test]
    fn campaign_trace_starts_with_the_baseline_requirements() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let trace = CampaignTrace::new(code);

        assert!(trace.is_baseline_complete());
        assert_eq!(trace.requirements().len(), 11);
    }
}
