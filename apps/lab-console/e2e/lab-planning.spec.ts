import { expect, test } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const viewports = [
  { width: 1440, height: 900 },
  { width: 1280, height: 720 }
];

test("an investigation dossier reaches a confirmed laboratory slot", async ({ page, request }) => {
  const suffix = Date.now().toString(36).toUpperCase();
  const projectCode = `CEM-E2E-${suffix}`;
  await page.route("**/api/v1/station-setups", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        station_setups: [
          {
            current_ready_revision: {
              definition: {
                laboratory_location_id: "LAB-LOCATION-CEM-1",
                laboratory_location_label: "Labo CEM 1"
              }
            }
          }
        ]
      })
    });
  });

  await page.setViewportSize(viewports[0]);
  await page.goto("/lab/");
  await page.getByRole("button", { name: "Dossiers d'essai" }).click();
  await expect(page.getByText("Aucun dossier d'essai.")).toBeVisible();

  await page.getByRole("button", { name: "Nouveau dossier" }).first().click();
  await page.getByLabel("Référence du dossier").fill(projectCode);
  await page.getByLabel("Client").fill("Industries Atlas");
  await page.getByRole("radio", { name: /Investigation/ }).check();
  await page.getByLabel("Responsable du dossier").fill("Claire Martin");
  const createResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/projects") &&
      response.request().method() === "POST"
  );
  await page.getByRole("button", { name: "Ouvrir le dossier" }).click();
  expect((await createResponse).ok()).toBeTruthy();

  await expect(page.getByRole("heading", { name: projectCode })).toBeVisible();
  await expect(page.getByText("Investigation", { exact: true })).toBeVisible();
  await completeReviewItem(page, "La demande du client est définie");
  await completeReviewItem(page, "Les écarts et adaptations sont consignés");

  const planningResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(`/api/v1/projects/${projectCode}/transitions/to-test-planning`) &&
      response.request().method() === "POST"
  );
  await page.getByRole("button", { name: "Passer à la planification" }).click();
  expect((await planningResponse).ok()).toBeTruthy();

  await page.getByRole("button", { name: "Planifier un essai" }).first().click();
  await page.getByLabel("Essai prévu").fill("Émission conduite");
  await page.getByLabel("Lieu").selectOption({ label: "Labo CEM 1" });
  await page.getByLabel("Équipement à tester").fill("Convertisseur prototype");
  const scheduleResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(`/api/v1/projects/${projectCode}/schedule-items`) &&
      response.request().method() === "POST"
  );
  await page.getByRole("button", { name: "Réserver le créneau" }).click();
  expect((await scheduleResponse).ok()).toBeTruthy();

  await expect(page.getByText("Émission conduite")).toBeVisible();
  await expect(page.getByText("Prévu", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Planifier un essai" }).click();
  await page.getByLabel("Essai prévu").fill("Essai en conflit");
  await page.getByLabel("Lieu").selectOption({ label: "Labo CEM 1" });
  await page.getByLabel("Équipement à tester").fill("Second prototype");
  const conflictResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(`/api/v1/projects/${projectCode}/schedule-items`) &&
      response.request().method() === "POST"
  );
  await page.getByRole("button", { name: "Réserver le créneau" }).click();
  expect((await conflictResponse).status()).toBe(409);
  await expect(page.getByRole("alert")).toContainText(
    "Claire Martin est déjà affecté au créneau « Émission conduite »"
  );
  await page.getByRole("button", { name: "Fermer", exact: true }).click();
  await expect(page.getByText("Essai en conflit")).toHaveCount(0);

  const confirmationResponse = page.waitForResponse(
    (response) =>
      response.url().includes(`/api/v1/projects/${projectCode}/schedule-items/`) &&
      response.url().endsWith("/transitions/confirm") &&
      response.request().method() === "POST"
  );
  await page.getByRole("button", { name: "Confirmer le créneau" }).click();
  expect((await confirmationResponse).ok()).toBeTruthy();

  await expect(page.getByText("Planning à jour")).toBeVisible();
  await expect(page.getByText("Confirmé", { exact: true })).toBeVisible();
  await page.getByText("Historique du dossier").click();
  await expect(page.getByText("Créneau d'essai réservé")).toBeVisible();
  await expect(page.getByText("État du créneau modifié")).toBeVisible();

  const persistedProject = await request.get(`/api/v1/projects/${projectCode}`);
  expect(persistedProject.ok(), await persistedProject.text()).toBeTruthy();
  expect((await persistedProject.json()).project.stage).toBe("test_planning");

  const persistedSchedule = await request.get(
    `/api/v1/projects/${projectCode}/schedule-items`
  );
  expect(persistedSchedule.ok(), await persistedSchedule.text()).toBeTruthy();
  const scheduleBody = await persistedSchedule.json();
  expect(scheduleBody.schedule_items).toHaveLength(1);
  expect(scheduleBody.schedule_items[0].status).toBe("confirmed");

  const auditResponse = await request.get(`/api/v1/projects/${projectCode}/audit-events`);
  expect(auditResponse.ok(), await auditResponse.text()).toBeTruthy();
  const auditBody = await auditResponse.json();
  expect(auditBody.audit_events.map((event: { action: string }) => event.action)).toEqual(
    expect.arrayContaining([
      "project_created",
      "contract_review_item_completed",
      "project_stage_advanced",
      "service_schedule_item_planned",
      "service_schedule_item_status_changed"
    ])
  );

  const outboxResponse = await request.get("/api/v1/sync/outbox");
  expect(outboxResponse.ok(), await outboxResponse.text()).toBeTruthy();
  expect(JSON.stringify(await outboxResponse.json())).toContain(projectCode);

  for (const viewport of viewports) {
    await page.setViewportSize(viewport);
    await page.goto("/lab/");
    await page.getByRole("button", { name: "Dossiers d'essai" }).click();
    await expect(page.getByRole("heading", { name: projectCode })).toBeVisible();
    await expect(page.getByText("Planning à jour")).toBeVisible();
    await assertNoHorizontalOverflow(page);
    await captureReleaseScreenshot(page, `dossier-planifie-${viewport.width}x${viewport.height}.png`);
  }
});

async function completeReviewItem(page: import("@playwright/test").Page, label: string) {
  const checkbox = page.getByRole("checkbox", { name: label });
  const response = page.waitForResponse(
    (candidate) =>
      candidate.url().includes("/contract-review/items/") &&
      candidate.url().endsWith("/complete") &&
      candidate.request().method() === "POST"
  );
  await checkbox.click();
  expect((await response).ok()).toBeTruthy();
  await expect(page.getByRole("checkbox", { name: label })).toBeChecked();
}

async function assertNoHorizontalOverflow(page: import("@playwright/test").Page) {
  const dimensions = await page.evaluate(() => ({
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth
  }));
  expect(dimensions.scrollWidth).toBeLessThanOrEqual(dimensions.clientWidth);
}

async function captureReleaseScreenshot(
  page: import("@playwright/test").Page,
  name: string
) {
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(80);
  const body = await page.screenshot({ animations: "disabled" });
  if (process.env.EMC_LOCUS_REFRESH_HISTORICAL_SCREENSHOTS !== "1") return;
  const evidenceDirectory = path.resolve(
    process.cwd(),
    "../../docs/ux/0.19.0/screenshots"
  );
  await mkdir(evidenceDirectory, { recursive: true });
  await writeFile(path.join(evidenceDirectory, name), body);
}
