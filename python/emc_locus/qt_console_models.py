"""View models for the Qt operator console."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


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
        )
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
