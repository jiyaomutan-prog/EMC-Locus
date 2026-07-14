use super::{AgentCommand, AgentError};
use crate::metrology_service::{
    assess_metrology_readiness, get_metrology_calibration_status, list_metrology_audit_events,
    list_metrology_calibrations, record_metrology_calibration, register_metrology_instrument,
    set_metrology_serviceability, AssessReadinessInput, MetrologyOperationContext,
    RecordCalibrationInput, RegisterInstrumentInput, SetServiceabilityInput,
};
use crate::{get_metrology_instrument, list_metrology_instruments};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetrologyAction {
    Register(Box<RegisterInstrumentInput>),
    List,
    Get {
        asset_id: String,
    },
    RecordCalibration(Box<RecordCalibrationInput>),
    ListCalibrations {
        asset_id: String,
    },
    Status {
        asset_id: String,
        checked_on: String,
    },
    SetServiceability(Box<SetServiceabilityInput>),
    Readiness(AssessReadinessInput),
    AuditEvents {
        entity_type: String,
        entity_id: String,
    },
}

pub(crate) fn parse_metrology_args<I>(mut args: I) -> Result<AgentCommand, AgentError>
where
    I: Iterator<Item = String>,
{
    let action = args
        .next()
        .ok_or_else(|| AgentError::new("missing_metrology_action", "missing metrology action"))?;
    let mut flags = parse_flags(args)?;
    let storage_root = required_path(&mut flags, "--storage-root")?;
    let action = match action.as_str() {
        "register-instrument" => MetrologyAction::Register(Box::new(RegisterInstrumentInput {
            asset_id: required_value(&mut flags, "--asset-id")?,
            family: required_value(&mut flags, "--family")?,
            category_code: Some(required_value(&mut flags, "--category-code")?),
            equipment_model_id: None,
            equipment_model_revision_id: None,
            equipment_model_checksum: None,
            manufacturer: required_value(&mut flags, "--manufacturer")?,
            model: required_value(&mut flags, "--model")?,
            serial_number: required_value(&mut flags, "--serial-number")?,
            part_number: optional_value(&mut flags, "--part-number"),
            calibration_requirement: required_value(&mut flags, "--calibration-requirement")?,
            calibration_period_months: optional_u32(&mut flags, "--calibration-period-months")?,
            calibration_due_warning_days: optional_u32(
                &mut flags,
                "--calibration-due-warning-days",
            )?,
            serviceability_status: optional_value(&mut flags, "--serviceability-status")
                .unwrap_or_else(|| "usable".to_owned()),
            serviceability_reason: optional_value(&mut flags, "--serviceability-reason")
                .unwrap_or_default(),
            capabilities_json: optional_value(&mut flags, "--capabilities-json")
                .unwrap_or_else(|| "[]".to_owned()),
            metrology_notes: optional_value(&mut flags, "--metrology-notes").unwrap_or_default(),
            context: operation_context_from_flags(&mut flags)?,
        })),
        "list-instruments" => MetrologyAction::List,
        "get-instrument" => MetrologyAction::Get {
            asset_id: required_value(&mut flags, "--asset-id")?,
        },
        "record-calibration" => {
            MetrologyAction::RecordCalibration(Box::new(RecordCalibrationInput {
                event_id: required_value(&mut flags, "--event-id")?,
                asset_id: required_value(&mut flags, "--asset-id")?,
                certificate_reference: required_value(&mut flags, "--certificate-reference")?,
                calibrated_at: required_value(&mut flags, "--calibrated-at")?,
                due_at: required_value(&mut flags, "--due-at")?,
                provider: required_value(&mut flags, "--provider")?,
                decision: optional_value(&mut flags, "--decision")
                    .unwrap_or_else(|| "conforming".to_owned()),
                as_found_status: optional_value(&mut flags, "--as-found-status"),
                as_left_status: optional_value(&mut flags, "--as-left-status"),
                adjustment_performed: optional_bool(&mut flags, "--adjustment-performed")?
                    .unwrap_or(false),
                uncertainty_summary_json: optional_value(&mut flags, "--uncertainty-summary-json")
                    .unwrap_or_else(|| "{}".to_owned()),
                traceability_reference: optional_value(&mut flags, "--traceability-reference"),
                comment: optional_value(&mut flags, "--comment").unwrap_or_default(),
                document_manifest_json: optional_value(&mut flags, "--document-manifest-json"),
                recorded_by: required_value(&mut flags, "--recorded-by")?,
                context: operation_context_from_flags(&mut flags)?,
            }))
        }
        "list-calibrations" => MetrologyAction::ListCalibrations {
            asset_id: required_value(&mut flags, "--asset-id")?,
        },
        "status" => MetrologyAction::Status {
            asset_id: required_value(&mut flags, "--asset-id")?,
            checked_on: required_value(&mut flags, "--checked-on")?,
        },
        "set-serviceability" => {
            MetrologyAction::SetServiceability(Box::new(SetServiceabilityInput {
                asset_id: required_value(&mut flags, "--asset-id")?,
                serviceability_status: required_value(&mut flags, "--serviceability-status")?,
                serviceability_reason: required_value(&mut flags, "--serviceability-reason")?,
                context: operation_context_from_flags(&mut flags)?,
            }))
        }
        "readiness" => MetrologyAction::Readiness(AssessReadinessInput {
            asset_ids: required_value(&mut flags, "--asset-ids")?
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect(),
            execution_mode: required_value(&mut flags, "--execution-mode")?,
            checked_on: required_value(&mut flags, "--checked-on")?,
            context: optional_value(&mut flags, "--context"),
        }),
        "audit-events" => MetrologyAction::AuditEvents {
            entity_type: optional_value(&mut flags, "--entity-type")
                .unwrap_or_else(|| "instrument".to_owned()),
            entity_id: required_value(&mut flags, "--entity-id")?,
        },
        other => {
            return Err(AgentError::new(
                "unknown_metrology_action",
                format!("unknown metrology action: {other}"),
            ))
        }
    };
    ensure_no_unknown_flags(flags)?;

    Ok(AgentCommand::Metrology {
        action,
        storage_root,
    })
}

