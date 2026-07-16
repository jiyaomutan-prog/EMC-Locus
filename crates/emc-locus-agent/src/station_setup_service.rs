use crate::equipment_repository::{
    load_equipment_model_revision, open_equipment_connection, StoredEquipmentModelRevision,
};
use crate::metrology_repository::{
    load_asset_characterization, load_instrument, open_metrology_connection,
};
use crate::metrology_service::{assess_metrology_readiness_report, AssessReadinessInput};
use crate::station_setup_dto::{
    revision_dto_unchecked, StationSetupAggregateDto, StationSetupAuditEventDto,
    StationSetupAuditListDto, StationSetupEnvelopeDto, StationSetupIdentityDto,
    StationSetupListDto, StationSetupOperationResultDto, StationSetupReadinessEnvelopeDto,
    StationSetupRevisionDto, StationSetupRevisionEnvelopeDto, StationSetupRevisionListDto,
};
use crate::station_setup_repository::{
    insert_station_setup_audit_event, insert_station_setup_identity,
    insert_station_setup_operation, insert_station_setup_outbox, insert_station_setup_revision,
    list_station_setup_identities, load_active_station_setup_draft,
    load_station_setup_audit_events, load_station_setup_identity, load_station_setup_operation,
    load_station_setup_revision, load_station_setup_revisions, mark_station_setup_ready,
    next_station_setup_revision_number, open_station_connection, open_station_connection_with_sync,
    replace_station_setup_draft, sha256_text, NewStationSetupIdentity, NewStationSetupRevision,
    ReplaceStationSetupDraft, StationSetupAuditInput, StationSetupOperationInput,
    StationSetupOutboxInput, StoredStationSetupIdentity, StoredStationSetupOperation,
    StoredStationSetupRevision,
};
use crate::{render_json, AgentError};
use emc_locus_core::{
    AssetCharacterizationDefinition, AuditActor, AuditReason, EquipmentModelDefinition,
    PortDirectionality, SignalDomain, SignalPortDefinition, StableId,
    StationMeasurementSetupDefinition, StationReadinessDimension, StationReadinessIssue,
    StationReadinessSeverity, StationSetupReadiness, STATION_SETUP_DEFINITION_SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StationOperationContext {
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub device_id: String,
    pub correlation_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateStationSetupInput {
    pub setup_id: String,
    pub label: String,
    pub laboratory_location_id: String,
    pub laboratory_location_label: String,
    pub planned_use_on: String,
    pub execution_mode: String,
    pub context: StationOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplaceStationSetupDraftInput {
    pub setup_id: String,
    pub revision_id: String,
    pub expected_definition_checksum: String,
    pub definition_json: String,
    pub context: StationOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkStationSetupReadyInput {
    pub setup_id: String,
    pub revision_id: String,
    pub expected_definition_checksum: String,
    pub context: StationOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeriveStationSetupRevisionInput {
    pub setup_id: String,
    pub source_revision_id: String,
    pub context: StationOperationContext,
}

pub fn create_station_setup(
    storage_root: &Path,
    input: CreateStationSetupInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    safe_id(&input.setup_id, "setup_id")?;
    safe_id(&input.laboratory_location_id, "laboratory_location_id")?;

    let definition = StationMeasurementSetupDefinition {
        definition_schema_version: STATION_SETUP_DEFINITION_SCHEMA_VERSION.to_owned(),
        setup_id: input.setup_id.trim().to_owned(),
        label: input.label.trim().to_owned(),
        laboratory_location_id: Some(input.laboratory_location_id.trim().to_owned()),
        laboratory_location_label: input.laboratory_location_label.trim().to_owned(),
        planned_use_on: input.planned_use_on.trim().to_owned(),
        execution_mode: input.execution_mode.trim().to_owned(),
        asset_bindings: Vec::new(),
        connections: Vec::new(),
        correction_selections: Vec::new(),
        notes: BTreeMap::new(),
    };
    let canonical = canonical_definition(&definition)?;
    let readiness = assess_station_setup_readiness(storage_root, &definition)?;
    let readiness_json = render_json(&readiness);
    let revision_id = format!("{}-rev-0001", canonical.setup_id);
    let payload_json = render_json(&json!({
        "definition": serde_json::from_str::<Value>(&canonical.canonical_json)
            .expect("canonical station setup definition must be valid JSON"),
        "reason": input.context.reason
    }));
    let payload_checksum = sha256_text(&payload_json);
    let timestamp = utc_timestamp()?;
    let mut connection = open_station_connection_with_sync(storage_root)?;

    if let Some(operation) = load_station_setup_operation(&connection, &input.context.operation_id)?
    {
        ensure_operation_replay(
            &operation,
            &input.context,
            &input.setup_id,
            "station_setup_created",
            &payload_checksum,
        )?;
        return operation_result(
            &connection,
            &input.setup_id,
            "station_setup_created",
            &input.context.operation_id,
            true,
        );
    }
    if load_station_setup_identity(&connection, &input.setup_id)?.is_some() {
        return Err(AgentError::new(
            "station_setup_exists",
            "a measurement setup with this identity already exists",
        ));
    }

    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_station_setup_identity(
        &transaction,
        NewStationSetupIdentity {
            setup_id: &canonical.setup_id,
            label: &canonical.label,
            created_by: input.context.actor.trim(),
            timestamp: &timestamp,
        },
    )?;
    insert_station_setup_revision(
        &transaction,
        NewStationSetupRevision {
            revision_id: &revision_id,
            setup_id: &canonical.setup_id,
            revision_number: 1,
            parent_revision_id: None,
            status: "draft",
            definition_schema_version: &canonical.definition_schema_version,
            definition_json: &canonical.canonical_json,
            definition_checksum: &canonical.definition_checksum,
            readiness_json: &readiness_json,
            created_by: input.context.actor.trim(),
            timestamp: &timestamp,
        },
    )?;
    persist_evidence(
        &transaction,
        &input.context,
        &canonical.setup_id,
        Some(&revision_id),
        "station_setup_created",
        None,
        Some(&revision_id),
        None,
        Some(&canonical.definition_checksum),
        "none",
        &revision_id,
        &payload_json,
        &payload_checksum,
        &timestamp,
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    operation_result(
        &connection,
        &canonical.setup_id,
        "station_setup_created",
        &input.context.operation_id,
        false,
    )
}

pub fn replace_station_setup_draft_definition(
    storage_root: &Path,
    input: ReplaceStationSetupDraftInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    safe_id(&input.setup_id, "setup_id")?;
    safe_id(&input.revision_id, "revision_id")?;
    canonical_checksum(
        &input.expected_definition_checksum,
        "expected_definition_checksum",
    )?;
    let definition = StationMeasurementSetupDefinition::from_json_str(&input.definition_json)
        .map_err(|issue| validation_error(vec![issue]))?;
    if definition.setup_id != input.setup_id.trim() {
        return Err(AgentError::new(
            "station_setup_identity_mismatch",
            "the definition belongs to another measurement setup",
        ));
    }
    let canonical = canonical_definition(&definition)?;
    let readiness = assess_station_setup_readiness(storage_root, &definition)?;
    let readiness_json = render_json(&readiness);
    let payload_json = render_json(&json!({
        "expected_definition_checksum": input.expected_definition_checksum,
        "definition": serde_json::from_str::<Value>(&canonical.canonical_json)
            .expect("canonical station setup definition must be valid JSON"),
        "reason": input.context.reason
    }));
    let payload_checksum = sha256_text(&payload_json);
    let timestamp = utc_timestamp()?;
    let mut connection = open_station_connection_with_sync(storage_root)?;

    if let Some(operation) = load_station_setup_operation(&connection, &input.context.operation_id)?
    {
        ensure_operation_replay(
            &operation,
            &input.context,
            &input.setup_id,
            "station_setup_draft_replaced",
            &payload_checksum,
        )?;
        return operation_result(
            &connection,
            &input.setup_id,
            "station_setup_draft_replaced",
            &input.context.operation_id,
            true,
        );
    }
    let stored = required_revision(&connection, &input.setup_id, &input.revision_id)?;
    if stored.status != "draft" {
        return Err(AgentError::new(
            "station_setup_revision_not_editable",
            "a setup marked ready cannot be modified; create a new draft",
        ));
    }
    if stored.definition_checksum != input.expected_definition_checksum {
        return Err(concurrency_error(&stored));
    }

    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    if !replace_station_setup_draft(
        &transaction,
        ReplaceStationSetupDraft {
            setup_id: &canonical.setup_id,
            revision_id: &input.revision_id,
            expected_definition_checksum: &input.expected_definition_checksum,
            label: &canonical.label,
            definition_schema_version: &canonical.definition_schema_version,
            definition_json: &canonical.canonical_json,
            definition_checksum: &canonical.definition_checksum,
            readiness_json: &readiness_json,
            timestamp: &timestamp,
        },
    )? {
        return Err(AgentError::new(
            "station_setup_concurrent_update",
            "the setup draft changed while it was being saved",
        ));
    }
    let base_revision = format!("draft:{}:{}", input.revision_id, stored.definition_checksum);
    let resulting_revision = format!(
        "draft:{}:{}",
        input.revision_id, canonical.definition_checksum
    );
    persist_evidence(
        &transaction,
        &input.context,
        &canonical.setup_id,
        Some(&input.revision_id),
        "station_setup_draft_replaced",
        Some(&input.revision_id),
        Some(&input.revision_id),
        Some(&stored.definition_checksum),
        Some(&canonical.definition_checksum),
        &base_revision,
        &resulting_revision,
        &payload_json,
        &payload_checksum,
        &timestamp,
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    operation_result(
        &connection,
        &canonical.setup_id,
        "station_setup_draft_replaced",
        &input.context.operation_id,
        false,
    )
}

pub fn mark_station_setup_revision_ready(
    storage_root: &Path,
    input: MarkStationSetupReadyInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    safe_id(&input.setup_id, "setup_id")?;
    safe_id(&input.revision_id, "revision_id")?;
    canonical_checksum(
        &input.expected_definition_checksum,
        "expected_definition_checksum",
    )?;
    let payload_json = render_json(&json!({
        "setup_id": input.setup_id,
        "revision_id": input.revision_id,
        "expected_definition_checksum": input.expected_definition_checksum,
        "reason": input.context.reason
    }));
    let payload_checksum = sha256_text(&payload_json);
    let timestamp = utc_timestamp()?;
    let mut connection = open_station_connection_with_sync(storage_root)?;

    if let Some(operation) = load_station_setup_operation(&connection, &input.context.operation_id)?
    {
        ensure_operation_replay(
            &operation,
            &input.context,
            &input.setup_id,
            "station_setup_marked_ready",
            &payload_checksum,
        )?;
        return operation_result(
            &connection,
            &input.setup_id,
            "station_setup_marked_ready",
            &input.context.operation_id,
            true,
        );
    }
    let stored = required_revision(&connection, &input.setup_id, &input.revision_id)?;
    if stored.status != "draft" {
        return Err(AgentError::new(
            "station_setup_revision_not_editable",
            "only a draft setup can be marked ready",
        ));
    }
    if stored.definition_checksum != input.expected_definition_checksum {
        return Err(concurrency_error(&stored));
    }
    let definition = validated_stored_definition(&stored)?;
    let readiness = assess_station_setup_readiness(storage_root, &definition)?;
    if !readiness.ready {
        return Err(AgentError::with_details(
            "station_setup_not_ready",
            "le montage contient encore des blocages avant câblage",
            serde_json::to_value(StationSetupReadinessEnvelopeDto {
                setup_id: input.setup_id.clone(),
                revision_id: input.revision_id.clone(),
                readiness,
            })
            .expect("station readiness must serialize"),
        ));
    }

    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    mark_station_setup_ready(
        &transaction,
        &input.setup_id,
        &input.revision_id,
        &timestamp,
    )?;
    let base_revision = format!("draft:{}", input.revision_id);
    let resulting_revision = format!("ready:{}", input.revision_id);
    persist_evidence(
        &transaction,
        &input.context,
        &input.setup_id,
        Some(&input.revision_id),
        "station_setup_marked_ready",
        Some(&input.revision_id),
        Some(&input.revision_id),
        Some(&stored.definition_checksum),
        Some(&stored.definition_checksum),
        &base_revision,
        &resulting_revision,
        &payload_json,
        &payload_checksum,
        &timestamp,
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    operation_result(
        &connection,
        &input.setup_id,
        "station_setup_marked_ready",
        &input.context.operation_id,
        false,
    )
}

pub fn derive_station_setup_revision(
    storage_root: &Path,
    input: DeriveStationSetupRevisionInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    safe_id(&input.setup_id, "setup_id")?;
    safe_id(&input.source_revision_id, "source_revision_id")?;
    let payload_json = render_json(&json!({
        "setup_id": input.setup_id,
        "source_revision_id": input.source_revision_id,
        "reason": input.context.reason
    }));
    let payload_checksum = sha256_text(&payload_json);
    let timestamp = utc_timestamp()?;
    let mut connection = open_station_connection_with_sync(storage_root)?;

    if let Some(operation) = load_station_setup_operation(&connection, &input.context.operation_id)?
    {
        ensure_operation_replay(
            &operation,
            &input.context,
            &input.setup_id,
            "station_setup_revision_derived",
            &payload_checksum,
        )?;
        return operation_result(
            &connection,
            &input.setup_id,
            "station_setup_revision_derived",
            &input.context.operation_id,
            true,
        );
    }
    if load_active_station_setup_draft(&connection, &input.setup_id)?.is_some() {
        return Err(AgentError::new(
            "station_setup_active_draft_exists",
            "finish or discard the current draft before creating another one",
        ));
    }
    let source = required_revision(&connection, &input.setup_id, &input.source_revision_id)?;
    if !matches!(source.status.as_str(), "ready" | "superseded") {
        return Err(AgentError::new(
            "station_setup_source_not_ready",
            "a new draft must be derived from a ready setup revision",
        ));
    }
    let definition = validated_stored_definition(&source)?;
    let readiness = assess_station_setup_readiness(storage_root, &definition)?;
    let readiness_json = render_json(&readiness);
    let revision_number = next_station_setup_revision_number(&connection, &input.setup_id)?;
    let revision_id = format!("{}-rev-{revision_number:04}", input.setup_id);

    let transaction = connection
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_station_setup_revision(
        &transaction,
        NewStationSetupRevision {
            revision_id: &revision_id,
            setup_id: &input.setup_id,
            revision_number,
            parent_revision_id: Some(&input.source_revision_id),
            status: "draft",
            definition_schema_version: &source.definition_schema_version,
            definition_json: &source.definition_json,
            definition_checksum: &source.definition_checksum,
            readiness_json: &readiness_json,
            created_by: input.context.actor.trim(),
            timestamp: &timestamp,
        },
    )?;
    persist_evidence(
        &transaction,
        &input.context,
        &input.setup_id,
        Some(&revision_id),
        "station_setup_revision_derived",
        Some(&input.source_revision_id),
        Some(&revision_id),
        Some(&source.definition_checksum),
        Some(&source.definition_checksum),
        &input.source_revision_id,
        &revision_id,
        &payload_json,
        &payload_checksum,
        &timestamp,
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    operation_result(
        &connection,
        &input.setup_id,
        "station_setup_revision_derived",
        &input.context.operation_id,
        false,
    )
}

pub fn list_station_setups(storage_root: &Path) -> Result<String, AgentError> {
    let connection = open_station_connection(storage_root)?;
    let station_setups = list_station_setup_identities(&connection)?
        .iter()
        .map(|identity| load_aggregate(&connection, identity))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(render_json(&StationSetupListDto { station_setups }))
}

pub fn get_station_setup(storage_root: &Path, setup_id: &str) -> Result<String, AgentError> {
    let connection = open_station_connection(storage_root)?;
    let identity = load_station_setup_identity(&connection, setup_id)?
        .ok_or_else(|| AgentError::new("station_setup_not_found", "measurement setup not found"))?;
    Ok(render_json(&StationSetupEnvelopeDto {
        station_setup: load_aggregate(&connection, &identity)?,
    }))
}

pub fn list_station_setup_revisions_json(
    storage_root: &Path,
    setup_id: &str,
) -> Result<String, AgentError> {
    let connection = open_station_connection(storage_root)?;
    require_identity(&connection, setup_id)?;
    let revisions = load_station_setup_revisions(&connection, setup_id)?
        .iter()
        .map(validated_revision_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(render_json(&StationSetupRevisionListDto {
        setup_id: setup_id.to_owned(),
        revisions,
    }))
}

pub fn get_station_setup_revision_json(
    storage_root: &Path,
    setup_id: &str,
    revision_id: &str,
) -> Result<String, AgentError> {
    let connection = open_station_connection(storage_root)?;
    let revision = required_revision(&connection, setup_id, revision_id)?;
    Ok(render_json(&StationSetupRevisionEnvelopeDto {
        revision: validated_revision_dto(&revision)?,
    }))
}

pub fn assess_station_setup_revision_json(
    storage_root: &Path,
    setup_id: &str,
    revision_id: &str,
) -> Result<String, AgentError> {
    let connection = open_station_connection(storage_root)?;
    let revision = required_revision(&connection, setup_id, revision_id)?;
    let definition = validated_stored_definition(&revision)?;
    Ok(render_json(&StationSetupReadinessEnvelopeDto {
        setup_id: setup_id.to_owned(),
        revision_id: revision_id.to_owned(),
        readiness: assess_station_setup_readiness(storage_root, &definition)?,
    }))
}

pub fn list_station_setup_audit_events_json(
    storage_root: &Path,
    setup_id: &str,
) -> Result<String, AgentError> {
    let connection = open_station_connection(storage_root)?;
    require_identity(&connection, setup_id)?;
    Ok(render_json(&StationSetupAuditListDto {
        setup_id: setup_id.to_owned(),
        audit_events: load_station_setup_audit_events(&connection, setup_id)?
            .into_iter()
            .map(StationSetupAuditEventDto::from)
            .collect(),
    }))
}

pub(crate) fn assess_station_setup_readiness(
    storage_root: &Path,
    definition: &StationMeasurementSetupDefinition,
) -> Result<StationSetupReadiness, AgentError> {
    let mut issues = definition.structural_readiness_issues();
    let binding_by_asset: BTreeMap<&str, &str> = definition
        .asset_bindings
        .iter()
        .map(|binding| (binding.asset_id.as_str(), binding.binding_id.as_str()))
        .collect();

    if !definition.asset_bindings.is_empty() {
        let report = assess_metrology_readiness_report(
            storage_root,
            AssessReadinessInput {
                asset_ids: definition
                    .asset_bindings
                    .iter()
                    .map(|binding| binding.asset_id.clone())
                    .collect(),
                execution_mode: definition.execution_mode.clone(),
                checked_on: definition.planned_use_on.clone(),
                context: Some(format!("Montage {}", definition.label)),
            },
        )?;
        for issue in report.blocking_issues {
            issues.push(metrology_issue(
                &issue.code,
                &issue.dimension,
                StationReadinessSeverity::Blocking,
                binding_by_asset.get(issue.asset_id.as_str()).copied(),
            ));
        }
        for issue in report.warnings {
            issues.push(metrology_issue(
                &issue.code,
                &issue.dimension,
                StationReadinessSeverity::Warning,
                binding_by_asset.get(issue.asset_id.as_str()).copied(),
            ));
        }
    }

    let metrology = open_metrology_connection(storage_root)?;
    let equipment = open_equipment_connection(storage_root)?;
    let mut models = BTreeMap::new();
    for binding in &definition.asset_bindings {
        let Some(instrument) = load_instrument(&metrology, &binding.asset_id)? else {
            continue;
        };
        if instrument.revision != binding.asset_revision
            || instrument.equipment_model_id.as_deref() != Some(binding.equipment_model_id.as_str())
            || instrument.equipment_model_revision_id.as_deref()
                != Some(binding.equipment_model_revision_id.as_str())
            || instrument.equipment_model_checksum.as_deref()
                != Some(binding.equipment_model_checksum.as_str())
        {
            issues.push(blocking_issue(
                "station_asset_reference_changed",
                StationReadinessDimension::AssetIdentity,
                "Le dossier du matériel a changé. Rechargez-le avant de valider le montage.",
                Some(&binding.binding_id),
                None,
            ));
            continue;
        }
        let Some(model_revision) = load_equipment_model_revision(
            &equipment,
            &binding.equipment_model_id,
            &binding.equipment_model_revision_id,
        )?
        else {
            issues.push(blocking_issue(
                "station_equipment_model_missing",
                StationReadinessDimension::AssetIdentity,
                "Le modèle approuvé du matériel n'est plus disponible localement.",
                Some(&binding.binding_id),
                None,
            ));
            continue;
        };
        if !matches!(model_revision.status.as_str(), "approved" | "superseded")
            || model_revision.definition_checksum != binding.equipment_model_checksum
        {
            issues.push(blocking_issue(
                "station_equipment_model_reference_mismatch",
                StationReadinessDimension::AssetIdentity,
                "La version du modèle ne correspond plus au matériel sélectionné.",
                Some(&binding.binding_id),
                None,
            ));
            continue;
        }
        if let Some(model) =
            validated_equipment_model(&model_revision, &mut issues, &binding.binding_id)
        {
            models.insert(binding.binding_id.as_str(), model);
        }
    }

    for connection in &definition.connections {
        let Some(from_model) = models.get(connection.from.binding_id.as_str()) else {
            continue;
        };
        let Some(to_model) = models.get(connection.to.binding_id.as_str()) else {
            continue;
        };
        let from_port = from_model
            .signal_ports
            .iter()
            .find(|port| port.port_id == connection.from.port_id);
        let to_port = to_model
            .signal_ports
            .iter()
            .find(|port| port.port_id == connection.to.port_id);
        let (Some(from_port), Some(to_port)) = (from_port, to_port) else {
            issues.push(blocking_issue(
                "station_connection_port_missing",
                StationReadinessDimension::PortCompatibility,
                "Un port sélectionné n'existe pas dans la version du modèle.",
                None,
                Some(&connection.connection_id),
            ));
            continue;
        };
        validate_port_compatibility(connection, from_port, to_port, &mut issues);
    }

    for selection in &definition.correction_selections {
        let binding = definition
            .asset_bindings
            .iter()
            .find(|binding| binding.binding_id == selection.binding_id)
            .expect("station setup integrity guarantees correction binding");
        let Some(characterization) =
            load_asset_characterization(&metrology, &selection.characterization_id)?
        else {
            issues.push(blocking_issue(
                "station_characterization_missing",
                StationReadinessDimension::CorrectionValidity,
                "La caractérisation choisie n'est plus disponible pour ce matériel.",
                Some(&binding.binding_id),
                None,
            ));
            continue;
        };
        if characterization.asset_id != binding.asset_id
            || characterization.definition_checksum != selection.characterization_checksum
            || characterization.characterization_kind != selection.correction_kind.as_str()
            || characterization.decision != "conforming"
            || characterization.performed_on > definition.planned_use_on
            || characterization.valid_until < definition.planned_use_on
        {
            issues.push(blocking_issue(
                "station_characterization_not_applicable",
                StationReadinessDimension::CorrectionValidity,
                "La caractérisation choisie n'est pas applicable à la date du montage.",
                Some(&binding.binding_id),
                None,
            ));
            continue;
        }
        let stored_definition =
            AssetCharacterizationDefinition::from_json_str(&characterization.definition_json)
                .map_err(|issue| {
                    AgentError::with_details(
                        "station_characterization_storage_invalid",
                        "stored material characterization is invalid",
                        json!({ "code": issue.code, "path": issue.path, "message": issue.message }),
                    )
                })?;
        let canonical = stored_definition.canonicalize().map_err(|validation| {
            AgentError::with_details(
                "station_characterization_storage_invalid",
                "stored material characterization is invalid",
                json!({ "validation": validation }),
            )
        })?;
        if canonical.definition_checksum != characterization.definition_checksum {
            return Err(AgentError::new(
                "station_characterization_storage_invalid",
                "stored material characterization checksum does not match its content",
            ));
        }
    }

    issues.sort_by(|left, right| {
        readiness_severity_order(left.severity)
            .cmp(&readiness_severity_order(right.severity))
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.binding_ids.cmp(&right.binding_ids))
            .then_with(|| left.connection_ids.cmp(&right.connection_ids))
    });
    Ok(StationSetupReadiness::from_issues(
        definition.planned_use_on.clone(),
        issues,
    ))
}

fn validated_equipment_model(
    stored: &StoredEquipmentModelRevision,
    issues: &mut Vec<StationReadinessIssue>,
    binding_id: &str,
) -> Option<EquipmentModelDefinition> {
    let model = match EquipmentModelDefinition::from_json_str(&stored.definition_json) {
        Ok(model) => model,
        Err(_) => {
            issues.push(blocking_issue(
                "station_equipment_model_storage_invalid",
                StationReadinessDimension::AssetIdentity,
                "La définition du modèle est illisible. Faites contrôler le référentiel.",
                Some(binding_id),
                None,
            ));
            return None;
        }
    };
    let Ok(canonical) = model.canonicalize() else {
        issues.push(blocking_issue(
            "station_equipment_model_storage_invalid",
            StationReadinessDimension::AssetIdentity,
            "La définition du modèle est incohérente. Faites contrôler le référentiel.",
            Some(binding_id),
            None,
        ));
        return None;
    };
    if canonical.definition_checksum != stored.definition_checksum {
        issues.push(blocking_issue(
            "station_equipment_model_storage_invalid",
            StationReadinessDimension::AssetIdentity,
            "L'empreinte du modèle ne correspond pas à sa définition.",
            Some(binding_id),
            None,
        ));
        return None;
    }
    Some(model)
}

fn validate_port_compatibility(
    connection: &emc_locus_core::StationConnectionDefinition,
    from: &SignalPortDefinition,
    to: &SignalPortDefinition,
    issues: &mut Vec<StationReadinessIssue>,
) {
    if !matches!(
        from.directionality,
        PortDirectionality::Output
            | PortDirectionality::Bidirectional
            | PortDirectionality::Through
    ) || !matches!(
        to.directionality,
        PortDirectionality::Input | PortDirectionality::Bidirectional | PortDirectionality::Through
    ) {
        issues.push(blocking_issue(
            "station_port_direction_mismatch",
            StationReadinessDimension::PortCompatibility,
            "La liaison ne va pas d'une sortie vers une entrée compatible.",
            None,
            Some(&connection.connection_id),
        ));
    }
    if from.signal_domain != to.signal_domain {
        issues.push(blocking_issue(
            "station_signal_domain_mismatch",
            StationReadinessDimension::PortCompatibility,
            "Les deux ports ne transportent pas le même domaine de signal.",
            None,
            Some(&connection.connection_id),
        ));
    }
    match (&from.connector_type, &to.connector_type) {
        (Some(left), Some(right)) if !left.trim().eq_ignore_ascii_case(right.trim()) => {
            issues.push(blocking_issue(
                "station_connector_mismatch",
                StationReadinessDimension::PortCompatibility,
                "Les connecteurs des deux ports sont incompatibles.",
                None,
                Some(&connection.connection_id),
            ));
        }
        (None, _) | (_, None) => issues.push(warning_issue(
            "station_connector_unknown",
            StationReadinessDimension::PortCompatibility,
            "Le connecteur n'est pas renseigné sur l'un des deux ports.",
            None,
            Some(&connection.connection_id),
        )),
        _ => {}
    }
    match (from.impedance, to.impedance) {
        (Some(left), Some(right)) if (left - right).abs() > left.abs().max(right.abs()) * 0.01 => {
            issues.push(blocking_issue(
                "station_impedance_mismatch",
                StationReadinessDimension::PortCompatibility,
                "Les impédances nominales des deux ports sont incompatibles.",
                None,
                Some(&connection.connection_id),
            ));
        }
        (None, _) | (_, None) if from.signal_domain == SignalDomain::Rf => {
            issues.push(warning_issue(
                "station_impedance_unknown",
                StationReadinessDimension::PortCompatibility,
                "L'impédance RF n'est pas renseignée sur l'un des deux ports.",
                None,
                Some(&connection.connection_id),
            ));
        }
        _ => {}
    }
    if let (Some(left_min), Some(left_max), Some(right_min), Some(right_max)) = (
        from.frequency_min,
        from.frequency_max,
        to.frequency_min,
        to.frequency_max,
    ) {
        if left_max < right_min || right_max < left_min {
            issues.push(blocking_issue(
                "station_frequency_range_mismatch",
                StationReadinessDimension::PortCompatibility,
                "Les plages de fréquence des deux ports ne se recouvrent pas.",
                None,
                Some(&connection.connection_id),
            ));
        }
    } else if from.signal_domain == SignalDomain::Rf {
        issues.push(warning_issue(
            "station_frequency_range_unknown",
            StationReadinessDimension::PortCompatibility,
            "La plage de fréquence n'est pas complètement renseignée pour cette liaison RF.",
            None,
            Some(&connection.connection_id),
        ));
    }
}

fn metrology_issue(
    code: &str,
    dimension: &str,
    severity: StationReadinessSeverity,
    binding_id: Option<&str>,
) -> StationReadinessIssue {
    let message = match code {
        "instrument_unknown" => "Le matériel n'est plus enregistré dans le registre métrologique.",
        "out_of_service" => "Le matériel est hors service.",
        "retired" => "Le matériel a été retiré du service.",
        "restricted" => "Le matériel comporte une restriction d'utilisation à examiner.",
        "calibration_missing" => "L'étalonnage requis est absent.",
        "calibration_expired" => "L'étalonnage requis est expiré à la date prévue.",
        "calibration_nonconforming" => "Le dernier étalonnage est non conforme.",
        "calibration_due_soon" => "L'échéance d'étalonnage est proche.",
        _ => "L'aptitude métrologique du matériel doit être vérifiée.",
    };
    StationReadinessIssue {
        code: code.to_owned(),
        severity,
        dimension: match dimension {
            "serviceability" => StationReadinessDimension::Serviceability,
            "calibration_validity" => StationReadinessDimension::CalibrationValidity,
            "missing_evidence" => StationReadinessDimension::MissingEvidence,
            "nonconformance" => StationReadinessDimension::Nonconformance,
            _ => StationReadinessDimension::AssetIdentity,
        },
        message: message.to_owned(),
        binding_ids: binding_id.into_iter().map(str::to_owned).collect(),
        connection_ids: Vec::new(),
    }
}

fn blocking_issue(
    code: &str,
    dimension: StationReadinessDimension,
    message: &str,
    binding_id: Option<&str>,
    connection_id: Option<&str>,
) -> StationReadinessIssue {
    readiness_issue(
        code,
        StationReadinessSeverity::Blocking,
        dimension,
        message,
        binding_id,
        connection_id,
    )
}

fn warning_issue(
    code: &str,
    dimension: StationReadinessDimension,
    message: &str,
    binding_id: Option<&str>,
    connection_id: Option<&str>,
) -> StationReadinessIssue {
    readiness_issue(
        code,
        StationReadinessSeverity::Warning,
        dimension,
        message,
        binding_id,
        connection_id,
    )
}

fn readiness_issue(
    code: &str,
    severity: StationReadinessSeverity,
    dimension: StationReadinessDimension,
    message: &str,
    binding_id: Option<&str>,
    connection_id: Option<&str>,
) -> StationReadinessIssue {
    StationReadinessIssue {
        code: code.to_owned(),
        severity,
        dimension,
        message: message.to_owned(),
        binding_ids: binding_id.into_iter().map(str::to_owned).collect(),
        connection_ids: connection_id.into_iter().map(str::to_owned).collect(),
    }
}

fn readiness_severity_order(value: StationReadinessSeverity) -> u8 {
    match value {
        StationReadinessSeverity::Blocking => 0,
        StationReadinessSeverity::Warning => 1,
    }
}

fn validated_stored_definition(
    stored: &StoredStationSetupRevision,
) -> Result<StationMeasurementSetupDefinition, AgentError> {
    let definition = StationMeasurementSetupDefinition::from_json_str(&stored.definition_json)
        .map_err(|issue| storage_validation_error(stored, json!({ "issue": issue })))?;
    let canonical = definition
        .canonicalize()
        .map_err(|issues| storage_validation_error(stored, json!({ "issues": issues })))?;
    if canonical.canonical_json != stored.definition_json
        || canonical.definition_checksum != stored.definition_checksum
        || canonical.definition_schema_version != stored.definition_schema_version
        || canonical.setup_id != stored.setup_id
    {
        return Err(storage_validation_error(
            stored,
            json!({ "message": "stored canonical definition evidence does not match" }),
        ));
    }
    Ok(definition)
}

fn validated_revision_dto(
    stored: &StoredStationSetupRevision,
) -> Result<StationSetupRevisionDto, AgentError> {
    let definition = validated_stored_definition(stored)?;
    let readiness =
        serde_json::from_str::<StationSetupReadiness>(&stored.readiness_json).map_err(|error| {
            storage_validation_error(stored, json!({ "readiness": error.to_string() }))
        })?;
    Ok(revision_dto_unchecked(stored, definition, readiness))
}

fn load_aggregate(
    connection: &rusqlite::Connection,
    identity: &StoredStationSetupIdentity,
) -> Result<StationSetupAggregateDto, AgentError> {
    let revisions = load_station_setup_revisions(connection, &identity.setup_id)?;
    let latest = revisions.first().ok_or_else(|| {
        AgentError::new(
            "station_setup_storage_invalid",
            "setup identity has no content revision",
        )
    })?;
    let active_draft_revision = revisions
        .iter()
        .find(|revision| revision.status == "draft")
        .map(validated_revision_dto)
        .transpose()?;
    let current_ready_revision = identity
        .current_ready_revision_id
        .as_deref()
        .map(|revision_id| {
            revisions
                .iter()
                .find(|revision| revision.revision_id == revision_id)
                .ok_or_else(|| {
                    AgentError::new(
                        "station_setup_storage_invalid",
                        "current ready revision reference is missing",
                    )
                })
                .and_then(validated_revision_dto)
        })
        .transpose()?;
    Ok(StationSetupAggregateDto {
        identity: StationSetupIdentityDto::from(identity),
        active_draft_revision,
        current_ready_revision,
        latest_revision: validated_revision_dto(latest)?,
    })
}

fn operation_result(
    connection: &rusqlite::Connection,
    setup_id: &str,
    operation: &str,
    operation_id: &str,
    replayed: bool,
) -> Result<String, AgentError> {
    let identity = require_identity(connection, setup_id)?;
    Ok(render_json(&StationSetupOperationResultDto {
        operation: operation.to_owned(),
        operation_id: operation_id.to_owned(),
        replayed,
        station_setup: load_aggregate(connection, &identity)?,
    }))
}

#[allow(clippy::too_many_arguments)]
fn persist_evidence(
    transaction: &rusqlite::Transaction<'_>,
    context: &StationOperationContext,
    setup_id: &str,
    revision_id: Option<&str>,
    action: &str,
    old_revision_id: Option<&str>,
    new_revision_id: Option<&str>,
    old_checksum: Option<&str>,
    new_checksum: Option<&str>,
    base_revision: &str,
    resulting_revision: &str,
    payload_json: &str,
    payload_checksum: &str,
    timestamp: &str,
) -> Result<(), AgentError> {
    let result_revision_id = new_revision_id.or(revision_id).unwrap_or("none");
    let result_checksum = new_checksum.or(old_checksum).unwrap_or("none");
    insert_station_setup_audit_event(
        transaction,
        StationSetupAuditInput {
            setup_id,
            revision_id,
            action,
            actor: context.actor.trim(),
            reason: context.reason.trim(),
            old_revision_id,
            new_revision_id,
            old_definition_checksum: old_checksum,
            new_definition_checksum: new_checksum,
            operation_id: &context.operation_id,
            device_id: &context.device_id,
            correlation_id: &context.correlation_id,
            payload_json,
            timestamp,
        },
    )?;
    insert_station_setup_operation(
        transaction,
        StationSetupOperationInput {
            operation_id: &context.operation_id,
            setup_id,
            action,
            actor: context.actor.trim(),
            device_id: &context.device_id,
            correlation_id: &context.correlation_id,
            payload_checksum,
            result_revision_id,
            result_definition_checksum: result_checksum,
            timestamp,
        },
    )?;
    insert_station_setup_outbox(
        transaction,
        StationSetupOutboxInput {
            operation_id: &context.operation_id,
            setup_id,
            operation_kind: action,
            base_revision,
            resulting_revision,
            actor: context.actor.trim(),
            device_id: &context.device_id,
            correlation_id: &context.correlation_id,
            payload_json,
            timestamp,
        },
    )?;
    Ok(())
}

fn ensure_operation_replay(
    operation: &StoredStationSetupOperation,
    context: &StationOperationContext,
    setup_id: &str,
    action: &str,
    payload_checksum: &str,
) -> Result<(), AgentError> {
    if operation.setup_id == setup_id
        && operation.action == action
        && operation.actor == context.actor.trim()
        && operation.device_id == context.device_id
        && operation.correlation_id == context.correlation_id
        && operation.payload_checksum == payload_checksum
    {
        return Ok(());
    }
    Err(AgentError::with_details(
        "operation_replay_mismatch",
        "operation_id is already used for a different station setup operation",
        json!({
            "operation_id": operation.operation_id,
            "existing_setup_id": operation.setup_id,
            "existing_action": operation.action
        }),
    ))
}

fn require_identity(
    connection: &rusqlite::Connection,
    setup_id: &str,
) -> Result<StoredStationSetupIdentity, AgentError> {
    load_station_setup_identity(connection, setup_id)?
        .ok_or_else(|| AgentError::new("station_setup_not_found", "measurement setup not found"))
}

fn required_revision(
    connection: &rusqlite::Connection,
    setup_id: &str,
    revision_id: &str,
) -> Result<StoredStationSetupRevision, AgentError> {
    let revision = load_station_setup_revision(connection, revision_id)?.ok_or_else(|| {
        AgentError::new(
            "station_setup_revision_not_found",
            "measurement setup revision not found",
        )
    })?;
    if revision.setup_id != setup_id {
        return Err(AgentError::new(
            "station_setup_revision_not_found",
            "measurement setup revision not found",
        ));
    }
    Ok(revision)
}

fn canonical_definition(
    definition: &StationMeasurementSetupDefinition,
) -> Result<emc_locus_core::CanonicalStationMeasurementSetupDefinition, AgentError> {
    definition.canonicalize().map_err(validation_error)
}

fn validation_error(issues: Vec<emc_locus_core::StationSetupValidationIssue>) -> AgentError {
    AgentError::with_details(
        "invalid_station_setup_definition",
        "le montage contient des données invalides",
        json!({ "issues": issues }),
    )
}

fn storage_validation_error(stored: &StoredStationSetupRevision, details: Value) -> AgentError {
    AgentError::with_details(
        "station_setup_storage_invalid",
        "stored station setup revision failed canonical validation",
        json!({
            "setup_id": stored.setup_id,
            "revision_id": stored.revision_id,
            "details": details
        }),
    )
}

fn concurrency_error(stored: &StoredStationSetupRevision) -> AgentError {
    AgentError::with_details(
        "station_setup_concurrent_update",
        "le brouillon a été modifié depuis son ouverture",
        json!({
            "revision_id": stored.revision_id,
            "current_definition_checksum": stored.definition_checksum,
            "status": stored.status
        }),
    )
}

fn validate_context(context: &StationOperationContext) -> Result<(), AgentError> {
    AuditActor::parse(context.actor.clone()).map_err(domain_error)?;
    AuditReason::parse(context.reason.clone()).map_err(domain_error)?;
    safe_id(&context.operation_id, "operation_id")?;
    safe_id(&context.device_id, "device_id")?;
    safe_id(&context.correlation_id, "correlation_id")?;
    Ok(())
}

fn safe_id(value: &str, field: &str) -> Result<(), AgentError> {
    StableId::parse(value.to_owned()).map_err(|error| {
        AgentError::new(
            "invalid_station_setup_request",
            format!("{field}: {error:?}"),
        )
    })?;
    Ok(())
}

fn canonical_checksum(value: &str, field: &str) -> Result<(), AgentError> {
    let valid = value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    });
    if valid {
        Ok(())
    } else {
        Err(AgentError::new(
            "invalid_station_setup_request",
            format!("{field} must use canonical sha256 form"),
        ))
    }
}

fn domain_error(error: emc_locus_core::DomainError) -> AgentError {
    AgentError::new("invalid_station_setup_request", format!("{error:?}"))
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_failed", error.to_string()))
}
