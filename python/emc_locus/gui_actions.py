"""Local write actions used by the static GUI workflow."""

from __future__ import annotations

import argparse
import calendar
from datetime import date, datetime, timedelta
import json
from pathlib import Path
from typing import Any

from .gui_bootstrap import build_bootstrap, write_bootstrap_js
from .local_agent_client import LocalAgentClient
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
PROJECT_EXECUTION_MODES = {"accredited", "non_accredited", "investigation"}
CONTRACT_REVIEW_REQUIRED_ITEMS = {
    "accredited": (
        "requirements_reviewed",
        "method_available",
        "resources_available",
        "impartiality_risk_reviewed",
    ),
    "non_accredited": (
        "scope_confirmed",
        "constraints_accepted",
    ),
    "investigation": (
        "investigation_goal_defined",
    ),
}

DATASET_RETENTION_ACTIONS = {
    "request-deletion": "deletion_requested",
    "approve-deletion": "deletion_approved",
    "reject-deletion": "deletion_rejected",
    "mark-deleted": "deleted",
}
INSTRUMENT_AVAILABILITY_STATUSES = {"available", "reserved", "out_of_service"}
INSTRUMENT_DOCUMENT_KINDS = {
    "certificate",
    "datasheet",
    "transducer_calculation",
    "script",
    "manual",
    "photo",
    "other",
}
SERVICE_SCHEDULE_STATUSES = {
    "planned",
    "confirmed",
    "in_progress",
    "completed",
    "cancelled",
}


def _parse_schedule_datetime(value: str, field_name: str) -> datetime:
    if "T" not in value:
        raise ValueError(f"{field_name} must be an ISO 8601 local date-time")
    try:
        parsed = datetime.fromisoformat(value)
    except ValueError as exc:
        raise ValueError(f"{field_name} must be an ISO 8601 local date-time") from exc
    if parsed.tzinfo is not None:
        raise ValueError(f"{field_name} must be a local date-time without timezone")
    return parsed


def _validate_service_schedule_block(
    planned_start_at: str,
    planned_end_at: str,
) -> None:
    start = _parse_schedule_datetime(planned_start_at, "planned_start_at")
    end = _parse_schedule_datetime(planned_end_at, "planned_end_at")
    if end <= start:
        raise ValueError("planned_end_at must be after planned_start_at")

    day = start.date()
    while day <= end.date():
        if day.weekday() >= 5:
            raise ValueError("service schedule items must stay within business days")
        day += timedelta(days=1)


def create_project_record(
    *,
    projects_db: Path | str | None,
    code: str,
    customer_name: str,
    execution_mode: str,
    actor: str,
    reason: str,
    stage: str = "quotation",
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    metrology_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
    agent_url: str | None = None,
    operation_id: str | None = None,
    correlation_id: str | None = None,
    device_id: str | None = None,
) -> dict[str, Any]:
    """Create a project with first audit evidence."""

    code = require_non_empty(code, "code")
    customer_name = require_non_empty(customer_name, "customer_name")
    execution_mode = require_non_empty(execution_mode, "execution_mode")
    if execution_mode not in PROJECT_EXECUTION_MODES:
        raise ValueError(f"unknown project execution mode: {execution_mode}")
    stage = require_non_empty(stage, "stage")
    if stage not in PROJECT_STAGE_FLOW:
        raise ValueError(f"unknown project stage: {stage}")
    actor = require_non_empty(actor, "actor")
    reason = require_non_empty(reason, "reason")

    if _has_text(agent_url):
        response = LocalAgentClient(str(agent_url)).create_project(
            code=code,
            customer_name=customer_name,
            execution_mode=execution_mode,
            stage=stage,
            actor=actor,
            reason=reason,
            operation_id=operation_id,
            correlation_id=correlation_id,
            device_id=device_id,
        )
        project = response["project"]
        if bootstrap_output is not None:
            refresh_bootstrap(
                output=bootstrap_output,
                migrations_root=migrations_root,
                projects_db=projects_db,
                metrology_db=metrology_db,
                test_definitions_db=test_definitions_db,
                measurement_data_db=measurement_data_db,
                update_catalog_db=update_catalog_db,
                agent_url=agent_url,
            )
        return {
            "code": project["code"],
            "customer_name": project["customer_name"],
            "execution_mode": project["execution_mode"],
            "stage": project["stage"],
            "operation_id": response["operation_id"],
            "replayed": response["replayed"],
        }

    if projects_db is None:
        raise ValueError("projects_db is required when no agent_url is configured")
    repository = ProjectRepository(Path(projects_db), Path(migrations_root))
    repository.initialize()
    if repository.get_project(code) is not None:
        raise ValueError("project already exists")
    audit_event_id = repository.create_project_with_audit(
        code=code,
        customer_name=customer_name,
        execution_mode=execution_mode,
        stage=stage,
        actor=actor,
        reason=reason,
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
        "code": code,
        "customer_name": customer_name,
        "execution_mode": execution_mode,
        "stage": stage,
        "audit_event_id": audit_event_id,
    }


