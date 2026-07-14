"""Operator-facing helpers for physical measurement setup preparation.

These helpers only shape local-agent responses for Qt. Readiness and all write
invariants remain owned by the Rust local agent.
"""

from __future__ import annotations

from copy import deepcopy
from typing import Any
import uuid


READINESS_DIMENSION_LABELS = {
    "structure": "Montage",
    "asset_identity": "Matériel",
    "serviceability": "État de service",
    "calibration_validity": "Étalonnage",
    "missing_evidence": "Preuve métrologique",
    "nonconformance": "Non-conformité",
    "port_compatibility": "Connexion",
    "correction_validity": "Correction du signal",
}


def current_station_revision(aggregate: dict[str, Any]) -> dict[str, Any] | None:
    """Return the revision the technician can act on."""

    for key in ("active_draft_revision", "current_ready_revision", "latest_revision"):
        value = aggregate.get(key)
        if isinstance(value, dict):
            return value
    return None


def eligible_station_instruments(payload: dict[str, Any]) -> list[dict[str, Any]]:
    """Keep physical assets whose model revision can expose typed ports."""

    instruments = payload.get("instruments")
    if not isinstance(instruments, list):
        return []
    eligible = [
        instrument
        for instrument in instruments
        if isinstance(instrument, dict)
        and all(
            isinstance(instrument.get(field), str) and instrument[field].strip()
            for field in (
                "asset_id",
                "revision",
                "equipment_model_id",
                "equipment_model_revision_id",
                "equipment_model_checksum",
            )
        )
    ]
    return sorted(
        eligible,
        key=lambda item: (
            str(item.get("manufacturer", "")).casefold(),
            str(item.get("model", "")).casefold(),
            str(item.get("serial_number", "")).casefold(),
        ),
    )


def instrument_display_label(instrument: dict[str, Any]) -> str:
    manufacturer = str(instrument.get("manufacturer", "")).strip()
    model = str(instrument.get("model", "")).strip()
    serial = str(instrument.get("serial_number", "")).strip()
    asset_id = str(instrument.get("asset_id", "")).strip()
    identity = " ".join(part for part in (manufacturer, model) if part) or "Matériel"
    evidence = " · ".join(part for part in (asset_id, f"S/N {serial}" if serial else "") if part)
    return f"{identity} — {evidence}" if evidence else identity


def build_asset_binding(
    instrument: dict[str, Any],
    role_label: str,
    *,
    binding_id: str | None = None,
) -> dict[str, Any]:
    role = role_label.strip()
    if not role:
        raise ValueError("Le rôle du matériel dans le montage est obligatoire.")
    required = (
        "asset_id",
        "revision",
        "equipment_model_id",
        "equipment_model_revision_id",
        "equipment_model_checksum",
    )
    missing = [field for field in required if not str(instrument.get(field, "")).strip()]
    if missing:
        raise ValueError("Le matériel ne possède pas de modèle approuvé avec des ports typés.")
    return {
        "binding_id": binding_id or f"material-{uuid.uuid4().hex[:10]}",
        "role_label": role,
        "asset_id": instrument["asset_id"],
        "asset_revision": instrument["revision"],
        "equipment_model_id": instrument["equipment_model_id"],
        "equipment_model_revision_id": instrument["equipment_model_revision_id"],
        "equipment_model_checksum": instrument["equipment_model_checksum"],
    }


def model_signal_ports(payload: dict[str, Any]) -> list[dict[str, Any]]:
    revision = payload.get("revision")
    definition = revision.get("definition") if isinstance(revision, dict) else None
    ports = definition.get("signal_ports") if isinstance(definition, dict) else None
    if not isinstance(ports, list):
        return []
    return [port for port in ports if isinstance(port, dict) and port.get("port_id")]


