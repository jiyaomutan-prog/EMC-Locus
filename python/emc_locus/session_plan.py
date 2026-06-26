"""Planning primitives for recurring EMC Locus development sessions."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class Workstream(str, Enum):
    DOMAIN = "domain"
    METROLOGY = "metrology"
    INSTRUMENT_CONTROL = "instrument_control"
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
            title="Metrology registry",
            objective=(
                "Represent instruments, calibration records, and pre-run "
                "validity checks for measurement campaigns."
            ),
            workstreams=(Workstream.METROLOGY, Workstream.QUALITY),
            expected_outputs=(
                "instrument entity",
                "calibration validity rules",
                "pre-run equipment checklist",
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
