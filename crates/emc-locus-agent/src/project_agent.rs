use super::{AgentCommand, AgentError};
use crate::project_service::{run_project_action, run_sync_action};
use std::{collections::BTreeMap, path::PathBuf};

#[cfg(test)]
use crate::project_service::{
    advance_to_test_planning, complete_review_item, contract_review_item_slug, create_project,
    list_audit_events, list_sync_outbox,
};
#[cfg(test)]
use std::path::Path;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectAction {
    Create(CreateProjectInput),
    List,
    Get { code: String },
    ContractReview { code: String },
    CompleteReviewItem(CompleteReviewItemInput),
    ToTestPlanning(AdvanceToTestPlanningInput),
    AuditEvents { code: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncAction {
    Outbox,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateProjectInput {
    pub code: String,
    pub customer_name: String,
    pub execution_mode: String,
    pub stage: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompleteReviewItemInput {
    pub code: String,
    pub item: String,
    pub actor: String,
    pub comment: Option<String>,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdvanceToTestPlanningInput {
    pub code: String,
    pub actor: String,
    pub reason: String,
    pub operation_id: String,
    pub correlation_id: String,
    pub device_id: String,
    pub deviation_authorized_by: Option<String>,
    pub deviation_reason: Option<String>,
}

pub(crate) fn parse_project_args<I>(mut args: I) -> Result<AgentCommand, AgentError>
where
    I: Iterator<Item = String>,
{
    let action = args
        .next()
        .ok_or_else(|| AgentError::new("missing_project_action", "missing project action"))?;
    let mut flags = parse_flags(args)?;
    let storage_root = required_path(&mut flags, "--storage-root")?;
    let action = match action.as_str() {
        "create" => {
            let operation_id = required_value(&mut flags, "--operation-id")?;
            let correlation_id = optional_value(&mut flags, "--correlation-id")
                .unwrap_or_else(|| operation_id.clone());
            ProjectAction::Create(CreateProjectInput {
                code: required_value(&mut flags, "--code")?,
                customer_name: required_value(&mut flags, "--customer-name")?,
                execution_mode: required_value(&mut flags, "--execution-mode")?,
                stage: optional_value(&mut flags, "--stage")
                    .unwrap_or_else(|| "contract_review".to_owned()),
                actor: required_value(&mut flags, "--actor")?,
                reason: required_value(&mut flags, "--reason")?,
                operation_id,
                correlation_id,
                device_id: optional_value(&mut flags, "--device-id")
                    .unwrap_or_else(|| "local-agent".to_owned()),
            })
        }
        "list" => ProjectAction::List,
        "get" => ProjectAction::Get {
            code: required_value(&mut flags, "--code")?,
        },
        "contract-review" => ProjectAction::ContractReview {
            code: required_value(&mut flags, "--code")?,
        },
        "complete-review-item" => {
            let operation_id = required_value(&mut flags, "--operation-id")?;
            let correlation_id = optional_value(&mut flags, "--correlation-id")
                .unwrap_or_else(|| operation_id.clone());
            ProjectAction::CompleteReviewItem(CompleteReviewItemInput {
                code: required_value(&mut flags, "--code")?,
                item: required_value(&mut flags, "--item")?,
                actor: required_value(&mut flags, "--actor")?,
                comment: optional_value(&mut flags, "--comment"),
                operation_id,
                correlation_id,
                device_id: optional_value(&mut flags, "--device-id")
                    .unwrap_or_else(|| "local-agent".to_owned()),
            })
        }
        "to-test-planning" => {
            let operation_id = required_value(&mut flags, "--operation-id")?;
            let correlation_id = optional_value(&mut flags, "--correlation-id")
                .unwrap_or_else(|| operation_id.clone());
            ProjectAction::ToTestPlanning(AdvanceToTestPlanningInput {
                code: required_value(&mut flags, "--code")?,
                actor: required_value(&mut flags, "--actor")?,
                reason: required_value(&mut flags, "--reason")?,
                operation_id,
                correlation_id,
                device_id: optional_value(&mut flags, "--device-id")
                    .unwrap_or_else(|| "local-agent".to_owned()),
                deviation_authorized_by: optional_value(&mut flags, "--deviation-authorized-by"),
                deviation_reason: optional_value(&mut flags, "--deviation-reason"),
            })
        }
        "audit-events" => ProjectAction::AuditEvents {
            code: required_value(&mut flags, "--code")?,
        },
        other => {
            return Err(AgentError::new(
                "unknown_project_action",
                format!("unknown project action: {other}"),
            ))
        }
    };
    ensure_no_unknown_flags(flags)?;

    Ok(AgentCommand::Projects {
        action,
        storage_root,
    })
}

pub(crate) fn parse_sync_args<I>(mut args: I) -> Result<AgentCommand, AgentError>
where
    I: Iterator<Item = String>,
{
    let action = args
        .next()
        .ok_or_else(|| AgentError::new("missing_sync_action", "missing sync action"))?;
    let mut flags = parse_flags(args)?;
    let storage_root = required_path(&mut flags, "--storage-root")?;
    let action = match action.as_str() {
        "outbox" => SyncAction::Outbox,
        other => {
            return Err(AgentError::new(
                "unknown_sync_action",
                format!("unknown sync action: {other}"),
            ))
        }
    };
    ensure_no_unknown_flags(flags)?;

    Ok(AgentCommand::Sync {
        action,
        storage_root,
    })
}

pub fn run_project_command(command: AgentCommand) -> Result<String, AgentError> {
    match command {
        AgentCommand::Projects {
            action,
            storage_root,
        } => run_project_action(action, storage_root),
        _ => Err(AgentError::new(
            "invalid_project_command",
            "expected a projects command",
        )),
    }
}

pub fn run_sync_command(command: AgentCommand) -> Result<String, AgentError> {
    match command {
        AgentCommand::Sync {
            action,
            storage_root,
        } => run_sync_action(action, storage_root),
        _ => Err(AgentError::new(
            "invalid_sync_command",
            "expected a sync command",
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
    Ok(PathBuf::from(required_value(flags, name)?))
}

fn required_value(
    flags: &mut BTreeMap<String, String>,
    name: &'static str,
) -> Result<String, AgentError> {
    optional_value(flags, name)
        .ok_or_else(|| AgentError::new("missing_argument", format!("missing {name}")))
}

fn optional_value(flags: &mut BTreeMap<String, String>, name: &str) -> Option<String> {
    flags.remove(name)
}

fn ensure_no_unknown_flags(flags: BTreeMap<String, String>) -> Result<(), AgentError> {
    if let Some(name) = flags.keys().next() {
        return Err(AgentError::new(
            "unknown_argument",
            format!("unknown argument: {name}"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{run_storage_action, StorageAction};
    use emc_locus_core::baseline_contract_review_items;

    #[test]
    fn parses_project_create_command() {
        let command = parse_project_args(
            [
                "create",
                "--storage-root",
                "E:/emc/data",
                "--code",
                "CEM-AGENT-001",
                "--customer-name",
                "Rail Lab",
                "--execution-mode",
                "accredited",
                "--actor",
                "quality.lead",
                "--reason",
                "contract accepted",
                "--operation-id",
                "op-create-001",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();

        assert!(matches!(command, AgentCommand::Projects { .. }));
    }

    #[test]
    fn creates_project_with_audit_and_outbox_transaction() {
        let storage_root = temporary_storage_root("agent-project-create");
        initialize_storage(&storage_root);

        let output = create_project(
            &storage_root,
            CreateProjectInput {
                code: "CEM-AGENT-001".to_owned(),
                customer_name: "Rail Lab".to_owned(),
                execution_mode: "accredited".to_owned(),
                stage: "contract_review".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "contract accepted".to_owned(),
                operation_id: "op-create-001".to_owned(),
                correlation_id: "corr-create-001".to_owned(),
                device_id: "station-a".to_owned(),
            },
        )
        .unwrap();
        let replay = create_project(
            &storage_root,
            CreateProjectInput {
                code: "CEM-AGENT-001".to_owned(),
                customer_name: "Rail Lab".to_owned(),
                execution_mode: "accredited".to_owned(),
                stage: "contract_review".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "contract accepted".to_owned(),
                operation_id: "op-create-001".to_owned(),
                correlation_id: "corr-create-001".to_owned(),
                device_id: "station-a".to_owned(),
            },
        )
        .unwrap();
        let outbox = list_sync_outbox(&storage_root).unwrap();
        let audits = list_audit_events(&storage_root, "CEM-AGENT-001").unwrap();

        assert!(output.contains("\"operation\":\"project_created\""));
        assert!(output.contains("\"revision\":\"rev-0001\""));
        assert!(replay.contains("\"replayed\":true"));
        assert!(outbox.contains("\"operation_kind\":\"project_created\""));
        assert!(audits.contains("\"action\":\"project_created\""));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn project_idempotency_replays_identical_payload_same_operation_id() {
        let storage_root = temporary_storage_root("agent-project-idempotent-replay");
        initialize_storage(&storage_root);
        let input = CreateProjectInput {
            code: "CEM-IDEMP-001".to_owned(),
            customer_name: "Idempotent Customer".to_owned(),
            execution_mode: "accredited".to_owned(),
            stage: "contract_review".to_owned(),
            actor: "quality.lead".to_owned(),
            reason: "contract accepted".to_owned(),
            operation_id: "op-idempotent-create".to_owned(),
            correlation_id: "corr-idempotent-create".to_owned(),
            device_id: "station-a".to_owned(),
        };

        let first = create_project(&storage_root, input.clone()).unwrap();
        let replay = create_project(&storage_root, input).unwrap();

        assert!(first.contains("\"replayed\":false"));
        assert!(replay.contains("\"replayed\":true"));
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn project_idempotency_rejects_different_payload_same_operation_id() {
        let storage_root = temporary_storage_root("agent-project-idempotent-mismatch");
        initialize_storage(&storage_root);
        create_project(
            &storage_root,
            CreateProjectInput {
                code: "CEM-IDEMP-002".to_owned(),
                customer_name: "Original Customer".to_owned(),
                execution_mode: "accredited".to_owned(),
                stage: "contract_review".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "contract accepted".to_owned(),
                operation_id: "op-idempotent-mismatch".to_owned(),
                correlation_id: "corr-idempotent-mismatch".to_owned(),
                device_id: "station-a".to_owned(),
            },
        )
        .unwrap();

        let error = create_project(
            &storage_root,
            CreateProjectInput {
                code: "CEM-IDEMP-002".to_owned(),
                customer_name: "Changed Customer".to_owned(),
                execution_mode: "accredited".to_owned(),
                stage: "contract_review".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "contract accepted".to_owned(),
                operation_id: "op-idempotent-mismatch".to_owned(),
                correlation_id: "corr-idempotent-mismatch".to_owned(),
                device_id: "station-a".to_owned(),
            },
        )
        .unwrap_err();

        assert_eq!(error.code, "operation_replay_mismatch");
        assert!(error.to_json().contains("expected_fingerprint"));
        assert!(error.to_json().contains("stored_fingerprint"));
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn rejects_planning_until_contract_review_is_complete() {
        let storage_root = temporary_storage_root("agent-project-gate");
        initialize_storage(&storage_root);
        create_project(
            &storage_root,
            CreateProjectInput {
                code: "CEM-GATE-001".to_owned(),
                customer_name: "Gate Customer".to_owned(),
                execution_mode: "accredited".to_owned(),
                stage: "contract_review".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "contract accepted".to_owned(),
                operation_id: "op-gate-create".to_owned(),
                correlation_id: "corr-gate-create".to_owned(),
                device_id: "station-a".to_owned(),
            },
        )
        .unwrap();

        let error = advance_to_test_planning(
            &storage_root,
            AdvanceToTestPlanningInput {
                code: "CEM-GATE-001".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "ready".to_owned(),
                operation_id: "op-gate-transition-early".to_owned(),
                correlation_id: "corr-gate-transition-early".to_owned(),
                device_id: "station-a".to_owned(),
                deviation_authorized_by: None,
                deviation_reason: None,
            },
        )
        .unwrap_err();

        assert_eq!(error.code, "contract_review_incomplete");
        assert!(error.to_json().contains("customer_request_defined"));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn advances_to_planning_after_required_review_items() {
        let storage_root = temporary_storage_root("agent-project-planning");
        initialize_storage(&storage_root);
        create_project(
            &storage_root,
            CreateProjectInput {
                code: "CEM-PLAN-001".to_owned(),
                customer_name: "Plan Customer".to_owned(),
                execution_mode: "accredited".to_owned(),
                stage: "contract_review".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "contract accepted".to_owned(),
                operation_id: "op-plan-create".to_owned(),
                correlation_id: "corr-plan-create".to_owned(),
                device_id: "station-a".to_owned(),
            },
        )
        .unwrap();
        for (index, item) in baseline_contract_review_items().iter().enumerate() {
            complete_review_item(
                &storage_root,
                CompleteReviewItemInput {
                    code: "CEM-PLAN-001".to_owned(),
                    item: contract_review_item_slug(*item).to_owned(),
                    actor: "quality.lead".to_owned(),
                    comment: None,
                    operation_id: format!("op-plan-review-{index}"),
                    correlation_id: format!("corr-plan-review-{index}"),
                    device_id: "station-a".to_owned(),
                },
            )
            .unwrap();
        }

        let output = advance_to_test_planning(
            &storage_root,
            AdvanceToTestPlanningInput {
                code: "CEM-PLAN-001".to_owned(),
                actor: "quality.lead".to_owned(),
                reason: "review complete".to_owned(),
                operation_id: "op-plan-transition".to_owned(),
                correlation_id: "corr-plan-transition".to_owned(),
                device_id: "station-a".to_owned(),
                deviation_authorized_by: None,
                deviation_reason: None,
            },
        )
        .unwrap();
        let outbox = list_sync_outbox(&storage_root).unwrap();

        assert!(output.contains("\"stage\":\"test_planning\""));
        assert!(outbox.contains("\"operation_kind\":\"project_stage_advanced\""));

        remove_temporary_storage_root(&storage_root);
    }

    fn initialize_storage(storage_root: &Path) {
        run_storage_action(
            StorageAction::Init,
            storage_root.to_path_buf(),
            repo_root().join("storage/sqlite"),
        )
        .unwrap();
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
