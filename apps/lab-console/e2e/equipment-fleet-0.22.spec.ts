import { execFileSync, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
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
const archivedInventoryCode = "INV-0220-LIEU-ARCHIVE";
const noLocationInventoryCode = "INV-0220-SANS-LIEU";
const requiredCalibrationInventoryCode = "INV-0220-CAL-REQUIS";
const expiredCalibrationInventoryCode = "INV-0220-CAL-EXPIRE";
const nonconformingInventoryCode = "INV-0220-CAL-NON-CONFORME";
const unresolvedAssetId = "LEGACY-0220-UNRESOLVED";
const historicalScreenshotBaseline = historicalScreenshotHashes();
let assetId = "";
let archivedLocationAssetId = "";
let noLocationAssetId = "";
let requiredCalibrationAssetId = "";
let expiredCalibrationAssetId = "";
let nonconformingAssetId = "";
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
    const generatorResponse = await request.get("/api/v1/equipment-models/EQM-PRESET-RF-GENERATOR");
    expect(generatorResponse.ok(), await generatorResponse.text()).toBeTruthy();
    const generator = (await generatorResponse.json()).equipment_model;
    await ensureModelCategory(
      request,
      generator.identity.category_code,
      generator.identity.root_category_id
        || generator.current_approved_revision.definition.template_snapshot?.root_category_id
        || "energy_sources",
      generator.current_approved_revision.definition.template_snapshot?.category_path?.at(-1)
        ?? "Générateurs RF"
    );
    await ensureApprovedModelCategories(request);

    firstLocation = await createLocation(request, "Zone parc 0.22 A", "Emplacement initial du scénario parc", "location-a");
    secondLocation = await createLocation(request, "Zone parc 0.22 B", "Emplacement de destination du scénario parc", "location-b");

    const created = await createFleetAsset(request, {
      inventory_code: inventoryCode,
      part_number: "NRP6AN",
      laboratory_location_id: firstLocation.location_id,
      service_state_reason: "Vérifié avant mise au parc",
      notes: "Exemplaire volontairement enregistré sans numéro de série.",
      operation_id: "op-0220-create-no-serial",
      reason: "prouver l'identité principale par code inventaire"
    });
    assetId = created.asset_id;

    archivedLocationAssetId = (await createFleetAsset(request, {
      inventory_code: archivedInventoryCode,
      serial_number: "NRP6AN-0220-ARCHIVE",
      laboratory_location_id: firstLocation.location_id,
      operation_id: "op-0220-create-archived-location-asset",
      reason: "préparer la preuve de lieu archivé"
    })).asset_id;
    noLocationAssetId = (await createFleetAsset(request, {
      inventory_code: noLocationInventoryCode,
      ownership_source: "software_license",
      notes: "Licence de traitement en attente d'affectation.",
      operation_id: "op-0220-create-no-location",
      reason: "prouver qu'un emplacement peut rester indéfini"
    })).asset_id;
    requiredCalibrationAssetId = (await createFleetAsset(request, {
      inventory_code: requiredCalibrationInventoryCode,
      serial_number: "NRP6AN-0220-CAL",
      laboratory_location_id: secondLocation.location_id,
      calibration_requirement: "required",
      calibration_period_months: 12,
      operation_id: "op-0220-create-required-calibration",
      reason: "préparer la preuve de statut métrologique daté"
    })).asset_id;
    expiredCalibrationAssetId = (await createFleetAsset(request, {
      inventory_code: expiredCalibrationInventoryCode,
      serial_number: "NRP6AN-0220-EXP",
      laboratory_location_id: secondLocation.location_id,
      calibration_requirement: "required",
      calibration_period_months: 12,
      operation_id: "op-0220-create-expired-calibration",
      reason: "préparer la preuve d'étalonnage expiré"
    })).asset_id;
    nonconformingAssetId = (await createFleetAsset(request, {
      inventory_code: nonconformingInventoryCode,
      serial_number: "NRP6AN-0220-NC",
      laboratory_location_id: secondLocation.location_id,
      calibration_requirement: "required",
      calibration_period_months: 12,
      operation_id: "op-0220-create-nonconforming-calibration",
      reason: "préparer la preuve de décision non conforme"
    })).asset_id;
    seedUnresolvedMigratedFixture(unresolvedAssetId, secondLocation);
  });

  test.afterAll(() => {
    expect(historicalScreenshotHashes()).toEqual(historicalScreenshotBaseline);
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
    await expect(page.locator(".linkedFleetAssets").getByText("Sans numéro de série").first()).toBeVisible();
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

  test("supports serial, no-serial and no-location assets without inventing invalid states", async ({ page, request }) => {
    const assetsResponse = await request.get("/api/v1/fleet/assets");
    expect(assetsResponse.ok(), await assetsResponse.text()).toBeTruthy();
    const assets = (await assetsResponse.json()).assets as PhysicalAsset[];
    expect(assets.find((asset) => asset.asset_id === archivedLocationAssetId)?.serial_number)
      .toBe("NRP6AN-0220-ARCHIVE");
    expect(assets.find((asset) => asset.asset_id === assetId)?.serial_number).toBeNull();
    const noLocation = assets.find((asset) => asset.asset_id === noLocationAssetId);
    expect(noLocation?.laboratory_location_id).toBeNull();
    expect(noLocation?.ownership_source).toBe("software_license");

    const missingReason = await request.post("/api/v1/fleet/assets", {
      data: fleetAssetPayload({
        inventory_code: "INV-0220-INVALID-NO-REASON",
        service_state: "out_of_service",
        administrative_availability: "unavailable",
        administrative_unavailability_reason: "État technique bloquant",
        service_state_reason: "",
        operation_id: "op-0220-invalid-no-service-reason",
        reason: "vérifier le refus sans motif technique"
      })
    });
    const missingReasonBody = await missingReason.json();
    expect(missingReason.status(), JSON.stringify(missingReasonBody)).toBe(400);
    expect(JSON.stringify(missingReasonBody)).toContain("service_state_reason_required");

    const nonUsable = await createFleetAsset(request, {
      inventory_code: "INV-0220-HORS-SERVICE",
      serial_number: "NRP6AN-0220-HS",
      service_state: "out_of_service",
      administrative_availability: "unavailable",
      administrative_unavailability_reason: "Immobilisé par son état technique",
      service_state_reason: "Connecteur endommagé",
      operation_id: "op-0220-create-out-of-service",
      reason: "prouver la combinaison technique cohérente"
    });
    expect(nonUsable.administrative_availability).toBe("unavailable");

    for (const [state, operationId] of [
      ["reserved", "op-0220-reject-manual-reserved"],
      ["in_test", "op-0220-reject-manual-in-test"]
    ] as const) {
      const current = await getAsset(request, assetId);
      const rejected = await request.post(
        `/api/v1/fleet/assets/${assetId}/transitions/administrative-availability`,
        {
          data: {
            expected_revision: current.revision,
            administrative_availability: state,
            administrative_unavailability_reason: "",
            actor: "fleet.e2e",
            reason: "refuser un fait opérationnel inventé",
            operation_id: operationId
          }
        }
      );
      expect(rejected.status()).toBe(409);
      expect(JSON.stringify(await rejected.json())).toContain("operational_usage_cannot_be_set_manually");
    }

    await page.goto("/lab/");
    await page.getByRole("button", { name: "Catalogue des modèles" }).click();
    await showDemoModels(page);
    await page.getByRole("button", { name: "Parc matériel" }).click();
    await page.getByRole("button", { name: "Ajouter un exemplaire" }).click();
    const dialog = page.getByRole("dialog", { name: "Ajouter un exemplaire" });
    await dialog.getByLabel(/Modèle constructeur/).selectOption(modelId);
    await dialog.getByLabel(/Code inventaire/).fill("INV-0220-FORM-REASON");
    await dialog.getByLabel(/État de service/).selectOption("out_of_service");
    await expect(dialog.getByLabel(/Motif de l'état de service/)).toBeVisible();
    await expect(dialog.getByLabel(/Disponibilité administrative/)).toBeDisabled();
    await expect(dialog.getByLabel(/Disponibilité administrative/)).toHaveValue("unavailable");
    await dialog.getByRole("button", { name: "Enregistrer l'exemplaire" }).click({ force: true });
    await expect(dialog.getByLabel(/Motif de l'état de service/)).toBeFocused();
    await expect(dialog.getByText(/renseignez le motif de l'état de service/i)).toBeVisible();
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
    const moved = await request.post(`/api/v1/fleet/assets/${assetId}/transitions/move`, {
      data: {
        expected_revision: assetBeforeMove.revision,
        destination_location_id: secondLocation.location_id,
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
        administrative_availability: "available",
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

  test("edits an asset at an archived location and preserves readable movement history", async ({ page, request }) => {
    const atArchivedLocation = await getAsset(request, archivedLocationAssetId);
    expect(atArchivedLocation.laboratory_location_id).toBe(firstLocation.location_id);
    expect(atArchivedLocation.laboratory_location_label).toBe("Zone parc 0.22 A renommée");
    expect(atArchivedLocation.laboratory_location_status).toBe("archived");

    const identified = await request.put(
      `/api/v1/fleet/assets/${archivedLocationAssetId}/identification`,
      {
        data: {
          expected_revision: atArchivedLocation.revision,
          inventory_code: `${archivedInventoryCode}-MOD`,
          serial_number: atArchivedLocation.serial_number,
          part_number: "NRP6AN",
          ownership_source: atArchivedLocation.ownership_source,
          notes: "Identification modifiée sans réaffecter le lieu archivé.",
          actor: "fleet.e2e",
          reason: "prouver qu'un lieu archivé ne bloque pas l'identification",
          operation_id: "op-0220-identify-at-archived-location"
        }
      }
    );
    expect(identified.ok(), await identified.text()).toBeTruthy();
    const identifiedAsset = (await identified.json()).asset as PhysicalAsset;
    expect(identifiedAsset.laboratory_location_id).toBe(firstLocation.location_id);

    const primary = await getAsset(request, assetId);
    const archivedDestination = await request.post(
      `/api/v1/fleet/assets/${assetId}/transitions/move`,
      {
        data: {
          expected_revision: primary.revision,
          destination_location_id: firstLocation.location_id,
          actor: "fleet.e2e",
          reason: "vérifier le refus d'une destination archivée",
          operation_id: "op-0220-reject-move-to-archived"
        }
      }
    );
    expect(archivedDestination.status()).toBe(409);
    expect(JSON.stringify(await archivedDestination.json())).toContain("laboratory_location_archived");

    const moved = await request.post(
      `/api/v1/fleet/assets/${archivedLocationAssetId}/transitions/move`,
      {
        data: {
          expected_revision: identifiedAsset.revision,
          destination_location_id: secondLocation.location_id,
          actor: "fleet.e2e",
          reason: "sortir l'exemplaire du lieu archivé",
          operation_id: "op-0220-move-from-archived"
        }
      }
    );
    expect(moved.ok(), await moved.text()).toBeTruthy();

    const audit = await request.get(`/api/v1/fleet/assets/${archivedLocationAssetId}/audit-events`);
    expect(audit.ok(), await audit.text()).toBeTruthy();
    const auditText = JSON.stringify(await audit.json());
    expect(auditText).toContain("op-0220-identify-at-archived-location");
    expect(auditText).toContain("op-0220-move-from-archived");
    expect(auditText).toContain("Zone parc 0.22 A renommée");
    expect(auditText).toContain("Zone parc 0.22 B");

    await page.goto("/lab/");
    await page.getByRole("button", { name: "Parc matériel" }).click();
    await page.getByLabel("Rechercher dans le parc").fill(`${archivedInventoryCode}-MOD`);
    await page.getByRole("treeitem", { name: new RegExp(`${archivedInventoryCode}-MOD`) }).click();
    await page.getByRole("button", { name: "Historique" }).click();
    await expect(page.getByText("Modification du code inventaire et de l’identification")).toBeVisible();
    await expect(page.getByText("Déplacement de l’exemplaire").last()).toBeVisible();
    await expect(page.getByText(/Zone parc 0.22 A renommée.*Zone parc 0.22 B/)).toBeVisible();
  });

  test("reconciles a migrated asset to one exact immutable model revision", async ({ page, request }) => {
    const beforeOptions = await request.get(
      `/api/v1/station-setups/asset-options?planned_use_on=2026-08-15&execution_mode=investigation&laboratory_location_id=${secondLocation.location_id}`
    );
    expect(beforeOptions.ok(), await beforeOptions.text()).toBeTruthy();
    const beforeOption = (await beforeOptions.json()).assets.find(
      (option: ExecutableAssetOption) => option.asset.asset_id === unresolvedAssetId
    ) as ExecutableAssetOption;
    expect(beforeOption.eligible).toBe(false);
    expect(beforeOption.blocking_reasons.map((reason) => reason.code)).toContain("model_reconciliation_required");
    const candidatesResponse = await request.get("/api/v1/fleet/model-reconciliation-candidates");
    expect(candidatesResponse.ok(), await candidatesResponse.text()).toBeTruthy();

    await page.goto("/lab/");
    await page.getByRole("button", { name: "Parc matériel" }).click();
    await page.getByLabel("Rechercher dans le parc").fill(unresolvedAssetId);
    await page.getByRole("treeitem", { name: new RegExp(unresolvedAssetId) }).click();
    await expect(page.getByText("Modèle constructeur à rapprocher")).toBeVisible();
    await expect(page.getByText(/ancien registre métrologique/)).toBeVisible();
    await page.getByRole("button", { name: "Historique" }).click();
    await expect(page.getByText("Import depuis l’ancien registre métrologique")).toBeVisible();

    await page.getByRole("button", { name: "Résumé" }).click();
    await page.getByRole("button", { name: "Rapprocher avec un modèle constructeur" }).click();
    const dialog = page.getByRole("dialog", { name: "Rapprocher avec un modèle constructeur" });
    await expect(dialog.getByLabel(/Modèle et version/)).toBeEnabled();
    await dialog.getByLabel(/Modèle et version/).selectOption(modelRevisionId);
    const reconciliationResponse = page.waitForResponse((response) =>
      response.url().endsWith(`/api/v1/fleet/assets/${unresolvedAssetId}/transitions/reconcile-model`)
      && response.request().method() === "POST"
    );
    await dialog.getByRole("button", { name: "Rapprocher cette version" }).click();
    expect((await reconciliationResponse).ok()).toBeTruthy();

    const reconciled = await getAsset(request, unresolvedAssetId);
    expect(reconciled.model_link_state).toBe("resolved");
    expect(reconciled.equipment_model_revision_id).toBe(modelRevisionId);
    expect(reconciled.equipment_model_checksum).toBe(modelChecksum);
    expect(reconciled.manufacturer).not.toBe("Ancien fabricant");

    const repoint = await request.post(
      `/api/v1/fleet/assets/${unresolvedAssetId}/transitions/reconcile-model`,
      {
        data: {
          expected_revision: reconciled.revision,
          equipment_model_id: modelId,
          equipment_model_revision_id: modelRevisionId,
          actor: "fleet.e2e",
          reason: "vérifier l'absence de repointage silencieux",
          operation_id: "op-0220-reject-second-reconciliation"
        }
      }
    );
    expect(repoint.status()).toBe(409);
    expect(JSON.stringify(await repoint.json())).toContain("physical_asset_model_already_resolved");

    const afterOptions = await request.get(
      `/api/v1/station-setups/asset-options?planned_use_on=2026-08-15&execution_mode=investigation&laboratory_location_id=${secondLocation.location_id}`
    );
    expect(afterOptions.ok(), await afterOptions.text()).toBeTruthy();
    const afterOption = (await afterOptions.json()).assets.find(
      (option: ExecutableAssetOption) => option.asset.asset_id === unresolvedAssetId
    ) as ExecutableAssetOption;
    expect(afterOption.eligible, JSON.stringify(afterOption.blocking_reasons)).toBe(true);

    const audit = await request.get(`/api/v1/fleet/assets/${unresolvedAssetId}/audit-events`);
    const auditText = JSON.stringify(await audit.json());
    expect(auditText).toContain("physical_asset_migrated_from_metrology");
    expect(auditText).toContain("physical_asset_model_reconciled");
    const outbox = await request.get("/api/v1/sync/outbox");
    const outboxText = JSON.stringify(await outbox.json());
    expect(outboxText).toContain(unresolvedAssetId);
    expect(outboxText).not.toContain("op-0220-reject-second-reconciliation");
  });

  test("renders one authoritative dated metrology contract", async ({ page, request }) => {
    await recordCalibration(request, requiredCalibrationAssetId, {
      eventId: "CAL-0220-VALID",
      decision: "conforming",
      calibratedAt: "2026-06-30",
      dueAt: "2027-06-30"
    });
    await recordCalibration(request, expiredCalibrationAssetId, {
      eventId: "CAL-0220-EXPIRED",
      decision: "conforming",
      calibratedAt: "2025-07-26",
      dueAt: "2026-07-26"
    });
    await recordCalibration(request, nonconformingAssetId, {
      eventId: "CAL-0220-NONCONFORMING",
      decision: "nonconforming",
      calibratedAt: "2026-07-20",
      dueAt: "2027-07-20"
    });

    const statuses = await Promise.all([
      metrologyStatus(request, requiredCalibrationAssetId, "2026-07-27"),
      metrologyStatus(request, requiredCalibrationAssetId, "2027-06-15"),
      metrologyStatus(request, requiredCalibrationAssetId, "2027-07-01"),
      metrologyStatus(request, expiredCalibrationAssetId, "2026-07-27"),
      metrologyStatus(request, nonconformingAssetId, "2026-07-27")
    ]);
    expect(statuses.map((status) => status.calibration_status)).toEqual([
      "valid",
      "due_soon",
      "expired",
      "expired",
      "nonconforming"
    ]);

    await page.goto("/lab/");
    await page.getByRole("button", { name: "Parc matériel" }).click();
    await openFleetAsset(page, requiredCalibrationInventoryCode);
    await expect(page.getByText(/Étalonnage valide jusqu’au/).first()).toBeVisible();
    await openFleetAsset(page, expiredCalibrationInventoryCode);
    await expect(page.getByText(/Étalonnage expiré depuis le/).first()).toBeVisible();
    await expect(page.getByText(/Étalonnage valide jusqu’au/)).toHaveCount(0);
    await openFleetAsset(page, nonconformingInventoryCode);
    await expect(page.getByText("Dernier étalonnage non conforme").first()).toBeVisible();
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

  test("keeps primary identity and work context visible during secondary failures", async ({ page }) => {
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
    await openFleetAsset(page, inventoryCode);

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
    await openFleetAsset(page, inventoryCode);

    await page.unroute("**/api/v1/driver-profiles*");
    await page.route(`**/api/v1/equipment-models/${modelId}/revisions`, async (route) => {
      await route.fulfill({
        status: 503,
        contentType: "application/json",
        body: JSON.stringify({ error: { code: "revisions_unavailable", message: "versions indisponibles pour le scénario" } })
      });
    });
    await page.route(`**/api/v1/equipment-models/${modelId}/audit-events`, async (route) => {
      await route.fulfill({
        status: 503,
        contentType: "application/json",
        body: JSON.stringify({ error: { code: "audit_unavailable", message: "audit indisponible pour le scénario" } })
      });
    });
    await openDemoModel(page);
    await expect(page.getByRole("heading", { name: /NRP6AN/ })).toBeVisible();
    await expect(page.getByText("Historique des versions indisponible")).toBeVisible();
    await expect(page.getByText("Historique des modifications indisponible")).toBeVisible();

    await page.unroute(`**/api/v1/equipment-models/${modelId}/revisions`);
    await page.unroute(`**/api/v1/equipment-models/${modelId}/audit-events`);
    await page.route("**/api/v1/projects/CEM-DEMO-PREP-001/schedule-items/PLAN-DEMO-PREP-001/preparation/options", async (route) => {
      await route.fulfill({
        status: 503,
        contentType: "application/json",
        body: JSON.stringify({ error: { code: "options_unavailable", message: "choix matériels indisponibles pour le scénario" } })
      });
    });
    await openPlannedPreparation(page);
    const preparation = page.getByRole("dialog", { name: "Préparer l'essai" });
    await expect(preparation.getByText("Verification RF du convertisseur Horizon")).toBeVisible();
    await expect(preparation.getByText("Convertisseur Horizon HCU-4")).toBeVisible();
    await expect(preparation.getByText("Choix de préparation temporairement indisponibles")).toBeVisible();
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
    await openFleetAsset(page, inventoryCode);
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
    await openFleetAsset(page, inventoryCode);
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

  test("derives reservation and active-test usage from the real planned workflow", async ({ request }) => {
    const scheduleResponse = await request.get("/api/v1/projects/CEM-DEMO-PREP-001/schedule-items");
    expect(scheduleResponse.ok(), await scheduleResponse.text()).toBeTruthy();
    const schedule = (await scheduleResponse.json()).schedule_items.find(
      (candidate: { item_code: string }) => candidate.item_code === "PLAN-DEMO-PREP-001"
    );
    expect(schedule.equipment_under_test).toBe("Convertisseur Horizon HCU-4");

    const currentPreparationResponse = await request.get(
      "/api/v1/projects/CEM-DEMO-PREP-001/schedule-items/PLAN-DEMO-PREP-001/preparation"
    );
    expect(currentPreparationResponse.ok(), await currentPreparationResponse.text()).toBeTruthy();
    const currentPreparation = (await currentPreparationResponse.json()).preparation;
    const assessment = await request.post(
      "/api/v1/projects/CEM-DEMO-PREP-001/schedule-items/PLAN-DEMO-PREP-001/preparation/assessments",
      {
        data: {
          expected_schedule_revision: schedule.revision,
          expected_current_revision_id: currentPreparation.current_revision?.revision_id ?? null,
          method_template_id: "METHOD-DEMO-RF-PREP",
          method_revision_id: "METHOD-DEMO-RF-PREP-rev-0001",
          station_setup_id: "SETUP-DEMO-RF-PREP",
          station_setup_revision_id: "SETUP-DEMO-RF-PREP-rev-0001",
          assignments: [{ slot_id: "measurement_receiver", binding_id: "power_meter" }],
          actor: "fleet.e2e",
          reason: "réserver le wattmètre par une vraie préparation",
          operation_id: "op-0220-assess-ready-preparation",
          device_id: "playwright-fleet-0220",
          correlation_id: "corr-0220-real-usage"
        }
      }
    );
    expect(assessment.ok(), await assessment.text()).toBeTruthy();
    const readyPreparation = (await assessment.json()).preparation;
    expect(readyPreparation.current_state).toBe("ready");
    expect(readyPreparation.current_revision.definition.station_setup.assets.some(
      (asset: { asset_id: string }) => asset.asset_id === "PM-DEMO-RF-001"
    )).toBe(true);

    const reservedAssets = await fleetAt(request, "2026-07-16T10:00:00Z", "2026-07-16");
    const reserved = reservedAssets.find((asset) => asset.asset_id === "PM-DEMO-RF-001")!;
    expect(reserved.operational_usage.state).toBe("reserved");
    expect(reserved.operational_usage.evidence.some((item) =>
      item.source_identifier === "PLAN-DEMO-PREP-001" && item.blocks_selection
    )).toBe(true);

    const started = await request.post(
      "/api/v1/projects/CEM-DEMO-PREP-001/schedule-items/PLAN-DEMO-PREP-001/transitions/start",
      {
        data: {
          expected_revision: schedule.revision,
          expected_preparation_revision_id: readyPreparation.current_revision.revision_id,
          expected_preparation_checksum: readyPreparation.current_revision.definition_checksum,
          actor: "fleet.e2e",
          reason: "démarrer le vrai essai préparé",
          operation_id: "op-0220-start-real-test",
          device_id: "playwright-fleet-0220",
          correlation_id: "corr-0220-real-usage"
        }
      }
    );
    expect(started.ok(), await started.text()).toBeTruthy();
    expect((await started.json()).schedule_item.status).toBe("in_progress");

    const inTestAssets = await fleetAt(request, "2026-07-16T10:00:00Z", "2026-07-16");
    const inTest = inTestAssets.find((asset) => asset.asset_id === "PM-DEMO-RF-001")!;
    expect(inTest.operational_usage.state).toBe("in_test");
    expect(inTest.operational_usage.evidence.some((item) =>
      item.source_identifier === "PLAN-DEMO-PREP-001" && item.blocks_selection
    )).toBe(true);
  });

  test("restarts the Local Agent and reloads fleet, audit and outbox evidence", async ({ request }) => {
    await restartAgent(request);

    const reloaded = await getAsset(request, unresolvedAssetId);
    expect(reloaded.model_link_state).toBe("resolved");
    expect(reloaded.equipment_model_revision_id).toBe(modelRevisionId);
    const noLocation = await getAsset(request, noLocationAssetId);
    expect(noLocation.laboratory_location_id).toBeNull();

    const audit = await request.get(`/api/v1/fleet/assets/${archivedLocationAssetId}/audit-events`);
    expect(audit.ok(), await audit.text()).toBeTruthy();
    expect(JSON.stringify(await audit.json())).toContain("op-0220-move-from-archived");
    const outbox = await request.get("/api/v1/sync/outbox");
    expect(outbox.ok(), await outbox.text()).toBeTruthy();
    expect(JSON.stringify(await outbox.json())).toContain("op-0220-start-real-test");
  });
});

function fleetAssetPayload(input: FleetAssetOverrides) {
  return {
    inventory_code: input.inventory_code,
    serial_number: input.serial_number,
    part_number: input.part_number ?? "NRP6AN",
    equipment_model_id: modelId,
    laboratory_location_id: input.laboratory_location_id,
    ownership_source: input.ownership_source ?? "laboratory_owned",
    service_state: input.service_state ?? "usable",
    administrative_availability: input.administrative_availability ?? "available",
    administrative_unavailability_reason: input.administrative_unavailability_reason,
    service_state_reason: input.service_state_reason ?? "Contrôle initial conforme",
    notes: input.notes ?? "Preuve réelle du parc 0.22.0.",
    calibration_requirement: input.calibration_requirement ?? "not_required",
    calibration_period_months: input.calibration_period_months,
    calibration_due_warning_days: 30,
    actor: "fleet.e2e",
    reason: input.reason,
    operation_id: input.operation_id,
    device_id: "playwright-fleet-0220",
    correlation_id: "corr-fleet-0220"
  };
}

async function createFleetAsset(request: APIRequestContext, input: FleetAssetOverrides) {
  const response = await request.post("/api/v1/fleet/assets", {
    data: fleetAssetPayload(input)
  });
  expect(response.ok(), await response.text()).toBeTruthy();
  return (await response.json()).asset as PhysicalAsset;
}

async function recordCalibration(
  request: APIRequestContext,
  targetAssetId: string,
  input: {
    eventId: string;
    decision: "conforming" | "nonconforming" | "indeterminate";
    calibratedAt: string;
    dueAt: string;
  }
) {
  const response = await request.post(`/api/v1/metrology/instruments/${targetAssetId}/calibrations`, {
    data: {
      event_id: input.eventId,
      certificate_reference: `CERT-${input.eventId}`,
      calibrated_at: input.calibratedAt,
      due_at: input.dueAt,
      provider: "Laboratoire accrédité E2E",
      decision: input.decision,
      uncertainty_summary: { amplitude_db: 0.4 },
      document_manifest: {
        object_id: `obj-${input.eventId.toLowerCase()}`,
        original_filename: `${input.eventId}.pdf`,
        mime_type: "application/pdf",
        size_bytes: 128,
        sha256: "c".repeat(64),
        storage_key: `metrology/${targetAssetId}/${input.eventId}.pdf`,
        revision: "A"
      },
      recorded_by: "metrology.e2e",
      actor: "metrology.e2e",
      reason: "enregistrer la décision métrologique de preuve",
      operation_id: `op-0220-record-${input.eventId}`,
      device_id: "playwright-fleet-0220",
      correlation_id: "corr-metrology-0220"
    }
  });
  expect(response.ok(), await response.text()).toBeTruthy();
}

async function metrologyStatus(
  request: APIRequestContext,
  targetAssetId: string,
  checkedOn: string
) {
  const response = await request.get(
    `/api/v1/metrology/instruments/${targetAssetId}/status?checked_on=${checkedOn}`
  );
  expect(response.ok(), await response.text()).toBeTruthy();
  return await response.json() as { calibration_status: string };
}

async function fleetAt(request: APIRequestContext, assessedAt: string, checkedOn: string) {
  const query = new URLSearchParams({ at: assessedAt, checked_on: checkedOn });
  const response = await request.get(`/api/v1/fleet/assets?${query.toString()}`);
  expect(response.ok(), await response.text()).toBeTruthy();
  return (await response.json()).assets as PhysicalAsset[];
}

async function openFleetAsset(page: Page, code: string) {
  await page.getByLabel("Rechercher dans le parc").fill(code);
  await page.getByRole("treeitem", { name: new RegExp(code) }).click();
  await expect(page.getByRole("heading", { name: code })).toBeVisible();
}

function seedUnresolvedMigratedFixture(asset: string, location: LaboratoryLocation) {
  const storageRoot = process.env.LAB_CONSOLE_E2E_STORAGE_ROOT;
  if (!storageRoot) throw new Error("LAB_CONSOLE_E2E_STORAGE_ROOT is required for the migrated fixture");
  const python = String.raw`
import hashlib, json, pathlib, sqlite3, sys
root = pathlib.Path(sys.argv[1])
asset_id, location_id, location_label = sys.argv[2:5]
occurred_at = "2026-07-01T00:00:00Z"
operation_id = "op-0220-import-unresolved"
migration = {"source":"metrology.sqlite/legacy_instruments_0_21_1","legacy_asset_id":asset_id,"legacy_model_reference":None}
payload = {"inventory_code":asset_id,"migration_evidence":migration}
payload_json = json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
checksum = "sha256:" + hashlib.sha256(payload_json.encode("utf-8")).hexdigest()
equipment = sqlite3.connect(root / "equipment.sqlite")
equipment.execute("PRAGMA foreign_keys = ON")
equipment.execute("""INSERT INTO physical_assets (
  asset_id, inventory_code, serial_number, part_number,
  equipment_model_id, equipment_model_revision_id, equipment_model_checksum,
  manufacturer_snapshot, model_name_snapshot, variant_snapshot,
  category_code_snapshot, category_path_json, laboratory_location_id,
  laboratory_location_label_snapshot, ownership_source, service_state,
  availability_state, service_state_reason, notes, revision,
  model_link_state, migrated_from_metrology, created_at, updated_at,
  migration_evidence_json
) VALUES (?, ?, NULL, NULL, NULL, NULL, NULL,
  'Ancien fabricant', 'Modèle à rapprocher', NULL, 'legacy_metrology',
  '["Ancien registre"]', ?, ?, 'laboratory_owned', 'usable',
  'available', '', 'Import historique', 1, 'migration_review_required', 1,
  ?, ?, ?)""", (asset_id, asset_id, location_id, location_label, occurred_at, occurred_at,
  json.dumps(migration, ensure_ascii=False, sort_keys=True, separators=(",", ":"))))
equipment.execute("INSERT INTO physical_asset_operations VALUES (?, ?, ?, ?, 1, ?)",
  (operation_id, asset_id, "physical_asset_migrated_from_metrology", checksum, occurred_at))
equipment.execute("""INSERT INTO physical_asset_audit_events (
  asset_id, sequence, action, actor, reason, old_revision, new_revision,
  operation_id, device_id, correlation_id, payload_json, payload_checksum, occurred_at
) VALUES (?, 1, 'physical_asset_migrated_from_metrology', 'migration.0.22.0',
  'Préserver un exemplaire de l’ancien registre', NULL, 1, ?, 'migration',
  'corr-migration-0220', ?, ?, ?)""", (asset_id, operation_id, payload_json, checksum, occurred_at))
equipment.commit()
equipment.close()
metrology = sqlite3.connect(root / "metrology.sqlite")
metrology.execute("""INSERT INTO metrology_asset_dossiers (
  asset_id, calibration_requirement, calibration_period_months,
  calibration_due_warning_days, metrology_notes, legacy_capabilities_json,
  revision, created_at, updated_at
) VALUES (?, 'not_required', NULL, 30, '', '[]', 1, ?, ?)""", (asset_id, occurred_at, occurred_at))
metrology.commit()
metrology.close()
`;
  execFileSync("py", ["-3", "-c", python, storageRoot, asset, location.location_id, location.label], {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: "pipe"
  });
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
  ], {
    cwd: repoRoot,
    windowsHide: true,
    stdio: "ignore"
  });
  if (!restarted.pid) throw new Error("The restarted Local Agent has no process identifier");
  writeFileSync(pidFile, String(restarted.pid), "utf8");
  restarted.unref();

  for (let attempt = 0; attempt < 120; attempt += 1) {
    try {
      const health = await request.get("/api/v1/health", { timeout: 500 });
      if (health.ok()) return;
    } catch {
      // The port is expected to be temporarily unavailable during restart.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error("The restarted Local Agent did not become ready");
}

function historicalScreenshotHashes() {
  const result: Record<string, string> = {};
  for (const version of ["0.18.0", "0.19.0", "0.20.0", "0.21.0", "0.21.1"]) {
    const directory = path.join(repoRoot, "docs", "ux", version);
    for (const file of listFiles(directory)) {
      if (!file.toLowerCase().endsWith(".png")) continue;
      const relative = path.relative(repoRoot, file).replaceAll("\\", "/");
      result[relative] = createHash("sha256").update(readFileSync(file)).digest("hex");
    }
  }
  return result;
}

function listFiles(directory: string): string[] {
  const files: string[] = [];
  for (const entry of readdirSync(directory)) {
    const candidate = path.join(directory, entry);
    if (statSync(candidate).isDirectory()) files.push(...listFiles(candidate));
    else files.push(candidate);
  }
  return files;
}

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

async function ensureApprovedModelCategories(request: APIRequestContext) {
  const response = await request.get("/api/v1/equipment-models?demo_mode=show");
  expect(response.ok(), await response.text()).toBeTruthy();
  const models = (await response.json()).equipment_models as Array<{
    identity: { category_code: string; root_category_id?: string };
    current_approved_revision?: {
      definition: {
        template_snapshot?: { root_category_id?: string; category_path?: string[] };
      };
    };
  }>;
  const fallbackRoots: Record<string, string> = {
    power_meter: "measurement_instruments_digitizers",
    rf_amplifier: "rf_equipment",
    can_bus_power_unit: "energy_sources",
    antenna: "sensors_transducers",
    dc_power_supply: "energy_sources",
    rf_generator: "signal_sources",
    transmitting_antenna: "actuators_emitters",
    adc_converter: "measurement_instruments_digitizers",
    daq_card: "measurement_instruments_digitizers",
    test_acquisition_software: "processing_control_systems"
  };
  for (const model of models) {
    const revision = model.current_approved_revision;
    if (!revision) continue;
    const rootCategoryId = model.identity.root_category_id
      || revision.definition.template_snapshot?.root_category_id
      || fallbackRoots[model.identity.category_code];
    if (!rootCategoryId || rootCategoryId === model.identity.category_code) continue;
    await ensureModelCategory(
      request,
      model.identity.category_code,
      rootCategoryId,
      revision.definition.template_snapshot?.category_path?.at(-1)
        ?? model.identity.category_code.replaceAll("_", " ")
    );
  }
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
  manufacturer: string;
  model_name: string;
  variant: string | null;
  laboratory_location_id: string | null;
  laboratory_location_label: string | null;
  laboratory_location_status: "active" | "archived" | null;
  ownership_source: string;
  service_state: string;
  administrative_availability: string;
  operational_usage: {
    state: string;
    evidence: Array<{
      source_identifier: string;
      blocks_selection: boolean;
    }>;
  };
  notes: string;
  revision: number;
  model_link_state: string;
}

interface ExecutableAssetOption {
  asset: PhysicalAsset;
  eligible: boolean;
  blocking_reasons: Array<{ code: string; message: string; next_action: string }>;
  warnings: Array<{ code: string; message: string; next_action: string }>;
}

interface FleetAssetOverrides {
  inventory_code: string;
  operation_id: string;
  reason: string;
  serial_number?: string;
  part_number?: string;
  laboratory_location_id?: string;
  ownership_source?: string;
  service_state?: string;
  administrative_availability?: string;
  administrative_unavailability_reason?: string;
  service_state_reason?: string;
  notes?: string;
  calibration_requirement?: string;
  calibration_period_months?: number;
}
