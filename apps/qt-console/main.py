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
from typing import Any


BOOTSTRAP_PREFIX = "window.EMC_LOCUS_BOOTSTRAP = "


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
        default=Path(__file__).resolve().parents[1] / "gui-shell" / "bootstrap.js",
        help="Path to a generated bootstrap.js file.",
    )
    args = parser.parse_args(argv)
    data = load_bootstrap_js(args.bootstrap)
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
    refresh = qt.QPushButton("Rafraichir")
    refresh.setEnabled(False)
    header.addWidget(title)
    header.addWidget(subtitle, 1)
    header.addWidget(refresh)
    layout.addLayout(header)

    tabs = qt.QTabWidget()
    tabs.addTab(_table(qt, "Projets", data.get("projects", [])), "Projets")
    tabs.addTab(_table(qt, "Instruments", data.get("instruments", [])), "Metrologie")
    tabs.addTab(_table(qt, "Methodes", data.get("methods", [])), "Methodes")
    tabs.addTab(_table(qt, "Donnees", data.get("datasets", [])), "Donnees")
    tabs.addTab(_table(qt, "Mises a jour", data.get("updates", [])), "Updates")
    layout.addWidget(tabs, 1)

    window.setCentralWidget(root)
    window.setStatusBar(qt.QStatusBar())
    window.statusBar().showMessage(f"Bootstrap local: {args.bootstrap}")
    window.setStyleSheet(_stylesheet())
    window.show()
    return application.exec()


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


def _table(qt: QtBindings, label: str, rows: Any) -> Any:
    normalized_rows = _normalize_rows(rows)
    column_count = max((len(row) for row in normalized_rows), default=1)
    table = qt.QTableWidget(len(normalized_rows), column_count)
    table.setAlternatingRowColors(True)
    table.setSelectionBehavior(qt.QAbstractItemView.SelectionBehavior.SelectRows)
    table.setEditTriggers(qt.QAbstractItemView.EditTrigger.NoEditTriggers)
    table.verticalHeader().setVisible(False)
    table.setHorizontalHeaderLabels([f"{label} {index + 1}" for index in range(column_count)])

    for row_index, row in enumerate(normalized_rows):
        for column_index, value in enumerate(row):
            item = qt.QTableWidgetItem(value)
            item.setFlags(item.flags() & ~qt.Qt.ItemFlag.ItemIsEditable)
            table.setItem(row_index, column_index, item)

    table.resizeColumnsToContents()
    table.horizontalHeader().setStretchLastSection(True)
    return table


def _normalize_rows(rows: Any) -> list[list[str]]:
    if not isinstance(rows, list):
        return []

    normalized: list[list[str]] = []
    for row in rows:
        if isinstance(row, dict):
            normalized.append([str(value) for value in row.values()])
        elif isinstance(row, list):
            normalized.append([str(value) for value in row])
        else:
            normalized.append([str(row)])
    return normalized


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
