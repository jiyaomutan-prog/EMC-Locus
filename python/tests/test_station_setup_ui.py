import unittest

from emc_locus.station_setup_ui import (
    applicable_characterizations,
    build_asset_binding,
    build_correction_selection,
    characterization_display_label,
    current_station_revision,
    eligible_station_instruments,
    is_station_v3_definition,
    port_display_label,
    readiness_lines,
    station_material_role_rows,
    station_revision_status_label,
)


class StationSetupUiTests(unittest.TestCase):
    def test_prefers_qualified_station_revision_when_no_draft_exists(self) -> None:
        qualified = {"revision_id": "SETUP-001-rev-0002", "status": "qualified"}
        ready = {"revision_id": "SETUP-001-rev-0001", "status": "ready"}

        self.assertIs(
            current_station_revision(
                {
                    "active_draft_revision": None,
                    "current_qualified_revision": qualified,
                    "current_ready_revision": ready,
                }
            ),
            qualified,
        )
        self.assertEqual(station_revision_status_label(qualified), "Définition validée")

    def test_shapes_v3_requirement_and_physical_assignment_separately(self) -> None:
        definition = station_v3_definition()

        rows = station_material_role_rows(definition)

        self.assertTrue(is_station_v3_definition(definition))
        self.assertEqual(rows[0]["role"], "Récepteur CEM")
        self.assertEqual(
            rows[0]["requirement"],
            "Aptitudes techniques · obligatoire · affecté dans le montage",
        )
        self.assertEqual(
            rows[0]["assignment"],
            "RX-01 · S/N 1234 · affectation physique figée",
        )
        self.assertEqual(
            rows[1]["assignment"],
            "Exemplaire imposé : SA-CABLE-001 · affectation à finaliser",
        )

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
            readiness_lines({"ready": True, "issues": []}),
            ["Aucun blocage détecté. Le montage peut être déclaré prêt."],
        )
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


def station_v3_definition() -> dict[str, object]:
    return {
        "definition_schema_version": "emc-locus.station-measurement-setup-definition.v3",
        "material_requirements": [
            {
                "requirement_id": "receiver",
                "role_label": "Récepteur CEM",
                "required": True,
                "selection_policy": "capability_match",
                "assignment_stage": "setup_definition",
            },
            {
                "requirement_id": "rf-cable",
                "role_label": "Câble RF",
                "required": True,
                "selection_policy": "exact_asset",
                "assignment_stage": "planned_test_preparation",
                "exact_asset_id": "SA-CABLE-001",
            },
        ],
        "material_assignments": [
            {
                "requirement_id": "receiver",
                "asset_id": "SA-RX-001",
                "inventory_code": "RX-01",
                "serial_number": "1234",
            }
        ],
    }


if __name__ == "__main__":
    unittest.main()
