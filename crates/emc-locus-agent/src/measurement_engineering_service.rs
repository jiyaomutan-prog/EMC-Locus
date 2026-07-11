use crate::{
    equipment_repository::{open_equipment_connection, open_equipment_connection_with_sync},
    measurement_engineering_dto::{
        EngineeringCurveEvaluationDto, EngineeringCurveEvaluationEnvelopeDto,
        MeasurementEngineeringAggregateDto, MeasurementEngineeringAuditEventDto,
        MeasurementEngineeringAuditEventsDto, MeasurementEngineeringEnvelopeDto,
        MeasurementEngineeringIdentityDto, MeasurementEngineeringListDto,
        MeasurementEngineeringOperationResultDto, MeasurementEngineeringRevisionDto,
        MeasurementEngineeringRevisionEnvelopeDto, MeasurementEngineeringRevisionListDto,
        MeasurementEngineeringValidationDto, MeasurementEngineeringValidationIssueDto,
    },
    measurement_engineering_repository::{
        ensure_measurement_engineering_operation_replay,
        existing_measurement_engineering_operation, insert_measurement_engineering_audit_event,
        insert_measurement_engineering_identity, insert_measurement_engineering_revision,
        insert_measurement_engineering_sync_operation,
        list_approved_measurement_engineering_revisions, list_measurement_engineering_identities,
        list_measurement_engineering_revisions, load_active_draft_measurement_engineering_revision,
        load_current_approved_measurement_engineering_revision,
        load_latest_measurement_engineering_revision, load_measurement_engineering_audit_events,
        load_measurement_engineering_identity, load_measurement_engineering_revision,
        next_measurement_engineering_revision_number,
        supersede_approved_measurement_engineering_revision,
        touch_measurement_engineering_identity, update_measurement_engineering_revision_definition,
        update_measurement_engineering_revision_status, MeasurementEngineeringAuditEventInput,
        MeasurementEngineeringOperationFingerprintInput, MeasurementEngineeringStorageKind,
        MeasurementEngineeringSyncOperationInput, NewMeasurementEngineeringIdentityRecord,
        NewMeasurementEngineeringRevisionRecord, StoredMeasurementEngineeringAggregate,
        StoredMeasurementEngineeringAuditEvent, StoredMeasurementEngineeringIdentity,
        StoredMeasurementEngineeringOperation, StoredMeasurementEngineeringRevision,
        UpdateMeasurementEngineeringRevisionDefinitionInput,
        UpdateMeasurementEngineeringRevisionStatusInput,
    },
    render_json, AgentError,
};
use emc_locus_core::equipment::DefinitionValidationIssue;
use emc_locus_core::measurement_engineering::{
    evaluate_engineering_curve, validate_acquisition_channel_recipe_with_context,
    AcquisitionChannelRecipeDefinition, CanonicalMeasurementEngineeringDefinition,
    CurveInterpolation, DaqChannelProfileDefinition, EngineeringCurveDefinition,
    MeasurementEngineeringAggregateKind, MeasurementEngineeringDefinition,
    MeasurementEngineeringRevisionStatus, ResolvedAcquisitionRecipeContext,
    ScalingProfileDefinition, SensorDefinition,
};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateMeasurementEngineeringInput {
    pub kind: MeasurementEngineeringAggregateKind,
    pub entity_id: String,
    pub definition_json: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplaceMeasurementEngineeringDefinitionInput {
    pub kind: MeasurementEngineeringAggregateKind,
    pub entity_id: String,
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
pub struct CreateMeasurementEngineeringRevisionInput {
    pub kind: MeasurementEngineeringAggregateKind,
    pub entity_id: String,
    pub source_revision_id: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloneMeasurementEngineeringInput {
    pub kind: MeasurementEngineeringAggregateKind,
    pub source_entity_id: String,
    pub source_revision_id: Option<String>,
    pub new_entity_id: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionMeasurementEngineeringRevisionInput {
    pub kind: MeasurementEngineeringAggregateKind,
    pub entity_id: String,
    pub revision_id: String,
    pub target_status: MeasurementEngineeringRevisionStatus,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluateEngineeringCurveInput {
    pub curve_id: String,
    pub revision_id: String,
    pub axis_values: BTreeMap<String, f64>,
}

pub fn validate_measurement_engineering_definition_json(
    storage_root: &Path,
    kind: MeasurementEngineeringAggregateKind,
    definition_json: &str,
) -> Result<String, AgentError> {
    let connection = open_equipment_connection(storage_root)?;
    match canonical_definition(&connection, kind, definition_json) {
        Ok(canonical) => Ok(render_json(&MeasurementEngineeringValidationDto {
            valid: true,
            issues: Vec::new(),
            definition_schema_version: Some(canonical.definition_schema_version),
            definition_checksum: Some(canonical.definition_checksum),
            canonical_json: Some(canonical.canonical_json),
        })),
        Err(issues) => Ok(render_validation_error(issues)),
    }
}

pub fn create_measurement_engineering_definition(
    storage_root: &Path,
    input: CreateMeasurementEngineeringInput,
) -> Result<String, AgentError> {
    validate_create_input(&input)?;
    let kind = MeasurementEngineeringStorageKind::from_core(input.kind);
    let revision_id = revision_id_for(&input.entity_id, 1);
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let definition = canonical_definition(&connection, input.kind, &input.definition_json)
        .map_err(validation_error)?;
    if definition.entity_id != input.entity_id {
        return Err(AgentError::with_details(
            "measurement_engineering_entity_mismatch",
            "route entity id and definition id must match",
            json!({
                "route_entity_id": input.entity_id,
                "definition_entity_id": definition.entity_id,
                "aggregate_kind": input.kind.as_str()
            }),
        ));
    }
    let payload_json = create_payload_json(&input, &definition);
    if let Some(operation) =
        existing_measurement_engineering_operation(&connection, &input.operation_id)?
    {
        ensure_measurement_engineering_operation_replay(
            &operation,
            &input.operation_id,
            MeasurementEngineeringOperationFingerprintInput {
                aggregate_kind: input.kind.as_str(),
                entity_type: kind.entity_type,
                entity_id: &input.entity_id,
                revision_id: Some(&revision_id),
                action: created_action(input.kind),
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
        return operation_replay_result(&connection, kind, &operation);
    }
    if load_measurement_engineering_identity(&connection, kind, &input.entity_id)?.is_some() {
        return Err(AgentError::new(
            "measurement_engineering_already_exists",
            format!(
                "{} already exists: {}",
                input.kind.as_str(),
                input.entity_id
            ),
        ));
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_measurement_engineering_identity(
        &transaction,
        kind,
        NewMeasurementEngineeringIdentityRecord {
            entity_id: &input.entity_id,
            label: &definition.label,
            summary_kind: &definition.summary_kind,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_measurement_engineering_revision(
        &transaction,
        kind,
        NewMeasurementEngineeringRevisionRecord {
            revision_id: &revision_id,
            entity_id: &input.entity_id,
            revision_number: 1,
            parent_revision_id: None,
            status: revision_status_text(&MeasurementEngineeringRevisionStatus::Draft),
            definition_schema_version: &definition.definition_schema_version,
            definition_json: &definition.canonical_json,
            definition_checksum: &definition.definition_checksum,
            label: &definition.label,
            summary_kind: &definition.summary_kind,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_audit_and_outbox(
        &transaction,
        kind,
        AuditOutboxInput {
            entity_id: &input.entity_id,
            revision_id: Some(&revision_id),
            action: created_action(input.kind),
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
            base_revision: "none",
            resulting_revision: &revision_id,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    operation_result_for_revision(
        &connection,
        kind,
        created_action(input.kind),
        &input.operation_id,
        false,
        &input.entity_id,
        &revision_id,
    )
}

pub fn list_measurement_engineering_definitions(
    storage_root: &Path,
    kind: MeasurementEngineeringAggregateKind,
) -> Result<String, AgentError> {
    let storage_kind = MeasurementEngineeringStorageKind::from_core(kind);
    let connection = open_equipment_connection(storage_root)?;
    let identities = list_measurement_engineering_identities(&connection, storage_kind)?;
    let items = identities
        .iter()
        .map(|identity| aggregate_for_identity(&connection, storage_kind, identity))
        .collect::<Result<Vec<_>, AgentError>>()?;
    Ok(render_json(&MeasurementEngineeringListDto {
        aggregate_kind: kind.as_str().to_owned(),
        collection_key: storage_kind.route_collection_key.to_owned(),
        items: items.iter().map(aggregate_dto).collect(),
    }))
}

pub fn get_measurement_engineering_definition(
    storage_root: &Path,
    kind: MeasurementEngineeringAggregateKind,
    entity_id: &str,
) -> Result<String, AgentError> {
    validate_stable_id(entity_id, "entity_id")?;
    let storage_kind = MeasurementEngineeringStorageKind::from_core(kind);
    let connection = open_equipment_connection(storage_root)?;
    let aggregate = load_aggregate(&connection, storage_kind, entity_id)?;
    Ok(render_json(&MeasurementEngineeringEnvelopeDto {
        aggregate_kind: kind.as_str().to_owned(),
        item: aggregate_dto(&aggregate),
    }))
}

pub fn list_measurement_engineering_revisions_json(
    storage_root: &Path,
    kind: MeasurementEngineeringAggregateKind,
    entity_id: &str,
) -> Result<String, AgentError> {
    validate_stable_id(entity_id, "entity_id")?;
    let storage_kind = MeasurementEngineeringStorageKind::from_core(kind);
    let connection = open_equipment_connection(storage_root)?;
    load_measurement_engineering_identity(&connection, storage_kind, entity_id)?.ok_or_else(
        || {
            AgentError::new(
                "measurement_engineering_not_found",
                "definition does not exist",
            )
        },
    )?;
    let revisions = list_measurement_engineering_revisions(&connection, storage_kind, entity_id)?;
    Ok(render_json(&MeasurementEngineeringRevisionListDto {
        aggregate_kind: kind.as_str().to_owned(),
        entity_id: entity_id.to_owned(),
        revisions: revisions.iter().map(revision_dto).collect(),
    }))
}

pub fn get_measurement_engineering_revision_json(
    storage_root: &Path,
    kind: MeasurementEngineeringAggregateKind,
    entity_id: &str,
    revision_id: &str,
) -> Result<String, AgentError> {
    validate_stable_id(entity_id, "entity_id")?;
    validate_stable_id(revision_id, "revision_id")?;
    let storage_kind = MeasurementEngineeringStorageKind::from_core(kind);
    let connection = open_equipment_connection(storage_root)?;
    let revision = load_required_revision(&connection, storage_kind, entity_id, revision_id)?;
    Ok(render_json(&MeasurementEngineeringRevisionEnvelopeDto {
        aggregate_kind: kind.as_str().to_owned(),
        revision: revision_dto(&revision),
    }))
}

pub fn replace_measurement_engineering_revision_definition(
    storage_root: &Path,
    input: ReplaceMeasurementEngineeringDefinitionInput,
) -> Result<String, AgentError> {
    validate_replace_input(&input)?;
    let kind = MeasurementEngineeringStorageKind::from_core(input.kind);
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let definition = canonical_definition(&connection, input.kind, &input.definition_json)
        .map_err(validation_error)?;
    if definition.entity_id != input.entity_id {
        return Err(AgentError::with_details(
            "measurement_engineering_entity_mismatch",
            "route entity id and definition id must match",
            json!({
                "route_entity_id": input.entity_id,
                "definition_entity_id": definition.entity_id
            }),
        ));
    }
    let payload_json = replace_payload_json(&input, &definition);
    if let Some(operation) =
        existing_measurement_engineering_operation(&connection, &input.operation_id)?
    {
        ensure_measurement_engineering_operation_replay(
            &operation,
            &input.operation_id,
            MeasurementEngineeringOperationFingerprintInput {
                aggregate_kind: input.kind.as_str(),
                entity_type: kind.entity_type,
                entity_id: &input.entity_id,
                revision_id: Some(&input.revision_id),
                action: replaced_action(input.kind),
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
        return operation_replay_result(&connection, kind, &operation);
    }
    let revision = load_required_revision(&connection, kind, &input.entity_id, &input.revision_id)?;
    if revision.status != "draft" {
        return Err(AgentError::with_details(
            "measurement_engineering_revision_immutable",
            "only draft measurement-engineering revisions can be modified",
            json!({
                "aggregate_kind": input.kind.as_str(),
                "entity_id": input.entity_id,
                "revision_id": input.revision_id,
                "status": revision.status
            }),
        ));
    }
    if revision.definition_checksum != input.expected_definition_checksum {
        return Err(AgentError::with_details(
            "measurement_engineering_definition_checksum_mismatch",
            "draft definition was modified by another operation",
            json!({
                "entity_id": input.entity_id,
                "revision_id": input.revision_id,
                "expected_definition_checksum": input.expected_definition_checksum,
                "actual_definition_checksum": revision.definition_checksum
            }),
        ));
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let updated = update_measurement_engineering_revision_definition(
        &transaction,
        kind,
        UpdateMeasurementEngineeringRevisionDefinitionInput {
            entity_id: &input.entity_id,
            revision_id: &input.revision_id,
            expected_definition_checksum: &input.expected_definition_checksum,
            definition_schema_version: &definition.definition_schema_version,
            definition_json: &definition.canonical_json,
            definition_checksum: &definition.definition_checksum,
            label: &definition.label,
            summary_kind: &definition.summary_kind,
            timestamp: &now,
        },
    )?;
    if updated == 0 {
        drop(transaction);
        return definition_update_conflict(&connection, kind, &input);
    }
    touch_measurement_engineering_identity(&transaction, kind, &input.entity_id, &now)?;
    insert_audit_and_outbox(
        &transaction,
        kind,
        AuditOutboxInput {
            entity_id: &input.entity_id,
            revision_id: Some(&input.revision_id),
            action: replaced_action(input.kind),
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
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    operation_result_for_revision(
        &connection,
        kind,
        replaced_action(input.kind),
        &input.operation_id,
        false,
        &input.entity_id,
        &input.revision_id,
    )
}

pub fn create_measurement_engineering_revision(
    storage_root: &Path,
    input: CreateMeasurementEngineeringRevisionInput,
) -> Result<String, AgentError> {
    validate_create_revision_input(&input)?;
    let kind = MeasurementEngineeringStorageKind::from_core(input.kind);
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let identity = load_measurement_engineering_identity(&connection, kind, &input.entity_id)?
        .ok_or_else(|| {
            AgentError::new(
                "measurement_engineering_not_found",
                "definition does not exist",
            )
        })?;
    let source = load_required_revision(
        &connection,
        kind,
        &input.entity_id,
        &input.source_revision_id,
    )?;
    if source.status != "approved" {
        return Err(AgentError::with_details(
            "measurement_engineering_revision_source_not_approved",
            "new revisions must derive from an approved revision",
            json!({
                "entity_id": input.entity_id,
                "source_revision_id": input.source_revision_id,
                "status": source.status
            }),
        ));
    }
    let payload_json = create_revision_payload_json(&input);
    if let Some(operation) =
        existing_measurement_engineering_operation(&connection, &input.operation_id)?
    {
        let replay_revision_id = operation.revision_id.as_deref().ok_or_else(|| {
            AgentError::new(
                "operation_replay_missing_entity",
                "operation exists but revision is missing",
            )
        })?;
        ensure_measurement_engineering_operation_replay(
            &operation,
            &input.operation_id,
            MeasurementEngineeringOperationFingerprintInput {
                aggregate_kind: input.kind.as_str(),
                entity_type: kind.entity_type,
                entity_id: &input.entity_id,
                revision_id: Some(replay_revision_id),
                action: revision_created_action(input.kind),
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
        return operation_replay_result(&connection, kind, &operation);
    }
    if let Some(active_draft) =
        load_active_draft_measurement_engineering_revision(&connection, kind, &input.entity_id)?
    {
        return Err(AgentError::with_details(
            "measurement_engineering_active_draft_exists",
            "an identity can only have one active draft revision",
            json!({
                "entity_id": input.entity_id,
                "existing_draft_revision_id": active_draft.revision_id,
                "source_revision_id": input.source_revision_id
            }),
        ));
    }
    let revision_number =
        next_measurement_engineering_revision_number(&connection, kind, &input.entity_id)?;
    let revision_id = revision_id_for(&input.entity_id, revision_number);
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_measurement_engineering_revision(
        &transaction,
        kind,
        NewMeasurementEngineeringRevisionRecord {
            revision_id: &revision_id,
            entity_id: &input.entity_id,
            revision_number,
            parent_revision_id: Some(&input.source_revision_id),
            status: "draft",
            definition_schema_version: &source.definition_schema_version,
            definition_json: &source.definition_json,
            definition_checksum: &source.definition_checksum,
            label: &source.label,
            summary_kind: &source.summary_kind,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    touch_measurement_engineering_identity(&transaction, kind, &identity.entity_id, &now)?;
    insert_audit_and_outbox(
        &transaction,
        kind,
        AuditOutboxInput {
            entity_id: &input.entity_id,
            revision_id: Some(&revision_id),
            action: revision_created_action(input.kind),
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
            base_revision: &input.source_revision_id,
            resulting_revision: &revision_id,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    operation_result_for_revision(
        &connection,
        kind,
        revision_created_action(input.kind),
        &input.operation_id,
        false,
        &input.entity_id,
        &revision_id,
    )
}

pub fn clone_measurement_engineering_definition(
    storage_root: &Path,
    input: CloneMeasurementEngineeringInput,
) -> Result<String, AgentError> {
    validate_clone_input(&input)?;
    let kind = MeasurementEngineeringStorageKind::from_core(input.kind);
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let source_identity =
        load_measurement_engineering_identity(&connection, kind, &input.source_entity_id)?
            .ok_or_else(|| {
                AgentError::new(
                    "measurement_engineering_not_found",
                    "source definition does not exist",
                )
            })?;
    let source_revision = match input.source_revision_id.as_deref() {
        Some(revision_id) => {
            load_required_revision(&connection, kind, &input.source_entity_id, revision_id)?
        }
        None => source_identity
            .current_approved_revision_id
            .as_deref()
            .ok_or_else(|| {
                AgentError::with_details(
                    "measurement_engineering_revision_source_not_approved",
                    "source definition has no approved revision to clone",
                    json!({ "entity_id": input.source_entity_id }),
                )
            })
            .and_then(|revision_id| {
                load_required_revision(&connection, kind, &input.source_entity_id, revision_id)
            })?,
    };
    if source_revision.status != "approved" {
        return Err(AgentError::with_details(
            "measurement_engineering_revision_source_not_approved",
            "clone source must be an approved revision",
            json!({
                "entity_id": input.source_entity_id,
                "source_revision_id": source_revision.revision_id,
                "status": source_revision.status
            }),
        ));
    }
    let cloned_json = rewrite_definition_id(
        input.kind,
        &source_revision.definition_json,
        &input.new_entity_id,
    )?;
    let definition =
        canonical_definition(&connection, input.kind, &cloned_json).map_err(validation_error)?;
    let revision_id = revision_id_for(&input.new_entity_id, 1);
    let payload_json = clone_payload_json(&input, &source_revision, &definition);
    if let Some(operation) =
        existing_measurement_engineering_operation(&connection, &input.operation_id)?
    {
        ensure_measurement_engineering_operation_replay(
            &operation,
            &input.operation_id,
            MeasurementEngineeringOperationFingerprintInput {
                aggregate_kind: input.kind.as_str(),
                entity_type: kind.entity_type,
                entity_id: &input.new_entity_id,
                revision_id: Some(&revision_id),
                action: cloned_action(input.kind),
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
        return operation_replay_result(&connection, kind, &operation);
    }
    if load_measurement_engineering_identity(&connection, kind, &input.new_entity_id)?.is_some() {
        return Err(AgentError::new(
            "measurement_engineering_already_exists",
            format!(
                "{} already exists: {}",
                input.kind.as_str(),
                input.new_entity_id
            ),
        ));
    }
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_measurement_engineering_identity(
        &transaction,
        kind,
        NewMeasurementEngineeringIdentityRecord {
            entity_id: &input.new_entity_id,
            label: &definition.label,
            summary_kind: &definition.summary_kind,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_measurement_engineering_revision(
        &transaction,
        kind,
        NewMeasurementEngineeringRevisionRecord {
            revision_id: &revision_id,
            entity_id: &input.new_entity_id,
            revision_number: 1,
            parent_revision_id: None,
            status: "draft",
            definition_schema_version: &definition.definition_schema_version,
            definition_json: &definition.canonical_json,
            definition_checksum: &definition.definition_checksum,
            label: &definition.label,
            summary_kind: &definition.summary_kind,
            created_by: &input.actor,
            timestamp: &now,
        },
    )?;
    insert_audit_and_outbox(
        &transaction,
        kind,
        AuditOutboxInput {
            entity_id: &input.new_entity_id,
            revision_id: Some(&revision_id),
            action: cloned_action(input.kind),
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
            base_revision: &source_revision.revision_id,
            resulting_revision: &revision_id,
            timestamp: &now,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    operation_result_for_revision(
        &connection,
        kind,
        cloned_action(input.kind),
        &input.operation_id,
        false,
        &input.new_entity_id,
        &revision_id,
    )
}

pub fn transition_measurement_engineering_revision(
    storage_root: &Path,
    input: TransitionMeasurementEngineeringRevisionInput,
) -> Result<String, AgentError> {
    validate_transition_input(&input)?;
    let kind = MeasurementEngineeringStorageKind::from_core(input.kind);
    let operation_kind = transition_operation_kind(input.kind, input.target_status);
    let payload_json = transition_payload_json(&input);
    let mut connection = open_equipment_connection_with_sync(storage_root)?;
    let revision = load_required_revision(&connection, kind, &input.entity_id, &input.revision_id)?;
    if let Some(operation) =
        existing_measurement_engineering_operation(&connection, &input.operation_id)?
    {
        ensure_measurement_engineering_operation_replay(
            &operation,
            &input.operation_id,
            MeasurementEngineeringOperationFingerprintInput {
                aggregate_kind: input.kind.as_str(),
                entity_type: kind.entity_type,
                entity_id: &input.entity_id,
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
        return operation_replay_result(&connection, kind, &operation);
    }
    let current_status = parse_revision_status(&revision.status)?;
    if !is_allowed_revision_transition(current_status, input.target_status) {
        return Err(AgentError::with_details(
            "measurement_engineering_revision_transition_not_allowed",
            "revision cannot transition to requested status",
            json!({
                "aggregate_kind": input.kind.as_str(),
                "entity_id": input.entity_id,
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
    if input.target_status == MeasurementEngineeringRevisionStatus::Approved {
        let definition = canonical_definition(&connection, input.kind, &revision.definition_json)
            .map_err(validation_error)?;
        if definition.definition_checksum != revision.definition_checksum {
            return Err(AgentError::with_details(
                "measurement_engineering_revision_checksum_invalid",
                "stored revision checksum no longer matches canonical definition",
                json!({
                    "entity_id": input.entity_id,
                    "revision_id": input.revision_id
                }),
            ));
        }
    }
    let approved_revisions_to_supersede =
        if input.target_status == MeasurementEngineeringRevisionStatus::Approved {
            list_approved_measurement_engineering_revisions(&connection, kind, &input.entity_id)?
                .into_iter()
                .filter(|approved| approved.revision_id != input.revision_id)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
    let now = utc_timestamp()?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let updated = update_measurement_engineering_revision_status(
        &transaction,
        kind,
        UpdateMeasurementEngineeringRevisionStatusInput {
            entity_id: &input.entity_id,
            revision_id: &input.revision_id,
            expected_current_status: &revision.status,
            status: revision_status_text(&input.target_status),
            timestamp: &now,
        },
    )?;
    if updated == 0 {
        drop(transaction);
        return transition_cas_conflict(&connection, kind, &input, &revision.status);
    }
    insert_audit_and_outbox(
        &transaction,
        kind,
        AuditOutboxInput {
            entity_id: &input.entity_id,
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
            base_revision: &status_cursor(&revision.status, &input.revision_id),
            resulting_revision: &status_cursor(
                revision_status_text(&input.target_status),
                &input.revision_id,
            ),
            timestamp: &now,
        },
    )?;
    if input.target_status == MeasurementEngineeringRevisionStatus::Approved {
        for superseded_revision in approved_revisions_to_supersede {
            let superseded = supersede_approved_measurement_engineering_revision(
                &transaction,
                kind,
                &input.entity_id,
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
            insert_audit_and_outbox(
                &transaction,
                kind,
                AuditOutboxInput {
                    entity_id: &input.entity_id,
                    revision_id: Some(&superseded_revision.revision_id),
                    action: superseded_action(input.kind),
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
                    base_revision: &status_cursor("approved", &superseded_revision.revision_id),
                    resulting_revision: &status_cursor(
                        "superseded",
                        &superseded_revision.revision_id,
                    ),
                    timestamp: &now,
                },
            )?;
        }
    }
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    operation_result_for_revision(
        &connection,
        kind,
        operation_kind,
        &input.operation_id,
        false,
        &input.entity_id,
        &input.revision_id,
    )
}

pub fn list_measurement_engineering_audit_events(
    storage_root: &Path,
    kind: MeasurementEngineeringAggregateKind,
    entity_id: &str,
) -> Result<String, AgentError> {
    validate_stable_id(entity_id, "entity_id")?;
    let storage_kind = MeasurementEngineeringStorageKind::from_core(kind);
    let connection = open_equipment_connection(storage_root)?;
    load_measurement_engineering_identity(&connection, storage_kind, entity_id)?.ok_or_else(
        || {
            AgentError::new(
                "measurement_engineering_not_found",
                "definition does not exist",
            )
        },
    )?;
    let events = load_measurement_engineering_audit_events(&connection, storage_kind, entity_id)?;
    Ok(render_json(&MeasurementEngineeringAuditEventsDto {
        aggregate_kind: kind.as_str().to_owned(),
        entity_id: entity_id.to_owned(),
        audit_events: events.iter().map(audit_event_dto).collect(),
    }))
}

pub fn evaluate_engineering_curve_revision(
    storage_root: &Path,
    input: EvaluateEngineeringCurveInput,
) -> Result<String, AgentError> {
    validate_stable_id(&input.curve_id, "curve_id")?;
    validate_stable_id(&input.revision_id, "revision_id")?;
    let kind = MeasurementEngineeringStorageKind::from_core(
        MeasurementEngineeringAggregateKind::EngineeringCurve,
    );
    let connection = open_equipment_connection(storage_root)?;
    let revision = load_required_revision(&connection, kind, &input.curve_id, &input.revision_id)?;
    let definition = serde_json::from_str::<EngineeringCurveDefinition>(&revision.definition_json)
        .map_err(|error| {
            validation_error(vec![DefinitionValidationIssue {
                severity: "error".to_owned(),
                code: "invalid_engineering_curve_definition_json".to_owned(),
                path: "$".to_owned(),
                message: error.to_string(),
                suggestion: None,
            }])
        })?;
    let evaluation = evaluate_engineering_curve(
        &definition,
        input.axis_values,
        &revision.revision_id,
        &revision.definition_checksum,
    )
    .map_err(validation_error)?;
    Ok(render_json(&EngineeringCurveEvaluationEnvelopeDto {
        evaluation: EngineeringCurveEvaluationDto {
            values: evaluation.values,
            axis_values: evaluation.axis_values,
            interpolation: curve_interpolation_text(evaluation.interpolation).to_owned(),
            extrapolated: evaluation.extrapolated,
            warning: evaluation.warning,
            source_revision_id: evaluation.source_revision_id,
            source_checksum: evaluation.source_checksum,
        },
    }))
}

fn curve_interpolation_text(value: CurveInterpolation) -> &'static str {
    match value {
        CurveInterpolation::LinearXLinearY => "linear_x_linear_y",
        CurveInterpolation::LogXLinearY => "log_x_linear_y",
        CurveInterpolation::LinearXLogY => "linear_x_log_y",
        CurveInterpolation::Nearest => "nearest",
        CurveInterpolation::StepPrevious => "step_previous",
        CurveInterpolation::StepNext => "step_next",
    }
}

fn canonical_definition(
    connection: &Connection,
    kind: MeasurementEngineeringAggregateKind,
    definition_json: &str,
) -> Result<CanonicalMeasurementEngineeringDefinition, Vec<DefinitionValidationIssue>> {
    let definition = match MeasurementEngineeringDefinition::from_json_str(kind, definition_json) {
        Ok(definition) => definition,
        Err(issue) => return Err(vec![issue]),
    };
    let mut issues = definition.validate_all();
    issues.extend(validate_cross_references(connection, &definition));
    if let MeasurementEngineeringDefinition::Recipe(recipe) = &definition {
        issues.extend(validate_recipe_context(connection, recipe));
    }
    if issues.iter().any(|item| item.severity == "error") {
        return Err(issues);
    }
    definition.canonicalize()
}

fn validate_cross_references(
    connection: &Connection,
    definition: &MeasurementEngineeringDefinition,
) -> Vec<DefinitionValidationIssue> {
    let mut issues = Vec::new();
    match definition {
        MeasurementEngineeringDefinition::Sensor(sensor) => {
            for reference in &sensor.scaling_profile_refs {
                ensure_reference_approved(
                    connection,
                    MeasurementEngineeringAggregateKind::ScalingProfile,
                    &reference.entity_id,
                    reference.revision_id.as_deref(),
                    reference.require_approved,
                    "scaling_profile_refs",
                    &mut issues,
                );
            }
            for reference in &sensor.correction_curve_refs {
                ensure_reference_approved(
                    connection,
                    MeasurementEngineeringAggregateKind::EngineeringCurve,
                    &reference.entity_id,
                    reference.revision_id.as_deref(),
                    reference.require_approved,
                    "correction_curve_refs",
                    &mut issues,
                );
            }
        }
        MeasurementEngineeringDefinition::Recipe(recipe) => {
            ensure_reference_approved(
                connection,
                MeasurementEngineeringAggregateKind::DaqChannelProfile,
                &recipe.daq_channel_profile_ref.entity_id,
                recipe.daq_channel_profile_ref.revision_id.as_deref(),
                recipe.daq_channel_profile_ref.require_approved,
                "daq_channel_profile_ref",
                &mut issues,
            );
            if let Some(reference) = &recipe.sensor_definition_ref {
                ensure_reference_approved(
                    connection,
                    MeasurementEngineeringAggregateKind::SensorDefinition,
                    &reference.entity_id,
                    reference.revision_id.as_deref(),
                    reference.require_approved,
                    "sensor_definition_ref",
                    &mut issues,
                );
            }
            if let Some(reference) = &recipe.scaling_profile_ref {
                ensure_reference_approved(
                    connection,
                    MeasurementEngineeringAggregateKind::ScalingProfile,
                    &reference.entity_id,
                    reference.revision_id.as_deref(),
                    reference.require_approved,
                    "scaling_profile_ref",
                    &mut issues,
                );
            }
            for reference in &recipe.correction_curve_refs {
                ensure_reference_approved(
                    connection,
                    MeasurementEngineeringAggregateKind::EngineeringCurve,
                    &reference.entity_id,
                    reference.revision_id.as_deref(),
                    reference.require_approved,
                    "correction_curve_refs",
                    &mut issues,
                );
            }
        }
        _ => {}
    }
    issues
}

fn validate_recipe_context(
    connection: &Connection,
    recipe: &AcquisitionChannelRecipeDefinition,
) -> Vec<DefinitionValidationIssue> {
    let daq = load_approved_definition::<DaqChannelProfileDefinition>(
        connection,
        MeasurementEngineeringAggregateKind::DaqChannelProfile,
        &recipe.daq_channel_profile_ref.entity_id,
        recipe.daq_channel_profile_ref.revision_id.as_deref(),
    );
    let sensor = recipe.sensor_definition_ref.as_ref().and_then(|reference| {
        load_approved_definition::<SensorDefinition>(
            connection,
            MeasurementEngineeringAggregateKind::SensorDefinition,
            &reference.entity_id,
            reference.revision_id.as_deref(),
        )
    });
    let scaling = recipe.scaling_profile_ref.as_ref().and_then(|reference| {
        load_approved_definition::<ScalingProfileDefinition>(
            connection,
            MeasurementEngineeringAggregateKind::ScalingProfile,
            &reference.entity_id,
            reference.revision_id.as_deref(),
        )
    });
    let curves = recipe
        .correction_curve_refs
        .iter()
        .filter_map(|reference| {
            load_approved_definition::<EngineeringCurveDefinition>(
                connection,
                MeasurementEngineeringAggregateKind::EngineeringCurve,
                &reference.entity_id,
                reference.revision_id.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    validate_acquisition_channel_recipe_with_context(
        recipe,
        ResolvedAcquisitionRecipeContext {
            daq_channel_profile: daq.as_ref(),
            sensor_definition: sensor.as_ref(),
            scaling_profile: scaling.as_ref(),
            correction_curves: curves.iter().collect(),
        },
    )
}

trait ParseStoredDefinition: Sized {
    fn parse(value: &str) -> Option<Self>;
}

impl ParseStoredDefinition for DaqChannelProfileDefinition {
    fn parse(value: &str) -> Option<Self> {
        serde_json::from_str(value).ok()
    }
}

impl ParseStoredDefinition for SensorDefinition {
    fn parse(value: &str) -> Option<Self> {
        serde_json::from_str(value).ok()
    }
}

impl ParseStoredDefinition for ScalingProfileDefinition {
    fn parse(value: &str) -> Option<Self> {
        serde_json::from_str(value).ok()
    }
}

impl ParseStoredDefinition for EngineeringCurveDefinition {
    fn parse(value: &str) -> Option<Self> {
        serde_json::from_str(value).ok()
    }
}

fn load_approved_definition<T: ParseStoredDefinition>(
    connection: &Connection,
    kind: MeasurementEngineeringAggregateKind,
    entity_id: &str,
    revision_id: Option<&str>,
) -> Option<T> {
    let storage_kind = MeasurementEngineeringStorageKind::from_core(kind);
    let revision = match revision_id {
        Some(revision_id) => {
            load_measurement_engineering_revision(connection, storage_kind, entity_id, revision_id)
                .ok()
                .flatten()
        }
        None => load_measurement_engineering_identity(connection, storage_kind, entity_id)
            .ok()
            .flatten()
            .and_then(|identity| {
                load_current_approved_measurement_engineering_revision(
                    connection,
                    storage_kind,
                    &identity,
                )
                .ok()
                .flatten()
            }),
    }?;
    (revision.status == "approved").then(|| T::parse(&revision.definition_json))?
}

fn ensure_reference_approved(
    connection: &Connection,
    reference_kind: MeasurementEngineeringAggregateKind,
    entity_id: &str,
    revision_id: Option<&str>,
    require_approved: bool,
    path: &str,
    issues: &mut Vec<DefinitionValidationIssue>,
) {
    let storage_kind = MeasurementEngineeringStorageKind::from_core(reference_kind);
    let identity = match load_measurement_engineering_identity(connection, storage_kind, entity_id)
    {
        Ok(Some(identity)) => identity,
        Ok(None) => {
            issues.push(validation_issue(
                "error",
                "definition_reference_not_found",
                path,
                format!(
                    "referenced {} does not exist: {entity_id}",
                    reference_kind.as_str()
                ),
            ));
            return;
        }
        Err(error) => {
            issues.push(validation_issue(
                "error",
                "definition_reference_query_failed",
                path,
                error.to_string(),
            ));
            return;
        }
    };
    let revision = match revision_id {
        Some(revision_id) => {
            match load_measurement_engineering_revision(
                connection,
                storage_kind,
                entity_id,
                revision_id,
            ) {
                Ok(Some(revision)) => revision,
                _ => {
                    issues.push(validation_issue(
                        "error",
                        "definition_reference_revision_not_found",
                        path,
                        format!("referenced revision does not exist: {revision_id}"),
                    ));
                    return;
                }
            }
        }
        None => match load_current_approved_measurement_engineering_revision(
            connection,
            storage_kind,
            &identity,
        ) {
            Ok(Some(revision)) => revision,
            _ => {
                issues.push(validation_issue(
                    "error",
                    "definition_reference_missing_approved_revision",
                    path,
                    format!(
                        "referenced {} has no approved revision: {entity_id}",
                        reference_kind.as_str()
                    ),
                ));
                return;
            }
        },
    };
    if require_approved && revision.status != "approved" {
        issues.push(validation_issue(
            "error",
            "definition_reference_not_approved",
            path,
            format!(
                "referenced {} revision is not approved: {}",
                reference_kind.as_str(),
                revision.revision_id
            ),
        ));
    }
}

fn render_validation_error(issues: Vec<DefinitionValidationIssue>) -> String {
    render_json(&MeasurementEngineeringValidationDto {
        valid: false,
        issues: issues.iter().map(validation_issue_dto).collect(),
        definition_schema_version: None,
        definition_checksum: None,
        canonical_json: None,
    })
}

fn validation_error(issues: Vec<DefinitionValidationIssue>) -> AgentError {
    AgentError::with_details(
        "invalid_measurement_engineering_definition",
        "measurement-engineering definition failed validation",
        json!({
            "issues": issues.iter().map(validation_issue_dto).collect::<Vec<_>>()
        }),
    )
}

fn validation_issue(
    severity: &str,
    code: &str,
    path: &str,
    message: impl Into<String>,
) -> DefinitionValidationIssue {
    DefinitionValidationIssue {
        severity: severity.to_owned(),
        code: code.to_owned(),
        path: path.to_owned(),
        message: message.into(),
        suggestion: None,
    }
}

fn load_aggregate(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    entity_id: &str,
) -> Result<StoredMeasurementEngineeringAggregate, AgentError> {
    let identity =
        load_measurement_engineering_identity(connection, kind, entity_id)?.ok_or_else(|| {
            AgentError::new(
                "measurement_engineering_not_found",
                "definition does not exist",
            )
        })?;
    aggregate_for_identity(connection, kind, &identity)
}

fn aggregate_for_identity(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    identity: &StoredMeasurementEngineeringIdentity,
) -> Result<StoredMeasurementEngineeringAggregate, AgentError> {
    Ok(StoredMeasurementEngineeringAggregate {
        identity: identity.clone(),
        current_approved_revision: load_current_approved_measurement_engineering_revision(
            connection, kind, identity,
        )?,
        latest_revision: load_latest_measurement_engineering_revision(
            connection,
            kind,
            &identity.entity_id,
        )?,
        active_draft_revision: load_active_draft_measurement_engineering_revision(
            connection,
            kind,
            &identity.entity_id,
        )?,
    })
}

fn load_required_revision(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    entity_id: &str,
    revision_id: &str,
) -> Result<StoredMeasurementEngineeringRevision, AgentError> {
    load_measurement_engineering_revision(connection, kind, entity_id, revision_id)?.ok_or_else(
        || {
            AgentError::new(
                "measurement_engineering_revision_not_found",
                "revision does not exist",
            )
        },
    )
}

fn operation_result_for_revision(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    operation: &str,
    operation_id: &str,
    replayed: bool,
    entity_id: &str,
    revision_id: &str,
) -> Result<String, AgentError> {
    let aggregate = load_aggregate(connection, kind, entity_id)?;
    let revision = load_required_revision(connection, kind, entity_id, revision_id)?;
    Ok(render_json(&MeasurementEngineeringOperationResultDto {
        operation: operation.to_owned(),
        operation_id: operation_id.to_owned(),
        replayed,
        aggregate_kind: kind.aggregate_kind.as_str().to_owned(),
        item: aggregate_dto(&aggregate),
        revision: revision_dto(&revision),
    }))
}

fn operation_replay_result(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    operation: &StoredMeasurementEngineeringOperation,
) -> Result<String, AgentError> {
    let revision_id = operation.revision_id.as_deref().ok_or_else(|| {
        AgentError::new(
            "operation_replay_missing_entity",
            "operation exists but revision is missing",
        )
    })?;
    operation_result_for_revision(
        connection,
        kind,
        &operation.action,
        &operation.operation_id,
        true,
        &operation.entity_id,
        revision_id,
    )
}

struct AuditOutboxInput<'a> {
    entity_id: &'a str,
    revision_id: Option<&'a str>,
    action: &'a str,
    actor: &'a str,
    reason: &'a str,
    operation_id: &'a str,
    correlation_id: &'a str,
    device_id: &'a str,
    old_revision_id: Option<&'a str>,
    new_revision_id: Option<&'a str>,
    old_definition_checksum: Option<&'a str>,
    new_definition_checksum: Option<&'a str>,
    payload_json: &'a str,
    base_revision: &'a str,
    resulting_revision: &'a str,
    timestamp: &'a str,
}

fn insert_audit_and_outbox(
    transaction: &rusqlite::Transaction<'_>,
    kind: MeasurementEngineeringStorageKind,
    input: AuditOutboxInput<'_>,
) -> Result<(), AgentError> {
    insert_measurement_engineering_audit_event(
        transaction,
        MeasurementEngineeringAuditEventInput {
            aggregate_kind: kind.aggregate_kind.as_str(),
            entity_id: input.entity_id,
            revision_id: input.revision_id,
            action: input.action,
            actor: input.actor,
            reason: input.reason,
            operation_id: input.operation_id,
            correlation_id: input.correlation_id,
            device_id: input.device_id,
            old_revision_id: input.old_revision_id,
            new_revision_id: input.new_revision_id,
            old_definition_checksum: input.old_definition_checksum,
            new_definition_checksum: input.new_definition_checksum,
            payload_json: input.payload_json,
            timestamp: input.timestamp,
        },
    )?;
    insert_measurement_engineering_sync_operation(
        transaction,
        MeasurementEngineeringSyncOperationInput {
            operation_id: input.operation_id,
            entity_type: kind.entity_type,
            entity_id: input.entity_id,
            operation_kind: input.action,
            base_revision: input.base_revision,
            resulting_revision: input.resulting_revision,
            actor_id: input.actor,
            device_id: input.device_id,
            correlation_id: input.correlation_id,
            payload_json: input.payload_json,
            timestamp: input.timestamp,
        },
    )
}

fn aggregate_dto(
    aggregate: &StoredMeasurementEngineeringAggregate,
) -> MeasurementEngineeringAggregateDto {
    MeasurementEngineeringAggregateDto {
        identity: identity_dto(&aggregate.identity),
        current_approved_revision: aggregate
            .current_approved_revision
            .as_ref()
            .map(revision_dto),
        latest_revision: aggregate.latest_revision.as_ref().map(revision_dto),
        active_draft_revision: aggregate.active_draft_revision.as_ref().map(revision_dto),
    }
}

fn identity_dto(
    identity: &StoredMeasurementEngineeringIdentity,
) -> MeasurementEngineeringIdentityDto {
    MeasurementEngineeringIdentityDto {
        aggregate_kind: identity.aggregate_kind.as_str().to_owned(),
        entity_id: identity.entity_id.clone(),
        label: identity.label.clone(),
        summary_kind: identity.summary_kind.clone(),
        current_approved_revision_id: identity.current_approved_revision_id.clone(),
        created_by: identity.created_by.clone(),
        created_at: identity.created_at.clone(),
        updated_at: identity.updated_at.clone(),
    }
}

fn revision_dto(
    revision: &StoredMeasurementEngineeringRevision,
) -> MeasurementEngineeringRevisionDto {
    MeasurementEngineeringRevisionDto {
        aggregate_kind: revision.aggregate_kind.as_str().to_owned(),
        revision_id: revision.revision_id.clone(),
        entity_id: revision.entity_id.clone(),
        revision_number: revision.revision_number,
        parent_revision_id: revision.parent_revision_id.clone(),
        status: revision.status.clone(),
        definition_schema_version: revision.definition_schema_version.clone(),
        definition: serde_json::from_str(&revision.definition_json).unwrap_or(Value::Null),
        definition_checksum: revision.definition_checksum.clone(),
        label: revision.label.clone(),
        summary_kind: revision.summary_kind.clone(),
        created_by: revision.created_by.clone(),
        created_at: revision.created_at.clone(),
        updated_at: revision.updated_at.clone(),
        submitted_at: revision.submitted_at.clone(),
        approved_at: revision.approved_at.clone(),
    }
}

fn audit_event_dto(
    event: &StoredMeasurementEngineeringAuditEvent,
) -> MeasurementEngineeringAuditEventDto {
    MeasurementEngineeringAuditEventDto {
        audit_id: event.audit_id,
        aggregate_kind: event.aggregate_kind.clone(),
        entity_id: event.entity_id.clone(),
        revision_id: event.revision_id.clone(),
        action: event.action.clone(),
        actor: event.actor.clone(),
        reason: event.reason.clone(),
        old_revision_id: event.old_revision_id.clone(),
        new_revision_id: event.new_revision_id.clone(),
        old_definition_checksum: event.old_definition_checksum.clone(),
        new_definition_checksum: event.new_definition_checksum.clone(),
        operation_id: event.operation_id.clone(),
        device_id: event.device_id.clone(),
        correlation_id: event.correlation_id.clone(),
        payload_json: serde_json::from_str(&event.payload_json).unwrap_or(Value::Null),
        occurred_at: event.occurred_at.clone(),
    }
}

fn validation_issue_dto(
    issue: &DefinitionValidationIssue,
) -> MeasurementEngineeringValidationIssueDto {
    MeasurementEngineeringValidationIssueDto {
        severity: issue.severity.clone(),
        code: issue.code.clone(),
        path: issue.path.clone(),
        message: issue.message.clone(),
        suggestion: issue.suggestion.clone(),
    }
}

fn created_action(kind: MeasurementEngineeringAggregateKind) -> &'static str {
    match kind {
        MeasurementEngineeringAggregateKind::SensorDefinition => "sensor_definition_created",
        MeasurementEngineeringAggregateKind::ScalingProfile => "scaling_profile_created",
        MeasurementEngineeringAggregateKind::EngineeringCurve => "engineering_curve_created",
        MeasurementEngineeringAggregateKind::DaqChannelProfile => "daq_channel_profile_created",
        MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe => {
            "acquisition_channel_recipe_created"
        }
    }
}

fn replaced_action(kind: MeasurementEngineeringAggregateKind) -> &'static str {
    match kind {
        MeasurementEngineeringAggregateKind::SensorDefinition => "sensor_definition_replaced",
        MeasurementEngineeringAggregateKind::ScalingProfile => "scaling_profile_replaced",
        MeasurementEngineeringAggregateKind::EngineeringCurve => "engineering_curve_replaced",
        MeasurementEngineeringAggregateKind::DaqChannelProfile => "daq_channel_profile_replaced",
        MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe => {
            "acquisition_channel_recipe_replaced"
        }
    }
}

fn revision_created_action(kind: MeasurementEngineeringAggregateKind) -> &'static str {
    match kind {
        MeasurementEngineeringAggregateKind::SensorDefinition => {
            "sensor_definition_revision_created"
        }
        MeasurementEngineeringAggregateKind::ScalingProfile => "scaling_profile_revision_created",
        MeasurementEngineeringAggregateKind::EngineeringCurve => {
            "engineering_curve_revision_created"
        }
        MeasurementEngineeringAggregateKind::DaqChannelProfile => {
            "daq_channel_profile_revision_created"
        }
        MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe => {
            "acquisition_channel_recipe_revision_created"
        }
    }
}

fn cloned_action(kind: MeasurementEngineeringAggregateKind) -> &'static str {
    match kind {
        MeasurementEngineeringAggregateKind::SensorDefinition => "sensor_definition_cloned",
        MeasurementEngineeringAggregateKind::ScalingProfile => "scaling_profile_cloned",
        MeasurementEngineeringAggregateKind::EngineeringCurve => "engineering_curve_cloned",
        MeasurementEngineeringAggregateKind::DaqChannelProfile => "daq_channel_profile_cloned",
        MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe => {
            "acquisition_channel_recipe_cloned"
        }
    }
}

fn superseded_action(kind: MeasurementEngineeringAggregateKind) -> &'static str {
    match kind {
        MeasurementEngineeringAggregateKind::SensorDefinition => "sensor_definition_superseded",
        MeasurementEngineeringAggregateKind::ScalingProfile => "scaling_profile_superseded",
        MeasurementEngineeringAggregateKind::EngineeringCurve => "engineering_curve_superseded",
        MeasurementEngineeringAggregateKind::DaqChannelProfile => "daq_channel_profile_superseded",
        MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe => {
            "acquisition_channel_recipe_superseded"
        }
    }
}

fn transition_operation_kind(
    kind: MeasurementEngineeringAggregateKind,
    target_status: MeasurementEngineeringRevisionStatus,
) -> &'static str {
    match target_status {
        MeasurementEngineeringRevisionStatus::UnderReview => match kind {
            MeasurementEngineeringAggregateKind::SensorDefinition => "sensor_definition_submitted",
            MeasurementEngineeringAggregateKind::ScalingProfile => "scaling_profile_submitted",
            MeasurementEngineeringAggregateKind::EngineeringCurve => "engineering_curve_submitted",
            MeasurementEngineeringAggregateKind::DaqChannelProfile => {
                "daq_channel_profile_submitted"
            }
            MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe => {
                "acquisition_channel_recipe_submitted"
            }
        },
        MeasurementEngineeringRevisionStatus::Approved => match kind {
            MeasurementEngineeringAggregateKind::SensorDefinition => "sensor_definition_approved",
            MeasurementEngineeringAggregateKind::ScalingProfile => "scaling_profile_approved",
            MeasurementEngineeringAggregateKind::EngineeringCurve => "engineering_curve_approved",
            MeasurementEngineeringAggregateKind::DaqChannelProfile => {
                "daq_channel_profile_approved"
            }
            MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe => {
                "acquisition_channel_recipe_approved"
            }
        },
        _ => "measurement_engineering_transition",
    }
}

fn parse_revision_status(value: &str) -> Result<MeasurementEngineeringRevisionStatus, AgentError> {
    match value {
        "draft" => Ok(MeasurementEngineeringRevisionStatus::Draft),
        "under_review" => Ok(MeasurementEngineeringRevisionStatus::UnderReview),
        "approved" => Ok(MeasurementEngineeringRevisionStatus::Approved),
        "superseded" => Ok(MeasurementEngineeringRevisionStatus::Superseded),
        "suspended" => Ok(MeasurementEngineeringRevisionStatus::Suspended),
        "retired" => Ok(MeasurementEngineeringRevisionStatus::Retired),
        _ => Err(AgentError::new(
            "measurement_engineering_revision_status_invalid",
            format!("invalid revision status: {value}"),
        )),
    }
}

fn is_allowed_revision_transition(
    current: MeasurementEngineeringRevisionStatus,
    target: MeasurementEngineeringRevisionStatus,
) -> bool {
    matches!(
        (current, target),
        (
            MeasurementEngineeringRevisionStatus::Draft,
            MeasurementEngineeringRevisionStatus::UnderReview
        ) | (
            MeasurementEngineeringRevisionStatus::UnderReview,
            MeasurementEngineeringRevisionStatus::Approved
        )
    )
}

fn revision_status_text(status: &MeasurementEngineeringRevisionStatus) -> &'static str {
    match status {
        MeasurementEngineeringRevisionStatus::Draft => "draft",
        MeasurementEngineeringRevisionStatus::UnderReview => "under_review",
        MeasurementEngineeringRevisionStatus::Approved => "approved",
        MeasurementEngineeringRevisionStatus::Superseded => "superseded",
        MeasurementEngineeringRevisionStatus::Suspended => "suspended",
        MeasurementEngineeringRevisionStatus::Retired => "retired",
    }
}

fn revision_id_for(entity_id: &str, revision_number: u32) -> String {
    format!("{entity_id}-rev-{revision_number:04}")
}

fn definition_cursor(prefix: &str, revision_id: &str, checksum: &str) -> String {
    format!("{prefix}:{revision_id}:{checksum}")
}

fn status_cursor(status: &str, revision_id: &str) -> String {
    format!("status:{status}:{revision_id}")
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_format_failed", error.to_string()))
}

fn create_payload_json(
    input: &CreateMeasurementEngineeringInput,
    definition: &CanonicalMeasurementEngineeringDefinition,
) -> String {
    render_json(&json!({
        "aggregate_kind": input.kind.as_str(),
        "entity_id": input.entity_id,
        "revision_id": revision_id_for(&input.entity_id, 1),
        "definition_checksum": definition.definition_checksum,
        "label": definition.label,
        "summary_kind": definition.summary_kind
    }))
}

fn replace_payload_json(
    input: &ReplaceMeasurementEngineeringDefinitionInput,
    definition: &CanonicalMeasurementEngineeringDefinition,
) -> String {
    render_json(&json!({
        "aggregate_kind": input.kind.as_str(),
        "entity_id": input.entity_id,
        "revision_id": input.revision_id,
        "expected_definition_checksum": input.expected_definition_checksum,
        "new_definition_checksum": definition.definition_checksum,
        "label": definition.label,
        "summary_kind": definition.summary_kind
    }))
}

fn create_revision_payload_json(input: &CreateMeasurementEngineeringRevisionInput) -> String {
    render_json(&json!({
        "aggregate_kind": input.kind.as_str(),
        "entity_id": input.entity_id,
        "source_revision_id": input.source_revision_id
    }))
}

fn clone_payload_json(
    input: &CloneMeasurementEngineeringInput,
    source: &StoredMeasurementEngineeringRevision,
    definition: &CanonicalMeasurementEngineeringDefinition,
) -> String {
    render_json(&json!({
        "aggregate_kind": input.kind.as_str(),
        "source_entity_id": input.source_entity_id,
        "source_revision_id": source.revision_id,
        "new_entity_id": input.new_entity_id,
        "new_definition_checksum": definition.definition_checksum
    }))
}

fn transition_payload_json(input: &TransitionMeasurementEngineeringRevisionInput) -> String {
    render_json(&json!({
        "aggregate_kind": input.kind.as_str(),
        "entity_id": input.entity_id,
        "revision_id": input.revision_id,
        "target_status": revision_status_text(&input.target_status)
    }))
}

fn supersede_payload_json(
    input: &TransitionMeasurementEngineeringRevisionInput,
    superseded: &StoredMeasurementEngineeringRevision,
) -> String {
    render_json(&json!({
        "aggregate_kind": input.kind.as_str(),
        "entity_id": input.entity_id,
        "superseded_revision_id": superseded.revision_id,
        "new_current_revision_id": input.revision_id
    }))
}

fn rewrite_definition_id(
    kind: MeasurementEngineeringAggregateKind,
    definition_json: &str,
    new_entity_id: &str,
) -> Result<String, AgentError> {
    let mut value = serde_json::from_str::<Value>(definition_json)
        .map_err(|error| AgentError::new("invalid_json_body", error.to_string()))?;
    let field = match kind {
        MeasurementEngineeringAggregateKind::SensorDefinition => "sensor_definition_id",
        MeasurementEngineeringAggregateKind::ScalingProfile => "scaling_profile_id",
        MeasurementEngineeringAggregateKind::EngineeringCurve => "curve_id",
        MeasurementEngineeringAggregateKind::DaqChannelProfile => "daq_channel_profile_id",
        MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe => "recipe_id",
    };
    value[field] = Value::String(new_entity_id.to_owned());
    Ok(render_json(&value))
}

fn definition_update_conflict(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    input: &ReplaceMeasurementEngineeringDefinitionInput,
) -> Result<String, AgentError> {
    let actual = load_measurement_engineering_revision(
        connection,
        kind,
        &input.entity_id,
        &input.revision_id,
    )?
    .map(|revision| revision.definition_checksum);
    Err(AgentError::with_details(
        "measurement_engineering_definition_concurrent_update",
        "draft definition was modified concurrently",
        json!({
            "entity_id": input.entity_id,
            "revision_id": input.revision_id,
            "expected_definition_checksum": input.expected_definition_checksum,
            "actual_definition_checksum": actual
        }),
    ))
}

fn transition_cas_conflict(
    connection: &Connection,
    kind: MeasurementEngineeringStorageKind,
    input: &TransitionMeasurementEngineeringRevisionInput,
    expected_status: &str,
) -> Result<String, AgentError> {
    let actual = load_measurement_engineering_revision(
        connection,
        kind,
        &input.entity_id,
        &input.revision_id,
    )?
    .map(|revision| revision.status);
    Err(AgentError::with_details(
        "measurement_engineering_revision_transition_conflict",
        "revision status changed concurrently",
        json!({
            "entity_id": input.entity_id,
            "revision_id": input.revision_id,
            "expected_status": expected_status,
            "actual_status": actual
        }),
    ))
}

fn validate_create_input(input: &CreateMeasurementEngineeringInput) -> Result<(), AgentError> {
    validate_common_operation(
        &input.entity_id,
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )
}

fn validate_replace_input(
    input: &ReplaceMeasurementEngineeringDefinitionInput,
) -> Result<(), AgentError> {
    validate_common_operation(
        &input.entity_id,
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    validate_stable_id(&input.revision_id, "revision_id")?;
    validate_checksum(
        &input.expected_definition_checksum,
        "expected_definition_checksum",
    )
}

fn validate_create_revision_input(
    input: &CreateMeasurementEngineeringRevisionInput,
) -> Result<(), AgentError> {
    validate_common_operation(
        &input.entity_id,
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    validate_stable_id(&input.source_revision_id, "source_revision_id")
}

fn validate_clone_input(input: &CloneMeasurementEngineeringInput) -> Result<(), AgentError> {
    validate_common_operation(
        &input.new_entity_id,
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    validate_stable_id(&input.source_entity_id, "source_entity_id")?;
    if let Some(source_revision_id) = &input.source_revision_id {
        validate_stable_id(source_revision_id, "source_revision_id")?;
    }
    Ok(())
}

fn validate_transition_input(
    input: &TransitionMeasurementEngineeringRevisionInput,
) -> Result<(), AgentError> {
    validate_common_operation(
        &input.entity_id,
        &input.actor,
        &input.reason,
        &input.operation_id,
        &input.correlation_id,
        &input.device_id,
    )?;
    validate_stable_id(&input.revision_id, "revision_id")
}

fn validate_common_operation(
    entity_id: &str,
    actor: &str,
    reason: &str,
    operation_id: &str,
    correlation_id: &str,
    device_id: &str,
) -> Result<(), AgentError> {
    validate_stable_id(entity_id, "entity_id")?;
    validate_stable_id(operation_id, "operation_id")?;
    validate_stable_id(correlation_id, "correlation_id")?;
    validate_stable_id(device_id, "device_id")?;
    if actor.trim().is_empty() {
        return Err(AgentError::new("invalid_actor", "actor must not be blank"));
    }
    if reason.trim().is_empty() {
        return Err(AgentError::new(
            "invalid_reason",
            "reason must not be blank",
        ));
    }
    Ok(())
}

fn validate_stable_id(value: &str, field: &str) -> Result<(), AgentError> {
    if value.trim().is_empty()
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | ':')
        })
    {
        return Err(AgentError::with_details(
            "invalid_stable_id",
            format!("{field} must use ASCII letters, digits, hyphen, underscore or colon"),
            json!({ "field": field, "value": value }),
        ));
    }
    Ok(())
}

fn validate_checksum(value: &str, field: &str) -> Result<(), AgentError> {
    let Some(rest) = value.strip_prefix("sha256:") else {
        return Err(AgentError::with_details(
            "invalid_checksum",
            format!("{field} must be sha256:<64 hex characters>"),
            json!({ "field": field }),
        ));
    };
    if rest.len() != 64 || !rest.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(AgentError::with_details(
            "invalid_checksum",
            format!("{field} must be sha256:<64 hex characters>"),
            json!({ "field": field }),
        ));
    }
    Ok(())
}
