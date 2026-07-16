import { expect, type APIRequestContext, type Page, test } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const screenshotViewports = [
  { width: 1440, height: 900 },
  { width: 1280, height: 720 }
];

test("serialized RF cable stays blocked until its measured loss is reviewed and active", async ({ page, request }) => {
  const suffix = Date.now().toString(36).toUpperCase();
  const modelId = `EQM-E2E-CABLE-${suffix}`;
  const firstAssetId = `CBL-014-${suffix}`;
  const secondAssetId = `CBL-015-${suffix}`;
  const model = await createApprovedEquipmentModel(request, modelId, cableModelDefinition());
  await registerMaterial(request, firstAssetId, `84752-${suffix}`, modelId, model);

  await page.goto("/lab/");
  await page.getByRole("button", { name: "Équipements" }).click();
  await page.getByRole("button", { name: new RegExp(`EMC Locus E2E Cable 1 m`) }).click();
  await page.getByRole("button", { name: "Entrées, sorties et corrections" }).click();
  await expect(page.getByRole("textbox", { name: "Que faut-il corriger ?" })).toHaveValue("Pertes du câble");
  await captureAtDesktopSizes(page, "model-correction-requirement");

  await page.getByRole("button", { name: "Matériels réels" }).click();
  await page.getByRole("button", { name: new RegExp(firstAssetId) }).click();
  await expect(page.getByText("Correction manquante")).toBeVisible();
  await expect(page.getByText("Non prêt pour un essai")).toBeVisible();
  await captureAtDesktopSizes(page, "physical-item-missing-correction");

  await page.getByRole("button", { name: "Mesurer cette correction" }).click();
  await page.getByLabel(/Source de la correction/).selectOption("calibration");
  await page.getByLabel(/Laboratoire ou prestataire/).fill("Laboratoire CEM interne");
  await page.getByLabel(/Méthode utilisée/).fill("MET-RF-CABLE-001");
  await page.getByLabel("Référence du certificat ou feuillet").fill(`CAL-CBL-${suffix}`);
  await page.getByLabel(/Tableau mesuré/).fill(
    "frequence_hz,amplitude_db\n1000000,0.12\n100000000,0.86\n1000000000,2.91"
  );
  await page.locator('.characterizationForm input[type="file"]').setInputFiles({
    name: `CAL-CBL-${suffix}.pdf`,
    mimeType: "application/pdf",
    buffer: Buffer.from("%PDF-1.4\nEMC Locus cable correction evidence")
  });
  await expect(page.getByRole("img", { name: /Correction de 0.12 à 2.91 dB/ })).toBeVisible();
  await captureAtDesktopSizes(page, "correction-import");
  const characterizationResponse = page.waitForResponse((response) =>
    response.url().endsWith(`/api/v1/metrology/instruments/${firstAssetId}/characterizations`)
      && response.request().method() === "POST"
  );
  await page.getByRole("button", { name: "Enregistrer puis préparer la revue" }).click();
  expect((await characterizationResponse).ok()).toBeTruthy();
  await expect(page.getByText("Brouillon à soumettre")).toBeVisible();

  await page.getByRole("button", { name: "Soumettre pour revue" }).click();
  await expect(page.getByText("En attente de revue")).toBeVisible();
  await expect(page.getByRole("button", { name: "Refuser" })).toBeVisible();
  await captureAtDesktopSizes(page, "correction-review");
  await page.getByRole("button", { name: "Approuver et activer" }).click();
  await expect(page.getByText("Active pour ce matériel")).toBeVisible();
  await expect(page.getByText("Prêt pour un essai")).toBeVisible();
  await expect(page.getByLabel("Corrections requises").getByText(`CAL-CBL-${suffix}`)).toBeVisible();
  await captureAtDesktopSizes(page, "physical-item-ready");

  await registerMaterial(request, secondAssetId, `84753-${suffix}`, modelId, model);
  await page.reload();
  await page.getByRole("button", { name: "Équipements" }).click();
  await page.getByRole("button", { name: "Matériels réels" }).click();
  await page.getByRole("button", { name: new RegExp(secondAssetId) }).click();
  await expect(page.getByText("Correction manquante")).toBeVisible();
  await expect(page.getByText("Non prêt pour un essai")).toBeVisible();
  await expect(page.getByText(`CAL-CBL-${suffix}`)).toHaveCount(0);
});

