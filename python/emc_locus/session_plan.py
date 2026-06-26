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
            title="Sync persistence adapters",
            objective=(
                "Persist synchronization conflict action plans and resolution "
                "evidence in the split repository storage layer."
            ),
            workstreams=(Workstream.STORAGE, Workstream.QUALITY),
            expected_outputs=(
                "sync persistence adapter",
                "resolution record APIs",
                "audit evidence smoke test",
            ),
        ),
        SessionPlan(
            title="Test definitions SQLite",
            objective=(
                "Add a first SQLite adapter for standards, methods, and "
                "processing templates in the test-definition domain."
            ),
            workstreams=(Workstream.STORAGE, Workstream.QUALITY),
            expected_outputs=(
                "test definition adapter",
                "method insert/query APIs",
                "processing template smoke test",
            ),
        ),
    ]
