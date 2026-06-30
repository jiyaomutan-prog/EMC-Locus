"""Data loading helpers for the Qt operator console."""

from __future__ import annotations

from pathlib import Path

from .gui_bootstrap import BootstrapData, build_bootstrap
from .local_agent_client import LocalAgentClient
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
    agent_url: str | None = None,
) -> BootstrapData:
    """Build console bootstrap data directly from available local repositories."""

    root = Path(migrations_root)
    agent_client = (
        LocalAgentClient(agent_url.strip())
        if agent_url is not None and agent_url.strip()
        else None
    )
    return build_bootstrap(
        project_agent=agent_client,
        metrology_agent=agent_client,
        projects=None
        if agent_client is not None
        else _repository_if_exists(ProjectRepository, projects_db, root),
        metrology=None
        if agent_client is not None
        else _repository_if_exists(MetrologyRepository, metrology_db, root),
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
