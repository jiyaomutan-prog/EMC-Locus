from __future__ import annotations

import sqlite3
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

from emc_locus import SyncRepository, TestDefinitionRepository, UpdateCatalogRepository


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


class SyncRepositoryTests(unittest.TestCase):
    def test_records_conflict_and_applies_resolution_plan(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = SyncRepository(
                Path(temporary_directory) / "sync.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            self.assertEqual(repository.metadata()["domain"], "sync")

            repository.record_conflict(
                conflict_id="conflict-001",
                domain="project_records",
                kind="concurrent_update",
                local_snapshot="project-local.1",
                reference_snapshot="project-reference.2",
            )
            plan_id = repository.apply_resolution_plan(
                conflict_id="conflict-001",
                resolution="keep_local",
                action="push_local_snapshot",
                planned_by="sync.operator",
                audit_event_reference="project-audit-42",
            )

            conflict = repository.get_conflict("conflict-001")
            plans = repository.action_plans_for_conflict("conflict-001")

            self.assertIsNotNone(plan_id)
            self.assertEqual(repository.conflict_count(), 1)
            self.assertEqual(repository.action_plan_count(), 1)
            self.assertEqual(conflict["status"], "resolved")
            self.assertEqual(conflict["resolution"], "keep_local")
            self.assertEqual(repository.list_conflicts("open"), [])
            self.assertEqual(plans[0]["id"], plan_id)
            self.assertEqual(plans[0]["sequence"], 1)
            self.assertEqual(plans[0]["domain"], "project_records")
            self.assertEqual(plans[0]["action"], "push_local_snapshot")
            self.assertEqual(plans[0]["requires_audit_event"], 1)
            self.assertEqual(plans[0]["audit_event_reference"], "project-audit-42")

    def test_defers_conflict_and_enforces_constraints(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = SyncRepository(
                Path(temporary_directory) / "sync.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            with self.assertRaises(sqlite3.IntegrityError):
                repository.record_conflict(
                    conflict_id="conflict-bad-domain",
                    domain="unknown",
                    kind="concurrent_update",
                    local_snapshot="local.1",
                    reference_snapshot="reference.1",
                )

            repository.record_conflict(
                conflict_id="conflict-002",
                domain="measurement_data",
                kind="checksum_mismatch",
                local_snapshot="measurement-local.1",
                reference_snapshot="measurement-reference.1",
            )
            plan_id = repository.apply_resolution_plan(
                conflict_id="conflict-002",
                resolution="defer",
                action="defer_for_review",
                planned_by="qa.lead",
            )

            conflict = repository.get_conflict("conflict-002")
            plans = repository.action_plans_for_conflict("conflict-002")

            self.assertIsNotNone(plan_id)
            self.assertEqual(conflict["status"], "deferred")
            self.assertEqual(conflict["resolution"], "defer")
            self.assertEqual(
                repository.list_conflicts("deferred")[0]["conflict_id"],
                "conflict-002",
            )
            self.assertEqual(plans[0]["sequence"], 1)
            self.assertEqual(plans[0]["action"], "defer_for_review")

            with self.assertRaises(sqlite3.IntegrityError):
                repository.record_action_plan(
                    conflict_id="missing-conflict",
                    domain="measurement_data",
                    kind="checksum_mismatch",
                    resolution="defer",
                    action="defer_for_review",
                    local_snapshot="measurement-local.1",
                    reference_snapshot="measurement-reference.1",
                    planned_by="qa.lead",
                )

            with self.assertRaises(sqlite3.IntegrityError):
                repository.record_action_plan(
                    conflict_id="conflict-002",
                    domain="project_records",
                    kind="checksum_mismatch",
                    resolution="defer",
                    action="defer_for_review",
                    local_snapshot="measurement-local.1",
                    reference_snapshot="measurement-reference.1",
                    planned_by="qa.lead",
                )


class UpdateCatalogRepositoryTests(unittest.TestCase):
    def test_records_accepted_install_validation_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = UpdateCatalogRepository(
                Path(temporary_directory) / "update_catalog.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            self.assertEqual(repository.metadata()["domain"], "update_catalog")

            repository.add_update_package(
                package_name="emc-locus-core",
                package_version="0.2.0",
                component="core_application",
                compatibility_range="0.1.0..0.1.9",
                signed_checksum="sha256:core-020",
            )
            evidence_id = repository.record_install_validation(
                package_name="emc-locus-core",
                package_version="0.2.0",
                component="core_application",
                installed_version="0.1.0",
                source="offline_bundle",
                compatibility_minimum_version="0.1.0",
                compatibility_maximum_version="0.1.9",
                validated_by="qa.lead",
            )
            install_id = repository.record_install(
                package_name="emc-locus-core",
                package_version="0.2.0",
                component="core_application",
                installed_by="qa.lead",
                source="offline_bundle",
                rollback_reference="emc-locus-core-0.1.0",
                validation_evidence_id=evidence_id,
            )

            evidence = repository.get_install_validation_evidence(evidence_id)
            install = repository.list_install_records()[0]

            self.assertEqual(repository.update_package_count(), 1)
            self.assertEqual(repository.validation_evidence_count(), 1)
            self.assertEqual(repository.install_record_count(), 1)
            self.assertEqual(evidence["validation_status"], "accepted")
            self.assertIsNone(evidence["reason"])
            self.assertEqual(evidence["signature_required"], 1)
            self.assertEqual(evidence["signature_present"], 1)
            self.assertEqual(install["id"], install_id)
            self.assertEqual(install["validation_evidence_id"], evidence_id)

    def test_rejects_invalid_update_metadata_and_install_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = UpdateCatalogRepository(
                Path(temporary_directory) / "update_catalog.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            with self.assertRaises(ValueError):
                repository.add_update_package(
                    package_name="emc core",
                    package_version="0.2.0",
                    component="core_application",
                    compatibility_range="0.1.0..0.1.9",
                    signed_checksum="sha256:core-020",
                )

            repository.add_update_package(
                package_name="emc-locus-core",
                package_version="0.2.0",
                component="core_application",
                compatibility_range="0.1.0..0.1.9",
                signed_checksum="sha256:core-020",
                offline_install_allowed=False,
            )
            evidence_id = repository.record_install_validation(
                package_name="emc-locus-core",
                package_version="0.2.0",
                component="core_application",
                installed_version="0.2.5",
                source="offline_bundle",
                compatibility_minimum_version="0.1.0",
                compatibility_maximum_version="0.1.9",
                validated_by="qa.lead",
            )

            evidence = repository.get_install_validation_evidence(evidence_id)

            self.assertEqual(evidence["validation_status"], "rejected")
            self.assertIn("offline_install_blocked", evidence["reason"])
            self.assertIn("incompatible_installed_version", evidence["reason"])
            with self.assertRaises(ValueError):
                repository.record_install(
                    package_name="emc-locus-core",
                    package_version="0.2.0",
                    component="core_application",
                    installed_by="qa.lead",
                    source="offline_bundle",
                    validation_evidence_id=evidence_id,
                )

    def test_initialize_applies_missing_update_catalog_migrations(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            database_path = Path(temporary_directory) / "update_catalog.sqlite"
            connection = sqlite3.connect(database_path)
            try:
                connection.executescript(
                    Path("storage/sqlite/update_catalog/0001_update_catalog.sql").read_text(
                        encoding="utf-8"
                    )
                )
            finally:
                connection.close()

            repository = UpdateCatalogRepository(database_path, Path("storage/sqlite"))
            repository.initialize()

            with closing(repository.connect()) as connection:
                version_rows = connection.execute(
                    "SELECT version FROM schema_migrations ORDER BY version"
                ).fetchall()
                evidence_table = connection.execute(
                    """
                    SELECT name
                    FROM sqlite_master
                    WHERE type = 'table'
                      AND name = 'update_install_validation_evidence'
                    """
                ).fetchone()

            self.assertEqual([row["version"] for row in version_rows], [1, 2])
            self.assertIsNotNone(evidence_table)


if __name__ == "__main__":
    unittest.main()
