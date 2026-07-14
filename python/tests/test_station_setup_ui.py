import unittest

from emc_locus.station_setup_ui import (
    applicable_characterizations,
    build_asset_binding,
    build_correction_selection,
    characterization_display_label,
    eligible_station_instruments,
    port_display_label,
    readiness_lines,
)


class StationSetupUiTests(unittest.TestCase):
    def test_keeps_only_assets_with_pinned_model_revision(self) -> None:
        complete = instrument()
        incomplete = {**instrument(), "asset_id": "SA-002", "equipment_model_checksum": None}

        self.assertEqual(
            [item["asset_id"] for item in eligible_station_instruments({"instruments": [incomplete, complete]})],
            ["SA-001"],
        )

    def test_builds_traceable_binding_without_operator_entering_ids(self) -> None:
        binding = build_asset_binding(instrument(), "Câble RF", binding_id="cable")

        self.assertEqual(binding["asset_id"], "SA-001")
        self.assertEqual(binding["asset_revision"], "rev-asset")
        self.assertEqual(binding["equipment_model_revision_id"], "EQM-CABLE-rev-0001")

    def test_formats_rf_port_from_physical_characteristics(self) -> None:
        label = port_display_label(
            {
                "port_id": "RF_B",
                "label": "Connecteur RF B",
                "directionality": "through",
                "connector_type": "N",
                "frequency_min": 10_000_000,
                "frequency_max": 1_000_000_000,
                "impedance": 50,
            }
        )

        self.assertEqual(
            label,
            "Connecteur RF B — traversant · N · 10 MHz à 1 GHz · 50 Ω",
        )

    def test_filters_and_labels_serial_specific_frequency_response(self) -> None:
        valid = characterization("CHAR-VALID", "2027-01-01")
        expired = characterization("CHAR-OLD", "2026-01-01")

        choices = applicable_characterizations(
            {"characterizations": [expired, valid]}, "2026-07-15"
        )
        selection = build_correction_selection("cable", choices[0], selection_id="loss")

        self.assertEqual([choice["characterization_id"] for choice in choices], ["CHAR-VALID"])
        self.assertIn("Réponse fréquentielle", characterization_display_label(choices[0]))
        self.assertEqual(selection["correction_kind"], "frequency_response")
        self.assertEqual(selection["binding_id"], "cable")

    def test_translates_readiness_dimensions_for_operator(self) -> None:
        self.assertEqual(
            readiness_lines(
                {
                    "ready": False,
                    "issues": [
                        {
                            "severity": "blocking",
                            "dimension": "nonconformance",
                            "message": "Le dernier étalonnage est non conforme.",
                        }
                    ],
                }
            ),
            ["BLOCAGE · Non-conformité · Le dernier étalonnage est non conforme."],
        )


def instrument() -> dict[str, object]:
    return {
        "asset_id": "SA-001",
        "revision": "rev-asset",
        "equipment_model_id": "EQM-CABLE",
        "equipment_model_revision_id": "EQM-CABLE-rev-0001",
        "equipment_model_checksum": "sha256:" + "a" * 64,
        "manufacturer": "Demo",
        "model": "RF Cable",
        "serial_number": "C001",
    }


def characterization(characterization_id: str, valid_until: str) -> dict[str, object]:
    return {
        "characterization_id": characterization_id,
        "characterization_kind": "frequency_response",
        "label": "Pertes mesurées",
        "performed_on": "2026-01-01",
        "valid_until": valid_until,
        "decision": "conforming",
        "definition_checksum": "sha256:" + "b" * 64,
    }


if __name__ == "__main__":
    unittest.main()
