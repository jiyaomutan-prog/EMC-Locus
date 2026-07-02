"""SQLite repository adapters for early local EMC Locus storage."""

from __future__ import annotations

from contextlib import closing
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
import hashlib
import json
from pathlib import Path
import re
import sqlite3

from .migrations import discover_migrations


UPDATE_COMPONENTS = {
    "core_application",
    "instrument_driver",
    "signal_processing_engine",
    "report_template_pack",
    "database_migration",
}
UPDATE_SOURCES = {"online_catalog", "offline_bundle"}
RETENTION_STATUSES = {
    "retained",
    "deletion_requested",
    "deletion_approved",
    "deletion_rejected",
    "deleted",
}
PROCESSING_GRAPH_STATUSES = {"draft", "active", "superseded", "rejected"}
PROCESSING_GRAPH_ARTIFACT_KINDS = {"processed_signal", "result_table"}
PROCESSING_GRAPH_EXECUTION_STATUSES = {"completed", "failed"}
INSTRUMENT_SERVICEABILITY_STATUSES = {
    "usable",
    "restricted",
    "out_of_service",
    "retired",
}
SERVICE_SCHEDULE_STATUSES = {
    "planned",
    "confirmed",
    "in_progress",
    "completed",
    "cancelled",
}
SYNC_CHECKPOINT_DIRECTIONS = {"push", "pull", "bidirectional"}
_PACKAGE_NAME = re.compile(r"^[A-Za-z0-9_.-]+$")
_SIGNAL_REFERENCE = re.compile(r"^[A-Za-z0-9_.-]+$")
_SOFTWARE_VERSION = re.compile(r"^\d+\.\d+\.\d+$")
_SHA256_CHECKSUM = re.compile(r"^sha256:[0-9A-Fa-f]{64}$")
_SCHEDULE_LOCAL_DATETIME = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$")


def utc_timestamp() -> str:
    """Return a compact UTC timestamp for deterministic storage columns."""

    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def serviceability_from_legacy_availability(availability: str) -> str:
    """Map the legacy availability field to the new serviceability concept."""

    return "out_of_service" if availability == "out_of_service" else "usable"


def _parse_schedule_datetime(value: str, field_name: str) -> datetime:
    if not _SCHEDULE_LOCAL_DATETIME.fullmatch(value):
        raise ValueError(f"{field_name} must use YYYY-MM-DDTHH:MM local date-time")
    try:
        parsed = datetime.fromisoformat(value)
    except ValueError as exc:
        raise ValueError(f"{field_name} must use YYYY-MM-DDTHH:MM local date-time") from exc
    if parsed.tzinfo is not None:
        raise ValueError(f"{field_name} must be a local date-time without timezone")
    return parsed


def validate_service_schedule_block(
    planned_start_at: str,
    planned_end_at: str,
) -> None:
    """Reject schedule blocks that are not one intra-day business block."""

    start = _parse_schedule_datetime(planned_start_at, "planned_start_at")
    end = _parse_schedule_datetime(planned_end_at, "planned_end_at")
    if end <= start:
        raise ValueError("planned_end_at must be after planned_start_at")
    if end.date() != start.date():
        raise ValueError("service schedule items must stay within one business day")

    day = start.date()
    while day <= end.date():
        if day.weekday() >= 5:
            raise ValueError("service schedule items must stay within business days")
        day += timedelta(days=1)


def validate_service_schedule_status(status: str) -> None:
    if status not in SERVICE_SCHEDULE_STATUSES:
        raise ValueError(f"unknown service schedule status: {status}")


def _validate_signal_reference(value: str, *, field_name: str) -> None:
    trimmed = value.strip()
    if not trimmed or not _SIGNAL_REFERENCE.fullmatch(trimmed):
        raise ValueError(f"invalid signal reference in {field_name}: {value}")


def _validate_raw_lineage_json(raw_lineage_json: str) -> None:
    try:
        raw_lineage = json.loads(raw_lineage_json)
    except json.JSONDecodeError as error:
        raise ValueError("raw lineage must be valid JSON") from error

    if not isinstance(raw_lineage, list):
        raise ValueError("raw lineage must be a JSON array")

    for signal_reference in raw_lineage:
        if not isinstance(signal_reference, str):
            raise ValueError("raw lineage entries must be signal references")
        _validate_signal_reference(signal_reference, field_name="raw lineage")


@dataclass(frozen=True)
class SQLiteDomainRepository:
    """A single SQLite database backed by one migration domain."""

    database_path: Path
    migrations_root: Path
    domain: str

    def initialize(self) -> None:
        self.database_path.parent.mkdir(parents=True, exist_ok=True)
        with closing(self.connect()) as connection:
            migrations = [
                migration
                for migration in discover_migrations(self.migrations_root)
                if migration.domain == self.domain
            ]
            with connection:
                applied_versions: set[int] = set()
                if self._has_schema(connection):
                    rows = connection.execute(
                        "SELECT version FROM schema_migrations"
                    ).fetchall()
                    applied_versions = {int(row["version"]) for row in rows}

                for migration in migrations:
                    if migration.version not in applied_versions:
                        connection.executescript(migration.path.read_text(encoding="utf-8"))

    def connect(self) -> sqlite3.Connection:
        connection = sqlite3.connect(self.database_path)
        connection.row_factory = sqlite3.Row
        connection.execute("PRAGMA foreign_keys = ON")
        return connection

    def metadata(self) -> dict[str, str]:
        with closing(self.connect()) as connection:
            rows = connection.execute("SELECT key, value FROM repository_metadata").fetchall()
        return {row["key"]: row["value"] for row in rows}

    @staticmethod
    def _has_schema(connection: sqlite3.Connection) -> bool:
        row = connection.execute(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'"
        ).fetchone()
        return row is not None


