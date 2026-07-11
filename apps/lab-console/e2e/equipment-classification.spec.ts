import { expect, test } from "@playwright/test";

test("equipment preset workflow covers RF cable and explicit CAN bus on ADC", async ({ page, request }) => {
  await page.goto("/lab/");
  await page.getByRole("button", { name: "Equipment" }).click();
  await expect(page.getByRole("heading", { name: "Model Catalog" })).toBeVisible();

  await page.getByRole("button", { name: /Nouveau modele/ }).click();
  const rfCreation = page.locator(".creationPanel");
  await rfCreation.getByLabel("Classification preset").selectOption("rf_cable");
  await expect(page.getByText("RF_A")).toBeVisible();
  await expect(page.getByText("RF_B")).toBeVisible();
  await rfCreation.getByLabel("Equipment model ID").fill("E2E-RF-CABLE");
  await rfCreation.getByLabel("Manufacturer").fill("E2E Demo");
  await rfCreation.getByLabel("Model name").fill("RF Cable E2E");

  const rfCreate = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/equipment-models/from-preset") &&
    response.request().method() === "POST"
  );
  await rfCreation.getByRole("button", { name: "Creer" }).click();
  expect((await rfCreate).ok()).toBeTruthy();

  await expect(page.getByText("Equipment Model Definition")).toBeVisible();
  await page.getByRole("button", { name: "Port Topology" }).click();
  await expect(page.locator('input[value="RF_A"]')).toBeVisible();
  await expect(page.locator('input[value="RF_B"]')).toBeVisible();
  await page.getByRole("button", { name: /Valider/ }).click();
  await expect(page.getByText("Definition valide")).toBeVisible();

  const rfSubmit = page.waitForResponse((response) =>
    response.url().includes("/transitions/submit-for-review") &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Soumettre/ }).click();
  expect((await rfSubmit).ok()).toBeTruthy();
  const rfApprove = page.waitForResponse((response) =>
    response.url().includes("/transitions/approve") && response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Approuver/ }).click();
  expect((await rfApprove).ok()).toBeTruthy();

  const rfModel = await request.get("/api/v1/equipment-models/E2E-RF-CABLE");
  expect(rfModel.ok()).toBeTruthy();
  const rfBody = await rfModel.json();
  expect(rfBody.equipment_model.current_approved_revision.definition.signal_ports.map((port: { port_id: string }) => port.port_id)).toEqual([
    "RF_A",
    "RF_B"
  ]);

  await page.getByRole("button", { name: /Nouveau modele/ }).click();
  const adcCreation = page.locator(".creationPanel");
  await adcCreation.getByLabel("Classification preset").selectOption("adc_converter");
  await expect(page.getByRole("heading", { name: "ADC converter" })).toBeVisible();
  await adcCreation.getByLabel("Equipment model ID").fill("E2E-ADC-CONVERTER");
  await adcCreation.getByLabel("Manufacturer").fill("E2E Demo");
  await adcCreation.getByLabel("Model name").fill("ADC Converter E2E");

  const adcCreate = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/equipment-models/from-preset") &&
    response.request().method() === "POST"
  );
  await adcCreation.getByRole("button", { name: "Creer" }).click();
  expect((await adcCreate).ok()).toBeTruthy();

  await page.getByRole("button", { name: "Port Topology" }).click();
  await expect(page.locator('input[value="ANALOG_IN"]')).toBeVisible();
  await expect(page.locator('input[value="DIGITAL_OUT"]')).toBeVisible();
  await expect(page.locator('input[value="CAN_BUS"]')).toHaveCount(0);
  await page.getByRole("button", { name: "Ajouter CAN bus" }).click();
  await expect(page.locator('input[value^="can_bus_"]')).toBeVisible();

  await page.getByRole("button", { name: /Valider/ }).click();
  await expect(page.getByText("Definition valide")).toBeVisible();
  const adcSave = page.waitForResponse((response) =>
    response.url().includes("/api/v1/equipment-models/E2E-ADC-CONVERTER/revisions/") &&
    response.url().endsWith("/definition") &&
    response.request().method() === "PUT"
  );
  await page.getByRole("button", { name: /Sauvegarder/ }).click();
  expect((await adcSave).ok()).toBeTruthy();

  const adcModel = await request.get("/api/v1/equipment-models/E2E-ADC-CONVERTER");
  expect(adcModel.ok()).toBeTruthy();
  const adcBody = await adcModel.json();
  const adcPorts = adcBody.equipment_model.latest_revision.definition.signal_ports.map(
    (port: { signal_domain: string }) => port.signal_domain
  );
  expect(adcPorts).toContain("can_bus");
});
