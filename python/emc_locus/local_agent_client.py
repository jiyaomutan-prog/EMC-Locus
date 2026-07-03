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

    def list_documents(
        self,
        *,
        owner_domain: str | None = None,
        owner_entity_type: str | None = None,
        owner_entity_id: str | None = None,
    ) -> dict[str, Any]:
        query = {
            key: value
            for key, value in {
                "owner_domain": owner_domain,
                "owner_entity_type": owner_entity_type,
                "owner_entity_id": owner_entity_id,
            }.items()
            if value
        }
        suffix = f"?{urlencode(query)}" if query else ""
        return self.request_json("GET", f"/api/v1/documents{suffix}")

    def get_document(self, document_id: str) -> dict[str, Any]:
        return self.request_json("GET", f"/api/v1/documents/{quote(document_id)}")

    def document_audit_events(self, document_id: str) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/documents/{quote(document_id)}/audit-events",
        )

    def list_test_templates(
        self,
        *,
        category_code: str | None = None,
    ) -> dict[str, Any]:
        query = {
            key: value
            for key, value in {
                "category_code": category_code,
            }.items()
            if value
        }
        suffix = f"?{urlencode(query)}" if query else ""
        return self.request_json("GET", f"/api/v1/test-templates{suffix}")

    def get_test_template(self, template_id: str) -> dict[str, Any]:
        return self.request_json("GET", f"/api/v1/test-templates/{quote(template_id)}")

    def list_test_template_revisions(self, template_id: str) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/test-templates/{quote(template_id)}/revisions",
        )

    def get_test_template_revision(
        self,
        template_id: str,
        revision_id: str,
    ) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/test-templates/{quote(template_id)}/revisions/{quote(revision_id)}",
        )

    def test_template_audit_events(self, template_id: str) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/test-templates/{quote(template_id)}/audit-events",
        )

    def list_equipment_models(
        self,
        *,
        manufacturer: str | None = None,
        equipment_class: str | None = None,
        category_code: str | None = None,
        status: str | None = None,
        search: str | None = None,
    ) -> dict[str, Any]:
        query = {
            key: value
            for key, value in {
                "manufacturer": manufacturer,
                "equipment_class": equipment_class,
                "category_code": category_code,
                "status": status,
                "search": search,
            }.items()
            if value
        }
        suffix = f"?{urlencode(query)}" if query else ""
        return self.request_json("GET", f"/api/v1/equipment-models{suffix}")

    def get_equipment_model(self, equipment_model_id: str) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/equipment-models/{quote(equipment_model_id)}",
        )

    def list_equipment_model_revisions(self, equipment_model_id: str) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/equipment-models/{quote(equipment_model_id)}/revisions",
        )

    def get_equipment_model_revision(
        self,
        equipment_model_id: str,
        revision_id: str,
    ) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/equipment-models/{quote(equipment_model_id)}/revisions/{quote(revision_id)}",
        )

    def equipment_model_audit_events(self, equipment_model_id: str) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/equipment-models/{quote(equipment_model_id)}/audit-events",
        )

    def validate_equipment_model_definition(self, definition: dict[str, Any]) -> dict[str, Any]:
        return self.request_json(
            "POST",
            "/api/v1/equipment-model-definitions/validate",
            {"definition": definition},
        )

    def create_equipment_model(
        self,
        *,
        equipment_model_id: str,
        definition: dict[str, Any],
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id("equipment-model-create", equipment_model_id)
        payload = {
            "equipment_model_id": equipment_model_id,
            "definition": definition,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json("POST", "/api/v1/equipment-models", payload)

    def clone_equipment_model(
        self,
        *,
        source_equipment_model_id: str,
        new_equipment_model_id: str,
        actor: str,
        reason: str,
        source_revision_id: str | None = None,
        manufacturer: str | None = None,
        model_name: str | None = None,
        variant: str | None = None,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "equipment-model-clone",
            source_equipment_model_id,
            new_equipment_model_id,
        )
        payload = {
            "new_equipment_model_id": new_equipment_model_id,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "source_revision_id", source_revision_id)
        _put_optional(payload, "manufacturer", manufacturer)
        _put_optional(payload, "model_name", model_name)
        _put_optional(payload, "variant", variant)
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/equipment-models/{quote(source_equipment_model_id)}/clone",
            payload,
        )

    def replace_equipment_model_revision_definition(
        self,
        *,
        equipment_model_id: str,
        revision_id: str,
        expected_definition_checksum: str,
        definition: dict[str, Any],
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "equipment-model-definition-replace",
            equipment_model_id,
            revision_id,
        )
        payload = {
            "expected_definition_checksum": expected_definition_checksum,
            "definition": definition,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "PUT",
            f"/api/v1/equipment-models/{quote(equipment_model_id)}/revisions/{quote(revision_id)}/definition",
            payload,
        )

    def create_equipment_model_revision(
        self,
        *,
        equipment_model_id: str,
        source_revision_id: str,
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "equipment-model-revision-create",
            equipment_model_id,
            source_revision_id,
        )
        payload = {
            "source_revision_id": source_revision_id,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/equipment-models/{quote(equipment_model_id)}/revisions",
            payload,
        )

    def submit_equipment_model_revision_for_review(
        self,
        *,
        equipment_model_id: str,
        revision_id: str,
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "equipment-model-revision-submit",
            equipment_model_id,
            revision_id,
        )
        payload = {
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/equipment-models/{quote(equipment_model_id)}/revisions/{quote(revision_id)}/transitions/submit-for-review",
            payload,
        )

    def approve_equipment_model_revision(
        self,
        *,
        equipment_model_id: str,
        revision_id: str,
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "equipment-model-revision-approve",
            equipment_model_id,
            revision_id,
        )
        payload = {
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/equipment-models/{quote(equipment_model_id)}/revisions/{quote(revision_id)}/transitions/approve",
            payload,
        )

    def list_driver_profiles(
        self,
        *,
        equipment_model_id: str | None = None,
        status: str | None = None,
        search: str | None = None,
    ) -> dict[str, Any]:
        query = {
            key: value
            for key, value in {
                "equipment_model_id": equipment_model_id,
                "status": status,
                "search": search,
            }.items()
            if value
        }
        suffix = f"?{urlencode(query)}" if query else ""
        return self.request_json("GET", f"/api/v1/driver-profiles{suffix}")

    def get_driver_profile(self, driver_profile_id: str) -> dict[str, Any]:
        return self.request_json("GET", f"/api/v1/driver-profiles/{quote(driver_profile_id)}")

    def list_driver_profile_revisions(self, driver_profile_id: str) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/driver-profiles/{quote(driver_profile_id)}/revisions",
        )

    def get_driver_profile_revision(
        self,
        driver_profile_id: str,
        revision_id: str,
    ) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/driver-profiles/{quote(driver_profile_id)}/revisions/{quote(revision_id)}",
        )

    def driver_profile_audit_events(self, driver_profile_id: str) -> dict[str, Any]:
        return self.request_json(
            "GET",
            f"/api/v1/driver-profiles/{quote(driver_profile_id)}/audit-events",
        )

    def validate_driver_profile_definition(self, definition: dict[str, Any]) -> dict[str, Any]:
        return self.request_json(
            "POST",
            "/api/v1/driver-profile-definitions/validate",
            {"definition": definition},
        )

    def create_driver_profile(
        self,
        *,
        driver_profile_id: str,
        label: str,
        definition: dict[str, Any],
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id("driver-profile-create", driver_profile_id)
        payload = {
            "driver_profile_id": driver_profile_id,
            "label": label,
            "definition": definition,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json("POST", "/api/v1/driver-profiles", payload)

    def replace_driver_profile_revision_definition(
        self,
        *,
        driver_profile_id: str,
        revision_id: str,
        expected_definition_checksum: str,
        definition: dict[str, Any],
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "driver-profile-definition-replace",
            driver_profile_id,
            revision_id,
        )
        payload = {
            "expected_definition_checksum": expected_definition_checksum,
            "definition": definition,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "PUT",
            f"/api/v1/driver-profiles/{quote(driver_profile_id)}/revisions/{quote(revision_id)}/definition",
            payload,
        )

    def create_driver_profile_revision(
        self,
        *,
        driver_profile_id: str,
        source_revision_id: str,
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "driver-profile-revision-create",
            driver_profile_id,
            source_revision_id,
        )
        payload = {
            "source_revision_id": source_revision_id,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/driver-profiles/{quote(driver_profile_id)}/revisions",
            payload,
        )

    def submit_driver_profile_revision_for_review(
        self,
        *,
        driver_profile_id: str,
        revision_id: str,
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "driver-profile-revision-submit",
            driver_profile_id,
            revision_id,
        )
        payload = {
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/driver-profiles/{quote(driver_profile_id)}/revisions/{quote(revision_id)}/transitions/submit-for-review",
            payload,
        )

    def approve_driver_profile_revision(
        self,
        *,
        driver_profile_id: str,
        revision_id: str,
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "driver-profile-revision-approve",
            driver_profile_id,
            revision_id,
        )
        payload = {
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/driver-profiles/{quote(driver_profile_id)}/revisions/{quote(revision_id)}/transitions/approve",
            payload,
        )

    def simulate_driver_profile(
        self,
        *,
        driver_profile_id: str,
        action_id: str,
        scenario: dict[str, Any],
        revision_id: str | None = None,
    ) -> dict[str, Any]:
        payload = {
            "driver_profile_id": driver_profile_id,
            "action_id": action_id,
            "scenario": scenario,
        }
        _put_optional(payload, "revision_id", revision_id)
        return self.request_json("POST", "/api/v1/driver-profile-simulations", payload)

    def communication_provider_status(self) -> dict[str, Any]:
        return self.request_json("GET", "/api/v1/equipment/communication-providers")

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

    def register_attached_document(
        self,
        *,
        document_id: str,
        classification: str,
        title: str,
        owner_domain: str,
        owner_entity_type: str,
        owner_entity_id: str,
        storage_uri: str,
        original_filename: str,
        mime_type: str,
        size_bytes: int,
        sha256: str,
        actor: str,
        reason: str,
        storage_backend: str = "object_store",
        revision: str = "A",
        applicability: str = "applicable",
        confidentiality: str = "internal",
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id("document-register", document_id)
        payload: dict[str, Any] = {
            "document_id": document_id,
            "classification": classification,
            "title": title,
            "owner_domain": owner_domain,
            "owner_entity_type": owner_entity_type,
            "owner_entity_id": owner_entity_id,
            "storage_backend": storage_backend,
            "storage_uri": storage_uri,
            "original_filename": original_filename,
            "mime_type": mime_type,
            "size_bytes": size_bytes,
            "sha256": sha256,
            "revision": revision,
            "applicability": applicability,
            "confidentiality": confidentiality,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json("POST", "/api/v1/documents", payload)

    def create_test_template(
        self,
        *,
        template_id: str,
        title: str,
        category_code: str,
        definition: dict[str, Any],
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id("test-template-create", template_id)
        payload: dict[str, Any] = {
            "template_id": template_id,
            "title": title,
            "category_code": category_code,
            "definition": definition,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json("POST", "/api/v1/test-templates", payload)

    def replace_test_template_revision_definition(
        self,
        *,
        template_id: str,
        revision_id: str,
        expected_definition_checksum: str,
        definition: dict[str, Any],
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "test-template-definition-replace",
            template_id,
            revision_id,
        )
        payload = {
            "expected_definition_checksum": expected_definition_checksum,
            "definition": definition,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "PUT",
            f"/api/v1/test-templates/{quote(template_id)}/revisions/{quote(revision_id)}/definition",
            payload,
        )

    def create_test_template_revision(
        self,
        *,
        template_id: str,
        source_revision_id: str,
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "test-template-revision-create",
            template_id,
            source_revision_id,
        )
        payload = {
            "source_revision_id": source_revision_id,
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/test-templates/{quote(template_id)}/revisions",
            payload,
        )

    def submit_test_template_revision_for_review(
        self,
        *,
        template_id: str,
        revision_id: str,
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "test-template-revision-submit",
            template_id,
            revision_id,
        )
        payload = {
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/test-templates/{quote(template_id)}/revisions/{quote(revision_id)}/transitions/submit-for-review",
            payload,
        )

    def approve_test_template_revision(
        self,
        *,
        template_id: str,
        revision_id: str,
        actor: str,
        reason: str,
        operation_id: str | None = None,
        correlation_id: str | None = None,
        device_id: str | None = None,
    ) -> dict[str, Any]:
        operation_id = operation_id or generate_operation_id(
            "test-template-revision-approve",
            template_id,
            revision_id,
        )
        payload = {
            "actor": actor,
            "reason": reason,
            "operation_id": operation_id,
        }
        _put_optional(payload, "correlation_id", correlation_id)
        _put_optional(payload, "device_id", device_id)
        return self.request_json(
            "POST",
            f"/api/v1/test-templates/{quote(template_id)}/revisions/{quote(revision_id)}/transitions/approve",
            payload,
        )

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
