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
            title="Measurement data retention policy",
            objective=(
                "Add policy hooks that protect immutable measurement datasets "
                "through retention status and reviewable deletion requests."
            ),
            workstreams=(Workstream.STORAGE, Workstream.QUALITY, Workstream.SIGNAL),
            expected_outputs=(
                "retention policy primitives",
                "SQLite retention evidence records",
                "immutable dataset smoke tests",
            ),
        ),
        SessionPlan(
            title="GUI service wiring",
            objective=(
                "Wire the static operator console shell to local Python "
                "repository services for projects, metrology, test "
                "definitions, measurement data, and update metadata."
            ),
            workstreams=(Workstream.UI, Workstream.STORAGE, Workstream.QUALITY),
            expected_outputs=(
                "local repository read API for the GUI",
                "fixture replacement path",
                "offline mode smoke test",
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
