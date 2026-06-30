"""Generate local GUI bootstrap data for the static operator console."""

from __future__ import annotations

import argparse
from copy import deepcopy
import json
from pathlib import Path
from typing import Any

from .local_agent_client import LocalAgentClient
from .sqlite_repositories import (
    MeasurementDataRepository,
    MetrologyRepository,
    ProjectRepository,
    TestDefinitionRepository,
    UpdateCatalogRepository,
)


BootstrapData = dict[str, list[Any]]

FALLBACK_BOOTSTRAP: BootstrapData = {
    "projects": [
        {
            "code": "CEM-2026-001",
            "customer": "Rail Motion",
            "stage": "Measuring",
            "mode": "Accredite",
            "blocker": "Calibration due soon",
            "run": "RUN-001",
            "method": "Railway harmonics",
        },
        {
            "code": "CEM-2026-002",
            "customer": "Aero Bench",
            "stage": "Contract review",
            "mode": "Non accredite",
            "blocker": "Aucun",
            "run": "RUN-004",
            "method": "Conducted immunity",
        },
        {
            "code": "CEM-2026-003",
            "customer": "Power Lab",
            "stage": "Investigation",
            "mode": "Investigation",
            "blocker": "Mode relaxe",
            "run": "RUN-007",
            "method": "Inrush current",
        },
    ],
    "contract_review_items": [
        ["CEM-2026-002", "requirements_reviewed", "yes", "quality.lead", "Mode non accredite accepte"],
        ["CEM-2026-002", "method_available", "yes", "technical.lead", "Methode conduite disponible"],
    ],
    "instruments": [
        ["RX-001", "Receiver", "Usable", "Available", "CERT-2026-001", "2027-01-01", "ok", "EMI test receiver", "detectors", "Rohde Schwarz", "ESW", "100001", "ESW44", "2026-01-01", "12", "2"],
        ["GEN-002", "Generator", "Usable", "Reserved", "CERT-2025-044", "2026-07-12", "warn", "RF signal generator", "scpi", "Keysight", "N5183B", "100002", "N5183B-540", "2025-07-12", "12", "1"],
        ["DAQ-OPEN-01", "DAQ", "Usable", "Available", "CERT-2026-112", "2027-03-18", "ok", "DAQ chassis and modules", "8 channels", "openDAQ", "Reference DAQ", "DAQ001", "ODAQ-8", "2026-03-18", "12", "3"],
        ["AMP-004", "Amplifier", "Out of service", "Available", "CERT-2024-090", "2025-12-04", "danger", "RF power amplifier", "interlock", "RF Lab", "AMP-250", "AMP004", "AMP-250", "2024-12-04", "12", "1"],
    ],
    "instrument_documents": [
        ["RX-001", "certificate", "Certificat 2026", "metrology/RX-001/cert-2026.pdf", "A", "receiver calibration"],
        ["RX-001", "datasheet", "Datasheet ESW", "metrology/RX-001/datasheet.pdf", "A", "technical data"],
        ["DAQ-OPEN-01", "script", "openDAQ init", "scripts/daq/opendaq_init.py", "A", "measurement setup"],
    ],
    "instrument_categories": [
        ["emi_receiver", "emc", "EMI test receiver", "required", "rf"],
        ["line_impedance_stabilization_network", "emc", "LISN and AMN", "required", "rf"],
        ["oscilloscope", "electronics", "Oscilloscope", "required", "electrical"],
        ["thermal_camera", "thermal", "Thermal camera", "conditional", "thermal"],
        ["sound_level_meter", "acoustic", "Sound level meter", "required", "acoustic"],
        ["accelerometer", "shock_vibration", "Accelerometer", "required", "mechanical"],
        ["spectrum_analyzer", "radio_rf", "Spectrum analyzer", "required", "rf"],
        ["daq_chassis", "data_monitoring", "DAQ chassis and modules", "required", "data_acquisition"],
    ],
    "methods": [
        ["EN61000-4-6-CS", "Conducted immunity", "frequency_sweep", "approved", "sha256:methodA"],
        ["RAIL-HARM-01", "Railway harmonics", "mixed_time_frequency", "approved", "sha256:railH"],
        ["INRUSH-DAQ-01", "Inrush current", "time_series", "draft", "sha256:inrushD"],
        ["AXLE-COUNT-01", "Axle counter", "event_triggered", "approved", "sha256:axle"],
    ],
    "test_categories": [
        ["emission", "", "Emission", "active"],
        ["emission_conducted", "emission", "Emission conduite", "active"],
        ["emission_radiated", "emission", "Emission rayonnee", "active"],
        ["immunity", "", "Immunite", "active"],
        ["immunity_conducted", "immunity", "Immunite conduite", "active"],
        ["immunity_radiated", "immunity", "Immunite rayonnee", "active"],
    ],
    "schedule": [
        ["PLAN-001", "CEM-2026-001", "Pre-scan emission conduite", "emission_conducted", "2026-07-01T09:00", "2026-07-01T12:00", "operator.one", "Lab A", "planned"],
        ["PLAN-002", "CEM-2026-001", "Immunite rayonnee", "immunity_radiated", "2026-07-02T13:00", "2026-07-02T17:00", "operator.two", "Chambre", "confirmed"],
    ],
    "datasets": [
        ["RUN-001", "raw_signal", "data/RUN-001/raw.opendata", "sha256:raw001", "Immutable"],
        ["RUN-001", "processed_signal", "data/RUN-001/current_fft.csv", "sha256:fft001", "Linked"],
        ["RUN-004", "raw_sweep", "data/RUN-004/sweep.csv", "sha256:sweep004", "Immutable"],
    ],
    "updates": [
        ["emc-locus-core", "0.2.0", "Signed", "Compatible", "offline_bundle"],
        ["driver-pack-visa", "0.1.0", "Signed", "Pending validation", "online_catalog"],
        ["report-template-fr", "0.1.1", "Signed", "Installed", "offline_bundle"],
    ],
    "project_audit_events": [],
    "sync_outbox": [],
}

