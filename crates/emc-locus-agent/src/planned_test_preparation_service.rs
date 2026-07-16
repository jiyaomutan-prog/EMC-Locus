use crate::equipment_repository::{load_equipment_model_revision, open_equipment_connection};
use crate::metrology_repository::{load_instrument, open_metrology_connection};
use crate::planned_test_preparation_dto::{
    PlannedTestPreparationAggregateDto, PlannedTestPreparationEnvelopeDto,
    PlannedTestPreparationOperationResultDto, PlannedTestPreparationOptionsDto,
    PlannedTestPreparationRevisionDto, PlannedTestPreparationRevisionEnvelopeDto,
    PlannedTestPreparationRevisionListDto, PlannedTestPreparationStationOptionDto,
};
use crate::planned_test_preparation_repository::{
    ensure_planned_test_preparation_tables, insert_planned_test_preparation_identity_if_missing,
    insert_planned_test_preparation_revision, list_planned_test_preparation_revisions,
    load_current_planned_test_preparation_revision, load_planned_test_preparation_identity,
    load_planned_test_preparation_operation, load_planned_test_preparation_revision,
    next_planned_test_preparation_revision_number,
    update_current_planned_test_preparation_revision, NewPlannedTestPreparationIdentity,
    NewPlannedTestPreparationRevision, StoredPlannedTestPreparationRevision,
};
use crate::project_repository::{
    existing_operation, insert_audit_event, insert_sync_operation, load_project,
    next_audit_sequence, open_project_connection, AuditEventInput, SyncOperationInput,
};
use crate::service_schedule_repository::{
    ensure_service_schedule_table, load_service_schedule_item, StoredServiceScheduleItem,
};
use crate::station_setup_repository::{
    list_station_setup_identities, load_station_setup_revision, open_station_connection,
    sha256_text, StoredStationSetupRevision,
};
use crate::station_setup_service::assess_station_setup_readiness;
use crate::test_template_repository::{
    list_test_template_identities, load_current_approved_test_template_revision,
    load_test_template_revision, open_test_template_connection, StoredTestTemplateRevision,
    TestTemplateListFilter,
};
use crate::{render_json, AgentError};
use emc_locus_core::test_definitions::{TemplateRevisionStatus, TestTemplateDefinition};
use emc_locus_core::{
    assess_planned_test_preparation, AuditActor, AuditReason, EquipmentModelDefinition,
    PlannedTestInstrumentAssignment, PlannedTestPreparationAssessmentInput,
    PlannedTestPreparationDefinition, PlannedTestPreparationState, PlannedTestScheduleSnapshot,
    PreparedEquipmentCapabilitySnapshot, PreparedStationAssetSnapshot,
    PreparedStationCorrectionSnapshot, PreparedStationSetupSnapshot, PreparedTestMethodSnapshot,
    ServiceScheduleStatus, StableId, StationMeasurementSetupDefinition, StationSetupReadiness,
    StationSetupRevisionStatus,
};
use serde::Serialize;
use serde_json::json;
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const PREPARATION_OPERATION: &str = "planned_test_preparation_assessed";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedTestPreparationOperationContext {
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub device_id: String,
    pub correlation_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssessPlannedTestPreparationInput {
    pub project_code: String,
    pub schedule_item_code: String,
    pub expected_schedule_revision: u64,
    pub expected_current_revision_id: Option<String>,
    pub method_template_id: String,
    pub method_revision_id: String,
    pub station_setup_id: String,
    pub station_setup_revision_id: String,
    pub assignments: Vec<PlannedTestInstrumentAssignment>,
    pub context: PlannedTestPreparationOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct PlannedTestPreparationStartEvidence {
    pub(crate) revision_id: String,
    pub(crate) definition_checksum: String,
}

#[derive(Serialize)]
struct AssessmentCommand<'a> {
    project_code: &'a str,
    schedule_item_code: &'a str,
    expected_schedule_revision: u64,
    expected_current_revision_id: Option<&'a str>,
    method_template_id: &'a str,
    method_revision_id: &'a str,
    station_setup_id: &'a str,
    station_setup_revision_id: &'a str,
    assignments: &'a [PlannedTestInstrumentAssignment],
    reason: &'a str,
}

pub fn get_planned_test_preparation(
    storage_root: &Path,
    project_code: &str,
    schedule_item_code: &str,
) -> Result<String, AgentError> {
    let connection = open_project_connection(storage_root)?;
    ensure_planned_test_preparation_tables(&connection)?;
    let schedule = load_owned_schedule(&connection, project_code, schedule_item_code)?;
    Ok(render_json(&PlannedTestPreparationEnvelopeDto {
        preparation: load_aggregate(&connection, &schedule)?,
    }))
}

pub fn list_planned_test_preparation_revisions_json(
    storage_root: &Path,
    project_code: &str,
    schedule_item_code: &str,
) -> Result<String, AgentError> {
    let connection = open_project_connection(storage_root)?;
    ensure_planned_test_preparation_tables(&connection)?;
    let schedule = load_owned_schedule(&connection, project_code, schedule_item_code)?;
    let identity =
        load_planned_test_preparation_identity(&connection, project_code, schedule_item_code)?;
    let current_revision_id = identity
        .as_ref()
        .and_then(|identity| identity.current_revision_id.as_deref());
    let current_status = schedule.to_domain()?.status();
    let revisions =
        list_planned_test_preparation_revisions(&connection, project_code, schedule_item_code)?
            .into_iter()
            .map(|revision| {
                stored_revision_dto(
                    &revision,
                    current_revision_id,
                    schedule.revision,
                    current_status,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
    Ok(render_json(&PlannedTestPreparationRevisionListDto {
        project_code: project_code.to_owned(),
        schedule_item_code: schedule_item_code.to_owned(),
        revisions,
    }))
}

pub fn get_planned_test_preparation_revision_json(
    storage_root: &Path,
    project_code: &str,
    schedule_item_code: &str,
    revision_id: &str,
) -> Result<String, AgentError> {
    let connection = open_project_connection(storage_root)?;
    ensure_planned_test_preparation_tables(&connection)?;
    let schedule = load_owned_schedule(&connection, project_code, schedule_item_code)?;
    let stored = load_planned_test_preparation_revision(&connection, revision_id)?
        .filter(|revision| {
            revision.project_code == project_code
                && revision.schedule_item_code == schedule_item_code
        })
        .ok_or_else(|| {
            AgentError::new(
                "planned_test_preparation_revision_not_found",
                "the requested preparation revision does not exist for this scheduled test",
            )
        })?;
    let identity =
        load_planned_test_preparation_identity(&connection, project_code, schedule_item_code)?;
    let current_revision_id = identity
        .as_ref()
        .and_then(|identity| identity.current_revision_id.as_deref());
    Ok(render_json(&PlannedTestPreparationRevisionEnvelopeDto {
        revision: stored_revision_dto(
            &stored,
            current_revision_id,
            schedule.revision,
            schedule.to_domain()?.status(),
        )?,
    }))
}

pub fn list_planned_test_preparation_options(
    storage_root: &Path,
    project_code: &str,
    schedule_item_code: &str,
) -> Result<String, AgentError> {
    let project_connection = open_project_connection(storage_root)?;
    let schedule = load_owned_schedule(&project_connection, project_code, schedule_item_code)?;
    let project = load_project(&project_connection, project_code)?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    ensure_schedule_is_preparable(&schedule)?;

    let templates = open_test_template_connection(storage_root)?;
    let mut methods = Vec::new();
    for identity in list_test_template_identities(&templates, TestTemplateListFilter::default())? {
        if let Some(revision) = load_current_approved_test_template_revision(&templates, &identity)?
        {
            methods.push(method_snapshot(&revision, true)?);
        }
    }
    methods.sort_by(|left, right| {
        left.title
            .cmp(&right.title)
            .then_with(|| left.template_id.cmp(&right.template_id))
    });

    let stations = open_station_connection(storage_root)?;
    let mut station_setups = Vec::new();
    for identity in list_station_setup_identities(&stations)? {
        let Some(revision_id) = identity.current_ready_revision_id.as_deref() else {
            continue;
        };
        let revision = load_station_setup_revision(&stations, revision_id)?.ok_or_else(|| {
            AgentError::new(
                "station_setup_storage_invalid",
                "a station identity references a missing ready revision",
            )
        })?;
        let loaded = station_snapshot(
            storage_root,
            &revision,
            true,
            &schedule.planned_start_at,
            &project.execution_mode,
        )?;
        station_setups.push(PlannedTestPreparationStationOptionDto {
            station_setup: loaded.snapshot,
            readiness: loaded.readiness,
        });
    }
    station_setups.sort_by(|left, right| {
        left.station_setup
            .label
            .cmp(&right.station_setup.label)
            .then_with(|| {
                left.station_setup
                    .setup_id
                    .cmp(&right.station_setup.setup_id)
            })
    });

    Ok(render_json(&PlannedTestPreparationOptionsDto {
        project_code: project_code.to_owned(),
        schedule_item_code: schedule_item_code.to_owned(),
        methods,
        station_setups,
    }))
}

pub fn assess_planned_test_preparation_for_schedule(
    storage_root: &Path,
    mut input: AssessPlannedTestPreparationInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    validate_id(&input.project_code, "project_code")?;
    validate_id(&input.schedule_item_code, "schedule_item_code")?;
    validate_id(&input.method_template_id, "method_template_id")?;
    validate_id(&input.method_revision_id, "method_revision_id")?;
    validate_id(&input.station_setup_id, "station_setup_id")?;
    validate_id(
        &input.station_setup_revision_id,
        "station_setup_revision_id",
    )?;
    if input.expected_schedule_revision == 0 {
        return Err(AgentError::with_details(
            "invalid_planned_test_preparation_request",
            "expected_schedule_revision must be at least 1",
            json!({ "field": "expected_schedule_revision" }),
        ));
    }
    if let Some(revision_id) = input.expected_current_revision_id.as_deref() {
        validate_id(revision_id, "expected_current_revision_id")?;
    }
    input.assignments.sort_by(|left, right| {
        left.slot_id
            .cmp(&right.slot_id)
            .then_with(|| left.binding_id.cmp(&right.binding_id))
    });
    let request_json = render_json(&AssessmentCommand {
        project_code: input.project_code.trim(),
        schedule_item_code: input.schedule_item_code.trim(),
        expected_schedule_revision: input.expected_schedule_revision,
        expected_current_revision_id: input.expected_current_revision_id.as_deref(),
        method_template_id: input.method_template_id.trim(),
        method_revision_id: input.method_revision_id.trim(),
        station_setup_id: input.station_setup_id.trim(),
        station_setup_revision_id: input.station_setup_revision_id.trim(),
        assignments: &input.assignments,
        reason: input.context.reason.trim(),
    });
    let request_checksum = sha256_text(&request_json);

    let mut projects = open_project_connection(storage_root)?;
    ensure_service_schedule_table(&projects)?;
    ensure_planned_test_preparation_tables(&projects)?;
    let project = load_project(&projects, &input.project_code)?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    let schedule = load_owned_schedule(&projects, &input.project_code, &input.schedule_item_code)?;
    ensure_schedule_is_preparable(&schedule)?;
    if let Some(revision) =
        load_planned_test_preparation_operation(&projects, &input.context.operation_id)?
    {
        ensure_assessment_replay(&revision, &input, &request_checksum)?;
        let schedule =
            load_owned_schedule(&projects, &input.project_code, &input.schedule_item_code)?;
        return operation_result(&projects, &schedule, &input.context.operation_id, true);
    }
    if existing_operation(&projects, &input.context.operation_id)?.is_some() {
        return Err(AgentError::with_details(
            "operation_replay_mismatch",
            "operation_id is already used by another operation",
            json!({ "operation_id": input.context.operation_id }),
        ));
    }

    if schedule.revision != input.expected_schedule_revision {
        return Err(AgentError::with_details(
            "planned_test_schedule_concurrent_update",
            "the scheduled test changed; refresh it before preparing the test",
            json!({
                "schedule_item_code": schedule.item_code,
                "expected_revision": input.expected_schedule_revision,
                "actual_revision": schedule.revision,
            }),
        ));
    }
    let identity = load_planned_test_preparation_identity(
        &projects,
        &input.project_code,
        &input.schedule_item_code,
    )?;
    let actual_current_revision_id = identity
        .as_ref()
        .and_then(|identity| identity.current_revision_id.as_deref());
    if actual_current_revision_id != input.expected_current_revision_id.as_deref() {
        return Err(AgentError::with_details(
            "planned_test_preparation_concurrent_update",
            "the preparation changed; refresh it before recording a new assessment",
            json!({
                "expected_current_revision_id": input.expected_current_revision_id,
                "actual_current_revision_id": actual_current_revision_id,
            }),
        ));
    }

    let method = load_selected_method(
        storage_root,
        &input.method_template_id,
        &input.method_revision_id,
        true,
    )?;
    let station_revision = load_selected_station(
        storage_root,
        &input.station_setup_id,
        &input.station_setup_revision_id,
    )?;
    let station = station_snapshot(
        storage_root,
        &station_revision,
        true,
        &schedule.planned_start_at,
        &project.execution_mode,
    )?;
    let schedule_snapshot = schedule_snapshot(&schedule, &project.execution_mode)?;
    let definition = assess_planned_test_preparation(PlannedTestPreparationAssessmentInput {
        schedule: schedule_snapshot,
        method,
        station_setup: station.snapshot,
        assignments: input.assignments.clone(),
        station_readiness: station.readiness,
    })
    .map_err(preparation_validation_error)?;
    let canonical = definition
        .canonicalize()
        .map_err(preparation_validation_error)?;

    let revision_number = next_planned_test_preparation_revision_number(
        &projects,
        &input.project_code,
        &input.schedule_item_code,
    )?;
    let revision_id = format!(
        "{}-prep-rev-{revision_number:04}",
        input.schedule_item_code.trim()
    );
    let timestamp = utc_timestamp()?;
    let parent_revision_id = actual_current_revision_id.map(str::to_owned);
    let recorded_state = if definition.verdict.ready {
        "ready"
    } else {
        "blocked"
    };
    let audit_sequence = next_audit_sequence(&projects, &input.project_code)?;
    let audit_payload = render_json(&json!({
        "schedule_item_code": input.schedule_item_code,
        "preparation_revision_id": revision_id,
        "parent_revision_id": parent_revision_id,
        "schedule_revision": schedule.revision,
        "method": {
            "template_id": definition.method.template_id,
            "revision_id": definition.method.revision_id,
            "definition_checksum": definition.method.definition_checksum,
        },
        "station_setup": {
            "setup_id": definition.station_setup.setup_id,
            "revision_id": definition.station_setup.revision_id,
            "definition_checksum": definition.station_setup.definition_checksum,
        },
        "verdict": definition.verdict,
        "definition_checksum": canonical.definition_checksum,
    }));
    let base_revision = parent_revision_id.as_deref().unwrap_or("none");
    let transaction = projects
        .transaction()
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    insert_planned_test_preparation_identity_if_missing(
        &transaction,
        NewPlannedTestPreparationIdentity {
            project_code: &input.project_code,
            schedule_item_code: &input.schedule_item_code,
            created_by: input.context.actor.trim(),
            timestamp: &timestamp,
        },
    )?;
    insert_planned_test_preparation_revision(
        &transaction,
        NewPlannedTestPreparationRevision {
            revision_id: &revision_id,
            project_code: &input.project_code,
            schedule_item_code: &input.schedule_item_code,
            revision_number,
            parent_revision_id: parent_revision_id.as_deref(),
            schedule_revision: schedule.revision,
            method_template_id: &definition.method.template_id,
            method_revision_id: &definition.method.revision_id,
            method_definition_checksum: &definition.method.definition_checksum,
            station_setup_id: &definition.station_setup.setup_id,
            station_setup_revision_id: &definition.station_setup.revision_id,
            station_setup_definition_checksum: &definition.station_setup.definition_checksum,
            verdict_state: recorded_state,
            definition_schema_version: &canonical.definition_schema_version,
            definition_json: &canonical.canonical_json,
            definition_checksum: &canonical.definition_checksum,
            operation_id: input.context.operation_id.trim(),
            request_checksum: &request_checksum,
            actor: input.context.actor.trim(),
            reason: input.context.reason.trim(),
            device_id: input.context.device_id.trim(),
            correlation_id: input.context.correlation_id.trim(),
            timestamp: &timestamp,
        },
    )?;
    update_current_planned_test_preparation_revision(
        &transaction,
        &input.project_code,
        &input.schedule_item_code,
        input.expected_current_revision_id.as_deref(),
        &revision_id,
        &timestamp,
    )?;
    insert_audit_event(
        &transaction,
        AuditEventInput {
            project_code: &input.project_code,
            sequence: audit_sequence,
            actor: input.context.actor.trim(),
            action: PREPARATION_OPERATION,
            reason: Some(input.context.reason.trim()),
            payload_json: &audit_payload,
            timestamp: &timestamp,
        },
    )?;
    insert_sync_operation(
        &transaction,
        SyncOperationInput {
            domain: "project_records",
            entity_type: "planned_test_preparation",
            operation_id: input.context.operation_id.trim(),
            entity_id: &input.schedule_item_code,
            operation_kind: PREPARATION_OPERATION,
            base_revision,
            resulting_revision: &revision_id,
            actor_id: input.context.actor.trim(),
            device_id: input.context.device_id.trim(),
            correlation_id: input.context.correlation_id.trim(),
            payload_json: &request_json,
            timestamp: &timestamp,
        },
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    operation_result(&projects, &schedule, &input.context.operation_id, false)
}

pub(crate) fn require_planned_test_preparation_for_start(
    storage_root: &Path,
    projects: &rusqlite::Connection,
    project_code: &str,
    schedule_item_code: &str,
    schedule_revision: u64,
    expected_preparation_revision_id: &str,
    expected_preparation_checksum: &str,
) -> Result<PlannedTestPreparationStartEvidence, AgentError> {
    ensure_planned_test_preparation_tables(projects)?;
    let project = load_project(projects, project_code)?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    let schedule = load_owned_schedule(projects, project_code, schedule_item_code)?;
    if schedule.revision != schedule_revision {
        return Err(AgentError::new(
            "planned_test_preparation_stale",
            "the scheduled test changed after the start request was prepared",
        ));
    }
    let stored =
        load_current_planned_test_preparation_revision(projects, project_code, schedule_item_code)?
            .ok_or_else(|| {
                AgentError::with_details(
                    "planned_test_preparation_required",
                    "prepare this scheduled test before starting it",
                    json!({ "schedule_item_code": schedule_item_code }),
                )
            })?;
    if stored.revision_id != expected_preparation_revision_id
        || stored.definition_checksum != expected_preparation_checksum
    {
        return Err(AgentError::with_details(
            "planned_test_preparation_changed_before_start",
            "La préparation de l'essai a changé pendant le démarrage. Vérifiez-la de nouveau.",
            json!({
                "schedule_item_code": schedule_item_code,
                "expected_preparation_revision_id": expected_preparation_revision_id,
                "actual_preparation_revision_id": &stored.revision_id,
                "expected_preparation_checksum": expected_preparation_checksum,
                "actual_preparation_checksum": &stored.definition_checksum,
            }),
        ));
    }
    let definition = validated_stored_definition(&stored)?;
    if definition.schedule.revision != schedule_revision {
        return Err(AgentError::with_details(
            "planned_test_preparation_stale",
            "the test preparation no longer matches the scheduled test",
            json!({
                "preparation_revision_id": stored.revision_id,
                "prepared_schedule_revision": definition.schedule.revision,
                "current_schedule_revision": schedule_revision,
            }),
        ));
    }
    if !definition.verdict.ready {
        return Err(not_ready_error(&stored.revision_id, &definition));
    }

    let method = load_selected_method(
        storage_root,
        &definition.method.template_id,
        &definition.method.revision_id,
        false,
    )?;
    if method.definition_checksum != definition.method.definition_checksum {
        return Err(AgentError::new(
            "planned_test_method_reference_changed",
            "the prepared method content no longer matches its frozen reference",
        ));
    }
    let station_revision = load_selected_station(
        storage_root,
        &definition.station_setup.setup_id,
        &definition.station_setup.revision_id,
    )?;
    let station = station_snapshot(
        storage_root,
        &station_revision,
        false,
        &schedule.planned_start_at,
        &project.execution_mode,
    )?;
    if station.snapshot.definition_checksum != definition.station_setup.definition_checksum {
        return Err(AgentError::new(
            "planned_test_station_reference_changed",
            "the prepared station setup no longer matches its frozen reference",
        ));
    }
    let current_definition =
        assess_planned_test_preparation(PlannedTestPreparationAssessmentInput {
            schedule: schedule_snapshot(&schedule, &project.execution_mode)?,
            method,
            station_setup: station.snapshot,
            assignments: definition.assignments.clone(),
            station_readiness: station.readiness,
        })
        .map_err(preparation_validation_error)?;
    if !current_definition.verdict.ready {
        return Err(not_ready_error(&stored.revision_id, &current_definition));
    }
    if !current_definition.permits_start(schedule_revision, ServiceScheduleStatus::Confirmed) {
        return Err(AgentError::new(
            "planned_test_not_confirmed",
            "the scheduled test must be confirmed before it can start",
        ));
    }
    Ok(PlannedTestPreparationStartEvidence {
        revision_id: stored.revision_id,
        definition_checksum: stored.definition_checksum,
    })
}

struct LoadedStationSnapshot {
    snapshot: PreparedStationSetupSnapshot,
    readiness: StationSetupReadiness,
}

fn load_selected_method(
    storage_root: &Path,
    template_id: &str,
    revision_id: &str,
    require_approved: bool,
) -> Result<PreparedTestMethodSnapshot, AgentError> {
    let connection = open_test_template_connection(storage_root)?;
    let revision =
        load_test_template_revision(&connection, template_id, revision_id)?.ok_or_else(|| {
            AgentError::new(
                "planned_test_method_not_found",
                "the selected test method revision does not exist",
            )
        })?;
    method_snapshot(&revision, require_approved)
}

fn method_snapshot(
    revision: &StoredTestTemplateRevision,
    require_approved: bool,
) -> Result<PreparedTestMethodSnapshot, AgentError> {
    let status = parse_template_status(&revision.status)?;
    let allowed = if require_approved {
        status == TemplateRevisionStatus::Approved
    } else {
        matches!(
            status,
            TemplateRevisionStatus::Approved | TemplateRevisionStatus::Superseded
        )
    };
    if !allowed {
        return Err(AgentError::with_details(
            "planned_test_method_not_approved",
            "only an approved test method can be selected for preparation",
            json!({ "revision_id": revision.revision_id, "status": revision.status }),
        ));
    }
    let definition =
        TestTemplateDefinition::from_json_str(&revision.definition_json).map_err(|error| {
            AgentError::with_details(
                "planned_test_method_storage_invalid",
                "the stored test method definition is invalid",
                json!({ "code": error.code, "message": error.message }),
            )
        })?;
    let canonical = definition.canonicalize().map_err(|error| {
        AgentError::with_details(
            "planned_test_method_storage_invalid",
            "the stored test method definition is invalid",
            json!({ "code": error.code, "message": error.message }),
        )
    })?;
    if canonical.definition_checksum != revision.definition_checksum
        || canonical.definition_schema_version != revision.definition_schema_version
    {
        return Err(AgentError::new(
            "planned_test_method_storage_invalid",
            "the stored test method checksum does not match its content",
        ));
    }
    Ok(PreparedTestMethodSnapshot {
        template_id: revision.template_id.clone(),
        revision_id: revision.revision_id.clone(),
        revision_number: revision.revision_number,
        revision_status: status,
        definition_checksum: revision.definition_checksum.clone(),
        title: definition.title,
        measurement_axis: definition.measurement_axis,
        method_code: definition.method_code,
        method_revision: definition.method_revision,
        standard_references: definition.standard_references,
        instrumentation_chain: definition.instrumentation_chain,
    })
}

fn load_selected_station(
    storage_root: &Path,
    setup_id: &str,
    revision_id: &str,
) -> Result<StoredStationSetupRevision, AgentError> {
    let connection = open_station_connection(storage_root)?;
    load_station_setup_revision(&connection, revision_id)?
        .filter(|revision| revision.setup_id == setup_id)
        .ok_or_else(|| {
            AgentError::new(
                "planned_test_station_setup_not_found",
                "the selected station setup revision does not exist",
            )
        })
}

fn station_snapshot(
    storage_root: &Path,
    revision: &StoredStationSetupRevision,
    require_ready: bool,
    scheduled_start_at: &str,
    execution_mode: &str,
) -> Result<LoadedStationSnapshot, AgentError> {
    let status = parse_station_status(&revision.status)?;
    let allowed = if require_ready {
        status == StationSetupRevisionStatus::Ready
    } else {
        matches!(
            status,
            StationSetupRevisionStatus::Ready | StationSetupRevisionStatus::Superseded
        )
    };
    if !allowed {
        return Err(AgentError::with_details(
            "planned_test_station_setup_not_ready",
            "only a ready station setup can be selected for preparation",
            json!({ "revision_id": revision.revision_id, "status": revision.status }),
        ));
    }
    let definition = StationMeasurementSetupDefinition::from_json_str(&revision.definition_json)
        .map_err(|issue| {
            AgentError::with_details(
                "planned_test_station_storage_invalid",
                "the stored station setup definition is invalid",
                json!({ "code": issue.code, "path": issue.path, "message": issue.message }),
            )
        })?;
    let canonical = definition.canonicalize().map_err(|validation| {
        AgentError::with_details(
            "planned_test_station_storage_invalid",
            "the stored station setup definition is invalid",
            json!({ "validation": validation }),
        )
    })?;
    if canonical.definition_checksum != revision.definition_checksum
        || canonical.definition_schema_version != revision.definition_schema_version
    {
        return Err(AgentError::new(
            "planned_test_station_storage_invalid",
            "the stored station setup checksum does not match its content",
        ));
    }

    let metrology = open_metrology_connection(storage_root)?;
    let equipment = open_equipment_connection(storage_root)?;
    let mut assets = Vec::new();
    for binding in &definition.asset_bindings {
        let instrument = load_instrument(&metrology, &binding.asset_id)?;
        let model_revision = load_equipment_model_revision(
            &equipment,
            &binding.equipment_model_id,
            &binding.equipment_model_revision_id,
        )?;
        let model = model_revision
            .as_ref()
            .and_then(|stored| {
                EquipmentModelDefinition::from_json_str(&stored.definition_json).ok()
            })
            .and_then(|definition| {
                definition
                    .canonicalize()
                    .ok()
                    .filter(|canonical| {
                        canonical.definition_checksum == binding.equipment_model_checksum
                    })
                    .map(|_| definition)
            });
        let capabilities = model
            .as_ref()
            .map(|model| {
                model
                    .capabilities
                    .iter()
                    .map(|capability| PreparedEquipmentCapabilitySnapshot {
                        capability_id: capability.capability_id.clone(),
                        label: capability.label.clone(),
                        capability_kind: capability.capability_kind.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        assets.push(PreparedStationAssetSnapshot {
            binding_id: binding.binding_id.clone(),
            role_label: binding.role_label.clone(),
            asset_id: binding.asset_id.clone(),
            asset_revision: binding.asset_revision.clone(),
            inventory_code: binding.asset_id.clone(),
            serial_number: instrument
                .as_ref()
                .map(|instrument| instrument.serial_number.clone())
                .unwrap_or_else(|| "Non disponible".to_owned()),
            manufacturer: instrument
                .as_ref()
                .map(|instrument| instrument.manufacturer.clone())
                .unwrap_or_else(|| "Non disponible".to_owned()),
            model_name: instrument
                .as_ref()
                .map(|instrument| instrument.model.clone())
                .unwrap_or_else(|| "Non disponible".to_owned()),
            equipment_model_id: binding.equipment_model_id.clone(),
            equipment_model_revision_id: binding.equipment_model_revision_id.clone(),
            equipment_model_checksum: binding.equipment_model_checksum.clone(),
            category_code: model
                .as_ref()
                .map(|model| model.category_code.clone())
                .unwrap_or_else(|| "indisponible".to_owned()),
            capabilities,
        });
    }
    let corrections = definition
        .correction_selections
        .iter()
        .map(|selection| PreparedStationCorrectionSnapshot {
            selection_id: selection.selection_id.clone(),
            binding_id: selection.binding_id.clone(),
            correction_kind: selection.correction_kind,
            characterization_id: selection.characterization_id.clone(),
            characterization_checksum: selection.characterization_checksum.clone(),
            label: selection.label.clone(),
        })
        .collect();
    let snapshot = PreparedStationSetupSnapshot {
        setup_id: revision.setup_id.clone(),
        revision_id: revision.revision_id.clone(),
        revision_number: revision.revision_number,
        revision_status: status,
        definition_checksum: revision.definition_checksum.clone(),
        label: definition.label.clone(),
        laboratory_location_id: definition.laboratory_location_id.clone(),
        laboratory_location_label: definition.laboratory_location_label.clone(),
        planned_use_on: definition.planned_use_on.clone(),
        execution_mode: definition.execution_mode.clone(),
        assets,
        corrections,
    };
    let mut contextual_definition = definition;
    contextual_definition.planned_use_on = scheduled_date(scheduled_start_at)?.to_owned();
    contextual_definition.execution_mode = execution_mode.to_owned();
    let readiness = assess_station_setup_readiness(storage_root, &contextual_definition)?;
    Ok(LoadedStationSnapshot {
        snapshot,
        readiness,
    })
}

fn schedule_snapshot(
    schedule: &StoredServiceScheduleItem,
    execution_mode: &str,
) -> Result<PlannedTestScheduleSnapshot, AgentError> {
    Ok(PlannedTestScheduleSnapshot {
        project_code: schedule.project_code.clone(),
        item_code: schedule.item_code.clone(),
        revision: schedule.revision,
        title: schedule.title.clone(),
        planned_start_at: schedule.planned_start_at.clone(),
        planned_end_at: schedule.planned_end_at.clone(),
        assigned_operator: schedule.assigned_operator.clone(),
        laboratory_location_id: schedule.laboratory_location_id.clone(),
        laboratory_location_label: schedule.laboratory_location_label.clone(),
        equipment_under_test: schedule.equipment_under_test.clone(),
        execution_mode: execution_mode.to_owned(),
        status: schedule.to_domain()?.status(),
    })
}

fn load_aggregate(
    connection: &rusqlite::Connection,
    schedule: &StoredServiceScheduleItem,
) -> Result<PlannedTestPreparationAggregateDto, AgentError> {
    let identity = load_planned_test_preparation_identity(
        connection,
        &schedule.project_code,
        &schedule.item_code,
    )?;
    let revisions = list_planned_test_preparation_revisions(
        connection,
        &schedule.project_code,
        &schedule.item_code,
    )?;
    let current_revision_id = identity
        .as_ref()
        .and_then(|identity| identity.current_revision_id.as_deref());
    let current_status = schedule.to_domain()?.status();
    let current_revision = match current_revision_id {
        Some(revision_id) => {
            let stored = revisions
                .iter()
                .find(|revision| revision.revision_id == revision_id)
                .ok_or_else(|| {
                    AgentError::new(
                        "planned_test_preparation_storage_invalid",
                        "the preparation identity references a missing revision",
                    )
                })?;
            Some(stored_revision_dto(
                stored,
                current_revision_id,
                schedule.revision,
                current_status,
            )?)
        }
        None => None,
    };
    let current_state = current_revision
        .as_ref()
        .map(|revision| revision.effective_state.clone())
        .unwrap_or_else(|| "missing".to_owned());
    let can_start = current_revision
        .as_ref()
        .is_some_and(|revision| revision.effective_state == "ready")
        && current_status == ServiceScheduleStatus::Confirmed;
    Ok(PlannedTestPreparationAggregateDto {
        project_code: schedule.project_code.clone(),
        schedule_item_code: schedule.item_code.clone(),
        current_state,
        can_start,
        current_revision,
        revision_count: revisions.len(),
    })
}

fn stored_revision_dto(
    stored: &StoredPlannedTestPreparationRevision,
    current_revision_id: Option<&str>,
    current_schedule_revision: u64,
    current_status: ServiceScheduleStatus,
) -> Result<PlannedTestPreparationRevisionDto, AgentError> {
    let definition = validated_stored_definition(stored)?;
    let is_current = current_revision_id == Some(stored.revision_id.as_str());
    let effective_state = if is_current && current_status == ServiceScheduleStatus::Planned {
        "inapplicable".to_owned()
    } else if is_current {
        preparation_state_text(definition.effective_state(current_schedule_revision)).to_owned()
    } else {
        "historical".to_owned()
    };
    let _can_start =
        is_current && definition.permits_start(current_schedule_revision, current_status);
    Ok(PlannedTestPreparationRevisionDto {
        revision_id: stored.revision_id.clone(),
        revision_number: stored.revision_number,
        parent_revision_id: stored.parent_revision_id.clone(),
        recorded_state: stored.verdict_state.clone(),
        effective_state,
        is_current,
        definition,
        definition_checksum: stored.definition_checksum.clone(),
        actor: stored.actor.clone(),
        reason: stored.reason.clone(),
        operation_id: stored.operation_id.clone(),
        device_id: stored.device_id.clone(),
        correlation_id: stored.correlation_id.clone(),
        created_at: stored.created_at.clone(),
    })
}

fn validated_stored_definition(
    stored: &StoredPlannedTestPreparationRevision,
) -> Result<PlannedTestPreparationDefinition, AgentError> {
    let definition = PlannedTestPreparationDefinition::from_json_str(&stored.definition_json)
        .map_err(|issue| {
            AgentError::with_details(
                "planned_test_preparation_storage_invalid",
                "the stored preparation definition is invalid",
                json!({ "code": issue.code, "path": issue.path, "message": issue.message }),
            )
        })?;
    let canonical = definition.canonicalize().map_err(|validation| {
        AgentError::with_details(
            "planned_test_preparation_storage_invalid",
            "the stored preparation definition is invalid",
            json!({ "validation": validation }),
        )
    })?;
    let expected_state = if definition.verdict.ready {
        "ready"
    } else {
        "blocked"
    };
    if canonical.definition_checksum != stored.definition_checksum
        || canonical.definition_schema_version != stored.definition_schema_version
        || definition.schedule.project_code != stored.project_code
        || definition.schedule.item_code != stored.schedule_item_code
        || definition.schedule.revision != stored.schedule_revision
        || definition.method.template_id != stored.method_template_id
        || definition.method.revision_id != stored.method_revision_id
        || definition.method.definition_checksum != stored.method_definition_checksum
        || definition.station_setup.setup_id != stored.station_setup_id
        || definition.station_setup.revision_id != stored.station_setup_revision_id
        || definition.station_setup.definition_checksum != stored.station_setup_definition_checksum
        || expected_state != stored.verdict_state
    {
        return Err(AgentError::new(
            "planned_test_preparation_storage_invalid",
            "the stored preparation metadata does not match its canonical definition",
        ));
    }
    Ok(definition)
}

fn load_owned_schedule(
    connection: &rusqlite::Connection,
    project_code: &str,
    schedule_item_code: &str,
) -> Result<StoredServiceScheduleItem, AgentError> {
    load_project(connection, project_code)?
        .ok_or_else(|| AgentError::new("project_not_found", "project does not exist"))?;
    let schedule =
        load_service_schedule_item(connection, schedule_item_code)?.ok_or_else(|| {
            AgentError::new(
                "service_schedule_item_not_found",
                "scheduled test does not exist",
            )
        })?;
    if schedule.project_code != project_code {
        return Err(AgentError::new(
            "service_schedule_item_not_found",
            "scheduled test does not belong to this project",
        ));
    }
    Ok(schedule)
}

fn ensure_schedule_is_preparable(schedule: &StoredServiceScheduleItem) -> Result<(), AgentError> {
    let status = schedule.to_domain()?.status();
    match status {
        ServiceScheduleStatus::Confirmed => Ok(()),
        ServiceScheduleStatus::Planned => Err(AgentError::with_details(
            "planned_test_schedule_not_confirmed",
            "Confirmez le créneau avant de préparer l'essai.",
            json!({ "status": status.as_str() }),
        )),
        _ => Err(AgentError::with_details(
            "planned_test_schedule_not_preparable",
            "only a confirmed scheduled test can be prepared",
            json!({ "status": status.as_str() }),
        )),
    }
}

fn operation_result(
    connection: &rusqlite::Connection,
    schedule: &StoredServiceScheduleItem,
    operation_id: &str,
    replayed: bool,
) -> Result<String, AgentError> {
    Ok(render_json(&PlannedTestPreparationOperationResultDto {
        operation: PREPARATION_OPERATION.to_owned(),
        operation_id: operation_id.to_owned(),
        replayed,
        preparation: load_aggregate(connection, schedule)?,
    }))
}

fn ensure_assessment_replay(
    stored: &StoredPlannedTestPreparationRevision,
    input: &AssessPlannedTestPreparationInput,
    request_checksum: &str,
) -> Result<(), AgentError> {
    if stored.project_code == input.project_code.trim()
        && stored.schedule_item_code == input.schedule_item_code.trim()
        && stored.request_checksum == request_checksum
        && stored.actor == input.context.actor.trim()
        && stored.device_id == input.context.device_id.trim()
        && stored.correlation_id == input.context.correlation_id.trim()
    {
        return Ok(());
    }
    Err(AgentError::with_details(
        "operation_replay_mismatch",
        "operation_id is already used for a different preparation assessment",
        json!({
            "operation_id": input.context.operation_id,
            "expected_request_checksum": request_checksum,
            "stored_request_checksum": stored.request_checksum,
        }),
    ))
}

fn not_ready_error(revision_id: &str, definition: &PlannedTestPreparationDefinition) -> AgentError {
    AgentError::with_details(
        "planned_test_preparation_not_ready",
        "the scheduled test cannot start because its preparation is blocked",
        json!({
            "preparation_revision_id": revision_id,
            "checked_on": definition.verdict.checked_on,
            "issues": definition.verdict.issues,
        }),
    )
}

fn validate_context(context: &PlannedTestPreparationOperationContext) -> Result<(), AgentError> {
    AuditActor::parse(context.actor.clone()).map_err(domain_error)?;
    AuditReason::parse(context.reason.clone()).map_err(domain_error)?;
    validate_id(&context.operation_id, "operation_id")?;
    validate_id(&context.device_id, "device_id")?;
    validate_id(&context.correlation_id, "correlation_id")?;
    Ok(())
}

fn validate_id(value: &str, field: &str) -> Result<(), AgentError> {
    StableId::parse(value.to_owned()).map_err(|error| {
        AgentError::with_details(
            "invalid_planned_test_preparation_request",
            format!("{field} must be a stable identifier"),
            json!({ "field": field, "cause": format!("{error:?}") }),
        )
    })?;
    Ok(())
}

fn scheduled_date(value: &str) -> Result<&str, AgentError> {
    value
        .get(0..10)
        .filter(|date| date.len() == 10)
        .ok_or_else(|| {
            AgentError::new(
                "service_schedule_storage_invalid",
                "the scheduled test does not expose a valid local date",
            )
        })
}

fn parse_template_status(value: &str) -> Result<TemplateRevisionStatus, AgentError> {
    match value {
        "draft" => Ok(TemplateRevisionStatus::Draft),
        "under_review" => Ok(TemplateRevisionStatus::UnderReview),
        "approved" => Ok(TemplateRevisionStatus::Approved),
        "suspended" => Ok(TemplateRevisionStatus::Suspended),
        "superseded" => Ok(TemplateRevisionStatus::Superseded),
        "retired" => Ok(TemplateRevisionStatus::Retired),
        _ => Err(AgentError::new(
            "planned_test_method_storage_invalid",
            "the stored test method status is invalid",
        )),
    }
}

fn parse_station_status(value: &str) -> Result<StationSetupRevisionStatus, AgentError> {
    match value {
        "draft" => Ok(StationSetupRevisionStatus::Draft),
        "ready" => Ok(StationSetupRevisionStatus::Ready),
        "superseded" => Ok(StationSetupRevisionStatus::Superseded),
        _ => Err(AgentError::new(
            "planned_test_station_storage_invalid",
            "the stored station setup status is invalid",
        )),
    }
}

fn preparation_state_text(state: PlannedTestPreparationState) -> &'static str {
    match state {
        PlannedTestPreparationState::Blocked => "blocked",
        PlannedTestPreparationState::Ready => "ready",
        PlannedTestPreparationState::Stale => "stale",
    }
}

fn preparation_validation_error(
    validation: Vec<emc_locus_core::PlannedTestPreparationValidationIssue>,
) -> AgentError {
    AgentError::with_details(
        "invalid_planned_test_preparation",
        "the planned test preparation is structurally invalid",
        json!({ "validation": validation }),
    )
}

fn domain_error(error: emc_locus_core::DomainError) -> AgentError {
    AgentError::new(
        "invalid_planned_test_preparation_request",
        format!("invalid planned test preparation command: {error:?}"),
    )
}

fn utc_timestamp() -> Result<String, AgentError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AgentError::new("timestamp_error", error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrology_service::{
        register_metrology_instrument, MetrologyOperationContext, RegisterInstrumentInput,
    };
    use crate::service_schedule_service::{
        set_start_consistency_test_action, transition_service_schedule_item,
        StartConsistencyTestAction, TransitionServiceScheduleItemInput,
    };
    use crate::{run_storage_action, StorageAction};
    use emc_locus_core::equipment::{
        EquipmentClass, FunctionalRole, PhysicalQuantity, PortDirectionality, PortFlowRole,
        SignalDomain, SignalPortDefinition, TechnologyTag,
        EQUIPMENT_MODEL_DEFINITION_SCHEMA_VERSION,
    };
    use emc_locus_core::test_definitions::{
        CalibrationRequirement, ExecutionSequenceStep, ExecutionStepKind,
        InstrumentSubstitutionPolicy, InstrumentationChainSlot, MeasurementAxis,
        VariableConstraints, VariableDefaultValue, VariableDefinition, VariableLockPolicy,
        VariableLockPolicyKind, VariableValueType, TEST_TEMPLATE_DEFINITION_SCHEMA_VERSION,
    };
    use emc_locus_core::{
        StationAssetBindingDefinition, StationConnectionDefinition, StationPortEndpoint,
        STATION_SETUP_DEFINITION_SCHEMA_VERSION,
    };
    use rusqlite::{params, Connection};
    use serde_json::Value;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    const PROJECT_CODE: &str = "CEM-PREP-001";
    const ITEM_CODE: &str = "PLAN-PREP-001";
    const METHOD_ID: &str = "METHOD-PREP-001";
    const METHOD_REVISION_ID: &str = "METHOD-PREP-001-rev-0001";
    const SETUP_ID: &str = "SETUP-PREP-001";
    const SETUP_REVISION_ID: &str = "SETUP-PREP-001-rev-0001";
    const MODEL_ID: &str = "MODEL-PREP-001";
    const MODEL_REVISION_ID: &str = "MODEL-PREP-001-rev-0001";
    const ASSET_ID: &str = "ASSET-PREP-001";
    const SOURCE_ASSET_ID: &str = "ASSET-PREP-SOURCE-001";
    const BINDING_ID: &str = "receiver-binding";
    const SOURCE_BINDING_ID: &str = "source-binding";
    const SLOT_ID: &str = "receiver";

    #[test]
    fn blocked_then_ready_assessment_controls_start_and_preserves_evidence() {
        let storage_root = prepared_storage("planned-test-preparation-flow");

        let blocked = assess_planned_test_preparation_for_schedule(
            &storage_root,
            assessment_input("op-prep-blocked", None, Vec::new()),
        )
        .unwrap();
        let blocked: Value = serde_json::from_str(&blocked).unwrap();
        assert_eq!(
            blocked["preparation"]["current_state"],
            Value::String("blocked".to_owned())
        );
        assert_eq!(blocked["preparation"]["revision_count"], 1);
        assert_eq!(
            blocked["preparation"]["current_revision"]["definition"]["verdict"]["issues"][0]
                ["code"],
            Value::String("planned_test_required_role_unassigned".to_owned())
        );

        let start_error = transition_service_schedule_item(
            &storage_root,
            start_input(
                "op-start-blocked",
                1,
                blocked["preparation"]["current_revision"]["revision_id"]
                    .as_str()
                    .unwrap(),
                blocked["preparation"]["current_revision"]["definition_checksum"]
                    .as_str()
                    .unwrap(),
            ),
        )
        .unwrap_err();
        assert_eq!(start_error.code, "planned_test_preparation_not_ready");

        let ready_input = assessment_input(
            "op-prep-ready",
            Some("PLAN-PREP-001-prep-rev-0001"),
            vec![PlannedTestInstrumentAssignment {
                slot_id: SLOT_ID.to_owned(),
                binding_id: BINDING_ID.to_owned(),
            }],
        );
        let ready =
            assess_planned_test_preparation_for_schedule(&storage_root, ready_input.clone())
                .unwrap();
        let ready: Value = serde_json::from_str(&ready).unwrap();
        assert_eq!(
            ready["preparation"]["current_state"],
            Value::String("ready".to_owned())
        );
        assert_eq!(ready["preparation"]["revision_count"], 2);
        assert_eq!(ready["preparation"]["can_start"], true);

        let replay =
            assess_planned_test_preparation_for_schedule(&storage_root, ready_input).unwrap();
        let replay: Value = serde_json::from_str(&replay).unwrap();
        assert_eq!(replay["replayed"], true);
        assert_eq!(replay["preparation"]["revision_count"], 2);

        let revisions =
            list_planned_test_preparation_revisions_json(&storage_root, PROJECT_CODE, ITEM_CODE)
                .unwrap();
        let revisions: Value = serde_json::from_str(&revisions).unwrap();
        assert_eq!(revisions["revisions"][0]["recorded_state"], "ready");
        assert_eq!(revisions["revisions"][1]["recorded_state"], "blocked");
        assert_eq!(revisions["revisions"][1]["effective_state"], "historical");

        let ready_revision_id = ready["preparation"]["current_revision"]["revision_id"]
            .as_str()
            .unwrap();
        let ready_checksum = ready["preparation"]["current_revision"]["definition_checksum"]
            .as_str()
            .unwrap();

        set_start_consistency_test_action(StartConsistencyTestAction::InstallBlockedPreparation {
            source_revision_id: "PLAN-PREP-001-prep-rev-0001".to_owned(),
            new_revision_id: "PLAN-PREP-001-prep-rev-race".to_owned(),
        });
        let preparation_race = transition_service_schedule_item(
            &storage_root,
            start_input(
                "op-start-preparation-race",
                1,
                ready_revision_id,
                ready_checksum,
            ),
        )
        .unwrap_err();
        assert_eq!(
            preparation_race.code,
            "planned_test_preparation_changed_before_start"
        );

        set_start_consistency_test_action(StartConsistencyTestAction::PointToPreparation {
            revision_id: "PLAN-PREP-001-prep-rev-0001".to_owned(),
        });
        let pointer_race = transition_service_schedule_item(
            &storage_root,
            start_input(
                "op-start-pointer-race",
                1,
                ready_revision_id,
                ready_checksum,
            ),
        )
        .unwrap_err();
        assert_eq!(
            pointer_race.code,
            "planned_test_preparation_changed_before_start"
        );

        set_start_consistency_test_action(StartConsistencyTestAction::AdvanceScheduleRevision);
        let schedule_race = transition_service_schedule_item(
            &storage_root,
            start_input(
                "op-start-schedule-race",
                1,
                ready_revision_id,
                ready_checksum,
            ),
        )
        .unwrap_err();
        assert_eq!(schedule_race.code, "service_schedule_concurrent_update");

        let projects = Connection::open(storage_root.join("projects.sqlite")).unwrap();
        let unchanged: (String, u64, String) = projects
            .query_row(
                concat!(
                    "SELECT s.status, s.revision, i.current_revision_id ",
                    "FROM service_schedule_items s ",
                    "JOIN planned_test_preparation_identities i ON i.schedule_item_code = s.item_code ",
                    "WHERE s.item_code = ?1"
                ),
                params![ITEM_CODE],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            unchanged,
            ("confirmed".to_owned(), 1, ready_revision_id.to_owned())
        );
        let race_revision_count: u64 = projects
            .query_row(
                "SELECT COUNT(*) FROM planned_test_preparation_revisions WHERE revision_id = 'PLAN-PREP-001-prep-rev-race'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(race_revision_count, 0);
        drop(projects);

        let start_input = start_input("op-start-ready", 1, ready_revision_id, ready_checksum);
        set_start_consistency_test_action(StartConsistencyTestAction::AssertReadinessWritersLocked);
        let started = transition_service_schedule_item(&storage_root, start_input.clone()).unwrap();
        let started: Value = serde_json::from_str(&started).unwrap();
        assert_eq!(started["schedule_item"]["status"], "in_progress");

        let replayed =
            transition_service_schedule_item(&storage_root, start_input.clone()).unwrap();
        let replayed: Value = serde_json::from_str(&replayed).unwrap();
        assert_eq!(replayed["replayed"], true);
        let mut mismatched = start_input;
        mismatched.expected_preparation_checksum = Some(format!("sha256:{}", "f".repeat(64)));
        let mismatch = transition_service_schedule_item(&storage_root, mismatched).unwrap_err();
        assert_eq!(mismatch.code, "operation_replay_mismatch");

        let projects = Connection::open(storage_root.join("projects.sqlite")).unwrap();
        let preparation_audit: u64 = projects
            .query_row(
                "SELECT COUNT(*) FROM project_audit_events WHERE project_code = ?1 AND action = ?2",
                params![PROJECT_CODE, PREPARATION_OPERATION],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(preparation_audit, 2);
        let start_payload: String = projects
            .query_row(
                "SELECT payload_json FROM project_audit_events WHERE project_code = ?1 AND action = 'service_schedule_item_status_changed' ORDER BY sequence DESC LIMIT 1",
                [PROJECT_CODE],
                |row| row.get(0),
            )
            .unwrap();
        let start_payload: Value = serde_json::from_str(&start_payload).unwrap();
        assert_eq!(
            start_payload["planned_test_preparation"]["revision_id"],
            "PLAN-PREP-001-prep-rev-0002"
        );
        assert_eq!(
            start_payload["planned_test_preparation"]["definition_checksum"],
            ready_checksum
        );
        drop(projects);

        let sync = Connection::open(storage_root.join("sync.sqlite")).unwrap();
        let preparation_outbox: u64 = sync
            .query_row(
                "SELECT COUNT(*) FROM sync_operations WHERE entity_type = 'planned_test_preparation'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(preparation_outbox, 2);
        let start_outbox_payload: String = sync
            .query_row(
                "SELECT payload_json FROM sync_operations WHERE operation_id = 'op-start-ready'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let start_outbox_payload: Value = serde_json::from_str(&start_outbox_payload).unwrap();
        assert_eq!(
            start_outbox_payload["expected_preparation_revision_id"],
            ready_revision_id
        );
        assert_eq!(
            start_outbox_payload["expected_preparation_checksum"],
            ready_checksum
        );
        drop(sync);

        let historical = get_planned_test_preparation_revision_json(
            &storage_root,
            PROJECT_CODE,
            ITEM_CODE,
            "PLAN-PREP-001-prep-rev-0001",
        )
        .unwrap();
        let historical: Value = serde_json::from_str(&historical).unwrap();
        assert_eq!(historical["revision"]["recorded_state"], "blocked");

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn stale_schedule_and_concurrent_preparation_are_rejected_explicitly() {
        let storage_root = prepared_storage("planned-test-preparation-concurrency");
        assess_planned_test_preparation_for_schedule(
            &storage_root,
            assessment_input("op-prep-first", None, Vec::new()),
        )
        .unwrap();

        let concurrent = assess_planned_test_preparation_for_schedule(
            &storage_root,
            assessment_input("op-prep-concurrent", None, Vec::new()),
        )
        .unwrap_err();
        assert_eq!(
            concurrent.code,
            "planned_test_preparation_concurrent_update"
        );

        let projects = Connection::open(storage_root.join("projects.sqlite")).unwrap();
        projects
            .execute(
                "UPDATE service_schedule_items SET revision = 2 WHERE item_code = ?1",
                [ITEM_CODE],
            )
            .unwrap();
        drop(projects);
        let stale = assess_planned_test_preparation_for_schedule(
            &storage_root,
            assessment_input(
                "op-prep-stale",
                Some("PLAN-PREP-001-prep-rev-0001"),
                Vec::new(),
            ),
        )
        .unwrap_err();
        assert_eq!(stale.code, "planned_test_schedule_concurrent_update");

        let aggregate =
            get_planned_test_preparation(&storage_root, PROJECT_CODE, ITEM_CODE).unwrap();
        let aggregate: Value = serde_json::from_str(&aggregate).unwrap();
        assert_eq!(aggregate["preparation"]["current_state"], "stale");

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn planned_schedule_requires_confirmation_and_keeps_prior_evidence_inapplicable() {
        let storage_root = prepared_storage("planned-test-confirmation-gate");
        assess_planned_test_preparation_for_schedule(
            &storage_root,
            assessment_input("op-prep-before-planned", None, Vec::new()),
        )
        .unwrap();

        let projects = Connection::open(storage_root.join("projects.sqlite")).unwrap();
        projects
            .execute(
                "UPDATE service_schedule_items SET status = 'planned' WHERE item_code = ?1",
                [ITEM_CODE],
            )
            .unwrap();
        let audit_count_before: u64 = projects
            .query_row(
                "SELECT COUNT(*) FROM project_audit_events WHERE project_code = ?1",
                [PROJECT_CODE],
                |row| row.get(0),
            )
            .unwrap();
        drop(projects);

        let aggregate =
            get_planned_test_preparation(&storage_root, PROJECT_CODE, ITEM_CODE).unwrap();
        let aggregate: Value = serde_json::from_str(&aggregate).unwrap();
        assert_eq!(aggregate["preparation"]["current_state"], "inapplicable");
        assert_eq!(aggregate["preparation"]["can_start"], false);
        assert_eq!(aggregate["preparation"]["revision_count"], 1);
        assert_eq!(
            aggregate["preparation"]["current_revision"]["effective_state"],
            "inapplicable"
        );

        let options_error =
            list_planned_test_preparation_options(&storage_root, PROJECT_CODE, ITEM_CODE)
                .unwrap_err();
        assert_eq!(options_error.code, "planned_test_schedule_not_confirmed");
        assert_eq!(
            options_error.message,
            "Confirmez le créneau avant de préparer l'essai."
        );

        let assessment_error = assess_planned_test_preparation_for_schedule(
            &storage_root,
            assessment_input(
                "op-prep-on-planned",
                Some("PLAN-PREP-001-prep-rev-0001"),
                Vec::new(),
            ),
        )
        .unwrap_err();
        assert_eq!(assessment_error.code, "planned_test_schedule_not_confirmed");
        let replay_error = assess_planned_test_preparation_for_schedule(
            &storage_root,
            assessment_input("op-prep-before-planned", None, Vec::new()),
        )
        .unwrap_err();
        assert_eq!(replay_error.code, "planned_test_schedule_not_confirmed");

        let projects = Connection::open(storage_root.join("projects.sqlite")).unwrap();
        let persisted: (String, u64, u64) = projects
            .query_row(
                concat!(
                    "SELECT s.status, ",
                    "(SELECT COUNT(*) FROM planned_test_preparation_revisions), ",
                    "(SELECT COUNT(*) FROM project_audit_events WHERE project_code = ?1) ",
                    "FROM service_schedule_items s WHERE s.item_code = ?2"
                ),
                params![PROJECT_CODE, ITEM_CODE],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(persisted, ("planned".to_owned(), 1, audit_count_before));
        drop(projects);
        let sync = Connection::open(storage_root.join("sync.sqlite")).unwrap();
        let outbox_count: u64 = sync
            .query_row(
                "SELECT COUNT(*) FROM sync_operations WHERE entity_type = 'planned_test_preparation'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(outbox_count, 1);
        drop(sync);

        remove_temporary_storage_root(&storage_root);
    }

    fn assessment_input(
        operation_id: &str,
        expected_current_revision_id: Option<&str>,
        assignments: Vec<PlannedTestInstrumentAssignment>,
    ) -> AssessPlannedTestPreparationInput {
        AssessPlannedTestPreparationInput {
            project_code: PROJECT_CODE.to_owned(),
            schedule_item_code: ITEM_CODE.to_owned(),
            expected_schedule_revision: 1,
            expected_current_revision_id: expected_current_revision_id.map(str::to_owned),
            method_template_id: METHOD_ID.to_owned(),
            method_revision_id: METHOD_REVISION_ID.to_owned(),
            station_setup_id: SETUP_ID.to_owned(),
            station_setup_revision_id: SETUP_REVISION_ID.to_owned(),
            assignments,
            context: PlannedTestPreparationOperationContext {
                actor: "operateur.cem".to_owned(),
                reason: "Vérification de la préparation planifiée".to_owned(),
                operation_id: operation_id.to_owned(),
                device_id: "lab-console-test".to_owned(),
                correlation_id: format!("corr-{operation_id}"),
            },
        }
    }

    fn start_input(
        operation_id: &str,
        expected_revision: u64,
        expected_preparation_revision_id: &str,
        expected_preparation_checksum: &str,
    ) -> TransitionServiceScheduleItemInput {
        TransitionServiceScheduleItemInput {
            project_code: PROJECT_CODE.to_owned(),
            item_code: ITEM_CODE.to_owned(),
            target_status: "in_progress".to_owned(),
            expected_revision,
            expected_preparation_revision_id: Some(expected_preparation_revision_id.to_owned()),
            expected_preparation_checksum: Some(expected_preparation_checksum.to_owned()),
            actor: "operateur.cem".to_owned(),
            reason: "Démarrage de l'essai préparé".to_owned(),
            operation_id: operation_id.to_owned(),
            correlation_id: format!("corr-{operation_id}"),
            device_id: "lab-console-test".to_owned(),
        }
    }

    fn prepared_storage(name: &str) -> PathBuf {
        let storage_root = temporary_storage_root(name);
        run_storage_action(
            StorageAction::Init,
            storage_root.clone(),
            repo_root().join("storage/sqlite"),
        )
        .unwrap();
        seed_project_and_schedule(&storage_root);
        seed_method(&storage_root);
        let model_checksum = seed_equipment_model(&storage_root);
        register_instrument(
            &storage_root,
            &model_checksum,
            ASSET_ID,
            "SN-RX-001",
            "op-register-prep-asset",
        );
        register_instrument(
            &storage_root,
            &model_checksum,
            SOURCE_ASSET_ID,
            "SN-SOURCE-001",
            "op-register-prep-source",
        );
        let metrology = open_metrology_connection(&storage_root).unwrap();
        let asset_revision = load_instrument(&metrology, ASSET_ID)
            .unwrap()
            .unwrap()
            .revision;
        let source_revision = load_instrument(&metrology, SOURCE_ASSET_ID)
            .unwrap()
            .unwrap()
            .revision;
        seed_station(
            &storage_root,
            &model_checksum,
            &asset_revision,
            &source_revision,
        );
        storage_root
    }

    fn seed_project_and_schedule(storage_root: &Path) {
        let connection = Connection::open(storage_root.join("projects.sqlite")).unwrap();
        connection
            .execute(
                "INSERT INTO projects (code, customer_name, stage, execution_mode, created_at) VALUES (?1, 'Client Atlas', 'test_planning', 'investigation', ?2)",
                params![PROJECT_CODE, "2026-07-15T08:00:00Z"],
            )
            .unwrap();
        connection
            .execute(
                concat!(
                    "INSERT INTO service_schedule_items (item_code, project_code, title, ",
                    "planned_start_at, planned_end_at, assigned_operator, location, ",
                    "laboratory_location_id, laboratory_location_label, ",
                    "equipment_under_test, status, created_at, updated_at, revision, created_by, updated_by) ",
                    "VALUES (?1, ?2, 'Émission conduite', '2026-07-16T09:00', ",
                    "'2026-07-16T12:00', 'Alice Martin', 'Poste CEM 1', ",
                    "'LAB-LOCATION-CEM-1', 'Poste CEM 1', 'Calculateur Atlas', ",
                    "'confirmed', ?3, ?3, 1, 'planificateur', 'planificateur')"
                ),
                params![ITEM_CODE, PROJECT_CODE, "2026-07-15T08:00:00Z"],
            )
            .unwrap();
    }

    fn seed_method(storage_root: &Path) {
        let definition = TestTemplateDefinition {
            definition_schema_version: TEST_TEMPLATE_DEFINITION_SCHEMA_VERSION.to_owned(),
            title: "Émission conduite simulée".to_owned(),
            description: "Méthode de préparation de test".to_owned(),
            measurement_axis: MeasurementAxis::FrequencySweep,
            method_code: Some("CE-SIM".to_owned()),
            method_revision: Some("A".to_owned()),
            standard_references: vec!["CISPR 32".to_owned()],
            variables: vec![VariableDefinition {
                variable_id: "frequency_hz".to_owned(),
                label: "Fréquence".to_owned(),
                value_type: VariableValueType::Number,
                default_value: Some(VariableDefaultValue::Number(1_000_000.0)),
                constraints: VariableConstraints {
                    required: true,
                    dimensionless: false,
                    unit: Some("Hz".to_owned()),
                    minimum: Some(150_000.0),
                    maximum: Some(30_000_000.0),
                    enum_values: Vec::new(),
                },
                description: Some("Point de vérification simulé".to_owned()),
            }],
            lock_policy: vec![VariableLockPolicy {
                variable_id: "frequency_hz".to_owned(),
                policy: VariableLockPolicyKind::EditableUntilExecution,
            }],
            instrumentation_chain: vec![InstrumentationChainSlot {
                slot_id: SLOT_ID.to_owned(),
                label: "Récepteur de mesure".to_owned(),
                required_category: Some("emi_receiver".to_owned()),
                required_capability: None,
                required: true,
                calibration_requirement: CalibrationRequirement::NotRequired,
                substitution_policy: InstrumentSubstitutionPolicy::SameCategory,
                depends_on_slots: Vec::new(),
            }],
            entry_step_id: "finish".to_owned(),
            sequence: vec![ExecutionSequenceStep {
                step_id: "finish".to_owned(),
                order: 10,
                kind: ExecutionStepKind::Finish,
                label: "Terminer".to_owned(),
                instruction: None,
                required_slots: Vec::new(),
                branches: Vec::new(),
            }],
            limits: Vec::new(),
            post_processing: Vec::new(),
            method_parameters: BTreeMap::new(),
        };
        let canonical = definition.canonicalize().unwrap();
        let connection = Connection::open(storage_root.join("test_definitions.sqlite")).unwrap();
        connection
            .execute(
                "INSERT INTO test_template_identities (template_id, title, category_code, current_approved_revision_id, created_by, created_at, updated_at) VALUES (?1, ?2, 'emission_conducted', ?3, 'methodiste', ?4, ?4)",
                params![METHOD_ID, definition.title, METHOD_REVISION_ID, "2026-07-15T08:00:00Z"],
            )
            .unwrap();
        connection
            .execute(
                concat!(
                    "INSERT INTO test_template_revisions (revision_id, template_id, revision_number, status, ",
                    "definition_schema_version, definition_json, definition_checksum, created_by, created_at, ",
                    "updated_at, submitted_at, approved_at) VALUES (?1, ?2, 1, 'approved', ?3, ?4, ?5, ",
                    "'methodiste', ?6, ?6, ?6, ?6)"
                ),
                params![
                    METHOD_REVISION_ID,
                    METHOD_ID,
                    canonical.definition_schema_version,
                    canonical.canonical_json,
                    canonical.definition_checksum,
                    "2026-07-15T08:00:00Z"
                ],
            )
            .unwrap();
    }

    fn seed_equipment_model(storage_root: &Path) -> String {
        let definition = EquipmentModelDefinition {
            definition_schema_version: EQUIPMENT_MODEL_DEFINITION_SCHEMA_VERSION.to_owned(),
            manufacturer: "Locus Instruments".to_owned(),
            model_name: "RX-1".to_owned(),
            variant: None,
            equipment_class: EquipmentClass::ManualEquipment,
            functional_role: FunctionalRole::MeasurementInstrument,
            category_code: "emi_receiver".to_owned(),
            signal_domains: vec![SignalDomain::Rf],
            technology_tags: vec![TechnologyTag::Rf50Ohm],
            specifications: Vec::new(),
            signal_ports: vec![
                SignalPortDefinition {
                    port_id: "rf_input".to_owned(),
                    label: "Entrée RF".to_owned(),
                    directionality: PortDirectionality::Input,
                    flow_role: PortFlowRole::MeasurementPort,
                    signal_domain: SignalDomain::Rf,
                    required: true,
                    connector_type: Some("N".to_owned()),
                    technology_tags: vec![TechnologyTag::Rf50Ohm],
                    quantity: PhysicalQuantity::Voltage,
                    unit: "dBuV".to_owned(),
                    impedance: Some(50.0),
                    frequency_min: Some(150_000.0),
                    frequency_max: Some(30_000_000.0),
                    voltage_max: Some(10.0),
                    current_max: None,
                    power_max: None,
                    channel_index: None,
                    differential: false,
                    isolated: false,
                    comment: None,
                },
                SignalPortDefinition {
                    port_id: "rf_output".to_owned(),
                    label: "Sortie RF".to_owned(),
                    directionality: PortDirectionality::Output,
                    flow_role: PortFlowRole::SourcePort,
                    signal_domain: SignalDomain::Rf,
                    required: false,
                    connector_type: Some("N".to_owned()),
                    technology_tags: vec![TechnologyTag::Rf50Ohm],
                    quantity: PhysicalQuantity::Voltage,
                    unit: "dBuV".to_owned(),
                    impedance: Some(50.0),
                    frequency_min: Some(150_000.0),
                    frequency_max: Some(30_000_000.0),
                    voltage_max: Some(10.0),
                    current_max: None,
                    power_max: None,
                    channel_index: None,
                    differential: false,
                    isolated: false,
                    comment: None,
                },
            ],
            signal_paths: Vec::new(),
            communication_interfaces: Vec::new(),
            capabilities: Vec::new(),
            custom_field_values: BTreeMap::new(),
            template_snapshot: None,
            is_demo: false,
            metadata: BTreeMap::new(),
        };
        let canonical = definition.canonicalize().unwrap();
        let connection = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        connection
            .execute(
                "INSERT INTO equipment_model_identities (equipment_model_id, manufacturer, model_name, equipment_class, category_code, current_approved_revision_id, created_by, created_at, updated_at) VALUES (?1, ?2, ?3, 'manual_equipment', ?4, ?5, 'catalogue', ?6, ?6)",
                params![MODEL_ID, definition.manufacturer, definition.model_name, definition.category_code, MODEL_REVISION_ID, "2026-07-15T08:00:00Z"],
            )
            .unwrap();
        connection
            .execute(
                concat!(
                    "INSERT INTO equipment_model_revisions (revision_id, equipment_model_id, revision_number, status, ",
                    "definition_schema_version, definition_json, definition_checksum, created_by, created_at, ",
                    "updated_at, submitted_at, approved_at) VALUES (?1, ?2, 1, 'approved', ?3, ?4, ?5, ",
                    "'catalogue', ?6, ?6, ?6, ?6)"
                ),
                params![
                    MODEL_REVISION_ID,
                    MODEL_ID,
                    canonical.definition_schema_version,
                    canonical.canonical_json,
                    canonical.definition_checksum,
                    "2026-07-15T08:00:00Z"
                ],
            )
            .unwrap();
        canonical.definition_checksum
    }

    fn register_instrument(
        storage_root: &Path,
        model_checksum: &str,
        asset_id: &str,
        serial_number: &str,
        operation_id: &str,
    ) {
        register_metrology_instrument(
            storage_root,
            RegisterInstrumentInput {
                asset_id: asset_id.to_owned(),
                family: "Récepteur de mesure".to_owned(),
                category_code: Some("emi_receiver".to_owned()),
                equipment_model_id: Some(MODEL_ID.to_owned()),
                equipment_model_revision_id: Some(MODEL_REVISION_ID.to_owned()),
                equipment_model_checksum: Some(model_checksum.to_owned()),
                manufacturer: "Locus Instruments".to_owned(),
                model: "RX-1".to_owned(),
                serial_number: serial_number.to_owned(),
                part_number: Some("RX-1".to_owned()),
                calibration_requirement: "not_required".to_owned(),
                calibration_period_months: None,
                calibration_due_warning_days: None,
                serviceability_status: "usable".to_owned(),
                serviceability_reason: "Matériel disponible".to_owned(),
                capabilities_json: "[]".to_owned(),
                metrology_notes: String::new(),
                context: MetrologyOperationContext {
                    actor: "metrologue".to_owned(),
                    reason: "Création du matériel de test".to_owned(),
                    operation_id: operation_id.to_owned(),
                    correlation_id: format!("corr-{operation_id}"),
                    device_id: "metrology-test".to_owned(),
                },
            },
        )
        .unwrap();
    }

    fn seed_station(
        storage_root: &Path,
        model_checksum: &str,
        asset_revision: &str,
        source_revision: &str,
    ) {
        let definition = StationMeasurementSetupDefinition {
            definition_schema_version: STATION_SETUP_DEFINITION_SCHEMA_VERSION.to_owned(),
            setup_id: SETUP_ID.to_owned(),
            label: "Chaîne émission conduite".to_owned(),
            laboratory_location_id: Some("LAB-LOCATION-CEM-1".to_owned()),
            laboratory_location_label: "Poste CEM 1".to_owned(),
            planned_use_on: "2026-07-16".to_owned(),
            execution_mode: "investigation".to_owned(),
            asset_bindings: vec![
                StationAssetBindingDefinition {
                    binding_id: BINDING_ID.to_owned(),
                    role_label: "Récepteur de mesure".to_owned(),
                    asset_id: ASSET_ID.to_owned(),
                    asset_revision: asset_revision.to_owned(),
                    equipment_model_id: MODEL_ID.to_owned(),
                    equipment_model_revision_id: MODEL_REVISION_ID.to_owned(),
                    equipment_model_checksum: model_checksum.to_owned(),
                },
                StationAssetBindingDefinition {
                    binding_id: SOURCE_BINDING_ID.to_owned(),
                    role_label: "Source de vérification".to_owned(),
                    asset_id: SOURCE_ASSET_ID.to_owned(),
                    asset_revision: source_revision.to_owned(),
                    equipment_model_id: MODEL_ID.to_owned(),
                    equipment_model_revision_id: MODEL_REVISION_ID.to_owned(),
                    equipment_model_checksum: model_checksum.to_owned(),
                },
            ],
            connections: vec![StationConnectionDefinition {
                connection_id: "rf-path".to_owned(),
                label: "Chemin RF de vérification".to_owned(),
                from: StationPortEndpoint {
                    binding_id: SOURCE_BINDING_ID.to_owned(),
                    port_id: "rf_output".to_owned(),
                },
                to: StationPortEndpoint {
                    binding_id: BINDING_ID.to_owned(),
                    port_id: "rf_input".to_owned(),
                },
            }],
            correction_selections: Vec::new(),
            notes: BTreeMap::new(),
        };
        let canonical = definition.canonicalize().unwrap();
        let mut connection = Connection::open(storage_root.join("station.sqlite")).unwrap();
        let transaction = connection.transaction().unwrap();
        transaction
            .execute(
                "INSERT INTO station_setup_identities (setup_id, label, current_ready_revision_id, created_by, created_at, updated_at) VALUES (?1, ?2, ?3, 'methodiste', ?4, ?4)",
                params![SETUP_ID, definition.label, SETUP_REVISION_ID, "2026-07-15T08:00:00Z"],
            )
            .unwrap();
        transaction
            .execute(
                concat!(
                    "INSERT INTO station_setup_revisions (revision_id, setup_id, revision_number, status, ",
                    "definition_schema_version, definition_json, definition_checksum, readiness_json, ",
                    "created_by, created_at, updated_at, ready_at) VALUES (?1, ?2, 1, 'ready', ?3, ?4, ?5, ",
                    "'{\"ready\":true,\"checked_on\":\"2026-07-16\",\"issues\":[]}', ",
                    "'methodiste', ?6, ?6, ?6)"
                ),
                params![
                    SETUP_REVISION_ID,
                    SETUP_ID,
                    canonical.definition_schema_version,
                    canonical.canonical_json,
                    canonical.definition_checksum,
                    "2026-07-15T08:00:00Z"
                ],
            )
            .unwrap();
        transaction.commit().unwrap();
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
