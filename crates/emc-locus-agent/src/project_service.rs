use super::{render_json, AgentError};
use crate::project_agent::{
    AdvanceToTestPlanningInput, CompleteReviewItemInput, CreateProjectInput, ProjectAction,
    SyncAction,
};
use crate::project_dto::{
    AuditEventDto, AuditEventsDto, CompletedReviewItemDto, ContractReviewDto,
    ContractReviewEnvelopeDto, ProjectDto, ProjectEnvelopeDto, ProjectListDto,
    ProjectOperationResultDto, ReviewItemOperationResultDto, SyncOperationDto, SyncOutboxDto,
};
use crate::project_repository::{
    ensure_operation_replay, existing_operation, insert_audit_event, insert_sync_operation,
    is_review_item_completed, load_project, load_projects, next_audit_sequence,
    open_project_connection, AuditEventInput, OperationFingerprintInput, StoredProject,
    SyncOperationInput,
};
use emc_locus_core::{
    can_transition, required_contract_review_items, AuditActor, AuditReason,
    ContractReviewChecklist, ContractReviewItem, DomainError, ExecutionMode, Project, ProjectCode,
    ProjectStage, StableId,
};
use rusqlite::{params, Connection};
use serde_json::json;
use std::path::{Path, PathBuf};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
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

