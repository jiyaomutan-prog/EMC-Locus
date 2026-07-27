import { execFileSync } from "node:child_process";
import { mkdir } from "node:fs/promises";
import path from "node:path";
import {
  expect,
  test,
  type APIRequestContext,
  type Page
} from "@playwright/test";

const repoRoot = path.resolve(process.cwd(), "../..");
const screenshotDirectory = path.join(repoRoot, "docs", "ux", "0.22.0", "screenshots");
const refreshScreenshots =
  process.env.EMC_LOCUS_REFRESH_RELEASE_SCREENSHOTS === "1"
  && process.env.EMC_LOCUS_RELEASE_SCREENSHOT_VERSION === "0.22.0";
const modelId = "EQM-DEMO-NRP6AN-FWD";
const inventoryCode = "INV-0220-SANS-SERIE";
let assetId = "";
let modelRevisionId = "";
let modelChecksum = "";
let firstLocation: LaboratoryLocation;
let secondLocation: LaboratoryLocation;

test.describe.serial("0.22.0 equipment fleet", () => {
  test.beforeAll(async ({ request }) => {
    seedDemoData();

    const modelResponse = await request.get(`/api/v1/equipment-models/${modelId}`);
    expect(modelResponse.ok(), await modelResponse.text()).toBeTruthy();
    const model = (await modelResponse.json()).equipment_model;
    modelRevisionId = model.current_approved_revision.revision_id;
    modelChecksum = model.current_approved_revision.definition_checksum;
    await ensureModelCategory(
      request,
      model.identity.category_code,
      model.identity.root_category_id
        || model.current_approved_revision.definition.template_snapshot?.root_category_id
        || "measurement_instruments_digitizers",
      model.current_approved_revision.definition.template_snapshot?.category_path?.at(-1) ?? "Wattmètres RF"
    );

    firstLocation = await createLocation(request, "Zone parc 0.22 A", "Emplacement initial du scénario parc", "location-a");
    secondLocation = await createLocation(request, "Zone parc 0.22 B", "Emplacement de destination du scénario parc", "location-b");

    const created = await request.post("/api/v1/fleet/assets", {
      data: {
        inventory_code: inventoryCode,
        part_number: "NRP6AN",
        equipment_model_id: modelId,
        laboratory_location_id: firstLocation.location_id,
        ownership_source: "laboratory_owned",
        service_state: "usable",
        availability_state: "available",
        service_state_reason: "Vérifié avant mise au parc",
        notes: "Exemplaire volontairement enregistré sans numéro de série.",
        calibration_requirement: "not_required",
        calibration_due_warning_days: 30,
        actor: "fleet.e2e",
        reason: "prouver l'identité principale par code inventaire",
        operation_id: "op-0220-create-no-serial"
      }
    });
    expect(created.ok(), await created.text()).toBeTruthy();
    assetId = (await created.json()).asset.asset_id;
  });

  test("separates a generic model from two pinned physical assets", async ({ page, request }) => {
    const modelResponse = await request.get(`/api/v1/equipment-models/${modelId}`);
    const model = (await modelResponse.json()).equipment_model;
    const definition = model.current_approved_revision.definition;
    expect(definition.inventory_code).toBeUndefined();
    expect(definition.serial_number).toBeUndefined();
    expect(definition.laboratory_location_id).toBeUndefined();

    const assetsResponse = await request.get("/api/v1/fleet/assets");
    const assets = (await assetsResponse.json()).assets as PhysicalAsset[];
    const sameModel = assets.filter((asset) => asset.equipment_model_id === modelId);
    expect(sameModel.length).toBeGreaterThanOrEqual(2);
    const noSerial = sameModel.find((asset) => asset.asset_id === assetId);
    expect(noSerial?.serial_number).toBeNull();
    expect(noSerial?.equipment_model_revision_id).toBe(modelRevisionId);
    expect(noSerial?.equipment_model_checksum).toBe(modelChecksum);

    await openDemoModel(page);
    await expect(page.getByText("Vous consultez un modèle générique.")).toBeVisible();
    await expect(page.locator(".linkedFleetAssets").getByText(inventoryCode)).toBeVisible();
    await expect(page.locator(".linkedFleetAssets").getByText("Sans numéro de série")).toBeVisible();
    await expect(page.getByText("Code inventaire:")).toHaveCount(0);
    await expect(page.getByText("Dernier étalonnage")).toHaveCount(0);

    await page.getByRole("button", { name: "Voir les exemplaires du parc" }).click();
    await page.getByLabel("Rechercher dans le parc").fill(inventoryCode);
    await page.getByRole("treeitem", { name: new RegExp(inventoryCode) }).click();
    await expect(page.getByRole("heading", { name: inventoryCode })).toBeVisible();
    await expect(page.getByText("Sans numéro de série").first()).toBeVisible();

    const exactRevision = page.waitForResponse((response) =>
      response.url().includes(`/api/v1/equipment-models/${modelId}/revisions/${modelRevisionId}`)
    );
    await page.getByRole("button", { name: "Ouvrir le modèle constructeur" }).click();
    expect((await exactRevision).ok()).toBeTruthy();
    await expect(page.getByText("Vous consultez un modèle générique.")).toBeVisible();
  });

  test("renames, moves and archives locations without losing stable identity", async ({ request }) => {
    const renamed = await request.put(`/api/v1/laboratory-locations/${firstLocation.location_id}`, {
      data: {
        expected_revision: firstLocation.revision,
        label: "Zone parc 0.22 A renommée",
        description: firstLocation.description,
        actor: "fleet.e2e",
        reason: "vérifier la stabilité de l'identité du lieu",
        operation_id: "op-0220-rename-location-a"
      }
    });
    expect(renamed.ok(), await renamed.text()).toBeTruthy();
    const renamedLocation = (await renamed.json()).location as LaboratoryLocation;
    expect(renamedLocation.location_id).toBe(firstLocation.location_id);

    const assetBeforeMove = await getAsset(request, assetId);
    const moved = await request.put(`/api/v1/fleet/assets/${assetId}`, {
      data: {
        expected_revision: assetBeforeMove.revision,
        inventory_code: assetBeforeMove.inventory_code,
        part_number: assetBeforeMove.part_number,
        laboratory_location_id: secondLocation.location_id,
        ownership_source: assetBeforeMove.ownership_source,
        notes: assetBeforeMove.notes,
        actor: "fleet.e2e",
        reason: "déplacer l'exemplaire vers un lieu actif",
        operation_id: "op-0220-move-asset"
      }
    });
    expect(moved.ok(), await moved.text()).toBeTruthy();
    expect((await moved.json()).asset.laboratory_location_id).toBe(secondLocation.location_id);

    const archived = await request.post(`/api/v1/laboratory-locations/${firstLocation.location_id}/archive`, {
      data: {
        expected_revision: renamedLocation.revision,
        actor: "fleet.e2e",
        reason: "retirer le lieu des nouvelles affectations",
        operation_id: "op-0220-archive-location-a"
      }
    });
    expect(archived.ok(), await archived.text()).toBeTruthy();

    const rejected = await request.post("/api/v1/fleet/assets", {
      data: {
        inventory_code: "INV-0220-ARCHIVED-LOCATION",
        equipment_model_id: modelId,
        laboratory_location_id: firstLocation.location_id,
        ownership_source: "laboratory_owned",
        service_state: "usable",
        availability_state: "available",
        calibration_requirement: "not_required",
        actor: "fleet.e2e",
        reason: "vérifier le refus d'un lieu archivé",
        operation_id: "op-0220-rejected-archived-location"
      }
    });
    expect(rejected.status()).toBe(409);
    expect(JSON.stringify(await rejected.json())).toContain("archiv");

    const assetAudit = await request.get(`/api/v1/fleet/assets/${assetId}/audit-events`);
    expect(assetAudit.ok(), await assetAudit.text()).toBeTruthy();
    expect(JSON.stringify(await assetAudit.json())).toContain("op-0220-move-asset");
    const locationAudit = await request.get(`/api/v1/laboratory-locations/${firstLocation.location_id}/audit-events`);
    expect(locationAudit.ok(), await locationAudit.text()).toBeTruthy();
    expect(JSON.stringify(await locationAudit.json())).toContain("op-0220-archive-location-a");
    const outbox = await request.get("/api/v1/sync/outbox");
    expect(outbox.ok(), await outbox.text()).toBeTruthy();
    const outboxJson = JSON.stringify(await outbox.json());
    expect(outboxJson).toContain("op-0220-move-asset");
    expect(outboxJson).not.toContain("op-0220-rejected-archived-location");
  });

  test("offers physical assets in station setup and planned-test preparation", async ({ page, request }) => {
    await page.goto("/lab/");
    await page.getByRole("button", { name: "Montages de mesure" }).click();
    await expect(page.getByRole("heading", { level: 1, name: "Montages de mesure" })).toBeVisible();
    await page.getByRole("button", { name: "Nouveau montage" }).click();
    const dialog = page.getByRole("dialog", { name: "Préparer un montage" });
    await dialog.getByLabel(/Nom du montage/).fill("Montage E2E parc 0.22");
    await dialog.getByLabel(/Lieu du laboratoire/).selectOption(secondLocation.location_id);
    await dialog.getByLabel(/Date d'utilisation/).fill("2026-07-30");
    await dialog.getByRole("button", { name: "Créer le brouillon" }).click();

    const assetSelector = page.getByLabel(/Exemplaire du parc/);
    await expect(assetSelector).toBeVisible();
    await expect(assetSelector.locator("option", { hasText: inventoryCode })).toHaveCount(1);
    await expect(assetSelector.locator("option", { hasText: modelId })).toHaveCount(0);

    const schedule = await request.get("/api/v1/projects/CEM-DEMO-PREP-001/schedule-items");
    expect(schedule.ok(), await schedule.text()).toBeTruthy();
    const item = (await schedule.json()).schedule_items.find((candidate: { item_code: string }) => candidate.item_code === "PLAN-DEMO-PREP-001");
    expect(item.equipment_under_test).toBe("Convertisseur Horizon HCU-4");
    const options = await request.get("/api/v1/projects/CEM-DEMO-PREP-001/schedule-items/PLAN-DEMO-PREP-001/preparation/options");
    expect(options.ok(), await options.text()).toBeTruthy();
    const optionsText = JSON.stringify(await options.json());
    expect(optionsText).toContain("PM-DEMO-RF-001");
    expect(optionsText).toContain("inventory_code");
    expect(optionsText).not.toContain('"asset_id":"EQM-');
  });

  test("keeps primary identity visible during metrology and driver failures", async ({ page }) => {
    await page.route("**/api/v1/metrology/instruments", async (route) => {
      await route.fulfill({
        status: 503,
        contentType: "application/json",
        body: JSON.stringify({ error: { code: "metrology_unavailable", message: "métrologie indisponible pour le scénario" } })
      });
    });
    await page.goto("/lab/");
    await page.getByRole("button", { name: "Parc matériel" }).click();
    await expect(page.getByText("Métrologie temporairement indisponible")).toBeVisible();
    await expect(page.getByRole("heading", { name: inventoryCode })).toBeVisible();

    await page.unroute("**/api/v1/metrology/instruments");
    await page.route("**/api/v1/driver-profiles*", async (route) => {
      await route.fulfill({
        status: 503,
        contentType: "application/json",
        body: JSON.stringify({ error: { code: "drivers_unavailable", message: "drivers indisponibles pour le scénario" } })
      });
    });
    await page.reload();
    await page.getByRole("button", { name: "Catalogue des modèles" }).click();
    await showDemoModels(page);
    await expect(page.getByText("Pilotage temporairement indisponible")).toBeVisible();
    await expect(page.getByRole("treeitem", { name: /NRP6AN/ })).toBeVisible();
    await page.getByRole("button", { name: "Parc matériel" }).click();
    await expect(page.getByRole("heading", { name: inventoryCode })).toBeVisible();
  });

  test("reviews the operator workflow and refreshes only 0.22.0 evidence", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/lab/");
    await page.getByRole("button", { name: "Catalogue des modèles" }).click();
    await showDemoModels(page);
    await expect(page.getByText("Ressources techniques", { exact: true })).toBeVisible();
    await capture(page, "technical-resources-navigation-1440x900.png");
    await capture(page, "hierarchical-model-catalogue-1440x900.png");
    await page.setViewportSize({ width: 1280, height: 720 });
    await capture(page, "hierarchical-model-catalogue-1280x720.png");

    await page.setViewportSize({ width: 1440, height: 900 });
    await page.getByRole("button", { name: "Parc matériel" }).click();
    await expect(page.getByRole("heading", { name: inventoryCode })).toBeVisible();
    await capture(page, "hierarchical-fleet-1440x900.png");
    await page.getByRole("button", { name: "Par emplacement" }).click();
    await capture(page, "fleet-grouped-by-location-1440x900.png");
    await page.getByRole("button", { name: "Par état de service" }).click();
    await capture(page, "fleet-grouped-by-service-state-1440x900.png");

    await page.getByLabel("Rechercher dans le parc").fill(inventoryCode);
    await page.getByRole("treeitem", { name: new RegExp(inventoryCode) }).click();
    await capture(page, "physical-asset-summary-1440x900.png");
    await page.getByRole("button", { name: "Métrologie", exact: true }).click();
    await capture(page, "asset-metrology-tab-1440x900.png");
    await page.getByRole("button", { name: "Ouvrir le modèle constructeur" }).click();
    await expect(page.getByText("Vous consultez un modèle générique.")).toBeVisible();
    await capture(page, "model-detail-with-linked-assets-1440x900.png");
    await capture(page, "generic-model-without-asset-calibration-1440x900.png");

    await page.getByRole("button", { name: "Créer un exemplaire dans le parc" }).click();
    await page.setViewportSize({ width: 1280, height: 720 });
    await capture(page, "create-physical-asset-from-model-1280x720.png");
    await capture(page, "disabled-action-explanation-1280x720.png");
    await page.getByRole("button", { name: "Fermer" }).click();
    await page.getByLabel("Rechercher dans le parc").fill(inventoryCode);
    await page.getByRole("treeitem", { name: new RegExp(inventoryCode) }).click();
    await capture(page, "physical-asset-without-serial-1280x720.png");

    await page.setViewportSize({ width: 1440, height: 900 });
    await page.getByRole("button", { name: "Lieux du laboratoire" }).click();
    await capture(page, "laboratory-location-registry-1440x900.png");
    await page.getByRole("button", { name: "Montages de mesure" }).click();
    await page.getByRole("button", { name: /Montage E2E parc 0.22/ }).click();
    await expect(page.getByLabel(/Exemplaire du parc/)).toBeVisible();
    await capture(page, "station-setup-physical-asset-selector-1440x900.png");

    await openPlannedPreparation(page);
    await expect(page.getByRole("dialog", { name: "Préparer l'essai" })).toBeVisible();
    await capture(page, "planned-test-preparation-physical-asset-selector-1440x900.png");

    await page.route("**/api/v1/metrology/instruments", async (route) => {
      await route.fulfill({ status: 503, contentType: "application/json", body: JSON.stringify({ error: { code: "metrology_unavailable", message: "service métrologique indisponible" } }) });
    });
    await page.goto("/lab/");
    await page.getByRole("button", { name: "Parc matériel" }).click();
    await expect(page.getByText("Métrologie temporairement indisponible")).toBeVisible();
    await expect(page.getByRole("heading", { name: inventoryCode })).toBeVisible();
    await capture(page, "targeted-metrology-failure-1440x900.png");
    await page.unroute("**/api/v1/metrology/instruments");

    await page.route("**/api/v1/driver-profiles*", async (route) => {
      await route.fulfill({ status: 503, contentType: "application/json", body: JSON.stringify({ error: { code: "drivers_unavailable", message: "service de pilotage indisponible" } }) });
    });
    await page.goto("/lab/");
    await page.getByRole("button", { name: "Catalogue des modèles" }).click();
    await showDemoModels(page);
    await expect(page.getByText("Pilotage temporairement indisponible")).toBeVisible();
    await expect(page.getByRole("treeitem", { name: /NRP6AN/ })).toBeVisible();
    await capture(page, "targeted-driver-failure-1440x900.png");

    const normalText = await page.locator("main").innerText();
    expect(normalText).not.toContain("sha256:");
    expect(normalText).not.toContain(modelRevisionId);
  });
});