STAGE_LABELS = {
    "quotation": "Quotation",
    "contract_review": "Contract review",
    "test_planning": "Test planning",
    "measuring": "Measuring",
    "technical_review": "Technical review",
    "report_issued": "Report issued",
    "archived": "Archived",
}

MODE_LABELS = {
    "accredited": "Accredite",
    "non_accredited": "Non accredite",
    "investigation": "Investigation",
}

AVAILABILITY_LABELS = {
    "available": "Available",
    "reserved": "Reserved",
    "out_of_service": "Out of service",
}

SERVICEABILITY_LABELS = {
    "usable": "Usable",
    "restricted": "Restricted",
    "out_of_service": "Out of service",
    "retired": "Retired",
}


def build_fixture_bootstrap() -> BootstrapData:
    """Return fixture data in the shape expected by the static GUI."""

    return deepcopy(FALLBACK_BOOTSTRAP)


def build_bootstrap(
    *,
    project_agent: LocalAgentClient | None = None,
    projects: ProjectRepository | None = None,
    metrology: MetrologyRepository | None = None,
    test_definitions: TestDefinitionRepository | None = None,
    measurement_data: MeasurementDataRepository | None = None,
    update_catalog: UpdateCatalogRepository | None = None,
) -> BootstrapData:
    """Build GUI bootstrap data from available local repositories."""

    payload = build_fixture_bootstrap()

    if project_agent is not None:
        project_rows = project_agent.list_projects().get("projects", [])
        payload["projects"] = [_agent_project_row(row) for row in project_rows]
        payload["contract_review_items"] = [
            _agent_contract_review_item_row(str(project["code"]), item)
            for project in project_rows
            for item in project_agent.contract_review(str(project["code"]))
            .get("contract_review", {})
            .get("completed_items", [])
        ]
        payload["project_audit_events"] = [
            _agent_audit_event_row(str(project["code"]), row)
            for project in project_rows
            for row in project_agent.audit_events(str(project["code"])).get(
                "audit_events", []
            )
        ]
        payload["sync_outbox"] = [
            _agent_sync_outbox_row(row)
            for row in project_agent.sync_outbox().get("sync_outbox", [])
        ]
        payload["schedule"] = []
    elif projects is not None:
        project_rows = projects.list_projects()
        payload["projects"] = [_project_row(row) for row in project_rows]
        payload["contract_review_items"] = [
            _contract_review_item_row(row)
            for project in project_rows
            for row in projects.contract_review_items(str(project["code"]))
        ]
        payload["schedule"] = [
            _schedule_row(row) for row in projects.list_service_schedule_items()
        ]

    if metrology is not None:
        instruments = metrology.list_instruments()
        payload["instruments"] = [_instrument_row(metrology, row) for row in instruments]
        payload["instrument_documents"] = [
            _instrument_document_row(document)
            for instrument in instruments
            for document in metrology.list_instrument_documents(str(instrument["asset_id"]))
        ]
        payload["instrument_categories"] = [
            _instrument_category_row(row) for row in metrology.list_instrument_categories()
        ]

    if test_definitions is not None:
        payload["methods"] = [
            _method_row(test_definitions, row)
            for row in test_definitions.list_test_methods()
        ]
        payload["test_categories"] = [
            _test_category_row(row) for row in test_definitions.list_all_test_categories()
        ]

    if measurement_data is not None:
        payload["datasets"] = [_dataset_row(row) for row in measurement_data.list_datasets()]
        payload["runtime"] = [
            _runtime_row(row) for row in measurement_data.latest_instrument_observations()
        ]

    if update_catalog is not None:
        payload["updates"] = [_update_row(row) for row in update_catalog.list_update_packages()]

    return payload


