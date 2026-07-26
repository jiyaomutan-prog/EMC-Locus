"""Qt operator console bootstrap for EMC Locus.

This is the desktop UI direction for measurement-station work. It intentionally
keeps PySide6 as an optional runtime dependency so repository validation can run
without installing the full Qt stack.
"""

from __future__ import annotations

import argparse
from copy import deepcopy
from dataclasses import dataclass
from datetime import date
import json
from pathlib import Path
import sys
from typing import Any
import uuid


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
from emc_locus.station_setup_ui import (
    applicable_characterizations,
    build_asset_binding,
    build_correction_selection,
    characterization_display_label,
    current_station_revision,
    editable_definition,
    eligible_station_instruments,
    instrument_display_label,
    model_signal_ports,
    port_display_label,
    readiness_lines,
)
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
    QTimer: Any
    QAbstractItemView: Any
    QApplication: Any
    QComboBox: Any
    QFormLayout: Any
    QFont: Any
    QGroupBox: Any
    QHBoxLayout: Any
    QLabel: Any
    QLineEdit: Any
    QMainWindow: Any
    QMessageBox: Any
    QPlainTextEdit: Any
    QPushButton: Any
    QScrollArea: Any
    QSplitter: Any
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
    parser.add_argument("--screenshot", type=Path)
    parser.add_argument("--screenshot-width", type=int, default=1440)
    parser.add_argument("--screenshot-height", type=int, default=900)
    args = parser.parse_args(argv)
    data = _load_console_data(args)
    view_model = build_console_view_model(data)
    qt = _load_qt()
    state = {"data": data, "view_model": view_model}

    application = qt.QApplication([])
    application.setFont(qt.QFont("Segoe UI", 9))
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
    subtitle = qt.QLabel("Locus Test Station · préparation locale des essais")
    subtitle.setObjectName("Subtitle")
    header.addWidget(title)
    header.addWidget(subtitle, 1)
    agent_mode = bool(args.agent_url and args.agent_url.strip())
    if not agent_mode:
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
        if not agent_mode:
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

    if not agent_mode:
        _populate_metrics(qt, metrics, state["view_model"])
    _populate_tabs(qt, tabs, args, state["data"], state["view_model"], refresh_console)
    window.statusBar().showMessage(_status_message(args))
    window.setStyleSheet(_stylesheet())
    window.show()
    if args.screenshot is not None:
        window.resize(args.screenshot_width, args.screenshot_height)

        def capture_window() -> None:
            args.screenshot.parent.mkdir(parents=True, exist_ok=True)
            if not window.grab().save(str(args.screenshot)):
                qt.QMessageBox.critical(window, "EMC Locus", "La capture Qt n’a pas pu être enregistrée.")
            application.quit()

        qt.QTimer.singleShot(350, capture_window)
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
    agent_url = getattr(args, "agent_url", None)
    if agent_url is not None and agent_url.strip():
        tabs.addTab(
            _station_setup_tab(qt, args, on_completed),
            "Préparation du poste",
        )
        return
    form_specs = build_operator_form_specs(bootstrap, _writable_repositories(args))
    tabs.addTab(_forms_tab(qt, args, form_specs, on_completed), "Saisie générale")
    for table_model in view_model.tables:
        tabs.addTab(_table(qt, table_model), table_model.tab_label)


