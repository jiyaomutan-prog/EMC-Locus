import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const viewports = [
  { width: 1440, height: 900 },
  { width: 1280, height: 720 }
];

test("a planner coordinates and reschedules a multi-project laboratory week", async ({
  page,
  request
}) => {
  const suffix = Date.now().toString(36).toUpperCase();
  const firstProject = `CEM-WEEK-A-${suffix}`;
  const secondProject = `CEM-WEEK-B-${suffix}`;
  const firstItem = `PLAN-WEEK-A-${suffix}`;
  const secondItem = `PLAN-WEEK-B-${suffix}`;
  const firstOperator = `Alice ${suffix}`;
  const secondOperator = `Bob ${suffix}`;
  const weekStart = mondayFor(new Date());
  const firstDate = addDays(weekStart, 1);
  const secondDate = addDays(weekStart, 2);
  const movedDate = addDays(weekStart, 3);

  await preparePlanningProject(request, firstProject, "Industries Atlas", suffix + "-A");
  await preparePlanningProject(request, secondProject, "Mobilités Boréal", suffix + "-B");
  await createScheduleItem(request, {
    projectCode: firstProject,
    itemCode: firstItem,
    title: "Émission conduite",
    date: firstDate,
    operator: firstOperator,
    location: `Labo Planning A ${suffix}`,
    equipment: "Convertisseur Atlas"
  });
  await createScheduleItem(request, {
    projectCode: secondProject,
    itemCode: secondItem,
    title: "Immunité rayonnée",
    date: secondDate,
    operator: secondOperator,
    location: `Chambre Planning B ${suffix}`,
    equipment: "Calculateur Boréal"
  });

  await page.setViewportSize(viewports[0]);
  await page.goto("/lab/");
  await page.getByRole("button", { name: "Planning du laboratoire" }).click();
  await expect(page.getByText(`${firstProject} · Industries Atlas`)).toBeVisible();
  await expect(page.getByText(`${secondProject} · Mobilités Boréal`)).toBeVisible();

  await page.getByLabel("Opérateur").selectOption(firstOperator);
  await expect(page.getByText(`${firstProject} · Industries Atlas`)).toBeVisible();
  await expect(page.getByText(`${secondProject} · Mobilités Boréal`)).toHaveCount(0);
  await page.getByRole("button", { name: "Effacer les filtres" }).click();

  await page
    .getByRole("button", { name: `Ouvrir Immunité rayonnée, dossier ${secondProject}` })
    .click();
  await page.getByRole("button", { name: "Déplacer" }).click();
  const rescheduleDialog = page.getByRole("dialog");
  await rescheduleDialog.getByLabel("Date", { exact: true }).fill(firstDate);
  await rescheduleDialog.getByLabel("Début", { exact: true }).fill("10:00");
  await rescheduleDialog.getByLabel("Fin", { exact: true }).fill("11:00");
  await rescheduleDialog.getByLabel("Opérateur", { exact: true }).fill(firstOperator);
  await rescheduleDialog
    .getByLabel("Raison du changement")
    .fill("Réorganisation de la semaine d'essais");

  const conflictResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(
        `/api/v1/projects/${secondProject}/schedule-items/${secondItem}/reschedule`
      ) && response.request().method() === "POST"
  );
  await rescheduleDialog.getByRole("button", { name: "Enregistrer le déplacement" }).click();
  expect((await conflictResponse).status()).toBe(409);
  await expect(page.getByRole("alert")).toContainText(
    `${firstOperator} est déjà affecté à « Émission conduite » du dossier ${firstProject}`
  );
  await expect(rescheduleDialog.getByLabel("Raison du changement")).toHaveValue(
    "Réorganisation de la semaine d'essais"
  );
  await expect(rescheduleDialog.getByLabel("Date", { exact: true })).toHaveValue(firstDate);

  await rescheduleDialog.getByLabel("Date", { exact: true }).fill(movedDate);
  await rescheduleDialog.getByLabel("Début", { exact: true }).fill("13:00");
  await rescheduleDialog.getByLabel("Fin", { exact: true }).fill("16:00");
  const movedResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(
        `/api/v1/projects/${secondProject}/schedule-items/${secondItem}/reschedule`
      ) && response.request().method() === "POST"
  );
  await rescheduleDialog.getByRole("button", { name: "Enregistrer le déplacement" }).click();
  expect((await movedResponse).ok()).toBeTruthy();
  await expect(page.getByRole("dialog").getByText(/13:00–16:00/)).toBeVisible();

  const persistedSchedule = await request.get(
    `/api/v1/projects/${secondProject}/schedule-items`
  );
  expect(persistedSchedule.ok(), await persistedSchedule.text()).toBeTruthy();
  const scheduleBody = await persistedSchedule.json();
  expect(scheduleBody.schedule_items[0]).toMatchObject({
    item_code: secondItem,
    planned_start_at: `${movedDate}T13:00`,
    planned_end_at: `${movedDate}T16:00`,
    assigned_operator: firstOperator,
    revision: 2,
    status: "planned"
  });

  const auditResponse = await request.get(`/api/v1/projects/${secondProject}/audit-events`);
  expect(auditResponse.ok(), await auditResponse.text()).toBeTruthy();
  const auditBody = await auditResponse.json();
  expect(auditBody.audit_events.map((event: { action: string }) => event.action)).toContain(
    "service_schedule_item_rescheduled"
  );
  expect(JSON.stringify(auditBody)).toContain(`${secondDate}T09:00`);
  expect(JSON.stringify(auditBody)).toContain(`${movedDate}T13:00`);

  const outboxResponse = await request.get("/api/v1/sync/outbox");
  expect(outboxResponse.ok(), await outboxResponse.text()).toBeTruthy();
  expect(JSON.stringify(await outboxResponse.json())).toContain(
    "service_schedule_item_rescheduled"
  );

  await page.getByRole("button", { name: "Fermer" }).click();
  for (const viewport of viewports) {
    await page.setViewportSize(viewport);
    await page.goto("/lab/");
    await page.getByRole("button", { name: "Planning du laboratoire" }).click();
    await expect(page.getByText(`${secondProject} · Mobilités Boréal`)).toBeVisible();
    await assertNoHorizontalOverflow(page);
    await captureReleaseScreenshot(page, `semaine-laboratoire-${viewport.width}x${viewport.height}.png`);

    await page
      .getByRole("button", { name: `Ouvrir Immunité rayonnée, dossier ${secondProject}` })
      .click();
    await expect(page.getByRole("dialog").getByText(/13:00–16:00/)).toBeVisible();
    await captureReleaseScreenshot(page, `detail-creneau-${viewport.width}x${viewport.height}.png`);
    await page.getByRole("button", { name: "Fermer" }).click();
  }

  await page
    .getByRole("button", { name: `Ouvrir Immunité rayonnée, dossier ${secondProject}` })
    .click();
  await page.getByRole("button", { name: "Ouvrir le dossier" }).click();
  await expect(page.getByRole("heading", { name: secondProject })).toBeVisible();
});

