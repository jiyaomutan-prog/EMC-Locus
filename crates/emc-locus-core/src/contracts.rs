use crate::{repositories::RepositoryDomain, DomainError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractSchemaVersion(String);

impl ContractSchemaVersion {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        parse_ascii_token(
            value,
            DomainError::EmptyContractSchemaVersion,
            DomainError::InvalidContractSchemaVersion,
        )
        .map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StableId(String);

impl StableId {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        parse_ascii_token(
            value,
            DomainError::EmptyStableId,
            DomainError::InvalidStableId,
        )
        .map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityRevision(String);

impl EntityRevision {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        parse_ascii_token(
            value,
            DomainError::EmptyEntityRevision,
            DomainError::InvalidEntityRevision,
        )
        .map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentChecksum(String);

impl ContentChecksum {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptyContentChecksum);
        }
        if !trimmed.starts_with("sha256:") {
            return Err(DomainError::InvalidContentChecksum(trimmed.to_owned()));
        }
        let payload = &trimmed["sha256:".len()..];
        if payload.len() != 64 || !payload.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(DomainError::InvalidContentChecksum(trimmed.to_owned()));
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UtcTimestamp(String);

impl UtcTimestamp {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptyUtcTimestamp);
        }
        if !trimmed.ends_with('Z') || !trimmed.contains('T') {
            return Err(DomainError::InvalidUtcTimestamp(trimmed.to_owned()));
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeOperationKind {
    ProjectCreated,
    ContractReviewItemCompleted,
    ProjectStageAdvanced,
    InstrumentRegistered,
    ServiceItemScheduled,
    DatasetRecorded,
    SyncResolutionRecorded,
}

impl ChangeOperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProjectCreated => "project_created",
            Self::ContractReviewItemCompleted => "contract_review_item_completed",
            Self::ProjectStageAdvanced => "project_stage_advanced",
            Self::InstrumentRegistered => "instrument_registered",
            Self::ServiceItemScheduled => "service_item_scheduled",
            Self::DatasetRecorded => "dataset_recorded",
            Self::SyncResolutionRecorded => "sync_resolution_recorded",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectManifest {
    schema_version: ContractSchemaVersion,
    object_id: StableId,
    logical_path: String,
    media_type: String,
    size_bytes: u64,
    checksum: ContentChecksum,
    worm_locked: bool,
    created_at: UtcTimestamp,
}

impl ObjectManifest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        schema_version: ContractSchemaVersion,
        object_id: StableId,
        logical_path: impl Into<String>,
        media_type: impl Into<String>,
        size_bytes: u64,
        checksum: ContentChecksum,
        worm_locked: bool,
        created_at: UtcTimestamp,
    ) -> Result<Self, DomainError> {
        let logical_path = required_text(logical_path, DomainError::EmptyObjectLogicalPath)?;
        let media_type = required_text(media_type, DomainError::EmptyObjectMediaType)?;
        if size_bytes == 0 {
            return Err(DomainError::EmptyObjectPayload);
        }
        Ok(Self {
            schema_version,
            object_id,
            logical_path,
            media_type,
            size_bytes,
            checksum,
            worm_locked,
            created_at,
        })
    }

    pub fn object_id(&self) -> &StableId {
        &self.object_id
    }

    pub fn checksum(&self) -> &ContentChecksum {
        &self.checksum
    }

    pub fn worm_locked(&self) -> bool {
        self.worm_locked
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntitySnapshot {
    snapshot_id: StableId,
    domain: RepositoryDomain,
    entity_type: String,
    entity_id: StableId,
    revision: EntityRevision,
    checksum: ContentChecksum,
}

impl EntitySnapshot {
    pub fn new(
        snapshot_id: StableId,
        domain: RepositoryDomain,
        entity_type: impl Into<String>,
        entity_id: StableId,
        revision: EntityRevision,
        checksum: ContentChecksum,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            snapshot_id,
            domain,
            entity_type: required_text(entity_type, DomainError::EmptyEntityType)?,
            entity_id,
            revision,
            checksum,
        })
    }

    pub fn snapshot_id(&self) -> &StableId {
        &self.snapshot_id
    }

    pub fn revision(&self) -> &EntityRevision {
        &self.revision
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeOperation {
    operation_id: StableId,
    domain: RepositoryDomain,
    entity_type: String,
    entity_id: StableId,
    operation_kind: ChangeOperationKind,
    base_revision: EntityRevision,
    resulting_revision: EntityRevision,
    actor_id: StableId,
    device_id: StableId,
    correlation_id: StableId,
    occurred_at: UtcTimestamp,
    payload_checksum: ContentChecksum,
}

impl ChangeOperation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operation_id: StableId,
        domain: RepositoryDomain,
        entity_type: impl Into<String>,
        entity_id: StableId,
        operation_kind: ChangeOperationKind,
        base_revision: EntityRevision,
        resulting_revision: EntityRevision,
        actor_id: StableId,
        device_id: StableId,
        correlation_id: StableId,
        occurred_at: UtcTimestamp,
        payload_checksum: ContentChecksum,
    ) -> Result<Self, DomainError> {
        if base_revision == resulting_revision {
            return Err(DomainError::UnchangedEntityRevision(
                resulting_revision.as_str().to_owned(),
            ));
        }
        Ok(Self {
            operation_id,
            domain,
            entity_type: required_text(entity_type, DomainError::EmptyEntityType)?,
            entity_id,
            operation_kind,
            base_revision,
            resulting_revision,
            actor_id,
            device_id,
            correlation_id,
            occurred_at,
            payload_checksum,
        })
    }

    pub fn operation_id(&self) -> &StableId {
        &self.operation_id
    }

    pub fn operation_kind(&self) -> ChangeOperationKind {
        self.operation_kind
    }

    pub fn base_revision(&self) -> &EntityRevision {
        &self.base_revision
    }

    pub fn resulting_revision(&self) -> &EntityRevision {
        &self.resulting_revision
    }
}

fn parse_ascii_token(
    value: impl Into<String>,
    empty_error: DomainError,
    invalid_error: fn(String) -> DomainError,
) -> Result<String, DomainError> {
    let value = value.into();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(empty_error);
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(invalid_error(trimmed.to_owned()));
    }
    Ok(trimmed.to_owned())
}

fn required_text(
    value: impl Into<String>,
    empty_error: DomainError,
) -> Result<String, DomainError> {
    let value = value.into();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(empty_error);
    }
    Ok(trimmed.to_owned())
}
