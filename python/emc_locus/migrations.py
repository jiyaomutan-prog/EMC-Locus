"""SQLite migration discovery and validation helpers."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re
import sqlite3


MIGRATION_FILENAME = re.compile(r"^(?P<version>\d{4})_(?P<slug>[a-z0-9_]+)\.sql$")


@dataclass(frozen=True)
class Migration:
    """A versioned SQL migration in one storage domain."""

    domain: str
    version: int
    slug: str
    path: Path


def discover_migrations(root: Path | str) -> tuple[Migration, ...]:
    """Return migrations ordered by domain and version."""

    root = Path(root)
    if not root.exists():
        raise FileNotFoundError(root)

    migrations: list[Migration] = []
    seen_versions: set[tuple[str, int]] = set()

    for domain_dir in sorted(path for path in root.iterdir() if path.is_dir()):
        domain = domain_dir.name

        for sql_file in sorted(domain_dir.glob("*.sql")):
            match = MIGRATION_FILENAME.match(sql_file.name)
            if match is None:
                raise ValueError(f"Invalid migration filename: {sql_file}")

            version = int(match.group("version"))
            key = (domain, version)
            if key in seen_versions:
                raise ValueError(f"Duplicate migration version {version:04d} in {domain}")

            seen_versions.add(key)
            migrations.append(
                Migration(
                    domain=domain,
                    version=version,
                    slug=match.group("slug"),
                    path=sql_file,
                )
            )

    return tuple(sorted(migrations, key=lambda item: (item.domain, item.version, item.slug)))


def validate_sqlite_migrations(root: Path | str) -> dict[str, int]:
    """Execute every domain's migrations in a fresh in-memory SQLite database."""

    migrations = discover_migrations(root)
    domains = sorted({migration.domain for migration in migrations})
    validated: dict[str, int] = {}

    for domain in domains:
        connection = sqlite3.connect(":memory:")
        try:
            domain_migrations = [
                migration for migration in migrations if migration.domain == domain
            ]
            for migration in sorted(domain_migrations, key=lambda item: item.version):
                sql = migration.path.read_text(encoding="utf-8")
                connection.executescript(sql)
            validated[domain] = len(domain_migrations)
        finally:
            connection.close()

    return validated
