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
