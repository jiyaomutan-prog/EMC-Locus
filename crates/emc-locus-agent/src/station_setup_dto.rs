use crate::station_setup_repository::{
    StoredStationSetupAuditEvent, StoredStationSetupIdentity, StoredStationSetupRevision,
};
use emc_locus_core::{StationMeasurementSetupDefinition, StationSetupReadiness};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StationSetupIdentityDto {
    pub(crate) setup_id: String,
    pub(crate) label: String,
    pub(crate) current_ready_revision_id: Option<String>,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StationSetupRevisionDto {
    pub(crate) revision_id: String,
    pub(crate) setup_id: String,
    pub(crate) revision_number: u32,
    pub(crate) parent_revision_id: Option<String>,
    pub(crate) status: String,
    pub(crate) definition_schema_version: String,
    pub(crate) definition: StationMeasurementSetupDefinition,
    pub(crate) definition_checksum: String,
    pub(crate) readiness: StationSetupReadiness,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) ready_at: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StationSetupAggregateDto {
    pub(crate) identity: StationSetupIdentityDto,
    pub(crate) active_draft_revision: Option<StationSetupRevisionDto>,
    pub(crate) current_ready_revision: Option<StationSetupRevisionDto>,
    pub(crate) latest_revision: StationSetupRevisionDto,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StationSetupEnvelopeDto {
    pub(crate) station_setup: StationSetupAggregateDto,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StationSetupListDto {
    pub(crate) station_setups: Vec<StationSetupAggregateDto>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StationSetupRevisionEnvelopeDto {
    pub(crate) revision: StationSetupRevisionDto,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StationSetupRevisionListDto {
    pub(crate) setup_id: String,
    pub(crate) revisions: Vec<StationSetupRevisionDto>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StationSetupOperationResultDto {
    pub(crate) operation: String,
    pub(crate) operation_id: String,
    pub(crate) replayed: bool,
    pub(crate) station_setup: StationSetupAggregateDto,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StationSetupReadinessEnvelopeDto {
    pub(crate) setup_id: String,
    pub(crate) revision_id: String,
    pub(crate) readiness: StationSetupReadiness,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StationSetupAuditEventDto {
    pub(crate) audit_id: u64,
    pub(crate) setup_id: String,
    pub(crate) revision_id: Option<String>,
    pub(crate) action: String,
    pub(crate) actor: String,
    pub(crate) reason: String,
    pub(crate) old_revision_id: Option<String>,
    pub(crate) new_revision_id: Option<String>,
    pub(crate) old_definition_checksum: Option<String>,
    pub(crate) new_definition_checksum: Option<String>,
    pub(crate) operation_id: String,
    pub(crate) device_id: String,
    pub(crate) correlation_id: String,
    pub(crate) payload_json: String,
    pub(crate) occurred_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct StationSetupAuditListDto {
    pub(crate) setup_id: String,
    pub(crate) audit_events: Vec<StationSetupAuditEventDto>,
}

impl From<&StoredStationSetupIdentity> for StationSetupIdentityDto {
    fn from(value: &StoredStationSetupIdentity) -> Self {
        Self {
            setup_id: value.setup_id.clone(),
            label: value.label.clone(),
            current_ready_revision_id: value.current_ready_revision_id.clone(),
            created_by: value.created_by.clone(),
            created_at: value.created_at.clone(),
            updated_at: value.updated_at.clone(),
        }
    }
}

impl From<StoredStationSetupAuditEvent> for StationSetupAuditEventDto {
    fn from(value: StoredStationSetupAuditEvent) -> Self {
        Self {
            audit_id: value.audit_id,
            setup_id: value.setup_id,
            revision_id: value.revision_id,
            action: value.action,
            actor: value.actor,
            reason: value.reason,
            old_revision_id: value.old_revision_id,
            new_revision_id: value.new_revision_id,
            old_definition_checksum: value.old_definition_checksum,
            new_definition_checksum: value.new_definition_checksum,
            operation_id: value.operation_id,
            device_id: value.device_id,
            correlation_id: value.correlation_id,
            payload_json: value.payload_json,
            occurred_at: value.occurred_at,
        }
    }
}

pub(crate) fn revision_dto_unchecked(
    stored: &StoredStationSetupRevision,
    definition: StationMeasurementSetupDefinition,
    readiness: StationSetupReadiness,
) -> StationSetupRevisionDto {
    StationSetupRevisionDto {
        revision_id: stored.revision_id.clone(),
        setup_id: stored.setup_id.clone(),
        revision_number: stored.revision_number,
        parent_revision_id: stored.parent_revision_id.clone(),
        status: stored.status.clone(),
        definition_schema_version: stored.definition_schema_version.clone(),
        definition,
        definition_checksum: stored.definition_checksum.clone(),
        readiness,
        created_by: stored.created_by.clone(),
        created_at: stored.created_at.clone(),
        updated_at: stored.updated_at.clone(),
        ready_at: stored.ready_at.clone(),
    }
}
