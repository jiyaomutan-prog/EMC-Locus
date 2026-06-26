use std::fmt;

use crate::{instrument::UpdatePolicy, repositories::SnapshotChecksum, DomainError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdatePackageName(String);

impl UpdatePackageName {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyUpdatePackageName);
        }

        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(DomainError::InvalidUpdatePackageName(trimmed.to_owned()));
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SoftwareVersion {
    major: u16,
    minor: u16,
    patch: u16,
}

impl SoftwareVersion {
    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn major(&self) -> u16 {
        self.major
    }

    pub fn minor(&self) -> u16 {
        self.minor
    }

    pub fn patch(&self) -> u16 {
        self.patch
    }
}

impl fmt::Display for SoftwareVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateSignature(String);

impl UpdateSignature {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyUpdateSignature);
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollbackReference(String);

impl RollbackReference {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyRollbackReference);
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateComponent {
    CoreApplication,
    InstrumentDriver,
    SignalProcessingEngine,
    ReportTemplatePack,
    DatabaseMigration,
}

impl UpdateComponent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CoreApplication => "core_application",
            Self::InstrumentDriver => "instrument_driver",
            Self::SignalProcessingEngine => "signal_processing_engine",
            Self::ReportTemplatePack => "report_template_pack",
            Self::DatabaseMigration => "database_migration",
        }
    }
}

pub fn baseline_update_components() -> Vec<UpdateComponent> {
    use UpdateComponent::*;

    vec![
        CoreApplication,
        InstrumentDriver,
        SignalProcessingEngine,
        ReportTemplatePack,
        DatabaseMigration,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateSource {
    OnlineCatalog,
    OfflineBundle,
}

impl UpdateSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OnlineCatalog => "online_catalog",
            Self::OfflineBundle => "offline_bundle",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VersionCompatibilityRange {
    minimum: SoftwareVersion,
    maximum: Option<SoftwareVersion>,
}

impl VersionCompatibilityRange {
    pub fn new(
        minimum: SoftwareVersion,
        maximum: Option<SoftwareVersion>,
    ) -> Result<Self, DomainError> {
        if let Some(maximum) = maximum {
            if maximum < minimum {
                return Err(DomainError::InvalidUpdateCompatibilityRange {
                    minimum_version: minimum.to_string(),
                    maximum_version: maximum.to_string(),
                });
            }
        }

        Ok(Self { minimum, maximum })
    }

    pub fn minimum(&self) -> SoftwareVersion {
        self.minimum
    }

    pub fn maximum(&self) -> Option<SoftwareVersion> {
        self.maximum
    }

    pub fn contains(&self, version: &SoftwareVersion) -> bool {
        *version >= self.minimum
            && self
                .maximum
                .map(|maximum| *version <= maximum)
                .unwrap_or(true)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateBundle {
    name: UpdatePackageName,
    package_version: SoftwareVersion,
    component: UpdateComponent,
    compatible_versions: VersionCompatibilityRange,
    checksum: SnapshotChecksum,
    signature: Option<UpdateSignature>,
    offline_install_allowed: bool,
    rollback_reference: Option<RollbackReference>,
}

impl UpdateBundle {
    pub fn new(
        name: UpdatePackageName,
        package_version: SoftwareVersion,
        component: UpdateComponent,
        compatible_versions: VersionCompatibilityRange,
        checksum: SnapshotChecksum,
        signature: Option<UpdateSignature>,
        offline_install_allowed: bool,
        rollback_reference: Option<RollbackReference>,
    ) -> Self {
        Self {
            name,
            package_version,
            component,
            compatible_versions,
            checksum,
            signature,
            offline_install_allowed,
            rollback_reference,
        }
    }

    pub fn name(&self) -> &UpdatePackageName {
        &self.name
    }

    pub fn package_version(&self) -> SoftwareVersion {
        self.package_version
    }

    pub fn component(&self) -> UpdateComponent {
        self.component
    }

    pub fn compatible_versions(&self) -> VersionCompatibilityRange {
        self.compatible_versions
    }

    pub fn checksum(&self) -> &SnapshotChecksum {
        &self.checksum
    }

    pub fn signature(&self) -> Option<&UpdateSignature> {
        self.signature.as_ref()
    }

    pub fn signed(&self) -> bool {
        self.signature.is_some()
    }

    pub fn offline_install_allowed(&self) -> bool {
        self.offline_install_allowed
    }

    pub fn rollback_reference(&self) -> Option<&RollbackReference> {
        self.rollback_reference.as_ref()
    }

    pub fn is_compatible_with(&self, installed_version: &SoftwareVersion) -> bool {
        self.compatible_versions.contains(installed_version)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateInstallPlan {
    bundle: UpdateBundle,
    source: UpdateSource,
    installed_version: SoftwareVersion,
}

impl UpdateInstallPlan {
    pub fn prepare(
        bundle: UpdateBundle,
        policy: UpdatePolicy,
        installed_version: SoftwareVersion,
        source: UpdateSource,
        measurement_active: bool,
    ) -> Result<Self, DomainError> {
        if policy.signed_packages_required() && !bundle.signed() {
            return Err(DomainError::UnsignedUpdatePackage(
                bundle.name().as_str().to_owned(),
            ));
        }

        if source == UpdateSource::OfflineBundle
            && (!policy.offline_install_allowed() || !bundle.offline_install_allowed())
        {
            return Err(DomainError::OfflineUpdateInstallNotAllowed(
                bundle.name().as_str().to_owned(),
            ));
        }

        if measurement_active && !policy.apply_during_measurement_allowed() {
            return Err(DomainError::UpdateDuringMeasurementBlocked(
                bundle.name().as_str().to_owned(),
            ));
        }

        if !bundle.is_compatible_with(&installed_version) {
            let compatible_versions = bundle.compatible_versions();
            return Err(DomainError::IncompatibleUpdatePackage {
                package: bundle.name().as_str().to_owned(),
                minimum_version: compatible_versions.minimum().to_string(),
                maximum_version: compatible_versions
                    .maximum()
                    .map(|version| version.to_string()),
                actual_version: installed_version.to_string(),
            });
        }

        Ok(Self {
            bundle,
            source,
            installed_version,
        })
    }

    pub fn bundle(&self) -> &UpdateBundle {
        &self.bundle
    }

    pub fn source(&self) -> UpdateSource {
        self.source
    }

    pub fn installed_version(&self) -> SoftwareVersion {
        self.installed_version
    }

    pub fn rollback_reference(&self) -> Option<&RollbackReference> {
        self.bundle.rollback_reference()
    }
}
