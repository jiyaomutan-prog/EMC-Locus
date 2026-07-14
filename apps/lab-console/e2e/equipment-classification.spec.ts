import { expect, test } from "@playwright/test";

test("equipment repository UX manages nested categories, fields and model creation without demo pollution", async ({ page, request }) => {
  const suffix = Date.now().toString(36).toUpperCase();
  const nestedLabel = `Amplificateurs faible bruit ${suffix}`;
  const fieldLabel = `Criticite UX ${suffix}`;
  const generatedCategoryId = `amplificateurs_faible_bruit_${suffix.toLowerCase()}`;
  const generatedFieldCode = `criticite_ux_${suffix.toLowerCase()}`;
  const modelId = `E2E-RF-LNA-${suffix}`;

  await page.goto("/lab/");
  await page.getByRole("button", { name: "Équipements" }).click();
  await expect(page.getByRole("heading", { name: "Équipements" })).toBeVisible();
  await expect(page.getByText("[DEMO]")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Administration du référentiel" })).toBeVisible();

  await page.getByRole("button", { name: "Administration du référentiel" }).click();
  await expect(page.locator(".equipmentStudio .eyebrow", { hasText: "Administration du référentiel" })).toBeVisible();
  const categoryTree = page.locator(".categoryPanel .categoryTree");
  await expect(categoryTree.locator('[data-category-id="general_equipment"]')).toBeVisible();
  for (const categoryId of [
    "energy_sources",
    "signal_sources",
    "rf_equipment",
    "sensors_transducers",
    "actuators_emitters",
    "measurement_instruments_digitizers",
    "processing_control_systems"
  ]) {
    await expect(categoryTree.locator(`[data-category-id="${categoryId}"]`)).toBeVisible();
  }

  const rfRow = categoryTree.locator('[data-category-id="rf_equipment"]');
  await rfRow.locator(".treeMenuButton").click();
  const menu = rfRow.locator(".treeActionMenu");
  await expect(menu.getByRole("button", { name: "Ajouter une sous-categorie" })).toBeVisible();
  await expect(menu.getByRole("button", { name: "Modifier le formulaire" })).toBeVisible();
  await menu.getByRole("button", { name: "Modifier le formulaire" }).click();
  await expect(page.locator(".adminTabs").getByRole("button", { name: "Formulaire", exact: true })).toHaveClass(/active/);

  await categoryTree.locator('[data-category-id="rf_amplifier"]').click();
  await page.getByRole("button", { name: "Sous-categories" }).click();
  await page.getByLabel(/Nom de la sous-categorie/).fill(nestedLabel);
  await expect(page.getByLabel(/Identifiant interne/)).toBeHidden();
  const categoryResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/equipment/categories") &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Creer la sous-categorie/ }).click();
  expect((await categoryResponse).ok()).toBeTruthy();
  await expect(categoryTree.locator(`[data-category-id="${generatedCategoryId}"]`)).toBeVisible();

  await categoryTree.locator(`[data-category-id="${generatedCategoryId}"]`).click();
  await page.locator(".adminTabs").getByRole("button", { name: "Formulaire", exact: true }).click();
  await page.getByLabel("Nom du champ").fill(fieldLabel);
  await page.getByLabel("Description / aide").fill("Niveau de criticite propre a cette categorie.");
  await expect(page.getByLabel("Nom technique")).toBeHidden();
  await page.getByPlaceholder("Nouvelle valeur").fill("Observation");
  await page.getByRole("button", { name: "Ajouter une valeur" }).click();
  await expect(page.getByText("Observation")).toBeVisible();
  const fieldResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/equipment/field-definitions") &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Creer le champ/ }).click();
  expect((await fieldResponse).ok()).toBeTruthy();

  const ruleResponse = page.waitForResponse((response) =>
    response.url().endsWith(`/api/v1/equipment/categories/${generatedCategoryId}/field-rules`) &&
    response.request().method() === "PUT"
  );
  await page.getByRole("button", { name: /Ajouter au formulaire/ }).click();
  expect((await ruleResponse).ok()).toBeTruthy();

  await page.getByRole("button", { name: "Apercu" }).click();
  await expect(page.getByText("Voici le formulaire que verra un technicien pour cette categorie.")).toBeVisible();
  await expect(page.getByText(fieldLabel)).toBeVisible();
  await expect(page.getByText("template_checksum")).toHaveCount(0);
  await expect(page.getByText(generatedFieldCode)).toHaveCount(0);

  await page.getByRole("button", { name: "Catalogue équipements" }).click();
  await page.getByRole("button", { name: /Nouveau modèle/ }).click();
  const wizard = page.locator(".creationPanel");
  await expect(wizard.getByText("Nouveau modèle équipement")).toBeVisible();
  await expect(wizard.getByRole("button", { name: /radiofr/i })).toHaveCount(0);
  await wizard.getByLabel(/radiofr/i).check();
  await wizard.getByRole("button", { name: "Continuer" }).click();
  await wizard.locator(`[data-category-id="${generatedCategoryId}"]`).click();
  await wizard.getByRole("button", { name: "Continuer" }).click();
  await wizard.getByLabel(/Fabricant/).fill("E2E Demo");
  await wizard.getByLabel(/Mod.le/).fill("LNA UX");
  await wizard.getByLabel(fieldLabel).selectOption("Normale");
  const fileResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/equipment/files") &&
    response.request().method() === "POST"
  );
  await wizard.locator('input[type="file"]').setInputFiles({
    name: "lna-datasheet.pdf",
    mimeType: "application/pdf",
    buffer: Buffer.from("%PDF-1.4\nE2E")
  });
  expect((await fileResponse).ok()).toBeTruthy();
  await expect(wizard.getByText("lna-datasheet.pdf")).toBeVisible();
  await wizard.getByRole("button", { name: "Continuer" }).click();
  await wizard.getByLabel("ID modele optionnel").fill(modelId);
  const createResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/equipment-models/from-category-template") &&
    response.request().method() === "POST"
  );
  await wizard.getByRole("button", { name: /Creer brouillon/ }).click();
  expect((await createResponse).ok()).toBeTruthy();

  await expect(page.getByText("Fiche modèle équipement")).toBeVisible();
  await page.getByRole("button", { name: "Synthese" }).click();
  await expect(page.locator("dd").filter({ hasText: nestedLabel }).first()).toBeVisible();
  await expect(page.getByText(generatedCategoryId)).toHaveCount(0);
  await expect(page.getByText(generatedFieldCode)).toHaveCount(0);

  await page.getByRole("button", { name: "Categorie et formulaire" }).click();
  await expect(page.getByText("Formulaire utilise")).toBeVisible();
  await expect(page.getByText("Template checksum")).toHaveCount(0);

  const model = await request.get(`/api/v1/equipment-models/${modelId}`);
  expect(model.ok()).toBeTruthy();
  const body = await model.json();
  expect(body.equipment_model.identity.root_category_id).toBe("rf_equipment");
  expect(body.equipment_model.identity.category_code).toBe(generatedCategoryId);
  expect(body.equipment_model.identity.is_demo).toBe(false);
  expect(body.equipment_model.latest_revision.definition.template_snapshot.category_id).toBe(generatedCategoryId);
  expect(body.equipment_model.latest_revision.definition.custom_field_values[generatedFieldCode]).toBe("Normale");
  expect(body.equipment_model.latest_revision.definition.custom_field_values.documentation.original_filename).toBe("lna-datasheet.pdf");

  const subtree = await request.get(`/api/v1/equipment-models?category_code=rf_amplifier&demo_mode=hide`);
  expect(subtree.ok()).toBeTruthy();
  expect(await subtree.text()).toContain(modelId);

  const submitResponse = page.waitForResponse((response) =>
    response.url().includes("submit-for-review") && response.request().method() === "POST"
  );
  await page.getByRole("button", { name: "Soumettre" }).click();
  expect((await submitResponse).ok()).toBeTruthy();
  const approveResponse = page.waitForResponse((response) =>
    response.url().endsWith("/transitions/approve") && response.request().method() === "POST"
  );
  await page.getByRole("button", { name: "Approuver" }).click();
  expect((await approveResponse).ok()).toBeTruthy();

  await page.getByRole("button", { name: "Matériels réels" }).click();
  const approvedModelSelect = page.getByLabel(/Modèle d.équipement/);
  await expect(approvedModelSelect.locator(`option[value="${modelId}"]`)).toHaveCount(1);
  await approvedModelSelect.selectOption(modelId);
  const assetId = `ASSET-RF-LNA-${suffix}`;
  await page.getByLabel(/Numéro d’inventaire/).fill(assetId);
  await page.getByLabel(/Numéro de série/).fill(`SN-${suffix}`);
  await page.getByLabel(/Part number/).fill("LNA-40DB");
  const assetResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/metrology/instruments") &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: "Enregistrer le matériel" }).click();
  expect((await assetResponse).ok()).toBeTruthy();
  await expect(page.getByRole("heading", { name: assetId })).toBeVisible();

  const asset = await request.get(`/api/v1/metrology/instruments/${assetId}`);
  expect(asset.ok()).toBeTruthy();
  const assetBody = await asset.json();
  expect(assetBody.instrument.serial_number).toBe(`SN-${suffix}`);
  expect(assetBody.instrument.manufacturer).toBe("E2E Demo");
  expect(assetBody.instrument.category_code).toBeNull();
  expect(assetBody.instrument.equipment_model_id).toBe(modelId);
  expect(assetBody.instrument.equipment_model_revision_id).toBe(body.equipment_model.latest_revision.revision_id);
  expect(assetBody.instrument.equipment_model_checksum).toBe(body.equipment_model.latest_revision.definition_checksum);
});

