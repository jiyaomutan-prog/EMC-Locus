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
            title="Optimized signal ops",
            objective=(
                "Add optimized FFT/windowing and resampling execution behind "
                "the deterministic signal fixtures."
            ),
            workstreams=(Workstream.SIGNAL, Workstream.STORAGE),
            expected_outputs=(
                "window function model",
                "resampling fixture",
                "FFT comparison tests",
            ),
        ),
        SessionPlan(
            title="Signed update bundles",
            objective=(
                "Model signed offline update bundles with compatibility and "
                "rollback evidence."
            ),
            workstreams=(Workstream.STORAGE, Workstream.QUALITY),
            expected_outputs=(
                "update bundle model",
                "compatibility checks",
                "rollback metadata",
            ),
        ),
    ]
