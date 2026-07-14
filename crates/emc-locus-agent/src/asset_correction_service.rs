use crate::asset_correction_repository::{
    approve_and_activate_asset_correction_assignment, insert_asset_correction_assignment,
    load_asset_correction_assignment, load_asset_correction_assignments,
    load_waiting_asset_corrections, reject_asset_correction_assignment,
    request_asset_correction_changes as request_assignment_changes,
    submit_asset_correction_assignment, NewAssetCorrectionAssignment,
    StoredAssetCorrectionAssignment,
};
use crate::equipment_repository::{load_equipment_model_revision, open_equipment_connection};
use crate::metrology_repository::{
    ensure_metrology_operation_replay, existing_metrology_operation, load_asset_characterization,
    load_instrument, next_metrology_audit_sequence, open_metrology_connection,
    open_metrology_connection_with_sync, MetrologyOperationFingerprintInput,
    StoredAssetCharacterization, StoredInstrument,
};
use crate::metrology_service::{
    revision_for, utc_timestamp, validate_operation_context, write_metrology_audit_and_outbox,
    MetrologyAuditWrite, MetrologyOperationContext,
};
use crate::{render_json, AgentError};
use emc_locus_core::{
    resolve_asset_corrections, validate_asset_correction_assignment,
    AssetCharacterizationDefinition, AssetCharacterizationKind, AssetCorrectionAssignment,
    AssetCorrectionAssignmentStatus, CorrectionRequirementDefinition, CorrectionRequirementKind,
    CorrectionSourceKind, EquipmentModelDefinition,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

const ENTITY_TYPE: &str = "asset_correction_assignment";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateAssetCorrectionAssignmentInput {
    pub assignment_id: String,
    pub asset_id: String,
    pub signal_path_id: String,
    pub requirement_id: String,
    pub source_event_id: String,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub conditions: BTreeMap<String, String>,
    pub context: MetrologyOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionAssetCorrectionInput {
    pub asset_id: String,
    pub assignment_id: String,
    pub expected_revision: String,
    pub context: MetrologyOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolveMaterialCorrectionsInput {
    pub asset_id: String,
    pub intended_use_on: String,
    pub execution_context: String,
    pub conditions: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct AssignmentEnvelope {
    assignment: AssetCorrectionAssignment,
    revision: String,
}

#[derive(Serialize)]
struct AssignmentListEnvelope {
    assignments: Vec<AssignmentEnvelope>,
}

pub fn create_asset_correction_assignment(
    storage_root: &Path,
    input: CreateAssetCorrectionAssignmentInput,
) -> Result<String, AgentError> {
    validate_operation_context(&input.context)?;
    require_token(&input.assignment_id, "assignment_id")?;
    require_token(&input.asset_id, "asset_id")?;
    require_token(&input.signal_path_id, "signal_path_id")?;
    require_token(&input.requirement_id, "requirement_id")?;
    require_token(&input.source_event_id, "source_event_id")?;

    let metrology = open_metrology_connection(storage_root)?;
    let instrument = required_instrument(&metrology, &input.asset_id)?;
    let source = required_source(&metrology, &input.source_event_id, &input.asset_id)?;
    let (model, model_id, model_revision_id, model_checksum) =
        pinned_model(storage_root, &instrument)?;
    let requirement = required_requirement(&model, &input.signal_path_id, &input.requirement_id)?;
    ensure_source_matches_requirement(&source, requirement)?;

    let valid_from = input
        .valid_from
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&source.valid_from)
        .to_owned();
    let valid_until = input
        .valid_until
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| Some(source.valid_until.clone()));
    ensure_assignment_within_source_validity(&valid_from, valid_until.as_deref(), &source)?;
    ensure_conditions_match(requirement, &input.conditions, &source)?;

    let assigned_at = utc_timestamp()?;
    let assignment = AssetCorrectionAssignment {
        assignment_id: input.assignment_id.trim().to_owned(),
        asset_id: input.asset_id.trim().to_owned(),
        equipment_model_id: model_id,
        equipment_model_revision_id: model_revision_id,
        equipment_model_checksum: model_checksum,
        signal_path_id: input.signal_path_id.trim().to_owned(),
        requirement_id: input.requirement_id.trim().to_owned(),
        correction_definition_id: source.characterization_id.clone(),
        correction_revision_id: source.revision.clone(),
        correction_checksum: source.definition_checksum.clone(),
        source_event_id: source.characterization_id.clone(),
        source_kind: parse_source_kind(&source.source_kind)?,
        valid_from,
        valid_until,
        status: AssetCorrectionAssignmentStatus::Draft,
        conditions: input.conditions.clone(),
        assigned_at: assigned_at.clone(),
        assigned_by: input.context.actor.trim().to_owned(),
        submitted_at: None,
        approved_at: None,
        approved_by: None,
        superseded_by: None,
    };
    validate_assignment(&assignment)?;

    let payload_json = render_json(&json!({ "assignment": assignment }));
    let mut connection = open_metrology_connection_with_sync(storage_root)?;
    if let Some(operation) = existing_metrology_operation(&connection, &input.context.operation_id)?
    {
        ensure_metrology_operation_replay(
            &operation,
            &input.context.operation_id,
            MetrologyOperationFingerprintInput {
                entity_type: ENTITY_TYPE,
                entity_id: &input.assignment_id,
                operation_kind: "asset_correction_created",
                base_revision: "rev-0000",
                actor_id: &input.context.actor,
                device_id: &input.context.device_id,
                correlation_id: &input.context.correlation_id,
                payload_json: &payload_json,
            },
        )?;
        return render_required_assignment(
            &connection,
            &input.assignment_id,
            Some(&input.asset_id),
        );
    }
    if load_asset_correction_assignment(&connection, &input.assignment_id)?.is_some() {
        return Err(AgentError::new(
            "asset_correction_already_exists",
            format!(
                "correction assignment already exists: {}",
                input.assignment_id
            ),
        ));
    }

    let revision = revision_for(ENTITY_TYPE, &input.assignment_id, &assigned_at);
    let sequence = next_metrology_audit_sequence(&connection, ENTITY_TYPE, &input.assignment_id)?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_asset_correction_assignment(
        &transaction,
        NewAssetCorrectionAssignment {
            assignment_id: &assignment.assignment_id,
            asset_id: &assignment.asset_id,
            equipment_model_id: &assignment.equipment_model_id,
            equipment_model_revision_id: &assignment.equipment_model_revision_id,
            equipment_model_checksum: &assignment.equipment_model_checksum,
            signal_path_id: &assignment.signal_path_id,
            requirement_id: &assignment.requirement_id,
            correction_definition_id: &assignment.correction_definition_id,
            correction_revision_id: &assignment.correction_revision_id,
            correction_checksum: &assignment.correction_checksum,
            source_event_id: &assignment.source_event_id,
            source_kind: assignment.source_kind_as_str(),
            valid_from: &assignment.valid_from,
            valid_until: assignment.valid_until.as_deref(),
            conditions_json: &render_json(&assignment.conditions),
            assigned_at: &assigned_at,
            assigned_by: &assignment.assigned_by,
            revision: &revision,
        },
    )?;
    write_metrology_audit_and_outbox(
        &transaction,
        MetrologyAuditWrite {
            entity_type: ENTITY_TYPE,
            entity_id: &assignment.assignment_id,
            sequence,
            action: "asset_correction_created",
            base_revision: "rev-0000",
            resulting_revision: &revision,
            context: &input.context,
            payload_json: &payload_json,
            timestamp: &assigned_at,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    let connection = open_metrology_connection(storage_root)?;
    render_required_assignment(&connection, &input.assignment_id, Some(&input.asset_id))
}

pub fn list_asset_correction_assignments(
    storage_root: &Path,
    asset_id: &str,
) -> Result<String, AgentError> {
    let connection = open_metrology_connection(storage_root)?;
    required_instrument(&connection, asset_id)?;
    render_assignment_list(load_asset_correction_assignments(&connection, asset_id)?)
}

pub fn get_asset_correction_assignment(
    storage_root: &Path,
    asset_id: &str,
    assignment_id: &str,
) -> Result<String, AgentError> {
    let connection = open_metrology_connection(storage_root)?;
    render_required_assignment(&connection, assignment_id, Some(asset_id))
}

pub fn list_asset_correction_review_queue(storage_root: &Path) -> Result<String, AgentError> {
    let connection = open_metrology_connection(storage_root)?;
    render_assignment_list(load_waiting_asset_corrections(&connection)?)
}

pub fn submit_asset_correction_for_review(
    storage_root: &Path,
    input: TransitionAssetCorrectionInput,
) -> Result<String, AgentError> {
    transition_assignment(storage_root, input, TransitionKind::Submit)
}

pub fn approve_and_activate_asset_correction(
    storage_root: &Path,
    input: TransitionAssetCorrectionInput,
) -> Result<String, AgentError> {
    transition_assignment(storage_root, input, TransitionKind::ApproveAndActivate)
}

pub fn reject_asset_correction(
    storage_root: &Path,
    input: TransitionAssetCorrectionInput,
) -> Result<String, AgentError> {
    transition_assignment(storage_root, input, TransitionKind::Reject)
}

pub fn request_asset_correction_changes(
    storage_root: &Path,
    input: TransitionAssetCorrectionInput,
) -> Result<String, AgentError> {
    transition_assignment(storage_root, input, TransitionKind::RequestChanges)
}

pub fn resolve_material_corrections(
    storage_root: &Path,
    input: ResolveMaterialCorrectionsInput,
) -> Result<String, AgentError> {
    require_token(&input.asset_id, "asset_id")?;
    validate_execution_context(&input.execution_context)?;
    validate_date(&input.intended_use_on, "intended_use_on")?;
    let connection = open_metrology_connection(storage_root)?;
    let instrument = required_instrument(&connection, &input.asset_id)?;
    let (model, model_id, model_revision_id, model_checksum) =
        pinned_model(storage_root, &instrument)?;
    let requirements = model
        .signal_paths
        .iter()
        .flat_map(|path| path.correction_requirements.iter().cloned())
        .collect::<Vec<_>>();
    if requirements.is_empty()
        && model
            .signal_paths
            .iter()
            .any(|path| !path.transformations.is_empty())
    {
        return Err(AgentError::new(
            "equipment_model_corrections_require_upgrade",
            "the pinned model still uses legacy transformation links and must be revised",
        ));
    }
    let assignments = load_asset_correction_assignments(&connection, &input.asset_id)?
        .iter()
        .map(assignment_from_stored)
        .collect::<Result<Vec<_>, _>>()?;
    let report = resolve_asset_corrections(
        input.asset_id.trim(),
        &requirements,
        &assignments,
        input.intended_use_on.trim(),
        input.execution_context.trim(),
        &input.conditions,
    );
    Ok(render_json(&json!({
        "asset_id": instrument.asset_id,
        "equipment_model_id": model_id,
        "equipment_model_revision_id": model_revision_id,
        "equipment_model_checksum": model_checksum,
        "report": report,
    })))
}

#[derive(Clone, Copy)]
enum TransitionKind {
    Submit,
    ApproveAndActivate,
    Reject,
    RequestChanges,
}

impl TransitionKind {
    fn action(self) -> &'static str {
        match self {
            Self::Submit => "asset_correction_submitted_for_review",
            Self::ApproveAndActivate => "asset_correction_approved_and_activated",
            Self::Reject => "asset_correction_rejected",
            Self::RequestChanges => "asset_correction_changes_requested",
        }
    }
}

fn transition_assignment(
    storage_root: &Path,
    input: TransitionAssetCorrectionInput,
    transition: TransitionKind,
) -> Result<String, AgentError> {
    validate_operation_context(&input.context)?;
    require_token(&input.asset_id, "asset_id")?;
    require_token(&input.assignment_id, "assignment_id")?;
    require_non_empty(&input.expected_revision, "expected_revision")?;
    let payload_json = render_json(&json!({
        "asset_id": input.asset_id.trim(),
        "assignment_id": input.assignment_id.trim(),
        "expected_revision": input.expected_revision.trim(),
        "transition": transition.action(),
    }));
    let mut connection = open_metrology_connection_with_sync(storage_root)?;
    if let Some(operation) = existing_metrology_operation(&connection, &input.context.operation_id)?
    {
        ensure_metrology_operation_replay(
            &operation,
            &input.context.operation_id,
            MetrologyOperationFingerprintInput {
                entity_type: ENTITY_TYPE,
                entity_id: input.assignment_id.trim(),
                operation_kind: transition.action(),
                base_revision: input.expected_revision.trim(),
                actor_id: &input.context.actor,
                device_id: &input.context.device_id,
                correlation_id: &input.context.correlation_id,
                payload_json: &payload_json,
            },
        )?;
        return render_required_assignment(
            &connection,
            &input.assignment_id,
            Some(&input.asset_id),
        );
    }
    let current = required_assignment(&connection, &input.assignment_id, Some(&input.asset_id))?;
    if current.revision != input.expected_revision.trim() {
        return Err(AgentError::with_details(
            "asset_correction_revision_conflict",
            "the correction assignment changed since it was opened",
            json!({
                "expected_revision": input.expected_revision.trim(),
                "actual_revision": current.revision,
            }),
        ));
    }
    if matches!(transition, TransitionKind::ApproveAndActivate) {
        validate_assignment_for_activation(storage_root, &connection, &current)?;
    }
    let timestamp = utc_timestamp()?;
    let resulting_revision = revision_for(transition.action(), &input.assignment_id, &timestamp);
    let sequence = next_metrology_audit_sequence(&connection, ENTITY_TYPE, &input.assignment_id)?;
    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    let mut superseded_assignments = Vec::new();
    let changed = match transition {
        TransitionKind::Submit => submit_asset_correction_assignment(
            &transaction,
            &input.assignment_id,
            &input.expected_revision,
            &timestamp,
            &resulting_revision,
        )?,
        TransitionKind::ApproveAndActivate => {
            superseded_assignments = approve_and_activate_asset_correction_assignment(
                &transaction,
                &current,
                &input.context.actor,
                &timestamp,
                &resulting_revision,
            )?;
            true
        }
        TransitionKind::Reject => reject_asset_correction_assignment(
            &transaction,
            &input.assignment_id,
            &input.expected_revision,
            &timestamp,
            &resulting_revision,
        )?,
        TransitionKind::RequestChanges => request_assignment_changes(
            &transaction,
            &input.assignment_id,
            &input.expected_revision,
            &timestamp,
            &resulting_revision,
        )?,
    };
    if !changed {
        return Err(AgentError::new(
            "asset_correction_transition_conflict",
            "the correction assignment is not in the required state or its revision changed",
        ));
    }
    write_metrology_audit_and_outbox(
        &transaction,
        MetrologyAuditWrite {
            entity_type: ENTITY_TYPE,
            entity_id: &input.assignment_id,
            sequence,
            action: transition.action(),
            base_revision: input.expected_revision.trim(),
            resulting_revision: &resulting_revision,
            context: &input.context,
            payload_json: &payload_json,
            timestamp: &timestamp,
        },
    )?;
    for superseded in superseded_assignments {
        let supersede_operation_id = format!(
            "{}:supersede:{}",
            input.context.operation_id, superseded.assignment_id
        );
        let supersede_revision = format!("superseded:{}:{}", superseded.assignment_id, timestamp);
        let supersede_payload = render_json(&json!({
            "asset_id": superseded.asset_id,
            "superseded_assignment_id": superseded.assignment_id,
            "replacement_assignment_id": input.assignment_id,
        }));
        let supersede_context = MetrologyOperationContext {
            actor: input.context.actor.clone(),
            reason: input.context.reason.clone(),
            operation_id: supersede_operation_id,
            correlation_id: input.context.correlation_id.clone(),
            device_id: input.context.device_id.clone(),
        };
        let supersede_sequence =
            next_metrology_audit_sequence(&transaction, ENTITY_TYPE, &superseded.assignment_id)?;
        write_metrology_audit_and_outbox(
            &transaction,
            MetrologyAuditWrite {
                entity_type: ENTITY_TYPE,
                entity_id: &superseded.assignment_id,
                sequence: supersede_sequence,
                action: "asset_correction_superseded",
                base_revision: &superseded.revision,
                resulting_revision: &supersede_revision,
                context: &supersede_context,
                payload_json: &supersede_payload,
                timestamp: &timestamp,
            },
        )?;
    }
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;
    let connection = open_metrology_connection(storage_root)?;
    render_required_assignment(&connection, &input.assignment_id, Some(&input.asset_id))
}

fn validate_assignment_for_activation(
    storage_root: &Path,
    connection: &rusqlite::Connection,
    stored: &StoredAssetCorrectionAssignment,
) -> Result<(), AgentError> {
    if stored.status != "waiting_for_review" {
        return Err(AgentError::new(
            "asset_correction_transition_conflict",
            "only a correction waiting for review can be activated",
        ));
    }
    let instrument = required_instrument(connection, &stored.asset_id)?;
    let (model, model_id, revision_id, checksum) = pinned_model(storage_root, &instrument)?;
    if model_id != stored.equipment_model_id
        || revision_id != stored.equipment_model_revision_id
        || checksum != stored.equipment_model_checksum
    {
        return Err(AgentError::new(
            "asset_correction_model_pin_changed",
            "the material model pin changed; create a correction assignment for the current model",
        ));
    }
    let requirement = required_requirement(&model, &stored.signal_path_id, &stored.requirement_id)?;
    let source = required_source(connection, &stored.source_event_id, &stored.asset_id)?;
    ensure_source_matches_requirement(&source, requirement)?;
    if source.decision != "conforming" {
        return Err(AgentError::new(
            "asset_correction_source_not_conforming",
            "only a conforming characterization can be activated",
        ));
    }
    if source.definition_checksum != stored.correction_checksum
        || source.revision != stored.correction_revision_id
    {
        return Err(AgentError::new(
            "asset_correction_source_pin_mismatch",
            "the pinned correction evidence no longer matches the recorded source",
        ));
    }
    validate_assignment(&assignment_from_stored(stored)?)
}

fn pinned_model(
    storage_root: &Path,
    instrument: &StoredInstrument,
) -> Result<(EquipmentModelDefinition, String, String, String), AgentError> {
    let model_id = instrument.equipment_model_id.as_deref().ok_or_else(|| {
        AgentError::new(
            "asset_correction_model_missing",
            "the material must be linked to an approved equipment model",
        )
    })?;
    let revision_id = instrument
        .equipment_model_revision_id
        .as_deref()
        .ok_or_else(|| {
            AgentError::new(
                "asset_correction_model_missing",
                "model revision is missing",
            )
        })?;
    let checksum = instrument
        .equipment_model_checksum
        .as_deref()
        .ok_or_else(|| {
            AgentError::new(
                "asset_correction_model_missing",
                "model checksum is missing",
            )
        })?;
    let connection = open_equipment_connection(storage_root)?;
    let revision =
        load_equipment_model_revision(&connection, model_id, revision_id)?.ok_or_else(|| {
            AgentError::new(
                "asset_correction_model_not_found",
                "the material's pinned equipment model revision does not exist",
            )
        })?;
    if !matches!(revision.status.as_str(), "approved" | "superseded")
        || revision.definition_checksum != checksum
    {
        return Err(AgentError::new(
            "asset_correction_model_pin_mismatch",
            "the material model pin is not an immutable approved revision",
        ));
    }
    let model =
        EquipmentModelDefinition::from_json_str(&revision.definition_json).map_err(|issue| {
            AgentError::with_details(
                "asset_correction_model_invalid",
                "the pinned equipment model definition is invalid",
                json!({ "issue": issue }),
            )
        })?;
    Ok((
        model,
        model_id.to_owned(),
        revision_id.to_owned(),
        checksum.to_owned(),
    ))
}

fn required_requirement<'a>(
    model: &'a EquipmentModelDefinition,
    signal_path_id: &str,
    requirement_id: &str,
) -> Result<&'a CorrectionRequirementDefinition, AgentError> {
    model
        .signal_paths
        .iter()
        .find(|path| path.path_id == signal_path_id)
        .and_then(|path| {
            path.correction_requirements
                .iter()
                .find(|requirement| requirement.requirement_id == requirement_id)
        })
        .ok_or_else(|| {
            AgentError::new(
                "asset_correction_requirement_not_found",
                format!("correction requirement does not exist: {signal_path_id}/{requirement_id}"),
            )
        })
}

fn required_instrument(
    connection: &rusqlite::Connection,
    asset_id: &str,
) -> Result<StoredInstrument, AgentError> {
    load_instrument(connection, asset_id.trim())?.ok_or_else(|| {
        AgentError::new(
            "metrology_instrument_not_found",
            format!("instrument does not exist: {}", asset_id.trim()),
        )
    })
}

fn required_source(
    connection: &rusqlite::Connection,
    source_event_id: &str,
    asset_id: &str,
) -> Result<StoredAssetCharacterization, AgentError> {
    let source =
        load_asset_characterization(connection, source_event_id.trim())?.ok_or_else(|| {
            AgentError::new(
                "asset_characterization_not_found",
                format!(
                    "asset characterization does not exist: {}",
                    source_event_id.trim()
                ),
            )
        })?;
    if source.asset_id != asset_id.trim() {
        return Err(AgentError::new(
            "asset_correction_source_asset_mismatch",
            "the characterization belongs to another material",
        ));
    }
    Ok(source)
}

fn ensure_source_matches_requirement(
    source: &StoredAssetCharacterization,
    requirement: &CorrectionRequirementDefinition,
) -> Result<(), AgentError> {
    let definition = AssetCharacterizationDefinition::from_json_str(&source.definition_json)
        .map_err(|issue| {
            AgentError::with_details(
                "asset_correction_source_invalid",
                "the characterization source is invalid",
                json!({ "issue": issue }),
            )
        })?;
    let canonical = definition.canonicalize().map_err(|issues| {
        AgentError::with_details(
            "asset_correction_source_invalid",
            "the characterization source is invalid",
            json!({ "issues": issues }),
        )
    })?;
    if canonical.definition_checksum != source.definition_checksum {
        return Err(AgentError::new(
            "asset_correction_source_checksum_mismatch",
            "the characterization source checksum does not match its canonical definition",
        ));
    }
    let compatible = matches!(
        (requirement.correction_kind, canonical.kind),
        (
            CorrectionRequirementKind::RawSignalConversion,
            AssetCharacterizationKind::TimeConversion
        ) | (
            CorrectionRequirementKind::FrequencyDependentCorrection,
            AssetCharacterizationKind::FrequencyResponse
        )
    );
    if !compatible {
        return Err(AgentError::new(
            "asset_correction_kind_mismatch",
            "the characterization type is incompatible with the model correction requirement",
        ));
    }
    Ok(())
}

fn ensure_assignment_within_source_validity(
    valid_from: &str,
    valid_until: Option<&str>,
    source: &StoredAssetCharacterization,
) -> Result<(), AgentError> {
    validate_date(valid_from, "valid_from")?;
    if let Some(value) = valid_until {
        validate_date(value, "valid_until")?;
        if value < valid_from {
            return Err(AgentError::new(
                "invalid_asset_correction_validity",
                "valid_until must be on or after valid_from",
            ));
        }
    }
    if valid_from < source.valid_from.as_str()
        || valid_until.is_some_and(|value| value > source.valid_until.as_str())
    {
        return Err(AgentError::new(
            "asset_correction_validity_exceeds_source",
            "assignment validity must remain within the characterization validity period",
        ));
    }
    Ok(())
}

fn ensure_conditions_match(
    requirement: &CorrectionRequirementDefinition,
    conditions: &BTreeMap<String, String>,
    source: &StoredAssetCharacterization,
) -> Result<(), AgentError> {
    for (key, expected) in &requirement.conditions {
        if conditions.get(key) != Some(expected) {
            return Err(AgentError::new(
                "asset_correction_condition_mismatch",
                format!("condition {key} must be {expected}"),
            ));
        }
    }
    let definition = AssetCharacterizationDefinition::from_json_str(&source.definition_json)
        .map_err(|_| {
            AgentError::new(
                "asset_correction_source_invalid",
                "invalid source definition",
            )
        })?;
    for (key, value) in conditions {
        if let Some(source_value) = definition.conditions.get(key) {
            let matches = match source_value {
                Value::String(source_value) => source_value == value,
                Value::Number(source_value) => source_value.to_string() == *value,
                Value::Bool(source_value) => source_value.to_string() == *value,
                _ => false,
            };
            if !matches {
                return Err(AgentError::new(
                    "asset_correction_condition_mismatch",
                    format!("condition {key} does not match the source evidence"),
                ));
            }
        }
    }
    Ok(())
}

fn render_assignment_list(
    stored: Vec<StoredAssetCorrectionAssignment>,
) -> Result<String, AgentError> {
    let assignments = stored
        .iter()
        .map(|value| {
            Ok(AssignmentEnvelope {
                assignment: assignment_from_stored(value)?,
                revision: value.revision.clone(),
            })
        })
        .collect::<Result<Vec<_>, AgentError>>()?;
    Ok(render_json(&AssignmentListEnvelope { assignments }))
}

fn render_required_assignment(
    connection: &rusqlite::Connection,
    assignment_id: &str,
    asset_id: Option<&str>,
) -> Result<String, AgentError> {
    let stored = required_assignment(connection, assignment_id, asset_id)?;
    Ok(render_json(&AssignmentEnvelope {
        assignment: assignment_from_stored(&stored)?,
        revision: stored.revision,
    }))
}

fn required_assignment(
    connection: &rusqlite::Connection,
    assignment_id: &str,
    asset_id: Option<&str>,
) -> Result<StoredAssetCorrectionAssignment, AgentError> {
    let stored =
        load_asset_correction_assignment(connection, assignment_id.trim())?.ok_or_else(|| {
            AgentError::new(
                "asset_correction_not_found",
                format!(
                    "correction assignment does not exist: {}",
                    assignment_id.trim()
                ),
            )
        })?;
    if asset_id.is_some_and(|value| stored.asset_id != value.trim()) {
        return Err(AgentError::new(
            "asset_correction_not_found",
            "the correction assignment does not belong to this material",
        ));
    }
    Ok(stored)
}

fn assignment_from_stored(
    stored: &StoredAssetCorrectionAssignment,
) -> Result<AssetCorrectionAssignment, AgentError> {
    let conditions = serde_json::from_str::<BTreeMap<String, String>>(&stored.conditions_json)
        .map_err(|error| AgentError::new("asset_correction_storage_invalid", error.to_string()))?;
    let assignment = AssetCorrectionAssignment {
        assignment_id: stored.assignment_id.clone(),
        asset_id: stored.asset_id.clone(),
        equipment_model_id: stored.equipment_model_id.clone(),
        equipment_model_revision_id: stored.equipment_model_revision_id.clone(),
        equipment_model_checksum: stored.equipment_model_checksum.clone(),
        signal_path_id: stored.signal_path_id.clone(),
        requirement_id: stored.requirement_id.clone(),
        correction_definition_id: stored.correction_definition_id.clone(),
        correction_revision_id: stored.correction_revision_id.clone(),
        correction_checksum: stored.correction_checksum.clone(),
        source_event_id: stored.source_event_id.clone(),
        source_kind: parse_source_kind(&stored.source_kind)?,
        valid_from: stored.valid_from.clone(),
        valid_until: stored.valid_until.clone(),
        status: parse_assignment_status(&stored.status)?,
        conditions,
        assigned_at: stored.assigned_at.clone(),
        assigned_by: stored.assigned_by.clone(),
        submitted_at: stored.submitted_at.clone(),
        approved_at: stored.approved_at.clone(),
        approved_by: stored.approved_by.clone(),
        superseded_by: stored.superseded_by.clone(),
    };
    validate_assignment(&assignment)?;
    Ok(assignment)
}

fn validate_assignment(assignment: &AssetCorrectionAssignment) -> Result<(), AgentError> {
    let issues = validate_asset_correction_assignment(assignment);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(AgentError::with_details(
            "invalid_asset_correction_assignment",
            "the asset correction assignment is invalid",
            json!({ "issues": issues }),
        ))
    }
}

