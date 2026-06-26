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
            title="Persistent repository adapters",
            objective=(
                "Create the first SQLite-backed adapters for metrology and "
                "project records using the versioned migrations."
            ),
            workstreams=(Workstream.METROLOGY, Workstream.STORAGE, Workstream.QUALITY),
            expected_outputs=(
                "metrology adapter skeleton",
                "project adapter skeleton",
                "migration-backed smoke test",
            ),
        ),
        SessionPlan(
            title="Measurement execution binding",
            objective=(
                "Connect accepted measurement-run plans to simulated runtime "
                "execution and dataset evidence."
            ),
            workstreams=(Workstream.DOMAIN, Workstream.INSTRUMENT_CONTROL, Workstream.STORAGE),
            expected_outputs=(
                "execution fixture",
                "runtime-to-evidence link",
                "blocked execution tests",
            ),
        ),
    ]