def complete_contract_review_item_action(
    *,
    projects_db: Path | str | None,
    project_code: str,
    item: str,
    completed_by: str,
    comment: str | None = None,
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    metrology_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
    agent_url: str | None = None,
    operation_id: str | None = None,
    correlation_id: str | None = None,
    device_id: str | None = None,
) -> dict[str, Any]:
    """Complete one contract-review checklist item with audit evidence."""

    project_code = require_non_empty(project_code, "project_code")
    item = require_non_empty(item, "item")
    completed_by = require_non_empty(completed_by, "completed_by")
    comment = comment.strip() if comment is not None else None

    if _has_text(agent_url):
        response = LocalAgentClient(str(agent_url)).complete_contract_review_item(
            project_code=project_code,
            item=item,
            actor=completed_by,
            comment=comment,
            operation_id=operation_id,
            correlation_id=correlation_id,
            device_id=device_id,
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
                agent_url=agent_url,
            )
        return {
            "project_code": project_code,
            "item": item,
            "completed_by": completed_by,
            "operation_id": response["operation_id"],
            "replayed": response["replayed"],
            "already_completed": response["already_completed"],
        }

    if projects_db is None:
        raise ValueError("projects_db is required when no agent_url is configured")
    repository = ProjectRepository(Path(projects_db), Path(migrations_root))
    repository.initialize()
    sequence = repository.complete_contract_review_item_with_audit(
        project_code=project_code,
        item=item,
        completed_by=completed_by,
        comment=comment,
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
        "project_code": project_code,
        "item": item,
        "completed_by": completed_by,
        "audit_sequence": sequence,
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
    part_number: str | None = None,
    calibration_period_months: int | None = None,
    metrology_notes: str = "",
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
    calibration_period_months = _positive_optional_int(
        calibration_period_months,
        "calibration_period_months",
    )

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
        due_at = _due_at_from_period(
            calibrated_at=calibrated_at,
            due_at=due_at,
            calibration_period_months=calibration_period_months,
        )
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
        part_number=part_number,
        calibration_period_months=calibration_period_months,
        metrology_notes=metrology_notes,
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
        "part_number": part_number,
        "calibration_period_months": calibration_period_months,
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
    due_at: str | None = None,
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
    provider = require_non_empty(provider, "provider")
    status_at_import = require_non_empty(status_at_import, "status_at_import")
    uncertainty_json = _normalized_json(uncertainty_json, "uncertainty_json")

    repository = MetrologyRepository(Path(metrology_db), Path(migrations_root))
    repository.initialize()
    instrument = repository.get_instrument(asset_id)
    if instrument is None:
        raise ValueError("instrument does not exist")
    period = instrument.get("calibration_period_months")
    calibration_period_months = int(period) if period is not None else None
    due_at = _due_at_from_period(
        calibrated_at=calibrated_at,
        due_at=due_at,
        calibration_period_months=calibration_period_months,
    )

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


def attach_metrology_document(
    *,
    metrology_db: Path | str,
    asset_id: str,
    document_kind: str,
    title: str,
    file_reference: str,
    uploaded_by: str,
    checksum: str | None = None,
    revision: str | None = None,
    applies_to_function: str | None = None,
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    projects_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
) -> dict[str, Any]:
    """Attach one certificate, datasheet, script, or other document to an asset."""

    asset_id = require_non_empty(asset_id, "asset_id")
    document_kind = require_non_empty(document_kind, "document_kind")
    if document_kind not in INSTRUMENT_DOCUMENT_KINDS:
        raise ValueError(f"unknown instrument document kind: {document_kind}")
    title = require_non_empty(title, "title")
    file_reference = require_non_empty(file_reference, "file_reference")
    uploaded_by = require_non_empty(uploaded_by, "uploaded_by")

    repository = MetrologyRepository(Path(metrology_db), Path(migrations_root))
    repository.initialize()
    if repository.get_instrument(asset_id) is None:
        raise ValueError("instrument does not exist")

    document_id = repository.add_instrument_document(
        asset_id=asset_id,
        document_kind=document_kind,
        title=title,
        file_reference=file_reference,
        uploaded_by=uploaded_by,
        checksum=checksum,
        revision=revision,
        applies_to_function=applies_to_function,
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
        "document_id": document_id,
        "asset_id": asset_id,
        "document_kind": document_kind,
        "title": title,
    }


def schedule_service_item(
    *,
    projects_db: Path | str,
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
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    metrology_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
) -> dict[str, Any]:
    """Plan one service/test execution item for the laboratory schedule."""

    item_code = require_non_empty(item_code, "item_code")
    project_code = require_non_empty(project_code, "project_code")
    title = require_non_empty(title, "title")
    planned_start_at = require_non_empty(planned_start_at, "planned_start_at")
    planned_end_at = require_non_empty(planned_end_at, "planned_end_at")
    _validate_service_schedule_block(planned_start_at, planned_end_at)
    assigned_operator = require_non_empty(assigned_operator, "assigned_operator")
    location = require_non_empty(location, "location")
    equipment_under_test = require_non_empty(equipment_under_test, "equipment_under_test")
    status = require_non_empty(status, "status")
    if status not in SERVICE_SCHEDULE_STATUSES:
        raise ValueError(f"unknown service schedule status: {status}")

    repository = ProjectRepository(Path(projects_db), Path(migrations_root))
    repository.initialize()
    if repository.get_project(project_code) is None:
        raise ValueError("project does not exist")
    schedule_id = repository.add_service_schedule_item(
        item_code=item_code,
        project_code=project_code,
        title=title,
        test_category_code=test_category_code,
        test_method_code=test_method_code,
        planned_start_at=planned_start_at,
        planned_end_at=planned_end_at,
        assigned_operator=assigned_operator,
        location=location,
        equipment_under_test=equipment_under_test,
        status=status,
        notes=notes,
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
        "schedule_id": schedule_id,
        "item_code": item_code,
        "project_code": project_code,
        "status": status,
    }


def create_test_category(
    *,
    test_definitions_db: Path | str,
    code: str,
    label: str,
    description: str,
    parent_code: str | None = None,
    active: bool = True,
    sort_order: int = 0,
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    projects_db: Path | str | None = None,
    metrology_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
) -> dict[str, Any]:
    """Create one adjustable test category for the method taxonomy."""

    code = require_non_empty(code, "code")
    label = require_non_empty(label, "label")
    description = require_non_empty(description, "description")
    repository = TestDefinitionRepository(Path(test_definitions_db), Path(migrations_root))
    repository.initialize()
    if parent_code is not None and repository.get_test_category(parent_code) is None:
        raise ValueError("parent test category does not exist")
    repository.add_test_category(
        code=code,
        parent_code=parent_code,
        label=label,
        description=description,
        active=active,
        sort_order=sort_order,
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
        "code": code,
        "parent_code": parent_code,
        "label": label,
        "active": active,
    }


def set_metrology_instrument_availability(
    *,
    metrology_db: Path | str,
    asset_id: str,
    availability: str,
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    projects_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
) -> dict[str, Any]:
    """Set the operational availability of an existing metrology asset."""

    asset_id = require_non_empty(asset_id, "asset_id")
    availability = require_non_empty(availability, "availability")
    if availability not in INSTRUMENT_AVAILABILITY_STATUSES:
        raise ValueError(f"unknown instrument availability: {availability}")

    repository = MetrologyRepository(Path(metrology_db), Path(migrations_root))
    repository.initialize()
    before = repository.get_instrument(asset_id)
    if before is None:
        raise ValueError("instrument does not exist")

    updated = repository.update_instrument_availability(
        asset_id=asset_id,
        availability=availability,
    )
    if not updated:
        raise ValueError("instrument does not exist")

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
        "previous_availability": str(before["availability"]),
        "new_availability": availability,
    }


