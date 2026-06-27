from __future__ import annotations

import sqlite3
import tempfile
import unittest
from contextlib import closing
from pathlib import Path

from emc_locus import (
    MeasurementDataRepository,
    MetrologyRepository,
    ProjectRepository,
    SyncRepository,
    TestDefinitionRepository,
    UpdateCatalogRepository,
    build_bootstrap,
    write_bootstrap_js,
)


class MeasurementDataRepositoryTests(unittest.TestCase):
    def test_records_reviewed_retention_workflow(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = MeasurementDataRepository(
                Path(temporary_directory) / "measurement_data.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            dataset_id = repository.add_dataset(
                project_code="CEM-2026-001",
                campaign_reference="CAMP-001",
                measurement_run_reference="RUN-001",
                kind="raw_signal",
                file_reference="data/RUN-001/raw.opendata",
                checksum="sha256:raw001",
            )

            self.assertEqual(
                repository.get_dataset(dataset_id)["retention_status"],
                "retained",
            )
            with self.assertRaises(ValueError):
                repository.record_retention_event(
                    dataset_id=dataset_id,
                    new_status="deleted",
                    actor="data.manager",
                    reason="Direct deletion should be blocked",
                )

            request_id = repository.record_retention_event(
                dataset_id=dataset_id,
                new_status="deletion_requested",
                actor="data.manager",
                reason="Retention period expired",
                audit_event_reference="audit-001",
            )
            approve_id = repository.record_retention_event(
                dataset_id=dataset_id,
                new_status="deletion_approved",
                actor="quality.manager",
                reason="Backup and lineage reviewed",
                audit_event_reference="audit-002",
            )
            delete_id = repository.record_retention_event(
                dataset_id=dataset_id,
                new_status="deleted",
                actor="data.manager",
                reason="Approved deletion executed",
                audit_event_reference="audit-003",
            )

            dataset = repository.get_dataset(dataset_id)
            events = repository.retention_events(dataset_id)
            deleted_datasets = repository.datasets_by_retention_status("deleted")

            self.assertEqual(dataset["retention_status"], "deleted")
            self.assertEqual(
                [event["id"] for event in events],
                [request_id, approve_id, delete_id],
            )
            self.assertEqual(events[0]["previous_status"], "retained")
            self.assertEqual(events[0]["new_status"], "deletion_requested")
            self.assertEqual(events[0]["audit_event_reference"], "audit-001")
            self.assertEqual(events[2]["reason"], "Approved deletion executed")
            self.assertEqual(deleted_datasets[0]["id"], dataset_id)

    def test_initialize_applies_retention_migration_to_existing_database(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            database_path = Path(temporary_directory) / "measurement_data.sqlite"
            connection = sqlite3.connect(database_path)
            try:
                connection.executescript(
                    Path("storage/sqlite/measurement_data/0001_measurement_data.sql").read_text(
                        encoding="utf-8"
                    )
                )
                connection.execute(
                    """
                    INSERT INTO datasets (
                        project_code,
                        campaign_reference,
                        measurement_run_reference,
                        kind,
                        file_reference,
                        checksum,
                        acquired_at,
                        immutable
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        "CEM-2026-001",
                        "CAMP-001",
                        "RUN-001",
                        "raw_signal",
                        "data/RUN-001/raw.opendata",
                        "sha256:raw001",
                        "1970-01-01T00:00:00Z",
                        1,
                    ),
                )
                connection.commit()
            finally:
                connection.close()

            repository = MeasurementDataRepository(database_path, Path("storage/sqlite"))
            repository.initialize()

            with closing(repository.connect()) as connection:
                version_rows = connection.execute(
                    "SELECT version FROM schema_migrations ORDER BY version"
                ).fetchall()
                retention_table = connection.execute(
                    """
                    SELECT name
                    FROM sqlite_master
                    WHERE type = 'table'
                      AND name = 'dataset_retention_events'
                    """
                ).fetchone()
                dataset = connection.execute("SELECT * FROM datasets").fetchone()

            self.assertEqual([row["version"] for row in version_rows], [1, 2])
            self.assertIsNotNone(retention_table)
            self.assertEqual(dataset["retention_status"], "retained")


class GuiBootstrapTests(unittest.TestCase):
    def test_builds_bootstrap_from_local_repositories(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            base = Path(temporary_directory)
            projects = ProjectRepository(base / "projects.sqlite", root)
            metrology = MetrologyRepository(base / "metrology.sqlite", root)
            test_definitions = TestDefinitionRepository(
                base / "test_definitions.sqlite",
                root,
            )
            measurement_data = MeasurementDataRepository(
                base / "measurement_data.sqlite",
                root,
            )
            update_catalog = UpdateCatalogRepository(base / "update_catalog.sqlite", root)

            for repository in (
                projects,
                metrology,
                test_definitions,
                measurement_data,
                update_catalog,
            ):
                repository.initialize()

            projects.create_project(
                code="CEM-BOOT-001",
                customer_name="Bootstrap Customer",
                execution_mode="investigation",
                stage="measuring",
            )
            metrology.add_instrument(
                asset_id="DAQ-001",
                family="DAQ",
                manufacturer="Open",
                model="DAQ",
                serial_number="001",
                calibration_requirement="required",
            )
            metrology.add_calibration_record(
                asset_id="DAQ-001",
                certificate_reference="CERT-001",
                calibrated_at="2026-01-01",
                due_at="2027-01-01",
                provider="Metrology Lab",
            )
            test_definitions.add_test_method(
                code="INRUSH-001",
                standard_code=None,
                name="Inrush current",
                family="inrush",
                measurement_axis="time_series",
            )
            test_definitions.add_method_revision(
                method_code="INRUSH-001",
                revision="A",
                status="approved",
                checksum="sha256:method001",
            )
            measurement_data.add_dataset(
                project_code="CEM-BOOT-001",
                campaign_reference="CAMP-BOOT-001",
                measurement_run_reference="RUN-BOOT-001",
                kind="raw_signal",
                file_reference="data/RUN-BOOT-001/raw.opendata",
                checksum="sha256:rawboot001",
            )
            update_catalog.add_update_package(
                package_name="driver-pack-opendaq",
                package_version="0.1.0",
                component="instrument_driver",
                compatibility_range="0.1.0..0.1.9",
                signed_checksum="sha256:driver001",
            )

            payload = build_bootstrap(
                projects=projects,
                metrology=metrology,
                test_definitions=test_definitions,
                measurement_data=measurement_data,
                update_catalog=update_catalog,
            )
            output_path = base / "bootstrap.js"
            write_bootstrap_js(output_path, payload)

            self.assertEqual(payload["projects"][0]["code"], "CEM-BOOT-001")
            self.assertEqual(payload["projects"][0]["stage"], "Measuring")
            self.assertEqual(payload["instruments"][0][0], "DAQ-001")
            self.assertEqual(payload["instruments"][0][5], "ok")
            self.assertEqual(payload["methods"][0][3], "approved")
            self.assertEqual(payload["datasets"][0][4], "Immutable")
            self.assertEqual(payload["updates"][0][0], "driver-pack-opendaq")
            self.assertIn("window.EMC_LOCUS_BOOTSTRAP", output_path.read_text())


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
