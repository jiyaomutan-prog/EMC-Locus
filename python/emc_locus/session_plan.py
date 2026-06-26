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
            title="Local repository snapshots",
            objective=(
                "Define local snapshot metadata, compatibility checks, and "
                "offline validation rules for split repositories."
            ),
            workstreams=(Workstream.STORAGE, Workstream.QUALITY),
            expected_outputs=(
                "snapshot metadata model",
                "compatibility rule set",
                "offline validation checklist",
            ),
        ),
        SessionPlan(
            title="Measurement-run readiness gate",
            objective=(
                "Connect metrology readiness reports to the first "
                "measurement-run planning model."
            ),
            workstreams=(Workstream.DOMAIN, Workstream.METROLOGY, Workstream.QUALITY),
            expected_outputs=(
                "measurement-run entity",
                "equipment selection model",
                "tests for accepted and blocked runs",
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
    ]