test("calibrated IEPE sensitivity takes precedence over the nominal model value", async ({ page, request }) => {
  const suffix = Date.now().toString(36).toUpperCase();
  const nominalId = `SCL-E2E-ACC-100MV-G-${suffix}`;
  const nominal = await createApprovedSampleConversion(request, nominalId, "Sensibilité nominale 100 mV/g");
  const modelId = `EQM-E2E-ACC-${suffix}`;
  const assetId = `ACC-003-${suffix}`;
  const model = await createApprovedEquipmentModel(
    request,
    modelId,
    accelerometerModelDefinition(nominalId, nominal.revisionId, nominal.checksum)
  );
  await registerMaterial(request, assetId, `ACC-SN-${suffix}`, modelId, model, {
    family: "IEPE accelerometer",
    manufacturer: "PCB Piezotronics",
    model: "352C33"
  });

  await page.goto("/lab/");
  await page.getByRole("button", { name: "Équipements" }).click();
  await page.getByRole("button", { name: "Matériels réels" }).click();
  await page.getByRole("button", { name: new RegExp(assetId) }).click();
  await expect(page.getByText("Correction manquante")).toBeVisible();
  await page.getByRole("button", { name: "Mesurer cette correction" }).click();
  await page.getByLabel(/Nom de la caractérisation/).fill("Sensibilité étalonnée 102,4 mV/g");
  await page.getByLabel(/Source de la correction/).selectOption("calibration");
  await page.getByLabel(/Laboratoire ou prestataire/).fill("Laboratoire vibration interne");
  await page.getByLabel(/Méthode utilisée/).fill("ISO-16063-21");
  await page.getByLabel("Référence du certificat ou feuillet").fill(`CAL-ACC-${suffix}`);
  await page.getByLabel("Grandeur du résultat").selectOption("acceleration");
  await page.getByLabel(/Facteur de conversion/).fill("95.768");
  await page.getByLabel(/Offset/).fill("0");
  await page.getByRole("button", { name: "Enregistrer puis préparer la revue" }).click();
  await expect(page.getByText("Brouillon à soumettre")).toBeVisible();
  await page.getByRole("button", { name: "Soumettre pour revue" }).click();
  await page.getByRole("button", { name: "Approuver et activer" }).click();

  await expect(page.getByText("Prêt pour un essai")).toBeVisible();
  await expect(page.getByText("Valeur propre à ce matériel")).toBeVisible();
  await expect(page.getByText("Sensibilité nominale 100 mV/g")).toBeVisible();
  await expect(page.getByText(/non sélectionnée/)).toBeVisible();
  await expect(page.getByLabel("Corrections requises").getByText(`CAL-ACC-${suffix}`)).toBeVisible();
  await expect(page.locator(".physicalAssetsLayout")).toContainText("m/s²");
  await expect(page.locator(".physicalAssetsLayout")).not.toContainText("m_per_s2");
  await captureAtDesktopSizes(page, "nominal-versus-calibrated");
});

async function createApprovedEquipmentModel(
  request: APIRequestContext,
  modelId: string,
  definition: Record<string, unknown>
) {
  const created = await request.post("/api/v1/equipment-models", {
    data: {
      equipment_model_id: modelId,
      definition,
      actor: "equipment.e2e",
      reason: "create correction workflow model",
      operation_id: `op-create-${modelId}`
    }
  });
  expect(created.ok(), await created.text()).toBeTruthy();
  const body = await created.json();
  const revision = body.revision ?? body.aggregate.latest_revision;
  await approveRevision(request, "equipment-models", modelId, revision.revision_id);
  return { revisionId: revision.revision_id as string, checksum: revision.definition_checksum as string };
}

