use crate::{
    identifiers::{AuditActor, AuditReason},
    instrument_runtime::InstrumentObservation,
    measurement::{MeasurementRunPlan, MeasurementRunReference},
    DomainError,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatasetReference(String);

impl DatasetReference {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyDatasetReference);
        }

        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(DomainError::InvalidDatasetReference(trimmed.to_owned()));
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatasetFileReference(String);

impl DatasetFileReference {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyDatasetFileReference);
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatasetChecksum(String);

impl DatasetChecksum {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyDatasetChecksum);
        }

        if !trimmed.starts_with("sha256:") || trimmed.len() <= "sha256:".len() {
            return Err(DomainError::InvalidDatasetChecksum(trimmed.to_owned()));
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatasetKind {
    RawSignal,
    RawSweep,
    CommandLog,
    ProcessedSignal,
    ResultTable,
    ReportExport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawDatasetRecord {
    run: MeasurementRunReference,
    reference: DatasetReference,
    kind: DatasetKind,
    file_reference: DatasetFileReference,
    checksum: DatasetChecksum,
    immutable: bool,
}

impl RawDatasetRecord {
    pub fn new(
        run: MeasurementRunReference,
        reference: DatasetReference,
        kind: DatasetKind,
        file_reference: DatasetFileReference,
        checksum: DatasetChecksum,
    ) -> Self {
        Self {
            run,
            reference,
            kind,
            file_reference,
            checksum,
            immutable: true,
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

    pub fn file_reference(&self) -> &DatasetFileReference {
        &self.file_reference
    }

    pub fn checksum(&self) -> &DatasetChecksum {
        &self.checksum
    }

    pub fn immutable(&self) -> bool {
        self.immutable
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatasetRetentionStatus {
    Retained,
    DeletionRequested,
    DeletionApproved,
    DeletionRejected,
    Deleted,
}

impl DatasetRetentionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Retained => "retained",
            Self::DeletionRequested => "deletion_requested",
            Self::DeletionApproved => "deletion_approved",
            Self::DeletionRejected => "deletion_rejected",
            Self::Deleted => "deleted",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatasetRetentionEvent {
    status: DatasetRetentionStatus,
    actor: AuditActor,
    reason: AuditReason,
}

impl DatasetRetentionEvent {
    fn new(status: DatasetRetentionStatus, actor: AuditActor, reason: AuditReason) -> Self {
        Self {
            status,
            actor,
            reason,
        }
    }

    pub fn status(&self) -> DatasetRetentionStatus {
        self.status
    }

    pub fn actor(&self) -> &AuditActor {
        &self.actor
    }

    pub fn reason(&self) -> &AuditReason {
        &self.reason
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatasetRetentionRecord {
    dataset: DatasetReference,
    checksum: DatasetChecksum,
    immutable: bool,
    status: DatasetRetentionStatus,
    events: Vec<DatasetRetentionEvent>,
}

impl DatasetRetentionRecord {
    pub fn for_raw_dataset(dataset: &RawDatasetRecord) -> Self {
        Self {
            dataset: dataset.reference().clone(),
            checksum: dataset.checksum().clone(),
            immutable: dataset.immutable(),
            status: DatasetRetentionStatus::Retained,
            events: Vec::new(),
        }
    }

    pub fn dataset(&self) -> &DatasetReference {
        &self.dataset
    }

    pub fn checksum(&self) -> &DatasetChecksum {
        &self.checksum
    }

    pub fn immutable(&self) -> bool {
        self.immutable
    }

    pub fn status(&self) -> DatasetRetentionStatus {
        self.status
    }

    pub fn events(&self) -> &[DatasetRetentionEvent] {
        &self.events
    }

    pub fn request_deletion(
        &mut self,
        actor: AuditActor,
        reason: AuditReason,
    ) -> Result<(), DomainError> {
        self.record_transition(DatasetRetentionStatus::DeletionRequested, actor, reason)
    }

    pub fn approve_deletion(
        &mut self,
        actor: AuditActor,
        reason: AuditReason,
    ) -> Result<(), DomainError> {
        self.record_transition(DatasetRetentionStatus::DeletionApproved, actor, reason)
    }

    pub fn reject_deletion(
        &mut self,
        actor: AuditActor,
        reason: AuditReason,
    ) -> Result<(), DomainError> {
        self.record_transition(DatasetRetentionStatus::DeletionRejected, actor, reason)
    }

    pub fn mark_deleted(
        &mut self,
        actor: AuditActor,
        reason: AuditReason,
    ) -> Result<(), DomainError> {
        self.record_transition(DatasetRetentionStatus::Deleted, actor, reason)
    }

    fn record_transition(
        &mut self,
        next: DatasetRetentionStatus,
        actor: AuditActor,
        reason: AuditReason,
    ) -> Result<(), DomainError> {
        if !self.can_transition_to(next) {
            return Err(DomainError::InvalidDatasetRetentionTransition {
                dataset: self.dataset.as_str().to_owned(),
                from: self.status.as_str().to_owned(),
                to: next.as_str().to_owned(),
            });
        }

        self.status = next;
        self.events
            .push(DatasetRetentionEvent::new(next, actor, reason));
        Ok(())
    }

    fn can_transition_to(&self, next: DatasetRetentionStatus) -> bool {
        matches!(
            (self.status, next),
            (
                DatasetRetentionStatus::Retained,
                DatasetRetentionStatus::DeletionRequested
            ) | (
                DatasetRetentionStatus::DeletionRequested,
                DatasetRetentionStatus::DeletionApproved
            ) | (
                DatasetRetentionStatus::DeletionRequested,
                DatasetRetentionStatus::DeletionRejected
            ) | (
                DatasetRetentionStatus::DeletionApproved,
                DatasetRetentionStatus::Deleted
            )
        ) || (!self.immutable
            && self.status == DatasetRetentionStatus::Retained
            && next == DatasetRetentionStatus::Deleted)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasurementRunEvidence {
    plan: MeasurementRunPlan,
    observations: Vec<InstrumentObservation>,
    raw_datasets: Vec<RawDatasetRecord>,
}

impl MeasurementRunEvidence {
    pub fn new(plan: MeasurementRunPlan) -> Self {
        Self {
            plan,
            observations: Vec::new(),
            raw_datasets: Vec::new(),
        }
    }

    pub fn plan(&self) -> &MeasurementRunPlan {
        &self.plan
    }

    pub fn observations(&self) -> &[InstrumentObservation] {
        &self.observations
    }

    pub fn raw_datasets(&self) -> &[RawDatasetRecord] {
        &self.raw_datasets
    }

    pub fn record_observation(&mut self, observation: InstrumentObservation) {
        self.observations.push(observation);
    }

    pub fn record_raw_dataset(&mut self, dataset: RawDatasetRecord) -> Result<(), DomainError> {
        if dataset.run() != self.plan.reference() {
            return Err(DomainError::DatasetRunMismatch {
                expected: self.plan.reference().as_str().to_owned(),
                actual: dataset.run().as_str().to_owned(),
            });
        }

        self.raw_datasets.push(dataset);
        Ok(())
    }

    pub fn has_raw_data(&self) -> bool {
        !self.raw_datasets.is_empty()
    }
}
