"""Thin HTTP client for the local EMC Locus agent."""

from __future__ import annotations

from dataclasses import dataclass
import json
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlencode, urljoin
from urllib.request import Request, urlopen
import uuid


class LocalAgentError(RuntimeError):
    """Structured error returned by the local agent."""

    def __init__(
        self,
        *,
        status: int | None,
        code: str,
        message: str,
        details: dict[str, Any] | None = None,
    ) -> None:
        super().__init__(message)
        self.status = status
        self.code = code
        self.message = message
        self.details = details or {}


@dataclass(frozen=True)
class LocalAgentClient:
    """Minimal JSON client; business rules stay in the Rust agent."""

    base_url: str
    timeout_seconds: float = 5.0

    def request_json(
        self,
        method: str,
        path: str,
        payload: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        url = urljoin(self.base_url.rstrip("/") + "/", path.lstrip("/"))
        body = None if payload is None else json.dumps(payload).encode("utf-8")
        request = Request(
            url,
            data=body,
            method=method,
            headers={"Content-Type": "application/json"},
        )
        try:
            with urlopen(request, timeout=self.timeout_seconds) as response:
                text = response.read().decode("utf-8")
        except HTTPError as error:
            raise _agent_error_from_http(error) from error
        except URLError as error:
            raise LocalAgentError(
                status=None,
                code="local_agent_unavailable",
                message=str(error.reason),
            ) from error

        if not text.strip():
            return {}
        parsed = json.loads(text)
        if not isinstance(parsed, dict):
            raise LocalAgentError(
                status=None,
                code="invalid_agent_response",
                message="agent response must be a JSON object",
            )
        return parsed

    def health(self) -> dict[str, Any]:
        return self.request_json("GET", "/api/v1/health")

    def storage_status(self) -> dict[str, Any]:
        return self.request_json("GET", "/api/v1/storage/status")

    def list_projects(self) -> dict[str, Any]:
        return self.request_json("GET", "/api/v1/projects")

    def get_project(self, code: str) -> dict[str, Any]:
        return self.request_json("GET", f"/api/v1/projects/{quote(code)}")

    def contract_review(self, project_code: str) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/projects/{quote(project_code)}/contract-review",
        )

    def audit_events(self, project_code: str) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/projects/{quote(project_code)}/audit-events",
        )

    def sync_outbox(self) -> dict[str, Any]:
        return self.request_json("GET", "/api/v1/sync/outbox")

    def list_project_test_executions(self, project_code: str) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/projects/{quote(project_code)}/test-executions",
        )

    def get_test_execution(self, attempt_id: str) -> dict[str, Any]:
        return self.request_json("GET", f"/api/v1/test-executions/{quote(attempt_id)}")

    def create_project(
        self,
        *,
        code: str,
        customer_name: str,
        execution_mode: str,
        actor: str,
        reason: str,
        stage: str = "contract_review",
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id("project-create", code)
        payload = {
            "code": code,
            "customer_name": customer_name,
            "execution_mode": execution_mode,
            "stage": stage,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json("POST", "/api/v1/projects", payload)

    def complete_contract_review_item(
        self,
        *,
        project_code: str,
        item: str,
        actor: str,
        comment: str | None = None,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id("contract-review", project_code, item)
        payload = {
            "actor": actor,
            "operation_id": operation_id,
        }
        _put_optional(payload, "comment", comment)
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/projects/{quote(project_code)}/contract-review/items/{quote(item)}/complete",
            payload,
        )

    def advance_to_test_planning(
        self,
        *,
        project_code: str,
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id("to-test-planning", project_code)
        payload = {
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/projects/{quote(project_code)}/transitions/to-test-planning",
            payload,
        )

    def list_metrology_instruments(self) -> dict[str, Any]:
        return self.request_json("GET", "/api/v1/metrology/instruments")

    def get_metrology_instrument(self, asset_id: str) -> dict[str, Any]:
        return self.request_json("GET", f"/api/v1/metrology/instruments/{quote(asset_id)}")

    def list_metrology_calibrations(self, asset_id: str) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/metrology/instruments/{quote(asset_id)}/calibrations",
        )

    def get_metrology_calibration_status(
        self,
        asset_id: str,
        checked_on: str,
    ) -> dict[str, Any]:
        query = urlencode({"checked_on": checked_on})
        return self.request_json(
            "GET",
            f"/api/v1/metrology/instruments/{quote(asset_id)}/status?{query}",
        )

    def assess_metrology_readiness(
        self,
        *,
        asset_ids: list[str],
        execution_mode: str,
        checked_on: str,
        context: str | None = None,
    ) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "asset_ids": asset_ids,
            "execution_mode": execution_mode,
            "checked_on": checked_on,
        }
        _put_optional(payload, "context", context)
        return self.request_json("POST", "/api/v1/metrology/readiness", payload)

    def metrology_audit_events(self, asset_id: str) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/metrology/instruments/{quote(asset_id)}/audit-events",
        )

    def register_metrology_instrument(
        self,
        *,
        asset_id: str,
        family: str,
        category_code: str,
        manufacturer: str,
        model: str,
        serial_number: str,
        calibration_requirement: str,
        actor: str,
        reason: str,
        part_number: str | None = None,
        calibration_period_months: int | None = None,
        calibration_due_warning_days: int | None = None,
        serviceability_status: str = "usable",
        serviceability_reason: str | None = None,
        capabilities_json: str = "[]",
        metrology_notes: str | None = None,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id("metrology-register", asset_id)
        payload: dict[str, Any] = {
            "asset_id": asset_id,
            "family": family,
            "category_code": category_code,
            "manufacturer": manufacturer,
            "model": model,
            "serial_number": serial_number,
            "calibration_requirement": calibration_requirement,
            "serviceability_status": serviceability_status,
            "capabilities_json": capabilities_json,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "part_number", part_number)
        _put_optional_int(payload, "calibration_period_months", calibration_period_months)
        _put_optional_int(
            payload,
            "calibration_due_warning_days",
            calibration_due_warning_days,
        )
        _put_optional(payload, "serviceability_reason", serviceability_reason)
        _put_optional(payload, "metrology_notes", metrology_notes)
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json("POST", "/api/v1/metrology/instruments", payload)

    def record_metrology_calibration(
        self,
        *,
        asset_id: str,
        event_id: str,
        certificate_reference: str,
        calibrated_at: str,
        due_at: str,
        provider: str,
        recorded_by: str,
        actor: str,
        reason: str,
        decision: str = "conforming",
        as_found_status: str | None = None,
        as_left_status: str | None = None,
        adjustment_performed: bool = False,
        uncertainty_summary_json: str = "{}",
        traceability_reference: str | None = None,
        comment: str | None = None,
        document_manifest_json: str | None = None,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id("metrology-calibration", event_id)
        payload = {
            "event_id": event_id,
            "certificate_reference": certificate_reference,
            "calibrated_at": calibrated_at,
            "due_at": due_at,
            "provider": provider,
            "recorded_by": recorded_by,
            "decision": decision,
            "uncertainty_summary_json": uncertainty_summary_json,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "as_found_status", as_found_status)
        _put_optional(payload, "as_left_status", as_left_status)
        _put_optional_bool(payload, "adjustment_performed", adjustment_performed)
        _put_optional(payload, "traceability_reference", traceability_reference)
        _put_optional(payload, "comment", comment)
        _put_optional(payload, "document_manifest_json", document_manifest_json)
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/metrology/instruments/{quote(asset_id)}/calibrations",
            payload,
        )

    def set_metrology_serviceability(
        self,
        *,
        asset_id: str,
        serviceability_status: str,
        serviceability_reason: str,
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id("metrology-serviceability", asset_id)
        payload = {
            "serviceability_status": serviceability_status,
            "serviceability_reason": serviceability_reason,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/metrology/instruments/{quote(asset_id)}/serviceability",
            payload,
        )

    def run_simulated_emc_test(
        self,
        *,
        attempt_id: str,
        project_code: str,
        test_method_reference: str,
        execution_mode: str,
        required_asset_ids: list[str],
        operator: str,
        checked_on: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "simulated-emc",
            project_code,
            attempt_id,
        )
        payload = {
            "attempt_id": attempt_id,
            "project_code": project_code,
            "test_method_reference": test_method_reference,
            "execution_mode": execution_mode,
            "required_asset_ids": required_asset_ids,
            "operator": operator,
            "checked_on": checked_on,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json("POST", "/api/v1/test-executions/simulated-emc", payload)


def generate_operation_id(prefix: str, *parts: str) -> str:
    safe_parts = [
        "".join(character if character.isalnum() or character in "-_." else "-" for character in part)
        for part in parts
        if part
    ]
    suffix = uuid.uuid4().hex
    tokens = [prefix, *safe_parts, suffix]
    return "-".join(token for token in tokens if token)


def _agent_error_from_http(error: HTTPError) -> LocalAgentError:
    text = error.read().decode("utf-8", errors="replace")
    try:
        payload = json.loads(text)
    except json.JSONDecodeError:
        return LocalAgentError(
            status=error.code,
            code="local_agent_http_error",
            message=text or str(error),
        )
    agent_error = payload.get("error") if isinstance(payload, dict) else None
    if not isinstance(agent_error, dict):
        return LocalAgentError(
            status=error.code,
            code="local_agent_http_error",
            message=text or str(error),
        )
    details = agent_error.get("details")
    return LocalAgentError(
        status=error.code,
        code=str(agent_error.get("code", "local_agent_http_error")),
        message=str(agent_error.get("message", str(error))),
        details=details if isinstance(details, dict) else None,
    )


def _put_optional(payload: dict[str, Any], key: str, value: str | None) -> None:
    if value is not None and value.strip():
        payload[key] = value.strip()


def _put_optional_int(payload: dict[str, Any], key: str, value: int | None) -> None:
    if value is not None:
        payload[key] = value


def _put_optional_bool(payload: dict[str, Any], key: str, value: bool) -> None:
    if value:
        payload[key] = value