fn parse_source_kind(value: &str) -> Result<CorrectionSourceKind, AgentError> {
    match value {
        "calibration" => Ok(CorrectionSourceKind::Calibration),
        "characterization" => Ok(CorrectionSourceKind::Characterization),
        "verification" => Ok(CorrectionSourceKind::Verification),
        "manufacturer_certificate" => Ok(CorrectionSourceKind::ManufacturerCertificate),
        "internal_measurement" => Ok(CorrectionSourceKind::InternalMeasurement),
        _ => Err(AgentError::new(
            "asset_correction_storage_invalid",
            format!("unknown correction source kind: {value}"),
        )),
    }
}

fn parse_assignment_status(value: &str) -> Result<AssetCorrectionAssignmentStatus, AgentError> {
    match value {
        "draft" => Ok(AssetCorrectionAssignmentStatus::Draft),
        "waiting_for_review" => Ok(AssetCorrectionAssignmentStatus::WaitingForReview),
        "approved" => Ok(AssetCorrectionAssignmentStatus::Approved),
        "active" => Ok(AssetCorrectionAssignmentStatus::Active),
        "expired" => Ok(AssetCorrectionAssignmentStatus::Expired),
        "superseded" => Ok(AssetCorrectionAssignmentStatus::Superseded),
        "rejected" => Ok(AssetCorrectionAssignmentStatus::Rejected),
        _ => Err(AgentError::new(
            "asset_correction_storage_invalid",
            format!("unknown correction assignment status: {value}"),
        )),
    }
}

