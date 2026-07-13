import { expect, test } from "@playwright/test";

test("equipment repository UX creates a model from category template without demo pollution", async ({ page, request }) => {
  await page.goto("/lab/");
  await page.getByRole("button", { name: "Equipment" }).click();
  await expect(page.getByRole("heading", { name: "Equipment Repository" })).toBeVisible();
  await expect(page.getByText("[DEMO]")).toHaveCount(0);
  await expect(page.locator(".categoryTree").getByRole("button", { name: "Équipements radiofréquences" })).toBeVisible();

  await page.getByRole("button", { name: "Repository Administration" }).click();
  await expect(page.getByRole("heading", { name: "Categories" })).toBeVisible();
  const admin = page.locator(".equipmentLayout").filter({ has: page.getByRole("heading", { name: "Categories" }) });
  const rfRootButton = admin.locator('button[data-category-id="rf_equipment"]');
  await rfRootButton.click();
  await expect(rfRootButton).toHaveClass(/active/);
  await page.getByLabel("Identifiant stable").fill("rf_switch_matrix_e2e");
  await page.getByLabel("Libelle").first().fill("Matrices RF E2E");
  await page.getByRole("button", { name: /Creer sous-categorie/ }).click();
  await expect(page.locator(".categoryTree").getByRole("button", { name: /Matrices RF E2E/ })).toBeVisible();
  await rfRootButton.click();
  await expect(rfRootButton).toHaveClass(/active/);

  await page.getByLabel("Code champ").fill("e2e_criticality");
  await page.getByLabel("Libelle").nth(1).fill("Criticité E2E");
  await page.getByLabel("Choix autorises").fill("faible, moyenne, forte");
  const fieldCreateResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/equipment/field-definitions") &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Creer champ/ }).click();
  expect((await fieldCreateResponse).ok()).toBeTruthy();
  await expect(page.getByLabel("Champ a ajouter").locator("option", { hasText: "Criticité E2E" })).toHaveCount(1);
  await page.getByLabel("Champ a ajouter").selectOption("field_e2e_criticality");
  const fieldSelect = page.getByLabel("Champ a ajouter");
  await expect(fieldSelect).toHaveValue("field_e2e_criticality");
  const ruleResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/equipment/categories/rf_equipment/field-rules") &&
    response.request().method() === "PUT"
  );
  await page.getByRole("button", { name: /Ajouter au template/ }).click();
  expect((await ruleResponse).ok()).toBeTruthy();
  await expect.poll(async () => {
    const response = await request.get("/api/v1/equipment/categories/rf_equipment/effective-template");
    const body = await response.json();
    return body.effective_template.fields.some(
      (field: { field: { field_code: string } }) => field.field.field_code === "e2e_criticality"
    );
  }).toBeTruthy();

  await page.getByRole("button", { name: "Equipment Repository" }).click();
  await page.getByRole("button", { name: /Nouveau modele/ }).click();
  const wizard = page.locator(".creationPanel");
  await expect(wizard.getByText("Nouveau modele equipement")).toBeVisible();
  await wizard.getByRole("button", { name: /Équipements radiofréquences/ }).click();
  await wizard.getByRole("combobox").first().selectOption("rf_cable");
  await expect(wizard.getByLabel(/Fabricant/)).toBeVisible();
  await wizard.getByLabel("ID modele optionnel").fill("E2E-RF-CABLE-TEMPLATE");
  await wizard.getByLabel(/Fabricant/).fill("E2E Demo");
  await wizard.getByLabel(/Modèle/).fill("RF Cable Template");
  await wizard.getByLabel(/Variante/).fill("1m N-N");
  await wizard.getByLabel(/Connecteur A/).fill("N");
  await wizard.getByLabel(/Connecteur B/).fill("N");
  await wizard.getByLabel(/Criticité E2E/).selectOption("moyenne");

  const createResponse = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/equipment-models/from-category-template") &&
    response.request().method() === "POST"
  );
  await wizard.getByRole("button", { name: /Creer brouillon/ }).click();
  expect((await createResponse).ok()).toBeTruthy();

  await expect(page.getByText("Equipment Model Definition")).toBeVisible();
  await page.getByRole("button", { name: "Summary" }).click();
  await expect(page.locator("dd").filter({ hasText: "Équipements radiofréquences > Câbles RF" }).first()).toBeVisible();
  await expect(page.getByText("rf_network_element")).toHaveCount(0);

  await page.getByRole("button", { name: "Category & Template" }).click();
  await expect(page.getByText("Template checksum")).toBeVisible();
  await page.getByRole("button", { name: "Ports & Connections" }).click();
  await expect(page.locator('input[value="rf_a"]')).toBeVisible();
  await expect(page.locator('input[value="rf_b"]')).toBeVisible();

  await page.getByRole("button", { name: /Valider/ }).click();
  await expect(page.getByText("Definition valide")).toBeVisible();

  const submitResponse = page.waitForResponse((response) =>
    response.url().includes("/transitions/submit-for-review") &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Soumettre/ }).click();
  expect((await submitResponse).ok()).toBeTruthy();

  const approveResponse = page.waitForResponse((response) =>
    response.url().includes("/transitions/approve") && response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Approuver/ }).click();
  expect((await approveResponse).ok()).toBeTruthy();

  const model = await request.get("/api/v1/equipment-models/E2E-RF-CABLE-TEMPLATE");
  expect(model.ok()).toBeTruthy();
  const body = await model.json();
  expect(body.equipment_model.identity.root_category_id).toBe("rf_equipment");
  expect(body.equipment_model.identity.is_demo).toBe(false);
  expect(body.equipment_model.current_approved_revision.definition.template_snapshot.category_id).toBe("rf_cable");
  expect(body.equipment_model.current_approved_revision.definition.custom_field_values.e2e_criticality).toBe("moyenne");
});
