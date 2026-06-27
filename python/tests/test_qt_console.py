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
            }
        )
        project_table = model.tables[0]
        dataset_table = model.tables[3]

        self.assertEqual(project_table.columns[0:3], ("Code", "Client", "Etape"))
        self.assertEqual(project_table.rows[0][0:3], ("CEM-QT-001", "Rail Motion", "Measuring"))
        self.assertEqual(dataset_table.columns, ("Run", "Type", "Fichier", "Checksum", "Retention"))
        self.assertEqual(dataset_table.rows[0][1], "raw_signal")

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
