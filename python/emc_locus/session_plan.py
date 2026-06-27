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
    UI = "ui"


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
            title="Serial and VISA IO adapters",
            objective=(
                "Expand the guarded instrument runtime with the next concrete "
                "IO-backed adapter after the TCP/IP implementation."
            ),
            workstreams=(Workstream.INSTRUMENT_CONTROL, Workstream.QUALITY),
            expected_outputs=(
                "serial or VISA adapter implementation",
                "simulated baseline comparison",
                "unavailable hardware test path",
            ),
        ),
    ]
