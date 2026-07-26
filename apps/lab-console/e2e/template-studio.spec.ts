import { expect, test } from "@playwright/test";

test("template studio workflow persists through API", async ({ page, request }) => {
  let templateId = "";

  await page.goto("/lab/");
  await expect(page.getByRole("heading", { name: "Méthodes d'essai" })).toBeVisible();

  await page.getByRole("button", { name: "Créer une méthode" }).click();
  await page.getByLabel("Nom de la méthode").fill("E2E LAB template");
  await page.getByRole("combobox", { name: "Catégorie" }).selectOption("emission_transient_time_domain");
  const createResponsePromise = page.waitForResponse((response) =>
    response.url().endsWith("/api/v1/test-templates") && response.request().method() === "POST"
  );
  await page.getByRole("button", { name: "Créer le brouillon" }).click();
  const createResponse = await createResponsePromise;
  expect(createResponse.ok()).toBeTruthy();
  templateId = (await createResponse.json()).test_template.identity.template_id;

  await expect(page.getByText("Éditeur de méthode")).toBeVisible();
  await expect(page.getByRole("heading", { name: "E2E LAB template" })).toBeVisible();

  await page.getByRole("button", { name: "Variables" }).click();
  await page.getByRole("button", { name: "Ajouter une variable" }).click();
  await expect(page.getByRole("textbox", { name: "Variable 2 ID" })).toHaveValue("variable_2");
  await expect(page.locator(".saveState.dirty")).toHaveText("Modifications non sauvegardees");

  await page.getByRole("button", { name: /Vérifier la définition/ }).click();
  await expect(page.getByText("Définition prête à être soumise")).toBeVisible();

  const saveResponse = page.waitForResponse((response) =>
    response.url().includes(`/api/v1/test-templates/${templateId}/revisions/`) &&
    response.url().endsWith("/definition") &&
    response.request().method() === "PUT"
  );
  await page.getByRole("button", { name: /Sauvegarder/ }).click();
  await saveResponse;
  await expect(page.getByText("Non modifie")).toBeVisible();

  const submitResponse = page.waitForResponse((response) =>
    response.url().includes("/transitions/submit-for-review") && response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Soumettre/ }).click();
  await submitResponse;
  await expect(page.getByText("En revue")).toBeVisible();

  const approveResponse = page.waitForResponse((response) =>
    response.url().includes("/transitions/approve") && response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Approuver/ }).click();
  await approveResponse;
  await expect(page.getByText("Approuvé")).toBeVisible();

  const deriveResponse = page.waitForResponse((response) =>
    response.url().endsWith(`/api/v1/test-templates/${templateId}/revisions`) &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Dériver/ }).click();
  await deriveResponse;
  await expect(page.getByText("Brouillon")).toBeVisible();

  await page.getByRole("button", { name: "Revisions" }).click();
  await expect(page.locator("td.mono").filter({ hasText: `${templateId}-rev-0001` })).toBeVisible();
  await expect(page.locator("td.mono").filter({ hasText: `${templateId}-rev-0002` })).toBeVisible();

  const templateResponse = await request.get(`/api/v1/test-templates/${templateId}`);
  expect(templateResponse.ok()).toBeTruthy();
  const templateBody = await templateResponse.json();
  expect(templateBody.test_template.current_approved_revision.revision_id).toBe(`${templateId}-rev-0001`);
  expect(templateBody.test_template.active_draft_revision.revision_id).toBe(`${templateId}-rev-0002`);

  const revisionsResponse = await request.get(`/api/v1/test-templates/${templateId}/revisions`);
  expect(revisionsResponse.ok()).toBeTruthy();
  const revisionsBody = await revisionsResponse.json();
  expect(revisionsBody.revisions.map((revision: { revision_id: string }) => revision.revision_id)).toEqual([
    `${templateId}-rev-0001`,
    `${templateId}-rev-0002`
  ]);
});
