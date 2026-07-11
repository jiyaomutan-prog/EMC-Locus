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


def _template_definition(sample_rate_hz: float = 100_000.0) -> dict[str, object]:
    return {
        "definition_schema_version": "emc-locus.test-template-definition.v1",
        "title": "Inrush current capture",
        "description": "Time-domain inrush capture for EMC investigations.",
        "measurement_axis": "time_series",
        "standard_references": ["IEC-61000-4-30"],
        "variables": [
            {
                "variable_id": "sample_rate_hz",
                "label": "Sample rate",
                "value_type": "number",
                "default_value": sample_rate_hz,
                "constraints": {
                    "required": True,
                    "unit": "Hz",
                    "minimum": 1000.0,
                    "maximum": 1000000.0,
                },
            }
        ],
        "lock_policy": [
            {
                "variable_id": "sample_rate_hz",
                "policy": "editable_until_campaign_freeze",
            }
        ],
        "instrumentation_chain": [
            {
                "slot_id": "daq",
                "label": "DAQ",
                "required_category": "daq_chassis",
                "required": True,
                "calibration_requirement": "if_used",
                "substitution_policy": "same_capability",
            }
        ],
        "entry_step_id": "capture",
        "sequence": [
            {
                "step_id": "capture",
                "order": 10,
                "kind": "acquire",
                "label": "Capture transient",
                "required_slots": ["daq"],
            }
        ],
        "limits": [
            {
                "limit_id": "peak_current",
                "kind": "scalar_threshold",
                "axis": "time_series",
                "unit": "A",
                "application_domain": "inrush",
                "source_reference": "method:TD-INRUSH:A",
                "threshold": 30.0,
                "variable_refs": ["sample_rate_hz"],
            }
        ],
        "post_processing": [
            {
                "operation_id": "peak",
                "order": 10,
                "operation_type": "peak",
                "inputs": ["raw.current"],
                "outputs": ["calculated.peak_current"],
                "parameters": {"absolute": True},
            }
        ],
        "method_parameters": {},
    }


def _equipment_model_definition() -> dict[str, object]:
    return {
        "definition_schema_version": "emc-locus.equipment-model-definition.v2",
        "manufacturer": "Demo",
        "model_name": "Python Power Meter",
        "variant": "FWD",
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
                "unit": "MHz",
                "minimum": 9.0,
                "maximum": 1000.0,
            }
        ],
        "signal_ports": [
            {
                "port_id": "rf_input",
                "label": "RF input",
                "directionality": "input",
                "flow_role": "measurement_port",
                "signal_domain": "rf",
                "connector_type": "N",
                "quantity": "power",
                "unit": "dBm",
                "impedance": 50.0,
            }
        ],
        "communication_interfaces": [
            {
                "interface_id": "tcp",
                "label": "SCPI TCP",
                "transport_kind": "ethernet_tcp",
                "access_provider_kind": "native_tcp",
                "protocol_kind": "scpi",
                "required": True,
                "default_interface": True,
                "default_configuration": {
                    "host": "127.0.0.1",
                    "port": 5025,
                    "write_terminator": "\n",
                    "read_terminator": "\n",
                    "timeout_ms": 1000,
                },
                "identification_strategy": {
                    "kind": "scpi_idn",
                    "query": "*IDN?",
                    "response_regex": "^Demo,Python Power Meter,",
                },
            }
        ],
        "capabilities": [
            {
                "capability_id": "measure_power",
                "label": "Measure power",
                "description": "Read RF power.",
                "capability_kind": "measure_power",
                "inputs": [],
                "outputs": [
                    {
                        "name": "power_dbm",
                        "value_type": "number",
                        "quantity": "power",
                        "unit": "dBm",
                        "required": True,
                    }
                ],
                "required_signal_ports": ["rf_input"],
                "safety_class": "read_only",
            }
        ],
        "metadata": {},
    }


