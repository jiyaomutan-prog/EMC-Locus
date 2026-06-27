"""View models for the Qt operator console."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


RUNTIME_COLUMNS = (
    "Instrument",
    "Transport",
    "Endpoint",
    "Etat",
    "Run",
    "Sequence",
    "Observation",
    "Tentatives",
)


@dataclass(frozen=True)
class TableViewModel:
    """A simple table contract independent from Qt bindings."""

    tab_label: str
    title: str
    columns: tuple[str, ...]
    rows: tuple[tuple[str, ...], ...]

    @property
    def row_count(self) -> int:
        return len(self.rows)

    @property
    def column_count(self) -> int:
        return len(self.columns)


@dataclass(frozen=True)
class ConsoleViewModel:
    """Top-level console data used by the first Qt shell."""

    tables: tuple[TableViewModel, ...]
    actions: tuple["OperatorActionIntent", ...]
    metrics: tuple["StatusMetric", ...]


@dataclass(frozen=True)
class OperatorActionIntent:
    """A command the Qt console may expose without owning write rules."""

    action_id: str
    label: str
    target_table: str
    enabled: bool
    reason: str


@dataclass(frozen=True)
class StatusMetric:
    """A compact operational metric for the Qt console header."""

    label: str
    value: str
    tone: str


def build_console_view_model(bootstrap: dict[str, Any]) -> ConsoleViewModel:
    """Convert bootstrap data into explicit Qt-facing table models."""

    return ConsoleViewModel(
        tables=(
            TableViewModel(
                tab_label="Projets",
                title="Projets",
                columns=("Code", "Client", "Etape", "Mode", "Blocage", "Run", "Methode"),
                rows=_project_rows(bootstrap.get("projects")),
            ),
            TableViewModel(
                tab_label="Metrologie",
                title="Instruments",
                columns=("Actif", "Famille", "Etat", "Certificat", "Validite", "Alerte"),
                rows=_list_rows(bootstrap.get("instruments"), 6),
            ),
            TableViewModel(
                tab_label="Categories",
                title="Categories instruments",
                columns=("Code", "Domaine", "Nom", "Calibration", "Profil"),
                rows=_list_rows(bootstrap.get("instrument_categories"), 5),
            ),
            TableViewModel(
                tab_label="Runtime",
                title="Instrument runtime",
                columns=RUNTIME_COLUMNS,
                rows=_list_rows(bootstrap.get("runtime"), len(RUNTIME_COLUMNS)),
            ),
            TableViewModel(
                tab_label="Methodes",
                title="Methodes",
                columns=("Code", "Nom", "Axe", "Statut", "Checksum"),
                rows=_list_rows(bootstrap.get("methods"), 5),
            ),
            TableViewModel(
                tab_label="Donnees",
                title="Donnees",
                columns=("Run", "Type", "Fichier", "Checksum", "Retention"),
                rows=_list_rows(bootstrap.get("datasets"), 5),
            ),
            TableViewModel(
                tab_label="Updates",
                title="Mises a jour",
                columns=("Package", "Version", "Signature", "Statut", "Source"),
                rows=_list_rows(bootstrap.get("updates"), 5),
            ),
        ),
        actions=_action_intents(bootstrap),
        metrics=_status_metrics(bootstrap),
    )


def _project_rows(rows: Any) -> tuple[tuple[str, ...], ...]:
    if not isinstance(rows, list):
        return ()

    normalized: list[tuple[str, ...]] = []
    for row in rows:
        if isinstance(row, dict):
            normalized.append(
                (
                    str(row.get("code", "")),
                    str(row.get("customer", "")),
                    str(row.get("stage", "")),
                    str(row.get("mode", "")),
                    str(row.get("blocker", "")),
                    str(row.get("run", "")),
                    str(row.get("method", "")),
                )
            )
        elif isinstance(row, list):
            normalized.append(_fixed_row(row, 7))
    return tuple(normalized)


def _list_rows(rows: Any, column_count: int) -> tuple[tuple[str, ...], ...]:
    if not isinstance(rows, list):
        return ()

    normalized: list[tuple[str, ...]] = []
    for row in rows:
        if isinstance(row, dict):
            normalized.append(_fixed_row(list(row.values()), column_count))
        elif isinstance(row, list):
            normalized.append(_fixed_row(row, column_count))
        else:
            normalized.append(_fixed_row([row], column_count))
    return tuple(normalized)


def _fixed_row(row: list[Any], column_count: int) -> tuple[str, ...]:
    values = [str(value) for value in row[:column_count]]
    values.extend("" for _ in range(column_count - len(values)))
    return tuple(values)


def _action_intents(bootstrap: dict[str, Any]) -> tuple[OperatorActionIntent, ...]:
    projects = _project_rows(bootstrap.get("projects"))
    datasets = _list_rows(bootstrap.get("datasets"), 5)
    updates = _list_rows(bootstrap.get("updates"), 5)

    project_enabled = any(row[2] != "Archived" for row in projects)
    retention_enabled = any(row[4] in {"Immutable", "retained"} for row in datasets)
    update_enabled = any(row[3].lower() not in {"installed", "installe"} for row in updates)

    return (
        OperatorActionIntent(
            action_id="advance_project",
            label="Avancer projet",
            target_table="projects",
            enabled=project_enabled,
            reason="Selectionner un projet non archive" if project_enabled else "Aucun projet actif",
        ),
        OperatorActionIntent(
            action_id="request_dataset_deletion",
            label="Demander suppression",
            target_table="datasets",
            enabled=retention_enabled,
            reason=(
                "Selectionner un dataset retenu"
                if retention_enabled
                else "Aucun dataset eligible"
            ),
        ),
        OperatorActionIntent(
            action_id="validate_update",
            label="Valider update",
            target_table="updates",
            enabled=update_enabled,
            reason=(
                "Selectionner un package non installe"
                if update_enabled
                else "Aucune update a valider"
            ),
        ),
    )


def _status_metrics(bootstrap: dict[str, Any]) -> tuple[StatusMetric, ...]:
    projects = _project_rows(bootstrap.get("projects"))
    instruments = _list_rows(bootstrap.get("instruments"), 6)
    categories = _list_rows(bootstrap.get("instrument_categories"), 5)
    runtime = _list_rows(bootstrap.get("runtime"), len(RUNTIME_COLUMNS))
    datasets = _list_rows(bootstrap.get("datasets"), 5)
    updates = _list_rows(bootstrap.get("updates"), 5)

    active_projects = sum(1 for row in projects if row[2] != "Archived")
    instrument_alerts = sum(1 for row in instruments if row[5] in {"warn", "danger"})
    runtime_failures = sum(1 for row in runtime if row[3].lower() in {"echec", "failed"})
    max_runtime_attempts = max((_positive_int(row[7]) for row in runtime), default=0)
    retained_datasets = sum(1 for row in datasets if row[4] in {"Immutable", "retained"})
    pending_updates = sum(1 for row in updates if row[3].lower() not in {"installed", "installe"})

    return (
        StatusMetric("Projets actifs", str(active_projects), "ok" if active_projects else "neutral"),
        StatusMetric(
            "Alertes metrologie",
            str(instrument_alerts),
            "warn" if instrument_alerts else "ok",
        ),
        StatusMetric(
            "Categories instruments",
            str(len(categories)),
            "ok" if categories else "neutral",
        ),
        StatusMetric(
            "Erreurs runtime",
            str(runtime_failures),
            "warn" if runtime_failures else "ok",
        ),
        StatusMetric(
            "Tentatives max",
            str(max_runtime_attempts),
            "warn" if max_runtime_attempts > 1 else "ok",
        ),
        StatusMetric(
            "Datasets retenus",
            str(retained_datasets),
            "ok" if retained_datasets else "neutral",
        ),
        StatusMetric(
            "Updates a traiter",
            str(pending_updates),
            "warn" if pending_updates else "ok",
        ),
    )


def _positive_int(value: str) -> int:
    try:
        parsed = int(value)
    except ValueError:
        return 0
    return max(parsed, 0)
