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


def row_to_dict(row: sqlite3.Row | None) -> dict[str, object] | None:
    if row is None:
        return None
    return dict(row)
