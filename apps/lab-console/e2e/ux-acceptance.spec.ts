import { expect, test } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const viewports = [
  { width: 1440, height: 900 },
  { width: 1280, height: 720 }
];

test("main operator paths stay clear at supported desktop sizes", async ({ page, request }, testInfo) => {
  const suffix = Date.now().toString(36).toUpperCase();
  const assetId = `UX-RF-CABLE-${suffix}`;
  const characterizationId = `CHAR-${assetId}`;
  const registration = await request.post("/api/v1/metrology/instruments", {
    data: {
      asset_id: assetId,
      family: "RF cable",
      equipment_model_id: "EQM-UX-RF-CABLE",
      equipment_model_revision_id: "EQM-UX-RF-CABLE-rev-0001",
      equipment_model_checksum: `sha256:${"b".repeat(64)}`,
      manufacturer: "Rosenberger",
      model: "RPC-3.50",
      serial_number: `UX-${suffix}`,
      calibration_requirement: "required",
      calibration_period_months: 12,
      serviceability_status: "usable",
      serviceability_reason: "Disponible au laboratoire",
      capabilities: { frequency_max_hz: 6_000_000_000 },
      actor: "ux.audit",
      reason: "prepare UX evidence",
      operation_id: `op-register-${assetId}`
    }
  });
  expect(registration.ok(), await registration.text()).toBeTruthy();
  const characterization = await request.post(
    `/api/v1/metrology/instruments/${assetId}/characterizations`,
    {
      data: {
        characterization_id: characterizationId,
        performed_on: "2026-07-01",
        valid_until: "2027-07-01",
        provider: "Laboratoire interne CEM",
        method_reference: "MET-RF-CABLE-001",
        decision: "conforming",
        definition: {
          definition_schema_version: "emc-locus.asset-characterization-definition.v1",
          characterization_id: characterizationId,
          asset_id: assetId,
          label: "Pertes mesurées du câble",
          correction: {
            correction_kind: "frequency_response",
            correction: {
              definition_schema_version: "emc-locus.engineering-curve-definition.v1",
              curve_id: characterizationId,
              curve_type: "cable_loss",
              label: "Pertes mesurées du câble",
              signal_representation: "frequency_domain_spectrum",
              independent_axes: [{ axis: "frequency", quantity: "frequency", unit: "Hz" }],
              dependent_values: [{
                value_id: "amplitude",
                quantity: "dimensionless",
                unit: "dB",
                component: "amplitude",
                operation: "add"
              }],
              points: [
                { axis_values: { frequency: 1_000_000 }, values: { amplitude: 0.1 } },
                { axis_values: { frequency: 100_000_000 }, values: { amplitude: 0.9 } },
                { axis_values: { frequency: 1_000_000_000 }, values: { amplitude: 2.9 } }
              ],
              interpolation: "log_x_linear_y",
              extrapolation_policy: "forbidden"
            }
          },
          uncertainty: {
            expanded_uncertainty: 0.2,
            unit: "dB",
            coverage_factor: 2,
            confidence_level_percent: 95
          }
        },
        certificate_reference: `CERT-${suffix}`,
        recorded_by: "metrology.operator",
        actor: "metrology.operator",
        reason: "prepare UX evidence",
        operation_id: `op-characterization-${assetId}`
      }
    }
  );
  expect(characterization.ok(), await characterization.text()).toBeTruthy();

  for (const viewport of viewports) {
    const size = `${viewport.width}x${viewport.height}`;
    await page.setViewportSize(viewport);
    await page.goto("/lab/");

    await expect(page.getByText("Agent local")).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    await expect(page.getByRole("heading", { name: "Méthodes d'essai" })).toBeVisible();
    await expect(page.getByText("Aucun template")).toHaveCount(0);
    await expect(page.getByText("bibliothèque API")).toHaveCount(0);
    await assertNoHorizontalOverflow(page);
    await capture(page, testInfo, `methodes-${size}.png`);

    await page.getByRole("button", { name: "Équipements" }).click();
    await page.getByRole("button", { name: "Signaux et corrections" }).click();
    await expect(page.getByRole("heading", { name: "Comment le signal est-il exploité ?" })).toBeVisible();
    await expect(page.getByRole("button", { name: /échantillons temporels/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /spectre en fréquence/ })).toBeVisible();
    await assertNoHorizontalOverflow(page);
    await capture(page, testInfo, `choix-signal-${size}.png`);

    await page.getByRole("button", { name: /spectre en fréquence/ }).click();
    const label = `Pertes câble RF contrôle ${size}`;
    await page.getByLabel("Nom de la correction").fill(label);
    await page.getByRole("button", { name: "Nouvelle correction" }).click();
    await expect(page.locator(".equipmentStudio").getByRole("heading", { name: label })).toBeVisible();
    await expect(page.getByText("Identifiant interne")).toHaveCount(0);
    await expect(page.getByText("Identifiant personnalisé")).toHaveCount(0);
    await expect(page.getByText("Aucun verdict serveur courant.")).toHaveCount(0);
    await expect(page.getByText("Empreinte de contrôle SHA-256")).toBeHidden();
    await assertNoHorizontalOverflow(page);
    await capture(page, testInfo, `correction-frequentielle-${size}.png`);

    await page.getByRole("button", { name: "Matériels réels" }).click();
    await page.getByRole("button", { name: new RegExp(assetId) }).click();
    await expect(page.getByRole("heading", { name: assetId })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Pertes mesurées du câble" })).toBeVisible();
    await assertNoHorizontalOverflow(page);
    await capture(page, testInfo, `dossier-metrologique-${size}.png`);

    await page.getByRole("button", { name: "Ajouter une caractérisation" }).click();
    await expect(page.getByRole("heading", { name: "Ajouter une caractérisation" })).toBeVisible();
    await expect(page.getByText("Correction selon la fréquence", { exact: true })).toBeVisible();
    await assertNoHorizontalOverflow(page);
    await capture(page, testInfo, `nouvelle-caracterisation-${size}.png`);
  }
});

async function assertNoHorizontalOverflow(page: import("@playwright/test").Page) {
  const dimensions = await page.evaluate(() => ({
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth
  }));
  expect(dimensions.scrollWidth).toBeLessThanOrEqual(dimensions.clientWidth);
}

async function capture(
  page: import("@playwright/test").Page,
  testInfo: import("@playwright/test").TestInfo,
  name: string
) {
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.waitForTimeout(80);
  const body = await page.screenshot({ animations: "disabled" });
  const evidenceDirectory = path.resolve(process.cwd(), "../../.codex/ux-audit/0.16.0");
  await mkdir(evidenceDirectory, { recursive: true });
  await writeFile(path.join(evidenceDirectory, name), body);
  await testInfo.attach(name, { body, contentType: "image/png" });
}