def set_metrology_instrument_capabilities(
    *,
    metrology_db: Path | str,
    asset_id: str,
    capabilities_json: str,
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    projects_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
) -> dict[str, Any]:
    """Replace the capability declaration of an existing metrology asset."""

    asset_id = require_non_empty(asset_id, "asset_id")
    capabilities_json = _normalized_json(capabilities_json, "capabilities_json")

    repository = MetrologyRepository(Path(metrology_db), Path(migrations_root))
    repository.initialize()
    before = repository.get_instrument(asset_id)
    if before is None:
        raise ValueError("instrument does not exist")

    updated = repository.update_instrument_capabilities(
        asset_id=asset_id,
        capabilities_json=capabilities_json,
    )
    if not updated:
        raise ValueError("instrument does not exist")

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
        "previous_capabilities_json": str(before["capabilities_json"]),
        "new_capabilities_json": capabilities_json,
    }


def advance_project_stage(
    *,
    projects_db: Path | str | None,
    code: str,
    actor: str,
    reason: str,
    migrations_root: Path | str = Path("storage/sqlite"),
    bootstrap_output: Path | str | None = None,
    metrology_db: Path | str | None = None,
    test_definitions_db: Path | str | None = None,
    measurement_data_db: Path | str | None = None,
    update_catalog_db: Path | str | None = None,
    agent_url: str | None = None,
    operation_id: str | None = None,
    correlation_id: str | None = None,
    device_id: str | None = None,
) -> dict[str, Any]:
    """Advance one project to the next workflow stage with audit evidence."""

    code = require_non_empty(code, "code")
    actor = require_non_empty(actor, "actor")
    reason = require_non_empty(reason, "reason")

    if _has_text(agent_url):
        response = LocalAgentClient(str(agent_url)).advance_to_test_planning(
            project_code=code,
            actor=actor,
            reason=reason,
            operation_id=operation_id,
            correlation_id=correlation_id,
            device_id=device_id,
        )
        project = response["project"]
        if bootstrap_output is not None:
            refresh_bootstrap(
                output=bootstrap_output,
                migrations_root=migrations_root,
                projects_db=projects_db,
                metrology_db=metrology_db,
                test_definitions_db=test_definitions_db,
                measurement_data_db=measurement_data_db,
                update_catalog_db=update_catalog_db,
                agent_url=agent_url,
            )
        return {
            "project_code": code,
            "new_stage": project["stage"],
            "operation_id": response["operation_id"],
            "replayed": response["replayed"],
        }

    if projects_db is None:
        raise ValueError("projects_db is required when no agent_url is configured")
    projects = ProjectRepository(Path(projects_db), Path(migrations_root))
    projects.initialize()

    project = projects.get_project(code)
    if project is None:
        raise ValueError("project does not exist")

    current_stage = str(project["stage"])
    next_stage = next_project_stage(current_stage)
    missing_review_items = _missing_contract_review_items(projects, project)
    if (
        current_stage == "contract_review"
        and next_stage == "test_planning"
        and missing_review_items
    ):
        raise ValueError(
            "contract review incomplete: " + ", ".join(missing_review_items)
        )
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
    agent_url: str | None = None,
) -> None:
    """Regenerate the browser-loadable GUI bootstrap from local repositories."""

    migrations_root = Path(migrations_root)
    project_agent = LocalAgentClient(str(agent_url)) if _has_text(agent_url) else None
    payload = build_bootstrap(
        project_agent=project_agent,
        projects=None
        if project_agent is not None
        else _open_repository(ProjectRepository, projects_db, migrations_root),
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
    refresh_parser.add_argument("--agent-url")

    project_parser = subcommands.add_parser("create-project")
    _add_repository_args(project_parser, include_projects=False)
    project_parser.add_argument("--projects-db", type=Path)
    project_parser.add_argument("--code", required=True)
    project_parser.add_argument("--customer-name", required=True)
    project_parser.add_argument(
        "--execution-mode",
        required=True,
        choices=sorted(PROJECT_EXECUTION_MODES),
    )
    project_parser.add_argument(
        "--stage",
        default="quotation",
        choices=PROJECT_STAGE_FLOW,
    )
    project_parser.add_argument("--actor", required=True)
    project_parser.add_argument("--reason", required=True)
    project_parser.add_argument("--bootstrap-output", type=Path)
    _add_agent_args(project_parser)

    contract_parser = subcommands.add_parser("complete-contract-review-item")
    _add_repository_args(contract_parser, include_projects=False)
    contract_parser.add_argument("--projects-db", type=Path)
    contract_parser.add_argument("--project-code", required=True)
    contract_parser.add_argument("--item", required=True)
    contract_parser.add_argument("--completed-by", required=True)
    contract_parser.add_argument("--comment")
    contract_parser.add_argument("--bootstrap-output", type=Path)
    _add_agent_args(contract_parser)

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
    register_parser.add_argument("--part-number")
    register_parser.add_argument("--calibration-period-months", type=int)
    register_parser.add_argument("--metrology-notes", default="")
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
    calibration_parser.add_argument("--due-at")
    calibration_parser.add_argument("--provider", required=True)
    calibration_parser.add_argument("--status-at-import", default="valid")
    calibration_parser.add_argument("--uncertainty-json", default="{}")
    calibration_parser.add_argument("--file-reference")
    calibration_parser.add_argument("--checksum")
    calibration_parser.add_argument("--bootstrap-output", type=Path)

    document_parser = subcommands.add_parser("attach-instrument-document")
    _add_repository_args(document_parser, include_metrology=False)
    document_parser.add_argument("--metrology-db", required=True, type=Path)
    document_parser.add_argument("--asset-id", required=True)
    document_parser.add_argument(
        "--document-kind",
        required=True,
        choices=sorted(INSTRUMENT_DOCUMENT_KINDS),
    )
    document_parser.add_argument("--title", required=True)
    document_parser.add_argument("--file-reference", required=True)
    document_parser.add_argument("--uploaded-by", required=True)
    document_parser.add_argument("--checksum")
    document_parser.add_argument("--revision")
    document_parser.add_argument("--applies-to-function")
    document_parser.add_argument("--bootstrap-output", type=Path)

    availability_parser = subcommands.add_parser("set-instrument-availability")
    _add_repository_args(availability_parser, include_metrology=False)
    availability_parser.add_argument("--metrology-db", required=True, type=Path)
    availability_parser.add_argument("--asset-id", required=True)
    availability_parser.add_argument(
        "--availability",
        required=True,
        choices=sorted(INSTRUMENT_AVAILABILITY_STATUSES),
    )
    availability_parser.add_argument("--bootstrap-output", type=Path)

    capabilities_parser = subcommands.add_parser("set-instrument-capabilities")
    _add_repository_args(capabilities_parser, include_metrology=False)
    capabilities_parser.add_argument("--metrology-db", required=True, type=Path)
    capabilities_parser.add_argument("--asset-id", required=True)
    capabilities_parser.add_argument("--capabilities-json", required=True)
    capabilities_parser.add_argument("--bootstrap-output", type=Path)

    schedule_parser = subcommands.add_parser("schedule-service-item")
    _add_repository_args(schedule_parser, include_projects=False)
    schedule_parser.add_argument("--projects-db", required=True, type=Path)
    schedule_parser.add_argument("--item-code", required=True)
    schedule_parser.add_argument("--project-code", required=True)
    schedule_parser.add_argument("--title", required=True)
    schedule_parser.add_argument("--planned-start-at", required=True)
    schedule_parser.add_argument("--planned-end-at", required=True)
    schedule_parser.add_argument("--assigned-operator", required=True)
    schedule_parser.add_argument("--location", required=True)
    schedule_parser.add_argument("--equipment-under-test", required=True)
    schedule_parser.add_argument("--test-category-code")
    schedule_parser.add_argument("--test-method-code")
    schedule_parser.add_argument(
        "--status",
        default="planned",
        choices=sorted(SERVICE_SCHEDULE_STATUSES),
    )
    schedule_parser.add_argument("--notes", default="")
    schedule_parser.add_argument("--bootstrap-output", type=Path)

    category_parser = subcommands.add_parser("create-test-category")
    _add_repository_args(
        category_parser,
        include_test_definitions=False,
        include_update_catalog=False,
    )
    category_parser.add_argument("--test-definitions-db", required=True, type=Path)
    category_parser.add_argument("--code", required=True)
    category_parser.add_argument("--label", required=True)
    category_parser.add_argument("--description", required=True)
    category_parser.add_argument("--parent-code")
    category_parser.add_argument("--inactive", action="store_true")
    category_parser.add_argument("--sort-order", type=int, default=0)
    category_parser.add_argument("--bootstrap-output", type=Path)

    advance_parser = subcommands.add_parser("advance-project")
    _add_repository_args(advance_parser, include_projects=False)
    advance_parser.add_argument("--projects-db", type=Path)
    advance_parser.add_argument("--code", required=True)
    advance_parser.add_argument("--actor", required=True)
    advance_parser.add_argument("--reason", required=True)
    advance_parser.add_argument("--bootstrap-output", type=Path)
    _add_agent_args(advance_parser)

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
            agent_url=args.agent_url,
        )
        return 0

    if args.command == "create-project":
        result = create_project_record(
            projects_db=args.projects_db,
            code=args.code,
            customer_name=args.customer_name,
            execution_mode=args.execution_mode,
            stage=args.stage,
            actor=args.actor,
            reason=args.reason,
            migrations_root=args.migrations_root,
            bootstrap_output=args.bootstrap_output,
            metrology_db=args.metrology_db,
            test_definitions_db=args.test_definitions_db,
            measurement_data_db=args.measurement_data_db,
            update_catalog_db=args.update_catalog_db,
            agent_url=args.agent_url,
            operation_id=args.operation_id,
            correlation_id=args.correlation_id,
            device_id=args.device_id,
        )
        print(json.dumps(result, sort_keys=True))
        return 0

    if args.command == "complete-contract-review-item":
        result = complete_contract_review_item_action(
            projects_db=args.projects_db,
            project_code=args.project_code,
            item=args.item,
            completed_by=args.completed_by,
            comment=args.comment,
            migrations_root=args.migrations_root,
            bootstrap_output=args.bootstrap_output,
            metrology_db=args.metrology_db,
            test_definitions_db=args.test_definitions_db,
            measurement_data_db=args.measurement_data_db,
            update_catalog_db=args.update_catalog_db,
        )
        print(json.dumps(result, sort_keys=True))
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
            part_number=args.part_number,
            calibration_period_months=args.calibration_period_months,
            metrology_notes=args.metrology_notes,
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

    if args.command == "attach-instrument-document":
        result = attach_metrology_document(
            metrology_db=args.metrology_db,
            asset_id=args.asset_id,
            document_kind=args.document_kind,
            title=args.title,
            file_reference=args.file_reference,
            uploaded_by=args.uploaded_by,
            checksum=args.checksum,
            revision=args.revision,
            applies_to_function=args.applies_to_function,
            migrations_root=args.migrations_root,
            bootstrap_output=args.bootstrap_output,
            projects_db=args.projects_db,
            test_definitions_db=args.test_definitions_db,
            measurement_data_db=args.measurement_data_db,
            update_catalog_db=args.update_catalog_db,
        )
        print(json.dumps(result, sort_keys=True))
        return 0

    if args.command == "set-instrument-availability":
        result = set_metrology_instrument_availability(
            metrology_db=args.metrology_db,
            asset_id=args.asset_id,
            availability=args.availability,
            migrations_root=args.migrations_root,
            bootstrap_output=args.bootstrap_output,
            projects_db=args.projects_db,
            test_definitions_db=args.test_definitions_db,
            measurement_data_db=args.measurement_data_db,
            update_catalog_db=args.update_catalog_db,
        )
        print(json.dumps(result, sort_keys=True))
        return 0

    if args.command == "set-instrument-capabilities":
        result = set_metrology_instrument_capabilities(
            metrology_db=args.metrology_db,
            asset_id=args.asset_id,
            capabilities_json=args.capabilities_json,
            migrations_root=args.migrations_root,
            bootstrap_output=args.bootstrap_output,
            projects_db=args.projects_db,
            test_definitions_db=args.test_definitions_db,
            measurement_data_db=args.measurement_data_db,
            update_catalog_db=args.update_catalog_db,
        )
        print(json.dumps(result, sort_keys=True))
        return 0

    if args.command == "schedule-service-item":
        result = schedule_service_item(
            projects_db=args.projects_db,
            item_code=args.item_code,
            project_code=args.project_code,
            title=args.title,
            planned_start_at=args.planned_start_at,
            planned_end_at=args.planned_end_at,
            assigned_operator=args.assigned_operator,
            location=args.location,
            equipment_under_test=args.equipment_under_test,
            test_category_code=args.test_category_code,
            test_method_code=args.test_method_code,
            status=args.status,
            notes=args.notes,
            migrations_root=args.migrations_root,
            bootstrap_output=args.bootstrap_output,
            metrology_db=args.metrology_db,
            test_definitions_db=args.test_definitions_db,
            measurement_data_db=args.measurement_data_db,
            update_catalog_db=args.update_catalog_db,
        )
        print(json.dumps(result, sort_keys=True))
        return 0

    if args.command == "create-test-category":
        result = create_test_category(
            test_definitions_db=args.test_definitions_db,
            code=args.code,
            label=args.label,
            description=args.description,
            parent_code=args.parent_code,
            active=not args.inactive,
            sort_order=args.sort_order,
            migrations_root=args.migrations_root,
            bootstrap_output=args.bootstrap_output,
            projects_db=args.projects_db,
            metrology_db=args.metrology_db,
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
        agent_url=args.agent_url,
        operation_id=args.operation_id,
        correlation_id=args.correlation_id,
        device_id=args.device_id,
    )
    print(json.dumps(result, sort_keys=True))
    return 0


