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
            title="Sync application services",
            objective=(
                "Add application-service primitives around split-repository "
                "synchronization conflicts."
            ),
            workstreams=(Workstream.STORAGE, Workstream.QUALITY),
            expected_outputs=(
                "sync service model",
                "conflict action plan",
                "audit-oriented tests",
            ),
        ),
        SessionPlan(
            title="Concrete transport adapters",
            objective=(
                "Add the first concrete VISA, TCP/IP, or serial transport "
                "adapter behind the tested transport boundary."
            ),
            workstreams=(Workstream.INSTRUMENT_CONTROL, Workstream.QUALITY),
            expected_outputs=(
                "hardware-adapter skeleton",
                "timeout policy",
                "simulated conformance reuse",
            ),
        ),
    ]
