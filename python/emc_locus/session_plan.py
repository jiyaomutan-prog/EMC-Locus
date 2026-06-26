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
            title="Report export bundle",
            objective=(
                "Model report export bundles with file references, checksums, "
                "and approval evidence."
            ),
            workstreams=(Workstream.REPORTING, Workstream.STORAGE, Workstream.QUALITY),
            expected_outputs=(
                "export bundle model",
                "checksum linkage",
                "approval evidence tests",
            ),
        ),
        SessionPlan(
            title="Adapter query APIs",
            objective=(
                "Expand SQLite adapters beyond smoke writes with read/query "
                "APIs for metrology and project records."
            ),
            workstreams=(Workstream.METROLOGY, Workstream.STORAGE, Workstream.QUALITY),
            expected_outputs=(
                "instrument lookup",
                "project lookup",
                "adapter smoke tests",
            ),
        ),
    ]
