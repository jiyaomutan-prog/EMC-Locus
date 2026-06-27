from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

from emc_locus import (
    ProjectRepository,
    build_console_bootstrap_from_repositories,
    build_console_view_model,
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
    def test_loads_bootstrap_js_payload_without_requiring_pyside(self) -> None:
        module = load_qt_console_module()
        with tempfile.TemporaryDirectory() as temporary_directory:
            bootstrap = Path(temporary_directory) / "bootstrap.js"
            bootstrap.write_text(
                'window.EMC_LOCUS_BOOTSTRAP = {"projects": [{"code": "CEM-QT-001"}]};\n',
                encoding="utf-8",
            )

            payload = module.load_bootstrap_js(bootstrap)

        self.assertEqual(payload["projects"][0]["code"], "CEM-QT-001")

    def test_rejects_non_bootstrap_payload(self) -> None:
        module = load_qt_console_module()
        with tempfile.TemporaryDirectory() as temporary_directory:
            bootstrap = Path(temporary_directory) / "bootstrap.js"
            bootstrap.write_text('{"projects": []}', encoding="utf-8")

            with self.assertRaises(ValueError):
                module.load_bootstrap_js(bootstrap)

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
                        "Available",
                        "CERT-1",
                        "2027-01-01",
                        "warn",
                        "DAQ chassis and modules",
                        "channels=8",
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
            }
        )
        tables = {table.tab_label: table for table in model.tables}
        project_table = tables["Projets"]
        metrology_table = tables["Metrologie"]
        category_table = tables["Categories"]
        dataset_table = tables["Donnees"]
        runtime_table = tables["Runtime"]
        actions = {action.action_id: action for action in model.actions}
        metrics = {metric.label: metric for metric in model.metrics}

        self.assertEqual(project_table.columns[0:3], ("Code", "Client", "Etape"))
        self.assertEqual(project_table.rows[0][0:3], ("CEM-QT-001", "Rail Motion", "Measuring"))
        self.assertEqual(
            metrology_table.columns,
            (
                "Actif",
                "Famille",
                "Etat",
                "Certificat",
                "Validite",
                "Alerte",
                "Categorie",
                "Capacites",
            ),
        )
        self.assertEqual(metrology_table.rows[0][5:8], ("warn", "DAQ chassis and modules", "channels=8"))
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
        self.assertEqual(metrics["Alertes metrologie"].tone, "warn")
        self.assertEqual(metrics["Categories instruments"].value, "1")
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


if __name__ == "__main__":
    unittest.main()