def port_display_label(port: dict[str, Any]) -> str:
    label = str(port.get("label") or port.get("port_id") or "Port")
    details = [
        _direction_label(str(port.get("directionality", ""))),
        str(port.get("connector_type", "")).strip(),
        _frequency_span(port),
        f"{_number(port['impedance'])} Ω" if isinstance(port.get("impedance"), (int, float)) else "",
    ]
    visible = " · ".join(detail for detail in details if detail)
    return f"{label} — {visible}" if visible else label


def applicable_characterizations(
    payload: dict[str, Any], planned_use_on: str
) -> list[dict[str, Any]]:
    values = payload.get("characterizations")
    if not isinstance(values, list):
        return []
    return sorted(
        [
            value
            for value in values
            if isinstance(value, dict)
            and value.get("decision") == "conforming"
            and str(value.get("performed_on", "")) <= planned_use_on
            and str(value.get("valid_until", "")) >= planned_use_on
            and value.get("characterization_kind") in {"time_conversion", "frequency_response"}
        ],
        key=lambda item: (str(item.get("characterization_kind")), str(item.get("label"))),
    )


def characterization_display_label(characterization: dict[str, Any]) -> str:
    kind = {
        "time_conversion": "Conversion temporelle",
        "frequency_response": "Réponse fréquentielle",
    }.get(str(characterization.get("characterization_kind")), "Correction")
    label = str(characterization.get("label", "Correction mesurée"))
    valid_until = str(characterization.get("valid_until", ""))
    suffix = f" · valable jusqu’au {valid_until}" if valid_until else ""
    return f"{kind} — {label}{suffix}"


def build_correction_selection(
    binding_id: str,
    characterization: dict[str, Any],
    *,
    selection_id: str | None = None,
) -> dict[str, Any]:
    return {
        "selection_id": selection_id or f"correction-{uuid.uuid4().hex[:10]}",
        "binding_id": binding_id,
        "correction_kind": characterization["characterization_kind"],
        "characterization_id": characterization["characterization_id"],
        "characterization_checksum": characterization["definition_checksum"],
        "label": characterization["label"],
    }


def editable_definition(revision: dict[str, Any]) -> dict[str, Any]:
    value = revision.get("definition")
    if not isinstance(value, dict):
        raise ValueError("La définition du montage est absente de la réponse de l’agent.")
    return deepcopy(value)


def readiness_lines(readiness: dict[str, Any]) -> list[str]:
    issues = readiness.get("issues")
    if not isinstance(issues, list) or not issues:
        return ["Aucun blocage détecté. Le montage peut être déclaré prêt à câbler."]
    lines: list[str] = []
    for issue in issues:
        if not isinstance(issue, dict):
            continue
        severity = "BLOCAGE" if issue.get("severity") == "blocking" else "ATTENTION"
        dimension = READINESS_DIMENSION_LABELS.get(
            str(issue.get("dimension", "")), "Contrôle"
        )
        lines.append(f"{severity} · {dimension} · {issue.get('message', 'Contrôle requis')}")
    return lines


def _direction_label(value: str) -> str:
    return {
        "input": "entrée",
        "output": "sortie",
        "bidirectional": "bidirectionnel",
        "through": "traversant",
        "communication": "communication",
    }.get(value, value.replace("_", " "))


def _frequency_span(port: dict[str, Any]) -> str:
    minimum = port.get("frequency_min")
    maximum = port.get("frequency_max")
    if not isinstance(minimum, (int, float)) or not isinstance(maximum, (int, float)):
        return ""
    return f"{_frequency(minimum)} à {_frequency(maximum)}"


def _frequency(value: float) -> str:
    if abs(value) >= 1_000_000_000:
        return f"{_number(value / 1_000_000_000)} GHz"
    if abs(value) >= 1_000_000:
        return f"{_number(value / 1_000_000)} MHz"
    if abs(value) >= 1_000:
        return f"{_number(value / 1_000)} kHz"
    return f"{_number(value)} Hz"


def _number(value: float) -> str:
    return f"{value:g}"