async function preparePlanningProject(
  request: APIRequestContext,
  projectCode: string,
  customerName: string,
  operationSuffix: string
) {
  const created = await request.post("/api/v1/projects", {
    data: {
      code: projectCode,
      customer_name: customerName,
      execution_mode: "investigation",
      actor: "Responsable laboratoire",
      reason: "Préparation du planning hebdomadaire",
      operation_id: `op-week-project-${operationSuffix}`
    }
  });
  expect(created.ok(), await created.text()).toBeTruthy();

  const reviewResponse = await request.get(`/api/v1/projects/${projectCode}/contract-review`);
  expect(reviewResponse.ok(), await reviewResponse.text()).toBeTruthy();
  const review = (await reviewResponse.json()).contract_review as { required_items: string[] };
  for (const [index, item] of review.required_items.entries()) {
    const completed = await request.post(
      `/api/v1/projects/${projectCode}/contract-review/items/${item}/complete`,
      {
        data: {
          actor: "Responsable laboratoire",
          comment: "Vérifié pour la planification",
          operation_id: `op-week-review-${operationSuffix}-${index}`
        }
      }
    );
    expect(completed.ok(), await completed.text()).toBeTruthy();
  }

  const advanced = await request.post(
    `/api/v1/projects/${projectCode}/transitions/to-test-planning`,
    {
      data: {
        actor: "Responsable laboratoire",
        reason: "Revue terminée",
        operation_id: `op-week-ready-${operationSuffix}`
      }
    }
  );
  expect(advanced.ok(), await advanced.text()).toBeTruthy();
}

async function createScheduleItem(
  request: APIRequestContext,
  input: {
    projectCode: string;
    itemCode: string;
    title: string;
    date: string;
    operator: string;
    location: string;
    equipment: string;
  }
) {
  const response = await request.post(`/api/v1/projects/${input.projectCode}/schedule-items`, {
    data: {
      item_code: input.itemCode,
      title: input.title,
      planned_start_at: `${input.date}T09:00`,
      planned_end_at: `${input.date}T12:00`,
      assigned_operator: input.operator,
      laboratory_location_id: `LAB-LOCATION-${input.itemCode}`,
      laboratory_location_label: input.location,
      equipment_under_test: input.equipment,
      actor: "Responsable laboratoire",
      reason: "Créneau convenu",
      operation_id: `op-${input.itemCode}`
    }
  });
  expect(response.ok(), await response.text()).toBeTruthy();
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
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(80);
  const body = await page.screenshot({ animations: "disabled", fullPage: false });
  const evidenceDirectory = path.resolve(process.cwd(), "../../docs/ux/0.20.0/screenshots");
  await mkdir(evidenceDirectory, { recursive: true });
  await writeFile(path.join(evidenceDirectory, name), body);
}
