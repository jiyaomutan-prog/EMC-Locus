from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from emc_locus import (
    MetrologyRepository,
    ProjectRepository,
    TestDefinitionRepository,
    build_console_bootstrap_from_repositories,
    build_console_view_model,
    build_operator_form_specs,
)


QT_CONSOLE_PATH = Path(__file__).resolve().parents[2] / "apps" / "qt-console" / "main.py"


def load_qt_console_module():
    spec = importlib.util.spec_from_file_location("emc_locus_qt_console", QT_CONSOLE_PATH)
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class QtConsoleTests(unittest.TestCase):
    def test_loads_strict_json_bootstrap_without_requiring_pyside(self) -> None:
        module = load_qt_console_module()
        with tempfile.TemporaryDirectory() as temporary_directory:
            bootstrap = Path(temporary_directory) / "bootstrap.json"
            bootstrap.write_text(
                '{"projects": [{"code": "CEM-QT-001"}]}\n',
                encoding="utf-8",
            )

            payload = module.load_bootstrap_json(bootstrap)

        self.assertEqual(payload["projects"][0]["code"], "CEM-QT-001")

    def test_rejects_legacy_javascript_bootstrap_payload(self) -> None:
        module = load_qt_console_module()
        with tempfile.TemporaryDirectory() as temporary_directory:
            bootstrap = Path(temporary_directory) / "bootstrap.js"
            bootstrap.write_text(
                'window.EMC_LOCUS_BOOTSTRAP = {"projects": []};',
                encoding="utf-8",
            )

            with self.assertRaises(ValueError):
                module.load_bootstrap_json(bootstrap)

    def test_builds_explicit_console_table_models(self) -> None:
        model = build_console_view_model(
            {
                "projects": [
                    {
                        "code": "CEM-QT-001",
                        "customer": "Rail Motion",
                        "stage": "Measuring",
                        "mode": "Accredite",
                        "blocker": "Aucun",
                        "run": "RUN-QT-001",
                        "method": "Inrush",
                    }
                ],
                "datasets": [["RUN-QT-001", "raw_signal", "raw.opendata", "sha256:raw", "Immutable"]],
                "instruments": [
                    [
                        "DAQ-001",
                        "DAQ",
                        "Usable",
                        "Available",
                        "CERT-1",
                        "2027-01-01",
                        "warn",
                        "DAQ chassis and modules",
                        "channels=8",
                        "openDAQ",
                        "Reference DAQ",
                        "DAQ001",
                        "ODAQ-8",
                        "2026-03-18",
                        "12",
                        "2",
                    ]
                ],
                "instrument_documents": [
                    ["DAQ-001", "script", "setup.py", "scripts/setup.py", "A", "setup"]
                ],
                "metrology_readiness": [
                    [
                        "bloque",
                        "DAQ-001",
                        "usable",
                        "due_soon",
                        "non",
                        "calibration_due_soon",
                        "",
                        "calibration_due_soon",
                    ]
                ],
                "schedule": [
                    [
                        "PLAN-QT-001",
                        "CEM-QT-001",
                        "Inrush",
                        "emission_transient_time_domain",
                        "2026-07-01T09:00",
                        "2026-07-01T12:00",
                        "operator.one",
                        "Lab A",
                        "planned",
                    ]
                ],
                "contract_review_items": [
                    [
                        "CEM-QT-001",
                        "requirements_reviewed",
                        "yes",
                        "quality.lead",
                        "Accepted",
                    ]
                ],
                "runtime": [
                    [
                        "DAQ-001",
                        "simulated",
                        "SIM::DAQ-001",
                        "OK",
                        "RUN-QT-001",
                        "7",
                        "READ? -> OK",
                        "2",
                    ],
                    [
                        "RX-001",
                        "tcp_ip",
                        "TCPIP::127.0.0.1::5025",
                        "Echec",
                        "RUN-QT-001",
                        "8",
                        "READ? -> timeout",
                        "3",
                    ],
                ],
                "updates": [["driver", "0.2.0", "Signed", "Available", "offline_bundle"]],
                "instrument_categories": [
                    ["daq_chassis", "data_monitoring", "DAQ chassis and modules", "required", "data_acquisition"]
                ],
                "test_categories": [
                    ["emission", "", "Emission", "active"],
                    ["emission_conducted", "emission", "Emission conduite", "active"],
                ],
            }
        )
        tables = {table.tab_label: table for table in model.tables}
        project_table = tables["Projets"]
        contract_table = tables["Revue contrat"]
        metrology_table = tables["Metrologie"]
        readiness_table = tables["Aptitude"]
        document_table = tables["Docs metro"]
        category_table = tables["Categories"]
        schedule_table = tables["Planning"]
        test_category_table = tables["Categories essais"]
        dataset_table = tables["Donnees"]
        runtime_table = tables["Runtime"]
        actions = {action.action_id: action for action in model.actions}
        metrics = {metric.label: metric for metric in model.metrics}

        self.assertEqual(project_table.columns[0:3], ("Code", "Client", "Etape"))
        self.assertEqual(project_table.rows[0][0:3], ("CEM-QT-001", "Rail Motion", "Measuring"))
        self.assertEqual(contract_table.rows[0][0:3], ("CEM-QT-001", "requirements_reviewed", "yes"))
        self.assertEqual(
            metrology_table.columns,
            (
                "Actif",
                "Famille",
                "Service",
                "Planning",
                "Certificat",
                "Validite",
                "Alerte",
                "Categorie",
                "Capacites",
                "Marque",
                "Modele",
                "Serie",
                "Part number",
                "Derniere cal",
                "Periodicite",
                "Docs",
            ),
        )
        self.assertEqual(
            metrology_table.rows[0][6:9],
            ("warn", "DAQ chassis and modules", "channels=8"),
        )
        self.assertEqual(
            metrology_table.rows[0][9:16],
            ("openDAQ", "Reference DAQ", "DAQ001", "ODAQ-8", "2026-03-18", "12", "2"),
        )
        self.assertEqual(document_table.rows[0][0:3], ("DAQ-001", "script", "setup.py"))
        self.assertEqual(readiness_table.rows[0][0:4], ("bloque", "DAQ-001", "usable", "due_soon"))
        self.assertEqual(schedule_table.rows[0][0:4], ("PLAN-QT-001", "CEM-QT-001", "Inrush", "emission_transient_time_domain"))
        self.assertEqual(test_category_table.rows[1][0:3], ("emission_conducted", "emission", "Emission conduite"))
        self.assertEqual(
            runtime_table.columns,
            (
                "Instrument",
                "Transport",
                "Endpoint",
                "Etat",
                "Run",
                "Sequence",
                "Observation",
                "Tentatives",
            ),
        )
        self.assertEqual(runtime_table.rows[0][2], "SIM::DAQ-001")
        self.assertEqual(runtime_table.rows[0][4:8], ("RUN-QT-001", "7", "READ? -> OK", "2"))
        self.assertEqual(category_table.columns, ("Code", "Domaine", "Nom", "Calibration", "Profil"))
        self.assertEqual(category_table.rows[0][0], "daq_chassis")
        self.assertEqual(dataset_table.columns, ("Run", "Type", "Fichier", "Checksum", "Retention"))
        self.assertEqual(dataset_table.rows[0][1], "raw_signal")
        self.assertTrue(actions["advance_project"].enabled)
        self.assertTrue(actions["request_dataset_deletion"].enabled)
        self.assertTrue(actions["validate_update"].enabled)
        self.assertEqual(metrics["Projets actifs"].value, "1")
        self.assertEqual(metrics["Items revue"].value, "1")
        self.assertEqual(metrics["Alertes metrologie"].tone, "warn")
        self.assertEqual(metrics["Categories instruments"].value, "1")
        self.assertEqual(metrics["Docs materiel"].value, "1")
        self.assertEqual(metrics["Planning"].value, "1")
        self.assertEqual(metrics["Erreurs runtime"].value, "1")
        self.assertEqual(metrics["Erreurs runtime"].tone, "warn")
        self.assertEqual(metrics["Tentatives max"].value, "3")
        self.assertEqual(metrics["Tentatives max"].tone, "warn")
        self.assertEqual(metrics["Datasets retenus"].value, "1")
        self.assertEqual(metrics["Updates a traiter"].value, "1")

    def test_console_action_intents_disable_completed_work(self) -> None:
        model = build_console_view_model(
            {
                "projects": [{"code": "ARCHIVED", "stage": "Archived"}],
                "datasets": [["RUN-QT-002", "processed_signal", "out.csv", "sha256:out", "deleted"]],
                "updates": [["core", "0.1.0", "Signed", "Installed", "offline_bundle"]],
            }
        )
        actions = {action.action_id: action for action in model.actions}
        metrics = {metric.label: metric for metric in model.metrics}

        self.assertFalse(actions["advance_project"].enabled)
        self.assertFalse(actions["request_dataset_deletion"].enabled)
        self.assertFalse(actions["validate_update"].enabled)
        self.assertEqual(metrics["Projets actifs"].value, "0")
        self.assertEqual(metrics["Updates a traiter"].tone, "ok")

    def test_builds_operator_form_specs_for_local_write_repositories(self) -> None:
        specs = build_operator_form_specs(
            {
                "instruments": [["DAQ-001", "DAQ"]],
                "instrument_categories": [
                    ["daq_chassis", "data_monitoring", "DAQ chassis and modules"]
                ],
                "projects": [{"code": "CEM-QT-001", "customer": "Rail Motion"}],
                "schedule": [
                    [
                        "PLAN-QT-001",
                        "CEM-QT-001",
                        "Emission conduite",
                        "emission_conducted",
                        "2026-07-01T09:00",
                        "2026-07-01T12:00",
                        "operator.one",
                        "Lab A",
                        "planned",
                    ],
                    [
                        "PLAN-QT-DONE",
                        "CEM-QT-001",
                        "Emission terminee",
                        "emission_conducted",
                        "2026-07-01T13:00",
                        "2026-07-01T15:00",
                        "operator.one",
                        "Lab A",
                        "completed",
                    ]
                ],
                "test_categories": [
                    ["emission", "", "Emission", "active"],
                    ["emission_conducted", "emission", "Emission conduite", "active"],
                ],
            },
            {"metrology", "projects", "test_definitions"},
        )
        by_id = {spec.action_id: spec for spec in specs}

        self.assertTrue(by_id["create_project"].enabled)
        self.assertTrue(by_id["complete_contract_review_item"].enabled)
        self.assertTrue(by_id["advance_project"].enabled)
        self.assertTrue(by_id["register_instrument"].enabled)
        self.assertTrue(by_id["attach_instrument_document"].enabled)
        self.assertTrue(by_id["set_instrument_serviceability"].enabled)
        self.assertFalse(by_id["run_simulated_emc_test"].enabled)
        self.assertTrue(by_id["schedule_service_item"].enabled)
        self.assertTrue(by_id["update_service_schedule_status"].enabled)
        self.assertTrue(by_id["create_test_category"].enabled)
        self.assertIn(
            ("daq_chassis", "daq_chassis - DAQ chassis and modules"),
            by_id["register_instrument"].fields[5].choices,
        )
        self.assertIn(
            ("CEM-QT-001", "CEM-QT-001 - Rail Motion"),
            by_id["schedule_service_item"].fields[1].choices,
        )
        self.assertEqual(
            by_id["schedule_service_item"].fields[10].choices,
            (("planned", "planned"),),
        )
        self.assertIn(
            ("PLAN-QT-001", "PLAN-QT-001 - Emission conduite - planned"),
            by_id["update_service_schedule_status"].fields[0].choices,
        )
        self.assertNotIn(
            ("PLAN-QT-DONE", "PLAN-QT-DONE - Emission terminee - completed"),
            by_id["update_service_schedule_status"].fields[0].choices,
        )
        self.assertEqual(
            by_id["update_service_schedule_status"].fields[1].choices,
            (
                ("confirmed", "confirmed"),
                ("cancelled", "cancelled"),
            ),
        )

        closed_specs = build_operator_form_specs(
            {
                "projects": [{"code": "CEM-QT-001", "customer": "Rail Motion"}],
                "schedule": [
                    [
                        "PLAN-QT-CLOSED",
                        "CEM-QT-001",
                        "Emission cloturee",
                        "emission_conducted",
                        "2026-07-01T09:00",
                        "2026-07-01T12:00",
                        "operator.one",
                        "Lab A",
                        "cancelled",
                    ]
                ],
            },
            {"projects"},
        )
        closed_by_id = {spec.action_id: spec for spec in closed_specs}
        self.assertFalse(closed_by_id["update_service_schedule_status"].enabled)
        self.assertEqual(
            closed_by_id["update_service_schedule_status"].disabled_reason,
            "Aucun bloc planning disponible",
        )

        disabled = build_operator_form_specs({}, set())
        self.assertFalse(disabled[0].enabled)
        self.assertEqual(disabled[0].disabled_reason, "Depot projets requis")

        agent_specs = build_operator_form_specs(
            {
                "instruments": [["SA-QT-AGENT", "receiver"]],
                "instrument_categories": [["spectrum_analyzer", "radio_rf", "Spectrum analyzer"]],
            },
            {"metrology", "metrology_agent"},
        )
        agent_by_id = {spec.action_id: spec for spec in agent_specs}
        self.assertTrue(agent_by_id["register_instrument"].enabled)
        self.assertTrue(agent_by_id["set_instrument_serviceability"].enabled)
        self.assertFalse(agent_by_id["attach_instrument_document"].enabled)

        execution_specs = build_operator_form_specs(
            {
                "instruments": [["SA-QT-AGENT", "receiver"]],
                "projects": [{"code": "CEM-QT-AGENT", "customer": "Agent Customer"}],
                "instrument_categories": [["spectrum_analyzer", "radio_rf", "Spectrum analyzer"]],
            },
            {"metrology", "projects", "metrology_agent", "test_execution_agent"},
        )
        execution_by_id = {spec.action_id: spec for spec in execution_specs}
        self.assertTrue(execution_by_id["run_simulated_emc_test"].enabled)
        self.assertEqual(
            execution_by_id["run_simulated_emc_test"].fields[2].default,
            "SIM-EMC-CONDUCTED",
        )

    def test_qt_form_action_registers_instrument_and_document_without_pyside(self) -> None:
        module = load_qt_console_module()
        with tempfile.TemporaryDirectory() as temporary_directory:
            metrology_db = Path(temporary_directory) / "metrology.sqlite"
            args = SimpleNamespace(
                metrology_db=metrology_db,
                projects_db=None,
                test_definitions_db=None,
                migrations_root=Path("storage/sqlite"),
            )

            module._execute_form_action(
                args,
                "register_instrument",
                {
                    "asset_id": "DAQ-QT-FORM",
                    "family": "DAQ",
                    "manufacturer": "openDAQ",
                    "model": "Reference",
                    "serial_number": "QT001",
                    "category_code": "daq_chassis",
                    "serviceability_status": "usable",
                    "serviceability_reason": "",
                    "part_number": "ODAQ-QT",
                    "calibration_period_months": "12",
                    "certificate_reference": "CERT-QT-001",
                    "calibrated_at": "2026-06-28",
                    "provider": "cal.lab",
                    "file_reference": "certs/CERT-QT-001.pdf",
                    "capabilities_json": '["time_series"]',
                    "metrology_notes": "Qt form smoke test",
                },
            )
            module._execute_form_action(
                args,
                "attach_instrument_document",
                {
                    "asset_id": "DAQ-QT-FORM",
                    "document_kind": "script",
                    "title": "Setup script",
                    "file_reference": "scripts/daq_setup.py",
                    "uploaded_by": "operator.one",
                    "checksum": "",
                    "revision": "A",
                    "applies_to_function": "setup",
                },
            )

            repository = MetrologyRepository(metrology_db, Path("storage/sqlite"))
            repository.initialize()
            instrument = repository.get_instrument("DAQ-QT-FORM")
            calibration = repository.latest_calibration_record("DAQ-QT-FORM")
            documents = repository.list_instrument_documents("DAQ-QT-FORM")

        self.assertEqual(instrument["part_number"], "ODAQ-QT")
        self.assertEqual(instrument["serviceability_status"], "usable")
        self.assertEqual(instrument["calibration_period_months"], 12)
        self.assertEqual(calibration["due_at"], "2027-06-28")
        self.assertEqual(documents[0]["document_kind"], "script")

    def test_qt_form_action_advances_project_through_agent_without_pyside(self) -> None:
        module = load_qt_console_module()
        args = SimpleNamespace(
            projects_db=Path("data/agent/projects.sqlite"),
            migrations_root=Path("storage/sqlite"),
            agent_url="http://127.0.0.1:8765",
        )

        with patch.object(module, "advance_project_stage") as advance:
            module._execute_form_action(
                args,
                "advance_project",
                {
                    "code": "CEM-QT-AGENT",
                    "actor": "quality.lead",
                    "reason": "Contract review complete",
                },
            )

        advance.assert_called_once()
        self.assertEqual(advance.call_args.kwargs["agent_url"], "http://127.0.0.1:8765")

    def test_qt_form_action_registers_instrument_through_agent_without_sqlite(self) -> None:
        module = load_qt_console_module()
        args = SimpleNamespace(
            metrology_db=None,
            migrations_root=Path("storage/sqlite"),
            agent_url="http://127.0.0.1:8765",
        )

        with patch.object(module, "register_metrology_instrument") as register:
            module._execute_form_action(
                args,
                "register_instrument",
                {
                    "asset_id": "SA-QT-AGENT",
                    "family": "receiver",
                    "manufacturer": "Example",
                    "model": "SA9000",
                    "serial_number": "SN-QT",
                    "category_code": "spectrum_analyzer",
                    "serviceability_status": "usable",
                    "serviceability_reason": "",
                    "part_number": "PN-QT",
                    "calibration_period_months": "12",
                    "certificate_reference": "",
                    "calibrated_at": "",
                    "provider": "",
                    "file_reference": "",
                    "capabilities_json": "{}",
                    "metrology_notes": "",
                },
            )

        register.assert_called_once()
        self.assertIsNone(register.call_args.kwargs["metrology_db"])
        self.assertEqual(register.call_args.kwargs["agent_url"], "http://127.0.0.1:8765")

    def test_qt_form_action_runs_simulated_emc_test_through_agent(self) -> None:
        module = load_qt_console_module()
        args = SimpleNamespace(
            projects_db=None,
            metrology_db=None,
            test_definitions_db=None,
            measurement_data_db=None,
            update_catalog_db=None,
            migrations_root=Path("storage/sqlite"),
            agent_url="http://127.0.0.1:8765",
        )

        with patch.object(module, "run_simulated_emc_test_action") as run_test:
            run_test.return_value = {
                "message": "Essai refuse RUN-QT: SA-QT/missing_evidence/calibration_missing"
            }
            message = module._execute_form_action(
                args,
                "run_simulated_emc_test",
                {
                    "attempt_id": "RUN-QT",
                    "project_code": "CEM-QT-AGENT",
                    "test_method_reference": "SIM-EMC-CONDUCTED",
                    "execution_mode": "accredited",
                    "asset_id": "SA-QT",
                    "operator": "operator.one",
                    "checked_on": "2026-07-01",
                    "reason": "operator launch",
                },
            )

        run_test.assert_called_once()
        self.assertEqual(
            run_test.call_args.kwargs["agent_url"],
            "http://127.0.0.1:8765",
        )
        self.assertIn("Essai refuse RUN-QT", message)

    def test_qt_agent_status_maps_storage_state_without_pyside(self) -> None:
        module = load_qt_console_module()

        connected = module._agent_status_from_storage_report(
            {
                "domains": [
                    {"domain": "projects", "status": "current"},
                    {"domain": "sync", "status": "current"},
                ]
            }
        )
        missing = module._agent_status_from_storage_report(
            {"domains": [{"domain": "projects", "status": "missing"}]}
        )
        migration = module._agent_status_from_storage_report(
            {"domains": [{"domain": "projects", "status": "migration_required"}]}
        )
        invalid = module._agent_status_from_storage_report(
            {"domains": [{"domain": "projects", "status": "invalid"}]}
        )

        self.assertEqual(connected.label, "Agent local: connecte")
        self.assertEqual(connected.tone, "ok")
        self.assertEqual(missing.label, "Agent local: stockage non initialise")
        self.assertEqual(migration.label, "Agent local: migration requise")
        self.assertEqual(invalid.label, "Agent local: erreur integrite")
        self.assertEqual(invalid.tone, "bad")

    def test_qt_agent_status_queries_local_agent_client(self) -> None:
        module = load_qt_console_module()

        with patch.object(module, "LocalAgentClient") as client_type:
            client = client_type.return_value
            client.storage_status.return_value = {
                "domains": [
                    {"domain": "projects", "status": "current"},
                    {"domain": "sync", "status": "current"},
                ]
            }

            status = module._agent_status("http://127.0.0.1:8765")

        client_type.assert_called_once_with("http://127.0.0.1:8765", timeout_seconds=1.0)
        client.storage_status.assert_called_once()
        self.assertEqual(status.label, "Agent local: connecte")

    def test_qt_form_action_schedules_service_and_creates_category_without_pyside(self) -> None:
        module = load_qt_console_module()
        with tempfile.TemporaryDirectory() as temporary_directory:
            base = Path(temporary_directory)
            projects_db = base / "projects.sqlite"
            test_definitions_db = base / "test_definitions.sqlite"
            args = SimpleNamespace(
                metrology_db=None,
                projects_db=projects_db,
                test_definitions_db=test_definitions_db,
                migrations_root=Path("storage/sqlite"),
            )

            module._execute_form_action(
                args,
                "create_project",
                {
                    "code": "CEM-QT-SCHEDULE",
                    "customer_name": "Rail Motion",
                    "execution_mode": "accredited",
                    "stage": "test_planning",
                    "actor": "operator.one",
                    "reason": "Qt form project creation",
                },
            )
            module._execute_form_action(
                args,
                "schedule_service_item",
                {
                    "item_code": "PLAN-QT-FORM",
                    "project_code": "CEM-QT-SCHEDULE",
                    "title": "Emission conduite",
                    "test_category_code": "emission_conducted",
                    "test_method_code": "",
                    "planned_start_at": "2026-07-01T09:00",
                    "planned_end_at": "2026-07-01T12:00",
                    "assigned_operator": "operator.one",
                    "location": "Lab A",
                    "equipment_under_test": "EUT rail",
                    "status": "planned",
                    "notes": "Qt form smoke test",
                },
            )
            module._execute_form_action(
                args,
                "update_service_schedule_status",
                {
                    "item_code": "PLAN-QT-FORM",
                    "status": "confirmed",
                    "actor": "operator.two",
                    "reason": "Qt form confirmation",
                },
            )
            module._execute_form_action(
                args,
                "complete_contract_review_item",
                {
                    "project_code": "CEM-QT-SCHEDULE",
                    "item": "requirements_reviewed",
                    "completed_by": "quality.lead",
                    "comment": "Qt form contract review",
                },
            )
            module._execute_form_action(
                args,
                "create_test_category",
                {
                    "code": "immunity_magnetic_field_qt",
                    "parent_code": "immunity_radiated",
                    "label": "Champ magnetique",
                    "description": "Categorie creee depuis la console Qt.",
                    "sort_order": "30",
                },
            )

            project_repository = ProjectRepository(projects_db, Path("storage/sqlite"))
            project_repository.initialize()
            schedule = project_repository.list_service_schedule_items()
            events = project_repository.audit_events("CEM-QT-SCHEDULE")
            contract_items = project_repository.contract_review_items("CEM-QT-SCHEDULE")
            test_repository = TestDefinitionRepository(
                test_definitions_db,
                Path("storage/sqlite"),
            )
            test_repository.initialize()
            category = test_repository.get_test_category("immunity_magnetic_field_qt")

        self.assertEqual(schedule[0]["item_code"], "PLAN-QT-FORM")
        self.assertEqual(schedule[0]["test_category_code"], "emission_conducted")
        self.assertIsNone(schedule[0]["test_method_code"])
        self.assertEqual(schedule[0]["status"], "confirmed")
        event_actions = {event["action"] for event in events}
        self.assertIn("project_created", event_actions)
        self.assertIn("service_schedule_item_planned", event_actions)
        self.assertIn("service_schedule_item_status_updated", event_actions)
        self.assertIn("contract_review_item_completed", event_actions)
        self.assertEqual(contract_items[0]["item"], "requirements_reviewed")
        self.assertEqual(category["parent_code"], "immunity_radiated")

    def test_builds_qt_console_bootstrap_from_local_repository_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects_db = Path(temporary_directory) / "projects.sqlite"
            repository = ProjectRepository(projects_db, Path("storage/sqlite"))
            repository.initialize()
            repository.create_project(
                code="CEM-QT-REPO",
                customer_name="Repository Customer",
                execution_mode="investigation",
                stage="measuring",
            )

            payload = build_console_bootstrap_from_repositories(
                migrations_root=Path("storage/sqlite"),
                projects_db=projects_db,
            )

        self.assertEqual(payload["projects"][0]["code"], "CEM-QT-REPO")
        self.assertEqual(payload["projects"][0]["customer"], "Repository Customer")

    def test_qt_console_project_views_refresh_from_agent(self) -> None:
        with patch("emc_locus.qt_console_data.LocalAgentClient") as client_type:
            client = client_type.return_value
            client.list_projects.return_value = {
                "projects": [
                    {
                        "code": "CEM-QT-AGENT",
                        "customer_name": "Agent Customer",
                        "execution_mode": "non_accredited",
                        "stage": "contract_review",
                    }
                ]
            }
            client.contract_review.return_value = {
                "contract_review": {
                    "completed_items": [
                        {
                            "item": "scope_confirmed",
                            "completed_by": "quality.lead",
                            "comment": "agent",
                        }
                    ]
                }
            }
            client.audit_events.return_value = {
                "audit_events": [
                    {
                        "sequence": 1,
                        "actor": "quality.lead",
                        "action": "project_created",
                        "reason": "agent",
                        "occurred_at": "2026-06-29T00:00:00Z",
                    }
                ]
            }
            client.list_project_test_executions.return_value = {
                "executions": [
                    {
                        "attempt_id": "RUN-QT-AGENT",
                        "project_code": "CEM-QT-AGENT",
                        "test_method_reference": "SIM-EMC-CONDUCTED",
                        "status": "refused",
                        "ready": False,
                        "operator": "operator.one",
                        "checked_on": "2026-07-01",
                        "completed_at": "2026-07-01T10:00:00Z",
                        "revision": "rev-run",
                    }
                ]
            }
            client.sync_outbox.return_value = {
                "sync_outbox": [
                    {
                        "operation_id": "op-agent",
                        "entity_id": "CEM-QT-AGENT",
                        "operation_kind": "project_created",
                        "base_revision": "rev-0000",
                        "resulting_revision": "rev-0001",
                        "status": "pending",
                    }
                ]
            }
            client.list_metrology_instruments.return_value = {"instruments": []}

            with patch("emc_locus.qt_console_data.ProjectRepository") as project_repository:
                payload = build_console_bootstrap_from_repositories(
                    migrations_root=Path("storage/sqlite"),
                    projects_db=Path("legacy-projects.sqlite"),
                    agent_url="http://127.0.0.1:8765",
                )

        client_type.assert_called_once_with("http://127.0.0.1:8765")
        project_repository.assert_not_called()
        self.assertEqual(payload["projects"][0]["code"], "CEM-QT-AGENT")
        self.assertEqual(payload["contract_review_items"][0][1], "scope_confirmed")
        self.assertEqual(payload["project_audit_events"][0][3], "project_created")
        self.assertEqual(payload["test_executions"][0][0], "RUN-QT-AGENT")
        self.assertEqual(payload["test_executions"][0][3], "refused")
        self.assertEqual(payload["sync_outbox"][0][0], "op-agent")

    def test_qt_console_metrology_views_refresh_from_agent(self) -> None:
        with patch("emc_locus.qt_console_data.LocalAgentClient") as client_type:
            client = client_type.return_value
            client.list_projects.return_value = {"projects": []}
            client.sync_outbox.return_value = {"sync_outbox": []}
            client.list_metrology_instruments.return_value = {
                "instruments": [
                    {
                        "asset_id": "SA-QT-AGENT",
                        "family": "receiver",
                        "category_code": "spectrum_analyzer",
                        "manufacturer": "Example",
                        "model": "SA9000",
                        "serial_number": "SN-QT",
                        "part_number": "PN-QT",
                        "availability": "available",
                        "serviceability_status": "usable",
                        "calibration_period_months": 12,
                        "capabilities_json": '{"rf": true}',
                        "latest_calibration_event": {
                            "event_id": "CAL-SA-QT-AGENT-2026",
                            "certificate_reference": "CERT-SA-QT-AGENT-2026",
                            "calibrated_at": "2026-06-30",
                            "due_at": "2027-06-30",
                            "revision": "calibration-event-0001",
                            "document_manifest_json": json.dumps(
                                {
                                    "object_id": "obj-cert",
                                    "original_filename": "cert.pdf",
                                    "local_reference": "certs/cert.pdf",
                                    "revision": "A",
                                }
                            ),
                        },
                    }
                ]
            }
            client.get_metrology_calibration_status.return_value = {
                "asset_id": "SA-QT-AGENT",
                "calibration_status": "valid",
                "due_at": "2027-06-30",
            }
            client.assess_metrology_readiness.return_value = {
                "ready": True,
                "checked_on": "2026-06-30",
                "execution_mode": "accredited",
                "instrument_results": [
                    {
                        "asset_id": "SA-QT-AGENT",
                        "serviceability_status": "usable",
                        "calibration_status": "valid",
                        "reasons": [],
                        "blocking": False,
                    }
                ],
                "blocking_issues": [],
                "warnings": [],
            }

            with patch("emc_locus.qt_console_data.MetrologyRepository") as metrology_repository:
                payload = build_console_bootstrap_from_repositories(
                    migrations_root=Path("storage/sqlite"),
                    metrology_db=Path("legacy-metrology.sqlite"),
                    agent_url="http://127.0.0.1:8765",
                )

        metrology_repository.assert_not_called()
        self.assertEqual(payload["instruments"][0][0], "SA-QT-AGENT")
        self.assertEqual(payload["instruments"][0][4], "CERT-SA-QT-AGENT-2026")
        self.assertEqual(payload["instruments"][0][6], "ok")
        self.assertEqual(payload["metrology_readiness"][0][0:5], ["pret", "SA-QT-AGENT", "usable", "valid", "non"])
        self.assertEqual(payload["instrument_documents"][0][3], "certs/cert.pdf")


if __name__ == "__main__":
    unittest.main()