def _driver_profile_definition() -> dict[str, object]:
    return {
        "definition_schema_version": "emc-locus.driver-profile-definition.v1",
        "equipment_model_id": "EQM-PY-POWER",
        "supported_model_revision_id": "EQM-PY-POWER-rev-0001",
        "supported_model_definition_checksum": "sha256:" + "a" * 64,
        "supported_firmware_ranges": ["*"],
        "communication_profiles": ["tcp"],
        "actions": [
            {
                "action_id": "measure_power",
                "label": "Measure power",
                "description": "Query power value.",
                "implements_capability_id": "measure_power",
                "inputs": [],
                "outputs": [
                    {
                        "name": "power_dbm",
                        "value_type": "number",
                        "quantity": "power",
                        "unit": "dBm",
                        "required": True,
                    }
                ],
                "safety_class": "read_only",
                "default_timeout_ms": 1000,
                "script": {
                    "steps": [
                        {
                            "step_id": "query_power",
                            "step_type": "io_query",
                            "interface_id": "tcp",
                            "payload_format": "text",
                            "payload": "MEAS:POW?",
                            "response_binding": "${result.power_dbm}",
                        },
                        {
                            "step_id": "return",
                            "step_type": "return",
                            "return_values": ["${result.power_dbm}"],
                        },
                    ]
                },
            }
        ],
        "metadata": {},
    }


