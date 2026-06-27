"""Local write actions used by the static GUI workflow."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from .gui_bootstrap import build_bootstrap, write_bootstrap_js
from .sqlite_repositories import (
    MeasurementDataRepository,
    MetrologyRepository,
    ProjectRepository,
    TestDefinitionRepository,
    UpdateCatalogRepository,
    require_non_empty,
)


PROJECT_STAGE_FLOW = (
    "quotation",
    "contract_review",
    "test_planning",
    "measuring",
    "technical_review",
    "report_issued",
    "archived",
)

DATASET_RETENTION_ACTIONS = {
    "request-deletion": "deletion_requested",
    "approve-deletion": "deletion_approved",
    "reject-deletion": "deletion_rejected",
    "mark-deleted": "deleted",
}


def advance_project_stage(
    *,
    projects_db: Path | str,
    code: str,
    actor: str,
    reason: str,
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    metrology_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
) -> dict[str, Any]:
    """Advance one project to the next workflow stage with audit evidence."""

    code = require_non_empty(code, "code")
    actor = require_non_empty(actor, "actor")
    reason = require_non_empty(reason, "reason")
    projects = ProjectRepository(Path(projects_db), Path(migrations_root))
    projects.initialize()

    project = projects.get_project(code)
    if project is None:
        raise ValueError("project does not exist")

    current_stage = str(project["stage"])
    next_stage = next_project_stage(current_stage)
    sequence = projects.set_project_stage_with_audit(
        code=code,
        stage=next_stage,
        actor=actor,
        reason=reason,
        action="gui_project_stage_advanced",
        payload_json=json.dumps(
            {"from": current_stage, "to": next_stage},
            sort_keys=True,
        ),
    )
    if sequence is None:
        raise ValueError("project does not exist")

    if bootstrap_output is not None:
        refresh_bootstrap(
            output=bootstrap_output,
            migrations_root=migrations_root,
            projects_db=projects_db,
            metrology_db=metrology_db,
            test_definitions_db=test_definitions_db,
            measurement_data_db=measurement_data_db,
            update_catalog_db=update_catalog_db,
        )

    return {
        "project_code": code,
        "previous_stage": current_stage,
        "new_stage": next_stage,
        "audit_sequence": sequence,
    }


def record_dataset_retention_action(
    *,
    measurement_data_db: Path | str,
    dataset_id: int,
    action: str,
    actor: str,
    reason: str,
    audit_event_reference: str | None = None,
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    projects_db: Path | str | None = None,
    metrology_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
) -> dict[str, Any]:
    """Record one reviewed dataset retention action and optionally refresh GUI data."""

    action = require_non_empty(action, "action")
    if action not in DATASET_RETENTION_ACTIONS:
        raise ValueError(f"unknown dataset retention action: {action}")
    actor = require_non_empty(actor, "actor")
    reason = require_non_empty(reason, "reason")

    repository = MeasurementDataRepository(Path(measurement_data_db), Path(migrations_root))
    repository.initialize()
    before = repository.get_dataset(dataset_id)
    if before is None:
        raise ValueError("dataset does not exist")

    new_status = DATASET_RETENTION_ACTIONS[action]
    event_id = repository.record_retention_event(
        dataset_id=dataset_id,
        new_status=new_status,
        actor=actor,
        reason=reason,
        audit_event_reference=audit_event_reference,
    )
    after = repository.get_dataset(dataset_id)

    if bootstrap_output is not None:
        refresh_bootstrap(
            output=bootstrap_output,
            migrations_root=migrations_root,
            projects_db=projects_db,
            metrology_db=metrology_db,
            test_definitions_db=test_definitions_db,
            measurement_data_db=measurement_data_db,
            update_catalog_db=update_catalog_db,
        )

    return {
        "dataset_id": dataset_id,
        "action": action,
        "previous_status": str(before["retention_status"]),
        "new_status": str(after["retention_status"]),
        "event_id": event_id,
    }


def next_project_stage(current_stage: str) -> str:
    current_stage = require_non_empty(current_stage, "current_stage")
    if current_stage not in PROJECT_STAGE_FLOW:
        raise ValueError(f"unknown project stage: {current_stage}")
    index = PROJECT_STAGE_FLOW.index(current_stage)
    if index == len(PROJECT_STAGE_FLOW) - 1:
        return current_stage
    return PROJECT_STAGE_FLOW[index + 1]


def refresh_bootstrap(
    *,
    output: Path | str,
    migrations_root: Path | str = Path("storage/sqlite"),
    projects_db: Path | str | None = None,
    metrology_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
) -> None:
    """Regenerate the browser-loadable GUI bootstrap from local repositories."""

    migrations_root = Path(migrations_root)
    payload = build_bootstrap(
        projects=_open_repository(ProjectRepository, projects_db, migrations_root),
        metrology=_open_repository(MetrologyRepository, metrology_db, migrations_root),
        test_definitions=_open_repository(
            TestDefinitionRepository,
            test_definitions_db,
            migrations_root,
        ),
        measurement_data=_open_repository(
            MeasurementDataRepository,
            measurement_data_db,
            migrations_root,
        ),
        update_catalog=_open_repository(
            UpdateCatalogRepository,
            update_catalog_db,
            migrations_root,
        ),
    )
    write_bootstrap_js(output, payload)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)

    refresh_parser = subcommands.add_parser("refresh-bootstrap")
    _add_repository_args(refresh_parser)
    refresh_parser.add_argument("--output", required=True, type=Path)

    advance_parser = subcommands.add_parser("advance-project")
    _add_repository_args(advance_parser, include_projects=False)
    advance_parser.add_argument("--projects-db", required=True, type=Path)
    advance_parser.add_argument("--code", required=True)
    advance_parser.add_argument("--actor", required=True)
    advance_parser.add_argument("--reason", required=True)
    advance_parser.add_argument("--bootstrap-output", type=Path)

    retention_parser = subcommands.add_parser("dataset-retention")
    _add_repository_args(retention_parser, include_measurement_data=False)
    retention_parser.add_argument("--measurement-data-db", required=True, type=Path)
    retention_parser.add_argument("--dataset-id", required=True, type=int)
    retention_parser.add_argument(
        "--action",
        required=True,
        choices=sorted(DATASET_RETENTION_ACTIONS),
    )
    retention_parser.add_argument("--actor", required=True)
    retention_parser.add_argument("--reason", required=True)
    retention_parser.add_argument("--audit-event-reference")
    retention_parser.add_argument("--bootstrap-output", type=Path)

    args = parser.parse_args(argv)
    if args.command == "refresh-bootstrap":
        refresh_bootstrap(
            output=args.output,
            migrations_root=args.migrations_root,
            projects_db=args.projects_db,
            metrology_db=args.metrology_db,
            test_definitions_db=args.test_definitions_db,
            measurement_data_db=args.measurement_data_db,
            update_catalog_db=args.update_catalog_db,
        )
        return 0

    if args.command == "dataset-retention":
        result = record_dataset_retention_action(
            measurement_data_db=args.measurement_data_db,
            dataset_id=args.dataset_id,
            action=args.action,
            actor=args.actor,
            reason=args.reason,
            audit_event_reference=args.audit_event_reference,
            migrations_root=args.migrations_root,
            bootstrap_output=args.bootstrap_output,
            projects_db=args.projects_db,
            metrology_db=args.metrology_db,
            test_definitions_db=args.test_definitions_db,
            update_catalog_db=args.update_catalog_db,
        )
        print(json.dumps(result, sort_keys=True))
        return 0

    result = advance_project_stage(
        projects_db=args.projects_db,
        code=args.code,
        actor=args.actor,
        reason=args.reason,
        migrations_root=args.migrations_root,
        bootstrap_output=args.bootstrap_output,
        metrology_db=args.metrology_db,
        test_definitions_db=args.test_definitions_db,
        measurement_data_db=args.measurement_data_db,
        update_catalog_db=args.update_catalog_db,
    )
    print(json.dumps(result, sort_keys=True))
    return 0


def _add_repository_args(
    parser: argparse.ArgumentParser,
    *,
    include_projects: bool = True,
    include_measurement_data: bool = True,
) -> None:
    parser.add_argument("--migrations-root", type=Path, default=Path("storage/sqlite"))
    if include_projects:
        parser.add_argument("--projects-db", type=Path)
    parser.add_argument("--metrology-db", type=Path)
    parser.add_argument("--test-definitions-db", type=Path)
    if include_measurement_data:
        parser.add_argument("--measurement-data-db", type=Path)
    parser.add_argument("--update-catalog-db", type=Path)


def _open_repository(
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
    repository = repository_type(Path(database_path), migrations_root)
    repository.initialize()
    return repository


if __name__ == "__main__":
    raise SystemExit(main())
