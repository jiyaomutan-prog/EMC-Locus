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
            title="Simulated instrument runtime",
            objective=(
                "Create a simulated instrument adapter so measurement workflows "
                "can execute repeatable command and observation sequences."
            ),
            workstreams=(Workstream.INSTRUMENT_CONTROL, Workstream.STORAGE),
            expected_outputs=(
                "simulated driver interface",
                "command log",
                "measurement-run fixture",
            ),
        ),
        SessionPlan(
            title="Signal execution engine",
            objective=(
                "Start executing approved signal-processing graph nodes for "
                "FFT, channel math, event timing, and raw-to-result lineage."
            ),
            workstreams=(Workstream.INSTRUMENT_CONTROL, Workstream.SIGNAL, Workstream.STORAGE),
            expected_outputs=(
                "deterministic FFT fixture",
                "channel math execution",
                "result artifact model",
            ),
        ),
    ]
