from __future__ import annotations

import sqlite3
import tempfile
import unittest
from pathlib import Path

from emc_locus import TestDefinitionRepository


class TestDefinitionRepositoryTests(unittest.TestCase):
    def test_records_method_revision_steps_and_approval(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = TestDefinitionRepository(
                Path(temporary_directory) / "test_definitions.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            self.assertEqual(repository.metadata()["domain"], "test_definitions")

            repository.add_standard(
                code="EN-TEST-001",
                title="Example EMC immunity standard",
                edition="2026",
                issuer="EMC Locus",
            )
            repository.add_test_method(
                code="TD-INRUSH",
                standard_code="EN-TEST-001",
                name="Inrush current capture",
                family="inrush",
                measurement_axis="time_series",
            )
            revision_id = repository.add_method_revision(
                method_code="TD-INRUSH",
                revision="A",
                parameters_json='{"sample_rate_hz": 1000}',
                acceptance_criteria_json='{"peak_a_max": 50}',
                processing_graph_json='{"ops": ["peak"]}',
                checksum="sha256:draft",
            )
            step_id = repository.add_test_step(
                method_revision_id=revision_id,
                sequence=1,
                name="Arm acquisition",
                instruction="Arm synchronized channels before energizing the EUT.",
                expected_evidence="runtime observation and raw dataset checksum",
            )

            approved = repository.approve_method_revision(
                method_code="TD-INRUSH",
                revision="A",
                approved_by="qa.lead",
                checksum="sha256:approved",
            )

            self.assertTrue(approved)
            self.assertEqual(repository.get_standard("EN-TEST-001")["status"], "active")
            self.assertEqual(repository.get_test_method("TD-INRUSH")["controlled"], 1)
            self.assertEqual(repository.list_test_methods("inrush")[0]["code"], "TD-INRUSH")
            self.assertEqual(repository.method_revisions("TD-INRUSH")[0]["status"], "approved")
            self.assertEqual(repository.method_revisions("TD-INRUSH")[0]["checksum"], "sha256:approved")
            self.assertEqual(repository.test_steps(revision_id)[0]["id"], step_id)

    def test_rejects_orphan_method_and_duplicate_step_sequence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = TestDefinitionRepository(
                Path(temporary_directory) / "test_definitions.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            with self.assertRaises(sqlite3.IntegrityError):
                repository.add_test_method(
                    code="TD-ORPHAN",
                    standard_code="MISSING-STANDARD",
                    name="Orphan method",
                    family="conducted",
                    measurement_axis="frequency_sweep",
                )

            repository.add_test_method(
                code="TD-STANDALONE",
                standard_code=None,
                name="Standalone investigation method",
                family="investigation",
                measurement_axis="mixed_time_frequency",
            )
            revision_id = repository.add_method_revision(
                method_code="TD-STANDALONE",
                revision="draft-1",
            )
            repository.add_test_step(
                method_revision_id=revision_id,
                sequence=1,
                name="Capture baseline",
                instruction="Capture the baseline signal.",
                expected_evidence="raw dataset checksum",
            )

            with self.assertRaises(sqlite3.IntegrityError):
                repository.add_test_step(
                    method_revision_id=revision_id,
                    sequence=1,
                    name="Duplicate sequence",
                    instruction="This should fail.",
                    expected_evidence="none",
                )

            self.assertFalse(
                repository.approve_method_revision(
                    method_code="TD-STANDALONE",
                    revision="missing",
                    approved_by="qa.lead",
                )
            )


if __name__ == "__main__":
    unittest.main()
