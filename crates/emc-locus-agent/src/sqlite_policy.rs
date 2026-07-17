use crate::AgentError;
use rusqlite::Connection;
use serde_json::json;

const PROJECT_SLICE_JOURNAL_MODE: &str = "DELETE";
const ATOMIC_MULTI_DATABASE_JOURNAL_MODES: [&str; 3] = ["delete", "truncate", "persist"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AttachedDatabase {
    Main,
    SyncDb,
    TestDefinitionsDb,
    StationDb,
    MetrologyDb,
    EquipmentDb,
}

impl AttachedDatabase {
    fn schema(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::SyncDb => "sync_db",
            Self::TestDefinitionsDb => "test_definitions_db",
            Self::StationDb => "station_db",
            Self::MetrologyDb => "metrology_db",
            Self::EquipmentDb => "equipment_db",
        }
    }
}

pub(crate) fn initialize_project_slice_journal_mode(
    connection: &Connection,
    database: AttachedDatabase,
    database_label: &str,
) -> Result<String, AgentError> {
    let mode = set_journal_mode(connection, database, PROJECT_SLICE_JOURNAL_MODE)?;
    ensure_multi_database_atomicity_mode(&mode, database_label)?;
    Ok(mode)
}

pub(crate) fn enforce_project_slice_journal_mode(
    connection: &Connection,
    database: AttachedDatabase,
    database_label: &str,
) -> Result<String, AgentError> {
    let mode = journal_mode(connection, database)?;
    ensure_multi_database_atomicity_mode(&mode, database_label)?;
    Ok(mode)
}

pub(crate) fn journal_mode(
    connection: &Connection,
    database: AttachedDatabase,
) -> Result<String, AgentError> {
    let pragma = format!("PRAGMA {}.journal_mode", database.schema());
    let mode: String = connection
        .query_row(&pragma, [], |row| row.get(0))
        .map_err(|error| AgentError::new("database_pragma_error", error.to_string()))?;
    Ok(normalize_journal_mode(&mode))
}

pub(crate) fn supports_multi_database_atomicity(mode: &str) -> bool {
    let mode = normalize_journal_mode(mode);
    ATOMIC_MULTI_DATABASE_JOURNAL_MODES.contains(&mode.as_str())
}

fn set_journal_mode(
    connection: &Connection,
    database: AttachedDatabase,
    mode: &str,
) -> Result<String, AgentError> {
    let pragma = format!("PRAGMA {}.journal_mode = {mode}", database.schema());
    let mode: String = connection
        .query_row(&pragma, [], |row| row.get(0))
        .map_err(|error| AgentError::new("database_pragma_error", error.to_string()))?;
    Ok(normalize_journal_mode(&mode))
}

fn ensure_multi_database_atomicity_mode(
    mode: &str,
    database_label: &str,
) -> Result<(), AgentError> {
    if supports_multi_database_atomicity(mode) {
        return Ok(());
    }
    Err(AgentError::with_details(
        "storage_journal_mode_incompatible",
        format!(
            "{database_label} uses journal_mode={mode}, which is incompatible with the Local Agent multi-SQLite transaction policy"
        ),
        json!({
            "database": database_label,
            "journal_mode": normalize_journal_mode(mode),
            "compatible_journal_modes": ATOMIC_MULTI_DATABASE_JOURNAL_MODES,
            "policy": "Local Agent multi-database writes require rollback-journal modes so SQLite can keep attached database commits atomic",
        }),
    ))
}

fn normalize_journal_mode(mode: &str) -> String {
    mode.trim().to_ascii_lowercase()
}
