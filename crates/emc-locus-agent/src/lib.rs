mod asset_correction_repository;
mod asset_correction_service;
mod document_dto;
mod document_repository;
mod document_service;
mod equipment_dto;
mod equipment_repository;
mod equipment_service;
mod file_store;
mod local_api;
mod measurement_engineering_dto;
mod measurement_engineering_repository;
mod measurement_engineering_service;
mod metrology_agent;
mod metrology_dto;
mod metrology_repository;
mod metrology_service;
mod project_agent;
mod project_dto;
mod project_repository;
mod project_service;
mod service_schedule_dto;
mod service_schedule_repository;
mod service_schedule_service;
mod sqlite_policy;
mod station_setup_dto;
mod station_setup_repository;
mod station_setup_service;
mod test_execution_dto;
mod test_execution_repository;
mod test_execution_service;
mod test_template_dto;
mod test_template_repository;
mod test_template_service;

pub use asset_correction_service::{
    approve_and_activate_asset_correction, create_asset_correction_assignment,
    get_asset_correction_assignment, list_asset_correction_assignments,
    list_asset_correction_review_queue, reject_asset_correction, request_asset_correction_changes,
    resolve_material_corrections, submit_asset_correction_for_review,
    CreateAssetCorrectionAssignmentInput, ResolveMaterialCorrectionsInput,
    TransitionAssetCorrectionInput,
};
pub use document_service::{
    get_document, list_document_audit_events, list_documents, register_attached_document,
    ListAttachedDocumentsInput, RegisterAttachedDocumentInput,
};
use emc_locus_core::{baseline_repository_domains, RepositoryDomain};
pub use equipment_service::{
    archive_equipment_category_json, archive_equipment_field_definition_json,
    clone_equipment_model, communication_provider_status, create_driver_profile,
    create_driver_profile_revision, create_equipment_category_json,
    create_equipment_field_definition_json, create_equipment_model,
    create_equipment_model_from_category_template, create_equipment_model_from_preset,
    create_equipment_model_revision, equipment_category_tree_json,
    equipment_effective_template_json, equipment_registries, get_classification_preset,
    get_driver_profile, get_driver_profile_revision, get_equipment_model,
    get_equipment_model_revision, list_classification_presets, list_driver_profile_revisions,
    list_driver_profiles, list_equipment_audit_events_for_driver,
    list_equipment_audit_events_for_model, list_equipment_categories_json,
    list_equipment_category_field_rules_json, list_equipment_field_definitions_json,
    list_equipment_model_revisions, list_equipment_models, move_equipment_category_json,
    replace_driver_profile_revision_definition, replace_equipment_category_field_rules_json,
    replace_equipment_model_revision_definition, simulate_driver_profile, store_equipment_file,
    transition_driver_profile_revision, transition_equipment_model_revision,
    update_equipment_category_json, update_equipment_field_definition_json,
    validate_driver_profile_definition_json, validate_equipment_model_definition_json,
    CloneEquipmentModelInput, CreateDriverProfileInput, CreateDriverProfileRevisionInput,
    CreateEquipmentCategoryInput, CreateEquipmentModelFromCategoryTemplateInput,
    CreateEquipmentModelFromPresetInput, CreateEquipmentModelInput,
    CreateEquipmentModelRevisionInput, EquipmentCategoryFieldRuleInput, ListDriverProfilesInput,
    ListEquipmentCategoriesInput, ListEquipmentFieldDefinitionsInput, ListEquipmentModelsInput,
    MoveEquipmentCategoryInput, ReplaceDriverProfileDefinitionInput,
    ReplaceEquipmentCategoryFieldRulesInput, ReplaceEquipmentModelDefinitionInput,
    SimulateDriverProfileInput, StoreEquipmentFileInput, TransitionDriverProfileRevisionInput,
    TransitionEquipmentModelRevisionInput, UpdateEquipmentCategoryInput,
    UpsertEquipmentFieldDefinitionInput,
};
pub use local_api::{run_local_api_server, ApiServerConfig};
pub use measurement_engineering_service::{
    clone_measurement_engineering_definition, create_measurement_engineering_definition,
    create_measurement_engineering_revision, evaluate_engineering_curve_revision,
    get_measurement_engineering_definition, get_measurement_engineering_revision_json,
    list_measurement_engineering_audit_events, list_measurement_engineering_definitions,
    list_measurement_engineering_revisions_json,
    replace_measurement_engineering_revision_definition,
    transition_measurement_engineering_revision, validate_measurement_engineering_definition_json,
    CloneMeasurementEngineeringInput, CreateMeasurementEngineeringInput,
    CreateMeasurementEngineeringRevisionInput, EvaluateEngineeringCurveInput,
    ReplaceMeasurementEngineeringDefinitionInput, TransitionMeasurementEngineeringRevisionInput,
};
pub use metrology_agent::{run_metrology_command, MetrologyAction};
pub use metrology_service::{
    assess_metrology_readiness, get_metrology_calibration_status, get_metrology_instrument,
    list_metrology_audit_events, list_metrology_calibrations, list_metrology_instruments,
    record_metrology_calibration, set_metrology_serviceability, AssessReadinessInput,
    MetrologyOperationContext, RecordCalibrationInput, SetServiceabilityInput,
};
pub use metrology_service::{register_metrology_instrument, RegisterInstrumentInput};
pub use project_agent::{run_project_command, run_sync_command, ProjectAction, SyncAction};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
pub use service_schedule_service::{
    create_service_schedule_item, list_project_service_schedule_items,
    transition_service_schedule_item, CreateServiceScheduleItemInput,
    TransitionServiceScheduleItemInput,
};
use sqlite_policy::{initialize_project_slice_journal_mode, journal_mode, AttachedDatabase};
pub use station_setup_service::{
    assess_station_setup_revision_json, create_station_setup, derive_station_setup_revision,
    get_station_setup, get_station_setup_revision_json, list_station_setup_audit_events_json,
    list_station_setup_revisions_json, list_station_setups, mark_station_setup_revision_ready,
    replace_station_setup_draft_definition, CreateStationSetupInput,
    DeriveStationSetupRevisionInput, MarkStationSetupReadyInput, ReplaceStationSetupDraftInput,
    StationOperationContext,
};
use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};
pub use test_template_service::{
    clone_test_template, create_test_template, create_test_template_revision,
    get_test_template_definition, get_test_template_revision, list_test_template_audit_events,
    list_test_template_definitions, list_test_template_revisions,
    replace_test_template_revision_definition, transition_test_template_revision,
    validate_test_template_definition_json, CloneTestTemplateInput, CreateTestTemplateInput,
    CreateTestTemplateRevisionInput, ListTestTemplatesInput, ReplaceTestTemplateDefinitionInput,
    TransitionTestTemplateRevisionInput,
};