async function createApprovedSampleConversion(request: APIRequestContext, entityId: string, label: string) {
  const created = await request.post("/api/v1/scaling-profiles", {
    data: {
      entity_id: entityId,
      definition: {
        definition_schema_version: "emc-locus.scaling-profile-definition.v1",
        scaling_profile_id: entityId,
        label,
        input_quantity: "voltage",
        input_unit: "V",
        output_quantity: "acceleration",
        output_unit: "m_per_s2",
        signal_representation: "time_domain_samples",
        scaling_kind: "linear",
        parameters: { scale: 98.0665, offset: 0 },
        metadata: { quality: "manufacturer_nominal" }
      },
      actor: "measurement.e2e",
      reason: "create nominal accelerometer sensitivity",
      operation_id: `op-create-${entityId}`
    }
  });
  expect(created.ok(), await created.text()).toBeTruthy();
  const body = await created.json();
  const aggregate = body.aggregate ?? body.item;
  const revision = body.revision ?? aggregate.latest_revision;
  await approveRevision(request, "scaling-profiles", entityId, revision.revision_id);
  return { revisionId: revision.revision_id as string, checksum: revision.definition_checksum as string };
}

async function approveRevision(request: APIRequestContext, collection: string, entityId: string, revisionId: string) {
  const submitted = await request.post(
    `/api/v1/${collection}/${entityId}/revisions/${revisionId}/transitions/submit-for-review`,
    { data: { actor: "quality.e2e", reason: "ready for review", operation_id: `op-submit-${entityId}` } }
  );
  expect(submitted.ok(), await submitted.text()).toBeTruthy();
  const approved = await request.post(
    `/api/v1/${collection}/${entityId}/revisions/${revisionId}/transitions/approve`,
    { data: { actor: "quality.e2e", reason: "approved for E2E", operation_id: `op-approve-${entityId}` } }
  );
  expect(approved.ok(), await approved.text()).toBeTruthy();
}

async function registerMaterial(
  request: APIRequestContext,
  assetId: string,
  serialNumber: string,
  modelId: string,
  model: { revisionId: string; checksum: string },
  identity: { family: string; manufacturer: string; model: string } = {
    family: "RF cable",
    manufacturer: "EMC Locus",
    model: "E2E Cable 1 m"
  }
) {
  const registered = await request.post("/api/v1/metrology/instruments", {
    data: {
      asset_id: assetId,
      family: identity.family,
      equipment_model_id: modelId,
      equipment_model_revision_id: model.revisionId,
      equipment_model_checksum: model.checksum,
      manufacturer: identity.manufacturer,
      model: identity.model,
      serial_number: serialNumber,
      calibration_requirement: "not_required",
      serviceability_status: "usable",
      serviceability_reason: "Contrôle initial conforme",
      capabilities: {},
      actor: "metrology.e2e",
      reason: "register serialized correction workflow material",
      operation_id: `op-register-${assetId}`
    }
  });
  expect(registered.ok(), await registered.text()).toBeTruthy();
}

function cableModelDefinition() {
  return {
    definition_schema_version: "emc-locus.equipment-model-definition.v2",
    manufacturer: "EMC Locus",
    model_name: "E2E Cable 1 m",
    equipment_class: "passive_component",
    functional_role: "rf_network_element",
    category_code: "rf_cable",
    signal_domains: ["rf"],
    technology_tags: ["rf_50_ohm"],
    specifications: [],
    signal_ports: [
      rfThroughPort("RF_A", "Connecteur A"),
      rfThroughPort("RF_B", "Connecteur B")
    ],
    signal_paths: [{
      path_id: "RF_THROUGH",
      label: "Transmission RF",
      input_port_id: "RF_A",
      output_port_id: "RF_B",
      transformations: [],
      correction_requirements: [{
        requirement_id: "cable_loss",
        display_name: "Pertes du câble",
        description: "Pertes propres à chaque câble sérialisé",
        signal_path_id: "RF_THROUGH",
        correction_kind: "frequency_dependent_correction",
        physical_purpose: "Compenser les pertes entre les deux connecteurs du câble réel.",
        operation: "add",
        input_quantity: "power",
        output_quantity: "power",
        expected_unit: "dB",
        required_for_use: true,
        asset_specific_policy: "asset_required",
        conditions: {}
      }]
    }],
    communication_interfaces: [],
    capabilities: [],
    metadata: {}
  };
}

