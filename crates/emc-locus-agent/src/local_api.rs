use super::{
    build_health_report, render_json, run_metrology_command, run_project_command,
    run_storage_action, run_sync_command, AgentCommand, AgentError, MetrologyAction, ProjectAction,
    StorageAction, SyncAction,
};
use crate::document_service::{
    get_document, list_document_audit_events, list_documents, register_attached_document,
    ListAttachedDocumentsInput, RegisterAttachedDocumentInput,
};
use crate::equipment_service::{
    clone_equipment_model, communication_provider_status, create_driver_profile,
    create_driver_profile_revision, create_equipment_model, create_equipment_model_from_preset,
    create_equipment_model_revision, equipment_registries, get_classification_preset,
    get_driver_profile, get_driver_profile_revision, get_equipment_model,
    get_equipment_model_revision, list_classification_presets, list_driver_profile_revisions,
    list_driver_profiles, list_equipment_audit_events_for_driver,
    list_equipment_audit_events_for_model, list_equipment_model_revisions, list_equipment_models,
    replace_driver_profile_revision_definition, replace_equipment_model_revision_definition,
    simulate_driver_profile, transition_driver_profile_revision,
    transition_equipment_model_revision, validate_driver_profile_definition_json,
    validate_equipment_model_definition_json, CloneEquipmentModelInput, CreateDriverProfileInput,
    CreateDriverProfileRevisionInput, CreateEquipmentModelFromPresetInput,
    CreateEquipmentModelInput, CreateEquipmentModelRevisionInput, ListDriverProfilesInput,
    ListEquipmentModelsInput, ReplaceDriverProfileDefinitionInput,
    ReplaceEquipmentModelDefinitionInput, SimulateDriverProfileInput,
    TransitionDriverProfileRevisionInput, TransitionEquipmentModelRevisionInput,
};
use crate::measurement_engineering_service::{
    clone_measurement_engineering_definition, create_measurement_engineering_definition,
    create_measurement_engineering_revision, evaluate_engineering_curve_revision,
    get_measurement_engineering_definition, get_measurement_engineering_revision_json,
    list_measurement_engineering_audit_events, list_measurement_engineering_definitions,
    list_measurement_engineering_revisions_json,
    replace_measurement_engineering_revision_definition,
    transition_measurement_engineering_revision, validate_measurement_engineering_definition_json,
    CloneMeasurementEngineeringInput, CreateMeasurementEngineeringInput,
    CreateMeasurementEngineeringRevisionInput, EvaluateEngineeringCurveInput,
    ReplaceMeasurementEngineeringDefinitionInput, TransitionMeasurementEngineeringRevisionInput,
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
use crate::test_template_service::{
    clone_test_template, create_test_template, create_test_template_revision,
    get_test_template_definition, get_test_template_revision, list_test_template_audit_events,
    list_test_template_definitions, list_test_template_revisions,
    replace_test_template_revision_definition, transition_test_template_revision,
    validate_test_template_definition_json, CloneTestTemplateInput, CreateTestTemplateInput,
    CreateTestTemplateRevisionInput, ListTestTemplatesInput, ReplaceTestTemplateDefinitionInput,
    TransitionTestTemplateRevisionInput,
};
use emc_locus_core::{
    equipment::EquipmentRevisionStatus,
    measurement_engineering::{
        MeasurementEngineeringAggregateKind, MeasurementEngineeringRevisionStatus,
    },
    test_definitions::TemplateRevisionStatus,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};
use tiny_http::{Header, Response, Server, StatusCode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiServerConfig {
    pub bind: String,
    pub storage_root: PathBuf,
    pub migrations_root: PathBuf,
    pub lab_console_dist: PathBuf,
    pub max_requests: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiResponse {
    pub status: u16,
    pub body: String,
    pub content_type: String,
    pub location: Option<String>,
}

impl ApiServerConfig {
    pub fn default_for(storage_root: PathBuf) -> Self {
        Self {
            bind: "127.0.0.1:8765".to_owned(),
            storage_root,
            migrations_root: PathBuf::from("storage/sqlite"),
            lab_console_dist: PathBuf::from("apps/lab-console/dist"),
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
    if let Some(lab_console_dist) = optional_value(&mut flags, "--lab-console-dist") {
        config.lab_console_dist = PathBuf::from(lab_console_dist);
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
        let content_type = Header::from_bytes("Content-Type", response.content_type.as_str())
            .expect("static content-type header is valid");
        let mut http_response = Response::from_string(response.body)
            .with_status_code(StatusCode(response.status))
            .with_header(content_type);
        if let Some(location) = response.location.as_deref() {
            let location =
                Header::from_bytes("Location", location).expect("static location header is valid");
            http_response = http_response.with_header(location);
        }
        request
            .respond(http_response)
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
    if let Some(response) = route_lab_console_request(method, url, config) {
        return response;
    }
    match route_api_request(method, url, body, config) {
        Ok(body) => json_response(200, body),
        Err(error) => json_response(status_for_error(error.code), error.to_json()),
    }
}

fn json_response(status: u16, body: String) -> ApiResponse {
    ApiResponse {
        status,
        body,
        content_type: "application/json".to_owned(),
        location: None,
    }
}

fn text_response(status: u16, content_type: &str, body: String) -> ApiResponse {
    ApiResponse {
        status,
        body,
        content_type: content_type.to_owned(),
        location: None,
    }
}

fn redirect_response(location: &str) -> ApiResponse {
    ApiResponse {
        status: 302,
        body: String::new(),
        content_type: "text/plain; charset=utf-8".to_owned(),
        location: Some(location.to_owned()),
    }
}

fn route_lab_console_request(
    method: &str,
    url: &str,
    config: &ApiServerConfig,
) -> Option<ApiResponse> {
    let path = url.split_once('?').map_or(url, |(path, _)| path);
    if method != "GET" {
        return None;
    }
    if path == "/" {
        return Some(redirect_response("/lab/"));
    }
    if path != "/lab" && !path.starts_with("/lab/") {
        return None;
    }
    Some(match lab_console_response(path, &config.lab_console_dist) {
        Ok(response) => response,
        Err(error) => json_response(status_for_error(error.code), error.to_json()),
    })
}

fn lab_console_response(path: &str, dist_root: &Path) -> Result<ApiResponse, AgentError> {
    let index_path = dist_root.join("index.html");
    if !index_path.is_file() {
        return Err(AgentError::new(
            "lab_console_build_missing",
            "LAB CONSOLE production build is not available",
        ));
    }
    if path == "/lab" {
        return Ok(redirect_response("/lab/"));
    }
    if path == "/lab/" {
        return serve_lab_file(dist_root, Path::new("index.html"));
    }
    if let Some(asset_path) = path.strip_prefix("/lab/assets/") {
        let relative = decode_url_path(asset_path)?;
        return serve_lab_file(dist_root, Path::new("assets").join(relative).as_path());
    }
    if path.starts_with("/lab/") {
        return serve_lab_file(dist_root, Path::new("index.html"));
    }
    Err(AgentError::new(
        "api_route_not_found",
        format!("route not found: GET {path}"),
    ))
}

fn serve_lab_file(dist_root: &Path, relative_path: &Path) -> Result<ApiResponse, AgentError> {
    ensure_safe_relative_path(relative_path)?;
    let canonical_root = fs::canonicalize(dist_root).map_err(|error| {
        AgentError::new(
            "lab_console_build_missing",
            format!("LAB CONSOLE production build is not available: {error}"),
        )
    })?;
    let candidate = canonical_root.join(relative_path);
    let canonical_file = fs::canonicalize(&candidate).map_err(|_| {
        AgentError::new(
            "api_route_not_found",
            format!("LAB CONSOLE asset not found: {}", relative_path.display()),
        )
    })?;
    if !canonical_file.starts_with(&canonical_root) || !canonical_file.is_file() {
        return Err(AgentError::new(
            "invalid_lab_console_path",
            "LAB CONSOLE path is outside the production build",
        ));
    }
    let body = fs::read_to_string(&canonical_file).map_err(|error| {
        AgentError::new(
            "lab_console_asset_read_failed",
            format!("cannot read {}: {error}", canonical_file.display()),
        )
    })?;
    Ok(text_response(200, lab_content_type(&canonical_file), body))
}

fn ensure_safe_relative_path(path: &Path) -> Result<(), AgentError> {
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(AgentError::new(
            "invalid_lab_console_path",
            "LAB CONSOLE path traversal is not allowed",
        ));
    }
    Ok(())
}

fn decode_url_path(path: &str) -> Result<PathBuf, AgentError> {
    let mut decoded = String::new();
    let mut chars = path.as_bytes().iter().copied().peekable();
    while let Some(byte) = chars.next() {
        if byte == b'%' {
            let high = chars.next().ok_or_else(invalid_url_escape)?;
            let low = chars.next().ok_or_else(invalid_url_escape)?;
            let high = hex_value(high).ok_or_else(invalid_url_escape)?;
            let low = hex_value(low).ok_or_else(invalid_url_escape)?;
            decoded.push(char::from((high << 4) | low));
        } else {
            decoded.push(char::from(byte));
        }
    }
    let path = PathBuf::from(decoded);
    ensure_safe_relative_path(&path)?;
    Ok(path)
}

fn invalid_url_escape() -> AgentError {
    AgentError::new(
        "invalid_lab_console_path",
        "LAB CONSOLE asset path contains an invalid URL escape",
    )
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn lab_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        _ => "text/plain; charset=utf-8",
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
    if parts.as_slice() == ["api", "v1", "documents"] && method == "GET" {
        return list_documents(&config.storage_root, list_documents_input(query));
    }
    if parts.as_slice() == ["api", "v1", "documents"] && method == "POST" {
        let payload = parse_json_body(body)?;
        return register_attached_document(
            config.storage_root.clone(),
            register_document_input(&payload)?,
        );
    }
    if parts.as_slice() == ["api", "v1", "equipment-models"] && method == "GET" {
        return list_equipment_models(&config.storage_root, list_equipment_models_input(query));
    }
    if parts.as_slice() == ["api", "v1", "equipment-models"] && method == "POST" {
        let payload = parse_json_body(body)?;
        return create_equipment_model(
            &config.storage_root,
            create_equipment_model_input(&payload)?,
        );
    }
    if parts.as_slice() == ["api", "v1", "equipment-models", "from-preset"] && method == "POST" {
        let payload = parse_json_body(body)?;
        return create_equipment_model_from_preset(
            &config.storage_root,
            create_equipment_model_from_preset_input(&payload)?,
        );
    }
    if parts.as_slice() == ["api", "v1", "equipment-model-definitions", "validate"]
        && method == "POST"
    {
        let payload = parse_json_body(body)?;
        return validate_equipment_model_definition_json(&required_json_or_string(
            &payload,
            "definition",
            "definition_json",
        )?);
    }
    if parts.as_slice() == ["api", "v1", "driver-profiles"] && method == "GET" {
        return list_driver_profiles(&config.storage_root, list_driver_profiles_input(query));
    }
    if parts.as_slice() == ["api", "v1", "driver-profiles"] && method == "POST" {
        let payload = parse_json_body(body)?;
        return create_driver_profile(&config.storage_root, create_driver_profile_input(&payload)?);
    }
    if parts.as_slice() == ["api", "v1", "driver-profile-definitions", "validate"]
        && method == "POST"
    {
        let payload = parse_json_body(body)?;
        return validate_driver_profile_definition_json(
            &config.storage_root,
            &required_json_or_string(&payload, "definition", "definition_json")?,
        );
    }
    if parts.as_slice() == ["api", "v1", "driver-profile-simulations"] && method == "POST" {
        let payload = parse_json_body(body)?;
        return simulate_driver_profile(&config.storage_root, driver_simulation_input(&payload)?);
    }
    if parts.len() == 3 && parts[0] == "api" && parts[1] == "v1" {
        if let Some(kind) = measurement_engineering_kind_for_collection(parts[2]) {
            if method == "GET" {
                return list_measurement_engineering_definitions(&config.storage_root, kind);
            }
            if method == "POST" {
                let payload = parse_json_body(body)?;
                return create_measurement_engineering_definition(
                    &config.storage_root,
                    measurement_engineering_create_input(kind, &payload)?,
                );
            }
        }
    }
    if parts.len() == 4 && parts[0] == "api" && parts[1] == "v1" && parts[3] == "validate" {
        if let Some(kind) = measurement_engineering_kind_for_validation(parts[2]) {
            if method == "POST" {
                let payload = parse_json_body(body)?;
                return validate_measurement_engineering_definition_json(
                    &config.storage_root,
                    kind,
                    &required_json_or_string(&payload, "definition", "definition_json")?,
                );
            }
        }
    }
    if parts.as_slice() == ["api", "v1", "equipment", "communication-providers"] && method == "GET"
    {
        return communication_provider_status();
    }
    if parts.as_slice() == ["api", "v1", "equipment", "registries"] && method == "GET" {
        return equipment_registries(&config.storage_root);
    }
    if parts.as_slice() == ["api", "v1", "equipment", "classification-presets"] && method == "GET" {
        return list_classification_presets(&config.storage_root);
    }
    if parts.as_slice() == ["api", "v1", "test-templates"] && method == "GET" {
        return list_test_template_definitions(
            &config.storage_root,
            list_test_templates_input(query),
        );
    }
    if parts.as_slice() == ["api", "v1", "test-templates"] && method == "POST" {
        let payload = parse_json_body(body)?;
        return create_test_template(&config.storage_root, create_test_template_input(&payload)?);
    }
    if parts.as_slice() == ["api", "v1", "test-template-definitions", "validate"]
        && method == "POST"
    {
        let payload = parse_json_body(body)?;
        return validate_test_template_definition_json(&required_json_or_string(
            &payload,
            "definition",
            "definition_json",
        )?);
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
        ["api", "v1", "documents", document_id] if method == "GET" => {
            get_document(&config.storage_root, document_id)
        }
        ["api", "v1", "documents", document_id, "audit-events"] if method == "GET" => {
            list_document_audit_events(&config.storage_root, document_id)
        }
        ["api", "v1", "equipment-models", equipment_model_id] if method == "GET" => {
            get_equipment_model(&config.storage_root, equipment_model_id)
        }
        ["api", "v1", "equipment", "classification-presets", preset_id] if method == "GET" => {
            get_classification_preset(&config.storage_root, preset_id)
        }
        ["api", "v1", "equipment-models", equipment_model_id, "clone"] if method == "POST" => {
            let payload = parse_json_body(body)?;
            clone_equipment_model(
                &config.storage_root,
                clone_equipment_model_input(equipment_model_id, &payload)?,
            )
        }
        ["api", "v1", "equipment-models", equipment_model_id, "revisions"] if method == "GET" => {
            list_equipment_model_revisions(&config.storage_root, equipment_model_id)
        }
        ["api", "v1", "equipment-models", equipment_model_id, "revisions"] if method == "POST" => {
            let payload = parse_json_body(body)?;
            create_equipment_model_revision(
                &config.storage_root,
                create_equipment_model_revision_input(equipment_model_id, &payload)?,
            )
        }
        ["api", "v1", "equipment-models", equipment_model_id, "revisions", revision_id]
            if method == "GET" =>
        {
            get_equipment_model_revision(&config.storage_root, equipment_model_id, revision_id)
        }
        ["api", "v1", "equipment-models", equipment_model_id, "revisions", revision_id, "definition"]
            if method == "PUT" =>
        {
            let payload = parse_json_body(body)?;
            replace_equipment_model_revision_definition(
                &config.storage_root,
                replace_equipment_model_definition_input(
                    equipment_model_id,
                    revision_id,
                    &payload,
                )?,
            )
        }
        ["api", "v1", "equipment-models", equipment_model_id, "revisions", revision_id, "transitions", "submit-for-review"]
            if method == "POST" =>
        {
            let payload = parse_json_body(body)?;
            transition_equipment_model_revision(
                &config.storage_root,
                equipment_model_revision_transition_input(
                    equipment_model_id,
                    revision_id,
                    EquipmentRevisionStatus::UnderReview,
                    &payload,
                )?,
            )
        }
        ["api", "v1", "equipment-models", equipment_model_id, "revisions", revision_id, "transitions", "approve"]
            if method == "POST" =>
        {
            let payload = parse_json_body(body)?;
            transition_equipment_model_revision(
                &config.storage_root,
                equipment_model_revision_transition_input(
                    equipment_model_id,
                    revision_id,
                    EquipmentRevisionStatus::Approved,
                    &payload,
                )?,
            )
        }
        ["api", "v1", "equipment-models", equipment_model_id, "audit-events"]
            if method == "GET" =>
        {
            list_equipment_audit_events_for_model(&config.storage_root, equipment_model_id)
        }
        ["api", "v1", "driver-profiles", driver_profile_id] if method == "GET" => {
            get_driver_profile(&config.storage_root, driver_profile_id)
        }
        ["api", "v1", "driver-profiles", driver_profile_id, "revisions"] if method == "GET" => {
            list_driver_profile_revisions(&config.storage_root, driver_profile_id)
        }
        ["api", "v1", "driver-profiles", driver_profile_id, "revisions"] if method == "POST" => {
            let payload = parse_json_body(body)?;
            create_driver_profile_revision(
                &config.storage_root,
                create_driver_profile_revision_input(driver_profile_id, &payload)?,
            )
        }
        ["api", "v1", "driver-profiles", driver_profile_id, "revisions", revision_id]
            if method == "GET" =>
        {
            get_driver_profile_revision(&config.storage_root, driver_profile_id, revision_id)
        }
        ["api", "v1", "driver-profiles", driver_profile_id, "revisions", revision_id, "definition"]
            if method == "PUT" =>
        {
            let payload = parse_json_body(body)?;
            replace_driver_profile_revision_definition(
                &config.storage_root,
                replace_driver_profile_definition_input(driver_profile_id, revision_id, &payload)?,
            )
        }
        ["api", "v1", "driver-profiles", driver_profile_id, "revisions", revision_id, "transitions", "submit-for-review"]
            if method == "POST" =>
        {
            let payload = parse_json_body(body)?;
            transition_driver_profile_revision(
                &config.storage_root,
                driver_profile_revision_transition_input(
                    driver_profile_id,
                    revision_id,
                    EquipmentRevisionStatus::UnderReview,
                    &payload,
                )?,
            )
        }
        ["api", "v1", "driver-profiles", driver_profile_id, "revisions", revision_id, "transitions", "approve"]
            if method == "POST" =>
        {
            let payload = parse_json_body(body)?;
            transition_driver_profile_revision(
                &config.storage_root,
                driver_profile_revision_transition_input(
                    driver_profile_id,
                    revision_id,
                    EquipmentRevisionStatus::Approved,
                    &payload,
                )?,
            )
        }
        ["api", "v1", "driver-profiles", driver_profile_id, "audit-events"] if method == "GET" => {
            list_equipment_audit_events_for_driver(&config.storage_root, driver_profile_id)
        }
        ["api", "v1", collection, entity_id]
            if method == "GET"
                && measurement_engineering_kind_for_collection(collection).is_some() =>
        {
            get_measurement_engineering_definition(
                &config.storage_root,
                measurement_engineering_kind_for_collection(collection).unwrap(),
                entity_id,
            )
        }
        ["api", "v1", collection, entity_id, "clone"]
            if method == "POST"
                && measurement_engineering_kind_for_collection(collection).is_some() =>
        {
            let payload = parse_json_body(body)?;
            clone_measurement_engineering_definition(
                &config.storage_root,
                measurement_engineering_clone_input(
                    measurement_engineering_kind_for_collection(collection).unwrap(),
                    entity_id,
                    &payload,
                )?,
            )
        }
        ["api", "v1", collection, entity_id, "revisions"]
            if method == "GET"
                && measurement_engineering_kind_for_collection(collection).is_some() =>
        {
            list_measurement_engineering_revisions_json(
                &config.storage_root,
                measurement_engineering_kind_for_collection(collection).unwrap(),
                entity_id,
            )
        }
        ["api", "v1", collection, entity_id, "revisions"]
            if method == "POST"
                && measurement_engineering_kind_for_collection(collection).is_some() =>
        {
            let payload = parse_json_body(body)?;
            create_measurement_engineering_revision(
                &config.storage_root,
                measurement_engineering_revision_input(
                    measurement_engineering_kind_for_collection(collection).unwrap(),
                    entity_id,
                    &payload,
                )?,
            )
        }
        ["api", "v1", collection, entity_id, "revisions", revision_id]
            if method == "GET"
                && measurement_engineering_kind_for_collection(collection).is_some() =>
        {
            get_measurement_engineering_revision_json(
                &config.storage_root,
                measurement_engineering_kind_for_collection(collection).unwrap(),
                entity_id,
                revision_id,
            )
        }
        ["api", "v1", collection, entity_id, "revisions", revision_id, "definition"]
            if method == "PUT"
                && measurement_engineering_kind_for_collection(collection).is_some() =>
        {
            let payload = parse_json_body(body)?;
            replace_measurement_engineering_revision_definition(
                &config.storage_root,
                measurement_engineering_replace_input(
                    measurement_engineering_kind_for_collection(collection).unwrap(),
                    entity_id,
                    revision_id,
                    &payload,
                )?,
            )
        }
        ["api", "v1", collection, entity_id, "revisions", revision_id, "transitions", "submit-for-review"]
            if method == "POST"
                && measurement_engineering_kind_for_collection(collection).is_some() =>
        {
            let payload = parse_json_body(body)?;
            transition_measurement_engineering_revision(
                &config.storage_root,
                measurement_engineering_transition_input(
                    measurement_engineering_kind_for_collection(collection).unwrap(),
                    entity_id,
                    revision_id,
                    MeasurementEngineeringRevisionStatus::UnderReview,
                    &payload,
                )?,
            )
        }
        ["api", "v1", collection, entity_id, "revisions", revision_id, "transitions", "approve"]
            if method == "POST"
                && measurement_engineering_kind_for_collection(collection).is_some() =>
        {
            let payload = parse_json_body(body)?;
            transition_measurement_engineering_revision(
                &config.storage_root,
                measurement_engineering_transition_input(
                    measurement_engineering_kind_for_collection(collection).unwrap(),
                    entity_id,
                    revision_id,
                    MeasurementEngineeringRevisionStatus::Approved,
                    &payload,
                )?,
            )
        }
        ["api", "v1", "engineering-curves", curve_id, "revisions", revision_id, "evaluate"]
            if method == "POST" =>
        {
            let payload = parse_json_body(body)?;
            evaluate_engineering_curve_revision(
                &config.storage_root,
                engineering_curve_evaluate_input(curve_id, revision_id, &payload)?,
            )
        }
        ["api", "v1", collection, entity_id, "audit-events"]
            if method == "GET"
                && measurement_engineering_kind_for_collection(collection).is_some() =>
        {
            list_measurement_engineering_audit_events(
                &config.storage_root,
                measurement_engineering_kind_for_collection(collection).unwrap(),
                entity_id,
            )
        }
        ["api", "v1", "test-templates", template_id] if method == "GET" => {
            get_test_template_definition(&config.storage_root, template_id)
        }
        ["api", "v1", "test-templates", template_id, "clone"] if method == "POST" => {
            let payload = parse_json_body(body)?;
            clone_test_template(
                &config.storage_root,
                clone_test_template_input(template_id, &payload)?,
            )
        }
        ["api", "v1", "test-templates", template_id, "revisions"] if method == "GET" => {
            list_test_template_revisions(&config.storage_root, template_id)
        }
        ["api", "v1", "test-templates", template_id, "revisions"] if method == "POST" => {
            let payload = parse_json_body(body)?;
            create_test_template_revision(
                &config.storage_root,
                create_test_template_revision_input(template_id, &payload)?,
            )
        }
        ["api", "v1", "test-templates", template_id, "revisions", revision_id]
            if method == "GET" =>
        {
            get_test_template_revision(&config.storage_root, template_id, revision_id)
        }
        ["api", "v1", "test-templates", template_id, "revisions", revision_id, "definition"]
            if method == "PUT" =>
        {
            let payload = parse_json_body(body)?;
            replace_test_template_revision_definition(
                &config.storage_root,
                replace_test_template_definition_input(template_id, revision_id, &payload)?,
            )
        }
        ["api", "v1", "test-templates", template_id, "revisions", revision_id, "transitions", "submit-for-review"]
            if method == "POST" =>
        {
            let payload = parse_json_body(body)?;
            transition_test_template_revision(
                &config.storage_root,
                test_template_revision_transition_input(
                    template_id,
                    revision_id,
                    TemplateRevisionStatus::UnderReview,
                    &payload,
                )?,
            )
        }
        ["api", "v1", "test-templates", template_id, "revisions", revision_id, "transitions", "approve"]
            if method == "POST" =>
        {
            let payload = parse_json_body(body)?;
            transition_test_template_revision(
                &config.storage_root,
                test_template_revision_transition_input(
                    template_id,
                    revision_id,
                    TemplateRevisionStatus::Approved,
                    &payload,
                )?,
            )
        }
        ["api", "v1", "test-templates", template_id, "audit-events"] if method == "GET" => {
            list_test_template_audit_events(&config.storage_root, template_id)
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

fn register_document_input(payload: &Value) -> Result<RegisterAttachedDocumentInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(RegisterAttachedDocumentInput {
        document_id: required_string(payload, "document_id")?,
        classification: required_string(payload, "classification")?,
        title: required_string(payload, "title")?,
        owner_domain: required_string(payload, "owner_domain")?,
        owner_entity_type: required_string(payload, "owner_entity_type")?,
        owner_entity_id: required_string(payload, "owner_entity_id")?,
        storage_backend: optional_string(payload, "storage_backend")
            .unwrap_or_else(|| "object_store".to_owned()),
        storage_uri: required_string(payload, "storage_uri")?,
        original_filename: required_string(payload, "original_filename")?,
        mime_type: required_string(payload, "mime_type")?,
        size_bytes: required_u64(payload, "size_bytes")?,
        sha256: required_string(payload, "sha256")?,
        revision: optional_string(payload, "revision").unwrap_or_else(|| "A".to_owned()),
        applicability: optional_string(payload, "applicability")
            .unwrap_or_else(|| "applicable".to_owned()),
        confidentiality: optional_string(payload, "confidentiality")
            .unwrap_or_else(|| "internal".to_owned()),
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn list_documents_input(query: &str) -> ListAttachedDocumentsInput {
    ListAttachedDocumentsInput {
        owner_domain: optional_query_value(query, "owner_domain"),
        owner_entity_type: optional_query_value(query, "owner_entity_type"),
        owner_entity_id: optional_query_value(query, "owner_entity_id"),
    }
}

fn create_equipment_model_input(payload: &Value) -> Result<CreateEquipmentModelInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CreateEquipmentModelInput {
        equipment_model_id: required_string(payload, "equipment_model_id")?,
        definition_json: required_json_or_string(payload, "definition", "definition_json")?,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn list_equipment_models_input(query: &str) -> ListEquipmentModelsInput {
    ListEquipmentModelsInput {
        manufacturer: optional_query_value(query, "manufacturer"),
        equipment_class: optional_query_value(query, "equipment_class"),
        category_code: optional_query_value(query, "category_code"),
        functional_role: optional_query_value(query, "functional_role"),
        signal_domain: optional_query_value(query, "signal_domain"),
        technology_tag: optional_query_value(query, "technology_tag"),
        status: optional_query_value(query, "status"),
        search: optional_query_value(query, "q").or_else(|| optional_query_value(query, "search")),
    }
}

fn create_equipment_model_from_preset_input(
    payload: &Value,
) -> Result<CreateEquipmentModelFromPresetInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CreateEquipmentModelFromPresetInput {
        preset_id: required_string(payload, "preset_id")?,
        equipment_model_id: required_string(payload, "equipment_model_id")?,
        manufacturer: required_string(payload, "manufacturer")?,
        model_name: required_string(payload, "model_name")?,
        variant: optional_string(payload, "variant"),
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn replace_equipment_model_definition_input(
    equipment_model_id: &str,
    revision_id: &str,
    payload: &Value,
) -> Result<ReplaceEquipmentModelDefinitionInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(ReplaceEquipmentModelDefinitionInput {
        equipment_model_id: equipment_model_id.to_owned(),
        revision_id: revision_id.to_owned(),
        expected_definition_checksum: required_string(payload, "expected_definition_checksum")?,
        definition_json: required_json_or_string(payload, "definition", "definition_json")?,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn create_equipment_model_revision_input(
    equipment_model_id: &str,
    payload: &Value,
) -> Result<CreateEquipmentModelRevisionInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CreateEquipmentModelRevisionInput {
        equipment_model_id: equipment_model_id.to_owned(),
        source_revision_id: required_string(payload, "source_revision_id")?,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn clone_equipment_model_input(
    source_equipment_model_id: &str,
    payload: &Value,
) -> Result<CloneEquipmentModelInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CloneEquipmentModelInput {
        source_equipment_model_id: source_equipment_model_id.to_owned(),
        source_revision_id: optional_string(payload, "source_revision_id"),
        new_equipment_model_id: required_string(payload, "new_equipment_model_id")?,
        manufacturer: optional_string(payload, "manufacturer"),
        model_name: optional_string(payload, "model_name"),
        variant: optional_string(payload, "variant"),
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn equipment_model_revision_transition_input(
    equipment_model_id: &str,
    revision_id: &str,
    target_status: EquipmentRevisionStatus,
    payload: &Value,
) -> Result<TransitionEquipmentModelRevisionInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(TransitionEquipmentModelRevisionInput {
        equipment_model_id: equipment_model_id.to_owned(),
        revision_id: revision_id.to_owned(),
        target_status,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn create_driver_profile_input(payload: &Value) -> Result<CreateDriverProfileInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CreateDriverProfileInput {
        driver_profile_id: required_string(payload, "driver_profile_id")?,
        label: required_string(payload, "label")?,
        definition_json: required_json_or_string(payload, "definition", "definition_json")?,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn list_driver_profiles_input(query: &str) -> ListDriverProfilesInput {
    ListDriverProfilesInput {
        equipment_model_id: optional_query_value(query, "equipment_model_id"),
        status: optional_query_value(query, "status"),
        search: optional_query_value(query, "search"),
    }
}

fn replace_driver_profile_definition_input(
    driver_profile_id: &str,
    revision_id: &str,
    payload: &Value,
) -> Result<ReplaceDriverProfileDefinitionInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(ReplaceDriverProfileDefinitionInput {
        driver_profile_id: driver_profile_id.to_owned(),
        revision_id: revision_id.to_owned(),
        expected_definition_checksum: required_string(payload, "expected_definition_checksum")?,
        definition_json: required_json_or_string(payload, "definition", "definition_json")?,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn create_driver_profile_revision_input(
    driver_profile_id: &str,
    payload: &Value,
) -> Result<CreateDriverProfileRevisionInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CreateDriverProfileRevisionInput {
        driver_profile_id: driver_profile_id.to_owned(),
        source_revision_id: required_string(payload, "source_revision_id")?,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn driver_profile_revision_transition_input(
    driver_profile_id: &str,
    revision_id: &str,
    target_status: EquipmentRevisionStatus,
    payload: &Value,
) -> Result<TransitionDriverProfileRevisionInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(TransitionDriverProfileRevisionInput {
        driver_profile_id: driver_profile_id.to_owned(),
        revision_id: revision_id.to_owned(),
        target_status,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn driver_simulation_input(payload: &Value) -> Result<SimulateDriverProfileInput, AgentError> {
    Ok(SimulateDriverProfileInput {
        driver_profile_id: required_string(payload, "driver_profile_id")?,
        revision_id: optional_string(payload, "revision_id"),
        action_id: required_string(payload, "action_id")?,
        scenario_json: required_json_or_string(payload, "scenario", "scenario_json")?,
    })
}

fn measurement_engineering_kind_for_collection(
    collection: &str,
) -> Option<MeasurementEngineeringAggregateKind> {
    Some(match collection {
        "sensor-definitions" => MeasurementEngineeringAggregateKind::SensorDefinition,
        "scaling-profiles" => MeasurementEngineeringAggregateKind::ScalingProfile,
        "engineering-curves" => MeasurementEngineeringAggregateKind::EngineeringCurve,
        "daq-channel-profiles" => MeasurementEngineeringAggregateKind::DaqChannelProfile,
        "acquisition-channel-recipes" => {
            MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe
        }
        _ => return None,
    })
}

fn measurement_engineering_kind_for_validation(
    collection: &str,
) -> Option<MeasurementEngineeringAggregateKind> {
    Some(match collection {
        "sensor-definition-definitions" => MeasurementEngineeringAggregateKind::SensorDefinition,
        "scaling-profile-definitions" => MeasurementEngineeringAggregateKind::ScalingProfile,
        "engineering-curve-definitions" => MeasurementEngineeringAggregateKind::EngineeringCurve,
        "daq-channel-profile-definitions" => MeasurementEngineeringAggregateKind::DaqChannelProfile,
        "acquisition-channel-recipe-definitions" => {
            MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe
        }
        _ => return None,
    })
}

fn measurement_engineering_create_input(
    kind: MeasurementEngineeringAggregateKind,
    payload: &Value,
) -> Result<CreateMeasurementEngineeringInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CreateMeasurementEngineeringInput {
        kind,
        entity_id: measurement_engineering_entity_id(kind, payload)?,
        definition_json: required_json_or_string(payload, "definition", "definition_json")?,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn measurement_engineering_replace_input(
    kind: MeasurementEngineeringAggregateKind,
    entity_id: &str,
    revision_id: &str,
    payload: &Value,
) -> Result<ReplaceMeasurementEngineeringDefinitionInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(ReplaceMeasurementEngineeringDefinitionInput {
        kind,
        entity_id: entity_id.to_owned(),
        revision_id: revision_id.to_owned(),
        expected_definition_checksum: required_string(payload, "expected_definition_checksum")?,
        definition_json: required_json_or_string(payload, "definition", "definition_json")?,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn measurement_engineering_revision_input(
    kind: MeasurementEngineeringAggregateKind,
    entity_id: &str,
    payload: &Value,
) -> Result<CreateMeasurementEngineeringRevisionInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CreateMeasurementEngineeringRevisionInput {
        kind,
        entity_id: entity_id.to_owned(),
        source_revision_id: required_string(payload, "source_revision_id")?,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn measurement_engineering_clone_input(
    kind: MeasurementEngineeringAggregateKind,
    source_entity_id: &str,
    payload: &Value,
) -> Result<CloneMeasurementEngineeringInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CloneMeasurementEngineeringInput {
        kind,
        source_entity_id: source_entity_id.to_owned(),
        source_revision_id: optional_string(payload, "source_revision_id"),
        new_entity_id: required_string(payload, "new_entity_id")?,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn measurement_engineering_transition_input(
    kind: MeasurementEngineeringAggregateKind,
    entity_id: &str,
    revision_id: &str,
    target_status: MeasurementEngineeringRevisionStatus,
    payload: &Value,
) -> Result<TransitionMeasurementEngineeringRevisionInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(TransitionMeasurementEngineeringRevisionInput {
        kind,
        entity_id: entity_id.to_owned(),
        revision_id: revision_id.to_owned(),
        target_status,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn engineering_curve_evaluate_input(
    curve_id: &str,
    revision_id: &str,
    payload: &Value,
) -> Result<EvaluateEngineeringCurveInput, AgentError> {
    let axis_values = payload
        .get("axis_values")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            AgentError::with_details(
                "missing_json_field",
                "axis_values is required",
                json!({ "field": "axis_values" }),
            )
        })?
        .iter()
        .map(|(key, value)| {
            value
                .as_f64()
                .map(|number| (key.clone(), number))
                .ok_or_else(|| {
                    AgentError::with_details(
                        "invalid_json_field",
                        "axis_values entries must be numbers",
                        json!({ "field": key }),
                    )
                })
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(EvaluateEngineeringCurveInput {
        curve_id: curve_id.to_owned(),
        revision_id: revision_id.to_owned(),
        axis_values,
    })
}

fn measurement_engineering_entity_id(
    kind: MeasurementEngineeringAggregateKind,
    payload: &Value,
) -> Result<String, AgentError> {
    if let Some(entity_id) = optional_string(payload, "entity_id") {
        return Ok(entity_id);
    }
    let key = match kind {
        MeasurementEngineeringAggregateKind::SensorDefinition => "sensor_definition_id",
        MeasurementEngineeringAggregateKind::ScalingProfile => "scaling_profile_id",
        MeasurementEngineeringAggregateKind::EngineeringCurve => "curve_id",
        MeasurementEngineeringAggregateKind::DaqChannelProfile => "daq_channel_profile_id",
        MeasurementEngineeringAggregateKind::AcquisitionChannelRecipe => "recipe_id",
    };
    required_string(payload, key)
}

fn create_test_template_input(payload: &Value) -> Result<CreateTestTemplateInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CreateTestTemplateInput {
        template_id: required_string(payload, "template_id")?,
        title: required_string(payload, "title")?,
        category_code: required_string(payload, "category_code")?,
        definition_json: required_json_or_string(payload, "definition", "definition_json")?,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn list_test_templates_input(query: &str) -> ListTestTemplatesInput {
    ListTestTemplatesInput {
        category_code: optional_query_value(query, "category_code"),
    }
}

fn replace_test_template_definition_input(
    template_id: &str,
    revision_id: &str,
    payload: &Value,
) -> Result<ReplaceTestTemplateDefinitionInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(ReplaceTestTemplateDefinitionInput {
        template_id: template_id.to_owned(),
        revision_id: revision_id.to_owned(),
        expected_definition_checksum: required_string(payload, "expected_definition_checksum")?,
        definition_json: required_json_or_string(payload, "definition", "definition_json")?,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn create_test_template_revision_input(
    template_id: &str,
    payload: &Value,
) -> Result<CreateTestTemplateRevisionInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CreateTestTemplateRevisionInput {
        template_id: template_id.to_owned(),
        source_revision_id: required_string(payload, "source_revision_id")?,
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn clone_test_template_input(
    source_template_id: &str,
    payload: &Value,
) -> Result<CloneTestTemplateInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(CloneTestTemplateInput {
        source_template_id: source_template_id.to_owned(),
        source_revision_id: optional_string(payload, "source_revision_id"),
        new_template_id: required_string(payload, "new_template_id")?,
        title: required_string(payload, "title")?,
        category_code: optional_string(payload, "category_code"),
        actor: required_string(payload, "actor")?,
        reason: required_string(payload, "reason")?,
        correlation_id: optional_string(payload, "correlation_id")
            .unwrap_or_else(|| operation_id.clone()),
        device_id: optional_string(payload, "device_id")
            .unwrap_or_else(|| "local-agent".to_owned()),
        operation_id,
    })
}

fn test_template_revision_transition_input(
    template_id: &str,
    revision_id: &str,
    target_status: TemplateRevisionStatus,
    payload: &Value,
) -> Result<TransitionTestTemplateRevisionInput, AgentError> {
    let operation_id = required_string(payload, "operation_id")?;
    Ok(TransitionTestTemplateRevisionInput {
        template_id: template_id.to_owned(),
        revision_id: revision_id.to_owned(),
        target_status,
        actor: required_string(payload, "actor")?,
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

fn optional_query_value(query: &str, key: &'static str) -> Option<String> {
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (candidate, value) = pair.split_once('=').unwrap_or((pair, ""));
        if candidate == key && !value.trim().is_empty() {
            return Some(value.to_owned());
        }
    }
    None
}

fn optional_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn required_json_or_string(
    payload: &Value,
    key: &'static str,
    string_key: &'static str,
) -> Result<String, AgentError> {
    if let Some(value) = payload.get(key) {
        return Ok(render_json(value));
    }
    optional_string(payload, string_key).ok_or_else(|| {
        AgentError::with_details(
            "missing_json_field",
            format!("missing required JSON field: {key}"),
            json!({ "field": key, "string_field": string_key }),
        )
    })
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

fn required_u64(payload: &Value, key: &'static str) -> Result<u64, AgentError> {
    payload.get(key).and_then(Value::as_u64).ok_or_else(|| {
        AgentError::with_details(
            "invalid_json_field",
            format!("{key} must be a non-negative integer"),
            json!({ "field": key }),
        )
    })
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
        | "attached_document_not_found"
        | "document_owner_not_found"
        | "project_not_found"
        | "test_execution_not_found"
        | "test_template_not_found"
        | "test_template_revision_not_found"
        | "test_template_category_not_found"
        | "test_template_method_revision_not_found"
        | "equipment_model_not_found"
        | "equipment_model_revision_not_found"
        | "equipment_model_class_not_found"
        | "equipment_classification_preset_not_found"
        | "driver_profile_not_found"
        | "driver_profile_revision_not_found"
        | "measurement_engineering_not_found"
        | "measurement_engineering_revision_not_found"
        | "metrology_instrument_not_found" => 404,
        "contract_review_incomplete"
        | "invalid_project_transition"
        | "project_already_exists"
        | "test_execution_attempt_exists"
        | "test_execution_template_not_approved"
        | "test_template_already_exists"
        | "test_template_definition_checksum_mismatch"
        | "test_template_definition_concurrent_update"
        | "test_template_active_draft_exists"
        | "test_template_revision_immutable"
        | "test_template_revision_source_not_approved"
        | "test_template_revision_transition_conflict"
        | "test_template_revision_transition_not_allowed"
        | "equipment_model_already_exists"
        | "driver_profile_already_exists"
        | "equipment_definition_checksum_mismatch"
        | "equipment_active_draft_exists"
        | "equipment_revision_immutable"
        | "equipment_revision_source_not_approved"
        | "equipment_revision_transition_conflict"
        | "equipment_revision_transition_not_allowed"
        | "equipment_classification_preset_deprecated"
        | "driver_model_revision_not_approved"
        | "driver_model_definition_checksum_mismatch"
        | "driver_simulation_revision_mismatch"
        | "driver_simulation_action_mismatch"
        | "measurement_engineering_already_exists"
        | "measurement_engineering_definition_checksum_mismatch"
        | "measurement_engineering_definition_concurrent_update"
        | "measurement_engineering_active_draft_exists"
        | "measurement_engineering_revision_immutable"
        | "measurement_engineering_revision_source_not_approved"
        | "measurement_engineering_revision_transition_conflict"
        | "measurement_engineering_revision_transition_not_allowed"
        | "measurement_engineering_revision_checksum_invalid"
        | "attached_document_already_exists"
        | "operation_replay_mismatch"
        | "metrology_instrument_already_exists"
        | "metrology_calibration_already_exists" => 409,
        "storage_not_initialized" | "lab_console_build_missing" => 503,
        "invalid_json_body"
        | "missing_json_field"
        | "missing_query_field"
        | "invalid_json_field"
        | "invalid_lab_console_path"
        | "invalid_attached_document"
        | "invalid_metrology_date"
        | "missing_argument"
        | "invalid_project_code"
        | "invalid_customer_name"
        | "invalid_actor"
        | "invalid_reason"
        | "domain_error"
        | "invalid_test_execution"
        | "invalid_test_template"
        | "invalid_test_template_definition"
        | "invalid_test_template_revision_status"
        | "invalid_equipment_identifier"
        | "invalid_equipment_registry_json"
        | "invalid_equipment_registry_value"
        | "invalid_manufacturer"
        | "invalid_model_name"
        | "invalid_equipment_model_definition"
        | "invalid_driver_profile_definition"
        | "invalid_driver_simulation"
        | "invalid_driver_simulation_scenario"
        | "invalid_checksum"
        | "invalid_stable_id"
        | "invalid_measurement_engineering_definition"
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
    fn local_api_serves_lab_console_build_and_keeps_api_accessible() {
        let storage_root = temporary_storage_root("agent-api-lab-static");
        let lab_dist = temporary_storage_root("agent-api-lab-dist");
        create_lab_dist_fixture(&lab_dist);
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: lab_dist.clone(),
            max_requests: None,
        };

        let root = handle_api_request("GET", "/", "", &config);
        let lab = handle_api_request("GET", "/lab/", "", &config);
        let asset = handle_api_request("GET", "/lab/assets/app.js", "", &config);
        let fallback = handle_api_request("GET", "/lab/templates/TT-DEEP", "", &config);
        let traversal = handle_api_request("GET", "/lab/assets/..%2Fsecret.txt", "", &config);
        let health = handle_api_request("GET", "/api/v1/health", "", &config);

        assert_eq!(root.status, 302);
        assert_eq!(root.location.as_deref(), Some("/lab/"));
        assert_eq!(lab.status, 200);
        assert_eq!(lab.content_type, "text/html; charset=utf-8");
        assert!(lab.body.contains("LAB CONSOLE"));
        assert_eq!(asset.status, 200);
        assert_eq!(asset.content_type, "text/javascript; charset=utf-8");
        assert!(asset.body.contains("lab console asset"));
        assert_eq!(fallback.status, 200);
        assert!(fallback.body.contains("LAB CONSOLE"));
        assert_eq!(traversal.status, 400);
        assert!(traversal.body.contains("invalid_lab_console_path"));
        assert_eq!(health.status, 200);
        assert!(health.body.contains("\"agent\":\"emc-locus-agent\""));

        remove_temporary_storage_root(&storage_root);
        remove_temporary_storage_root(&lab_dist);
    }

    #[test]
    fn local_api_reports_missing_lab_console_build_explicitly() {
        let storage_root = temporary_storage_root("agent-api-lab-missing");
        let missing_dist = temporary_storage_root("agent-api-lab-missing-dist");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: missing_dist.clone(),
            max_requests: None,
        };

        let response = handle_api_request("GET", "/lab/", "", &config);

        assert_eq!(response.status, 503);
        assert!(response.body.contains("lab_console_build_missing"));
        assert!(response
            .body
            .contains("LAB CONSOLE production build is not available"));

        remove_temporary_storage_root(&storage_root);
        remove_temporary_storage_root(&missing_dist);
    }

    #[test]
    fn local_api_validates_test_template_definitions_structurally() {
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: PathBuf::from("unused"),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
            max_requests: None,
        };
        let valid = handle_api_request(
            "POST",
            "/api/v1/test-template-definitions/validate",
            &format!(
                r#"{{"definition": {}}}"#,
                template_definition(100_000, None)
            ),
            &config,
        );
        let invalid_definition = template_definition_without_variable_unit();
        let invalid = handle_api_request(
            "POST",
            "/api/v1/test-template-definitions/validate",
            &format!(r#"{{"definition": {invalid_definition}}}"#),
            &config,
        );

        assert_eq!(valid.status, 200);
        assert!(valid.body.contains("\"valid\":true"));
        assert!(valid.body.contains("\"definition_checksum\":\"sha256:"));
        assert_eq!(invalid.status, 200);
        assert!(invalid.body.contains("\"valid\":false"));
        assert!(invalid.body.contains("\"severity\":\"error\""));
        assert!(invalid.body.contains("\"code\":\"missing_variable_unit\""));
        assert!(invalid.body.contains("\"path\":\"variables\""));
    }

    #[test]
    fn local_api_runs_equipment_model_driver_and_simulation_workflow() {
        let storage_root = temporary_storage_root("agent-api-equipment-workflow");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
            max_requests: None,
        };
        let initialized = handle_api_request("POST", "/api/v1/storage/initialize", "", &config);
        assert_eq!(initialized.status, 200, "{}", initialized.body);

        let model_definition = equipment_model_definition("R&S", "NRP6AN", Some("FWD"));
        let create_model = handle_api_request(
            "POST",
            "/api/v1/equipment-models",
            &json!({
                "equipment_model_id": "EQM-NRP6AN-FWD",
                "definition": model_definition,
                "actor": "equipment.author",
                "reason": "create power meter model",
                "operation_id": "op-eqm-create"
            })
            .to_string(),
            &config,
        );
        assert_eq!(create_model.status, 200, "{}", create_model.body);
        assert!(create_model
            .body
            .contains("\"operation\":\"equipment_model_created\""));
        let model_revision_id = "EQM-NRP6AN-FWD-rev-0001";

        let submit_model = handle_api_request(
            "POST",
            &format!(
                "/api/v1/equipment-models/EQM-NRP6AN-FWD/revisions/{model_revision_id}/transitions/submit-for-review"
            ),
            &transition_body("equipment.author", "submit model", "op-eqm-submit"),
            &config,
        );
        assert_eq!(submit_model.status, 200, "{}", submit_model.body);
        let approve_model = handle_api_request(
            "POST",
            &format!(
                "/api/v1/equipment-models/EQM-NRP6AN-FWD/revisions/{model_revision_id}/transitions/approve"
            ),
            &transition_body("quality.approver", "approve model", "op-eqm-approve"),
            &config,
        );
        assert_eq!(approve_model.status, 200, "{}", approve_model.body);

        let model_detail = handle_api_request(
            "GET",
            "/api/v1/equipment-models/EQM-NRP6AN-FWD",
            "",
            &config,
        );
        assert_eq!(model_detail.status, 200, "{}", model_detail.body);
        let model_json: Value = serde_json::from_str(&model_detail.body).unwrap();
        let model_checksum = model_json["equipment_model"]["current_approved_revision"]
            ["definition_checksum"]
            .as_str()
            .unwrap();

        let driver_definition = driver_profile_definition(model_revision_id, model_checksum);
        let validate_driver = handle_api_request(
            "POST",
            "/api/v1/driver-profile-definitions/validate",
            &json!({ "definition": driver_definition.clone() }).to_string(),
            &config,
        );
        assert_eq!(validate_driver.status, 200, "{}", validate_driver.body);
        assert!(validate_driver.body.contains("\"valid\":true"));

        let create_driver = handle_api_request(
            "POST",
            "/api/v1/driver-profiles",
            &json!({
                "driver_profile_id": "DRV-NRP6AN-SCPI",
                "label": "NRP6AN SCPI simulation driver",
                "definition": driver_definition,
                "actor": "driver.author",
                "reason": "create SCPI power meter driver",
                "operation_id": "op-driver-create"
            })
            .to_string(),
            &config,
        );
        assert_eq!(create_driver.status, 200, "{}", create_driver.body);
        let create_driver_json: Value = serde_json::from_str(&create_driver.body).unwrap();
        let driver_initial_checksum = create_driver_json["revision"]["definition_checksum"]
            .as_str()
            .unwrap();
        let driver_revision_id = "DRV-NRP6AN-SCPI-rev-0001";

        let uppercase_driver_checksum = uppercase_checksum_payload(driver_initial_checksum);
        let driver_uppercase_checksum = handle_api_request(
            "PUT",
            &format!(
                "/api/v1/driver-profiles/DRV-NRP6AN-SCPI/revisions/{driver_revision_id}/definition"
            ),
            &json!({
                "expected_definition_checksum": uppercase_driver_checksum,
                "definition": driver_profile_definition(model_revision_id, model_checksum),
                "actor": "driver.author",
                "reason": "attempt uppercase checksum",
                "operation_id": "op-driver-uppercase-checksum"
            })
            .to_string(),
            &config,
        );
        assert_eq!(
            driver_uppercase_checksum.status, 400,
            "{}",
            driver_uppercase_checksum.body
        );
        assert!(driver_uppercase_checksum.body.contains("invalid_checksum"));

        let submit_driver = handle_api_request(
            "POST",
            &format!(
                "/api/v1/driver-profiles/DRV-NRP6AN-SCPI/revisions/{driver_revision_id}/transitions/submit-for-review"
            ),
            &transition_body("driver.author", "submit driver", "op-driver-submit"),
            &config,
        );
        assert_eq!(submit_driver.status, 200, "{}", submit_driver.body);
        let approve_driver = handle_api_request(
            "POST",
            &format!(
                "/api/v1/driver-profiles/DRV-NRP6AN-SCPI/revisions/{driver_revision_id}/transitions/approve"
            ),
            &transition_body("quality.approver", "approve driver", "op-driver-approve"),
            &config,
        );
        assert_eq!(approve_driver.status, 200, "{}", approve_driver.body);

        let simulation = handle_api_request(
            "POST",
            "/api/v1/driver-profile-simulations",
            &json!({
                "driver_profile_id": "DRV-NRP6AN-SCPI",
                "revision_id": driver_revision_id,
                "action_id": "measure_powers",
                "scenario": {
                    "scenario_id": "scenario-power-ok",
                    "driver_revision_id": driver_revision_id,
                    "action_id": "measure_powers",
                    "input_values": {},
                    "expected_transport_operations": ["query"],
                    "simulated_responses": ["-12.5"],
                    "expected_outputs": {"result.power_dbm": -12.5},
                    "expected_messages": [],
                    "expected_final_state": {}
                }
            })
            .to_string(),
            &config,
        );
        assert_eq!(simulation.status, 200, "{}", simulation.body);
        assert!(simulation.body.contains("\"operation\":\"query\""));
        assert!(simulation.body.contains("power_dbm"));

        let providers = handle_api_request(
            "GET",
            "/api/v1/equipment/communication-providers",
            "",
            &config,
        );
        assert_eq!(providers.status, 200, "{}", providers.body);
        assert!(providers.body.contains("\"provider\":\"visa\""));
        assert!(providers.body.contains("No VISA implementation installed"));

        let audit = handle_api_request(
            "GET",
            "/api/v1/equipment-models/EQM-NRP6AN-FWD/audit-events",
            "",
            &config,
        );
        assert_eq!(audit.status, 200, "{}", audit.body);
        assert!(audit
            .body
            .contains("\"action\":\"equipment_model_approved\""));

        let outbox = handle_api_request("GET", "/api/v1/sync/outbox", "", &config);
        assert_eq!(outbox.status, 200, "{}", outbox.body);
        assert!(outbox.body.contains("\"domain\":\"equipment\""));
        assert!(outbox
            .body
            .contains("\"operation_kind\":\"driver_profile_approved\""));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_runs_measurement_engineering_workflow() {
        let storage_root = temporary_storage_root("agent-api-measurement-engineering");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
            max_requests: None,
        };
        assert_eq!(
            handle_api_request("POST", "/api/v1/storage/initialize", "", &config).status,
            200
        );

        let initial_scaling_definition = scaling_definition("demo-current-probe-10mv-a", 100.0);
        let scaling_validation = handle_api_request(
            "POST",
            "/api/v1/scaling-profile-definitions/validate",
            &render_json(&json!({ "definition": initial_scaling_definition })),
            &config,
        );
        assert_eq!(
            scaling_validation.status, 200,
            "{}",
            scaling_validation.body
        );
        assert!(scaling_validation.body.contains("\"valid\":true"));

        let scaling_created = handle_api_request(
            "POST",
            "/api/v1/scaling-profiles",
            &create_measurement_body(
                "demo-current-probe-10mv-a",
                initial_scaling_definition.clone(),
                "op-scaling-create",
            ),
            &config,
        );
        assert_eq!(scaling_created.status, 200, "{}", scaling_created.body);
        let scaling_created_json: Value = serde_json::from_str(&scaling_created.body).unwrap();
        let scaling_revision_id = scaling_created_json["revision"]["revision_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let scaling_initial_checksum = scaling_created_json["revision"]["definition_checksum"]
            .as_str()
            .unwrap()
            .to_owned();
        let uppercase_scaling_checksum = uppercase_checksum_payload(&scaling_initial_checksum);
        let scaling_uppercase_checksum = handle_api_request(
            "PUT",
            &format!(
                "/api/v1/scaling-profiles/demo-current-probe-10mv-a/revisions/{scaling_revision_id}/definition"
            ),
            &replace_measurement_body(
                initial_scaling_definition.clone(),
                &uppercase_scaling_checksum,
                "op-scaling-uppercase-checksum",
            ),
            &config,
        );
        assert_eq!(
            scaling_uppercase_checksum.status, 400,
            "{}",
            scaling_uppercase_checksum.body
        );
        assert!(scaling_uppercase_checksum.body.contains("invalid_checksum"));
        let scaling_create_replay = handle_api_request(
            "POST",
            "/api/v1/scaling-profiles",
            &create_measurement_body(
                "demo-current-probe-10mv-a",
                initial_scaling_definition.clone(),
                "op-scaling-create",
            ),
            &config,
        );
        assert_eq!(
            scaling_create_replay.status, 200,
            "{}",
            scaling_create_replay.body
        );
        assert!(scaling_create_replay.body.contains("\"replayed\":true"));

        let scaling_updated_definition = scaling_definition("demo-current-probe-10mv-a", 101.0);
        let scaling_edited = handle_api_request(
            "PUT",
            &format!(
                "/api/v1/scaling-profiles/demo-current-probe-10mv-a/revisions/{scaling_revision_id}/definition"
            ),
            &replace_measurement_body(
                scaling_updated_definition.clone(),
                &scaling_initial_checksum,
                "op-scaling-save",
            ),
            &config,
        );
        assert_eq!(scaling_edited.status, 200, "{}", scaling_edited.body);
        let scaling_edited_json: Value = serde_json::from_str(&scaling_edited.body).unwrap();
        let scaling_checksum = scaling_edited_json["revision"]["definition_checksum"]
            .as_str()
            .unwrap()
            .to_owned();
        let scaling_edit_replay = handle_api_request(
            "PUT",
            &format!(
                "/api/v1/scaling-profiles/demo-current-probe-10mv-a/revisions/{scaling_revision_id}/definition"
            ),
            &replace_measurement_body(
                scaling_updated_definition.clone(),
                &scaling_initial_checksum,
                "op-scaling-save",
            ),
            &config,
        );
        assert_eq!(
            scaling_edit_replay.status, 200,
            "{}",
            scaling_edit_replay.body
        );
        assert!(scaling_edit_replay.body.contains("\"replayed\":true"));

        let scaling_stale = handle_api_request(
            "PUT",
            &format!(
                "/api/v1/scaling-profiles/demo-current-probe-10mv-a/revisions/{scaling_revision_id}/definition"
            ),
            &replace_measurement_body(
                scaling_updated_definition,
                &scaling_initial_checksum,
                "op-scaling-stale",
            ),
            &config,
        );
        assert_eq!(scaling_stale.status, 409, "{}", scaling_stale.body);
        assert!(scaling_stale
            .body
            .contains("measurement_engineering_definition_checksum_mismatch"));

        approve_measurement_revision(
            &config,
            "scaling-profiles",
            "demo-current-probe-10mv-a",
            &scaling_revision_id,
            "op-scaling-submit",
            "op-scaling-approve",
        );
        let scaling_submit_replay = handle_api_request(
            "POST",
            &format!(
                "/api/v1/scaling-profiles/demo-current-probe-10mv-a/revisions/{scaling_revision_id}/transitions/submit-for-review"
            ),
            &transition_body(
                "measurement.author",
                "submit measurement engineering definition",
                "op-scaling-submit",
            ),
            &config,
        );
        assert_eq!(
            scaling_submit_replay.status, 200,
            "{}",
            scaling_submit_replay.body
        );
        assert!(scaling_submit_replay.body.contains("\"replayed\":true"));
        let scaling_approve_replay = handle_api_request(
            "POST",
            &format!(
                "/api/v1/scaling-profiles/demo-current-probe-10mv-a/revisions/{scaling_revision_id}/transitions/approve"
            ),
            &transition_body(
                "quality.approver",
                "approve measurement engineering definition",
                "op-scaling-approve",
            ),
            &config,
        );
        assert_eq!(
            scaling_approve_replay.status, 200,
            "{}",
            scaling_approve_replay.body
        );
        assert!(scaling_approve_replay.body.contains("\"replayed\":true"));

        let curve_revision_id = create_and_approve_measurement(
            &config,
            "engineering-curves",
            "demo-current-probe-transfer",
            curve_definition("demo-current-probe-transfer", "current_probe_transfer"),
            "curve",
        );
        let evaluation = handle_api_request(
            "POST",
            &format!(
                "/api/v1/engineering-curves/demo-current-probe-transfer/revisions/{curve_revision_id}/evaluate"
            ),
            r#"{"axis_values":{"frequency":100000000.0}}"#,
            &config,
        );
        assert_eq!(evaluation.status, 200, "{}", evaluation.body);
        assert!(evaluation.body.contains("\"correction_db\":1.0"));

        create_and_approve_measurement(
            &config,
            "daq-channel-profiles",
            "demo-daq-ai-10v",
            daq_definition("demo-daq-ai-10v"),
            "daq",
        );

        create_and_approve_measurement(
            &config,
            "sensor-definitions",
            "demo-current-probe",
            sensor_definition(
                "demo-current-probe",
                "demo-current-probe-10mv-a",
                "demo-current-probe-transfer",
            ),
            "sensor",
        );

        let recipe_revision_id = create_and_approve_measurement(
            &config,
            "acquisition-channel-recipes",
            "current-a",
            recipe_definition(
                "current-a",
                "demo-daq-ai-10v",
                "demo-current-probe",
                "demo-current-probe-10mv-a",
                "demo-current-probe-transfer",
            ),
            "recipe",
        );

        let recipe_revisions = handle_api_request(
            "GET",
            "/api/v1/acquisition-channel-recipes/current-a/revisions",
            "",
            &config,
        );
        assert_eq!(recipe_revisions.status, 200, "{}", recipe_revisions.body);
        assert!(recipe_revisions.body.contains(&recipe_revision_id));
        assert!(recipe_revisions.body.contains("\"status\":\"approved\""));

        let audit = handle_api_request(
            "GET",
            "/api/v1/acquisition-channel-recipes/current-a/audit-events",
            "",
            &config,
        );
        assert_eq!(audit.status, 200, "{}", audit.body);
        assert!(audit
            .body
            .contains("\"action\":\"acquisition_channel_recipe_approved\""));

        let outbox = handle_api_request("GET", "/api/v1/sync/outbox", "", &config);
        assert_eq!(outbox.status, 200, "{}", outbox.body);
        assert!(outbox.body.contains("\"domain\":\"equipment\""));
        assert!(outbox
            .body
            .contains("\"operation_kind\":\"engineering_curve_approved\""));
        assert!(outbox
            .body
            .contains("\"operation_kind\":\"acquisition_channel_recipe_approved\""));
        assert!(outbox.body.contains(&scaling_checksum));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_creates_equipment_model_from_classification_preset_and_filters_summaries() {
        let storage_root = temporary_storage_root("agent-api-equipment-presets");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
            max_requests: None,
        };
        assert_eq!(
            handle_api_request("POST", "/api/v1/storage/initialize", "", &config).status,
            200
        );

        let registries = handle_api_request("GET", "/api/v1/equipment/registries", "", &config);
        assert_eq!(registries.status, 200, "{}", registries.body);
        assert!(registries.body.contains("\"code\":\"acquisition_device\""));
        assert!(registries.body.contains("\"code\":\"can_bus\""));

        let preset_list = handle_api_request(
            "GET",
            "/api/v1/equipment/classification-presets",
            "",
            &config,
        );
        assert_eq!(preset_list.status, 200, "{}", preset_list.body);
        assert!(preset_list.body.contains("\"preset_id\":\"adc_converter\""));
        assert!(preset_list
            .body
            .contains("\"preset_id\":\"can_bus_controlled_unit\""));

        let adc_preset = handle_api_request(
            "GET",
            "/api/v1/equipment/classification-presets/adc_converter",
            "",
            &config,
        );
        assert_eq!(adc_preset.status, 200, "{}", adc_preset.body);
        assert!(adc_preset
            .body
            .contains("\"technology_tags\":[\"adc_converter\""));
        assert!(!adc_preset.body.contains("\"signal_domain\":\"can_bus\""));

        let created = handle_api_request(
            "POST",
            "/api/v1/equipment-models/from-preset",
            &json!({
                "preset_id": "adc_converter",
                "equipment_model_id": "EQM-ADC-PRESET",
                "manufacturer": "Demo",
                "model_name": "ADC-16",
                "variant": "portable",
                "actor": "equipment.author",
                "reason": "create ADC converter from backend preset",
                "operation_id": "op-eqm-adc-from-preset"
            })
            .to_string(),
            &config,
        );
        assert_eq!(created.status, 200, "{}", created.body);
        assert!(created
            .body
            .contains("\"operation\":\"equipment_model_created_from_preset\""));
        assert!(created
            .body
            .contains("\"classification_preset_id\":\"adc_converter\""));
        let created_json: Value = serde_json::from_str(&created.body).unwrap();
        let mut edited_definition = created_json["revision"]["definition"].clone();
        edited_definition["signal_domains"]
            .as_array_mut()
            .unwrap()
            .push(json!("usb"));
        edited_definition["technology_tags"]
            .as_array_mut()
            .unwrap()
            .push(json!("usb"));

        let before_draft_save_usb_filter = handle_api_request(
            "GET",
            "/api/v1/equipment-models?technology_tag=usb",
            "",
            &config,
        );
        assert_eq!(
            before_draft_save_usb_filter.status, 200,
            "{}",
            before_draft_save_usb_filter.body
        );
        assert!(!before_draft_save_usb_filter.body.contains("EQM-ADC-PRESET"));

        let draft_saved = handle_api_request(
            "PUT",
            "/api/v1/equipment-models/EQM-ADC-PRESET/revisions/EQM-ADC-PRESET-rev-0001/definition",
            &json!({
                "expected_definition_checksum": created_json["revision"]["definition_checksum"],
                "definition": edited_definition,
                "actor": "equipment.author",
                "reason": "add USB summary tag to draft",
                "operation_id": "op-eqm-adc-draft-save-usb"
            })
            .to_string(),
            &config,
        );
        assert_eq!(draft_saved.status, 200, "{}", draft_saved.body);

        let after_draft_save_usb_filter = handle_api_request(
            "GET",
            "/api/v1/equipment-models?technology_tag=usb",
            "",
            &config,
        );
        assert_eq!(
            after_draft_save_usb_filter.status, 200,
            "{}",
            after_draft_save_usb_filter.body
        );
        assert!(after_draft_save_usb_filter.body.contains("EQM-ADC-PRESET"));

        let converter_filter = handle_api_request(
            "GET",
            "/api/v1/equipment-models?functional_role=converter&signal_domain=analog_voltage&technology_tag=adc_converter&q=ADC",
            "",
            &config,
        );
        assert_eq!(converter_filter.status, 200, "{}", converter_filter.body);
        assert!(converter_filter.body.contains("EQM-ADC-PRESET"));

        let can_filter = handle_api_request(
            "GET",
            "/api/v1/equipment-models?signal_domain=can_bus",
            "",
            &config,
        );
        assert_eq!(can_filter.status, 200, "{}", can_filter.body);
        assert!(!can_filter.body.contains("EQM-ADC-PRESET"));

        let submit = handle_api_request(
            "POST",
            "/api/v1/equipment-models/EQM-ADC-PRESET/revisions/EQM-ADC-PRESET-rev-0001/transitions/submit-for-review",
            &transition_body("equipment.author", "submit ADC preset model", "op-eqm-adc-submit"),
            &config,
        );
        assert_eq!(submit.status, 200, "{}", submit.body);
        let approve = handle_api_request(
            "POST",
            "/api/v1/equipment-models/EQM-ADC-PRESET/revisions/EQM-ADC-PRESET-rev-0001/transitions/approve",
            &transition_body("quality.approver", "approve ADC preset model", "op-eqm-adc-approve"),
            &config,
        );
        assert_eq!(approve.status, 200, "{}", approve.body);

        let approved_filter = handle_api_request(
            "GET",
            "/api/v1/equipment-models?functional_role=converter&technology_tag=adc_converter&status=approved",
            "",
            &config,
        );
        assert_eq!(approved_filter.status, 200, "{}", approved_filter.body);
        assert!(approved_filter.body.contains("EQM-ADC-PRESET"));

        let audit = handle_api_request(
            "GET",
            "/api/v1/equipment-models/EQM-ADC-PRESET/audit-events",
            "",
            &config,
        );
        assert_eq!(audit.status, 200, "{}", audit.body);
        assert!(audit
            .body
            .contains("\"action\":\"equipment_model_created_from_preset\""));
        assert!(audit
            .body
            .contains("\"action\":\"equipment_model_approved\""));

        let outbox = handle_api_request("GET", "/api/v1/sync/outbox", "", &config);
        assert_eq!(outbox.status, 200, "{}", outbox.body);
        assert!(outbox
            .body
            .contains("\"operation_kind\":\"equipment_model_created_from_preset\""));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_guards_equipment_validation_cas_and_immutability() {
        let storage_root = temporary_storage_root("agent-api-equipment-cas");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
            max_requests: None,
        };
        assert_eq!(
            handle_api_request("POST", "/api/v1/storage/initialize", "", &config).status,
            200
        );

        let mut invalid_definition = equipment_model_definition("Demo", "Bad", None);
        invalid_definition["signal_ports"][0]["unit"] = json!("Hz");
        let duplicated_port = invalid_definition["signal_ports"][0].clone();
        invalid_definition["signal_ports"]
            .as_array_mut()
            .unwrap()
            .push(duplicated_port);
        let validation = handle_api_request(
            "POST",
            "/api/v1/equipment-model-definitions/validate",
            &json!({ "definition": invalid_definition }).to_string(),
            &config,
        );
        assert_eq!(validation.status, 200, "{}", validation.body);
        assert!(validation.body.contains("\"valid\":false"));
        assert!(validation.body.contains("quantity_unit_mismatch"));
        assert!(validation.body.contains("duplicate_signal_port_id"));

        let definition = equipment_model_definition("Demo", "Amplifier", None);
        let created = handle_api_request(
            "POST",
            "/api/v1/equipment-models",
            &json!({
                "equipment_model_id": "EQM-DEMO-AMP",
                "definition": definition,
                "actor": "equipment.author",
                "reason": "create model",
                "operation_id": "op-amp-create"
            })
            .to_string(),
            &config,
        );
        assert_eq!(created.status, 200, "{}", created.body);
        let created_json: Value = serde_json::from_str(&created.body).unwrap();
        let initial_checksum = created_json["revision"]["definition_checksum"]
            .as_str()
            .unwrap();

        let uppercase_checksum = uppercase_checksum_payload(initial_checksum);
        let uppercase_checksum_edit = handle_api_request(
            "PUT",
            "/api/v1/equipment-models/EQM-DEMO-AMP/revisions/EQM-DEMO-AMP-rev-0001/definition",
            &json!({
                "expected_definition_checksum": uppercase_checksum,
                "definition": equipment_model_definition("Demo", "Amplifier Mk2", None),
                "actor": "equipment.author",
                "reason": "attempt uppercase checksum",
                "operation_id": "op-amp-uppercase-checksum"
            })
            .to_string(),
            &config,
        );
        assert_eq!(
            uppercase_checksum_edit.status, 400,
            "{}",
            uppercase_checksum_edit.body
        );
        assert!(uppercase_checksum_edit.body.contains("invalid_checksum"));

        let mut edited_definition = equipment_model_definition("Demo", "Amplifier Mk2", None);
        edited_definition["metadata"]["edited"] = json!(true);
        let edited = handle_api_request(
            "PUT",
            "/api/v1/equipment-models/EQM-DEMO-AMP/revisions/EQM-DEMO-AMP-rev-0001/definition",
            &json!({
                "expected_definition_checksum": initial_checksum,
                "definition": edited_definition,
                "actor": "equipment.author",
                "reason": "edit draft",
                "operation_id": "op-amp-edit"
            })
            .to_string(),
            &config,
        );
        assert_eq!(edited.status, 200, "{}", edited.body);
        let edited_json: Value = serde_json::from_str(&edited.body).unwrap();
        let updated_checksum = edited_json["revision"]["definition_checksum"]
            .as_str()
            .unwrap();

        let stale = handle_api_request(
            "PUT",
            "/api/v1/equipment-models/EQM-DEMO-AMP/revisions/EQM-DEMO-AMP-rev-0001/definition",
            &json!({
                "expected_definition_checksum": initial_checksum,
                "definition": equipment_model_definition("Demo", "Amplifier Mk3", None),
                "actor": "equipment.author",
                "reason": "stale edit",
                "operation_id": "op-amp-stale"
            })
            .to_string(),
            &config,
        );
        assert_eq!(stale.status, 409, "{}", stale.body);
        assert!(stale
            .body
            .contains("equipment_definition_checksum_mismatch"));

        assert_eq!(
            handle_api_request(
                "POST",
                "/api/v1/equipment-models/EQM-DEMO-AMP/revisions/EQM-DEMO-AMP-rev-0001/transitions/submit-for-review",
                &transition_body("equipment.author", "submit", "op-amp-submit"),
                &config
            )
            .status,
            200
        );
        let immutable = handle_api_request(
            "PUT",
            "/api/v1/equipment-models/EQM-DEMO-AMP/revisions/EQM-DEMO-AMP-rev-0001/definition",
            &json!({
                "expected_definition_checksum": updated_checksum,
                "definition": equipment_model_definition("Demo", "Amplifier Mk4", None),
                "actor": "equipment.author",
                "reason": "edit submitted",
                "operation_id": "op-amp-immutable"
            })
            .to_string(),
            &config,
        );
        assert_eq!(immutable.status, 409, "{}", immutable.body);
        assert!(immutable.body.contains("equipment_revision_immutable"));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_runs_project_vertical_slice_through_routes() {
        let storage_root = temporary_storage_root("agent-api-slice");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
    fn local_api_registers_attached_document_with_audit_and_outbox() {
        let storage_root = temporary_storage_root("agent-api-documents");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
                r#"{
                    "code": "CEM-DOC-001",
                    "customer_name": "Document Customer",
                    "execution_mode": "accredited",
                    "actor": "quality.lead",
                    "reason": "contract accepted",
                    "operation_id": "op-doc-project"
                }"#,
                &config,
            )
            .status,
            200
        );

        let register_body = &format!(
            r#"{{
                "document_id": "DOC-CEM-DOC-001-REQ-A",
                "classification": "client_document",
                "title": "Customer EMC requirements",
                "owner_domain": "locus_lab_management",
                "owner_entity_type": "project",
                "owner_entity_id": "CEM-DOC-001",
                "storage_backend": "object_store",
                "storage_uri": "objects/projects/CEM-DOC-001/requirements-A.pdf",
                "original_filename": "requirements.pdf",
                "mime_type": "application/pdf",
                "size_bytes": 12345,
                "sha256": "{}",
                "revision": "A",
                "applicability": "applicable",
                "confidentiality": "customer_visible",
                "actor": "project.manager",
                "reason": "customer requirement received",
                "operation_id": "op-doc-register"
            }}"#,
            "e".repeat(64)
        );
        let registered = handle_api_request("POST", "/api/v1/documents", register_body, &config);
        assert_eq!(registered.status, 200);
        assert!(registered
            .body
            .contains("\"document_id\":\"DOC-CEM-DOC-001-REQ-A\""));
        assert!(registered
            .body
            .contains("\"owner_domain\":\"locus_lab_management\""));

        let detail = handle_api_request(
            "GET",
            "/api/v1/documents/DOC-CEM-DOC-001-REQ-A",
            "",
            &config,
        );
        let filtered = handle_api_request(
            "GET",
            "/api/v1/documents?owner_domain=locus_lab_management&owner_entity_type=project&owner_entity_id=CEM-DOC-001",
            "",
            &config,
        );
        let audit = handle_api_request(
            "GET",
            "/api/v1/documents/DOC-CEM-DOC-001-REQ-A/audit-events",
            "",
            &config,
        );
        let outbox = handle_api_request("GET", "/api/v1/sync/outbox", "", &config);

        assert_eq!(detail.status, 200);
        assert_eq!(filtered.status, 200);
        assert_eq!(audit.status, 200);
        assert_eq!(outbox.status, 200);
        assert!(detail.body.contains("\"storage_backend\":\"object_store\""));
        assert_eq!(filtered.body.matches("\"document_id\"").count(), 1);
        assert!(audit
            .body
            .contains("\"action\":\"attached_document_registered\""));
        assert!(outbox
            .body
            .contains("\"entity_type\":\"attached_document\""));
        assert!(outbox
            .body
            .contains("\"operation_kind\":\"attached_document_registered\""));

        let replay = handle_api_request("POST", "/api/v1/documents", register_body, &config);
        assert_eq!(replay.status, 200);
        assert!(replay.body.contains("\"replayed\":true"));

        let conflict = handle_api_request(
            "POST",
            "/api/v1/documents",
            &format!(
                r#"{{
                    "document_id": "DOC-CEM-DOC-001-REQ-A",
                    "classification": "client_document",
                    "title": "Changed title",
                    "owner_domain": "locus_lab_management",
                    "owner_entity_type": "project",
                    "owner_entity_id": "CEM-DOC-001",
                    "storage_uri": "objects/projects/CEM-DOC-001/requirements-A.pdf",
                    "original_filename": "requirements.pdf",
                    "mime_type": "application/pdf",
                    "size_bytes": 12345,
                    "sha256": "{}",
                    "actor": "project.manager",
                    "reason": "conflicting replay",
                    "operation_id": "op-doc-register"
                }}"#,
                "e".repeat(64)
            ),
            &config,
        );
        assert_eq!(conflict.status, 409);
        assert!(conflict.body.contains("operation_replay_mismatch"));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_creates_test_template_with_audit_and_outbox() {
        let storage_root = temporary_storage_root("agent-api-test-template");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
            max_requests: None,
        };

        assert_eq!(
            handle_api_request("POST", "/api/v1/storage/initialize", "", &config).status,
            200
        );

        let template_body = create_template_body(
            "TT-INRUSH-001",
            "op-create-test-template",
            &template_definition(100_000, None),
        );
        let created = handle_api_request("POST", "/api/v1/test-templates", &template_body, &config);
        assert_eq!(created.status, 200);
        assert!(created
            .body
            .contains("\"revision_id\":\"TT-INRUSH-001-rev-0001\""));
        assert!(created.body.contains("\"status\":\"draft\""));
        let created_json: Value = serde_json::from_str(&created.body).unwrap();
        let revision_id = created_json["revision"]["revision_id"].as_str().unwrap();
        let initial_checksum = created_json["revision"]["definition_checksum"]
            .as_str()
            .unwrap();

        let uppercase_checksum = uppercase_checksum_payload(initial_checksum);
        let uppercase_checksum_body = format!(
            r#"{{
                "expected_definition_checksum": "{uppercase_checksum}",
                "definition": {},
                "actor": "method.author",
                "reason": "attempt uppercase checksum",
                "operation_id": "op-edit-test-template-uppercase-checksum"
            }}"#,
            template_definition(150_000, None)
        );
        let uppercase_checksum_edit = handle_api_request(
            "PUT",
            "/api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0001/definition",
            &uppercase_checksum_body,
            &config,
        );
        assert_eq!(uppercase_checksum_edit.status, 400);
        assert!(uppercase_checksum_edit
            .body
            .contains("invalid_test_template"));

        let detail = handle_api_request("GET", "/api/v1/test-templates/TT-INRUSH-001", "", &config);
        let filtered = handle_api_request(
            "GET",
            "/api/v1/test-templates?category_code=emission_transient_time_domain",
            "",
            &config,
        );
        let revision_detail = handle_api_request(
            "GET",
            "/api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0001",
            "",
            &config,
        );
        let audit = handle_api_request(
            "GET",
            "/api/v1/test-templates/TT-INRUSH-001/audit-events",
            "",
            &config,
        );
        let outbox = handle_api_request("GET", "/api/v1/sync/outbox", "", &config);

        assert_eq!(detail.status, 200);
        assert_eq!(filtered.status, 200);
        assert_eq!(revision_detail.status, 200);
        assert_eq!(audit.status, 200);
        assert_eq!(outbox.status, 200);
        assert!(detail.body.contains("\"instrumentation_chain\""));
        assert!(filtered.body.contains("\"template_id\":\"TT-INRUSH-001\""));
        assert!(audit.body.contains("\"action\":\"test_template_created\""));
        assert!(outbox.body.contains("\"domain\":\"test_definitions\""));
        assert!(outbox
            .body
            .contains("\"entity_type\":\"test_template_revision\""));
        assert!(outbox
            .body
            .contains("\"operation_kind\":\"test_template_created\""));

        let edited_body = format!(
            r#"{{
                "expected_definition_checksum": "{initial_checksum}",
                "definition": {},
                "actor": "method.author",
                "reason": "increase sample rate before review",
                "operation_id": "op-edit-test-template"
            }}"#,
            template_definition(200_000, None)
        );
        let edited = handle_api_request(
            "PUT",
            "/api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0001/definition",
            &edited_body,
            &config,
        );
        assert_eq!(edited.status, 200);
        let edited_json: Value = serde_json::from_str(&edited.body).unwrap();
        let edited_checksum = edited_json["revision"]["definition_checksum"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_ne!(edited_checksum, initial_checksum);

        let stale_body = format!(
            r#"{{
                "expected_definition_checksum": "{initial_checksum}",
                "definition": {},
                "actor": "method.author",
                "reason": "stale concurrent edit",
                "operation_id": "op-edit-test-template-stale"
            }}"#,
            template_definition(300_000, None)
        );
        let stale_edit = handle_api_request(
            "PUT",
            "/api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0001/definition",
            &stale_body,
            &config,
        );
        assert_eq!(stale_edit.status, 409);
        assert!(stale_edit
            .body
            .contains("test_template_definition_checksum_mismatch"));

        let early_approval = handle_api_request(
            "POST",
            "/api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0001/transitions/approve",
            r#"{
                "actor": "quality.lead",
                "reason": "approval attempted before review",
                "operation_id": "op-approve-too-early"
            }"#,
            &config,
        );
        assert_eq!(early_approval.status, 409);
        assert!(early_approval
            .body
            .contains("test_template_revision_transition_not_allowed"));

        let submitted = handle_api_request(
            "POST",
            "/api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0001/transitions/submit-for-review",
            r#"{
                "actor": "method.author",
                "reason": "definition ready for technical review",
                "operation_id": "op-submit-test-template"
            }"#,
            &config,
        );
        assert_eq!(submitted.status, 200);
        assert!(submitted.body.contains("\"status\":\"under_review\""));
        assert!(submitted
            .body
            .contains("\"operation\":\"test_template_submitted_for_review\""));

        let immutable_edit = handle_api_request(
            "PUT",
            "/api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0001/definition",
            &format!(
                r#"{{
                    "expected_definition_checksum": "{edited_checksum}",
                    "definition": {},
                    "actor": "method.author",
                    "reason": "attempt edit under review",
                    "operation_id": "op-edit-under-review"
                }}"#,
                template_definition(250_000, None)
            ),
            &config,
        );
        assert_eq!(immutable_edit.status, 409);
        assert!(immutable_edit
            .body
            .contains("test_template_revision_immutable"));

        let submit_replay = handle_api_request(
            "POST",
            "/api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0001/transitions/submit-for-review",
            r#"{
                "actor": "method.author",
                "reason": "definition ready for technical review",
                "operation_id": "op-submit-test-template"
            }"#,
            &config,
        );
        assert_eq!(submit_replay.status, 200);
        assert!(submit_replay.body.contains("\"replayed\":true"));

        let approved = handle_api_request(
            "POST",
            "/api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0001/transitions/approve",
            r#"{
                "actor": "technical.reviewer",
                "reason": "technical review accepted",
                "operation_id": "op-approve-test-template"
            }"#,
            &config,
        );
        assert_eq!(approved.status, 200);
        assert!(approved.body.contains("\"status\":\"approved\""));
        assert!(approved
            .body
            .contains("\"operation\":\"test_template_approved\""));

        let approved_edit = handle_api_request(
            "PUT",
            "/api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0001/definition",
            &format!(
                r#"{{
                    "expected_definition_checksum": "{edited_checksum}",
                    "definition": {},
                    "actor": "method.author",
                    "reason": "attempt edit approved",
                    "operation_id": "op-edit-approved"
                }}"#,
                template_definition(300_000, None)
            ),
            &config,
        );
        assert_eq!(approved_edit.status, 409);
        assert!(approved_edit
            .body
            .contains("test_template_revision_immutable"));

        let second_revision = handle_api_request(
            "POST",
            "/api/v1/test-templates/TT-INRUSH-001/revisions",
            r#"{
                "source_revision_id": "TT-INRUSH-001-rev-0001",
                "actor": "method.author",
                "reason": "prepare next controlled revision",
                "operation_id": "op-create-test-template-rev2"
            }"#,
            &config,
        );
        assert_eq!(second_revision.status, 200);
        assert!(second_revision
            .body
            .contains("\"revision_id\":\"TT-INRUSH-001-rev-0002\""));
        assert!(second_revision.body.contains("\"status\":\"draft\""));

        let aggregate_with_draft =
            handle_api_request("GET", "/api/v1/test-templates/TT-INRUSH-001", "", &config);
        assert_eq!(aggregate_with_draft.status, 200);
        assert!(aggregate_with_draft
            .body
            .contains("\"current_approved_revision\":{\"revision_id\":\"TT-INRUSH-001-rev-0001\""));
        assert!(aggregate_with_draft
            .body
            .contains("\"active_draft_revision\":{\"revision_id\":\"TT-INRUSH-001-rev-0002\""));

        let duplicate_draft = handle_api_request(
            "POST",
            "/api/v1/test-templates/TT-INRUSH-001/revisions",
            r#"{
                "source_revision_id": "TT-INRUSH-001-rev-0001",
                "actor": "method.author",
                "reason": "attempt second active draft",
                "operation_id": "op-create-test-template-rev3-blocked"
            }"#,
            &config,
        );
        assert_eq!(duplicate_draft.status, 409);
        assert!(duplicate_draft
            .body
            .contains("test_template_active_draft_exists"));

        let submitted_second = handle_api_request(
            "POST",
            "/api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0002/transitions/submit-for-review",
            r#"{
                "actor": "method.author",
                "reason": "revision two ready for review",
                "operation_id": "op-submit-test-template-rev2"
            }"#,
            &config,
        );
        assert_eq!(submitted_second.status, 200);
        assert!(submitted_second
            .body
            .contains("\"status\":\"under_review\""));

        let approved_second = handle_api_request(
            "POST",
            "/api/v1/test-templates/TT-INRUSH-001/revisions/TT-INRUSH-001-rev-0002/transitions/approve",
            r#"{
                "actor": "technical.reviewer",
                "reason": "revision two accepted",
                "operation_id": "op-approve-test-template-rev2"
            }"#,
            &config,
        );
        assert_eq!(approved_second.status, 200);
        assert!(approved_second.body.contains("\"status\":\"approved\""));

        let revisions = handle_api_request(
            "GET",
            "/api/v1/test-templates/TT-INRUSH-001/revisions",
            "",
            &config,
        );
        assert_eq!(revisions.status, 200);
        assert!(revisions
            .body
            .contains("\"revision_id\":\"TT-INRUSH-001-rev-0001\""));
        assert!(revisions
            .body
            .contains("\"revision_id\":\"TT-INRUSH-001-rev-0002\""));
        let revisions_json: Value = serde_json::from_str(&revisions.body).unwrap();
        let revision_statuses: std::collections::HashMap<_, _> = revisions_json["revisions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|revision| {
                (
                    revision["revision_id"].as_str().unwrap().to_owned(),
                    revision["status"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        assert_eq!(revision_statuses["TT-INRUSH-001-rev-0001"], "superseded");
        assert_eq!(revision_statuses["TT-INRUSH-001-rev-0002"], "approved");

        let approved_list = handle_api_request(
            "GET",
            "/api/v1/test-templates?category_code=emission_transient_time_domain",
            "",
            &config,
        );
        let lifecycle_audit = handle_api_request(
            "GET",
            "/api/v1/test-templates/TT-INRUSH-001/audit-events",
            "",
            &config,
        );
        let lifecycle_outbox = handle_api_request("GET", "/api/v1/sync/outbox", "", &config);
        assert_eq!(approved_list.status, 200);
        assert_eq!(lifecycle_audit.status, 200);
        assert_eq!(lifecycle_outbox.status, 200);
        assert!(approved_list
            .body
            .contains("\"current_approved_revision_id\":\"TT-INRUSH-001-rev-0002\""));
        assert!(lifecycle_audit
            .body
            .contains("\"action\":\"test_template_submitted_for_review\""));
        assert!(lifecycle_audit
            .body
            .contains("\"action\":\"test_template_approved\""));
        assert!(lifecycle_audit
            .body
            .contains("\"action\":\"test_template_revision_superseded\""));
        assert!(lifecycle_outbox
            .body
            .contains("\"operation_kind\":\"test_template_submitted_for_review\""));
        assert!(lifecycle_outbox
            .body
            .contains("\"operation_kind\":\"test_template_approved\""));
        assert!(lifecycle_outbox
            .body
            .contains("\"operation_kind\":\"test_template_revision_superseded\""));
        assert!(lifecycle_outbox
            .body
            .contains("\"operation_kind\":\"test_template_revision_created\""));

        let replay = handle_api_request("POST", "/api/v1/test-templates", &template_body, &config);
        assert_eq!(replay.status, 200);
        assert!(replay.body.contains("\"replayed\":true"));

        let conflict = handle_api_request(
            "POST",
            "/api/v1/test-templates",
            &create_template_body(
                "TT-INRUSH-001",
                "op-create-test-template",
                &template_definition(400_000, None),
            ),
            &config,
        );
        assert_eq!(conflict.status, 409);
        assert!(conflict.body.contains("operation_replay_mismatch"));

        assert_eq!(revision_id, "TT-INRUSH-001-rev-0001");

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_clones_approved_test_template_with_audit_and_outbox() {
        let storage_root = temporary_storage_root("agent-api-test-template-clone");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
            max_requests: None,
        };

        assert_eq!(
            handle_api_request("POST", "/api/v1/storage/initialize", "", &config).status,
            200
        );
        let created = handle_api_request(
            "POST",
            "/api/v1/test-templates",
            &create_template_body(
                "TT-CLONE-SOURCE",
                "op-clone-source-create",
                &template_definition(100_000, None),
            ),
            &config,
        );
        assert_eq!(created.status, 200);
        assert_eq!(
            handle_api_request(
                "POST",
                "/api/v1/test-templates/TT-CLONE-SOURCE/revisions/TT-CLONE-SOURCE-rev-0001/transitions/submit-for-review",
                r#"{
                    "actor": "method.author",
                    "reason": "source ready",
                    "operation_id": "op-clone-source-submit"
                }"#,
                &config,
            )
            .status,
            200
        );
        assert_eq!(
            handle_api_request(
                "POST",
                "/api/v1/test-templates/TT-CLONE-SOURCE/revisions/TT-CLONE-SOURCE-rev-0001/transitions/approve",
                r#"{
                    "actor": "quality.reviewer",
                    "reason": "source accepted",
                    "operation_id": "op-clone-source-approve"
                }"#,
                &config,
            )
            .status,
            200
        );

        let cloned = handle_api_request(
            "POST",
            "/api/v1/test-templates/TT-CLONE-SOURCE/clone",
            r#"{
                "source_revision_id": "TT-CLONE-SOURCE-rev-0001",
                "new_template_id": "TT-CLONE-COPY",
                "title": "Copied inrush template",
                "actor": "method.author",
                "reason": "create variant from approved source",
                "operation_id": "op-template-clone-copy"
            }"#,
            &config,
        );
        assert_eq!(cloned.status, 200);
        assert!(cloned
            .body
            .contains("\"operation\":\"test_template_cloned\""));
        assert!(cloned.body.contains("\"template_id\":\"TT-CLONE-COPY\""));
        assert!(cloned.body.contains("\"status\":\"draft\""));
        let cloned_json: Value = serde_json::from_str(&cloned.body).unwrap();
        assert_eq!(
            cloned_json["revision"]["definition"]["title"],
            "Copied inrush template"
        );

        let source_revisions = handle_api_request(
            "GET",
            "/api/v1/test-templates/TT-CLONE-SOURCE/revisions",
            "",
            &config,
        );
        let clone_audit = handle_api_request(
            "GET",
            "/api/v1/test-templates/TT-CLONE-COPY/audit-events",
            "",
            &config,
        );
        let outbox = handle_api_request("GET", "/api/v1/sync/outbox", "", &config);

        assert_eq!(source_revisions.status, 200);
        assert!(source_revisions.body.contains("\"status\":\"approved\""));
        assert_eq!(clone_audit.status, 200);
        assert!(clone_audit
            .body
            .contains("\"action\":\"test_template_cloned\""));
        assert!(clone_audit
            .body
            .contains("\"old_revision_id\":\"TT-CLONE-SOURCE-rev-0001\""));
        assert!(outbox
            .body
            .contains("\"operation_kind\":\"test_template_cloned\""));

        let replay = handle_api_request(
            "POST",
            "/api/v1/test-templates/TT-CLONE-SOURCE/clone",
            r#"{
                "source_revision_id": "TT-CLONE-SOURCE-rev-0001",
                "new_template_id": "TT-CLONE-COPY",
                "title": "Copied inrush template",
                "actor": "method.author",
                "reason": "create variant from approved source",
                "operation_id": "op-template-clone-copy"
            }"#,
            &config,
        );
        assert_eq!(replay.status, 200);
        assert!(replay.body.contains("\"replayed\":true"));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_requires_approved_method_revision_for_test_template() {
        let storage_root = temporary_storage_root("agent-api-test-template-method-status");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
            max_requests: None,
        };

        assert_eq!(
            handle_api_request("POST", "/api/v1/storage/initialize", "", &config).status,
            200
        );
        let connection =
            rusqlite::Connection::open(storage_root.join("test_definitions.sqlite")).unwrap();
        connection
            .execute_batch(
                r#"
                INSERT INTO test_methods (
                    code, standard_code, name, family, measurement_axis,
                    controlled, category_code, created_at, updated_at
                )
                VALUES (
                    'TD-INRUSH', NULL, 'Inrush method', 'inrush',
                    'time_series', 1, 'emission_transient_time_domain',
                    '2026-07-01T00:00:00Z', '2026-07-01T00:00:00Z'
                );
                INSERT INTO test_method_revisions (
                    method_code, revision, status, parameters_json,
                    acceptance_criteria_json, processing_graph_json
                )
                VALUES
                    ('TD-INRUSH', 'DRAFT', 'draft', '{}', '{}', '{}'),
                    ('TD-INRUSH', 'A', 'approved', '{}', '{}', '{}');
                "#,
            )
            .unwrap();
        drop(connection);

        let draft_method_template = create_template_body(
            "TT-DRAFT-METHOD",
            "op-draft-method-template",
            &template_definition(100_000, Some(("TD-INRUSH", "DRAFT"))),
        );
        let rejected = handle_api_request(
            "POST",
            "/api/v1/test-templates",
            &draft_method_template,
            &config,
        );
        assert_eq!(rejected.status, 404);
        assert!(rejected
            .body
            .contains("test_template_method_revision_not_found"));

        let approved_method_template = create_template_body(
            "TT-APPROVED-METHOD",
            "op-approved-method-template",
            &template_definition(100_000, Some(("TD-INRUSH", "A"))),
        );
        let created = handle_api_request(
            "POST",
            "/api/v1/test-templates",
            &approved_method_template,
            &config,
        );
        assert_eq!(created.status, 200);
        assert!(created
            .body
            .contains("\"template_id\":\"TT-APPROVED-METHOD\""));
        assert!(created.body.contains("\"method_revision\":\"A\""));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_requires_approved_test_template_for_simulated_execution() {
        let storage_root = temporary_storage_root("agent-api-execution-approved-template");
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: storage_root.clone(),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
                r#"{
                    "code": "CEM-TPL-001",
                    "customer_name": "Template-linked EMC Customer",
                    "execution_mode": "accredited",
                    "actor": "quality.lead",
                    "reason": "contract accepted",
                    "operation_id": "op-template-exec-project"
                }"#,
                &config,
            )
            .status,
            200
        );
        assert_eq!(
            handle_api_request(
                "POST",
                "/api/v1/metrology/instruments",
                r#"{
                    "asset_id": "SA-TPL-001",
                    "family": "SpectrumAnalyzer",
                    "category_code": "spectrum_analyzer",
                    "manufacturer": "Rohde Schwarz",
                    "model": "FSW",
                    "serial_number": "TPL-001",
                    "calibration_requirement": "required",
                    "calibration_period_months": 12,
                    "capabilities": {"frequency_max_hz": 30000000},
                    "actor": "metrology.admin",
                    "reason": "register template-linked asset",
                    "operation_id": "op-template-exec-register"
                }"#,
                &config,
            )
            .status,
            200
        );
        assert_eq!(
            handle_api_request(
                "POST",
                "/api/v1/test-templates",
                &create_template_body(
                    "TT-INRUSH-EXEC",
                    "op-template-exec-create",
                    &template_definition(100_000, None),
                ),
                &config,
            )
            .status,
            200
        );

        let draft_execution = handle_api_request(
            "POST",
            "/api/v1/test-executions/simulated-emc",
            r#"{
                "attempt_id": "RUN-TPL-001",
                "project_code": "CEM-TPL-001",
                "test_method_reference": "TT-INRUSH-EXEC",
                "execution_mode": "accredited",
                "required_asset_ids": ["SA-TPL-001"],
                "operator": "operator.one",
                "checked_on": "2026-07-01",
                "reason": "operator launch against draft template",
                "operation_id": "op-template-exec-draft"
            }"#,
            &config,
        );
        assert_eq!(draft_execution.status, 409);
        assert!(draft_execution
            .body
            .contains("test_execution_template_not_approved"));

        assert_eq!(
            handle_api_request(
                "POST",
                "/api/v1/test-templates/TT-INRUSH-EXEC/revisions/TT-INRUSH-EXEC-rev-0001/transitions/submit-for-review",
                r#"{
                    "actor": "method.author",
                    "reason": "ready for technical review",
                    "operation_id": "op-template-exec-submit"
                }"#,
                &config,
            )
            .status,
            200
        );
        assert_eq!(
            handle_api_request(
                "POST",
                "/api/v1/test-templates/TT-INRUSH-EXEC/revisions/TT-INRUSH-EXEC-rev-0001/transitions/approve",
                r#"{
                    "actor": "technical.reviewer",
                    "reason": "approved for simulated execution",
                    "operation_id": "op-template-exec-approve"
                }"#,
                &config,
            )
            .status,
            200
        );

        let approved_execution = handle_api_request(
            "POST",
            "/api/v1/test-executions/simulated-emc",
            r#"{
                "attempt_id": "RUN-TPL-002",
                "project_code": "CEM-TPL-001",
                "test_method_reference": "TT-INRUSH-EXEC",
                "execution_mode": "accredited",
                "required_asset_ids": ["SA-TPL-001"],
                "operator": "operator.one",
                "checked_on": "2026-07-01",
                "reason": "operator launch against approved template",
                "operation_id": "op-template-exec-approved"
            }"#,
            &config,
        );
        assert_eq!(approved_execution.status, 200);
        assert!(approved_execution.body.contains("\"status\":\"refused\""));
        assert!(approved_execution
            .body
            .contains("\"test_method_reference\":\"TT-INRUSH-EXEC\""));
        let approved_json: Value = serde_json::from_str(&approved_execution.body).unwrap();
        assert_eq!(
            approved_json["execution"]["test_template_revision"]["template_id"]
                .as_str()
                .unwrap(),
            "TT-INRUSH-EXEC"
        );
        assert_eq!(
            approved_json["execution"]["test_template_revision"]["revision_id"]
                .as_str()
                .unwrap(),
            "TT-INRUSH-EXEC-rev-0001"
        );
        assert!(
            approved_json["execution"]["test_template_revision"]["definition_checksum"]
                .as_str()
                .unwrap()
                .starts_with("sha256:")
        );
        assert!(approved_execution
            .body
            .contains("\"code\":\"equipment_readiness_blocked\""));

        remove_temporary_storage_root(&storage_root);
    }

    #[test]
    fn local_api_rejects_invalid_json_payloads() {
        let config = ApiServerConfig {
            bind: "127.0.0.1:0".to_owned(),
            storage_root: PathBuf::from("unused"),
            migrations_root: repo_root().join("storage/sqlite"),
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
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

    #[test]
    fn real_http_server_persists_test_template_revision_workflow_across_restart() {
        let storage_root = temporary_storage_root("agent-api-real-test-template-revisions");
        let migrations_root = repo_root().join("storage/sqlite");
        let first_port = free_loopback_port();
        let first_address = format!("127.0.0.1:{first_port}");
        let first_server = spawn_server(ApiServerConfig {
            bind: first_address.clone(),
            storage_root: storage_root.clone(),
            migrations_root: migrations_root.clone(),
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
            max_requests: Some(12),
        });

        assert_eq!(wait_for_http(&first_address, "/api/v1/health").0, 200);
        assert_eq!(
            http_request("POST", &first_address, "/api/v1/storage/initialize", "").0,
            200
        );

        let create_body = create_template_body(
            "TT-HTTP-REV",
            "op-http-template-create",
            &template_definition(100_000, None),
        );
        let created = http_request(
            "POST",
            &first_address,
            "/api/v1/test-templates",
            &create_body,
        );
        assert_eq!(created.0, 200);
        let created_json: Value = serde_json::from_str(&created.1).unwrap();
        let checksum = created_json["revision"]["definition_checksum"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(
            created_json["revision"]["revision_id"].as_str().unwrap(),
            "TT-HTTP-REV-rev-0001"
        );

        let edited = http_request(
            "PUT",
            &first_address,
            "/api/v1/test-templates/TT-HTTP-REV/revisions/TT-HTTP-REV-rev-0001/definition",
            &format!(
                r#"{{
                    "expected_definition_checksum": "{checksum}",
                    "definition": {},
                    "actor": "method.author",
                    "reason": "HTTP E2E draft replacement",
                    "operation_id": "op-http-template-edit"
                }}"#,
                template_definition(200_000, None)
            ),
        );
        assert_eq!(edited.0, 200);
        assert!(edited
            .1
            .contains("\"operation\":\"test_template_definition_replaced\""));

        assert_eq!(
            http_request(
                "POST",
                &first_address,
                "/api/v1/test-templates/TT-HTTP-REV/revisions/TT-HTTP-REV-rev-0001/transitions/submit-for-review",
                r#"{
                    "actor": "method.author",
                    "reason": "HTTP E2E submit",
                    "operation_id": "op-http-template-submit"
                }"#,
            )
            .0,
            200
        );
        assert_eq!(
            http_request(
                "POST",
                &first_address,
                "/api/v1/test-templates/TT-HTTP-REV/revisions/TT-HTTP-REV-rev-0001/transitions/approve",
                r#"{
                    "actor": "quality.reviewer",
                    "reason": "HTTP E2E approve",
                    "operation_id": "op-http-template-approve"
                }"#,
            )
            .0,
            200
        );
        let second_revision = http_request(
            "POST",
            &first_address,
            "/api/v1/test-templates/TT-HTTP-REV/revisions",
            r#"{
                "source_revision_id": "TT-HTTP-REV-rev-0001",
                "actor": "method.author",
                "reason": "HTTP E2E next draft",
                "operation_id": "op-http-template-rev2"
            }"#,
        );
        assert_eq!(second_revision.0, 200);
        assert!(second_revision
            .1
            .contains("\"revision_id\":\"TT-HTTP-REV-rev-0002\""));
        assert_eq!(
            http_request(
                "POST",
                &first_address,
                "/api/v1/test-templates/TT-HTTP-REV/revisions/TT-HTTP-REV-rev-0002/transitions/submit-for-review",
                r#"{
                    "actor": "method.author",
                    "reason": "HTTP E2E submit revision two",
                    "operation_id": "op-http-template-submit-rev2"
                }"#,
            )
            .0,
            200
        );
        assert_eq!(
            http_request(
                "POST",
                &first_address,
                "/api/v1/test-templates/TT-HTTP-REV/revisions/TT-HTTP-REV-rev-0002/transitions/approve",
                r#"{
                    "actor": "quality.reviewer",
                    "reason": "HTTP E2E approve revision two",
                    "operation_id": "op-http-template-approve-rev2"
                }"#,
            )
            .0,
            200
        );

        let revisions = http_request(
            "GET",
            &first_address,
            "/api/v1/test-templates/TT-HTTP-REV/revisions",
            "",
        );
        let audit = http_request(
            "GET",
            &first_address,
            "/api/v1/test-templates/TT-HTTP-REV/audit-events",
            "",
        );
        let outbox = http_request("GET", &first_address, "/api/v1/sync/outbox", "");
        assert_eq!(revisions.0, 200);
        assert_eq!(audit.0, 200);
        assert_eq!(outbox.0, 200);
        assert!(revisions
            .1
            .contains("\"revision_id\":\"TT-HTTP-REV-rev-0001\""));
        assert!(revisions
            .1
            .contains("\"revision_id\":\"TT-HTTP-REV-rev-0002\""));
        assert!(revisions.1.contains("\"status\":\"superseded\""));
        assert!(audit.1.contains("\"action\":\"test_template_approved\""));
        assert!(audit
            .1
            .contains("\"action\":\"test_template_revision_superseded\""));
        assert!(outbox
            .1
            .contains("\"operation_kind\":\"test_template_revision_created\""));
        assert!(outbox
            .1
            .contains("\"operation_kind\":\"test_template_revision_superseded\""));
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
            lab_console_dist: repo_root().join("apps/lab-console/dist"),
            max_requests: Some(4),
        });
        assert_eq!(wait_for_http(&second_address, "/api/v1/health").0, 200);
        let reloaded_template = http_request(
            "GET",
            &second_address,
            "/api/v1/test-templates/TT-HTTP-REV",
            "",
        );
        let reloaded_revisions = http_request(
            "GET",
            &second_address,
            "/api/v1/test-templates/TT-HTTP-REV/revisions",
            "",
        );
        let reloaded_outbox = http_request("GET", &second_address, "/api/v1/sync/outbox", "");
        assert_eq!(reloaded_template.0, 200);
        assert_eq!(reloaded_revisions.0, 200);
        assert_eq!(reloaded_outbox.0, 200);
        assert!(reloaded_template
            .1
            .contains("\"current_approved_revision_id\":\"TT-HTTP-REV-rev-0002\""));
        assert!(reloaded_revisions
            .1
            .contains("\"revision_id\":\"TT-HTTP-REV-rev-0002\""));
        assert!(reloaded_revisions.1.contains("\"status\":\"superseded\""));
        assert!(reloaded_outbox
            .1
            .contains("\"entity_type\":\"test_template_revision\""));
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

    fn transition_body(actor: &str, reason: &str, operation_id: &str) -> String {
        json!({
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id
        })
        .to_string()
    }

    fn equipment_model_definition(
        manufacturer: &str,
        model_name: &str,
        variant: Option<&str>,
    ) -> Value {
        json!({
            "definition_schema_version": "emc-locus.equipment-model-definition.v2",
            "manufacturer": manufacturer,
            "model_name": model_name,
            "variant": variant,
            "equipment_class": "controllable_instrument",
            "functional_role": "measurement_instrument",
            "category_code": "power_meter",
            "signal_domains": ["rf", "ethernet"],
            "technology_tags": ["rf_50_ohm", "ethernet", "raw_tcp", "scpi"],
            "specifications": [
                {
                    "specification_id": "frequency_range",
                    "label": "Frequency range",
                    "quantity": "frequency",
                    "unit": "Hz",
                    "minimum": 9000.0,
                    "maximum": 1000000000.0
                },
                {
                    "specification_id": "power_range",
                    "label": "Power range",
                    "quantity": "power",
                    "unit": "dBm",
                    "minimum": -70.0,
                    "maximum": 30.0
                }
            ],
            "signal_ports": [
                {
                    "port_id": "rf_input",
                    "label": "RF Input",
                    "directionality": "input",
                    "flow_role": "measurement_port",
                    "signal_domain": "rf",
                    "connector_type": "N",
                    "quantity": "power",
                    "unit": "dBm",
                    "impedance": 50.0,
                    "frequency_min": 9000.0,
                    "frequency_max": 1000000000.0,
                    "power_max": 30.0
                }
            ],
            "communication_interfaces": [
                {
                    "interface_id": "tcp",
                    "label": "SCPI TCP",
                    "transport_kind": "ethernet_tcp",
                    "access_provider_kind": "native_tcp",
                    "protocol_kind": "scpi",
                    "required": true,
                    "default_interface": true,
                    "configuration_schema": {
                        "host": {"type": "text"},
                        "port": {"type": "integer"}
                    },
                    "default_configuration": {
                        "host": "127.0.0.1",
                        "port": 5025
                    },
                    "framing": "lf",
                    "identification_strategy": {
                        "strategy_id": "idn",
                        "strategy_type": "scpi_idn",
                        "query": "*IDN?",
                        "response_regex": "^R&S,NRP6AN,"
                    }
                }
            ],
            "capabilities": [
                {
                    "capability_id": "measure_power",
                    "label": "Measure power",
                    "description": "Measure RF power on the selected input.",
                    "capability_kind": "measure_power",
                    "inputs": [],
                    "outputs": [
                        {
                            "name": "power_dbm",
                            "value_type": "number",
                            "quantity": "power",
                            "unit": "dBm",
                            "required": true
                        }
                    ],
                    "required_signal_ports": ["rf_input"],
                    "safety_class": "read_only"
                }
            ],
            "metadata": {
                "demo": true
            }
        })
    }

    fn driver_profile_definition(model_revision_id: &str, model_checksum: &str) -> Value {
        json!({
            "definition_schema_version": "emc-locus.driver-profile-definition.v1",
            "equipment_model_id": "EQM-NRP6AN-FWD",
            "supported_model_revision_id": model_revision_id,
            "supported_model_definition_checksum": model_checksum,
            "supported_firmware_ranges": ["*"],
            "communication_profiles": ["tcp"],
            "actions": [
                {
                    "action_id": "measure_powers",
                    "label": "Measure powers",
                    "description": "Query the simulated SCPI power reading.",
                    "implements_capability_id": "measure_power",
                    "inputs": [],
                    "outputs": [
                        {
                            "name": "power_dbm",
                            "value_type": "number",
                            "quantity": "power",
                            "unit": "dBm",
                            "required": true
                        }
                    ],
                    "safety_class": "read_only",
                    "default_timeout_ms": 1000,
                    "script": {
                        "steps": [
                            {
                                "step_id": "query-power",
                                "step_type": "io_query",
                                "interface_id": "tcp",
                                "payload_format": "text",
                                "payload": "MEAS:POW?",
                                "response_binding": "${result.power_dbm}",
                                "timeout_ms": 1000
                            },
                            {
                                "step_id": "return",
                                "step_type": "return"
                            }
                        ]
                    },
                    "safe_to_retry": true,
                    "idempotent": true
                }
            ],
            "metadata": {
                "demo": true
            }
        })
    }

    fn create_and_approve_measurement(
        config: &ApiServerConfig,
        collection: &str,
        entity_id: &str,
        definition: Value,
        operation_prefix: &str,
    ) -> String {
        let created = handle_api_request(
            "POST",
            &format!("/api/v1/{collection}"),
            &create_measurement_body(
                entity_id,
                definition,
                &format!("op-{operation_prefix}-create"),
            ),
            config,
        );
        assert_eq!(created.status, 200, "{}", created.body);
        let created_json: Value = serde_json::from_str(&created.body).unwrap();
        let revision_id = created_json["revision"]["revision_id"]
            .as_str()
            .unwrap()
            .to_owned();
        approve_measurement_revision(
            config,
            collection,
            entity_id,
            &revision_id,
            &format!("op-{operation_prefix}-submit"),
            &format!("op-{operation_prefix}-approve"),
        );
        revision_id
    }

    fn approve_measurement_revision(
        config: &ApiServerConfig,
        collection: &str,
        entity_id: &str,
        revision_id: &str,
        submit_operation_id: &str,
        approve_operation_id: &str,
    ) {
        let submit = handle_api_request(
            "POST",
            &format!(
                "/api/v1/{collection}/{entity_id}/revisions/{revision_id}/transitions/submit-for-review"
            ),
            &transition_body(
                "measurement.author",
                "submit measurement engineering definition",
                submit_operation_id,
            ),
            config,
        );
        assert_eq!(submit.status, 200, "{}", submit.body);
        let approve = handle_api_request(
            "POST",
            &format!(
                "/api/v1/{collection}/{entity_id}/revisions/{revision_id}/transitions/approve"
            ),
            &transition_body(
                "quality.approver",
                "approve measurement engineering definition",
                approve_operation_id,
            ),
            config,
        );
        assert_eq!(approve.status, 200, "{}", approve.body);
    }

    fn create_measurement_body(entity_id: &str, definition: Value, operation_id: &str) -> String {
        render_json(&json!({
            "entity_id": entity_id,
            "definition": definition,
            "actor": "measurement.author",
            "reason": "create measurement engineering definition",
            "operation_id": operation_id
        }))
    }

    fn replace_measurement_body(
        definition: Value,
        expected_definition_checksum: &str,
        operation_id: &str,
    ) -> String {
        render_json(&json!({
            "definition": definition,
            "expected_definition_checksum": expected_definition_checksum,
            "actor": "measurement.author",
            "reason": "replace measurement engineering draft",
            "operation_id": operation_id
        }))
    }

    fn uppercase_checksum_payload(checksum: &str) -> String {
        format!(
            "sha256:{}",
            checksum["sha256:".len()..].to_ascii_uppercase()
        )
    }

    fn scaling_definition(scaling_profile_id: &str, scale: f64) -> Value {
        json!({
            "definition_schema_version": "emc-locus.scaling-profile-definition.v1",
            "scaling_profile_id": scaling_profile_id,
            "label": "Demo Current Probe 10mV/A",
            "input_quantity": "voltage",
            "input_unit": "V",
            "output_quantity": "current",
            "output_unit": "A",
            "scaling_kind": "linear",
            "parameters": {
                "scale": scale,
                "offset": 0.0
            },
            "validity_domain": {
                "note": "10 mV/A current-probe sensitivity"
            },
            "metadata": {
                "demo": true
            }
        })
    }

    fn curve_definition(curve_id: &str, curve_type: &str) -> Value {
        json!({
            "definition_schema_version": "emc-locus.engineering-curve-definition.v1",
            "curve_id": curve_id,
            "curve_type": curve_type,
            "label": "Demo current probe transfer",
            "independent_axes": [
                {
                    "axis": "frequency",
                    "quantity": "frequency",
                    "unit": "Hz"
                }
            ],
            "dependent_values": [
                {
                    "value_id": "correction_db",
                    "quantity": "dimensionless",
                    "unit": "dB"
                }
            ],
            "points": [
                {
                    "axis_values": { "frequency": 10000000.0 },
                    "values": { "correction_db": 0.0 }
                },
                {
                    "axis_values": { "frequency": 100000000.0 },
                    "values": { "correction_db": 1.0 }
                },
                {
                    "axis_values": { "frequency": 1000000000.0 },
                    "values": { "correction_db": 3.0 }
                }
            ],
            "interpolation": "log_x_linear_y",
            "extrapolation_policy": "forbidden",
            "source_document_reference": "demo:current-probe-transfer",
            "source_checksum": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "metadata": { "demo": true }
        })
    }

    fn daq_definition(daq_channel_profile_id: &str) -> Value {
        json!({
            "definition_schema_version": "emc-locus.daq-channel-profile-definition.v1",
            "daq_channel_profile_id": daq_channel_profile_id,
            "label": "Demo DAQ AI +/-10V",
            "channel_kind": "analog_input",
            "signal_domain": "analog_voltage",
            "input_quantity": "voltage",
            "input_unit": "V",
            "supported_ranges": [
                {
                    "minimum": -10.0,
                    "maximum": 10.0,
                    "unit": "V"
                }
            ],
            "resolution_bits": 16,
            "max_sampling_rate": 1000000.0,
            "min_sampling_rate": 1.0,
            "coupling_modes": ["dc"],
            "input_modes": ["single_ended", "differential"],
            "anti_alias_filter": "optional",
            "synchronization": "shared_clock_ready",
            "triggering": "software_or_external",
            "metadata": { "demo": true }
        })
    }

    fn sensor_definition(
        sensor_definition_id: &str,
        scaling_profile_id: &str,
        curve_id: &str,
    ) -> Value {
        json!({
            "definition_schema_version": "emc-locus.sensor-definition.v1",
            "sensor_definition_id": sensor_definition_id,
            "manufacturer": "EMC Locus",
            "model_name": "Demo Current Probe 10mV/A",
            "sensor_family": "current_probe",
            "physical_input_quantity": "current",
            "engineering_output_quantity": "current",
            "engineering_output_unit": "A",
            "electrical_output_quantity": "voltage",
            "electrical_output_unit": "V",
            "signal_domain": "analog_voltage",
            "technology_tags": ["voltage_input"],
            "input_mode_requirement": "single_ended",
            "nominal_range": {
                "minimum": -100.0,
                "maximum": 100.0,
                "unit": "A"
            },
            "frequency_range": {
                "minimum_hz": 10.0,
                "maximum_hz": 100000000.0
            },
            "settling_time_ms": 1.0,
            "scaling_profile_refs": [
                {
                    "entity_id": scaling_profile_id,
                    "require_approved": true
                }
            ],
            "correction_curve_refs": [
                {
                    "entity_id": curve_id,
                    "require_approved": true
                }
            ],
            "metadata": { "demo": true }
        })
    }

    fn recipe_definition(
        recipe_id: &str,
        daq_channel_profile_id: &str,
        sensor_definition_id: &str,
        scaling_profile_id: &str,
        curve_id: &str,
    ) -> Value {
        json!({
            "definition_schema_version": "emc-locus.acquisition-channel-recipe-definition.v1",
            "recipe_id": recipe_id,
            "label": "current_A",
            "output_channel_name": "current_A",
            "output_quantity": "current",
            "output_unit": "A",
            "daq_channel_profile_ref": {
                "entity_id": daq_channel_profile_id,
                "require_approved": true
            },
            "sensor_definition_ref": {
                "entity_id": sensor_definition_id,
                "require_approved": true
            },
            "scaling_profile_ref": {
                "entity_id": scaling_profile_id,
                "require_approved": true
            },
            "correction_curve_refs": [
                {
                    "entity_id": curve_id,
                    "require_approved": true
                }
            ],
            "sample_rate": 1000000.0,
            "range": {
                "minimum": -10.0,
                "maximum": 10.0,
                "unit": "V"
            },
            "coupling": "dc",
            "input_mode": "single_ended",
            "validation_rules": ["sample_rate_within_daq_profile", "range_within_daq_profile"],
            "metadata": { "demo": true }
        })
    }

    fn create_template_body(
        template_id: &str,
        operation_id: &str,
        definition_json: &str,
    ) -> String {
        format!(
            r#"{{
                "template_id": "{template_id}",
                "title": "Inrush current capture",
                "category_code": "emission_transient_time_domain",
                "definition": {definition_json},
                "actor": "method.author",
                "reason": "controlled template operation",
                "operation_id": "{operation_id}"
            }}"#
        )
    }

    fn template_definition(sample_rate_hz: u32, method: Option<(&str, &str)>) -> String {
        let method_fields = method.map_or_else(String::new, |(code, revision)| {
            format!(
                r#""method_code": "{code}",
                "method_revision": "{revision}","#
            )
        });
        format!(
            r#"{{
                "definition_schema_version": "emc-locus.test-template-definition.v1",
                "title": "Inrush current capture",
                "description": "Time-domain inrush capture for EMC investigations.",
                "measurement_axis": "time_series",
                {method_fields}
                "standard_references": ["IEC-61000-4-30"],
                "variables": [
                    {{
                        "variable_id": "sample_rate_hz",
                        "label": "Sample rate",
                        "value_type": "number",
                        "default_value": {sample_rate_hz}.0,
                        "constraints": {{
                            "required": true,
                            "unit": "Hz",
                            "minimum": 1000.0,
                            "maximum": 1000000.0
                        }},
                        "description": "DAQ sample rate"
                    }}
                ],
                "lock_policy": [
                    {{
                        "variable_id": "sample_rate_hz",
                        "policy": "editable_until_campaign_freeze"
                    }}
                ],
                "instrumentation_chain": [
                    {{
                        "slot_id": "current_probe",
                        "label": "Current probe",
                        "required_category": "current_probe",
                        "required": true,
                        "calibration_requirement": "required",
                        "substitution_policy": "approved_equivalent"
                    }},
                    {{
                        "slot_id": "daq",
                        "label": "DAQ",
                        "required_category": "daq_chassis",
                        "required_capability": "time_series_capture",
                        "required": true,
                        "calibration_requirement": "if_used",
                        "substitution_policy": "same_capability",
                        "depends_on_slots": ["current_probe"]
                    }}
                ],
                "entry_step_id": "arm",
                "sequence": [
                    {{
                        "step_id": "arm",
                        "order": 10,
                        "kind": "configure_instrument",
                        "label": "Arm acquisition",
                        "instruction": "Arm acquisition and wait for trigger.",
                        "required_slots": ["daq"],
                        "branches": [
                            {{
                                "rule_id": "manual_abort",
                                "condition": "operator_abort",
                                "destination_step_id": "finish",
                                "allow_cycle": false
                            }}
                        ]
                    }},
                    {{
                        "step_id": "capture",
                        "order": 20,
                        "kind": "acquire",
                        "label": "Capture transient",
                        "instruction": "Capture the inrush event.",
                        "required_slots": ["current_probe", "daq"]
                    }},
                    {{
                        "step_id": "finish",
                        "order": 30,
                        "kind": "finish",
                        "label": "Finish"
                    }}
                ],
                "limits": [
                    {{
                        "limit_id": "peak_current",
                        "kind": "scalar_threshold",
                        "axis": "time_series",
                        "unit": "A",
                        "application_domain": "inrush",
                        "source_reference": "method:TD-INRUSH:A",
                        "threshold": 30.0,
                        "attention_rule": "warn_above_80_percent",
                        "variable_refs": ["sample_rate_hz"]
                    }}
                ],
                "post_processing": [
                    {{
                        "operation_id": "peak",
                        "order": 10,
                        "operation_type": "peak",
                        "inputs": ["raw.current"],
                        "outputs": ["calculated.peak_current"],
                        "parameters": {{"absolute": true}}
                    }}
                ],
                "method_parameters": {{"alpha": {{"b": 2, "a": [3, 1]}}}}
            }}"#
        )
    }

    fn template_definition_without_variable_unit() -> String {
        let mut definition: Value =
            serde_json::from_str(&template_definition(100_000, None)).unwrap();
        definition["variables"][0]["constraints"]
            .as_object_mut()
            .unwrap()
            .remove("unit");
        render_json(&definition)
    }

    fn create_lab_dist_fixture(dist: &Path) {
        fs::create_dir_all(dist.join("assets")).unwrap();
        fs::write(
            dist.join("index.html"),
            "<!doctype html><html><body><div id=\"root\">LAB CONSOLE</div><script type=\"module\" src=\"/lab/assets/app.js\"></script></body></html>",
        )
        .unwrap();
        fs::write(
            dist.join("assets").join("app.js"),
            "console.log('lab console asset');",
        )
        .unwrap();
        fs::write(dist.join("assets").join("style.css"), "body{margin:0;}").unwrap();
    }

    fn free_loopback_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }
}
