import { expect, type Page, type APIRequestContext, test } from "@playwright/test";

type MeasurementSpace = {
  tab: string;
  heading?: string;
  collection: string;
  createButton: string;
};

const spaces = {
  scaling: {
    tab: "Conversions temporelles",
    collection: "scaling-profiles",
    createButton: "Nouvelle conversion"
  },
  curves: {
    tab: "Réponses fréquentielles",
    collection: "engineering-curves",
    createButton: "Nouvelle réponse"
  },
  sensors: {
    tab: "Capteurs / transducteurs",
    heading: "Capteurs et transducteurs",
    collection: "sensor-definitions",
    createButton: "Nouveau capteur"
  },
  daq: {
    tab: "Voies DAQ",
    heading: "Entrées / sorties DAQ",
    collection: "daq-channel-profiles",
    createButton: "Nouvelle voie DAQ"
  },
  recipes: {
    tab: "Chaînes d'acquisition",
    collection: "acquisition-channel-recipes",
    createButton: "Nouvelle chaîne"
  }
} satisfies Record<string, MeasurementSpace>;

test("measurement engineering workflow creates approved channel recipe", async ({ page, request }) => {
  const suffix = Date.now().toString(36).toUpperCase();
  const scalingLabel = `E2E conversion 10 mV/A ${suffix}`;
  const currentCurveLabel = `E2E transfert pince courant ${suffix}`;
  const cableCurveLabel = `E2E pertes câble RF ${suffix}`;
  const sensorLabel = `E2E pince courant 10 mV/A ${suffix}`;
  const daqLabel = `E2E voie DAQ ±10 V ${suffix}`;
  const recipeLabel = `E2E chaîne courant ${suffix}`;

  await page.goto("/lab/");
  await page.getByRole("button", { name: "Équipements" }).click();
  await page.getByRole("button", { name: "Signaux et corrections" }).click();

  const scalingId = await createMeasurementDraft(page, spaces.scaling, scalingLabel);
  await approveCurrentDraft(page, spaces.scaling, scalingId);

  const currentCurveId = await createMeasurementDraft(page, spaces.curves, currentCurveLabel);
  await page.getByLabel("Type de réponse").selectOption("current_probe_transfer");
  await importCurveCsv(
    page,
    "frequence_hz,amplitude_db\n10000000,0\n100000000,0.8\n1000000000,1.6"
  );
  await approveCurrentDraft(page, spaces.curves, currentCurveId, { save: true });

  const cableCurveId = await createMeasurementDraft(page, spaces.curves, cableCurveLabel);
  await importCurveCsv(
    page,
    "frequence_hz,amplitude_db\n10000000,0.2\n100000000,1.2\n1000000000,2.2"
  );
  await saveCurrentDraft(page, spaces.curves, cableCurveId);
  await page.getByRole("button", { name: "Vérification ponctuelle" }).click();
  await page.getByLabel("Fréquence (Hz)").fill("100000000");
  const evaluationResponse = page.waitForResponse((response) =>
    response.url().includes(`/api/v1/engineering-curves/${cableCurveId}/revisions/`) &&
    response.url().endsWith("/evaluate") &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: /Calculer la correction/ }).click();
  const evaluatedCable = await evaluationResponse;
  expect(evaluatedCable.ok()).toBeTruthy();
  const evaluatedCableBody = await evaluatedCable.json();
  expect(evaluatedCableBody.evaluation.values.amplitude_correction_db).toBe(1.2);
  await expect(page.locator(".editorPane")).toContainText("Correction d’amplitude");
  await approveCurrentDraft(page, spaces.curves, cableCurveId);

  const sensorId = await createMeasurementDraft(page, spaces.sensors, sensorLabel);
  await sectionButton(page, "Conversion temporelle").click();
  await expect(page.getByRole("cell", { name: scalingLabel, exact: true })).toBeVisible();
  await sectionButton(page, "Réponse fréquentielle").click();
  if (await page.getByRole("cell", { name: currentCurveLabel, exact: true }).count() === 0) {
    await page.locator(".editorPane select").selectOption({ label: `${currentCurveLabel} — révision 1` });
    await page.locator(".editorPane").getByRole("button", { name: "Ajouter" }).click();
  }
  await expect(page.getByRole("cell", { name: currentCurveLabel, exact: true })).toBeVisible();
  await approveCurrentDraft(page, spaces.sensors, sensorId);

  const daqId = await createMeasurementDraft(page, spaces.daq, daqLabel);
  await approveCurrentDraft(page, spaces.daq, daqId);

  const recipeId = await createMeasurementDraft(page, spaces.recipes, recipeLabel);
  await sectionButton(page, "Chaîne de mesure").click();
  const chain = page.locator(".chainSummary");
  await expect(chain).toContainText(daqLabel);
  await expect(chain).toContainText(sensorLabel);
  await expect(chain).toContainText(scalingLabel);
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
  expect(evaluationBody.evaluation.values.amplitude_correction_db).toBe(1.2);
  expect(evaluationBody.evaluation.extrapolated).toBe(false);
});

async function createMeasurementDraft(
  page: Page,
  space: MeasurementSpace,
  label: string
) {
  await page.locator(".equipmentSubnav").getByRole("button", { name: space.tab }).click();
  await expect(page.locator(".measurementHeader").getByRole("heading", { name: space.heading ?? space.tab })).toBeVisible();
  await page.locator(".measurementCreateBar input").fill(label);
  const createResponse = page.waitForResponse((response) =>
    response.url().endsWith(`/api/v1/${space.collection}`) &&
    response.request().method() === "POST"
  );
  await page.getByRole("button", { name: space.createButton }).click();
  const response = await createResponse;
  expect(response.ok()).toBeTruthy();
  const body = await response.json();
  await expect(page.locator(".equipmentStudio").getByRole("heading", { name: label })).toBeVisible();
  return (body.aggregate ?? body.item).identity.entity_id as string;
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
  await page.getByRole("button", { name: /Vérifier la définition/ }).click();
  expect((await validationResponse).ok()).toBeTruthy();
  await expect(page.getByText("Définition prête à être soumise")).toBeVisible();

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
  await expect(page.locator(".studioTitleMeta .status", { hasText: "Approuvé" })).toBeVisible();
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
  await sectionButton(page, "Amplitude / phase").click();
  const editor = page.locator(".editorPane");
  const firstDataRow = csv.trim().split(/\r?\n/)[1].split(",");
  await editor.locator('textarea[placeholder^="frequence_hz,"]').fill(csv);
  await editor.getByRole("button", { name: /Importer CSV/ }).click();
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
