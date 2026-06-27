"""Data loading helpers for the Qt operator console."""

from __future__ import annotations

from pathlib import Path

from .gui_bootstrap import BootstrapData, build_bootstrap
from .sqlite_repositories import (
    MeasurementDataRepository,
    MetrologyRepository,
    ProjectRepository,
    TestDefinitionRepository,
    UpdateCatalogRepository,
)


def build_console_bootstrap_from_repositories(
    *,
    migrations_root: Path | str,
    projects_db: Path | str | None = None,
    metrology_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
) -> BootstrapData:
    """Build console bootstrap data directly from available local repositories."""

    root = Path(migrations_root)
    return build_bootstrap(
        projects=_repository_if_exists(ProjectRepository, projects_db, root),
        metrology=_repository_if_exists(MetrologyRepository, metrology_db, root),
        test_definitions=_repository_if_exists(
            TestDefinitionRepository,
            test_definitions_db,
            root,
        ),
        measurement_data=_repository_if_exists(
            MeasurementDataRepository,
            measurement_data_db,
            root,
        ),
        update_catalog=_repository_if_exists(UpdateCatalogRepository, update_catalog_db, root),
    )


def _repository_if_exists(
    repository_type: type[
        ProjectRepository
        | MetrologyRepository
        | TestDefinitionRepository
        | MeasurementDataRepository
        | UpdateCatalogRepository
    ],
    database_path: Path | str | None,
    migrations_root: Path,
):
    if database_path is None:
        return None

    path = Path(database_path)
    if not path.exists():
        return None

    repository = repository_type(path, migrations_root)
    repository.initialize()
    return repository