function accelerometerModelDefinition(nominalId: string, revisionId: string, checksum: string) {
  return {
    definition_schema_version: "emc-locus.equipment-model-definition.v2",
    manufacturer: "PCB Piezotronics",
    model_name: "352C33",
    equipment_class: "sensor",
    functional_role: "sensor",
    category_code: "accelerometer",
    signal_domains: ["environmental", "analog_voltage"],
    technology_tags: ["iepe"],
    specifications: [],
    signal_ports: [
      {
        port_id: "MECHANICAL_INPUT",
        label: "Accélération mesurée",
        directionality: "input",
        flow_role: "field_side_port",
        signal_domain: "environmental",
        technology_tags: [],
        quantity: "acceleration",
        unit: "m_per_s2"
      },
      {
        port_id: "IEPE_OUTPUT",
        label: "Sortie IEPE",
        directionality: "output",
        flow_role: "transducer_output_port",
        signal_domain: "analog_voltage",
        connector_type: "10-32 UNF",
        technology_tags: ["iepe"],
        quantity: "voltage",
        unit: "V"
      }
    ],
    signal_paths: [{
      path_id: "SENSITIVITY_PATH",
      label: "Sensibilité accéléromètre",
      input_port_id: "MECHANICAL_INPUT",
      output_port_id: "IEPE_OUTPUT",
      transformations: [],
      correction_requirements: [{
        requirement_id: "sensitivity",
        display_name: "Sensibilité de l’accéléromètre",
        description: "Sensibilité propre au numéro de série",
        signal_path_id: "SENSITIVITY_PATH",
        correction_kind: "raw_signal_conversion",
        physical_purpose: "Convertir la tension IEPE en accélération mesurée.",
        operation: "multiply",
        input_quantity: "voltage",
        output_quantity: "acceleration",
        expected_unit: "m_per_s2",
        required_for_use: true,
        asset_specific_policy: "asset_preferred",
        model_default_reference: {
          correction_kind: "raw_signal_conversion",
          definition_id: nominalId,
          revision_id: revisionId,
          definition_checksum: checksum,
          quality: "manufacturer_nominal"
        },
        conditions: {}
      }]
    }],
    communication_interfaces: [],
    capabilities: [],
    metadata: {}
  };
}

function rfThroughPort(portId: string, label: string) {
  return {
    port_id: portId,
    label,
    directionality: "through",
    flow_role: "through_port",
    signal_domain: "rf",
    connector_type: "N",
    technology_tags: ["rf_50_ohm"],
    quantity: "power",
    unit: "dBm",
    impedance: 50,
    frequency_min: 1_000_000,
    frequency_max: 1_000_000_000
  };
}

async function captureAtDesktopSizes(page: Page, name: string) {
  const directory = path.resolve(process.cwd(), "../../docs/ux/0.18.0/screenshots");
  const refreshHistorical = process.env.EMC_LOCUS_REFRESH_HISTORICAL_SCREENSHOTS === "1";
  if (refreshHistorical) await mkdir(directory, { recursive: true });
  for (const viewport of screenshotViewports) {
    await page.setViewportSize(viewport);
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.waitForTimeout(80);
    const image = await page.screenshot({ animations: "disabled", fullPage: false });
    if (refreshHistorical) {
      await writeFile(path.join(directory, `${name}-${viewport.width}x${viewport.height}.png`), image);
    }
  }
}
