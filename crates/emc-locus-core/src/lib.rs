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
            let deviation = deviation.ok_or_else(|| DomainError::IncompleteContractReview {
                missing_items: missing_items.clone(),
            })?;

            self.append_audit_event(
                deviation.authorized_by,
                AuditAction::ContractReviewDeviationAuthorized { missing_items },
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
pub enum ExecutionMode {
    Accredited,
    NonAccredited,
    Investigation,
}

impl ExecutionMode {
    pub fn constraint_profile(self) -> QualityConstraintProfile {
        match self {
            Self::Accredited => QualityConstraintProfile {
                mode: self,
                stage_gates_required: true,
                valid_calibration_required: true,
                controlled_method_required: true,
                report_approval_required: true,
                deviations_allowed: true,
                exploratory_steps_allowed: false,
            },
            Self::NonAccredited => QualityConstraintProfile {
                mode: self,
                stage_gates_required: true,
                valid_calibration_required: false,
                controlled_method_required: true,
                report_approval_required: false,
                deviations_allowed: true,
                exploratory_steps_allowed: false,
            },
            Self::Investigation => QualityConstraintProfile {
                mode: self,
                stage_gates_required: false,
                valid_calibration_required: false,
                controlled_method_required: false,
                report_approval_required: false,
                deviations_allowed: true,
                exploratory_steps_allowed: true,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QualityConstraintProfile {
    mode: ExecutionMode,
    stage_gates_required: bool,
    valid_calibration_required: bool,
    controlled_method_required: bool,
    report_approval_required: bool,
    deviations_allowed: bool,
    exploratory_steps_allowed: bool,
}

impl QualityConstraintProfile {
    pub fn mode(&self) -> ExecutionMode {
        self.mode
    }

    pub fn stage_gates_required(&self) -> bool {
        self.stage_gates_required
    }

    pub fn valid_calibration_required(&self) -> bool {
        self.valid_calibration_required
    }

    pub fn controlled_method_required(&self) -> bool {
        self.controlled_method_required
    }

    pub fn report_approval_required(&self) -> bool {
        self.report_approval_required
    }

    pub fn deviations_allowed(&self) -> bool {
        self.deviations_allowed
    }

    pub fn exploratory_steps_allowed(&self) -> bool {
        self.exploratory_steps_allowed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityMode {
    Connected,
    OfflineField,
}

impl ConnectivityMode {
    pub fn requires_local_references(self) -> bool {
        matches!(self, Self::OfflineField)
    }

    pub fn allows_measurement_acquisition(self) -> bool {
        true
    }

    pub fn can_require_remote_login(self) -> bool {
        matches!(self, Self::Connected)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryDomain {
    Metrology,
    TestDefinitions,
    InstrumentDrivers,
    ProjectRecords,
    MeasurementData,
    ReportTemplates,
    UpdateCatalog,
}

pub fn baseline_repository_domains() -> Vec<RepositoryDomain> {
    use RepositoryDomain::*;

    vec![
        Metrology,
        TestDefinitions,
        InstrumentDrivers,
        ProjectRecords,
        MeasurementData,
        ReportTemplates,
        UpdateCatalog,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncDirection {
    LocalOnly,
    PullFromReference,
    PushToReference,
    Bidirectional,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepositoryPolicy {
    domain: RepositoryDomain,
    sync_direction: SyncDirection,
    local_snapshot_required: bool,
}

impl RepositoryPolicy {
    pub fn new(domain: RepositoryDomain, connectivity: ConnectivityMode) -> Self {
        let local_snapshot_required = connectivity.requires_local_references()
            || matches!(
                domain,
                RepositoryDomain::Metrology
                    | RepositoryDomain::TestDefinitions
                    | RepositoryDomain::InstrumentDrivers
                    | RepositoryDomain::ProjectRecords
                    | RepositoryDomain::MeasurementData
            );

        let sync_direction = match domain {
            RepositoryDomain::MeasurementData | RepositoryDomain::ProjectRecords => {
                SyncDirection::Bidirectional
            }
            RepositoryDomain::Metrology
            | RepositoryDomain::TestDefinitions
            | RepositoryDomain::InstrumentDrivers
            | RepositoryDomain::ReportTemplates
            | RepositoryDomain::UpdateCatalog => SyncDirection::PullFromReference,
        };

        Self {
            domain,
            sync_direction,
            local_snapshot_required,
        }
    }

    pub fn domain(&self) -> RepositoryDomain {
        self.domain
    }

    pub fn sync_direction(&self) -> SyncDirection {
        self.sync_direction
    }

    pub fn local_snapshot_required(&self) -> bool {
        self.local_snapshot_required
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentTransport {
    Visa,
    Gpib,
    Serial,
    TcpIp,
    Udp,
    UsbTmc,
    Can,
    Lin,
    ModbusTcp,
    ModbusRtu,
    Rest,
    VendorSdk,
    Manual,
    Simulated,
}

pub fn baseline_instrument_transports() -> Vec<InstrumentTransport> {
    use InstrumentTransport::*;

    vec![
        Visa, Gpib, Serial, TcpIp, Udp, UsbTmc, Can, Lin, ModbusTcp, ModbusRtu, Rest, VendorSdk,
        Manual, Simulated,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdatePolicy {
    signed_packages_required: bool,
    offline_install_allowed: bool,
    apply_during_measurement_allowed: bool,
}

impl UpdatePolicy {
    pub fn laboratory_default() -> Self {
        Self {
            signed_packages_required: true,
            offline_install_allowed: true,
            apply_during_measurement_allowed: false,
        }
    }

    pub fn signed_packages_required(&self) -> bool {
        self.signed_packages_required
    }

    pub fn offline_install_allowed(&self) -> bool {
        self.offline_install_allowed
    }

    pub fn apply_during_measurement_allowed(&self) -> bool {
        self.apply_during_measurement_allowed
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

        let error = record
            .advance_to(ProjectStage::Measuring, actor, reason)
            .unwrap_err();

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
        assert_eq!(checklist.missing_items(), baseline_contract_review_items());
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
    fn accredited_mode_keeps_strict_quality_constraints() {
        let profile = ExecutionMode::Accredited.constraint_profile();

        assert_eq!(profile.mode(), ExecutionMode::Accredited);
        assert!(profile.stage_gates_required());
        assert!(profile.valid_calibration_required());
        assert!(profile.controlled_method_required());
        assert!(profile.report_approval_required());
        assert!(profile.deviations_allowed());
        assert!(!profile.exploratory_steps_allowed());
    }

    #[test]
    fn non_accredited_mode_relaxes_metrology_and_report_constraints() {
        let profile = ExecutionMode::NonAccredited.constraint_profile();

        assert_eq!(profile.mode(), ExecutionMode::NonAccredited);
        assert!(profile.stage_gates_required());
        assert!(!profile.valid_calibration_required());
        assert!(profile.controlled_method_required());
        assert!(!profile.report_approval_required());
        assert!(profile.deviations_allowed());
        assert!(!profile.exploratory_steps_allowed());
    }

    #[test]
    fn investigation_mode_allows_exploratory_work() {
        let profile = ExecutionMode::Investigation.constraint_profile();

        assert_eq!(profile.mode(), ExecutionMode::Investigation);
        assert!(!profile.stage_gates_required());
        assert!(!profile.valid_calibration_required());
        assert!(!profile.controlled_method_required());
        assert!(!profile.report_approval_required());
        assert!(profile.deviations_allowed());
        assert!(profile.exploratory_steps_allowed());
    }

    #[test]
    fn offline_field_mode_requires_local_references_but_allows_acquisition() {
        let mode = ConnectivityMode::OfflineField;

        assert!(mode.requires_local_references());
        assert!(mode.allows_measurement_acquisition());
        assert!(!mode.can_require_remote_login());
    }

    #[test]
    fn repository_policy_keeps_core_references_available_offline() {
        let domains = baseline_repository_domains();

        assert!(domains.contains(&RepositoryDomain::Metrology));
        assert!(domains.contains(&RepositoryDomain::TestDefinitions));
        assert!(domains.contains(&RepositoryDomain::InstrumentDrivers));
        assert!(domains.contains(&RepositoryDomain::ProjectRecords));
        assert!(domains.contains(&RepositoryDomain::MeasurementData));

        let policy =
            RepositoryPolicy::new(RepositoryDomain::Metrology, ConnectivityMode::OfflineField);
        assert_eq!(policy.domain(), RepositoryDomain::Metrology);
        assert_eq!(policy.sync_direction(), SyncDirection::PullFromReference);
        assert!(policy.local_snapshot_required());
    }

    #[test]
    fn instrument_transport_baseline_covers_common_lab_communications() {
        let transports = baseline_instrument_transports();

        assert!(transports.contains(&InstrumentTransport::Visa));
        assert!(transports.contains(&InstrumentTransport::Gpib));
        assert!(transports.contains(&InstrumentTransport::Serial));
        assert!(transports.contains(&InstrumentTransport::TcpIp));
        assert!(transports.contains(&InstrumentTransport::UsbTmc));
        assert!(transports.contains(&InstrumentTransport::Can));
        assert!(transports.contains(&InstrumentTransport::Rest));
        assert!(transports.contains(&InstrumentTransport::VendorSdk));
        assert!(transports.contains(&InstrumentTransport::Simulated));
    }

    #[test]
    fn update_policy_requires_signed_packages_and_blocks_live_measurement_updates() {
        let policy = UpdatePolicy::laboratory_default();

        assert!(policy.signed_packages_required());
        assert!(policy.offline_install_allowed());
        assert!(!policy.apply_during_measurement_allowed());
    }

    #[test]
    fn campaign_trace_starts_with_the_baseline_requirements() {
        let code = ProjectCode::parse("CEM-2026-001").unwrap();
        let trace = CampaignTrace::new(code);

        assert!(trace.is_baseline_complete());
        assert_eq!(trace.requirements().len(), 11);
    }
}