class MetrologyRepository(SQLiteDomainRepository):
    """SQLite adapter for instrument and calibration records."""

    def __init__(self, database_path: Path | str, migrations_root: Path | str) -> None:
        super().__init__(Path(database_path), Path(migrations_root), "metrology")

    def add_instrument(
        self,
        *,
        asset_id: str,
        family: str,
        manufacturer: str,
        model: str,
        serial_number: str,
        calibration_requirement: str,
        availability: str = "available",
        capabilities_json: str = "[]",
        category_code: str | None = None,
        part_number: str | None = None,
        calibration_period_months: int | None = None,
        metrology_notes: str = "",
        serviceability_status: str | None = None,
        serviceability_reason: str = "",
    ) -> None:
        now = utc_timestamp()
        serviceability_status = serviceability_status or serviceability_from_legacy_availability(
            availability
        )
        with closing(self.connect()) as connection:
            with connection:
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
                        category_code,
                        part_number,
                        calibration_period_months,
                        metrology_notes,
                        serviceability_status,
                        serviceability_reason,
                        serviceability_updated_at,
                        legacy_availability,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        asset_id,
                        family,
                        manufacturer,
                        model,
                        serial_number,
                        availability,
                        calibration_requirement,
                        capabilities_json,
                        category_code,
                        part_number,
                        calibration_period_months,
                        metrology_notes,
                        serviceability_status,
                        serviceability_reason,
                        now,
                        availability,
                        now,
                        now,
                    ),
                )

    def register_instrument(
        self,
        *,
        asset_id: str,
        family: str,
        manufacturer: str,
        model: str,
        serial_number: str,
        calibration_requirement: str,
        availability: str = "available",
        capabilities_json: str = "[]",
        category_code: str | None = None,
        part_number: str | None = None,
        calibration_period_months: int | None = None,
        metrology_notes: str = "",
        serviceability_status: str | None = None,
        serviceability_reason: str = "",
        certificate_reference: str | None = None,
        calibrated_at: str | None = None,
        due_at: str | None = None,
        provider: str | None = None,
        status_at_import: str = "valid",
        uncertainty_json: str = "{}",
        file_reference: str | None = None,
        checksum: str | None = None,
    ) -> None:
        """Register an instrument and optional initial calibration atomically."""

        now = utc_timestamp()
        serviceability_status = serviceability_status or serviceability_from_legacy_availability(
            availability
        )
        with closing(self.connect()) as connection:
            with connection:
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
                        category_code,
                        part_number,
                        calibration_period_months,
                        metrology_notes,
                        serviceability_status,
                        serviceability_reason,
                        serviceability_updated_at,
                        legacy_availability,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        asset_id,
                        family,
                        manufacturer,
                        model,
                        serial_number,
                        availability,
                        calibration_requirement,
                        capabilities_json,
                        category_code,
                        part_number,
                        calibration_period_months,
                        metrology_notes,
                        serviceability_status,
                        serviceability_reason,
                        now,
                        availability,
                        now,
                        now,
                    ),
                )

                if certificate_reference is not None:
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
                            asset_id,
                            certificate_reference,
                            calibrated_at,
                            due_at,
                            provider,
                            status_at_import,
                            uncertainty_json,
                            file_reference,
                            checksum,
                            now,
                        ),
                    )

    def add_calibration_record(
        self,
        *,
        asset_id: str,
        certificate_reference: str,
        calibrated_at: str,
        due_at: str,
        provider: str,
        status_at_import: str = "valid",
        uncertainty_json: str = "{}",
        file_reference: str | None = None,
        checksum: str | None = None,
    ) -> None:
        with closing(self.connect()) as connection:
            with connection:
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
                        asset_id,
                        certificate_reference,
                        calibrated_at,
                        due_at,
                        provider,
                        status_at_import,
                        uncertainty_json,
                        file_reference,
                        checksum,
                        utc_timestamp(),
                    ),
                )

    def add_instrument_document(
        self,
        *,
        asset_id: str,
        document_kind: str,
        title: str,
        file_reference: str,
        uploaded_by: str,
        checksum: str | None = None,
        revision: str | None = None,
        applies_to_function: str | None = None,
    ) -> int:
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    INSERT INTO instrument_documents (
                        asset_id,
                        document_kind,
                        title,
                        file_reference,
                        checksum,
                        revision,
                        applies_to_function,
                        uploaded_by,
                        created_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        asset_id,
                        document_kind,
                        title,
                        file_reference,
                        checksum,
                        revision,
                        applies_to_function,
                        uploaded_by,
                        utc_timestamp(),
                    ),
                )
        return int(cursor.lastrowid)

    def instrument_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute("SELECT COUNT(*) AS count FROM instruments").fetchone()
        return int(row["count"])

    def calibration_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute("SELECT COUNT(*) AS count FROM calibration_records").fetchone()
        return int(row["count"])

    def document_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT COUNT(*) AS count FROM instrument_documents WHERE active = 1"
            ).fetchone()
        return int(row["count"])

    def category_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT COUNT(*) AS count FROM instrument_categories"
            ).fetchone()
        return int(row["count"])

    def list_instrument_categories(
        self,
        *,
        domain: str | None = None,
    ) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            if domain is None:
                rows = connection.execute(
                    """
                    SELECT *
                    FROM instrument_categories
                    WHERE active = 1
                    ORDER BY domain, label
                    """
                ).fetchall()
            else:
                rows = connection.execute(
                    """
                    SELECT *
                    FROM instrument_categories
                    WHERE active = 1 AND domain = ?
                    ORDER BY label
                    """,
                    (domain,),
                ).fetchall()
        return [dict(row) for row in rows]

    def get_instrument_category(self, code: str) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT * FROM instrument_categories WHERE code = ?",
                (code,),
            ).fetchone()
        return row_to_dict(row)

    def list_instrument_category_sources(
        self,
        category_code: str,
    ) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM instrument_category_sources
                WHERE category_code = ?
                ORDER BY source_name, source_url
                """,
                (category_code,),
            ).fetchall()
        return [dict(row) for row in rows]

    def get_instrument(self, asset_id: str) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT * FROM instruments WHERE asset_id = ?",
                (asset_id,),
            ).fetchone()
        return row_to_dict(row)

    def list_instruments(self) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                "SELECT * FROM instruments ORDER BY asset_id"
            ).fetchall()
        return [dict(row) for row in rows]

    def instruments_by_category(self, category_code: str) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT instruments.*
                FROM instruments
                WHERE category_code = ?
                ORDER BY asset_id
                """,
                (category_code,),
            ).fetchall()
        return [dict(row) for row in rows]

    def instruments_by_category_domain(self, domain: str) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT instruments.*,
                       instrument_categories.label AS category_label,
                       instrument_categories.domain AS category_domain
                FROM instruments
                JOIN instrument_categories
                  ON instrument_categories.code = instruments.category_code
                WHERE instrument_categories.domain = ?
                ORDER BY instruments.asset_id
                """,
                (domain,),
            ).fetchall()
        return [dict(row) for row in rows]

    def latest_calibration_record(self, asset_id: str) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                """
                SELECT *
                FROM calibration_records
                WHERE asset_id = ?
                ORDER BY due_at DESC, calibrated_at DESC, id DESC
                LIMIT 1
                """,
                (asset_id,),
            ).fetchone()
        return row_to_dict(row)

    def list_instrument_documents(
        self,
        asset_id: str,
        *,
        document_kind: str | None = None,
    ) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            if document_kind is None:
                rows = connection.execute(
                    """
                    SELECT *
                    FROM instrument_documents
                    WHERE asset_id = ? AND active = 1
                    ORDER BY document_kind, title, id
                    """,
                    (asset_id,),
                ).fetchall()
            else:
                rows = connection.execute(
                    """
                    SELECT *
                    FROM instrument_documents
                    WHERE asset_id = ? AND document_kind = ? AND active = 1
                    ORDER BY title, id
                    """,
                    (asset_id, document_kind),
                ).fetchall()
        return [dict(row) for row in rows]

    def update_instrument_availability(self, *, asset_id: str, availability: str) -> bool:
        now = utc_timestamp()
        serviceability_status = serviceability_from_legacy_availability(availability)
        serviceability_reason = (
            "Set through legacy availability compatibility path"
            if availability == "out_of_service"
            else ""
        )
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    UPDATE instruments
                    SET availability = ?,
                        legacy_availability = ?,
                        serviceability_status = ?,
                        serviceability_reason = ?,
                        serviceability_updated_at = ?,
                        updated_at = ?
                    WHERE asset_id = ?
                    """,
                    (
                        availability,
                        availability,
                        serviceability_status,
                        serviceability_reason,
                        now,
                        now,
                        asset_id,
                    ),
                )
        return cursor.rowcount == 1

    def update_instrument_serviceability(
        self,
        *,
        asset_id: str,
        serviceability_status: str,
        serviceability_reason: str = "",
    ) -> bool:
        now = utc_timestamp()
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    UPDATE instruments
                    SET serviceability_status = ?,
                        serviceability_reason = ?,
                        serviceability_updated_at = ?,
                        updated_at = ?
                    WHERE asset_id = ?
                    """,
                    (
                        serviceability_status,
                        serviceability_reason,
                        now,
                        now,
                        asset_id,
                    ),
                )
        return cursor.rowcount == 1

    def update_instrument_capabilities(
        self,
        *,
        asset_id: str,
        capabilities_json: str,
    ) -> bool:
        now = utc_timestamp()
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    UPDATE instruments
                    SET capabilities_json = ?, updated_at = ?
                    WHERE asset_id = ?
                    """,
                    (capabilities_json, now, asset_id),
                )
        return cursor.rowcount == 1

    def update_calibration_attachment(
        self,
        *,
        asset_id: str,
        certificate_reference: str,
        file_reference: str | None,
        checksum: str | None,
    ) -> bool:
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    UPDATE calibration_records
                    SET file_reference = ?, checksum = ?
                    WHERE asset_id = ? AND certificate_reference = ?
                    """,
                    (file_reference, checksum, asset_id, certificate_reference),
                )
        return cursor.rowcount == 1


class ProjectRepository(SQLiteDomainRepository):
    """SQLite adapter for projects and project audit events."""

    def __init__(self, database_path: Path | str, migrations_root: Path | str) -> None:
        super().__init__(Path(database_path), Path(migrations_root), "projects")

    def create_project(
        self,
        *,
        code: str,
        customer_name: str,
        execution_mode: str,
        stage: str = "quotation",
    ) -> None:
        with closing(self.connect()) as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO projects (
                        code,
                        customer_name,
                        stage,
                        execution_mode,
                        created_at
                    )
                    VALUES (?, ?, ?, ?, ?)
                    """,
                    (code, customer_name, stage, execution_mode, utc_timestamp()),
                )

    def create_project_with_audit(
        self,
        *,
        code: str,
        customer_name: str,
        execution_mode: str,
        actor: str,
        reason: str,
        stage: str = "quotation",
    ) -> int:
        """Create a project and its first audit event in one transaction."""

        now = utc_timestamp()
        payload_json = json.dumps(
            {
                "customer_name": customer_name,
                "execution_mode": execution_mode,
                "stage": stage,
            },
            sort_keys=True,
        )
        with closing(self.connect()) as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO projects (
                        code,
                        customer_name,
                        stage,
                        execution_mode,
                        created_at
                    )
                    VALUES (?, ?, ?, ?, ?)
                    """,
                    (code, customer_name, stage, execution_mode, now),
                )
                cursor = connection.execute(
                    """
                    INSERT INTO project_audit_events (
                        project_code,
                        sequence,
                        actor,
                        action,
                        reason,
                        payload_json,
                        occurred_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        code,
                        1,
                        actor,
                        "project_created",
                        reason,
                        payload_json,
                        now,
                    ),
                )
        return int(cursor.lastrowid)

    def append_audit_event(
        self,
        *,
        project_code: str,
        sequence: int,
        actor: str,
        action: str,
        reason: str | None = None,
        payload_json: str = "{}",
    ) -> None:
        with closing(self.connect()) as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO project_audit_events (
                        project_code,
                        sequence,
                        actor,
                        action,
                        reason,
                        payload_json,
                        occurred_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        project_code,
                        sequence,
                        actor,
                        action,
                        reason,
                        payload_json,
                        utc_timestamp(),
                    ),
                )

    def project_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute("SELECT COUNT(*) AS count FROM projects").fetchone()
        return int(row["count"])

    def audit_event_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT COUNT(*) AS count FROM project_audit_events"
            ).fetchone()
        return int(row["count"])

    def get_project(self, code: str) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT * FROM projects WHERE code = ?",
                (code,),
            ).fetchone()
        return row_to_dict(row)

    def list_projects(self) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute("SELECT * FROM projects ORDER BY code").fetchall()
        return [dict(row) for row in rows]

    def audit_events(self, project_code: str) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM project_audit_events
                WHERE project_code = ?
                ORDER BY sequence
                """,
                (project_code,),
            ).fetchall()
        return [dict(row) for row in rows]

    def set_project_stage_with_audit(
        self,
        *,
        code: str,
        stage: str,
        actor: str,
        reason: str | None = None,
        action: str = "project_stage_set",
        payload_json: str = "{}",
    ) -> int | None:
        """Persist a domain-approved project stage change and audit event."""

        now = utc_timestamp()
        with closing(self.connect()) as connection:
            with connection:
                project = connection.execute(
                    "SELECT code FROM projects WHERE code = ?",
                    (code,),
                ).fetchone()
                if project is None:
                    return None

                sequence_row = connection.execute(
                    """
                    SELECT COALESCE(MAX(sequence), 0) + 1 AS next_sequence
                    FROM project_audit_events
                    WHERE project_code = ?
                    """,
                    (code,),
                ).fetchone()
                sequence = int(sequence_row["next_sequence"])
                archived_at = now if stage == "archived" else None

                connection.execute(
                    """
                    UPDATE projects
                    SET stage = ?,
                        archived_at = CASE WHEN ? IS NULL THEN archived_at ELSE ? END
                    WHERE code = ?
                    """,
                    (stage, archived_at, archived_at, code),
                )
                connection.execute(
                    """
                    INSERT INTO project_audit_events (
                        project_code,
                        sequence,
                        actor,
                        action,
                        reason,
                        payload_json,
                        occurred_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (code, sequence, actor, action, reason, payload_json, now),
                )
        return sequence

    def complete_contract_review_item(
        self,
        *,
        project_code: str,
        item: str,
        completed_by: str,
        comment: str | None = None,
    ) -> None:
        now = utc_timestamp()
        with closing(self.connect()) as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO contract_review_items (
                        project_code,
                        item,
                        completed,
                        completed_by,
                        completed_at,
                        comment
                    )
                    VALUES (?, ?, 1, ?, ?, ?)
                    ON CONFLICT(project_code, item) DO UPDATE SET
                        completed = 1,
                        completed_by = excluded.completed_by,
                        completed_at = excluded.completed_at,
                        comment = excluded.comment
                    """,
                    (project_code, item, completed_by, now, comment),
                )

    def complete_contract_review_item_with_audit(
        self,
        *,
        project_code: str,
        item: str,
        completed_by: str,
        comment: str | None = None,
    ) -> int | None:
        """Complete a contract-review item and append audit evidence."""

        now = utc_timestamp()
        payload_json = json.dumps(
            {
                "item": item,
                "completed": True,
                "comment": comment,
            },
            sort_keys=True,
        )
        with closing(self.connect()) as connection:
            with connection:
                project = connection.execute(
                    "SELECT code FROM projects WHERE code = ?",
                    (project_code,),
                ).fetchone()
                if project is None:
                    return None

                connection.execute(
                    """
                    INSERT INTO contract_review_items (
                        project_code,
                        item,
                        completed,
                        completed_by,
                        completed_at,
                        comment
                    )
                    VALUES (?, ?, 1, ?, ?, ?)
                    ON CONFLICT(project_code, item) DO UPDATE SET
                        completed = 1,
                        completed_by = excluded.completed_by,
                        completed_at = excluded.completed_at,
                        comment = excluded.comment
                    """,
                    (project_code, item, completed_by, now, comment),
                )
                sequence_row = connection.execute(
                    """
                    SELECT COALESCE(MAX(sequence), 0) + 1 AS next_sequence
                    FROM project_audit_events
                    WHERE project_code = ?
                    """,
                    (project_code,),
                ).fetchone()
                sequence = int(sequence_row["next_sequence"])
                connection.execute(
                    """
                    INSERT INTO project_audit_events (
                        project_code,
                        sequence,
                        actor,
                        action,
                        reason,
                        payload_json,
                        occurred_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        project_code,
                        sequence,
                        completed_by,
                        "contract_review_item_completed",
                        comment,
                        payload_json,
                        now,
                    ),
                )
        return sequence

    def contract_review_items(self, project_code: str) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM contract_review_items
                WHERE project_code = ?
                ORDER BY item
                """,
                (project_code,),
            ).fetchall()
        return [dict(row) for row in rows]

    def add_service_schedule_item(
        self,
        *,
        item_code: str,
        project_code: str,
        title: str,
        planned_start_at: str,
        planned_end_at: str,
        assigned_operator: str,
        location: str,
        equipment_under_test: str,
        test_category_code: str | None = None,
        test_method_code: str | None = None,
        status: str = "planned",
        notes: str = "",
    ) -> int:
        item_code = require_non_empty(item_code, "item_code")
        project_code = require_non_empty(project_code, "project_code")
        title = require_non_empty(title, "title")
        planned_start_at = require_non_empty(planned_start_at, "planned_start_at")
        planned_end_at = require_non_empty(planned_end_at, "planned_end_at")
        assigned_operator = require_non_empty(assigned_operator, "assigned_operator")
        location = require_non_empty(location, "location")
        equipment_under_test = require_non_empty(equipment_under_test, "equipment_under_test")
        test_category_code = optional_text_or_none(test_category_code)
        test_method_code = optional_text_or_none(test_method_code)
        status = require_non_empty(status, "status")
        validate_service_schedule_block(planned_start_at, planned_end_at)
        validate_service_schedule_status(status)
        now = utc_timestamp()
        with closing(self.connect()) as connection:
            with connection:
                project = connection.execute(
                    "SELECT 1 FROM projects WHERE code = ?",
                    (project_code,),
                ).fetchone()
                if project is None:
                    raise ValueError("project does not exist")
                cursor = connection.execute(
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
                        now,
                        now,
                    ),
                )
        return int(cursor.lastrowid)

    def list_service_schedule_items(
        self,
        *,
        project_code: str | None = None,
        status: str | None = None,
    ) -> list[dict[str, object]]:
        filters: list[str] = []
        parameters: list[str] = []
        if project_code is not None:
            project_code = require_non_empty(project_code, "project_code")
            filters.append("project_code = ?")
            parameters.append(project_code)
        if status is not None:
            status = require_non_empty(status, "status")
            validate_service_schedule_status(status)
            filters.append("status = ?")
            parameters.append(status)

        where_clause = f"WHERE {' AND '.join(filters)}" if filters else ""
        with closing(self.connect()) as connection:
            rows = connection.execute(
                f"""
                SELECT *
                FROM service_schedule_items
                {where_clause}
                ORDER BY planned_start_at, planned_end_at, item_code
                """,
                tuple(parameters),
            ).fetchall()
        return [dict(row) for row in rows]

    def update_service_schedule_status(
        self,
        *,
        item_code: str,
        status: str,
    ) -> bool:
        item_code = require_non_empty(item_code, "item_code")
        status = require_non_empty(status, "status")
        validate_service_schedule_status(status)
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    UPDATE service_schedule_items
                    SET status = ?, updated_at = ?
                    WHERE item_code = ?
                    """,
                    (status, utc_timestamp(), item_code),
                )
        return cursor.rowcount == 1


