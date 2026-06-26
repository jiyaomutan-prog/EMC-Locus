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
            title="Update catalog persistence",
            objective=(
                "Persist signed update bundles and install records in the "
                "update catalog repository."
            ),
            workstreams=(Workstream.STORAGE, Workstream.QUALITY),
            expected_outputs=(
                "update catalog adapter",
                "bundle insert/query APIs",
                "install record smoke test",
            ),
        ),
        SessionPlan(
            title="Optimized signal resampling",
            objective=(
                "Add optimized FFT and interpolation-based resampling behind "
                "the deterministic signal execution fixtures."
            ),
            workstreams=(Workstream.SIGNAL, Workstream.QUALITY),
            expected_outputs=(
                "FFT backend boundary",
                "interpolation resampling fixture",
                "comparison tests",
            ),
        ),
    ]
