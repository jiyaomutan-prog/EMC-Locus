use crate::{
    render_json,
    test_template_repository::{
        insert_test_template_sync_operation, open_test_template_connection,
        open_test_template_connection_with_sync, TestTemplateSyncOperationInput,
    },
    AgentError,
};
use emc_locus_core::{
    compile_execution_plan, preview_sub_range, validate_method_hierarchy,
    CanonicalMethodDefinition, ExecutionConfigurationDefinition,
    MeasurementSystemTemplateDefinition, MethodHierarchyNode, RegulationProfileDefinition,
    SubRangeDefinition, VersionedMethodDefinition,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowAggregateKind {
    MeasurementSystemTemplate,
    RegulationProfile,
}

impl WorkflowAggregateKind {
    pub fn from_collection(value: &str) -> Option<Self> {
        match value {
            "measurement-system-templates" => Some(Self::MeasurementSystemTemplate),
            "regulation-profiles" => Some(Self::RegulationProfile),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::MeasurementSystemTemplate => "measurement_system_template",
            Self::RegulationProfile => "regulation_profile",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowOperationContext {
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkflowValidationContext {
    pub method_definition_json: Option<String>,
    pub regulation_profiles_json: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateHierarchyNodeInput {
    pub node: MethodHierarchyNode,
    pub context: WorkflowOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateHierarchyNodeInput {
    pub node: MethodHierarchyNode,
    pub expected_revision: u32,
    pub context: WorkflowOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateWorkflowDefinitionInput {
    pub kind: WorkflowAggregateKind,
    pub entity_id: String,
    pub label: String,
    pub classification: String,
    pub definition_json: String,
    pub validation: WorkflowValidationContext,
    pub context: WorkflowOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplaceWorkflowDefinitionInput {
    pub kind: WorkflowAggregateKind,
    pub entity_id: String,
    pub revision_id: String,
    pub expected_definition_checksum: String,
    pub definition_json: String,
    pub validation: WorkflowValidationContext,
    pub context: WorkflowOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateWorkflowRevisionInput {
    pub kind: WorkflowAggregateKind,
    pub entity_id: String,
    pub source_revision_id: String,
    pub context: WorkflowOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionWorkflowRevisionInput {
    pub kind: WorkflowAggregateKind,
    pub entity_id: String,
    pub revision_id: String,
    pub target_status: String,
    pub validation: WorkflowValidationContext,
    pub context: WorkflowOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileExecutionPlanInput {
    pub method_template_id: String,
    pub method_revision_id: String,
    pub method_definition_json: String,
    pub system_definition_json: String,
    pub regulation_profiles_json: Vec<String>,
    pub execution_configuration_json: Option<String>,
    pub context: WorkflowOperationContext,
}

pub fn preview_method_sub_range(
    definition_json: &str,
    maximum_points: u64,
) -> Result<String, AgentError> {
    let definition: SubRangeDefinition = serde_json::from_str(definition_json)
        .map_err(|error| AgentError::new("invalid_sub_range_json", error.to_string()))?;
    let preview =
        preview_sub_range(&definition, maximum_points).map_err(workflow_validation_error)?;
    Ok(render_json(&json!({ "preview": preview })))
}

pub fn compile_method_execution_plan(
    storage_root: &Path,
    input: CompileExecutionPlanInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let method = match VersionedMethodDefinition::from_json_str(&input.method_definition_json)
        .map_err(workflow_validation_error)?
    {
        VersionedMethodDefinition::V2(method) => method,
        VersionedMethodDefinition::V1(_) => {
            return Err(AgentError::new(
                "execution_plan_method_v2_required",
                "create a 0.22.2 successor draft before compiling an execution plan",
            ));
        }
    };
    let system: MeasurementSystemTemplateDefinition =
        serde_json::from_str(&input.system_definition_json).map_err(|error| {
            AgentError::new(
                "invalid_measurement_system_template_json",
                error.to_string(),
            )
        })?;
    let profiles = input
        .regulation_profiles_json
        .iter()
        .map(|value| {
            serde_json::from_str::<RegulationProfileDefinition>(value).map_err(|error| {
                AgentError::new("invalid_regulation_profile_json", error.to_string())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let configuration = input
        .execution_configuration_json
        .as_deref()
        .map(serde_json::from_str::<ExecutionConfigurationDefinition>)
        .transpose()
        .map_err(|error| {
            AgentError::new("invalid_execution_configuration_json", error.to_string())
        })?;
    let preview = compile_execution_plan(&method, &system, &profiles, configuration.as_ref());
    let payload = render_json(&json!({
        "method_template_id": input.method_template_id,
        "method_revision_id": input.method_revision_id,
        "method_checksum": method.canonicalize().map_err(workflow_validation_error)?.definition_checksum,
        "system_checksum": system.canonicalize(&method.functional_roles, &profiles).map_err(workflow_validation_error)?.definition_checksum,
        "has_execution_configuration": configuration.is_some(),
    }));
    let request_checksum = operation_checksum(&payload, &input.context);
    let mut connection = open_test_template_connection_with_sync(storage_root)?;
    if let Some(response) =
        operation_replay(&connection, &input.context.operation_id, &request_checksum)?
    {
        return Ok(response);
    }
    let now = utc_timestamp()?;
    let response = render_json(&json!({
        "operation": "execution_preview_compiled",
        "replayed": false,
        "preview": preview,
    }));
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sql_error("execution_preview_transaction_failed"))?;
    record_mutation(
        &transaction,
        "execution_preview",
        &input.method_template_id,
        Some(&input.method_revision_id),
        "execution_preview_compiled",
        None,
        None,
        &payload,
        &request_checksum,
        &response,
        &input.context,
        &now,
    )?;
    transaction
        .commit()
        .map_err(sql_error("execution_preview_commit_failed"))?;
    Ok(response)
}

pub fn list_method_hierarchy(storage_root: &Path) -> Result<String, AgentError> {
    let connection = open_test_template_connection(storage_root)?;
    let mut statement = connection
        .prepare(
            "SELECT node_id, parent_node_id, node_kind, label, position, archived, revision FROM method_hierarchy_nodes ORDER BY parent_node_id, position, label",
        )
        .map_err(sql_error("method_hierarchy_query_failed"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(json!({
                "node_id": row.get::<_, String>(0)?,
                "parent_node_id": row.get::<_, Option<String>>(1)?,
                "node_kind": row.get::<_, String>(2)?,
                "label": row.get::<_, String>(3)?,
                "position": row.get::<_, u32>(4)?,
                "archived": row.get::<_, i64>(5)? != 0,
                "revision": row.get::<_, u32>(6)?,
            }))
        })
        .map_err(sql_error("method_hierarchy_query_failed"))?;
    let nodes = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("method_hierarchy_query_failed"))?;
    Ok(render_json(&json!({ "nodes": nodes })))
}

pub fn create_method_hierarchy_node(
    storage_root: &Path,
    input: CreateHierarchyNodeInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let payload = render_json(&json!({ "node": input.node }));
    let request_checksum = operation_checksum(&payload, &input.context);
    let mut connection = open_test_template_connection_with_sync(storage_root)?;
    if let Some(response) =
        operation_replay(&connection, &input.context.operation_id, &request_checksum)?
    {
        return Ok(response);
    }
    let mut nodes = load_hierarchy(&connection)?;
    nodes.push(input.node.clone());
    reject_hierarchy_issues(&nodes)?;
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sql_error("method_hierarchy_transaction_failed"))?;
    transaction
        .execute(
            "INSERT INTO method_hierarchy_nodes (node_id, parent_node_id, node_kind, label, position, archived, revision, created_by, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, ?8)",
            params![
                input.node.node_id,
                input.node.parent_node_id,
                hierarchy_kind(&input.node),
                input.node.label.trim(),
                input.node.position,
                i64::from(input.node.archived),
                input.context.actor,
                now,
            ],
        )
        .map_err(sql_error("method_hierarchy_write_failed"))?;
    let response = render_json(
        &json!({ "operation": "hierarchy_node_created", "replayed": false, "node": input.node, "revision": 1 }),
    );
    record_mutation(
        &transaction,
        "method_hierarchy",
        &input.node.node_id,
        None,
        "hierarchy_node_created",
        None,
        None,
        &payload,
        &request_checksum,
        &response,
        &input.context,
        &now,
    )?;
    transaction
        .commit()
        .map_err(sql_error("method_hierarchy_commit_failed"))?;
    Ok(response)
}

pub fn update_method_hierarchy_node(
    storage_root: &Path,
    input: UpdateHierarchyNodeInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let payload =
        render_json(&json!({ "node": input.node, "expected_revision": input.expected_revision }));
    let request_checksum = operation_checksum(&payload, &input.context);
    let mut connection = open_test_template_connection_with_sync(storage_root)?;
    if let Some(response) =
        operation_replay(&connection, &input.context.operation_id, &request_checksum)?
    {
        return Ok(response);
    }
    let previous = load_hierarchy_node(&connection, &input.node.node_id)?.ok_or_else(|| {
        AgentError::new(
            "method_hierarchy_node_not_found",
            "hierarchy node does not exist",
        )
    })?;
    let mut nodes = load_hierarchy(&connection)?;
    if let Some(node) = nodes
        .iter_mut()
        .find(|node| node.node_id == input.node.node_id)
    {
        *node = input.node.clone();
    }
    reject_hierarchy_issues(&nodes)?;
    let action = if previous.archived != input.node.archived {
        if input.node.archived {
            "hierarchy_node_archived"
        } else {
            "hierarchy_node_restored"
        }
    } else if previous.parent_node_id != input.node.parent_node_id {
        "hierarchy_node_moved"
    } else {
        "hierarchy_node_changed"
    };
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sql_error("method_hierarchy_transaction_failed"))?;
    let changed = transaction
        .execute(
            "UPDATE method_hierarchy_nodes SET parent_node_id = ?2, node_kind = ?3, label = ?4, position = ?5, archived = ?6, revision = revision + 1, updated_at = ?7 WHERE node_id = ?1 AND revision = ?8",
            params![
                input.node.node_id,
                input.node.parent_node_id,
                hierarchy_kind(&input.node),
                input.node.label.trim(),
                input.node.position,
                i64::from(input.node.archived),
                now,
                input.expected_revision,
            ],
        )
        .map_err(sql_error("method_hierarchy_write_failed"))?;
    if changed != 1 {
        return Err(AgentError::with_details(
            "method_hierarchy_revision_conflict",
            "the hierarchy node changed since it was loaded",
            json!({ "node_id": input.node.node_id, "expected_revision": input.expected_revision }),
        ));
    }
    let revision = input.expected_revision + 1;
    let response = render_json(
        &json!({ "operation": action, "replayed": false, "node": input.node, "revision": revision }),
    );
    record_mutation(
        &transaction,
        "method_hierarchy",
        &input.node.node_id,
        None,
        action,
        None,
        None,
        &payload,
        &request_checksum,
        &response,
        &input.context,
        &now,
    )?;
    transaction
        .commit()
        .map_err(sql_error("method_hierarchy_commit_failed"))?;
    Ok(response)
}

pub fn list_workflow_definitions(
    storage_root: &Path,
    kind: WorkflowAggregateKind,
) -> Result<String, AgentError> {
    let connection = open_test_template_connection(storage_root)?;
    let mut statement = connection
        .prepare("SELECT entity_id FROM method_workflow_identities WHERE aggregate_kind = ?1 ORDER BY label, entity_id")
        .map_err(sql_error("method_workflow_query_failed"))?;
    let rows = statement
        .query_map(params![kind.as_str()], |row| row.get::<_, String>(0))
        .map_err(sql_error("method_workflow_query_failed"))?;
    let mut definitions = Vec::new();
    for row in rows {
        definitions.push(workflow_aggregate(
            &connection,
            kind,
            &row.map_err(sql_error("method_workflow_query_failed"))?,
        )?);
    }
    Ok(render_json(&json!({ "definitions": definitions })))
}

pub fn get_workflow_definition(
    storage_root: &Path,
    kind: WorkflowAggregateKind,
    entity_id: &str,
) -> Result<String, AgentError> {
    let connection = open_test_template_connection(storage_root)?;
    Ok(render_json(&json!({
        "definition": workflow_aggregate(&connection, kind, entity_id)?
    })))
}

pub fn get_workflow_revision(
    storage_root: &Path,
    kind: WorkflowAggregateKind,
    entity_id: &str,
    revision_id: &str,
) -> Result<String, AgentError> {
    let connection = open_test_template_connection(storage_root)?;
    let revision =
        workflow_revision(&connection, kind, entity_id, revision_id)?.ok_or_else(|| {
            AgentError::new(
                "method_workflow_revision_not_found",
                "workflow revision does not exist",
            )
        })?;
    Ok(render_json(&json!({ "revision": revision })))
}

pub fn create_workflow_definition(
    storage_root: &Path,
    input: CreateWorkflowDefinitionInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    validate_id_and_label(&input.entity_id, &input.label, &input.classification)?;
    let canonical = canonical_workflow_definition(
        input.kind,
        &input.entity_id,
        &input.definition_json,
        &input.validation,
    )?;
    let revision_id = workflow_revision_id(&input.entity_id, 1);
    let payload = render_json(&json!({
        "entity_id": input.entity_id,
        "label": input.label.trim(),
        "classification": input.classification.trim(),
        "definition_checksum": canonical.definition_checksum,
    }));
    let request_checksum = operation_checksum(&payload, &input.context);
    let mut connection = open_test_template_connection_with_sync(storage_root)?;
    if let Some(response) =
        operation_replay(&connection, &input.context.operation_id, &request_checksum)?
    {
        return Ok(response);
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sql_error("method_workflow_transaction_failed"))?;
    transaction.execute(
        "INSERT INTO method_workflow_identities (aggregate_kind, entity_id, label, classification, current_approved_revision_id, created_by, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?6)",
        params![input.kind.as_str(), input.entity_id, input.label.trim(), input.classification.trim(), input.context.actor, now],
    ).map_err(sql_error("method_workflow_write_failed"))?;
    insert_workflow_revision(
        &transaction,
        input.kind,
        &input.entity_id,
        &revision_id,
        1,
        None,
        "draft",
        &canonical,
        &input.context.actor,
        &now,
    )?;
    let revision = revision_value(
        &revision_id,
        &input.entity_id,
        1,
        None,
        "draft",
        &canonical,
        &input.context.actor,
        &now,
        None,
        None,
    )?;
    let response = render_json(
        &json!({ "operation": "workflow_definition_created", "replayed": false, "revision": revision }),
    );
    record_mutation(
        &transaction,
        input.kind.as_str(),
        &input.entity_id,
        Some(&revision_id),
        "workflow_definition_created",
        None,
        Some(&canonical.definition_checksum),
        &payload,
        &request_checksum,
        &response,
        &input.context,
        &now,
    )?;
    transaction
        .commit()
        .map_err(sql_error("method_workflow_commit_failed"))?;
    Ok(response)
}

pub fn replace_workflow_definition(
    storage_root: &Path,
    input: ReplaceWorkflowDefinitionInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let canonical = canonical_workflow_definition(
        input.kind,
        &input.entity_id,
        &input.definition_json,
        &input.validation,
    )?;
    let payload = render_json(&json!({
        "entity_id": input.entity_id,
        "revision_id": input.revision_id,
        "expected_definition_checksum": input.expected_definition_checksum,
        "new_definition_checksum": canonical.definition_checksum,
    }));
    let request_checksum = operation_checksum(&payload, &input.context);
    let mut connection = open_test_template_connection_with_sync(storage_root)?;
    if let Some(response) =
        operation_replay(&connection, &input.context.operation_id, &request_checksum)?
    {
        return Ok(response);
    }
    let existing = workflow_revision(
        &connection,
        input.kind,
        &input.entity_id,
        &input.revision_id,
    )?
    .ok_or_else(|| {
        AgentError::new(
            "method_workflow_revision_not_found",
            "workflow revision does not exist",
        )
    })?;
    if existing["status"] != "draft" {
        return Err(AgentError::new(
            "method_workflow_revision_immutable",
            "only a draft workflow revision can be changed",
        ));
    }
    let old_checksum = existing["definition_checksum"].as_str().unwrap_or_default();
    if old_checksum != input.expected_definition_checksum {
        return Err(AgentError::with_details(
            "method_workflow_definition_conflict",
            "the workflow definition changed since it was loaded",
            json!({ "expected_definition_checksum": input.expected_definition_checksum, "actual_definition_checksum": old_checksum }),
        ));
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sql_error("method_workflow_transaction_failed"))?;
    let changed = transaction.execute(
        "UPDATE method_workflow_revisions SET definition_schema_version = ?4, definition_json = ?5, definition_checksum = ?6, updated_at = ?7 WHERE aggregate_kind = ?1 AND entity_id = ?2 AND revision_id = ?3 AND status = 'draft' AND definition_checksum = ?8",
        params![input.kind.as_str(), input.entity_id, input.revision_id, canonical.definition_schema_version, canonical.canonical_json, canonical.definition_checksum, now, input.expected_definition_checksum],
    ).map_err(sql_error("method_workflow_write_failed"))?;
    if changed != 1 {
        return Err(AgentError::new(
            "method_workflow_definition_conflict",
            "the workflow draft changed concurrently",
        ));
    }
    let response = render_json(
        &json!({ "operation": "workflow_definition_changed", "replayed": false, "revision_id": input.revision_id, "definition_checksum": canonical.definition_checksum }),
    );
    record_mutation(
        &transaction,
        input.kind.as_str(),
        &input.entity_id,
        Some(&input.revision_id),
        "workflow_definition_changed",
        Some(old_checksum),
        Some(&canonical.definition_checksum),
        &payload,
        &request_checksum,
        &response,
        &input.context,
        &now,
    )?;
    transaction
        .commit()
        .map_err(sql_error("method_workflow_commit_failed"))?;
    Ok(response)
}

pub fn create_workflow_revision(
    storage_root: &Path,
    input: CreateWorkflowRevisionInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    let payload = render_json(
        &json!({ "entity_id": input.entity_id, "source_revision_id": input.source_revision_id }),
    );
    let request_checksum = operation_checksum(&payload, &input.context);
    let mut connection = open_test_template_connection_with_sync(storage_root)?;
    if let Some(response) =
        operation_replay(&connection, &input.context.operation_id, &request_checksum)?
    {
        return Ok(response);
    }
    let source = workflow_revision(
        &connection,
        input.kind,
        &input.entity_id,
        &input.source_revision_id,
    )?
    .ok_or_else(|| {
        AgentError::new(
            "method_workflow_revision_not_found",
            "source workflow revision does not exist",
        )
    })?;
    if source["status"] != "approved" {
        return Err(AgentError::new(
            "method_workflow_source_not_approved",
            "new workflow revisions derive from an approved revision",
        ));
    }
    let active_draft: Option<String> = connection.query_row(
        "SELECT revision_id FROM method_workflow_revisions WHERE aggregate_kind = ?1 AND entity_id = ?2 AND status = 'draft'",
        params![input.kind.as_str(), input.entity_id], |row| row.get(0),
    ).optional().map_err(sql_error("method_workflow_query_failed"))?;
    if active_draft.is_some() {
        return Err(AgentError::new(
            "method_workflow_active_draft_exists",
            "an active workflow draft already exists",
        ));
    }
    let revision_number: u32 = connection.query_row(
        "SELECT COALESCE(MAX(revision_number), 0) + 1 FROM method_workflow_revisions WHERE aggregate_kind = ?1 AND entity_id = ?2",
        params![input.kind.as_str(), input.entity_id], |row| row.get(0),
    ).map_err(sql_error("method_workflow_query_failed"))?;
    let revision_id = workflow_revision_id(&input.entity_id, revision_number);
    let canonical = CanonicalMethodDefinition {
        definition_schema_version: source["definition_schema_version"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        canonical_json: render_json(&source["definition"]),
        definition_checksum: source["definition_checksum"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
    };
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sql_error("method_workflow_transaction_failed"))?;
    insert_workflow_revision(
        &transaction,
        input.kind,
        &input.entity_id,
        &revision_id,
        revision_number,
        Some(&input.source_revision_id),
        "draft",
        &canonical,
        &input.context.actor,
        &now,
    )?;
    let response = render_json(
        &json!({ "operation": "workflow_revision_created", "replayed": false, "revision_id": revision_id, "definition_checksum": canonical.definition_checksum }),
    );
    record_mutation(
        &transaction,
        input.kind.as_str(),
        &input.entity_id,
        Some(&revision_id),
        "workflow_revision_created",
        Some(&canonical.definition_checksum),
        Some(&canonical.definition_checksum),
        &payload,
        &request_checksum,
        &response,
        &input.context,
        &now,
    )?;
    transaction
        .commit()
        .map_err(sql_error("method_workflow_commit_failed"))?;
    Ok(response)
}

pub fn transition_workflow_revision(
    storage_root: &Path,
    input: TransitionWorkflowRevisionInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    if !matches!(input.target_status.as_str(), "validated" | "approved") {
        return Err(AgentError::new(
            "invalid_method_workflow_transition",
            "supported transitions are validated and approved",
        ));
    }
    let payload = render_json(
        &json!({ "entity_id": input.entity_id, "revision_id": input.revision_id, "target_status": input.target_status }),
    );
    let request_checksum = operation_checksum(&payload, &input.context);
    let mut connection = open_test_template_connection_with_sync(storage_root)?;
    if let Some(response) =
        operation_replay(&connection, &input.context.operation_id, &request_checksum)?
    {
        return Ok(response);
    }
    let existing = workflow_revision(
        &connection,
        input.kind,
        &input.entity_id,
        &input.revision_id,
    )?
    .ok_or_else(|| {
        AgentError::new(
            "method_workflow_revision_not_found",
            "workflow revision does not exist",
        )
    })?;
    let current = existing["status"].as_str().unwrap_or_default();
    let expected = if input.target_status == "validated" {
        "draft"
    } else {
        "validated"
    };
    if current != expected {
        return Err(AgentError::with_details(
            "invalid_method_workflow_transition",
            "workflow transition is not allowed",
            json!({ "current_status": current, "target_status": input.target_status }),
        ));
    }
    let stored_json = render_json(&existing["definition"]);
    let canonical = canonical_workflow_definition(
        input.kind,
        &input.entity_id,
        &stored_json,
        &input.validation,
    )?;
    if canonical.definition_checksum != existing["definition_checksum"] {
        return Err(AgentError::new(
            "method_workflow_checksum_mismatch",
            "stored workflow definition checksum is inconsistent",
        ));
    }
    let now = utc_timestamp()?;
    let action = if input.target_status == "validated" {
        "workflow_revision_validated"
    } else {
        "workflow_revision_approved"
    };
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sql_error("method_workflow_transaction_failed"))?;
    let changed = transaction.execute(
        "UPDATE method_workflow_revisions SET status = ?4, updated_at = ?5, validated_at = CASE WHEN ?4 = 'validated' THEN ?5 ELSE validated_at END, approved_at = CASE WHEN ?4 = 'approved' THEN ?5 ELSE approved_at END WHERE aggregate_kind = ?1 AND entity_id = ?2 AND revision_id = ?3 AND status = ?6",
        params![input.kind.as_str(), input.entity_id, input.revision_id, input.target_status, now, expected],
    ).map_err(sql_error("method_workflow_write_failed"))?;
    if changed != 1 {
        return Err(AgentError::new(
            "method_workflow_transition_conflict",
            "workflow revision changed concurrently",
        ));
    }
    if input.target_status == "approved" {
        transaction.execute(
            "UPDATE method_workflow_revisions SET status = 'superseded', updated_at = ?4 WHERE aggregate_kind = ?1 AND entity_id = ?2 AND revision_id <> ?3 AND status = 'approved'",
            params![input.kind.as_str(), input.entity_id, input.revision_id, now],
        ).map_err(sql_error("method_workflow_write_failed"))?;
        transaction.execute(
            "UPDATE method_workflow_identities SET current_approved_revision_id = ?3, updated_at = ?4 WHERE aggregate_kind = ?1 AND entity_id = ?2",
            params![input.kind.as_str(), input.entity_id, input.revision_id, now],
        ).map_err(sql_error("method_workflow_write_failed"))?;
    }
    let response = render_json(
        &json!({ "operation": action, "replayed": false, "revision_id": input.revision_id, "status": input.target_status, "definition_checksum": canonical.definition_checksum }),
    );
    record_mutation(
        &transaction,
        input.kind.as_str(),
        &input.entity_id,
        Some(&input.revision_id),
        action,
        Some(&canonical.definition_checksum),
        Some(&canonical.definition_checksum),
        &payload,
        &request_checksum,
        &response,
        &input.context,
        &now,
    )?;
    transaction
        .commit()
        .map_err(sql_error("method_workflow_commit_failed"))?;
    Ok(response)
}

pub fn list_workflow_audit_events(
    storage_root: &Path,
    kind: WorkflowAggregateKind,
    entity_id: &str,
) -> Result<String, AgentError> {
    let connection = open_test_template_connection(storage_root)?;
    let mut statement = connection.prepare(
        "SELECT audit_id, revision_id, action, actor, reason, old_definition_checksum, new_definition_checksum, operation_id, device_id, correlation_id, payload_json, occurred_at FROM method_workflow_audit_events WHERE aggregate_kind = ?1 AND entity_id = ?2 ORDER BY audit_id"
    ).map_err(sql_error("method_workflow_audit_query_failed"))?;
    let rows = statement.query_map(params![kind.as_str(), entity_id], |row| {
        let payload_json: String = row.get(10)?;
        Ok(json!({
            "audit_id": row.get::<_, u64>(0)?, "revision_id": row.get::<_, Option<String>>(1)?,
            "action": row.get::<_, String>(2)?, "actor": row.get::<_, String>(3)?,
            "reason": row.get::<_, String>(4)?, "old_definition_checksum": row.get::<_, Option<String>>(5)?,
            "new_definition_checksum": row.get::<_, Option<String>>(6)?, "operation_id": row.get::<_, String>(7)?,
            "device_id": row.get::<_, String>(8)?, "correlation_id": row.get::<_, String>(9)?,
            "payload": serde_json::from_str::<Value>(&payload_json).unwrap_or(Value::Null), "occurred_at": row.get::<_, String>(11)?,
        }))
    }).map_err(sql_error("method_workflow_audit_query_failed"))?;
    let events = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("method_workflow_audit_query_failed"))?;
    Ok(render_json(&json!({ "audit_events": events })))
}

fn canonical_workflow_definition(
    kind: WorkflowAggregateKind,
    entity_id: &str,
    definition_json: &str,
    context: &WorkflowValidationContext,
) -> Result<CanonicalMethodDefinition, AgentError> {
    let method = context
        .method_definition_json
        .as_deref()
        .map(VersionedMethodDefinition::from_json_str)
        .transpose()
        .map_err(workflow_validation_error)?;
    let method = match method {
        Some(VersionedMethodDefinition::V2(method)) => Some(method),
        Some(VersionedMethodDefinition::V1(_)) => {
            return Err(AgentError::new(
                "method_workflow_v2_required",
                "workflow validation requires a method v2 definition",
            ));
        }
        None => None,
    };
    let profiles = context
        .regulation_profiles_json
        .iter()
        .map(|value| {
            serde_json::from_str::<RegulationProfileDefinition>(value).map_err(|error| {
                AgentError::new("invalid_regulation_profile_json", error.to_string())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    match kind {
        WorkflowAggregateKind::MeasurementSystemTemplate => {
            let definition: MeasurementSystemTemplateDefinition =
                serde_json::from_str(definition_json).map_err(|error| {
                    AgentError::new(
                        "invalid_measurement_system_template_json",
                        error.to_string(),
                    )
                })?;
            if definition.template_id != entity_id {
                return Err(AgentError::new(
                    "measurement_system_template_id_mismatch",
                    "definition template_id must match the aggregate identity",
                ));
            }
            definition
                .canonicalize(
                    method
                        .as_ref()
                        .map_or(&[], |method| method.functional_roles.as_slice()),
                    &profiles,
                )
                .map_err(workflow_validation_error)
        }
        WorkflowAggregateKind::RegulationProfile => {
            let definition: RegulationProfileDefinition = serde_json::from_str(definition_json)
                .map_err(|error| {
                    AgentError::new("invalid_regulation_profile_json", error.to_string())
                })?;
            if definition.profile_id != entity_id {
                return Err(AgentError::new(
                    "regulation_profile_id_mismatch",
                    "definition profile_id must match the aggregate identity",
                ));
            }
            let method = method.ok_or_else(|| {
                AgentError::new(
                    "method_context_required",
                    "regulation profile validation requires a method v2 definition",
                )
            })?;
            definition
                .canonicalize(&method.variables, &method.functional_roles)
                .map_err(workflow_validation_error)
        }
    }
}

fn workflow_aggregate(
    connection: &Connection,
    kind: WorkflowAggregateKind,
    entity_id: &str,
) -> Result<Value, AgentError> {
    let identity = connection.query_row(
        "SELECT label, classification, current_approved_revision_id, created_by, created_at, updated_at FROM method_workflow_identities WHERE aggregate_kind = ?1 AND entity_id = ?2",
        params![kind.as_str(), entity_id], |row| Ok(json!({
            "aggregate_kind": kind.as_str(), "entity_id": entity_id,
            "label": row.get::<_, String>(0)?, "classification": row.get::<_, String>(1)?,
            "current_approved_revision_id": row.get::<_, Option<String>>(2)?, "created_by": row.get::<_, String>(3)?,
            "created_at": row.get::<_, String>(4)?, "updated_at": row.get::<_, String>(5)?,
        })),
    ).optional().map_err(sql_error("method_workflow_query_failed"))?
        .ok_or_else(|| AgentError::new("method_workflow_definition_not_found", "workflow definition does not exist"))?;
    let mut statement = connection.prepare(
        "SELECT revision_id FROM method_workflow_revisions WHERE aggregate_kind = ?1 AND entity_id = ?2 ORDER BY revision_number DESC"
    ).map_err(sql_error("method_workflow_query_failed"))?;
    let rows = statement
        .query_map(params![kind.as_str(), entity_id], |row| {
            row.get::<_, String>(0)
        })
        .map_err(sql_error("method_workflow_query_failed"))?;
    let mut revisions = Vec::new();
    for row in rows {
        let revision_id = row.map_err(sql_error("method_workflow_query_failed"))?;
        if let Some(revision) = workflow_revision(connection, kind, entity_id, &revision_id)? {
            revisions.push(revision);
        }
    }
    Ok(json!({ "identity": identity, "revisions": revisions }))
}

fn workflow_revision(
    connection: &Connection,
    kind: WorkflowAggregateKind,
    entity_id: &str,
    revision_id: &str,
) -> Result<Option<Value>, AgentError> {
    connection.query_row(
        "SELECT revision_number, parent_revision_id, status, definition_schema_version, definition_json, definition_checksum, created_by, created_at, updated_at, validated_at, approved_at FROM method_workflow_revisions WHERE aggregate_kind = ?1 AND entity_id = ?2 AND revision_id = ?3",
        params![kind.as_str(), entity_id, revision_id], |row| {
            let definition_json: String = row.get(4)?;
            Ok(json!({
                "revision_id": revision_id, "entity_id": entity_id, "revision_number": row.get::<_, u32>(0)?,
                "parent_revision_id": row.get::<_, Option<String>>(1)?, "status": row.get::<_, String>(2)?,
                "definition_schema_version": row.get::<_, String>(3)?,
                "definition": serde_json::from_str::<Value>(&definition_json).unwrap_or(Value::Null),
                "definition_checksum": row.get::<_, String>(5)?, "created_by": row.get::<_, String>(6)?,
                "created_at": row.get::<_, String>(7)?, "updated_at": row.get::<_, String>(8)?,
                "validated_at": row.get::<_, Option<String>>(9)?, "approved_at": row.get::<_, Option<String>>(10)?,
            }))
        },
    ).optional().map_err(sql_error("method_workflow_query_failed"))
}

#[allow(clippy::too_many_arguments)]
fn insert_workflow_revision(
    transaction: &Transaction<'_>,
    kind: WorkflowAggregateKind,
    entity_id: &str,
    revision_id: &str,
    revision_number: u32,
    parent_revision_id: Option<&str>,
    status: &str,
    definition: &CanonicalMethodDefinition,
    actor: &str,
    now: &str,
) -> Result<(), AgentError> {
    transaction.execute(
        "INSERT INTO method_workflow_revisions (revision_id, aggregate_kind, entity_id, revision_number, parent_revision_id, status, definition_schema_version, definition_json, definition_checksum, created_by, created_at, updated_at, validated_at, approved_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, NULL, NULL)",
        params![revision_id, kind.as_str(), entity_id, revision_number, parent_revision_id, status, definition.definition_schema_version, definition.canonical_json, definition.definition_checksum, actor, now],
    ).map_err(sql_error("method_workflow_write_failed"))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn revision_value(
    revision_id: &str,
    entity_id: &str,
    revision_number: u32,
    parent_revision_id: Option<&str>,
    status: &str,
    definition: &CanonicalMethodDefinition,
    actor: &str,
    now: &str,
    validated_at: Option<&str>,
    approved_at: Option<&str>,
) -> Result<Value, AgentError> {
    let definition_value: Value =
        serde_json::from_str(&definition.canonical_json).map_err(|error| {
            AgentError::new(
                "method_workflow_definition_decode_failed",
                error.to_string(),
            )
        })?;
    Ok(json!({
        "revision_id": revision_id, "entity_id": entity_id, "revision_number": revision_number,
        "parent_revision_id": parent_revision_id, "status": status,
        "definition_schema_version": definition.definition_schema_version,
        "definition": definition_value, "definition_checksum": definition.definition_checksum,
        "created_by": actor, "created_at": now, "updated_at": now,
        "validated_at": validated_at, "approved_at": approved_at,
    }))
}

#[allow(clippy::too_many_arguments)]
fn record_mutation(
    transaction: &Transaction<'_>,
    aggregate_kind: &str,
    entity_id: &str,
    revision_id: Option<&str>,
    action: &str,
    old_checksum: Option<&str>,
    new_checksum: Option<&str>,
    payload: &str,
    request_checksum: &str,
    response: &str,
    context: &WorkflowOperationContext,
    now: &str,
) -> Result<(), AgentError> {
    transaction.execute(
        "INSERT INTO method_workflow_audit_events (aggregate_kind, entity_id, revision_id, action, actor, reason, old_definition_checksum, new_definition_checksum, operation_id, device_id, correlation_id, payload_json, payload_checksum, occurred_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![aggregate_kind, entity_id, revision_id, action, context.actor, context.reason, old_checksum, new_checksum, context.operation_id, context.device_id, context.correlation_id, payload, request_checksum, now],
    ).map_err(sql_error("method_workflow_audit_write_failed"))?;
    transaction.execute(
        "INSERT INTO method_workflow_operations (operation_id, aggregate_kind, entity_id, action, request_checksum, result_revision_id, result_definition_checksum, response_json, occurred_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![context.operation_id, aggregate_kind, entity_id, action, request_checksum, revision_id, new_checksum, response, now],
    ).map_err(sql_error("method_workflow_operation_write_failed"))?;
    insert_test_template_sync_operation(
        transaction,
        TestTemplateSyncOperationInput {
            operation_id: &context.operation_id,
            entity_type: aggregate_kind,
            entity_id,
            operation_kind: action,
            base_revision: old_checksum.unwrap_or("none"),
            resulting_revision: revision_id.unwrap_or(new_checksum.unwrap_or(action)),
            actor_id: &context.actor,
            device_id: &context.device_id,
            correlation_id: &context.correlation_id,
            payload_json: payload,
            timestamp: now,
        },
    )?;
    Ok(())
}

fn operation_replay(
    connection: &Connection,
    operation_id: &str,
    request_checksum: &str,
) -> Result<Option<String>, AgentError> {
    let stored: Option<(String, String)> = connection.query_row(
        "SELECT request_checksum, response_json FROM method_workflow_operations WHERE operation_id = ?1",
        params![operation_id], |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional().map_err(sql_error("method_workflow_operation_query_failed"))?;
    match stored {
        Some((stored_checksum, response)) if stored_checksum == request_checksum => {
            let mut value: Value = serde_json::from_str(&response).map_err(|error| {
                AgentError::new("method_workflow_replay_decode_failed", error.to_string())
            })?;
            value["replayed"] = Value::Bool(true);
            Ok(Some(render_json(&value)))
        }
        Some((stored_checksum, _)) => Err(AgentError::with_details(
            "operation_replay_mismatch",
            "operation_id is already used for another method-workflow request",
            json!({ "operation_id": operation_id, "stored_checksum": stored_checksum, "request_checksum": request_checksum }),
        )),
        None => Ok(None),
    }
}

fn load_hierarchy(connection: &Connection) -> Result<Vec<MethodHierarchyNode>, AgentError> {
    let mut statement = connection.prepare(
        "SELECT node_id, parent_node_id, node_kind, label, position, archived FROM method_hierarchy_nodes ORDER BY parent_node_id, position, label"
    ).map_err(sql_error("method_hierarchy_query_failed"))?;
    let rows = statement
        .query_map([], |row| {
            let kind: String = row.get(2)?;
            Ok(MethodHierarchyNode {
                node_id: row.get(0)?,
                parent_node_id: row.get(1)?,
                node_kind: serde_json::from_value(Value::String(kind)).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?,
                label: row.get(3)?,
                position: row.get(4)?,
                archived: row.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(sql_error("method_hierarchy_query_failed"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("method_hierarchy_query_failed"))
}

fn load_hierarchy_node(
    connection: &Connection,
    node_id: &str,
) -> Result<Option<MethodHierarchyNode>, AgentError> {
    Ok(load_hierarchy(connection)?
        .into_iter()
        .find(|node| node.node_id == node_id))
}

fn reject_hierarchy_issues(nodes: &[MethodHierarchyNode]) -> Result<(), AgentError> {
    if let Some(issue) = validate_method_hierarchy(nodes).into_iter().next() {
        return Err(AgentError::with_details(
            "invalid_method_hierarchy",
            issue.message,
            json!({ "validation_code": issue.code, "path": issue.path }),
        ));
    }
    Ok(())
}

fn hierarchy_kind(node: &MethodHierarchyNode) -> String {
    serde_json::to_value(&node.node_kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .expect("hierarchy node kind serializes as a string")
}

fn validate_context(context: &WorkflowOperationContext) -> Result<(), AgentError> {
    for (field, value) in [
        ("actor", context.actor.as_str()),
        ("reason", context.reason.as_str()),
        ("operation_id", context.operation_id.as_str()),
        ("correlation_id", context.correlation_id.as_str()),
        ("device_id", context.device_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(AgentError::with_details(
                "invalid_method_workflow_context",
                format!("{field} is required"),
                json!({ "field": field }),
            ));
        }
    }
    Ok(())
}

fn validate_id_and_label(
    entity_id: &str,
    label: &str,
    classification: &str,
) -> Result<(), AgentError> {
    if entity_id.trim().is_empty() || label.trim().is_empty() || classification.trim().is_empty() {
        return Err(AgentError::new(
            "invalid_method_workflow_identity",
            "entity_id, label and classification are required",
        ));
    }
    Ok(())
}

fn workflow_validation_error(error: emc_locus_core::MethodWorkflowValidationIssue) -> AgentError {
    AgentError::with_details(
        "invalid_method_workflow_definition",
        error.message,
        json!({ "validation_code": error.code, "path": error.path }),
    )
}

fn workflow_revision_id(entity_id: &str, revision_number: u32) -> String {
    format!("{entity_id}-rev-{revision_number:04}")
}

fn checksum(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("sha256:{digest:x}")
}

fn operation_checksum(payload: &str, context: &WorkflowOperationContext) -> String {
    checksum(&render_json(&json!({
        "payload": serde_json::from_str::<Value>(payload)
            .expect("method workflow operation payload is canonical JSON"),
        "actor": context.actor,
        "reason": context.reason,
        "correlation_id": context.correlation_id,
        "device_id": context.device_id,
    })))
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_error", error.to_string()))
}

fn sql_error(code: &'static str) -> impl FnOnce(rusqlite::Error) -> AgentError {
    move |error| AgentError::new(code, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{run_storage_action, StorageAction};
    use emc_locus_core::MethodHierarchyNodeKind;
    use std::{fs, path::PathBuf, time::SystemTime};

    #[test]
    fn hierarchy_and_workflow_are_transactional_revisioned_and_restart_safe() {
        let storage_root = temporary_storage_root("method-workflow");
        run_storage_action(StorageAction::Init, storage_root.clone(), migrations_root()).unwrap();

        create_method_hierarchy_node(
            &storage_root,
            CreateHierarchyNodeInput {
                node: hierarchy_node("immunity", None, MethodHierarchyNodeKind::Domain, 10),
                context: context("hierarchy-root"),
            },
        )
        .unwrap();
        create_method_hierarchy_node(
            &storage_root,
            CreateHierarchyNodeInput {
                node: hierarchy_node(
                    "conducted",
                    Some("immunity"),
                    MethodHierarchyNodeKind::TestFamily,
                    10,
                ),
                context: context("hierarchy-child"),
            },
        )
        .unwrap();
        let before_refusal = evidence_counts(&storage_root);
        let cycle = update_method_hierarchy_node(
            &storage_root,
            UpdateHierarchyNodeInput {
                node: hierarchy_node(
                    "immunity",
                    Some("conducted"),
                    MethodHierarchyNodeKind::Domain,
                    10,
                ),
                expected_revision: 1,
                context: context("hierarchy-cycle"),
            },
        )
        .unwrap_err();
        assert_eq!(cycle.code, "invalid_method_hierarchy");
        assert_eq!(evidence_counts(&storage_root), before_refusal);

        let definition = system_definition("system-conducted", "Initial notes");
        let create_input = CreateWorkflowDefinitionInput {
            kind: WorkflowAggregateKind::MeasurementSystemTemplate,
            entity_id: "system-conducted".to_owned(),
            label: "Chaine conduite".to_owned(),
            classification: "immunity_conducted".to_owned(),
            definition_json: definition.clone(),
            validation: WorkflowValidationContext::default(),
            context: context("system-create"),
        };
        let created = create_workflow_definition(&storage_root, create_input.clone()).unwrap();
        let created_value: Value = serde_json::from_str(&created).unwrap();
        let first_checksum = created_value["revision"]["definition_checksum"]
            .as_str()
            .unwrap()
            .to_owned();
        let replay: Value = serde_json::from_str(
            &create_workflow_definition(&storage_root, create_input.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(replay["replayed"], true);
        let mut mismatch = create_input;
        mismatch.context.actor = "another.operator".to_owned();
        assert_eq!(
            create_workflow_definition(&storage_root, mismatch)
                .unwrap_err()
                .code,
            "operation_replay_mismatch"
        );

        let replaced: Value = serde_json::from_str(
            &replace_workflow_definition(
                &storage_root,
                ReplaceWorkflowDefinitionInput {
                    kind: WorkflowAggregateKind::MeasurementSystemTemplate,
                    entity_id: "system-conducted".to_owned(),
                    revision_id: "system-conducted-rev-0001".to_owned(),
                    expected_definition_checksum: first_checksum.clone(),
                    definition_json: system_definition("system-conducted", "Reviewed notes"),
                    validation: WorkflowValidationContext::default(),
                    context: context("system-replace"),
                },
            )
            .unwrap(),
        )
        .unwrap();
        let replacement_checksum = replaced["definition_checksum"].as_str().unwrap().to_owned();
        let before_conflict = evidence_counts(&storage_root);
        assert_eq!(
            replace_workflow_definition(
                &storage_root,
                ReplaceWorkflowDefinitionInput {
                    kind: WorkflowAggregateKind::MeasurementSystemTemplate,
                    entity_id: "system-conducted".to_owned(),
                    revision_id: "system-conducted-rev-0001".to_owned(),
                    expected_definition_checksum: first_checksum,
                    definition_json: system_definition("system-conducted", "Stale edit"),
                    validation: WorkflowValidationContext::default(),
                    context: context("system-stale"),
                },
            )
            .unwrap_err()
            .code,
            "method_workflow_definition_conflict"
        );
        assert_eq!(evidence_counts(&storage_root), before_conflict);

        transition_workflow_revision(
            &storage_root,
            transition_input("validated", "system-validate"),
        )
        .unwrap();
        transition_workflow_revision(
            &storage_root,
            transition_input("approved", "system-approve"),
        )
        .unwrap();

        let aggregate: Value = serde_json::from_str(
            &get_workflow_definition(
                &storage_root,
                WorkflowAggregateKind::MeasurementSystemTemplate,
                "system-conducted",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            aggregate["definition"]["identity"]["current_approved_revision_id"],
            "system-conducted-rev-0001"
        );
        assert_eq!(
            aggregate["definition"]["revisions"][0]["definition_checksum"],
            replacement_checksum
        );

        let connection = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        let alias: String = connection
            .query_row(
                "SELECT canonical_category_id FROM equipment_category_aliases WHERE alias_code = 'spectrum_analyzer'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(alias, "frequency_selective_measurement_instruments");
        drop(connection);
        fs::remove_dir_all(&storage_root).unwrap();
    }

    fn transition_input(status: &str, operation_id: &str) -> TransitionWorkflowRevisionInput {
        TransitionWorkflowRevisionInput {
            kind: WorkflowAggregateKind::MeasurementSystemTemplate,
            entity_id: "system-conducted".to_owned(),
            revision_id: "system-conducted-rev-0001".to_owned(),
            target_status: status.to_owned(),
            validation: WorkflowValidationContext::default(),
            context: context(operation_id),
        }
    }

    fn hierarchy_node(
        node_id: &str,
        parent_node_id: Option<&str>,
        node_kind: MethodHierarchyNodeKind,
        position: u32,
    ) -> MethodHierarchyNode {
        MethodHierarchyNode {
            node_id: node_id.to_owned(),
            parent_node_id: parent_node_id.map(str::to_owned),
            node_kind,
            label: node_id.to_owned(),
            position,
            archived: false,
        }
    }

    fn context(operation_id: &str) -> WorkflowOperationContext {
        WorkflowOperationContext {
            actor: "lab.engineer".to_owned(),
            reason: "0.22.2 workflow test".to_owned(),
            operation_id: operation_id.to_owned(),
            correlation_id: format!("corr-{operation_id}"),
            device_id: "test-agent".to_owned(),
        }
    }

    fn system_definition(template_id: &str, notes: &str) -> String {
        render_json(&json!({
            "definition_schema_version": "emc-locus.measurement-system-template-definition.v1",
            "template_id": template_id,
            "label": "Chaine de mesure conduite",
            "classification": "immunity_conducted",
            "nodes": [],
            "edges": [],
            "correction_points": [],
            "regulation_loops": [],
            "notes": notes,
        }))
    }

    fn evidence_counts(storage_root: &Path) -> (u64, u64) {
        let connection = Connection::open(storage_root.join("test_definitions.sqlite")).unwrap();
        let audit = connection
            .query_row(
                "SELECT COUNT(*) FROM method_workflow_audit_events",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let sync = Connection::open(storage_root.join("sync.sqlite"))
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM sync_operations WHERE entity_type IN ('method_hierarchy', 'measurement_system_template')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        (audit, sync)
    }

    fn migrations_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap()
            .join("storage/sqlite")
    }

    fn temporary_storage_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "emc-locus-agent-{label}-{}-{nonce}",
            std::process::id()
        ))
    }
}