class MeasurementDataRepository(SQLiteDomainRepository):
    """SQLite adapter for immutable datasets and signal-processing artifacts."""

    def __init__(self, database_path: Path | str, migrations_root: Path | str) -> None:
        super().__init__(Path(database_path), Path(migrations_root), "measurement_data")

    def add_dataset(
        self,
        *,
        project_code: str,
        campaign_reference: str,
        measurement_run_reference: str,
        kind: str,
        file_reference: str,
        checksum: str,
        immutable: bool = True,
    ) -> int:
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
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
                        project_code,
                        campaign_reference,
                        measurement_run_reference,
                        kind,
                        file_reference,
                        checksum,
                        utc_timestamp(),
                        int(immutable),
                    ),
                )
        return int(cursor.lastrowid)

    def dataset_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute("SELECT COUNT(*) AS count FROM datasets").fetchone()
        return int(row["count"])

    def get_dataset(self, dataset_id: int) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT * FROM datasets WHERE id = ?",
                (dataset_id,),
            ).fetchone()
        return row_to_dict(row)

    def get_dataset_by_checksum(self, checksum: str) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT * FROM datasets WHERE checksum = ? ORDER BY id LIMIT 1",
                (checksum,),
            ).fetchone()
        return row_to_dict(row)

    def datasets_for_run(
        self,
        *,
        measurement_run_reference: str,
    ) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM datasets
                WHERE measurement_run_reference = ?
                ORDER BY acquired_at, id
                """,
                (measurement_run_reference,),
            ).fetchall()
        return [dict(row) for row in rows]

    def list_datasets(self) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM datasets
                ORDER BY acquired_at, id
                """
            ).fetchall()
        return [dict(row) for row in rows]

    def record_instrument_observation(
        self,
        *,
        project_code: str,
        campaign_reference: str,
        measurement_run_reference: str,
        sequence: int,
        instrument_code: str,
        transport: str,
        endpoint: str,
        command_message: str,
        response_message: str,
        success: bool,
        exchange_attempts: int,
        raw_payload_json: str = "{}",
    ) -> int:
        if sequence < 1:
            raise ValueError("sequence must be positive")
        if exchange_attempts < 1:
            raise ValueError("exchange_attempts must be positive")

        project_code = require_non_empty(project_code, "project_code")
        campaign_reference = require_non_empty(campaign_reference, "campaign_reference")
        measurement_run_reference = require_non_empty(
            measurement_run_reference, "measurement_run_reference"
        )
        instrument_code = require_non_empty(instrument_code, "instrument_code")
        transport = require_non_empty(transport, "transport")
        endpoint = require_non_empty(endpoint, "endpoint")
        command_message = require_non_empty(command_message, "command_message")
        response_message = str(response_message)
        raw_payload_json = require_non_empty(raw_payload_json, "raw_payload_json")
        success_value = int(success)
        observation_checksum = instrument_observation_checksum(
            project_code=project_code,
            campaign_reference=campaign_reference,
            measurement_run_reference=measurement_run_reference,
            sequence=sequence,
            instrument_code=instrument_code,
            transport=transport,
            endpoint=endpoint,
            command_message=command_message,
            response_message=response_message,
            success=success_value,
            exchange_attempts=exchange_attempts,
            raw_payload_json=raw_payload_json,
        )

        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    INSERT INTO instrument_observations (
                        project_code,
                        campaign_reference,
                        measurement_run_reference,
                        sequence,
                        instrument_code,
                        transport,
                        endpoint,
                        command_message,
                        response_message,
                        success,
                        exchange_attempts,
                        observed_at,
                        raw_payload_json,
                        observation_checksum
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        project_code,
                        campaign_reference,
                        measurement_run_reference,
                        sequence,
                        instrument_code,
                        transport,
                        endpoint,
                        command_message,
                        response_message,
                        success_value,
                        exchange_attempts,
                        utc_timestamp(),
                        raw_payload_json,
                        observation_checksum,
                    ),
                )
        return int(cursor.lastrowid)

    def get_instrument_observation_by_checksum(
        self,
        observation_checksum: str,
    ) -> dict[str, object] | None:
        observation_checksum = require_non_empty(
            observation_checksum, "observation_checksum"
        )
        with closing(self.connect()) as connection:
            row = connection.execute(
                """
                SELECT *
                FROM instrument_observations
                WHERE observation_checksum = ?
                """,
                (observation_checksum,),
            ).fetchone()
        return row_to_dict(row)

    def instrument_observations_for_run(
        self,
        measurement_run_reference: str,
    ) -> list[dict[str, object]]:
        measurement_run_reference = require_non_empty(
            measurement_run_reference, "measurement_run_reference"
        )
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM instrument_observations
                WHERE measurement_run_reference = ?
                ORDER BY observed_at, id
                """,
                (measurement_run_reference,),
            ).fetchall()
        return [dict(row) for row in rows]

    def instrument_observations_for_instrument(
        self,
        *,
        measurement_run_reference: str,
        instrument_code: str,
    ) -> list[dict[str, object]]:
        measurement_run_reference = require_non_empty(
            measurement_run_reference, "measurement_run_reference"
        )
        instrument_code = require_non_empty(instrument_code, "instrument_code")
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM instrument_observations
                WHERE measurement_run_reference = ?
                  AND instrument_code = ?
                ORDER BY sequence, id
                """,
                (measurement_run_reference, instrument_code),
            ).fetchall()
        return [dict(row) for row in rows]

    def latest_instrument_observations(self) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT observation.*
                FROM instrument_observations observation
                JOIN (
                    SELECT
                        measurement_run_reference,
                        instrument_code,
                        MAX(sequence) AS sequence
                    FROM instrument_observations
                    GROUP BY measurement_run_reference, instrument_code
                ) latest
                    ON latest.measurement_run_reference = observation.measurement_run_reference
                   AND latest.instrument_code = observation.instrument_code
                   AND latest.sequence = observation.sequence
                ORDER BY observation.observed_at DESC, observation.id DESC
                """
            ).fetchall()
        return [dict(row) for row in rows]

    def record_retention_event(
        self,
        *,
        dataset_id: int,
        new_status: str,
        actor: str,
        reason: str,
        audit_event_reference: str | None = None,
    ) -> int:
        new_status = validate_retention_status(new_status)
        actor = require_non_empty(actor, "actor")
        reason = require_non_empty(reason, "reason")
        if audit_event_reference is not None:
            audit_event_reference = require_non_empty(
                audit_event_reference, "audit_event_reference"
            )

        with closing(self.connect()) as connection:
            with connection:
                dataset = connection.execute(
                    """
                    SELECT id, immutable, retention_status
                    FROM datasets
                    WHERE id = ?
                    """,
                    (dataset_id,),
                ).fetchone()
                if dataset is None:
                    raise ValueError("dataset does not exist")

                previous_status = str(dataset["retention_status"])
                immutable = bool(dataset["immutable"])
                if not retention_transition_allowed(
                    previous_status, new_status, immutable
                ):
                    raise ValueError(
                        "invalid retention transition: "
                        f"{previous_status} -> {new_status}"
                    )

                now = utc_timestamp()
                cursor = connection.execute(
                    """
                    INSERT INTO dataset_retention_events (
                        dataset_id,
                        previous_status,
                        new_status,
                        actor,
                        reason,
                        event_at,
                        audit_event_reference
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        dataset_id,
                        previous_status,
                        new_status,
                        actor,
                        reason,
                        now,
                        audit_event_reference,
                    ),
                )
                connection.execute(
                    """
                    UPDATE datasets
                    SET retention_status = ?
                    WHERE id = ?
                    """,
                    (new_status, dataset_id),
                )
        return int(cursor.lastrowid)

    def retention_events(self, dataset_id: int) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM dataset_retention_events
                WHERE dataset_id = ?
                ORDER BY id
                """,
                (dataset_id,),
            ).fetchall()
        return [dict(row) for row in rows]

    def datasets_by_retention_status(self, status: str) -> list[dict[str, object]]:
        status = validate_retention_status(status)
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM datasets
                WHERE retention_status = ?
                ORDER BY acquired_at, id
                """,
                (status,),
            ).fetchall()
        return [dict(row) for row in rows]

    def add_signal_channel(
        self,
        *,
        dataset_id: int,
        name: str,
        source_kind: str,
        unit: str,
        sample_rate_hz: float | None = None,
        sample_count: int | None = None,
        synchronization_reference: str | None = None,
    ) -> int:
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    INSERT INTO signal_channels (
                        dataset_id,
                        name,
                        source_kind,
                        unit,
                        sample_rate_hz,
                        sample_count,
                        synchronization_reference
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        dataset_id,
                        name,
                        source_kind,
                        unit,
                        sample_rate_hz,
                        sample_count,
                        synchronization_reference,
                    ),
                )
        return int(cursor.lastrowid)

    def signal_channels(self, dataset_id: int) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM signal_channels
                WHERE dataset_id = ?
                ORDER BY name
                """,
                (dataset_id,),
            ).fetchall()
        return [dict(row) for row in rows]

    def add_processing_graph(
        self,
        *,
        source_dataset_id: int,
        graph_reference: str,
        operations_json: str,
        created_by: str,
        checksum: str,
    ) -> int:
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    INSERT INTO processing_graphs (
                        source_dataset_id,
                        graph_reference,
                        operations_json,
                        created_by,
                        created_at,
                        checksum
                    )
                    VALUES (?, ?, ?, ?, ?, ?)
                    """,
                    (
                        source_dataset_id,
                        graph_reference,
                        operations_json,
                        created_by,
                        utc_timestamp(),
                        checksum,
                    ),
                )
        return int(cursor.lastrowid)

    def processing_graphs_for_dataset(self, dataset_id: int) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM processing_graphs
                WHERE source_dataset_id = ?
                ORDER BY created_at, id
                """,
                (dataset_id,),
            ).fetchall()
        return [dict(row) for row in rows]

    def add_processing_graph_instance(
        self,
        *,
        source_dataset_id: int,
        graph_reference: str,
        graph_revision: str,
        operations_json: str,
        created_by: str,
        software_version: str,
        graph_checksum: str,
        source_dataset_checksum: str | None = None,
        status: str = "active",
    ) -> int:
        if status not in PROCESSING_GRAPH_STATUSES:
            raise ValueError(f"invalid processing graph status: {status}")
        software_version = require_non_empty(software_version, "software_version")

        with closing(self.connect()) as connection:
            with connection:
                dataset = connection.execute(
                    "SELECT checksum FROM datasets WHERE id = ?",
                    (source_dataset_id,),
                ).fetchone()
                if dataset is None:
                    raise ValueError("source dataset does not exist")

                stored_dataset_checksum = str(dataset["checksum"])
                if source_dataset_checksum is None:
                    source_dataset_checksum = stored_dataset_checksum
                elif source_dataset_checksum != stored_dataset_checksum:
                    raise ValueError("source dataset checksum mismatch")

                cursor = connection.execute(
                    """
                    INSERT INTO processing_graph_instances (
                        source_dataset_id,
                        graph_reference,
                        graph_revision,
                        operations_json,
                        created_by,
                        created_at,
                        software_version,
                        source_dataset_checksum,
                        graph_checksum,
                        status
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        source_dataset_id,
                        graph_reference,
                        graph_revision,
                        operations_json,
                        created_by,
                        utc_timestamp(),
                        software_version,
                        source_dataset_checksum,
                        graph_checksum,
                        status,
                    ),
                )
        return int(cursor.lastrowid)

    def get_processing_graph_instance(
        self,
        instance_id: int,
    ) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                """
                SELECT *
                FROM processing_graph_instances
                WHERE id = ?
                """,
                (instance_id,),
            ).fetchone()
        return dict(row) if row else None

    def processing_graph_instance(
        self,
        *,
        source_dataset_id: int,
        graph_reference: str,
        graph_revision: str,
    ) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                """
                SELECT *
                FROM processing_graph_instances
                WHERE source_dataset_id = ?
                  AND graph_reference = ?
                  AND graph_revision = ?
                """,
                (source_dataset_id, graph_reference, graph_revision),
            ).fetchone()
        return dict(row) if row else None

    def processing_graph_instances_for_dataset(
        self,
        dataset_id: int,
    ) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM processing_graph_instances
                WHERE source_dataset_id = ?
                ORDER BY graph_reference, graph_revision, id
                """,
                (dataset_id,),
            ).fetchall()
        return [dict(row) for row in rows]

    def add_processing_graph_instance_artifact(
        self,
        *,
        processing_graph_instance_id: int,
        output_signal_reference: str,
        artifact_kind: str,
        file_reference: str,
        checksum: str,
        raw_lineage_json: str = "[]",
    ) -> int:
        if artifact_kind not in PROCESSING_GRAPH_ARTIFACT_KINDS:
            raise ValueError(f"invalid processing graph artifact kind: {artifact_kind}")
        _validate_signal_reference(
            output_signal_reference,
            field_name="output signal reference",
        )
        _validate_raw_lineage_json(raw_lineage_json)

        with closing(self.connect()) as connection:
            with connection:
                instance = connection.execute(
                    "SELECT id FROM processing_graph_instances WHERE id = ?",
                    (processing_graph_instance_id,),
                ).fetchone()
                if instance is None:
                    raise ValueError("processing graph instance does not exist")

                cursor = connection.execute(
                    """
                    INSERT INTO processing_graph_instance_artifacts (
                        processing_graph_instance_id,
                        output_signal_reference,
                        artifact_kind,
                        file_reference,
                        checksum,
                        created_at,
                        raw_lineage_json
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        processing_graph_instance_id,
                        output_signal_reference,
                        artifact_kind,
                        file_reference,
                        checksum,
                        utc_timestamp(),
                        raw_lineage_json,
                    ),
                )
        return int(cursor.lastrowid)

    def processing_graph_instance_artifacts(
        self,
        processing_graph_instance_id: int,
    ) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM processing_graph_instance_artifacts
                WHERE processing_graph_instance_id = ?
                ORDER BY created_at, id
                """,
                (processing_graph_instance_id,),
            ).fetchall()
        return [dict(row) for row in rows]

    def add_processing_graph_execution(
        self,
        *,
        processing_graph_instance_id: int,
        execution_reference: str,
        executed_by: str,
        software_version: str,
        status: str,
        output_artifact_count: int,
        notes: str | None = None,
    ) -> int:
        if status not in PROCESSING_GRAPH_EXECUTION_STATUSES:
            raise ValueError(f"invalid processing graph execution status: {status}")
        if output_artifact_count < 0:
            raise ValueError("output artifact count must be non-negative")
        if status == "completed" and output_artifact_count == 0:
            raise ValueError("completed processing graph execution requires artifacts")
        software_version = require_non_empty(software_version, "software_version")

        with closing(self.connect()) as connection:
            with connection:
                instance = connection.execute(
                    "SELECT id FROM processing_graph_instances WHERE id = ?",
                    (processing_graph_instance_id,),
                ).fetchone()
                if instance is None:
                    raise ValueError("processing graph instance does not exist")

                artifact_count = connection.execute(
                    """
                    SELECT COUNT(*) AS artifact_count
                    FROM processing_graph_instance_artifacts
                    WHERE processing_graph_instance_id = ?
                    """,
                    (processing_graph_instance_id,),
                ).fetchone()["artifact_count"]
                if output_artifact_count != artifact_count:
                    raise ValueError(
                        "processing graph execution artifact count does not "
                        "match persisted artifacts"
                    )

                cursor = connection.execute(
                    """
                    INSERT INTO processing_graph_executions (
                        processing_graph_instance_id,
                        execution_reference,
                        executed_by,
                        executed_at,
                        software_version,
                        status,
                        output_artifact_count,
                        notes
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        processing_graph_instance_id,
                        execution_reference,
                        executed_by,
                        utc_timestamp(),
                        software_version,
                        status,
                        output_artifact_count,
                        notes,
                    ),
                )
        return int(cursor.lastrowid)

    def processing_graph_executions(
        self,
        processing_graph_instance_id: int,
    ) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM processing_graph_executions
                WHERE processing_graph_instance_id = ?
                ORDER BY executed_at, id
                """,
                (processing_graph_instance_id,),
            ).fetchall()
        return [dict(row) for row in rows]

    def add_result_artifact(
        self,
        *,
        processing_graph_id: int,
        artifact_kind: str,
        file_reference: str,
        checksum: str,
    ) -> int:
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    INSERT INTO result_artifacts (
                        processing_graph_id,
                        artifact_kind,
                        file_reference,
                        checksum,
                        created_at
                    )
                    VALUES (?, ?, ?, ?, ?)
                    """,
                    (
                        processing_graph_id,
                        artifact_kind,
                        file_reference,
                        checksum,
                        utc_timestamp(),
                    ),
                )
        return int(cursor.lastrowid)

    def result_artifacts(self, processing_graph_id: int) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM result_artifacts
                WHERE processing_graph_id = ?
                ORDER BY created_at, id
                """,
                (processing_graph_id,),
            ).fetchall()
        return [dict(row) for row in rows]


class TestDefinitionRepository(SQLiteDomainRepository):
    """SQLite adapter for standards, methods, revisions, and test steps."""

    def __init__(self, database_path: Path | str, migrations_root: Path | str) -> None:
        super().__init__(Path(database_path), Path(migrations_root), "test_definitions")

    def add_standard(
        self,
        *,
        code: str,
        title: str,
        edition: str,
        issuer: str,
        status: str = "active",
    ) -> None:
        now = utc_timestamp()
        with closing(self.connect()) as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO standards (
                        code,
                        title,
                        edition,
                        issuer,
                        status,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (code, title, edition, issuer, status, now, now),
                )

    def get_standard(self, code: str) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT * FROM standards WHERE code = ?",
                (code,),
            ).fetchone()
        return row_to_dict(row)

    def list_standards(self) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute("SELECT * FROM standards ORDER BY code").fetchall()
        return [dict(row) for row in rows]

    def add_test_method(
        self,
        *,
        code: str,
        standard_code: str | None,
        name: str,
        family: str,
        measurement_axis: str,
        controlled: bool = True,
        category_code: str | None = None,
    ) -> None:
        now = utc_timestamp()
        with closing(self.connect()) as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO test_methods (
                        code,
                        standard_code,
                        name,
                        family,
                        measurement_axis,
                        controlled,
                        category_code,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        code,
                        standard_code,
                        name,
                        family,
                        measurement_axis,
                        int(controlled),
                        category_code,
                        now,
                        now,
                    ),
                )

    def add_test_category(
        self,
        *,
        code: str,
        label: str,
        description: str,
        parent_code: str | None = None,
        active: bool = True,
        sort_order: int = 0,
    ) -> None:
        now = utc_timestamp()
        with closing(self.connect()) as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO test_categories (
                        code,
                        parent_code,
                        label,
                        description,
                        active,
                        sort_order,
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        code,
                        parent_code,
                        label,
                        description,
                        int(active),
                        sort_order,
                        now,
                        now,
                    ),
                )

    def list_test_categories(
        self,
        *,
        parent_code: str | None = None,
        active_only: bool = True,
    ) -> list[dict[str, object]]:
        clauses: list[str] = []
        parameters: list[str] = []
        if parent_code is None:
            clauses.append("parent_code IS NULL")
        else:
            clauses.append("parent_code = ?")
            parameters.append(parent_code)
        if active_only:
            clauses.append("active = 1")

        with closing(self.connect()) as connection:
            rows = connection.execute(
                f"""
                SELECT *
                FROM test_categories
                WHERE {' AND '.join(clauses)}
                ORDER BY sort_order, label, code
                """,
                tuple(parameters),
            ).fetchall()
        return [dict(row) for row in rows]

    def list_all_test_categories(self, *, active_only: bool = True) -> list[dict[str, object]]:
        where_clause = "WHERE active = 1" if active_only else ""
        with closing(self.connect()) as connection:
            rows = connection.execute(
                f"""
                SELECT *
                FROM test_categories
                {where_clause}
                ORDER BY COALESCE(parent_code, ''), sort_order, label, code
                """
            ).fetchall()
        return [dict(row) for row in rows]

    def get_test_category(self, code: str) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT * FROM test_categories WHERE code = ?",
                (code,),
            ).fetchone()
        return row_to_dict(row)

    def get_test_method(self, code: str) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT * FROM test_methods WHERE code = ?",
                (code,),
            ).fetchone()
        return row_to_dict(row)

    def list_test_methods(self, family: str | None = None) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            if family is None:
                rows = connection.execute(
                    "SELECT * FROM test_methods ORDER BY family, code"
                ).fetchall()
            else:
                rows = connection.execute(
                    """
                    SELECT *
                    FROM test_methods
                    WHERE family = ?
                    ORDER BY code
                    """,
                    (family,),
                ).fetchall()
        return [dict(row) for row in rows]

    def add_method_revision(
        self,
        *,
        method_code: str,
        revision: str,
        status: str = "draft",
        parameters_json: str = "{}",
        acceptance_criteria_json: str = "{}",
        processing_graph_json: str = "{}",
        checksum: str | None = None,
    ) -> int:
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    INSERT INTO test_method_revisions (
                        method_code,
                        revision,
                        status,
                        parameters_json,
                        acceptance_criteria_json,
                        processing_graph_json,
                        checksum
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        method_code,
                        revision,
                        status,
                        parameters_json,
                        acceptance_criteria_json,
                        processing_graph_json,
                        checksum,
                    ),
                )
        return int(cursor.lastrowid)

    def approve_method_revision(
        self,
        *,
        method_code: str,
        revision: str,
        approved_by: str,
        checksum: str | None = None,
    ) -> bool:
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    UPDATE test_method_revisions
                    SET status = 'approved',
                        approved_by = ?,
                        approved_at = ?,
                        checksum = COALESCE(?, checksum)
                    WHERE method_code = ? AND revision = ?
                    """,
                    (approved_by, utc_timestamp(), checksum, method_code, revision),
                )
        return cursor.rowcount == 1

    def method_revisions(self, method_code: str) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM test_method_revisions
                WHERE method_code = ?
                ORDER BY revision
                """,
                (method_code,),
            ).fetchall()
        return [dict(row) for row in rows]

    def add_test_step(
        self,
        *,
        method_revision_id: int,
        sequence: int,
        name: str,
        instruction: str,
        expected_evidence: str,
    ) -> int:
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    INSERT INTO test_steps (
                        method_revision_id,
                        sequence,
                        name,
                        instruction,
                        expected_evidence
                    )
                    VALUES (?, ?, ?, ?, ?)
                    """,
                    (
                        method_revision_id,
                        sequence,
                        name,
                        instruction,
                        expected_evidence,
                    ),
                )
        return int(cursor.lastrowid)

    def test_steps(self, method_revision_id: int) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM test_steps
                WHERE method_revision_id = ?
                ORDER BY sequence
                """,
                (method_revision_id,),
            ).fetchall()
        return [dict(row) for row in rows]


class SyncRepository(SQLiteDomainRepository):
    """SQLite adapter for synchronization conflicts and action plans."""

    def __init__(self, database_path: Path | str, migrations_root: Path | str) -> None:
        super().__init__(Path(database_path), Path(migrations_root), "sync")

    def record_conflict(
        self,
        *,
        conflict_id: str,
        domain: str,
        kind: str,
        local_snapshot: str,
        reference_snapshot: str,
    ) -> None:
        now = utc_timestamp()
        with closing(self.connect()) as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO sync_conflicts (
                        conflict_id,
                        domain,
                        kind,
                        local_snapshot,
                        reference_snapshot,
                        status,
                        detected_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, 'open', ?, ?)
                    """,
                    (
                        conflict_id,
                        domain,
                        kind,
                        local_snapshot,
                        reference_snapshot,
                        now,
                        now,
                    ),
                )

    def conflict_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute("SELECT COUNT(*) AS count FROM sync_conflicts").fetchone()
        return int(row["count"])

    def action_plan_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT COUNT(*) AS count FROM sync_conflict_action_plans"
            ).fetchone()
        return int(row["count"])

    def operation_count(self, status: str | None = None) -> int:
        with closing(self.connect()) as connection:
            if status is None:
                row = connection.execute(
                    "SELECT COUNT(*) AS count FROM sync_operations"
                ).fetchone()
            else:
                row = connection.execute(
                    """
                    SELECT COUNT(*) AS count
                    FROM sync_operations
                    WHERE status = ?
                    """,
                    (status,),
                ).fetchone()
        return int(row["count"])

    def record_operation(
        self,
        *,
        operation_id: str,
        domain: str,
        entity_type: str,
        entity_id: str,
        operation_kind: str,
        base_revision: str,
        resulting_revision: str,
        actor_id: str,
        device_id: str,
        correlation_id: str,
        payload_checksum: str,
        payload_json: str = "{}",
        occurred_at: str | None = None,
    ) -> None:
        payload_json = normalized_json_text(payload_json, "payload_json")
        now = utc_timestamp()
        occurred_at = require_non_empty(occurred_at or now, "occurred_at")
        with closing(self.connect()) as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO sync_operations (
                        operation_id,
                        domain,
                        entity_type,
                        entity_id,
                        operation_kind,
                        base_revision,
                        resulting_revision,
                        actor_id,
                        device_id,
                        correlation_id,
                        payload_json,
                        payload_checksum,
                        status,
                        occurred_at,
                        recorded_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?)
                    """,
                    (
                        require_non_empty(operation_id, "operation_id"),
                        require_non_empty(domain, "domain"),
                        require_non_empty(entity_type, "entity_type"),
                        require_non_empty(entity_id, "entity_id"),
                        require_non_empty(operation_kind, "operation_kind"),
                        require_non_empty(base_revision, "base_revision"),
                        require_non_empty(resulting_revision, "resulting_revision"),
                        require_non_empty(actor_id, "actor_id"),
                        require_non_empty(device_id, "device_id"),
                        require_non_empty(correlation_id, "correlation_id"),
                        payload_json,
                        require_sha256_checksum(payload_checksum, "payload_checksum"),
                        occurred_at,
                        now,
                    ),
                )

    def get_operation(self, operation_id: str) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT * FROM sync_operations WHERE operation_id = ?",
                (operation_id,),
            ).fetchone()
        return row_to_dict(row)

    def list_operations(
        self,
        *,
        status: str | None = None,
        domain: str | None = None,
    ) -> list[dict[str, object]]:
        clauses: list[str] = []
        values: list[str] = []
        if status is not None:
            clauses.append("status = ?")
            values.append(status)
        if domain is not None:
            clauses.append("domain = ?")
            values.append(domain)
        where = f"WHERE {' AND '.join(clauses)}" if clauses else ""
        with closing(self.connect()) as connection:
            rows = connection.execute(
                f"""
                SELECT *
                FROM sync_operations
                {where}
                ORDER BY recorded_at, operation_id
                """,
                tuple(values),
            ).fetchall()
        return [dict(row) for row in rows]

    def mark_operation_applied(self, operation_id: str) -> bool:
        now = utc_timestamp()
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    UPDATE sync_operations
                    SET status = 'applied',
                        applied_at = ?,
                        error_message = NULL
                    WHERE operation_id = ? AND status = 'pending'
                    """,
                    (now, operation_id),
                )
        return cursor.rowcount == 1

    def mark_operation_failed(self, *, operation_id: str, error_message: str) -> bool:
        now = utc_timestamp()
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    UPDATE sync_operations
                    SET status = 'failed',
                        applied_at = ?,
                        error_message = ?
                    WHERE operation_id = ? AND status = 'pending'
                    """,
                    (now, require_non_empty(error_message, "error_message"), operation_id),
                )
        return cursor.rowcount == 1

    def snapshot_count(self, domain: str | None = None) -> int:
        with closing(self.connect()) as connection:
            if domain is None:
                row = connection.execute(
                    "SELECT COUNT(*) AS count FROM sync_entity_snapshots"
                ).fetchone()
            else:
                row = connection.execute(
                    """
                    SELECT COUNT(*) AS count
                    FROM sync_entity_snapshots
                    WHERE domain = ?
                    """,
                    (domain,),
                ).fetchone()
        return int(row["count"])

    def record_entity_snapshot(
        self,
        *,
        snapshot_id: str,
        domain: str,
        entity_type: str,
        entity_id: str,
        revision: str,
        snapshot_checksum: str,
        payload_json: str = "{}",
        source_operation_id: str | None = None,
        captured_at: str | None = None,
    ) -> None:
        payload_json = normalized_json_text(payload_json, "payload_json")
        captured_at = require_non_empty(captured_at or utc_timestamp(), "captured_at")
        with closing(self.connect()) as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO sync_entity_snapshots (
                        snapshot_id,
                        domain,
                        entity_type,
                        entity_id,
                        revision,
                        snapshot_checksum,
                        payload_json,
                        source_operation_id,
                        captured_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        require_non_empty(snapshot_id, "snapshot_id"),
                        require_non_empty(domain, "domain"),
                        require_non_empty(entity_type, "entity_type"),
                        require_non_empty(entity_id, "entity_id"),
                        require_non_empty(revision, "revision"),
                        require_sha256_checksum(snapshot_checksum, "snapshot_checksum"),
                        payload_json,
                        optional_non_empty(source_operation_id, "source_operation_id"),
                        captured_at,
                    ),
                )

    def apply_pending_operation_snapshot(
        self,
        *,
        operation_id: str,
        snapshot_id: str,
        snapshot_checksum: str,
        payload_json: str = "{}",
        captured_at: str | None = None,
    ) -> bool:
        payload_json = normalized_json_text(payload_json, "payload_json")
        snapshot_id = require_non_empty(snapshot_id, "snapshot_id")
        snapshot_checksum = require_sha256_checksum(snapshot_checksum, "snapshot_checksum")
        applied_at = utc_timestamp()
        captured_at = require_non_empty(captured_at or applied_at, "captured_at")
        with closing(self.connect()) as connection:
            with connection:
                operation = connection.execute(
                    """
                    SELECT *
                    FROM sync_operations
                    WHERE operation_id = ?
                      AND status = 'pending'
                    """,
                    (require_non_empty(operation_id, "operation_id"),),
                ).fetchone()
                if operation is None:
                    return False

                connection.execute(
                    """
                    INSERT INTO sync_entity_snapshots (
                        snapshot_id,
                        domain,
                        entity_type,
                        entity_id,
                        revision,
                        snapshot_checksum,
                        payload_json,
                        source_operation_id,
                        captured_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        snapshot_id,
                        operation["domain"],
                        operation["entity_type"],
                        operation["entity_id"],
                        operation["resulting_revision"],
                        snapshot_checksum,
                        payload_json,
                        operation["operation_id"],
                        captured_at,
                    ),
                )
                cursor = connection.execute(
                    """
                    UPDATE sync_operations
                    SET status = 'applied',
                        applied_at = ?,
                        error_message = NULL
                    WHERE operation_id = ?
                      AND status = 'pending'
                    """,
                    (applied_at, operation_id),
                )
        return cursor.rowcount == 1

    def get_entity_snapshot(self, snapshot_id: str) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT * FROM sync_entity_snapshots WHERE snapshot_id = ?",
                (snapshot_id,),
            ).fetchone()
        return row_to_dict(row)

    def latest_entity_snapshot(
        self,
        *,
        domain: str,
        entity_type: str,
        entity_id: str,
    ) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                """
                SELECT *
                FROM sync_entity_snapshots
                WHERE domain = ?
                  AND entity_type = ?
                  AND entity_id = ?
                ORDER BY captured_at DESC, snapshot_id DESC
                LIMIT 1
                """,
                (domain, entity_type, entity_id),
            ).fetchone()
        return row_to_dict(row)

    def record_snapshot_conflict(
        self,
        *,
        conflict_id: str,
        local_snapshot_id: str,
        reference_snapshot_id: str,
        kind: str = "checksum_mismatch",
    ) -> bool:
        now = utc_timestamp()
        with closing(self.connect()) as connection:
            with connection:
                local_snapshot = connection.execute(
                    "SELECT * FROM sync_entity_snapshots WHERE snapshot_id = ?",
                    (require_non_empty(local_snapshot_id, "local_snapshot_id"),),
                ).fetchone()
                reference_snapshot = connection.execute(
                    "SELECT * FROM sync_entity_snapshots WHERE snapshot_id = ?",
                    (require_non_empty(reference_snapshot_id, "reference_snapshot_id"),),
                ).fetchone()
                if local_snapshot is None:
                    raise ValueError("local snapshot does not exist")
                if reference_snapshot is None:
                    raise ValueError("reference snapshot does not exist")
                if (
                    local_snapshot["domain"] != reference_snapshot["domain"]
                    or local_snapshot["entity_type"] != reference_snapshot["entity_type"]
                    or local_snapshot["entity_id"] != reference_snapshot["entity_id"]
                ):
                    raise ValueError("snapshots do not describe the same entity")
                if (
                    local_snapshot["snapshot_checksum"]
                    == reference_snapshot["snapshot_checksum"]
                ):
                    return False

                connection.execute(
                    """
                    INSERT INTO sync_conflicts (
                        conflict_id,
                        domain,
                        kind,
                        local_snapshot,
                        reference_snapshot,
                        status,
                        detected_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, 'open', ?, ?)
                    """,
                    (
                        require_non_empty(conflict_id, "conflict_id"),
                        local_snapshot["domain"],
                        require_non_empty(kind, "kind"),
                        local_snapshot["snapshot_id"],
                        reference_snapshot["snapshot_id"],
                        now,
                        now,
                    ),
                )
        return True

    def upsert_checkpoint(
        self,
        *,
        peer_id: str,
        domain: str,
        direction: str,
        checkpoint_token: str,
        last_operation_id: str | None = None,
        last_snapshot_id: str | None = None,
        updated_at: str | None = None,
    ) -> None:
        now = require_non_empty(updated_at or utc_timestamp(), "updated_at")
        with closing(self.connect()) as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO sync_checkpoints (
                        peer_id,
                        domain,
                        direction,
                        last_operation_id,
                        last_snapshot_id,
                        checkpoint_token,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(peer_id, domain, direction)
                    DO UPDATE SET
                        last_operation_id = excluded.last_operation_id,
                        last_snapshot_id = excluded.last_snapshot_id,
                        checkpoint_token = excluded.checkpoint_token,
                        updated_at = excluded.updated_at
                    """,
                    (
                        require_non_empty(peer_id, "peer_id"),
                        require_non_empty(domain, "domain"),
                        validate_sync_checkpoint_direction(direction),
                        optional_non_empty(last_operation_id, "last_operation_id"),
                        optional_non_empty(last_snapshot_id, "last_snapshot_id"),
                        require_non_empty(checkpoint_token, "checkpoint_token"),
                        now,
                    ),
                )

    def get_checkpoint(
        self,
        *,
        peer_id: str,
        domain: str,
        direction: str,
    ) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                """
                SELECT *
                FROM sync_checkpoints
                WHERE peer_id = ?
                  AND domain = ?
                  AND direction = ?
                """,
                (
                    peer_id,
                    domain,
                    validate_sync_checkpoint_direction(direction),
                ),
            ).fetchone()
        return row_to_dict(row)

    def list_checkpoints(
        self,
        *,
        peer_id: str | None = None,
        domain: str | None = None,
    ) -> list[dict[str, object]]:
        clauses: list[str] = []
        values: list[str] = []
        if peer_id is not None:
            clauses.append("peer_id = ?")
            values.append(peer_id)
        if domain is not None:
            clauses.append("domain = ?")
            values.append(domain)
        where = f"WHERE {' AND '.join(clauses)}" if clauses else ""
        with closing(self.connect()) as connection:
            rows = connection.execute(
                f"""
                SELECT *
                FROM sync_checkpoints
                {where}
                ORDER BY updated_at, peer_id, domain, direction
                """,
                tuple(values),
            ).fetchall()
        return [dict(row) for row in rows]

    def get_conflict(self, conflict_id: str) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT * FROM sync_conflicts WHERE conflict_id = ?",
                (conflict_id,),
            ).fetchone()
        return row_to_dict(row)

    def list_conflicts(self, status: str | None = None) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            if status is None:
                rows = connection.execute(
                    "SELECT * FROM sync_conflicts ORDER BY detected_at, conflict_id"
                ).fetchall()
            else:
                rows = connection.execute(
                    """
                    SELECT *
                    FROM sync_conflicts
                    WHERE status = ?
                    ORDER BY detected_at, conflict_id
                    """,
                    (status,),
                ).fetchall()
        return [dict(row) for row in rows]

    def record_action_plan(
        self,
        *,
        conflict_id: str,
        domain: str,
        kind: str,
        resolution: str,
        action: str,
        local_snapshot: str,
        reference_snapshot: str,
        planned_by: str,
        requires_audit_event: bool = True,
    ) -> int:
        with closing(self.connect()) as connection:
            with connection:
                cursor = self._insert_action_plan(
                    connection,
                    conflict_id=conflict_id,
                    domain=domain,
                    kind=kind,
                    resolution=resolution,
                    action=action,
                    local_snapshot=local_snapshot,
                    reference_snapshot=reference_snapshot,
                    planned_by=planned_by,
                    requires_audit_event=requires_audit_event,
                )
        return int(cursor.lastrowid)

    def apply_resolution_plan(
        self,
        *,
        conflict_id: str,
        resolution: str,
        action: str,
        planned_by: str,
        requires_audit_event: bool = True,
        audit_event_reference: str | None = None,
    ) -> int | None:
        """Persist a plan and update the conflict outcome in one transaction."""

        now = utc_timestamp()
        status = "deferred" if resolution == "defer" else "resolved"
        with closing(self.connect()) as connection:
            with connection:
                conflict = connection.execute(
                    "SELECT * FROM sync_conflicts WHERE conflict_id = ?",
                    (conflict_id,),
                ).fetchone()
                if conflict is None or conflict["status"] == "resolved":
                    return None

                cursor = self._insert_action_plan(
                    connection,
                    conflict_id=conflict_id,
                    domain=str(conflict["domain"]),
                    kind=str(conflict["kind"]),
                    resolution=resolution,
                    action=action,
                    local_snapshot=str(conflict["local_snapshot"]),
                    reference_snapshot=str(conflict["reference_snapshot"]),
                    planned_by=planned_by,
                    requires_audit_event=requires_audit_event,
                )
                connection.execute(
                    """
                    UPDATE sync_conflicts
                    SET status = ?,
                        resolution = ?,
                        updated_at = ?
                    WHERE conflict_id = ?
                    """,
                    (status, resolution, now, conflict_id),
                )
                if audit_event_reference is not None:
                    connection.execute(
                        """
                        UPDATE sync_conflict_action_plans
                        SET applied_at = ?,
                            audit_event_reference = ?
                        WHERE id = ?
                        """,
                        (now, audit_event_reference, cursor.lastrowid),
                    )
        return int(cursor.lastrowid)

    def suggest_conflict_action_plan(
        self,
        *,
        conflict_id: str,
        planned_by: str,
        requires_audit_event: bool = True,
    ) -> int | None:
        with closing(self.connect()) as connection:
            with connection:
                conflict = connection.execute(
                    "SELECT * FROM sync_conflicts WHERE conflict_id = ?",
                    (require_non_empty(conflict_id, "conflict_id"),),
                ).fetchone()
                if conflict is None or conflict["status"] == "resolved":
                    return None

                existing = connection.execute(
                    """
                    SELECT id
                    FROM sync_conflict_action_plans
                    WHERE conflict_id = ?
                      AND applied_at IS NULL
                    ORDER BY sequence
                    LIMIT 1
                    """,
                    (conflict_id,),
                ).fetchone()
                if existing is not None:
                    return int(existing["id"])

                resolution, action = suggested_sync_action(str(conflict["kind"]))
                cursor = self._insert_action_plan(
                    connection,
                    conflict_id=conflict_id,
                    domain=str(conflict["domain"]),
                    kind=str(conflict["kind"]),
                    resolution=resolution,
                    action=action,
                    local_snapshot=str(conflict["local_snapshot"]),
                    reference_snapshot=str(conflict["reference_snapshot"]),
                    planned_by=planned_by,
                    requires_audit_event=requires_audit_event,
                )
        return int(cursor.lastrowid)

    def action_plans_for_conflict(self, conflict_id: str) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM sync_conflict_action_plans
                WHERE conflict_id = ?
                ORDER BY sequence
                """,
                (conflict_id,),
            ).fetchall()
        return [dict(row) for row in rows]

    @staticmethod
    def _insert_action_plan(
        connection: sqlite3.Connection,
        *,
        conflict_id: str,
        domain: str,
        kind: str,
        resolution: str,
        action: str,
        local_snapshot: str,
        reference_snapshot: str,
        planned_by: str,
        requires_audit_event: bool,
    ) -> sqlite3.Cursor:
        sequence_row = connection.execute(
            """
            SELECT COALESCE(MAX(sequence), 0) + 1 AS next_sequence
            FROM sync_conflict_action_plans
            WHERE conflict_id = ?
            """,
            (conflict_id,),
        ).fetchone()
        sequence = int(sequence_row["next_sequence"])
        return connection.execute(
            """
            INSERT INTO sync_conflict_action_plans (
                conflict_id,
                sequence,
                domain,
                kind,
                resolution,
                action,
                local_snapshot,
                reference_snapshot,
                requires_audit_event,
                planned_by,
                planned_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                conflict_id,
                sequence,
                domain,
                kind,
                resolution,
                action,
                local_snapshot,
                reference_snapshot,
                int(requires_audit_event),
                planned_by,
                utc_timestamp(),
            ),
        )


class UpdateCatalogRepository(SQLiteDomainRepository):
    """SQLite adapter for signed update package and install metadata."""

    def __init__(self, database_path: Path | str, migrations_root: Path | str) -> None:
        super().__init__(Path(database_path), Path(migrations_root), "update_catalog")

    def add_update_package(
        self,
        *,
        package_name: str,
        package_version: str,
        component: str,
        compatibility_range: str,
        signed_checksum: str,
        offline_install_allowed: bool = True,
    ) -> None:
        package_name = validate_update_package_name(package_name)
        package_version = validate_software_version(package_version)
        component = validate_update_component(component)
        compatibility_range = require_non_empty(
            compatibility_range, "compatibility_range"
        )
        signed_checksum = require_non_empty(signed_checksum, "signed_checksum")

        with closing(self.connect()) as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO update_packages (
                        package_name,
                        package_version,
                        component,
                        compatibility_range,
                        signed_checksum,
                        offline_install_allowed,
                        created_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        package_name,
                        package_version,
                        component,
                        compatibility_range,
                        signed_checksum,
                        int(offline_install_allowed),
                        utc_timestamp(),
                    ),
                )

    def update_package_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute("SELECT COUNT(*) AS count FROM update_packages").fetchone()
        return int(row["count"])

    def get_update_package(
        self,
        *,
        package_name: str,
        package_version: str,
        component: str,
    ) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                """
                SELECT *
                FROM update_packages
                WHERE package_name = ? AND package_version = ? AND component = ?
                """,
                (package_name, package_version, component),
            ).fetchone()
        return row_to_dict(row)

    def list_update_packages(self, component: str | None = None) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            if component is None:
                rows = connection.execute(
                    """
                    SELECT *
                    FROM update_packages
                    ORDER BY component, package_name, package_version
                    """
                ).fetchall()
            else:
                rows = connection.execute(
                    """
                    SELECT *
                    FROM update_packages
                    WHERE component = ?
                    ORDER BY package_name, package_version
                    """,
                    (component,),
                ).fetchall()
        return [dict(row) for row in rows]

    def record_install_validation(
        self,
        *,
        package_name: str,
        package_version: str,
        component: str,
        installed_version: str,
        source: str,
        compatibility_minimum_version: str,
        validated_by: str,
        compatibility_maximum_version: str | None = None,
        signature_required: bool = True,
        policy_offline_install_allowed: bool = True,
        measurement_active: bool = False,
        apply_during_measurement_allowed: bool = False,
    ) -> int:
        with closing(self.connect()) as connection:
            with connection:
                evidence = self._build_install_validation_evidence(
                    connection,
                    package_name=package_name,
                    package_version=package_version,
                    component=component,
                    installed_version=installed_version,
                    source=source,
                    compatibility_minimum_version=compatibility_minimum_version,
                    compatibility_maximum_version=compatibility_maximum_version,
                    signature_required=signature_required,
                    policy_offline_install_allowed=policy_offline_install_allowed,
                    measurement_active=measurement_active,
                    apply_during_measurement_allowed=apply_during_measurement_allowed,
                    validated_by=validated_by,
                )
                cursor = self._insert_install_validation_evidence(connection, evidence)
        return int(cursor.lastrowid)

    def get_install_validation_evidence(
        self,
        validation_evidence_id: int,
    ) -> dict[str, object] | None:
        with closing(self.connect()) as connection:
            row = connection.execute(
                """
                SELECT *
                FROM update_install_validation_evidence
                WHERE id = ?
                """,
                (validation_evidence_id,),
            ).fetchone()
        return row_to_dict(row)

    def validation_evidence_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT COUNT(*) AS count FROM update_install_validation_evidence"
            ).fetchone()
        return int(row["count"])

    def record_install(
        self,
        *,
        package_name: str,
        package_version: str,
        component: str,
        installed_by: str,
        source: str,
        rollback_reference: str | None = None,
        validation_evidence_id: int | None = None,
    ) -> int:
        package_name = validate_update_package_name(package_name)
        package_version = validate_software_version(package_version)
        component = validate_update_component(component)
        source = validate_update_source(source)
        installed_by = require_non_empty(installed_by, "installed_by")
        rollback_reference = (
            require_non_empty(rollback_reference, "rollback_reference")
            if rollback_reference is not None
            else None
        )

        with closing(self.connect()) as connection:
            with connection:
                if validation_evidence_id is not None:
                    self._require_accepted_validation_evidence(
                        connection,
                        validation_evidence_id=validation_evidence_id,
                        package_name=package_name,
                        package_version=package_version,
                        component=component,
                        source=source,
                    )

                cursor = connection.execute(
                    """
                    INSERT INTO update_install_records (
                        package_name,
                        package_version,
                        component,
                        installed_by,
                        installed_at,
                        source,
                        rollback_reference,
                        validation_evidence_id
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        package_name,
                        package_version,
                        component,
                        installed_by,
                        utc_timestamp(),
                        source,
                        rollback_reference,
                        validation_evidence_id,
                    ),
                )
        return int(cursor.lastrowid)

    def install_record_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute(
                "SELECT COUNT(*) AS count FROM update_install_records"
            ).fetchone()
        return int(row["count"])

    def list_install_records(self) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM update_install_records
                ORDER BY installed_at, id
                """
            ).fetchall()
        return [dict(row) for row in rows]

    def install_records_for_package(
        self,
        *,
        package_name: str,
        component: str,
    ) -> list[dict[str, object]]:
        with closing(self.connect()) as connection:
            rows = connection.execute(
                """
                SELECT *
                FROM update_install_records
                WHERE package_name = ? AND component = ?
                ORDER BY installed_at, id
                """,
                (package_name, component),
            ).fetchall()
        return [dict(row) for row in rows]

    @staticmethod
    def _build_install_validation_evidence(
        connection: sqlite3.Connection,
        *,
        package_name: str,
        package_version: str,
        component: str,
        installed_version: str,
        source: str,
        compatibility_minimum_version: str,
        compatibility_maximum_version: str | None,
        signature_required: bool,
        policy_offline_install_allowed: bool,
        measurement_active: bool,
        apply_during_measurement_allowed: bool,
        validated_by: str,
    ) -> dict[str, object]:
        package_name = validate_update_package_name(package_name)
        package_version = validate_software_version(package_version)
        component = validate_update_component(component)
        installed_version = validate_software_version(installed_version)
        source = validate_update_source(source)
        compatibility_minimum_version = validate_software_version(
            compatibility_minimum_version
        )
        if compatibility_maximum_version is not None:
            compatibility_maximum_version = validate_software_version(
                compatibility_maximum_version
            )
            if version_tuple(compatibility_maximum_version) < version_tuple(
                compatibility_minimum_version
            ):
                raise ValueError("compatibility_maximum_version must not be below minimum")
        validated_by = require_non_empty(validated_by, "validated_by")

        package = connection.execute(
            """
            SELECT *
            FROM update_packages
            WHERE package_name = ? AND package_version = ? AND component = ?
            """,
            (package_name, package_version, component),
        ).fetchone()
        if package is None:
            raise ValueError("update package must exist before validation")

        signature_present = bool(str(package["signed_checksum"]).strip())
        package_offline_install_allowed = bool(package["offline_install_allowed"])
        reasons: list[str] = []

        if signature_required and not signature_present:
            reasons.append("unsigned_package")
        if source == "offline_bundle" and (
            not policy_offline_install_allowed or not package_offline_install_allowed
        ):
            reasons.append("offline_install_blocked")
        if measurement_active and not apply_during_measurement_allowed:
            reasons.append("measurement_active")

        installed = version_tuple(installed_version)
        if installed < version_tuple(compatibility_minimum_version) or (
            compatibility_maximum_version is not None
            and installed > version_tuple(compatibility_maximum_version)
        ):
            reasons.append("incompatible_installed_version")

        return {
            "package_name": package_name,
            "package_version": package_version,
            "component": component,
            "installed_version": installed_version,
            "source": source,
            "validation_status": "rejected" if reasons else "accepted",
            "signature_required": int(signature_required),
            "signature_present": int(signature_present),
            "compatibility_minimum_version": compatibility_minimum_version,
            "compatibility_maximum_version": compatibility_maximum_version,
            "package_offline_install_allowed": int(package_offline_install_allowed),
            "policy_offline_install_allowed": int(policy_offline_install_allowed),
            "measurement_active": int(measurement_active),
            "apply_during_measurement_allowed": int(apply_during_measurement_allowed),
            "reason": ";".join(reasons) if reasons else None,
            "validated_by": validated_by,
        }

    @staticmethod
    def _insert_install_validation_evidence(
        connection: sqlite3.Connection,
        evidence: dict[str, object],
    ) -> sqlite3.Cursor:
        return connection.execute(
            """
            INSERT INTO update_install_validation_evidence (
                package_name,
                package_version,
                component,
                installed_version,
                source,
                validation_status,
                signature_required,
                signature_present,
                compatibility_minimum_version,
                compatibility_maximum_version,
                package_offline_install_allowed,
                policy_offline_install_allowed,
                measurement_active,
                apply_during_measurement_allowed,
                reason,
                validated_by,
                validated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                evidence["package_name"],
                evidence["package_version"],
                evidence["component"],
                evidence["installed_version"],
                evidence["source"],
                evidence["validation_status"],
                evidence["signature_required"],
                evidence["signature_present"],
                evidence["compatibility_minimum_version"],
                evidence["compatibility_maximum_version"],
                evidence["package_offline_install_allowed"],
                evidence["policy_offline_install_allowed"],
                evidence["measurement_active"],
                evidence["apply_during_measurement_allowed"],
                evidence["reason"],
                evidence["validated_by"],
                utc_timestamp(),
            ),
        )

    @staticmethod
    def _require_accepted_validation_evidence(
        connection: sqlite3.Connection,
        *,
        validation_evidence_id: int,
        package_name: str,
        package_version: str,
        component: str,
        source: str,
    ) -> None:
        evidence = connection.execute(
            """
            SELECT *
            FROM update_install_validation_evidence
            WHERE id = ?
            """,
            (validation_evidence_id,),
        ).fetchone()
        if evidence is None:
            raise ValueError("validation evidence does not exist")
        if evidence["validation_status"] != "accepted":
            raise ValueError("validation evidence must be accepted before install")
        if (
            evidence["package_name"] != package_name
            or evidence["package_version"] != package_version
            or evidence["component"] != component
            or evidence["source"] != source
        ):
            raise ValueError("validation evidence does not match install record")


