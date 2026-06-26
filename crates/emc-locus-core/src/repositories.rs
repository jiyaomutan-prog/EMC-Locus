use crate::DomainError;

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

impl RepositoryDomain {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Metrology => "metrology",
            Self::TestDefinitions => "test_definitions",
            Self::InstrumentDrivers => "instrument_drivers",
            Self::ProjectRecords => "project_records",
            Self::MeasurementData => "measurement_data",
            Self::ReportTemplates => "report_templates",
            Self::UpdateCatalog => "update_catalog",
        }
    }
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositorySnapshotId(String);

impl RepositorySnapshotId {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyRepositorySnapshotId);
        }

        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(DomainError::InvalidRepositorySnapshotId(trimmed.to_owned()));
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncConflictId(String);

impl SyncConflictId {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptySyncConflictId);
        }

        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(DomainError::InvalidSyncConflictId(trimmed.to_owned()));
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncConflictKind {
    ConcurrentUpdate,
    DeletedInReference,
    DeletedLocally,
    ChecksumMismatch,
    SchemaMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncConflictStatus {
    Open,
    Resolved,
    Deferred,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncConflictResolution {
    KeepLocal,
    KeepReference,
    ManualMerge,
    AcceptDeletion,
    Defer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncConflictRecord {
    id: SyncConflictId,
    domain: RepositoryDomain,
    kind: SyncConflictKind,
    local_snapshot: RepositorySnapshotId,
    reference_snapshot: RepositorySnapshotId,
    status: SyncConflictStatus,
    resolution: Option<SyncConflictResolution>,
}

impl SyncConflictRecord {
    pub fn new(
        id: SyncConflictId,
        domain: RepositoryDomain,
        kind: SyncConflictKind,
        local_snapshot: RepositorySnapshotId,
        reference_snapshot: RepositorySnapshotId,
    ) -> Self {
        Self {
            id,
            domain,
            kind,
            local_snapshot,
            reference_snapshot,
            status: SyncConflictStatus::Open,
            resolution: None,
        }
    }

    pub fn id(&self) -> &SyncConflictId {
        &self.id
    }

    pub fn domain(&self) -> RepositoryDomain {
        self.domain
    }

    pub fn kind(&self) -> SyncConflictKind {
        self.kind
    }

    pub fn local_snapshot(&self) -> &RepositorySnapshotId {
        &self.local_snapshot
    }

    pub fn reference_snapshot(&self) -> &RepositorySnapshotId {
        &self.reference_snapshot
    }

    pub fn status(&self) -> SyncConflictStatus {
        self.status
    }

    pub fn resolution(&self) -> Option<SyncConflictResolution> {
        self.resolution
    }

    pub fn resolve(&mut self, resolution: SyncConflictResolution) -> Result<(), DomainError> {
        if self.status == SyncConflictStatus::Resolved {
            return Err(DomainError::SyncConflictAlreadyResolved(
                self.id.as_str().to_owned(),
            ));
        }

        self.status = match resolution {
            SyncConflictResolution::Defer => SyncConflictStatus::Deferred,
            _ => SyncConflictStatus::Resolved,
        };
        self.resolution = Some(resolution);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotChecksum(String);

impl SnapshotChecksum {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptySnapshotChecksum);
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositorySnapshot {
    domain: RepositoryDomain,
    id: RepositorySnapshotId,
    schema_version: u32,
    checksum: SnapshotChecksum,
    signed: bool,
}

impl RepositorySnapshot {
    pub fn new(
        domain: RepositoryDomain,
        id: RepositorySnapshotId,
        schema_version: u32,
        checksum: SnapshotChecksum,
        signed: bool,
    ) -> Result<Self, DomainError> {
        if schema_version == 0 {
            return Err(DomainError::InvalidRepositorySchemaVersion(schema_version));
        }

        Ok(Self {
            domain,
            id,
            schema_version,
            checksum,
            signed,
        })
    }

    pub fn domain(&self) -> RepositoryDomain {
        self.domain
    }

    pub fn id(&self) -> &RepositorySnapshotId {
        &self.id
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn checksum(&self) -> &SnapshotChecksum {
        &self.checksum
    }

    pub fn signed(&self) -> bool {
        self.signed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepositorySnapshotRequirement {
    domain: RepositoryDomain,
    minimum_schema_version: u32,
    signature_required: bool,
}

impl RepositorySnapshotRequirement {
    pub fn new(
        domain: RepositoryDomain,
        minimum_schema_version: u32,
        signature_required: bool,
    ) -> Result<Self, DomainError> {
        if minimum_schema_version == 0 {
            return Err(DomainError::InvalidRepositorySchemaVersion(
                minimum_schema_version,
            ));
        }

        Ok(Self {
            domain,
            minimum_schema_version,
            signature_required,
        })
    }

    pub fn domain(&self) -> RepositoryDomain {
        self.domain
    }

    pub fn minimum_schema_version(&self) -> u32 {
        self.minimum_schema_version
    }

    pub fn signature_required(&self) -> bool {
        self.signature_required
    }
}

pub fn offline_field_snapshot_requirements() -> Vec<RepositorySnapshotRequirement> {
    baseline_repository_domains()
        .into_iter()
        .map(|domain| RepositorySnapshotRequirement {
            domain,
            minimum_schema_version: 1,
            signature_required: true,
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldRepositoryPackage {
    snapshots: Vec<RepositorySnapshot>,
}

impl FieldRepositoryPackage {
    pub fn new(snapshots: Vec<RepositorySnapshot>) -> Result<Self, DomainError> {
        let mut seen = Vec::new();
        for snapshot in &snapshots {
            if seen.contains(&snapshot.domain()) {
                return Err(DomainError::DuplicateRepositorySnapshot(
                    snapshot.domain().as_str().to_owned(),
                ));
            }
            seen.push(snapshot.domain());
        }

        Ok(Self { snapshots })
    }

    pub fn snapshots(&self) -> &[RepositorySnapshot] {
        &self.snapshots
    }

    pub fn snapshot_for(&self, domain: RepositoryDomain) -> Option<&RepositorySnapshot> {
        self.snapshots
            .iter()
            .find(|snapshot| snapshot.domain() == domain)
    }

    pub fn validate(
        &self,
        requirements: &[RepositorySnapshotRequirement],
    ) -> Result<(), DomainError> {
        for requirement in requirements {
            let snapshot = self.snapshot_for(requirement.domain()).ok_or_else(|| {
                DomainError::MissingRepositorySnapshot(requirement.domain().as_str().to_owned())
            })?;

            if requirement.signature_required() && !snapshot.signed() {
                return Err(DomainError::UnsignedRepositorySnapshot(
                    requirement.domain().as_str().to_owned(),
                ));
            }

            if snapshot.schema_version() < requirement.minimum_schema_version() {
                return Err(DomainError::IncompatibleRepositorySnapshot {
                    domain: requirement.domain().as_str().to_owned(),
                    minimum_schema_version: requirement.minimum_schema_version(),
                    actual_schema_version: snapshot.schema_version(),
                });
            }
        }

        Ok(())
    }
}
