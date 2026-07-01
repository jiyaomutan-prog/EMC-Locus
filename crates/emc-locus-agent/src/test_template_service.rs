use crate::{
    render_json,
    test_template_dto::{
        TestTemplateAggregateDto, TestTemplateAuditEventDto, TestTemplateAuditEventsDto,
        TestTemplateDefinitionValidationDto, TestTemplateDefinitionValidationIssueDto,
        TestTemplateEnvelopeDto, TestTemplateIdentityDto, TestTemplateListDto,
        TestTemplateOperationResultDto, TestTemplateRevisionDto, TestTemplateRevisionEnvelopeDto,
        TestTemplateRevisionListDto,
    },
    test_template_repository::{
        approved_method_revision_exists, ensure_test_template_operation_replay,
        existing_test_template_operation, insert_test_template_audit_event,
        insert_test_template_identity, insert_test_template_revision,
        insert_test_template_sync_operation, list_approved_test_template_revisions,
        list_test_template_identities, list_test_template_revisions as load_revision_history,
        load_active_draft_test_template_revision, load_current_approved_test_template_revision,
        load_latest_test_template_revision, load_test_template_audit_events,
        load_test_template_identity, load_test_template_revision,
        next_test_template_revision_number, open_test_template_connection,
        open_test_template_connection_with_sync, supersede_approved_test_template_revision,
        test_category_exists, touch_test_template_identity,
        update_test_template_revision_definition, update_test_template_revision_status,
        NewTestTemplateIdentityRecord, NewTestTemplateRevisionRecord, StoredTestTemplateAggregate,
        StoredTestTemplateIdentity, StoredTestTemplateOperation, StoredTestTemplateRevision,
        TestTemplateAuditEventInput, TestTemplateListFilter, TestTemplateOperationFingerprintInput,
        TestTemplateSyncOperationInput, UpdateTestTemplateRevisionDefinitionInput,
        UpdateTestTemplateRevisionStatusInput,
    },
    AgentError,
};
use emc_locus_core::{
    test_definitions::{
        CanonicalTestTemplateDefinition, TemplateRevisionStatus, TestTemplateDefinition,
        TestTemplateValidationError,
    },
    AuditActor, AuditReason, DomainError, StableId,
};
use serde_json::{json, Value};
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateTestTemplateInput {
    pub template_id: String,
    pub title: String,
    pub category_code: String,
    pub definition_json: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListTestTemplatesInput {
    pub category_code: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplaceTestTemplateDefinitionInput {
    pub template_id: String,
    pub revision_id: String,
    pub expected_definition_checksum: String,
    pub definition_json: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateTestTemplateRevisionInput {
    pub template_id: String,
    pub source_revision_id: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloneTestTemplateInput {
    pub source_template_id: String,
    pub source_revision_id: Option<String>,
    pub new_template_id: String,
    pub title: String,
    pub category_code: Option<String>,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionTestTemplateRevisionInput {
    pub template_id: String,
    pub revision_id: String,
    pub target_status: TemplateRevisionStatus,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

pub fn validate_test_template_definition_json(definition_json: &str) -> Result<String, AgentError> {
    match TestTemplateDefinition::from_json_str(definition_json) {
        Ok(definition) => match definition.canonicalize() {
            Ok(canonical) => Ok(render_json(&TestTemplateDefinitionValidationDto {
                valid: true,
                issues: Vec::new(),
                definition_schema_version: Some(canonical.definition_schema_version),
                definition_checksum: Some(canonical.definition_checksum),
                canonical_json: Some(canonical.canonical_json),
            })),
            Err(error) => Ok(render_validation_error(error)),
        },
        Err(error) => Ok(render_validation_error(error)),
    }
}

pub fn create_test_template(
    storage_root: &Path,
    input: CreateTestTemplateInput,
) -> Result<String, AgentError> {
    validate_create_input(&input)?;
    let definition = canonical_definition(&input.definition_json)?;
    let revision_id = revision_id_for(&input.template_id, 1);
    let payload_json = create_payload_json(&input, &definition);
    let mut connection = open_test_template_connection_with_sync(storage_root)?;

    if let Some(operation) = existing_test_template_operation(&connection, &input.operation_id)? {
        ensure_test_template_operation_replay(
            &operation,
            &input.operation_id,
            TestTemplateOperationFingerprintInput {
                entity_type: "test_template_revision",
                template_id: &input.template_id,
                revision_id: Some(&revision_id),
                action: "test_template_created",
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: None,
                new_revision_id: Some(&revision_id),
                old_definition_checksum: None,
                new_definition_checksum: Some(&definition.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return operation_replay_result(&connection, &operation);
    }
    if load_test_template_identity(&connection, &input.template_id)?.is_some() {
        return Err(AgentError::new(
            "test_template_already_exists",
            format!("test template already exists: {}", input.template_id),
        ));
    }
    ensure_category_and_method(&connection, &input.category_code, &definition)?;

    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_test_template_identity(
        &transaction,
        NewTestTemplateIdentityRecord {
            template_id: &input.template_id,
            title: input.title.trim(),
            category_code: &input.category_code,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_test_template_revision(
        &transaction,
        NewTestTemplateRevisionRecord {
            revision_id: &revision_id,
            template_id: &input.template_id,
            revision_number: 1,
            parent_revision_id: None,
            status: revision_status_text(&TemplateRevisionStatus::Draft),
            definition_schema_version: &definition.definition_schema_version,
            definition_json: &definition.canonical_json,
            definition_checksum: &definition.definition_checksum,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_test_template_audit_event(
        &transaction,
        TestTemplateAuditEventInput {
            template_id: &input.template_id,
            revision_id: Some(&revision_id),
            action: "test_template_created",
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: None,
            new_revision_id: Some(&revision_id),
            old_definition_checksum: None,
            new_definition_checksum: Some(&definition.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    insert_test_template_sync_operation(
        &transaction,
        TestTemplateSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "test_template_revision",
            entity_id: &revision_id,
            operation_kind: "test_template_created",
            base_revision: "none",
            resulting_revision: &revision_id,
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

    operation_result_for_revision(
        &connection,
        "test_template_created",
        &input.operation_id,
        false,
        &input.template_id,
        &revision_id,
    )
}

pub fn list_test_template_definitions(
    storage_root: &Path,
    input: ListTestTemplatesInput,
) -> Result<String, AgentError> {
    validate_optional_id(input.category_code.as_deref(), "category_code")?;
    let connection = open_test_template_connection(storage_root)?;
    let identities = list_test_template_identities(
        &connection,
        TestTemplateListFilter {
            category_code: input.category_code.as_deref(),
        },
    )?;
    let test_templates = identities
        .iter()
        .map(|identity| aggregate_for_identity(&connection, identity))
        .collect::<Result<Vec<_>, AgentError>>()?;
    Ok(render_json(&TestTemplateListDto {
        test_templates: test_templates.iter().map(aggregate_dto).collect(),
    }))
}

pub fn get_test_template_definition(
    storage_root: &Path,
    template_id: &str,
) -> Result<String, AgentError> {
    validate_stable_id(template_id, "template_id")?;
    let connection = open_test_template_connection(storage_root)?;
    let aggregate = load_aggregate(&connection, template_id)?;
    Ok(render_json(&TestTemplateEnvelopeDto {
        test_template: aggregate_dto(&aggregate),
    }))
}

pub fn list_test_template_revisions(
    storage_root: &Path,
    template_id: &str,
) -> Result<String, AgentError> {
    validate_stable_id(template_id, "template_id")?;
    let connection = open_test_template_connection(storage_root)?;
    load_test_template_identity(&connection, template_id)?.ok_or_else(|| {
        AgentError::new("test_template_not_found", "test template does not exist")
    })?;
    let revisions = load_revision_history(&connection, template_id)?;
    Ok(render_json(&TestTemplateRevisionListDto {
        template_id: template_id.to_owned(),
        revisions: revisions.iter().map(revision_dto).collect(),
    }))
}

pub fn get_test_template_revision(
    storage_root: &Path,
    template_id: &str,
    revision_id: &str,
) -> Result<String, AgentError> {
    validate_stable_id(template_id, "template_id")?;
    validate_stable_id(revision_id, "revision_id")?;
    let connection = open_test_template_connection(storage_root)?;
    let revision = load_required_revision(&connection, template_id, revision_id)?;
    Ok(render_json(&TestTemplateRevisionEnvelopeDto {
        revision: revision_dto(&revision),
    }))
}

pub fn replace_test_template_revision_definition(
    storage_root: &Path,
    input: ReplaceTestTemplateDefinitionInput,
) -> Result<String, AgentError> {
    validate_replace_input(&input)?;
    let definition = canonical_definition(&input.definition_json)?;
    let payload_json = replace_definition_payload_json(&input, &definition);
    let mut connection = open_test_template_connection_with_sync(storage_root)?;

    if let Some(operation) = existing_test_template_operation(&connection, &input.operation_id)? {
        ensure_test_template_operation_replay(
            &operation,
            &input.operation_id,
            TestTemplateOperationFingerprintInput {
                entity_type: "test_template_revision",
                template_id: &input.template_id,
                revision_id: Some(&input.revision_id),
                action: "test_template_definition_replaced",
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: Some(&input.revision_id),
                new_revision_id: Some(&input.revision_id),
                old_definition_checksum: Some(&input.expected_definition_checksum),
                new_definition_checksum: Some(&definition.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return operation_replay_result(&connection, &operation);
    }

    let revision = load_required_revision(&connection, &input.template_id, &input.revision_id)?;
    if revision.status != revision_status_text(&TemplateRevisionStatus::Draft) {
        return Err(AgentError::with_details(
            "test_template_revision_immutable",
            "only draft template revisions can be modified",
            json!({
                "template_id": input.template_id,
                "revision_id": input.revision_id,
                "status": revision.status,
            }),
        ));
    }
    if revision.definition_checksum != input.expected_definition_checksum {
        return Err(AgentError::with_details(
            "test_template_definition_checksum_mismatch",
            "draft definition was modified by another operation",
            json!({
                "template_id": input.template_id,
                "revision_id": input.revision_id,
                "expected_definition_checksum": input.expected_definition_checksum,
                "actual_definition_checksum": revision.definition_checksum,
            }),
        ));
    }
    ensure_category_and_method(
        &connection,
        &revision_category(&connection, &input.template_id)?,
        &definition,
    )?;

    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let updated = update_test_template_revision_definition(
        &transaction,
        UpdateTestTemplateRevisionDefinitionInput {
            template_id: &input.template_id,
            revision_id: &input.revision_id,
            expected_definition_checksum: &input.expected_definition_checksum,
            definition_schema_version: &definition.definition_schema_version,
            definition_json: &definition.canonical_json,
            definition_checksum: &definition.definition_checksum,
            timestamp: &now,
        },
    )?;
    if updated == 0 {
        drop(transaction);
        return definition_update_conflict(&connection, &input);
    }
    touch_test_template_identity(&transaction, &input.template_id, &now)?;
    insert_test_template_audit_event(
        &transaction,
        TestTemplateAuditEventInput {
            template_id: &input.template_id,
            revision_id: Some(&input.revision_id),
            action: "test_template_definition_replaced",
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: Some(&input.revision_id),
            new_revision_id: Some(&input.revision_id),
            old_definition_checksum: Some(&input.expected_definition_checksum),
            new_definition_checksum: Some(&definition.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    insert_test_template_sync_operation(
        &transaction,
        TestTemplateSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "test_template_revision",
            entity_id: &input.revision_id,
            operation_kind: "test_template_definition_replaced",
            base_revision: &definition_cursor(
                "draft-before",
                &input.revision_id,
                &input.expected_definition_checksum,
            ),
            resulting_revision: &definition_cursor(
                "draft-after",
                &input.revision_id,
                &definition.definition_checksum,
            ),
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

    operation_result_for_revision(
        &connection,
        "test_template_definition_replaced",
        &input.operation_id,
        false,
        &input.template_id,
        &input.revision_id,
    )
}

pub fn create_test_template_revision(
    storage_root: &Path,
    input: CreateTestTemplateRevisionInput,
) -> Result<String, AgentError> {
    validate_create_revision_input(&input)?;
    let payload_json = create_revision_payload_json(&input);
    let mut connection = open_test_template_connection_with_sync(storage_root)?;
    let identity =
        load_test_template_identity(&connection, &input.template_id)?.ok_or_else(|| {
            AgentError::new("test_template_not_found", "test template does not exist")
        })?;
    let source =
        load_required_revision(&connection, &input.template_id, &input.source_revision_id)?;
    if source.status != revision_status_text(&TemplateRevisionStatus::Approved) {
        return Err(AgentError::with_details(
            "test_template_revision_source_not_approved",
            "new template revisions must derive from an approved revision",
            json!({
                "template_id": input.template_id,
                "source_revision_id": input.source_revision_id,
                "status": source.status,
            }),
        ));
    }

    if let Some(operation) = existing_test_template_operation(&connection, &input.operation_id)? {
        let replay_revision_id = operation.revision_id.as_deref().ok_or_else(|| {
            AgentError::new(
                "operation_replay_missing_entity",
                "operation exists but template revision is missing",
            )
        })?;
        ensure_test_template_operation_replay(
            &operation,
            &input.operation_id,
            TestTemplateOperationFingerprintInput {
                entity_type: "test_template_revision",
                template_id: &input.template_id,
                revision_id: Some(replay_revision_id),
                action: "test_template_revision_created",
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: Some(&input.source_revision_id),
                new_revision_id: Some(replay_revision_id),
                old_definition_checksum: Some(&source.definition_checksum),
                new_definition_checksum: Some(&source.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return operation_replay_result(&connection, &operation);
    }
    if let Some(active_draft) =
        load_active_draft_test_template_revision(&connection, &input.template_id)?
    {
        return Err(AgentError::with_details(
            "test_template_active_draft_exists",
            "a template identity can only have one active draft revision",
            json!({
                "template_id": input.template_id,
                "existing_draft_revision_id": active_draft.revision_id,
                "source_revision_id": input.source_revision_id,
            }),
        ));
    }

    let revision_number = next_test_template_revision_number(&connection, &input.template_id)?;
    let revision_id = revision_id_for(&input.template_id, revision_number);

    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_test_template_revision(
        &transaction,
        NewTestTemplateRevisionRecord {
            revision_id: &revision_id,
            template_id: &input.template_id,
            revision_number,
            parent_revision_id: Some(&input.source_revision_id),
            status: revision_status_text(&TemplateRevisionStatus::Draft),
            definition_schema_version: &source.definition_schema_version,
            definition_json: &source.definition_json,
            definition_checksum: &source.definition_checksum,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    touch_test_template_identity(&transaction, &identity.template_id, &now)?;
    insert_test_template_audit_event(
        &transaction,
        TestTemplateAuditEventInput {
            template_id: &input.template_id,
            revision_id: Some(&revision_id),
            action: "test_template_revision_created",
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: Some(&input.source_revision_id),
            new_revision_id: Some(&revision_id),
            old_definition_checksum: Some(&source.definition_checksum),
            new_definition_checksum: Some(&source.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    insert_test_template_sync_operation(
        &transaction,
        TestTemplateSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "test_template_revision",
            entity_id: &revision_id,
            operation_kind: "test_template_revision_created",
            base_revision: &input.source_revision_id,
            resulting_revision: &revision_id,
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

    operation_result_for_revision(
        &connection,
        "test_template_revision_created",
        &input.operation_id,
        false,
        &input.template_id,
        &revision_id,
    )
}

pub fn clone_test_template(
    storage_root: &Path,
    input: CloneTestTemplateInput,
) -> Result<String, AgentError> {
    validate_clone_input(&input)?;
    let mut connection = open_test_template_connection_with_sync(storage_root)?;
    let source_identity = load_test_template_identity(&connection, &input.source_template_id)?
        .ok_or_else(|| {
            AgentError::new(
                "test_template_not_found",
                "source test template does not exist",
            )
        })?;
    let source_revision = match input.source_revision_id.as_deref() {
        Some(source_revision_id) => {
            load_required_revision(&connection, &input.source_template_id, source_revision_id)?
        }
        None => source_identity
            .current_approved_revision_id
            .as_deref()
            .ok_or_else(|| {
                AgentError::with_details(
                    "test_template_revision_source_not_approved",
                    "source template has no approved revision to clone",
                    json!({ "template_id": input.source_template_id }),
                )
            })
            .and_then(|revision_id| {
                load_required_revision(&connection, &input.source_template_id, revision_id)
            })?,
    };
    if source_revision.status != revision_status_text(&TemplateRevisionStatus::Approved) {
        return Err(AgentError::with_details(
            "test_template_revision_source_not_approved",
            "template clone source must be an approved revision",
            json!({
                "template_id": input.source_template_id,
                "source_revision_id": source_revision.revision_id,
                "status": source_revision.status,
            }),
        ));
    }
    let category_code = input
        .category_code
        .as_deref()
        .unwrap_or(&source_identity.category_code);
    let definition = canonical_definition(&source_revision.definition_json)?;
    let payload_json = clone_payload_json(&input, category_code, &source_revision, &definition);
    let revision_id = revision_id_for(&input.new_template_id, 1);

    if let Some(operation) = existing_test_template_operation(&connection, &input.operation_id)? {
        ensure_test_template_operation_replay(
            &operation,
            &input.operation_id,
            TestTemplateOperationFingerprintInput {
                entity_type: "test_template_revision",
                template_id: &input.new_template_id,
                revision_id: Some(&revision_id),
                action: "test_template_cloned",
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: Some(&source_revision.revision_id),
                new_revision_id: Some(&revision_id),
                old_definition_checksum: Some(&source_revision.definition_checksum),
                new_definition_checksum: Some(&definition.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return operation_replay_result(&connection, &operation);
    }
    if load_test_template_identity(&connection, &input.new_template_id)?.is_some() {
        return Err(AgentError::new(
            "test_template_already_exists",
            format!("test template already exists: {}", input.new_template_id),
        ));
    }
    ensure_category_and_method(&connection, category_code, &definition)?;

    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_test_template_identity(
        &transaction,
        NewTestTemplateIdentityRecord {
            template_id: &input.new_template_id,
            title: input.title.trim(),
            category_code,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_test_template_revision(
        &transaction,
        NewTestTemplateRevisionRecord {
            revision_id: &revision_id,
            template_id: &input.new_template_id,
            revision_number: 1,
            parent_revision_id: None,
            status: revision_status_text(&TemplateRevisionStatus::Draft),
            definition_schema_version: &definition.definition_schema_version,
            definition_json: &definition.canonical_json,
            definition_checksum: &definition.definition_checksum,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_test_template_audit_event(
        &transaction,
        TestTemplateAuditEventInput {
            template_id: &input.new_template_id,
            revision_id: Some(&revision_id),
            action: "test_template_cloned",
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: Some(&source_revision.revision_id),
            new_revision_id: Some(&revision_id),
            old_definition_checksum: Some(&source_revision.definition_checksum),
            new_definition_checksum: Some(&definition.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    insert_test_template_sync_operation(
        &transaction,
        TestTemplateSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "test_template_revision",
            entity_id: &revision_id,
            operation_kind: "test_template_cloned",
            base_revision: &source_revision.revision_id,
            resulting_revision: &revision_id,
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

    operation_result_for_revision(
        &connection,
        "test_template_cloned",
        &input.operation_id,
        false,
        &input.new_template_id,
        &revision_id,
    )
}

pub fn transition_test_template_revision(
    storage_root: &Path,
    input: TransitionTestTemplateRevisionInput,
) -> Result<String, AgentError> {
    validate_transition_input(&input)?;
    let operation_kind = transition_operation_kind(&input.target_status);
    let payload_json = transition_payload_json(&input);
    let mut connection = open_test_template_connection_with_sync(storage_root)?;
    let revision = load_required_revision(&connection, &input.template_id, &input.revision_id)?;

    if let Some(operation) = existing_test_template_operation(&connection, &input.operation_id)? {
        ensure_test_template_operation_replay(
            &operation,
            &input.operation_id,
            TestTemplateOperationFingerprintInput {
                entity_type: "test_template_revision",
                template_id: &input.template_id,
                revision_id: Some(&input.revision_id),
                action: operation_kind,
                actor: &input.actor,
                device_id: &input.device_id,
                correlation_id: &input.correlation_id,
                old_revision_id: Some(&input.revision_id),
                new_revision_id: Some(&input.revision_id),
                old_definition_checksum: Some(&revision.definition_checksum),
                new_definition_checksum: Some(&revision.definition_checksum),
                payload_json: &payload_json,
            },
        )?;
        return operation_replay_result(&connection, &operation);
    }

    let current_status = parse_revision_status(&revision.status)?;
    if !is_allowed_revision_transition(&current_status, &input.target_status) {
        return Err(AgentError::with_details(
            "test_template_revision_transition_not_allowed",
            "template revision cannot transition to requested status",
            json!({
                "template_id": input.template_id,
                "revision_id": input.revision_id,
                "from": revision.status,
                "to": revision_status_text(&input.target_status),
                "allowed": [
                    { "from": "draft", "to": "under_review" },
                    { "from": "under_review", "to": "approved" }
                ]
            }),
        ));
    }
    let approved_revisions_to_supersede = if input.target_status == TemplateRevisionStatus::Approved
    {
        list_approved_test_template_revisions(&connection, &input.template_id)?
            .into_iter()
            .filter(|approved| approved.revision_id != input.revision_id)
            .collect()
    } else {
        Vec::new()
    };

    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let updated = update_test_template_revision_status(
        &transaction,
        UpdateTestTemplateRevisionStatusInput {
            template_id: &input.template_id,
            revision_id: &input.revision_id,
            expected_current_status: &revision.status,
            status: revision_status_text(&input.target_status),
            timestamp: &now,
        },
    )?;
    if updated == 0 {
        drop(transaction);
        return transition_cas_conflict(&connection, &input, &revision.status);
    }
    insert_test_template_audit_event(
        &transaction,
        TestTemplateAuditEventInput {
            template_id: &input.template_id,
            revision_id: Some(&input.revision_id),
            action: operation_kind,
            actor: &input.actor,
            reason: &input.reason,
            operation_id: &input.operation_id,
            correlation_id: &input.correlation_id,
            device_id: &input.device_id,
            old_revision_id: Some(&input.revision_id),
            new_revision_id: Some(&input.revision_id),
            old_definition_checksum: Some(&revision.definition_checksum),
            new_definition_checksum: Some(&revision.definition_checksum),
            payload_json: &payload_json,
            timestamp: &now,
        },
    )?;
    if input.target_status == TemplateRevisionStatus::Approved {
        for superseded_revision in approved_revisions_to_supersede {
            let superseded = supersede_approved_test_template_revision(
                &transaction,
                &input.template_id,
                &superseded_revision.revision_id,
                &now,
            )?;
            if superseded == 0 {
                continue;
            }
            let supersede_operation_id = format!(
                "{}:supersede:{}",
                input.operation_id, superseded_revision.revision_id
            );
            let supersede_payload_json = supersede_payload_json(&input, &superseded_revision);
            insert_test_template_audit_event(
                &transaction,
                TestTemplateAuditEventInput {
                    template_id: &input.template_id,
                    revision_id: Some(&superseded_revision.revision_id),
                    action: "test_template_revision_superseded",
                    actor: &input.actor,
                    reason: &input.reason,
                    operation_id: &supersede_operation_id,
                    correlation_id: &input.correlation_id,
                    device_id: &input.device_id,
                    old_revision_id: Some(&superseded_revision.revision_id),
                    new_revision_id: Some(&input.revision_id),
                    old_definition_checksum: Some(&superseded_revision.definition_checksum),
                    new_definition_checksum: Some(&revision.definition_checksum),
                    payload_json: &supersede_payload_json,
                    timestamp: &now,
                },
            )?;
            insert_test_template_sync_operation(
                &transaction,
                TestTemplateSyncOperationInput {
                    operation_id: &supersede_operation_id,
                    entity_type: "test_template_revision",
                    entity_id: &superseded_revision.revision_id,
                    operation_kind: "test_template_revision_superseded",
                    base_revision: &status_cursor("approved", &superseded_revision.revision_id),
                    resulting_revision: &status_cursor(
                        "superseded",
                        &superseded_revision.revision_id,
                    ),
                    actor_id: &input.actor,
                    device_id: &input.device_id,
                    correlation_id: &input.correlation_id,
                    payload_json: &supersede_payload_json,
                    timestamp: &now,
                },
            )?;
        }
    }
    insert_test_template_sync_operation(
        &transaction,
        TestTemplateSyncOperationInput {
            operation_id: &input.operation_id,
            entity_type: "test_template_revision",
            entity_id: &input.revision_id,
            operation_kind,
            base_revision: &status_cursor(&revision.status, &input.revision_id),
            resulting_revision: &status_cursor(
                revision_status_text(&input.target_status),
                &input.revision_id,
            ),
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

    operation_result_for_revision(
        &connection,
        operation_kind,
        &input.operation_id,
        false,
        &input.template_id,
        &input.revision_id,
    )
}

pub fn list_test_template_audit_events(
    storage_root: &Path,
    template_id: &str,
) -> Result<String, AgentError> {
    validate_stable_id(template_id, "template_id")?;
    let connection = open_test_template_connection(storage_root)?;
    load_test_template_identity(&connection, template_id)?.ok_or_else(|| {
        AgentError::new("test_template_not_found", "test template does not exist")
    })?;
    let events = load_test_template_audit_events(&connection, template_id)?;
    Ok(render_json(&TestTemplateAuditEventsDto {
        template_id: template_id.to_owned(),
        audit_events: events
            .iter()
            .map(|event| TestTemplateAuditEventDto {
                audit_id: event.audit_id,
                template_id: event.template_id.clone(),
                revision_id: event.revision_id.clone(),
                actor: event.actor.clone(),
                action: event.action.clone(),
                reason: event.reason.clone(),
                operation_id: event.operation_id.clone(),
                correlation_id: event.correlation_id.clone(),
                device_id: event.device_id.clone(),
                old_revision_id: event.old_revision_id.clone(),
                new_revision_id: event.new_revision_id.clone(),
                old_definition_checksum: event.old_definition_checksum.clone(),
                new_definition_checksum: event.new_definition_checksum.clone(),
                payload_json: event.payload_json.clone(),
                occurred_at: event.occurred_at.clone(),
            })
            .collect(),
    }))
}

fn validate_create_input(input: &CreateTestTemplateInput) -> Result<(), AgentError> {
    validate_stable_id(&input.template_id, "template_id")?;
    validate_non_empty(&input.title, "title")?;
    validate_stable_id(&input.category_code, "category_code")?;
    validate_operation_context(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )
}

fn validate_replace_input(input: &ReplaceTestTemplateDefinitionInput) -> Result<(), AgentError> {
    validate_stable_id(&input.template_id, "template_id")?;
    validate_stable_id(&input.revision_id, "revision_id")?;
    validate_checksum(
        &input.expected_definition_checksum,
        "expected_definition_checksum",
    )?;
    validate_operation_context(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )
}

fn validate_create_revision_input(
    input: &CreateTestTemplateRevisionInput,
) -> Result<(), AgentError> {
    validate_stable_id(&input.template_id, "template_id")?;
    validate_stable_id(&input.source_revision_id, "source_revision_id")?;
    validate_operation_context(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )
}

fn validate_clone_input(input: &CloneTestTemplateInput) -> Result<(), AgentError> {
    validate_stable_id(&input.source_template_id, "source_template_id")?;
    validate_optional_id(input.source_revision_id.as_deref(), "source_revision_id")?;
    validate_stable_id(&input.new_template_id, "new_template_id")?;
    validate_non_empty(&input.title, "title")?;
    validate_optional_id(input.category_code.as_deref(), "category_code")?;
    validate_operation_context(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )
}

fn validate_transition_input(
    input: &TransitionTestTemplateRevisionInput,
) -> Result<(), AgentError> {
    validate_stable_id(&input.template_id, "template_id")?;
    validate_stable_id(&input.revision_id, "revision_id")?;
    validate_operation_context(
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )
}

fn validate_operation_context(
    actor: &str,
    reason: &str,
    operation_id: &str,
    correlation_id: &str,
    device_id: &str,
) -> Result<(), AgentError> {
    AuditActor::parse(actor.to_owned()).map_err(domain_error)?;
    AuditReason::parse(reason.to_owned()).map_err(domain_error)?;
    validate_stable_id(operation_id, "operation_id")?;
    validate_stable_id(correlation_id, "correlation_id")?;
    validate_stable_id(device_id, "device_id")?;
    Ok(())
}

fn validate_optional_id(value: Option<&str>, field: &'static str) -> Result<(), AgentError> {
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

fn validate_checksum(value: &str, field: &'static str) -> Result<(), AgentError> {
    if value.trim().starts_with("sha256:")
        && value.trim().len() == 71
        && value.trim()["sha256:".len()..]
            .chars()
            .all(|ch| ch.is_ascii_hexdigit())
    {
        return Ok(());
    }
    Err(AgentError::with_details(
        "invalid_test_template",
        format!("{field} must be a sha256 checksum"),
        json!({ "field": field }),
    ))
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

fn canonical_definition(
    definition_json: &str,
) -> Result<CanonicalTestTemplateDefinition, AgentError> {
    let definition =
        TestTemplateDefinition::from_json_str(definition_json).map_err(validation_error)?;
    definition.canonicalize().map_err(validation_error)
}

fn ensure_category_and_method(
    connection: &rusqlite::Connection,
    category_code: &str,
    definition: &CanonicalTestTemplateDefinition,
) -> Result<(), AgentError> {
    if !test_category_exists(connection, category_code)? {
        return Err(AgentError::new(
            "test_template_category_not_found",
            format!("test category does not exist or is inactive: {category_code}"),
        ));
    }
    let definition_value = definition_value(definition)?;
    let method_code = definition_value
        .get("method_code")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let method_revision = definition_value
        .get("method_revision")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if let (Some(method_code), Some(method_revision)) = (method_code, method_revision) {
        if !approved_method_revision_exists(connection, &method_code, &method_revision)? {
            return Err(AgentError::new(
                "test_template_method_revision_not_found",
                format!("approved method revision does not exist: {method_code}/{method_revision}"),
            ));
        }
    }
    Ok(())
}

fn validation_error(error: TestTemplateValidationError) -> AgentError {
    AgentError::with_details(
        "invalid_test_template_definition",
        error.message,
        json!({ "validation_code": error.code }),
    )
}

fn render_validation_error(error: TestTemplateValidationError) -> String {
    render_json(&TestTemplateDefinitionValidationDto {
        valid: false,
        issues: vec![TestTemplateDefinitionValidationIssueDto {
            severity: "error".to_owned(),
            code: error.code.to_owned(),
            path: validation_path_for_code(error.code).to_owned(),
            message: error.message,
        }],
        definition_schema_version: None,
        definition_checksum: None,
        canonical_json: None,
    })
}

fn validation_path_for_code(code: &str) -> &'static str {
    match code {
        "default_value_type_mismatch"
        | "default_below_minimum"
        | "default_above_maximum"
        | "enum_default_not_allowed" => "variables[].default_value",
        "duplicate_variable_id"
        | "empty_enum_values"
        | "duplicate_enum_value"
        | "invalid_numeric_bounds"
        | "missing_variable_unit"
        | "missing_variables" => "variables",
        "unknown_lock_variable" | "duplicate_lock_policy" => "lock_policy",
        "duplicate_slot_id"
        | "missing_slot_requirement"
        | "unknown_slot_reference"
        | "self_slot_reference" => "instrumentation_chain",
        "missing_sequence_steps"
        | "unknown_entry_step"
        | "duplicate_step_id"
        | "duplicate_step_order"
        | "unknown_step_slot_reference"
        | "duplicate_branch_rule_id"
        | "unknown_branch_destination"
        | "undeclared_sequence_cycle"
        | "unreachable_sequence_step" => "sequence",
        "duplicate_limit_id" | "missing_scalar_threshold" | "unknown_limit_variable" => "limits",
        "duplicate_post_processing_operation"
        | "duplicate_post_processing_order"
        | "missing_post_processing_inputs"
        | "missing_post_processing_outputs"
        | "duplicate_post_processing_output"
        | "invalid_post_processing_dependency" => "post_processing",
        _ => "$",
    }
}

fn domain_error(error: DomainError) -> AgentError {
    match error {
        DomainError::EmptyAuditActor => AgentError::new("invalid_actor", "actor is required"),
        DomainError::EmptyAuditReason => AgentError::new("invalid_reason", "reason is required"),
        other => AgentError::new("domain_error", format!("{other:?}")),
    }
}

fn revision_category(
    connection: &rusqlite::Connection,
    template_id: &str,
) -> Result<String, AgentError> {
    load_test_template_identity(connection, template_id)?
        .map(|identity| identity.category_code)
        .ok_or_else(|| AgentError::new("test_template_not_found", "test template does not exist"))
}

fn load_required_revision(
    connection: &rusqlite::Connection,
    template_id: &str,
    revision_id: &str,
) -> Result<StoredTestTemplateRevision, AgentError> {
    load_test_template_revision(connection, template_id, revision_id)?.ok_or_else(|| {
        AgentError::new(
            "test_template_revision_not_found",
            "template revision does not exist",
        )
    })
}

fn definition_update_conflict(
    connection: &rusqlite::Connection,
    input: &ReplaceTestTemplateDefinitionInput,
) -> Result<String, AgentError> {
    let Some(revision) =
        load_test_template_revision(connection, &input.template_id, &input.revision_id)?
    else {
        return Err(AgentError::new(
            "test_template_revision_not_found",
            "template revision does not exist",
        ));
    };
    if revision.status != revision_status_text(&TemplateRevisionStatus::Draft) {
        return Err(AgentError::with_details(
            "test_template_revision_immutable",
            "only draft template revisions can be modified",
            json!({
                "template_id": input.template_id,
                "revision_id": input.revision_id,
                "status": revision.status,
            }),
        ));
    }
    if revision.definition_checksum != input.expected_definition_checksum {
        return Err(AgentError::with_details(
            "test_template_definition_checksum_mismatch",
            "draft definition was modified by another operation",
            json!({
                "template_id": input.template_id,
                "revision_id": input.revision_id,
                "expected_definition_checksum": input.expected_definition_checksum,
                "actual_definition_checksum": revision.definition_checksum,
            }),
        ));
    }
    Err(AgentError::with_details(
        "test_template_definition_concurrent_update",
        "draft definition update lost the compare-and-swap race",
        json!({
            "template_id": input.template_id,
            "revision_id": input.revision_id,
            "expected_definition_checksum": input.expected_definition_checksum,
        }),
    ))
}

fn transition_cas_conflict(
    connection: &rusqlite::Connection,
    input: &TransitionTestTemplateRevisionInput,
    expected_status: &str,
) -> Result<String, AgentError> {
    let Some(revision) =
        load_test_template_revision(connection, &input.template_id, &input.revision_id)?
    else {
        return Err(AgentError::new(
            "test_template_revision_not_found",
            "template revision does not exist",
        ));
    };
    Err(AgentError::with_details(
        "test_template_revision_transition_conflict",
        "template revision status changed before the transition could be committed",
        json!({
            "template_id": input.template_id,
            "revision_id": input.revision_id,
            "expected_status": expected_status,
            "actual_status": revision.status,
            "target_status": revision_status_text(&input.target_status),
        }),
    ))
}

fn load_aggregate(
    connection: &rusqlite::Connection,
    template_id: &str,
) -> Result<StoredTestTemplateAggregate, AgentError> {
    let identity = load_test_template_identity(connection, template_id)?.ok_or_else(|| {
        AgentError::new("test_template_not_found", "test template does not exist")
    })?;
    aggregate_for_identity(connection, &identity)
}

fn aggregate_for_identity(
    connection: &rusqlite::Connection,
    identity: &StoredTestTemplateIdentity,
) -> Result<StoredTestTemplateAggregate, AgentError> {
    Ok(StoredTestTemplateAggregate {
        identity: identity.clone(),
        current_approved_revision: load_current_approved_test_template_revision(
            connection, identity,
        )?,
        latest_revision: load_latest_test_template_revision(connection, &identity.template_id)?,
        active_draft_revision: load_active_draft_test_template_revision(
            connection,
            &identity.template_id,
        )?,
    })
}

fn operation_replay_result(
    connection: &rusqlite::Connection,
    operation: &StoredTestTemplateOperation,
) -> Result<String, AgentError> {
    let revision_id = operation.revision_id.as_deref().ok_or_else(|| {
        AgentError::new(
            "operation_replay_missing_entity",
            "operation exists but template revision is missing",
        )
    })?;
    operation_result_for_revision(
        connection,
        &operation.action,
        &operation.operation_id,
        true,
        &operation.template_id,
        revision_id,
    )
}

fn operation_result_for_revision(
    connection: &rusqlite::Connection,
    operation: &str,
    operation_id: &str,
    replayed: bool,
    template_id: &str,
    revision_id: &str,
) -> Result<String, AgentError> {
    let aggregate = load_aggregate(connection, template_id)?;
    let revision = load_required_revision(connection, template_id, revision_id)?;
    Ok(render_json(&TestTemplateOperationResultDto {
        operation: operation.to_owned(),
        operation_id: operation_id.to_owned(),
        replayed,
        test_template: aggregate_dto(&aggregate),
        revision: revision_dto(&revision),
    }))
}

fn aggregate_dto(aggregate: &StoredTestTemplateAggregate) -> TestTemplateAggregateDto {
    TestTemplateAggregateDto {
        identity: identity_dto(&aggregate.identity),
        current_approved_revision: aggregate
            .current_approved_revision
            .as_ref()
            .map(revision_dto),
        latest_revision: aggregate.latest_revision.as_ref().map(revision_dto),
        active_draft_revision: aggregate.active_draft_revision.as_ref().map(revision_dto),
    }
}

fn identity_dto(identity: &StoredTestTemplateIdentity) -> TestTemplateIdentityDto {
    TestTemplateIdentityDto {
        template_id: identity.template_id.clone(),
        title: identity.title.clone(),
        category_code: identity.category_code.clone(),
        current_approved_revision_id: identity.current_approved_revision_id.clone(),
        created_by: identity.created_by.clone(),
        created_at: identity.created_at.clone(),
        updated_at: identity.updated_at.clone(),
    }
}

fn revision_dto(revision: &StoredTestTemplateRevision) -> TestTemplateRevisionDto {
    TestTemplateRevisionDto {
        revision_id: revision.revision_id.clone(),
        template_id: revision.template_id.clone(),
        revision_number: revision.revision_number,
        parent_revision_id: revision.parent_revision_id.clone(),
        status: revision.status.clone(),
        definition_schema_version: revision.definition_schema_version.clone(),
        definition: parse_json_value(&revision.definition_json),
        definition_checksum: revision.definition_checksum.clone(),
        created_by: revision.created_by.clone(),
        created_at: revision.created_at.clone(),
        updated_at: revision.updated_at.clone(),
        submitted_at: revision.submitted_at.clone(),
        approved_at: revision.approved_at.clone(),
    }
}

fn create_payload_json(
    input: &CreateTestTemplateInput,
    definition: &CanonicalTestTemplateDefinition,
) -> String {
    render_json(&json!({
        "template_id": input.template_id,
        "title": input.title.trim(),
        "category_code": input.category_code,
        "definition_schema_version": definition.definition_schema_version,
        "definition_checksum": definition.definition_checksum,
        "definition": definition_value(definition).expect("canonical definition is valid JSON"),
    }))
}

fn replace_definition_payload_json(
    input: &ReplaceTestTemplateDefinitionInput,
    definition: &CanonicalTestTemplateDefinition,
) -> String {
    render_json(&json!({
        "template_id": input.template_id,
        "revision_id": input.revision_id,
        "expected_definition_checksum": input.expected_definition_checksum,
        "new_definition_checksum": definition.definition_checksum,
        "definition": definition_value(definition).expect("canonical definition is valid JSON"),
    }))
}

fn create_revision_payload_json(input: &CreateTestTemplateRevisionInput) -> String {
    render_json(&json!({
        "template_id": input.template_id,
        "source_revision_id": input.source_revision_id,
        "reason": input.reason.trim(),
    }))
}

fn clone_payload_json(
    input: &CloneTestTemplateInput,
    category_code: &str,
    source_revision: &StoredTestTemplateRevision,
    definition: &CanonicalTestTemplateDefinition,
) -> String {
    render_json(&json!({
        "source_template_id": input.source_template_id,
        "source_revision_id": source_revision.revision_id,
        "new_template_id": input.new_template_id,
        "title": input.title.trim(),
        "category_code": category_code,
        "definition_schema_version": definition.definition_schema_version,
        "definition_checksum": definition.definition_checksum,
        "source_definition_checksum": source_revision.definition_checksum,
        "reason": input.reason.trim(),
    }))
}

fn transition_payload_json(input: &TransitionTestTemplateRevisionInput) -> String {
    render_json(&json!({
        "template_id": input.template_id,
        "revision_id": input.revision_id,
        "target_status": revision_status_text(&input.target_status),
        "reason": input.reason.trim(),
    }))
}

fn supersede_payload_json(
    input: &TransitionTestTemplateRevisionInput,
    superseded_revision: &StoredTestTemplateRevision,
) -> String {
    render_json(&json!({
        "template_id": input.template_id,
        "superseded_revision_id": superseded_revision.revision_id,
        "approved_revision_id": input.revision_id,
        "reason": input.reason.trim(),
    }))
}

fn definition_value(definition: &CanonicalTestTemplateDefinition) -> Result<Value, AgentError> {
    serde_json::from_str(&definition.canonical_json).map_err(|error| {
        AgentError::new("test_template_definition_decode_failed", error.to_string())
    })
}

fn parse_json_value(value: &str) -> Value {
    serde_json::from_str(value).expect("persisted test-template JSON must be valid")
}

fn revision_id_for(template_id: &str, revision_number: u32) -> String {
    format!("{template_id}-rev-{revision_number:04}")
}

fn revision_status_text(status: &TemplateRevisionStatus) -> &'static str {
    match status {
        TemplateRevisionStatus::Draft => "draft",
        TemplateRevisionStatus::UnderReview => "under_review",
        TemplateRevisionStatus::Approved => "approved",
        TemplateRevisionStatus::Suspended => "suspended",
        TemplateRevisionStatus::Superseded => "superseded",
        TemplateRevisionStatus::Retired => "retired",
    }
}

fn parse_revision_status(value: &str) -> Result<TemplateRevisionStatus, AgentError> {
    match value {
        "draft" => Ok(TemplateRevisionStatus::Draft),
        "under_review" => Ok(TemplateRevisionStatus::UnderReview),
        "approved" => Ok(TemplateRevisionStatus::Approved),
        "suspended" => Ok(TemplateRevisionStatus::Suspended),
        "superseded" => Ok(TemplateRevisionStatus::Superseded),
        "retired" => Ok(TemplateRevisionStatus::Retired),
        other => Err(AgentError::with_details(
            "invalid_test_template_revision_status",
            "stored template revision status is unsupported",
            json!({ "status": other }),
        )),
    }
}

fn transition_operation_kind(target_status: &TemplateRevisionStatus) -> &'static str {
    match target_status {
        TemplateRevisionStatus::UnderReview => "test_template_submitted_for_review",
        TemplateRevisionStatus::Approved => "test_template_approved",
        _ => "test_template_transitioned",
    }
}

fn is_allowed_revision_transition(
    current: &TemplateRevisionStatus,
    target: &TemplateRevisionStatus,
) -> bool {
    matches!(
        (current, target),
        (
            TemplateRevisionStatus::Draft,
            TemplateRevisionStatus::UnderReview
        ) | (
            TemplateRevisionStatus::UnderReview,
            TemplateRevisionStatus::Approved
        )
    )
}

fn status_cursor(status: &str, revision_id: &str) -> String {
    format!("{status}:{revision_id}")
}

fn definition_cursor(status: &str, revision_id: &str, checksum: &str) -> String {
    format!("{status}:{revision_id}:{checksum}")
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_error", error.to_string()))
}
