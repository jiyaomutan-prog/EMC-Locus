import { expect, type Page, type APIRequestContext, test } from "@playwright/test";

type MeasurementSpace = {
  tab: string;
  collection: string;
  createButton: string;
};

const spaces = {
  scaling: {
    tab: "Profils de scaling",
    collection: "scaling-profiles",
    createButton: "Creer scaling"
  },
  curves: {
    tab: "Courbes d'ingénierie",
    collection: "engineering-curves",
    createButton: "Creer courbe"
  },
  sensors: {
    tab: "Capteurs / transducteurs",
    collection: "sensor-definitions",
    createButton: "Creer capteur"
  },
  daq: {
    tab: "Voies DAQ",
    collection: "daq-channel-profiles",
    createButton: "Creer profil DAQ"
  },
  recipes: {
    tab: "Recettes d'acquisition",
    collection: "acquisition-channel-recipes",
    createButton: "Creer recette"
  }
} satisfies Record<string, MeasurementSpace>;

test("measurement engineering workflow creates approved channel recipe", async ({ page, request }) => {
  const suffix = Date.now().toString(36).toUpperCase();
  const scalingId = `E2E-SCL-CURRENT-${suffix}`;
  const currentCurveId = `E2E-CURRENT-CURVE-${suffix}`;
  const cableCurveId = `E2E-CABLE-LOSS-${suffix}`;
  const sensorId = `E2E-SNS-CURRENT-${suffix}`;
  const daqId = `E2E-DAQ-AI-${suffix}`;
  const recipeId = `E2E-REC-CURRENT-${suffix}`;

  await page.goto("/lab/");
  await page.getByRole("button", { name: "Équipements" }).click();
  await page.getByRole("button", { name: "Ingénierie de mesure" }).click();

  await createMeasurementDraft(page, spaces.scaling, scalingId, "E2E 10 mV/A scaling");
  await approveCurrentDraft(page, spaces.scaling, scalingId);

  await createMeasurementDraft(page, spaces.curves, currentCurveId, "E2E current probe transfer");
  await page.getByLabel("Curve type").selectOption("current_probe_transfer");
  await importCurveCsv(
    page,
    "frequency_hz,correction_db\n10000000,0\n100000000,0.8\n1000000000,1.6"
  );
  await approveCurrentDraft(page, spaces.curves, currentCurveId, { save: true });

  await createMeasurementDraft(page, spaces.curves, cableCurveId, "E2E RF cable loss");
  await importCurveCsv(
    page,
    "frequency_hz,correction_db\n10000000,0.2\n100000000,1.2\n1000000000,2.2"
  );
  await saveCurrentDraft(page, spaces.curves, cableCurveId);
  await page.getByRole("button", { name: "Evaluation" }).click();
  await page.getByLabel("Frequency Hz").fill("100000000");
  const evaluationResponse = page.waitForResponse((response) =>
    response.url().includes(`/api/v1/engineering-curves/${cableCurveId}/revisions/`) &&
    response.url().endsWith("/evaluate") &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Evaluer la courbe/ }).click();
  const evaluatedCable = await evaluationResponse;
  expect(evaluatedCable.ok()).toBeTruthy();
  const evaluatedCableBody = await evaluatedCable.json();
  expect(evaluatedCableBody.evaluation.values.correction_db).toBe(1.2);
  await expect(page.locator(".editorPane")).toContainText("correction_db");
  await approveCurrentDraft(page, spaces.curves, cableCurveId);

  await createMeasurementDraft(page, spaces.sensors, sensorId, "E2E current probe 10mV/A");
  await sectionButton(page, "Scaling").click();
  await expect(page.getByRole("cell", { name: scalingId, exact: true })).toBeVisible();
  await sectionButton(page, "Courbes de correction").click();
  await expect(page.getByRole("cell", { name: currentCurveId, exact: true })).toBeVisible();
  await approveCurrentDraft(page, spaces.sensors, sensorId);

  await createMeasurementDraft(page, spaces.daq, daqId, "E2E DAQ AI +/-10V");
  await approveCurrentDraft(page, spaces.daq, daqId);

  await createMeasurementDraft(page, spaces.recipes, recipeId, "E2E current_A logical channel");
  await sectionButton(page, "Chaine de mesure").click();
  const chain = page.locator(".chainSummary");
  await expect(chain).toContainText(daqId);
  await expect(chain).toContainText(sensorId);
  await expect(chain).toContainText(scalingId);
  await expect(chain).toContainText("current_A [A]");
  await approveCurrentDraft(page, spaces.recipes, recipeId);

  await expectApproved(request, spaces.scaling.collection, scalingId);
  await expectApproved(request, spaces.curves.collection, currentCurveId);
  await expectApproved(request, spaces.curves.collection, cableCurveId);
  await expectApproved(request, spaces.sensors.collection, sensorId);
  await expectApproved(request, spaces.daq.collection, daqId);
  const recipe = await expectApproved(request, spaces.recipes.collection, recipeId);
  expect(recipe.item.current_approved_revision.definition.output_channel_name).toBe("current_A");

  const approvedCableRevision = `${cableCurveId}-rev-0001`;
  const apiEvaluation = await request.post(
    `/api/v1/engineering-curves/${cableCurveId}/revisions/${approvedCableRevision}/evaluate`,
    { data: { axis_values: { frequency: 100000000 } } }
  );
  expect(apiEvaluation.ok()).toBeTruthy();
  const evaluationBody = await apiEvaluation.json();
  expect(evaluationBody.evaluation.values.correction_db).toBe(1.2);
  expect(evaluationBody.evaluation.extrapolated).toBe(false);
});

