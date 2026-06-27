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
            title="GUI local write actions",
            objective=(
                "Replace the remaining fixture-only GUI actions with local "
                "Python-backed write actions and a refresh path for the "
                "generated bootstrap data."
            ),
            workstreams=(Workstream.UI, Workstream.STORAGE, Workstream.QUALITY),
            expected_outputs=(
                "project stage write action",
                "bootstrap regeneration command",
                "operator workflow smoke test",
            ),
        ),
        SessionPlan(
            title="Instrument IO adapters",
            objective=(
                "Add the first guarded IO-backed implementation behind one "
                "transport adapter skeleton."
            ),
            workstreams=(Workstream.INSTRUMENT_CONTROL, Workstream.QUALITY),
            expected_outputs=(
                "IO adapter implementation",
                "simulated baseline comparison",
                "unavailable hardware test path",
            ),
        ),
    ]
