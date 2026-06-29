use super::{json_option_string, json_string, AgentCommand, AgentError};
use emc_locus_core::{
    can_transition, required_contract_review_items, AuditActor, AuditReason,
    ContractReviewChecklist, ContractReviewItem, DomainError, ExecutionMode, Project, ProjectCode,
    ProjectStage, StableId,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectAction {
    Create(CreateProjectInput),
    List,
    Get { code: String },
    ContractReview { code: String },
    CompleteReviewItem(CompleteReviewItemInput),
    ToTestPlanning(AdvanceToTestPlanningInput),
    AuditEvents { code: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncAction {
    Outbox,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateProjectInput {
    pub code: String,
    pub customer_name: String,
    pub execution_mode: String,
    pub stage: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompleteReviewItemInput {
    pub code: String,
    pub item: String,
    pub actor: String,
    pub comment: Option<String>,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdvanceToTestPlanningInput {
    pub code: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
    pub deviation_authorized_by: Option<String>,
    pub deviation_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StoredProject {
    code: String,
    customer_name: String,
    stage: String,
    execution_mode: String,
    created_at: String,
    archived_at: Option<String>,
    revision_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StoredOperation {
    operation_id: String,
    entity_id: String,
    operation_kind: String,
    resulting_revision: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ContractReviewStatus {
    project_code: String,
    execution_mode: ExecutionMode,
    required_items: Vec<ContractReviewItem>,
    completed_items: Vec<CompletedReviewItem>,
    missing_items: Vec<ContractReviewItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompletedReviewItem {
    item: String,
    completed_by: Option<String>,
    completed_at: Option<String>,
    comment: Option<String>,
}

pub(crate) fn parse_project_args<I>(mut args: I) -> Result<AgentCommand, AgentError>
where
    I: Iterator<Item = String>,
{
    let action = args
        .next()
        .ok_or_else(|| AgentError::new("missing_project_action", "missing project action"))?;
    let mut flags = parse_flags(args)?;
    let storage_root = required_path(&mut flags, "--storage-root")?;
    let action = match action.as_str() {
        "create" => {
            let operation_id = required_value(&mut flags, "--operation-id")?;
            let correlation_id = optional_value(&mut flags, "--correlation-id")
                .unwrap_or_else(|| operation_id.clone());
            ProjectAction::Create(CreateProjectInput {
                code: required_value(&mut flags, "--code")?,
                customer_name: required_value(&mut flags, "--customer-name")?,
                execution_mode: required_value(&mut flags, "--execution-mode")?,
                stage: optional_value(&mut flags, "--stage")
                    .unwrap_or_else(|| "contract_review".to_owned()),
                actor: required_value(&mut flags, "--actor")?,
                reason: required_value(&mut flags, "--reason")?,
                operation_id,
                correlation_id,
                device_id: optional_value(&mut flags, "--device-id")
                    .unwrap_or_else(|| "local-agent".to_owned()),
            })
        }
        "list" => ProjectAction::List,
        "get" => ProjectAction::Get {
            code: required_value(&mut flags, "--code")?,
        },
        "contract-review" => ProjectAction::ContractReview {
            code: required_value(&mut flags, "--code")?,
        },
        "complete-review-item" => {
            let operation_id = required_value(&mut flags, "--operation-id")?;
            let correlation_id = optional_value(&mut flags, "--correlation-id")
                .unwrap_or_else(|| operation_id.clone());
            ProjectAction::CompleteReviewItem(CompleteReviewItemInput {
                code: required_value(&mut flags, "--code")?,
                item: required_value(&mut flags, "--item")?,
                actor: required_value(&mut flags, "--actor")?,
                comment: optional_value(&mut flags, "--comment"),
                operation_id,
                correlation_id,
                device_id: optional_value(&mut flags, "--device-id")
                    .unwrap_or_else(|| "local-agent".to_owned()),
            })
        }
        "to-test-planning" => {
            let operation_id = required_value(&mut flags, "--operation-id")?;
            let correlation_id = optional_value(&mut flags, "--correlation-id")
                .unwrap_or_else(|| operation_id.clone());
            ProjectAction::ToTestPlanning(AdvanceToTestPlanningInput {
                code: required_value(&mut flags, "--code")?,
                actor: required_value(&mut flags, "--actor")?,
                reason: required_value(&mut flags, "--reason")?,
                operation_id,
                correlation_id,
                device_id: optional_value(&mut flags, "--device-id")
                    .unwrap_or_else(|| "local-agent".to_owned()),
                deviation_authorized_by: optional_value(&mut flags, "--deviation-authorized-by"),
                deviation_reason: optional_value(&mut flags, "--deviation-reason"),
            })
        }
        "audit-events" => ProjectAction::AuditEvents {
            code: required_value(&mut flags, "--code")?,
        },
        other => {
            return Err(AgentError::new(
                "unknown_project_action",
                format!("unknown project action: {other}"),
            ))
        }
    };
    ensure_no_unknown_flags(flags)?;

    Ok(AgentCommand::Projects {
        action,
        storage_root,
    })
}

pub(crate) fn parse_sync_args<I>(mut args: I) -> Result<AgentCommand, AgentError>
where
    I: Iterator<Item = String>,
{
    let action = args
        .next()
        .ok_or_else(|| AgentError::new("missing_sync_action", "missing sync action"))?;
    let mut flags = parse_flags(args)?;
    let storage_root = required_path(&mut flags, "--storage-root")?;
    let action = match action.as_str() {
        "outbox" => SyncAction::Outbox,
        other => {
            return Err(AgentError::new(
                "unknown_sync_action",
                format!("unknown sync action: {other}"),
            ))
        }
    };
    ensure_no_unknown_flags(flags)?;

    Ok(AgentCommand::Sync {
        action,
        storage_root,
    })
}

pub fn run_project_command(command: AgentCommand) -> Result<String, AgentError> {
    match command {
        AgentCommand::Projects {
            action,
            storage_root,
        } => run_project_action(action, storage_root),
        _ => Err(AgentError::new(
            "invalid_project_command",
            "expected a projects command",
        )),
    }
}

pub fn run_sync_command(command: AgentCommand) -> Result<String, AgentError> {
    match command {
        AgentCommand::Sync {
            action,
            storage_root,
        } => match action {
            SyncAction::Outbox => list_sync_outbox(&storage_root),
        },
        _ => Err(AgentError::new(
            "invalid_sync_command",
            "expected a sync command",
        )),
    }
}

fn run_project_action(action: ProjectAction, storage_root: PathBuf) -> Result<String, AgentError> {
    match action {
        ProjectAction::Create(input) => create_project(&storage_root, input),
        ProjectAction::List => list_projects(&storage_root),
        ProjectAction::Get { code } => get_project(&storage_root, &code),
        ProjectAction::ContractReview { code } => get_contract_review(&storage_root, &code),
        ProjectAction::CompleteReviewItem(input) => complete_review_item(&storage_root, input),
        ProjectAction::ToTestPlanning(input) => advance_to_test_planning(&storage_root, input),
        ProjectAction::AuditEvents { code } => list_audit_events(&storage_root, &code),
    }
}

fn create_project(storage_root: &Path, input: CreateProjectInput) -> Result<String, AgentError> {
    let code = ProjectCode::parse(input.code.clone()).map_err(domain_error)?;
    let execution_mode = parse_execution_mode(&input.execution_mode)?;
    let stage = parse_create_stage(&input.stage)?;
    let actor = AuditActor::parse(input.actor.clone()).map_err(domain_error)?;
    AuditReason::parse(input.reason.clone()).map_err(domain_error)?;
    validate_stable_id(&input.operation_id, "operation_id")?;
    validate_stable_id(&input.correlation_id, "correlation_id")?;
    validate_stable_id(&input.device_id, "device_id")?;

    let mut project =
        Project::new(code.clone(), input.customer_name.clone()).map_err(domain_error)?;
    if stage == ProjectStage::ContractReview {
        project
            .advance_to(ProjectStage::ContractReview)
            .map_err(domain_error)?;
    }

    let mut connection = open_project_connection(storage_root)?;
    if let Some(operation) = existing_operation(&connection, &input.operation_id)? {
        ensure_operation_replay(
            &operation,
            code.as_str(),
            "project_created",
            &input.operation_id,
        )?;
        let project = load_project(&connection, code.as_str())?.ok_or_else(|| {
            AgentError::new(
                "operation_replay_missing_entity",
                "operation exists but project is missing",
            )
        })?;
        return Ok(project_result_json(
            "project_created",
            &project,
            true,
            &operation.operation_id,
        ));
    }
    if load_project(&connection, code.as_str())?.is_some() {
        return Err(AgentError::new(
            "project_already_exists",
            format!("project already exists: {}", code.as_str()),
        ));
    }

    let now = utc_timestamp()?;
    let stage_slug = project_stage_slug(stage);
    let mode_slug = execution_mode_slug(execution_mode);
    let payload_json =
        project_payload_json(code.as_str(), &input.customer_name, mode_slug, stage_slug);
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    transaction
        .execute(
            concat!(
                "INSERT INTO projects (code, customer_name, stage, execution_mode, created_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5)"
            ),
            params![
                code.as_str(),
                input.customer_name.trim(),
                stage_slug,
                mode_slug,
                now
            ],
        )
        .map_err(|error| AgentError::new("project_write_failed", error.to_string()))?;
    insert_audit_event(
        &transaction,
        AuditEventInput {
            project_code: code.as_str(),
            sequence: 1,
            actor: actor.as_str(),
            action: "project_created",
            reason: Some(&input.reason),
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    insert_sync_operation(
        &transaction,
        SyncOperationInput {
            operation_id: &input.operation_id,
            entity_id: code.as_str(),
            operation_kind: "project_created",
            base_revision: "rev-0000",
            resulting_revision: "rev-0001",
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    let project = load_project(&connection, code.as_str())?.ok_or_else(|| {
        AgentError::new(
            "project_read_failed",
            "created project could not be reloaded",
        )
    })?;
    Ok(project_result_json(
        "project_created",
        &project,
        false,
        &input.operation_id,
    ))
}

fn list_projects(storage_root: &Path) -> Result<String, AgentError> {
    let connection = open_project_connection(storage_root)?;
    let projects = load_projects(&connection)?;
    let projects_json = projects
        .iter()
        .map(project_json)
        .collect::<Vec<_>>()
        .join(",\n    ");
    Ok(format!(
        "{{\n  \"projects\": [\n    {}\n  ]\n}}",
        projects_json
    ))
}

fn get_project(storage_root: &Path, code: &str) -> Result<String, AgentError> {
    let code = ProjectCode::parse(code).map_err(domain_error)?;
    let connection = open_project_connection(storage_root)?;
    let project = load_project(&connection, code.as_str())?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    Ok(format!("{{\n  \"project\": {}\n}}", project_json(&project)))
}

fn get_contract_review(storage_root: &Path, code: &str) -> Result<String, AgentError> {
    let code = ProjectCode::parse(code).map_err(domain_error)?;
    let connection = open_project_connection(storage_root)?;
    let status = load_contract_review_status(&connection, code.as_str())?;
    Ok(contract_review_json(&status))
}

fn complete_review_item(
    storage_root: &Path,
    input: CompleteReviewItemInput,
) -> Result<String, AgentError> {
    let code = ProjectCode::parse(input.code.clone()).map_err(domain_error)?;
    let item = parse_contract_review_item(&input.item)?;
    let actor = AuditActor::parse(input.actor.clone()).map_err(domain_error)?;
    validate_stable_id(&input.operation_id, "operation_id")?;
    validate_stable_id(&input.correlation_id, "correlation_id")?;
    validate_stable_id(&input.device_id, "device_id")?;

    let mut connection = open_project_connection(storage_root)?;
    if let Some(operation) = existing_operation(&connection, &input.operation_id)? {
        ensure_operation_replay(
            &operation,
            code.as_str(),
            "contract_review_item_completed",
            &input.operation_id,
        )?;
        let status = load_contract_review_status(&connection, code.as_str())?;
        return Ok(review_item_result_json(
            &status,
            true,
            false,
            &operation.resulting_revision,
            &input.operation_id,
        ));
    }
    load_project(&connection, code.as_str())?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    let canonical_item = contract_review_item_slug(item);
    if is_review_item_completed(&connection, code.as_str(), canonical_item)? {
        let status = load_contract_review_status(&connection, code.as_str())?;
        return Ok(review_item_result_json(
            &status,
            false,
            true,
            &revision_text(status.completed_items.len() as u64),
            &input.operation_id,
        ));
    }

    let now = utc_timestamp()?;
    let next_sequence = next_audit_sequence(&connection, code.as_str())?;
    let base_revision = revision_text(next_sequence.saturating_sub(1));
    let resulting_revision = revision_text(next_sequence);
    let comment = input
        .comment
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let payload_json = review_item_payload_json(canonical_item, comment);
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    transaction
        .execute(
            concat!(
                "INSERT INTO contract_review_items ",
                "(project_code, item, completed, completed_by, completed_at, comment) ",
                "VALUES (?1, ?2, 1, ?3, ?4, ?5) ",
                "ON CONFLICT(project_code, item) DO UPDATE SET ",
                "completed = 1, completed_by = excluded.completed_by, ",
                "completed_at = excluded.completed_at, comment = excluded.comment"
            ),
            params![code.as_str(), canonical_item, actor.as_str(), now, comment],
        )
        .map_err(|error| AgentError::new("contract_review_write_failed", error.to_string()))?;
    insert_audit_event(
        &transaction,
        AuditEventInput {
            project_code: code.as_str(),
            sequence: next_sequence,
            actor: actor.as_str(),
            action: "contract_review_item_completed",
            reason: comment,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    insert_sync_operation(
        &transaction,
        SyncOperationInput {
            operation_id: &input.operation_id,
            entity_id: code.as_str(),
            operation_kind: "contract_review_item_completed",
            base_revision: &base_revision,
            resulting_revision: &resulting_revision,
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    let status = load_contract_review_status(&connection, code.as_str())?;
    Ok(review_item_result_json(
        &status,
        false,
        false,
        &resulting_revision,
        &input.operation_id,
    ))
}

fn advance_to_test_planning(
    storage_root: &Path,
    input: AdvanceToTestPlanningInput,
) -> Result<String, AgentError> {
    let code = ProjectCode::parse(input.code.clone()).map_err(domain_error)?;
    let actor = AuditActor::parse(input.actor.clone()).map_err(domain_error)?;
    AuditReason::parse(input.reason.clone()).map_err(domain_error)?;
    validate_stable_id(&input.operation_id, "operation_id")?;
    validate_stable_id(&input.correlation_id, "correlation_id")?;
    validate_stable_id(&input.device_id, "device_id")?;

    let mut connection = open_project_connection(storage_root)?;
    if let Some(operation) = existing_operation(&connection, &input.operation_id)? {
        ensure_operation_replay(
            &operation,
            code.as_str(),
            "project_stage_advanced",
            &input.operation_id,
        )?;
        let project = load_project(&connection, code.as_str())?.ok_or_else(|| {
            AgentError::new(
                "operation_replay_missing_entity",
                "operation exists but project is missing",
            )
        })?;
        return Ok(project_result_json(
            "project_stage_advanced",
            &project,
            true,
            &operation.operation_id,
        ));
    }

    let project = load_project(&connection, code.as_str())?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    let current_stage = parse_project_stage(&project.stage)?;
    if !can_transition(current_stage, ProjectStage::TestPlanning) {
        return Err(AgentError::with_details(
            "invalid_project_transition",
            "project cannot transition to test_planning from its current stage",
            format!(
                "{{\"from\":{},\"to\":\"test_planning\"}}",
                json_string(&project.stage)
            ),
        ));
    }

    let status = load_contract_review_status(&connection, code.as_str())?;
    let deviation = validate_deviation(&input, status.execution_mode, &status.missing_items)?;
    if !status.missing_items.is_empty() && deviation.is_none() {
        return Err(AgentError::with_details(
            "contract_review_incomplete",
            "Contract review is incomplete",
            missing_items_details_json(&status.missing_items),
        ));
    }

    let now = utc_timestamp()?;
    let first_sequence = next_audit_sequence(&connection, code.as_str())?;
    let mut final_sequence = first_sequence;
    let base_revision = revision_text(first_sequence.saturating_sub(1));
    let transition_payload = transition_payload_json(&project.stage, "test_planning");
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;

    if let Some((authorized_by, reason)) = deviation {
        let deviation_payload = deviation_payload_json(&status.missing_items);
        insert_audit_event(
            &transaction,
            AuditEventInput {
                project_code: code.as_str(),
                sequence: first_sequence,
                actor: &authorized_by,
                action: "contract_review_deviation_authorized",
                reason: Some(&reason),
                payload_json: &deviation_payload,
                timestamp: &now,
            },
        )?;
        final_sequence += 1;
    }

    transaction
        .execute(
            "UPDATE projects SET stage = 'test_planning' WHERE code = ?1",
            params![code.as_str()],
        )
        .map_err(|error| AgentError::new("project_write_failed", error.to_string()))?;
    insert_audit_event(
        &transaction,
        AuditEventInput {
            project_code: code.as_str(),
            sequence: final_sequence,
            actor: actor.as_str(),
            action: "project_stage_advanced",
            reason: Some(&input.reason),
            payload_json: &transition_payload,
            timestamp: &now,
        },
    )?;
    let resulting_revision = revision_text(final_sequence);
    insert_sync_operation(
        &transaction,
        SyncOperationInput {
            operation_id: &input.operation_id,
            entity_id: code.as_str(),
            operation_kind: "project_stage_advanced",
            base_revision: &base_revision,
            resulting_revision: &resulting_revision,
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &transition_payload,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    let project = load_project(&connection, code.as_str())?.ok_or_else(|| {
        AgentError::new(
            "project_read_failed",
            "updated project could not be reloaded",
        )
    })?;
    Ok(project_result_json(
        "project_stage_advanced",
        &project,
        false,
        &input.operation_id,
    ))
}

fn list_audit_events(storage_root: &Path, code: &str) -> Result<String, AgentError> {
    let code = ProjectCode::parse(code).map_err(domain_error)?;
    let connection = open_project_connection(storage_root)?;
    load_project(&connection, code.as_str())?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    let mut statement = connection
        .prepare(concat!(
            "SELECT sequence, actor, action, reason, payload_json, occurred_at ",
            "FROM project_audit_events WHERE project_code = ?1 ORDER BY sequence"
        ))
        .map_err(|error| AgentError::new("audit_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![code.as_str()], |row| {
            Ok(format!(
                concat!(
                    "{{\"sequence\":{},\"actor\":{},\"action\":{},",
                    "\"reason\":{},\"payload_json\":{},\"occurred_at\":{}}}"
                ),
                row.get::<_, u64>(0)?,
                json_string(&row.get::<_, String>(1)?),
                json_string(&row.get::<_, String>(2)?),
                json_option_string(row.get::<_, Option<String>>(3)?.as_deref()),
                json_string(&row.get::<_, String>(4)?),
                json_string(&row.get::<_, String>(5)?)
            ))
        })
        .map_err(|error| AgentError::new("audit_query_failed", error.to_string()))?;
    let mut events = Vec::new();
    for row in rows {
        events.push(row.map_err(|error| AgentError::new("audit_query_failed", error.to_string()))?);
    }
    Ok(format!(
        "{{\n  \"project_code\": {},\n  \"audit_events\": [\n    {}\n  ]\n}}",
        json_string(code.as_str()),
        events.join(",\n    ")
    ))
}

fn list_sync_outbox(storage_root: &Path) -> Result<String, AgentError> {
    let connection = open_project_connection(storage_root)?;
    let mut statement = connection
        .prepare(
            concat!(
                "SELECT operation_id, domain, entity_type, entity_id, operation_kind, ",
                "base_revision, resulting_revision, actor_id, device_id, correlation_id, ",
                "payload_json, payload_checksum, status, occurred_at, recorded_at ",
                "FROM sync_db.sync_operations WHERE status = 'pending' ORDER BY recorded_at, operation_id"
            ),
        )
        .map_err(|error| AgentError::new("sync_outbox_query_failed", error.to_string()))?;
    let rows = statement
        .query_map([], |row| {
            Ok(format!(
                concat!(
                    "{{\"operation_id\":{},\"domain\":{},\"entity_type\":{},\"entity_id\":{},",
                    "\"operation_kind\":{},\"base_revision\":{},\"resulting_revision\":{},",
                    "\"actor_id\":{},\"device_id\":{},\"correlation_id\":{},",
                    "\"payload_json\":{},\"payload_checksum\":{},\"status\":{},",
                    "\"occurred_at\":{},\"recorded_at\":{}}}"
                ),
                json_string(&row.get::<_, String>(0)?),
                json_string(&row.get::<_, String>(1)?),
                json_string(&row.get::<_, String>(2)?),
                json_string(&row.get::<_, String>(3)?),
                json_string(&row.get::<_, String>(4)?),
                json_string(&row.get::<_, String>(5)?),
                json_string(&row.get::<_, String>(6)?),
                json_string(&row.get::<_, String>(7)?),
                json_string(&row.get::<_, String>(8)?),
                json_string(&row.get::<_, String>(9)?),
                json_string(&row.get::<_, String>(10)?),
                json_string(&row.get::<_, String>(11)?),
                json_string(&row.get::<_, String>(12)?),
                json_string(&row.get::<_, String>(13)?),
                json_string(&row.get::<_, String>(14)?)
            ))
        })
        .map_err(|error| AgentError::new("sync_outbox_query_failed", error.to_string()))?;
    let mut operations = Vec::new();
    for row in rows {
        operations.push(
            row.map_err(|error| AgentError::new("sync_outbox_query_failed", error.to_string()))?,
        );
    }
    Ok(format!(
        "{{\n  \"sync_outbox\": [\n    {}\n  ]\n}}",
        operations.join(",\n    ")
    ))
}

fn open_project_connection(storage_root: &Path) -> Result<Connection, AgentError> {
    let projects_database = storage_root.join("projects.sqlite");
    let sync_database = storage_root.join("sync.sqlite");
    if !projects_database.exists() || !sync_database.exists() {
        return Err(AgentError::new(
            "storage_not_initialized",
            "project commands require initialized projects.sqlite and sync.sqlite",
        ));
    }
    let connection = Connection::open(&projects_database).map_err(|error| {
        AgentError::new(
            "database_open_error",
            format!("cannot open {}: {error}", projects_database.display()),
        )
    })?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| AgentError::new("database_pragma_error", error.to_string()))?;
    let sync_path = sync_database.to_string_lossy().to_string();
    connection
        .execute("ATTACH DATABASE ?1 AS sync_db", params![sync_path])
        .map_err(|error| AgentError::new("database_attach_error", error.to_string()))?;
    ensure_project_tables(&connection)?;
    Ok(connection)
}

fn ensure_project_tables(connection: &Connection) -> Result<(), AgentError> {
    for (schema, table) in [
        ("main", "projects"),
        ("main", "project_audit_events"),
        ("main", "contract_review_items"),
        ("sync_db", "sync_operations"),
    ] {
        if !table_exists(connection, schema, table)? {
            return Err(AgentError::new(
                "storage_not_initialized",
                format!("missing required table {schema}.{table}"),
            ));
        }
    }
    Ok(())
}

fn table_exists(connection: &Connection, schema: &str, table: &str) -> Result<bool, AgentError> {
    let sql =
        format!("SELECT COUNT(*) FROM {schema}.sqlite_master WHERE type = 'table' AND name = ?1");
    let count: u32 = connection
        .query_row(&sql, params![table], |row| row.get(0))
        .map_err(|error| AgentError::new("database_invalid", error.to_string()))?;
    Ok(count > 0)
}

fn load_project(connection: &Connection, code: &str) -> Result<Option<StoredProject>, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT p.code, p.customer_name, p.stage, p.execution_mode, p.created_at, p.archived_at, ",
                "COALESCE((SELECT MAX(sequence) FROM project_audit_events e WHERE e.project_code = p.code), 0) AS revision_sequence ",
                "FROM projects p WHERE p.code = ?1"
            ),
            params![code],
            stored_project_from_row,
        )
        .optional()
        .map_err(|error| AgentError::new("project_query_failed", error.to_string()))
}

fn load_projects(connection: &Connection) -> Result<Vec<StoredProject>, AgentError> {
    let mut statement = connection
        .prepare(
            concat!(
                "SELECT p.code, p.customer_name, p.stage, p.execution_mode, p.created_at, p.archived_at, ",
                "COALESCE((SELECT MAX(sequence) FROM project_audit_events e WHERE e.project_code = p.code), 0) AS revision_sequence ",
                "FROM projects p ORDER BY p.code"
            ),
        )
        .map_err(|error| AgentError::new("project_query_failed", error.to_string()))?;
    let rows = statement
        .query_map([], stored_project_from_row)
        .map_err(|error| AgentError::new("project_query_failed", error.to_string()))?;
    let mut projects = Vec::new();
    for row in rows {
        projects
            .push(row.map_err(|error| AgentError::new("project_query_failed", error.to_string()))?);
    }
    Ok(projects)
}

fn stored_project_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredProject> {
    Ok(StoredProject {
        code: row.get(0)?,
        customer_name: row.get(1)?,
        stage: row.get(2)?,
        execution_mode: row.get(3)?,
        created_at: row.get(4)?,
        archived_at: row.get(5)?,
        revision_sequence: row.get(6)?,
    })
}

fn load_contract_review_status(
    connection: &Connection,
    code: &str,
) -> Result<ContractReviewStatus, AgentError> {
    let project = load_project(connection, code)?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    let project_code = ProjectCode::parse(code).map_err(domain_error)?;
    let execution_mode = parse_execution_mode(&project.execution_mode)?;
    let mut checklist = ContractReviewChecklist::new(project_code);
    let mut completed_items = Vec::new();
    let mut statement = connection
        .prepare(concat!(
            "SELECT item, completed_by, completed_at, comment ",
            "FROM contract_review_items WHERE project_code = ?1 AND completed = 1 ORDER BY item"
        ))
        .map_err(|error| AgentError::new("contract_review_query_failed", error.to_string()))?;
    let rows = statement
        .query_map(params![code], |row| {
            Ok(CompletedReviewItem {
                item: row.get(0)?,
                completed_by: row.get(1)?,
                completed_at: row.get(2)?,
                comment: row.get(3)?,
            })
        })
        .map_err(|error| AgentError::new("contract_review_query_failed", error.to_string()))?;
    for row in rows {
        let completed = row
            .map_err(|error| AgentError::new("contract_review_query_failed", error.to_string()))?;
        if let Ok(item) = parse_contract_review_item(&completed.item) {
            checklist.mark_complete(item);
        }
        completed_items.push(completed);
    }
    let required_items = required_contract_review_items(execution_mode);
    let missing_items = checklist.missing_items_for_mode(execution_mode);
    Ok(ContractReviewStatus {
        project_code: code.to_owned(),
        execution_mode,
        required_items,
        completed_items,
        missing_items,
    })
}

fn is_review_item_completed(
    connection: &Connection,
    code: &str,
    item: &str,
) -> Result<bool, AgentError> {
    let completed = connection
        .query_row(
            concat!(
                "SELECT completed FROM contract_review_items ",
                "WHERE project_code = ?1 AND item = ?2"
            ),
            params![code, item],
            |row| row.get::<_, u8>(0),
        )
        .optional()
        .map_err(|error| AgentError::new("contract_review_query_failed", error.to_string()))?;
    Ok(completed == Some(1))
}

fn existing_operation(
    connection: &Connection,
    operation_id: &str,
) -> Result<Option<StoredOperation>, AgentError> {
    connection
        .query_row(
            concat!(
                "SELECT operation_id, entity_id, operation_kind, resulting_revision ",
                "FROM sync_db.sync_operations WHERE operation_id = ?1"
            ),
            params![operation_id],
            |row| {
                Ok(StoredOperation {
                    operation_id: row.get(0)?,
                    entity_id: row.get(1)?,
                    operation_kind: row.get(2)?,
                    resulting_revision: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|error| AgentError::new("sync_outbox_query_failed", error.to_string()))
}

fn ensure_operation_replay(
    operation: &StoredOperation,
    entity_id: &str,
    operation_kind: &str,
    operation_id: &str,
) -> Result<(), AgentError> {
    if operation.entity_id == entity_id && operation.operation_kind == operation_kind {
        return Ok(());
    }
    Err(AgentError::with_details(
        "operation_replay_mismatch",
        "operation_id is already used for a different command",
        format!(
            "{{\"operation_id\":{},\"existing_entity_id\":{},\"existing_operation_kind\":{}}}",
            json_string(operation_id),
            json_string(&operation.entity_id),
            json_string(&operation.operation_kind)
        ),
    ))
}

fn next_audit_sequence(connection: &Connection, code: &str) -> Result<u64, AgentError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(sequence), 0) + 1 FROM project_audit_events WHERE project_code = ?1",
            params![code],
            |row| row.get(0),
        )
        .map_err(|error| AgentError::new("audit_query_failed", error.to_string()))
}

struct AuditEventInput<'a> {
    project_code: &'a str,
    sequence: u64,
    actor: &'a str,
    action: &'a str,
    reason: Option<&'a str>,
    payload_json: &'a str,
    timestamp: &'a str,
}

fn insert_audit_event(
    transaction: &Transaction<'_>,
    input: AuditEventInput<'_>,
) -> Result<(), AgentError> {
    transaction
        .execute(
            concat!(
                "INSERT INTO project_audit_events ",
                "(project_code, sequence, actor, action, reason, payload_json, occurred_at) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
            ),
            params![
                input.project_code,
                input.sequence,
                input.actor,
                input.action,
                input.reason,
                input.payload_json,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("audit_write_failed", error.to_string()))?;
    Ok(())
}

struct SyncOperationInput<'a> {
    operation_id: &'a str,
    entity_id: &'a str,
    operation_kind: &'a str,
    base_revision: &'a str,
    resulting_revision: &'a str,
    actor_id: &'a str,
    device_id: &'a str,
    correlation_id: &'a str,
    payload_json: &'a str,
    timestamp: &'a str,
}

fn insert_sync_operation(
    transaction: &Transaction<'_>,
    input: SyncOperationInput<'_>,
) -> Result<(), AgentError> {
    let checksum = payload_checksum(input.payload_json);
    transaction
        .execute(
            concat!(
                "INSERT INTO sync_db.sync_operations ",
                "(operation_id, domain, entity_type, entity_id, operation_kind, ",
                "base_revision, resulting_revision, actor_id, device_id, correlation_id, ",
                "payload_json, payload_checksum, status, occurred_at, recorded_at) ",
                "VALUES (?1, 'project_records', 'project', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'pending', ?11, ?11)"
            ),
            params![
                input.operation_id,
                input.entity_id,
                input.operation_kind,
                input.base_revision,
                input.resulting_revision,
                input.actor_id,
                input.device_id,
                input.correlation_id,
                input.payload_json,
                checksum,
                input.timestamp
            ],
        )
        .map_err(|error| AgentError::new("sync_outbox_write_failed", error.to_string()))?;
    Ok(())
}

fn parse_flags<I>(args: I) -> Result<BTreeMap<String, String>, AgentError>
where
    I: Iterator<Item = String>,
{
    let mut flags = BTreeMap::new();
    let mut args = args.peekable();
    while let Some(argument) = args.next() {
        if !argument.starts_with("--") {
            return Err(AgentError::new(
                "unknown_argument",
                format!("unknown argument: {argument}"),
            ));
        }
        let value = args.next().ok_or_else(|| {
            AgentError::new("missing_argument", format!("missing value for {argument}"))
        })?;
        flags.insert(argument, value);
    }
    Ok(flags)
}

fn required_path(
    flags: &mut BTreeMap<String, String>,
    name: &'static str,
) -> Result<PathBuf, AgentError> {
    Ok(PathBuf::from(required_value(flags, name)?))
}

fn required_value(
    flags: &mut BTreeMap<String, String>,
    name: &'static str,
) -> Result<String, AgentError> {
    optional_value(flags, name)
        .ok_or_else(|| AgentError::new("missing_argument", format!("missing {name}")))
}

fn optional_value(flags: &mut BTreeMap<String, String>, name: &str) -> Option<String> {
    flags.remove(name)
}

fn ensure_no_unknown_flags(flags: BTreeMap<String, String>) -> Result<(), AgentError> {
    if let Some(name) = flags.keys().next() {
        return Err(AgentError::new(
            "unknown_argument",
            format!("unknown argument: {name}"),
        ));
    }
    Ok(())
}

fn parse_create_stage(stage: &str) -> Result<ProjectStage, AgentError> {
    let stage = parse_project_stage(stage)?;
    if matches!(
        stage,
        ProjectStage::Quotation | ProjectStage::ContractReview
    ) {
        Ok(stage)
    } else {
        Err(AgentError::new(
            "invalid_initial_project_stage",
            "project creation only supports quotation or contract_review",
        ))
    }
}

fn parse_project_stage(stage: &str) -> Result<ProjectStage, AgentError> {
    match stage {
        "quotation" => Ok(ProjectStage::Quotation),
        "contract_review" => Ok(ProjectStage::ContractReview),
        "test_planning" => Ok(ProjectStage::TestPlanning),
        "measuring" => Ok(ProjectStage::Measuring),
        "technical_review" => Ok(ProjectStage::TechnicalReview),
        "report_issued" => Ok(ProjectStage::ReportIssued),
        "archived" => Ok(ProjectStage::Archived),
        other => Err(AgentError::new(
            "unknown_project_stage",
            format!("unknown project stage: {other}"),
        )),
    }
}

fn project_stage_slug(stage: ProjectStage) -> &'static str {
    match stage {
        ProjectStage::Quotation => "quotation",
        ProjectStage::ContractReview => "contract_review",
        ProjectStage::TestPlanning => "test_planning",
        ProjectStage::Measuring => "measuring",
        ProjectStage::TechnicalReview => "technical_review",
        ProjectStage::ReportIssued => "report_issued",
        ProjectStage::Archived => "archived",
    }
}

fn parse_execution_mode(mode: &str) -> Result<ExecutionMode, AgentError> {
    match mode {
        "accredited" => Ok(ExecutionMode::Accredited),
        "non_accredited" => Ok(ExecutionMode::NonAccredited),
        "investigation" => Ok(ExecutionMode::Investigation),
        other => Err(AgentError::new(
            "unknown_execution_mode",
            format!("unknown execution mode: {other}"),
        )),
    }
}

fn execution_mode_slug(mode: ExecutionMode) -> &'static str {
    match mode {
        ExecutionMode::Accredited => "accredited",
        ExecutionMode::NonAccredited => "non_accredited",
        ExecutionMode::Investigation => "investigation",
    }
}

fn parse_contract_review_item(item: &str) -> Result<ContractReviewItem, AgentError> {
    match item {
        "customer_request_defined"
        | "requirements_reviewed"
        | "scope_confirmed"
        | "investigation_goal_defined" => Ok(ContractReviewItem::CustomerRequestDefined),
        "test_method_selected" | "method_available" => Ok(ContractReviewItem::TestMethodSelected),
        "laboratory_capability_confirmed" | "resources_available" => {
            Ok(ContractReviewItem::LaboratoryCapabilityConfirmed)
        }
        "equipment_availability_checked" => Ok(ContractReviewItem::EquipmentAvailabilityChecked),
        "calibration_status_reviewed" => Ok(ContractReviewItem::CalibrationStatusReviewed),
        "impartiality_risks_reviewed" | "impartiality_risk_reviewed" => {
            Ok(ContractReviewItem::ImpartialityRisksReviewed)
        }
        "data_retention_agreed" => Ok(ContractReviewItem::DataRetentionAgreed),
        "report_requirements_agreed" => Ok(ContractReviewItem::ReportRequirementsAgreed),
        "deviations_recorded" | "constraints_accepted" => {
            Ok(ContractReviewItem::DeviationsRecorded)
        }
        other => Err(AgentError::new(
            "unknown_contract_review_item",
            format!("unknown contract-review item: {other}"),
        )),
    }
}

fn contract_review_item_slug(item: ContractReviewItem) -> &'static str {
    match item {
        ContractReviewItem::CustomerRequestDefined => "customer_request_defined",
        ContractReviewItem::TestMethodSelected => "test_method_selected",
        ContractReviewItem::LaboratoryCapabilityConfirmed => "laboratory_capability_confirmed",
        ContractReviewItem::EquipmentAvailabilityChecked => "equipment_availability_checked",
        ContractReviewItem::CalibrationStatusReviewed => "calibration_status_reviewed",
        ContractReviewItem::ImpartialityRisksReviewed => "impartiality_risks_reviewed",
        ContractReviewItem::DataRetentionAgreed => "data_retention_agreed",
        ContractReviewItem::ReportRequirementsAgreed => "report_requirements_agreed",
        ContractReviewItem::DeviationsRecorded => "deviations_recorded",
    }
}

fn validate_stable_id(value: &str, field: &'static str) -> Result<(), AgentError> {
    StableId::parse(value.to_owned()).map_err(|error| {
        let mut agent_error = domain_error(error);
        agent_error.message = format!("{field} must be a non-empty stable ASCII token");
        agent_error
    })?;
    Ok(())
}

fn validate_deviation(
    input: &AdvanceToTestPlanningInput,
    mode: ExecutionMode,
    missing_items: &[ContractReviewItem],
) -> Result<Option<(String, String)>, AgentError> {
    match (&input.deviation_authorized_by, &input.deviation_reason) {
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => Err(AgentError::new(
            "invalid_deviation",
            "deviation requires both --deviation-authorized-by and --deviation-reason",
        )),
        (Some(authorized_by), Some(reason)) => {
            if missing_items.is_empty() {
                return Err(AgentError::new(
                    "unnecessary_deviation",
                    "deviation was provided but no contract-review item is missing",
                ));
            }
            if !mode.constraint_profile().deviations_allowed() {
                return Err(AgentError::new(
                    "deviation_not_allowed",
                    "execution mode does not allow deviations",
                ));
            }
            AuditActor::parse(authorized_by.clone()).map_err(domain_error)?;
            AuditReason::parse(reason.clone()).map_err(domain_error)?;
            Ok(Some((authorized_by.clone(), reason.clone())))
        }
    }
}

fn domain_error(error: DomainError) -> AgentError {
    match error {
        DomainError::EmptyProjectCode => {
            AgentError::new("invalid_project_code", "project code is required")
        }
        DomainError::InvalidProjectCode(value) => AgentError::new(
            "invalid_project_code",
            format!("invalid project code: {value}"),
        ),
        DomainError::EmptyCustomerName => {
            AgentError::new("invalid_customer_name", "customer name is required")
        }
        DomainError::EmptyAuditActor => AgentError::new("invalid_actor", "actor is required"),
        DomainError::EmptyAuditReason => AgentError::new("invalid_reason", "reason is required"),
        DomainError::InvalidProjectTransition { from, to } => AgentError::with_details(
            "invalid_project_transition",
            "invalid project stage transition",
            format!(
                "{{\"from\":{},\"to\":{}}}",
                json_string(project_stage_slug(from)),
                json_string(project_stage_slug(to))
            ),
        ),
        DomainError::IncompleteContractReview { missing_items } => AgentError::with_details(
            "contract_review_incomplete",
            "Contract review is incomplete",
            missing_items_details_json(&missing_items),
        ),
        other => AgentError::new("domain_error", format!("{other:?}")),
    }
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_error", error.to_string()))
}

fn revision_text(sequence: u64) -> String {
    format!("rev-{sequence:04}")
}

fn payload_checksum(payload_json: &str) -> String {
    let digest = Sha256::digest(payload_json.as_bytes());
    format!("sha256:{digest:x}")
}

fn project_payload_json(
    code: &str,
    customer_name: &str,
    execution_mode: &str,
    stage: &str,
) -> String {
    format!(
        "{{\"code\":{},\"customer_name\":{},\"execution_mode\":{},\"stage\":{}}}",
        json_string(code),
        json_string(customer_name.trim()),
        json_string(execution_mode),
        json_string(stage)
    )
}

fn review_item_payload_json(item: &str, comment: Option<&str>) -> String {
    format!(
        "{{\"item\":{},\"completed\":true,\"comment\":{}}}",
        json_string(item),
        json_option_string(comment)
    )
}

fn transition_payload_json(from: &str, to: &str) -> String {
    format!(
        "{{\"from\":{},\"to\":{}}}",
        json_string(from),
        json_string(to)
    )
}

fn deviation_payload_json(missing_items: &[ContractReviewItem]) -> String {
    format!(
        "{{\"missing_items\":[{}]}}",
        missing_items_json_array(missing_items)
    )
}

fn project_result_json(
    operation: &str,
    project: &StoredProject,
    replayed: bool,
    operation_id: &str,
) -> String {
    format!(
        "{{\n  \"operation\": {},\n  \"operation_id\": {},\n  \"replayed\": {},\n  \"project\": {}\n}}",
        json_string(operation),
        json_string(operation_id),
        replayed,
        project_json(project)
    )
}

fn review_item_result_json(
    status: &ContractReviewStatus,
    replayed: bool,
    already_completed: bool,
    resulting_revision: &str,
    operation_id: &str,
) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"operation\": \"contract_review_item_completed\",\n",
            "  \"operation_id\": {},\n",
            "  \"replayed\": {},\n",
            "  \"already_completed\": {},\n",
            "  \"resulting_revision\": {},\n",
            "  \"contract_review\": {}\n",
            "}}"
        ),
        json_string(operation_id),
        replayed,
        already_completed,
        json_string(resulting_revision),
        contract_review_json_compact(status)
    )
}

fn project_json(project: &StoredProject) -> String {
    format!(
        concat!(
            "{{\"code\":{},\"customer_name\":{},\"stage\":{},\"execution_mode\":{},",
            "\"created_at\":{},\"archived_at\":{},\"revision\":{}}}"
        ),
        json_string(&project.code),
        json_string(&project.customer_name),
        json_string(&project.stage),
        json_string(&project.execution_mode),
        json_string(&project.created_at),
        json_option_string(project.archived_at.as_deref()),
        json_string(&revision_text(project.revision_sequence))
    )
}

fn contract_review_json(status: &ContractReviewStatus) -> String {
    format!(
        "{{\n  \"contract_review\": {}\n}}",
        contract_review_json_compact(status)
    )
}

fn contract_review_json_compact(status: &ContractReviewStatus) -> String {
    let completed = status
        .completed_items
        .iter()
        .map(|item| {
            format!(
                concat!(
                    "{{\"item\":{},\"completed_by\":{},\"completed_at\":{},",
                    "\"comment\":{}}}"
                ),
                json_string(&item.item),
                json_option_string(item.completed_by.as_deref()),
                json_option_string(item.completed_at.as_deref()),
                json_option_string(item.comment.as_deref())
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{\"project_code\":{},\"execution_mode\":{},\"required_items\":[{}],",
            "\"completed_items\":[{}],\"missing_items\":[{}],\"complete\":{}}}"
        ),
        json_string(&status.project_code),
        json_string(execution_mode_slug(status.execution_mode)),
        missing_items_json_array(&status.required_items),
        completed,
        missing_items_json_array(&status.missing_items),
        status.missing_items.is_empty()
    )
}

fn missing_items_details_json(missing_items: &[ContractReviewItem]) -> String {
    format!(
        "{{\"missing_items\":[{}]}}",
        missing_items_json_array(missing_items)
    )
}

fn missing_items_json_array(items: &[ContractReviewItem]) -> String {
    items
        .iter()
        .map(|item| json_string(contract_review_item_slug(*item)))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{run_storage_action, StorageAction};
    use emc_locus_core::baseline_contract_review_items;

    #[test]
    fn parses_project_create_command() {
        let command = parse_project_args(
            [
                "create",
                "--storage-root",
                "E:/emc/data",
                "--code",
                "CEM-AGENT-001",
                "--customer-name",
                "Rail Lab",
                "--execution-mode",
                "accredited",
                "--actor",
                "quality.lead",
                "--reason",
                "contract accepted",
                "--operation-id",
                "op-create-001",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();

        assert!(matches!(command, AgentCommand::Projects { .. }));
    }

    #[test]
    fn creates_project_with_audit_and_outbox_transaction() {
        let storage_root = temporary_storage_root("agent-project-create");
        initialize_storage(&storage_root);

        let output = create_project(
            &storage_root,
            CreateProjectInput {
                code: "CEM-AGENT-001".to_owned(),
                customer_name: "Rail Lab".to_owned(),
                execution_mode: "accredited".to_owned(),
                stage: "contract_review".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "contract accepted".to_owned(),
                operation_id: "op-create-001".to_owned(),
                correlation_id: "corr-create-001".to_owned(),
                device_id: "station-a".to_owned(),
            },
        )
        .unwrap();
        let replay = create_project(
            &storage_root,
            CreateProjectInput {
                code: "CEM-AGENT-001".to_owned(),
                customer_name: "Rail Lab".to_owned(),
                execution_mode: "accredited".to_owned(),
                stage: "contract_review".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "contract accepted".to_owned(),
                operation_id: "op-create-001".to_owned(),
                correlation_id: "corr-create-001".to_owned(),
                device_id: "station-a".to_owned(),
            },
        )
        .unwrap();
        let outbox = list_sync_outbox(&storage_root).unwrap();
        let audits = list_audit_events(&storage_root, "CEM-AGENT-001").unwrap();

        assert!(output.contains("\"operation\": \"project_created\""));
        assert!(output.contains("\"revision\":\"rev-0001\""));
        assert!(replay.contains("\"replayed\": true"));
        assert!(outbox.contains("\"operation_kind\":\"project_created\""));
        assert!(audits.contains("\"action\":\"project_created\""));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn rejects_planning_until_contract_review_is_complete() {
        let storage_root = temporary_storage_root("agent-project-gate");
        initialize_storage(&storage_root);
        create_project(
            &storage_root,
            CreateProjectInput {
                code: "CEM-GATE-001".to_owned(),
                customer_name: "Gate Customer".to_owned(),
                execution_mode: "accredited".to_owned(),
                stage: "contract_review".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "contract accepted".to_owned(),
                operation_id: "op-gate-create".to_owned(),
                correlation_id: "corr-gate-create".to_owned(),
                device_id: "station-a".to_owned(),
            },
        )
        .unwrap();

        let error = advance_to_test_planning(
            &storage_root,
            AdvanceToTestPlanningInput {
                code: "CEM-GATE-001".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "ready".to_owned(),
                operation_id: "op-gate-transition-early".to_owned(),
                correlation_id: "corr-gate-transition-early".to_owned(),
                device_id: "station-a".to_owned(),
                deviation_authorized_by: None,
                deviation_reason: None,
            },
        )
        .unwrap_err();

        assert_eq!(error.code, "contract_review_incomplete");
        assert!(error.to_json().contains("customer_request_defined"));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn advances_to_planning_after_required_review_items() {
        let storage_root = temporary_storage_root("agent-project-planning");
        initialize_storage(&storage_root);
        create_project(
            &storage_root,
            CreateProjectInput {
                code: "CEM-PLAN-001".to_owned(),
                customer_name: "Plan Customer".to_owned(),
                execution_mode: "accredited".to_owned(),
                stage: "contract_review".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "contract accepted".to_owned(),
                operation_id: "op-plan-create".to_owned(),
                correlation_id: "corr-plan-create".to_owned(),
                device_id: "station-a".to_owned(),
            },
        )
        .unwrap();
        for (index, item) in baseline_contract_review_items().iter().enumerate() {
            complete_review_item(
                &storage_root,
                CompleteReviewItemInput {
                    code: "CEM-PLAN-001".to_owned(),
                    item: contract_review_item_slug(*item).to_owned(),
                    actor: "quality.lead".to_owned(),
                    comment: None,
                    operation_id: format!("op-plan-review-{index}"),
                    correlation_id: format!("corr-plan-review-{index}"),
                    device_id: "station-a".to_owned(),
                },
            )
            .unwrap();
        }

        let output = advance_to_test_planning(
            &storage_root,
            AdvanceToTestPlanningInput {
                code: "CEM-PLAN-001".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "review complete".to_owned(),
                operation_id: "op-plan-transition".to_owned(),
                correlation_id: "corr-plan-transition".to_owned(),
                device_id: "station-a".to_owned(),
                deviation_authorized_by: None,
                deviation_reason: None,
            },
        )
        .unwrap();
        let outbox = list_sync_outbox(&storage_root).unwrap();

        assert!(output.contains("\"stage\":\"test_planning\""));
        assert!(outbox.contains("\"operation_kind\":\"project_stage_advanced\""));

        remove_temporary_storage_root(&storage_root);
    }

    fn initialize_storage(storage_root: &Path) {
        run_storage_action(
            StorageAction::Init,
            storage_root.to_path_buf(),
            repo_root().join("storage/sqlite"),
        )
        .unwrap();
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
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}