pub const AGENT_NAME: &str = "emc-locus-agent";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentCommand {
    Health {
        storage_root: PathBuf,
    },
    Storage {
        action: StorageAction,
        storage_root: PathBuf,
        migrations_root: PathBuf,
    },
    Projects {
        action: ProjectAction,
        storage_root: PathBuf,
    },
    Metrology {
        action: MetrologyAction,
        storage_root: PathBuf,
    },
    Sync {
        action: SyncAction,
        storage_root: PathBuf,
    },
    Serve {
        config: ApiServerConfig,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageAction {
    Init,
    Status,
    Verify,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
    details: Option<Value>,
}

impl AgentError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub(crate) fn with_details(
        code: &'static str,
        message: impl Into<String>,
        details: Value,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details: Some(details),
        }
    }

    pub fn to_json(&self) -> String {
        let error = AgentErrorEnvelope {
            error: AgentErrorDto {
                code: self.code,
                message: &self.message,
                details: self.details.as_ref(),
            },
        };
        render_json(&error)
    }
}

impl fmt::Display for AgentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

#[derive(Serialize)]
struct AgentErrorEnvelope<'a> {
    error: AgentErrorDto<'a>,
}

#[derive(Serialize)]
struct AgentErrorDto<'a> {
    code: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<&'a Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageReport {
    pub action: StorageAction,
    pub storage_root: PathBuf,
    pub migrations_root: PathBuf,
    pub domains: Vec<StorageDomainReport>,
}

impl StorageReport {
    pub fn to_json(&self) -> String {
        render_json(&StorageReportDto::from(self))
    }
}

