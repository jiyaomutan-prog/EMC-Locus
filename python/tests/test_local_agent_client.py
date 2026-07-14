from __future__ import annotations

import base64
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


def _scaling_profile_definition() -> dict[str, object]:
    return {
        "definition_schema_version": "emc-locus.scaling-profile-definition.v1",
        "scaling_profile_id": "SCL-PY-CURRENT-10MV-A",
        "label": "Demo current probe 10 mV/A",
        "input_quantity": "voltage",
        "input_unit": "V",
        "output_quantity": "current",
        "output_unit": "A",
        "scaling_kind": "linear",
        "parameters": {"scale": 100.0, "offset": 0.0},
        "validity_domain": {},
        "source_reference": "demo:current-probe-10mv-a",
        "metadata": {},
    }


def _engineering_curve_definition() -> dict[str, object]:
    return {
        "definition_schema_version": "emc-locus.engineering-curve-definition.v1",
        "curve_id": "CURVE-PY-CABLE-LOSS",
        "curve_type": "cable_loss",
        "label": "Demo cable loss",
        "independent_axes": [
            {"axis": "frequency", "quantity": "frequency", "unit": "Hz"}
        ],
        "dependent_values": [
            {
                "value_id": "correction_db",
                "quantity": "dimensionless",
                "unit": "dB",
            }
        ],
        "units": {"frequency": "Hz", "correction_db": "dB"},
        "points": [
            {
                "axis_values": {"frequency": 10_000_000.0},
                "values": {"correction_db": 0.35},
            },
            {
                "axis_values": {"frequency": 100_000_000.0},
                "values": {"correction_db": 1.25},
            },
            {
                "axis_values": {"frequency": 1_000_000_000.0},
                "values": {"correction_db": 3.8},
            },
        ],
        "interpolation": "log_x_linear_y",
        "extrapolation_policy": "clamp",
        "validity_domain": {},
        "conditions": {},
        "source_document_reference": "demo:cable-certificate",
        "source_checksum": "sha256:" + "c" * 64,
        "metadata": {},
    }


def _daq_channel_profile_definition() -> dict[str, object]:
    return {
        "definition_schema_version": "emc-locus.daq-channel-profile-definition.v1",
        "daq_channel_profile_id": "DAQ-PY-AI-10V",
        "label": "Demo DAQ AI +/-10 V",
        "channel_kind": "analog_input",
        "signal_domain": "analog_voltage",
        "input_quantity": "voltage",
        "input_unit": "V",
        "supported_ranges": [{"minimum": -10.0, "maximum": 10.0, "unit": "V"}],
        "resolution_bits": 16,
        "max_sampling_rate": 1_000_000.0,
        "min_sampling_rate": 1.0,
        "coupling_modes": ["dc", "ac"],
        "input_modes": ["single_ended", "differential", "iepe"],
        "anti_alias_filter": "available",
        "excitation_capabilities": [
            {"excitation_kind": "iepe", "nominal_value": 4.0, "unit": "mA"}
        ],
        "iepe_support": True,
        "synchronization": "shared_clock",
        "triggering": "digital_trigger",
        "metadata": {},
    }


def _sensor_definition() -> dict[str, object]:
    return {
        "definition_schema_version": "emc-locus.sensor-definition.v1",
        "sensor_definition_id": "SNS-PY-CURRENT-PROBE",
        "manufacturer": "Demo",
        "model_name": "Current Probe 10mV/A",
        "variant": "python-client",
        "sensor_family": "current_probe",
        "physical_input_quantity": "current",
        "engineering_output_quantity": "current",
        "engineering_output_unit": "A",
        "electrical_output_quantity": "voltage",
        "electrical_output_unit": "V",
        "signal_domain": "analog_voltage",
        "technology_tags": ["voltage_input"],
        "required_excitation": {"excitation_kind": "none", "external_allowed": False},
        "input_mode_requirement": "differential",
        "nominal_range": {"minimum": -100.0, "maximum": 100.0, "unit": "A"},
        "safe_range": {"minimum": -200.0, "maximum": 200.0, "unit": "A"},
        "orientation_axes": [],
        "settling_time_ms": 1.0,
        "frequency_range": {"minimum_hz": 10.0, "maximum_hz": 100_000_000.0},
        "scaling_profile_refs": [
            {
                "entity_id": "SCL-PY-CURRENT-10MV-A",
                "revision_id": "SCL-PY-CURRENT-10MV-A-rev-0001",
                "require_approved": True,
            }
        ],
        "correction_curve_refs": [],
        "metadata": {},
    }