def _add_repository_args(
    parser: argparse.ArgumentParser,
    *,
    include_projects: bool = True,
    include_metrology: bool = True,
    include_test_definitions: bool = True,
    include_measurement_data: bool = True,
    include_update_catalog: bool = True,
) -> None:
    parser.add_argument("--migrations-root", type=Path, default=Path("storage/sqlite"))
    if include_projects:
        parser.add_argument("--projects-db", type=Path)
    if include_metrology:
        parser.add_argument("--metrology-db", type=Path)
    if include_test_definitions:
        parser.add_argument("--test-definitions-db", type=Path)
    if include_measurement_data:
        parser.add_argument("--measurement-data-db", type=Path)
    if include_update_catalog:
        parser.add_argument("--update-catalog-db", type=Path)


def _add_agent_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--agent-url")
    parser.add_argument("--operation-id")
    parser.add_argument("--correlation-id")
    parser.add_argument("--device-id")


def _normalized_json(value: str, field_name: str) -> str:
    text = require_non_empty(value, field_name)
    try:
        parsed = json.loads(text)
    except json.JSONDecodeError as error:
        raise ValueError(f"{field_name} must contain valid JSON") from error
    return json.dumps(parsed, sort_keys=True)


def _positive_optional_int(value: int | None, field_name: str) -> int | None:
    if value is None:
        return None
    if value <= 0:
        raise ValueError(f"{field_name} must be positive")
    return value


def _due_at_from_period(
    *,
    calibrated_at: str,
    due_at: str | None,
    calibration_period_months: int | None,
) -> str:
    if _has_text(due_at):
        return require_non_empty(str(due_at), "due_at")
    if calibration_period_months is None:
        raise ValueError("due_at is required when no calibration period is set")
    calibrated_date = date.fromisoformat(calibrated_at)
    due_date = _add_months(calibrated_date, calibration_period_months)
    return due_date.isoformat()


def _add_months(value: date, months: int) -> date:
    month_index = value.month - 1 + months
    year = value.year + month_index // 12
    month = month_index % 12 + 1
    day = min(value.day, calendar.monthrange(year, month)[1])
    return date(year, month, day)


def _has_text(value: str | None) -> bool:
    return value is not None and bool(value.strip())


def _missing_contract_review_items(
    repository: ProjectRepository,
    project: dict[str, object],
) -> tuple[str, ...]:
    required = CONTRACT_REVIEW_REQUIRED_ITEMS.get(str(project["execution_mode"]), ())
    if not required:
        return ()
    completed = {
        str(row["item"])
        for row in repository.contract_review_items(str(project["code"]))
        if int(row["completed"])
    }
    return tuple(item for item in required if item not in completed)


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