def _station_setup_tab(
    qt: QtBindings,
    args: argparse.Namespace,
    on_completed: Any,
) -> Any:
    client = LocalAgentClient(args.agent_url.strip())
    state: dict[str, Any] = {
        "setups": [],
        "instruments": [],
        "aggregate": None,
        "revision": None,
        "definition": None,
        "dirty": False,
        "ports": {},
        "characterizations": {},
    }

    root = qt.QWidget()
    root_layout = qt.QVBoxLayout(root)
    root_layout.setContentsMargins(12, 12, 12, 12)
    root_layout.setSpacing(10)

    command_bar = qt.QHBoxLayout()
    setup_selector = qt.QComboBox()
    setup_selector.setMinimumWidth(330)
    refresh_button = qt.QPushButton("Rafraîchir")
    refresh_button.setObjectName("SecondaryButton")
    new_button = qt.QPushButton("Nouveau montage")
    command_bar.addWidget(qt.QLabel("Montage"))
    command_bar.addWidget(setup_selector, 1)
    command_bar.addWidget(refresh_button)
    command_bar.addWidget(new_button)
    root_layout.addLayout(command_bar)

    create_group = qt.QGroupBox("Nouveau montage de mesure")
    create_layout = qt.QHBoxLayout(create_group)
    new_label = qt.QLineEdit()
    new_label.setPlaceholderText("ex. Mesure d’émission conduite")
    new_station = qt.QComboBox()
    new_station.addItem("Nouveau poste…", None)
    new_station_label = qt.QLineEdit()
    new_station_label.setPlaceholderText("ex. Poste CEM mobile")
    new_date = qt.QLineEdit(date.today().isoformat())
    new_date.setMaximumWidth(120)
    new_mode = qt.QComboBox()
    new_mode.addItem("Accrédité", "accredited")
    new_mode.addItem("Non accrédité", "non_accredited")
    new_mode.addItem("Investigation", "investigation")
    create_button = qt.QPushButton("Créer")
    cancel_create_button = qt.QPushButton("Annuler")
    cancel_create_button.setObjectName("SecondaryButton")
    create_layout.addWidget(qt.QLabel("Nom"))
    create_layout.addWidget(new_label, 2)
    create_layout.addWidget(qt.QLabel("Poste"))
    create_layout.addWidget(new_station, 2)
    create_layout.addWidget(new_station_label, 2)
    create_layout.addWidget(qt.QLabel("Date"))
    create_layout.addWidget(new_date)
    create_layout.addWidget(new_mode)
    create_layout.addWidget(cancel_create_button)
    create_layout.addWidget(create_button)
    create_group.setVisible(False)
    root_layout.addWidget(create_group)

    summary = qt.QLabel("Sélectionnez ou créez un montage.")
    summary.setObjectName("StationSummary")
    summary.setWordWrap(True)
    root_layout.addWidget(summary)

    splitter = qt.QSplitter(qt.Qt.Orientation.Horizontal)
    root_layout.addWidget(splitter, 1)

    material_pane = qt.QWidget()
    material_layout = qt.QVBoxLayout(material_pane)
    material_layout.setContentsMargins(0, 0, 6, 0)
    material_group = qt.QGroupBox("Matériels réels du montage")
    material_group_layout = qt.QVBoxLayout(material_group)
    material_table = qt.QTableWidget(0, 2)
    _configure_station_table(
        qt,
        material_table,
        ("Rôle", "Matériel et aptitude"),
    )
    material_table.setWordWrap(True)
    material_group_layout.addWidget(material_table, 1)
    instrument_selector = qt.QComboBox()
    instrument_selector.setMinimumContentsLength(28)
    role_input = qt.QLineEdit()
    role_input.setPlaceholderText("Rôle dans le montage, ex. câble RF")
    material_actions = qt.QHBoxLayout()
    add_material_button = qt.QPushButton("Ajouter")
    remove_material_button = qt.QPushButton("Retirer")
    remove_material_button.setObjectName("SecondaryButton")
    material_actions.addWidget(add_material_button)
    material_actions.addWidget(remove_material_button)
    material_group_layout.addWidget(instrument_selector)
    material_group_layout.addWidget(role_input)
    material_group_layout.addLayout(material_actions)
    material_layout.addWidget(material_group, 1)
    splitter.addWidget(material_pane)

    chain_pane = qt.QWidget()
    chain_layout = qt.QVBoxLayout(chain_pane)
    chain_layout.setContentsMargins(6, 0, 0, 0)

    connection_group = qt.QGroupBox("Liaisons entre entrées et sorties")
    connection_layout = qt.QVBoxLayout(connection_group)
    connection_table = qt.QTableWidget(0, 3)
    _configure_station_table(qt, connection_table, ("Liaison", "Depuis", "Vers"))
    connection_table.setMaximumHeight(155)
    connection_layout.addWidget(connection_table)
    connection_choices = qt.QHBoxLayout()
    source_binding = qt.QComboBox()
    source_port = qt.QComboBox()
    destination_binding = qt.QComboBox()
    destination_port = qt.QComboBox()
    connection_choices.addWidget(source_binding, 1)
    connection_choices.addWidget(source_port, 2)
    connection_choices.addWidget(qt.QLabel("→"))
    connection_choices.addWidget(destination_binding, 1)
    connection_choices.addWidget(destination_port, 2)
    connection_layout.addLayout(connection_choices)
    connection_actions = qt.QHBoxLayout()
    add_connection_button = qt.QPushButton("Relier les ports")
    remove_connection_button = qt.QPushButton("Retirer la liaison")
    remove_connection_button.setObjectName("SecondaryButton")
    connection_actions.addWidget(add_connection_button)
    connection_actions.addWidget(remove_connection_button)
    connection_actions.addStretch(1)
    connection_layout.addLayout(connection_actions)
    chain_layout.addWidget(connection_group)

    correction_group = qt.QGroupBox("Corrections propres au numéro de série")
    correction_layout = qt.QVBoxLayout(correction_group)
    correction_table = qt.QTableWidget(0, 3)
    _configure_station_table(qt, correction_table, ("Matériel", "Traitement", "Correction"))
    correction_table.setMaximumHeight(120)
    correction_layout.addWidget(correction_table)
    correction_choices = qt.QHBoxLayout()
    correction_binding = qt.QComboBox()
    characterization_selector = qt.QComboBox()
    add_correction_button = qt.QPushButton("Utiliser cette correction")
    remove_correction_button = qt.QPushButton("Retirer")
    remove_correction_button.setObjectName("SecondaryButton")
    correction_choices.addWidget(correction_binding, 1)
    correction_choices.addWidget(characterization_selector, 3)
    correction_choices.addWidget(add_correction_button)
    correction_choices.addWidget(remove_correction_button)
    correction_layout.addLayout(correction_choices)
    chain_layout.addWidget(correction_group)

    readiness_group = qt.QGroupBox("Contrôle avant câblage")
    readiness_layout = qt.QVBoxLayout(readiness_group)
    readiness_result = qt.QPlainTextEdit()
    readiness_result.setReadOnly(True)
    readiness_result.setMaximumHeight(115)
    readiness_result.setPlainText(
        "Le contrôle vérifiera le montage, les ports, l’état métrologique et les corrections sélectionnées."
    )
    readiness_layout.addWidget(readiness_result)
    workflow_actions = qt.QHBoxLayout()
    save_button = qt.QPushButton("Sauvegarder le brouillon")
    check_button = qt.QPushButton("Contrôler")
    check_button.setObjectName("SecondaryButton")
    ready_button = qt.QPushButton("Prêt à câbler")
    derive_button = qt.QPushButton("Nouvelle révision")
    derive_button.setObjectName("SecondaryButton")
    workflow_actions.addWidget(save_button)
    workflow_actions.addWidget(check_button)
    workflow_actions.addStretch(1)
    workflow_actions.addWidget(derive_button)
    workflow_actions.addWidget(ready_button)
    readiness_layout.addLayout(workflow_actions)
    chain_layout.addWidget(readiness_group)
    chain_layout.addStretch(1)
    splitter.addWidget(chain_pane)
    splitter.setSizes([470, 800])

    def fail(error: Exception) -> None:
        message = str(error)
        if isinstance(error, LocalAgentError):
            message = error.message
            details = error.details.get("readiness")
            if isinstance(details, dict):
                show_readiness(details)
        qt.QMessageBox.critical(root, "EMC Locus", message)

    def show_readiness(readiness: dict[str, Any]) -> None:
        readiness_result.setPlainText("\n".join(readiness_lines(readiness)))
        readiness_result.setObjectName(
            "StationReadinessReady" if readiness.get("ready") else "StationReadinessBlocked"
        )
        if hasattr(readiness_result, "style"):
            readiness_result.style().unpolish(readiness_result)
            readiness_result.style().polish(readiness_result)

    def binding_label(binding: dict[str, Any]) -> str:
        return str(binding.get("role_label") or binding.get("asset_id") or "Matériel")

    def active_definition() -> dict[str, Any] | None:
        value = state.get("definition")
        return value if isinstance(value, dict) else None

    def set_dirty() -> None:
        state["dirty"] = True
        update_summary()
        update_controls()

    def update_summary() -> None:
        revision = state.get("revision")
        definition = active_definition()
        if not isinstance(revision, dict) or definition is None:
            summary.setText("Aucun montage ouvert. Créez un montage pour sélectionner les matériels réels.")
            return
        status = {
            "draft": "Brouillon",
            "ready": "Prêt à câbler",
            "superseded": "Remplacé",
        }.get(str(revision.get("status")), str(revision.get("status", "")))
        dirty = " · modifications non sauvegardées" if state["dirty"] else ""
        location_label = definition.get(
            "laboratory_location_label",
            definition.get("station_label", ""),
        )
        summary.setText(
            f"{definition.get('label', 'Montage')} · {location_label} · "
            f"{definition.get('planned_use_on', '')} · {status}{dirty}"
        )

    def update_controls() -> None:
        revision = state.get("revision")
        definition = active_definition()
        draft = isinstance(revision, dict) and revision.get("status") == "draft"
        has_definition = definition is not None
        for widget in (
            instrument_selector,
            role_input,
            add_material_button,
            remove_material_button,
            source_binding,
            source_port,
            destination_binding,
            destination_port,
            add_connection_button,
            remove_connection_button,
            correction_binding,
            characterization_selector,
            add_correction_button,
            remove_correction_button,
            save_button,
        ):
            widget.setEnabled(bool(draft))
        check_button.setEnabled(has_definition and not state["dirty"])
        ready_button.setEnabled(bool(draft))
        derive_button.setEnabled(
            isinstance(revision, dict) and revision.get("status") in {"ready", "superseded"}
        )

    def refresh_materials() -> None:
        definition = active_definition() or {}
        bindings = definition.get("asset_bindings", [])
        by_asset = {item.get("asset_id"): item for item in state["instruments"]}
        material_table.setRowCount(len(bindings))
        for row, binding in enumerate(bindings):
            instrument = by_asset.get(binding.get("asset_id"), {})
            manufacturer = str(instrument.get("manufacturer", "")).strip()
            model = str(instrument.get("model", "")).strip()
            identity = " · ".join(
                part for part in (
                    model or manufacturer,
                    f"S/N {instrument.get('serial_number')}"
                    if instrument.get("serial_number")
                    else "",
                ) if part
            )
            status = _instrument_station_status(instrument)
            values = (binding_label(binding), f"{identity}\n{status}")
            for column, value in enumerate(values):
                item = qt.QTableWidgetItem(value)
                tooltip = value.replace("\n", " · ")
                if column == 1 and manufacturer and manufacturer not in identity:
                    tooltip = f"{manufacturer} · {tooltip}"
                item.setToolTip(tooltip)
                material_table.setItem(row, column, item)
            material_table.setRowHeight(row, 62)
        material_table.setColumnWidth(0, 90)
        material_table.horizontalHeader().setStretchLastSection(True)
        refresh_binding_choices()

    def refill_binding_combo(combo: Any, bindings: list[dict[str, Any]]) -> None:
        current = combo.currentData()
        combo.blockSignals(True)
        combo.clear()
        for binding in bindings:
            combo.addItem(binding_label(binding), binding.get("binding_id"))
        index = combo.findData(current)
        if index >= 0:
            combo.setCurrentIndex(index)
        combo.blockSignals(False)

    def refresh_binding_choices() -> None:
        bindings = (active_definition() or {}).get("asset_bindings", [])
        for combo in (source_binding, destination_binding, correction_binding):
            refill_binding_combo(combo, bindings)
        if (
            destination_binding.count() > 1
            and destination_binding.currentData() == source_binding.currentData()
        ):
            destination_binding.setCurrentIndex(1)
        refresh_source_ports()
        refresh_destination_ports()
        refresh_characterizations()

    def binding_for(binding_id: str | None) -> dict[str, Any] | None:
        return next(
            (
                binding
                for binding in (active_definition() or {}).get("asset_bindings", [])
                if binding.get("binding_id") == binding_id
            ),
            None,
        )

    def ports_for(binding_id: str | None) -> list[dict[str, Any]]:
        binding = binding_for(binding_id)
        if binding is None:
            return []
        key = (
            binding.get("equipment_model_id"),
            binding.get("equipment_model_revision_id"),
        )
        if key not in state["ports"]:
            response = client.get_equipment_model_revision(str(key[0]), str(key[1]))
            state["ports"][key] = model_signal_ports(response)
        return state["ports"][key]

    def refill_port_combo(combo: Any, binding_combo: Any, directions: set[str]) -> None:
        combo.clear()
        for port in ports_for(binding_combo.currentData()):
            if port.get("directionality") in directions:
                combo.addItem(port_display_label(port), port.get("port_id"))

    def refresh_source_ports(*_args: Any) -> None:
        try:
            refill_port_combo(source_port, source_binding, {"output", "bidirectional", "through"})
        except Exception as error:  # noqa: BLE001 - Qt boundary.
            fail(error)

    def refresh_destination_ports(*_args: Any) -> None:
        try:
            refill_port_combo(destination_port, destination_binding, {"input", "bidirectional", "through"})
        except Exception as error:  # noqa: BLE001 - Qt boundary.
            fail(error)

    def refresh_connections() -> None:
        definition = active_definition() or {}
        connections = definition.get("connections", [])
        labels = {
            binding.get("binding_id"): binding_label(binding)
            for binding in definition.get("asset_bindings", [])
        }
        connection_table.setRowCount(len(connections))
        for row, connection in enumerate(connections):
            values = (
                str(connection.get("label", "Liaison")),
                f"{labels.get(connection.get('from', {}).get('binding_id'), 'Matériel')} · {connection.get('from', {}).get('port_id', '')}",
                f"{labels.get(connection.get('to', {}).get('binding_id'), 'Matériel')} · {connection.get('to', {}).get('port_id', '')}",
            )
            for column, value in enumerate(values):
                connection_table.setItem(row, column, qt.QTableWidgetItem(value))
        connection_table.resizeColumnsToContents()
        connection_table.horizontalHeader().setStretchLastSection(True)

    def refresh_characterizations(*_args: Any) -> None:
        characterization_selector.clear()
        binding = binding_for(correction_binding.currentData())
        definition = active_definition()
        if binding is None or definition is None:
            return
        asset_id = str(binding.get("asset_id"))
        try:
            if asset_id not in state["characterizations"]:
                state["characterizations"][asset_id] = client.list_asset_characterizations(asset_id)
            choices = applicable_characterizations(
                state["characterizations"][asset_id],
                str(definition.get("planned_use_on", "")),
            )
            for characterization in choices:
                characterization_selector.addItem(
                    characterization_display_label(characterization),
                    characterization,
                )
            selected = next(
                (
                    correction
                    for correction in definition.get("correction_selections", [])
                    if correction.get("binding_id") == binding.get("binding_id")
                ),
                None,
            )
            if isinstance(selected, dict):
                for index in range(characterization_selector.count()):
                    candidate = characterization_selector.itemData(index)
                    if (
                        isinstance(candidate, dict)
                        and candidate.get("characterization_id")
                        == selected.get("characterization_id")
                    ):
                        characterization_selector.setCurrentIndex(index)
                        break
        except Exception as error:  # noqa: BLE001 - Qt boundary.
            fail(error)

    def refresh_corrections() -> None:
        definition = active_definition() or {}
        corrections = definition.get("correction_selections", [])
        labels = {
            binding.get("binding_id"): binding_label(binding)
            for binding in definition.get("asset_bindings", [])
        }
        correction_table.setRowCount(len(corrections))
        for row, correction in enumerate(corrections):
            values = (
                labels.get(correction.get("binding_id"), "Matériel"),
                _station_correction_kind_label(str(correction.get("correction_kind", ""))),
                str(correction.get("label", "Correction")),
            )
            for column, value in enumerate(values):
                correction_table.setItem(row, column, qt.QTableWidgetItem(value))
        correction_table.resizeColumnsToContents()
        correction_table.horizontalHeader().setStretchLastSection(True)

    def open_aggregate(aggregate: dict[str, Any]) -> None:
        revision = current_station_revision(aggregate)
        state["aggregate"] = aggregate
        state["revision"] = revision
        state["definition"] = editable_definition(revision) if revision else None
        state["dirty"] = False
        state["ports"] = {}
        state["characterizations"] = {}
        refresh_materials()
        refresh_connections()
        refresh_corrections()
        if revision and isinstance(revision.get("readiness"), dict):
            show_readiness(revision["readiness"])
        update_summary()
        update_controls()

    def open_selected(*_args: Any) -> None:
        setup_id = setup_selector.currentData()
        if not setup_id:
            state["aggregate"] = None
            state["revision"] = None
            state["definition"] = None
            refresh_materials()
            refresh_connections()
            refresh_corrections()
            update_summary()
            update_controls()
            return
        try:
            response = client.get_station_setup(str(setup_id))
            open_aggregate(response["station_setup"])
        except Exception as error:  # noqa: BLE001 - Qt boundary.
            fail(error)

    def refresh_all(target_setup_id: str | None = None) -> None:
        try:
            setups_response = client.list_station_setups()
            instruments_response = client.list_metrology_instruments()
            state["setups"] = setups_response.get("station_setups", [])
            state["instruments"] = eligible_station_instruments(instruments_response)
            selected_location_id = new_station.currentData()
            locations: dict[str, str] = {}
            for aggregate in state["setups"]:
                revision = current_station_revision(aggregate)
                definition = revision.get("definition") if isinstance(revision, dict) else None
                if not isinstance(definition, dict):
                    continue
                location_id = str(definition.get("laboratory_location_id", "")).strip()
                location_label = str(
                    definition.get("laboratory_location_label", "")
                ).strip()
                if location_id and location_label:
                    locations[location_id] = location_label
            new_station.blockSignals(True)
            new_station.clear()
            new_station.addItem("Nouveau poste…", None)
            for location_id, location_label in sorted(
                locations.items(), key=lambda item: item[1].casefold()
            ):
                new_station.addItem(location_label, location_id)
            location_index = new_station.findData(selected_location_id)
            new_station.setCurrentIndex(location_index if location_index >= 0 else 0)
            new_station.blockSignals(False)
            new_station_label.setVisible(new_station.currentData() is None)
            instrument_selector.clear()
            for instrument in state["instruments"]:
                instrument_selector.addItem(instrument_display_label(instrument), instrument)
            setup_selector.blockSignals(True)
            setup_selector.clear()
            for aggregate in state["setups"]:
                identity = aggregate.get("identity", {})
                setup_selector.addItem(identity.get("label", "Montage"), identity.get("setup_id"))
            setup_selector.blockSignals(False)
            target = target_setup_id or setup_selector.currentData()
            index = setup_selector.findData(target)
            if index < 0 and setup_selector.count() > 0:
                index = 0
            if index >= 0:
                setup_selector.setCurrentIndex(index)
                open_selected()
            else:
                open_selected()
        except Exception as error:  # noqa: BLE001 - Qt boundary.
            fail(error)

    def create_setup(*_args: Any) -> None:
        existing_location_id = new_station.currentData()
        location_label = (
            new_station.currentText().strip()
            if existing_location_id
            else new_station_label.text().strip()
        )
        if not new_label.text().strip() or not location_label:
            qt.QMessageBox.warning(root, "EMC Locus", "Renseignez le nom du montage et le poste.")
            return
        setup_id = f"SETUP-{uuid.uuid4().hex[:12].upper()}"
        location_id = str(existing_location_id) if existing_location_id else (
            f"LAB-LOCATION-{uuid.uuid4().hex[:12].upper()}"
        )
        try:
            client.create_station_setup(
                setup_id=setup_id,
                label=new_label.text().strip(),
                laboratory_location_id=location_id,
                laboratory_location_label=location_label,
                planned_use_on=new_date.text().strip(),
                execution_mode=str(new_mode.currentData()),
                actor="test.technician",
                reason="préparation du montage de mesure",
            )
            create_group.setVisible(False)
            new_label.clear()
            new_station_label.clear()
            refresh_all(setup_id)
            on_completed("Montage créé")
        except Exception as error:  # noqa: BLE001 - Qt boundary.
            fail(error)

    def add_material(*_args: Any) -> None:
        definition = active_definition()
        instrument = instrument_selector.currentData()
        if definition is None or not isinstance(instrument, dict):
            return
        if any(
            binding.get("asset_id") == instrument.get("asset_id")
            for binding in definition.get("asset_bindings", [])
        ):
            qt.QMessageBox.warning(root, "EMC Locus", "Ce matériel est déjà présent dans le montage.")
            return
        try:
            definition.setdefault("asset_bindings", []).append(
                build_asset_binding(instrument, role_input.text())
            )
            role_input.clear()
            set_dirty()
            refresh_materials()
            refresh_connections()
            refresh_corrections()
        except Exception as error:  # noqa: BLE001 - Qt boundary.
            fail(error)

    def remove_material(*_args: Any) -> None:
        definition = active_definition()
        row = material_table.currentRow()
        if definition is None or row < 0 or row >= len(definition.get("asset_bindings", [])):
            return
        binding_id = definition["asset_bindings"][row]["binding_id"]
        del definition["asset_bindings"][row]
        definition["connections"] = [
            connection
            for connection in definition.get("connections", [])
            if binding_id
            not in {
                connection.get("from", {}).get("binding_id"),
                connection.get("to", {}).get("binding_id"),
            }
        ]
        definition["correction_selections"] = [
            correction
            for correction in definition.get("correction_selections", [])
            if correction.get("binding_id") != binding_id
        ]
        set_dirty()
        refresh_materials()
        refresh_connections()
        refresh_corrections()

    def add_connection(*_args: Any) -> None:
        definition = active_definition()
        source_binding_id = source_binding.currentData()
        destination_binding_id = destination_binding.currentData()
        if definition is None or not all(
            (source_binding_id, source_port.currentData(), destination_binding_id, destination_port.currentData())
        ):
            return
        if source_binding_id == destination_binding_id:
            qt.QMessageBox.warning(root, "EMC Locus", "Une liaison doit relier deux matériels distincts.")
            return
        source = binding_for(source_binding_id) or {}
        destination = binding_for(destination_binding_id) or {}
        definition.setdefault("connections", []).append(
            {
                "connection_id": f"link-{uuid.uuid4().hex[:10]}",
                "label": f"{binding_label(source)} vers {binding_label(destination)}",
                "from": {"binding_id": source_binding_id, "port_id": source_port.currentData()},
                "to": {"binding_id": destination_binding_id, "port_id": destination_port.currentData()},
            }
        )
        set_dirty()
        refresh_connections()

    def remove_connection(*_args: Any) -> None:
        definition = active_definition()
        row = connection_table.currentRow()
        if definition is None or row < 0 or row >= len(definition.get("connections", [])):
            return
        del definition["connections"][row]
        set_dirty()
        refresh_connections()

    def add_correction(*_args: Any) -> None:
        definition = active_definition()
        binding_id = correction_binding.currentData()
        characterization = characterization_selector.currentData()
        if definition is None or not binding_id or not isinstance(characterization, dict):
            return
        kind = characterization.get("characterization_kind")
        definition["correction_selections"] = [
            correction
            for correction in definition.get("correction_selections", [])
            if (correction.get("binding_id"), correction.get("correction_kind"))
            != (binding_id, kind)
        ]
        definition["correction_selections"].append(
            build_correction_selection(str(binding_id), characterization)
        )
        set_dirty()
        refresh_corrections()

    def remove_correction(*_args: Any) -> None:
        definition = active_definition()
        row = correction_table.currentRow()
        if definition is None or row < 0 or row >= len(definition.get("correction_selections", [])):
            return
        del definition["correction_selections"][row]
        set_dirty()
        refresh_corrections()

    def save_draft(show_message: bool = True) -> bool:
        revision = state.get("revision")
        definition = active_definition()
        aggregate = state.get("aggregate")
        if not isinstance(revision, dict) or definition is None or not isinstance(aggregate, dict):
            return False
        if revision.get("status") != "draft":
            return True
        try:
            result = client.replace_station_setup_draft(
                setup_id=str(aggregate["identity"]["setup_id"]),
                revision_id=str(revision["revision_id"]),
                expected_definition_checksum=str(revision["definition_checksum"]),
                definition=deepcopy(definition),
                actor="test.technician",
                reason="mise à jour du montage de mesure",
            )
            open_aggregate(result["station_setup"])
            if show_message:
                qt.QMessageBox.information(root, "EMC Locus", "Le brouillon du montage est sauvegardé.")
            return True
        except Exception as error:  # noqa: BLE001 - Qt boundary.
            fail(error)
            return False

    def assess_setup(*_args: Any) -> None:
        revision = state.get("revision")
        aggregate = state.get("aggregate")
        if state["dirty"]:
            qt.QMessageBox.warning(root, "EMC Locus", "Sauvegardez le brouillon avant de le contrôler.")
            return
        if not isinstance(revision, dict) or not isinstance(aggregate, dict):
            return
        try:
            response = client.assess_station_setup(
                str(aggregate["identity"]["setup_id"]),
                str(revision["revision_id"]),
            )
            show_readiness(response["readiness"])
        except Exception as error:  # noqa: BLE001 - Qt boundary.
            fail(error)

    def mark_ready(*_args: Any) -> None:
        if state["dirty"] and not save_draft(show_message=False):
            return
        revision = state.get("revision")
        aggregate = state.get("aggregate")
        if not isinstance(revision, dict) or not isinstance(aggregate, dict):
            return
        try:
            result = client.mark_station_setup_ready(
                setup_id=str(aggregate["identity"]["setup_id"]),
                revision_id=str(revision["revision_id"]),
                expected_definition_checksum=str(revision["definition_checksum"]),
                actor="test.technician",
                reason="contrôle terminé avant câblage",
            )
            open_aggregate(result["station_setup"])
            qt.QMessageBox.information(root, "EMC Locus", "Le montage est prêt à câbler.")
            on_completed("Montage prêt à câbler")
        except Exception as error:  # noqa: BLE001 - Qt boundary.
            fail(error)

    def derive_revision(*_args: Any) -> None:
        revision = state.get("revision")
        aggregate = state.get("aggregate")
        if not isinstance(revision, dict) or not isinstance(aggregate, dict):
            return
        try:
            result = client.derive_station_setup_draft(
                setup_id=str(aggregate["identity"]["setup_id"]),
                source_revision_id=str(revision["revision_id"]),
                actor="test.technician",
                reason="adapter le montage pour une nouvelle utilisation",
            )
            open_aggregate(result["station_setup"])
        except Exception as error:  # noqa: BLE001 - Qt boundary.
            fail(error)

    setup_selector.currentIndexChanged.connect(open_selected)
    refresh_button.clicked.connect(lambda _checked=False: refresh_all(setup_selector.currentData()))
    new_button.clicked.connect(lambda _checked=False: create_group.setVisible(True))
    new_station.currentIndexChanged.connect(
        lambda _index: new_station_label.setVisible(new_station.currentData() is None)
    )
    cancel_create_button.clicked.connect(lambda _checked=False: create_group.setVisible(False))
    create_button.clicked.connect(create_setup)
    add_material_button.clicked.connect(add_material)
    remove_material_button.clicked.connect(remove_material)
    source_binding.currentIndexChanged.connect(refresh_source_ports)
    destination_binding.currentIndexChanged.connect(refresh_destination_ports)
    correction_binding.currentIndexChanged.connect(refresh_characterizations)
    add_connection_button.clicked.connect(add_connection)
    remove_connection_button.clicked.connect(remove_connection)
    add_correction_button.clicked.connect(add_correction)
    remove_correction_button.clicked.connect(remove_correction)
    save_button.clicked.connect(lambda _checked=False: save_draft())
    check_button.clicked.connect(assess_setup)
    ready_button.clicked.connect(mark_ready)
    derive_button.clicked.connect(derive_revision)

    refresh_all()
    return root


