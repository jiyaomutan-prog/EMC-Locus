use super::{
    build_health_report, render_json, run_metrology_command, run_project_command,
    run_storage_action, run_sync_command, AgentCommand, AgentError, MetrologyAction, ProjectAction,
    StorageAction, SyncAction,
};
use crate::metrology_service::{
    AssessReadinessInput, MetrologyOperationContext, RecordCalibrationInput,
    RegisterInstrumentInput, SetServiceabilityInput,
};
use crate::project_agent::{
    AdvanceToTestPlanningInput, CompleteReviewItemInput, CreateProjectInput,
};
use crate::test_execution_service::{
    get_simulated_test_execution, list_project_simulated_test_executions, run_simulated_emc_test,
    RunSimulatedEmcTestInput,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf};
use tiny_http::{Header, Response, Server, StatusCode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiServerConfig {
    pub bind: String,
    pub storage_root: PathBuf,
    pub migrations_root: PathBuf,
    pub max_requests: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiResponse {
    pub status: u16,
    pub body: String,
}

impl ApiServerConfig {
    pub fn default_for(storage_root: PathBuf) -> Self {
        Self {
            bind: "127.0.0.1:8765".to_owned(),
            storage_root,
            migrations_root: PathBuf::from("storage/sqlite"),
            max_requests: None,
        }
    }
}

pub(crate) fn parse_serve_args<I>(args: I) -> Result<AgentCommand, AgentError>
where
    I: Iterator<Item = String>,
{
    let mut flags = parse_flags(args)?;
    let storage_root = PathBuf::from(required_value(&mut flags, "--storage-root")?);
    let mut config = ApiServerConfig::default_for(storage_root);
    if let Some(bind) = optional_value(&mut flags, "--bind") {
        config.bind = bind;
    }
    if let Some(migrations_root) = optional_value(&mut flags, "--migrations-root") {
        config.migrations_root = PathBuf::from(migrations_root);
    }
    if let Some(max_requests) = optional_value(&mut flags, "--max-requests") {
        config.max_requests = Some(max_requests.parse::<usize>().map_err(|_| {
            AgentError::new(
                "invalid_argument",
                "--max-requests must be a positive integer",
            )
        })?);
    }
    ensure_no_unknown_flags(flags)?;
    Ok(AgentCommand::Serve { config })
}

pub fn run_local_api_server(config: ApiServerConfig) -> Result<(), AgentError> {
    let server = Server::http(&config.bind)
        .map_err(|error| AgentError::new("api_bind_failed", error.to_string()))?;
    println!(
        "{}",
        render_json(&json!({
            "agent": "emc-locus-agent",
            "api": "v1",
            "bind": server.server_addr().to_string(),
            "storage_root": config.storage_root.to_string_lossy(),
        }))
    );

    for (handled, mut request) in server.incoming_requests().enumerate() {
        let method = request.method().as_str().to_owned();
        let url = request.url().to_owned();
        let mut body = String::new();
        request
            .as_reader()
            .read_to_string(&mut body)
            .map_err(|error| AgentError::new("api_read_failed", error.to_string()))?;
        let response = handle_api_request(&method, &url, &body, &config);
        let content_type = Header::from_bytes("Content-Type", "application/json")
            .expect("static content-type header is valid");
        request
            .respond(
                Response::from_string(response.body)
                    .with_status_code(StatusCode(response.status))
                    .with_header(content_type),
            )
            .map_err(|error| AgentError::new("api_response_failed", error.to_string()))?;

        if config
            .max_requests
            .is_some_and(|max_requests| handled + 1 >= max_requests)
        {
            break;
        }
    }
    Ok(())
}

pub fn handle_api_request(
    method: &str,
    url: &str,
    body: &str,
    config: &ApiServerConfig,
) -> ApiResponse {
    match route_api_request(method, url, body, config) {
        Ok(body) => ApiResponse { status: 200, body },
        Err(error) => ApiResponse {
            status: status_for_error(error.code),
            body: error.to_json(),
        },
    }
}

fn route_api_request(
    method: &str,
    url: &str,
    body: &str,
    config: &ApiServerConfig,
) -> Result<String, AgentError> {
    let path = url.split_once('?').map_or(url, |(path, _)| path);
    let query = url.split_once('?').map(|(_, query)| query).unwrap_or("");
    let parts = path
        .trim_matches('/')
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if parts.as_slice() == ["api", "v1", "health"] && method == "GET" {
        return Ok(build_health_report(&config.storage_root).to_json());
    }
    if parts.as_slice() == ["api", "v1", "storage", "initialize"] && method == "POST" {
        return run_storage_action(
            StorageAction::Init,
            config.storage_root.clone(),
            config.migrations_root.clone(),
        )
        .map(|report| report.to_json());
    }
    if parts.as_slice() == ["api", "v1", "storage", "status"] && method == "GET" {
        return run_storage_action(
            StorageAction::Status,
            config.storage_root.clone(),
            config.migrations_root.clone(),
        )
        .map(|report| report.to_json());
    }
    if parts.as_slice() == ["api", "v1", "projects"] && method == "GET" {
        return run_project_command(AgentCommand::Projects {
            action: ProjectAction::List,
            storage_root: config.storage_root.clone(),
        });
    }
    if parts.as_slice() == ["api", "v1", "projects"] && method == "POST" {
        let payload = parse_json_body(body)?;
        return run_project_command(AgentCommand::Projects {
            action: ProjectAction::Create(create_project_input(&payload)?),
            storage_root: config.storage_root.clone(),
        });
    }
    if parts.as_slice() == ["api", "v1", "metrology", "instruments"] && method == "GET" {
        return run_metrology_command(AgentCommand::Metrology {
            action: MetrologyAction::List,
            storage_root: config.storage_root.clone(),
        });
    }
    if parts.as_slice() == ["api", "v1", "metrology", "instruments"] && method == "POST" {
        let payload = parse_json_body(body)?;
        return run_metrology_command(AgentCommand::Metrology {
            action: MetrologyAction::Register(Box::new(register_instrument_input(&payload)?)),
            storage_root: config.storage_root.clone(),
        });
    }
    if parts.as_slice() == ["api", "v1", "sync", "outbox"] && method == "GET" {
        return run_sync_command(AgentCommand::Sync {
            action: SyncAction::Outbox,
            storage_root: config.storage_root.clone(),
        });
    }
    if parts.as_slice() == ["api", "v1", "test-executions", "simulated-emc"] && method == "POST" {
        let payload = parse_json_body(body)?;
        return run_simulated_emc_test(&config.storage_root, simulated_emc_input(&payload)?);
    }

    match parts.as_slice() {
        ["api", "v1", "projects", code] if method == "GET" => {
            run_project_command(AgentCommand::Projects {
                action: ProjectAction::Get {
                    code: (*code).to_owned(),
                },
                storage_root: config.storage_root.clone(),
            })
        }
        ["api", "v1", "projects", code, "contract-review"] if method == "GET" => {
            run_project_command(AgentCommand::Projects {
                action: ProjectAction::ContractReview {
                    code: (*code).to_owned(),
                },
                storage_root: config.storage_root.clone(),
            })
        }
        ["api", "v1", "projects", code, "contract-review", "items", item, "complete"]
            if method == "POST" =>
        {
            let payload = parse_json_body(body)?;
            run_project_command(AgentCommand::Projects {
                action: ProjectAction::CompleteReviewItem(complete_review_item_input(
                    code, item, &payload,
                )?),
                storage_root: config.storage_root.clone(),
            })
        }
        ["api", "v1", "projects", code, "transitions", "to-test-planning"] if method == "POST" => {
            let payload = parse_json_body(body)?;
            run_project_command(AgentCommand::Projects {
                action: ProjectAction::ToTestPlanning(to_test_planning_input(code, &payload)?),
                storage_root: config.storage_root.clone(),
            })
        }
        ["api", "v1", "projects", code, "audit-events"] if method == "GET" => {
            run_project_command(AgentCommand::Projects {
                action: ProjectAction::AuditEvents {
                    code: (*code).to_owned(),
                },
                storage_root: config.storage_root.clone(),
            })
        }
        ["api", "v1", "projects", code, "test-executions"] if method == "GET" => {
            list_project_simulated_test_executions(&config.storage_root, code)
        }
        ["api", "v1", "test-executions", attempt_id] if method == "GET" => {
            get_simulated_test_execution(&config.storage_root, attempt_id)
        }
        ["api", "v1", "metrology", "instruments", asset_id] if method == "GET" => {
            run_metrology_command(AgentCommand::Metrology {
                action: MetrologyAction::Get {
                    asset_id: (*asset_id).to_owned(),
                },
                storage_root: config.storage_root.clone(),
            })
        }
        ["api", "v1", "metrology", "instruments", asset_id, "calibrations"] if method == "GET" => {
            run_metrology_command(AgentCommand::Metrology {
                action: MetrologyAction::ListCalibrations {
                    asset_id: (*asset_id).to_owned(),
                },
                storage_root: config.storage_root.clone(),
            })
        }
        ["api", "v1", "metrology", "instruments", asset_id, "calibrations"] if method == "POST" => {
            let payload = parse_json_body(body)?;
            run_metrology_command(AgentCommand::Metrology {
                action: MetrologyAction::RecordCalibration(Box::new(record_calibration_input(
                    asset_id, &payload,
                )?)),
                storage_root: config.storage_root.clone(),
            })
        }
        ["api", "v1", "metrology", "instruments", asset_id, "status"] if method == "GET" => {
            let checked_on = required_query_value(query, "checked_on")?;
            run_metrology_command(AgentCommand::Metrology {
                action: MetrologyAction::Status {
                    asset_id: (*asset_id).to_owned(),
                    checked_on,
                },
                storage_root: config.storage_root.clone(),
            })
        }
        ["api", "v1", "metrology", "instruments", asset_id, "serviceability"]
            if method == "POST" =>
        {
            let payload = parse_json_body(body)?;
            run_metrology_command(AgentCommand::Metrology {
                action: MetrologyAction::SetServiceability(Box::new(serviceability_input(
                    asset_id, &payload,
                )?)),
                storage_root: config.storage_root.clone(),
            })
        }
        ["api", "v1", "metrology", "readiness"] if method == "POST" => {
            let payload = parse_json_body(body)?;
            run_metrology_command(AgentCommand::Metrology {
                action: MetrologyAction::Readiness(readiness_input(&payload)?),
                storage_root: config.storage_root.clone(),
            })
        }
        ["api", "v1", "metrology", "instruments", asset_id, "audit-events"] if method == "GET" => {
            run_metrology_command(AgentCommand::Metrology {
                action: MetrologyAction::AuditEvents {
                    entity_type: "instrument".to_owned(),
                    entity_id: (*asset_id).to_owned(),
                },
                storage_root: config.storage_root.clone(),
            })
        }
        _ => Err(AgentError::new(
            "api_route_not_found",
            format!("route not found: {method} {path}"),
        )),
    }
}

fn parse_json_body(body: &str) -> Result<Value, AgentError> {
    if body.trim().is_empty() {
        return Err(AgentError::new(
            "invalid_json_body",
            "request body must be a JSON object",
        ));
    }
    let value = serde_json::from_str::<Value>(body)
        .map_err(|error| AgentError::new("invalid_json_body", error.to_string()))?;
    if value.as_object().is_none() {
        return Err(AgentError::new(
            "invalid_json_body",
            "request body must be a JSON object",
        ));
    }
    Ok(value)
}

fn create_project_input(payload: &Value) -> Result<CreateProjectInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CreateProjectInput {
        code: required_string(payload, "code")?,
        customer_name: required_string(payload, "customer_name")?,
        execution_mode: required_string(payload, "execution_mode")?,
        stage: optional_string(payload, "stage").unwrap_or_else(|| "contract_review".to_owned()),
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn complete_review_item_input(
    code: &str,
    item: &str,
    payload: &Value,
) -> Result<CompleteReviewItemInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CompleteReviewItemInput {
        code: code.to_owned(),
        item: item.to_owned(),
        actor: required_string(payload, "actor")?,
        comment: optional_string(payload, "comment"),
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn to_test_planning_input(
    code: &str,
    payload: &Value,
) -> Result<AdvanceToTestPlanningInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(AdvanceToTestPlanningInput {
        code: code.to_owned(),
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        deviation_authorized_by: optional_string(payload, "deviation_authorized_by"),
        deviation_reason: optional_string(payload, "deviation_reason"),
        operation_id,
    })
}

fn register_instrument_input(payload: &Value) -> Result<RegisterInstrumentInput, AgentError> {
    Ok(RegisterInstrumentInput {
        asset_id: required_string(payload, "asset_id")?,
        family: required_string(payload, "family")?,
        category_code: required_string(payload, "category_code")?,
        manufacturer: required_string(payload, "manufacturer")?,
        model: required_string(payload, "model")?,
        serial_number: required_string(payload, "serial_number")?,
        part_number: optional_string(payload, "part_number"),
        calibration_requirement: required_string(payload, "calibration_requirement")?,
        calibration_period_months: optional_u32(payload, "calibration_period_months")?,
        calibration_due_warning_days: optional_u32(payload, "calibration_due_warning_days")?,
        serviceability_status: optional_string(payload, "serviceability_status")
            .unwrap_or_else(|| "usable".to_owned()),
        serviceability_reason: optional_string(payload, "serviceability_reason")
            .unwrap_or_default(),
        capabilities_json: payload
            .get("capabilities")
            .map(render_json)
            .or_else(|| optional_string(payload, "capabilities_json"))
            .unwrap_or_else(|| "[]".to_owned()),
        metrology_notes: optional_string(payload, "metrology_notes").unwrap_or_default(),
        context: operation_context(payload)?,
    })
}

fn record_calibration_input(
    asset_id: &str,
    payload: &Value,
) -> Result<RecordCalibrationInput, AgentError> {
    Ok(RecordCalibrationInput {
        event_id: required_string(payload, "event_id")?,
        asset_id: asset_id.to_owned(),
        certificate_reference: required_string(payload, "certificate_reference")?,
        calibrated_at: required_string(payload, "calibrated_at")?,
        due_at: required_string(payload, "due_at")?,
        provider: required_string(payload, "provider")?,
        decision: optional_string(payload, "decision").unwrap_or_else(|| "conforming".to_owned()),
        as_found_status: optional_string(payload, "as_found_status"),
        as_left_status: optional_string(payload, "as_left_status"),
        adjustment_performed: optional_bool(payload, "adjustment_performed")?.unwrap_or(false),
        uncertainty_summary_json: payload
            .get("uncertainty_summary")
            .map(render_json)
            .or_else(|| optional_string(payload, "uncertainty_summary_json"))
            .unwrap_or_else(|| "{}".to_owned()),
        traceability_reference: optional_string(payload, "traceability_reference"),
        comment: optional_string(payload, "comment").unwrap_or_default(),
        document_manifest_json: payload
            .get("document_manifest")
            .map(render_json)
            .or_else(|| optional_string(payload, "document_manifest_json")),
        recorded_by: required_string(payload, "recorded_by")?,
        context: operation_context(payload)?,
    })
}

fn serviceability_input(
    asset_id: &str,
    payload: &Value,
) -> Result<SetServiceabilityInput, AgentError> {
    Ok(SetServiceabilityInput {
        asset_id: asset_id.to_owned(),
        serviceability_status: required_string(payload, "serviceability_status")?,
        serviceability_reason: required_string(payload, "serviceability_reason")?,
        context: operation_context(payload)?,
    })
}

fn readiness_input(payload: &Value) -> Result<AssessReadinessInput, AgentError> {
    let asset_ids = payload
        .get("asset_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AgentError::with_details(
                "missing_json_field",
                "asset_ids is required",
                json!({ "field": "asset_ids" }),
            )
        })?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    Ok(AssessReadinessInput {
        asset_ids,
        execution_mode: required_string(payload, "execution_mode")?,
        checked_on: required_string(payload, "checked_on")?,
        context: optional_string(payload, "context"),
    })
}

fn simulated_emc_input(payload: &Value) -> Result<RunSimulatedEmcTestInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    let required_asset_ids = payload
        .get("required_asset_ids")
        .or_else(|| payload.get("asset_ids"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AgentError::with_details(
                "missing_json_field",
                "required_asset_ids is required",
                json!({ "field": "required_asset_ids" }),
            )
        })?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();

    Ok(RunSimulatedEmcTestInput {
        attempt_id: required_string(payload, "attempt_id")?,
        project_code: required_string(payload, "project_code")?,
        test_method_reference: required_string(payload, "test_method_reference")?,
        execution_mode: required_string(payload, "execution_mode")?,
        required_asset_ids,
        operator: required_string(payload, "operator")?,
        checked_on: required_string(payload, "checked_on")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn operation_context(payload: &Value) -> Result<MetrologyOperationContext, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(MetrologyOperationContext {
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn required_string(payload: &Value, key: &'static str) -> Result<String, AgentError> {
    optional_string(payload, key).ok_or_else(|| {
        AgentError::with_details(
            "missing_json_field",
            format!("missing required JSON field: {key}"),
            json!({ "field": key }),
        )
    })
}

fn required_query_value(query: &str, key: &'static str) -> Result<String, AgentError> {
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (candidate, value) = pair.split_once('=').unwrap_or((pair, ""));
        if candidate == key && !value.trim().is_empty() {
            return Ok(value.to_owned());
        }
    }
    Err(AgentError::with_details(
        "missing_query_field",
        format!("{key} query parameter is required"),
        json!({ "field": key }),
    ))
}

fn optional_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn optional_u32(payload: &Value, key: &str) -> Result<Option<u32>, AgentError> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    let Some(value) = value.as_u64() else {
        return Err(AgentError::with_details(
            "invalid_json_field",
            format!("{key} must be a positive integer"),
            json!({ "field": key }),
        ));
    };
    if value == 0 || value > u64::from(u32::MAX) {
        return Err(AgentError::with_details(
            "invalid_json_field",
            format!("{key} must be a positive integer"),
            json!({ "field": key }),
        ));
    }
    Ok(Some(value as u32))
}

fn optional_bool(payload: &Value, key: &str) -> Result<Option<bool>, AgentError> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    let Some(value) = value.as_bool() else {
        return Err(AgentError::with_details(
            "invalid_json_field",
            format!("{key} must be a boolean"),
            json!({ "field": key }),
        ));
    };
    Ok(Some(value))
}

fn status_for_error(code: &str) -> u16 {
    match code {
        "api_route_not_found"
        | "project_not_found"
        | "test_execution_not_found"
        | "metrology_instrument_not_found" => 404,
        "contract_review_incomplete"
        | "invalid_project_transition"
        | "project_already_exists"
        | "test_execution_attempt_exists"
        | "operation_replay_mismatch"
        | "metrology_instrument_already_exists"
        | "metrology_calibration_already_exists" => 409,
        "storage_not_initialized" => 503,
        "invalid_json_body"
        | "missing_json_field"
        | "missing_query_field"
        | "invalid_json_field"
        | "invalid_metrology_date"
        | "missing_argument"
        | "invalid_project_code"
        | "invalid_customer_name"
        | "invalid_actor"
        | "invalid_reason"
        | "domain_error"
        | "invalid_test_execution"
        | "invalid_metrology_calibration"
        | "invalid_metrology_instrument"
        | "invalid_metrology_readiness"
        | "unknown_execution_mode"
        | "unknown_contract_review_item" => 400,
        _ => 500,
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
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn local_api_runs_project_vertical_slice_through_routes() {
        let storage_root = temporary_storage_root("agent-api-slice");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            max_requests: None,
        };
        let initialized = handle_api_request("POST", "/api/v1/storage/initialize", "", &config);
        assert_eq!(initialized.status, 200);
        let storage_status = handle_api_request("GET", "/api/v1/storage/status", "", &config);
        assert_eq!(storage_status.status, 200);
        assert!(storage_status.body.contains("\"action\":\"status\""));
        assert!(storage_status.body.contains("\"status\":\"current\""));

        let created = handle_api_request(
            "POST",
            "/api/v1/projects",
            r#"{
                "code": "CEM-API-001",
                "customer_name": "API Customer",
                "execution_mode": "accredited",
                "actor": "quality.lead",
                "reason": "contract accepted",
                "operation_id": "op-api-create"
            }"#,
            &config,
        );
        assert_eq!(created.status, 200);
        assert!(created.body.contains("\"stage\":\"contract_review\""));

        let early = handle_api_request(
            "POST",
            "/api/v1/projects/CEM-API-001/transitions/to-test-planning",
            r#"{
                "actor": "quality.lead",
                "reason": "ready too soon",
                "operation_id": "op-api-early"
            }"#,
            &config,
        );
        assert_eq!(early.status, 409);
        assert!(early.body.contains("contract_review_incomplete"));

        for (index, item) in [
            "customer_request_defined",
            "test_method_selected",
            "laboratory_capability_confirmed",
            "equipment_availability_checked",
            "calibration_status_reviewed",
            "impartiality_risks_reviewed",
            "data_retention_agreed",
            "report_requirements_agreed",
            "deviations_recorded",
        ]
        .iter()
        .enumerate()
        {
            let body = format!(
                r#"{{"actor":"quality.lead","comment":"ok","operation_id":"op-api-review-{index}"}}"#
            );
            let completed = handle_api_request(
                "POST",
                &format!("/api/v1/projects/CEM-API-001/contract-review/items/{item}/complete"),
                &body,
                &config,
            );
            assert_eq!(completed.status, 200);
        }

        let transition = handle_api_request(
            "POST",
            "/api/v1/projects/CEM-API-001/transitions/to-test-planning",
            r#"{
                "actor": "quality.lead",
                "reason": "review complete",
                "operation_id": "op-api-transition"
            }"#,
            &config,
        );
        assert_eq!(transition.status, 200);
        assert!(transition.body.contains("\"stage\":\"test_planning\""));

        let outbox = handle_api_request("GET", "/api/v1/sync/outbox", "", &config);
        let audit = handle_api_request(
            "GET",
            "/api/v1/projects/CEM-API-001/audit-events",
            "",
            &config,
        );

        assert_eq!(outbox.status, 200);
        assert_eq!(audit.status, 200);
        assert!(outbox
            .body
            .contains("\"operation_kind\":\"project_stage_advanced\""));
        assert!(audit.body.contains("\"action\":\"project_stage_advanced\""));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_reports_storage_status_before_initialization() {
        let storage_root = temporary_storage_root("agent-api-storage-status");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            max_requests: None,
        };

        let response = handle_api_request("GET", "/api/v1/storage/status", "", &config);

        assert_eq!(response.status, 200);
        assert!(response.body.contains("\"status\":\"missing\""));
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_reports_not_found_for_unknown_routes() {
        let config = ApiServerConfig::default_for(PathBuf::from("unused"));
        let response = handle_api_request("GET", "/api/v1/unknown", "", &config);

        assert_eq!(response.status, 404);
        assert!(response.body.contains("api_route_not_found"));
    }

    #[test]
    fn local_api_rejects_invalid_json_payloads() {
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: PathBuf::from("unused"),
            migrations_root: repo_root().join("storage/sqlite"),
            max_requests: None,
        };

        let response = handle_api_request("POST", "/api/v1/projects", "{bad-json", &config);

        assert_eq!(response.status, 400);
        assert!(response.body.contains("invalid_json_body"));
    }

    #[test]
    fn local_api_registers_and_lists_metrology_instruments() {
        let storage_root = temporary_storage_root("agent-api-metrology");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            max_requests: None,
        };

        assert_eq!(
            handle_api_request("POST", "/api/v1/storage/initialize", "", &config).status,
            200
        );
        let created = handle_api_request(
            "POST",
            "/api/v1/metrology/instruments",
            r#"{
                "asset_id": "SA-API-001",
                "family": "SpectrumAnalyzer",
                "category_code": "spectrum_analyzer",
                "manufacturer": "Rohde Schwarz",
                "model": "FSW",
                "serial_number": "API-001",
                "part_number": "FSW44",
                "calibration_requirement": "required",
                "calibration_period_months": 12,
                "calibration_due_warning_days": 45,
                "capabilities": {"frequency_max_hz": 44000000000},
                "metrology_notes": "registered through local API",
                "actor": "metrology.admin",
                "reason": "initial API registration",
                "operation_id": "op-api-register-SA-API-001"
            }"#,
            &config,
        );
        assert_eq!(created.status, 200);
        assert!(created.body.contains("\"asset_id\":\"SA-API-001\""));
        assert!(created
            .body
            .contains("\"serviceability_status\":\"usable\""));

        let list = handle_api_request("GET", "/api/v1/metrology/instruments", "", &config);
        assert_eq!(list.status, 200);
        assert!(list.body.contains("\"instruments\""));
        assert!(list.body.contains("\"SA-API-001\""));

        let detail = handle_api_request(
            "GET",
            "/api/v1/metrology/instruments/SA-API-001",
            "",
            &config,
        );
        assert_eq!(detail.status, 200);
        assert!(detail.body.contains("\"part_number\":\"FSW44\""));
        assert!(detail.body.contains("\"calibration_due_warning_days\":45"));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_records_calibration_and_computes_status() {
        let storage_root = temporary_storage_root("agent-api-metrology-calibration");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            max_requests: None,
        };

        assert_eq!(
            handle_api_request("POST", "/api/v1/storage/initialize", "", &config).status,
            200
        );
        assert_eq!(
            handle_api_request(
                "POST",
                "/api/v1/metrology/instruments",
                r#"{
                    "asset_id": "SA-API-CAL-001",
                    "family": "SpectrumAnalyzer",
                    "category_code": "spectrum_analyzer",
                    "manufacturer": "Rohde Schwarz",
                    "model": "FSW",
                    "serial_number": "API-CAL-001",
                    "calibration_requirement": "required",
                    "calibration_due_warning_days": 45,
                    "actor": "metrology.admin",
                    "reason": "initial API registration",
                    "operation_id": "op-api-register-SA-API-CAL-001"
                }"#,
                &config,
            )
            .status,
            200
        );

        let missing = handle_api_request(
            "GET",
            "/api/v1/metrology/instruments/SA-API-CAL-001/status?checked_on=2026-07-01",
            "",
            &config,
        );
        assert_eq!(missing.status, 200);
        assert!(missing.body.contains("\"calibration_status\":\"missing\""));

        let calibration = handle_api_request(
            "POST",
            "/api/v1/metrology/instruments/SA-API-CAL-001/calibrations",
            &format!(
                r#"{{
                    "event_id": "CAL-SA-API-CAL-001-2026",
                    "certificate_reference": "CERT-SA-API-CAL-001-2026",
                    "calibrated_at": "2026-06-30",
                    "due_at": "2027-06-30",
                    "provider": "Accredited Lab",
                    "decision": "conforming",
                    "uncertainty_summary": {{"level_db": 0.6}},
                    "document_manifest": {{
                        "object_id": "obj-cert-api",
                        "original_filename": "cert.pdf",
                        "mime_type": "application/pdf",
                        "size_bytes": 12,
                        "sha256": "{}",
                        "storage_key": "metrology/SA-API-CAL-001/cert.pdf",
                        "revision": "A"
                    }},
                    "recorded_by": "metrology.admin",
                    "actor": "metrology.admin",
                    "reason": "annual calibration",
                    "operation_id": "op-api-record-CAL-SA-API-CAL-001-2026"
                }}"#,
                "b".repeat(64)
            ),
            &config,
        );
        assert_eq!(calibration.status, 200);
        assert!(calibration
            .body
            .contains("\"event_id\":\"CAL-SA-API-CAL-001-2026\""));

        let valid = handle_api_request(
            "GET",
            "/api/v1/metrology/instruments/SA-API-CAL-001/status?checked_on=2026-07-01",
            "",
            &config,
        );
        assert_eq!(valid.status, 200);
        assert!(valid.body.contains("\"calibration_status\":\"valid\""));

        let due_soon = handle_api_request(
            "GET",
            "/api/v1/metrology/instruments/SA-API-CAL-001/status?checked_on=2027-06-01",
            "",
            &config,
        );
        assert_eq!(due_soon.status, 200);
        assert!(due_soon
            .body
            .contains("\"calibration_status\":\"due_soon\""));

        let list = handle_api_request(
            "GET",
            "/api/v1/metrology/instruments/SA-API-CAL-001/calibrations",
            "",
            &config,
        );
        assert_eq!(list.status, 200);
        assert!(list.body.contains("\"calibration_events\""));

        let ready = handle_api_request(
            "POST",
            "/api/v1/metrology/readiness",
            r#"{
                "asset_ids": ["SA-API-CAL-001"],
                "execution_mode": "accredited",
                "checked_on": "2026-07-01",
                "context": "pre-run check"
            }"#,
            &config,
        );
        assert_eq!(ready.status, 200);
        assert!(ready.body.contains("\"ready\":true"));

        let serviceability = handle_api_request(
            "POST",
            "/api/v1/metrology/instruments/SA-API-CAL-001/serviceability",
            r#"{
                "serviceability_status": "out_of_service",
                "serviceability_reason": "damaged input connector",
                "actor": "metrology.admin",
                "reason": "asset quarantine",
                "operation_id": "op-api-serviceability-SA-API-CAL-001"
            }"#,
            &config,
        );
        assert_eq!(serviceability.status, 200);
        assert!(serviceability
            .body
            .contains("\"serviceability_status\":\"out_of_service\""));

        let blocked = handle_api_request(
            "POST",
            "/api/v1/metrology/readiness",
            r#"{
                "asset_ids": ["SA-API-CAL-001"],
                "execution_mode": "accredited",
                "checked_on": "2026-07-01"
            }"#,
            &config,
        );
        assert_eq!(blocked.status, 200);
        assert!(blocked.body.contains("\"ready\":false"));
        assert!(blocked.body.contains("\"code\":\"out_of_service\""));

        let audit = handle_api_request(
            "GET",
            "/api/v1/metrology/instruments/SA-API-CAL-001/audit-events",
            "",
            &config,
        );
        assert_eq!(audit.status, 200);
        assert!(audit.body.contains("\"instrument_registered\""));
        assert!(audit.body.contains("\"instrument_serviceability_changed\""));

        let outbox = handle_api_request("GET", "/api/v1/sync/outbox", "", &config);
        assert_eq!(outbox.status, 200);
        assert!(outbox.body.contains("\"domain\":\"metrology\""));
        assert!(outbox
            .body
            .contains("\"operation_kind\":\"instrument_serviceability_changed\""));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_accepts_investigation_project_with_reduced_review_gate() {
        let storage_root = temporary_storage_root("agent-api-investigation");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            max_requests: None,
        };
        assert_eq!(
            handle_api_request("POST", "/api/v1/storage/initialize", "", &config).status,
            200
        );
        assert_eq!(
            handle_api_request(
                "POST",
                "/api/v1/projects",
                r#"{"code":"CEM-INV-001","customer_name":"Investigation Customer","execution_mode":"investigation","actor":"quality.lead","reason":"field investigation","operation_id":"op-inv-create"}"#,
                &config,
            )
            .status,
            200
        );

        for (index, item) in ["customer_request_defined", "deviations_recorded"]
            .iter()
            .enumerate()
        {
            let body = format!(
                r#"{{"actor":"quality.lead","comment":"ok","operation_id":"op-inv-review-{index}"}}"#
            );
            assert_eq!(
                handle_api_request(
                    "POST",
                    &format!("/api/v1/projects/CEM-INV-001/contract-review/items/{item}/complete"),
                    &body,
                    &config,
                )
                .status,
                200
            );
        }

        let transition = handle_api_request(
            "POST",
            "/api/v1/projects/CEM-INV-001/transitions/to-test-planning",
            r#"{"actor":"quality.lead","reason":"investigation scope accepted","operation_id":"op-inv-transition"}"#,
            &config,
        );

        assert_eq!(transition.status, 200);
        assert!(transition.body.contains("\"stage\":\"test_planning\""));
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_accepts_non_accredited_project_with_reduced_review_gate() {
        let storage_root = temporary_storage_root("agent-api-non-accredited");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            max_requests: None,
        };
        assert_eq!(
            handle_api_request("POST", "/api/v1/storage/initialize", "", &config).status,
            200
        );
        assert_eq!(
            handle_api_request(
                "POST",
                "/api/v1/projects",
                r#"{"code":"CEM-NAC-001","customer_name":"Non Accredited Customer","execution_mode":"non_accredited","actor":"quality.lead","reason":"non accredited service","operation_id":"op-nac-create"}"#,
                &config,
            )
            .status,
            200
        );

        for (index, item) in [
            "customer_request_defined",
            "test_method_selected",
            "laboratory_capability_confirmed",
            "deviations_recorded",
        ]
        .iter()
        .enumerate()
        {
            let body = format!(
                r#"{{"actor":"quality.lead","comment":"ok","operation_id":"op-nac-review-{index}"}}"#
            );
            assert_eq!(
                handle_api_request(
                    "POST",
                    &format!("/api/v1/projects/CEM-NAC-001/contract-review/items/{item}/complete"),
                    &body,
                    &config,
                )
                .status,
                200
            );
        }

        let transition = handle_api_request(
            "POST",
            "/api/v1/projects/CEM-NAC-001/transitions/to-test-planning",
            r#"{"actor":"quality.lead","reason":"non accredited review complete","operation_id":"op-nac-transition"}"#,
            &config,
        );

        assert_eq!(transition.status, 200);
        assert!(transition.body.contains("\"stage\":\"test_planning\""));
        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn real_http_server_persists_project_slice_after_restart() {
        let storage_root = temporary_storage_root("agent-api-real-server");
        let migrations_root = repo_root().join("storage/sqlite");
        let first_port = free_loopback_port();
        let first_address = format!("127.0.0.1:{first_port}");
        let first_server = spawn_server(ApiServerConfig {
            bind: first_address.clone(),
            storage_root: storage_root.clone(),
            migrations_root: migrations_root.clone(),
            max_requests: Some(17),
        });

        let health = wait_for_http(&first_address, "/api/v1/health");
        assert_eq!(health.0, 200);
        assert_eq!(
            http_request("POST", &first_address, "/api/v1/storage/initialize", "").0,
            200
        );
        assert_eq!(
            http_request(
                "POST",
                &first_address,
                "/api/v1/projects",
                r#"{"code":"CEM-E2E-001","customer_name":"E2E Customer","execution_mode":"accredited","actor":"quality.lead","reason":"contract accepted","operation_id":"op-e2e-create"}"#,
            )
            .0,
            200
        );
        let early = http_request(
            "POST",
            &first_address,
            "/api/v1/projects/CEM-E2E-001/transitions/to-test-planning",
            r#"{"actor":"quality.lead","reason":"too early","operation_id":"op-e2e-early"}"#,
        );
        assert_eq!(early.0, 409);
        assert!(early.1.contains("contract_review_incomplete"));

        for (index, item) in [
            "customer_request_defined",
            "test_method_selected",
            "laboratory_capability_confirmed",
            "equipment_availability_checked",
            "calibration_status_reviewed",
            "impartiality_risks_reviewed",
            "data_retention_agreed",
            "report_requirements_agreed",
            "deviations_recorded",
        ]
        .iter()
        .enumerate()
        {
            let body = format!(
                r#"{{"actor":"quality.lead","comment":"ok","operation_id":"op-e2e-review-{index}"}}"#
            );
            assert_eq!(
                http_request(
                    "POST",
                    &first_address,
                    &format!("/api/v1/projects/CEM-E2E-001/contract-review/items/{item}/complete"),
                    &body,
                )
                .0,
                200
            );
        }

        let transition = http_request(
            "POST",
            &first_address,
            "/api/v1/projects/CEM-E2E-001/transitions/to-test-planning",
            r#"{"actor":"quality.lead","reason":"review complete","operation_id":"op-e2e-transition"}"#,
        );
        assert_eq!(transition.0, 200);
        assert!(transition.1.contains("\"stage\":\"test_planning\""));
        let transition_replay = http_request(
            "POST",
            &first_address,
            "/api/v1/projects/CEM-E2E-001/transitions/to-test-planning",
            r#"{"actor":"quality.lead","reason":"review complete","operation_id":"op-e2e-transition"}"#,
        );
        assert_eq!(transition_replay.0, 200);
        assert!(transition_replay.1.contains("\"replayed\":true"));
        let outbox = http_request("GET", &first_address, "/api/v1/sync/outbox", "");
        let audit = http_request(
            "GET",
            &first_address,
            "/api/v1/projects/CEM-E2E-001/audit-events",
            "",
        );
        assert_eq!(outbox.0, 200);
        assert_eq!(audit.0, 200);
        assert_eq!(outbox.1.matches("\"operation_id\"").count(), 11);
        assert_eq!(audit.1.matches("\"sequence\"").count(), 11);
        first_server
            .join()
            .expect("server thread panicked")
            .unwrap();

        let second_port = free_loopback_port();
        let second_address = format!("127.0.0.1:{second_port}");
        let second_server = spawn_server(ApiServerConfig {
            bind: second_address.clone(),
            storage_root: storage_root.clone(),
            migrations_root,
            max_requests: Some(2),
        });
        assert_eq!(wait_for_http(&second_address, "/api/v1/health").0, 200);
        let reloaded = http_request("GET", &second_address, "/api/v1/projects/CEM-E2E-001", "");
        assert_eq!(reloaded.0, 200);
        assert!(reloaded.1.contains("\"stage\":\"test_planning\""));
        second_server
            .join()
            .expect("server thread panicked")
            .unwrap();

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn real_http_server_runs_metrology_vertical_slice_after_restart() {
        let storage_root = temporary_storage_root("agent-api-real-metrology");
        let migrations_root = repo_root().join("storage/sqlite");
        let first_port = free_loopback_port();
        let first_address = format!("127.0.0.1:{first_port}");
        let first_server = spawn_server(ApiServerConfig {
            bind: first_address.clone(),
            storage_root: storage_root.clone(),
            migrations_root: migrations_root.clone(),
            max_requests: Some(15),
        });

        let health = wait_for_http(&first_address, "/api/v1/health");
        assert_eq!(health.0, 200);
        assert_eq!(
            http_request("POST", &first_address, "/api/v1/storage/initialize", "").0,
            200
        );

        let register_body = r#"{
            "asset_id": "SA-E2E-001",
            "family": "SpectrumAnalyzer",
            "category_code": "spectrum_analyzer",
            "manufacturer": "Rohde Schwarz",
            "model": "FSW",
            "serial_number": "E2E-001",
            "part_number": "FSW44",
            "calibration_requirement": "required",
            "calibration_period_months": 12,
            "calibration_due_warning_days": 45,
            "capabilities": {"frequency_max_hz": 44000000000},
            "actor": "metrology.admin",
            "reason": "initial E2E registration",
            "operation_id": "op-e2e-metrology-register"
        }"#;
        let registered = http_request(
            "POST",
            &first_address,
            "/api/v1/metrology/instruments",
            register_body,
        );
        assert_eq!(registered.0, 200);
        assert!(registered.1.contains("\"asset_id\":\"SA-E2E-001\""));

        let missing = http_request(
            "POST",
            &first_address,
            "/api/v1/metrology/readiness",
            r#"{
                "asset_ids": ["SA-E2E-001"],
                "execution_mode": "accredited",
                "checked_on": "2026-07-01"
            }"#,
        );
        assert_eq!(missing.0, 200);
        assert!(missing.1.contains("\"ready\":false"));
        assert!(missing.1.contains("\"code\":\"calibration_missing\""));

        let calibration = http_request(
            "POST",
            &first_address,
            "/api/v1/metrology/instruments/SA-E2E-001/calibrations",
            &format!(
                r#"{{
                    "event_id": "CAL-SA-E2E-001-2026",
                    "certificate_reference": "CERT-SA-E2E-001-2026",
                    "calibrated_at": "2026-06-30",
                    "due_at": "2027-06-30",
                    "provider": "Accredited Lab",
                    "decision": "conforming",
                    "uncertainty_summary": {{"level_db": 0.6}},
                    "document_manifest": {{
                        "object_id": "obj-cert-e2e",
                        "original_filename": "cert.pdf",
                        "mime_type": "application/pdf",
                        "size_bytes": 12,
                        "sha256": "{}",
                        "storage_key": "metrology/SA-E2E-001/cert.pdf",
                        "revision": "A"
                    }},
                    "recorded_by": "metrology.admin",
                    "actor": "metrology.admin",
                    "reason": "annual calibration",
                    "operation_id": "op-e2e-metrology-calibration"
                }}"#,
                "c".repeat(64)
            ),
        );
        assert_eq!(calibration.0, 200);
        assert!(calibration
            .1
            .contains("\"event_id\":\"CAL-SA-E2E-001-2026\""));

        let ready = http_request(
            "POST",
            &first_address,
            "/api/v1/metrology/readiness",
            r#"{
                "asset_ids": ["SA-E2E-001"],
                "execution_mode": "accredited",
                "checked_on": "2026-07-01"
            }"#,
        );
        assert_eq!(ready.0, 200);
        assert!(ready.1.contains("\"ready\":true"));

        let due_soon = http_request(
            "POST",
            &first_address,
            "/api/v1/metrology/readiness",
            r#"{
                "asset_ids": ["SA-E2E-001"],
                "execution_mode": "accredited",
                "checked_on": "2027-06-15"
            }"#,
        );
        assert_eq!(due_soon.0, 200);
        assert!(due_soon.1.contains("\"calibration_status\":\"due_soon\""));
        assert!(due_soon.1.contains("\"warnings\""));

        let out_of_service = http_request(
            "POST",
            &first_address,
            "/api/v1/metrology/instruments/SA-E2E-001/serviceability",
            r#"{
                "serviceability_status": "out_of_service",
                "serviceability_reason": "Damaged input connector",
                "actor": "metrology.admin",
                "reason": "asset quarantine",
                "operation_id": "op-e2e-metrology-out-of-service"
            }"#,
        );
        assert_eq!(out_of_service.0, 200);
        assert!(out_of_service
            .1
            .contains("\"serviceability_status\":\"out_of_service\""));

        let blocked = http_request(
            "POST",
            &first_address,
            "/api/v1/metrology/readiness",
            r#"{
                "asset_ids": ["SA-E2E-001"],
                "execution_mode": "accredited",
                "checked_on": "2026-07-01"
            }"#,
        );
        assert_eq!(blocked.0, 200);
        assert!(blocked.1.contains("\"ready\":false"));
        assert!(blocked.1.contains("\"code\":\"out_of_service\""));

        assert_eq!(
            http_request(
                "POST",
                &first_address,
                "/api/v1/metrology/instruments/SA-E2E-001/serviceability",
                r#"{
                    "serviceability_status": "usable",
                    "serviceability_reason": "Repair verified",
                    "actor": "metrology.admin",
                    "reason": "return to service",
                    "operation_id": "op-e2e-metrology-return-to-service"
                }"#,
            )
            .0,
            200
        );

        let ready_again = http_request(
            "POST",
            &first_address,
            "/api/v1/metrology/readiness",
            r#"{
                "asset_ids": ["SA-E2E-001"],
                "execution_mode": "accredited",
                "checked_on": "2026-07-01"
            }"#,
        );
        assert_eq!(ready_again.0, 200);
        assert!(ready_again.1.contains("\"ready\":true"));

        let replay = http_request(
            "POST",
            &first_address,
            "/api/v1/metrology/instruments",
            register_body,
        );
        assert_eq!(replay.0, 200);
        assert!(replay.1.contains("\"asset_id\":\"SA-E2E-001\""));

        let conflict = http_request(
            "POST",
            &first_address,
            "/api/v1/metrology/instruments",
            r#"{
                "asset_id": "SA-E2E-001",
                "family": "SpectrumAnalyzer",
                "category_code": "spectrum_analyzer",
                "manufacturer": "Rohde Schwarz",
                "model": "Changed",
                "serial_number": "E2E-001",
                "calibration_requirement": "required",
                "actor": "metrology.admin",
                "reason": "conflicting registration",
                "operation_id": "op-e2e-metrology-register"
            }"#,
        );
        assert_eq!(conflict.0, 409);
        assert!(conflict.1.contains("operation_replay_mismatch"));

        let audit = http_request(
            "GET",
            &first_address,
            "/api/v1/metrology/instruments/SA-E2E-001/audit-events",
            "",
        );
        let outbox = http_request("GET", &first_address, "/api/v1/sync/outbox", "");
        assert_eq!(audit.0, 200);
        assert_eq!(outbox.0, 200);
        assert!(audit.1.contains("\"instrument_registered\""));
        assert!(audit.1.contains("\"instrument_serviceability_changed\""));
        assert_eq!(audit.1.matches("\"sequence\"").count(), 3);
        assert!(outbox.1.contains("\"domain\":\"metrology\""));
        assert!(outbox
            .1
            .contains("\"operation_kind\":\"calibration_recorded\""));
        assert_eq!(outbox.1.matches("\"operation_id\"").count(), 4);
        first_server
            .join()
            .expect("server thread panicked")
            .unwrap();

        let second_port = free_loopback_port();
        let second_address = format!("127.0.0.1:{second_port}");
        let second_server = spawn_server(ApiServerConfig {
            bind: second_address.clone(),
            storage_root: storage_root.clone(),
            migrations_root,
            max_requests: Some(5),
        });
        assert_eq!(wait_for_http(&second_address, "/api/v1/health").0, 200);
        let reloaded = http_request(
            "GET",
            &second_address,
            "/api/v1/metrology/instruments/SA-E2E-001",
            "",
        );
        let reloaded_ready = http_request(
            "POST",
            &second_address,
            "/api/v1/metrology/readiness",
            r#"{
                "asset_ids": ["SA-E2E-001"],
                "execution_mode": "accredited",
                "checked_on": "2026-07-01"
            }"#,
        );
        let reloaded_audit = http_request(
            "GET",
            &second_address,
            "/api/v1/metrology/instruments/SA-E2E-001/audit-events",
            "",
        );
        let reloaded_outbox = http_request("GET", &second_address, "/api/v1/sync/outbox", "");
        assert_eq!(reloaded.0, 200);
        assert_eq!(reloaded_ready.0, 200);
        assert_eq!(reloaded_audit.0, 200);
        assert_eq!(reloaded_outbox.0, 200);
        assert!(reloaded.1.contains("\"asset_id\":\"SA-E2E-001\""));
        assert!(reloaded_ready.1.contains("\"ready\":true"));
        assert!(reloaded_audit.1.contains("\"instrument_registered\""));
        assert!(reloaded_outbox.1.contains("\"domain\":\"metrology\""));
        second_server
            .join()
            .expect("server thread panicked")
            .unwrap();

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn real_http_server_runs_simulated_emc_test_with_metrology_preflight() {
        let storage_root = temporary_storage_root("agent-api-real-simulated-emc");
        let migrations_root = repo_root().join("storage/sqlite");
        let first_port = free_loopback_port();
        let first_address = format!("127.0.0.1:{first_port}");
        let first_server = spawn_server(ApiServerConfig {
            bind: first_address.clone(),
            storage_root: storage_root.clone(),
            migrations_root: migrations_root.clone(),
            max_requests: Some(13),
        });

        assert_eq!(wait_for_http(&first_address, "/api/v1/health").0, 200);
        assert_eq!(
            http_request("POST", &first_address, "/api/v1/storage/initialize", "").0,
            200
        );
        assert_eq!(
            http_request(
                "POST",
                &first_address,
                "/api/v1/projects",
                r#"{
                    "code": "CEM-SIM-001",
                    "customer_name": "Simulated EMC Customer",
                    "execution_mode": "accredited",
                    "actor": "quality.lead",
                    "reason": "contract accepted",
                    "operation_id": "op-sim-project"
                }"#,
            )
            .0,
            200
        );
        assert_eq!(
            http_request(
                "POST",
                &first_address,
                "/api/v1/metrology/instruments",
                r#"{
                    "asset_id": "SA-SIM-001",
                    "family": "SpectrumAnalyzer",
                    "category_code": "spectrum_analyzer",
                    "manufacturer": "Rohde Schwarz",
                    "model": "FSW",
                    "serial_number": "SIM-001",
                    "calibration_requirement": "required",
                    "calibration_period_months": 12,
                    "capabilities": {"frequency_max_hz": 30000000},
                    "actor": "metrology.admin",
                    "reason": "register preflight asset",
                    "operation_id": "op-sim-register"
                }"#,
            )
            .0,
            200
        );

        let refused = http_request(
            "POST",
            &first_address,
            "/api/v1/test-executions/simulated-emc",
            r#"{
                "attempt_id": "RUN-SIM-001",
                "project_code": "CEM-SIM-001",
                "test_method_reference": "SIM-EMC-CONDUCTED",
                "execution_mode": "accredited",
                "required_asset_ids": ["SA-SIM-001"],
                "operator": "operator.one",
                "checked_on": "2026-07-01",
                "reason": "operator launch",
                "operation_id": "op-sim-run-refused"
            }"#,
        );
        assert_eq!(refused.0, 200);
        assert!(refused.1.contains("\"status\":\"refused\""));
        assert!(refused
            .1
            .contains("\"code\":\"equipment_readiness_blocked\""));
        assert!(refused.1.contains("\"code\":\"calibration_missing\""));
        assert!(refused.1.contains("\"dimension\":\"missing_evidence\""));

        let calibration = http_request(
            "POST",
            &first_address,
            "/api/v1/metrology/instruments/SA-SIM-001/calibrations",
            &format!(
                r#"{{
                    "event_id": "CAL-SA-SIM-001-2026",
                    "certificate_reference": "CERT-SA-SIM-001-2026",
                    "calibrated_at": "2026-06-30",
                    "due_at": "2027-06-30",
                    "provider": "Accredited Lab",
                    "decision": "conforming",
                    "uncertainty_summary": {{"level_db": 0.6}},
                    "document_manifest": {{
                        "object_id": "obj-cert-sim",
                        "original_filename": "cert.pdf",
                        "mime_type": "application/pdf",
                        "size_bytes": 12,
                        "sha256": "{}",
                        "storage_key": "metrology/SA-SIM-001/cert.pdf",
                        "revision": "A"
                    }},
                    "recorded_by": "metrology.admin",
                    "actor": "metrology.admin",
                    "reason": "annual calibration",
                    "operation_id": "op-sim-calibration"
                }}"#,
                "d".repeat(64)
            ),
        );
        assert_eq!(calibration.0, 200);

        let completed_body = r#"{
            "attempt_id": "RUN-SIM-002",
            "project_code": "CEM-SIM-001",
            "test_method_reference": "SIM-EMC-CONDUCTED",
            "execution_mode": "accredited",
            "required_asset_ids": ["SA-SIM-001"],
            "operator": "operator.one",
            "checked_on": "2026-07-01",
            "reason": "operator launch after calibration",
            "operation_id": "op-sim-run-completed"
        }"#;
        let completed = http_request(
            "POST",
            &first_address,
            "/api/v1/test-executions/simulated-emc",
            completed_body,
        );
        assert_eq!(completed.0, 200);
        assert!(completed.1.contains("\"status\":\"completed\""));
        assert!(completed.1.contains("\"ready\":true"));
        assert!(completed.1.contains("\"simulation_result\""));
        assert!(completed
            .1
            .contains("\"strategy\":\"deterministic_conducted_emission_level_sweep\""));

        let loaded = http_request(
            "GET",
            &first_address,
            "/api/v1/test-executions/RUN-SIM-002",
            "",
        );
        let list = http_request(
            "GET",
            &first_address,
            "/api/v1/projects/CEM-SIM-001/test-executions",
            "",
        );
        assert_eq!(loaded.0, 200);
        assert_eq!(list.0, 200);
        assert!(loaded.1.contains("\"attempt_id\":\"RUN-SIM-002\""));
        assert!(list.1.contains("\"attempt_id\":\"RUN-SIM-001\""));
        assert!(list.1.contains("\"attempt_id\":\"RUN-SIM-002\""));

        let replay = http_request(
            "POST",
            &first_address,
            "/api/v1/test-executions/simulated-emc",
            completed_body,
        );
        assert_eq!(replay.0, 200);
        assert!(replay.1.contains("\"replayed\":true"));

        let conflict = http_request(
            "POST",
            &first_address,
            "/api/v1/test-executions/simulated-emc",
            r#"{
                "attempt_id": "RUN-SIM-002",
                "project_code": "CEM-SIM-001",
                "test_method_reference": "SIM-EMC-RADIATED",
                "execution_mode": "accredited",
                "required_asset_ids": ["SA-SIM-001"],
                "operator": "operator.one",
                "checked_on": "2026-07-01",
                "reason": "conflicting replay",
                "operation_id": "op-sim-run-completed"
            }"#,
        );
        assert_eq!(conflict.0, 409);
        assert!(conflict.1.contains("operation_replay_mismatch"));

        let audit = http_request(
            "GET",
            &first_address,
            "/api/v1/projects/CEM-SIM-001/audit-events",
            "",
        );
        let outbox = http_request("GET", &first_address, "/api/v1/sync/outbox", "");
        assert_eq!(audit.0, 200);
        assert_eq!(outbox.0, 200);
        assert!(audit
            .1
            .contains("\"action\":\"simulated_test_execution_refused\""));
        assert!(audit
            .1
            .contains("\"action\":\"simulated_test_execution_completed\""));
        assert!(outbox
            .1
            .contains("\"entity_type\":\"simulated_test_execution\""));
        assert!(outbox
            .1
            .contains("\"operation_kind\":\"simulated_test_execution_completed\""));

        first_server
            .join()
            .expect("server thread panicked")
            .unwrap();

        let second_port = free_loopback_port();
        let second_address = format!("127.0.0.1:{second_port}");
        let second_server = spawn_server(ApiServerConfig {
            bind: second_address.clone(),
            storage_root: storage_root.clone(),
            migrations_root,
            max_requests: Some(3),
        });
        assert_eq!(wait_for_http(&second_address, "/api/v1/health").0, 200);
        let reloaded = http_request(
            "GET",
            &second_address,
            "/api/v1/test-executions/RUN-SIM-002",
            "",
        );
        let reloaded_outbox = http_request("GET", &second_address, "/api/v1/sync/outbox", "");
        assert_eq!(reloaded.0, 200);
        assert_eq!(reloaded_outbox.0, 200);
        assert!(reloaded.1.contains("\"status\":\"completed\""));
        assert!(reloaded_outbox
            .1
            .contains("\"entity_type\":\"simulated_test_execution\""));
        second_server
            .join()
            .expect("server thread panicked")
            .unwrap();

        remove_temporary_storage_root(&storage_root);
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
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

    fn remove_temporary_storage_root(root: &std::path::Path) {
        if root.exists() {
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    fn spawn_server(config: ApiServerConfig) -> thread::JoinHandle<Result<(), AgentError>> {
        thread::spawn(move || run_local_api_server(config))
    }

    fn wait_for_http(address: &str, path: &str) -> (u16, String) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match try_http_request("GET", address, path, "") {
                Ok(response) => return response,
                Err(error) if Instant::now() < deadline => {
                    let _ = error;
                    thread::sleep(Duration::from_millis(25));
                }
                Err(error) => panic!("server did not become ready: {error}"),
            }
        }
    }

    fn http_request(method: &str, address: &str, path: &str, body: &str) -> (u16, String) {
        try_http_request(method, address, path, body).unwrap()
    }

    fn try_http_request(
        method: &str,
        address: &str,
        path: &str,
        body: &str,
    ) -> std::io::Result<(u16, String)> {
        let mut stream = TcpStream::connect(address)?;
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes())?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        let status = response
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(0);
        let body = response
            .split_once("\r\n\r\n")
            .map_or_else(String::new, |(_, body)| body.to_owned());
        Ok((status, body))
    }

    fn free_loopback_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }
}
