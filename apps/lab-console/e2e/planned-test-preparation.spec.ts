import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const viewports = [
  { width: 1440, height: 900 },
  { width: 1280, height: 720 }
];

test("an operator resolves a blocked preparation before starting the planned test", async ({
  page,
  request
}) => {
  const suffix = Date.now().toString(36).toUpperCase();
  const projectCode = `CEM-PREP-${suffix}`;
  const itemCode = `PLAN-PREP-${suffix}`;
  const methodId = `METHOD-PREP-${suffix}`;
  const methodTitle = `Vérification RF ${suffix}`;
  const setupId = `SETUP-PREP-${suffix}`;
  const setupLabel = `Chaîne RF ${suffix}`;
  const generatorModelId = `EQM-GEN-${suffix}`;
  const meterModelId = `EQM-PM-${suffix}`;
  const generatorAssetId = `GEN-${suffix}`;
  const meterAssetId = `PM-${suffix}`;
  const plannedDate = addDays(mondayFor(new Date()), 3);

  const generatorModel = await createApprovedPresetModel(request, {
    modelId: generatorModelId,
    presetId: "rf_generator",
    manufacturer: "Locus Demo",
    modelName: `Generator ${suffix}`,
    operationPrefix: `prep-generator-${suffix}`
  });
  const meterModel = await createApprovedPresetModel(request, {
    modelId: meterModelId,
    presetId: "rf_power_meter",
    manufacturer: "Locus Demo",
    modelName: `Wattmeter ${suffix}`,
    operationPrefix: `prep-meter-${suffix}`
  });
  const generator = await registerInstrument(request, {
    assetId: generatorAssetId,
    family: "Générateur RF",
    categoryCode: "rf_signal_generator",
    serialNumber: `GEN-${suffix}`,
    model: generatorModel,
    operationId: `op-prep-register-generator-${suffix}`
  });
  const meter = await registerInstrument(request, {
    assetId: meterAssetId,
    family: "Wattmètre RF",
    categoryCode: "rf_power_meter",
    serialNumber: `PM-${suffix}`,
    model: meterModel,
    operationId: `op-prep-register-meter-${suffix}`
  });
  await createApprovedMethod(request, {
    methodId,
    title: methodTitle,
    requiredCategory: "rf_power_meter",
    operationPrefix: `prep-method-${suffix}`
  });
  await createReadyStation(request, {
    setupId,
    label: setupLabel,
    plannedDate,
    generator,
    meter,
    generatorModel,
    meterModel,
    operationPrefix: `prep-station-${suffix}`
  });
  await createConfirmedSchedule(request, {
    projectCode,
    itemCode,
    plannedDate,
    title: `Vérification RF du convertisseur ${suffix}`,
    operator: `Alice ${suffix}`,
    operationPrefix: `prep-project-${suffix}`
  });

  await page.setViewportSize(viewports[0]);
  await page.goto("/lab/");
  await page.getByRole("button", { name: "Planning du laboratoire" }).click();
  await page
    .getByRole("button", {
      name: `Ouvrir Vérification RF du convertisseur ${suffix}, dossier ${projectCode}`
    })
    .click();
  const slotDialog = page.getByRole("dialog");
  await expect(slotDialog.getByText("À préparer", { exact: true }).first()).toBeVisible();
  await expect(slotDialog.getByRole("button", { name: "Démarrer l'essai" })).toBeDisabled();
  await slotDialog.getByRole("button", { name: "Préparer l'essai" }).click();

  const preparationDialog = page.getByRole("dialog", { name: "Préparer l'essai" });
  await preparationDialog
    .getByRole("combobox", { name: "Méthode" })
    .selectOption({ label: `${methodTitle} · version 1` });
  await preparationDialog
    .getByRole("combobox", { name: "Montage" })
    .selectOption({ label: `${setupLabel} · Poste CEM ${suffix}` });

  const blockedResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(
        `/api/v1/projects/${projectCode}/schedule-items/${itemCode}/preparation/assessments`
      ) && response.request().method() === "POST"
  );
  await preparationDialog.getByRole("button", { name: "Vérifier la préparation" }).click();
  expect((await blockedResponse).ok()).toBeTruthy();
  await expect(preparationDialog.getByText("Préparation bloquée", { exact: true })).toBeVisible();
  await expect(preparationDialog.getByText(/n'a pas de matériel affecté/)).toBeVisible();
  await expect(preparationDialog.getByText("Contrôle n° 1", { exact: true }).last()).toBeVisible();
  await assertNoHorizontalOverflow(page);
  await expect(page.locator("body")).toHaveCSS("overflow", "hidden");
  await captureReleaseScreenshot(page, "preparation-bloquee-1440x900.png");

  await preparationDialog
    .getByRole("combobox", { name: "Matériel pour Wattmètre RF" })
    .selectOption({
      label: `Wattmètre RF · Locus Demo Wattmeter ${suffix} · n° série PM-${suffix}`
    });
  const readyResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(
        `/api/v1/projects/${projectCode}/schedule-items/${itemCode}/preparation/assessments`
      ) && response.request().method() === "POST"
  );
  await preparationDialog.getByRole("button", { name: "Vérifier la préparation" }).click();
  expect((await readyResponse).ok()).toBeTruthy();
  await expect(preparationDialog.getByText("Prêt à démarrer", { exact: true })).toBeVisible();
  await expect(preparationDialog.getByText("Contrôle n° 2", { exact: true }).last()).toBeVisible();
  await expect(preparationDialog.getByText("Contrôle n° 1", { exact: true }).last()).toBeVisible();

  for (const viewport of viewports) {
    await page.setViewportSize(viewport);
    await assertNoHorizontalOverflow(page);
    await expect(page.locator("body")).toHaveCSS("overflow", "hidden");
    await captureReleaseScreenshot(
      page,
      `preparation-prete-${viewport.width}x${viewport.height}.png`
    );
  }

  await preparationDialog.getByRole("button", { name: "Retour au créneau" }).click();
  await expect(slotDialog.getByRole("button", { name: "Démarrer l'essai" })).toBeEnabled();
  const startResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(
        `/api/v1/projects/${projectCode}/schedule-items/${itemCode}/transitions/start`
      ) && response.request().method() === "POST"
  );
  await slotDialog.getByRole("button", { name: "Démarrer l'essai" }).click();
  expect((await startResponse).ok()).toBeTruthy();
  await expect(slotDialog.getByText("En cours", { exact: true })).toBeVisible();
  await expect(slotDialog.getByText("À revérifier", { exact: true })).toHaveCount(0);
  await captureReleaseScreenshot(page, "essai-demarre-1280x720.png");

  const scheduleResponse = await request.get(
    `/api/v1/projects/${projectCode}/schedule-items`
  );
  expect(scheduleResponse.ok(), await scheduleResponse.text()).toBeTruthy();
  expect((await scheduleResponse.json()).schedule_items[0].status).toBe("in_progress");

  const revisionsResponse = await request.get(
    `/api/v1/projects/${projectCode}/schedule-items/${itemCode}/preparation/revisions`
  );
  expect(revisionsResponse.ok(), await revisionsResponse.text()).toBeTruthy();
  const revisions = (await revisionsResponse.json()).revisions;
  expect(revisions).toHaveLength(2);
  expect(revisions.map((revision: { recorded_state: string }) => revision.recorded_state)).toEqual(
    ["ready", "blocked"]
  );

  const auditResponse = await request.get(`/api/v1/projects/${projectCode}/audit-events`);
  expect(auditResponse.ok(), await auditResponse.text()).toBeTruthy();
  const auditText = JSON.stringify(await auditResponse.json());
  expect(auditText).toContain("planned_test_preparation_assessed");
  expect(auditText).toContain(`${itemCode}-prep-rev-0002`);

  const outboxResponse = await request.get("/api/v1/sync/outbox");
  expect(outboxResponse.ok(), await outboxResponse.text()).toBeTruthy();
  expect(JSON.stringify(await outboxResponse.json())).toContain(
    "planned_test_preparation_assessed"
  );
});

