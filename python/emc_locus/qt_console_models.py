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
INSTRUMENT_COLUMNS = (
    "Actif",
    "Famille",
    "Etat",
    "Certificat",
    "Validite",
    "Alerte",
    "Categorie",
    "Capacites",
    "Marque",
    "Modele",
    "Serie",
    "Part number",
    "Derniere cal",
    "Periodicite",
    "Docs",
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
class FormFieldSpec:
    """One operator-editable field in a Qt action form."""

    field_id: str
    label: str
    widget: str
    required: bool = False
    default: str = ""
    choices: tuple[tuple[str, str], ...] = ()


@dataclass(frozen=True)
class OperatorFormSpec:
    """A Qt operator form that maps to a local repository action."""

    action_id: str
    title: str
    submit_label: str
    enabled: bool
    disabled_reason: str
    fields: tuple[FormFieldSpec, ...]


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
                columns=INSTRUMENT_COLUMNS,
                rows=_list_rows(bootstrap.get("instruments"), len(INSTRUMENT_COLUMNS)),
            ),
            TableViewModel(
                tab_label="Docs metro",
                title="Documents materiel",
                columns=("Actif", "Type", "Titre", "Fichier", "Revision", "Fonction"),
                rows=_list_rows(bootstrap.get("instrument_documents"), 6),
            ),
            TableViewModel(
                tab_label="Categories",
                title="Categories instruments",
                columns=("Code", "Domaine", "Nom", "Calibration", "Profil"),
                rows=_list_rows(bootstrap.get("instrument_categories"), 5),
            ),
            TableViewModel(
                tab_label="Planning",
                title="Planning de service",
                columns=(
                    "Plan",
                    "Projet",
                    "Essai",
                    "Categorie",
                    "Debut",
                    "Fin",
                    "Operateur",
                    "Lieu",
                    "Statut",
                ),
                rows=_list_rows(bootstrap.get("schedule"), 9),
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
                tab_label="Categories essais",
                title="Categories essais",
                columns=("Code", "Parent", "Nom", "Etat"),
                rows=_list_rows(bootstrap.get("test_categories"), 4),
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


def build_operator_form_specs(
    bootstrap: dict[str, Any],
    writable_repositories: set[str] | frozenset[str],
) -> tuple[OperatorFormSpec, ...]:
    """Build testable Qt form contracts for local write actions."""

    instrument_categories = _choice_rows(
        bootstrap.get("instrument_categories"),
        value_index=0,
        label_index=2,
    )
    instruments = _choice_rows(
        bootstrap.get("instruments"),
        value_index=0,
        label_index=0,
    )
    projects = _project_choices(bootstrap.get("projects"))
    test_categories = _choice_rows(
        bootstrap.get("test_categories"),
        value_index=0,
        label_index=2,
        include_empty=True,
    )
    parent_test_categories = _choice_rows(
        bootstrap.get("test_categories"),
        value_index=0,
        label_index=2,
        include_empty=True,
    )

    has_metrology = "metrology" in writable_repositories
    has_projects = "projects" in writable_repositories
    has_test_definitions = "test_definitions" in writable_repositories

    return (
        OperatorFormSpec(
            action_id="create_project",
            title="Nouveau projet",
            submit_label="Creer projet",
            enabled=has_projects,
            disabled_reason="" if has_projects else "Depot projets requis",
            fields=(
                FormFieldSpec("code", "Code", "text", required=True),
                FormFieldSpec("customer_name", "Client", "text", required=True),
                FormFieldSpec(
                    "execution_mode",
                    "Mode",
                    "choice",
                    required=True,
                    choices=(
                        ("accredited", "Accredite"),
                        ("non_accredited", "Non accredite"),
                        ("investigation", "Investigation"),
                    ),
                ),
                FormFieldSpec(
                    "stage",
                    "Etape",
                    "choice",
                    required=True,
                    choices=(
                        ("quotation", "Devis"),
                        ("contract_review", "Revue contrat"),
                        ("test_planning", "Planning essais"),
                        ("measuring", "Mesure"),
                        ("technical_review", "Revue technique"),
                        ("report_issued", "Rapport fourni"),
                        ("archived", "Archive"),
                    ),
                ),
                FormFieldSpec("actor", "Acteur", "text", required=True),
                FormFieldSpec("reason", "Raison", "multiline", required=True),
            ),
        ),
        OperatorFormSpec(
            action_id="register_instrument",
            title="Nouveau materiel",
            submit_label="Enregistrer",
            enabled=has_metrology and bool(instrument_categories),
            disabled_reason=_disabled_reason(
                has_metrology,
                bool(instrument_categories),
                "Depot metrologie requis",
                "Aucune categorie instrument disponible",
            ),
            fields=(
                FormFieldSpec("asset_id", "Identifiant", "text", required=True),
                FormFieldSpec("family", "Famille", "text", required=True),
                FormFieldSpec("manufacturer", "Marque", "text", required=True),
                FormFieldSpec("model", "Modele", "text", required=True),
                FormFieldSpec("serial_number", "Numero serie", "text", required=True),
                FormFieldSpec(
                    "category_code",
                    "Categorie",
                    "choice",
                    required=True,
                    choices=instrument_categories,
                ),
                FormFieldSpec("part_number", "Part number", "text"),
                FormFieldSpec("calibration_period_months", "Periodicite mois", "text"),
                FormFieldSpec("certificate_reference", "Certificat", "text"),
                FormFieldSpec("calibrated_at", "Date calibration", "text"),
                FormFieldSpec("provider", "Prestataire", "text"),
                FormFieldSpec("file_reference", "Fichier certificat", "text"),
                FormFieldSpec("capabilities_json", "Capacites JSON", "multiline", default="[]"),
                FormFieldSpec("metrology_notes", "Notes", "multiline"),
            ),
        ),
        OperatorFormSpec(
            action_id="attach_instrument_document",
            title="Document materiel",
            submit_label="Ajouter document",
            enabled=has_metrology and bool(instruments),
            disabled_reason=_disabled_reason(
                has_metrology,
                bool(instruments),
                "Depot metrologie requis",
                "Aucun instrument disponible",
            ),
            fields=(
                FormFieldSpec(
                    "asset_id",
                    "Materiel",
                    "choice",
                    required=True,
                    choices=instruments,
                ),
                FormFieldSpec(
                    "document_kind",
                    "Type",
                    "choice",
                    required=True,
                    choices=(
                        ("certificate", "Certificat"),
                        ("datasheet", "Datasheet"),
                        ("transducer_calculation", "Feuillet transducteur"),
                        ("script", "Script"),
                        ("manual", "Manuel"),
                        ("photo", "Photo"),
                        ("other", "Autre"),
                    ),
                ),
                FormFieldSpec("title", "Titre", "text", required=True),
                FormFieldSpec("file_reference", "Fichier", "text", required=True),
                FormFieldSpec("revision", "Revision", "text"),
                FormFieldSpec("applies_to_function", "Fonction", "text"),
                FormFieldSpec("uploaded_by", "Ajoute par", "text", required=True),
                FormFieldSpec("checksum", "Checksum", "text"),
            ),
        ),
        OperatorFormSpec(
            action_id="schedule_service_item",
            title="Planning service",
            submit_label="Planifier",
            enabled=has_projects and bool(projects),
            disabled_reason=_disabled_reason(
                has_projects,
                bool(projects),
                "Depot projets requis",
                "Aucun projet disponible",
            ),
            fields=(
                FormFieldSpec("item_code", "Code planning", "text", required=True),
                FormFieldSpec(
                    "project_code",
                    "Projet",
                    "choice",
                    required=True,
                    choices=projects,
                ),
                FormFieldSpec("title", "Essai", "text", required=True),
                FormFieldSpec("test_category_code", "Categorie essai", "choice", choices=test_categories),
                FormFieldSpec("test_method_code", "Methode", "text"),
                FormFieldSpec("planned_start_at", "Debut", "text", required=True),
                FormFieldSpec("planned_end_at", "Fin", "text", required=True),
                FormFieldSpec("assigned_operator", "Operateur", "text", required=True),
                FormFieldSpec("location", "Lieu", "text", required=True),
                FormFieldSpec("equipment_under_test", "EUT", "text", required=True),
                FormFieldSpec(
                    "status",
                    "Statut",
                    "choice",
                    required=True,
                    choices=(
                        ("planned", "planned"),
                        ("confirmed", "confirmed"),
                        ("in_progress", "in_progress"),
                        ("completed", "completed"),
                        ("cancelled", "cancelled"),
                    ),
                ),
                FormFieldSpec("notes", "Notes", "multiline"),
            ),
        ),
        OperatorFormSpec(
            action_id="create_test_category",
            title="Categorie essai",
            submit_label="Creer categorie",
            enabled=has_test_definitions,
            disabled_reason=(
                "" if has_test_definitions else "Depot definitions essais requis"
            ),
            fields=(
                FormFieldSpec("code", "Code", "text", required=True),
                FormFieldSpec("parent_code", "Parent", "choice", choices=parent_test_categories),
                FormFieldSpec("label", "Nom", "text", required=True),
                FormFieldSpec("description", "Description", "multiline", required=True),
                FormFieldSpec("sort_order", "Ordre", "text", default="0"),
            ),
        ),
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


def _choice_rows(
    rows: Any,
    *,
    value_index: int,
    label_index: int,
    include_empty: bool = False,
) -> tuple[tuple[str, str], ...]:
    choices: list[tuple[str, str]] = []
    if include_empty:
        choices.append(("", ""))
    if not isinstance(rows, list):
        return tuple(choices)

    for row in rows:
        if not isinstance(row, list):
            continue
        if len(row) <= value_index:
            continue
        value = str(row[value_index])
        if not value:
            continue
        label = str(row[label_index]) if len(row) > label_index else value
        choices.append((value, f"{value} - {label}" if label != value else value))
    return tuple(choices)


def _project_choices(rows: Any) -> tuple[tuple[str, str], ...]:
    if not isinstance(rows, list):
        return ()

    choices: list[tuple[str, str]] = []
    for row in rows:
        if isinstance(row, dict):
            code = str(row.get("code", ""))
            customer = str(row.get("customer", ""))
            if code:
                choices.append((code, f"{code} - {customer}" if customer else code))
        elif isinstance(row, list) and row:
            code = str(row[0])
            customer = str(row[1]) if len(row) > 1 else ""
            if code:
                choices.append((code, f"{code} - {customer}" if customer else code))
    return tuple(choices)


def _disabled_reason(
    repository_available: bool,
    data_available: bool,
    repository_reason: str,
    data_reason: str,
) -> str:
    if not repository_available:
        return repository_reason
    if not data_available:
        return data_reason
    return ""


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
    instruments = _list_rows(bootstrap.get("instruments"), len(INSTRUMENT_COLUMNS))
    documents = _list_rows(bootstrap.get("instrument_documents"), 6)
    categories = _list_rows(bootstrap.get("instrument_categories"), 5)
    schedule = _list_rows(bootstrap.get("schedule"), 9)
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
            "Docs materiel",
            str(len(documents)),
            "ok" if documents else "neutral",
        ),
        StatusMetric(
            "Planning",
            str(len(schedule)),
            "warn" if any(row[8] in {"planned", "confirmed"} for row in schedule) else "ok",
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