def row_to_dict(row: sqlite3.Row | None) -> dict[str, object] | None:
    if row is None:
        return None
    return dict(row)


def require_non_empty(value: str, field_name: str) -> str:
    trimmed = value.strip()
    if not trimmed:
        raise ValueError(f"{field_name} must not be empty")
    return trimmed


def optional_non_empty(value: str | None, field_name: str) -> str | None:
    if value is None:
        return None
    return require_non_empty(value, field_name)


def optional_text_or_none(value: str | None) -> str | None:
    if value is None:
        return None
    trimmed = value.strip()
    return trimmed or None


def normalized_json_text(value: str, field_name: str) -> str:
    text = require_non_empty(value, field_name)
    try:
        parsed = json.loads(text)
    except json.JSONDecodeError as error:
        raise ValueError(f"{field_name} must contain valid JSON") from error
    return json.dumps(parsed, sort_keys=True)


def require_sha256_checksum(value: str, field_name: str) -> str:
    trimmed = require_non_empty(value, field_name)
    if not _SHA256_CHECKSUM.fullmatch(trimmed):
        raise ValueError(f"{field_name} must be a sha256 checksum")
    return trimmed


def validate_sync_checkpoint_direction(value: str) -> str:
    trimmed = require_non_empty(value, "direction")
    if trimmed not in SYNC_CHECKPOINT_DIRECTIONS:
        raise ValueError(f"unknown sync checkpoint direction: {trimmed}")
    return trimmed