pub fn run_metrology_command(command: AgentCommand) -> Result<String, AgentError> {
    match command {
        AgentCommand::Metrology {
            action,
            storage_root,
        } => match action {
            MetrologyAction::Register(input) => {
                register_metrology_instrument(&storage_root, *input)
            }
            MetrologyAction::List => list_metrology_instruments(&storage_root),
            MetrologyAction::Get { asset_id } => get_metrology_instrument(&storage_root, &asset_id),
            MetrologyAction::RecordCalibration(input) => {
                record_metrology_calibration(&storage_root, *input)
            }
            MetrologyAction::ListCalibrations { asset_id } => {
                list_metrology_calibrations(&storage_root, &asset_id)
            }
            MetrologyAction::Status {
                asset_id,
                checked_on,
            } => get_metrology_calibration_status(&storage_root, &asset_id, &checked_on),
            MetrologyAction::SetServiceability(input) => {
                set_metrology_serviceability(&storage_root, *input)
            }
            MetrologyAction::Readiness(input) => assess_metrology_readiness(&storage_root, input),
            MetrologyAction::AuditEvents {
                entity_type,
                entity_id,
            } => list_metrology_audit_events(&storage_root, &entity_type, &entity_id),
        },
        _ => Err(AgentError::new(
            "invalid_metrology_command",
            "expected a metrology command",
        )),
    }
}

fn parse_flags<I>(args: I) -> Result<BTreeMap<String, String>, AgentError>
where
    I: Iterator<Item = String>,
{
    let mut flags = BTreeMap::new();
    let mut args = args.peekable();
    while let Some(argument) = args.next() {
        if !argument.starts_with("--") {
            return Err(AgentError::new(
                "unknown_argument",
                format!("unknown argument: {argument}"),
            ));
        }
        let value = args.next().ok_or_else(|| {
            AgentError::new("missing_argument", format!("missing value for {argument}"))
        })?;
        flags.insert(argument, value);
    }
    Ok(flags)
}