#[derive(Serialize)]
struct StorageReportDto {
    action: &'static str,
    storage_root: String,
    migrations_root: String,
    domains: Vec<StorageDomainReportDto>,
}

impl From<&StorageReport> for StorageReportDto {
    fn from(report: &StorageReport) -> Self {
        Self {
            action: report.action.as_str(),
            storage_root: report.storage_root.to_string_lossy().to_string(),
            migrations_root: report.migrations_root.to_string_lossy().to_string(),
            domains: report
                .domains
                .iter()
                .map(StorageDomainReportDto::from)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageDomainReport {
    pub domain: &'static str,
    pub database_path: PathBuf,
    pub exists: bool,
    pub schema_version: Option<u32>,
    pub latest_migration: u32,
    pub status: StorageDomainStatus,
    pub foreign_keys_enabled: Option<bool>,
    pub integrity_check: Option<String>,
    pub journal_mode: Option<String>,
    pub atomicity_compatible: Option<bool>,
}

#[derive(Serialize)]
struct StorageDomainReportDto {
    domain: &'static str,
    database_path: String,
    exists: bool,
    schema_version: Option<u32>,
    latest_migration: u32,
    status: &'static str,
    foreign_keys_enabled: Option<bool>,
    integrity_check: Option<String>,
    journal_mode: Option<String>,
    atomicity_compatible: Option<bool>,
}

impl From<&StorageDomainReport> for StorageDomainReportDto {
    fn from(report: &StorageDomainReport) -> Self {
        Self {
            domain: report.domain,
            database_path: report.database_path.to_string_lossy().to_string(),
            exists: report.exists,
            schema_version: report.schema_version,
            latest_migration: report.latest_migration,
            status: report.status.as_str(),
            foreign_keys_enabled: report.foreign_keys_enabled,
            integrity_check: report.integrity_check.clone(),
            journal_mode: report.journal_mode.clone(),
            atomicity_compatible: report.atomicity_compatible,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageDomainStatus {
    Missing,
    Current,
    MigrationRequired,
    Invalid,
}

impl StorageDomainStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Current => "current",
            Self::MigrationRequired => "migration_required",
            Self::Invalid => "invalid",
        }
    }
}

impl StorageAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Status => "status",
            Self::Verify => "verify",
        }
    }
}

impl Error for AgentError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HealthReport {
    pub agent: &'static str,
    pub version: &'static str,
    pub storage_root: PathBuf,
    pub storage_root_exists: bool,
    pub domains: Vec<&'static str>,
}

impl HealthReport {
    pub fn to_json(&self) -> String {
        render_json(&HealthReportDto::from(self))
    }
}

#[derive(Serialize)]
struct HealthReportDto {
    agent: &'static str,
    version: &'static str,
    storage_root: String,
    storage_root_exists: bool,
    domains: Vec<&'static str>,
}

impl From<&HealthReport> for HealthReportDto {
    fn from(report: &HealthReport) -> Self {
        Self {
            agent: report.agent,
            version: report.version,
            storage_root: report.storage_root.to_string_lossy().to_string(),
            storage_root_exists: report.storage_root_exists,
            domains: report.domains.clone(),
        }
    }
}

pub fn parse_agent_args<I, S>(args: I) -> Result<AgentCommand, AgentError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let command = args
        .next()
        .ok_or_else(|| AgentError::new("missing_command", "missing command"))?;
    if command == "storage" {
        return parse_storage_args(args);
    }
    if command == "projects" {
        return project_agent::parse_project_args(args);
    }
    if command == "metrology" {
        return metrology_agent::parse_metrology_args(args);
    }
    if command == "sync" {
        return project_agent::parse_sync_args(args);
    }
    if command == "serve" {
        return local_api::parse_serve_args(args);
    }
    if command != "health" {
        return Err(AgentError::new(
            "unknown_command",
            format!("unknown command: {command}"),
        ));
    }

    let mut storage_root = PathBuf::from(".");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--storage-root" => {
                let value = args.next().ok_or_else(|| {
                    AgentError::new("missing_argument", "missing value for --storage-root")
                })?;
                storage_root = PathBuf::from(value);
            }
            unknown => {
                return Err(AgentError::new(
                    "unknown_argument",
                    format!("unknown argument: {unknown}"),
                ))
            }
        }
    }

    Ok(AgentCommand::Health { storage_root })
}

