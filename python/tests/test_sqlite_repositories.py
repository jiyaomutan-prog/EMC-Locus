from __future__ import annotations

import json
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
    advance_project_stage,
    attach_metrology_document,
    build_bootstrap,
    complete_contract_review_item_action,
    create_project_record,
    create_test_category,
    next_project_stage,
    register_metrology_instrument,
    record_metrology_calibration,
    record_dataset_retention_action,
    record_update_install_action,
    record_update_validation_action,
    schedule_service_item,
    set_metrology_instrument_capabilities,
    set_metrology_instrument_availability,
    set_metrology_instrument_serviceability,
    update_service_schedule_status_action,
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
                graph_instance_table = connection.execute(
                    """
                    SELECT name
                    FROM sqlite_master
                    WHERE type = 'table'
                      AND name = 'processing_graph_instances'
                    """
                ).fetchone()
                graph_artifact_table = connection.execute(
                    """
                    SELECT name
                    FROM sqlite_master
                    WHERE type = 'table'
                      AND name = 'processing_graph_instance_artifacts'
                    """
                ).fetchone()
                graph_execution_table = connection.execute(
                    """
                    SELECT name
                    FROM sqlite_master
                    WHERE type = 'table'
                      AND name = 'processing_graph_executions'
                    """
                ).fetchone()
                observation_table = connection.execute(
                    """
                    SELECT name
                    FROM sqlite_master
                    WHERE type = 'table'
                      AND name = 'instrument_observations'
                    """
                ).fetchone()
                observation_columns = connection.execute(
                    "PRAGMA table_info(instrument_observations)"
                ).fetchall()
                dataset = connection.execute("SELECT * FROM datasets").fetchone()

            self.assertEqual([row["version"] for row in version_rows], [1, 2, 3, 4, 5, 6, 7])
            self.assertIsNotNone(retention_table)
            self.assertIsNotNone(graph_instance_table)
            self.assertIsNotNone(graph_artifact_table)
            self.assertIsNotNone(graph_execution_table)
            self.assertIsNotNone(observation_table)
            self.assertIn(
                "observation_checksum",
                {row["name"] for row in observation_columns},
            )
            self.assertEqual(dataset["retention_status"], "retained")

    def test_records_instrument_observations_for_runtime_traceability(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = MeasurementDataRepository(
                Path(temporary_directory) / "measurement_data.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            first_id = repository.record_instrument_observation(
                project_code="CEM-OBS-001",
                campaign_reference="CAMP-OBS-001",
                measurement_run_reference="RUN-OBS-001",
                sequence=1,
                instrument_code="RX-001",
                transport="tcp_ip",
                endpoint="TCPIP::127.0.0.1::5025",
                command_message="FREQ 1000000",
                response_message="OK",
                success=True,
                exchange_attempts=1,
            )
            second_id = repository.record_instrument_observation(
                project_code="CEM-OBS-001",
                campaign_reference="CAMP-OBS-001",
                measurement_run_reference="RUN-OBS-001",
                sequence=2,
                instrument_code="RX-001",
                transport="tcp_ip",
                endpoint="TCPIP::127.0.0.1::5025",
                command_message="READ?",
                response_message="timeout",
                success=False,
                exchange_attempts=3,
                raw_payload_json='{"error":"timeout"}',
            )
            generator_id = repository.record_instrument_observation(
                project_code="CEM-OBS-001",
                campaign_reference="CAMP-OBS-001",
                measurement_run_reference="RUN-OBS-001",
                sequence=1,
                instrument_code="GEN-001",
                transport="visa",
                endpoint="USB0::0x1234::0x5678::SN001::INSTR",
                command_message="OUTP ON",
                response_message="OK",
                success=True,
                exchange_attempts=1,
            )

            run_rows = repository.instrument_observations_for_run("RUN-OBS-001")
            receiver_rows = repository.instrument_observations_for_instrument(
                measurement_run_reference="RUN-OBS-001",
                instrument_code="RX-001",
            )
            latest_by_instrument = {
                row["instrument_code"]: row
                for row in repository.latest_instrument_observations()
            }
            second_checksum = str(receiver_rows[1]["observation_checksum"])
            checksum_row = repository.get_instrument_observation_by_checksum(second_checksum)

            self.assertEqual([row["id"] for row in receiver_rows], [first_id, second_id])
            self.assertEqual(len(run_rows), 3)
            self.assertEqual(receiver_rows[1]["success"], 0)
            self.assertEqual(receiver_rows[1]["exchange_attempts"], 3)
            self.assertTrue(str(receiver_rows[0]["observation_checksum"]).startswith("sha256:"))
            self.assertEqual(len(str(receiver_rows[0]["observation_checksum"])), 71)
            self.assertNotEqual(
                receiver_rows[0]["observation_checksum"],
                receiver_rows[1]["observation_checksum"],
            )
            self.assertEqual(checksum_row["id"], second_id)
            self.assertEqual(latest_by_instrument["RX-001"]["id"], second_id)
            self.assertEqual(latest_by_instrument["GEN-001"]["id"], generator_id)

            with self.assertRaises(sqlite3.IntegrityError):
                repository.record_instrument_observation(
                    project_code="CEM-OBS-001",
                    campaign_reference="CAMP-OBS-001",
                    measurement_run_reference="RUN-OBS-001",
                    sequence=2,
                    instrument_code="RX-001",
                    transport="tcp_ip",
                    endpoint="TCPIP::127.0.0.1::5025",
                    command_message="READ?",
                    response_message="duplicate",
                    success=False,
                    exchange_attempts=1,
                )

            with self.assertRaises(ValueError):
                repository.record_instrument_observation(
                    project_code="CEM-OBS-001",
                    campaign_reference="CAMP-OBS-001",
                    measurement_run_reference="RUN-OBS-001",
                    sequence=3,
                    instrument_code="RX-001",
                    transport="tcp_ip",
                    endpoint="TCPIP::127.0.0.1::5025",
                    command_message="READ?",
                    response_message="invalid attempts",
                    success=False,
                    exchange_attempts=0,
                )

    def test_records_revisioned_processing_graph_instances(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = MeasurementDataRepository(
                Path(temporary_directory) / "measurement_data.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            dataset_id = repository.add_dataset(
                project_code="CEM-2026-FFT",
                campaign_reference="CAMP-FFT-001",
                measurement_run_reference="RUN-FFT-001",
                kind="raw_signal",
                file_reference="data/RUN-FFT-001/raw.opendata",
                checksum="sha256:rawfft001",
            )
            first_revision_id = repository.add_processing_graph_instance(
                source_dataset_id=dataset_id,
                graph_reference="inrush-fft",
                graph_revision="A",
                operations_json='{"nodes": ["fft_current"]}',
                created_by="signal.engineer",
                software_version="0.1.0",
                graph_checksum="sha256:graphfft001",
                source_dataset_checksum="sha256:rawfft001",
            )
            second_revision_id = repository.add_processing_graph_instance(
                source_dataset_id=dataset_id,
                graph_reference="inrush-fft",
                graph_revision="B",
                operations_json='{"nodes": ["hann_window", "fft_current"]}',
                created_by="technical.reviewer",
                software_version="0.1.1",
                graph_checksum="sha256:graphfft002",
            )

            first_revision = repository.get_processing_graph_instance(first_revision_id)
            second_revision = repository.processing_graph_instance(
                source_dataset_id=dataset_id,
                graph_reference="inrush-fft",
                graph_revision="B",
            )
            artifact_id = repository.add_processing_graph_instance_artifact(
                processing_graph_instance_id=second_revision_id,
                output_signal_reference="current_l1_fft",
                artifact_kind="processed_signal",
                file_reference="data/RUN-FFT-001/current_l1_fft.csv",
                checksum="sha256:resultfft001",
                raw_lineage_json='["current_l1"]',
            )
            execution_id = repository.add_processing_graph_execution(
                processing_graph_instance_id=second_revision_id,
                execution_reference="exec-fft-001",
                executed_by="signal.engine",
                software_version="0.1.0",
                status="completed",
                output_artifact_count=1,
                notes="deterministic fixture",
            )
            all_instances = repository.processing_graph_instances_for_dataset(dataset_id)
            artifacts = repository.processing_graph_instance_artifacts(second_revision_id)
            executions = repository.processing_graph_executions(second_revision_id)

            self.assertEqual(
                first_revision["source_dataset_checksum"],
                "sha256:rawfft001",
            )
            self.assertEqual(first_revision["graph_revision"], "A")
            self.assertEqual(second_revision["id"], second_revision_id)
            self.assertEqual(second_revision["software_version"], "0.1.1")
            self.assertEqual(
                [row["id"] for row in all_instances],
                [first_revision_id, second_revision_id],
            )
            self.assertEqual(artifacts[0]["id"], artifact_id)
            self.assertEqual(artifacts[0]["output_signal_reference"], "current_l1_fft")
            self.assertEqual(artifacts[0]["raw_lineage_json"], '["current_l1"]')
            self.assertEqual(executions[0]["id"], execution_id)
            self.assertEqual(executions[0]["execution_reference"], "exec-fft-001")
            self.assertEqual(executions[0]["output_artifact_count"], 1)

            with self.assertRaises(ValueError):
                repository.add_processing_graph_instance(
                    source_dataset_id=dataset_id,
                    graph_reference="inrush-fft",
                    graph_revision="C",
                    operations_json="{}",
                    created_by="signal.engineer",
                    software_version="0.1.2",
                    graph_checksum="sha256:graphfft003",
                    source_dataset_checksum="sha256:wrong",
                )

            with self.assertRaisesRegex(
                ValueError,
                "operations_json must contain valid JSON",
            ):
                repository.add_processing_graph(
                    source_dataset_id=dataset_id,
                    graph_reference="legacy-invalid-json",
                    operations_json='{"nodes": [',
                    created_by="signal.engineer",
                    checksum="sha256:legacyinvalidjson",
                )

            with self.assertRaisesRegex(
                ValueError,
                "operations_json must be a JSON object or array",
            ):
                repository.add_processing_graph_instance(
                    source_dataset_id=dataset_id,
                    graph_reference="inrush-fft",
                    graph_revision="C",
                    operations_json='"fft_current"',
                    created_by="signal.engineer",
                    software_version="0.1.2",
                    graph_checksum="sha256:graphfft003",
                )

            with self.assertRaises(ValueError):
                repository.add_processing_graph_instance(
                    source_dataset_id=dataset_id,
                    graph_reference="inrush-fft",
                    graph_revision="C",
                    operations_json="{}",
                    created_by="signal.engineer",
                    software_version=" ",
                    graph_checksum="sha256:graphfft003",
                )

            with self.assertRaises(ValueError):
                repository.add_processing_graph_instance_artifact(
                    processing_graph_instance_id=second_revision_id,
                    output_signal_reference="raw_copy",
                    artifact_kind="raw_signal",
                    file_reference="data/RUN-FFT-001/raw-copy.opendata",
                    checksum="sha256:rawcopy001",
                )

            with self.assertRaises(ValueError):
                repository.add_processing_graph_instance_artifact(
                    processing_graph_instance_id=second_revision_id,
                    output_signal_reference="current l1 fft",
                    artifact_kind="processed_signal",
                    file_reference="data/RUN-FFT-001/current-l1-fft.csv",
                    checksum="sha256:invalidsignalref",
                    raw_lineage_json='["current_l1"]',
                )

            with self.assertRaises(ValueError):
                repository.add_processing_graph_instance_artifact(
                    processing_graph_instance_id=second_revision_id,
                    output_signal_reference="current_l1_fft_invalid_lineage",
                    artifact_kind="processed_signal",
                    file_reference="data/RUN-FFT-001/current-l1-fft-invalid.csv",
                    checksum="sha256:invalidlineage",
                    raw_lineage_json='{"source": "current_l1"}',
                )

            with self.assertRaises(ValueError):
                repository.add_processing_graph_instance_artifact(
                    processing_graph_instance_id=second_revision_id,
                    output_signal_reference="current_l1_fft_invalid_signal",
                    artifact_kind="processed_signal",
                    file_reference="data/RUN-FFT-001/current-l1-fft-invalid-signal.csv",
                    checksum="sha256:invalidlineagesignal",
                    raw_lineage_json='["current l1"]',
                )

            with self.assertRaises(ValueError):
                repository.add_processing_graph_execution(
                    processing_graph_instance_id=second_revision_id,
                    execution_reference="exec-fft-empty",
                    executed_by="signal.engine",
                    software_version="0.1.0",
                    status="completed",
                    output_artifact_count=0,
                )

            with self.assertRaises(ValueError):
                repository.add_processing_graph_execution(
                    processing_graph_instance_id=first_revision_id,
                    execution_reference="exec-fft-missing-artifact",
                    executed_by="signal.engine",
                    software_version="0.1.0",
                    status="completed",
                    output_artifact_count=1,
                )

            with self.assertRaises(ValueError):
                repository.add_processing_graph_execution(
                    processing_graph_instance_id=second_revision_id,
                    execution_reference="exec-fft-count-mismatch",
                    executed_by="signal.engine",
                    software_version="0.1.0",
                    status="completed",
                    output_artifact_count=2,
                )

            with self.assertRaises(ValueError):
                repository.add_processing_graph_execution(
                    processing_graph_instance_id=second_revision_id,
                    execution_reference="exec-fft-empty-software-version",
                    executed_by="signal.engine",
                    software_version=" ",
                    status="completed",
                    output_artifact_count=1,
                )

            failed_execution_id = repository.add_processing_graph_execution(
                processing_graph_instance_id=first_revision_id,
                execution_reference="exec-fft-failed-no-artifacts",
                executed_by="signal.engine",
                software_version="0.1.0",
                status="failed",
                output_artifact_count=0,
                notes="input validation failed",
            )
            failed_executions = repository.processing_graph_executions(
                first_revision_id,
            )

            self.assertEqual(failed_executions[0]["id"], failed_execution_id)
            self.assertEqual(failed_executions[0]["status"], "failed")
            self.assertEqual(failed_executions[0]["output_artifact_count"], 0)

            with self.assertRaises(ValueError):
                repository.add_processing_graph_execution(
                    processing_graph_instance_id=first_revision_id,
                    execution_reference="exec-fft-failed-count-mismatch",
                    executed_by="signal.engine",
                    software_version="0.1.0",
                    status="failed",
                    output_artifact_count=1,
                )


class MetrologyRepositoryTests(unittest.TestCase):
    def test_lists_instrument_categories_and_links_assets(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = MetrologyRepository(
                Path(temporary_directory) / "metrology.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            category = repository.get_instrument_category("daq_chassis")
            electronics = repository.list_instrument_categories(domain="electronics")
            daq_sources = repository.list_instrument_category_sources("daq_chassis")

            self.assertEqual(repository.category_count(), 34)
            self.assertIsNotNone(category)
            assert category is not None
            self.assertEqual(category["domain"], "data_monitoring")
            self.assertIn(
                "openDAQ device",
                json.loads(str(category["typical_instruments_json"])),
            )
            self.assertIn("oscilloscope", {row["code"] for row in electronics})
            self.assertTrue(any("NI" in str(source["source_name"]) for source in daq_sources))

            repository.add_instrument(
                asset_id="DAQ-001",
                family="DAQ",
                manufacturer="Open",
                model="DAQ",
                serial_number="001",
                calibration_requirement="required",
                category_code="daq_chassis",
            )

            instrument = repository.get_instrument("DAQ-001")
            by_category = repository.instruments_by_category("daq_chassis")
            by_domain = repository.instruments_by_category_domain("data_monitoring")

            self.assertEqual(instrument["category_code"], "daq_chassis")
            self.assertEqual(by_category[0]["asset_id"], "DAQ-001")
            self.assertEqual(by_domain[0]["category_label"], "DAQ chassis and modules")

            with self.assertRaises(sqlite3.IntegrityError):
                repository.add_instrument(
                    asset_id="BAD-001",
                    family="Unknown",
                    manufacturer="Unknown",
                    model="Unknown",
                    serial_number="BAD",
                    calibration_requirement="required",
                    category_code="missing_category",
                )

    def test_applies_category_migration_to_existing_metrology_database(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            database_path = Path(temporary_directory) / "metrology.sqlite"
            connection = sqlite3.connect(database_path)
            try:
                connection.executescript(
                    Path("storage/sqlite/metrology/0001_metrology_registry.sql").read_text(
                        encoding="utf-8"
                    )
                )
                connection.executemany(
                    """
                    INSERT INTO instruments (
                        asset_id,
                        family,
                        manufacturer,
                        model,
                        serial_number,
                        availability,
                        calibration_requirement,
                        capabilities_json,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        (
                            "LEGACY-001",
                            "Receiver",
                            "Legacy",
                            "Model",
                            "SN-001",
                            "available",
                            "required",
                            "[]",
                            "1970-01-01T00:00:00Z",
                            "1970-01-01T00:00:00Z",
                        ),
                        (
                            "LEGACY-RES",
                            "Receiver",
                            "Legacy",
                            "Model",
                            "SN-RES",
                            "reserved",
                            "required",
                            "[]",
                            "1970-01-01T00:00:00Z",
                            "1970-01-01T00:00:00Z",
                        ),
                        (
                            "LEGACY-OOS",
                            "Receiver",
                            "Legacy",
                            "Model",
                            "SN-OOS",
                            "out_of_service",
                            "required",
                            "[]",
                            "1970-01-01T00:00:00Z",
                            "1970-01-01T00:00:00Z",
                        ),
                    ),
                )
                connection.execute(
                    """
                    INSERT INTO calibration_records (
                        asset_id,
                        certificate_reference,
                        calibrated_at,
                        due_at,
                        provider,
                        status_at_import,
                        uncertainty_json,
                        file_reference,
                        checksum,
                        created_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        "LEGACY-001",
                        "CERT-LEGACY-001",
                        "2026-01-15",
                        "2027-01-15",
                        "legacy lab",
                        "valid",
                        '{"level_db": 0.8}',
                        "legacy/cert.pdf",
                        "sha256:" + "a" * 64,
                        "2026-01-16T00:00:00Z",
                    ),
                )
                connection.commit()
            finally:
                connection.close()

            repository = MetrologyRepository(database_path, Path("storage/sqlite"))
            repository.initialize()
            repository.initialize()

            with closing(repository.connect()) as connection:
                version_rows = connection.execute(
                    "SELECT version FROM schema_migrations ORDER BY version"
                ).fetchall()
                instrument_columns = connection.execute(
                    "PRAGMA table_info(instruments)"
                ).fetchall()
                calibration_events_exists = (
                    connection.execute(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'calibration_events'"
                    ).fetchone()[0]
                    == 1
                )
                metrology_audit_events_exists = (
                    connection.execute(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'metrology_audit_events'"
                    ).fetchone()[0]
                    == 1
                )
                calibration_event_rows = connection.execute(
                    """
                    SELECT *
                    FROM calibration_events
                    WHERE asset_id = ?
                    ORDER BY event_id
                    """,
                    ("LEGACY-001",),
                ).fetchall()

            self.assertEqual([row["version"] for row in version_rows], [1, 2, 3, 4, 5, 6, 7])
            self.assertIn("category_code", {row["name"] for row in instrument_columns})
            self.assertIn("part_number", {row["name"] for row in instrument_columns})
            self.assertIn("calibration_period_months", {row["name"] for row in instrument_columns})
            self.assertIn(
                "calibration_due_warning_days",
                {row["name"] for row in instrument_columns},
            )
            self.assertIn("serviceability_status", {row["name"] for row in instrument_columns})
            self.assertIn("legacy_availability", {row["name"] for row in instrument_columns})
            self.assertTrue(calibration_events_exists)
            self.assertTrue(metrology_audit_events_exists)
            self.assertEqual(repository.category_count(), 34)
            self.assertIsNone(repository.get_instrument("LEGACY-001")["category_code"])
            self.assertEqual(
                repository.get_instrument("LEGACY-001")["serviceability_status"],
                "usable",
            )
            self.assertEqual(
                repository.get_instrument("LEGACY-RES")["legacy_availability"],
                "reserved",
            )
            self.assertEqual(
                repository.get_instrument("LEGACY-RES")["serviceability_status"],
                "usable",
            )
            self.assertIn(
                "legacy reservation",
                repository.get_instrument("LEGACY-RES")["serviceability_reason"],
            )
            self.assertEqual(
                repository.get_instrument("LEGACY-OOS")["serviceability_status"],
                "out_of_service",
            )
            self.assertEqual(len(calibration_event_rows), 1)
            self.assertEqual(calibration_event_rows[0]["event_id"], "legacy-calibration-0001")
            self.assertEqual(calibration_event_rows[0]["decision"], "conforming")
            self.assertEqual(
                calibration_event_rows[0]["uncertainty_summary_json"],
                '{"level_db": 0.8}',
            )
            manifest = json.loads(calibration_event_rows[0]["document_manifest_json"])
            self.assertEqual(manifest["local_reference"], "legacy/cert.pdf")
            self.assertEqual(manifest["sha256"], "a" * 64)


class ProjectRepositoryScheduleTests(unittest.TestCase):
    def test_repository_rejects_weekend_service_schedule_blocks(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-WEEKEND",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "business days"):
                projects.add_service_schedule_item(
                    item_code="PLAN-REPO-WEEKEND",
                    project_code="CEM-REPO-WEEKEND",
                    title="Weekend repository bypass attempt",
                    planned_start_at="2026-07-04T09:00",
                    planned_end_at="2026-07-04T12:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_repository_rejects_multi_day_service_schedule_blocks(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-SCH",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "one business day"):
                projects.add_service_schedule_item(
                    item_code="PLAN-REPO-MULTIDAY",
                    project_code="CEM-REPO-SCH",
                    title="Repository bypass attempt",
                    planned_start_at="2026-07-02T14:00",
                    planned_end_at="2026-07-03T10:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_repository_rejects_non_canonical_schedule_datetimes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-DATETIME",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "YYYY-MM-DDTHH:MM"):
                projects.add_service_schedule_item(
                    item_code="PLAN-REPO-WEEKDATE",
                    project_code="CEM-REPO-DATETIME",
                    title="Week date bypass attempt",
                    planned_start_at="2026-W27-3T09:00",
                    planned_end_at="2026-07-01T12:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_repository_rejects_non_positive_service_schedule_duration(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-DURATION",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "after planned_start_at"):
                projects.add_service_schedule_item(
                    item_code="PLAN-REPO-ZERO-DURATION",
                    project_code="CEM-REPO-DURATION",
                    title="Zero duration bypass attempt",
                    planned_start_at="2026-07-01T09:00",
                    planned_end_at="2026-07-01T09:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_repository_rejects_missing_project_service_schedule_items(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()

            with self.assertRaisesRegex(ValueError, "project does not exist"):
                projects.add_service_schedule_item(
                    item_code="PLAN-REPO-MISSING-PROJECT",
                    project_code="CEM-REPO-MISSING",
                    title="Missing project bypass attempt",
                    planned_start_at="2026-07-01T09:00",
                    planned_end_at="2026-07-01T12:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_repository_rejects_service_schedule_before_test_planning(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-SCHEDULE-STAGE",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="contract_review",
            )

            with self.assertRaisesRegex(ValueError, "test_planning"):
                projects.add_service_schedule_item(
                    item_code="PLAN-REPO-SCHEDULE-STAGE",
                    project_code="CEM-REPO-SCHEDULE-STAGE",
                    title="Premature planning attempt",
                    planned_start_at="2026-07-01T09:00",
                    planned_end_at="2026-07-01T12:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_repository_rejects_unknown_service_schedule_status_on_insert(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-STATUS",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "unknown service schedule status"):
                projects.add_service_schedule_item(
                    item_code="PLAN-REPO-BAD-STATUS",
                    project_code="CEM-REPO-STATUS",
                    title="Invalid status bypass attempt",
                    planned_start_at="2026-07-01T09:00",
                    planned_end_at="2026-07-01T12:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                    status="waiting_for_parts",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_repository_rejects_non_planned_service_schedule_initial_status(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-INITIAL-STATUS",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "initial status must be planned"):
                projects.add_service_schedule_item(
                    item_code="PLAN-REPO-SKIPPED-INITIAL",
                    project_code="CEM-REPO-INITIAL-STATUS",
                    title="Skipped initial status attempt",
                    planned_start_at="2026-07-01T09:00",
                    planned_end_at="2026-07-01T12:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                    status="confirmed",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_repository_normalizes_service_schedule_status_text(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-STATUS-NORMALIZE",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            projects.add_service_schedule_item(
                item_code="PLAN-REPO-STATUS-NORMALIZE",
                project_code="CEM-REPO-STATUS-NORMALIZE",
                title="Normalized status text",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
                status=" planned ",
            )
            projects.update_service_schedule_status(
                item_code="PLAN-REPO-STATUS-NORMALIZE",
                status=" confirmed ",
            )

            schedule = projects.list_service_schedule_items(
                project_code="CEM-REPO-STATUS-NORMALIZE",
                status=" confirmed ",
            )
            self.assertEqual(len(schedule), 1)
            self.assertEqual(schedule[0]["status"], "confirmed")

    def test_repository_rejects_duplicate_service_schedule_item_code_on_insert(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-DUPLICATE-ITEM",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            projects.add_service_schedule_item(
                item_code="PLAN-REPO-DUPLICATE-ITEM",
                project_code="CEM-REPO-DUPLICATE-ITEM",
                title="First duplicate guard",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
            )

            with self.assertRaisesRegex(
                ValueError,
                "service schedule item already exists",
            ):
                projects.add_service_schedule_item(
                    item_code="PLAN-REPO-DUPLICATE-ITEM",
                    project_code="CEM-REPO-DUPLICATE-ITEM",
                    title="Second duplicate guard",
                    planned_start_at="2026-07-01T13:00",
                    planned_end_at="2026-07-01T15:00",
                    assigned_operator="operator.two",
                    location="Lab B",
                    equipment_under_test="EUT rail",
                )

            schedule = projects.list_service_schedule_items(
                project_code="CEM-REPO-DUPLICATE-ITEM",
            )
            self.assertEqual(len(schedule), 1)
            self.assertEqual(schedule[0]["title"], "First duplicate guard")

    def test_repository_rejects_unknown_service_schedule_status_on_update(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-STATUS-UPDATE",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            projects.add_service_schedule_item(
                item_code="PLAN-REPO-STATUS-UPDATE",
                project_code="CEM-REPO-STATUS-UPDATE",
                title="Status update guard",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
            )

            with self.assertRaisesRegex(ValueError, "unknown service schedule status"):
                projects.update_service_schedule_status(
                    item_code="PLAN-REPO-STATUS-UPDATE",
                    status="waiting_for_parts",
                )

            schedule = projects.list_service_schedule_items(
                project_code="CEM-REPO-STATUS-UPDATE",
            )
            self.assertEqual(schedule[0]["status"], "planned")

    def test_repository_rejects_unchanged_service_schedule_status_update(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-STATUS-UNCHANGED",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            projects.add_service_schedule_item(
                item_code="PLAN-REPO-STATUS-UNCHANGED",
                project_code="CEM-REPO-STATUS-UNCHANGED",
                title="Unchanged status guard",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
            )

            with self.assertRaisesRegex(ValueError, "status is unchanged"):
                projects.update_service_schedule_status(
                    item_code="PLAN-REPO-STATUS-UNCHANGED",
                    status="planned",
                )

            schedule = projects.list_service_schedule_items(
                project_code="CEM-REPO-STATUS-UNCHANGED",
            )
            self.assertEqual(schedule[0]["status"], "planned")

    def test_repository_rejects_terminal_service_schedule_status_update(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-STATUS-TERMINAL",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            projects.add_service_schedule_item(
                item_code="PLAN-REPO-STATUS-TERMINAL",
                project_code="CEM-REPO-STATUS-TERMINAL",
                title="Terminal status guard",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
            )
            projects.update_service_schedule_status(
                item_code="PLAN-REPO-STATUS-TERMINAL",
                status="confirmed",
            )
            projects.update_service_schedule_status(
                item_code="PLAN-REPO-STATUS-TERMINAL",
                status="in_progress",
            )
            projects.update_service_schedule_status(
                item_code="PLAN-REPO-STATUS-TERMINAL",
                status="completed",
            )

            with self.assertRaisesRegex(ValueError, "status is terminal"):
                projects.update_service_schedule_status(
                    item_code="PLAN-REPO-STATUS-TERMINAL",
                    status="in_progress",
                )

            schedule = projects.list_service_schedule_items(
                project_code="CEM-REPO-STATUS-TERMINAL",
            )
            self.assertEqual(schedule[0]["status"], "completed")

    def test_repository_rejects_backward_service_schedule_status_update(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-STATUS-BACKWARD",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            projects.add_service_schedule_item(
                item_code="PLAN-REPO-STATUS-BACKWARD",
                project_code="CEM-REPO-STATUS-BACKWARD",
                title="Backward status guard",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
            )
            projects.update_service_schedule_status(
                item_code="PLAN-REPO-STATUS-BACKWARD",
                status="confirmed",
            )

            with self.assertRaisesRegex(
                ValueError,
                "invalid service schedule status transition",
            ):
                projects.update_service_schedule_status(
                    item_code="PLAN-REPO-STATUS-BACKWARD",
                    status="planned",
                )

            schedule = projects.list_service_schedule_items(
                project_code="CEM-REPO-STATUS-BACKWARD",
            )
            self.assertEqual(schedule[0]["status"], "confirmed")

    def test_repository_rejects_empty_service_schedule_item_code_on_update(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()

            with self.assertRaisesRegex(ValueError, "item_code must not be empty"):
                projects.update_service_schedule_status(
                    item_code="  ",
                    status="confirmed",
                )

    def test_repository_rejects_unknown_service_schedule_item_on_update(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()

            with self.assertRaisesRegex(
                ValueError,
                "service schedule item does not exist",
            ):
                projects.update_service_schedule_status(
                    item_code="PLAN-REPO-MISSING-UPDATE",
                    status="confirmed",
                )

    def test_repository_rejects_orphan_service_schedule_item_on_update(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            with closing(sqlite3.connect(projects.database_path)) as connection:
                connection.execute("PRAGMA foreign_keys = OFF")
                connection.execute(
                    """
                    INSERT INTO service_schedule_items (
                        item_code,
                        project_code,
                        title,
                        planned_start_at,
                        planned_end_at,
                        assigned_operator,
                        location,
                        equipment_under_test,
                        status,
                        notes,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        "PLAN-REPO-ORPHAN-UPDATE",
                        "CEM-REPO-MISSING-PROJECT",
                        "Orphan status update guard",
                        "2026-07-01T09:00",
                        "2026-07-01T12:00",
                        "operator.one",
                        "Lab A",
                        "EUT rail",
                        "planned",
                        "",
                        "2026-07-01T08:00:00Z",
                        "2026-07-01T08:00:00Z",
                    ),
                )
                connection.commit()

            with self.assertRaisesRegex(
                ValueError,
                "service schedule project does not exist",
            ):
                projects.update_service_schedule_status(
                    item_code="PLAN-REPO-ORPHAN-UPDATE",
                    status="confirmed",
                )

    def test_repository_rejects_orphan_service_schedule_item_on_list(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            with closing(sqlite3.connect(projects.database_path)) as connection:
                connection.execute("PRAGMA foreign_keys = OFF")
                connection.execute(
                    """
                    INSERT INTO service_schedule_items (
                        item_code,
                        project_code,
                        title,
                        planned_start_at,
                        planned_end_at,
                        assigned_operator,
                        location,
                        equipment_under_test,
                        status,
                        notes,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        "PLAN-REPO-ORPHAN-LIST",
                        "CEM-REPO-MISSING-PROJECT",
                        "Orphan schedule list guard",
                        "2026-07-01T09:00",
                        "2026-07-01T12:00",
                        "operator.one",
                        "Lab A",
                        "EUT rail",
                        "planned",
                        "",
                        "2026-07-01T08:00:00Z",
                        "2026-07-01T08:00:00Z",
                    ),
                )
                connection.commit()

            with self.assertRaisesRegex(
                ValueError,
                "service schedule project does not exist",
            ):
                projects.list_service_schedule_items()

    def test_repository_rejects_unknown_service_schedule_status_on_list(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-CORRUPT-STATUS",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            with closing(sqlite3.connect(projects.database_path)) as connection:
                connection.execute("PRAGMA ignore_check_constraints = ON")
                connection.execute(
                    """
                    INSERT INTO service_schedule_items (
                        item_code,
                        project_code,
                        title,
                        planned_start_at,
                        planned_end_at,
                        assigned_operator,
                        location,
                        equipment_under_test,
                        status,
                        notes,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        "PLAN-REPO-CORRUPT-STATUS",
                        "CEM-REPO-CORRUPT-STATUS",
                        "Corrupted schedule status guard",
                        "2026-07-01T09:00",
                        "2026-07-01T12:00",
                        "operator.one",
                        "Lab A",
                        "EUT rail",
                        "waiting_for_parts",
                        "",
                        "2026-07-01T08:00:00Z",
                        "2026-07-01T08:00:00Z",
                    ),
                )
                connection.commit()

            with self.assertRaisesRegex(
                ValueError,
                "unknown service schedule status",
            ):
                projects.list_service_schedule_items()

    def test_repository_normalizes_service_schedule_status_on_list(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-CORRUPT-STATUS-NORM",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            with closing(sqlite3.connect(projects.database_path)) as connection:
                connection.execute("PRAGMA ignore_check_constraints = ON")
                connection.execute(
                    """
                    INSERT INTO service_schedule_items (
                        item_code,
                        project_code,
                        title,
                        planned_start_at,
                        planned_end_at,
                        assigned_operator,
                        location,
                        equipment_under_test,
                        status,
                        notes,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        "PLAN-REPO-CORRUPT-STATUS-NORM",
                        "CEM-REPO-CORRUPT-STATUS-NORM",
                        "Corrupted schedule status normalization",
                        "2026-07-01T09:00",
                        "2026-07-01T12:00",
                        "operator.one",
                        "Lab A",
                        "EUT rail",
                        " confirmed ",
                        "",
                        "2026-07-01T08:00:00Z",
                        "2026-07-01T08:00:00Z",
                    ),
                )
                connection.commit()

            schedule = projects.list_service_schedule_items()

            self.assertEqual(schedule[0]["status"], "confirmed")

    def test_repository_rejects_invalid_service_schedule_block_on_list(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-CORRUPT-BLOCK",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            with closing(sqlite3.connect(projects.database_path)) as connection:
                connection.execute(
                    """
                    INSERT INTO service_schedule_items (
                        item_code,
                        project_code,
                        title,
                        planned_start_at,
                        planned_end_at,
                        assigned_operator,
                        location,
                        equipment_under_test,
                        status,
                        notes,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        "PLAN-REPO-CORRUPT-BLOCK",
                        "CEM-REPO-CORRUPT-BLOCK",
                        "Corrupted schedule block guard",
                        "2026-07-04T09:00",
                        "2026-07-04T12:00",
                        "operator.one",
                        "Lab A",
                        "EUT rail",
                        "planned",
                        "",
                        "2026-07-01T08:00:00Z",
                        "2026-07-01T08:00:00Z",
                    ),
                )
                connection.commit()

            with self.assertRaisesRegex(
                ValueError,
                "business days",
            ):
                projects.list_service_schedule_items()

    def test_repository_rejects_empty_service_schedule_required_text_on_list(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-CORRUPT-TEXT",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            with closing(sqlite3.connect(projects.database_path)) as connection:
                connection.execute(
                    """
                    INSERT INTO service_schedule_items (
                        item_code,
                        project_code,
                        title,
                        planned_start_at,
                        planned_end_at,
                        assigned_operator,
                        location,
                        equipment_under_test,
                        status,
                        notes,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        "PLAN-REPO-CORRUPT-TEXT",
                        "CEM-REPO-CORRUPT-TEXT",
                        "Corrupted schedule text guard",
                        "2026-07-01T09:00",
                        "2026-07-01T12:00",
                        "  ",
                        "Lab A",
                        "EUT rail",
                        "planned",
                        "",
                        "2026-07-01T08:00:00Z",
                        "2026-07-01T08:00:00Z",
                    ),
                )
                connection.commit()

            with self.assertRaisesRegex(
                ValueError,
                "assigned_operator must not be empty",
            ):
                projects.list_service_schedule_items()

    def test_repository_normalizes_required_service_schedule_text_on_list(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-CORRUPT-REQUIRED",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            with closing(sqlite3.connect(projects.database_path)) as connection:
                connection.execute(
                    """
                    INSERT INTO service_schedule_items (
                        item_code,
                        project_code,
                        title,
                        planned_start_at,
                        planned_end_at,
                        assigned_operator,
                        location,
                        equipment_under_test,
                        status,
                        notes,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        " PLAN-REPO-CORRUPT-REQUIRED ",
                        "CEM-REPO-CORRUPT-REQUIRED",
                        "  Corrupted required text guard  ",
                        "2026-07-01T09:00",
                        "2026-07-01T12:00",
                        " operator.one ",
                        " Lab A ",
                        " EUT rail ",
                        "planned",
                        "",
                        "2026-07-01T08:00:00Z",
                        "2026-07-01T08:00:00Z",
                    ),
                )
                connection.commit()

            schedule = projects.list_service_schedule_items()

            self.assertEqual(schedule[0]["item_code"], "PLAN-REPO-CORRUPT-REQUIRED")
            self.assertEqual(schedule[0]["title"], "Corrupted required text guard")
            self.assertEqual(schedule[0]["assigned_operator"], "operator.one")
            self.assertEqual(schedule[0]["location"], "Lab A")
            self.assertEqual(schedule[0]["equipment_under_test"], "EUT rail")

    def test_repository_normalizes_optional_service_schedule_text_on_list(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-CORRUPT-OPTIONAL",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            with closing(sqlite3.connect(projects.database_path)) as connection:
                connection.execute(
                    """
                    INSERT INTO service_schedule_items (
                        item_code,
                        project_code,
                        title,
                        test_category_code,
                        test_method_code,
                        planned_start_at,
                        planned_end_at,
                        assigned_operator,
                        location,
                        equipment_under_test,
                        status,
                        notes,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        "PLAN-REPO-CORRUPT-OPTIONAL",
                        "CEM-REPO-CORRUPT-OPTIONAL",
                        "Corrupted optional text guard",
                        "  ",
                        " method_conducted ",
                        "2026-07-01T09:00",
                        "2026-07-01T12:00",
                        "operator.one",
                        "Lab A",
                        "EUT rail",
                        "planned",
                        "  ",
                        "2026-07-01T08:00:00Z",
                        "2026-07-01T08:00:00Z",
                    ),
                )
                connection.commit()

            schedule = projects.list_service_schedule_items()

            self.assertIsNone(schedule[0]["test_category_code"])
            self.assertEqual(schedule[0]["test_method_code"], "method_conducted")
            self.assertEqual(schedule[0]["notes"], "")

    def test_repository_rejects_empty_service_schedule_item_code_on_insert(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-EMPTY-ITEM",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "item_code must not be empty"):
                projects.add_service_schedule_item(
                    item_code="  ",
                    project_code="CEM-REPO-EMPTY-ITEM",
                    title="Empty item code bypass attempt",
                    planned_start_at="2026-07-01T09:00",
                    planned_end_at="2026-07-01T12:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_repository_rejects_missing_service_schedule_item_code_on_insert(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-MISSING-ITEM",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "item_code must not be empty"):
                projects.add_service_schedule_item(
                    item_code=None,
                    project_code="CEM-REPO-MISSING-ITEM",
                    title="Missing item code bypass attempt",
                    planned_start_at="2026-07-01T09:00",
                    planned_end_at="2026-07-01T12:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_repository_rejects_empty_service_schedule_operator(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-EMPTY-SCHEDULE",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "assigned_operator must not be empty"):
                projects.add_service_schedule_item(
                    item_code="PLAN-REPO-EMPTY-OPERATOR",
                    project_code="CEM-REPO-EMPTY-SCHEDULE",
                    title="Empty operator bypass attempt",
                    planned_start_at="2026-07-01T09:00",
                    planned_end_at="2026-07-01T12:00",
                    assigned_operator="  ",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_repository_normalizes_optional_service_schedule_references(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-OPTIONAL-REFS",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            projects.add_service_schedule_item(
                item_code="PLAN-REPO-OPTIONAL-REFS",
                project_code="CEM-REPO-OPTIONAL-REFS",
                title="Optional reference guard",
                test_category_code=" emission_conducted ",
                test_method_code="  ",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
            )

            schedule = projects.list_service_schedule_items(
                project_code="CEM-REPO-OPTIONAL-REFS",
            )
            self.assertEqual(schedule[0]["test_category_code"], "emission_conducted")
            self.assertIsNone(schedule[0]["test_method_code"])

    def test_repository_normalizes_optional_service_schedule_notes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-NOTES",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            projects.add_service_schedule_item(
                item_code="PLAN-REPO-NOTES",
                project_code="CEM-REPO-NOTES",
                title="Optional notes guard",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
                notes=None,
            )

            schedule = projects.list_service_schedule_items(project_code="CEM-REPO-NOTES")
            self.assertEqual(schedule[0]["notes"], "")

    def test_repository_records_service_schedule_item_with_project_audit(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-SCHEDULE-AUDIT",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            schedule_id, audit_sequence = projects.add_service_schedule_item_with_audit(
                item_code="PLAN-REPO-SCHEDULE-AUDIT",
                project_code="CEM-REPO-SCHEDULE-AUDIT",
                title="Audited planning row",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
                actor="operator.one",
                reason="Planned from operator console",
            )

            schedule = projects.list_service_schedule_items(
                project_code="CEM-REPO-SCHEDULE-AUDIT",
            )
            events = projects.audit_events("CEM-REPO-SCHEDULE-AUDIT")
            payload = json.loads(events[0]["payload_json"])

            self.assertGreater(schedule_id, 0)
            self.assertEqual(audit_sequence, 1)
            self.assertEqual(schedule[0]["item_code"], "PLAN-REPO-SCHEDULE-AUDIT")
            self.assertEqual(events[0]["sequence"], 1)
            self.assertEqual(events[0]["actor"], "operator.one")
            self.assertEqual(events[0]["action"], "service_schedule_item_planned")
            self.assertEqual(events[0]["reason"], "Planned from operator console")
            self.assertEqual(payload["item_code"], "PLAN-REPO-SCHEDULE-AUDIT")
            self.assertEqual(payload["planned_start_at"], "2026-07-01T09:00")
            self.assertEqual(payload["status"], "planned")

    def test_repository_records_service_schedule_status_update_with_project_audit(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-SCHEDULE-STATUS-AUDIT",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            projects.add_service_schedule_item(
                item_code="PLAN-REPO-SCHEDULE-STATUS-AUDIT",
                project_code="CEM-REPO-SCHEDULE-STATUS-AUDIT",
                title="Audited status update",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
            )

            audit_sequence = projects.update_service_schedule_status_with_audit(
                item_code="PLAN-REPO-SCHEDULE-STATUS-AUDIT",
                status="confirmed",
                actor="operator.two",
                reason="Operator confirmed laboratory slot",
            )

            schedule = projects.list_service_schedule_items(
                project_code="CEM-REPO-SCHEDULE-STATUS-AUDIT",
            )
            events = projects.audit_events("CEM-REPO-SCHEDULE-STATUS-AUDIT")
            payload = json.loads(events[0]["payload_json"])

            self.assertEqual(audit_sequence, 1)
            self.assertEqual(schedule[0]["status"], "confirmed")
            self.assertEqual(events[0]["sequence"], 1)
            self.assertEqual(events[0]["actor"], "operator.two")
            self.assertEqual(events[0]["action"], "service_schedule_item_status_updated")
            self.assertEqual(events[0]["reason"], "Operator confirmed laboratory slot")
            self.assertEqual(payload["item_code"], "PLAN-REPO-SCHEDULE-STATUS-AUDIT")
            self.assertEqual(payload["previous_status"], "planned")
            self.assertEqual(payload["new_status"], "confirmed")

    def test_repository_rejects_unchanged_service_schedule_status_audit(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-STATUS-AUDIT-UNCHANGED",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            projects.add_service_schedule_item(
                item_code="PLAN-REPO-STATUS-AUDIT-UNCHANGED",
                project_code="CEM-REPO-STATUS-AUDIT-UNCHANGED",
                title="Unchanged audited status update",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
            )

            with self.assertRaisesRegex(ValueError, "status is unchanged"):
                projects.update_service_schedule_status_with_audit(
                    item_code="PLAN-REPO-STATUS-AUDIT-UNCHANGED",
                    status="planned",
                    actor="operator.two",
                    reason="Duplicate confirmation",
                )

            schedule = projects.list_service_schedule_items(
                project_code="CEM-REPO-STATUS-AUDIT-UNCHANGED",
            )
            events = projects.audit_events("CEM-REPO-STATUS-AUDIT-UNCHANGED")

            self.assertEqual(schedule[0]["status"], "planned")
            self.assertEqual(events, [])

    def test_repository_rejects_terminal_service_schedule_status_audit(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-STATUS-AUDIT-TERMINAL",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            projects.add_service_schedule_item(
                item_code="PLAN-REPO-STATUS-AUDIT-TERMINAL",
                project_code="CEM-REPO-STATUS-AUDIT-TERMINAL",
                title="Terminal audited status update",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
            )
            projects.update_service_schedule_status_with_audit(
                item_code="PLAN-REPO-STATUS-AUDIT-TERMINAL",
                status="cancelled",
                actor="operator.two",
                reason="Slot cancelled",
            )

            with self.assertRaisesRegex(ValueError, "status is terminal"):
                projects.update_service_schedule_status_with_audit(
                    item_code="PLAN-REPO-STATUS-AUDIT-TERMINAL",
                    status="confirmed",
                    actor="operator.two",
                    reason="Reopen cancelled slot",
                )

            schedule = projects.list_service_schedule_items(
                project_code="CEM-REPO-STATUS-AUDIT-TERMINAL",
            )
            events = projects.audit_events("CEM-REPO-STATUS-AUDIT-TERMINAL")

            self.assertEqual(schedule[0]["status"], "cancelled")
            self.assertEqual(len(events), 1)
            self.assertEqual(events[0]["action"], "service_schedule_item_status_updated")

    def test_repository_rejects_backward_service_schedule_status_audit(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-STATUS-AUDIT-BACKWARD",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            projects.add_service_schedule_item(
                item_code="PLAN-REPO-STATUS-AUDIT-BACKWARD",
                project_code="CEM-REPO-STATUS-AUDIT-BACKWARD",
                title="Backward audited status update",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
            )
            projects.update_service_schedule_status(
                item_code="PLAN-REPO-STATUS-AUDIT-BACKWARD",
                status="confirmed",
            )
            projects.update_service_schedule_status(
                item_code="PLAN-REPO-STATUS-AUDIT-BACKWARD",
                status="in_progress",
            )

            with self.assertRaisesRegex(
                ValueError,
                "invalid service schedule status transition",
            ):
                projects.update_service_schedule_status_with_audit(
                    item_code="PLAN-REPO-STATUS-AUDIT-BACKWARD",
                    status="confirmed",
                    actor="operator.two",
                    reason="Move back to confirmed",
                )

            schedule = projects.list_service_schedule_items(
                project_code="CEM-REPO-STATUS-AUDIT-BACKWARD",
            )
            events = projects.audit_events("CEM-REPO-STATUS-AUDIT-BACKWARD")

            self.assertEqual(schedule[0]["status"], "in_progress")
            self.assertEqual(events, [])

    def test_repository_normalizes_service_schedule_list_filters(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()
            projects.create_project(
                code="CEM-REPO-LIST-FILTERS",
                customer_name="Repository Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            projects.add_service_schedule_item(
                item_code="PLAN-REPO-LIST-FILTERS",
                project_code="CEM-REPO-LIST-FILTERS",
                title="List filter guard",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
            )
            projects.update_service_schedule_status(
                item_code="PLAN-REPO-LIST-FILTERS",
                status="confirmed",
            )

            schedule = projects.list_service_schedule_items(
                project_code=" CEM-REPO-LIST-FILTERS ",
                status=" confirmed ",
            )

            self.assertEqual(len(schedule), 1)
            self.assertEqual(schedule[0]["item_code"], "PLAN-REPO-LIST-FILTERS")

    def test_repository_rejects_malformed_service_schedule_list_filters(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()

            with self.assertRaisesRegex(ValueError, "project_code must not be empty"):
                projects.list_service_schedule_items(project_code="  ")

            with self.assertRaisesRegex(ValueError, "unknown service schedule status"):
                projects.list_service_schedule_items(status="waiting_for_parts")

    def test_repository_rejects_unknown_service_schedule_project_filter(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects = ProjectRepository(
                Path(temporary_directory) / "projects.sqlite",
                Path("storage/sqlite"),
            )
            projects.initialize()

            with self.assertRaisesRegex(ValueError, "project does not exist"):
                projects.list_service_schedule_items(
                    project_code="CEM-REPO-MISSING-FILTER",
                )


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
                stage="test_planning",
            )
            projects.complete_contract_review_item(
                project_code="CEM-BOOT-001",
                item="requirements_reviewed",
                completed_by="quality.boot",
                comment="Accepted investigation scope",
            )
            projects.add_service_schedule_item(
                item_code="PLAN-BOOT-001",
                project_code="CEM-BOOT-001",
                title="Inrush current capture",
                test_category_code="emission_transient_time_domain",
                planned_start_at="2026-07-03T09:00",
                planned_end_at="2026-07-03T11:00",
                assigned_operator="operator.boot",
                location="Lab B",
                equipment_under_test="EUT boot",
            )
            projects.set_project_stage_with_audit(
                code="CEM-BOOT-001",
                stage="measuring",
                actor="operator.boot",
                reason="Fixture project already entered measurement",
            )
            metrology.add_instrument(
                asset_id="DAQ-001",
                family="DAQ",
                manufacturer="Open",
                model="DAQ",
                serial_number="001",
                calibration_requirement="required",
                capabilities_json='{"channels": 8}',
                category_code="daq_chassis",
                part_number="ODAQ-8",
                calibration_period_months=12,
            )
            metrology.add_calibration_record(
                asset_id="DAQ-001",
                certificate_reference="CERT-001",
                calibrated_at="2026-01-01",
                due_at="2027-01-01",
                provider="Metrology Lab",
            )
            metrology.add_instrument_document(
                asset_id="DAQ-001",
                document_kind="script",
                title="DAQ setup script",
                file_reference="scripts/daq/setup.py",
                uploaded_by="operator.boot",
            )
            test_definitions.add_test_method(
                code="INRUSH-001",
                standard_code=None,
                name="Inrush current",
                family="inrush",
                measurement_axis="time_series",
                category_code="emission_transient_time_domain",
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
            measurement_data.record_instrument_observation(
                project_code="CEM-BOOT-001",
                campaign_reference="CAMP-BOOT-001",
                measurement_run_reference="RUN-BOOT-001",
                sequence=1,
                instrument_code="DAQ-001",
                transport="simulated",
                endpoint="SIM::DAQ-001",
                command_message="READ?",
                response_message="OK",
                success=True,
                exchange_attempts=2,
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
            self.assertEqual(payload["contract_review_items"][0][0], "CEM-BOOT-001")
            self.assertEqual(payload["contract_review_items"][0][1], "requirements_reviewed")
            self.assertEqual(payload["contract_review_items"][0][2], "yes")
            self.assertEqual(payload["instruments"][0][0], "DAQ-001")
            self.assertEqual(payload["instruments"][0][2], "Usable")
            self.assertEqual(payload["instruments"][0][3], "Available")
            self.assertEqual(payload["instruments"][0][6], "ok")
            self.assertEqual(payload["instruments"][0][7], "DAQ chassis and modules")
            self.assertEqual(payload["instruments"][0][8], "channels=8")
            self.assertEqual(payload["instruments"][0][12], "ODAQ-8")
            self.assertEqual(payload["instruments"][0][13], "2026-01-01")
            self.assertEqual(payload["instruments"][0][14], "12")
            self.assertEqual(payload["instruments"][0][15], "1")
            self.assertEqual(payload["instrument_documents"][0][2], "DAQ setup script")
            self.assertEqual(payload["schedule"][0][0], "PLAN-BOOT-001")
            self.assertIn(
                "daq_chassis",
                {row[0] for row in payload["instrument_categories"]},
            )
            self.assertIn(
                "emission_transient_time_domain",
                {row[0] for row in payload["test_categories"]},
            )
            self.assertEqual(payload["methods"][0][3], "approved")
            self.assertEqual(payload["datasets"][0][4], "Immutable")
            self.assertEqual(payload["runtime"][0][0], "DAQ-001")
            self.assertEqual(payload["runtime"][0][3], "OK")
            self.assertEqual(payload["runtime"][0][4], "RUN-BOOT-001")
            self.assertEqual(payload["runtime"][0][5], "1")
            self.assertEqual(payload["runtime"][0][6], "READ? -> OK")
            self.assertEqual(payload["runtime"][0][7], "2")
            self.assertEqual(payload["updates"][0][0], "driver-pack-opendaq")
            self.assertIn("window.EMC_LOCUS_BOOTSTRAP", output_path.read_text())


class GuiActionTests(unittest.TestCase):
    def test_create_project_record_writes_initial_audit_and_refreshes_bootstrap(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            base = Path(temporary_directory)
            projects_db = base / "projects.sqlite"
            bootstrap_output = base / "bootstrap.js"

            result = create_project_record(
                projects_db=projects_db,
                code="CEM-CREATE-001",
                customer_name="Create Customer",
                execution_mode="investigation",
                actor="operator.one",
                reason="Initial field investigation request",
                bootstrap_output=bootstrap_output,
            )

            projects = ProjectRepository(projects_db, Path("storage/sqlite"))
            projects.initialize()
            project = projects.get_project("CEM-CREATE-001")
            events = projects.audit_events("CEM-CREATE-001")
            bootstrap_text = bootstrap_output.read_text()

            self.assertEqual(result["stage"], "quotation")
            self.assertEqual(result["audit_event_id"], events[0]["id"])
            self.assertEqual(project["execution_mode"], "investigation")
            self.assertEqual(events[0]["sequence"], 1)
            self.assertEqual(events[0]["action"], "project_created")
            self.assertIn("Initial field investigation request", events[0]["reason"])
            self.assertIn("CEM-CREATE-001", bootstrap_text)

    def test_complete_contract_review_item_records_audit_and_refreshes_bootstrap(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            base = Path(temporary_directory)
            projects_db = base / "projects.sqlite"
            bootstrap_output = base / "bootstrap.js"
            projects = ProjectRepository(projects_db, Path("storage/sqlite"))
            projects.initialize()
            projects.create_project(
                code="CEM-REVIEW-001",
                customer_name="Review Customer",
                execution_mode="accredited",
                stage="contract_review",
            )

            result = complete_contract_review_item_action(
                projects_db=projects_db,
                project_code="CEM-REVIEW-001",
                item="method_available",
                completed_by="quality.lead",
                comment="Approved method is available",
                bootstrap_output=bootstrap_output,
            )

            items = projects.contract_review_items("CEM-REVIEW-001")
            events = projects.audit_events("CEM-REVIEW-001")
            bootstrap_text = bootstrap_output.read_text()

            self.assertEqual(result["audit_sequence"], 1)
            self.assertEqual(items[0]["item"], "method_available")
            self.assertEqual(items[0]["completed"], 1)
            self.assertEqual(events[0]["action"], "contract_review_item_completed")
            self.assertIn("method_available", events[0]["payload_json"])
            self.assertIn("method_available", bootstrap_text)

    def test_advance_project_stage_records_audit_and_refreshes_bootstrap(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            base = Path(temporary_directory)
            projects_db = base / "projects.sqlite"
            bootstrap_output = base / "bootstrap.js"
            projects = ProjectRepository(projects_db, root)
            projects.initialize()
            projects.create_project(
                code="CEM-ACT-001",
                customer_name="Action Customer",
                execution_mode="accredited",
                stage="quotation",
            )

            result = advance_project_stage(
                projects_db=projects_db,
                code="CEM-ACT-001",
                actor="operator.one",
                reason="Contract review is ready",
                bootstrap_output=bootstrap_output,
            )

            project = projects.get_project("CEM-ACT-001")
            events = projects.audit_events("CEM-ACT-001")
            bootstrap_text = bootstrap_output.read_text()

            self.assertEqual(result["previous_stage"], "quotation")
            self.assertEqual(result["new_stage"], "contract_review")
            self.assertEqual(result["audit_sequence"], 1)
            self.assertEqual(project["stage"], "contract_review")
            self.assertEqual(events[0]["action"], "gui_project_stage_advanced")
            self.assertIn('"from": "quotation"', events[0]["payload_json"])
            self.assertIn("CEM-ACT-001", bootstrap_text)
            self.assertIn("Contract review", bootstrap_text)

    def test_accredited_project_cannot_enter_planning_without_contract_review_items(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects_db = Path(temporary_directory) / "projects.sqlite"
            projects = ProjectRepository(projects_db, Path("storage/sqlite"))
            projects.initialize()
            projects.create_project(
                code="CEM-GATE-ACC",
                customer_name="Gate Customer",
                execution_mode="accredited",
                stage="contract_review",
            )

            with self.assertRaisesRegex(ValueError, "requirements_reviewed"):
                advance_project_stage(
                    projects_db=projects_db,
                    code="CEM-GATE-ACC",
                    actor="quality.lead",
                    reason="Trying to enter planning too early",
                )

            for item in (
                "requirements_reviewed",
                "method_available",
                "resources_available",
                "impartiality_risk_reviewed",
            ):
                complete_contract_review_item_action(
                    projects_db=projects_db,
                    project_code="CEM-GATE-ACC",
                    item=item,
                    completed_by="quality.lead",
                    comment="Required for accredited planning",
                )

            result = advance_project_stage(
                projects_db=projects_db,
                code="CEM-GATE-ACC",
                actor="quality.lead",
                reason="Contract review checklist complete",
            )

            self.assertEqual(result["new_stage"], "test_planning")

    def test_investigation_project_uses_reduced_contract_review_gate(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            projects_db = Path(temporary_directory) / "projects.sqlite"
            projects = ProjectRepository(projects_db, Path("storage/sqlite"))
            projects.initialize()
            projects.create_project(
                code="CEM-GATE-INV",
                customer_name="Investigation Customer",
                execution_mode="investigation",
                stage="contract_review",
            )

            with self.assertRaisesRegex(ValueError, "investigation_goal_defined"):
                advance_project_stage(
                    projects_db=projects_db,
                    code="CEM-GATE-INV",
                    actor="operator.one",
                    reason="Need an investigation goal",
                )

            complete_contract_review_item_action(
                projects_db=projects_db,
                project_code="CEM-GATE-INV",
                item="investigation_goal_defined",
                completed_by="operator.one",
                comment="Find root cause of transient reset",
            )
            result = advance_project_stage(
                projects_db=projects_db,
                code="CEM-GATE-INV",
                actor="operator.one",
                reason="Investigation gate complete",
            )

            self.assertEqual(result["new_stage"], "test_planning")

    def test_next_project_stage_rejects_unknown_stage(self) -> None:
        with self.assertRaises(ValueError):
            next_project_stage("unknown")

    def test_schedule_service_item_records_planned_test_and_refreshes_bootstrap(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            base = Path(temporary_directory)
            projects_db = base / "projects.sqlite"
            bootstrap_output = base / "bootstrap.js"
            projects = ProjectRepository(projects_db, root)
            projects.initialize()
            projects.create_project(
                code="CEM-SCH-001",
                customer_name="Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            result = schedule_service_item(
                projects_db=projects_db,
                item_code="PLAN-SCH-001",
                project_code="CEM-SCH-001",
                title="Emission conduite",
                test_category_code="emission_conducted",
                test_method_code="EN55032-CE",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
                bootstrap_output=bootstrap_output,
            )

            schedule = projects.list_service_schedule_items(project_code="CEM-SCH-001")
            bootstrap_text = bootstrap_output.read_text()

            events = projects.audit_events("CEM-SCH-001")
            self.assertEqual(result["item_code"], "PLAN-SCH-001")
            self.assertEqual(result["audit_sequence"], 1)
            self.assertEqual(schedule[0]["title"], "Emission conduite")
            self.assertEqual(schedule[0]["status"], "planned")
            self.assertEqual(events[0]["action"], "service_schedule_item_planned")
            self.assertIn("PLAN-SCH-001", events[0]["payload_json"])
            self.assertIn("PLAN-SCH-001", bootstrap_text)
            self.assertIn("Emission conduite", bootstrap_text)

    def test_schedule_service_item_rejects_non_planned_initial_status(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            projects_db = Path(temporary_directory) / "projects.sqlite"
            projects = ProjectRepository(projects_db, root)
            projects.initialize()
            projects.create_project(
                code="CEM-SCH-INITIAL-STATUS",
                customer_name="Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "initial status must be planned"):
                schedule_service_item(
                    projects_db=projects_db,
                    item_code="PLAN-SCH-SKIPPED-INITIAL",
                    project_code="CEM-SCH-INITIAL-STATUS",
                    title="Skipped initial status",
                    planned_start_at="2026-07-01T09:00",
                    planned_end_at="2026-07-01T12:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                    status="confirmed",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_schedule_service_item_rejects_unknown_test_category_reference(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            base = Path(temporary_directory)
            projects_db = base / "projects.sqlite"
            test_definitions_db = base / "test_definitions.sqlite"
            projects = ProjectRepository(projects_db, root)
            projects.initialize()
            projects.create_project(
                code="CEM-SCH-BAD-CATEGORY",
                customer_name="Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "test category does not exist"):
                schedule_service_item(
                    projects_db=projects_db,
                    test_definitions_db=test_definitions_db,
                    item_code="PLAN-SCH-BAD-CATEGORY",
                    project_code="CEM-SCH-BAD-CATEGORY",
                    title="Unknown category",
                    test_category_code="missing_category",
                    planned_start_at="2026-07-01T09:00",
                    planned_end_at="2026-07-01T12:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_schedule_service_item_rejects_unknown_test_method_reference(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            base = Path(temporary_directory)
            projects_db = base / "projects.sqlite"
            test_definitions_db = base / "test_definitions.sqlite"
            projects = ProjectRepository(projects_db, root)
            projects.initialize()
            projects.create_project(
                code="CEM-SCH-BAD-METHOD",
                customer_name="Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "test method does not exist"):
                schedule_service_item(
                    projects_db=projects_db,
                    test_definitions_db=test_definitions_db,
                    item_code="PLAN-SCH-BAD-METHOD",
                    project_code="CEM-SCH-BAD-METHOD",
                    title="Unknown method",
                    test_category_code="emission_conducted",
                    test_method_code="MISSING-METHOD",
                    planned_start_at="2026-07-01T09:00",
                    planned_end_at="2026-07-01T12:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_schedule_service_item_rejects_weekend_blocks(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            projects_db = Path(temporary_directory) / "projects.sqlite"
            projects = ProjectRepository(projects_db, root)
            projects.initialize()
            projects.create_project(
                code="CEM-SCH-WEEKEND",
                customer_name="Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "business days"):
                schedule_service_item(
                    projects_db=projects_db,
                    item_code="PLAN-SCH-WEEKEND",
                    project_code="CEM-SCH-WEEKEND",
                    title="Weekend emission",
                    planned_start_at="2026-07-04T09:00",
                    planned_end_at="2026-07-04T12:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_schedule_service_item_rejects_multi_day_business_blocks(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            projects_db = Path(temporary_directory) / "projects.sqlite"
            projects = ProjectRepository(projects_db, root)
            projects.initialize()
            projects.create_project(
                code="CEM-SCH-MULTIDAY",
                customer_name="Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "one business day"):
                schedule_service_item(
                    projects_db=projects_db,
                    item_code="PLAN-SCH-MULTIDAY",
                    project_code="CEM-SCH-MULTIDAY",
                    title="Multi-day emission",
                    planned_start_at="2026-07-02T14:00",
                    planned_end_at="2026-07-03T10:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_schedule_service_item_rejects_invalid_local_datetime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            projects_db = Path(temporary_directory) / "projects.sqlite"
            projects = ProjectRepository(projects_db, root)
            projects.initialize()
            projects.create_project(
                code="CEM-SCH-DATETIME",
                customer_name="Schedule Customer",
                execution_mode="accredited",
                stage="test_planning",
            )

            with self.assertRaisesRegex(ValueError, "local date-time"):
                schedule_service_item(
                    projects_db=projects_db,
                    item_code="PLAN-SCH-DATETIME",
                    project_code="CEM-SCH-DATETIME",
                    title="Invalid timestamp",
                    planned_start_at="2026-07-01",
                    planned_end_at="2026-07-01T12:00",
                    assigned_operator="operator.one",
                    location="Lab A",
                    equipment_under_test="EUT rail",
                )

            self.assertEqual(projects.list_service_schedule_items(), [])

    def test_update_service_schedule_status_records_audit_and_refreshes_bootstrap(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            base = Path(temporary_directory)
            projects_db = base / "projects.sqlite"
            bootstrap_output = base / "bootstrap.js"
            projects = ProjectRepository(projects_db, root)
            projects.initialize()
            projects.create_project(
                code="CEM-SCH-STATUS",
                customer_name="Schedule Status Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            schedule_service_item(
                projects_db=projects_db,
                item_code="PLAN-SCH-STATUS",
                project_code="CEM-SCH-STATUS",
                title="Emission conduite",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
            )

            result = update_service_schedule_status_action(
                projects_db=projects_db,
                item_code="PLAN-SCH-STATUS",
                status="confirmed",
                actor="operator.two",
                reason="Lab slot confirmed",
                bootstrap_output=bootstrap_output,
            )

            schedule = projects.list_service_schedule_items(project_code="CEM-SCH-STATUS")
            events = projects.audit_events("CEM-SCH-STATUS")
            bootstrap_text = bootstrap_output.read_text()

            self.assertEqual(result["item_code"], "PLAN-SCH-STATUS")
            self.assertEqual(result["audit_sequence"], 2)
            self.assertEqual(schedule[0]["status"], "confirmed")
            self.assertEqual(events[-1]["action"], "service_schedule_item_status_updated")
            self.assertIn('"previous_status": "planned"', events[-1]["payload_json"])
            self.assertIn('"new_status": "confirmed"', events[-1]["payload_json"])
            self.assertIn("confirmed", bootstrap_text)

    def test_update_service_schedule_status_normalizes_status_text(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            projects_db = Path(temporary_directory) / "projects.sqlite"
            projects = ProjectRepository(projects_db, root)
            projects.initialize()
            projects.create_project(
                code="CEM-SCH-STATUS-NORMALIZE",
                customer_name="Schedule Status Customer",
                execution_mode="accredited",
                stage="test_planning",
            )
            schedule_service_item(
                projects_db=projects_db,
                item_code="PLAN-SCH-STATUS-NORMALIZE",
                project_code="CEM-SCH-STATUS-NORMALIZE",
                title="Emission conduite",
                planned_start_at="2026-07-01T09:00",
                planned_end_at="2026-07-01T12:00",
                assigned_operator="operator.one",
                location="Lab A",
                equipment_under_test="EUT rail",
            )

            result = update_service_schedule_status_action(
                projects_db=projects_db,
                item_code="PLAN-SCH-STATUS-NORMALIZE",
                status=" confirmed ",
                actor="operator.two",
                reason="Lab slot confirmed",
            )

            schedule = projects.list_service_schedule_items(
                project_code="CEM-SCH-STATUS-NORMALIZE",
            )
            events = projects.audit_events("CEM-SCH-STATUS-NORMALIZE")
            payload = json.loads(events[-1]["payload_json"])
            self.assertEqual(result["status"], "confirmed")
            self.assertEqual(schedule[0]["status"], "confirmed")
            self.assertEqual(payload["new_status"], "confirmed")

    def test_register_metrology_instrument_records_asset_certificate_and_bootstrap(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            base = Path(temporary_directory)
            metrology_db = base / "metrology.sqlite"
            bootstrap_output = base / "bootstrap.js"

            result = register_metrology_instrument(
                metrology_db=metrology_db,
                asset_id="RX-ACT-001",
                family="Receiver",
                manufacturer="Rohde Schwarz",
                model="ESW",
                serial_number="100001",
                category_code="emi_receiver",
                part_number="ESW44",
                calibration_period_months=12,
                capabilities_json='{"frequency_max_hz": 44000000000}',
                certificate_reference="CERT-RX-001",
                calibrated_at="2026-06-01",
                provider="Accredited Lab",
                uncertainty_json='{"level_db": 0.6}',
                bootstrap_output=bootstrap_output,
            )

            repository = MetrologyRepository(metrology_db, root)
            instrument = repository.get_instrument("RX-ACT-001")
            calibration = repository.latest_calibration_record("RX-ACT-001")
            bootstrap_text = bootstrap_output.read_text()

            self.assertEqual(result["category_code"], "emi_receiver")
            self.assertEqual(result["category_label"], "EMI test receiver")
            self.assertEqual(result["part_number"], "ESW44")
            self.assertEqual(result["calibration_period_months"], 12)
            self.assertEqual(result["calibration_requirement"], "required")
            self.assertEqual(result["serviceability_status"], "usable")
            self.assertTrue(result["calibration_recorded"])
            self.assertEqual(instrument["category_code"], "emi_receiver")
            self.assertEqual(instrument["part_number"], "ESW44")
            self.assertEqual(instrument["serviceability_status"], "usable")
            self.assertEqual(instrument["calibration_period_months"], 12)
            self.assertEqual(calibration["certificate_reference"], "CERT-RX-001")
            self.assertEqual(calibration["due_at"], "2027-06-01")
            self.assertIn("RX-ACT-001", bootstrap_text)
            self.assertIn("EMI test receiver", bootstrap_text)

            document = attach_metrology_document(
                metrology_db=metrology_db,
                asset_id="RX-ACT-001",
                document_kind="datasheet",
                title="ESW datasheet",
                file_reference="metrology/RX-ACT-001/datasheet.pdf",
                uploaded_by="metrology.admin",
                applies_to_function="receiver limits",
                bootstrap_output=bootstrap_output,
            )
            documents = repository.list_instrument_documents("RX-ACT-001")

            self.assertEqual(document["document_kind"], "datasheet")
            self.assertEqual(documents[0]["title"], "ESW datasheet")
            self.assertIn("ESW datasheet", bootstrap_output.read_text())

    def test_register_metrology_instrument_rejects_unknown_or_incomplete_data(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            metrology_db = Path(temporary_directory) / "metrology.sqlite"

            with self.assertRaises(ValueError):
                register_metrology_instrument(
                    metrology_db=metrology_db,
                    asset_id="BAD-CAT",
                    family="Receiver",
                    manufacturer="Unknown",
                    model="Unknown",
                    serial_number="BAD-CAT",
                    category_code="missing_category",
                )

            with self.assertRaises(ValueError):
                register_metrology_instrument(
                    metrology_db=metrology_db,
                    asset_id="BAD-CERT",
                    family="Receiver",
                    manufacturer="Unknown",
                    model="Unknown",
                    serial_number="BAD-CERT",
                    category_code="emi_receiver",
                    certificate_reference="CERT-INCOMPLETE",
                )

            repository = MetrologyRepository(metrology_db, Path("storage/sqlite"))
            self.assertIsNone(repository.get_instrument("BAD-CERT"))

    def test_record_metrology_calibration_updates_existing_asset_and_bootstrap(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            base = Path(temporary_directory)
            metrology_db = base / "metrology.sqlite"
            bootstrap_output = base / "bootstrap.js"

            register_metrology_instrument(
                metrology_db=metrology_db,
                asset_id="DAQ-CAL-001",
                family="DAQ",
                manufacturer="Open",
                model="DAQ",
                serial_number="CAL-001",
                category_code="daq_chassis",
            )

            result = record_metrology_calibration(
                metrology_db=metrology_db,
                asset_id="DAQ-CAL-001",
                certificate_reference="CERT-CAL-001",
                calibrated_at="2026-06-15",
                due_at="2027-06-15",
                provider="Metrology Lab",
                uncertainty_json='{"voltage": 0.02}',
                bootstrap_output=bootstrap_output,
            )

            repository = MetrologyRepository(metrology_db, root)
            calibration = repository.latest_calibration_record("DAQ-CAL-001")
            bootstrap_text = bootstrap_output.read_text()

            self.assertEqual(result["certificate_reference"], "CERT-CAL-001")
            self.assertEqual(result["due_at"], "2027-06-15")
            self.assertEqual(calibration["certificate_reference"], "CERT-CAL-001")
            self.assertEqual(calibration["uncertainty_json"], '{"voltage": 0.02}')
            self.assertIn("CERT-CAL-001", bootstrap_text)
            self.assertIn("2027-06-15", bootstrap_text)

    def test_record_metrology_calibration_rejects_missing_asset_or_bad_json(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            metrology_db = Path(temporary_directory) / "metrology.sqlite"

            with self.assertRaises(ValueError):
                record_metrology_calibration(
                    metrology_db=metrology_db,
                    asset_id="MISSING",
                    certificate_reference="CERT-MISSING",
                    calibrated_at="2026-06-15",
                    due_at="2027-06-15",
                    provider="Metrology Lab",
                )

            register_metrology_instrument(
                metrology_db=metrology_db,
                asset_id="DAQ-CAL-002",
                family="DAQ",
                manufacturer="Open",
                model="DAQ",
                serial_number="CAL-002",
                category_code="daq_chassis",
            )

            with self.assertRaises(ValueError):
                record_metrology_calibration(
                    metrology_db=metrology_db,
                    asset_id="DAQ-CAL-002",
                    certificate_reference="CERT-BAD-JSON",
                    calibrated_at="2026-06-15",
                    due_at="2027-06-15",
                    provider="Metrology Lab",
                    uncertainty_json="{bad-json",
                )

            repository = MetrologyRepository(metrology_db, Path("storage/sqlite"))
            self.assertIsNone(repository.latest_calibration_record("DAQ-CAL-002"))

    def test_set_metrology_instrument_availability_updates_bootstrap(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            base = Path(temporary_directory)
            metrology_db = base / "metrology.sqlite"
            bootstrap_output = base / "bootstrap.js"

            register_metrology_instrument(
                metrology_db=metrology_db,
                asset_id="AMP-STATUS-001",
                family="Amplifier",
                manufacturer="RF Lab",
                model="AMP",
                serial_number="STATUS-001",
                category_code="rf_power_amplifier",
            )

            result = set_metrology_instrument_availability(
                metrology_db=metrology_db,
                asset_id="AMP-STATUS-001",
                availability="out_of_service",
                bootstrap_output=bootstrap_output,
            )

            repository = MetrologyRepository(metrology_db, Path("storage/sqlite"))
            instrument = repository.get_instrument("AMP-STATUS-001")
            bootstrap_text = bootstrap_output.read_text()

            self.assertEqual(result["previous_availability"], "available")
            self.assertEqual(result["previous_serviceability_status"], "usable")
            self.assertEqual(result["new_availability"], "out_of_service")
            self.assertEqual(result["new_serviceability_status"], "out_of_service")
            self.assertEqual(instrument["availability"], "out_of_service")
            self.assertEqual(instrument["serviceability_status"], "out_of_service")
            self.assertIn("AMP-STATUS-001", bootstrap_text)
            self.assertIn("Out of service", bootstrap_text)
            self.assertIn("danger", bootstrap_text)

    def test_set_metrology_instrument_serviceability_updates_bootstrap(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            base = Path(temporary_directory)
            metrology_db = base / "metrology.sqlite"
            bootstrap_output = base / "bootstrap.js"

            register_metrology_instrument(
                metrology_db=metrology_db,
                asset_id="SA-SERVICE-001",
                family="Spectrum analyzer",
                manufacturer="RF Bench",
                model="SA",
                serial_number="SERVICE-001",
                category_code="spectrum_analyzer",
                calibration_period_months=12,
                certificate_reference="CERT-SA-SERVICE-001",
                calibrated_at="2026-06-30",
                provider="Metrology Lab",
            )

            result = set_metrology_instrument_serviceability(
                metrology_db=metrology_db,
                asset_id="SA-SERVICE-001",
                serviceability_status="restricted",
                serviceability_reason="Input attenuator under investigation",
                bootstrap_output=bootstrap_output,
            )

            repository = MetrologyRepository(metrology_db, Path("storage/sqlite"))
            instrument = repository.get_instrument("SA-SERVICE-001")
            bootstrap_text = bootstrap_output.read_text()

            self.assertEqual(result["previous_serviceability_status"], "usable")
            self.assertEqual(result["new_serviceability_status"], "restricted")
            self.assertEqual(instrument["availability"], "available")
            self.assertEqual(instrument["serviceability_status"], "restricted")
            self.assertEqual(
                instrument["serviceability_reason"],
                "Input attenuator under investigation",
            )
            self.assertIn("SA-SERVICE-001", bootstrap_text)
            self.assertIn("Restricted", bootstrap_text)
            self.assertIn("warn", bootstrap_text)

    def test_set_metrology_instrument_availability_rejects_invalid_requests(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            metrology_db = Path(temporary_directory) / "metrology.sqlite"

            with self.assertRaises(ValueError):
                set_metrology_instrument_availability(
                    metrology_db=metrology_db,
                    asset_id="MISSING",
                    availability="available",
                )

            register_metrology_instrument(
                metrology_db=metrology_db,
                asset_id="DMM-STATUS-001",
                family="DMM",
                manufacturer="Bench",
                model="DMM",
                serial_number="STATUS-001",
                category_code="digital_multimeter",
            )

            with self.assertRaises(ValueError):
                set_metrology_instrument_availability(
                    metrology_db=metrology_db,
                    asset_id="DMM-STATUS-001",
                    availability="unknown",
                )

            repository = MetrologyRepository(metrology_db, Path("storage/sqlite"))
            self.assertEqual(
                repository.get_instrument("DMM-STATUS-001")["availability"],
                "available",
            )

    def test_set_metrology_instrument_capabilities_updates_existing_asset(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            metrology_db = Path(temporary_directory) / "metrology.sqlite"
            bootstrap_output = Path(temporary_directory) / "bootstrap.js"

            register_metrology_instrument(
                metrology_db=metrology_db,
                asset_id="DAQ-CAP-001",
                family="DAQ",
                manufacturer="Open",
                model="DAQ",
                serial_number="CAP-001",
                category_code="daq_chassis",
            )

            result = set_metrology_instrument_capabilities(
                metrology_db=metrology_db,
                asset_id="DAQ-CAP-001",
                capabilities_json='{"channels": 8, "transports": ["opendaq", "ethernet"]}',
                bootstrap_output=bootstrap_output,
            )

            repository = MetrologyRepository(metrology_db, Path("storage/sqlite"))
            instrument = repository.get_instrument("DAQ-CAP-001")

            self.assertEqual(result["previous_capabilities_json"], "[]")
            self.assertEqual(
                result["new_capabilities_json"],
                '{"channels": 8, "transports": ["opendaq", "ethernet"]}',
            )
            self.assertEqual(
                instrument["capabilities_json"],
                '{"channels": 8, "transports": ["opendaq", "ethernet"]}',
            )
            self.assertIn("DAQ-CAP-001", bootstrap_output.read_text())

    def test_set_metrology_instrument_capabilities_rejects_missing_asset_or_bad_json(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            metrology_db = Path(temporary_directory) / "metrology.sqlite"

            with self.assertRaises(ValueError):
                set_metrology_instrument_capabilities(
                    metrology_db=metrology_db,
                    asset_id="MISSING",
                    capabilities_json="{}",
                )

            register_metrology_instrument(
                metrology_db=metrology_db,
                asset_id="DMM-CAP-001",
                family="DMM",
                manufacturer="Bench",
                model="DMM",
                serial_number="CAP-001",
                category_code="digital_multimeter",
                capabilities_json='{"digits": 6.5}',
            )

            with self.assertRaises(ValueError):
                set_metrology_instrument_capabilities(
                    metrology_db=metrology_db,
                    asset_id="DMM-CAP-001",
                    capabilities_json="{bad-json",
                )

            repository = MetrologyRepository(metrology_db, Path("storage/sqlite"))
            self.assertEqual(
                repository.get_instrument("DMM-CAP-001")["capabilities_json"],
                '{"digits": 6.5}',
            )

    def test_dataset_retention_action_records_event_and_refreshes_bootstrap(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            base = Path(temporary_directory)
            measurement_data_db = base / "measurement_data.sqlite"
            bootstrap_output = base / "bootstrap.js"
            measurement_data = MeasurementDataRepository(measurement_data_db, root)
            measurement_data.initialize()
            dataset_id = measurement_data.add_dataset(
                project_code="CEM-ACT-001",
                campaign_reference="CAMP-ACT-001",
                measurement_run_reference="RUN-ACT-001",
                kind="raw_signal",
                file_reference="data/RUN-ACT-001/raw.opendata",
                checksum="sha256:rawact001",
            )

            request = record_dataset_retention_action(
                measurement_data_db=measurement_data_db,
                dataset_id=dataset_id,
                action="request-deletion",
                actor="data.manager",
                reason="Retention period expired",
                audit_event_reference="audit-ret-001",
                bootstrap_output=bootstrap_output,
            )
            approval = record_dataset_retention_action(
                measurement_data_db=measurement_data_db,
                dataset_id=dataset_id,
                action="approve-deletion",
                actor="quality.manager",
                reason="Deletion request reviewed",
            )

            dataset = measurement_data.get_dataset(dataset_id)
            events = measurement_data.retention_events(dataset_id)
            bootstrap_text = bootstrap_output.read_text()

            self.assertEqual(request["previous_status"], "retained")
            self.assertEqual(request["new_status"], "deletion_requested")
            self.assertEqual(approval["previous_status"], "deletion_requested")
            self.assertEqual(approval["new_status"], "deletion_approved")
            self.assertEqual(dataset["retention_status"], "deletion_approved")
            self.assertEqual(events[0]["audit_event_reference"], "audit-ret-001")
            self.assertIn("RUN-ACT-001", bootstrap_text)
            self.assertIn("deletion_requested", bootstrap_text)

    def test_update_actions_record_validation_install_and_refresh_bootstrap(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path("storage/sqlite")
            base = Path(temporary_directory)
            update_catalog_db = base / "update_catalog.sqlite"
            bootstrap_output = base / "bootstrap.js"
            update_catalog = UpdateCatalogRepository(update_catalog_db, root)
            update_catalog.initialize()
            update_catalog.add_update_package(
                package_name="driver-pack-visa",
                package_version="0.2.0",
                component="instrument_driver",
                compatibility_range="0.1.0..0.1.9",
                signed_checksum="sha256:driver020",
            )

            validation = record_update_validation_action(
                update_catalog_db=update_catalog_db,
                package_name="driver-pack-visa",
                package_version="0.2.0",
                component="instrument_driver",
                installed_version="0.1.0",
                source="offline_bundle",
                compatibility_minimum_version="0.1.0",
                compatibility_maximum_version="0.1.9",
                validated_by="qa.lead",
                bootstrap_output=bootstrap_output,
            )
            install = record_update_install_action(
                update_catalog_db=update_catalog_db,
                package_name="driver-pack-visa",
                package_version="0.2.0",
                component="instrument_driver",
                installed_by="qa.lead",
                source="offline_bundle",
                rollback_reference="driver-pack-visa-0.1.0",
                validation_evidence_id=validation["validation_evidence_id"],
                bootstrap_output=bootstrap_output,
            )

            evidence = update_catalog.get_install_validation_evidence(
                validation["validation_evidence_id"]
            )
            install_records = update_catalog.list_install_records()
            bootstrap_text = bootstrap_output.read_text()

            self.assertEqual(validation["validation_status"], "accepted")
            self.assertIsNone(validation["reason"])
            self.assertEqual(evidence["validation_status"], "accepted")
            self.assertEqual(install_records[0]["id"], install["install_id"])
            self.assertEqual(
                install_records[0]["validation_evidence_id"],
                validation["validation_evidence_id"],
            )
            self.assertIn("driver-pack-visa", bootstrap_text)


class TestDefinitionRepositoryTests(unittest.TestCase):
    def test_lists_default_and_custom_test_categories(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            test_definitions_db = Path(temporary_directory) / "test_definitions.sqlite"
            repository = TestDefinitionRepository(test_definitions_db, Path("storage/sqlite"))
            repository.initialize()

            roots = repository.list_test_categories()
            emission_children = repository.list_test_categories(parent_code="emission")
            custom = create_test_category(
                test_definitions_db=test_definitions_db,
                code="immunity_magnetic_field",
                parent_code="immunity_radiated",
                label="Champ magnetique",
                description="Essais d immunite au champ magnetique basse frequence.",
                sort_order=30,
            )

            self.assertIn("emission", {row["code"] for row in roots})
            self.assertIn("immunity", {row["code"] for row in roots})
            self.assertIn("emission_conducted", {row["code"] for row in emission_children})
            self.assertEqual(custom["parent_code"], "immunity_radiated")
            self.assertEqual(
                repository.get_test_category("immunity_magnetic_field")["label"],
                "Champ magnetique",
            )

    def test_template_revision_migration_resets_0_8_4_template_rows(self) -> None:
        migrations = Path("storage/sqlite/test_definitions")
        connection = sqlite3.connect(":memory:")
        try:
            for name in [
                "0001_test_definitions.sql",
                "0002_test_categories.sql",
                "0003_test_templates.sql",
            ]:
                connection.executescript((migrations / name).read_text(encoding="utf-8"))
            connection.execute(
                """
                INSERT INTO test_templates (
                    template_id, template_revision, title, description,
                    category_code, measurement_axis, status, variables_json,
                    lock_policy_json, instrumentation_chain_json, sequence_json,
                    limits_json, post_processing_json, created_by, created_at,
                    updated_at
                )
                VALUES (
                    'TT-OLD', 'A', 'Old template', 'Old JSON shape',
                    'emission_transient_time_domain', 'time_series', 'draft',
                    '{}', '{}', '[]', '[]', '[]', '[]',
                    'method.author', '2026-06-30T00:00:00Z',
                    '2026-06-30T00:00:00Z'
                )
                """
            )

            connection.executescript(
                (migrations / "0004_template_revision_aggregate.sql").read_text(
                    encoding="utf-8"
                )
            )
            connection.executescript(
                (migrations / "0005_single_active_draft.sql").read_text(
                    encoding="utf-8"
                )
            )

            tables = {
                row[0]
                for row in connection.execute(
                    "SELECT name FROM sqlite_master WHERE type = 'table'"
                )
            }
            self.assertNotIn("test_templates", tables)
            self.assertIn("test_template_identities", tables)
            self.assertIn("test_template_revisions", tables)
            self.assertIn("test_template_audit_events", tables)
            self.assertEqual(
                connection.execute(
                    "SELECT MAX(version) FROM schema_migrations"
                ).fetchone()[0],
                5,
            )
            self.assertEqual(
                connection.execute(
                    """
                    SELECT COUNT(*) FROM sqlite_master
                    WHERE type = 'index'
                      AND name = 'test_template_one_active_draft_idx'
                    """
                ).fetchone()[0],
                1,
            )
            self.assertEqual(
                connection.execute(
                    "SELECT value FROM repository_metadata WHERE key = 'test_template_0_9_migration_policy'"
                ).fetchone()[0],
                "reset_0_8_template_rows_no_dual_runtime",
            )
            self.assertEqual(
                connection.execute(
                    "SELECT value FROM repository_metadata WHERE key = 'test_template_active_draft_policy'"
                ).fetchone()[0],
                "one_active_draft_per_template",
            )
        finally:
            connection.close()

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

    def test_records_operation_journal_with_status_transitions(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = SyncRepository(
                Path(temporary_directory) / "sync.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            repository.record_operation(
                operation_id="op-project-001",
                domain="project_records",
                entity_type="project",
                entity_id="CEM-2026-001",
                operation_kind="contract_review_item_completed",
                base_revision="rev-0001",
                resulting_revision="rev-0002",
                actor_id="quality.lead",
                device_id="station-lab-a",
                correlation_id="corr-001",
                payload_json='{"item":"method_available"}',
                payload_checksum="sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                occurred_at="2026-07-01T06:55:00Z",
            )

            operation = repository.get_operation("op-project-001")
            pending = repository.list_operations(status="pending")

            self.assertEqual(repository.operation_count(), 1)
            self.assertEqual(repository.operation_count("pending"), 1)
            self.assertEqual(operation["entity_id"], "CEM-2026-001")
            self.assertEqual(operation["payload_json"], '{"item": "method_available"}')
            self.assertEqual(pending[0]["operation_id"], "op-project-001")
            self.assertTrue(repository.mark_operation_applied("op-project-001"))
            self.assertFalse(repository.mark_operation_applied("op-project-001"))
            self.assertEqual(repository.get_operation("op-project-001")["status"], "applied")

            with self.assertRaises(ValueError):
                repository.record_operation(
                    operation_id="op-project-invalid-checksum",
                    domain="project_records",
                    entity_type="project",
                    entity_id="CEM-2026-001",
                    operation_kind="contract_review_item_completed",
                    base_revision="rev-0002",
                    resulting_revision="rev-0003",
                    actor_id="quality.lead",
                    device_id="station-lab-a",
                    correlation_id="corr-001",
                    payload_checksum="sha256:too-short",
                )

            with self.assertRaises(sqlite3.IntegrityError):
                repository.record_operation(
                    operation_id="op-project-001",
                    domain="project_records",
                    entity_type="project",
                    entity_id="CEM-2026-001",
                    operation_kind="duplicate",
                    base_revision="rev-0002",
                    resulting_revision="rev-0003",
                    actor_id="quality.lead",
                    device_id="station-lab-a",
                    correlation_id="corr-001",
                    payload_checksum="sha256:1123456789abcdef1123456789abcdef1123456789abcdef1123456789abcdef",
                )

    def test_records_entity_snapshots_and_sync_checkpoints(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = SyncRepository(
                Path(temporary_directory) / "sync.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            repository.record_operation(
                operation_id="op-project-002",
                domain="project_records",
                entity_type="project",
                entity_id="CEM-2026-002",
                operation_kind="project_stage_advanced",
                base_revision="rev-0001",
                resulting_revision="rev-0002",
                actor_id="quality.lead",
                device_id="station-lab-a",
                correlation_id="corr-002",
                payload_checksum="sha256:2123456789abcdef2123456789abcdef2123456789abcdef2123456789abcdef",
            )
            repository.record_entity_snapshot(
                snapshot_id="snap-project-002-a",
                domain="project_records",
                entity_type="project",
                entity_id="CEM-2026-002",
                revision="rev-0002",
                snapshot_checksum="sha256:3123456789abcdef3123456789abcdef3123456789abcdef3123456789abcdef",
                payload_json='{"stage":"contract_review"}',
                source_operation_id="op-project-002",
                captured_at="2026-07-01T07:00:00Z",
            )
            repository.record_entity_snapshot(
                snapshot_id="snap-project-002-b",
                domain="project_records",
                entity_type="project",
                entity_id="CEM-2026-002",
                revision="rev-0003",
                snapshot_checksum="sha256:4123456789abcdef4123456789abcdef4123456789abcdef4123456789abcdef",
                payload_json='{"stage":"test_planning"}',
                captured_at="2026-07-01T07:05:00Z",
            )

            snapshot = repository.get_entity_snapshot("snap-project-002-a")
            latest = repository.latest_entity_snapshot(
                domain="project_records",
                entity_type="project",
                entity_id="CEM-2026-002",
            )

            self.assertEqual(repository.snapshot_count(), 2)
            self.assertEqual(repository.snapshot_count("project_records"), 2)
            self.assertEqual(snapshot["source_operation_id"], "op-project-002")
            self.assertEqual(snapshot["payload_json"], '{"stage": "contract_review"}')
            self.assertEqual(latest["revision"], "rev-0003")

            with self.assertRaises(ValueError):
                repository.record_entity_snapshot(
                    snapshot_id="snap-project-invalid",
                    domain="project_records",
                    entity_type="project",
                    entity_id="CEM-2026-002",
                    revision="rev-0004",
                    snapshot_checksum="sha256:too-short",
                )

            with self.assertRaises(sqlite3.IntegrityError):
                repository.record_entity_snapshot(
                    snapshot_id="snap-project-duplicate",
                    domain="project_records",
                    entity_type="project",
                    entity_id="CEM-2026-002",
                    revision="rev-0003",
                    snapshot_checksum="sha256:5123456789abcdef5123456789abcdef5123456789abcdef5123456789abcdef",
                )

            repository.upsert_checkpoint(
                peer_id="central-main",
                domain="project_records",
                direction="push",
                checkpoint_token="rev-0002",
                last_operation_id="op-project-002",
                last_snapshot_id="snap-project-002-a",
                updated_at="2026-07-01T07:10:00Z",
            )
            repository.upsert_checkpoint(
                peer_id="central-main",
                domain="project_records",
                direction="push",
                checkpoint_token="rev-0003",
                last_operation_id="op-project-002",
                last_snapshot_id="snap-project-002-b",
                updated_at="2026-07-01T07:11:00Z",
            )

            checkpoint = repository.get_checkpoint(
                peer_id="central-main",
                domain="project_records",
                direction="push",
            )
            checkpoints = repository.list_checkpoints(peer_id="central-main")

            self.assertEqual(len(checkpoints), 1)
            self.assertEqual(checkpoint["checkpoint_token"], "rev-0003")
            self.assertEqual(checkpoint["last_snapshot_id"], "snap-project-002-b")

            with self.assertRaises(ValueError):
                repository.upsert_checkpoint(
                    peer_id="central-main",
                    domain="project_records",
                    direction="sideways",
                    checkpoint_token="rev-0003",
                )

    def test_applies_pending_operation_as_entity_snapshot(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = SyncRepository(
                Path(temporary_directory) / "sync.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            repository.record_operation(
                operation_id="op-project-003",
                domain="project_records",
                entity_type="project",
                entity_id="CEM-2026-003",
                operation_kind="project_stage_advanced",
                base_revision="rev-0001",
                resulting_revision="rev-0002",
                actor_id="quality.lead",
                device_id="station-lab-a",
                correlation_id="corr-003",
                payload_checksum="sha256:6123456789abcdef6123456789abcdef6123456789abcdef6123456789abcdef",
            )

            applied = repository.apply_pending_operation_snapshot(
                operation_id="op-project-003",
                snapshot_id="snap-project-003-a",
                snapshot_checksum="sha256:7123456789abcdef7123456789abcdef7123456789abcdef7123456789abcdef",
                payload_json='{"stage":"test_planning"}',
                captured_at="2026-07-01T07:20:00Z",
            )
            replayed = repository.apply_pending_operation_snapshot(
                operation_id="op-project-003",
                snapshot_id="snap-project-003-b",
                snapshot_checksum="sha256:8123456789abcdef8123456789abcdef8123456789abcdef8123456789abcdef",
                payload_json='{"stage":"test_planning"}',
            )

            operation = repository.get_operation("op-project-003")
            snapshot = repository.get_entity_snapshot("snap-project-003-a")

            self.assertTrue(applied)
            self.assertFalse(replayed)
            self.assertEqual(operation["status"], "applied")
            self.assertEqual(repository.snapshot_count("project_records"), 1)
            self.assertEqual(snapshot["revision"], "rev-0002")
            self.assertEqual(snapshot["source_operation_id"], "op-project-003")
            self.assertEqual(snapshot["payload_json"], '{"stage": "test_planning"}')

            repository.record_operation(
                operation_id="op-project-004",
                domain="project_records",
                entity_type="project",
                entity_id="CEM-2026-004",
                operation_kind="project_stage_advanced",
                base_revision="rev-0001",
                resulting_revision="rev-0002",
                actor_id="quality.lead",
                device_id="station-lab-a",
                correlation_id="corr-004",
                payload_checksum="sha256:9123456789abcdef9123456789abcdef9123456789abcdef9123456789abcdef",
            )

            with self.assertRaises(ValueError):
                repository.apply_pending_operation_snapshot(
                    operation_id="op-project-004",
                    snapshot_id="snap-project-004-a",
                    snapshot_checksum="sha256:too-short",
                )

            self.assertEqual(repository.get_operation("op-project-004")["status"], "pending")

    def test_records_conflict_from_divergent_snapshots(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = SyncRepository(
                Path(temporary_directory) / "sync.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            repository.record_entity_snapshot(
                snapshot_id="snap-project-005-local",
                domain="project_records",
                entity_type="project",
                entity_id="CEM-2026-005",
                revision="rev-local",
                snapshot_checksum="sha256:a123456789abcdefa123456789abcdefa123456789abcdefa123456789abcdef",
            )
            repository.record_entity_snapshot(
                snapshot_id="snap-project-005-reference",
                domain="project_records",
                entity_type="project",
                entity_id="CEM-2026-005",
                revision="rev-reference",
                snapshot_checksum="sha256:b123456789abcdefb123456789abcdefb123456789abcdefb123456789abcdef",
            )

            created = repository.record_snapshot_conflict(
                conflict_id="conflict-project-005",
                local_snapshot_id="snap-project-005-local",
                reference_snapshot_id="snap-project-005-reference",
            )
            conflict = repository.get_conflict("conflict-project-005")

            self.assertTrue(created)
            self.assertEqual(repository.conflict_count(), 1)
            self.assertEqual(conflict["domain"], "project_records")
            self.assertEqual(conflict["kind"], "checksum_mismatch")
            self.assertEqual(conflict["local_snapshot"], "snap-project-005-local")
            self.assertEqual(
                conflict["reference_snapshot"],
                "snap-project-005-reference",
            )

            repository.record_entity_snapshot(
                snapshot_id="snap-project-006-local",
                domain="project_records",
                entity_type="project",
                entity_id="CEM-2026-006",
                revision="rev-local",
                snapshot_checksum="sha256:c123456789abcdefc123456789abcdefc123456789abcdefc123456789abcdef",
            )
            repository.record_entity_snapshot(
                snapshot_id="snap-project-006-reference",
                domain="project_records",
                entity_type="project",
                entity_id="CEM-2026-006",
                revision="rev-reference",
                snapshot_checksum="sha256:c123456789abcdefc123456789abcdefc123456789abcdefc123456789abcdef",
            )

            self.assertFalse(
                repository.record_snapshot_conflict(
                    conflict_id="conflict-project-006",
                    local_snapshot_id="snap-project-006-local",
                    reference_snapshot_id="snap-project-006-reference",
                )
            )
            self.assertEqual(repository.conflict_count(), 1)

            with self.assertRaises(ValueError):
                repository.record_snapshot_conflict(
                    conflict_id="conflict-mismatch",
                    local_snapshot_id="snap-project-005-local",
                    reference_snapshot_id="snap-project-006-reference",
                )

    def test_suggests_conflict_action_plan_without_resolving(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            repository = SyncRepository(
                Path(temporary_directory) / "sync.sqlite",
                Path("storage/sqlite"),
            )
            repository.initialize()

            repository.record_conflict(
                conflict_id="conflict-suggest-001",
                domain="project_records",
                kind="checksum_mismatch",
                local_snapshot="snap-local",
                reference_snapshot="snap-reference",
            )

            plan_id = repository.suggest_conflict_action_plan(
                conflict_id="conflict-suggest-001",
                planned_by="qa.lead",
            )
            same_plan_id = repository.suggest_conflict_action_plan(
                conflict_id="conflict-suggest-001",
                planned_by="qa.lead",
            )

            conflict = repository.get_conflict("conflict-suggest-001")
            plans = repository.action_plans_for_conflict("conflict-suggest-001")

            self.assertEqual(plan_id, same_plan_id)
            self.assertEqual(repository.action_plan_count(), 1)
            self.assertEqual(conflict["status"], "open")
            self.assertIsNone(conflict["resolution"])
            self.assertEqual(plans[0]["resolution"], "manual_merge")
            self.assertEqual(plans[0]["action"], "manual_merge")
            self.assertEqual(plans[0]["requires_audit_event"], 1)

    def test_initialize_applies_operation_journal_migration_to_existing_sync_database(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            database_path = Path(temporary_directory) / "sync.sqlite"
            connection = sqlite3.connect(database_path)
            try:
                connection.executescript(
                    Path("storage/sqlite/sync/0001_sync_conflicts.sql").read_text(
                        encoding="utf-8"
                    )
                )
            finally:
                connection.close()

            repository = SyncRepository(database_path, Path("storage/sqlite"))
            repository.initialize()

            with closing(repository.connect()) as connection:
                version_rows = connection.execute(
                    "SELECT version FROM schema_migrations ORDER BY version"
                ).fetchall()
                operation_table = connection.execute(
                    """
                    SELECT name
                    FROM sqlite_master
                    WHERE type = 'table'
                      AND name = 'sync_operations'
                    """
                ).fetchone()
                snapshot_table = connection.execute(
                    """
                    SELECT name
                    FROM sqlite_master
                    WHERE type = 'table'
                      AND name = 'sync_entity_snapshots'
                    """
                ).fetchone()
                checkpoint_table = connection.execute(
                    """
                    SELECT name
                    FROM sqlite_master
                    WHERE type = 'table'
                      AND name = 'sync_checkpoints'
                    """
                ).fetchone()

            self.assertEqual([row["version"] for row in version_rows], [1, 2, 3, 4])
            self.assertIsNotNone(operation_table)
            self.assertIsNotNone(snapshot_table)
            self.assertIsNotNone(checkpoint_table)


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
