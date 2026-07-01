use crate::{
    render_json,
    test_template_dto::{
        TestTemplateAuditEventDto, TestTemplateAuditEventsDto, TestTemplateDto,
        TestTemplateEnvelopeDto, TestTemplateListDto, TestTemplateOperationResultDto,
    },
    test_template_repository::{
        approved_method_revision_exists, ensure_test_template_operation_replay,
        existing_test_template_operation, insert_test_template, insert_test_template_audit_event,
        insert_test_template_sync_operation, list_test_templates, load_test_template,
        load_test_template_audit_events, next_test_template_audit_sequence,
        open_test_template_connection, open_test_template_connection_with_sync,
        test_category_exists, update_test_template_status, NewTestTemplateRecord,
        StoredTestTemplate, TestTemplateAuditEventInput, TestTemplateListFilter,
        TestTemplateOperationFingerprintInput, TestTemplateSyncOperationInput,
    },
    AgentError,
};
use emc_locus_core::{AuditActor, AuditReason, DomainError, StableId};
use serde_json::{json, Value};
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateTestTemplateInput {
    pub template_id: String,
    pub template_revision: String,
    pub title: String,
    pub description: String,
    pub category_code: String,
    pub measurement_axis: String,
    pub method_code: Option<String>,
    pub method_revision: Option<String>,
    pub status: String,
    pub variables_json: String,
    pub lock_policy_json: String,
    pub instrumentation_chain_json: String,
    pub sequence_json: String,
    pub limits_json: String,
    pub post_processing_json: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListTestTemplatesInput {
    pub category_code: Option<String>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionTestTemplateStatusInput {
    pub template_id: String,
    pub target_status: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

pub fn create_test_template(
    storage_root: &Path,
    input: CreateTestTemplateInput,
) -> Result<String, AgentError> {
    validate_create_input(&input)?;
    let payload_json = test_template_payload_json(&input);
    let mut connection = open_test_template_connection_with_sync(storage_root)?;
    if let Some(operation) = existing_test_template_operation(&connection, &input.operation_id)? {
        ensure_test_template_operation_replay(
            &operation,
            &input.operation_id,
            TestTemplateOperationFingerprintInput {
                entity_type: "test_template",
                entity_id: &input.template_id,
                operation_kind: "test_template_created",
                base_revision: "rev-0000",
                actor_id: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                payload_json: &payload_json,
            },
        )?;
        let template = load_test_template(&connection, &input.template_id)?.ok_or_else(|| {
            AgentError::new(
                "operation_replay_missing_entity",
                "operation exists but test template is missing",
            )
        })?;
        return Ok(test_template_result_json(
            "test_template_created",
            &input.operation_id,
            true,
            &template,
        ));
    }
    if load_test_template(&connection, &input.template_id)?.is_some() {
        return Err(AgentError::new(
            "test_template_already_exists",
            format!("test template already exists: {}", input.template_id),
        ));
    }
    if !test_category_exists(&connection, &input.category_code)? {
        return Err(AgentError::new(
            "test_template_category_not_found",
            format!(
                "test category does not exist or is inactive: {}",
                input.category_code
            ),
        ));
    }
    if let (Some(method_code), Some(method_revision)) = (
        input.method_code.as_deref(),
        input.method_revision.as_deref(),
    ) {
        if !approved_method_revision_exists(&connection, method_code, method_revision)? {
            return Err(AgentError::new(
                "test_template_method_revision_not_found",
                format!("approved method revision does not exist: {method_code}/{method_revision}"),
            ));
        }
    }

    let now = utc_timestamp()?;
    let sequence = next_test_template_audit_sequence(&connection, &input.template_id)?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_test_template(
        &transaction,
        NewTestTemplateRecord {
            template_id: &input.template_id,
            template_revision: input.template_revision.trim(),
            title: input.title.trim(),
            description: input.description.trim(),
            category_code: &input.category_code,
            measurement_axis: &input.measurement_axis,
            method_code: input.method_code.as_deref().map(str::trim),
            method_revision: input.method_revision.as_deref().map(str::trim),
            status: &input.status,
            variables_json: &input.variables_json,
            lock_policy_json: &input.lock_policy_json,
            instrumentation_chain_json: &input.instrumentation_chain_json,
            sequence_json: &input.sequence_json,
            limits_json: &input.limits_json,
            post_processing_json: &input.post_processing_json,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_test_template_audit_event(
        &transaction,
        TestTemplateAuditEventInput {
            template_id: &input.template_id,
            sequence,
            actor: &input.actor,
            action: "test_template_created",
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            base_revision: "rev-0000",
            resulting_revision: "rev-0001",
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    insert_test_template_sync_operation(
        &transaction,
        TestTemplateSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "test_template",
            entity_id: &input.template_id,
            operation_kind: "test_template_created",
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

    let template = load_test_template(&connection, &input.template_id)?.ok_or_else(|| {
        AgentError::new(
            "test_template_query_failed",
            "test template was not readable after insert",
        )
    })?;
    Ok(test_template_result_json(
        "test_template_created",
        &input.operation_id,
        false,
        &template,
    ))
}

pub fn list_test_template_definitions(
    storage_root: &Path,
    input: ListTestTemplatesInput,
) -> Result<String, AgentError> {
    validate_optional_filter(input.category_code.as_deref(), "category_code")?;
    if let Some(status) = input.status.as_deref() {
        validate_template_status(status)?;
    }
    let connection = open_test_template_connection(storage_root)?;
    let templates = list_test_templates(
        &connection,
        TestTemplateListFilter {
            category_code: input.category_code.as_deref(),
            status: input.status.as_deref(),
        },
    )?;
    Ok(render_json(&TestTemplateListDto {
        test_templates: templates.iter().map(test_template_dto).collect(),
    }))
}

pub fn transition_test_template_status(
    storage_root: &Path,
    input: TransitionTestTemplateStatusInput,
) -> Result<String, AgentError> {
    validate_transition_input(&input)?;
    let operation_kind = transition_operation_kind(&input.target_status)?;
    let payload_json = test_template_transition_payload_json(&input);
    let mut connection = open_test_template_connection_with_sync(storage_root)?;
    if let Some(operation) = existing_test_template_operation(&connection, &input.operation_id)? {
        ensure_test_template_operation_replay(
            &operation,
            &input.operation_id,
            TestTemplateOperationFingerprintInput {
                entity_type: "test_template",
                entity_id: &input.template_id,
                operation_kind,
                base_revision: &operation.base_revision,
                actor_id: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                payload_json: &payload_json,
            },
        )?;
        let template = load_test_template(&connection, &input.template_id)?.ok_or_else(|| {
            AgentError::new(
                "operation_replay_missing_entity",
                "operation exists but test template is missing",
            )
        })?;
        return Ok(test_template_result_json(
            operation_kind,
            &input.operation_id,
            true,
            &template,
        ));
    }

    let template = load_test_template(&connection, &input.template_id)?.ok_or_else(|| {
        AgentError::new("test_template_not_found", "test template does not exist")
    })?;
    if !is_allowed_template_transition(&template.status, &input.target_status) {
        return Err(AgentError::with_details(
            "test_template_transition_not_allowed",
            "test template cannot transition to requested status",
            json!({
                "template_id": input.template_id,
                "from": template.status,
                "to": input.target_status,
                "allowed": [
                    { "from": "draft", "to": "under_review" },
                    { "from": "under_review", "to": "approved" }
                ]
            }),
        ));
    }

    let now = utc_timestamp()?;
    let sequence = next_test_template_audit_sequence(&connection, &input.template_id)?;
    let base_revision = revision_text(sequence.saturating_sub(1));
    let resulting_revision = revision_text(sequence);
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    update_test_template_status(&transaction, &input.template_id, &input.target_status, &now)?;
    insert_test_template_audit_event(
        &transaction,
        TestTemplateAuditEventInput {
            template_id: &input.template_id,
            sequence,
            actor: &input.actor,
            action: operation_kind,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            base_revision: &base_revision,
            resulting_revision: &resulting_revision,
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    insert_test_template_sync_operation(
        &transaction,
        TestTemplateSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "test_template",
            entity_id: &input.template_id,
            operation_kind,
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

    let template = load_test_template(&connection, &input.template_id)?.ok_or_else(|| {
        AgentError::new(
            "test_template_query_failed",
            "updated test template was not readable after transition",
        )
    })?;
    Ok(test_template_result_json(
        operation_kind,
        &input.operation_id,
        false,
        &template,
    ))
}

pub fn get_test_template_definition(
    storage_root: &Path,
    template_id: &str,
) -> Result<String, AgentError> {
    validate_stable_id(template_id, "template_id")?;
    let connection = open_test_template_connection(storage_root)?;
    let template = load_test_template(&connection, template_id)?.ok_or_else(|| {
        AgentError::new("test_template_not_found", "test template does not exist")
    })?;
    Ok(render_json(&TestTemplateEnvelopeDto {
        test_template: test_template_dto(&template),
    }))
}

pub fn list_test_template_audit_events(
    storage_root: &Path,
    template_id: &str,
) -> Result<String, AgentError> {
    validate_stable_id(template_id, "template_id")?;
    let connection = open_test_template_connection(storage_root)?;
    load_test_template(&connection, template_id)?.ok_or_else(|| {
        AgentError::new("test_template_not_found", "test template does not exist")
    })?;
    let events = load_test_template_audit_events(&connection, template_id)?;
    Ok(render_json(&TestTemplateAuditEventsDto {
        template_id: template_id.to_owned(),
        audit_events: events
            .iter()
            .map(|event| TestTemplateAuditEventDto {
                sequence: event.sequence,
                actor: event.actor.clone(),
                action: event.action.clone(),
                reason: event.reason.clone(),
                operation_id: event.operation_id.clone(),
                correlation_id: event.correlation_id.clone(),
                device_id: event.device_id.clone(),
                base_revision: event.base_revision.clone(),
                resulting_revision: event.resulting_revision.clone(),
                payload_json: event.payload_json.clone(),
                occurred_at: event.occurred_at.clone(),
            })
            .collect(),
    }))
}

fn validate_create_input(input: &CreateTestTemplateInput) -> Result<(), AgentError> {
    validate_stable_id(&input.template_id, "template_id")?;
    validate_non_empty(&input.template_revision, "template_revision")?;
    validate_non_empty(&input.title, "title")?;
    validate_non_empty(&input.description, "description")?;
    validate_stable_id(&input.category_code, "category_code")?;
    validate_measurement_axis(&input.measurement_axis)?;
    validate_template_status(&input.status)?;
    if input.status != "draft" {
        return Err(AgentError::with_details(
            "invalid_test_template",
            "new test templates must start as draft",
            json!({ "field": "status", "value": input.status }),
        ));
    }
    match (
        input.method_code.as_deref(),
        input.method_revision.as_deref(),
    ) {
        (Some(method_code), Some(method_revision)) => {
            validate_stable_id(method_code, "method_code")?;
            validate_non_empty(method_revision, "method_revision")?;
        }
        (None, None) => {}
        _ => {
            return Err(AgentError::new(
                "invalid_test_template",
                "method_code and method_revision must be provided together",
            ));
        }
    }
    validate_json_object(&input.variables_json, "variables")?;
    validate_json_object(&input.lock_policy_json, "lock_policy")?;
    validate_json_array(&input.instrumentation_chain_json, "instrumentation_chain")?;
    validate_json_array(&input.sequence_json, "sequence")?;
    validate_json_array(&input.limits_json, "limits")?;
    validate_json_array(&input.post_processing_json, "post_processing")?;
    AuditActor::parse(input.actor.clone()).map_err(domain_error)?;
    AuditReason::parse(input.reason.clone()).map_err(domain_error)?;
    validate_stable_id(&input.operation_id, "operation_id")?;
    validate_stable_id(&input.correlation_id, "correlation_id")?;
    validate_stable_id(&input.device_id, "device_id")?;
    Ok(())
}

fn validate_transition_input(input: &TransitionTestTemplateStatusInput) -> Result<(), AgentError> {
    validate_stable_id(&input.template_id, "template_id")?;
    validate_transition_target(&input.target_status)?;
    AuditActor::parse(input.actor.clone()).map_err(domain_error)?;
    AuditReason::parse(input.reason.clone()).map_err(domain_error)?;
    validate_stable_id(&input.operation_id, "operation_id")?;
    validate_stable_id(&input.correlation_id, "correlation_id")?;
    validate_stable_id(&input.device_id, "device_id")?;
    Ok(())
}

fn validate_optional_filter(value: Option<&str>, field: &'static str) -> Result<(), AgentError> {
    if let Some(value) = value {
        validate_stable_id(value, field)?;
    }
    Ok(())
}

fn validate_stable_id(value: &str, field: &'static str) -> Result<(), AgentError> {
    StableId::parse(value.to_owned()).map_err(|error| {
        let mut agent_error = domain_error(error);
        agent_error.message = format!("{field} must be a non-empty stable ASCII token");
        agent_error
    })?;
    Ok(())
}

fn validate_non_empty(value: &str, field: &'static str) -> Result<(), AgentError> {
    if value.trim().is_empty() {
        return Err(AgentError::with_details(
            "invalid_test_template",
            format!("{field} is required"),
            json!({ "field": field }),
        ));
    }
    Ok(())
}

fn validate_measurement_axis(value: &str) -> Result<(), AgentError> {
    validate_enum(
        value,
        "measurement_axis",
        &[
            "frequency_sweep",
            "time_series",
            "event_triggered",
            "mixed_time_frequency",
        ],
    )
}

fn validate_transition_target(value: &str) -> Result<(), AgentError> {
    validate_enum(value, "target_status", &["under_review", "approved"])
}

fn validate_template_status(value: &str) -> Result<(), AgentError> {
    validate_enum(
        value,
        "status",
        &[
            "draft",
            "under_review",
            "approved",
            "suspended",
            "superseded",
            "retired",
        ],
    )
}

fn validate_enum(value: &str, field: &'static str, allowed: &[&str]) -> Result<(), AgentError> {
    if allowed.contains(&value) {
        return Ok(());
    }
    Err(AgentError::with_details(
        "invalid_test_template",
        format!("{field} has an unsupported value"),
        json!({ "field": field, "value": value, "allowed": allowed }),
    ))
}

fn transition_operation_kind(target_status: &str) -> Result<&'static str, AgentError> {
    match target_status {
        "under_review" => Ok("test_template_submitted_for_review"),
        "approved" => Ok("test_template_approved"),
        other => Err(AgentError::with_details(
            "invalid_test_template",
            "target_status has an unsupported transition value",
            json!({ "field": "target_status", "value": other }),
        )),
    }
}

fn is_allowed_template_transition(current_status: &str, target_status: &str) -> bool {
    matches!(
        (current_status, target_status),
        ("draft", "under_review") | ("under_review", "approved")
    )
}

fn validate_json_object(value: &str, field: &'static str) -> Result<Value, AgentError> {
    let parsed = serde_json::from_str::<Value>(value).map_err(|error| {
        AgentError::with_details(
            "invalid_test_template",
            format!("{field} must be valid JSON: {error}"),
            json!({ "field": field }),
        )
    })?;
    if !parsed.is_object() {
        return Err(AgentError::with_details(
            "invalid_test_template",
            format!("{field} must be a JSON object"),
            json!({ "field": field }),
        ));
    }
    Ok(parsed)
}

fn validate_json_array(value: &str, field: &'static str) -> Result<Value, AgentError> {
    let parsed = serde_json::from_str::<Value>(value).map_err(|error| {
        AgentError::with_details(
            "invalid_test_template",
            format!("{field} must be valid JSON: {error}"),
            json!({ "field": field }),
        )
    })?;
    if !parsed.is_array() {
        return Err(AgentError::with_details(
            "invalid_test_template",
            format!("{field} must be a JSON array"),
            json!({ "field": field }),
        ));
    }
    Ok(parsed)
}

fn domain_error(error: DomainError) -> AgentError {
    match error {
        DomainError::EmptyAuditActor => AgentError::new("invalid_actor", "actor is required"),
        DomainError::EmptyAuditReason => AgentError::new("invalid_reason", "reason is required"),
        other => AgentError::new("domain_error", format!("{other:?}")),
    }
}

fn revision_text(sequence: u64) -> String {
    format!("rev-{sequence:04}")
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_error", error.to_string()))
}

fn test_template_payload_json(input: &CreateTestTemplateInput) -> String {
    render_json(&json!({
        "template_id": input.template_id,
        "template_revision": input.template_revision.trim(),
        "title": input.title.trim(),
        "description": input.description.trim(),
        "category_code": input.category_code,
        "measurement_axis": input.measurement_axis,
        "method_code": input.method_code.as_deref().map(str::trim),
        "method_revision": input.method_revision.as_deref().map(str::trim),
        "status": input.status,
        "variables": parse_json_value(&input.variables_json),
        "lock_policy": parse_json_value(&input.lock_policy_json),
        "instrumentation_chain": parse_json_value(&input.instrumentation_chain_json),
        "sequence": parse_json_value(&input.sequence_json),
        "limits": parse_json_value(&input.limits_json),
        "post_processing": parse_json_value(&input.post_processing_json),
    }))
}

fn test_template_transition_payload_json(input: &TransitionTestTemplateStatusInput) -> String {
    render_json(&json!({
        "template_id": input.template_id,
        "target_status": input.target_status,
        "reason": input.reason.trim(),
    }))
}

fn test_template_result_json(
    operation: &str,
    operation_id: &str,
    replayed: bool,
    template: &StoredTestTemplate,
) -> String {
    render_json(&TestTemplateOperationResultDto {
        operation: operation.to_owned(),
        operation_id: operation_id.to_owned(),
        replayed,
        test_template: test_template_dto(template),
    })
}

fn test_template_dto(template: &StoredTestTemplate) -> TestTemplateDto {
    TestTemplateDto {
        template_id: template.template_id.clone(),
        template_revision: template.template_revision.clone(),
        title: template.title.clone(),
        description: template.description.clone(),
        category_code: template.category_code.clone(),
        measurement_axis: template.measurement_axis.clone(),
        method_code: template.method_code.clone(),
        method_revision: template.method_revision.clone(),
        status: template.status.clone(),
        variables: parse_json_value(&template.variables_json),
        lock_policy: parse_json_value(&template.lock_policy_json),
        instrumentation_chain: parse_json_value(&template.instrumentation_chain_json),
        sequence: parse_json_value(&template.sequence_json),
        limits: parse_json_value(&template.limits_json),
        post_processing: parse_json_value(&template.post_processing_json),
        created_by: template.created_by.clone(),
        created_at: template.created_at.clone(),
        updated_at: template.updated_at.clone(),
    }
}

fn parse_json_value(value: &str) -> Value {
    serde_json::from_str(value).expect("persisted test-template JSON must be valid")
}
