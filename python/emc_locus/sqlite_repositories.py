"""SQLite repository adapters for early local EMC Locus storage."""

from __future__ import annotations

from contextlib import closing
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
import sqlite3

from .migrations import discover_migrations


def utc_timestamp() -> str:
    """Return a compact UTC timestamp for deterministic storage columns."""

    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


@dataclass(frozen=True)
class SQLiteDomainRepository:
    """A single SQLite database backed by one migration domain."""

    database_path: Path
    migrations_root: Path
    domain: str

    def initialize(self) -> None:
        self.database_path.parent.mkdir(parents=True, exist_ok=True)
        with closing(self.connect()) as connection:
            if self._has_schema(connection):
                return

            migrations = [
                migration
                for migration in discover_migrations(self.migrations_root)
                if migration.domain == self.domain
            ]
            with connection:
                for migration in migrations:
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
    ) -> None:
        now = utc_timestamp()
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
                        created_at,
                        updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
                        now,
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

    def instrument_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute("SELECT COUNT(*) AS count FROM instruments").fetchone()
        return int(row["count"])

    def calibration_count(self) -> int:
        with closing(self.connect()) as connection:
            row = connection.execute("SELECT COUNT(*) AS count FROM calibration_records").fetchone()
        return int(row["count"])

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

    def update_instrument_availability(self, *, asset_id: str, availability: str) -> bool:
        now = utc_timestamp()
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    UPDATE instruments
                    SET availability = ?, updated_at = ?
                    WHERE asset_id = ?
                    """,
                    (availability, now, asset_id),
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

    def record_install(
        self,
        *,
        package_name: str,
        package_version: str,
        component: str,
        installed_by: str,
        source: str,
        rollback_reference: str | None = None,
    ) -> int:
        with closing(self.connect()) as connection:
            with connection:
                cursor = connection.execute(
                    """
                    INSERT INTO update_install_records (
                        package_name,
                        package_version,
                        component,
                        installed_by,
                        installed_at,
                        source,
                        rollback_reference
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        package_name,
                        package_version,
                        component,
                        installed_by,
                        utc_timestamp(),
                        source,
                        rollback_reference,
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


def row_to_dict(row: sqlite3.Row | None) -> dict[str, object] | None:
    if row is None:
        return None
    return dict(row)
