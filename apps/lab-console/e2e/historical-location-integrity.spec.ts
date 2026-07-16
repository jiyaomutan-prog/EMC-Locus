import {
  expect,
  request as playwrightRequest,
  test,
  type APIRequestContext,
  type APIResponse,
  type Page
} from "@playwright/test";
import { execFileSync, spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { randomUUID } from "node:crypto";
import { once } from "node:events";
import { mkdir, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import path from "node:path";

const repoRoot = path.resolve(process.cwd(), "../..");
const agentExecutable = path.join(
  repoRoot,
  "target",
  "e2e-agent-build",
  "debug",
  "emc-locus-agent.exe"
);
const evidenceDirectory = path.join(repoRoot, "docs", "ux", "0.21.1", "screenshots");

test("a historical location is identified before the physical resource can be booked", async ({
  page
}) => {
  test.setTimeout(600_000);
  const suffix = randomUUID().replaceAll("-", "").slice(0, 10).toUpperCase();
  const storageRelative = path.join("data", `e2e-legacy-location-${suffix}`);
  const storageRoot = path.join(repoRoot, storageRelative);
  const projectsDatabase = path.join(storageRoot, "projects.sqlite");
  const port = await freePort();
  const baseURL = `http://127.0.0.1:${port}`;
  const legacyProject = `CEM-LEGACY-${suffix}`;
  const candidateProject = `CEM-CANDIDATE-${suffix}`;
  const legacyItem = `PLAN-LEGACY-${suffix}`;
  const candidateItem = `PLAN-CANDIDATE-${suffix}`;
  const legacyTitle = "Immunité rayonnée historique";
  const historicalLabel = "Ancien libellé Poste CEM 1";
  const bookingDate = addDays(mondayFor(new Date()), 4);
  const plannedStart = `${bookingDate}T13:00`;
  const plannedEnd = `${bookingDate}T15:00`;
  let agent: RunningAgent | null = null;
  let api: APIRequestContext | null = null;

  try {
    initializeStorage(storageRelative);
    agent = startAgent(storageRelative, port);
    await waitForAgent(baseURL, agent);
    api = await playwrightRequest.newContext({ baseURL });

    seedLaboratoryLocations(baseURL);
    await createAlternativeLocation(api, bookingDate);
    await preparePlanningProject(api, legacyProject, "Laboratoire historique", `${suffix}-legacy`);
    await preparePlanningProject(api, candidateProject, "Industries candidate", `${suffix}-candidate`);
    insertHistoricalScheduleItem(projectsDatabase, {
      itemCode: legacyItem,
      projectCode: legacyProject,
      title: legacyTitle,
      plannedStart,
      plannedEnd,
      operator: "Alice Martin",
      historicalLabel
    });

    const candidateAuditBefore = await projectAudit(api, candidateProject);
    const outboxBefore = await outboxOperations(api);

    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto(`${baseURL}/lab/`);
    await page.getByRole("button", { name: "Dossiers d'essai" }).click();
    await page.getByLabel("Rechercher un dossier").fill(candidateProject);
    await page.getByRole("button", { name: new RegExp(candidateProject) }).click();
    await expect(page.getByRole("heading", { name: candidateProject })).toBeVisible();
    await page.getByRole("button", { name: "Planifier un essai" }).first().click();
    const bookingDialog = page.getByRole("dialog");
    await bookingDialog.getByLabel("Essai prévu").fill("Essai candidat en conflit");
    await bookingDialog.getByLabel("Date", { exact: true }).fill(bookingDate);
    await bookingDialog.getByLabel("Début", { exact: true }).fill("13:30");
    await bookingDialog.getByLabel("Fin", { exact: true }).fill("14:30");
    await bookingDialog.getByLabel("Opérateur", { exact: true }).fill("Bob Durand");
    await bookingDialog.getByLabel("Lieu").selectOption({ label: "Poste CEM 1" });
    await bookingDialog.getByLabel("Équipement à tester").fill("Prototype candidat");
    const legacyConflictResponse = page.waitForResponse(
      (response) =>
        response.url().endsWith(`/api/v1/projects/${candidateProject}/schedule-items`)
        && response.request().method() === "POST"
    );
    await bookingDialog.getByRole("button", { name: "Réserver le créneau" }).click();
    expect((await legacyConflictResponse).status()).toBe(409);
    await expect(bookingDialog.getByRole("alert")).toContainText(
      "Un créneau existant utilise encore un lieu non identifié"
    );
    await expect(bookingDialog.getByRole("alert")).toContainText(legacyTitle);
    await expect(bookingDialog.getByRole("alert")).toContainText(legacyProject);
    await expect(bookingDialog.getByRole("alert")).toContainText(historicalLabel);
    await captureReleaseScreenshot(page, "conflit-lieu-historique-1280x720.png");

    expect(await projectSchedule(api, candidateProject)).toHaveLength(0);
    expect(await projectAudit(api, candidateProject)).toEqual(candidateAuditBefore);
    expect(await outboxOperations(api)).toEqual(outboxBefore);

    await bookingDialog.getByRole("button", { name: "Fermer", exact: true }).click();
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.getByRole("button", { name: "Planning du laboratoire" }).click();
    await expect(page.getByRole("button", { name: `Ouvrir ${legacyTitle}, dossier ${legacyProject}` })).toBeVisible();
    await page.getByRole("button", { name: `Ouvrir ${legacyTitle}, dossier ${legacyProject}` }).click();
    const planningDialog = page.getByRole("dialog");
    await expect(planningDialog.getByText("Lieu à identifier", { exact: true })).toBeVisible();
    await expect(planningDialog.getByText(`Libellé historique : ${historicalLabel}`)).toBeVisible();
    await expect(planningDialog.getByRole("button", { name: "Identifier le lieu" })).toBeVisible();
    await expect(planningDialog).not.toContainText("LAB-LOCATION-DEMO-CEM-1");
    await captureReleaseScreenshot(page, "creneau-lieu-a-identifier-1440x900.png");

    await planningDialog.getByRole("button", { name: "Identifier le lieu" }).click();
    await planningDialog.getByLabel("Lieu réel").selectOption({ label: "Poste CEM 1" });
    await planningDialog
      .getByLabel("Motif de l’identification")
      .fill("Vérification du dossier papier et du plan d’implantation");
    const identificationResponse = page.waitForResponse(
      (response) =>
        response.url().endsWith(
          `/api/v1/projects/${legacyProject}/schedule-items/${legacyItem}/location-identification`
        ) && response.request().method() === "POST"
    );
    await planningDialog.getByRole("button", { name: "Enregistrer le lieu" }).click();
    expect((await identificationResponse).ok()).toBeTruthy();
    await expect(planningDialog.getByText("Poste CEM 1", { exact: true })).toBeVisible();
    await expect(planningDialog.getByText("Lieu à identifier", { exact: true })).toHaveCount(0);

    const identified = (await projectSchedule(api, legacyProject))[0];
    expect(identified).toMatchObject({
      item_code: legacyItem,
      revision: 2,
      laboratory_location_id: "LAB-LOCATION-DEMO-CEM-1",
      laboratory_location_label: "Poste CEM 1",
      status: "planned"
    });
    const legacyAudit = await projectAudit(api, legacyProject);
    expect(JSON.stringify(legacyAudit)).toContain("service_schedule_item_location_identified");
    expect(JSON.stringify(legacyAudit)).toContain(historicalLabel);
    expect(JSON.stringify(legacyAudit)).toContain("LAB-LOCATION-DEMO-CEM-1");
    expect(JSON.stringify(await outboxOperations(api))).toContain(
      "service_schedule_item_location_identified"
    );

    const sameLocationConflict = await api.post(
      `/api/v1/projects/${candidateProject}/schedule-items`,
      {
        data: scheduleCommand({
          itemCode: candidateItem,
          title: "Nouvelle réservation même poste",
          plannedStart: `${bookingDate}T13:30`,
          plannedEnd: `${bookingDate}T14:30`,
          operator: "Bob Durand",
          locationId: "LAB-LOCATION-DEMO-CEM-1",
          locationLabel: "Libellé actuel différent",
          operationId: `op-e2e-same-location-${suffix}`
        })
      }
    );
    expect(sameLocationConflict.status()).toBe(409);
    expect((await sameLocationConflict.json()).error.code).toBe(
      "service_schedule_location_conflict"
    );
    expect(await projectSchedule(api, candidateProject)).toHaveLength(0);

    const differentLocation = await api.post(
      `/api/v1/projects/${candidateProject}/schedule-items`,
      {
        data: scheduleCommand({
          itemCode: candidateItem,
          title: "Nouvelle réservation autre poste",
          plannedStart: `${bookingDate}T13:30`,
          plannedEnd: `${bookingDate}T14:30`,
          operator: "Bob Durand",
          locationId: "LAB-LOCATION-E2E-ALT",
          locationLabel: "Poste CEM 2",
          operationId: `op-e2e-different-location-${suffix}`
        })
      }
    );
    await expectApiOk(differentLocation);

    await api.dispose();
    api = null;
    await stopAgent(agent);
    agent = startAgent(storageRelative, port);
    await waitForAgent(baseURL, agent);
    api = await playwrightRequest.newContext({ baseURL });

    expect((await projectSchedule(api, legacyProject))[0]).toMatchObject({
      item_code: legacyItem,
      revision: 2,
      laboratory_location_id: "LAB-LOCATION-DEMO-CEM-1",
      laboratory_location_label: "Poste CEM 1"
    });
    expect((await projectSchedule(api, candidateProject))[0]).toMatchObject({
      item_code: candidateItem,
      laboratory_location_id: "LAB-LOCATION-E2E-ALT",
      laboratory_location_label: "Poste CEM 2"
    });
    expect(JSON.stringify(await projectAudit(api, legacyProject))).toContain(
      "service_schedule_item_location_identified"
    );
    expect(JSON.stringify(await outboxOperations(api))).toContain(
      `op-e2e-different-location-${suffix}`
    );
  } finally {
    await api?.dispose();
    if (agent) await stopAgent(agent);
    await removeIsolatedStorage(storageRoot);
  }
});

function initializeStorage(storageRelative: string) {
  execFileSync(
    agentExecutable,
    ["storage", "init", "--storage-root", storageRelative, "--migrations-root", "storage/sqlite"],
    { cwd: repoRoot, stdio: "pipe" }
  );
}

function seedLaboratoryLocations(baseURL: string) {
  for (const script of ["seed-equipment-demo.ps1", "seed-planned-test-preparation-demo.ps1"]) {
    execFileSync(
      "powershell.exe",
      [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        path.join(repoRoot, "scripts", script),
        "-AgentUrl",
        baseURL
      ],
      { cwd: repoRoot, stdio: "pipe" }
    );
  }
}

async function createAlternativeLocation(api: APIRequestContext, plannedUseOn: string) {
  const source = await responseJson<StationSetupResponse>(
    await api.get("/api/v1/station-setups/SETUP-DEMO-RF-PREP")
  );
  const sourceRevision = source.station_setup.current_ready_revision;
  expect(sourceRevision).toBeTruthy();
  const created = await responseJson<StationSetupResponse>(
    await api.post("/api/v1/station-setups", {
      data: {
        setup_id: "SETUP-E2E-ALT",
        label: "Chaîne RF alternative",
        laboratory_location_id: "LAB-LOCATION-E2E-ALT",
        laboratory_location_label: "Poste CEM 2",
        planned_use_on: plannedUseOn,
        execution_mode: "investigation",
        actor: "e2e.technician",
        reason: "Créer le second poste stable du scénario historique",
        operation_id: "op-e2e-alt-setup-create"
      }
    })
  );
  const draft = created.station_setup.active_draft_revision;
  expect(draft).toBeTruthy();
  const definition = structuredClone(sourceRevision!.definition);
  Object.assign(definition, {
    setup_id: "SETUP-E2E-ALT",
    label: "Chaîne RF alternative",
    laboratory_location_id: "LAB-LOCATION-E2E-ALT",
    laboratory_location_label: "Poste CEM 2",
    planned_use_on: plannedUseOn
  });
  const saved = await responseJson<StationSetupResponse>(
    await api.put(`/api/v1/station-setups/SETUP-E2E-ALT/revisions/${draft!.revision_id}/definition`, {
      data: {
        expected_definition_checksum: draft!.definition_checksum,
        definition,
        actor: "e2e.technician",
        reason: "Affecter la chaîne vérifiée au second poste",
        operation_id: "op-e2e-alt-setup-save"
      }
    })
  );
  const savedDraft = saved.station_setup.active_draft_revision;
  const readiness = await responseJson<{ readiness: { ready: boolean } }>(
    await api.get(
      `/api/v1/station-setups/SETUP-E2E-ALT/revisions/${savedDraft!.revision_id}/readiness`
    )
  );
  expect(readiness.readiness.ready).toBe(true);
  await expectApiOk(
    await api.post(
      `/api/v1/station-setups/SETUP-E2E-ALT/revisions/${savedDraft!.revision_id}/transitions/ready`,
      {
        data: {
          expected_definition_checksum: savedDraft!.definition_checksum,
          actor: "e2e.technician",
          reason: "Valider le second poste stable",
          operation_id: "op-e2e-alt-setup-ready"
        }
      }
    )
  );
}

async function preparePlanningProject(
  api: APIRequestContext,
  projectCode: string,
  customerName: string,
  operationSuffix: string
) {
  await expectApiOk(
    await api.post("/api/v1/projects", {
      data: {
        code: projectCode,
        customer_name: customerName,
        execution_mode: "investigation",
        actor: "Responsable laboratoire",
        reason: "Préparer le scénario d'intégrité du planning",
        operation_id: `op-e2e-project-${operationSuffix}`
      }
    })
  );
  const review = await responseJson<{ contract_review: { required_items: string[] } }>(
    await api.get(`/api/v1/projects/${projectCode}/contract-review`)
  );
  for (const [index, item] of review.contract_review.required_items.entries()) {
    await expectApiOk(
      await api.post(`/api/v1/projects/${projectCode}/contract-review/items/${item}/complete`, {
        data: {
          actor: "Responsable laboratoire",
          comment: "Vérifié pour le scénario E2E",
          operation_id: `op-e2e-review-${operationSuffix}-${index}`
        }
      })
    );
  }
  await expectApiOk(
    await api.post(`/api/v1/projects/${projectCode}/transitions/to-test-planning`, {
      data: {
        actor: "Responsable laboratoire",
        reason: "Revue terminée pour la planification",
        operation_id: `op-e2e-ready-${operationSuffix}`
      }
    })
  );
}

function insertHistoricalScheduleItem(
  database: string,
  input: {
    itemCode: string;
    projectCode: string;
    title: string;
    plannedStart: string;
    plannedEnd: string;
    operator: string;
    historicalLabel: string;
  }
) {
  const script = [
    "import sqlite3, sys",
    "db, item, project, title, start, end, operator, label = sys.argv[1:]",
    "connection = sqlite3.connect(db)",
    "connection.execute(\"\"\"INSERT INTO service_schedule_items (item_code, project_code, title, planned_start_at, planned_end_at, assigned_operator, location, laboratory_location_id, laboratory_location_label, equipment_under_test, status, notes, created_at, updated_at, revision, created_by, updated_by) VALUES (?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, 'planned', ?, ?, ?, 1, ?, ?)\"\"\", (item, project, title, start, end, operator, label, label, 'Équipement historique', 'Import antérieur à 0.21.1', '2026-07-16T12:00:00Z', '2026-07-16T12:00:00Z', 'legacy-import', 'legacy-import'))",
    "connection.commit()",
    "row = connection.execute('SELECT laboratory_location_id, laboratory_location_label FROM service_schedule_items WHERE item_code = ?', (item,)).fetchone()",
    "assert row == (None, label), row",
    "connection.close()"
  ].join("\n");
  execFileSync(
    "py",
    [
      "-c",
      script,
      database,
      input.itemCode,
      input.projectCode,
      input.title,
      input.plannedStart,
      input.plannedEnd,
      input.operator,
      input.historicalLabel
    ],
    { cwd: repoRoot, stdio: "pipe" }
  );
}

function scheduleCommand(input: {
  itemCode: string;
  title: string;
  plannedStart: string;
  plannedEnd: string;
  operator: string;
  locationId: string;
  locationLabel: string;
  operationId: string;
}) {
  return {
    item_code: input.itemCode,
    title: input.title,
    planned_start_at: input.plannedStart,
    planned_end_at: input.plannedEnd,
    assigned_operator: input.operator,
    laboratory_location_id: input.locationId,
    laboratory_location_label: input.locationLabel,
    equipment_under_test: "Prototype candidat",
    actor: "Responsable laboratoire",
    reason: "Vérifier la réservation de la ressource physique",
    operation_id: input.operationId
  };
}

async function projectSchedule(api: APIRequestContext, projectCode: string) {
  return (
    await responseJson<{ schedule_items: Array<Record<string, unknown>> }>(
      await api.get(`/api/v1/projects/${projectCode}/schedule-items`)
    )
  ).schedule_items;
}

async function projectAudit(api: APIRequestContext, projectCode: string) {
  return (
    await responseJson<{ audit_events: Array<Record<string, unknown>> }>(
      await api.get(`/api/v1/projects/${projectCode}/audit-events`)
    )
  ).audit_events;
}

async function outboxOperations(api: APIRequestContext) {
  const body = await responseJson<Record<string, unknown>>(await api.get("/api/v1/sync/outbox"));
  return (body.operations ?? body.outbox ?? body) as unknown;
}

async function expectApiOk(response: APIResponse) {
  const body = await response.text();
  expect(response.ok(), body).toBeTruthy();
}

async function responseJson<T>(response: APIResponse): Promise<T> {
  const body = await response.text();
  expect(response.ok(), body).toBeTruthy();
  return JSON.parse(body) as T;
}

interface StationSetupRevision {
  revision_id: string;
  definition_checksum: string;
  definition: Record<string, unknown>;
}

interface StationSetupResponse {
  station_setup: {
    current_ready_revision: StationSetupRevision | null;
    active_draft_revision: StationSetupRevision | null;
  };
}

interface RunningAgent {
  process: ChildProcessWithoutNullStreams;
  logs: () => string;
}

function startAgent(storageRelative: string, port: number): RunningAgent {
  let stdout = "";
  let stderr = "";
  const process = spawn(
    agentExecutable,
    [
      "serve",
      "--storage-root",
      storageRelative,
      "--migrations-root",
      "storage/sqlite",
      "--bind",
      `127.0.0.1:${port}`,
      "--lab-console-dist",
      "apps/lab-console/dist"
    ],
    { cwd: repoRoot, windowsHide: true }
  );
  process.stdout.setEncoding("utf8");
  process.stderr.setEncoding("utf8");
  process.stdout.on("data", (chunk: string) => { stdout += chunk; });
  process.stderr.on("data", (chunk: string) => { stderr += chunk; });
  return { process, logs: () => `${stdout}\n${stderr}`.trim() };
}

async function stopAgent(agent: RunningAgent) {
  if (agent.process.exitCode !== null) return;
  const exited = once(agent.process, "exit");
  agent.process.kill();
  await Promise.race([exited, new Promise((resolve) => setTimeout(resolve, 2_000))]);
  if (agent.process.exitCode === null) {
    if (process.platform === "win32" && agent.process.pid) {
      try {
        execFileSync(
          "taskkill.exe",
          ["/PID", String(agent.process.pid), "/T", "/F"],
          { stdio: "pipe" }
        );
      } catch (caught) {
        if (processExists(agent.process.pid)) throw caught;
      }
    } else {
      agent.process.kill("SIGKILL");
    }
    await Promise.race([exited, new Promise((resolve) => setTimeout(resolve, 5_000))]);
  }
  if (agent.process.exitCode === null && processExists(agent.process.pid)) {
    throw new Error(`Isolated agent did not stop.\n${agent.logs()}`);
  }
}

function processExists(pid: number | undefined): boolean {
  if (!pid) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

async function waitForAgent(baseURL: string, agent: RunningAgent) {
  for (let attempt = 0; attempt < 120; attempt += 1) {
    if (agent.process.exitCode !== null) {
      throw new Error(`Isolated agent exited before readiness.\n${agent.logs()}`);
    }
    try {
      const response = await fetch(`${baseURL}/api/v1/health`);
      if (response.ok) return;
    } catch {
      // The listener is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Isolated agent did not become ready.\n${agent.logs()}`);
}

async function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close();
        reject(new Error("Unable to allocate an isolated TCP port"));
        return;
      }
      const port = address.port;
      server.close((error) => (error ? reject(error) : resolve(port)));
    });
  });
}

async function removeIsolatedStorage(storageRoot: string) {
  const dataRoot = path.join(repoRoot, "data") + path.sep;
  const resolved = path.resolve(storageRoot);
  if (!resolved.startsWith(dataRoot) || !path.basename(resolved).startsWith("e2e-legacy-location-")) {
    throw new Error(`Refusing to remove unexpected E2E path: ${resolved}`);
  }
  await rm(resolved, { recursive: true, force: true });
}

async function captureReleaseScreenshot(page: Page, name: string) {
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(80);
  const body = await page.screenshot({ animations: "disabled" });
  if (process.env.EMC_LOCUS_REFRESH_0211_SCREENSHOTS !== "1") return;
  await mkdir(evidenceDirectory, { recursive: true });
  await writeFile(path.join(evidenceDirectory, name), body);
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
