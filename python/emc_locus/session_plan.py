"""Planning primitives for recurring EMC Locus development sessions."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class Workstream(str, Enum):
    DOMAIN = "domain"
    METROLOGY = "metrology"
    INSTRUMENT_CONTROL = "instrument_control"
    SIGNAL = "signal"
    STORAGE = "storage"
    REPORTING = "reporting"
    QUALITY = "quality"


@dataclass(frozen=True)
class SessionPlan:
    """A small, reviewable development objective for an automated session."""

    title: str
    objective: str
    workstreams: tuple[Workstream, ...]
    expected_outputs: tuple[str, ...]


def default_backlog() -> list[SessionPlan]:
    """Return the first useful development sessions for the project."""

    return [
        SessionPlan(
            title="Project lifecycle and audit events",
            objective=(
                "Model the project lifecycle from quotation to archive and "
                "record every controlled transition as an audit event."
            ),
            workstreams=(Workstream.DOMAIN, Workstream.QUALITY),
            expected_outputs=(
                "Rust lifecycle model",
                "audit event model",
                "unit tests for allowed and rejected transitions",
            ),
        ),
        SessionPlan(
            title="Metrology registry and calibration validity",
            objective=(
                "Represent instruments, status, calibration records, and "
                "pre-run validity checks in a metrology-first model."
            ),
            workstreams=(Workstream.METROLOGY, Workstream.QUALITY),
            expected_outputs=(
                "instrument entity",
                "instrument status rules",
                "calibration validity rules",
                "pre-run equipment checklist",
            ),
        ),
        SessionPlan(
            title="Simulated DAQ and signal graph",
            objective=(
                "Create deterministic time-series fixtures and a minimal "
                "processing graph for FFT, channel math, event timing, and "
                "raw-to-result lineage."
            ),
            workstreams=(Workstream.INSTRUMENT_CONTROL, Workstream.SIGNAL, Workstream.STORAGE),
            expected_outputs=(
                "simulated DAQ source",
                "signal-processing graph model",
                "lineage fixture",
            ),
        ),
        SessionPlan(
            title="Simulated instrument runtime",
            objective=(
                "Create a simulated instrument adapter so measurement workflows "
                "can be tested before real hardware integration."
            ),
            workstreams=(Workstream.INSTRUMENT_CONTROL, Workstream.STORAGE),
            expected_outputs=(
                "simulated driver interface",
                "command log",
                "measurement-run fixture",
            ),
        ),
        SessionPlan(
            title="Local repository snapshots",
            objective=(
                "Define the local snapshot and synchronization boundaries for "
                "metrology, test definitions, drivers, projects, measurement "
                "data, report templates, and update metadata."
            ),
            workstreams=(Workstream.STORAGE, Workstream.QUALITY),
            expected_outputs=(
                "snapshot metadata model",
                "sync direction rules",
                "offline validation checklist",
            ),
        ),
    ]