fn required_path(
    flags: &mut BTreeMap<String, String>,
    name: &'static str,
) -> Result<PathBuf, AgentError> {
    required_value(flags, name).map(PathBuf::from)
}

fn required_value(
    flags: &mut BTreeMap<String, String>,
    name: &'static str,
) -> Result<String, AgentError> {
    flags
        .remove(name)
        .ok_or_else(|| AgentError::new("missing_argument", format!("missing required {name}")))
}

fn optional_value(flags: &mut BTreeMap<String, String>, name: &'static str) -> Option<String> {
    flags.remove(name)
}

fn optional_u32(
    flags: &mut BTreeMap<String, String>,
    name: &'static str,
) -> Result<Option<u32>, AgentError> {
    optional_value(flags, name)
        .map(|value| {
            value.parse::<u32>().map_err(|_| {
                AgentError::new(
                    "invalid_argument",
                    format!("{name} must be a positive integer"),
                )
            })
        })
        .transpose()
}

fn optional_bool(
    flags: &mut BTreeMap<String, String>,
    name: &'static str,
) -> Result<Option<bool>, AgentError> {
    optional_value(flags, name)
        .map(|value| match value.as_str() {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" => Ok(false),
            _ => Err(AgentError::new(
                "invalid_argument",
                format!("{name} must be true or false"),
            )),
        })
        .transpose()
}

fn operation_context_from_flags(
    flags: &mut BTreeMap<String, String>,
) -> Result<MetrologyOperationContext, AgentError> {
    let operation_id = required_value(flags, "--operation-id")?;
    Ok(MetrologyOperationContext {
        actor: required_value(flags, "--actor")?,
        reason: required_value(flags, "--reason")?,
        correlation_id: optional_value(flags, "--correlation-id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_value(flags, "--device-id").unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn ensure_no_unknown_flags(flags: BTreeMap<String, String>) -> Result<(), AgentError> {
    if let Some(unknown) = flags.keys().next() {
        return Err(AgentError::new(
            "unknown_argument",
            format!("unknown argument: {unknown}"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_metrology_register_command() {
        let command = parse_metrology_args(
            [
                "register-instrument",
                "--storage-root",
                "E:/emc-locus/data",
                "--asset-id",
                "SA-001",
                "--family",
                "SpectrumAnalyzer",
                "--category-code",
                "spectrum_analyzer",
                "--manufacturer",
                "Rohde Schwarz",
                "--model",
                "FSW",
                "--serial-number",
                "100001",
                "--calibration-requirement",
                "required",
                "--calibration-period-months",
                "12",
                "--actor",
                "metrology.admin",
                "--reason",
                "Initial registration",
                "--operation-id",
                "op-register-SA-001",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();

        assert_eq!(
            command,
            AgentCommand::Metrology {
                storage_root: PathBuf::from("E:/emc-locus/data"),
                action: MetrologyAction::Register(Box::new(RegisterInstrumentInput {
                    asset_id: "SA-001".to_owned(),
                    family: "SpectrumAnalyzer".to_owned(),
                    category_code: Some("spectrum_analyzer".to_owned()),
                    equipment_model_id: None,
                    equipment_model_revision_id: None,
                    equipment_model_checksum: None,
                    manufacturer: "Rohde Schwarz".to_owned(),
                    model: "FSW".to_owned(),
                    serial_number: "100001".to_owned(),
                    part_number: None,
                    calibration_requirement: "required".to_owned(),
                    calibration_period_months: Some(12),
                    calibration_due_warning_days: None,
                    serviceability_status: "usable".to_owned(),
                    serviceability_reason: String::new(),
                    capabilities_json: "[]".to_owned(),
                    metrology_notes: String::new(),
                    context: MetrologyOperationContext {
                        actor: "metrology.admin".to_owned(),
                        reason: "Initial registration".to_owned(),
                        operation_id: "op-register-SA-001".to_owned(),
                        correlation_id: "op-register-SA-001".to_owned(),
                        device_id: "local-agent".to_owned(),
                    },
                }))
            }
        );
    }
}
