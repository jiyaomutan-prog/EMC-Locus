import { expect, test } from "@playwright/test";

test("equipment repository UX manages nested categories, fields and model creation without demo pollution", async ({ page, request }) => {
  const suffix = Date.now().toString(36).toUpperCase();
  const nestedLabel = `Amplificateurs faible bruit ${suffix}`;
  const fieldLabel = `Criticite UX ${suffix}`;
  const generatedCategoryId = `amplificateurs_faible_bruit_${suffix.toLowerCase()}`;
  const generatedFieldCode = `criticite_ux_${suffix.toLowerCase()}`;
  let modelId = "";
  const locationId = await createLaboratoryLocation(request, `Zone RF ${suffix}`);

  await page.goto("/lab/");
  await page.getByRole("button", { name: "Catalogue des modèles" }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Catalogue des modèles" })).toBeVisible();
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
  await expect(menu.getByRole("button", { name: "Ajouter une sous-catégorie" })).toBeVisible();
  await expect(menu.getByRole("button", { name: "Modifier le formulaire" })).toBeVisible();
  await menu.getByRole("button", { name: "Modifier le formulaire" }).click();
  await expect(page.locator(".adminTabs").getByRole("button", { name: "Formulaire", exact: true })).toHaveClass(/active/);

  await categoryTree.locator('[data-category-id="rf_amplifier"]').click();
  await page.getByRole("button", { name: "Sous-catégories" }).click();
  await page.getByLabel(/Nom de la sous-catégorie/).fill(nestedLabel);
  await expect(page.getByLabel(/Identifiant interne/)).toBeHidden();
  const categoryResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/equipment/categories") &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Créer la sous-catégorie/ }).click();
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
  await page.getByRole("button", { name: /Créer le champ/ }).click();
  expect((await fieldResponse).ok()).toBeTruthy();

  const ruleResponse = page.waitForResponse((response) =>
    response.url().endsWith(`/api/v1/equipment/categories/${generatedCategoryId}/field-rules`) &&
    response.request().method() === "PUT"
  );
  await page.getByRole("button", { name: /Ajouter au formulaire/ }).click();
  expect((await ruleResponse).ok()).toBeTruthy();

  await page.getByRole("button", { name: "Aperçu" }).click();
  await expect(page.getByText("Voici le formulaire que verra un technicien pour cette catégorie.")).toBeVisible();
  await expect(page.getByText(fieldLabel)).toBeVisible();
  await expect(page.getByText("template_checksum")).toHaveCount(0);
  await expect(page.getByText(generatedFieldCode)).toHaveCount(0);

  await page.getByRole("button", { name: "Catalogue des modèles" }).click();
  await page.getByRole("button", { name: /Nouveau modèle/ }).click();
  const wizard = page.locator(".creationPanel");
  await expect(wizard.getByText("Nouveau modèle constructeur")).toBeVisible();
  await expect(wizard.getByText("Vous créez une définition générique. Aucun exemplaire ne sera ajouté au parc.")).toBeVisible();
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
  await expect(wizard.getByLabel(/identifiant interne/i)).toHaveCount(0);
  const createResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/equipment-models/from-category-template") &&
    response.request().method() === "POST"
  );
  await wizard.getByRole("button", { name: /Créer le brouillon/ }).click();
  const createdModelResponse = await createResponse;
  expect(createdModelResponse.ok()).toBeTruthy();
  const createdModelBody = await createdModelResponse.json();
  modelId = createdModelBody.aggregate.identity.equipment_model_id as string;

  await expect(page.getByText("Vous consultez un modèle générique.")).toBeVisible();
  await page.getByRole("button", { name: "Synthèse" }).click();
  await expect(page.locator("dd").filter({ hasText: nestedLabel }).first()).toBeVisible();
  await expect(page.getByText(generatedCategoryId)).toHaveCount(0);
  await expect(page.getByText(generatedFieldCode)).toHaveCount(0);

  await page.getByRole("button", { name: "Catégorie et champs" }).click();
  await expect(page.getByText("Formulaire utilisé")).toBeVisible();
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
  await page.getByRole("button", { name: "Soumettre pour approbation" }).click();
  expect((await submitResponse).ok()).toBeTruthy();
  const approveResponse = page.waitForResponse((response) =>
    response.url().endsWith("/transitions/approve") && response.request().method() === "POST"
  );
  await page.getByRole("button", { name: "Approuver la version" }).click();
  expect((await approveResponse).ok()).toBeTruthy();

  await page.getByRole("button", { name: "Créer un exemplaire dans le parc" }).click();
  await expect(page.getByText("Vous ajoutez un exemplaire réellement utilisé par le laboratoire.")).toBeVisible();
  const inventoryCode = `ASSET-RF-LNA-${suffix}`;
  await page.getByLabel(/Code inventaire/).fill(inventoryCode);
  await page.getByLabel(/Numéro de série/).fill(`SN-${suffix}`);
  await page.getByLabel(/Référence fabricant/).fill("LNA-40DB");
  await page.getByLabel(/Emplacement/).selectOption(locationId);
  const assetResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/fleet/assets") &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: "Enregistrer l'exemplaire" }).click();
  const createdAssetResponse = await assetResponse;
  expect(createdAssetResponse.ok()).toBeTruthy();
  const createdAssetBody = await createdAssetResponse.json();
  const assetId = createdAssetBody.asset.asset_id as string;
  await expect(page.getByRole("heading", { name: inventoryCode })).toBeVisible();

  const asset = await request.get(`/api/v1/fleet/assets/${assetId}`);
  expect(asset.ok()).toBeTruthy();
  const assetBody = await asset.json();
  expect(assetBody.asset.inventory_code).toBe(inventoryCode);
  expect(assetBody.asset.serial_number).toBe(`SN-${suffix}`);
  expect(assetBody.asset.manufacturer).toBe("E2E Demo");
  expect(assetBody.asset.category_code).toBe(generatedCategoryId);
  expect(assetBody.asset.equipment_model_id).toBe(modelId);
  expect(assetBody.asset.equipment_model_revision_id).toBe(body.equipment_model.latest_revision.revision_id);
  expect(assetBody.asset.equipment_model_checksum).toBe(body.equipment_model.latest_revision.definition_checksum);
});

test("new equipment model wizard uses category choices instead of primary action buttons", async ({ page }) => {
  await page.goto("/lab/");
  await page.getByRole("button", { name: "Catalogue des modèles" }).click();
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

async function createLaboratoryLocation(request: import("@playwright/test").APIRequestContext, label: string) {
  const response = await request.post("/api/v1/laboratory-locations", {
    data: {
      label,
      description: "Emplacement créé pour le scénario E2E du parc matériel",
      actor: "equipment.e2e",
      reason: "préparer l'enregistrement d'un exemplaire",
      operation_id: `op-create-location-${Date.now()}`
    }
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  const body = await response.json();
  return body.location.location_id as string;
}
