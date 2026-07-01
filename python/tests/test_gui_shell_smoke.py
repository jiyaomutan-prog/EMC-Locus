from __future__ import annotations

import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
GUI_SHELL = ROOT / "apps" / "gui-shell"
BOOTSTRAP_PREFIX = "window.EMC_LOCUS_BOOTSTRAP ="


class GuiShellSmokeTests(unittest.TestCase):
    def test_bootstrap_is_js_with_strict_json_payload(self) -> None:
        bootstrap = (GUI_SHELL / "bootstrap.js").read_text(encoding="utf-8")

        self.assertTrue(bootstrap.startswith(BOOTSTRAP_PREFIX))
        payload = bootstrap[len(BOOTSTRAP_PREFIX) :].strip()
        if payload.endswith(";"):
            payload = payload[:-1].strip()
        data = json.loads(payload)

        self.assertEqual(data["lab_console_version"], "ia-0.2")
        self.assertEqual(data["prototype"], "lab-console-information-architecture")

    def test_static_shell_files_are_addressed_with_relative_paths(self) -> None:
        for name in ("index.html", "styles.css", "bootstrap.js", "app.js"):
            self.assertTrue((GUI_SHELL / name).is_file(), name)

        index = (GUI_SHELL / "index.html").read_text(encoding="utf-8")
        self.assertIn('href="./styles.css"', index)
        self.assertIn('src="./bootstrap.js"', index)
        self.assertIn('src="./app.js"', index)


if __name__ == "__main__":
    unittest.main()