trait SourceKindText {
    fn source_kind_as_str(&self) -> &'static str;
}

impl SourceKindText for AssetCorrectionAssignment {
    fn source_kind_as_str(&self) -> &'static str {
        match self.source_kind {
            CorrectionSourceKind::Calibration => "calibration",
            CorrectionSourceKind::Characterization => "characterization",
            CorrectionSourceKind::Verification => "verification",
            CorrectionSourceKind::ManufacturerCertificate => "manufacturer_certificate",
            CorrectionSourceKind::InternalMeasurement => "internal_measurement",
        }
    }
}

fn require_token(value: &str, field: &'static str) -> Result<(), AgentError> {
    let value = value.trim();
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(AgentError::new(
            "invalid_asset_correction_assignment",
            format!("{field} must be a machine-safe identifier"),
        ));
    }
    Ok(())
}

fn require_non_empty(value: &str, field: &'static str) -> Result<(), AgentError> {
    if value.trim().is_empty() {
        return Err(AgentError::new(
            "invalid_asset_correction_assignment",
            format!("{field} is required"),
        ));
    }
    Ok(())
}

fn validate_execution_context(value: &str) -> Result<(), AgentError> {
    match value.trim() {
        "accredited" | "non_accredited" | "investigation" | "simulation" => Ok(()),
        other => Err(AgentError::new(
            "invalid_asset_correction_resolution",
            format!("unknown execution context: {other}"),
        )),
    }
}

fn validate_date(value: &str, field: &'static str) -> Result<(), AgentError> {
    let value = value.trim();
    let valid = value.len() == 10
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value[0..4].parse::<u16>().is_ok()
        && value[5..7]
            .parse::<u8>()
            .is_ok_and(|month| (1..=12).contains(&month))
        && value[8..10]
            .parse::<u8>()
            .is_ok_and(|day| (1..=31).contains(&day));
    if valid {
        Ok(())
    } else {
        Err(AgentError::new(
            "invalid_asset_correction_resolution",
            format!("{field} must use YYYY-MM-DD"),
        ))
    }
}
