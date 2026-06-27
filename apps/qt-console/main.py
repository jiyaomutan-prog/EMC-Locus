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


BOOTSTRAP_PREFIX = "window.EMC_LOCUS_BOOTSTRAP = "
REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPOSITORY_ROOT / "python"
if str(PYTHON_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))

from emc_locus.qt_console_data import build_console_bootstrap_from_repositories
from emc_locus.qt_console_models import TableViewModel, build_console_view_model


@dataclass(frozen=True)
class QtBindings:
    """PySide6 objects used by the console, loaded lazily."""

    Qt: Any
    QAbstractItemView: Any
    QApplication: Any
    QHBoxLayout: Any
    QLabel: Any
    QMainWindow: Any
    QPushButton: Any
    QStatusBar: Any
    QTabWidget: Any
    QTableWidget: Any
    QTableWidgetItem: Any
    QVBoxLayout: Any
    QWidget: Any


def load_bootstrap_js(path: Path) -> dict[str, Any]:
    """Load the GUI bootstrap payload generated for the static prototype."""

    text = path.read_text(encoding="utf-8").strip()
    if not text.startswith(BOOTSTRAP_PREFIX):
        raise ValueError(f"unsupported bootstrap format: {path}")

    payload = text[len(BOOTSTRAP_PREFIX) :]
    if payload.endswith(";"):
        payload = payload[:-1]

    data = json.loads(payload)
    if not isinstance(data, dict):
        raise ValueError("bootstrap payload must be a JSON object")
    return data


def run(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--bootstrap",
        type=Path,
        default=None,
        help="Path to a generated bootstrap.js file.",
    )
    parser.add_argument("--migrations-root", type=Path, default=REPOSITORY_ROOT / "storage" / "sqlite")
    parser.add_argument("--projects-db", type=Path)
    parser.add_argument("--metrology-db", type=Path)
    parser.add_argument("--test-definitions-db", type=Path)
    parser.add_argument("--measurement-data-db", type=Path)
    parser.add_argument("--update-catalog-db", type=Path)
    args = parser.parse_args(argv)
    data = _load_console_data(args)
    view_model = build_console_view_model(data)
    qt = _load_qt()

    application = qt.QApplication([])
    window = qt.QMainWindow()
    window.setWindowTitle("EMC Locus")
    window.resize(1320, 820)

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
    layout.addLayout(header)

    tabs = qt.QTabWidget()
    for table_model in view_model.tables:
        tabs.addTab(_table(qt, table_model), table_model.tab_label)
    layout.addWidget(tabs, 1)

    window.setCentralWidget(root)
    window.setStatusBar(qt.QStatusBar())
    window.statusBar().showMessage(_status_message(args))
    window.setStyleSheet(_stylesheet())
    window.show()
    return application.exec()


def _load_console_data(args: argparse.Namespace) -> dict[str, Any]:
    if _has_repository_paths(args):
        return build_console_bootstrap_from_repositories(
            migrations_root=args.migrations_root,
            projects_db=args.projects_db,
            metrology_db=args.metrology_db,
            test_definitions_db=args.test_definitions_db,
            measurement_data_db=args.measurement_data_db,
            update_catalog_db=args.update_catalog_db,
        )

    bootstrap = args.bootstrap or REPOSITORY_ROOT / "apps" / "gui-shell" / "bootstrap.js"
    return load_bootstrap_js(bootstrap)


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
    if _has_repository_paths(args):
        return "Donnees chargees depuis les depots SQLite locaux"

    bootstrap = args.bootstrap or REPOSITORY_ROOT / "apps" / "gui-shell" / "bootstrap.js"
    return f"Bootstrap local: {bootstrap}"


def _load_qt() -> QtBindings:
    try:
        from PySide6.QtCore import Qt
        from PySide6.QtWidgets import (
            QAbstractItemView,
            QApplication,
            QHBoxLayout,
            QLabel,
            QMainWindow,
            QPushButton,
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
        QAbstractItemView=QAbstractItemView,
        QApplication=QApplication,
        QHBoxLayout=QHBoxLayout,
        QLabel=QLabel,
        QMainWindow=QMainWindow,
        QPushButton=QPushButton,
        QStatusBar=QStatusBar,
        QTabWidget=QTabWidget,
        QTableWidget=QTableWidget,
        QTableWidgetItem=QTableWidgetItem,
        QVBoxLayout=QVBoxLayout,
        QWidget=QWidget,
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
    """


if __name__ == "__main__":
    raise SystemExit(run())
