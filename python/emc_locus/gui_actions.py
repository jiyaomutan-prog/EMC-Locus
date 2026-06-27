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


def register_metrology_instrument(
    *,
    metrology_db: Path | str,
    asset_id: str,
    family: str,
    manufacturer: str,
    model: str,
    serial_number: str,
    category_code: str,
    calibration_requirement: str | None = None,
    availability: str = "available",
    capabilities_json: str = "[]",
    certificate_reference: str | None = None,
    calibrated_at: str | None = None,
    due_at: str | None = None,
    provider: str | None = None,
    status_at_import: str = "valid",
    uncertainty_json: str = "{}",
    file_reference: str | None = None,
    checksum: str | None = None,
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    projects_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
) -> dict[str, Any]:
    """Register one metrology instrument and optional initial certificate."""

    asset_id = require_non_empty(asset_id, "asset_id")
    family = require_non_empty(family, "family")
    manufacturer = require_non_empty(manufacturer, "manufacturer")
    model = require_non_empty(model, "model")
    serial_number = require_non_empty(serial_number, "serial_number")
    category_code = require_non_empty(category_code, "category_code")
    availability = require_non_empty(availability, "availability")
    capabilities_json = _normalized_json(capabilities_json, "capabilities_json")
    uncertainty_json = _normalized_json(uncertainty_json, "uncertainty_json")

    repository = MetrologyRepository(Path(metrology_db), Path(migrations_root))
    repository.initialize()
    category = repository.get_instrument_category(category_code)
    if category is None:
        raise ValueError(f"unknown instrument category: {category_code}")

    if calibration_requirement is None:
        calibration_requirement = str(category["default_calibration_requirement"])
    calibration_requirement = require_non_empty(
        calibration_requirement,
        "calibration_requirement",
    )

    has_certificate = any(
        _has_text(value) for value in (certificate_reference, calibrated_at, due_at, provider)
    )
    if has_certificate:
        certificate_reference = require_non_empty(
            str(certificate_reference or ""),
            "certificate_reference",
        )
        calibrated_at = require_non_empty(str(calibrated_at or ""), "calibrated_at")
        due_at = require_non_empty(str(due_at or ""), "due_at")
        provider = require_non_empty(str(provider or ""), "provider")
    else:
        certificate_reference = None
        calibrated_at = None
        due_at = None
        provider = None

    repository.register_instrument(
        asset_id=asset_id,
        family=family,
        manufacturer=manufacturer,
        model=model,
        serial_number=serial_number,
        calibration_requirement=calibration_requirement,
        availability=availability,
        capabilities_json=capabilities_json,
        category_code=category_code,
        certificate_reference=certificate_reference,
        calibrated_at=calibrated_at,
        due_at=due_at,
        provider=provider,
        status_at_import=status_at_import,
        uncertainty_json=uncertainty_json,
        file_reference=file_reference,
        checksum=checksum,
    )
    latest_calibration = repository.latest_calibration_record(asset_id)

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
        "asset_id": asset_id,
        "category_code": category_code,
        "category_label": str(category["label"]),
        "calibration_requirement": calibration_requirement,
        "calibration_recorded": latest_calibration is not None,
        "certificate_reference": (
            str(latest_calibration["certificate_reference"])
            if latest_calibration is not None
            else None
        ),
    }


def record_metrology_calibration(
    *,
    metrology_db: Path | str,
    asset_id: str,
    certificate_reference: str,
    calibrated_at: str,
    due_at: str,
    provider: str,
    status_at_import: str = "valid",
    uncertainty_json: str = "{}",
    file_reference: str | None = None,
    checksum: str | None = None,
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    projects_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
) -> dict[str, Any]:
    """Record a calibration certificate for an existing metrology asset."""

    asset_id = require_non_empty(asset_id, "asset_id")
    certificate_reference = require_non_empty(
        certificate_reference,
        "certificate_reference",
    )
    calibrated_at = require_non_empty(calibrated_at, "calibrated_at")
    due_at = require_non_empty(due_at, "due_at")
    provider = require_non_empty(provider, "provider")
    status_at_import = require_non_empty(status_at_import, "status_at_import")
    uncertainty_json = _normalized_json(uncertainty_json, "uncertainty_json")

    repository = MetrologyRepository(Path(metrology_db), Path(migrations_root))
    repository.initialize()
    instrument = repository.get_instrument(asset_id)
    if instrument is None:
        raise ValueError("instrument does not exist")

    repository.add_calibration_record(
        asset_id=asset_id,
        certificate_reference=certificate_reference,
        calibrated_at=calibrated_at,
        due_at=due_at,
        provider=provider,
        status_at_import=status_at_import,
        uncertainty_json=uncertainty_json,
        file_reference=file_reference,
        checksum=checksum,
    )

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
        "asset_id": asset_id,
        "certificate_reference": certificate_reference,
        "due_at": due_at,
        "status_at_import": status_at_import,
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