def _configure_station_table(
    qt: QtBindings,
    table: Any,
    columns: tuple[str, ...],
) -> None:
    table.setAlternatingRowColors(True)
    table.setSelectionBehavior(qt.QAbstractItemView.SelectionBehavior.SelectRows)
    table.setSelectionMode(qt.QAbstractItemView.SelectionMode.SingleSelection)
    table.setEditTriggers(qt.QAbstractItemView.EditTrigger.NoEditTriggers)
    table.verticalHeader().setVisible(False)
    table.setHorizontalHeaderLabels(list(columns))
    table.horizontalHeader().setStretchLastSection(True)


def _instrument_station_status(instrument: dict[str, Any]) -> str:
    serviceability = {
        "usable": "En service",
        "restricted": "Restreint",
        "out_of_service": "Hors service",
        "retired": "Retiré",
    }.get(str(instrument.get("serviceability_status")), "À contrôler")
    calibration = instrument.get("latest_calibration") or instrument.get("latest_calibration_event")
    if instrument.get("calibration_requirement") == "not_required":
        return f"{serviceability} · étalonnage non requis"
    if isinstance(calibration, dict) and calibration.get("due_at"):
        return f"{serviceability} · échéance {calibration['due_at']}"
    return f"{serviceability} · étalonnage absent"