def _acquisition_channel_recipe_definition() -> dict[str, object]:
    return {
        "definition_schema_version": "emc-locus.acquisition-channel-recipe-definition.v1",
        "recipe_id": "REC-PY-CURRENT-A",
        "label": "current_A logical channel",
        "output_channel_name": "current_A",
        "output_quantity": "current",
        "output_unit": "A",
        "daq_channel_profile_ref": {
            "entity_id": "DAQ-PY-AI-10V",
            "revision_id": "DAQ-PY-AI-10V-rev-0001",
            "require_approved": True,
        },
        "sensor_definition_ref": {
            "entity_id": "SNS-PY-CURRENT-PROBE",
            "revision_id": "SNS-PY-CURRENT-PROBE-rev-0001",
            "require_approved": True,
        },
        "scaling_profile_ref": {
            "entity_id": "SCL-PY-CURRENT-10MV-A",
            "revision_id": "SCL-PY-CURRENT-10MV-A-rev-0001",
            "require_approved": True,
        },
        "correction_curve_refs": [],
        "sample_rate": 1_000_000.0,
        "range": {"minimum": -10.0, "maximum": 10.0, "unit": "V"},
        "coupling": "dc",
        "input_mode": "differential",
        "excitation": {"excitation_kind": "none", "external_allowed": False},
        "filtering": "anti_alias_on",
        "triggering": "software",
        "validation_rules": [],
        "metadata": {},
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
            "http://127.0.0.1:8765/api/v1/equipment-models?manufacturer=Demo&equipment_class=controllable_instrument&category_code=power_meter&functional_role=measurement_instrument&signal_domain=rf&technology_tag=rf_50_ohm&status=approved&q=power": {
                "equipment_models": []
            },
            "http://127.0.0.1:8765/api/v1/equipment/registries": {
                "functional_roles": [{"code": "measurement_instrument"}],
                "signal_domains": [{"code": "rf"}],
                "port_directionalities": [],
                "flow_roles": [],
                "technology_tags": [],
            },
            "http://127.0.0.1:8765/api/v1/equipment/categories": {
                "categories": [{"category_id": "rf_equipment"}]
            },
            "http://127.0.0.1:8765/api/v1/equipment/categories/tree": {
                "categories": [{"category_id": "rf_equipment", "children": []}]
            },
            "http://127.0.0.1:8765/api/v1/equipment/field-definitions?scope=equipment_model": {
                "field_definitions": [{"field_id": "field_manufacturer"}]
            },
            "http://127.0.0.1:8765/api/v1/equipment/categories/rf_cable/field-rules": {
                "rules": [{"field_id": "field_manufacturer"}]
            },
            "http://127.0.0.1:8765/api/v1/equipment/categories/rf_cable/effective-template": {
                "effective_template": {"category": {"category_id": "rf_cable"}, "fields": []}
            },
            "http://127.0.0.1:8765/api/v1/equipment/classification-presets": {
                "presets": [{"preset_id": "rf_power_meter"}]
            },
            "http://127.0.0.1:8765/api/v1/equipment/classification-presets/rf_power_meter": {
                "preset": {"preset_id": "rf_power_meter"}
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
                    functional_role="measurement_instrument",
                    signal_domain="rf",
                    technology_tag="rf_50_ohm",
                    status="approved",
                    q="power",
                )["equipment_models"],
                [],
            )
            self.assertEqual(client.equipment_registries()["functional_roles"][0]["code"], "measurement_instrument")
            self.assertEqual(client.list_equipment_categories()["categories"][0]["category_id"], "rf_equipment")
            self.assertEqual(client.equipment_category_tree()["categories"][0]["category_id"], "rf_equipment")
            self.assertEqual(
                client.list_equipment_field_definitions(scope="equipment_model")["field_definitions"][0]["field_id"],
                "field_manufacturer",
            )
            self.assertEqual(
                client.list_equipment_category_field_rules("rf_cable")["rules"][0]["field_id"],
                "field_manufacturer",
            )
            self.assertEqual(
                client.equipment_effective_template("rf_cable")["effective_template"]["category"]["category_id"],
                "rf_cable",
            )
            self.assertEqual(
                client.list_equipment_classification_presets()["presets"][0]["preset_id"],
                "rf_power_meter",
            )
            self.assertEqual(
                client.get_equipment_classification_preset("rf_power_meter")["preset"]["preset_id"],
                "rf_power_meter",
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

        self.assertEqual(len(captured), 19)

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
            client.create_equipment_model_from_preset(
                preset_id="rf_power_meter",
                equipment_model_id="EQM-PY-PRESET",
                manufacturer="Demo",
                model_name="Preset Power Meter",
                actor="catalog.author",
                reason="create model from preset",
                operation_id="op-model-from-preset",
            )
            client.create_equipment_model_from_category_template(
                category_id="rf_cable",
                equipment_model_id="EQM-PY-TEMPLATE",
                field_values={
                    "manufacturer": "Demo",
                    "model_name": "Template cable",
                    "variant": "1 m",
                },
                actor="catalog.author",
                reason="create model from template",
                operation_id="op-model-from-template",
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
        self.assertEqual(captured[2][1], "http://127.0.0.1:8765/api/v1/equipment-models/from-preset")
        self.assertEqual(captured[2][2]["preset_id"], "rf_power_meter")
        self.assertFalse(captured[2][2]["is_demo"])
        self.assertEqual(captured[3][1], "http://127.0.0.1:8765/api/v1/equipment-models/from-category-template")
        self.assertEqual(captured[3][2]["category_id"], "rf_cable")
        self.assertFalse(captured[3][2]["is_demo"])
        self.assertEqual(captured[4][2]["expected_definition_checksum"], "sha256:" + "a" * 64)
        self.assertEqual(captured[5][2]["source_revision_id"], "EQM-PY-POWER-rev-0001")
        self.assertEqual(captured[6][2]["new_equipment_model_id"], "EQM-PY-POWER-CLONE")
        self.assertEqual(captured[7][2]["operation_id"], "op-model-submit")
        self.assertEqual(captured[8][2]["operation_id"], "op-model-approve")

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

    def test_station_setup_client_uses_business_routes_and_cas(self) -> None:
        captured: list[tuple[str, str, dict[str, object] | None]] = []

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            body = json.loads(request.data.decode("utf-8")) if request.data else None
            captured.append((request.get_method(), request.full_url, body))
            return _FakeResponse(
                {
                    "operation": "station_setup_operation",
                    "operation_id": body.get("operation_id", "read") if body else "read",
                    "replayed": False,
                    "station_setup": {
                        "identity": {"setup_id": "SETUP-PY-001"},
                        "latest_revision": {"revision_id": "SETUP-PY-001-rev-0001"},
                    },
                }
            )

        definition = {
            "definition_schema_version": "emc-locus.station-measurement-setup-definition.v1",
            "setup_id": "SETUP-PY-001",
            "label": "Chaîne RF",
            "station_label": "Salle CEM 1",
            "planned_use_on": "2026-07-15",
            "execution_mode": "accredited",
            "asset_bindings": [],
            "connections": [],
            "correction_selections": [],
        }
        client = LocalAgentClient("http://127.0.0.1:8765")
        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            client.create_station_setup(
                setup_id="SETUP-PY-001",
                label="Chaîne RF",
                station_label="Salle CEM 1",
                planned_use_on="2026-07-15",
                execution_mode="accredited",
                actor="operator.one",
                reason="préparer la mesure",
                operation_id="op-station-create",
            )
            client.replace_station_setup_draft(
                setup_id="SETUP-PY-001",
                revision_id="SETUP-PY-001-rev-0001",
                expected_definition_checksum="sha256:" + "a" * 64,
                definition=definition,
                actor="operator.one",
                reason="ajouter les matériels",
                operation_id="op-station-save",
            )
            client.assess_station_setup("SETUP-PY-001", "SETUP-PY-001-rev-0001")
            client.mark_station_setup_ready(
                setup_id="SETUP-PY-001",
                revision_id="SETUP-PY-001-rev-0001",
                expected_definition_checksum="sha256:" + "b" * 64,
                actor="operator.one",
                reason="montage vérifié",
                operation_id="op-station-ready",
            )
            client.derive_station_setup_draft(
                setup_id="SETUP-PY-001",
                source_revision_id="SETUP-PY-001-rev-0001",
                actor="operator.one",
                reason="adapter le montage",
                operation_id="op-station-derive",
            )

        self.assertEqual(captured[0][0:2], ("POST", "http://127.0.0.1:8765/api/v1/station-setups"))
        self.assertEqual(captured[0][2]["station_label"], "Salle CEM 1")
        self.assertEqual(captured[1][0], "PUT")
        self.assertTrue(captured[1][1].endswith("/SETUP-PY-001-rev-0001/definition"))
        self.assertEqual(captured[1][2]["expected_definition_checksum"], "sha256:" + "a" * 64)
        self.assertEqual(captured[2][0], "GET")
        self.assertTrue(captured[2][1].endswith("/SETUP-PY-001-rev-0001/readiness"))
        self.assertTrue(captured[3][1].endswith("/transitions/ready"))
        self.assertEqual(captured[4][2]["source_revision_id"], "SETUP-PY-001-rev-0001")

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

    def test_posts_measurement_engineering_lifecycle_payloads(self) -> None:
        captured: list[tuple[str, str, dict[str, object]]] = []

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            body = json.loads(request.data.decode("utf-8"))
            captured.append((request.get_method(), request.full_url, body))
            if request.full_url.endswith("/validate"):
                return _FakeResponse(
                    {
                        "valid": True,
                        "issues": [],
                        "definition_checksum": "sha256:" + "d" * 64,
                    }
                )
            status = "draft"
            if "submit-for-review" in request.full_url:
                status = "under_review"
            if request.full_url.endswith("/transitions/approve"):
                status = "approved"
            return _FakeResponse(
                {
                    "operation_id": body.get("operation_id", "op"),
                    "replayed": False,
                    "revision": {"revision_id": "REV-PY-0001", "status": status},
                }
            )

        client = LocalAgentClient("http://127.0.0.1:8765")
        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            self.assertTrue(client.validate_sensor_definition(_sensor_definition())["valid"])
            client.create_sensor_definition(
                entity_id="SNS-PY-CURRENT-PROBE",
                definition=_sensor_definition(),
                actor="measurement.author",
                reason="create current probe",
                operation_id="op-sensor-create",
            )
            client.submit_sensor_definition_revision_for_review(
                entity_id="SNS-PY-CURRENT-PROBE",
                revision_id="SNS-PY-CURRENT-PROBE-rev-0001",
                actor="measurement.author",
                reason="ready",
                operation_id="op-sensor-submit",
            )
            client.approve_sensor_definition_revision(
                entity_id="SNS-PY-CURRENT-PROBE",
                revision_id="SNS-PY-CURRENT-PROBE-rev-0001",
                actor="measurement.reviewer",
                reason="accepted",
                operation_id="op-sensor-approve",
            )

            self.assertTrue(client.validate_scaling_profile(_scaling_profile_definition())["valid"])
            client.create_scaling_profile(
                entity_id="SCL-PY-CURRENT-10MV-A",
                definition=_scaling_profile_definition(),
                actor="measurement.author",
                reason="create scaling",
                operation_id="op-scaling-create",
            )
            client.replace_scaling_profile_revision(
                entity_id="SCL-PY-CURRENT-10MV-A",
                revision_id="SCL-PY-CURRENT-10MV-A-rev-0001",
                expected_definition_checksum="sha256:" + "d" * 64,
                definition=_scaling_profile_definition(),
                actor="measurement.author",
                reason="save draft",
                operation_id="op-scaling-save",
            )
            client.create_scaling_profile_revision(
                entity_id="SCL-PY-CURRENT-10MV-A",
                source_revision_id="SCL-PY-CURRENT-10MV-A-rev-0001",
                actor="measurement.author",
                reason="derive next revision",
                operation_id="op-scaling-derive",
            )
            client.clone_scaling_profile(
                source_entity_id="SCL-PY-CURRENT-10MV-A",
                source_revision_id="SCL-PY-CURRENT-10MV-A-rev-0001",
                new_entity_id="SCL-PY-CURRENT-10MV-A-CLONE",
                actor="measurement.author",
                reason="clone scaling",
                operation_id="op-scaling-clone",
            )
            client.submit_scaling_profile_revision_for_review(
                entity_id="SCL-PY-CURRENT-10MV-A",
                revision_id="SCL-PY-CURRENT-10MV-A-rev-0001",
                actor="measurement.author",
                reason="ready",
                operation_id="op-scaling-submit",
            )
            client.approve_scaling_profile_revision(
                entity_id="SCL-PY-CURRENT-10MV-A",
                revision_id="SCL-PY-CURRENT-10MV-A-rev-0001",
                actor="measurement.reviewer",
                reason="accepted",
                operation_id="op-scaling-approve",
            )

            self.assertTrue(client.validate_engineering_curve(_engineering_curve_definition())["valid"])
            client.create_engineering_curve(
                entity_id="CURVE-PY-CABLE-LOSS",
                definition=_engineering_curve_definition(),
                actor="measurement.author",
                reason="create curve",
                operation_id="op-curve-create",
            )
            client.submit_engineering_curve_revision_for_review(
                entity_id="CURVE-PY-CABLE-LOSS",
                revision_id="CURVE-PY-CABLE-LOSS-rev-0001",
                actor="measurement.author",
                reason="ready",
                operation_id="op-curve-submit",
            )
            client.approve_engineering_curve_revision(
                entity_id="CURVE-PY-CABLE-LOSS",
                revision_id="CURVE-PY-CABLE-LOSS-rev-0001",
                actor="measurement.reviewer",
                reason="accepted",
                operation_id="op-curve-approve",
            )

            self.assertTrue(client.validate_daq_channel_profile(_daq_channel_profile_definition())["valid"])
            client.create_daq_channel_profile(
                entity_id="DAQ-PY-AI-10V",
                definition=_daq_channel_profile_definition(),
                actor="measurement.author",
                reason="create daq profile",
                operation_id="op-daq-create",
            )
            client.submit_daq_channel_profile_revision_for_review(
                entity_id="DAQ-PY-AI-10V",
                revision_id="DAQ-PY-AI-10V-rev-0001",
                actor="measurement.author",
                reason="ready",
                operation_id="op-daq-submit",
            )
            client.approve_daq_channel_profile_revision(
                entity_id="DAQ-PY-AI-10V",
                revision_id="DAQ-PY-AI-10V-rev-0001",
                actor="measurement.reviewer",
                reason="accepted",
                operation_id="op-daq-approve",
            )

            self.assertTrue(
                client.validate_acquisition_channel_recipe(
                    _acquisition_channel_recipe_definition()
                )["valid"]
            )
            client.create_acquisition_channel_recipe(
                entity_id="REC-PY-CURRENT-A",
                definition=_acquisition_channel_recipe_definition(),
                actor="measurement.author",
                reason="create recipe",
                operation_id="op-recipe-create",
            )
            client.submit_acquisition_channel_recipe_revision_for_review(
                entity_id="REC-PY-CURRENT-A",
                revision_id="REC-PY-CURRENT-A-rev-0001",
                actor="measurement.author",
                reason="ready",
                operation_id="op-recipe-submit",
            )
            client.approve_acquisition_channel_recipe_revision(
                entity_id="REC-PY-CURRENT-A",
                revision_id="REC-PY-CURRENT-A-rev-0001",
                actor="measurement.reviewer",
                reason="accepted",
                operation_id="op-recipe-approve",
            )

        urls = [item[1] for item in captured]
        self.assertIn(
            "http://127.0.0.1:8765/api/v1/sensor-definition-definitions/validate",
            urls,
        )
        self.assertIn(
            "http://127.0.0.1:8765/api/v1/scaling-profiles/SCL-PY-CURRENT-10MV-A/revisions/SCL-PY-CURRENT-10MV-A-rev-0001/definition",
            urls,
        )
        self.assertIn(
            "http://127.0.0.1:8765/api/v1/scaling-profiles/SCL-PY-CURRENT-10MV-A/clone",
            urls,
        )
        self.assertIn(
            "http://127.0.0.1:8765/api/v1/engineering-curve-definitions/validate",
            urls,
        )
        self.assertIn(
            "http://127.0.0.1:8765/api/v1/daq-channel-profile-definitions/validate",
            urls,
        )
        self.assertIn(
            "http://127.0.0.1:8765/api/v1/acquisition-channel-recipe-definitions/validate",
            urls,
        )
        scaling_save = next(item for item in captured if item[2].get("operation_id") == "op-scaling-save")
        self.assertEqual(scaling_save[0], "PUT")
        self.assertEqual(scaling_save[2]["expected_definition_checksum"], "sha256:" + "d" * 64)
        scaling_clone = next(item for item in captured if item[2].get("operation_id") == "op-scaling-clone")
        self.assertEqual(scaling_clone[2]["new_entity_id"], "SCL-PY-CURRENT-10MV-A-CLONE")

    def test_reads_measurement_engineering_routes_and_evaluates_curve(self) -> None:
        captured: list[tuple[str, str, dict[str, object] | None]] = []

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            body = None
            if request.data:
                body = json.loads(request.data.decode("utf-8"))
            captured.append((request.get_method(), request.full_url, body))
            if request.full_url.endswith("/evaluate"):
                return _FakeResponse(
                    {
                        "evaluation": {
                            "values": {"correction_db": 1.25},
                            "axis_values": {"frequency": 100_000_000.0},
                            "interpolation": "log_x_linear_y",
                            "extrapolated": False,
                            "source_revision_id": "CURVE-PY-CABLE-LOSS-rev-0001",
                            "source_checksum": "sha256:" + "e" * 64,
                        }
                    }
                )
            if request.full_url.endswith("/revisions"):
                return _FakeResponse({"revisions": []})
            if request.full_url.endswith("/audit-events"):
                return _FakeResponse({"audit_events": []})
            return _FakeResponse({"ok": True})

        client = LocalAgentClient("http://127.0.0.1:8765")
        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            client.list_sensor_definitions()
            client.get_sensor_definition("SNS-PY-CURRENT-PROBE")
            client.list_sensor_definition_revisions("SNS-PY-CURRENT-PROBE")
            client.get_sensor_definition_revision(
                "SNS-PY-CURRENT-PROBE",
                "SNS-PY-CURRENT-PROBE-rev-0001",
            )
            client.sensor_definition_audit_events("SNS-PY-CURRENT-PROBE")

            client.list_scaling_profiles()
            client.get_scaling_profile("SCL-PY-CURRENT-10MV-A")
            client.list_scaling_profile_revisions("SCL-PY-CURRENT-10MV-A")
            client.get_scaling_profile_revision(
                "SCL-PY-CURRENT-10MV-A",
                "SCL-PY-CURRENT-10MV-A-rev-0001",
            )
            client.scaling_profile_audit_events("SCL-PY-CURRENT-10MV-A")

            client.list_engineering_curves()
            client.get_engineering_curve("CURVE-PY-CABLE-LOSS")
            client.list_engineering_curve_revisions("CURVE-PY-CABLE-LOSS")
            client.get_engineering_curve_revision(
                "CURVE-PY-CABLE-LOSS",
                "CURVE-PY-CABLE-LOSS-rev-0001",
            )
            curve_evaluation = client.evaluate_engineering_curve(
                curve_id="CURVE-PY-CABLE-LOSS",
                revision_id="CURVE-PY-CABLE-LOSS-rev-0001",
                axis_values={"frequency": 100_000_000.0},
            )
            client.engineering_curve_audit_events("CURVE-PY-CABLE-LOSS")

            client.list_daq_channel_profiles()
            client.get_daq_channel_profile("DAQ-PY-AI-10V")
            client.list_daq_channel_profile_revisions("DAQ-PY-AI-10V")
            client.get_daq_channel_profile_revision(
                "DAQ-PY-AI-10V",
                "DAQ-PY-AI-10V-rev-0001",
            )
            client.daq_channel_profile_audit_events("DAQ-PY-AI-10V")

            client.list_acquisition_channel_recipes()
            client.get_acquisition_channel_recipe("REC-PY-CURRENT-A")
            client.list_acquisition_channel_recipe_revisions("REC-PY-CURRENT-A")
            client.get_acquisition_channel_recipe_revision(
                "REC-PY-CURRENT-A",
                "REC-PY-CURRENT-A-rev-0001",
            )
            client.acquisition_channel_recipe_audit_events("REC-PY-CURRENT-A")

        self.assertEqual(curve_evaluation["evaluation"]["interpolation"], "log_x_linear_y")
        self.assertIn(
            (
                "POST",
                "http://127.0.0.1:8765/api/v1/engineering-curves/CURVE-PY-CABLE-LOSS/revisions/CURVE-PY-CABLE-LOSS-rev-0001/evaluate",
                {"axis_values": {"frequency": 100_000_000.0}},
            ),
            captured,
        )
        self.assertIn(
            (
                "GET",
                "http://127.0.0.1:8765/api/v1/acquisition-channel-recipes/REC-PY-CURRENT-A/audit-events",
                None,
            ),
            captured,
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
            "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001/characterizations": {
                "characterizations": []
            },
            "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001/characterizations/CHAR-001": {
                "characterization": {"characterization_id": "CHAR-001"}
            },
            "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001/characterizations/CHAR-001/audit-events": {
                "audit_events": []
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
                client.list_asset_characterizations("SA-READ-001")["characterizations"],
                [],
            )
            self.assertEqual(
                client.get_asset_characterization("SA-READ-001", "CHAR-001")[
                    "characterization"
                ]["characterization_id"],
                "CHAR-001",
            )
            self.assertEqual(
                client.asset_characterization_audit_events("SA-READ-001", "CHAR-001")[
                    "audit_events"
                ],
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
                    "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001/characterizations",
                    None,
                ),
                (
                    "GET",
                    "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001/characterizations/CHAR-001",
                    None,
                ),
                (
                    "GET",
                    "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-READ-001/characterizations/CHAR-001/audit-events",
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

    def test_records_asset_characterization_and_uploads_metrology_evidence(self) -> None:
        captured: list[tuple[str, str, dict[str, object]]] = []

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            body = json.loads(request.data.decode("utf-8"))
            captured.append((request.get_method(), request.full_url, body))
            if request.full_url.endswith("/api/v1/metrology/files"):
                return _FakeResponse({"file": {"original_filename": body["original_filename"]}})
            return _FakeResponse(
                {"characterization": {"characterization_id": body["characterization_id"]}}
            )

        definition = {
            "definition_schema_version": "emc-locus.asset-characterization-definition.v1",
            "characterization_id": "CHAR-PY-001",
            "asset_id": "SA-PY-001",
            "label": "Measured cable loss",
            "correction": {
                "correction_kind": "frequency_response",
                "correction": {"curve_id": "CHAR-PY-001"},
            },
        }
        client = LocalAgentClient("http://127.0.0.1:8765")
        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            uploaded = client.upload_metrology_file(
                original_filename="certificate.pdf",
                mime_type="application/pdf",
                content=b"certificate-evidence",
            )
            recorded = client.record_asset_characterization(
                asset_id="SA-PY-001",
                characterization_id="CHAR-PY-001",
                performed_on="2026-07-14",
                valid_until="2027-07-14",
                provider="Internal laboratory",
                method_reference="MET-RF-CABLE-001",
                definition=definition,
                recorded_by="metrology.operator",
                actor="metrology.operator",
                reason="record measured loss",
                certificate_reference="CERT-PY-001",
                document_manifest=uploaded["file"],
                operation_id="op-char-py-001",
            )

        self.assertEqual(recorded["characterization"]["characterization_id"], "CHAR-PY-001")
        self.assertEqual(captured[0][0:2], (
            "POST",
            "http://127.0.0.1:8765/api/v1/metrology/files",
        ))
        self.assertEqual(
            base64.b64decode(str(captured[0][2]["content_base64"])),
            b"certificate-evidence",
        )
        self.assertEqual(captured[1][0:2], (
            "POST",
            "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-PY-001/characterizations",
        ))
        self.assertEqual(captured[1][2]["definition"], definition)
        self.assertEqual(captured[1][2]["document_manifest"], uploaded["file"])
        self.assertEqual(captured[1][2]["operation_id"], "op-char-py-001")

    def test_manages_serial_specific_correction_workflow(self) -> None:
        captured: list[tuple[str, str, dict[str, object] | None]] = []

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            body = None
            if request.data is not None:
                body = json.loads(request.data.decode("utf-8"))
            captured.append((request.get_method(), request.full_url, body))
            return _FakeResponse({"ok": True})

        client = LocalAgentClient("http://127.0.0.1:8765")
        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            client.list_asset_correction_assignments("SA-CABLE-001")
            client.list_asset_correction_review_queue()
            client.create_asset_correction_assignment(
                asset_id="SA-CABLE-001",
                assignment_id="CORR-CABLE-001",
                signal_path_id="RF_THROUGH",
                requirement_id="rf_loss",
                source_event_id="CHAR-CABLE-001",
                actor="metrology.operator",
                reason="bind measured loss",
                conditions={"polarization": "horizontal"},
                operation_id="op-correction-create",
            )
            client.transition_asset_correction_assignment(
                asset_id="SA-CABLE-001",
                assignment_id="CORR-CABLE-001",
                transition="approve-and-activate",
                expected_revision="rev-review",
                actor="metrology.reviewer",
                reason="review accepted",
                operation_id="op-correction-approve",
            )
            client.resolve_material_corrections(
                asset_id="SA-CABLE-001",
                intended_use_on="2026-08-01",
                execution_context="accredited",
                conditions={"polarization": "horizontal"},
            )

        self.assertEqual(
            captured[0],
            (
                "GET",
                "http://127.0.0.1:8765/api/v1/metrology/instruments/SA-CABLE-001/corrections",
                None,
            ),
        )
        self.assertEqual(
            captured[1][1],
            "http://127.0.0.1:8765/api/v1/metrology/corrections/review-queue",
        )
        self.assertEqual(captured[2][2]["requirement_id"], "rf_loss")
        self.assertEqual(captured[2][2]["conditions"]["polarization"], "horizontal")
        self.assertEqual(captured[3][2]["expected_revision"], "rev-review")
        self.assertTrue(captured[3][1].endswith("/transitions/approve-and-activate"))
        self.assertEqual(captured[4][2]["execution_context"], "accredited")

    def test_registers_physical_asset_with_pinned_equipment_model(self) -> None:
        captured: dict[str, object] = {}

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            captured.update(json.loads(request.data.decode("utf-8")))
            return _FakeResponse({"instrument": {"asset_id": "SA-PY-MODEL-001"}})

        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            LocalAgentClient("http://127.0.0.1:8765").register_metrology_instrument(
                asset_id="SA-PY-MODEL-001",
                family="RfCable",
                category_code="rf_cable",
                manufacturer="Demo",
                model="Cable 1 GHz",
                serial_number="CAB-001",
                calibration_requirement="not_required",
                equipment_model_id="EQM-PY-CABLE",
                equipment_model_revision_id="EQM-PY-CABLE-rev-0001",
                equipment_model_checksum="sha256:" + "a" * 64,
                actor="metrology.admin",
                reason="register physical cable",
                operation_id="op-register-pinned-model",
            )

        self.assertEqual(captured["equipment_model_id"], "EQM-PY-CABLE")
        self.assertEqual(
            captured["equipment_model_revision_id"],
            "EQM-PY-CABLE-rev-0001",
        )
        self.assertEqual(
            captured["equipment_model_checksum"],
            "sha256:" + "a" * 64,
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