async function createMeasurementDraft(
  page: Page,
  space: MeasurementSpace,
  entityId: string,
  label: string
) {
  await page.locator(".equipmentSubnav").getByRole("button", { name: space.tab }).click();
  await expect(page.locator(".measurementHeader").getByRole("heading", { name: space.tab })).toBeVisible();
  const customIdInput = page.getByLabel("Nouvel ID");
  if (!(await customIdInput.isVisible())) {
    await page.getByText("Identifiant personnalisé").click();
  }
  await customIdInput.fill(entityId);
  await page.getByLabel("Libellé / modèle").fill(label);
  const createResponse = page.waitForResponse((response) =>
    response.url().endsWith(`/api/v1/${space.collection}`) &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: space.createButton }).click();
  expect((await createResponse).ok()).toBeTruthy();
  await expect(page.locator(".equipmentStudio").getByRole("heading", { name: label })).toBeVisible();
}

async function approveCurrentDraft(
  page: Page,
  space: MeasurementSpace,
  entityId: string,
  options: { save?: boolean } = {}
) {
  const validationResponse = page.waitForResponse((response) =>
    response.url().endsWith("/validate") && response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Valider/ }).click();
  expect((await validationResponse).ok()).toBeTruthy();
  await expect(page.getByText("Definition valide")).toBeVisible();

  if (options.save) {
    const saveResponse = page.waitForResponse((response) =>
      response.url().includes(`/api/v1/${space.collection}/${entityId}/revisions/`) &&
      response.url().endsWith("/definition") &&
      response.request().method() === "PUT"
    );
    await page.getByRole("button", { name: /Sauvegarder/ }).click();
    expect((await saveResponse).ok()).toBeTruthy();
  }

  const submitResponse = page.waitForResponse((response) =>
    response.url().includes(`/api/v1/${space.collection}/${entityId}/revisions/`) &&
    response.url().includes("/transitions/submit-for-review") &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Soumettre/ }).click();
  expect((await submitResponse).ok()).toBeTruthy();
  await expect(page.locator(".studioTitleMeta .status", { hasText: "En revue" })).toBeVisible();

  const approveResponse = page.waitForResponse((response) =>
    response.url().includes(`/api/v1/${space.collection}/${entityId}/revisions/`) &&
    response.url().includes("/transitions/approve") &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Approuver/ }).click();
  expect((await approveResponse).ok()).toBeTruthy();
  await expect(page.locator(".studioTitleMeta .status", { hasText: "Approuve" })).toBeVisible();
}

async function saveCurrentDraft(page: Page, space: MeasurementSpace, entityId: string) {
  const saveResponse = page.waitForResponse((response) =>
    response.url().includes(`/api/v1/${space.collection}/${entityId}/revisions/`) &&
    response.url().endsWith("/definition") &&
    response.request().method() === "PUT"
  );
  await page.getByRole("button", { name: /Sauvegarder/ }).click();
  expect((await saveResponse).ok()).toBeTruthy();
}

async function importCurveCsv(page: Page, csv: string) {
  await sectionButton(page, "Table courbe").click();
  const editor = page.locator(".editorPane");
  const firstDataRow = csv.trim().split(/\r?\n/)[1].split(",");
  await editor.locator('textarea[placeholder="frequency_hz,correction_db"]').fill(csv);
  await editor.getByRole("button", { name: /Import CSV/ }).click();
  await expect(editor.locator("tbody tr").first().locator("input").nth(0)).toHaveValue(firstDataRow[0]);
  await expect(editor.locator("tbody tr").first().locator("input").nth(1)).toHaveValue(firstDataRow[1]);
  await expect(editor.getByRole("img", { name: "1D curve plot" })).toBeVisible();
}

function sectionButton(page: Page, name: string) {
  return page.locator(".sectionNav").getByRole("button", { name });
}

async function expectApproved(request: APIRequestContext, collection: string, entityId: string) {
  const response = await request.get(`/api/v1/${collection}/${entityId}`);
  expect(response.ok()).toBeTruthy();
  const body = await response.json();
  expect(body.item.current_approved_revision.status).toBe("approved");
  return body;
}