interface ApprovedModel {
  modelId: string;
  revisionId: string;
  checksum: string;
  manufacturer: string;
  modelName: string;
}

interface RegisteredInstrument {
  asset_id: string;
  revision: string;
}

async function createApprovedPresetModel(
  request: APIRequestContext,
  input: {
    modelId: string;
    presetId: string;
    manufacturer: string;
    modelName: string;
    operationPrefix: string;
  }
): Promise<ApprovedModel> {
  const created = await request.post("/api/v1/equipment-models/from-preset", {
    data: {
      preset_id: input.presetId,
      equipment_model_id: input.modelId,
      manufacturer: input.manufacturer,
      model_name: input.modelName,
      actor: "E2E catalogue",
      reason: "Préparer le matériel du scénario de pré-vol",
      is_demo: true,
      operation_id: `op-${input.operationPrefix}-create`
    }
  });
  expect(created.ok(), await created.text()).toBeTruthy();
  const createdBody = await created.json();
  const revisionId = createdBody.revision.revision_id as string;
  for (const [transition, actor] of [
    ["submit-for-review", "E2E reviewer"],
    ["approve", "E2E approver"]
  ]) {
    const response = await request.post(
      `/api/v1/equipment-models/${input.modelId}/revisions/${revisionId}/transitions/${transition}`,
      {
        data: {
          actor,
          reason: "Valider le modèle du scénario de pré-vol",
          operation_id: `op-${input.operationPrefix}-${transition}`
        }
      }
    );
    expect(response.ok(), await response.text()).toBeTruthy();
  }
  const aggregateResponse = await request.get(`/api/v1/equipment-models/${input.modelId}`);
  expect(aggregateResponse.ok(), await aggregateResponse.text()).toBeTruthy();
  const aggregate = (await aggregateResponse.json()).equipment_model;
  return {
    modelId: input.modelId,
    revisionId,
    checksum: aggregate.current_approved_revision.definition_checksum,
    manufacturer: input.manufacturer,
    modelName: input.modelName
  };
}

