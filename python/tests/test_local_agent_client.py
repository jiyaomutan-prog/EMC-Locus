from __future__ import annotations

import json
import unittest
from unittest.mock import patch
from urllib.error import HTTPError

from emc_locus.gui_actions import (
    advance_project_stage,
    complete_contract_review_item_action,
    create_project_record,
)
from emc_locus.local_agent_client import (
    LocalAgentClient,
    LocalAgentError,
    generate_operation_id,
)


class _FakeResponse:
    def __init__(self, payload: dict[str, object]) -> None:
        self.payload = payload

    def __enter__(self) -> "_FakeResponse":
        return self

    def __exit__(self, *_: object) -> None:
        return None

    def read(self) -> bytes:
        return json.dumps(self.payload).encode("utf-8")

    def close(self) -> None:
        return None


class LocalAgentClientTests(unittest.TestCase):
    def test_posts_project_creation_payload(self) -> None:
        captured: dict[str, object] = {}

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            captured["url"] = request.full_url
            captured["method"] = request.get_method()
            captured["body"] = json.loads(request.data.decode("utf-8"))
            captured["timeout"] = timeout
            return _FakeResponse(
                {
                    "operation_id": "op-create",
                    "replayed": False,
                    "project": {
                        "code": "CEM-CLIENT-001",
                        "customer_name": "Client Customer",
                        "execution_mode": "accredited",
                        "stage": "contract_review",
                    },
                }
            )

        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            response = LocalAgentClient("http://127.0.0.1:8765").create_project(
                code="CEM-CLIENT-001",
                customer_name="Client Customer",
                execution_mode="accredited",
                actor="quality.lead",
                reason="contract accepted",
                operation_id="op-create",
            )

        self.assertEqual(captured["url"], "http://127.0.0.1:8765/api/v1/projects")
        self.assertEqual(captured["method"], "POST")
        self.assertEqual(captured["body"]["operation_id"], "op-create")
        self.assertEqual(response["project"]["stage"], "contract_review")

    def test_raises_structured_agent_error(self) -> None:
        payload = {
            "error": {
                "code": "contract_review_incomplete",
                "message": "Contract review is incomplete",
                "details": {"missing_items": ["customer_request_defined"]},
            }
        }
        error = HTTPError(
            "http://127.0.0.1:8765/api/v1/projects/CEM/transitions/to-test-planning",
            409,
            "Conflict",
            hdrs=None,
            fp=_FakeResponse(payload),
        )

        with patch("emc_locus.local_agent_client.urlopen", side_effect=error):
            with self.assertRaises(LocalAgentError) as raised:
                LocalAgentClient("http://127.0.0.1:8765").advance_to_test_planning(
                    project_code="CEM",
                    actor="quality.lead",
                    reason="ready",
                    operation_id="op-transition",
                )

        self.assertEqual(raised.exception.status, 409)
        self.assertEqual(raised.exception.code, "contract_review_incomplete")
        self.assertEqual(
            raised.exception.details["missing_items"],
            ["customer_request_defined"],
        )

    def test_generated_operation_ids_are_stable_ascii_tokens(self) -> None:
        operation_id = generate_operation_id("project-create", "CEM 001")

        self.assertRegex(operation_id, r"^project-create-CEM-001-[0-9a-f]{32}$")


class GuiActionAgentPathTests(unittest.TestCase):
    def test_create_project_uses_agent_without_project_repository(self) -> None:
        with patch("emc_locus.gui_actions.LocalAgentClient") as client_type:
            client = client_type.return_value
            client.create_project.return_value = {
                "operation_id": "op-agent-create",
                "replayed": False,
                "project": {
                    "code": "CEM-AGENT-PY",
                    "customer_name": "Agent Customer",
                    "execution_mode": "accredited",
                    "stage": "contract_review",
                },
            }

            result = create_project_record(
                projects_db=None,
                code="CEM-AGENT-PY",
                customer_name="Agent Customer",
                execution_mode="accredited",
                actor="quality.lead",
                reason="contract accepted",
                agent_url="http://127.0.0.1:8765",
                operation_id="op-agent-create",
            )

        client.create_project.assert_called_once()
        self.assertEqual(result["stage"], "contract_review")
        self.assertEqual(result["operation_id"], "op-agent-create")

    def test_complete_review_item_uses_agent_without_project_repository(self) -> None:
        with patch("emc_locus.gui_actions.LocalAgentClient") as client_type:
            client = client_type.return_value
            client.complete_contract_review_item.return_value = {
                "operation_id": "op-agent-review",
                "replayed": False,
                "already_completed": False,
            }

            result = complete_contract_review_item_action(
                projects_db=None,
                project_code="CEM-AGENT-PY",
                item="customer_request_defined",
                completed_by="quality.lead",
                agent_url="http://127.0.0.1:8765",
                operation_id="op-agent-review",
            )

        client.complete_contract_review_item.assert_called_once()
        self.assertEqual(result["operation_id"], "op-agent-review")

    def test_transition_uses_agent_without_project_repository(self) -> None:
        with patch("emc_locus.gui_actions.LocalAgentClient") as client_type:
            client = client_type.return_value
            client.advance_to_test_planning.return_value = {
                "operation_id": "op-agent-plan",
                "replayed": False,
                "project": {
                    "code": "CEM-AGENT-PY",
                    "stage": "test_planning",
                },
            }

            result = advance_project_stage(
                projects_db=None,
                code="CEM-AGENT-PY",
                actor="quality.lead",
                reason="review complete",
                agent_url="http://127.0.0.1:8765",
                operation_id="op-agent-plan",
            )

        client.advance_to_test_planning.assert_called_once()
        self.assertEqual(result["new_stage"], "test_planning")


if __name__ == "__main__":
    unittest.main()