def record_update_validation_action(
    *,
    update_catalog_db: Path | str,
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
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    projects_db: Path | str | None = None,
    metrology_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
) -> dict[str, Any]:
    """Record update validation evidence and optionally refresh GUI data."""

    repository = UpdateCatalogRepository(Path(update_catalog_db), Path(migrations_root))
    repository.initialize()
    evidence_id = repository.record_install_validation(
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
    evidence = repository.get_install_validation_evidence(evidence_id)

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
        "validation_evidence_id": evidence_id,
        "validation_status": str(evidence["validation_status"]),
        "reason": evidence["reason"],
    }


def record_update_install_action(
    *,
    update_catalog_db: Path | str,
    package_name: str,
    package_version: str,
    component: str,
    installed_by: str,
    source: str,
    rollback_reference: str | None = None,
    validation_evidence_id: int | None = None,
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    projects_db: Path | str | None = None,
    metrology_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
) -> dict[str, Any]:
    """Record an update install action and optionally refresh GUI data."""

    repository = UpdateCatalogRepository(Path(update_catalog_db), Path(migrations_root))
    repository.initialize()
    install_id = repository.record_install(
        package_name=package_name,
        package_version=package_version,
        component=component,
        installed_by=installed_by,
        source=source,
        rollback_reference=rollback_reference,
        validation_evidence_id=validation_evidence_id,
    )

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
        "install_id": install_id,
        "package_name": package_name,
        "package_version": package_version,
        "component": component,
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

    register_parser = subcommands.add_parser("register-instrument")
    _add_repository_args(register_parser, include_metrology=False)
    register_parser.add_argument("--metrology-db", required=True, type=Path)
    register_parser.add_argument("--asset-id", required=True)
    register_parser.add_argument("--family", required=True)
    register_parser.add_argument("--manufacturer", required=True)
    register_parser.add_argument("--model", required=True)
    register_parser.add_argument("--serial-number", required=True)
    register_parser.add_argument("--category-code", required=True)
    register_parser.add_argument("--calibration-requirement")
    register_parser.add_argument("--availability", default="available")
    register_parser.add_argument("--capabilities-json", default="[]")
    register_parser.add_argument("--certificate-reference")
    register_parser.add_argument("--calibrated-at")
    register_parser.add_argument("--due-at")
    register_parser.add_argument("--provider")
    register_parser.add_argument("--status-at-import", default="valid")
    register_parser.add_argument("--uncertainty-json", default="{}")
    register_parser.add_argument("--file-reference")
    register_parser.add_argument("--checksum")
    register_parser.add_argument("--bootstrap-output", type=Path)

    calibration_parser = subcommands.add_parser("record-calibration")
    _add_repository_args(calibration_parser, include_metrology=False)
    calibration_parser.add_argument("--metrology-db", required=True, type=Path)
    calibration_parser.add_argument("--asset-id", required=True)
    calibration_parser.add_argument("--certificate-reference", required=True)
    calibration_parser.add_argument("--calibrated-at", required=True)
    calibration_parser.add_argument("--due-at", required=True)
    calibration_parser.add_argument("--provider", required=True)
    calibration_parser.add_argument("--status-at-import", default="valid")
    calibration_parser.add_argument("--uncertainty-json", default="{}")
    calibration_parser.add_argument("--file-reference")
    calibration_parser.add_argument("--checksum")
    calibration_parser.add_argument("--bootstrap-output", type=Path)

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

    validation_parser = subcommands.add_parser("validate-update")
    _add_repository_args(validation_parser, include_update_catalog=False)
    validation_parser.add_argument("--update-catalog-db", required=True, type=Path)
    validation_parser.add_argument("--package-name", required=True)
    validation_parser.add_argument("--package-version", required=True)
    validation_parser.add_argument("--component", required=True)
    validation_parser.add_argument("--installed-version", required=True)
    validation_parser.add_argument("--source", required=True)
    validation_parser.add_argument("--compatibility-minimum-version", required=True)
    validation_parser.add_argument("--compatibility-maximum-version")
    validation_parser.add_argument("--validated-by", required=True)
    validation_parser.add_argument("--no-signature-required", action="store_true")
    validation_parser.add_argument("--block-offline-install", action="store_true")
    validation_parser.add_argument("--measurement-active", action="store_true")
    validation_parser.add_argument("--allow-apply-during-measurement", action="store_true")
    validation_parser.add_argument("--bootstrap-output", type=Path)

    install_parser = subcommands.add_parser("install-update")
    _add_repository_args(install_parser, include_update_catalog=False)
    install_parser.add_argument("--update-catalog-db", required=True, type=Path)
    install_parser.add_argument("--package-name", required=True)
    install_parser.add_argument("--package-version", required=True)
    install_parser.add_argument("--component", required=True)
    install_parser.add_argument("--installed-by", required=True)
    install_parser.add_argument("--source", required=True)
    install_parser.add_argument("--rollback-reference")
    install_parser.add_argument("--validation-evidence-id", type=int)
    install_parser.add_argument("--bootstrap-output", type=Path)

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

    if args.command == "register-instrument":
        result = register_metrology_instrument(
            metrology_db=args.metrology_db,
            asset_id=args.asset_id,
            family=args.family,
            manufacturer=args.manufacturer,
            model=args.model,
            serial_number=args.serial_number,
            category_code=args.category_code,
            calibration_requirement=args.calibration_requirement,
            availability=args.availability,
            capabilities_json=args.capabilities_json,
            certificate_reference=args.certificate_reference,
            calibrated_at=args.calibrated_at,
            due_at=args.due_at,
            provider=args.provider,
            status_at_import=args.status_at_import,
            uncertainty_json=args.uncertainty_json,
            file_reference=args.file_reference,
            checksum=args.checksum,
            migrations_root=args.migrations_root,
            bootstrap_output=args.bootstrap_output,
            projects_db=args.projects_db,
            test_definitions_db=args.test_definitions_db,
            measurement_data_db=args.measurement_data_db,
            update_catalog_db=args.update_catalog_db,
        )
        print(json.dumps(result, sort_keys=True))
        return 0

    if args.command == "record-calibration":
        result = record_metrology_calibration(
            metrology_db=args.metrology_db,
            asset_id=args.asset_id,
            certificate_reference=args.certificate_reference,
            calibrated_at=args.calibrated_at,
            due_at=args.due_at,
            provider=args.provider,
            status_at_import=args.status_at_import,
            uncertainty_json=args.uncertainty_json,
            file_reference=args.file_reference,
            checksum=args.checksum,
            migrations_root=args.migrations_root,
            bootstrap_output=args.bootstrap_output,
            projects_db=args.projects_db,
            test_definitions_db=args.test_definitions_db,
            measurement_data_db=args.measurement_data_db,
            update_catalog_db=args.update_catalog_db,
        )
        print(json.dumps(result, sort_keys=True))
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

    if args.command == "validate-update":
        result = record_update_validation_action(
            update_catalog_db=args.update_catalog_db,
            package_name=args.package_name,
            package_version=args.package_version,
            component=args.component,
            installed_version=args.installed_version,
            source=args.source,
            compatibility_minimum_version=args.compatibility_minimum_version,
            compatibility_maximum_version=args.compatibility_maximum_version,
            validated_by=args.validated_by,
            signature_required=not args.no_signature_required,
            policy_offline_install_allowed=not args.block_offline_install,
            measurement_active=args.measurement_active,
            apply_during_measurement_allowed=args.allow_apply_during_measurement,
            migrations_root=args.migrations_root,
            bootstrap_output=args.bootstrap_output,
            projects_db=args.projects_db,
            metrology_db=args.metrology_db,
            test_definitions_db=args.test_definitions_db,
            measurement_data_db=args.measurement_data_db,
        )
        print(json.dumps(result, sort_keys=True))
        return 0

    if args.command == "install-update":
        result = record_update_install_action(
            update_catalog_db=args.update_catalog_db,
            package_name=args.package_name,
            package_version=args.package_version,
            component=args.component,
            installed_by=args.installed_by,
            source=args.source,
            rollback_reference=args.rollback_reference,
            validation_evidence_id=args.validation_evidence_id,
            migrations_root=args.migrations_root,
            bootstrap_output=args.bootstrap_output,
            projects_db=args.projects_db,
            metrology_db=args.metrology_db,
            test_definitions_db=args.test_definitions_db,
            measurement_data_db=args.measurement_data_db,
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
    include_metrology: bool = True,
    include_measurement_data: bool = True,
    include_update_catalog: bool = True,
) -> None:
    parser.add_argument("--migrations-root", type=Path, default=Path("storage/sqlite"))
    if include_projects:
        parser.add_argument("--projects-db", type=Path)
    if include_metrology:
        parser.add_argument("--metrology-db", type=Path)
    parser.add_argument("--test-definitions-db", type=Path)
    if include_measurement_data:
        parser.add_argument("--measurement-data-db", type=Path)
    if include_update_catalog:
        parser.add_argument("--update-catalog-db", type=Path)


def _normalized_json(value: str, field_name: str) -> str:
    text = require_non_empty(value, field_name)
    try:
        parsed = json.loads(text)
    except json.JSONDecodeError as error:
        raise ValueError(f"{field_name} must contain valid JSON") from error
    return json.dumps(parsed, sort_keys=True)


def _has_text(value: str | None) -> bool:
    return value is not None and bool(value.strip())


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
