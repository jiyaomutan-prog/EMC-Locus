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
            title="Measurement data records",
            objective=(
                "Connect accepted measurement-run plans to raw dataset records, "
                "checksums, and command-observation evidence."
            ),
            workstreams=(Workstream.DOMAIN, Workstream.INSTRUMENT_CONTROL, Workstream.STORAGE),
            expected_outputs=(
                "raw dataset record",
                "checksum value object",
                "run evidence linkage",
            ),
        ),
        SessionPlan(
            title="Signal execution engine",
            objective=(
                "Start executing approved signal-processing graph nodes for "
                "FFT, channel math, event timing, and raw-to-result lineage."
            ),
            workstreams=(Workstream.INSTRUMENT_CONTROL, Workstream.SIGNAL, Workstream.STORAGE),
            expected_outputs=(
                "deterministic FFT fixture",
                "channel math execution",
                "result artifact model",
            ),
        ),
    ]
