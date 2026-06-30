from __future__ import annotations

import json
import unittest
from unittest.mock import patch
from urllib.error import HTTPError

from emc_locus.gui_actions import (
    advance_project_stage,
    complete_contract_review_item_action,
    create_project_record,
    record_metrology_calibration,
    register_metrology_instrument,
    run_simulated_emc_test_action,
    set_metrology_instrument_serviceability,
)
from emc_locus.local_agent_client import (
    LocalAgentClient,
    LocalAgentError,
    generate_operation_id,
)
from emc_locus.sqlite_repositories import MetrologyRepository, ProjectRepository


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

    def test_local_agent_client_idempotency_conflict_maps_to_structured_error(self) -> None:
        payload = {
            "error": {
                "code": "operation_replay_mismatch",
                "message": "operation_id is already used for a different canonical operation fingerprint",
                "details": {
                    "operation_id": "op-conflict",
                    "expected_fingerprint": "sha256:expected",
                    "stored_fingerprint": "sha256:stored",
                },
            }
        }
        error = HTTPError(
            "http://127.0.0.1:8765/api/v1/projects",
            409,
            "Conflict",
            hdrs=None,
            fp=_FakeResponse(payload),
        )

        with patch("emc_locus.local_agent_client.urlopen", side_effect=error):
            with self.assertRaises(LocalAgentError) as raised:
                LocalAgentClient("http://127.0.0.1:8765").create_project(
                    code="CEM-CONFLICT",
                    customer_name="Changed",
                    execution_mode="accredited",
                    actor="quality.lead",
                    reason="contract accepted",
                    operation_id="op-conflict",
                )

        self.assertEqual(raised.exception.status, 409)
        self.assertEqual(raised.exception.code, "operation_replay_mismatch")
        self.assertEqual(raised.exception.details["operation_id"], "op-conflict")

    def test_generated_operation_ids_are_stable_ascii_tokens(self) -> None:
        operation_id = generate_operation_id("project-create", "CEM 001")

        self.assertRegex(operation_id, r"^project-create-CEM-001-[0-9a-f]{32}$")

    def test_reads_storage_status(self) -> None:
        captured: dict[str, object] = {}

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            captured["url"] = request.full_url
            captured["method"] = request.get_method()
            captured["timeout"] = timeout
            return _FakeResponse(
                {
                    "action": "status",
                    "domains": [
                        {
                            "domain": "projects",
                            "status": "current",
                        }
                    ],
                }
            )

        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            response = LocalAgentClient(
                "http://127.0.0.1:8765",
                timeout_seconds=1.0,
            ).storage_status()

        self.assertEqual(captured["url"], "http://127.0.0.1:8765/api/v1/storage/status")
        self.assertEqual(captured["method"], "GET")
        self.assertEqual(captured["timeout"], 1.0)
        self.assertEqual(response["domains"][0]["status"], "current")

    def test_reads_project_slice_routes(self) -> None:
        captured: list[tuple[str, str]] = []
        payloads = {
            "http://127.0.0.1:8765/api/v1/projects": {"projects": []},
            "http://127.0.0.1:8765/api/v1/projects/CEM-READ-001": {
                "project": {"code": "CEM-READ-001"}
            },
            "http://127.0.0.1:8765/api/v1/projects/CEM-READ-001/contract-review": {
                "contract_review": {"project_code": "CEM-READ-001"}
            },
            "http://127.0.0.1:8765/api/v1/projects/CEM-READ-001/audit-events": {
                "audit_events": []
            },
            "http://127.0.0.1:8765/api/v1/projects/CEM-READ-001/test-executions": {
                "executions": []
            },
            "http://127.0.0.1:8765/api/v1/test-executions/RUN-READ-001": {
                "execution": {"attempt_id": "RUN-READ-001"}
            },
            "http://127.0.0.1:8765/api/v1/sync/outbox": {"sync_outbox": []},
        }

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            captured.append((request.get_method(), request.full_url))
            return _FakeResponse(payloads[request.full_url])

        client = LocalAgentClient("http://127.0.0.1:8765")
        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            self.assertEqual(client.list_projects()["projects"], [])
            self.assertEqual(client.get_project("CEM-READ-001")["project"]["code"], "CEM-READ-001")
            self.assertEqual(
                client.contract_review("CEM-READ-001")["contract_review"]["project_code"],
                "CEM-READ-001",
            )
            self.assertEqual(client.audit_events("CEM-READ-001")["audit_events"], [])
            self.assertEqual(
                client.list_project_test_executions("CEM-READ-001")["executions"],
                [],
            )
            self.assertEqual(
                client.get_test_execution("RUN-READ-001")["execution"]["attempt_id"],
                "RUN-READ-001",
            )
            self.assertEqual(client.sync_outbox()["sync_outbox"], [])

        self.assertEqual(
            captured,
            [
                ("GET", "http://127.0.0.1:8765/api/v1/projects"),
                ("GET", "http://127.0.0.1:8765/api/v1/projects/CEM-READ-001"),
                (
                    "GET",
                    "http://127.0.0.1:8765/api/v1/projects/CEM-READ-001/contract-review",
                ),
                ("GET", "http://127.0.0.1:8765/api/v1/projects/CEM-READ-001/audit-events"),
                ("GET", "http://127.0.0.1:8765/api/v1/projects/CEM-READ-001/test-executions"),
                ("GET", "http://127.0.0.1:8765/api/v1/test-executions/RUN-READ-001"),
                ("GET", "http://127.0.0.1:8765/api/v1/sync/outbox"),
            ],
        )

    def test_posts_simulated_emc_execution_payload(self) -> None:
        captured: dict[str, object] = {}

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            captured["url"] = request.full_url
            captured["method"] = request.get_method()
            captured["body"] = json.loads(request.data.decode("utf-8"))
            return _FakeResponse(
                {
                    "operation_id": "op-run",
                    "replayed": False,
                    "execution": {
                        "attempt_id": "RUN-SIM-PY",
                        "status": "completed",
                    },
                }
            )

        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            response = LocalAgentClient("http://127.0.0.1:8765").run_simulated_emc_test(
                attempt_id="RUN-SIM-PY",
                project_code="CEM-PY",
                test_method_reference="SIM-EMC-CONDUCTED",
                execution_mode="accredited",
                required_asset_ids=["SA-PY"],
                operator="operator.one",
                checked_on="2026-07-01",
                reason="operator launch",
                operation_id="op-run",
            )

        self.assertEqual(
            captured["url"],
            "http://127.0.0.1:8765/api/v1/test-executions/simulated-emc",
        )
        self.assertEqual(captured["method"], "POST")
        self.assertEqual(captured["body"]["required_asset_ids"], ["SA-PY"])
        self.assertEqual(response["execution"]["status"], "completed")

    def test_reads_metrology_slice_routes(self) -> None:
        captured: list[tuple[str, str, dict[str, object] | None]] = []
        payloads = {
            "http://127.0.0.1:8765/api/v1/metrology/instruments": {"instruments": []},
            "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001": {
                "instrument": {"asset_id": "SA-READ-001"}
            },
            "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001/calibrations": {
                "calibration_events": []
            },
            "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001/status?checked_on=2026-07-01": {
                "calibration_status": "valid"
            },
            "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001/audit-events": {
                "audit_events": []
            },
            "http://127.0.0.1:8765/api/v1/metrology/readiness": {
                "ready": True,
                "instrument_results": [],
                "blocking_issues": [],
                "warnings": [],
            },
        }

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            body = None
            if request.data is not None:
                body = json.loads(request.data.decode("utf-8"))
            captured.append((request.get_method(), request.full_url, body))
            return _FakeResponse(payloads[request.full_url])

        client = LocalAgentClient("http://127.0.0.1:8765")
        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            self.assertEqual(client.list_metrology_instruments()["instruments"], [])
            self.assertEqual(
                client.get_metrology_instrument("SA-READ-001")["instrument"]["asset_id"],
                "SA-READ-001",
            )
            self.assertEqual(
                client.list_metrology_calibrations("SA-READ-001")["calibration_events"],
                [],
            )
            self.assertEqual(
                client.get_metrology_calibration_status(
                    "SA-READ-001",
                    "2026-07-01",
                )["calibration_status"],
                "valid",
            )
            self.assertTrue(
                client.assess_metrology_readiness(
                    asset_ids=["SA-READ-001"],
                    execution_mode="accredited",
                    checked_on="2026-07-01",
                    context="pre-run",
                )["ready"]
            )
            self.assertEqual(client.metrology_audit_events("SA-READ-001")["audit_events"], [])

        self.assertEqual(
            captured,
            [
                ("GET", "http://127.0.0.1:8765/api/v1/metrology/instruments", None),
                (
                    "GET",
                    "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001",
                    None,
                ),
                (
                    "GET",
                    "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001/calibrations",
                    None,
                ),
                (
                    "GET",
                    "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001/status?checked_on=2026-07-01",
                    None,
                ),
                (
                    "POST",
                    "http://127.0.0.1:8765/api/v1/metrology/readiness",
                    {
                        "asset_ids": ["SA-READ-001"],
                        "execution_mode": "accredited",
                        "checked_on": "2026-07-01",
                        "context": "pre-run",
                    },
                ),
                (
                    "GET",
                    "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001/audit-events",
                    None,
                ),
            ],
        )


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

    def test_simulated_emc_test_uses_agent_and_returns_operator_message(self) -> None:
        with patch("emc_locus.gui_actions.LocalAgentClient") as client_type:
            client = client_type.return_value
            client.run_simulated_emc_test.return_value = {
                "operation_id": "op-sim",
                "replayed": False,
                "execution": {
                    "attempt_id": "RUN-SIM-PY",
                    "project_code": "CEM-PY",
                    "status": "refused",
                    "readiness": {"ready": False},
                    "refusal": {
                        "causes": [
                            {
                                "asset_id": "SA-PY",
                                "dimension": "missing_evidence",
                                "code": "calibration_missing",
                            }
                        ]
                    },
                },
            }

            result = run_simulated_emc_test_action(
                agent_url="http://127.0.0.1:8765",
                attempt_id="RUN-SIM-PY",
                project_code="CEM-PY",
                test_method_reference="SIM-EMC-CONDUCTED",
                execution_mode="accredited",
                asset_id="SA-PY",
                operator="operator.one",
                checked_on="2026-07-01",
                reason="operator launch",
                operation_id="op-sim",
            )

        client.run_simulated_emc_test.assert_called_once()
        self.assertEqual(result["status"], "refused")
        self.assertIn("SA-PY/missing_evidence/calibration_missing", result["message"])

    def test_gui_actions_project_reads_use_agent_when_agent_url_present(self) -> None:
        with patch("emc_locus.gui_actions.LocalAgentClient") as client_type:
            client = client_type.return_value
            client.list_projects.return_value = {
                "projects": [
                    {
                        "code": "CEM-AGENT-READ",
                        "customer_name": "Agent Read",
                        "execution_mode": "accredited",
                        "stage": "contract_review",
                    }
                ]
            }
            client.contract_review.return_value = {
                "contract_review": {
                    "completed_items": [
                        {
                            "item": "customer_request_defined",
                            "completed_by": "quality.lead",
                            "comment": "agent",
                        }
                    ]
                }
            }
            client.audit_events.return_value = {"audit_events": []}
            client.sync_outbox.return_value = {"sync_outbox": []}
            client.list_metrology_instruments.return_value = {"instruments": []}

            from emc_locus.gui_actions import refresh_bootstrap

            with patch("emc_locus.gui_actions._open_repository") as open_repository:
                with patch("emc_locus.gui_actions.write_bootstrap_js") as write_bootstrap:
                    refresh_bootstrap(
                        output="ignored.js",
                        projects_db="legacy-projects.sqlite",
                        agent_url="http://127.0.0.1:8765",
                    )

        client_type.assert_called_once_with("http://127.0.0.1:8765")
        client.list_projects.assert_called_once()
        client.contract_review.assert_called_once_with("CEM-AGENT-READ")
        client.audit_events.assert_called_once_with("CEM-AGENT-READ")
        client.sync_outbox.assert_called_once()
        self.assertNotIn(ProjectRepository, [call.args[0] for call in open_repository.call_args_list])
        self.assertNotIn(MetrologyRepository, [call.args[0] for call in open_repository.call_args_list])
        payload = write_bootstrap.call_args.args[1]
        self.assertEqual(payload["projects"][0]["code"], "CEM-AGENT-READ")
        self.assertEqual(payload["contract_review_items"][0][1], "customer_request_defined")

    def test_gui_actions_project_writes_use_agent_when_agent_url_present(self) -> None:
        with patch("emc_locus.gui_actions.LocalAgentClient") as client_type:
            with patch("emc_locus.gui_actions.ProjectRepository") as project_repository:
                client = client_type.return_value
                client.create_project.return_value = {
                    "operation_id": "op-create",
                    "replayed": False,
                    "project": {
                        "code": "CEM-AGENT-WRITE",
                        "customer_name": "Agent Write",
                        "execution_mode": "accredited",
                        "stage": "contract_review",
                    },
                }
                client.complete_contract_review_item.return_value = {
                    "operation_id": "op-review",
                    "replayed": False,
                    "already_completed": False,
                }
                client.advance_to_test_planning.return_value = {
                    "operation_id": "op-plan",
                    "replayed": False,
                    "project": {
                        "code": "CEM-AGENT-WRITE",
                        "stage": "test_planning",
                    },
                }

                create_project_record(
                    projects_db=None,
                    code="CEM-AGENT-WRITE",
                    customer_name="Agent Write",
                    execution_mode="accredited",
                    actor="quality.lead",
                    reason="accepted",
                    agent_url="http://127.0.0.1:8765",
                    operation_id="op-create",
                )
                complete_contract_review_item_action(
                    projects_db=None,
                    project_code="CEM-AGENT-WRITE",
                    item="customer_request_defined",
                    completed_by="quality.lead",
                    agent_url="http://127.0.0.1:8765",
                    operation_id="op-review",
                )
                advance_project_stage(
                    projects_db=None,
                    code="CEM-AGENT-WRITE",
                    actor="quality.lead",
                    reason="ready",
                    agent_url="http://127.0.0.1:8765",
                    operation_id="op-plan",
                )

        client.create_project.assert_called_once()
        client.complete_contract_review_item.assert_called_once()
        client.advance_to_test_planning.assert_called_once()
        project_repository.assert_not_called()

    def test_gui_actions_metrology_writes_use_agent_without_metrology_repository(self) -> None:
        with patch("emc_locus.gui_actions.LocalAgentClient") as client_type:
            with patch("emc_locus.gui_actions.MetrologyRepository") as metrology_repository:
                client = client_type.return_value
                client.register_metrology_instrument.return_value = {
                    "instrument": {
                        "asset_id": "SA-AGENT-WRITE",
                        "serviceability_status": "usable",
                    }
                }
                client.record_metrology_calibration.return_value = {
                    "calibration_event": {
                        "event_id": "CAL-SA-AGENT-WRITE-CERT-001",
                        "due_at": "2027-06-30",
                    }
                }
                client.set_metrology_serviceability.return_value = {
                    "instrument": {
                        "asset_id": "SA-AGENT-WRITE",
                        "serviceability_status": "restricted",
                    }
                }

                registered = register_metrology_instrument(
                    metrology_db=None,
                    asset_id="SA-AGENT-WRITE",
                    family="receiver",
                    manufacturer="Example",
                    model="SA9000",
                    serial_number="SN-001",
                    category_code="spectrum_analyzer",
                    calibration_period_months=12,
                    certificate_reference="CERT-001",
                    calibrated_at="2026-06-30",
                    provider="cal.lab",
                    file_reference="certs/CERT-001.pdf",
                    checksum="a" * 64,
                    agent_url="http://127.0.0.1:8765",
                )
                calibration = record_metrology_calibration(
                    metrology_db=None,
                    asset_id="SA-AGENT-WRITE",
                    certificate_reference="CERT-002",
                    calibrated_at="2026-07-01",
                    due_at="2027-07-01",
                    provider="cal.lab",
                    agent_url="http://127.0.0.1:8765",
                )
                serviceability = set_metrology_instrument_serviceability(
                    metrology_db=None,
                    asset_id="SA-AGENT-WRITE",
                    serviceability_status="restricted",
                    serviceability_reason="Use below 1 GHz only",
                    agent_url="http://127.0.0.1:8765",
                )

        client.register_metrology_instrument.assert_called_once()
        self.assertEqual(client.record_metrology_calibration.call_count, 2)
        initial_certificate = client.record_metrology_calibration.call_args_list[0].kwargs
        manifest = json.loads(initial_certificate["document_manifest_json"])
        self.assertEqual(manifest["sha256"], "a" * 64)
        self.assertEqual(manifest["local_reference"], "certs/CERT-001.pdf")
        client.set_metrology_serviceability.assert_called_once()
        metrology_repository.assert_not_called()
        self.assertEqual(registered["agent_url"], "http://127.0.0.1:8765")
        self.assertEqual(calibration["due_at"], "2027-06-30")
        self.assertEqual(serviceability["new_serviceability_status"], "restricted")


if __name__ == "__main__":
    unittest.main()
