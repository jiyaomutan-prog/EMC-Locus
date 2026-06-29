from __future__ import annotations

import re
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
        lock_text = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        revision_control = (ROOT / "docs" / "revision-control.md").read_text(
            encoding="utf-8"
        )

        self.assertRegex(version, r"^\d+\.\d+\.\d+$")
        self.assertEqual(workspace["workspace"]["package"]["version"], version)
        self.assertEqual(pyproject["project"]["version"], version)
        self.assertIn(f"Current software version: `{version}`.", readme)
        self.assertIn(f"Version `{version}` was validated", revision_control)

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
