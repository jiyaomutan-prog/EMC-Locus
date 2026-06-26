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
        SessionPlan(
            title="Concrete transport adapters",
            objective=(
                "Add the first concrete VISA, TCP/IP, or serial adapter behind "
                "the tested transport boundary."
            ),
            workstreams=(Workstream.INSTRUMENT_CONTROL, Workstream.QUALITY),
            expected_outputs=(
                "hardware adapter skeleton",
                "timeout policy",
                "simulated conformance reuse",
            ),
        ),
    ]