def suggested_sync_action(kind: str) -> tuple[str, str]:
    trimmed = require_non_empty(kind, "kind")
    if trimmed in {"checksum_mismatch", "concurrent_update"}:
        return ("manual_merge", "manual_merge")
    if trimmed in {"deleted_in_reference", "deleted_locally", "schema_mismatch"}:
        return ("defer", "defer_for_review")
    raise ValueError(f"unknown sync conflict kind: {trimmed}")


def validate_retention_status(value: str) -> str:
    trimmed = require_non_empty(value, "retention_status")
    if trimmed not in RETENTION_STATUSES:
        raise ValueError(f"unknown retention status: {trimmed}")
    return trimmed


def retention_transition_allowed(previous: str, next_status: str, immutable: bool) -> bool:
    if (previous, next_status) in {
        ("retained", "deletion_requested"),
        ("deletion_requested", "deletion_approved"),
        ("deletion_requested", "deletion_rejected"),
        ("deletion_approved", "deleted"),
    }:
        return True
    return not immutable and (previous, next_status) == ("retained", "deleted")


def instrument_observation_checksum(
    *,
    project_code: str,
    campaign_reference: str,
    measurement_run_reference: str,
    sequence: int,
    instrument_code: str,
    transport: str,
    endpoint: str,
    command_message: str,
    response_message: str,
    success: int,
    exchange_attempts: int,
    raw_payload_json: str,
) -> str:
    payload = {
        "campaign_reference": campaign_reference,
        "command_message": command_message,
        "endpoint": endpoint,
        "exchange_attempts": exchange_attempts,
        "instrument_code": instrument_code,
        "measurement_run_reference": measurement_run_reference,
        "project_code": project_code,
        "raw_payload_json": raw_payload_json,
        "response_message": response_message,
        "sequence": sequence,
        "success": success,
        "transport": transport,
    }
    encoded = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return f"sha256:{hashlib.sha256(encoded).hexdigest()}"


def validate_update_package_name(value: str) -> str:
    trimmed = require_non_empty(value, "package_name")
    if _PACKAGE_NAME.fullmatch(trimmed) is None:
        raise ValueError("package_name must contain only ASCII letters, digits, '.', '_', or '-'")
    return trimmed


def validate_software_version(value: str) -> str:
    trimmed = require_non_empty(value, "software_version")
    if _SOFTWARE_VERSION.fullmatch(trimmed) is None:
        raise ValueError("software_version must use MAJOR.MINOR.PATCH")
    return trimmed


def validate_update_component(value: str) -> str:
    trimmed = require_non_empty(value, "component")
    if trimmed not in UPDATE_COMPONENTS:
        raise ValueError(f"unknown update component: {trimmed}")
    return trimmed


def validate_update_source(value: str) -> str:
    trimmed = require_non_empty(value, "source")
    if trimmed not in UPDATE_SOURCES:
        raise ValueError(f"unknown update source: {trimmed}")
    return trimmed


def version_tuple(value: str) -> tuple[int, int, int]:
    validated = validate_software_version(value)
    major, minor, patch = validated.split(".")
    return int(major), int(minor), int(patch)
