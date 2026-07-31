import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import { spawn } from "node:child_process";
import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const plannedDate = "2026-07-31";
const stationLabel = "Chaîne CEM CA-001";
const screenshotRoot = path.resolve(process.cwd(), "../../docs/ux/0.22.1/screenshots");

let locationId = "";
let cableModel: ApprovedModel;
let sourceModel: ApprovedModel;
let setupId = "";
let caAssetId = "";
let esAssetId = "";

test.describe.serial("0.22.1 station materials and calibration acceptance", () => {
  test.beforeAll(async ({ request }) => {
    locationId = await createLocation(request);
    cableModel = await createApprovedModel(request, "EQM-RADIAL-IMR-400", cableModelDefinition());
    sourceModel = await createApprovedModel(request, "EQM-ES01-SOURCE", sourceModelDefinition());
    esAssetId = (await createFleetAsset(request, {
      inventoryCode: "ES01",
      serialNumber: "ES01-SN-001",
      model: sourceModel,
      locationId,
      calibrationRequirement: "not_required"
    })).asset_id;
    caAssetId = (await createFleetAsset(request, {
      inventoryCode: "CA-001",
      serialNumber: "IMR400-SN-001",
      model: cableModel,
      calibrationRequirement: "required"
    })).asset_id;
  });

  test("certificate values remain a characterization until a calibration event is recorded", async ({
    page,
    request
  }) => {
    await page.goto("/lab/");
    await page.getByRole("button", { name: "Métrologie du parc" }).click();
    await page.getByRole("button", { name: /CA-001/ }).click();
    await expect(page.getByText("Corrections incomplètes")).toBeVisible();
    await expect(page.getByText("Aucun événement d'étalonnage")).toBeVisible();

    await page.getByRole("button", { name: "Mesurer cette correction" }).click();
    await page.getByLabel(/Origine des valeurs mesurées/).selectOption("calibration");
    await expect(page.getByText(/ne crée pas l'événement d'étalonnage global/i)).toBeVisible();
    await page.getByLabel(/Laboratoire ou prestataire/).fill("EMITECH");
    await page.getByLabel(/Méthode utilisée/).fill("CAL-RF-CABLE-001");
    await page.getByLabel("Référence du certificat ou feuillet").fill("CERT-EMITECH-CA-001");
    await page.getByLabel(/Tableau mesuré/).fill(
      "frequence_hz,amplitude_db\n1000000,0.15\n100000000,0.92\n1000000000,3.10"
    );
    await page.locator('.characterizationForm input[type="file"]').setInputFiles({
      name: "CERT-EMITECH-CA-001.pdf",
      mimeType: "application/pdf",
      buffer: Buffer.from("%PDF-1.4\nEMC Locus 0.22.1 correction evidence")
    });
    await capture(page, "calibration-certificate-values-form-1440x900.png", 1440, 900);
    await page.getByRole("button", { name: "Enregistrer puis préparer la revue" }).click();
    await expect(page.getByText("Brouillon à soumettre")).toBeVisible();
    await page.getByRole("button", { name: "Soumettre pour revue" }).click();
    await page.getByRole("button", { name: "Approuver et activer" }).click();

    await expect(page.getByText("Corrections requises disponibles")).toBeVisible();
    await expect(page.getByText("Aucun événement d'étalonnage")).toBeVisible();
    await expect(page.getByText("Étalonnage requis").first()).toBeVisible();
    await expect(page.getByText("Prêt pour un essai")).toHaveCount(0);
    await capture(page, "missing-calibration-with-corrections-available-1440x900.png", 1440, 900);
    await capture(page, "correction-verdict-wording-1280x720.png", 1280, 720);

    const calibrations = await request.get(`/api/v1/metrology/instruments/${caAssetId}/calibrations`);
    expect(calibrations.ok(), await calibrations.text()).toBeTruthy();
    expect((await calibrations.json()).calibration_events).toHaveLength(0);
    const characterizations = await request.get(
      `/api/v1/metrology/instruments/${caAssetId}/characterizations`
    );
    expect(characterizations.ok(), await characterizations.text()).toBeTruthy();
    expect((await characterizations.json()).characterizations).toEqual(
      expect.arrayContaining([expect.objectContaining({ source_kind: "calibration" })])
    );
    const resolution = await request.post(
      `/api/v1/metrology/instruments/${caAssetId}/corrections/resolve`,
      { data: { intended_use_on: plannedDate, execution_context: "accredited", conditions: {} } }
    );
    expect(resolution.ok(), await resolution.text()).toBeTruthy();
    expect((await resolution.json()).report.ready).toBe(true);
  });

  test("exact CA-001 requirement stays immutable while blockers are resolved", async ({ page, request }) => {
    await page.goto("/lab/");
    await page.getByRole("button", { name: "Montages de mesure" }).click();
    await page.getByRole("button", { name: "Nouveau montage" }).click();
    const createDialog = page.getByRole("dialog");
    await createDialog.getByLabel(/Nom du montage/).fill(stationLabel);
    await createDialog.getByLabel(/Lieu du laboratoire/).selectOption(locationId);
    await createDialog.getByLabel(/Date d'utilisation/).fill(plannedDate);
    await createDialog.getByLabel(/Mode d'essai/).selectOption("accredited");
    await createDialog.getByRole("button", { name: "Créer le brouillon" }).click();
    await expect(page.getByRole("heading", { name: stationLabel })).toBeVisible();

    await page.getByRole("button", { name: "Ajouter un rôle" }).click();
    await capture(page, "role-type-selection-1440x900.png", 1440, 900);
    let roleDialog = page.getByRole("dialog");
    await roleDialog.getByLabel(/Nom du rôle/).fill("Câble RF de réserve");
    await roleDialog.getByLabel(/Catégorie/).selectOption("rf_cable");
    await roleDialog.getByRole("checkbox", { name: "Rôle obligatoire" }).uncheck();
    await capture(page, "hierarchical-category-role-1280x720.png", 1280, 720);
    await roleDialog.getByRole("button", { name: "Ajouter au montage" }).click();

    await page.getByRole("button", { name: "Ajouter un rôle" }).click();
    roleDialog = page.getByRole("dialog");
    await roleDialog.getByRole("button", { name: "Exiger des aptitudes techniques" }).click();
    await roleDialog.getByLabel(/Nom du rôle/).fill("Récepteur CEM alternatif");
    await roleDialog.getByRole("checkbox", { name: "Rôle obligatoire" }).uncheck();
    await roleDialog.getByLabel(/Aptitude stable/).fill("measure_emission");
    await roleDialog.getByLabel(/Détecteurs ou modes/).fill("peak, quasi_peak");
    await roleDialog.getByLabel(/Action de driver requise/).fill("measure_emission");
    await capture(page, "capability-role-1440x900.png", 1440, 900);
    await roleDialog.getByRole("button", { name: "Ajouter au montage" }).click();

    await addExactRole(page, "Source RF imposée", esAssetId, "output", "not_required");
    await page.getByRole("button", { name: "Ajouter un rôle" }).click();
    roleDialog = page.getByRole("dialog");
    await roleDialog.getByRole("button", { name: "Imposer un exemplaire du parc" }).click();
    await roleDialog.getByLabel(/Nom du rôle/).fill("Câble CA-001 imposé");
    await roleDialog.getByLabel(/Exemplaire du parc/).selectOption(caAssetId);
    await roleDialog.getByLabel(/Exigence d'étalonnage/).selectOption("required");
    await roleDialog.getByLabel(/Direction/).selectOption("input");
    await capture(page, "exact-blocked-asset-1280x720.png", 1280, 720);
    await roleDialog.getByRole("button", { name: "Ajouter au montage" }).click();

    await page.getByRole("button", { name: "Enregistrer comme exigence du montage" }).click();
    await expect(page.getByText("Exemplaire imposé :", { exact: false }).filter({ hasText: "CA-001" })).toBeVisible();
    const setupResponse = await request.get("/api/v1/station-setups");
    const setups = (await setupResponse.json()).station_setups as Array<{
      identity: { setup_id: string; label: string };
    }>;
    setupId = setups.find((setup) => setup.identity.label === stationLabel)!.identity.setup_id;
    await capture(page, "exact-requirement-saved-1440x900.png", 1440, 900);

    const roleCards = page.locator(".stationRequirementList > article");
    await roleCards.filter({ hasText: "Câble RF de réserve" }).getByRole("button", { name: "Vérifier les candidats" }).click();
    await expect(page.getByText("Compatible mais indisponible")).toBeVisible();
    await capture(page, "compatible-but-unavailable-candidates-1280x720.png", 1280, 720);

    await roleCards.filter({ hasText: "Récepteur CEM alternatif" }).getByRole("button", { name: "Vérifier les candidats" }).click();
    await expect(page.getByText("Incompatible avec les aptitudes requises").first()).toBeVisible();
    await capture(page, "incompatible-candidates-1440x900.png", 1440, 900);

    await roleCards.filter({ hasText: "Câble CA-001 imposé" }).getByRole("button", { name: "Vérifier les candidats" }).click();
    await expect(page.getByText("Exemplaire imposé mais non apte")).toBeVisible();
    await expect(page.getByText("Corrections requises disponibles")).toBeVisible();
    await expect(page.getByText("Aucun étalonnage valide", { exact: true })).toBeVisible();
    await expect(page.getByText("Emplacement non défini", { exact: true })).toBeVisible();
    await expect(page.getByText("Non apte pour l'utilisation prévue")).toBeVisible();
    await expect(page.getByRole("button", { name: "Affecter pour l'utilisation prévue" })).toBeDisabled();
    await capture(page, "ca-001-blocked-requirement-1440x900.png", 1440, 900);

    await page.getByRole("button", { name: "Retirer Câble RF de réserve" }).click();
    await page.getByRole("button", { name: "Retirer Récepteur CEM alternatif" }).click();
    await page.getByRole("button", { name: "Relier les rôles dans l'ordre" }).click();
    await page.getByRole("button", { name: "Enregistrer comme exigence du montage" }).click();
    await expect(page.getByText("1 liaison(s) logique(s)")).toBeVisible();
    await page.getByRole("button", { name: "Valider la définition" }).click();
    await expect(page.getByText("Définition validée", { exact: true }).first()).toBeVisible();
    await capture(page, "qualified-setup-1280x720.png", 1280, 720);

    const caBeforeMove = await getAsset(request, caAssetId);
    await moveAsset(request, caAssetId, caBeforeMove.revision, locationId, "op-0221-move-ca-001");
    const afterMove = await stationCandidates(request, setupId, caAssetId);
    expect(afterMove.operational_blockers.map((reason: { code: string }) => reason.code)).not.toContain(
      "location_missing"
    );
    expect(afterMove.operational_blockers.map((reason: { code: string }) => reason.code)).toContain(
      "calibration_missing"
    );
    expect(afterMove.correction_readiness).toBe("available");

    await page.getByRole("button", { name: "Métrologie du parc" }).click();
    await page.getByRole("button", { name: /CA-001/ }).click();
    await page.getByRole("button", { name: "Enregistrer un étalonnage" }).first().click();
    await page.getByLabel(/Date d'étalonnage/).fill(plannedDate);
    await page.getByLabel(/Prochaine échéance/).fill("2027-07-31");
    await page.getByLabel(/Référence du certificat/).fill("CERT-EMITECH-CA-001");
    await page.getByLabel(/Laboratoire ou prestataire/).fill("EMITECH");
    await page.getByLabel(/Décision globale/).selectOption("conforming");
    await page.getByLabel(/État constaté avant intervention/).selectOption("conforming");
    await page.getByLabel(/État laissé après intervention/).selectOption("conforming");
    await page.getByLabel(/Incertitude élargie/).fill("0.20");
    await page.getByLabel(/^Unité/).fill("dB");
    await page.getByLabel(/Résumé de l'incertitude/).fill("U = 0,20 dB, k = 2");
    await page.getByLabel(/Référence de traçabilité/).fill("COFRAC-2-1111");
    await page.getByLabel(/Commentaire/).fill("Étalonnage conforme du câble imposé CA-001");
    await page.locator('.calibrationForm input[type="file"]').setInputFiles({
      name: "CERT-EMITECH-CA-001.pdf",
      mimeType: "application/pdf",
      buffer: Buffer.from("%PDF-1.4\nEMC Locus 0.22.1 calibration evidence")
    });
    await capture(page, "calibration-form-1440x900.png", 1440, 900);
    await page.locator(".calibrationForm").getByRole("button", { name: "Enregistrer l'étalonnage" }).click();
    await expect(page.getByText("CERT-EMITECH-CA-001").first()).toBeVisible();
    await expect(page.getByText("Étalonnage valide").first()).toBeVisible();
    await expect(page.getByText("Corrections requises disponibles")).toBeVisible();
    await capture(page, "calibration-history-1280x720.png", 1280, 720);
    await capture(page, "valid-calibration-after-recording-1440x900.png", 1440, 900);

    await page.getByRole("button", { name: "Montages de mesure" }).click();
    await page.getByRole("button", { name: stationLabel }).click();
    await page.locator(".stationRequirementList > article")
      .filter({ hasText: "Câble CA-001 imposé" })
      .getByRole("button", { name: "Vérifier les candidats" })
      .click();
    await page.setViewportSize({ width: 1280, height: 720 });
    const exactEligibleVerdict = page.getByText("Compatible et disponible");
    await expect(exactEligibleVerdict).toBeVisible();
    await exactEligibleVerdict.evaluate((element) =>
      element.scrollIntoView({ block: "center", inline: "nearest" })
    );
    await page.waitForTimeout(80);
    await capture(page, "exact-eligible-asset-1280x720.png", 1280, 720, false);
    await page.getByRole("button", { name: "Finaliser les affectations dans un brouillon" }).click();
    await expect(page.getByText("Brouillon", { exact: true }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: "Ajouter un rôle" })).toBeVisible();
    await assignRole(page, "Source RF imposée");
    await assignRole(page, "Câble CA-001 imposé");
    await page.getByRole("button", { name: "Contrôler l'aptitude opérationnelle" }).click();
    await expect(page.getByRole("heading", { name: "Montage apte à être utilisé" })).toBeVisible();
    await page.getByRole("button", { name: "Valider la définition" }).click();
    await page.getByRole("button", { name: "Déclarer prêt à utiliser" }).click();
    await expect(page.getByText("Prêt à utiliser", { exact: true }).first()).toBeVisible();
    await capture(page, "operationally-ready-setup-1440x900.png", 1440, 900);
  });

  test("UTF-8, secondary failure isolation and restart preserve 0.22.1 evidence", async ({
    page,
    request
  }) => {
    seedHistoricalV2StationFixture();
    const historicalBefore = await request.get(
      "/api/v1/station-setups/SETUP-HISTORICAL-V2/revisions/SETUP-HISTORICAL-V2-rev-0001"
    );
    expect(historicalBefore.ok(), await historicalBefore.text()).toBeTruthy();
    const historicalChecksum = (await historicalBefore.json()).revision.definition_checksum;
    await page.goto("/lab/");
    await page.getByRole("button", { name: "Montages de mesure" }).click();
    await page.getByRole("button", { name: /Montage historique v2/ }).click();
    await expect(page.getByRole("heading", { name: "Montage historique v2", level: 2 })).toBeVisible();
    await expect(page.getByText("Montage historique v2", { exact: true }).last()).toBeVisible();
    await capture(page, "v2-compatibility-1280x720.png", 1280, 720);
    await page.getByRole("button", { name: "Créer un brouillon v3" }).click();
    await expect(page.getByRole("button", { name: "Ajouter un rôle" })).toBeVisible();
    const historicalAfter = await request.get(
      "/api/v1/station-setups/SETUP-HISTORICAL-V2/revisions/SETUP-HISTORICAL-V2-rev-0001"
    );
    expect((await historicalAfter.json()).revision.definition_checksum).toBe(historicalChecksum);

    const calibration = await request.get(`/api/v1/metrology/instruments/${caAssetId}/calibrations`);
    expect(calibration.headers()["content-type"]).toBe("application/json; charset=utf-8");
    expect(await calibration.text()).toContain("Étalonnage conforme du câble imposé CA-001");
    const error = await request.get("/api/v1/station-setups/INCONNU-É/revisions");
    expect(error.ok()).toBe(false);
    expect(error.headers()["content-type"]).toBe("application/json; charset=utf-8");
    expect(await error.text()).not.toContain("�");

    await page.route(`**/api/v1/metrology/instruments/${caAssetId}/calibrations`, async (route) => {
      await route.fulfill({
        status: 503,
        contentType: "application/json; charset=utf-8",
        body: JSON.stringify({ error: { code: "calibration_history_unavailable", message: "Historique indisponible" } })
      });
    });
    await page.goto("/lab/");
    await page.getByRole("button", { name: "Métrologie du parc" }).click();
    await page.getByRole("button", { name: /CA-001/ }).click();
    await expect(page.getByText("État métrologique partiellement indisponible")).toBeVisible();
    await expect(page.getByText("Corrections requises disponibles")).toBeVisible();
    await expect(page.getByText("CA-001", { exact: false }).first()).toBeVisible();
    await page.unroute(`**/api/v1/metrology/instruments/${caAssetId}/calibrations`);

    await restartAgent(request);
    const station = await request.get(`/api/v1/station-setups/${setupId}`);
    expect(station.ok(), await station.text()).toBeTruthy();
    const stationBody = await station.json();
    const latestDefinition = stationBody.station_setup.latest_revision.definition;
    expect(latestDefinition.material_requirements).toEqual(
      expect.arrayContaining([expect.objectContaining({ exact_asset_id: caAssetId })])
    );
    expect(latestDefinition.material_assignments).toEqual(
      expect.arrayContaining([expect.objectContaining({ asset_id: caAssetId })])
    );
    const calibrations = await request.get(`/api/v1/metrology/instruments/${caAssetId}/calibrations`);
    expect((await calibrations.json()).calibration_events).toEqual(
      expect.arrayContaining([expect.objectContaining({ certificate_reference: "CERT-EMITECH-CA-001" })])
    );
    const characterizations = await request.get(
      `/api/v1/metrology/instruments/${caAssetId}/characterizations`
    );
    expect((await characterizations.json()).characterizations).toHaveLength(1);
    const corrections = await request.get(`/api/v1/metrology/instruments/${caAssetId}/corrections`);
    expect((await corrections.json()).assignments).toEqual(
      expect.arrayContaining([expect.objectContaining({ assignment: expect.objectContaining({ status: "active" }) })])
    );
  });
});

async function addExactRole(
  page: Page,
  label: string,
  assetId: string,
  direction: "input" | "output",
  calibrationRequirement: "required" | "not_required"
) {
  await page.getByRole("button", { name: "Ajouter un rôle" }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByRole("button", { name: "Imposer un exemplaire du parc" }).click();
  await dialog.getByLabel(/Nom du rôle/).fill(label);
  await dialog.getByLabel(/Exemplaire du parc/).selectOption(assetId);
  await dialog.getByLabel(/Exigence d'étalonnage/).selectOption(calibrationRequirement);
  await dialog.getByLabel(/Direction/).selectOption(direction);
  await dialog.getByRole("button", { name: "Ajouter au montage" }).click();
}

function seedHistoricalV2StationFixture() {
  const storageRoot = process.env.LAB_CONSOLE_E2E_STORAGE_ROOT;
  if (!storageRoot) throw new Error("LAB_CONSOLE_E2E_STORAGE_ROOT is required for the v2 fixture");
  const python = String.raw`
import hashlib, json, pathlib, sqlite3, sys
root = pathlib.Path(sys.argv[1])
location_id, es_asset_id, ca_asset_id = sys.argv[2:5]
equipment = sqlite3.connect(root / "equipment.sqlite")
equipment.row_factory = sqlite3.Row
def asset(asset_id):
    return equipment.execute("""SELECT asset_id, revision, equipment_model_id,
        equipment_model_revision_id, equipment_model_checksum
        FROM physical_assets WHERE asset_id = ?""", (asset_id,)).fetchone()
source, cable = asset(es_asset_id), asset(ca_asset_id)
definition = {
    "definition_schema_version": "emc-locus.station-measurement-setup-definition.v2",
    "setup_id": "SETUP-HISTORICAL-V2",
    "label": "Montage historique v2",
    "laboratory_location_id": location_id,
    "laboratory_location_label": "labo CEM",
    "planned_use_on": "2026-07-31",
    "execution_mode": "accredited",
    "asset_bindings": [
        {"binding_id": "source", "role_label": "Source RF historique",
         "asset_id": source["asset_id"], "asset_revision": str(source["revision"]),
         "equipment_model_id": source["equipment_model_id"],
         "equipment_model_revision_id": source["equipment_model_revision_id"],
         "equipment_model_checksum": source["equipment_model_checksum"]},
        {"binding_id": "cable", "role_label": "Câble historique",
         "asset_id": cable["asset_id"], "asset_revision": str(cable["revision"]),
         "equipment_model_id": cable["equipment_model_id"],
         "equipment_model_revision_id": cable["equipment_model_revision_id"],
         "equipment_model_checksum": cable["equipment_model_checksum"]}
    ],
    "connections": [{"connection_id": "rf-path", "label": "Source vers câble",
        "from": {"binding_id": "source", "port_id": "RF_OUT"},
        "to": {"binding_id": "cable", "port_id": "RF_A"}}],
    "correction_selections": []
}
definition["asset_bindings"].sort(key=lambda item: item["binding_id"])
canonical = json.dumps(definition, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
checksum = "sha256:" + hashlib.sha256(canonical.encode("utf-8")).hexdigest()
readiness = json.dumps({"checked_on": "2026-07-31", "ready": True, "issues": []},
                       ensure_ascii=False, sort_keys=True, separators=(",", ":"))
station = sqlite3.connect(root / "station.sqlite")
station.execute("""INSERT INTO station_setup_identities
    (setup_id, label, current_ready_revision_id, created_by, created_at, updated_at)
    VALUES (?, ?, ?, ?, ?, ?)""", ("SETUP-HISTORICAL-V2", "Montage historique v2",
    "SETUP-HISTORICAL-V2-rev-0001", "migration.0220", "2026-07-30T08:00:00Z",
    "2026-07-30T08:00:00Z"))
station.execute("""INSERT INTO station_setup_revisions
    (revision_id, setup_id, revision_number, status, definition_schema_version,
     definition_json, definition_checksum, readiness_json, created_by, created_at,
     updated_at, ready_at)
    VALUES (?, ?, 1, 'ready', ?, ?, ?, ?, ?, ?, ?, ?)""",
    ("SETUP-HISTORICAL-V2-rev-0001", "SETUP-HISTORICAL-V2",
     "emc-locus.station-measurement-setup-definition.v2", canonical, checksum, readiness,
     "migration.0220", "2026-07-30T08:00:00Z", "2026-07-30T08:00:00Z",
     "2026-07-30T08:00:00Z"))
station.commit()
station.close()
equipment.close()
`;
  execFileSync("py", ["-3", "-c", python, storageRoot, locationId, esAssetId, caAssetId], {
    cwd: path.resolve(process.cwd(), "../.."),
    encoding: "utf8",
    stdio: "pipe"
  });
}

async function assignRole(page: Page, label: string) {
  const card = page.locator(".stationRequirementList > article").filter({ hasText: label });
  await card.getByRole("button", { name: "Vérifier les candidats" }).click();
  const panel = page.locator(".stationCandidatePanel");
  await expect(panel.getByText("Compatible et disponible")).toBeVisible();
  await panel.getByRole("button", { name: "Affecter pour l'utilisation prévue" }).click();
  const save = page.getByRole("button", { name: "Enregistrer comme exigence du montage" });
  await save.click();
  await expect(save).toBeDisabled();
}

async function createLocation(request: APIRequestContext) {
  const response = await request.post("/api/v1/laboratory-locations", {
    data: {
      label: "labo CEM",
      description: "Laboratoire CEM pour l'acceptation 0.22.1",
      actor: "station.e2e",
      reason: "préparer le lieu du scénario CA-001",
      operation_id: "op-0221-create-labo-cem"
    }
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  return (await response.json()).location.location_id as string;
}

async function createApprovedModel(
  request: APIRequestContext,
  modelId: string,
  definition: Record<string, unknown>
): Promise<ApprovedModel> {
  const created = await request.post("/api/v1/equipment-models", {
    data: {
      equipment_model_id: modelId,
      definition,
      actor: "catalogue.e2e",
      reason: "créer un modèle pour l'acceptation 0.22.1",
      operation_id: `op-0221-create-${modelId}`
    }
  });
  expect(created.ok(), await created.text()).toBeTruthy();
  const createdBody = await created.json();
  const revision = createdBody.revision ?? createdBody.aggregate.latest_revision;
  for (const transition of ["submit-for-review", "approve"]) {
    const response = await request.post(
      `/api/v1/equipment-models/${modelId}/revisions/${revision.revision_id}/transitions/${transition}`,
      {
        data: {
          actor: "quality.e2e",
          reason: "valider le modèle de preuve 0.22.1",
          operation_id: `op-0221-${transition}-${modelId}`
        }
      }
    );
    expect(response.ok(), await response.text()).toBeTruthy();
  }
  return {
    modelId,
    revisionId: revision.revision_id,
    checksum: revision.definition_checksum
  };
}

async function createFleetAsset(
  request: APIRequestContext,
  input: {
    inventoryCode: string;
    serialNumber: string;
    model: ApprovedModel;
    locationId?: string;
    calibrationRequirement: "required" | "not_required";
  }
) {
  const response = await request.post("/api/v1/fleet/assets", {
    data: {
      inventory_code: input.inventoryCode,
      serial_number: input.serialNumber,
      part_number: input.inventoryCode === "CA-001" ? "IMR-400" : "ES01",
      equipment_model_id: input.model.modelId,
      laboratory_location_id: input.locationId,
      ownership_source: "laboratory_owned",
      service_state: "usable",
      administrative_availability: "available",
      service_state_reason: "Contrôle initial conforme",
      notes: "Preuve déterministe 0.22.1",
      calibration_requirement: input.calibrationRequirement,
      calibration_period_months: input.calibrationRequirement === "required" ? 12 : undefined,
      calibration_due_warning_days: 30,
      actor: "fleet.e2e",
      reason: "enregistrer l'exemplaire du scénario CA-001",
      operation_id: `op-0221-create-${input.inventoryCode}`,
      device_id: "playwright-0221",
      correlation_id: "corr-0221-ca-001"
    }
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  return (await response.json()).asset as { asset_id: string };
}

async function getAsset(request: APIRequestContext, assetId: string) {
  const response = await request.get(`/api/v1/fleet/assets/${assetId}`);
  expect(response.ok(), await response.text()).toBeTruthy();
  return (await response.json()).asset as { revision: number };
}

async function moveAsset(
  request: APIRequestContext,
  assetId: string,
  expectedRevision: number,
  destinationLocationId: string,
  operationId: string
) {
  const response = await request.post(`/api/v1/fleet/assets/${assetId}/transitions/move`, {
    data: {
      expected_revision: expectedRevision,
      destination_location_id: destinationLocationId,
      actor: "fleet.e2e",
      reason: "déplacer CA-001 vers le labo CEM",
      operation_id: operationId,
      device_id: "playwright-0221",
      correlation_id: "corr-0221-ca-001"
    }
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  return (await response.json()).asset as { asset_id: string };
}

async function stationCandidates(request: APIRequestContext, currentSetupId: string, assetId: string) {
  const station = await request.get(`/api/v1/station-setups/${currentSetupId}`);
  const aggregate = (await station.json()).station_setup;
  const revision = aggregate.current_qualified_revision ?? aggregate.active_draft_revision;
  const requirement = revision.definition.material_requirements.find(
    (item: { exact_asset_id?: string }) => item.exact_asset_id === assetId
  );
  const query = new URLSearchParams({
    planned_use_on: plannedDate,
    execution_mode: "accredited",
    laboratory_location_id: locationId
  });
  const response = await request.get(
    `/api/v1/station-setups/${currentSetupId}/revisions/${revision.revision_id}/material-requirements/${requirement.requirement_id}/candidates?${query}`
  );
  expect(response.ok(), await response.text()).toBeTruthy();
  return (await response.json()).candidates.find(
    (candidate: { asset: { asset_id: string } }) => candidate.asset.asset_id === assetId
  );
}

async function capture(
  page: Page,
  name: string,
  width: number,
  height: number,
  resetScroll = true
) {
  await page.setViewportSize({ width, height });
  if (resetScroll) {
    await page.evaluate(() => window.scrollTo(0, 0));
  }
  await page.waitForTimeout(100);
  const image = await page.screenshot({ animations: "disabled", fullPage: false });
  if (
    process.env.EMC_LOCUS_REFRESH_RELEASE_SCREENSHOTS === "1"
    && process.env.EMC_LOCUS_RELEASE_SCREENSHOT_VERSION === "0.22.1"
  ) {
    await mkdir(screenshotRoot, { recursive: true });
    await writeFile(path.join(screenshotRoot, name), image);
  }
}

async function restartAgent(request: APIRequestContext) {
  const currentPid = Number(process.env.LAB_CONSOLE_E2E_AGENT_PID);
  const executable = process.env.LAB_CONSOLE_E2E_AGENT_EXECUTABLE;
  const storageRelative = process.env.LAB_CONSOLE_E2E_STORAGE_RELATIVE;
  const bind = process.env.LAB_CONSOLE_E2E_AGENT_BIND;
  const pidFile = process.env.LAB_CONSOLE_E2E_RESTARTED_AGENT_PID_FILE;
  if (!currentPid || !executable || !storageRelative || !bind || !pidFile) {
    throw new Error("The isolated E2E runner did not expose restart metadata");
  }
  process.kill(currentPid);
  await new Promise((resolve) => setTimeout(resolve, 350));
  const restarted = spawn(executable, [
    "serve",
    "--storage-root", storageRelative,
    "--migrations-root", "storage/sqlite",
    "--bind", bind,
    "--lab-console-dist", "apps/lab-console/dist"
  ], { cwd: path.resolve(process.cwd(), "../.."), windowsHide: true, stdio: "ignore" });
  if (!restarted.pid) throw new Error("The restarted Local Agent has no process identifier");
  writeFileSync(pidFile, String(restarted.pid), "utf8");
  restarted.unref();
  for (let attempt = 0; attempt < 120; attempt += 1) {
    try {
      const health = await request.get("/api/v1/health", { timeout: 500 });
      if (health.ok()) return;
    } catch {
      // The port is temporarily unavailable during the explicit restart.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error("The restarted Local Agent did not become ready");
}

function cableModelDefinition() {
  return {
    definition_schema_version: "emc-locus.equipment-model-definition.v2",
    manufacturer: "Radial",
    model_name: "IMR-400",
    equipment_class: "passive_component",
    functional_role: "rf_network_element",
    category_code: "rf_cable",
    signal_domains: ["rf"],
    technology_tags: ["rf_50_ohm"],
    specifications: [],
    signal_ports: [rfThroughPort("RF_A", "Connecteur A"), rfThroughPort("RF_B", "Connecteur B")],
    signal_paths: [{
      path_id: "RF_THROUGH",
      label: "Transmission RF",
      input_port_id: "RF_A",
      output_port_id: "RF_B",
      transformations: [],
      correction_requirements: [{
        requirement_id: "cable_loss",
        display_name: "Pertes du câble",
        description: "Pertes propres au câble CA-001",
        signal_path_id: "RF_THROUGH",
        correction_kind: "frequency_dependent_correction",
        physical_purpose: "Compenser les pertes jusqu'au plan de référence.",
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

function sourceModelDefinition() {
  return {
    definition_schema_version: "emc-locus.equipment-model-definition.v2",
    manufacturer: "Locus Instruments",
    model_name: "ES01",
    equipment_class: "manual_equipment",
    functional_role: "signal_source",
    category_code: "signal_sources",
    signal_domains: ["rf"],
    technology_tags: ["rf_50_ohm"],
    specifications: [],
    signal_ports: [{
      port_id: "RF_OUT",
      label: "Sortie RF",
      directionality: "output",
      flow_role: "source_port",
      signal_domain: "rf",
      connector_type: "N",
      technology_tags: ["rf_50_ohm"],
      quantity: "power",
      unit: "dBm",
      impedance: 50,
      frequency_min: 9_000,
      frequency_max: 1_000_000_000
    }],
    signal_paths: [],
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

interface ApprovedModel {
  modelId: string;
  revisionId: string;
  checksum: string;
}