test("new equipment model wizard uses category choices instead of primary action buttons", async ({ page }) => {
  await page.goto("/lab/");
  await page.getByRole("button", { name: "Équipements" }).click();
  await page.getByRole("button", { name: /Nouveau modèle/ }).click();
  const wizard = page.locator(".creationPanel");

  await expect(wizard.locator(".choiceList")).toBeVisible();
  await expect(wizard.locator('input[type="radio"][name="equipment-root-category"]')).toHaveCount(7);
  await expect(wizard.getByRole("button", { name: /Sources d'energie/ })).toHaveCount(0);
  await expect(wizard.getByRole("button", { name: /radiofr/i })).toHaveCount(0);

  await wizard.getByLabel(/radiofr/i).check();
  await wizard.getByRole("button", { name: "Continuer" }).click();
  await expect(wizard.locator(".categoryTree")).toBeVisible();
  const rfRow = wizard.locator('[data-category-id="rf_equipment"]');
  await expect(rfRow).toBeVisible();
  await rfRow.locator(".treeDisclosure").click();
  await expect(wizard.locator('[data-category-id="rf_cable"]')).toHaveCount(0);
  await rfRow.locator(".treeDisclosure").click();
  await expect(wizard.locator('[data-category-id="rf_cable"]')).toBeVisible();
});
