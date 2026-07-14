import { expect, test } from "@playwright/test";

test("metrologist records and reloads a serial-specific RF cable response", async ({ page, request }) => {
  const suffix = Date.now().toString(36).toUpperCase();
  const assetId = `E2E-RF-CABLE-${suffix}`;
  const serialNumber = `RF-${suffix}`;
  const registration = await request.post("/api/v1/metrology/instruments", {
    data: {
      asset_id: assetId,
      family: "RF cable",
      equipment_model_id: "EQM-E2E-RF-CABLE",
      equipment_model_revision_id: "EQM-E2E-RF-CABLE-rev-0001",
      equipment_model_checksum: `sha256:${"a".repeat(64)}`,
      manufacturer: "Huber+Suhner",
      model: "Sucoflex 104",
      serial_number: serialNumber,
      part_number: "22510146",
      calibration_requirement: "required",
      calibration_period_months: 12,
      calibration_due_warning_days: 45,
      serviceability_status: "usable",
      serviceability_reason: "Contrôle visuel conforme",
      capabilities: { frequency_min_hz: 1_000_000, frequency_max_hz: 1_000_000_000 },
      metrology_notes: "Câble de référence E2E",
      actor: "metrology.e2e",
      reason: "prepare characterization E2E",
      operation_id: `op-register-${assetId}`
    }
  });
  expect(registration.ok(), await registration.text()).toBeTruthy();

  await page.goto("/lab/");
  await page.getByRole("button", { name: "Équipements" }).click();
  await page.getByRole("button", { name: "Matériels réels" }).click();
  await page.getByRole("button", { name: new RegExp(assetId) }).click();
  await expect(page.getByRole("heading", { name: assetId })).toBeVisible();
  await expect(page.locator(".assetRecordHeader").getByText(`N° de série ${serialNumber}`)).toBeVisible();

  await page.getByRole("button", { name: "Ajouter une caractérisation" }).click();
  await expect(page.getByRole("heading", { name: "Ajouter une caractérisation" })).toBeVisible();
  await page.getByLabel(/Nom de la caractérisation/).fill("Pertes après contrôle annuel");
  await page.getByLabel(/Laboratoire ou prestataire/).fill("Laboratoire interne CEM");
  await page.getByLabel(/Méthode utilisée/).fill("MET-RF-CABLE-001");
  await page.getByLabel("Référence du certificat ou feuillet").fill(`CERT-${suffix}`);
  await page.getByLabel(/Tableau mesuré/).fill(
    "frequence_hz,amplitude_db\n1000000,0.12\n100000000,0.86\n1000000000,2.91"
  );
  const fileResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/metrology/files") &&
    response.request().method() === "POST"
  );
  await page.locator('.characterizationForm input[type="file"]').setInputFiles({
    name: `certificate-${suffix}.pdf`,
    mimeType: "application/pdf",
    buffer: Buffer.from("%PDF-1.4\nEMC Locus characterization evidence")
  });
  const characterizationResponse = page.waitForResponse((response) =>
    response.url().endsWith(`/api/v1/metrology/instruments/${assetId}/characterizations`) &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: "Enregistrer puis préparer la revue" }).click();
  expect((await fileResponse).ok()).toBeTruthy();
  expect((await characterizationResponse).ok()).toBeTruthy();

  await expect(page.getByRole("heading", { name: "Pertes après contrôle annuel" })).toBeVisible();
  await expect(page.getByText("3 points, de 1 MHz à 1 GHz")).toBeVisible();
  await expect(page.getByText(`certificate-${suffix}.pdf`)).toBeVisible();
  await expect(page.getByText("Applicable", { exact: true })).toBeVisible();

  const list = await request.get(`/api/v1/metrology/instruments/${assetId}/characterizations`);
  expect(list.ok()).toBeTruthy();
  const listBody = await list.json();
  expect(listBody.characterizations).toHaveLength(1);
  expect(listBody.characterizations[0].definition.correction.correction.points).toHaveLength(3);
  expect(listBody.characterizations[0].document_manifest.original_filename).toBe(
    `certificate-${suffix}.pdf`
  );

  const characterizationId = listBody.characterizations[0].characterization_id;
  const audit = await request.get(
    `/api/v1/metrology/instruments/${assetId}/characterizations/${characterizationId}/audit-events`
  );
  expect(audit.ok()).toBeTruthy();
  expect(await audit.text()).toContain("asset_characterization_recorded");
  const outbox = await request.get("/api/v1/sync/outbox");
  expect(outbox.ok()).toBeTruthy();
  expect(await outbox.text()).toContain(characterizationId);

  await page.reload();
  await page.getByRole("button", { name: "Équipements" }).click();
  await page.getByRole("button", { name: "Matériels réels" }).click();
  await page.getByRole("button", { name: new RegExp(assetId) }).click();
  await expect(page.getByRole("heading", { name: "Pertes après contrôle annuel" })).toBeVisible();
  await expect(page.getByText("3 points, de 1 MHz à 1 GHz")).toBeVisible();
});
