mod local_api;
mod project_agent;

use emc_locus_core::{baseline_repository_domains, RepositoryDomain};
pub use local_api::{run_local_api_server, ApiServerConfig};
pub use project_agent::{run_project_command, run_sync_command, ProjectAction, SyncAction};
use rusqlite::Connection;
use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
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
    details_json: Option<String>,
}

impl AgentError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details_json: None,
        }
    }

    pub(crate) fn with_details(
        code: &'static str,
        message: impl Into<String>,
        details_json: String,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details_json: Some(details_json),
        }
    }

    pub fn to_json(&self) -> String {
        let details = self
            .details_json
            .as_ref()
            .map_or_else(String::new, |details| {
                format!(",\n    \"details\": {details}")
            });
        format!(
            "{{\n  \"error\": {{\n    \"code\": {},\n    \"message\": {}{}\n  }}\n}}",
            json_string(self.code),
            json_string(&self.message),
            details
        )
    }
}

impl fmt::Display for AgentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
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
        let domains = self
            .domains
            .iter()
            .map(StorageDomainReport::to_json)
            .collect::<Vec<_>>()
            .join(",\n    ");
        format!(
            concat!(
                "{{\n",
                "  \"action\": {},\n",
                "  \"storage_root\": {},\n",
                "  \"migrations_root\": {},\n",
                "  \"domains\": [\n    {}\n  ]\n",
                "}}"
            ),
            json_string(self.action.as_str()),
            json_string(&self.storage_root.to_string_lossy()),
            json_string(&self.migrations_root.to_string_lossy()),
            domains
        )
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
}

impl StorageDomainReport {
    fn to_json(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "      \"domain\": {},\n",
                "      \"database_path\": {},\n",
                "      \"exists\": {},\n",
                "      \"schema_version\": {},\n",
                "      \"latest_migration\": {},\n",
                "      \"status\": {},\n",
                "      \"foreign_keys_enabled\": {},\n",
                "      \"integrity_check\": {}\n",
                "    }}"
            ),
            json_string(self.domain),
            json_string(&self.database_path.to_string_lossy()),
            self.exists,
            json_option_u32(self.schema_version),
            self.latest_migration,
            json_string(self.status.as_str()),
            json_option_bool(self.foreign_keys_enabled),
            json_option_string(self.integrity_check.as_deref())
        )
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
        let domains = self
            .domains
            .iter()
            .map(|domain| json_string(domain))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            concat!(
                "{{\n",
                "  \"agent\": {},\n",
                "  \"version\": {},\n",
                "  \"storage_root\": {},\n",
                "  \"storage_root_exists\": {},\n",
                "  \"domains\": [{}]\n",
                "}}"
            ),
            json_string(self.agent),
            json_string(self.version),
            json_string(&self.storage_root.to_string_lossy()),
            self.storage_root_exists,
            domains
        )
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

fn project_slice_domains() -> [StorageDomainSpec; 2] {
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
        apply_missing_migrations(&connection, &migrations)?;
    }

    let schema_version = current_schema_version(&connection)?;
    let foreign_keys_enabled = pragma_foreign_keys(&connection)?;
    let integrity_check = integrity_check(&connection)?;
    let status = if schema_version.is_none() {
        StorageDomainStatus::Invalid
    } else if schema_version.unwrap_or(0) < latest_migration {
        StorageDomainStatus::MigrationRequired
    } else if integrity_check.as_deref() == Some("ok") && foreign_keys_enabled {
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

pub(crate) fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

fn json_option_u32(value: Option<u32>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

fn json_option_bool(value: Option<bool>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

pub(crate) fn json_option_string(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_owned(), json_string)
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

        assert_eq!(report.domains.len(), 2);
        assert!(storage_root.join("projects.sqlite").exists());
        assert!(storage_root.join("sync.sqlite").exists());
        assert!(report
            .domains
            .iter()
            .all(|domain| domain.status == StorageDomainStatus::Current));
        assert_eq!(
            second_report
                .domains
                .iter()
                .find(|domain| domain.domain == "projects")
                .unwrap()
                .schema_version,
            Some(2)
        );

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

        assert!(json.contains("\"agent\": \"emc-locus-agent\""));
        assert!(json.contains("\"storage_root\": \"E:/lab \\\"A\\\"\""));
        assert!(json.contains("\"storage_root_exists\": false"));
        assert!(json.contains("\"domains\": [\"metrology\", \"project_records\"]"));
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

    fn remove_temporary_storage_root(root: &Path) {
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }
}