def _station_correction_kind_label(value: str) -> str:
    return {
        "time_conversion": "Conversion temporelle",
        "frequency_response": "Réponse fréquentielle",
    }.get(value, value.replace("_", " "))


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
        from PySide6.QtCore import QObject, QRunnable, Qt, QThreadPool, QTimer, Signal, Slot
        from PySide6.QtGui import QFont
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
            QSplitter,
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
        QTimer=QTimer,
        QAbstractItemView=QAbstractItemView,
        QApplication=QApplication,
        QComboBox=QComboBox,
        QFormLayout=QFormLayout,
        QFont=QFont,
        QGroupBox=QGroupBox,
        QHBoxLayout=QHBoxLayout,
        QLabel=QLabel,
        QLineEdit=QLineEdit,
        QMainWindow=QMainWindow,
        QMessageBox=QMessageBox,
        QPlainTextEdit=QPlainTextEdit,
        QPushButton=QPushButton,
        QScrollArea=QScrollArea,
        QSplitter=QSplitter,
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
    QLabel#StationSummary {
        background: #edf4f0;
        border-left: 4px solid #2f6f5e;
        color: #20342b;
        padding: 8px 10px;
        font-size: 13px;
        font-weight: 600;
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
    QPlainTextEdit#StationReadinessReady {
        background: #eef8f1;
        border: 1px solid #9cc8a8;
        color: #174627;
    }
    QPlainTextEdit#StationReadinessBlocked {
        background: #fff5ec;
        border: 1px solid #dda67a;
        color: #6b2f10;
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
    QPushButton#SecondaryButton {
        background: #eef1ed;
        border: 1px solid #c5cbc4;
        color: #26342d;
    }
    QPushButton#SecondaryButton:hover {
        background: #e1e7e2;
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
    QSplitter::handle {
        background: #dfe4dd;
        width: 2px;
    }
    """


if __name__ == "__main__":
    raise SystemExit(run())
