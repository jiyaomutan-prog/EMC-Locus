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
    build_bootstrap,
    next_project_stage,
    record_dataset_retention_action,
    record_update_install_action,
    record_update_validation_action,
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

            with self.assertRaises(ValueError):
                repository.add_processing_graph_instance_artifact(
                    processing_graph_instance_id=second_revision_id,
                    output_signal_reference="raw_copy",
                    artifact_kind="raw_signal",
                    file_reference="data/RUN-FFT-001/raw-copy.opendata",
                    checksum="sha256:rawcopy001",
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
                connection.execute(
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
                )
                connection.commit()
            finally:
                connection.close()

            repository = MetrologyRepository(database_path, Path("storage/sqlite"))
            repository.initialize()

            with closing(repository.connect()) as connection:
                version_rows = connection.execute(
                    "SELECT version FROM schema_migrations ORDER BY version"
                ).fetchall()
                instrument_columns = connection.execute(
                    "PRAGMA table_info(instruments)"
                ).fetchall()

            self.assertEqual([row["version"] for row in version_rows], [1, 2])
            self.assertIn("category_code", {row["name"] for row in instrument_columns})
            self.assertEqual(repository.category_count(), 34)
            self.assertIsNone(repository.get_instrument("LEGACY-001")["category_code"])


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
                category_code="daq_chassis",
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
            self.assertEqual(payload["instruments"][0][0], "DAQ-001")
            self.assertEqual(payload["instruments"][0][5], "ok")
            self.assertIn(
                "daq_chassis",
                {row[0] for row in payload["instrument_categories"]},
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

    def test_next_project_stage_rejects_unknown_stage(self) -> None:
        with self.assertRaises(ValueError):
            next_project_stage("unknown")

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
