import { describe, expect, test } from "vitest";
import { metrologyStatusLabel } from "./metrologyStatus";
import type { MetrologyStatus } from "./models/fleet";

describe("authoritative metrology labels", () => {
  test.each([
    ["valid", "2027-08-31", "Étalonnage valide jusqu’au 31 août 2027"],
    ["due_soon", "2026-08-01", "Échéance d’étalonnage proche : 1 août 2026"],
    ["expired", "2026-07-26", "Étalonnage expiré depuis le 26 juil. 2026"],
    ["missing", null, "Aucun étalonnage valide"],
    ["not_required", null, "Étalonnage non requis"],
    ["nonconforming", "2027-08-31", "Dernier étalonnage non conforme"],
    ["indeterminate", null, "Décision métrologique indéterminée"],
    ["unavailable", null, "Métrologie temporairement indisponible"]
  ] as Array<[MetrologyStatus, string | null, string]>) (
    "renders %s without recomputing domain state",
    (status, due_at, expected) => {
      expect(metrologyStatusLabel({ status, due_at })).toBe(expected);
    }
  );

  test("never labels an explicit expired status as valid", () => {
    expect(metrologyStatusLabel({ status: "expired", due_at: "2099-01-01" }))
      .not.toContain("valide jusqu’au");
  });
});