fn parse_storage_args<I>(mut args: I) -> Result<AgentCommand, AgentError>
where
    I: Iterator<Item = String>,
{
    let action = match args
        .next()
        .ok_or_else(|| AgentError::new("missing_storage_action", "missing storage action"))?
        .as_str()
    {
        "init" => StorageAction::Init,
        "status" => StorageAction::Status,
        "verify" => StorageAction::Verify,
        other => {
            return Err(AgentError::new(
                "unknown_storage_action",
                format!("unknown storage action: {other}"),
            ))
        }
    };

    let mut storage_root = None;
    let mut migrations_root = PathBuf::from("storage/sqlite");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--storage-root" => {
                let value = args.next().ok_or_else(|| {
                    AgentError::new("missing_argument", "missing value for --storage-root")
                })?;
                storage_root = Some(PathBuf::from(value));
            }
            "--migrations-root" => {
                let value = args.next().ok_or_else(|| {
                    AgentError::new("missing_argument", "missing value for --migrations-root")
                })?;
                migrations_root = PathBuf::from(value);
            }
            unknown => {
                return Err(AgentError::new(
                    "unknown_argument",
                    format!("unknown argument: {unknown}"),
                ))
            }
        }
    }

    let storage_root = storage_root.ok_or_else(|| {
        AgentError::new(
            "missing_storage_root",
            "storage commands require --storage-root",
        )
    })?;

    Ok(AgentCommand::Storage {
        action,
        storage_root,
        migrations_root,
    })
}

pub fn build_health_report(storage_root: impl AsRef<Path>) -> HealthReport {
    let storage_root = storage_root.as_ref().to_path_buf();
    let storage_root_exists = storage_root.exists();
    let domains = baseline_repository_domains()
        .into_iter()
        .map(RepositoryDomain::as_str)
        .collect();

    HealthReport {
        agent: AGENT_NAME,
        version: env!("CARGO_PKG_VERSION"),
        storage_root,
        storage_root_exists,
        domains,
    }
}

pub fn run_storage_command(command: AgentCommand) -> Result<StorageReport, AgentError> {
    match command {
        AgentCommand::Storage {
            action,
            storage_root,
            migrations_root,
        } => run_storage_action(action, storage_root, migrations_root),
        _ => Err(AgentError::new(
            "invalid_storage_command",
            "expected a storage command",
        )),
    }
}

