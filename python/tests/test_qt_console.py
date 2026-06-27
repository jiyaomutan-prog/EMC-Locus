from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


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

    def test_normalizes_rows_for_qt_table_models(self) -> None:
        module = load_qt_console_module()

        rows = module._normalize_rows(
            [
                {"code": "CEM-QT-001", "stage": "Measuring"},
                ["RUN-QT-001", "raw_signal"],
                "standalone",
            ]
        )

        self.assertEqual(rows[0], ["CEM-QT-001", "Measuring"])
        self.assertEqual(rows[1], ["RUN-QT-001", "raw_signal"])
        self.assertEqual(rows[2], ["standalone"])


if __name__ == "__main__":
    unittest.main()
