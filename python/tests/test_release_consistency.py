from __future__ import annotations

import re
import json
import tomllib
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class ReleaseConsistencyTests(unittest.TestCase):
    def test_release_versions_are_consistent(self) -> None:
        version = (ROOT / "VERSION").read_text(encoding="utf-8").strip()

        workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        pyproject = tomllib.loads(
            (ROOT / "python" / "pyproject.toml").read_text(encoding="utf-8")
        )
        lab_package = json.loads(
            (ROOT / "apps" / "lab-console" / "package.json").read_text(
                encoding="utf-8"
            )
        )
        lab_package_lock = json.loads(
            (ROOT / "apps" / "lab-console" / "package-lock.json").read_text(
                encoding="utf-8"
            )
        )
        lock_text = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        revision_control = (ROOT / "docs" / "revision-control.md").read_text(
            encoding="utf-8"
        )
        lab_readme = (ROOT / "apps" / "lab-console" / "README.md").read_text(
            encoding="utf-8"
        )
        gitattributes = (ROOT / ".gitattributes").read_text(encoding="utf-8")

        self.assertRegex(version, r"^\d+\.\d+\.\d+$")
        self.assertEqual(workspace["workspace"]["package"]["version"], version)
        self.assertEqual(pyproject["project"]["version"], version)
        self.assertEqual(lab_package["version"], version)
        self.assertEqual(lab_package_lock["version"], version)
        self.assertEqual(lab_package_lock["packages"][""]["version"], version)
        self.assertIn(f"Current software version: `{version}`.", readme)
        self.assertIn(f"Version `{version}` was validated", revision_control)
        self.assertNotIn("pnpm", readme.lower())
        self.assertNotIn("pnpm", lab_readme.lower())
        self.assertIn("apps/lab-console/**/*.html text eol=lf", gitattributes)

        locked_versions = {
            name: package_version
            for name, package_version in re.findall(
                r'name = "(emc-locus-[^"]+)"\s+version = "([^"]+)"',
                lock_text,
            )
        }
        self.assertEqual(locked_versions["emc-locus-agent"], version)
        self.assertEqual(locked_versions["emc-locus-core"], version)


if __name__ == "__main__":
    unittest.main()