pub fn run_storage_action(
    action: StorageAction,
    storage_root: PathBuf,
    migrations_root: PathBuf,
) -> Result<StorageReport, AgentError> {
    if !migrations_root.is_dir() {
        return Err(AgentError::new(
            "missing_migrations_root",
            format!(
                "migrations root does not exist: {}",
                migrations_root.display()
            ),
        ));
    }
    if matches!(action, StorageAction::Init) {
        fs::create_dir_all(&storage_root).map_err(|error| {
            AgentError::new(
                "storage_directory_error",
                format!(
                    "cannot create storage root {}: {error}",
                    storage_root.display()
                ),
            )
        })?;
    }

    let mut domains = Vec::new();
    for domain in project_slice_domains() {
        domains.push(inspect_or_initialize_domain(
            domain,
            action,
            &storage_root,
            &migrations_root,
        )?);
    }

    Ok(StorageReport {
        action,
        storage_root,
        migrations_root,
        domains,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StorageDomainSpec {
    domain: &'static str,
    database_file: &'static str,
    migration_folder: &'static str,
}

fn project_slice_domains() -> [StorageDomainSpec; 6] {
    [
        StorageDomainSpec {
            domain: "projects",
            database_file: "projects.sqlite",
            migration_folder: "projects",
        },
        StorageDomainSpec {
            domain: "sync",
            database_file: "sync.sqlite",
            migration_folder: "sync",
        },
        StorageDomainSpec {
            domain: "metrology",
            database_file: "metrology.sqlite",
            migration_folder: "metrology",
        },
        StorageDomainSpec {
            domain: "equipment",
            database_file: "equipment.sqlite",
            migration_folder: "equipment",
        },
        StorageDomainSpec {
            domain: "test_definitions",
            database_file: "test_definitions.sqlite",
            migration_folder: "test_definitions",
        },
        StorageDomainSpec {
            domain: "station_configurations",
            database_file: "station.sqlite",
            migration_folder: "station",
        },
    ]
}

fn inspect_or_initialize_domain(
    domain: StorageDomainSpec,
    action: StorageAction,
    storage_root: &Path,
    migrations_root: &Path,
) -> Result<StorageDomainReport, AgentError> {
    let database_path = storage_root.join(domain.database_file);
    let migrations = discover_domain_migrations(migrations_root, domain)?;
    let latest_migration = migrations
        .last()
        .map(|migration| migration.version)
        .unwrap_or(0);

    if !database_path.exists() && !matches!(action, StorageAction::Init) {
        return Ok(StorageDomainReport {
            domain: domain.domain,
            database_path,
            exists: false,
            schema_version: None,
            latest_migration,
            status: StorageDomainStatus::Missing,
            foreign_keys_enabled: None,
            integrity_check: None,
            journal_mode: None,
            atomicity_compatible: None,
        });
    }

    let connection = Connection::open(&database_path).map_err(|error| {
        AgentError::new(
            "database_open_error",
            format!("cannot open {}: {error}", database_path.display()),
        )
    })?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| {
            AgentError::new(
                "database_pragma_error",
                format!(
                    "cannot enable foreign keys for {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    if matches!(action, StorageAction::Init) {
        initialize_project_slice_journal_mode(
            &connection,
            AttachedDatabase::Main,
            domain.database_file,
        )?;
        apply_missing_migrations(&connection, &migrations)?;
    }

    let schema_version = current_schema_version(&connection)?;
    let foreign_keys_enabled = pragma_foreign_keys(&connection)?;
    let integrity_check = integrity_check(&connection)?;
    let journal_mode = journal_mode(&connection, AttachedDatabase::Main)?;
    let atomicity_compatible = sqlite_policy::supports_multi_database_atomicity(&journal_mode);
    let status = if schema_version.is_none() {
        StorageDomainStatus::Invalid
    } else if schema_version.unwrap_or(0) < latest_migration {
        StorageDomainStatus::MigrationRequired
    } else if integrity_check.as_deref() == Some("ok")
        && foreign_keys_enabled
        && atomicity_compatible
    {
        StorageDomainStatus::Current
    } else {
        StorageDomainStatus::Invalid
    };

    if matches!(action, StorageAction::Verify) && status != StorageDomainStatus::Current {
        return Err(AgentError::new(
            "storage_verify_failed",
            format!("{} database is not current", domain.domain),
        ));
    }

    Ok(StorageDomainReport {
        domain: domain.domain,
        database_path,
        exists: true,
        schema_version,
        latest_migration,
        status,
        foreign_keys_enabled: Some(foreign_keys_enabled),
        integrity_check,
        journal_mode: Some(journal_mode),
        atomicity_compatible: Some(atomicity_compatible),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Migration {
    version: u32,
    path: PathBuf,
}

fn discover_domain_migrations(
    migrations_root: &Path,
    domain: StorageDomainSpec,
) -> Result<Vec<Migration>, AgentError> {
    let folder = migrations_root.join(domain.migration_folder);
    let entries = fs::read_dir(&folder).map_err(|error| {
        AgentError::new(
            "migration_discovery_error",
            format!("cannot read migrations {}: {error}", folder.display()),
        )
    })?;
    let mut migrations = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| {
                AgentError::new(
                    "migration_discovery_error",
                    format!("bad directory entry: {error}"),
                )
            })?
            .path();
        if path.extension().and_then(|value| value.to_str()) != Some("sql") {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                AgentError::new(
                    "migration_discovery_error",
                    "migration filename is not UTF-8",
                )
            })?;
        let version_text = file_name
            .split_once('_')
            .map(|(version, _)| version)
            .ok_or_else(|| {
                AgentError::new(
                    "migration_discovery_error",
                    format!("migration filename is missing version: {file_name}"),
                )
            })?;
        let version = version_text.parse::<u32>().map_err(|_| {
            AgentError::new(
                "migration_discovery_error",
                format!("migration filename has invalid version: {file_name}"),
            )
        })?;
        migrations.push(Migration { version, path });
    }
    migrations.sort_by_key(|migration| migration.version);
    for (index, migration) in migrations.iter().enumerate() {
        let expected = (index + 1) as u32;
        if migration.version != expected {
            return Err(AgentError::new(
                "migration_sequence_error",
                format!(
                    "{} migrations are not contiguous at version {}",
                    domain.domain, expected
                ),
            ));
        }
    }
    Ok(migrations)
}

fn apply_missing_migrations(
    connection: &Connection,
    migrations: &[Migration],
) -> Result<(), AgentError> {
    let applied = applied_schema_versions(connection)?;
    for migration in migrations {
        if applied.contains(&migration.version) {
            continue;
        }
        let sql = fs::read_to_string(&migration.path).map_err(|error| {
            AgentError::new(
                "migration_read_error",
                format!("cannot read {}: {error}", migration.path.display()),
            )
        })?;
        connection.execute_batch(&sql).map_err(|error| {
            AgentError::new(
                "migration_apply_error",
                format!("cannot apply {}: {error}", migration.path.display()),
            )
        })?;
    }
    Ok(())
}

fn applied_schema_versions(connection: &Connection) -> Result<Vec<u32>, AgentError> {
    if !table_exists(connection, "schema_migrations")? {
        return Ok(Vec::new());
    }
    let mut statement = connection
        .prepare("SELECT version FROM schema_migrations ORDER BY version")
        .map_err(|error| AgentError::new("schema_query_error", error.to_string()))?;
    let rows = statement
        .query_map([], |row| row.get::<_, u32>(0))
        .map_err(|error| AgentError::new("schema_query_error", error.to_string()))?;
    let mut versions = Vec::new();
    for row in rows {
        versions
            .push(row.map_err(|error| AgentError::new("schema_query_error", error.to_string()))?);
    }
    Ok(versions)
}

fn current_schema_version(connection: &Connection) -> Result<Option<u32>, AgentError> {
    Ok(applied_schema_versions(connection)?.into_iter().max())
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool, AgentError> {
    let count: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get(0),
        )
        .map_err(|error| AgentError::new("database_invalid", error.to_string()))?;
    Ok(count > 0)
}

fn pragma_foreign_keys(connection: &Connection) -> Result<bool, AgentError> {
    let value: u32 = connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .map_err(|error| AgentError::new("database_pragma_error", error.to_string()))?;
    Ok(value == 1)
}

fn integrity_check(connection: &Connection) -> Result<Option<String>, AgentError> {
    let value: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| AgentError::new("database_integrity_error", error.to_string()))?;
    Ok(Some(value))
}

pub(crate) fn render_json(value: &impl Serialize) -> String {
    serde_json::to_string(value).expect("agent DTO serialization should not fail")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_health_command_with_storage_root() {
        let command = parse_agent_args(["health", "--storage-root", "E:/emc-locus"]).unwrap();

        assert_eq!(
            command,
            AgentCommand::Health {
                storage_root: PathBuf::from("E:/emc-locus")
            }
        );
    }

    #[test]
    fn parses_storage_init_command_with_required_paths() {
        let command = parse_agent_args([
            "storage",
            "init",
            "--storage-root",
            "E:/emc-locus/data",
            "--migrations-root",
            "E:/emc-locus/storage/sqlite",
        ])
        .unwrap();

        assert_eq!(
            command,
            AgentCommand::Storage {
                action: StorageAction::Init,
                storage_root: PathBuf::from("E:/emc-locus/data"),
                migrations_root: PathBuf::from("E:/emc-locus/storage/sqlite")
            }
        );
    }

    #[test]
    fn rejects_unknown_command() {
        assert_eq!(
            parse_agent_args(["daemon"]).unwrap_err().to_string(),
            "unknown command: daemon"
        );
    }

    #[test]
    fn storage_init_creates_project_slice_databases_idempotently() {
        let storage_root = temporary_storage_root("agent-storage-init");
        let migrations_root = repo_root().join("storage/sqlite");

        let report = run_storage_action(
            StorageAction::Init,
            storage_root.clone(),
            migrations_root.clone(),
        )
        .unwrap();
        let second_report =
            run_storage_action(StorageAction::Init, storage_root.clone(), migrations_root).unwrap();

        assert_eq!(report.domains.len(), 6);
        assert!(storage_root.join("projects.sqlite").exists());
        assert!(storage_root.join("sync.sqlite").exists());
        assert!(storage_root.join("metrology.sqlite").exists());
        assert!(storage_root.join("equipment.sqlite").exists());
        assert!(storage_root.join("test_definitions.sqlite").exists());
        assert!(storage_root.join("station.sqlite").exists());
        assert!(report
            .domains
            .iter()
            .all(|domain| domain.status == StorageDomainStatus::Current));
        assert!(report.domains.iter().all(|domain| {
            domain.journal_mode.as_deref() == Some("delete")
                && domain.atomicity_compatible == Some(true)
        }));
        assert_eq!(
            second_report
                .domains
                .iter()
                .find(|domain| domain.domain == "projects")
                .unwrap()
                .schema_version,
            Some(6)
        );
        assert_eq!(
            second_report
                .domains
                .iter()
                .find(|domain| domain.domain == "sync")
                .unwrap()
                .schema_version,
            Some(5)
        );
        assert_eq!(
            second_report
                .domains
                .iter()
                .find(|domain| domain.domain == "metrology")
                .unwrap()
                .schema_version,
            Some(10)
        );
        assert_eq!(
            second_report
                .domains
                .iter()
                .find(|domain| domain.domain == "equipment")
                .unwrap()
                .schema_version,
            Some(6)
        );
        assert_eq!(
            second_report
                .domains
                .iter()
                .find(|domain| domain.domain == "test_definitions")
                .unwrap()
                .schema_version,
            Some(5)
        );
        assert_eq!(
            second_report
                .domains
                .iter()
                .find(|domain| domain.domain == "station_configurations")
                .unwrap()
                .schema_version,
            Some(1)
        );

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn storage_init_preserves_v5_schedule_rows_when_agent_ownership_is_added() {
        let storage_root = temporary_storage_root("agent-storage-project-v6-migration");
        let migrations_root = repo_root().join("storage/sqlite");
        fs::create_dir_all(&storage_root).unwrap();
        let projects_database = storage_root.join("projects.sqlite");
        let connection = Connection::open(&projects_database).unwrap();
        for migration in [
            "0001_project_records.sql",
            "0002_service_schedule.sql",
            "0003_simulated_test_executions.sql",
            "0004_attached_documents.sql",
            "0005_simulated_execution_template_revision.sql",
        ] {
            let sql = fs::read_to_string(migrations_root.join("projects").join(migration)).unwrap();
            connection.execute_batch(&sql).unwrap();
        }
        connection
            .execute(
                concat!(
                    "INSERT INTO projects ",
                    "(code, customer_name, stage, execution_mode, created_at) ",
                    "VALUES ('CEM-LEGACY-PLAN', 'Legacy customer', 'test_planning', ",
                    "'non_accredited', '2026-07-14T08:00:00Z')"
                ),
                [],
            )
            .unwrap();
        connection
            .execute(
                concat!(
                    "INSERT INTO service_schedule_items ",
                    "(item_code, project_code, title, planned_start_at, planned_end_at, ",
                    "assigned_operator, location, equipment_under_test, status, notes, ",
                    "created_at, updated_at) VALUES ",
                    "('PLAN-LEGACY-001', 'CEM-LEGACY-PLAN', 'Legacy test', ",
                    "'2026-07-15T09:00', '2026-07-15T10:00', 'Alice', 'Lab 1', 'EUT', ",
                    "'planned', '', '2026-07-14T08:00:00Z', '2026-07-14T08:00:00Z')"
                ),
                [],
            )
            .unwrap();
        drop(connection);

        let report =
            run_storage_action(StorageAction::Init, storage_root.clone(), migrations_root).unwrap();
        let connection = Connection::open(projects_database).unwrap();
        let migrated: (u64, String, String) = connection
            .query_row(
                concat!(
                    "SELECT revision, created_by, updated_by FROM service_schedule_items ",
                    "WHERE item_code = 'PLAN-LEGACY-001'"
                ),
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(
            report
                .domains
                .iter()
                .find(|domain| domain.domain == "projects")
                .unwrap()
                .schema_version,
            Some(6)
        );
        assert_eq!(
            migrated,
            (1, "legacy-import".to_owned(), "legacy-import".to_owned())
        );
        drop(connection);
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn project_storage_status_reports_incompatible_journal_mode() {
        let storage_root = temporary_storage_root("agent-storage-wal-policy");
        let migrations_root = repo_root().join("storage/sqlite");
        run_storage_action(
            StorageAction::Init,
            storage_root.clone(),
            migrations_root.clone(),
        )
        .unwrap();
        force_journal_mode(&storage_root.join("projects.sqlite"), "WAL");

        let report = run_storage_action(
            StorageAction::Status,
            storage_root.clone(),
            migrations_root.clone(),
        )
        .unwrap();
        let projects = report
            .domains
            .iter()
            .find(|domain| domain.domain == "projects")
            .unwrap();
        let sync = report
            .domains
            .iter()
            .find(|domain| domain.domain == "sync")
            .unwrap();
        let verify_error =
            run_storage_action(StorageAction::Verify, storage_root.clone(), migrations_root)
                .unwrap_err();

        assert_eq!(projects.status, StorageDomainStatus::Invalid);
        assert_eq!(projects.journal_mode.as_deref(), Some("wal"));
        assert_eq!(projects.atomicity_compatible, Some(false));
        assert_eq!(sync.status, StorageDomainStatus::Current);
        assert_eq!(verify_error.code, "storage_verify_failed");

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn storage_status_reports_missing_before_initialization() {
        let storage_root = temporary_storage_root("agent-storage-status");
        let migrations_root = repo_root().join("storage/sqlite");

        let report =
            run_storage_action(StorageAction::Status, storage_root.clone(), migrations_root)
                .unwrap();

        assert!(report
            .domains
            .iter()
            .all(|domain| domain.status == StorageDomainStatus::Missing));
        assert!(report.domains.iter().all(|domain| {
            domain.journal_mode.is_none() && domain.atomicity_compatible.is_none()
        }));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn storage_verify_rejects_invalid_database() {
        let storage_root = temporary_storage_root("agent-storage-invalid");
        fs::create_dir_all(&storage_root).unwrap();
        fs::write(storage_root.join("projects.sqlite"), b"not sqlite").unwrap();
        let migrations_root = repo_root().join("storage/sqlite");

        let error =
            run_storage_action(StorageAction::Verify, storage_root.clone(), migrations_root)
                .unwrap_err();

        assert_eq!(error.code, "database_invalid");

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn renders_health_report_as_json() {
        let report = HealthReport {
            agent: AGENT_NAME,
            version: "0.0.0-test",
            storage_root: PathBuf::from("E:/lab \"A\""),
            storage_root_exists: false,
            domains: vec!["metrology", "project_records"],
        };

        let json = report.to_json();

        assert!(json.contains("\"agent\":\"emc-locus-agent\""));
        assert!(json.contains("\"storage_root\":\"E:/lab \\\"A\\\"\""));
        assert!(json.contains("\"storage_root_exists\":false"));
        assert!(json.contains("\"domains\":[\"metrology\",\"project_records\"]"));
    }

    #[test]
    fn health_report_exposes_repository_domains() {
        let report = build_health_report(".");

        assert!(report.storage_root_exists);
        assert!(report.domains.contains(&"metrology"));
        assert!(report.domains.contains(&"project_records"));
        assert!(report.domains.contains(&"measurement_data"));
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("agent crate lives under crates")
            .to_path_buf()
    }

    fn temporary_storage_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "emc-locus-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        if root.exists() {
            remove_temporary_storage_root(&root);
        }
        root
    }

    fn force_journal_mode(database_path: &Path, mode: &str) {
        let connection = Connection::open(database_path).unwrap();
        let pragma = format!("PRAGMA journal_mode = {mode}");
        let observed: String = connection.query_row(&pragma, [], |row| row.get(0)).unwrap();
        assert_eq!(observed, mode.to_ascii_lowercase());
    }

    fn remove_temporary_storage_root(root: &Path) {
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }
}