def build_agent_project_bootstrap(client: LocalAgentClient) -> BootstrapData:
    """Build only the migrated project slice from the local Rust agent."""

    return build_bootstrap(project_agent=client)


def write_bootstrap_js(output_path: Path | str, payload: BootstrapData) -> None:
    """Write a browser-loadable bootstrap file for file:// usage."""

    output_path = Path(output_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    encoded = json.dumps(payload, indent=2, sort_keys=True)
    output_path.write_text(f"window.EMC_LOCUS_BOOTSTRAP = {encoded};\n", encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--migrations-root", type=Path, default=Path("storage/sqlite"))
    parser.add_argument("--projects-db", type=Path)
    parser.add_argument("--metrology-db", type=Path)
    parser.add_argument("--test-definitions-db", type=Path)
    parser.add_argument("--measurement-data-db", type=Path)
    parser.add_argument("--update-catalog-db", type=Path)
    args = parser.parse_args(argv)

    payload = build_bootstrap(
        projects=_repository_if_exists(ProjectRepository, args.projects_db, args.migrations_root),
        metrology=_repository_if_exists(MetrologyRepository, args.metrology_db, args.migrations_root),
        test_definitions=_repository_if_exists(
            TestDefinitionRepository,
            args.test_definitions_db,
            args.migrations_root,
        ),
        measurement_data=_repository_if_exists(
            MeasurementDataRepository,
            args.measurement_data_db,
            args.migrations_root,
        ),
        update_catalog=_repository_if_exists(
            UpdateCatalogRepository,
            args.update_catalog_db,
            args.migrations_root,
        ),
    )
    write_bootstrap_js(args.output, payload)
    return 0


def _repository_if_exists(
    repository_type: type[
        ProjectRepository
        | MetrologyRepository
        | TestDefinitionRepository
        | MeasurementDataRepository
        | UpdateCatalogRepository
    ],
    database_path: Path | None,
    migrations_root: Path,
):
    if database_path is None or not database_path.exists():
        return None
    repository = repository_type(database_path, migrations_root)
    repository.initialize()
    return repository


def _project_row(row: dict[str, object]) -> dict[str, str]:
    return {
        "code": str(row["code"]),
        "customer": str(row["customer_name"]),
        "stage": STAGE_LABELS.get(str(row["stage"]), str(row["stage"])),
        "mode": MODE_LABELS.get(str(row["execution_mode"]), str(row["execution_mode"])),
        "blocker": "Aucun",
        "run": "",
        "method": "",
    }


def _agent_project_row(row: Any) -> dict[str, str]:
    if not isinstance(row, dict):
        raise ValueError("agent project rows must be JSON objects")
    return {
        "code": str(row["code"]),
        "customer": str(row["customer_name"]),
        "stage": STAGE_LABELS.get(str(row["stage"]), str(row["stage"])),
        "mode": MODE_LABELS.get(str(row["execution_mode"]), str(row["execution_mode"])),
        "blocker": "Aucun",
        "run": "",
        "method": "",
    }


def _agent_contract_review_item_row(project_code: str, row: Any) -> list[str]:
    if not isinstance(row, dict):
        raise ValueError("agent contract-review rows must be JSON objects")
    return [
        project_code,
        str(row["item"]),
        "yes",
        str(row.get("completed_by") or ""),
        str(row.get("comment") or ""),
    ]


def _agent_audit_event_row(project_code: str, row: Any) -> list[str]:
    if not isinstance(row, dict):
        raise ValueError("agent audit rows must be JSON objects")
    return [
        project_code,
        str(row["sequence"]),
        str(row["actor"]),
        str(row["action"]),
        str(row.get("reason") or ""),
        str(row["occurred_at"]),
    ]


def _agent_sync_outbox_row(row: Any) -> list[str]:
    if not isinstance(row, dict):
        raise ValueError("agent sync outbox rows must be JSON objects")
    return [
        str(row["operation_id"]),
        str(row["entity_id"]),
        str(row["operation_kind"]),
        str(row["base_revision"]),
        str(row["resulting_revision"]),
        str(row["status"]),
    ]


def _schedule_row(row: dict[str, object]) -> list[str]:
    return [
        str(row["item_code"]),
        str(row["project_code"]),
        str(row["title"]),
        str(row.get("test_category_code") or ""),
        str(row["planned_start_at"]),
        str(row["planned_end_at"]),
        str(row["assigned_operator"]),
        str(row["location"]),
        str(row["status"]),
    ]


def _contract_review_item_row(row: dict[str, object]) -> list[str]:
    return [
        str(row["project_code"]),
        str(row["item"]),
        "yes" if int(row["completed"]) else "no",
        str(row.get("completed_by") or ""),
        str(row.get("comment") or ""),
    ]


def _instrument_row(
    repository: MetrologyRepository,
    row: dict[str, object],
) -> list[str]:
    calibration = repository.latest_calibration_record(str(row["asset_id"]))
    availability = str(row["availability"])
    serviceability = str(
        row.get("serviceability_status")
        or ("out_of_service" if availability == "out_of_service" else "usable")
    )
    calibration_status = str(calibration["status_at_import"]) if calibration else "missing"
    category_label = _instrument_category_label(repository, row)
    return [
        str(row["asset_id"]),
        str(row["family"]),
        SERVICEABILITY_LABELS.get(serviceability, serviceability),
        AVAILABILITY_LABELS.get(availability, availability),
        str(calibration["certificate_reference"]) if calibration else "missing",
        str(calibration["due_at"]) if calibration else "missing",
        _instrument_tone(serviceability, calibration_status),
        category_label,
        _capabilities_preview(str(row["capabilities_json"])),
        str(row.get("manufacturer") or ""),
        str(row.get("model") or ""),
        str(row.get("serial_number") or ""),
        str(row.get("part_number") or ""),
        str(calibration["calibrated_at"]) if calibration else "missing",
        str(row.get("calibration_period_months") or ""),
        str(len(repository.list_instrument_documents(str(row["asset_id"])))),
    ]


def _instrument_document_row(row: dict[str, object]) -> list[str]:
    return [
        str(row["asset_id"]),
        str(row["document_kind"]),
        str(row["title"]),
        str(row["file_reference"]),
        str(row.get("revision") or ""),
        str(row.get("applies_to_function") or ""),
    ]


def _instrument_category_label(
    repository: MetrologyRepository,
    row: dict[str, object],
) -> str:
    category_code = row.get("category_code")
    if category_code is None:
        return str(row["family"])

    category = repository.get_instrument_category(str(category_code))
    if category is None:
        return str(category_code)
    return str(category["label"])


def _capabilities_preview(capabilities_json: str) -> str:
    try:
        parsed = json.loads(capabilities_json)
    except json.JSONDecodeError:
        return "invalid-json"

    if parsed in ({}, []):
        return "none"
    if isinstance(parsed, dict):
        parts = [f"{key}={parsed[key]}" for key in sorted(parsed)[:3]]
        if len(parsed) > 3:
            parts.append("...")
        return ", ".join(parts)
    if isinstance(parsed, list):
        return ", ".join(str(item) for item in parsed[:3]) or "none"
    return str(parsed)


def _instrument_category_row(row: dict[str, object]) -> list[str]:
    return [
        str(row["code"]),
        str(row["domain"]),
        str(row["label"]),
        str(row["default_calibration_requirement"]),
        str(row["calibration_profile"]),
    ]


def _test_category_row(row: dict[str, object]) -> list[str]:
    return [
        str(row["code"]),
        str(row.get("parent_code") or ""),
        str(row["label"]),
        "active" if int(row["active"]) else "inactive",
    ]


def _method_row(
    repository: TestDefinitionRepository,
    row: dict[str, object],
) -> list[str]:
    revisions = repository.method_revisions(str(row["code"]))
    revision = revisions[-1] if revisions else {}
    return [
        str(row["code"]),
        str(row["name"]),
        str(row["measurement_axis"]),
        str(revision.get("status", "draft")),
        str(revision.get("checksum") or "unchecksummed"),
    ]


def _dataset_row(row: dict[str, object]) -> list[str]:
    state = "Immutable" if int(row["immutable"]) else str(row["retention_status"])
    if str(row["retention_status"]) != "retained":
        state = str(row["retention_status"])
    return [
        str(row["measurement_run_reference"]),
        str(row["kind"]),
        str(row["file_reference"]),
        str(row["checksum"]),
        state,
    ]


def _runtime_row(row: dict[str, object]) -> list[str]:
    return [
        str(row["instrument_code"]),
        str(row["transport"]),
        str(row["endpoint"]),
        "OK" if int(row["success"]) else "Echec",
        str(row["measurement_run_reference"]),
        str(row["sequence"]),
        f'{row["command_message"]} -> {row["response_message"]}',
        str(row["exchange_attempts"]),
    ]


def _update_row(row: dict[str, object]) -> list[str]:
    source = "offline_bundle" if int(row["offline_install_allowed"]) else "online_catalog"
    return [
        str(row["package_name"]),
        str(row["package_version"]),
        "Signed" if str(row["signed_checksum"]).strip() else "Unsigned",
        "Available",
        source,
    ]


def _instrument_tone(serviceability: str, calibration_status: str) -> str:
    if serviceability in {"out_of_service", "retired"}:
        return "danger"
    if calibration_status in {"expired", "missing"}:
        return "danger"
    if serviceability == "restricted" or calibration_status == "due_soon":
        return "warn"
    return "ok"


if __name__ == "__main__":
    raise SystemExit(main())
