use crate::planned_test_preparation_service::require_planned_test_preparation_for_start;
use crate::project_repository::{
    ensure_operation_replay, existing_operation, insert_audit_event, insert_sync_operation,
    load_project, next_audit_sequence, open_project_connection, AuditEventInput,
    OperationFingerprintInput, SyncOperationInput,
};
use crate::service_schedule_dto::{
    LaboratoryScheduleItemDto, LaboratoryWeekScheduleDto, ServiceScheduleItemDto,
    ServiceScheduleListDto, ServiceScheduleOperationResultDto,
};
use crate::service_schedule_repository::{
    ensure_service_schedule_table, find_service_schedule_conflict, insert_service_schedule_item,
    load_laboratory_service_schedule_items, load_project_service_schedule_items,
    load_service_schedule_item, update_service_schedule_assignment, update_service_schedule_status,
    ScheduleConflict, StoredLaboratoryScheduleItem, StoredServiceScheduleItem,
};
use crate::{render_json, AgentError};
use emc_locus_core::{
    AuditActor, AuditReason, PlanningValidationIssue, ProjectCode, ScheduleResourceConflictKind,
    ServiceScheduleItem, ServiceScheduleItemInput, ServiceScheduleRescheduleInput,
    ServiceScheduleStatus, ServiceScheduleWeek, StableId,
};
use serde_json::json;
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateServiceScheduleItemInput {
    pub project_code: String,
    pub item_code: String,
    pub title: String,
    pub planned_start_at: String,
    pub planned_end_at: String,
    pub assigned_operator: String,
    pub location: String,
    pub equipment_under_test: String,
    pub test_category_code: Option<String>,
    pub test_method_code: Option<String>,
    pub notes: Option<String>,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionServiceScheduleItemInput {
    pub project_code: String,
    pub item_code: String,
    pub target_status: String,
    pub expected_revision: u64,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RescheduleServiceScheduleItemInput {
    pub project_code: String,
    pub item_code: String,
    pub planned_start_at: String,
    pub planned_end_at: String,
    pub assigned_operator: String,
    pub location: String,
    pub expected_revision: u64,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

pub fn create_service_schedule_item(
    storage_root: &Path,
    input: CreateServiceScheduleItemInput,
) -> Result<String, AgentError> {
    let project_code = ProjectCode::parse(input.project_code.clone()).map_err(domain_error)?;
    let actor = AuditActor::parse(input.actor.clone()).map_err(domain_error)?;
    AuditReason::parse(input.reason.clone()).map_err(domain_error)?;
    validate_stable_id(&input.operation_id, "operation_id")?;
    validate_stable_id(&input.correlation_id, "correlation_id")?;
    validate_stable_id(&input.device_id, "device_id")?;
    let item = ServiceScheduleItem::create(ServiceScheduleItemInput {
        item_code: input.item_code.clone(),
        project_code: project_code.clone(),
        title: input.title.clone(),
        planned_start_at: input.planned_start_at.clone(),
        planned_end_at: input.planned_end_at.clone(),
        assigned_operator: input.assigned_operator.clone(),
        location: input.location.clone(),
        equipment_under_test: input.equipment_under_test.clone(),
        test_category_code: input.test_category_code.clone(),
        test_method_code: input.test_method_code.clone(),
        status: ServiceScheduleStatus::Planned,
        notes: input.notes.clone(),
    })
    .map_err(planning_error)?;
    let payload_json = create_command_payload(&item, &input.reason);

    let mut connection = open_project_connection(storage_root)?;
    ensure_service_schedule_table(&connection)?;
    if let Some(operation) = existing_operation(&connection, &input.operation_id)? {
        ensure_operation_replay(
            &operation,
            &input.operation_id,
            OperationFingerprintInput {
                domain: "project_records",
                entity_type: "service_schedule_item",
                entity_id: item.item_code(),
                operation_kind: "service_schedule_item_planned",
                base_revision: &operation.base_revision,
                actor_id: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                payload_json: &payload_json,
            },
        )?;
        let stored =
            load_service_schedule_item(&connection, item.item_code())?.ok_or_else(|| {
                AgentError::new(
                    "operation_replay_missing_entity",
                    "operation exists but service schedule item is missing",
                )
            })?;
        return schedule_operation_json(
            "service_schedule_item_planned",
            &input.operation_id,
            true,
            &stored,
        );
    }

    let project = load_project(&connection, project_code.as_str())?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    if project.stage != "test_planning" {
        return Err(AgentError::with_details(
            "project_not_ready_for_scheduling",
            "the project must complete contract review before tests can be planned",
            json!({ "project_code": project.code, "current_stage": project.stage }),
        ));
    }
    if load_service_schedule_item(&connection, item.item_code())?.is_some() {
        return Err(AgentError::with_details(
            "service_schedule_item_already_exists",
            "a service schedule item already uses this reference",
            json!({ "item_code": item.item_code() }),
        ));
    }
    if let Some(conflict) = find_service_schedule_conflict(&connection, &item, None)? {
        return Err(schedule_conflict_error(conflict));
    }

    let timestamp = utc_timestamp()?;
    let audit_sequence = next_audit_sequence(&connection, project_code.as_str())?;
    let audit_payload = schedule_snapshot_payload(&item);
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_service_schedule_item(&transaction, &item, actor.as_str(), &timestamp)?;
    insert_audit_event(
        &transaction,
        AuditEventInput {
            project_code: project_code.as_str(),
            sequence: audit_sequence,
            actor: actor.as_str(),
            action: "service_schedule_item_planned",
            reason: Some(&input.reason),
            payload_json: &audit_payload,
            timestamp: &timestamp,
        },
    )?;
    insert_sync_operation(
        &transaction,
        SyncOperationInput {
            domain: "project_records",
            entity_type: "service_schedule_item",
            operation_id: &input.operation_id,
            entity_id: item.item_code(),
            operation_kind: "service_schedule_item_planned",
            base_revision: "rev-0000",
            resulting_revision: "rev-0001",
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &timestamp,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    let stored = load_service_schedule_item(&connection, item.item_code())?.ok_or_else(|| {
        AgentError::new("service_schedule_read_failed", "created item is missing")
    })?;
    schedule_operation_json(
        "service_schedule_item_planned",
        &input.operation_id,
        false,
        &stored,
    )
}

pub fn list_project_service_schedule_items(
    storage_root: &Path,
    project_code: &str,
) -> Result<String, AgentError> {
    let project_code = ProjectCode::parse(project_code.to_owned()).map_err(domain_error)?;
    let connection = open_project_connection(storage_root)?;
    ensure_service_schedule_table(&connection)?;
    load_project(&connection, project_code.as_str())?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    let items = load_project_service_schedule_items(&connection, project_code.as_str())?;
    let schedule_items = items
        .iter()
        .map(schedule_item_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(render_json(&ServiceScheduleListDto {
        project_code: project_code.as_str().to_owned(),
        schedule_items,
    }))
}

pub fn list_laboratory_week_schedule(
    storage_root: &Path,
    week_start: &str,
) -> Result<String, AgentError> {
    let week = ServiceScheduleWeek::parse(week_start).map_err(planning_error)?;
    let connection = open_project_connection(storage_root)?;
    ensure_service_schedule_table(&connection)?;
    let items = load_laboratory_service_schedule_items(
        &connection,
        &week.query_start_at(),
        &week.query_end_at_exclusive(),
    )?;
    let schedule_items = items
        .iter()
        .map(laboratory_schedule_item_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(render_json(&LaboratoryWeekScheduleDto {
        week_start: week.start_date().to_owned(),
        week_end: week.end_date().to_owned(),
        schedule_items,
    }))
}

pub fn reschedule_service_schedule_item(
    storage_root: &Path,
    input: RescheduleServiceScheduleItemInput,
) -> Result<String, AgentError> {
    let project_code = ProjectCode::parse(input.project_code.clone()).map_err(domain_error)?;
    let actor = AuditActor::parse(input.actor.clone()).map_err(domain_error)?;
    AuditReason::parse(input.reason.clone()).map_err(domain_error)?;
    validate_stable_id(&input.operation_id, "operation_id")?;
    validate_stable_id(&input.correlation_id, "correlation_id")?;
    validate_stable_id(&input.device_id, "device_id")?;
    if input.expected_revision == 0 {
        return Err(AgentError::with_details(
            "invalid_service_schedule_request",
            "expected_revision must be at least 1",
            json!({ "field": "expected_revision" }),
        ));
    }
    let payload_json = reschedule_command_payload(&input);

    let mut connection = open_project_connection(storage_root)?;
    ensure_service_schedule_table(&connection)?;
    if let Some(operation) = existing_operation(&connection, &input.operation_id)? {
        ensure_operation_replay(
            &operation,
            &input.operation_id,
            OperationFingerprintInput {
                domain: "project_records",
                entity_type: "service_schedule_item",
                entity_id: &input.item_code,
                operation_kind: "service_schedule_item_rescheduled",
                base_revision: &operation.base_revision,
                actor_id: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                payload_json: &payload_json,
            },
        )?;
        let stored =
            load_service_schedule_item(&connection, &input.item_code)?.ok_or_else(|| {
                AgentError::new(
                    "operation_replay_missing_entity",
                    "operation exists but service schedule item is missing",
                )
            })?;
        return schedule_operation_json(
            "service_schedule_item_rescheduled",
            &input.operation_id,
            true,
            &stored,
        );
    }

    load_project(&connection, project_code.as_str())?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    let stored = load_service_schedule_item(&connection, &input.item_code)?.ok_or_else(|| {
        AgentError::new("service_schedule_item_not_found", "schedule item not found")
    })?;
    if stored.project_code != project_code.as_str() {
        return Err(AgentError::new(
            "service_schedule_item_not_found",
            "schedule item does not belong to this project",
        ));
    }
    if stored.revision != input.expected_revision {
        return Err(AgentError::with_details(
            "service_schedule_concurrent_update",
            "the service schedule item changed; refresh the planning before trying again",
            json!({
                "item_code": stored.item_code,
                "expected_revision": input.expected_revision,
                "actual_revision": stored.revision,
            }),
        ));
    }
    let current = stored.to_domain()?;
    let moved = current
        .rescheduled(ServiceScheduleRescheduleInput {
            planned_start_at: input.planned_start_at.clone(),
            planned_end_at: input.planned_end_at.clone(),
            assigned_operator: input.assigned_operator.clone(),
            location: input.location.clone(),
        })
        .map_err(|issue| reschedule_error(issue, current.status()))?;
    if let Some(conflict) = find_service_schedule_conflict(&connection, &moved, Some(stored.id))? {
        return Err(schedule_conflict_error(conflict));
    }

    let timestamp = utc_timestamp()?;
    let audit_sequence = next_audit_sequence(&connection, project_code.as_str())?;
    let audit_payload = render_json(&json!({
        "item_code": moved.item_code(),
        "previous": schedule_assignment_snapshot(&current),
        "new": schedule_assignment_snapshot(&moved),
        "status": moved.status().as_str(),
    }));
    let base_revision = revision_text(stored.revision);
    let resulting_revision = revision_text(stored.revision + 1);
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    update_service_schedule_assignment(
        &transaction,
        stored.id,
        stored.revision,
        &moved,
        actor.as_str(),
        &timestamp,
    )?;
    insert_audit_event(
        &transaction,
        AuditEventInput {
            project_code: project_code.as_str(),
            sequence: audit_sequence,
            actor: actor.as_str(),
            action: "service_schedule_item_rescheduled",
            reason: Some(&input.reason),
            payload_json: &audit_payload,
            timestamp: &timestamp,
        },
    )?;
    insert_sync_operation(
        &transaction,
        SyncOperationInput {
            domain: "project_records",
            entity_type: "service_schedule_item",
            operation_id: &input.operation_id,
            entity_id: moved.item_code(),
            operation_kind: "service_schedule_item_rescheduled",
            base_revision: &base_revision,
            resulting_revision: &resulting_revision,
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &timestamp,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    let updated = load_service_schedule_item(&connection, moved.item_code())?.ok_or_else(|| {
        AgentError::new(
            "service_schedule_read_failed",
            "rescheduled item is missing",
        )
    })?;
    schedule_operation_json(
        "service_schedule_item_rescheduled",
        &input.operation_id,
        false,
        &updated,
    )
}

pub fn transition_service_schedule_item(
    storage_root: &Path,
    input: TransitionServiceScheduleItemInput,
) -> Result<String, AgentError> {
    let project_code = ProjectCode::parse(input.project_code.clone()).map_err(domain_error)?;
    let actor = AuditActor::parse(input.actor.clone()).map_err(domain_error)?;
    AuditReason::parse(input.reason.clone()).map_err(domain_error)?;
    validate_stable_id(&input.operation_id, "operation_id")?;
    validate_stable_id(&input.correlation_id, "correlation_id")?;
    validate_stable_id(&input.device_id, "device_id")?;
    if input.expected_revision == 0 {
        return Err(AgentError::with_details(
            "invalid_service_schedule_request",
            "expected_revision must be at least 1",
            json!({ "field": "expected_revision" }),
        ));
    }
    let target = ServiceScheduleStatus::parse(&input.target_status).map_err(planning_error)?;
    let payload_json = transition_command_payload(target, &input.reason);

    let mut connection = open_project_connection(storage_root)?;
    ensure_service_schedule_table(&connection)?;
    if let Some(operation) = existing_operation(&connection, &input.operation_id)? {
        ensure_operation_replay(
            &operation,
            &input.operation_id,
            OperationFingerprintInput {
                domain: "project_records",
                entity_type: "service_schedule_item",
                entity_id: &input.item_code,
                operation_kind: "service_schedule_item_status_changed",
                base_revision: &operation.base_revision,
                actor_id: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                payload_json: &payload_json,
            },
        )?;
        let stored =
            load_service_schedule_item(&connection, &input.item_code)?.ok_or_else(|| {
                AgentError::new(
                    "operation_replay_missing_entity",
                    "operation exists but service schedule item is missing",
                )
            })?;
        return schedule_operation_json(
            "service_schedule_item_status_changed",
            &input.operation_id,
            true,
            &stored,
        );
    }

    load_project(&connection, project_code.as_str())?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    let stored = load_service_schedule_item(&connection, &input.item_code)?.ok_or_else(|| {
        AgentError::new("service_schedule_item_not_found", "schedule item not found")
    })?;
    if stored.project_code != project_code.as_str() {
        return Err(AgentError::new(
            "service_schedule_item_not_found",
            "schedule item does not belong to this project",
        ));
    }
    if stored.revision != input.expected_revision {
        return Err(AgentError::with_details(
            "service_schedule_concurrent_update",
            "the service schedule item changed; refresh the project before trying again",
            json!({
                "item_code": stored.item_code,
                "expected_revision": input.expected_revision,
                "actual_revision": stored.revision,
            }),
        ));
    }
    let mut item = stored.to_domain()?;
    let previous_status = item.status();
    item.transition_to(target).map_err(|issue| {
        AgentError::with_details(
            "invalid_service_schedule_transition",
            "the requested planning action is not available from the current state",
            json!({
                "from": previous_status.as_str(),
                "to": target.as_str(),
                "cause": issue.code,
            }),
        )
    })?;
    let preparation_evidence = if target == ServiceScheduleStatus::InProgress {
        Some(require_planned_test_preparation_for_start(
            storage_root,
            project_code.as_str(),
            &input.item_code,
            stored.revision,
        )?)
    } else {
        None
    };

    let timestamp = utc_timestamp()?;
    let audit_sequence = next_audit_sequence(&connection, project_code.as_str())?;
    let audit_payload = render_json(&json!({
        "item_code": item.item_code(),
        "previous_status": previous_status.as_str(),
        "new_status": target.as_str(),
        "planned_test_preparation": preparation_evidence,
    }));
    let base_revision = revision_text(stored.revision);
    let resulting_revision = revision_text(stored.revision + 1);
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    update_service_schedule_status(
        &transaction,
        stored.id,
        stored.revision,
        target,
        actor.as_str(),
        &timestamp,
    )?;
    insert_audit_event(
        &transaction,
        AuditEventInput {
            project_code: project_code.as_str(),
            sequence: audit_sequence,
            actor: actor.as_str(),
            action: "service_schedule_item_status_changed",
            reason: Some(&input.reason),
            payload_json: &audit_payload,
            timestamp: &timestamp,
        },
    )?;
    insert_sync_operation(
        &transaction,
        SyncOperationInput {
            domain: "project_records",
            entity_type: "service_schedule_item",
            operation_id: &input.operation_id,
            entity_id: item.item_code(),
            operation_kind: "service_schedule_item_status_changed",
            base_revision: &base_revision,
            resulting_revision: &resulting_revision,
            actor_id: &input.actor,
            device_id: &input.device_id,
            correlation_id: &input.correlation_id,
            payload_json: &payload_json,
            timestamp: &timestamp,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    let updated = load_service_schedule_item(&connection, item.item_code())?.ok_or_else(|| {
        AgentError::new(
            "service_schedule_read_failed",
            "updated schedule item is missing",
        )
    })?;
    schedule_operation_json(
        "service_schedule_item_status_changed",
        &input.operation_id,
        false,
        &updated,
    )
}

fn schedule_item_dto(
    stored: &StoredServiceScheduleItem,
) -> Result<ServiceScheduleItemDto, AgentError> {
    let item = stored.to_domain()?;
    Ok(ServiceScheduleItemDto {
        item_code: item.item_code().to_owned(),
        project_code: item.project_code().as_str().to_owned(),
        title: item.title().to_owned(),
        test_category_code: item.test_category_code().map(str::to_owned),
        test_method_code: item.test_method_code().map(str::to_owned),
        planned_start_at: item.planned_start_at().to_owned(),
        planned_end_at: item.planned_end_at().to_owned(),
        assigned_operator: item.assigned_operator().to_owned(),
        location: item.location().to_owned(),
        equipment_under_test: item.equipment_under_test().to_owned(),
        status: item.status().as_str().to_owned(),
        notes: item.notes().to_owned(),
        revision: stored.revision,
        created_by: stored.created_by.clone(),
        updated_by: stored.updated_by.clone(),
        created_at: stored.created_at.clone(),
        updated_at: stored.updated_at.clone(),
        available_transitions: item
            .status()
            .allowed_targets()
            .iter()
            .map(|status| status.as_str().to_owned())
            .collect(),
        can_reschedule: item.status().can_reschedule(),
    })
}

fn laboratory_schedule_item_dto(
    stored: &StoredLaboratoryScheduleItem,
) -> Result<LaboratoryScheduleItemDto, AgentError> {
    Ok(LaboratoryScheduleItemDto {
        schedule_item: schedule_item_dto(&stored.schedule_item)?,
        customer_name: stored.customer_name.clone(),
        project_stage: stored.project_stage.clone(),
    })
}

fn schedule_operation_json(
    operation: &str,
    operation_id: &str,
    replayed: bool,
    item: &StoredServiceScheduleItem,
) -> Result<String, AgentError> {
    Ok(render_json(&ServiceScheduleOperationResultDto {
        operation: operation.to_owned(),
        operation_id: operation_id.to_owned(),
        replayed,
        schedule_item: schedule_item_dto(item)?,
    }))
}

fn create_command_payload(item: &ServiceScheduleItem, reason: &str) -> String {
    render_json(&json!({
        "schedule_item": serde_json::from_str::<serde_json::Value>(&schedule_snapshot_payload(item))
            .expect("schedule snapshot is valid JSON"),
        "reason": reason,
    }))
}

fn schedule_snapshot_payload(item: &ServiceScheduleItem) -> String {
    render_json(&json!({
        "item_code": item.item_code(),
        "project_code": item.project_code().as_str(),
        "title": item.title(),
        "test_category_code": item.test_category_code(),
        "test_method_code": item.test_method_code(),
        "planned_start_at": item.planned_start_at(),
        "planned_end_at": item.planned_end_at(),
        "assigned_operator": item.assigned_operator(),
        "location": item.location(),
        "equipment_under_test": item.equipment_under_test(),
        "status": item.status().as_str(),
        "notes": item.notes(),
    }))
}

fn transition_command_payload(target: ServiceScheduleStatus, reason: &str) -> String {
    render_json(&json!({ "target_status": target.as_str(), "reason": reason }))
}

fn reschedule_command_payload(input: &RescheduleServiceScheduleItemInput) -> String {
    render_json(&json!({
        "planned_start_at": input.planned_start_at,
        "planned_end_at": input.planned_end_at,
        "assigned_operator": input.assigned_operator,
        "location": input.location,
        "expected_revision": input.expected_revision,
        "reason": input.reason,
    }))
}

fn schedule_assignment_snapshot(item: &ServiceScheduleItem) -> serde_json::Value {
    json!({
        "planned_start_at": item.planned_start_at(),
        "planned_end_at": item.planned_end_at(),
        "assigned_operator": item.assigned_operator(),
        "location": item.location(),
    })
}

fn schedule_conflict_error(conflict: ScheduleConflict) -> AgentError {
    let item = conflict.conflicting_item;
    let (code, message, resource, value) = match conflict.kind {
        ScheduleResourceConflictKind::Operator => (
            "service_schedule_operator_conflict",
            "the assigned operator already has a test planned during this period",
            "operator",
            item.assigned_operator.clone(),
        ),
        ScheduleResourceConflictKind::Location => (
            "service_schedule_location_conflict",
            "the selected laboratory location is already reserved during this period",
            "location",
            item.location.clone(),
        ),
    };
    AgentError::with_details(
        code,
        message,
        json!({
            "resource": resource,
            "value": value,
            "conflicting_item": {
                "item_code": item.item_code,
                "project_code": item.project_code,
                "title": item.title,
                "planned_start_at": item.planned_start_at,
                "planned_end_at": item.planned_end_at,
                "assigned_operator": item.assigned_operator,
                "location": item.location,
            }
        }),
    )
}

fn validate_stable_id(value: &str, field: &'static str) -> Result<(), AgentError> {
    StableId::parse(value.to_owned()).map_err(|_| {
        AgentError::with_details(
            "invalid_stable_id",
            format!("{field} must be a non-empty stable identifier"),
            json!({ "field": field }),
        )
    })?;
    Ok(())
}

fn planning_error(issue: PlanningValidationIssue) -> AgentError {
    AgentError::with_details(
        "invalid_service_schedule_request",
        issue.message,
        json!({ "field": issue.field, "cause": issue.code }),
    )
}

fn reschedule_error(issue: PlanningValidationIssue, status: ServiceScheduleStatus) -> AgentError {
    if issue.code == "schedule_status_not_reschedulable" {
        AgentError::with_details(
            "service_schedule_item_not_reschedulable",
            "the schedule item can no longer be moved",
            json!({ "status": status.as_str(), "cause": issue.code }),
        )
    } else {
        planning_error(issue)
    }
}

fn domain_error(error: emc_locus_core::DomainError) -> AgentError {
    match error {
        emc_locus_core::DomainError::EmptyProjectCode => {
            AgentError::new("invalid_project_code", "project code is required")
        }
        emc_locus_core::DomainError::InvalidProjectCode(value) => AgentError::new(
            "invalid_project_code",
            format!("invalid project code: {value}"),
        ),
        emc_locus_core::DomainError::EmptyAuditActor => {
            AgentError::new("invalid_actor", "actor is required")
        }
        emc_locus_core::DomainError::EmptyAuditReason => {
            AgentError::new("invalid_reason", "reason is required")
        }
        other => AgentError::new("domain_error", format!("{other:?}")),
    }
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_failed", error.to_string()))
}

fn revision_text(revision: u64) -> String {
    format!("rev-{revision:04}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_agent::{
        AdvanceToTestPlanningInput, CompleteReviewItemInput, CreateProjectInput,
    };
    use crate::project_service::{
        advance_to_test_planning, complete_review_item, contract_review_item_slug, create_project,
        list_audit_events, list_sync_outbox,
    };
    use crate::{run_storage_action, StorageAction};
    use emc_locus_core::{required_contract_review_items, ExecutionMode};
    use rusqlite::Connection;
    use std::path::{Path, PathBuf};

    #[test]
    fn refuses_scheduling_before_contract_review_reaches_planning() {
        let storage_root = temporary_storage_root("service-schedule-stage-gate");
        initialize_storage(&storage_root);
        create_project_for_test(&storage_root, "CEM-STAGE-001", false);

        let error = create_service_schedule_item(
            &storage_root,
            schedule_input("CEM-STAGE-001", "PLAN-STAGE-001", "Alice", "Labo 1"),
        )
        .unwrap_err();

        assert_eq!(error.code, "project_not_ready_for_scheduling");
        assert_eq!(table_count(&storage_root, "service_schedule_items"), 0);
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn creates_replays_and_persists_schedule_with_audit_and_outbox() {
        let storage_root = temporary_storage_root("service-schedule-create");
        initialize_storage(&storage_root);
        create_project_for_test(&storage_root, "CEM-PLAN-001", true);
        let input = schedule_input("CEM-PLAN-001", "PLAN-001", "Alice", "Labo 1");

        let first = create_service_schedule_item(&storage_root, input.clone()).unwrap();
        let replay = create_service_schedule_item(&storage_root, input).unwrap();
        let listed = list_project_service_schedule_items(&storage_root, "CEM-PLAN-001").unwrap();
        let audits = list_audit_events(&storage_root, "CEM-PLAN-001").unwrap();
        let outbox = list_sync_outbox(&storage_root).unwrap();

        assert!(first.contains("\"replayed\":false"));
        assert!(first.contains("\"revision\":1"));
        assert!(replay.contains("\"replayed\":true"));
        assert!(listed.contains("\"item_code\":\"PLAN-001\""));
        assert!(audits.contains("service_schedule_item_planned"));
        assert!(outbox.contains("\"entity_type\":\"service_schedule_item\""));
        assert_eq!(table_count(&storage_root, "service_schedule_items"), 1);
        assert_eq!(operation_count(&storage_root, "op-PLAN-001"), 1);
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn rejects_operator_and_location_conflicts_without_partial_evidence() {
        let storage_root = temporary_storage_root("service-schedule-conflicts");
        initialize_storage(&storage_root);
        create_project_for_test(&storage_root, "CEM-CONFLICT-001", true);
        create_service_schedule_item(
            &storage_root,
            schedule_input("CEM-CONFLICT-001", "PLAN-CONFLICT-001", "Alice", "Labo 1"),
        )
        .unwrap();

        let operator_error = create_service_schedule_item(
            &storage_root,
            schedule_input("CEM-CONFLICT-001", "PLAN-CONFLICT-002", "Alice", "Labo 2"),
        )
        .unwrap_err();
        let location_error = create_service_schedule_item(
            &storage_root,
            schedule_input("CEM-CONFLICT-001", "PLAN-CONFLICT-003", "Bob", "Labo 1"),
        )
        .unwrap_err();

        assert_eq!(operator_error.code, "service_schedule_operator_conflict");
        assert!(operator_error.to_json().contains("PLAN-CONFLICT-001"));
        assert_eq!(location_error.code, "service_schedule_location_conflict");
        assert!(location_error.to_json().contains("Labo 1"));
        assert_eq!(table_count(&storage_root, "service_schedule_items"), 1);
        assert_eq!(operation_count(&storage_root, "op-PLAN-CONFLICT-002"), 0);
        assert_eq!(operation_count(&storage_root, "op-PLAN-CONFLICT-003"), 0);
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn transitions_sequentially_and_rejects_stale_revision() {
        let storage_root = temporary_storage_root("service-schedule-transitions");
        initialize_storage(&storage_root);
        create_project_for_test(&storage_root, "CEM-STATUS-001", true);
        create_service_schedule_item(
            &storage_root,
            schedule_input("CEM-STATUS-001", "PLAN-STATUS-001", "Alice", "Labo 1"),
        )
        .unwrap();

        let confirmed = transition_service_schedule_item(
            &storage_root,
            transition_input(
                "CEM-STATUS-001",
                "PLAN-STATUS-001",
                "confirmed",
                1,
                "confirm",
            ),
        )
        .unwrap();
        let stale = transition_service_schedule_item(
            &storage_root,
            transition_input(
                "CEM-STATUS-001",
                "PLAN-STATUS-001",
                "in_progress",
                1,
                "stale",
            ),
        )
        .unwrap_err();
        let started = transition_service_schedule_item(
            &storage_root,
            transition_input(
                "CEM-STATUS-001",
                "PLAN-STATUS-001",
                "in_progress",
                2,
                "start",
            ),
        )
        .unwrap();

        assert!(confirmed.contains("\"status\":\"confirmed\""));
        assert!(confirmed.contains("\"revision\":2"));
        assert_eq!(stale.code, "service_schedule_concurrent_update");
        assert!(started.contains("\"status\":\"in_progress\""));
        assert!(started.contains("\"revision\":3"));
        assert_eq!(operation_count(&storage_root, "op-status-stale"), 0);
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn rejects_skipped_status_transition_without_mutation() {
        let storage_root = temporary_storage_root("service-schedule-transition-gate");
        initialize_storage(&storage_root);
        create_project_for_test(&storage_root, "CEM-SKIP-001", true);
        create_service_schedule_item(
            &storage_root,
            schedule_input("CEM-SKIP-001", "PLAN-SKIP-001", "Alice", "Labo 1"),
        )
        .unwrap();

        let error = transition_service_schedule_item(
            &storage_root,
            transition_input("CEM-SKIP-001", "PLAN-SKIP-001", "completed", 1, "skip"),
        )
        .unwrap_err();
        let listed = list_project_service_schedule_items(&storage_root, "CEM-SKIP-001").unwrap();

        assert_eq!(error.code, "invalid_service_schedule_transition");
        assert!(listed.contains("\"status\":\"planned\""));
        assert!(listed.contains("\"revision\":1"));
        assert_eq!(operation_count(&storage_root, "op-status-skip"), 0);
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn lists_a_laboratory_week_across_projects_in_time_order() {
        let storage_root = temporary_storage_root("laboratory-week-list");
        initialize_storage(&storage_root);
        create_project_for_test(&storage_root, "CEM-WEEK-001", true);
        create_project_for_test(&storage_root, "CEM-WEEK-002", true);
        let mut first = schedule_input("CEM-WEEK-001", "PLAN-WEEK-001", "Alice", "Labo 1");
        first.planned_start_at = "2026-07-13T13:00".to_owned();
        first.planned_end_at = "2026-07-13T16:00".to_owned();
        let mut second = schedule_input("CEM-WEEK-002", "PLAN-WEEK-002", "Bob", "Labo 2");
        second.planned_start_at = "2026-07-14T09:00".to_owned();
        second.planned_end_at = "2026-07-14T11:00".to_owned();
        let mut next_week = schedule_input("CEM-WEEK-002", "PLAN-NEXT-001", "Claire", "Labo 3");
        next_week.planned_start_at = "2026-07-20T09:00".to_owned();
        next_week.planned_end_at = "2026-07-20T11:00".to_owned();
        create_service_schedule_item(&storage_root, first).unwrap();
        create_service_schedule_item(&storage_root, second).unwrap();
        create_service_schedule_item(&storage_root, next_week).unwrap();

        let week = list_laboratory_week_schedule(&storage_root, "2026-07-13").unwrap();

        assert!(week.contains("\"week_start\":\"2026-07-13\""));
        assert!(week.contains("\"week_end\":\"2026-07-17\""));
        assert!(week.contains("PLAN-WEEK-001"));
        assert!(week.contains("PLAN-WEEK-002"));
        assert!(!week.contains("PLAN-NEXT-001"));
        assert!(week.contains("\"customer_name\":\"Client ferroviaire\""));
        assert!(week.contains("\"can_reschedule\":true"));
        assert!(week.find("PLAN-WEEK-001") < week.find("PLAN-WEEK-002"));
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn reschedules_replays_and_rejects_conflicts_without_partial_evidence() {
        let storage_root = temporary_storage_root("service-schedule-reschedule");
        initialize_storage(&storage_root);
        create_project_for_test(&storage_root, "CEM-MOVE-001", true);
        create_project_for_test(&storage_root, "CEM-MOVE-002", true);
        create_service_schedule_item(
            &storage_root,
            schedule_input("CEM-MOVE-001", "PLAN-BUSY-001", "Alice", "Labo 1"),
        )
        .unwrap();
        let mut movable = schedule_input("CEM-MOVE-002", "PLAN-MOVE-001", "Bob", "Labo 2");
        movable.planned_start_at = "2026-07-15T13:00".to_owned();
        movable.planned_end_at = "2026-07-15T16:00".to_owned();
        create_service_schedule_item(&storage_root, movable).unwrap();

        let conflict = reschedule_service_schedule_item(
            &storage_root,
            reschedule_input(
                "CEM-MOVE-002",
                "PLAN-MOVE-001",
                "2026-07-15T10:00",
                "2026-07-15T11:00",
                "Alice",
                "Labo 2",
                1,
                "conflict",
            ),
        )
        .unwrap_err();
        assert_eq!(conflict.code, "service_schedule_operator_conflict");
        assert!(conflict.to_json().contains("PLAN-BUSY-001"));
        assert_eq!(operation_count(&storage_root, "op-reschedule-conflict"), 0);

        let input = reschedule_input(
            "CEM-MOVE-002",
            "PLAN-MOVE-001",
            "2026-07-16T09:00",
            "2026-07-16T12:00",
            "Alice",
            "Labo 2",
            1,
            "success",
        );
        let moved = reschedule_service_schedule_item(&storage_root, input.clone()).unwrap();
        let replayed = reschedule_service_schedule_item(&storage_root, input).unwrap();
        let audits = list_audit_events(&storage_root, "CEM-MOVE-002").unwrap();
        let outbox = list_sync_outbox(&storage_root).unwrap();

        assert!(moved.contains("\"planned_start_at\":\"2026-07-16T09:00\""));
        assert!(moved.contains("\"revision\":2"));
        assert!(moved.contains("\"replayed\":false"));
        assert!(replayed.contains("\"replayed\":true"));
        assert!(audits.contains("service_schedule_item_rescheduled"));
        assert!(audits.contains("2026-07-15T13:00"));
        assert!(audits.contains("2026-07-16T09:00"));
        assert!(outbox.contains("service_schedule_item_rescheduled"));
        assert_eq!(operation_count(&storage_root, "op-reschedule-success"), 1);

        let stale = reschedule_service_schedule_item(
            &storage_root,
            reschedule_input(
                "CEM-MOVE-002",
                "PLAN-MOVE-001",
                "2026-07-17T09:00",
                "2026-07-17T12:00",
                "Alice",
                "Labo 2",
                1,
                "stale",
            ),
        )
        .unwrap_err();
        assert_eq!(stale.code, "service_schedule_concurrent_update");
        assert_eq!(operation_count(&storage_root, "op-reschedule-stale"), 0);
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn refuses_rescheduling_after_the_test_has_started() {
        let storage_root = temporary_storage_root("service-schedule-reschedule-state");
        initialize_storage(&storage_root);
        create_project_for_test(&storage_root, "CEM-MOVE-STATE", true);
        create_service_schedule_item(
            &storage_root,
            schedule_input("CEM-MOVE-STATE", "PLAN-MOVE-STATE", "Alice", "Labo 1"),
        )
        .unwrap();
        transition_service_schedule_item(
            &storage_root,
            transition_input(
                "CEM-MOVE-STATE",
                "PLAN-MOVE-STATE",
                "confirmed",
                1,
                "move-confirm",
            ),
        )
        .unwrap();
        transition_service_schedule_item(
            &storage_root,
            transition_input(
                "CEM-MOVE-STATE",
                "PLAN-MOVE-STATE",
                "in_progress",
                2,
                "move-start",
            ),
        )
        .unwrap();

        let error = reschedule_service_schedule_item(
            &storage_root,
            reschedule_input(
                "CEM-MOVE-STATE",
                "PLAN-MOVE-STATE",
                "2026-07-16T09:00",
                "2026-07-16T12:00",
                "Alice",
                "Labo 1",
                3,
                "started",
            ),
        )
        .unwrap_err();

        assert_eq!(error.code, "service_schedule_item_not_reschedulable");
        assert_eq!(operation_count(&storage_root, "op-reschedule-started"), 0);
        remove_temporary_storage_root(&storage_root);
    }

    fn schedule_input(
        project_code: &str,
        item_code: &str,
        operator: &str,
        location: &str,
    ) -> CreateServiceScheduleItemInput {
        CreateServiceScheduleItemInput {
            project_code: project_code.to_owned(),
            item_code: item_code.to_owned(),
            title: "Émission conduite".to_owned(),
            planned_start_at: "2026-07-15T09:00".to_owned(),
            planned_end_at: "2026-07-15T12:00".to_owned(),
            assigned_operator: operator.to_owned(),
            location: location.to_owned(),
            equipment_under_test: "Convertisseur ferroviaire".to_owned(),
            test_category_code: Some("emission_conducted".to_owned()),
            test_method_code: None,
            notes: Some("Premier créneau".to_owned()),
            actor: "responsable.laboratoire".to_owned(),
            reason: "Préparation des essais".to_owned(),
            operation_id: format!("op-{item_code}"),
            correlation_id: format!("corr-{item_code}"),
            device_id: "lab-console".to_owned(),
        }
    }

    fn transition_input(
        project_code: &str,
        item_code: &str,
        target_status: &str,
        expected_revision: u64,
        suffix: &str,
    ) -> TransitionServiceScheduleItemInput {
        TransitionServiceScheduleItemInput {
            project_code: project_code.to_owned(),
            item_code: item_code.to_owned(),
            target_status: target_status.to_owned(),
            expected_revision,
            actor: "responsable.laboratoire".to_owned(),
            reason: "Mise à jour du planning".to_owned(),
            operation_id: format!("op-status-{suffix}"),
            correlation_id: format!("corr-status-{suffix}"),
            device_id: "lab-console".to_owned(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn reschedule_input(
        project_code: &str,
        item_code: &str,
        planned_start_at: &str,
        planned_end_at: &str,
        assigned_operator: &str,
        location: &str,
        expected_revision: u64,
        suffix: &str,
    ) -> RescheduleServiceScheduleItemInput {
        RescheduleServiceScheduleItemInput {
            project_code: project_code.to_owned(),
            item_code: item_code.to_owned(),
            planned_start_at: planned_start_at.to_owned(),
            planned_end_at: planned_end_at.to_owned(),
            assigned_operator: assigned_operator.to_owned(),
            location: location.to_owned(),
            expected_revision,
            actor: "responsable.laboratoire".to_owned(),
            reason: "Réorganisation du laboratoire".to_owned(),
            operation_id: format!("op-reschedule-{suffix}"),
            correlation_id: format!("corr-reschedule-{suffix}"),
            device_id: "lab-console".to_owned(),
        }
    }

    fn create_project_for_test(storage_root: &Path, code: &str, ready_for_planning: bool) {
        create_project(
            storage_root,
            CreateProjectInput {
                code: code.to_owned(),
                customer_name: "Client ferroviaire".to_owned(),
                execution_mode: "investigation".to_owned(),
                stage: "contract_review".to_owned(),
                actor: "responsable.laboratoire".to_owned(),
                reason: "Demande reçue".to_owned(),
                operation_id: format!("op-create-{code}"),
                correlation_id: format!("corr-create-{code}"),
                device_id: "lab-console".to_owned(),
            },
        )
        .unwrap();
        if !ready_for_planning {
            return;
        }
        for (index, item) in required_contract_review_items(ExecutionMode::Investigation)
            .into_iter()
            .enumerate()
        {
            complete_review_item(
                storage_root,
                CompleteReviewItemInput {
                    code: code.to_owned(),
                    item: contract_review_item_slug(item).to_owned(),
                    actor: "responsable.laboratoire".to_owned(),
                    comment: Some("Vérifié".to_owned()),
                    operation_id: format!("op-review-{code}-{index}"),
                    correlation_id: format!("corr-review-{code}-{index}"),
                    device_id: "lab-console".to_owned(),
                },
            )
            .unwrap();
        }
        advance_to_test_planning(
            storage_root,
            AdvanceToTestPlanningInput {
                code: code.to_owned(),
                actor: "responsable.laboratoire".to_owned(),
                reason: "Revue terminée".to_owned(),
                operation_id: format!("op-plan-{code}"),
                correlation_id: format!("corr-plan-{code}"),
                device_id: "lab-console".to_owned(),
                deviation_authorized_by: None,
                deviation_reason: None,
            },
        )
        .unwrap();
    }

    fn initialize_storage(storage_root: &Path) {
        run_storage_action(
            StorageAction::Init,
            storage_root.to_path_buf(),
            repo_root().join("storage/sqlite"),
        )
        .unwrap();
    }

    fn table_count(storage_root: &Path, table: &str) -> u64 {
        let connection = Connection::open(storage_root.join("projects.sqlite")).unwrap();
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    fn operation_count(storage_root: &Path, operation_id: &str) -> u64 {
        let connection = Connection::open(storage_root.join("sync.sqlite")).unwrap();
        connection
            .query_row(
                "SELECT COUNT(*) FROM sync_operations WHERE operation_id = ?1",
                [operation_id],
                |row| row.get(0),
            )
            .unwrap()
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
