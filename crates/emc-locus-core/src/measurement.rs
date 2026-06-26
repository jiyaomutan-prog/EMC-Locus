use crate::{
    identifiers::ProjectCode,
    metrology::{EquipmentReadinessReport, InstrumentCode, MetrologyDate, MetrologyRegistry},
    quality::ExecutionMode,
    DomainError,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasurementRunReference(String);

impl MeasurementRunReference {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyMeasurementRunReference);
        }

        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(DomainError::InvalidMeasurementRunReference(
                trimmed.to_owned(),
            ));
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestMethodReference(String);

impl TestMethodReference {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyTestMethodReference);
        }

        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(DomainError::InvalidTestMethodReference(trimmed.to_owned()));
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasurementRunPlan {
    project: ProjectCode,
    reference: MeasurementRunReference,
    method: TestMethodReference,
    mode: ExecutionMode,
    equipment: Vec<InstrumentCode>,
    readiness_report: EquipmentReadinessReport,
}

impl MeasurementRunPlan {
    pub fn plan(
        project: ProjectCode,
        reference: MeasurementRunReference,
        method: TestMethodReference,
        mode: ExecutionMode,
        equipment: Vec<InstrumentCode>,
        registry: &MetrologyRegistry,
        checked_on: MetrologyDate,
    ) -> Result<Self, DomainError> {
        if equipment.is_empty() {
            return Err(DomainError::EmptyEquipmentSelection);
        }

        let readiness_report = registry.assess_equipment_readiness(&equipment, mode, checked_on);
        let blocking_issue_count = readiness_report.blocking_issues().len();
        if blocking_issue_count > 0 {
            return Err(DomainError::EquipmentReadinessBlocked {
                blocking_issue_count,
            });
        }

        Ok(Self {
            project,
            reference,
            method,
            mode,
            equipment,
            readiness_report,
        })
    }

    pub fn project(&self) -> &ProjectCode {
        &self.project
    }

    pub fn reference(&self) -> &MeasurementRunReference {
        &self.reference
    }

    pub fn method(&self) -> &TestMethodReference {
        &self.method
    }

    pub fn mode(&self) -> ExecutionMode {
        self.mode
    }

    pub fn equipment(&self) -> &[InstrumentCode] {
        &self.equipment
    }

    pub fn readiness_report(&self) -> &EquipmentReadinessReport {
        &self.readiness_report
    }
}