async function registerInstrument(
  request: APIRequestContext,
  input: {
    assetId: string;
    family: string;
    categoryCode: string;
    serialNumber: string;
    model: ApprovedModel;
    operationId: string;
  }
): Promise<RegisteredInstrument> {
  const response = await request.post("/api/v1/metrology/instruments", {
    data: {
      asset_id: input.assetId,
      family: input.family,
      category_code: input.categoryCode,
      equipment_model_id: input.model.modelId,
      equipment_model_revision_id: input.model.revisionId,
      equipment_model_checksum: input.model.checksum,
      manufacturer: input.model.manufacturer,
      model: input.model.modelName,
      serial_number: input.serialNumber,
      part_number: input.model.modelName,
      calibration_requirement: "not_required",
      serviceability_status: "usable",
      serviceability_reason: "Matériel E2E contrôlé",
      capabilities: {},
      actor: "E2E métrologie",
      reason: "Enregistrer le matériel du scénario de pré-vol",
      operation_id: input.operationId
    }
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  return (await response.json()).instrument;
}

async function createApprovedMethod(
  request: APIRequestContext,
  input: {
    methodId: string;
    title: string;
    requiredCategory: string;
    operationPrefix: string;
  }
) {
  const created = await request.post("/api/v1/test-templates", {
    data: {
      template_id: input.methodId,
      title: input.title,
      category_code: "emission_conducted",
      definition: {
        definition_schema_version: "emc-locus.test-template-definition.v1",
        title: input.title,
        description: "Méthode contrôlée pour la préparation E2E.",
        measurement_axis: "frequency_sweep",
        standard_references: ["MÉTHODE INTERNE E2E-RF-01"],
        variables: [
          {
            variable_id: "frequency_hz",
            label: "Fréquence",
            value_type: "integer",
            default_value: 1_000_000,
            constraints: {
              required: true,
              dimensionless: false,
              unit: "Hz",
              minimum: 100_000,
              maximum: 1_000_000_000,
              enum_values: []
            }
          }
        ],
        lock_policy: [
          { variable_id: "frequency_hz", policy: "editable_until_execution" }
        ],
        instrumentation_chain: [
          {
            slot_id: "measurement_receiver",
            label: "Wattmètre RF",
            required_category: input.requiredCategory,
            required: true,
            calibration_requirement: "not_required",
            substitution_policy: "same_category",
            depends_on_slots: []
          }
        ],
        entry_step_id: "finish",
        sequence: [
          {
            step_id: "finish",
            order: 10,
            kind: "finish",
            label: "Clôturer",
            required_slots: [],
            branches: []
          }
        ],
        limits: [],
        post_processing: [],
        method_parameters: {}
      },
      actor: "E2E méthodiste",
      reason: "Créer la méthode du scénario de pré-vol",
      operation_id: `op-${input.operationPrefix}-create`
    }
  });
  expect(created.ok(), await created.text()).toBeTruthy();
  const revisionId = (await created.json()).revision.revision_id as string;
  for (const [transition, actor] of [
    ["submit-for-review", "E2E reviewer"],
    ["approve", "E2E approver"]
  ]) {
    const response = await request.post(
      `/api/v1/test-templates/${input.methodId}/revisions/${revisionId}/transitions/${transition}`,
      {
        data: {
          actor,
          reason: "Valider la méthode du scénario de pré-vol",
          operation_id: `op-${input.operationPrefix}-${transition}`
        }
      }
    );
    expect(response.ok(), await response.text()).toBeTruthy();
  }
}

async function createReadyStation(
  request: APIRequestContext,
  input: {
    setupId: string;
    label: string;
    plannedDate: string;
    generator: RegisteredInstrument;
    meter: RegisteredInstrument;
    generatorModel: ApprovedModel;
    meterModel: ApprovedModel;
    operationPrefix: string;
  }
) {
  const created = await request.post("/api/v1/station-setups", {
    data: {
      setup_id: input.setupId,
      label: input.label,
      station_label: `Poste CEM ${input.setupId.replace("SETUP-PREP-", "")}`,
      planned_use_on: input.plannedDate,
      execution_mode: "investigation",
      actor: "E2E technicien",
      reason: "Créer le montage du scénario de pré-vol",
      operation_id: `op-${input.operationPrefix}-create`
    }
  });
  expect(created.ok(), await created.text()).toBeTruthy();
  const aggregate = (await created.json()).station_setup;
  const draft = aggregate.active_draft_revision;
  const definition = {
    definition_schema_version: "emc-locus.station-measurement-setup-definition.v1",
    setup_id: input.setupId,
    label: input.label,
    station_label: `Poste CEM ${input.setupId.replace("SETUP-PREP-", "")}`,
    planned_use_on: input.plannedDate,
    execution_mode: "investigation",
    asset_bindings: [
      stationBinding("rf_generator", "Générateur RF", input.generator, input.generatorModel),
      stationBinding("power_meter", "Wattmètre RF", input.meter, input.meterModel)
    ],
    connections: [
      {
        connection_id: "rf_verification_path",
        label: "Sortie générateur vers entrée wattmètre",
        from: { binding_id: "rf_generator", port_id: "RF_OUT" },
        to: { binding_id: "power_meter", port_id: "RF_IN" }
      }
    ],
    correction_selections: [],
    notes: { purpose: "Pré-vol E2E" }
  };
  const saved = await request.put(
    `/api/v1/station-setups/${input.setupId}/revisions/${draft.revision_id}/definition`,
    {
      data: {
        expected_definition_checksum: draft.definition_checksum,
        definition,
        actor: "E2E technicien",
        reason: "Affecter et raccorder les matériels du scénario de pré-vol",
        operation_id: `op-${input.operationPrefix}-save`
      }
    }
  );
  expect(saved.ok(), await saved.text()).toBeTruthy();
  const savedDraft = (await saved.json()).station_setup.active_draft_revision;
  const readiness = await request.get(
    `/api/v1/station-setups/${input.setupId}/revisions/${draft.revision_id}/readiness`
  );
  expect(readiness.ok(), await readiness.text()).toBeTruthy();
  expect((await readiness.json()).readiness.ready).toBe(true);
  const ready = await request.post(
    `/api/v1/station-setups/${input.setupId}/revisions/${draft.revision_id}/transitions/ready`,
    {
      data: {
        expected_definition_checksum: savedDraft.definition_checksum,
        actor: "E2E technicien",
        reason: "Valider le montage du scénario de pré-vol",
        operation_id: `op-${input.operationPrefix}-ready`
      }
    }
  );
  expect(ready.ok(), await ready.text()).toBeTruthy();
}

function stationBinding(
  bindingId: string,
  roleLabel: string,
  instrument: RegisteredInstrument,
  model: ApprovedModel
) {
  return {
    binding_id: bindingId,
    role_label: roleLabel,
    asset_id: instrument.asset_id,
    asset_revision: instrument.revision,
    equipment_model_id: model.modelId,
    equipment_model_revision_id: model.revisionId,
    equipment_model_checksum: model.checksum
  };
}

async function createConfirmedSchedule(
  request: APIRequestContext,
  input: {
    projectCode: string;
    itemCode: string;
    plannedDate: string;
    title: string;
    operator: string;
    operationPrefix: string;
  }
) {
  const created = await request.post("/api/v1/projects", {
    data: {
      code: input.projectCode,
      customer_name: "Industries Horizon",
      execution_mode: "investigation",
      actor: "E2E responsable laboratoire",
      reason: "Créer le dossier du scénario de pré-vol",
      operation_id: `op-${input.operationPrefix}-create`
    }
  });
  expect(created.ok(), await created.text()).toBeTruthy();
  const reviewResponse = await request.get(
    `/api/v1/projects/${input.projectCode}/contract-review`
  );
  expect(reviewResponse.ok(), await reviewResponse.text()).toBeTruthy();
  const requiredItems = (await reviewResponse.json()).contract_review.required_items as string[];
  for (const [index, item] of requiredItems.entries()) {
    const completed = await request.post(
      `/api/v1/projects/${input.projectCode}/contract-review/items/${item}/complete`,
      {
        data: {
          actor: "E2E responsable laboratoire",
          comment: "Vérifié pour le scénario de pré-vol",
          operation_id: `op-${input.operationPrefix}-review-${index}`
        }
      }
    );
    expect(completed.ok(), await completed.text()).toBeTruthy();
  }
  const advanced = await request.post(
    `/api/v1/projects/${input.projectCode}/transitions/to-test-planning`,
    {
      data: {
        actor: "E2E responsable laboratoire",
        reason: "Revue terminée pour le scénario de pré-vol",
        operation_id: `op-${input.operationPrefix}-plan`
      }
    }
  );
  expect(advanced.ok(), await advanced.text()).toBeTruthy();
  const scheduled = await request.post(
    `/api/v1/projects/${input.projectCode}/schedule-items`,
    {
      data: {
        item_code: input.itemCode,
        title: input.title,
        planned_start_at: `${input.plannedDate}T09:00`,
        planned_end_at: `${input.plannedDate}T12:00`,
        assigned_operator: input.operator,
        location: `Poste CEM ${input.projectCode.replace("CEM-PREP-", "")}`,
        equipment_under_test: "Convertisseur Horizon HCU-4",
        actor: "E2E responsable laboratoire",
        reason: "Planifier le scénario de pré-vol",
        operation_id: `op-${input.operationPrefix}-schedule`
      }
    }
  );
  expect(scheduled.ok(), await scheduled.text()).toBeTruthy();
  const scheduleItem = (await scheduled.json()).schedule_item;
  const confirmed = await request.post(
    `/api/v1/projects/${input.projectCode}/schedule-items/${input.itemCode}/transitions/confirm`,
    {
      data: {
        expected_revision: scheduleItem.revision,
        actor: "E2E responsable laboratoire",
        reason: "Confirmer le créneau du scénario de pré-vol",
        operation_id: `op-${input.operationPrefix}-confirm`
      }
    }
  );
  expect(confirmed.ok(), await confirmed.text()).toBeTruthy();
}

function mondayFor(date: Date): string {
  const monday = new Date(date.getFullYear(), date.getMonth(), date.getDate(), 12);
  const day = monday.getDay();
  monday.setDate(monday.getDate() - (day === 0 ? 6 : day - 1));
  return isoDate(monday);
}

function addDays(value: string, count: number): string {
  const [year, month, day] = value.split("-").map(Number);
  const date = new Date(year, month - 1, day, 12);
  date.setDate(date.getDate() + count);
  return isoDate(date);
}

function isoDate(date: Date): string {
  return [
    date.getFullYear(),
    String(date.getMonth() + 1).padStart(2, "0"),
    String(date.getDate()).padStart(2, "0")
  ].join("-");
}

async function assertNoHorizontalOverflow(page: Page) {
  const dimensions = await page.evaluate(() => ({
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth
  }));
  expect(dimensions.scrollWidth).toBeLessThanOrEqual(dimensions.clientWidth);
}

async function captureReleaseScreenshot(page: Page, name: string) {
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(80);
  const body = await page.screenshot({ animations: "disabled", fullPage: false });
  const evidenceDirectory = path.resolve(process.cwd(), "../../docs/ux/0.21.0/screenshots");
  await mkdir(evidenceDirectory, { recursive: true });
  await writeFile(path.join(evidenceDirectory, name), body);
}