async function createLocation(request: APIRequestContext, label: string, description: string, operationSuffix: string) {
  const response = await request.post("/api/v1/laboratory-locations", {
    data: {
      label,
      description,
      actor: "fleet.e2e",
      reason: "préparer le scénario du parc 0.22.0",
      operation_id: `op-0220-create-${operationSuffix}`
    }
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  return (await response.json()).location as LaboratoryLocation;
}

async function ensureModelCategory(
  request: APIRequestContext,
  categoryId: string,
  rootCategoryId: string,
  label: string
) {
  const listed = await request.get("/api/v1/equipment/categories?include_inactive=true");
  expect(listed.ok(), await listed.text()).toBeTruthy();
  const categories = (await listed.json()).categories as Array<{ category_id: string }>;
  if (categories.some((category) => category.category_id === categoryId)) return;
  const created = await request.post("/api/v1/equipment/categories", {
    data: {
      category_id: categoryId,
      parent_category_id: rootCategoryId,
      label,
      description: "Catégorie du modèle de démonstration utilisée par la preuve 0.22.0.",
      sort_order: 100
    }
  });
  expect(created.ok(), await created.text()).toBeTruthy();
}

async function getAsset(request: APIRequestContext, id: string) {
  const response = await request.get(`/api/v1/fleet/assets/${id}`);
  expect(response.ok(), await response.text()).toBeTruthy();
  return (await response.json()).asset as PhysicalAsset;
}

function seedDemoData() {
  const agentUrl = process.env.LAB_CONSOLE_E2E_BASE_URL ?? "http://127.0.0.1:8765";
  for (const script of ["seed-equipment-demo.ps1", "seed-planned-test-preparation-demo.ps1"]) {
    execFileSync(
      "powershell.exe",
      ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", path.join(repoRoot, "scripts", script), "-AgentUrl", agentUrl],
      { cwd: repoRoot, encoding: "utf8", stdio: "pipe" }
    );
  }
}

async function openDemoModel(page: Page) {
  await page.goto("/lab/");
  await page.getByRole("button", { name: "Catalogue des modèles" }).click();
  await showDemoModels(page);
  await page.getByRole("treeitem", { name: /NRP6AN/ }).click();
  await expect(page.getByRole("heading", { name: /NRP6AN/ })).toBeVisible();
}

async function showDemoModels(page: Page) {
  const filters = page.locator("details.advancedFilters");
  if (!(await filters.evaluate((element) => (element as HTMLDetailsElement).open))) {
    await filters.locator("summary").click();
  }
  const refreshed = page.waitForResponse((response) =>
    response.url().includes("/api/v1/equipment-models?")
    && response.url().includes("demo_mode=show")
  );
  await page.getByLabel("Filtrer les données de démonstration").selectOption("show");
  expect((await refreshed).ok()).toBeTruthy();
  await expandModelHierarchy(page, /NRP6AN/);
  await expect(page.getByRole("treeitem", { name: /NRP6AN/ })).toBeVisible();
  if (await filters.evaluate((element) => (element as HTMLDetailsElement).open)) {
    await filters.locator("summary").click();
  }
}

async function expandModelHierarchy(page: Page, target: RegExp) {
  for (let round = 0; round < 8; round += 1) {
    if (await page.getByRole("treeitem", { name: target }).count()) return;
    const collapsed = page.locator('.modelHierarchy [role="treeitem"][aria-expanded="false"]');
    const count = await collapsed.count();
    if (count === 0) return;
    await collapsed.evaluateAll((elements) => {
      for (const element of elements) (element as HTMLElement).click();
    });
    await page.waitForTimeout(40);
  }
}

async function openPlannedPreparation(page: Page) {
  await page.goto("/lab/");
  await page.getByRole("button", { name: "Planning du laboratoire" }).click();
  await page.getByLabel("Atteindre une date").fill("2026-07-16");
  await page.getByRole("button", { name: /Ouvrir Verification RF du convertisseur Horizon, dossier CEM-DEMO-PREP-001/ }).click();
  const slot = page.getByRole("dialog");
  await slot.getByRole("button", { name: "Préparer l'essai" }).click();
}

async function capture(page: Page, filename: string) {
  if (!refreshScreenshots) return;
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(80);
  await mkdir(screenshotDirectory, { recursive: true });
  await page.screenshot({
    path: path.join(screenshotDirectory, filename),
    animations: "disabled",
    fullPage: false
  });
}

interface LaboratoryLocation {
  location_id: string;
  label: string;
  description: string;
  status: "active" | "archived";
  revision: number;
}

interface PhysicalAsset {
  asset_id: string;
  inventory_code: string;
  serial_number: string | null;
  part_number: string | null;
  equipment_model_id: string | null;
  equipment_model_revision_id: string | null;
  equipment_model_checksum: string | null;
  laboratory_location_id: string | null;
  ownership_source: string;
  notes: string;
  revision: number;
}
