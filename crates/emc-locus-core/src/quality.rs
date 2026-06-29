use crate::identifiers::{AuditActor, AuditReason, ProjectCode};

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

pub fn required_contract_review_items(mode: ExecutionMode) -> Vec<ContractReviewItem> {
    use ContractReviewItem::*;

    match mode {
        ExecutionMode::Accredited => baseline_contract_review_items(),
        ExecutionMode::NonAccredited => vec![
            CustomerRequestDefined,
            TestMethodSelected,
            LaboratoryCapabilityConfirmed,
            DeviationsRecorded,
        ],
        ExecutionMode::Investigation => vec![CustomerRequestDefined, DeviationsRecorded],
    }
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

    pub fn missing_items_for_mode(&self, mode: ExecutionMode) -> Vec<ContractReviewItem> {
        required_contract_review_items(mode)
            .into_iter()
            .filter(|item| !self.completed_items.contains(item))
            .collect()
    }

    pub fn is_complete(&self) -> bool {
        baseline_contract_review_items()
            .iter()
            .all(|item| self.completed_items.contains(item))
    }

    pub fn is_complete_for_mode(&self, mode: ExecutionMode) -> bool {
        required_contract_review_items(mode)
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
