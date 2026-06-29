"""Thin HTTP client for the local EMC Locus agent."""

from __future__ import annotations

from dataclasses import dataclass
import json
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urljoin
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
