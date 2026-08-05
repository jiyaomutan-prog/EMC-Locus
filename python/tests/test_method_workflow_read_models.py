from __future__ import annotations

import json
import unittest
from unittest.mock import patch

from emc_locus import (
    LocalAgentClient,
    build_execution_plan_table,
    build_method_workflow_tables,
    build_station_mapping_table,
)


class _Response:
    def __init__(self, payload: dict[str, object]) -> None:
        self.payload = payload

    def __enter__(self) -> "_Response":
        return self

    def __exit__(self, *args: object) -> None:
        return None

    def read(self) -> bytes:
        return json.dumps(self.payload).encode("utf-8")


class MethodWorkflowReadModelTests(unittest.TestCase):
    def test_python_client_reads_workflow_and_requests_agent_preview(self) -> None:
        captured: list[tuple[str, str, dict[str, object]]] = []

        def fake_urlopen(request, timeout: float):  # type: ignore[no-untyped-def]
            payload = json.loads(request.data.decode("utf-8")) if request.data else {}
            captured.append((request.get_method(), request.full_url, payload))
            return _Response({"preview": {"ordered_phases": []}})

        client = LocalAgentClient("http://127.0.0.1:8765")
        with patch("emc_locus.local_agent_client.urlopen", fake_urlopen):
            client.list_method_hierarchy()
            client.list_measurement_system_templates()
            client.get_measurement_system_template("SYS 001")
            client.list_regulation_profiles()
            client.get_regulation_profile("REG 001")
            client.get_execution_configuration("EXEC 001")
            client.preview_execution_plan(
                method_template_id="METHOD-001",
                method_revision_id="METHOD-001-rev-0002",
                method_definition={"definition_schema_version": "emc-locus.test-method-definition.v2"},
                system_definition={
                    "definition_schema_version": "emc-locus.measurement-system-template-definition.v1"
                },
                operation_id="op-preview-python",
            )

        self.assertEqual(
            [item[1] for item in captured[:6]],
            [
                "http://127.0.0.1:8765/api/v1/method-hierarchy",
                "http://127.0.0.1:8765/api/v1/measurement-system-templates",
                "http://127.0.0.1:8765/api/v1/measurement-system-templates/SYS%20001",
                "http://127.0.0.1:8765/api/v1/regulation-profiles",
                "http://127.0.0.1:8765/api/v1/regulation-profiles/REG%20001",
                "http://127.0.0.1:8765/api/v1/execution-configurations/EXEC%20001",
            ],
        )
        self.assertEqual(captured[-1][0:2], ("POST", "http://127.0.0.1:8765/api/v1/execution-plans/preview"))
        self.assertEqual(captured[-1][2]["operation_id"], "op-preview-python")
        self.assertEqual(captured[-1][2]["device_id"], "qt-console")

    def test_builds_operator_tables_without_reimplementing_validation(self) -> None:
        method = {
            "identity": {"template_id": "METHOD-001", "title": "Immunité conduite"},
            "active_draft_revision": {
                "revision_number": 2,
                "status": "draft",
                "definition_schema_version": "emc-locus.test-method-definition.v2",
                "definition": {"functional_roles": [{"role_id": "generator"}], "sub_ranges": []},
            },
        }
        system = {
            "identity": {"entity_id": "SYS-001", "label": "Chaîne conduite", "classification": "immunity"},
            "revisions": [{
                "revision_number": 1,
                "status": "validated",
                "definition": {"nodes": [{"node_id": "generator"}], "edges": [], "regulation_loops": []},
            }],
        }
        profile = {
            "identity": {"entity_id": "REG-001", "label": "Boucle puissance"},
            "revisions": [{
                "revision_number": 3,
                "status": "approved",
                "definition": {"control_mode": "closed_loop", "regulated_quantity": "Niveau", "regulated_unit": "dB"},
            }],
        }

        tables = build_method_workflow_tables(
            {"test_templates": [method]},
            {"definitions": [system]},
            {"definitions": [profile]},
        )

        self.assertEqual(tables[0].rows[0][2], "Workflow 0.22.2")
        self.assertEqual(tables[1].rows[0][5:8], ("1", "0", "0"))
        self.assertEqual(tables[2].rows[0][2], "Boucle fermée")

    def test_presents_agent_plan_and_exact_station_assignments(self) -> None:
        plan = build_execution_plan_table({
            "preview": {
                "ordered_phases": [{"label": "Balayage", "node_kind": "sweep", "depth": 1, "maximum_iterations": 120}],
                "blockers": [{"message": "Récepteur non affecté", "next_action": "Préparer le poste"}],
                "warnings": [],
            }
        })
        mapping = build_station_mapping_table({
            "definition": {
                "assignments": [{
                    "role_id": "receiver",
                    "requirement_id": "measurement-receiver",
                    "asset_id": "RX-001",
                    "equipment_model_revision_id": "MODEL-RX-rev-0002",
                }]
            }
        })

        self.assertEqual(plan.rows[0], ("Phase", "Balayage", "Sweep", "Niveau 1", "Maximum 120 itérations"))
        self.assertEqual(plan.rows[1][0], "Blocage")
        self.assertEqual(mapping.rows[0][2], "RX-001")


if __name__ == "__main__":
    unittest.main()
