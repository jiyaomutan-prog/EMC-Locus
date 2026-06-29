use super::{
    build_health_report, json_string, run_project_command, run_storage_action, run_sync_command,
    AgentCommand, AgentError, ProjectAction, StorageAction, SyncAction,
};
use crate::project_agent::{
    AdvanceToTestPlanningInput, CompleteReviewItemInput, CreateProjectInput,
};
use serde_json::Value;
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
        "{{\"agent\":\"emc-locus-agent\",\"api\":\"v1\",\"bind\":{},\"storage_root\":{}}}",
        json_string(&server.server_addr().to_string()),
        json_string(&config.storage_root.to_string_lossy())
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
    if parts.as_slice() == ["api", "v1", "sync", "outbox"] && method == "GET" {
        return run_sync_command(AgentCommand::Sync {
            action: SyncAction::Outbox,
            storage_root: config.storage_root.clone(),
        });
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

fn required_string(payload: &Value, key: &'static str) -> Result<String, AgentError> {
    optional_string(payload, key).ok_or_else(|| {
        AgentError::with_details(
            "missing_json_field",
            format!("missing required JSON field: {key}"),
            format!("{{\"field\":{}}}", json_string(key)),
        )
    })
}

fn optional_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn status_for_error(code: &str) -> u16 {
    match code {
        "api_route_not_found" | "project_not_found" => 404,
        "contract_review_incomplete"
        | "invalid_project_transition"
        | "project_already_exists"
        | "operation_replay_mismatch" => 409,
        "storage_not_initialized" => 503,
        "invalid_json_body"
        | "missing_json_field"
        | "missing_argument"
        | "invalid_project_code"
        | "invalid_customer_name"
        | "invalid_actor"
        | "invalid_reason"
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
    fn local_api_reports_not_found_for_unknown_routes() {
        let config = ApiServerConfig::default_for(PathBuf::from("unused"));
        let response = handle_api_request("GET", "/api/v1/unknown", "", &config);

        assert_eq!(response.status, 404);
        assert!(response.body.contains("api_route_not_found"));
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
            max_requests: Some(16),
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
