from __future__ import annotations

import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
LAB_CONSOLE = ROOT / "apps" / "lab-console"
QT_BOOTSTRAP = ROOT / "apps" / "qt-console" / "demo" / "bootstrap.json"


class LabConsoleStaticBuildTests(unittest.TestCase):
    def test_lab_console_package_declares_release_build_scripts(self) -> None:
        package = json.loads((LAB_CONSOLE / "package.json").read_text(encoding="utf-8"))

        self.assertEqual(package["version"], "0.10.0")
        self.assertEqual(package["scripts"]["build"], "tsc --noEmit && vite build")
        self.assertIn("test", package["scripts"])
        self.assertIn("typecheck", package["scripts"])
        self.assertIn("lint", package["scripts"])

    def test_lab_console_distribution_is_served_under_lab_base(self) -> None:
        index = (LAB_CONSOLE / "dist" / "index.html").read_text(encoding="utf-8")

        self.assertIn('/lab/assets/', index)
        self.assertNotIn("CEM-2026-001", index)
        self.assertNotIn("Client demo", index)
        assets = list((LAB_CONSOLE / "dist" / "assets").glob("*"))
        self.assertTrue(assets, "LAB CONSOLE dist assets must be versioned for release launch")

    def test_qt_static_fixture_is_strict_json_not_lab_javascript(self) -> None:
        raw = QT_BOOTSTRAP.read_text(encoding="utf-8")

        self.assertFalse(raw.lstrip().startswith("window."))
        payload = json.loads(raw)
        self.assertEqual(payload["qt_console_version"], "static-demo-v1")
        self.assertEqual(payload["projects"][0]["code"], "QT-DEMO-001")


if __name__ == "__main__":
    unittest.main()