pub(crate) fn run_sync_action(
    action: SyncAction,
    storage_root: PathBuf,
) -> Result<String, AgentError> {
    match action {
        SyncAction::Outbox => list_sync_outbox(&storage_root),
    }
}
pub(crate) fn run_project_action(
    action: ProjectAction,
    storage_root: PathBuf,
) -> Result<String, AgentError> {
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

pub(crate) fn create_project(
    storage_root: &Path,
    input: CreateProjectInput,
) -> Result<String, AgentError> {
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
    let stage_slug = project_stage_slug(stage);
    let mode_slug = execution_mode_slug(execution_mode);
    let payload_json =
        project_payload_json(code.as_str(), &input.customer_name, mode_slug, stage_slug);

    let mut connection = open_project_connection(storage_root)?;
    if let Some(operation) = existing_operation(&connection, &input.operation_id)? {
        ensure_operation_replay(
            &operation,
            &input.operation_id,
            OperationFingerprintInput {
                entity_id: code.as_str(),
                operation_kind: "project_created",
                base_revision: "rev-0000",
                actor_id: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                payload_json: &payload_json,
            },
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
    Ok(render_json(&ProjectListDto {
        projects: projects.iter().map(project_dto).collect(),
    }))
}

fn get_project(storage_root: &Path, code: &str) -> Result<String, AgentError> {
    let code = ProjectCode::parse(code).map_err(domain_error)?;
    let connection = open_project_connection(storage_root)?;
    let project = load_project(&connection, code.as_str())?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    Ok(render_json(&ProjectEnvelopeDto {
        project: project_dto(&project),
    }))
}

fn get_contract_review(storage_root: &Path, code: &str) -> Result<String, AgentError> {
    let code = ProjectCode::parse(code).map_err(domain_error)?;
    let connection = open_project_connection(storage_root)?;
    let status = load_contract_review_status(&connection, code.as_str())?;
    Ok(contract_review_json(&status))
}

pub(crate) fn complete_review_item(
    storage_root: &Path,
    input: CompleteReviewItemInput,
) -> Result<String, AgentError> {
    let code = ProjectCode::parse(input.code.clone()).map_err(domain_error)?;
    let item = parse_contract_review_item(&input.item)?;
    let actor = AuditActor::parse(input.actor.clone()).map_err(domain_error)?;
    validate_stable_id(&input.operation_id, "operation_id")?;
    validate_stable_id(&input.correlation_id, "correlation_id")?;
    validate_stable_id(&input.device_id, "device_id")?;
    let canonical_item = contract_review_item_slug(item);
    let comment = input
        .comment
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let payload_json = review_item_payload_json(canonical_item, comment);

    let mut connection = open_project_connection(storage_root)?;
    if let Some(operation) = existing_operation(&connection, &input.operation_id)? {
        ensure_operation_replay(
            &operation,
            &input.operation_id,
            OperationFingerprintInput {
                entity_id: code.as_str(),
                operation_kind: "contract_review_item_completed",
                base_revision: &operation.base_revision,
                actor_id: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                payload_json: &payload_json,
            },
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

pub(crate) fn advance_to_test_planning(
    storage_root: &Path,
    input: AdvanceToTestPlanningInput,
) -> Result<String, AgentError> {
    let code = ProjectCode::parse(input.code.clone()).map_err(domain_error)?;
    let actor = AuditActor::parse(input.actor.clone()).map_err(domain_error)?;
    AuditReason::parse(input.reason.clone()).map_err(domain_error)?;
    validate_stable_id(&input.operation_id, "operation_id")?;
    validate_stable_id(&input.correlation_id, "correlation_id")?;
    validate_stable_id(&input.device_id, "device_id")?;
    let payload_json = transition_command_payload_json(
        "test_planning",
        &input.reason,
        input.deviation_authorized_by.as_deref(),
        input.deviation_reason.as_deref(),
    );

    let mut connection = open_project_connection(storage_root)?;
    if let Some(operation) = existing_operation(&connection, &input.operation_id)? {
        ensure_operation_replay(
            &operation,
            &input.operation_id,
            OperationFingerprintInput {
                entity_id: code.as_str(),
                operation_kind: "project_stage_advanced",
                base_revision: &operation.base_revision,
                actor_id: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                payload_json: &payload_json,
            },
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
            json!({ "from": project.stage, "to": "test_planning" }),
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

pub(crate) fn list_audit_events(storage_root: &Path, code: &str) -> Result<String, AgentError> {
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
            Ok(AuditEventDto {
                sequence: row.get(0)?,
                actor: row.get(1)?,
                action: row.get(2)?,
                reason: row.get(3)?,
                payload_json: row.get(4)?,
                occurred_at: row.get(5)?,
            })
        })
        .map_err(|error| AgentError::new("audit_query_failed", error.to_string()))?;
    let mut events = Vec::new();
    for row in rows {
        events.push(row.map_err(|error| AgentError::new("audit_query_failed", error.to_string()))?);
    }
    Ok(render_json(&AuditEventsDto {
        project_code: code.as_str().to_owned(),
        audit_events: events,
    }))
}

pub(crate) fn list_sync_outbox(storage_root: &Path) -> Result<String, AgentError> {
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
            Ok(SyncOperationDto {
                operation_id: row.get(0)?,
                domain: row.get(1)?,
                entity_type: row.get(2)?,
                entity_id: row.get(3)?,
                operation_kind: row.get(4)?,
                base_revision: row.get(5)?,
                resulting_revision: row.get(6)?,
                actor_id: row.get(7)?,
                device_id: row.get(8)?,
                correlation_id: row.get(9)?,
                payload_json: row.get(10)?,
                payload_checksum: row.get(11)?,
                status: row.get(12)?,
                occurred_at: row.get(13)?,
                recorded_at: row.get(14)?,
            })
        })
        .map_err(|error| AgentError::new("sync_outbox_query_failed", error.to_string()))?;
    let mut operations = Vec::new();
    for row in rows {
        operations.push(
            row.map_err(|error| AgentError::new("sync_outbox_query_failed", error.to_string()))?,
        );
    }
    Ok(render_json(&SyncOutboxDto {
        sync_outbox: operations,
    }))
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

pub(crate) fn contract_review_item_slug(item: ContractReviewItem) -> &'static str {
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
            json!({ "from": project_stage_slug(from), "to": project_stage_slug(to) }),
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

fn project_payload_json(
    code: &str,
    customer_name: &str,
    execution_mode: &str,
    stage: &str,
) -> String {
    render_json(&json!({
        "code": code,
        "customer_name": customer_name.trim(),
        "execution_mode": execution_mode,
        "stage": stage,
    }))
}

fn review_item_payload_json(item: &str, comment: Option<&str>) -> String {
    render_json(&json!({
        "item": item,
        "completed": true,
        "comment": comment,
    }))
}

fn transition_payload_json(from: &str, to: &str) -> String {
    render_json(&json!({
        "from": from,
        "to": to,
    }))
}

fn transition_command_payload_json(
    to: &str,
    reason: &str,
    deviation_authorized_by: Option<&str>,
    deviation_reason: Option<&str>,
) -> String {
    render_json(&json!({
        "to": to,
        "reason": reason.trim(),
        "deviation_authorized_by": deviation_authorized_by
            .map(str::trim)
            .filter(|value| !value.is_empty()),
        "deviation_reason": deviation_reason
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    }))
}

fn deviation_payload_json(missing_items: &[ContractReviewItem]) -> String {
    render_json(&json!({
        "missing_items": contract_review_item_slugs(missing_items),
    }))
}

fn project_result_json(
    operation: &str,
    project: &StoredProject,
    replayed: bool,
    operation_id: &str,
) -> String {
    render_json(&ProjectOperationResultDto {
        operation: operation.to_owned(),
        operation_id: operation_id.to_owned(),
        replayed,
        project: project_dto(project),
    })
}

fn review_item_result_json(
    status: &ContractReviewStatus,
    replayed: bool,
    already_completed: bool,
    resulting_revision: &str,
    operation_id: &str,
) -> String {
    render_json(&ReviewItemOperationResultDto {
        operation: "contract_review_item_completed".to_owned(),
        operation_id: operation_id.to_owned(),
        replayed,
        already_completed,
        resulting_revision: resulting_revision.to_owned(),
        contract_review: contract_review_dto(status),
    })
}

fn project_dto(project: &StoredProject) -> ProjectDto {
    ProjectDto {
        code: project.code.clone(),
        customer_name: project.customer_name.clone(),
        stage: project.stage.clone(),
        execution_mode: project.execution_mode.clone(),
        created_at: project.created_at.clone(),
        archived_at: project.archived_at.clone(),
        revision: revision_text(project.revision_sequence),
    }
}

fn contract_review_json(status: &ContractReviewStatus) -> String {
    render_json(&ContractReviewEnvelopeDto {
        contract_review: contract_review_dto(status),
    })
}

fn contract_review_dto(status: &ContractReviewStatus) -> ContractReviewDto {
    ContractReviewDto {
        project_code: status.project_code.clone(),
        execution_mode: execution_mode_slug(status.execution_mode).to_owned(),
        required_items: contract_review_item_slugs(&status.required_items),
        completed_items: status
            .completed_items
            .iter()
            .map(|item| CompletedReviewItemDto {
                item: item.item.clone(),
                completed_by: item.completed_by.clone(),
                completed_at: item.completed_at.clone(),
                comment: item.comment.clone(),
            })
            .collect(),
        missing_items: contract_review_item_slugs(&status.missing_items),
        complete: status.missing_items.is_empty(),
    }
}

fn missing_items_details_json(missing_items: &[ContractReviewItem]) -> serde_json::Value {
    json!({
        "missing_items": contract_review_item_slugs(missing_items),
    })
}

fn contract_review_item_slugs(items: &[ContractReviewItem]) -> Vec<String> {
    items
        .iter()
        .map(|item| contract_review_item_slug(*item).to_owned())
        .collect::<Vec<_>>()
}
