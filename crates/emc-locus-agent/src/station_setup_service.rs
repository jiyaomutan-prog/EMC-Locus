use crate::equipment_repository::{
    list_driver_profile_identities, list_equipment_categories,
    load_current_approved_driver_profile_revision, load_equipment_model_revision,
    open_equipment_connection, DriverProfileListFilter, StoredEquipmentModelRevision,
};
use crate::fleet_dto::{AssetSelectionReasonDto, ExecutablePhysicalAssetOptionListDto};
use crate::fleet_repository::{load_physical_asset, open_fleet_connection};
use crate::fleet_service::{executable_physical_asset_options, PhysicalAssetSelectionContext};
use crate::metrology_assessment::{
    assess_metrology_source, parse_checked_on, MetrologyAssessmentSource,
};
use crate::metrology_repository::{
    load_asset_characterization, load_instrument, load_latest_calibration_event,
    open_metrology_connection,
};
use crate::metrology_service::{assess_metrology_readiness_report, AssessReadinessInput};
use crate::station_setup_dto::{
    revision_dto_unchecked, StationMaterialCandidateDto, StationMaterialCandidateListDto,
    StationSetupAggregateDto, StationSetupAuditEventDto, StationSetupAuditListDto,
    StationSetupEnvelopeDto, StationSetupIdentityDto, StationSetupListDto,
    StationSetupOperationResultDto, StationSetupReadinessEnvelopeDto, StationSetupRevisionDto,
    StationSetupRevisionEnvelopeDto, StationSetupRevisionListDto,
};
use crate::station_setup_repository::{
    insert_station_setup_audit_event, insert_station_setup_identity,
    insert_station_setup_operation, insert_station_setup_outbox, insert_station_setup_revision,
    list_station_setup_identities, load_active_station_setup_draft,
    load_attached_laboratory_location, load_attached_physical_asset_snapshot,
    load_station_setup_audit_events, load_station_setup_identity, load_station_setup_operation,
    load_station_setup_revision, load_station_setup_revisions, mark_station_setup_qualified,
    mark_station_setup_ready, next_station_setup_revision_number, open_station_connection,
    open_station_connection_with_sync, replace_station_setup_draft, sha256_text,
    AttachedLaboratoryLocation, NewStationSetupIdentity, NewStationSetupRevision,
    ReplaceStationSetupDraft, StationSetupAuditInput, StationSetupOperationInput,
    StationSetupOutboxInput, StoredStationSetupIdentity, StoredStationSetupOperation,
    StoredStationSetupRevision,
};
use crate::{render_json, AgentError};
use emc_locus_core::{
    evaluate_station_material_requirement, station_setup_qualification_issues,
    AssetCharacterizationDefinition, AuditActor, AuditReason, DriverProfileDefinition,
    EquipmentModelDefinition, MetrologyAssessmentStatus, MetrologyDate, PortDirectionality,
    SignalDomain, SignalPortDefinition, StableId, StationCalibrationRequirement,
    StationCompatibilityReason, StationCompatibilityState, StationLogicalConnectionDefinition,
    StationLogicalPortEndpoint, StationLogicalPortRequirementDefinition,
    StationMaterialAssignmentDefinition, StationMaterialAssignmentStage,
    StationMaterialRequirementDefinition, StationMaterialSelectionPolicy,
    StationMaterialSubstitutionPolicy, StationMeasurementSetupDefinition,
    StationPhysicalPortMappingDefinition, StationReadinessDimension, StationReadinessIssue,
    StationReadinessSeverity, StationSetupReadiness, STATION_SETUP_DEFINITION_SCHEMA_VERSION,
    STATION_SETUP_V2_DEFINITION_SCHEMA_VERSION,
};
use rusqlite::{OptionalExtension, TransactionBehavior};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
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
pub struct MarkStationSetupQualifiedInput {
    pub setup_id: String,
    pub revision_id: String,
    pub expected_definition_checksum: String,
    pub context: StationOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeriveStationSetupRevisionInput {
    pub setup_id: String,
    pub source_revision_id: String,
    pub upgrade_to_v3: bool,
    pub context: StationOperationContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListStationSetupAssetOptionsInput {
    pub planned_use_on: String,
    pub execution_mode: String,
    pub laboratory_location_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListStationMaterialCandidatesInput {
    pub setup_id: String,
    pub revision_id: String,
    pub requirement_id: String,
    pub planned_use_on: String,
    pub execution_mode: String,
    pub laboratory_location_id: String,
    pub excluded_schedule_item_code: Option<String>,
}

struct OperationalStationAsset<'a> {
    reference_id: &'a str,
    asset_id: &'a str,
    asset_revision: &'a str,
    equipment_model_id: &'a str,
    equipment_model_revision_id: &'a str,
    equipment_model_checksum: &'a str,
    requirement: Option<&'a StationMaterialRequirementDefinition>,
    selected_ports: &'a [StationPhysicalPortMappingDefinition],
}

pub fn list_station_setup_asset_options_json(
    storage_root: &Path,
    input: ListStationSetupAssetOptionsInput,
) -> Result<String, AgentError> {
    let checked_on = parse_checked_on(input.planned_use_on.trim(), "planned_use_on")?;
    if !matches!(
        input.execution_mode.trim(),
        "accredited" | "non_accredited" | "investigation"
    ) {
        return Err(AgentError::new(
            "invalid_station_setup_request",
            "execution_mode must be accredited, non_accredited or investigation",
        ));
    }
    safe_id(
        input.laboratory_location_id.trim(),
        "laboratory_location_id",
    )?;
    let assessed_at = OffsetDateTime::parse(
        &format!("{}T12:00:00Z", input.planned_use_on.trim()),
        &Rfc3339,
    )
    .map_err(|error| {
        AgentError::new(
            "invalid_station_setup_request",
            format!("planned_use_on: {error}"),
        )
    })?;
    let context = PhysicalAssetSelectionContext {
        assessed_at,
        checked_on,
        execution_mode: input.execution_mode.trim().to_owned(),
        laboratory_location_id: Some(input.laboratory_location_id.trim().to_owned()),
        excluded_schedule_item_code: None,
    };
    let assets = executable_physical_asset_options(storage_root, &context)?;
    Ok(render_json(&ExecutablePhysicalAssetOptionListDto {
        assessed_at: assessed_at
            .format(&Rfc3339)
            .map_err(|error| AgentError::new("timestamp_failed", error.to_string()))?,
        checked_on: input.planned_use_on.trim().to_owned(),
        execution_mode: context.execution_mode,
        laboratory_location_id: context.laboratory_location_id,
        assets,
    }))
}

pub fn list_station_material_candidates_json(
    storage_root: &Path,
    input: ListStationMaterialCandidatesInput,
) -> Result<String, AgentError> {
    Ok(render_json(&list_station_material_candidates(
        storage_root,
        input,
    )?))
}

pub(crate) fn list_station_material_candidates(
    storage_root: &Path,
    input: ListStationMaterialCandidatesInput,
) -> Result<StationMaterialCandidateListDto, AgentError> {
    safe_id(&input.setup_id, "setup_id")?;
    safe_id(&input.revision_id, "revision_id")?;
    safe_id(&input.requirement_id, "requirement_id")?;
    safe_id(&input.laboratory_location_id, "laboratory_location_id")?;
    let checked_on = parse_checked_on(input.planned_use_on.trim(), "planned_use_on")?;
    if !matches!(
        input.execution_mode.trim(),
        "accredited" | "non_accredited" | "investigation"
    ) {
        return Err(AgentError::new(
            "invalid_station_setup_request",
            "execution_mode must be accredited, non_accredited or investigation",
        ));
    }
    let station = open_station_connection(storage_root)?;
    let stored = load_station_setup_revision(&station, &input.revision_id)?.ok_or_else(|| {
        AgentError::new(
            "station_setup_revision_not_found",
            "the station setup revision does not exist",
        )
    })?;
    if stored.setup_id != input.setup_id {
        return Err(AgentError::new(
            "station_setup_revision_not_found",
            "the station setup revision does not belong to this setup",
        ));
    }
    let definition = validated_stored_definition(&stored)?;
    if definition.definition_schema_version != STATION_SETUP_DEFINITION_SCHEMA_VERSION {
        return Err(AgentError::new(
            "station_material_requirements_not_supported",
            "material requirement candidates require a station v3 revision",
        ));
    }
    let requirement = definition
        .material_requirements
        .iter()
        .find(|candidate| candidate.requirement_id == input.requirement_id)
        .ok_or_else(|| {
            AgentError::new(
                "station_material_requirement_not_found",
                "the station material requirement does not exist",
            )
        })?;
    let assessed_at = OffsetDateTime::parse(
        &format!("{}T12:00:00Z", input.planned_use_on.trim()),
        &Rfc3339,
    )
    .map_err(|error| AgentError::new("invalid_station_setup_request", error.to_string()))?;
    let option_context = PhysicalAssetSelectionContext {
        assessed_at,
        checked_on,
        execution_mode: input.execution_mode.trim().to_owned(),
        laboratory_location_id: Some(input.laboratory_location_id.trim().to_owned()),
        excluded_schedule_item_code: input.excluded_schedule_item_code.clone(),
    };
    let options = executable_physical_asset_options(storage_root, &option_context)?;
    let equipment = open_equipment_connection(storage_root)?;
    let metrology = open_metrology_connection(storage_root)?;
    let categories = list_equipment_categories(&equipment, true)?;
    let driver_identities = list_driver_profile_identities(
        &equipment,
        DriverProfileListFilter {
            equipment_model_id: None,
            status: None,
            search: None,
        },
    )?;
    let exact_asset_id = requirement.exact_asset_id.as_deref();
    let mut candidates = Vec::new();
    for option in options
        .into_iter()
        .filter(|option| exact_asset_id.is_none_or(|expected| option.asset.asset_id == expected))
    {
        let model_id = option.asset.equipment_model_id.as_deref();
        let revision_id = option.asset.equipment_model_revision_id.as_deref();
        let model = match (model_id, revision_id) {
            (Some(model_id), Some(revision_id)) => {
                load_equipment_model_revision(&equipment, model_id, revision_id)?.and_then(
                    |revision| {
                        EquipmentModelDefinition::from_json_str(&revision.definition_json).ok()
                    },
                )
            }
            _ => None,
        };
        let category_ids = category_lineage_ids(&categories, option.asset.category_code.as_str());
        let driver_actions = model_id
            .map(|model_id| approved_driver_actions(&equipment, &driver_identities, model_id))
            .transpose()?
            .unwrap_or_default();
        let compatibility = model.as_ref().map_or_else(
            || emc_locus_core::StationRequirementCompatibility {
                state: StationCompatibilityState::Indeterminate,
                requirement_compatible: false,
                reasons: vec![StationCompatibilityReason {
                    code: "model_evidence_missing".to_owned(),
                    dimension: "model_pin".to_owned(),
                    message: "La version exacte du modèle constructeur est indisponible."
                        .to_owned(),
                    next_action: Some(
                        "Rapprocher l'exemplaire avec une version de modèle approuvée.".to_owned(),
                    ),
                }],
                logical_port_candidates: BTreeMap::new(),
            },
            |model| {
                evaluate_station_material_requirement(
                    requirement,
                    &option.asset.asset_id,
                    model_id.unwrap_or_default(),
                    &category_ids,
                    model,
                    &driver_actions,
                )
            },
        );
        let requirement_compatible = compatibility.requirement_compatible;
        let calibration_blocker = station_requirement_calibration_blocker(
            &metrology,
            requirement,
            &option.asset.asset_id,
            checked_on,
        )?;
        let operationally_eligible = option.eligible && calibration_blocker.is_none();
        let mut operational_blockers = option.blocking_reasons;
        if let Some(blocker) = calibration_blocker {
            if !operational_blockers
                .iter()
                .any(|reason| reason.code == blocker.code)
            {
                operational_blockers.push(blocker);
            }
        }
        let mut next_actions: Vec<String> = compatibility
            .reasons
            .iter()
            .filter_map(|reason| reason.next_action.clone())
            .chain(
                operational_blockers
                    .iter()
                    .map(|reason| reason.next_action.clone()),
            )
            .collect();
        next_actions.sort();
        next_actions.dedup();
        let compatibility_state = match compatibility.state {
            StationCompatibilityState::Compatible => "compatible",
            StationCompatibilityState::Incompatible => "incompatible",
            StationCompatibilityState::Indeterminate => "indeterminate",
        };
        let capability_evidence = model
            .as_ref()
            .map(|model| {
                model
                    .capabilities
                    .iter()
                    .map(|capability| capability.capability_kind.clone())
                    .collect()
            })
            .unwrap_or_default();
        candidates.push(StationMaterialCandidateDto {
            category_evidence: option.asset.category_path.clone(),
            capability_evidence,
            technical_constraint_results: compatibility.reasons.clone(),
            driver_evidence: driver_actions.iter().cloned().collect(),
            logical_port_resolution_candidates: compatibility.logical_port_candidates,
            compatibility_blockers: compatibility.reasons,
            operational_blockers,
            warnings: option.warnings,
            next_actions,
            exact_asset_required: requirement.selection_policy
                == StationMaterialSelectionPolicy::ExactAsset,
            assignable: requirement_compatible && operationally_eligible,
            requirement_compatible,
            compatibility_state: compatibility_state.to_owned(),
            operationally_eligible,
            asset: option.asset,
        });
    }
    candidates.sort_by(|left, right| {
        right
            .exact_asset_required
            .cmp(&left.exact_asset_required)
            .then_with(|| right.assignable.cmp(&left.assignable))
            .then_with(|| {
                right
                    .requirement_compatible
                    .cmp(&left.requirement_compatible)
            })
            .then_with(|| left.asset.inventory_code.cmp(&right.asset.inventory_code))
    });
    let request_context_key = sha256_text(&render_json(&json!({
        "setup_id": input.setup_id,
        "revision_id": input.revision_id,
        "definition_checksum": stored.definition_checksum,
        "requirement_id": input.requirement_id,
        "planned_use_on": input.planned_use_on,
        "execution_mode": input.execution_mode,
        "laboratory_location_id": input.laboratory_location_id,
        "excluded_schedule_item_code": input.excluded_schedule_item_code,
    })));
    Ok(StationMaterialCandidateListDto {
        setup_id: input.setup_id,
        revision_id: input.revision_id,
        requirement_id: input.requirement_id,
        request_context_key,
        planned_use_on: input.planned_use_on,
        execution_mode: input.execution_mode,
        laboratory_location_id: input.laboratory_location_id,
        candidates,
    })
}

fn category_lineage_ids(
    categories: &[crate::equipment_repository::StoredEquipmentCategory],
    leaf_category_id: &str,
) -> Vec<String> {
    let mut lineage = Vec::new();
    let mut current = Some(leaf_category_id);
    while let Some(category_id) = current {
        lineage.push(category_id.to_owned());
        current = categories
            .iter()
            .find(|category| category.category_id == category_id)
            .and_then(|category| category.parent_category_id.as_deref());
        if lineage.len() > categories.len() + 1 {
            break;
        }
    }
    lineage.reverse();
    lineage
}

fn approved_driver_actions(
    equipment: &rusqlite::Connection,
    identities: &[crate::equipment_repository::StoredDriverProfileIdentity],
    model_id: &str,
) -> Result<BTreeSet<String>, AgentError> {
    let mut actions = BTreeSet::new();
    for identity in identities
        .iter()
        .filter(|identity| identity.equipment_model_id == model_id)
    {
        let Some(revision) = load_current_approved_driver_profile_revision(equipment, identity)?
        else {
            continue;
        };
        let Ok(definition) = DriverProfileDefinition::from_json_str(&revision.definition_json)
        else {
            continue;
        };
        for action in definition.actions {
            actions.insert(action.action_id);
            actions.insert(action.implements_capability_id);
        }
    }
    Ok(actions)
}

fn station_requirement_calibration_blocker(
    metrology: &rusqlite::Connection,
    requirement: &StationMaterialRequirementDefinition,
    asset_id: &str,
    checked_on: MetrologyDate,
) -> Result<Option<AssetSelectionReasonDto>, AgentError> {
    if requirement.calibration_requirement != StationCalibrationRequirement::Required {
        return Ok(None);
    }
    let Some(instrument) = load_instrument(metrology, asset_id)? else {
        return Ok(Some(AssetSelectionReasonDto {
            code: "metrology_unavailable".to_owned(),
            message: "Le rôle exige un étalonnage, mais le dossier métrologique est absent."
                .to_owned(),
            next_action: "Enregistrez l'exemplaire dans le registre métrologique.".to_owned(),
        }));
    };
    let latest = load_latest_calibration_event(metrology, asset_id)?;
    let status = assess_metrology_source(
        checked_on,
        MetrologyAssessmentSource {
            calibration_requirement: "required".to_owned(),
            calibration_period_months: instrument.calibration_period_months,
            calibration_due_warning_days: instrument.calibration_due_warning_days,
            calibrated_at: latest.as_ref().map(|event| event.calibrated_at.clone()),
            due_at: latest.as_ref().map(|event| event.due_at.clone()),
            decision: latest.as_ref().map(|event| event.decision.clone()),
            latest_calibration_event_id: latest.as_ref().map(|event| event.event_id.clone()),
            latest_calibration_revision: latest.as_ref().map(|event| event.revision.clone()),
        },
    );
    let reason = match status.assessment.status {
        MetrologyAssessmentStatus::Valid | MetrologyAssessmentStatus::DueSoon => return Ok(None),
        MetrologyAssessmentStatus::Missing | MetrologyAssessmentStatus::NotRequired => (
            "calibration_missing",
            "Le rôle exige un étalonnage valide, mais aucun événement valable n'est disponible.",
            "Enregistrez un étalonnage conforme couvrant la date prévue.",
        ),
        MetrologyAssessmentStatus::Expired => (
            "calibration_expired",
            "L'étalonnage exigé par le rôle est expiré à la date prévue.",
            "Renouvelez l'étalonnage avant l'affectation.",
        ),
        MetrologyAssessmentStatus::Nonconforming => (
            "calibration_nonconforming",
            "Le dernier étalonnage exigé par le rôle est non conforme.",
            "Traitez la non-conformité et enregistrez une décision métrologique acceptable.",
        ),
        MetrologyAssessmentStatus::Indeterminate => (
            "calibration_indeterminate",
            "La décision d'étalonnage exigée par le rôle est indéterminée.",
            "Faites statuer la décision métrologique avant l'affectation.",
        ),
        MetrologyAssessmentStatus::Unavailable => (
            "metrology_unavailable",
            "Le statut d'étalonnage exigé par le rôle ne peut pas être établi.",
            "Complétez les preuves métrologiques de l'exemplaire.",
        ),
    };
    Ok(Some(AssetSelectionReasonDto {
        code: reason.0.to_owned(),
        message: reason.1.to_owned(),
        next_action: reason.2.to_owned(),
    }))
}

pub fn create_station_setup(
    storage_root: &Path,
    input: CreateStationSetupInput,
) -> Result<String, AgentError> {
    validate_context(&input.context)?;
    safe_id(&input.setup_id, "setup_id")?;
    safe_id(&input.laboratory_location_id, "laboratory_location_id")?;
    let request_payload_json = render_json(&json!({
        "setup_id": input.setup_id.trim(),
        "label": input.label.trim(),
        "laboratory_location_id": input.laboratory_location_id.trim(),
        "planned_use_on": input.planned_use_on.trim(),
        "execution_mode": input.execution_mode.trim(),
        "reason": input.context.reason
    }));
    let payload_checksum = sha256_text(&request_payload_json);
    let timestamp = utc_timestamp()?;
    let mut connection = open_station_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    if let Some(operation) =
        load_station_setup_operation(&transaction, &input.context.operation_id)?
    {
        ensure_operation_replay(
            &operation,
            &input.context,
            &input.setup_id,
            "station_setup_created",
            &payload_checksum,
        )?;
        drop(transaction);
        return operation_result(
            &connection,
            &input.setup_id,
            "station_setup_created",
            &input.context.operation_id,
            true,
        );
    }
    if load_station_setup_identity(&transaction, &input.setup_id)?.is_some() {
        return Err(AgentError::new(
            "station_setup_exists",
            "a measurement setup with this identity already exists",
        ));
    }
    let location =
        require_active_station_location(&transaction, input.laboratory_location_id.trim())?;
    let definition = StationMeasurementSetupDefinition {
        definition_schema_version: STATION_SETUP_DEFINITION_SCHEMA_VERSION.to_owned(),
        setup_id: input.setup_id.trim().to_owned(),
        label: input.label.trim().to_owned(),
        laboratory_location_id: Some(location.location_id.clone()),
        laboratory_location_label: location.label,
        planned_use_on: input.planned_use_on.trim().to_owned(),
        execution_mode: input.execution_mode.trim().to_owned(),
        asset_bindings: Vec::new(),
        connections: Vec::new(),
        correction_selections: Vec::new(),
        material_requirements: Vec::new(),
        material_assignments: Vec::new(),
        logical_connections: Vec::new(),
        notes: BTreeMap::new(),
    };
    let canonical = canonical_definition(&definition)?;
    let readiness = assess_station_setup_readiness(storage_root, &definition)?;
    let readiness_json = render_json(&readiness);
    let revision_id = format!("{}-rev-0001", canonical.setup_id);
    let payload_json = station_evidence_payload(&canonical, &input.context.reason);
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
    let mut definition = StationMeasurementSetupDefinition::from_json_str(&input.definition_json)
        .map_err(|issue| validation_error(vec![issue]))?;
    if definition.setup_id != input.setup_id.trim() {
        return Err(AgentError::new(
            "station_setup_identity_mismatch",
            "the definition belongs to another measurement setup",
        ));
    }
    let request_payload_json = render_json(&json!({
        "expected_definition_checksum": input.expected_definition_checksum,
        "definition": station_definition_request_value(&definition),
        "reason": input.context.reason
    }));
    let payload_checksum = sha256_text(&request_payload_json);
    let timestamp = utc_timestamp()?;
    let mut connection = open_station_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    if let Some(operation) =
        load_station_setup_operation(&transaction, &input.context.operation_id)?
    {
        ensure_operation_replay(
            &operation,
            &input.context,
            &input.setup_id,
            "station_setup_draft_replaced",
            &payload_checksum,
        )?;
        drop(transaction);
        return operation_result(
            &connection,
            &input.setup_id,
            "station_setup_draft_replaced",
            &input.context.operation_id,
            true,
        );
    }
    let stored = required_revision(&transaction, &input.setup_id, &input.revision_id)?;
    if stored.status != "draft" || stored.qualified_at.is_some() {
        return Err(AgentError::new(
            "station_setup_revision_not_editable",
            "a qualified or ready setup cannot be modified; create a new draft",
        ));
    }
    if stored.definition_checksum != input.expected_definition_checksum {
        return Err(concurrency_error(&stored));
    }
    if stored.definition_schema_version != definition.definition_schema_version {
        return Err(AgentError::new(
            "station_setup_schema_change_requires_derived_revision",
            "derive an explicit revision when changing the station definition schema",
        ));
    }
    bind_current_station_location(&transaction, &mut definition)?;
    let canonical = canonical_definition(&definition)?;
    let readiness = assess_station_setup_readiness(storage_root, &definition)?;
    let readiness_json = render_json(&readiness);
    let payload_json = station_evidence_payload(&canonical, &input.context.reason);
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

pub fn mark_station_setup_revision_qualified(
    storage_root: &Path,
    input: MarkStationSetupQualifiedInput,
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
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    if let Some(operation) =
        load_station_setup_operation(&transaction, &input.context.operation_id)?
    {
        ensure_operation_replay(
            &operation,
            &input.context,
            &input.setup_id,
            "station_setup_qualified",
            &payload_checksum,
        )?;
        drop(transaction);
        return operation_result(
            &connection,
            &input.setup_id,
            "station_setup_qualified",
            &input.context.operation_id,
            true,
        );
    }
    let stored = required_revision(&transaction, &input.setup_id, &input.revision_id)?;
    if stored.status != "draft" || stored.qualified_at.is_some() {
        return Err(AgentError::new(
            "station_setup_revision_not_editable",
            "only an unqualified draft can be qualified",
        ));
    }
    if stored.definition_checksum != input.expected_definition_checksum {
        return Err(concurrency_error(&stored));
    }
    let definition = validated_stored_definition(&stored)?;
    if definition.definition_schema_version != STATION_SETUP_DEFINITION_SCHEMA_VERSION {
        return Err(AgentError::new(
            "station_setup_qualification_requires_v3",
            "logical qualification requires a station v3 definition",
        ));
    }
    validate_current_station_location(&transaction, &definition)?;
    let issues = station_setup_qualification_issues(&definition);
    if issues
        .iter()
        .any(|issue| issue.severity == StationReadinessSeverity::Blocking)
    {
        return Err(AgentError::with_details(
            "station_setup_not_qualified",
            "la définition logique du montage contient encore des blocages",
            json!({
                "setup_id": input.setup_id,
                "revision_id": input.revision_id,
                "issues": issues
            }),
        ));
    }
    mark_station_setup_qualified(
        &transaction,
        &input.setup_id,
        &input.revision_id,
        &timestamp,
    )?;
    persist_evidence(
        &transaction,
        &input.context,
        &input.setup_id,
        Some(&input.revision_id),
        "station_setup_qualified",
        Some(&input.revision_id),
        Some(&input.revision_id),
        Some(&stored.definition_checksum),
        Some(&stored.definition_checksum),
        &format!("draft:{}", input.revision_id),
        &format!("qualified:{}", input.revision_id),
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
        "station_setup_qualified",
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
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    if let Some(operation) =
        load_station_setup_operation(&transaction, &input.context.operation_id)?
    {
        ensure_operation_replay(
            &operation,
            &input.context,
            &input.setup_id,
            "station_setup_marked_ready",
            &payload_checksum,
        )?;
        drop(transaction);
        return operation_result(
            &connection,
            &input.setup_id,
            "station_setup_marked_ready",
            &input.context.operation_id,
            true,
        );
    }
    let stored = required_revision(&transaction, &input.setup_id, &input.revision_id)?;
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
    if definition.definition_schema_version == STATION_SETUP_DEFINITION_SCHEMA_VERSION
        && stored.qualified_at.is_none()
    {
        return Err(AgentError::new(
            "station_setup_not_qualified",
            "qualify the logical setup definition before declaring it ready",
        ));
    }
    validate_current_station_location(&transaction, &definition)?;
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
        "upgrade_to_v3": input.upgrade_to_v3,
        "reason": input.context.reason
    }));
    let operation_kind = if input.upgrade_to_v3 {
        "station_setup_v2_draft_upgraded"
    } else {
        "station_setup_revision_derived"
    };
    let payload_checksum = sha256_text(&payload_json);
    let timestamp = utc_timestamp()?;
    let mut connection = open_station_connection_with_sync(storage_root)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AgentError::new("transaction_begin_failed", error.to_string()))?;
    if let Some(operation) =
        load_station_setup_operation(&transaction, &input.context.operation_id)?
    {
        ensure_operation_replay(
            &operation,
            &input.context,
            &input.setup_id,
            operation_kind,
            &payload_checksum,
        )?;
        drop(transaction);
        return operation_result(
            &connection,
            &input.setup_id,
            operation_kind,
            &input.context.operation_id,
            true,
        );
    }
    if load_active_station_setup_draft(&transaction, &input.setup_id)?.is_some() {
        return Err(AgentError::new(
            "station_setup_active_draft_exists",
            "finish or discard the current draft before creating another one",
        ));
    }
    let source = required_revision(&transaction, &input.setup_id, &input.source_revision_id)?;
    let source_is_qualified_v3 = source.status == "draft"
        && source.qualified_at.is_some()
        && source.definition_schema_version == STATION_SETUP_DEFINITION_SCHEMA_VERSION;
    if !matches!(source.status.as_str(), "ready" | "superseded") && !source_is_qualified_v3 {
        return Err(AgentError::new(
            "station_setup_source_not_ready",
            "a new draft must be derived from a qualified or ready setup revision",
        ));
    }
    let mut definition = validated_stored_definition(&source)?;
    if input.upgrade_to_v3 {
        if definition.definition_schema_version != STATION_SETUP_V2_DEFINITION_SCHEMA_VERSION {
            return Err(AgentError::new(
                "station_setup_upgrade_source_not_v2",
                "only a station v2 revision can be explicitly upgraded to v3",
            ));
        }
        definition = upgrade_v2_station_definition(&transaction, definition)?;
    }
    bind_current_station_location(&transaction, &mut definition)?;
    let canonical = canonical_definition(&definition)?;
    let readiness = assess_station_setup_readiness(storage_root, &definition)?;
    let readiness_json = render_json(&readiness);
    let revision_number = next_station_setup_revision_number(&transaction, &input.setup_id)?;
    let revision_id = format!("{}-rev-{revision_number:04}", input.setup_id);
    let evidence_payload_json = render_json(&json!({
        "setup_id": input.setup_id,
        "source_revision_id": input.source_revision_id,
        "definition": serde_json::from_str::<Value>(&canonical.canonical_json)
            .expect("canonical station setup definition must be valid JSON"),
        "reason": input.context.reason
    }));
    insert_station_setup_revision(
        &transaction,
        NewStationSetupRevision {
            revision_id: &revision_id,
            setup_id: &input.setup_id,
            revision_number,
            parent_revision_id: Some(&input.source_revision_id),
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
        &input.setup_id,
        Some(&revision_id),
        operation_kind,
        Some(&input.source_revision_id),
        Some(&revision_id),
        Some(&source.definition_checksum),
        Some(&canonical.definition_checksum),
        &input.source_revision_id,
        &revision_id,
        &evidence_payload_json,
        &payload_checksum,
        &timestamp,
    )?;
    transaction
        .commit()
        .map_err(|error| AgentError::new("transaction_commit_failed", error.to_string()))?;

    operation_result(
        &connection,
        &input.setup_id,
        operation_kind,
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
    let assessed_at = OffsetDateTime::parse(
        &format!("{}T12:00:00Z", definition.planned_use_on),
        &Rfc3339,
    )
    .map_err(|error| {
        AgentError::new(
            "invalid_station_setup_request",
            format!("planned_use_on: {error}"),
        )
    })?;
    assess_station_setup_readiness_for_context(storage_root, definition, assessed_at, None)
}

pub(crate) fn assess_station_setup_readiness_for_context(
    storage_root: &Path,
    definition: &StationMeasurementSetupDefinition,
    assessed_at: OffsetDateTime,
    excluded_schedule_item_code: Option<&str>,
) -> Result<StationSetupReadiness, AgentError> {
    let mut issues = definition.structural_readiness_issues();
    let mut operational_assets =
        Vec::with_capacity(definition.asset_bindings.len() + definition.material_assignments.len());
    operational_assets.extend(definition.asset_bindings.iter().map(|binding| {
        OperationalStationAsset {
            reference_id: &binding.binding_id,
            asset_id: &binding.asset_id,
            asset_revision: &binding.asset_revision,
            equipment_model_id: &binding.equipment_model_id,
            equipment_model_revision_id: &binding.equipment_model_revision_id,
            equipment_model_checksum: &binding.equipment_model_checksum,
            requirement: None,
            selected_ports: &[],
        }
    }));
    operational_assets.extend(definition.material_assignments.iter().map(|assignment| {
        let requirement = definition
            .material_requirements
            .iter()
            .find(|requirement| requirement.requirement_id == assignment.requirement_id)
            .expect("station setup integrity guarantees material requirement");
        OperationalStationAsset {
            reference_id: &assignment.requirement_id,
            asset_id: &assignment.asset_id,
            asset_revision: &assignment.asset_revision,
            equipment_model_id: &assignment.equipment_model_id,
            equipment_model_revision_id: &assignment.equipment_model_revision_id,
            equipment_model_checksum: &assignment.equipment_model_checksum,
            requirement: Some(requirement),
            selected_ports: &assignment.selected_ports,
        }
    }));
    let mut references_by_asset: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for asset in &operational_assets {
        references_by_asset
            .entry(asset.asset_id)
            .or_default()
            .push(asset.reference_id);
    }

    if !operational_assets.is_empty() {
        let checked_on = parse_checked_on(&definition.planned_use_on, "planned_use_on")?;
        let option_context = PhysicalAssetSelectionContext {
            assessed_at,
            checked_on,
            execution_mode: definition.execution_mode.clone(),
            laboratory_location_id: definition.laboratory_location_id.clone(),
            excluded_schedule_item_code: excluded_schedule_item_code.map(str::to_owned),
        };
        let selection_options = executable_physical_asset_options(storage_root, &option_context)?;
        for operational_asset in &operational_assets {
            let Some(option) = selection_options
                .iter()
                .find(|option| option.asset.asset_id == operational_asset.asset_id)
            else {
                continue;
            };
            for reason in &option.blocking_reasons {
                issues.push(selection_readiness_issue(
                    reason,
                    StationReadinessSeverity::Blocking,
                    operational_asset.reference_id,
                ));
            }
            for reason in &option.warnings {
                issues.push(selection_readiness_issue(
                    reason,
                    StationReadinessSeverity::Warning,
                    operational_asset.reference_id,
                ));
            }
        }
        let metrology = open_metrology_connection(storage_root)?;
        for operational_asset in &operational_assets {
            let Some(requirement) = operational_asset.requirement else {
                continue;
            };
            if let Some(reason) = station_requirement_calibration_blocker(
                &metrology,
                requirement,
                operational_asset.asset_id,
                checked_on,
            )? {
                issues.push(selection_readiness_issue(
                    &reason,
                    StationReadinessSeverity::Blocking,
                    operational_asset.reference_id,
                ));
            }
        }
        let report = assess_metrology_readiness_report(
            storage_root,
            AssessReadinessInput {
                asset_ids: operational_assets
                    .iter()
                    .map(|asset| asset.asset_id.to_owned())
                    .collect(),
                execution_mode: definition.execution_mode.clone(),
                checked_on: definition.planned_use_on.clone(),
                context: Some(format!("Montage {}", definition.label)),
            },
        )?;
        for issue in report.blocking_issues {
            let references = references_by_asset.get(issue.asset_id.as_str());
            if let Some(references) = references {
                for reference in references {
                    issues.push(metrology_issue(
                        &issue.code,
                        &issue.dimension,
                        StationReadinessSeverity::Blocking,
                        Some(reference),
                    ));
                }
            } else {
                issues.push(metrology_issue(
                    &issue.code,
                    &issue.dimension,
                    StationReadinessSeverity::Blocking,
                    None,
                ));
            }
        }
        for issue in report.warnings {
            let references = references_by_asset.get(issue.asset_id.as_str());
            if let Some(references) = references {
                for reference in references {
                    issues.push(metrology_issue(
                        &issue.code,
                        &issue.dimension,
                        StationReadinessSeverity::Warning,
                        Some(reference),
                    ));
                }
            } else {
                issues.push(metrology_issue(
                    &issue.code,
                    &issue.dimension,
                    StationReadinessSeverity::Warning,
                    None,
                ));
            }
        }
    }

    let metrology = open_metrology_connection(storage_root)?;
    let equipment = open_fleet_connection(storage_root)?;
    let categories = list_equipment_categories(&equipment, true)?;
    let driver_identities = list_driver_profile_identities(
        &equipment,
        DriverProfileListFilter {
            equipment_model_id: None,
            status: None,
            search: None,
        },
    )?;
    let mut models = BTreeMap::new();
    for operational_asset in &operational_assets {
        let Some(asset) = load_physical_asset(&equipment, operational_asset.asset_id)? else {
            issues.push(blocking_issue(
                "station_physical_asset_missing",
                StationReadinessDimension::AssetIdentity,
                "L'exemplaire du parc sélectionné n'existe plus. Choisissez un matériel du parc disponible.",
                Some(operational_asset.reference_id),
                None,
            ));
            continue;
        };
        if !asset_revision_matches(
            operational_asset.asset_revision,
            asset.revision,
            &asset.asset_id,
            &asset.updated_at,
        ) || asset.equipment_model_id.as_deref() != Some(operational_asset.equipment_model_id)
            || asset.equipment_model_revision_id.as_deref()
                != Some(operational_asset.equipment_model_revision_id)
            || asset.equipment_model_checksum.as_deref()
                != Some(operational_asset.equipment_model_checksum)
        {
            issues.push(blocking_issue(
                "station_asset_reference_changed",
                StationReadinessDimension::AssetIdentity,
                "Le dossier du matériel a changé. Rechargez-le avant de valider le montage.",
                Some(operational_asset.reference_id),
                None,
            ));
            continue;
        }
        let Some(model_revision) = load_equipment_model_revision(
            &equipment,
            operational_asset.equipment_model_id,
            operational_asset.equipment_model_revision_id,
        )?
        else {
            issues.push(blocking_issue(
                "station_equipment_model_missing",
                StationReadinessDimension::AssetIdentity,
                "Le modèle approuvé du matériel n'est plus disponible localement.",
                Some(operational_asset.reference_id),
                None,
            ));
            continue;
        };
        if !matches!(model_revision.status.as_str(), "approved" | "superseded")
            || model_revision.definition_checksum != operational_asset.equipment_model_checksum
        {
            issues.push(blocking_issue(
                "station_equipment_model_reference_mismatch",
                StationReadinessDimension::AssetIdentity,
                "La version du modèle ne correspond plus au matériel sélectionné.",
                Some(operational_asset.reference_id),
                None,
            ));
            continue;
        }
        if let Some(model) =
            validated_equipment_model(&model_revision, &mut issues, operational_asset.reference_id)
        {
            if let Some(requirement) = operational_asset.requirement {
                let category_ids = category_lineage_ids(&categories, &asset.category_code_snapshot);
                let driver_actions = approved_driver_actions(
                    &equipment,
                    &driver_identities,
                    operational_asset.equipment_model_id,
                )?;
                let compatibility = evaluate_station_material_requirement(
                    requirement,
                    operational_asset.asset_id,
                    operational_asset.equipment_model_id,
                    &category_ids,
                    &model,
                    &driver_actions,
                );
                for reason in &compatibility.reasons {
                    issues.push(requirement_readiness_issue(
                        reason,
                        operational_asset.reference_id,
                    ));
                }
                for logical_port in &requirement.logical_ports {
                    let selected = operational_asset
                        .selected_ports
                        .iter()
                        .find(|mapping| mapping.logical_port_id == logical_port.logical_port_id);
                    let compatible_ports = compatibility
                        .logical_port_candidates
                        .get(&logical_port.logical_port_id);
                    match selected {
                        None => issues.push(blocking_issue(
                            "station_material_logical_port_unassigned",
                            StationReadinessDimension::PortCompatibility,
                            "Un port logique du rôle matériel n'a pas de port physique affecté.",
                            Some(operational_asset.reference_id),
                            None,
                        )),
                        Some(mapping)
                            if !compatible_ports.is_some_and(|ports| {
                                ports.iter().any(|port| port == &mapping.actual_port_id)
                            }) =>
                        {
                            issues.push(blocking_issue(
                                "station_material_port_mapping_incompatible",
                                StationReadinessDimension::PortCompatibility,
                                "Le port physique affecté ne satisfait plus le port logique du rôle matériel.",
                                Some(operational_asset.reference_id),
                                None,
                            ));
                        }
                        Some(_) => {}
                    }
                }
            }
            models.insert(operational_asset.reference_id.to_owned(), model);
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
    issues.dedup_by(|left, right| {
        left.code == right.code
            && left.severity == right.severity
            && left.binding_ids == right.binding_ids
            && left.connection_ids == right.connection_ids
    });
    Ok(StationSetupReadiness::from_issues(
        definition.planned_use_on.clone(),
        issues,
    ))
}

fn asset_revision_matches(stored: &str, revision: u64, asset_id: &str, updated_at: &str) -> bool {
    if stored == revision.to_string() {
        return true;
    }

    // Station revisions created before 0.22.0 used the metrology adapter's
    // deterministic token. Keep those historical snapshots readable while all
    // new selectors use the authoritative fleet revision number.
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(b"emc-locus-agent:instrument:");
    hasher.update(asset_id.as_bytes());
    hasher.update(b":");
    hasher.update(updated_at.as_bytes());
    let legacy_prefix = format!("rev-{}", &format!("{:x}", hasher.finalize())[..12]);
    stored == legacy_prefix
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

fn selection_readiness_issue(
    reason: &AssetSelectionReasonDto,
    severity: StationReadinessSeverity,
    binding_id: &str,
) -> StationReadinessIssue {
    let dimension = if reason.code.starts_with("calibration_") {
        if reason.code == "calibration_nonconforming" {
            StationReadinessDimension::Nonconformance
        } else if reason.code == "calibration_missing" {
            StationReadinessDimension::MissingEvidence
        } else {
            StationReadinessDimension::CalibrationValidity
        }
    } else if reason.code == "metrology_unavailable" {
        StationReadinessDimension::MissingEvidence
    } else if reason.code.starts_with("model_") || reason.code.starts_with("location_") {
        StationReadinessDimension::AssetIdentity
    } else {
        StationReadinessDimension::Serviceability
    };
    StationReadinessIssue {
        code: format!("station_{}", reason.code),
        severity,
        dimension,
        message: format!("{} {}", reason.message, reason.next_action),
        binding_ids: vec![binding_id.to_owned()],
        connection_ids: Vec::new(),
    }
}

fn requirement_readiness_issue(
    reason: &StationCompatibilityReason,
    requirement_id: &str,
) -> StationReadinessIssue {
    StationReadinessIssue {
        code: format!("station_requirement_{}", reason.code),
        severity: StationReadinessSeverity::Blocking,
        dimension: if reason.dimension == "port" {
            StationReadinessDimension::PortCompatibility
        } else {
            StationReadinessDimension::AssetIdentity
        },
        message: reason.message.clone(),
        binding_ids: vec![requirement_id.to_owned()],
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

fn upgrade_v2_station_definition(
    transaction: &rusqlite::Transaction<'_>,
    mut definition: StationMeasurementSetupDefinition,
) -> Result<StationMeasurementSetupDefinition, AgentError> {
    let bindings = definition.asset_bindings.clone();
    let connections = definition.connections.clone();
    let mut material_requirements = Vec::new();
    let mut material_assignments = Vec::new();
    for binding in &bindings {
        let asset = load_attached_physical_asset_snapshot(transaction, &binding.asset_id)?
            .ok_or_else(|| {
                AgentError::new(
                    "station_setup_upgrade_asset_missing",
                    format!("physical asset no longer exists: {}", binding.asset_id),
                )
            })?;
        if asset.asset_id != binding.asset_id
            || asset.revision.to_string() != binding.asset_revision
        {
            return Err(AgentError::with_details(
                "station_setup_upgrade_asset_revision_stale",
                "the physical asset changed after the v2 setup was recorded",
                json!({
                    "asset_id": binding.asset_id,
                    "v2_asset_revision": binding.asset_revision,
                    "current_asset_revision": asset.revision
                }),
            ));
        }
        let model = attached_equipment_model_definition(
            transaction,
            &binding.equipment_model_id,
            &binding.equipment_model_revision_id,
            &binding.equipment_model_checksum,
        )?;
        let mut logical_ports: BTreeMap<String, StationLogicalPortRequirementDefinition> =
            BTreeMap::new();
        for connection in &connections {
            for (endpoint, directionality) in [
                (&connection.from, PortDirectionality::Output),
                (&connection.to, PortDirectionality::Input),
            ] {
                if endpoint.binding_id != binding.binding_id {
                    continue;
                }
                let actual = model
                    .signal_ports
                    .iter()
                    .find(|port| port.port_id == endpoint.port_id)
                    .ok_or_else(|| {
                        AgentError::with_details(
                            "station_setup_upgrade_port_missing",
                            "a v2 physical port no longer exists in its pinned model revision",
                            json!({
                                "binding_id": binding.binding_id,
                                "port_id": endpoint.port_id,
                                "model_revision_id": binding.equipment_model_revision_id
                            }),
                        )
                    })?;
                logical_ports
                    .entry(endpoint.port_id.clone())
                    .and_modify(|port| {
                        if port.directionality != directionality {
                            port.directionality = PortDirectionality::Bidirectional;
                        }
                    })
                    .or_insert_with(|| StationLogicalPortRequirementDefinition {
                        logical_port_id: endpoint.port_id.clone(),
                        label: actual.label.clone(),
                        directionality,
                        signal_domain: actual.signal_domain,
                        connector_requirement: actual.connector_type.clone(),
                        impedance_ohm: actual.impedance,
                        frequency_range: range_from_actual(
                            actual.frequency_min,
                            actual.frequency_max,
                            "Hz",
                        ),
                        voltage_range: range_from_actual(None, actual.voltage_max, "V"),
                        current_range: range_from_actual(None, actual.current_max, "A"),
                        power_range: range_from_actual(None, actual.power_max, "W"),
                    });
            }
        }
        material_requirements.push(StationMaterialRequirementDefinition {
            requirement_id: binding.binding_id.clone(),
            role_label: binding.role_label.clone(),
            description: "Rôle converti explicitement depuis une révision station v2.".to_owned(),
            required: true,
            selection_policy: StationMaterialSelectionPolicy::ExactAsset,
            assignment_stage: StationMaterialAssignmentStage::SetupDefinition,
            substitution_policy: StationMaterialSubstitutionPolicy::NoSubstitution,
            calibration_requirement: StationCalibrationRequirement::IfUsed,
            category_requirement: None,
            capability_requirement: None,
            exact_asset_id: Some(binding.asset_id.clone()),
            logical_ports: logical_ports.values().cloned().collect(),
        });
        material_assignments.push(StationMaterialAssignmentDefinition {
            requirement_id: binding.binding_id.clone(),
            asset_id: binding.asset_id.clone(),
            asset_revision: binding.asset_revision.clone(),
            inventory_code: asset.inventory_code,
            serial_number: asset.serial_number,
            equipment_model_id: binding.equipment_model_id.clone(),
            equipment_model_revision_id: binding.equipment_model_revision_id.clone(),
            equipment_model_checksum: binding.equipment_model_checksum.clone(),
            selected_ports: logical_ports
                .keys()
                .map(|port_id| StationPhysicalPortMappingDefinition {
                    logical_port_id: port_id.clone(),
                    actual_port_id: port_id.clone(),
                })
                .collect(),
            assignment_context: "v2_upgrade".to_owned(),
            assigned_on: definition.planned_use_on.clone(),
        });
    }
    let logical_connections = connections
        .iter()
        .map(|connection| StationLogicalConnectionDefinition {
            connection_id: connection.connection_id.clone(),
            label: connection.label.clone(),
            from: StationLogicalPortEndpoint {
                requirement_id: connection.from.binding_id.clone(),
                logical_port_id: connection.from.port_id.clone(),
            },
            to: StationLogicalPortEndpoint {
                requirement_id: connection.to.binding_id.clone(),
                logical_port_id: connection.to.port_id.clone(),
            },
        })
        .collect();
    if !definition.correction_selections.is_empty() {
        definition.notes.insert(
            "v2_correction_selections".to_owned(),
            serde_json::to_value(&definition.correction_selections).expect("selections serialize"),
        );
    }
    definition.definition_schema_version = STATION_SETUP_DEFINITION_SCHEMA_VERSION.to_owned();
    definition.asset_bindings.clear();
    definition.connections.clear();
    definition.correction_selections.clear();
    definition.material_requirements = material_requirements;
    definition.material_assignments = material_assignments;
    definition.logical_connections = logical_connections;
    Ok(definition)
}

fn attached_equipment_model_definition(
    transaction: &rusqlite::Transaction<'_>,
    model_id: &str,
    revision_id: &str,
    expected_checksum: &str,
) -> Result<EquipmentModelDefinition, AgentError> {
    let stored = transaction
        .query_row(
            "SELECT definition_json, definition_checksum, status
             FROM equipment_db.equipment_model_revisions
             WHERE equipment_model_id = ?1 AND revision_id = ?2",
            rusqlite::params![model_id, revision_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|error| AgentError::new("station_setup_query_failed", error.to_string()))?
        .ok_or_else(|| {
            AgentError::new(
                "station_setup_upgrade_model_missing",
                "the pinned equipment model revision no longer exists",
            )
        })?;
    if stored.1 != expected_checksum || !matches!(stored.2.as_str(), "approved" | "superseded") {
        return Err(AgentError::new(
            "station_setup_upgrade_model_pin_invalid",
            "the v2 model pin is no longer an immutable matching revision",
        ));
    }
    let definition = EquipmentModelDefinition::from_json_str(&stored.0).map_err(|issue| {
        AgentError::with_details(
            "station_setup_upgrade_model_invalid",
            "the pinned model definition is invalid",
            json!({ "issue": issue }),
        )
    })?;
    let canonical = definition.canonicalize().map_err(|issues| {
        AgentError::with_details(
            "station_setup_upgrade_model_invalid",
            "the pinned model definition is invalid",
            json!({ "issues": issues }),
        )
    })?;
    if canonical.definition_checksum != expected_checksum {
        return Err(AgentError::new(
            "station_setup_upgrade_model_pin_invalid",
            "the v2 model checksum does not match canonical content",
        ));
    }
    Ok(definition)
}

fn range_from_actual(
    minimum: Option<f64>,
    maximum: Option<f64>,
    unit: &str,
) -> Option<emc_locus_core::StationRangeConstraint> {
    (minimum.is_some() || maximum.is_some()).then(|| emc_locus_core::StationRangeConstraint {
        minimum,
        maximum,
        unit: unit.to_owned(),
    })
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
        .find(|revision| revision.status == "draft" && revision.qualified_at.is_none())
        .map(validated_revision_dto)
        .transpose()?;
    let current_qualified_revision = identity
        .current_qualified_revision_id
        .as_deref()
        .map(|revision_id| {
            revisions
                .iter()
                .find(|revision| revision.revision_id == revision_id)
                .ok_or_else(|| {
                    AgentError::new(
                        "station_setup_storage_invalid",
                        "current qualified revision reference is missing",
                    )
                })
                .and_then(validated_revision_dto)
        })
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
        current_qualified_revision,
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

fn require_active_station_location(
    connection: &rusqlite::Connection,
    location_id: &str,
) -> Result<AttachedLaboratoryLocation, AgentError> {
    let location =
        load_attached_laboratory_location(connection, location_id)?.ok_or_else(|| {
            AgentError::with_details(
                "station_setup_location_not_found",
                "Le lieu sélectionné n'existe pas dans le registre du laboratoire.",
                json!({
                    "laboratory_location_id": location_id,
                    "next_action": "Créez ou sélectionnez un lieu actif du laboratoire."
                }),
            )
        })?;
    if location.status != "active" {
        return Err(AgentError::with_details(
            "station_setup_location_archived",
            "Le lieu sélectionné est archivé et ne peut pas recevoir un nouveau montage.",
            json!({
                "laboratory_location_id": location.location_id,
                "laboratory_location_label": location.label,
                "next_action": "Sélectionnez un lieu actif du laboratoire."
            }),
        ));
    }
    Ok(location)
}

fn bind_current_station_location(
    connection: &rusqlite::Connection,
    definition: &mut StationMeasurementSetupDefinition,
) -> Result<(), AgentError> {
    let Some(location_id) = definition
        .laboratory_location_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
    else {
        definition.laboratory_location_id = None;
        definition.laboratory_location_label.clear();
        return Ok(());
    };
    safe_id(&location_id, "laboratory_location_id")?;
    let location = require_active_station_location(connection, &location_id)?;
    definition.laboratory_location_id = Some(location.location_id);
    definition.laboratory_location_label = location.label;
    Ok(())
}

fn validate_current_station_location(
    connection: &rusqlite::Connection,
    definition: &StationMeasurementSetupDefinition,
) -> Result<(), AgentError> {
    if let Some(location_id) = definition
        .laboratory_location_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        require_active_station_location(connection, location_id)?;
    }
    Ok(())
}

fn station_definition_request_value(definition: &StationMeasurementSetupDefinition) -> Value {
    let mut value =
        serde_json::to_value(definition).expect("station setup request definition must serialize");
    if let Some(object) = value.as_object_mut() {
        object.remove("laboratory_location_label");
    }
    value
}

fn station_evidence_payload(
    canonical: &emc_locus_core::CanonicalStationMeasurementSetupDefinition,
    reason: &str,
) -> String {
    render_json(&json!({
        "definition": serde_json::from_str::<Value>(&canonical.canonical_json)
            .expect("canonical station setup definition must be valid JSON"),
        "reason": reason
    }))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrology_service::{
        record_metrology_calibration, register_metrology_instrument, MetrologyOperationContext,
        RecordCalibrationInput, RegisterInstrumentInput,
    };
    use crate::{run_storage_action, StorageAction};
    use emc_locus_core::{
        EquipmentClass, FunctionalRole, PhysicalQuantity, PortFlowRole, TechnologyTag,
        EQUIPMENT_MODEL_DEFINITION_SCHEMA_VERSION,
    };
    use rusqlite::{params, Connection};
    use serde_json::Value;
    use std::{path::PathBuf, thread, time::Duration};

    #[test]
    fn create_derives_location_label_and_replay_survives_later_archive() {
        let storage_root = initialized_storage("station-location-create");
        insert_location(&storage_root, "LOC-STATION-A", "Lieu autoritatif");
        let input = create_input("SETUP-ATOMIC-A", "LOC-STATION-A", "op-station-create-a");

        let created = json_value(&create_station_setup(&storage_root, input.clone()).unwrap());
        assert_eq!(
            created["station_setup"]["active_draft_revision"]["definition"]
                ["laboratory_location_label"],
            "Lieu autoritatif"
        );

        set_location(&storage_root, "LOC-STATION-A", "Lieu archivé", "archived");
        let replayed = json_value(&create_station_setup(&storage_root, input).unwrap());
        assert_eq!(replayed["replayed"], true);
        assert_eq!(
            replayed["station_setup"]["active_draft_revision"]["definition"]
                ["laboratory_location_label"],
            "Lieu autoritatif"
        );

        let before = evidence_counts(&storage_root);
        let refused = create_station_setup(
            &storage_root,
            create_input(
                "SETUP-ATOMIC-REFUSED",
                "LOC-STATION-A",
                "op-station-refused",
            ),
        )
        .unwrap_err();
        assert_eq!(refused.code, "station_setup_location_archived");
        assert_eq!(evidence_counts(&storage_root), before);

        insert_location(&storage_root, "LOC-STATION-CREATE-RACE", "Lieu concurrent");
        let before_race = evidence_counts(&storage_root);
        let equipment = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        equipment.execute_batch("BEGIN IMMEDIATE").unwrap();
        equipment
            .execute(
                "UPDATE laboratory_locations SET status = 'archived', revision = revision + 1
                 WHERE location_id = 'LOC-STATION-CREATE-RACE'",
                [],
            )
            .unwrap();
        let worker_root = storage_root.clone();
        let worker = thread::spawn(move || {
            create_station_setup(
                &worker_root,
                create_input(
                    "SETUP-ATOMIC-CREATE-RACE",
                    "LOC-STATION-CREATE-RACE",
                    "op-station-create-race",
                ),
            )
        });
        thread::sleep(Duration::from_millis(100));
        equipment.execute_batch("COMMIT").unwrap();
        let refused_race = worker.join().unwrap().unwrap_err();
        assert_eq!(refused_race.code, "station_setup_location_archived");
        assert_eq!(evidence_counts(&storage_root), before_race);
        let _ = std::fs::remove_dir_all(storage_root);
    }

    #[test]
    fn concurrent_archive_blocks_draft_location_reassignment_without_evidence() {
        let storage_root = initialized_storage("station-location-concurrent-update");
        insert_location(&storage_root, "LOC-STATION-B", "Lieu initial");
        let created = json_value(
            &create_station_setup(
                &storage_root,
                create_input("SETUP-ATOMIC-B", "LOC-STATION-B", "op-station-create-b"),
            )
            .unwrap(),
        );
        let revision = &created["station_setup"]["active_draft_revision"];
        let initial_checksum = revision["definition_checksum"].as_str().unwrap().to_owned();
        let mut definition = revision["definition"].clone();
        definition["laboratory_location_label"] = json!("Libellé fourni par le client");
        definition["notes"] = json!({"operator_note": "Première modification"});
        set_location(
            &storage_root,
            "LOC-STATION-B",
            "Lieu renommé par le registre",
            "active",
        );
        let saved = json_value(
            &replace_station_setup_draft_definition(
                &storage_root,
                ReplaceStationSetupDraftInput {
                    setup_id: "SETUP-ATOMIC-B".to_owned(),
                    revision_id: "SETUP-ATOMIC-B-rev-0001".to_owned(),
                    expected_definition_checksum: initial_checksum,
                    definition_json: definition.to_string(),
                    context: context("op-station-save-server-label"),
                },
            )
            .unwrap(),
        );
        let saved_revision = &saved["station_setup"]["active_draft_revision"];
        assert_eq!(
            saved_revision["definition"]["laboratory_location_label"],
            "Lieu renommé par le registre"
        );
        let expected_checksum = saved_revision["definition_checksum"]
            .as_str()
            .unwrap()
            .to_owned();
        let mut concurrent_definition = saved_revision["definition"].clone();
        concurrent_definition["laboratory_location_label"] = json!("Deuxième faux libellé");
        concurrent_definition["notes"] = json!({"operator_note": "Modification concurrente"});
        let before = evidence_counts(&storage_root);

        let equipment = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        equipment.execute_batch("BEGIN IMMEDIATE").unwrap();
        equipment
            .execute(
                "UPDATE laboratory_locations SET status = 'archived', revision = revision + 1
                 WHERE location_id = 'LOC-STATION-B'",
                [],
            )
            .unwrap();
        let worker_root = storage_root.clone();
        let worker = thread::spawn(move || {
            replace_station_setup_draft_definition(
                &worker_root,
                ReplaceStationSetupDraftInput {
                    setup_id: "SETUP-ATOMIC-B".to_owned(),
                    revision_id: "SETUP-ATOMIC-B-rev-0001".to_owned(),
                    expected_definition_checksum: expected_checksum,
                    definition_json: concurrent_definition.to_string(),
                    context: context("op-station-concurrent-update"),
                },
            )
        });
        thread::sleep(Duration::from_millis(100));
        equipment.execute_batch("COMMIT").unwrap();

        let refused = worker.join().unwrap().unwrap_err();
        assert_eq!(refused.code, "station_setup_location_archived");
        assert_eq!(evidence_counts(&storage_root), before);
        let current = json_value(&get_station_setup(&storage_root, "SETUP-ATOMIC-B").unwrap());
        assert_eq!(
            current["station_setup"]["active_draft_revision"]["definition_checksum"],
            saved_revision["definition_checksum"]
        );
        let _ = std::fs::remove_dir_all(storage_root);
    }

    #[test]
    fn derived_revision_refreshes_label_without_rewriting_historical_snapshot() {
        let storage_root = initialized_storage("station-location-history");
        insert_location(&storage_root, "LOC-STATION-C", "Lieu historique");
        create_station_setup(
            &storage_root,
            create_input("SETUP-ATOMIC-C", "LOC-STATION-C", "op-station-create-c"),
        )
        .unwrap();
        let station = Connection::open(storage_root.join("station.sqlite")).unwrap();
        station
            .execute(
                "UPDATE station_setup_revisions
                 SET status = 'ready', ready_at = '2026-07-27T09:00:00Z',
                     updated_at = '2026-07-27T09:00:00Z'
                 WHERE revision_id = 'SETUP-ATOMIC-C-rev-0001'",
                [],
            )
            .unwrap();
        station
            .execute(
                "UPDATE station_setup_identities
                 SET current_ready_revision_id = 'SETUP-ATOMIC-C-rev-0001'
                 WHERE setup_id = 'SETUP-ATOMIC-C'",
                [],
            )
            .unwrap();
        drop(station);
        set_location(&storage_root, "LOC-STATION-C", "Lieu renommé", "active");

        let derived = json_value(
            &derive_station_setup_revision(
                &storage_root,
                DeriveStationSetupRevisionInput {
                    setup_id: "SETUP-ATOMIC-C".to_owned(),
                    source_revision_id: "SETUP-ATOMIC-C-rev-0001".to_owned(),
                    upgrade_to_v3: false,
                    context: context("op-station-derive-c"),
                },
            )
            .unwrap(),
        );
        assert_eq!(
            derived["station_setup"]["active_draft_revision"]["definition"]
                ["laboratory_location_label"],
            "Lieu renommé"
        );
        let historical = json_value(
            &get_station_setup_revision_json(
                &storage_root,
                "SETUP-ATOMIC-C",
                "SETUP-ATOMIC-C-rev-0001",
            )
            .unwrap(),
        );
        assert_eq!(
            historical["revision"]["definition"]["laboratory_location_label"],
            "Lieu historique"
        );
        assert_eq!(
            historical["revision"]["definition"]["laboratory_location_id"],
            "LOC-STATION-C"
        );
        let _ = std::fs::remove_dir_all(storage_root);
    }

    #[test]
    fn exact_blocked_requirements_can_be_qualified_but_not_declared_ready() {
        let storage_root = initialized_storage("station-v3-qualified-exact");
        insert_location(&storage_root, "LOC-STATION-V3", "Labo CEM");
        let created = json_value(
            &create_station_setup(
                &storage_root,
                create_input("SETUP-V3-EXACT", "LOC-STATION-V3", "op-v3-create"),
            )
            .unwrap(),
        );
        let draft = &created["station_setup"]["active_draft_revision"];
        let mut definition = draft["definition"].clone();
        definition["material_requirements"] = json!([
            {
                "requirement_id": "source",
                "role_label": "Source de vérification",
                "required": true,
                "selection_policy": "exact_asset",
                "assignment_stage": "setup_definition",
                "substitution_policy": "no_substitution",
                "calibration_requirement": "not_required",
                "exact_asset_id": "ES01",
                "logical_ports": [{
                    "logical_port_id": "rf",
                    "label": "Sortie RF",
                    "directionality": "output",
                    "signal_domain": "rf"
                }]
            },
            {
                "requirement_id": "cable",
                "role_label": "Câble RF imposé",
                "required": true,
                "selection_policy": "exact_asset",
                "assignment_stage": "setup_definition",
                "substitution_policy": "no_substitution",
                "calibration_requirement": "required",
                "exact_asset_id": "CA-001",
                "logical_ports": [{
                    "logical_port_id": "rf",
                    "label": "Entrée RF",
                    "directionality": "input",
                    "signal_domain": "rf"
                }]
            }
        ]);
        definition["logical_connections"] = json!([{
            "connection_id": "rf-path",
            "label": "Source vers câble",
            "from": {"requirement_id": "source", "logical_port_id": "rf"},
            "to": {"requirement_id": "cable", "logical_port_id": "rf"}
        }]);
        let saved = json_value(
            &replace_station_setup_draft_definition(
                &storage_root,
                ReplaceStationSetupDraftInput {
                    setup_id: "SETUP-V3-EXACT".to_owned(),
                    revision_id: "SETUP-V3-EXACT-rev-0001".to_owned(),
                    expected_definition_checksum: draft["definition_checksum"]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                    definition_json: definition.to_string(),
                    context: context("op-v3-save-exact"),
                },
            )
            .unwrap(),
        );
        assert!(
            saved["station_setup"]["active_draft_revision"]["definition"]["material_requirements"]
                .as_array()
                .unwrap()
                .iter()
                .any(|requirement| requirement["exact_asset_id"] == "CA-001")
        );
        let checksum = saved["station_setup"]["active_draft_revision"]["definition_checksum"]
            .as_str()
            .unwrap()
            .to_owned();
        let qualified = json_value(
            &mark_station_setup_revision_qualified(
                &storage_root,
                MarkStationSetupQualifiedInput {
                    setup_id: "SETUP-V3-EXACT".to_owned(),
                    revision_id: "SETUP-V3-EXACT-rev-0001".to_owned(),
                    expected_definition_checksum: checksum.clone(),
                    context: context("op-v3-qualified"),
                },
            )
            .unwrap(),
        );
        assert!(qualified["station_setup"]["active_draft_revision"].is_null());
        assert_eq!(
            qualified["station_setup"]["current_qualified_revision"]["status"],
            "qualified"
        );

        let before_refusal = evidence_counts(&storage_root);
        let edit_refusal = replace_station_setup_draft_definition(
            &storage_root,
            ReplaceStationSetupDraftInput {
                setup_id: "SETUP-V3-EXACT".to_owned(),
                revision_id: "SETUP-V3-EXACT-rev-0001".to_owned(),
                expected_definition_checksum: checksum.clone(),
                definition_json: definition.to_string(),
                context: context("op-v3-edit-qualified"),
            },
        )
        .unwrap_err();
        assert_eq!(edit_refusal.code, "station_setup_revision_not_editable");
        assert_eq!(evidence_counts(&storage_root), before_refusal);

        let ready_refusal = mark_station_setup_revision_ready(
            &storage_root,
            MarkStationSetupReadyInput {
                setup_id: "SETUP-V3-EXACT".to_owned(),
                revision_id: "SETUP-V3-EXACT-rev-0001".to_owned(),
                expected_definition_checksum: checksum.clone(),
                context: context("op-v3-ready-refused"),
            },
        )
        .unwrap_err();
        assert_eq!(ready_refusal.code, "station_setup_not_ready");
        assert_eq!(evidence_counts(&storage_root), before_refusal);

        let derived = json_value(
            &derive_station_setup_revision(
                &storage_root,
                DeriveStationSetupRevisionInput {
                    setup_id: "SETUP-V3-EXACT".to_owned(),
                    source_revision_id: "SETUP-V3-EXACT-rev-0001".to_owned(),
                    upgrade_to_v3: false,
                    context: context("op-v3-finalize-assignments"),
                },
            )
            .unwrap(),
        );
        assert_eq!(
            derived["station_setup"]["active_draft_revision"]["parent_revision_id"],
            "SETUP-V3-EXACT-rev-0001"
        );
        assert_eq!(
            derived["station_setup"]["active_draft_revision"]["definition_checksum"],
            checksum
        );
        assert_eq!(
            derived["station_setup"]["current_qualified_revision"]["revision_id"],
            "SETUP-V3-EXACT-rev-0001"
        );
        let _ = std::fs::remove_dir_all(storage_root);
    }

    #[test]
    fn v3_readiness_revalidates_physical_port_assignments() {
        let storage_root = initialized_storage("station-v3-physical-readiness");
        insert_location(&storage_root, "LOC-STATION-PORTS", "Labo ports");
        let model_checksum = seed_bidirectional_test_model(&storage_root);
        register_station_test_asset(
            &storage_root,
            "SA-SOURCE-001",
            "SRC-001",
            "LOC-STATION-PORTS",
            &model_checksum,
        );
        register_station_test_asset(
            &storage_root,
            "SA-SINK-001",
            "SINK-001",
            "LOC-STATION-PORTS",
            &model_checksum,
        );
        let created = json_value(
            &create_station_setup(
                &storage_root,
                create_input("SETUP-V3-PORTS", "LOC-STATION-PORTS", "op-v3-ports-create"),
            )
            .unwrap(),
        );
        let draft = &created["station_setup"]["active_draft_revision"];
        let mut definition = draft["definition"].clone();
        definition["material_requirements"] = json!([
            station_test_requirement("source", "Source", "SA-SOURCE-001", "output"),
            station_test_requirement("sink", "Récepteur", "SA-SINK-001", "input")
        ]);
        definition["material_assignments"] = json!([
            station_test_assignment(
                &storage_root,
                "source",
                "SA-SOURCE-001",
                "SRC-001",
                "rf_input",
                &model_checksum
            ),
            station_test_assignment(
                &storage_root,
                "sink",
                "SA-SINK-001",
                "SINK-001",
                "rf_input",
                &model_checksum
            )
        ]);
        definition["logical_connections"] = json!([{
            "connection_id": "rf-path",
            "label": "Source vers récepteur",
            "from": {"requirement_id": "source", "logical_port_id": "rf"},
            "to": {"requirement_id": "sink", "logical_port_id": "rf"}
        }]);

        let saved = json_value(
            &replace_station_setup_draft_definition(
                &storage_root,
                ReplaceStationSetupDraftInput {
                    setup_id: "SETUP-V3-PORTS".to_owned(),
                    revision_id: "SETUP-V3-PORTS-rev-0001".to_owned(),
                    expected_definition_checksum: draft["definition_checksum"]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                    definition_json: definition.to_string(),
                    context: context("op-v3-ports-invalid"),
                },
            )
            .unwrap(),
        );
        let issues = saved["station_setup"]["active_draft_revision"]["readiness"]["issues"]
            .as_array()
            .unwrap();
        assert!(issues.iter().any(|issue| {
            issue["code"] == "station_material_port_mapping_incompatible"
                && issue["binding_ids"] == json!(["source"])
        }));

        definition["material_assignments"][0]["selected_ports"][0]["actual_port_id"] =
            json!("rf_output");
        let corrected = json_value(
            &replace_station_setup_draft_definition(
                &storage_root,
                ReplaceStationSetupDraftInput {
                    setup_id: "SETUP-V3-PORTS".to_owned(),
                    revision_id: "SETUP-V3-PORTS-rev-0001".to_owned(),
                    expected_definition_checksum: saved["station_setup"]["active_draft_revision"]
                        ["definition_checksum"]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                    definition_json: definition.to_string(),
                    context: context("op-v3-ports-corrected"),
                },
            )
            .unwrap(),
        );
        assert_eq!(
            corrected["station_setup"]["active_draft_revision"]["readiness"]["ready"],
            true
        );

        definition["material_requirements"][0]["calibration_requirement"] = json!("required");
        let calibration_required = json_value(
            &replace_station_setup_draft_definition(
                &storage_root,
                ReplaceStationSetupDraftInput {
                    setup_id: "SETUP-V3-PORTS".to_owned(),
                    revision_id: "SETUP-V3-PORTS-rev-0001".to_owned(),
                    expected_definition_checksum: corrected["station_setup"]
                        ["active_draft_revision"]["definition_checksum"]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                    definition_json: definition.to_string(),
                    context: context("op-v3-calibration-required"),
                },
            )
            .unwrap(),
        );
        assert!(
            calibration_required["station_setup"]["active_draft_revision"]["readiness"]["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|issue| {
                    issue["code"] == "station_calibration_missing"
                        && issue["binding_ids"] == json!(["source"])
                })
        );
        let blocked_candidates = list_station_material_candidates(
            &storage_root,
            ListStationMaterialCandidatesInput {
                setup_id: "SETUP-V3-PORTS".to_owned(),
                revision_id: "SETUP-V3-PORTS-rev-0001".to_owned(),
                requirement_id: "source".to_owned(),
                planned_use_on: "2026-07-28".to_owned(),
                execution_mode: "investigation".to_owned(),
                laboratory_location_id: "LOC-STATION-PORTS".to_owned(),
                excluded_schedule_item_code: None,
            },
        )
        .unwrap();
        let source_candidate = blocked_candidates
            .candidates
            .iter()
            .find(|candidate| candidate.asset.asset_id == "SA-SOURCE-001")
            .unwrap();
        assert!(!source_candidate.operationally_eligible);
        assert!(source_candidate
            .operational_blockers
            .iter()
            .any(|reason| reason.code == "calibration_missing"));

        record_metrology_calibration(
            &storage_root,
            RecordCalibrationInput {
                event_id: "CAL-SA-SOURCE-001".to_owned(),
                asset_id: "SA-SOURCE-001".to_owned(),
                certificate_reference: "CERT-SA-SOURCE-001".to_owned(),
                calibrated_at: "2026-07-27".to_owned(),
                due_at: "2027-07-27".to_owned(),
                provider: "EMITECH".to_owned(),
                decision: "conforming".to_owned(),
                as_found_status: Some("conforming".to_owned()),
                as_left_status: Some("conforming".to_owned()),
                adjustment_performed: false,
                uncertainty_summary_json: "{\"summary\":\"0.5 dB\"}".to_owned(),
                traceability_reference: Some("SI-RF-001".to_owned()),
                comment: "Étalonnage du rôle source".to_owned(),
                document_manifest_json: None,
                recorded_by: "metrologue".to_owned(),
                context: MetrologyOperationContext {
                    actor: "metrologue".to_owned(),
                    reason: "Validation de l'étalonnage exigé par le montage".to_owned(),
                    operation_id: "op-v3-source-calibration".to_owned(),
                    correlation_id: "corr-v3-source-calibration".to_owned(),
                    device_id: "metrology-test".to_owned(),
                },
            },
        )
        .unwrap();
        let refreshed = json_value(
            &assess_station_setup_revision_json(
                &storage_root,
                "SETUP-V3-PORTS",
                "SETUP-V3-PORTS-rev-0001",
            )
            .unwrap(),
        );
        assert_eq!(refreshed["readiness"]["ready"], true);
        let eligible_candidates = list_station_material_candidates(
            &storage_root,
            ListStationMaterialCandidatesInput {
                setup_id: "SETUP-V3-PORTS".to_owned(),
                revision_id: "SETUP-V3-PORTS-rev-0001".to_owned(),
                requirement_id: "source".to_owned(),
                planned_use_on: "2026-07-28".to_owned(),
                execution_mode: "investigation".to_owned(),
                laboratory_location_id: "LOC-STATION-PORTS".to_owned(),
                excluded_schedule_item_code: None,
            },
        )
        .unwrap();
        assert!(
            eligible_candidates
                .candidates
                .iter()
                .find(|candidate| candidate.asset.asset_id == "SA-SOURCE-001")
                .unwrap()
                .assignable
        );
        let _ = std::fs::remove_dir_all(storage_root);
    }

    #[test]
    fn draft_replacement_cannot_change_definition_schema() {
        let storage_root = initialized_storage("station-schema-boundary");
        insert_location(&storage_root, "LOC-STATION-SCHEMA", "Labo schéma");
        let created = json_value(
            &create_station_setup(
                &storage_root,
                create_input("SETUP-SCHEMA", "LOC-STATION-SCHEMA", "op-schema-create"),
            )
            .unwrap(),
        );
        let draft = &created["station_setup"]["active_draft_revision"];
        let mut definition = draft["definition"].clone();
        definition["definition_schema_version"] = json!(STATION_SETUP_V2_DEFINITION_SCHEMA_VERSION);
        let before = evidence_counts(&storage_root);

        let refusal = replace_station_setup_draft_definition(
            &storage_root,
            ReplaceStationSetupDraftInput {
                setup_id: "SETUP-SCHEMA".to_owned(),
                revision_id: "SETUP-SCHEMA-rev-0001".to_owned(),
                expected_definition_checksum: draft["definition_checksum"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
                definition_json: definition.to_string(),
                context: context("op-schema-downgrade"),
            },
        )
        .unwrap_err();

        assert_eq!(
            refusal.code,
            "station_setup_schema_change_requires_derived_revision"
        );
        assert_eq!(evidence_counts(&storage_root), before);
        let _ = std::fs::remove_dir_all(storage_root);
    }

    fn initialized_storage(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "emc-locus-{name}-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        run_storage_action(
            StorageAction::Init,
            root.clone(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("storage/sqlite"),
        )
        .unwrap();
        root
    }

    fn insert_location(storage_root: &Path, location_id: &str, label: &str) {
        let connection = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        connection
            .execute(
                "INSERT INTO laboratory_locations
                 (location_id, label, description, status, revision, created_at, updated_at)
                 VALUES (?1, ?2, 'Lieu de test', 'active', 1,
                         '2026-07-27T08:00:00Z', '2026-07-27T08:00:00Z')",
                params![location_id, label],
            )
            .unwrap();
    }

    fn set_location(storage_root: &Path, location_id: &str, label: &str, status: &str) {
        let connection = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        connection
            .execute(
                "UPDATE laboratory_locations
                 SET label = ?2, status = ?3, revision = revision + 1
                 WHERE location_id = ?1",
                params![location_id, label, status],
            )
            .unwrap();
    }

    fn seed_bidirectional_test_model(storage_root: &Path) -> String {
        let definition = EquipmentModelDefinition {
            definition_schema_version: EQUIPMENT_MODEL_DEFINITION_SCHEMA_VERSION.to_owned(),
            manufacturer: "Locus Instruments".to_owned(),
            model_name: "RF-IO".to_owned(),
            variant: None,
            equipment_class: EquipmentClass::ManualEquipment,
            functional_role: FunctionalRole::MeasurementInstrument,
            category_code: "emi_receiver".to_owned(),
            signal_domains: vec![SignalDomain::Rf],
            technology_tags: vec![TechnologyTag::Rf50Ohm],
            specifications: Vec::new(),
            signal_ports: vec![
                station_test_port(
                    "rf_input",
                    "Entrée RF",
                    PortDirectionality::Input,
                    PortFlowRole::MeasurementPort,
                ),
                station_test_port(
                    "rf_output",
                    "Sortie RF",
                    PortDirectionality::Output,
                    PortFlowRole::SourcePort,
                ),
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
        let equipment = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        equipment
            .execute(
                "INSERT INTO equipment_model_identities
                 (equipment_model_id, manufacturer, model_name, equipment_class, category_code,
                  current_approved_revision_id, created_by, created_at, updated_at)
                 VALUES ('EQM-RF-IO', 'Locus Instruments', 'RF-IO', 'manual_equipment',
                         'emi_receiver', 'EQM-RF-IO-rev-0001', 'catalogue', ?1, ?1)",
                ["2026-07-27T08:00:00Z"],
            )
            .unwrap();
        equipment
            .execute(
                "INSERT INTO equipment_model_revisions
                 (revision_id, equipment_model_id, revision_number, status,
                  definition_schema_version, definition_json, definition_checksum, created_by,
                  created_at, updated_at, submitted_at, approved_at)
                 VALUES ('EQM-RF-IO-rev-0001', 'EQM-RF-IO', 1, 'approved', ?1, ?2, ?3,
                         'catalogue', ?4, ?4, ?4, ?4)",
                params![
                    canonical.definition_schema_version,
                    canonical.canonical_json,
                    canonical.definition_checksum,
                    "2026-07-27T08:00:00Z"
                ],
            )
            .unwrap();
        canonical.definition_checksum
    }

    fn station_test_port(
        port_id: &str,
        label: &str,
        directionality: PortDirectionality,
        flow_role: PortFlowRole,
    ) -> SignalPortDefinition {
        SignalPortDefinition {
            port_id: port_id.to_owned(),
            label: label.to_owned(),
            directionality,
            flow_role,
            signal_domain: SignalDomain::Rf,
            required: true,
            connector_type: Some("N".to_owned()),
            technology_tags: vec![TechnologyTag::Rf50Ohm],
            quantity: PhysicalQuantity::Voltage,
            unit: "V".to_owned(),
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
        }
    }

    fn register_station_test_asset(
        storage_root: &Path,
        asset_id: &str,
        serial_number: &str,
        location_id: &str,
        model_checksum: &str,
    ) {
        register_metrology_instrument(
            storage_root,
            RegisterInstrumentInput {
                asset_id: asset_id.to_owned(),
                family: "Matériel RF".to_owned(),
                category_code: Some("emi_receiver".to_owned()),
                equipment_model_id: Some("EQM-RF-IO".to_owned()),
                equipment_model_revision_id: Some("EQM-RF-IO-rev-0001".to_owned()),
                equipment_model_checksum: Some(model_checksum.to_owned()),
                manufacturer: "Locus Instruments".to_owned(),
                model: "RF-IO".to_owned(),
                serial_number: serial_number.to_owned(),
                part_number: Some("RF-IO".to_owned()),
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
                    operation_id: format!("op-register-{asset_id}"),
                    correlation_id: format!("corr-register-{asset_id}"),
                    device_id: "metrology-test".to_owned(),
                },
            },
        )
        .unwrap();
        let equipment = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        equipment
            .execute(
                "UPDATE physical_assets
                 SET laboratory_location_id = ?2,
                     laboratory_location_label_snapshot = 'Labo ports',
                     revision = revision + 1,
                     updated_at = '2026-07-27T09:00:00Z'
                 WHERE asset_id = ?1",
                params![asset_id, location_id],
            )
            .unwrap();
    }

    fn station_test_requirement(
        requirement_id: &str,
        role_label: &str,
        asset_id: &str,
        directionality: &str,
    ) -> Value {
        json!({
            "requirement_id": requirement_id,
            "role_label": role_label,
            "required": true,
            "selection_policy": "exact_asset",
            "assignment_stage": "setup_definition",
            "substitution_policy": "no_substitution",
            "calibration_requirement": "not_required",
            "exact_asset_id": asset_id,
            "logical_ports": [{
                "logical_port_id": "rf",
                "label": "Port RF",
                "directionality": directionality,
                "signal_domain": "rf",
                "connector_requirement": "N",
                "impedance_ohm": 50.0
            }]
        })
    }

    fn station_test_assignment(
        storage_root: &Path,
        requirement_id: &str,
        asset_id: &str,
        inventory_code: &str,
        actual_port_id: &str,
        model_checksum: &str,
    ) -> Value {
        let equipment = Connection::open(storage_root.join("equipment.sqlite")).unwrap();
        let revision: u64 = equipment
            .query_row(
                "SELECT revision FROM physical_assets WHERE asset_id = ?1",
                [asset_id],
                |row| row.get(0),
            )
            .unwrap();
        json!({
            "requirement_id": requirement_id,
            "asset_id": asset_id,
            "asset_revision": revision.to_string(),
            "inventory_code": inventory_code,
            "serial_number": inventory_code,
            "equipment_model_id": "EQM-RF-IO",
            "equipment_model_revision_id": "EQM-RF-IO-rev-0001",
            "equipment_model_checksum": model_checksum,
            "selected_ports": [{"logical_port_id": "rf", "actual_port_id": actual_port_id}],
            "assignment_context": "station_setup",
            "assigned_on": "2026-07-27"
        })
    }

    fn create_input(
        setup_id: &str,
        location_id: &str,
        operation_id: &str,
    ) -> CreateStationSetupInput {
        CreateStationSetupInput {
            setup_id: setup_id.to_owned(),
            label: "Montage atomique".to_owned(),
            laboratory_location_id: location_id.to_owned(),
            planned_use_on: "2026-07-28".to_owned(),
            execution_mode: "investigation".to_owned(),
            context: context(operation_id),
        }
    }

    fn context(operation_id: &str) -> StationOperationContext {
        StationOperationContext {
            actor: "station.technician".to_owned(),
            reason: "Test de l'atomicité lieu et montage".to_owned(),
            operation_id: operation_id.to_owned(),
            device_id: "test-device".to_owned(),
            correlation_id: operation_id.to_owned(),
        }
    }

    fn evidence_counts(storage_root: &Path) -> (u64, u64, u64, u64, u64) {
        let station = Connection::open(storage_root.join("station.sqlite")).unwrap();
        let sync = Connection::open(storage_root.join("sync.sqlite")).unwrap();
        (
            station
                .query_row("SELECT COUNT(*) FROM station_setup_identities", [], |row| {
                    row.get(0)
                })
                .unwrap(),
            station
                .query_row("SELECT COUNT(*) FROM station_setup_revisions", [], |row| {
                    row.get(0)
                })
                .unwrap(),
            station
                .query_row(
                    "SELECT COUNT(*) FROM station_setup_audit_events",
                    [],
                    |row| row.get(0),
                )
                .unwrap(),
            station
                .query_row("SELECT COUNT(*) FROM station_setup_operations", [], |row| {
                    row.get(0)
                })
                .unwrap(),
            sync.query_row(
                "SELECT COUNT(*) FROM sync_operations WHERE domain = 'station_configurations'",
                [],
                |row| row.get(0),
            )
            .unwrap(),
        )
    }

    fn json_value(payload: &str) -> Value {
        serde_json::from_str(payload).unwrap()
    }
}
