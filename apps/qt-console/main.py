"""Qt operator console bootstrap for EMC Locus.

This is the desktop UI direction for measurement-station work. It intentionally
keeps PySide6 as an optional runtime dependency so repository validation can run
without installing the full Qt stack.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
from pathlib import Path
import sys
from typing import Any


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPOSITORY_ROOT / "python"
if str(PYTHON_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))

from emc_locus.qt_console_data import build_console_bootstrap_from_repositories
from emc_locus.gui_actions import (
    advance_project_stage,
    attach_metrology_document,
    complete_contract_review_item_action,
    create_project_record,
    create_test_category,
    register_metrology_instrument,
    run_simulated_emc_test_action,
    schedule_service_item,
    set_metrology_instrument_serviceability,
    update_service_schedule_status_action,
)
from emc_locus.local_agent_client import LocalAgentClient, LocalAgentError
from emc_locus.qt_console_models import (
    FormFieldSpec,
    OperatorFormSpec,
    TableViewModel,
    build_console_view_model,
    build_operator_form_specs,
)


@dataclass(frozen=True)
class AgentStatus:
    """Operator-facing local-agent state."""

    label: str
    tone: str
    detail: str


@dataclass(frozen=True)
class QtBindings:
    """PySide6 objects used by the console, loaded lazily."""

    Qt: Any
    QObject: Any
    QRunnable: Any
    QAbstractItemView: Any
    QApplication: Any
    QComboBox: Any
    QFormLayout: Any
    QGroupBox: Any
    QHBoxLayout: Any
    QLabel: Any
    QLineEdit: Any
    QMainWindow: Any
    QMessageBox: Any
    QPlainTextEdit: Any
    QPushButton: Any
    QScrollArea: Any
    Signal: Any
    QStatusBar: Any
    QTabWidget: Any
    QTableWidget: Any
    QTableWidgetItem: Any
    QThreadPool: Any
    QVBoxLayout: Any
    QWidget: Any
    Slot: Any


def load_bootstrap_json(path: Path) -> dict[str, Any]:
    """Load the strict JSON fixture used by the static Qt demo."""

    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError("bootstrap payload must be a JSON object")
    return data


def run(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--bootstrap",
        type=Path,
        default=None,
        help="Path to a strict JSON bootstrap fixture for static Qt mode.",
    )
    parser.add_argument("--migrations-root", type=Path, default=REPOSITORY_ROOT / "storage" / "sqlite")
    parser.add_argument("--projects-db", type=Path)
    parser.add_argument("--metrology-db", type=Path)
    parser.add_argument("--test-definitions-db", type=Path)
    parser.add_argument("--measurement-data-db", type=Path)
    parser.add_argument("--update-catalog-db", type=Path)
    parser.add_argument("--agent-url")
    args = parser.parse_args(argv)
    data = _load_console_data(args)
    view_model = build_console_view_model(data)
    qt = _load_qt()
    state = {"data": data, "view_model": view_model}

    application = qt.QApplication([])
    window = qt.QMainWindow()
    window.setWindowTitle("EMC Locus")
    window.resize(1320, 820)
    window.setStatusBar(qt.QStatusBar())

    root = qt.QWidget()
    layout = qt.QVBoxLayout(root)
    layout.setContentsMargins(18, 18, 18, 18)
    layout.setSpacing(14)

    header = qt.QHBoxLayout()
    title = qt.QLabel("EMC Locus")
    title.setObjectName("Title")
    subtitle = qt.QLabel("Poste operateur local - prototype Qt")
    subtitle.setObjectName("Subtitle")
    header.addWidget(title)
    header.addWidget(subtitle, 1)
    for action in view_model.actions:
        button = qt.QPushButton(action.label)
        button.setEnabled(action.enabled)
        button.setToolTip(action.reason)
        header.addWidget(button)
    agent_status_label = qt.QLabel()
    agent_status_label.setMinimumWidth(210)
    _apply_agent_status(qt, agent_status_label, args)
    header.addWidget(agent_status_label)
    layout.addLayout(header)

    metrics = qt.QHBoxLayout()
    metrics.setSpacing(10)
    layout.addLayout(metrics)

    tabs = qt.QTabWidget()
    layout.addWidget(tabs, 1)

    window.setCentralWidget(root)

    def refresh_console(message: str | None = None) -> None:
        state["data"] = _load_console_data(args)
        state["view_model"] = build_console_view_model(state["data"])
        _apply_agent_status(qt, agent_status_label, args)
        _populate_metrics(qt, metrics, state["view_model"])
        _populate_tabs(
            qt,
            tabs,
            args,
            state["data"],
            state["view_model"],
            refresh_console,
        )
        if message:
            window.statusBar().showMessage(message)

    _populate_metrics(qt, metrics, state["view_model"])
    _populate_tabs(qt, tabs, args, state["data"], state["view_model"], refresh_console)
    window.statusBar().showMessage(_status_message(args))
    window.setStyleSheet(_stylesheet())
    window.show()
    return application.exec()


def _apply_agent_status(qt: QtBindings, label: Any, args: argparse.Namespace) -> None:
    status = _agent_status(getattr(args, "agent_url", None))
    label.setText(status.label)
    label.setToolTip(status.detail)
    label.setObjectName(f"AgentStatus-{status.tone}")
    if hasattr(label, "style"):
        label.style().unpolish(label)
        label.style().polish(label)


def _agent_status(agent_url: str | None) -> AgentStatus:
    if not agent_url or not agent_url.strip():
        return AgentStatus(
            label="Agent local: non configure",
            tone="neutral",
            detail="Aucun --agent-url fourni.",
        )
    try:
        report = LocalAgentClient(agent_url.strip(), timeout_seconds=1.0).storage_status()
    except LocalAgentError as error:
        if error.code == "local_agent_unavailable":
            label = "Agent local: indisponible"
        else:
            label = f"Agent local: {error.code}"
        return AgentStatus(label=label, tone="warn", detail=error.message)
    except Exception as error:  # noqa: BLE001 - startup status must not crash Qt.
        return AgentStatus(
            label="Agent local: statut inconnu",
            tone="warn",
            detail=str(error),
        )
    return _agent_status_from_storage_report(report)


def _agent_status_from_storage_report(report: dict[str, Any]) -> AgentStatus:
    domains = report.get("domains")
    if not isinstance(domains, list):
        return AgentStatus(
            label="Agent local: reponse invalide",
            tone="warn",
            detail="Le statut stockage ne contient pas de liste domains.",
        )

    statuses = {
        str(domain.get("status", "unknown"))
        for domain in domains
        if isinstance(domain, dict)
    }
    if "invalid" in statuses:
        return AgentStatus(
            label="Agent local: erreur integrite",
            tone="bad",
            detail="Au moins une base locale est invalide.",
        )
    if "migration_required" in statuses:
        return AgentStatus(
            label="Agent local: migration requise",
            tone="warn",
            detail="Au moins une base locale doit etre migree.",
        )
    if "missing" in statuses:
        return AgentStatus(
            label="Agent local: stockage non initialise",
            tone="warn",
            detail="Initialiser le stockage local avant les operations projet.",
        )
    if statuses == {"current"}:
        return AgentStatus(
            label="Agent local: connecte",
            tone="ok",
            detail="Agent joignable et bases project/sync a jour.",
        )
    return AgentStatus(
        label="Agent local: statut partiel",
        tone="warn",
        detail=f"Statuts stockage: {', '.join(sorted(statuses)) or 'aucun'}",
    )


def _metric(qt: QtBindings, label: str, value: str, tone: str) -> Any:
    widget = qt.QWidget()
    widget.setObjectName(f"Metric-{tone}")
    layout = qt.QVBoxLayout(widget)
    layout.setContentsMargins(12, 9, 12, 9)
    value_label = qt.QLabel(value)
    value_label.setObjectName("MetricValue")
    text_label = qt.QLabel(label)
    text_label.setObjectName("MetricLabel")
    layout.addWidget(value_label)
    layout.addWidget(text_label)
    return widget


def _populate_metrics(qt: QtBindings, layout: Any, view_model: Any) -> None:
    while layout.count():
        item = layout.takeAt(0)
        widget = item.widget() if item is not None else None
        if widget is not None:
            widget.deleteLater()

    for metric in view_model.metrics:
        layout.addWidget(_metric(qt, metric.label, metric.value, metric.tone))
    layout.addStretch(1)


def _populate_tabs(
    qt: QtBindings,
    tabs: Any,
    args: argparse.Namespace,
    bootstrap: dict[str, Any],
    view_model: Any,
    on_completed: Any,
) -> None:
    tabs.clear()
    form_specs = build_operator_form_specs(bootstrap, _writable_repositories(args))
    tabs.addTab(_forms_tab(qt, args, form_specs, on_completed), "Saisie")
    for table_model in view_model.tables:
        tabs.addTab(_table(qt, table_model), table_model.tab_label)


def _forms_tab(
    qt: QtBindings,
    args: argparse.Namespace,
    form_specs: tuple[OperatorFormSpec, ...],
    on_completed: Any,
) -> Any:
    scroll = qt.QScrollArea()
    scroll.setWidgetResizable(True)
    content = qt.QWidget()
    layout = qt.QVBoxLayout(content)
    layout.setContentsMargins(10, 10, 10, 10)
    layout.setSpacing(12)

    for form_spec in form_specs:
        layout.addWidget(_action_form(qt, args, form_spec, on_completed))
    layout.addStretch(1)

    scroll.setWidget(content)
    return scroll


def _action_form(
    qt: QtBindings,
    args: argparse.Namespace,
    form_spec: OperatorFormSpec,
    on_completed: Any,
) -> Any:
    group = qt.QGroupBox(form_spec.title)
    group.setEnabled(form_spec.enabled)
    if form_spec.disabled_reason:
        group.setToolTip(form_spec.disabled_reason)

    layout = qt.QFormLayout(group)
    layout.setContentsMargins(12, 12, 12, 12)
    layout.setSpacing(8)
    widgets: dict[str, Any] = {}

    for field in form_spec.fields:
        widget = _field_widget(qt, field)
        widget.setEnabled(form_spec.enabled)
        widgets[field.field_id] = widget
        layout.addRow(field.label, widget)

    button = qt.QPushButton(form_spec.submit_label)
    button.setEnabled(form_spec.enabled)
    if form_spec.disabled_reason:
        button.setToolTip(form_spec.disabled_reason)
    button.clicked.connect(
        lambda checked=False, spec=form_spec, fields=widgets, submit_button=button: _submit_action_form(
            qt,
            args,
            spec,
            fields,
            on_completed,
            submit_button,
        )
    )
    layout.addRow("", button)
    return group


def _field_widget(qt: QtBindings, field: FormFieldSpec) -> Any:
    if field.widget == "choice":
        widget = qt.QComboBox()
        for value, label in field.choices:
            widget.addItem(label, value)
        return widget

    if field.widget == "multiline":
        widget = qt.QPlainTextEdit()
        widget.setPlainText(field.default)
        widget.setMaximumHeight(82)
        return widget

    widget = qt.QLineEdit()
    widget.setText(field.default)
    return widget


def _submit_action_form(
    qt: QtBindings,
    args: argparse.Namespace,
    form_spec: OperatorFormSpec,
    widgets: dict[str, Any],
    on_completed: Any,
    submit_button: Any | None = None,
) -> None:
    try:
        values = _form_values(form_spec, widgets)
    except Exception as error:  # noqa: BLE001 - GUI boundary must show failures.
        qt.QMessageBox.critical(None, "EMC Locus", str(error))
        return

    _run_form_action_worker(qt, args, form_spec, values, on_completed, submit_button)


def _run_form_action_worker(
    qt: QtBindings,
    args: argparse.Namespace,
    form_spec: OperatorFormSpec,
    values: dict[str, str],
    on_completed: Any,
    submit_button: Any | None,
) -> None:
    if not all(
        hasattr(qt, name)
        for name in ("QObject", "QRunnable", "QThreadPool", "Signal", "Slot")
    ):
        _submit_action_form_synchronously(qt, args, form_spec, values, on_completed)
        return

    if submit_button is not None:
        submit_button.setEnabled(False)

    class WorkerSignals(qt.QObject):
        completed = qt.Signal(str)
        failed = qt.Signal(str)

    class FormActionWorker(qt.QRunnable):
        def __init__(self) -> None:
            super().__init__()
            self.signals = WorkerSignals()

        @qt.Slot()
        def run(self) -> None:
            try:
                message = (
                    _execute_form_action(args, form_spec.action_id, values)
                    or "Enregistrement effectue"
                )
            except Exception as error:  # noqa: BLE001 - report through Qt boundary.
                self.signals.failed.emit(str(error))
                return
            self.signals.completed.emit(message)

    worker = FormActionWorker()

    def complete(message: str) -> None:
        if submit_button is not None:
            submit_button.setEnabled(form_spec.enabled)
            setattr(submit_button, "_emc_locus_worker", None)
        on_completed(f"{form_spec.title}: {message}")
        qt.QMessageBox.information(None, "EMC Locus", message)

    def fail(message: str) -> None:
        if submit_button is not None:
            submit_button.setEnabled(form_spec.enabled)
            setattr(submit_button, "_emc_locus_worker", None)
        qt.QMessageBox.critical(None, "EMC Locus", message)

    worker.signals.completed.connect(complete)
    worker.signals.failed.connect(fail)
    if submit_button is not None:
        setattr(submit_button, "_emc_locus_worker", worker)
    qt.QThreadPool.globalInstance().start(worker)


def _submit_action_form_synchronously(
    qt: QtBindings,
    args: argparse.Namespace,
    form_spec: OperatorFormSpec,
    values: dict[str, str],
    on_completed: Any,
) -> None:
    try:
        message = (
            _execute_form_action(args, form_spec.action_id, values)
            or "Enregistrement effectue"
        )
    except Exception as error:  # noqa: BLE001 - GUI boundary must show failures.
        qt.QMessageBox.critical(None, "EMC Locus", str(error))
        return

    on_completed(f"{form_spec.title}: {message}")
    qt.QMessageBox.information(None, "EMC Locus", message)


def _form_values(
    form_spec: OperatorFormSpec,
    widgets: dict[str, Any],
) -> dict[str, str]:
    values: dict[str, str] = {}
    fields = {field.field_id: field for field in form_spec.fields}
    for field_id, widget in widgets.items():
        value = _widget_value(widget)
        if fields[field_id].required and not value.strip():
            raise ValueError(f"{fields[field_id].label}: valeur requise")
        values[field_id] = value.strip()
    return values


def _widget_value(widget: Any) -> str:
    if hasattr(widget, "currentData"):
        data = widget.currentData()
        return "" if data is None else str(data)
    if hasattr(widget, "toPlainText"):
        return str(widget.toPlainText())
    return str(widget.text())


def _execute_form_action(
    args: argparse.Namespace,
    action_id: str,
    values: dict[str, str],
) -> str | None:
    if action_id == "register_instrument":
        register_metrology_instrument(
            metrology_db=getattr(args, "metrology_db", None),
            migrations_root=args.migrations_root,
            asset_id=values["asset_id"],
            family=values["family"],
            manufacturer=values["manufacturer"],
            model=values["model"],
            serial_number=values["serial_number"],
            category_code=values["category_code"],
            serviceability_status=values["serviceability_status"],
            serviceability_reason=values["serviceability_reason"],
            part_number=_optional(values["part_number"]),
            calibration_period_months=_optional_int(
                values["calibration_period_months"],
                "Periodicite mois",
            ),
            metrology_notes=values["metrology_notes"],
            certificate_reference=_optional(values["certificate_reference"]),
            calibrated_at=_optional(values["calibrated_at"]),
            provider=_optional(values["provider"]),
            file_reference=_optional(values["file_reference"]),
            capabilities_json=values["capabilities_json"] or "[]",
            agent_url=getattr(args, "agent_url", None),
        )
        return

    if action_id == "create_project":
        create_project_record(
            projects_db=_required_path(args.projects_db, "projects"),
            migrations_root=args.migrations_root,
            code=values["code"],
            customer_name=values["customer_name"],
            execution_mode=values["execution_mode"],
            stage=values["stage"],
            actor=values["actor"],
            reason=values["reason"],
            agent_url=getattr(args, "agent_url", None),
        )
        return

    if action_id == "complete_contract_review_item":
        complete_contract_review_item_action(
            projects_db=_required_path(args.projects_db, "projects"),
            migrations_root=args.migrations_root,
            project_code=values["project_code"],
            item=values["item"],
            completed_by=values["completed_by"],
            comment=_optional(values["comment"]),
            agent_url=getattr(args, "agent_url", None),
        )
        return

    if action_id == "advance_project":
        advance_project_stage(
            projects_db=args.projects_db,
            migrations_root=args.migrations_root,
            code=values["code"],
            actor=values["actor"],
            reason=values["reason"],
            agent_url=getattr(args, "agent_url", None),
        )
        return

    if action_id == "run_simulated_emc_test":
        result = run_simulated_emc_test_action(
            agent_url=getattr(args, "agent_url", None),
            migrations_root=args.migrations_root,
            attempt_id=values["attempt_id"],
            project_code=values["project_code"],
            test_method_reference=values["test_method_reference"],
            execution_mode=values["execution_mode"],
            asset_id=values["asset_id"],
            operator=values["operator"],
            checked_on=values["checked_on"],
            reason=values["reason"],
            projects_db=getattr(args, "projects_db", None),
            metrology_db=getattr(args, "metrology_db", None),
            test_definitions_db=getattr(args, "test_definitions_db", None),
            measurement_data_db=getattr(args, "measurement_data_db", None),
            update_catalog_db=getattr(args, "update_catalog_db", None),
        )
        return str(result["message"])

    if action_id == "attach_instrument_document":
        attach_metrology_document(
            metrology_db=_required_path(args.metrology_db, "metrology"),
            migrations_root=args.migrations_root,
            asset_id=values["asset_id"],
            document_kind=values["document_kind"],
            title=values["title"],
            file_reference=values["file_reference"],
            uploaded_by=values["uploaded_by"],
            checksum=_optional(values["checksum"]),
            revision=_optional(values["revision"]),
            applies_to_function=_optional(values["applies_to_function"]),
        )
        return

    if action_id == "set_instrument_serviceability":
        set_metrology_instrument_serviceability(
            metrology_db=getattr(args, "metrology_db", None),
            migrations_root=args.migrations_root,
            asset_id=values["asset_id"],
            serviceability_status=values["serviceability_status"],
            serviceability_reason=values["serviceability_reason"],
            agent_url=getattr(args, "agent_url", None),
        )
        return

    if action_id == "schedule_service_item":
        schedule_service_item(
            projects_db=_required_path(args.projects_db, "projects"),
            migrations_root=args.migrations_root,
            item_code=values["item_code"],
            project_code=values["project_code"],
            title=values["title"],
            planned_start_at=values["planned_start_at"],
            planned_end_at=values["planned_end_at"],
            assigned_operator=values["assigned_operator"],
            location=values["location"],
            equipment_under_test=values["equipment_under_test"],
            test_category_code=_optional(values["test_category_code"]),
            test_method_code=_optional(values["test_method_code"]),
            status=values["status"],
            notes=values["notes"],
        )
        return

    if action_id == "update_service_schedule_status":
        update_service_schedule_status_action(
            projects_db=_required_path(args.projects_db, "projects"),
            migrations_root=args.migrations_root,
            item_code=values["item_code"],
            status=values["status"],
            actor=values["actor"],
            reason=_optional(values["reason"]),
        )
        return

    if action_id == "create_test_category":
        create_test_category(
            test_definitions_db=_required_path(
                args.test_definitions_db,
                "test_definitions",
            ),
            migrations_root=args.migrations_root,
            code=values["code"],
            parent_code=_optional(values["parent_code"]),
            label=values["label"],
            description=values["description"],
            sort_order=_int_or_zero(values["sort_order"], "Ordre"),
        )
        return

    raise ValueError(f"unknown form action: {action_id}")


def _required_path(path: Path | None, domain: str) -> Path:
    if path is None:
        raise ValueError(f"missing {domain} database path")
    return path


def _optional(value: str) -> str | None:
    stripped = value.strip()
    return stripped if stripped else None


def _optional_int(value: str, label: str) -> int | None:
    stripped = value.strip()
    if not stripped:
        return None
    try:
        parsed = int(stripped)
    except ValueError as error:
        raise ValueError(f"{label}: nombre entier attendu") from error
    return parsed


def _int_or_zero(value: str, label: str) -> int:
    parsed = _optional_int(value, label)
    return 0 if parsed is None else parsed


def _writable_repositories(args: argparse.Namespace) -> set[str]:
    writable: set[str] = set()
    agent_url = getattr(args, "agent_url", None)
    if agent_url is not None and agent_url.strip():
        writable.add("metrology")
        writable.add("metrology_agent")
        writable.add("projects")
        writable.add("test_execution_agent")
    if args.metrology_db is not None:
        writable.add("metrology")
        writable.add("metrology_documents")
    if args.projects_db is not None:
        writable.add("projects")
    if args.test_definitions_db is not None:
        writable.add("test_definitions")
    return writable


def _load_console_data(args: argparse.Namespace) -> dict[str, Any]:
    agent_url = getattr(args, "agent_url", None)
    if (agent_url is not None and agent_url.strip()) or _has_repository_paths(args):
        return build_console_bootstrap_from_repositories(
            migrations_root=args.migrations_root,
            projects_db=args.projects_db,
            metrology_db=args.metrology_db,
            test_definitions_db=args.test_definitions_db,
            measurement_data_db=args.measurement_data_db,
            update_catalog_db=args.update_catalog_db,
            agent_url=agent_url,
        )

    bootstrap = args.bootstrap or REPOSITORY_ROOT / "apps" / "qt-console" / "demo" / "bootstrap.json"
    return load_bootstrap_json(bootstrap)


def _has_repository_paths(args: argparse.Namespace) -> bool:
    return any(
        (
            args.projects_db,
            args.metrology_db,
            args.test_definitions_db,
            args.measurement_data_db,
            args.update_catalog_db,
        )
    )


def _status_message(args: argparse.Namespace) -> str:
    agent_url = getattr(args, "agent_url", None)
    if agent_url is not None and agent_url.strip():
        return "Donnees chargees via l'agent local"

    if _has_repository_paths(args):
        return "Donnees chargees depuis les depots SQLite locaux"

    bootstrap = args.bootstrap or REPOSITORY_ROOT / "apps" / "qt-console" / "demo" / "bootstrap.json"
    return f"Fixture JSON Qt: {bootstrap}"


def _load_qt() -> QtBindings:
    try:
        from PySide6.QtCore import QObject, QRunnable, Qt, QThreadPool, Signal, Slot
        from PySide6.QtWidgets import (
            QAbstractItemView,
            QApplication,
            QComboBox,
            QFormLayout,
            QGroupBox,
            QHBoxLayout,
            QLabel,
            QLineEdit,
            QMainWindow,
            QMessageBox,
            QPlainTextEdit,
            QPushButton,
            QScrollArea,
            QStatusBar,
            QTabWidget,
            QTableWidget,
            QTableWidgetItem,
            QVBoxLayout,
            QWidget,
        )
    except ModuleNotFoundError as error:
        raise SystemExit(
            "PySide6 is required to run the Qt console. Install it in a lab "
            "Python environment with: py -m pip install PySide6"
        ) from error

    return QtBindings(
        Qt=Qt,
        QObject=QObject,
        QRunnable=QRunnable,
        QAbstractItemView=QAbstractItemView,
        QApplication=QApplication,
        QComboBox=QComboBox,
        QFormLayout=QFormLayout,
        QGroupBox=QGroupBox,
        QHBoxLayout=QHBoxLayout,
        QLabel=QLabel,
        QLineEdit=QLineEdit,
        QMainWindow=QMainWindow,
        QMessageBox=QMessageBox,
        QPlainTextEdit=QPlainTextEdit,
        QPushButton=QPushButton,
        QScrollArea=QScrollArea,
        Signal=Signal,
        QStatusBar=QStatusBar,
        QTabWidget=QTabWidget,
        QTableWidget=QTableWidget,
        QTableWidgetItem=QTableWidgetItem,
        QThreadPool=QThreadPool,
        QVBoxLayout=QVBoxLayout,
        QWidget=QWidget,
        Slot=Slot,
    )


def _table(qt: QtBindings, model: TableViewModel) -> Any:
    table = qt.QTableWidget(model.row_count, model.column_count)
    table.setAlternatingRowColors(True)
    table.setSelectionBehavior(qt.QAbstractItemView.SelectionBehavior.SelectRows)
    table.setEditTriggers(qt.QAbstractItemView.EditTrigger.NoEditTriggers)
    table.verticalHeader().setVisible(False)
    table.setHorizontalHeaderLabels(list(model.columns))

    for row_index, row in enumerate(model.rows):
        for column_index, value in enumerate(row):
            item = qt.QTableWidgetItem(value)
            item.setFlags(item.flags() & ~qt.Qt.ItemFlag.ItemIsEditable)
            table.setItem(row_index, column_index, item)

    table.resizeColumnsToContents()
    table.horizontalHeader().setStretchLastSection(True)
    return table


def _stylesheet() -> str:
    return """
    QMainWindow {
        background: #f6f7f4;
    }
    QLabel#Title {
        color: #17211b;
        font-size: 28px;
        font-weight: 700;
    }
    QLabel#Subtitle {
        color: #59635d;
        font-size: 13px;
    }
    QLabel#AgentStatus-ok {
        background: #e8f3ec;
        border: 1px solid #b7d5c0;
        color: #17211b;
        padding: 6px 10px;
        font-weight: 700;
    }
    QLabel#AgentStatus-warn {
        background: #fff5dc;
        border: 1px solid #dec777;
        color: #4c3b06;
        padding: 6px 10px;
        font-weight: 700;
    }
    QLabel#AgentStatus-bad {
        background: #fde8e5;
        border: 1px solid #dd9a91;
        color: #651f18;
        padding: 6px 10px;
        font-weight: 700;
    }
    QLabel#AgentStatus-neutral {
        background: #eef0ed;
        border: 1px solid #d4d8d2;
        color: #3f4842;
        padding: 6px 10px;
        font-weight: 700;
    }
    QTabWidget::pane {
        border: 1px solid #d6d9d2;
        background: white;
    }
    QTabBar::tab {
        background: #e9ece5;
        border: 1px solid #d6d9d2;
        padding: 9px 14px;
    }
    QTabBar::tab:selected {
        background: white;
        border-bottom-color: white;
    }
    QTableWidget {
        gridline-color: #d8ddd5;
        selection-background-color: #2f6f5e;
        selection-color: white;
        font-size: 12px;
    }
    QWidget#Metric-ok {
        background: #e8f3ec;
        border: 1px solid #b7d5c0;
    }
    QWidget#Metric-warn {
        background: #fff5dc;
        border: 1px solid #dec777;
    }
    QWidget#Metric-neutral {
        background: #eef0ed;
        border: 1px solid #d4d8d2;
    }
    QLabel#MetricValue {
        color: #17211b;
        font-size: 18px;
        font-weight: 700;
    }
    QLabel#MetricLabel {
        color: #59635d;
        font-size: 11px;
    }
    QPushButton {
        background: #2f6f5e;
        border: 0;
        color: white;
        padding: 8px 14px;
        font-weight: 600;
    }
    QPushButton:disabled {
        background: #9aa59e;
    }
    QGroupBox {
        background: #ffffff;
        border: 1px solid #d6d9d2;
        margin-top: 9px;
        padding-top: 12px;
        font-weight: 700;
    }
    QGroupBox::title {
        subcontrol-origin: margin;
        left: 10px;
        padding: 0 4px;
    }
    QLineEdit, QPlainTextEdit, QComboBox {
        border: 1px solid #cbd0c8;
        padding: 6px;
        background: white;
        selection-background-color: #2f6f5e;
    }
    """


if __name__ == "__main__":
    raise SystemExit(run())