def _driver_simulation_scenario() -> dict[str, object]:
    return {
        "scenario_id": "PY-SIM-POWER",
        "driver_revision_id": "DRV-PY-POWER-rev-0001",
        "action_id": "measure_power",
        "input_values": {},
        "simulated_responses": [
            {
                "step_id": "query_power",
                "response": "-12.5",
            }
        ],
        "expected_outputs": {"power_dbm": -12.5},
    }


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
            "http://127.0.0.1:8765/api/v1/documents": {"documents": []},
            "http://127.0.0.1:8765/api/v1/documents/DOC-READ-001": {
                "document": {"document_id": "DOC-READ-001"}
            },
            "http://127.0.0.1:8765/api/v1/documents/DOC-READ-001/audit-events": {
                "audit_events": []
            },
            "http://127.0.0.1:8765/api/v1/test-templates": {"test_templates": []},
            "http://127.0.0.1:8765/api/v1/test-templates/TT-READ-001": {
                "test_template": {"identity": {"template_id": "TT-READ-001"}}
            },
            "http://127.0.0.1:8765/api/v1/test-templates/TT-READ-001/revisions": {
                "revisions": []
            },
            "http://127.0.0.1:8765/api/v1/test-templates/TT-READ-001/revisions/TT-READ-001-rev-0001": {
                "revision": {"revision_id": "TT-READ-001-rev-0001"}
            },
            "http://127.0.0.1:8765/api/v1/test-templates/TT-READ-001/audit-events": {
                "audit_events": []
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
            self.assertEqual(client.list_documents()["documents"], [])
            self.assertEqual(
                client.get_document("DOC-READ-001")["document"]["document_id"],
                "DOC-READ-001",
            )
            self.assertEqual(client.document_audit_events("DOC-READ-001")["audit_events"], [])
            self.assertEqual(client.list_test_templates()["test_templates"], [])
            self.assertEqual(
                client.get_test_template("TT-READ-001")["test_template"]["identity"]["template_id"],
                "TT-READ-001",
            )
            self.assertEqual(client.list_test_template_revisions("TT-READ-001")["revisions"], [])
            self.assertEqual(
                client.get_test_template_revision(
                    "TT-READ-001",
                    "TT-READ-001-rev-0001",
                )["revision"]["revision_id"],
                "TT-READ-001-rev-0001",
            )
            self.assertEqual(client.test_template_audit_events("TT-READ-001")["audit_events"], [])
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
                ("GET", "http://127.0.0.1:8765/api/v1/documents"),
                ("GET", "http://127.0.0.1:8765/api/v1/documents/DOC-READ-001"),
                ("GET", "http://127.0.0.1:8765/api/v1/documents/DOC-READ-001/audit-events"),
                ("GET", "http://127.0.0.1:8765/api/v1/test-templates"),
                ("GET", "http://127.0.0.1:8765/api/v1/test-templates/TT-READ-001"),
                ("GET", "http://127.0.0.1:8765/api/v1/test-templates/TT-READ-001/revisions"),
                (
                    "GET",
                    "http://127.0.0.1:8765/api/v1/test-templates/TT-READ-001/revisions/TT-READ-001-rev-0001",
                ),
                ("GET", "http://127.0.0.1:8765/api/v1/test-templates/TT-READ-001/audit-events"),
                ("GET", "http://127.0.0.1:8765/api/v1/sync/outbox"),
            ],
        )

    def test_reads_equipment_catalog_routes(self) -> None:
        captured: list[tuple[str, str]] = []
        payloads = {
            "http://127.0.0.1:8765/api/v1/equipment-models?manufacturer=Demo&equipment_class=controllable_instrument&category_code=power_meter&status=approved&search=power": {
                "equipment_models": []
            },
            "http://127.0.0.1:8765/api/v1/equipment-models/EQM-PY-POWER": {
                "equipment_model": {"identity": {"equipment_model_id": "EQM-PY-POWER"}}
            },
            "http://127.0.0.1:8765/api/v1/equipment-models/EQM-PY-POWER/revisions": {
                "revisions": []
            },
            "http://127.0.0.1:8765/api/v1/equipment-models/EQM-PY-POWER/revisions/EQM-PY-POWER-rev-0001": {
                "revision": {"revision_id": "EQM-PY-POWER-rev-0001"}
            },
            "http://127.0.0.1:8765/api/v1/equipment-models/EQM-PY-POWER/audit-events": {
                "audit_events": []
            },
            "http://127.0.0.1:8765/api/v1/driver-profiles?equipment_model_id=EQM-PY-POWER&status=approved&search=power": {
                "driver_profiles": []
            },
            "http://127.0.0.1:8765/api/v1/driver-profiles/DRV-PY-POWER": {
                "driver_profile": {"identity": {"driver_profile_id": "DRV-PY-POWER"}}
            },
            "http://127.0.0.1:8765/api/v1/driver-profiles/DRV-PY-POWER/revisions": {
                "revisions": []
            },
            "http://127.0.0.1:8765/api/v1/driver-profiles/DRV-PY-POWER/revisions/DRV-PY-POWER-rev-0001": {
                "revision": {"revision_id": "DRV-PY-POWER-rev-0001"}
            },
            "http://127.0.0.1:8765/api/v1/driver-profiles/DRV-PY-POWER/audit-events": {
                "audit_events": []
            },
            "http://127.0.0.1:8765/api/v1/equipment/communication-providers": {
                "providers": [{"provider": "simulation", "available": True}]
            },
        }

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            captured.append((request.get_method(), request.full_url))
            return _FakeResponse(payloads[request.full_url])

        client = LocalAgentClient("http://127.0.0.1:8765")
        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            self.assertEqual(
                client.list_equipment_models(
                    manufacturer="Demo",
                    equipment_class="controllable_instrument",
                    category_code="power_meter",
                    status="approved",
                    search="power",
                )["equipment_models"],
                [],
            )
            self.assertEqual(
                client.get_equipment_model("EQM-PY-POWER")["equipment_model"]["identity"]["equipment_model_id"],
                "EQM-PY-POWER",
            )
            self.assertEqual(client.list_equipment_model_revisions("EQM-PY-POWER")["revisions"], [])
            self.assertEqual(
                client.get_equipment_model_revision(
                    "EQM-PY-POWER",
                    "EQM-PY-POWER-rev-0001",
                )["revision"]["revision_id"],
                "EQM-PY-POWER-rev-0001",
            )
            self.assertEqual(client.equipment_model_audit_events("EQM-PY-POWER")["audit_events"], [])
            self.assertEqual(
                client.list_driver_profiles(
                    equipment_model_id="EQM-PY-POWER",
                    status="approved",
                    search="power",
                )["driver_profiles"],
                [],
            )
            self.assertEqual(
                client.get_driver_profile("DRV-PY-POWER")["driver_profile"]["identity"]["driver_profile_id"],
                "DRV-PY-POWER",
            )
            self.assertEqual(client.list_driver_profile_revisions("DRV-PY-POWER")["revisions"], [])
            self.assertEqual(
                client.get_driver_profile_revision(
                    "DRV-PY-POWER",
                    "DRV-PY-POWER-rev-0001",
                )["revision"]["revision_id"],
                "DRV-PY-POWER-rev-0001",
            )
            self.assertEqual(client.driver_profile_audit_events("DRV-PY-POWER")["audit_events"], [])
            self.assertTrue(client.communication_provider_status()["providers"][0]["available"])

        self.assertEqual(len(captured), 11)

    def test_posts_equipment_model_revision_payloads(self) -> None:
        captured: list[tuple[str, str, dict[str, object]]] = []

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            body = json.loads(request.data.decode("utf-8"))
            captured.append((request.get_method(), request.full_url, body))
            if request.full_url.endswith("/validate"):
                return _FakeResponse({"valid": True, "issues": [], "definition_checksum": "sha256:" + "a" * 64})
            return _FakeResponse(
                {
                    "operation_id": body.get("operation_id", "op"),
                    "replayed": False,
                    "revision": {
                        "revision_id": "EQM-PY-POWER-rev-0001",
                        "status": "draft",
                    },
                }
            )

        client = LocalAgentClient("http://127.0.0.1:8765")
        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            self.assertTrue(client.validate_equipment_model_definition(_equipment_model_definition())["valid"])
            client.create_equipment_model(
                equipment_model_id="EQM-PY-POWER",
                definition=_equipment_model_definition(),
                actor="catalog.author",
                reason="create model",
                operation_id="op-model-create",
            )
            client.replace_equipment_model_revision_definition(
                equipment_model_id="EQM-PY-POWER",
                revision_id="EQM-PY-POWER-rev-0001",
                expected_definition_checksum="sha256:" + "a" * 64,
                definition=_equipment_model_definition(),
                actor="catalog.author",
                reason="save draft",
                operation_id="op-model-save",
            )
            client.create_equipment_model_revision(
                equipment_model_id="EQM-PY-POWER",
                source_revision_id="EQM-PY-POWER-rev-0001",
                actor="catalog.author",
                reason="derive next draft",
                operation_id="op-model-derive",
            )
            client.clone_equipment_model(
                source_equipment_model_id="EQM-PY-POWER",
                new_equipment_model_id="EQM-PY-POWER-CLONE",
                model_name="Python Power Meter Clone",
                actor="catalog.author",
                reason="clone model",
                operation_id="op-model-clone",
            )
            client.submit_equipment_model_revision_for_review(
                equipment_model_id="EQM-PY-POWER",
                revision_id="EQM-PY-POWER-rev-0001",
                actor="catalog.author",
                reason="ready",
                operation_id="op-model-submit",
            )
            client.approve_equipment_model_revision(
                equipment_model_id="EQM-PY-POWER",
                revision_id="EQM-PY-POWER-rev-0001",
                actor="catalog.reviewer",
                reason="accepted",
                operation_id="op-model-approve",
            )

        self.assertEqual(captured[0][1], "http://127.0.0.1:8765/api/v1/equipment-model-definitions/validate")
        self.assertEqual(captured[1][2]["equipment_model_id"], "EQM-PY-POWER")
        self.assertEqual(captured[2][2]["expected_definition_checksum"], "sha256:" + "a" * 64)
        self.assertEqual(captured[3][2]["source_revision_id"], "EQM-PY-POWER-rev-0001")
        self.assertEqual(captured[4][2]["new_equipment_model_id"], "EQM-PY-POWER-CLONE")
        self.assertEqual(captured[5][2]["operation_id"], "op-model-submit")
        self.assertEqual(captured[6][2]["operation_id"], "op-model-approve")

    def test_posts_driver_profile_revision_and_simulation_payloads(self) -> None:
        captured: list[tuple[str, str, dict[str, object]]] = []

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            body = json.loads(request.data.decode("utf-8"))
            captured.append((request.get_method(), request.full_url, body))
            if request.full_url.endswith("/validate"):
                return _FakeResponse({"valid": True, "issues": [], "definition_checksum": "sha256:" + "b" * 64})
            if request.full_url.endswith("/driver-profile-simulations"):
                return _FakeResponse({"simulation": {"status": "passed", "trace": []}})
            return _FakeResponse(
                {
                    "operation_id": body.get("operation_id", "op"),
                    "replayed": False,
                    "revision": {
                        "revision_id": "DRV-PY-POWER-rev-0001",
                        "status": "draft",
                    },
                }
            )

        client = LocalAgentClient("http://127.0.0.1:8765")
        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            self.assertTrue(client.validate_driver_profile_definition(_driver_profile_definition())["valid"])
            client.create_driver_profile(
                driver_profile_id="DRV-PY-POWER",
                label="Python power meter SCPI",
                definition=_driver_profile_definition(),
                actor="driver.author",
                reason="create driver",
                operation_id="op-driver-create",
            )
            client.replace_driver_profile_revision_definition(
                driver_profile_id="DRV-PY-POWER",
                revision_id="DRV-PY-POWER-rev-0001",
                expected_definition_checksum="sha256:" + "b" * 64,
                definition=_driver_profile_definition(),
                actor="driver.author",
                reason="save draft",
                operation_id="op-driver-save",
            )
            client.create_driver_profile_revision(
                driver_profile_id="DRV-PY-POWER",
                source_revision_id="DRV-PY-POWER-rev-0001",
                actor="driver.author",
                reason="derive next draft",
                operation_id="op-driver-derive",
            )
            client.submit_driver_profile_revision_for_review(
                driver_profile_id="DRV-PY-POWER",
                revision_id="DRV-PY-POWER-rev-0001",
                actor="driver.author",
                reason="ready",
                operation_id="op-driver-submit",
            )
            client.approve_driver_profile_revision(
                driver_profile_id="DRV-PY-POWER",
                revision_id="DRV-PY-POWER-rev-0001",
                actor="driver.reviewer",
                reason="accepted",
                operation_id="op-driver-approve",
            )
            simulation = client.simulate_driver_profile(
                driver_profile_id="DRV-PY-POWER",
                revision_id="DRV-PY-POWER-rev-0001",
                action_id="measure_power",
                scenario=_driver_simulation_scenario(),
            )

        self.assertEqual(captured[0][1], "http://127.0.0.1:8765/api/v1/driver-profile-definitions/validate")
        self.assertEqual(captured[1][2]["driver_profile_id"], "DRV-PY-POWER")
        self.assertEqual(captured[2][2]["expected_definition_checksum"], "sha256:" + "b" * 64)
        self.assertEqual(captured[3][2]["source_revision_id"], "DRV-PY-POWER-rev-0001")
        self.assertEqual(captured[4][2]["operation_id"], "op-driver-submit")
        self.assertEqual(captured[5][2]["operation_id"], "op-driver-approve")
        self.assertEqual(captured[6][2]["scenario"]["scenario_id"], "PY-SIM-POWER")
        self.assertEqual(simulation["simulation"]["status"], "passed")

    def test_posts_attached_document_payload(self) -> None:
        captured: dict[str, object] = {}

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            captured["url"] = request.full_url
            captured["method"] = request.get_method()
            captured["body"] = json.loads(request.data.decode("utf-8"))
            return _FakeResponse(
                {
                    "operation_id": "op-doc",
                    "replayed": False,
                    "document": {
                        "document_id": "DOC-PY-001",
                        "owner_domain": "locus_lab_management",
                    },
                }
            )

        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            response = LocalAgentClient("http://127.0.0.1:8765").register_attached_document(
                document_id="DOC-PY-001",
                classification="client_document",
                title="Customer requirements",
                owner_domain="locus_lab_management",
                owner_entity_type="project",
                owner_entity_id="CEM-PY",
                storage_uri="objects/projects/CEM-PY/requirements.pdf",
                original_filename="requirements.pdf",
                mime_type="application/pdf",
                size_bytes=123,
                sha256="f" * 64,
                actor="project.manager",
                reason="customer requirement received",
                operation_id="op-doc",
            )

        self.assertEqual(captured["url"], "http://127.0.0.1:8765/api/v1/documents")
        self.assertEqual(captured["method"], "POST")
        body = captured["body"]
        self.assertEqual(body["document_id"], "DOC-PY-001")
        self.assertEqual(body["storage_backend"], "object_store")
        self.assertEqual(body["sha256"], "f" * 64)
        self.assertEqual(response["document"]["owner_domain"], "locus_lab_management")

    def test_list_documents_encodes_owner_filter(self) -> None:
        captured: dict[str, object] = {}

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            captured["url"] = request.full_url
            captured["method"] = request.get_method()
            return _FakeResponse({"documents": []})

        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            LocalAgentClient("http://127.0.0.1:8765").list_documents(
                owner_domain="locus_lab_management",
                owner_entity_type="project",
                owner_entity_id="CEM PY",
            )

        self.assertEqual(captured["method"], "GET")
        self.assertEqual(
            captured["url"],
            "http://127.0.0.1:8765/api/v1/documents?owner_domain=locus_lab_management&owner_entity_type=project&owner_entity_id=CEM+PY",
        )

    def test_posts_test_template_payload(self) -> None:
        captured: dict[str, object] = {}

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            captured["url"] = request.full_url
            captured["method"] = request.get_method()
            captured["body"] = json.loads(request.data.decode("utf-8"))
            return _FakeResponse(
                {
                    "operation_id": "op-template",
                    "replayed": False,
                    "test_template": {
                        "identity": {"template_id": "TT-PY-001"},
                    },
                    "revision": {
                        "revision_id": "TT-PY-001-rev-0001",
                        "status": "draft",
                    },
                }
            )

        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            response = LocalAgentClient("http://127.0.0.1:8765").create_test_template(
                template_id="TT-PY-001",
                title="Python client template",
                category_code="emission_transient_time_domain",
                definition=_template_definition(),
                actor="method.author",
                reason="first draft",
                operation_id="op-template",
            )

        self.assertEqual(captured["url"], "http://127.0.0.1:8765/api/v1/test-templates")
        self.assertEqual(captured["method"], "POST")
        body = captured["body"]
        self.assertEqual(body["template_id"], "TT-PY-001")
        self.assertEqual(body["definition"]["definition_schema_version"], "emc-locus.test-template-definition.v1")
        self.assertEqual(body["definition"]["variables"][0]["variable_id"], "sample_rate_hz")
        self.assertEqual(body["definition"]["instrumentation_chain"][0]["slot_id"], "daq")
        self.assertEqual(response["revision"]["status"], "draft")

    def test_list_test_templates_encodes_filters(self) -> None:
        captured: dict[str, object] = {}

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            captured["url"] = request.full_url
            captured["method"] = request.get_method()
            return _FakeResponse({"test_templates": []})

        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            LocalAgentClient("http://127.0.0.1:8765").list_test_templates(
                category_code="emission_transient_time_domain",
            )

        self.assertEqual(captured["method"], "GET")
        self.assertEqual(
            captured["url"],
            "http://127.0.0.1:8765/api/v1/test-templates?category_code=emission_transient_time_domain",
        )

    def test_posts_test_template_revision_edit_and_derivation_payloads(self) -> None:
        captured: list[tuple[str, str, dict[str, object]]] = []

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            body = json.loads(request.data.decode("utf-8"))
            captured.append((request.get_method(), request.full_url, body))
            operation = (
                "test_template_definition_replaced"
                if request.get_method() == "PUT"
                else "test_template_revision_created"
            )
            revision_id = (
                "TT-PY-001-rev-0001"
                if request.get_method() == "PUT"
                else "TT-PY-001-rev-0002"
            )
            return _FakeResponse(
                {
                    "operation": operation,
                    "operation_id": body["operation_id"],
                    "replayed": False,
                    "revision": {"revision_id": revision_id, "status": "draft"},
                }
            )

        client = LocalAgentClient("http://127.0.0.1:8765")
        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            edited = client.replace_test_template_revision_definition(
                template_id="TT-PY-001",
                revision_id="TT-PY-001-rev-0001",
                expected_definition_checksum="sha256:" + "a" * 64,
                definition=_template_definition(200_000.0),
                actor="method.author",
                reason="update draft",
                operation_id="op-edit",
            )
            derived = client.create_test_template_revision(
                template_id="TT-PY-001",
                source_revision_id="TT-PY-001-rev-0001",
                actor="method.author",
                reason="derive next revision",
                operation_id="op-rev2",
            )

        self.assertEqual(edited["operation"], "test_template_definition_replaced")
        self.assertEqual(derived["operation"], "test_template_revision_created")
        self.assertEqual(
            captured,
            [
                (
                    "PUT",
                    "http://127.0.0.1:8765/api/v1/test-templates/TT-PY-001/revisions/TT-PY-001-rev-0001/definition",
                    {
                        "expected_definition_checksum": "sha256:" + "a" * 64,
                        "definition": _template_definition(200_000.0),
                        "actor": "method.author",
                        "reason": "update draft",
                        "operation_id": "op-edit",
                    },
                ),
                (
                    "POST",
                    "http://127.0.0.1:8765/api/v1/test-templates/TT-PY-001/revisions",
                    {
                        "source_revision_id": "TT-PY-001-rev-0001",
                        "actor": "method.author",
                        "reason": "derive next revision",
                        "operation_id": "op-rev2",
                    },
                ),
            ],
        )

    def test_posts_test_template_lifecycle_transition_payloads(self) -> None:
        captured: list[tuple[str, str, dict[str, object]]] = []

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            body = json.loads(request.data.decode("utf-8"))
            captured.append((request.get_method(), request.full_url, body))
            status = "under_review" if "submit-for-review" in request.full_url else "approved"
            return _FakeResponse(
                {
                    "operation_id": body["operation_id"],
                    "replayed": False,
                    "revision": {
                        "revision_id": "TT-PY-001-rev-0001",
                        "status": status,
                    },
                }
            )

        client = LocalAgentClient("http://127.0.0.1:8765")
        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            submitted = client.submit_test_template_revision_for_review(
                template_id="TT-PY-001",
                revision_id="TT-PY-001-rev-0001",
                actor="method.author",
                reason="ready for review",
                operation_id="op-submit",
            )
            approved = client.approve_test_template_revision(
                template_id="TT-PY-001",
                revision_id="TT-PY-001-rev-0001",
                actor="technical.reviewer",
                reason="review accepted",
                operation_id="op-approve",
            )

        self.assertEqual(submitted["revision"]["status"], "under_review")
        self.assertEqual(approved["revision"]["status"], "approved")
        self.assertEqual(
            captured,
            [
                (
                    "POST",
                    "http://127.0.0.1:8765/api/v1/test-templates/TT-PY-001/revisions/TT-PY-001-rev-0001/transitions/submit-for-review",
                    {
                        "actor": "method.author",
                        "reason": "ready for review",
                        "operation_id": "op-submit",
                    },
                ),
                (
                    "POST",
                    "http://127.0.0.1:8765/api/v1/test-templates/TT-PY-001/revisions/TT-PY-001-rev-0001/transitions/approve",
                    {
                        "actor": "technical.reviewer",
                        "reason": "review accepted",
                        "operation_id": "op-approve",
                    },
                ),
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
