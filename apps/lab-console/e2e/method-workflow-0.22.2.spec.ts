import { execFileSync, spawn } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { expect, test, type APIRequestContext, type Page } from "@playwright/test";

const repoRoot = path.resolve(process.cwd(), "../..");
const screenshotRoot = path.join(repoRoot, "docs", "ux", "0.22.2", "screenshots");
const refreshScreenshots =
  process.env.EMC_LOCUS_REFRESH_RELEASE_SCREENSHOTS === "1"
  && process.env.EMC_LOCUS_RELEASE_SCREENSHOT_VERSION === "0.22.2";
const context = {
  actor: "method.workflow.e2e",
  reason: "Valider le workflow méthodes 0.22.2",
  device_id: "playwright-0222",
  correlation_id: "corr-method-workflow-0222"
};
const fixtureSuffix = "0222";
const preparationMethodId = `METHOD-DEMO-RF-PREP-${fixtureSuffix}`;
const preparationSetupId = `SETUP-DEMO-RF-PREP-${fixtureSuffix}`;
const preparationProjectCode = `CEM-DEMO-PREP-${fixtureSuffix}`;
const preparationScheduleCode = `PLAN-DEMO-PREP-${fixtureSuffix}`;
const preparationPowerMeterId = `PM-DEMO-RF-${fixtureSuffix}`;

let methodRevision: Record<string, unknown>;
let methodDefinition: Record<string, unknown>;
let systemRevision: Record<string, unknown>;
let systemDefinition: Record<string, unknown>;
let regulationRevision: Record<string, unknown>;
let regulationDefinition: Record<string, unknown>;
let legacySuccessor: Record<string, unknown>;
let preparationRevision: Record<string, unknown>;

test.describe.serial("0.22.2 method workflow and control UX", () => {
  test("onboards an empty database without hiding prerequisites", async ({ page, request }) => {
    const methods = await request.get("/api/v1/test-templates");
    expect(methods.ok(), await methods.text()).toBeTruthy();
    const existingMethods = (await methods.json()).test_templates as unknown[];
    if (existingMethods.length > 0) {
      await page.route("**/api/v1/test-templates", (route) => route.fulfill({ status: 200, contentType: "application/json; charset=utf-8", body: JSON.stringify({ test_templates: [] }) }));
      await page.route("**/api/v1/method-hierarchy", (route) => route.fulfill({ status: 200, contentType: "application/json; charset=utf-8", body: JSON.stringify({ nodes: [] }) }));
      await page.route("**/api/v1/measurement-system-templates", (route) => route.fulfill({ status: 200, contentType: "application/json; charset=utf-8", body: JSON.stringify({ definitions: [] }) }));
      await page.route("**/api/v1/regulation-profiles", (route) => route.fulfill({ status: 200, contentType: "application/json; charset=utf-8", body: JSON.stringify({ definitions: [] }) }));
    }

    await openMethodWorkflow(page);
    await expect(page.getByText("Aucune méthode")).toBeVisible();
    await expect(page.getByText("Aucun classement")).toBeVisible();
    await expect(page.getByRole("button", { name: "Créer le premier domaine" })).toBeVisible();
    await capture(page, "empty-state-onboarding-1440x900.png", 1440, 900);
  });

  test("creates hierarchy and preserves an explicit legacy successor", async ({ page, request }) => {
    seedPreparationDemo();

    await createHierarchyNode(request, "emc", undefined, "domain", "CEM", 0);
    await createHierarchyNode(request, "immunity", "emc", "test_family", "Immunité", 0);
    await createHierarchyNode(request, "conducted", "immunity", "procedure", "Conduite", 0);
    const moved = await putOk(request, "/api/v1/method-hierarchy/conducted", {
      node: { node_id: "conducted", parent_node_id: "immunity", node_kind: "procedure", label: "Immunité conduite", position: 10, archived: false },
      expected_revision: 1,
      ...context,
      operation_id: "op-0222-hierarchy-move"
    });
    expect(moved.operation).toBe("hierarchy_node_changed");

    const before = await getOk(request, `/api/v1/test-templates/${preparationMethodId}`);
    const source = before.test_template.current_approved_revision;
    const successor = await postOk(
      request,
      `/api/v1/test-templates/${preparationMethodId}/revisions/successor-0.22.2`,
      { source_revision_id: source.revision_id, ...context, operation_id: "op-0222-legacy-successor" }
    );
    legacySuccessor = successor.revision;
    expect(legacySuccessor.definition_schema_version).toBe("emc-locus.test-method-definition.v2");
    const history = await getOk(request, `/api/v1/test-templates/${preparationMethodId}/revisions`);
    expect(history.revisions.map((revision: { definition_schema_version: string }) => revision.definition_schema_version)).toEqual(
      expect.arrayContaining(["emc-locus.test-template-definition.v1", "emc-locus.test-method-definition.v2"])
    );

    await openMethodWorkflow(page);
    await expect(page.getByText("CEM", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: /CEM Domaine/ }).click();
    await capture(page, "hierarchy-1440x900.png", 1440, 900);
  });

  test("rejects missing feedback then stores reusable regulation and topology", async ({ request }) => {
    methodDefinition = conductedImmunityMethod();
    const created = await postOk(request, "/api/v1/test-templates", {
      template_id: "METHOD-CI-0222",
      title: methodDefinition.title,
      category_code: "immunity_conducted",
      definition: methodDefinition,
      ...context,
      operation_id: "op-0222-method-create"
    });
    methodRevision = created.revision;

    regulationDefinition = closedLoopRegulation();
    const invalidProfile = { ...regulationDefinition, profile_id: "REG-INVALID-0222", feedback_role_id: undefined, feedback_signal_variable_id: undefined };
    const invalid = await request.post("/api/v1/regulation-profiles", {
      data: {
        entity_id: "REG-INVALID-0222",
        label: "Boucle sans retour",
        classification: "laboratory",
        definition: invalidProfile,
        method_definition: methodDefinition,
        ...context,
        operation_id: "op-0222-regulation-invalid"
      }
    });
    expect(invalid.ok()).toBeFalsy();
    expect(JSON.stringify(await invalid.json())).toContain("missing_regulation_feedback");

    const profile = await postOk(request, "/api/v1/regulation-profiles", {
      entity_id: "REG-CLOSED-0222",
      label: regulationDefinition.label,
      classification: "immunity",
      definition: regulationDefinition,
      method_definition: methodDefinition,
      ...context,
      operation_id: "op-0222-regulation-create"
    });
    regulationRevision = profile.revision;
    await transitionWorkflow(request, "regulation-profiles", "REG-CLOSED-0222", regulationRevision, "validate", { method_definition: methodDefinition });
    await transitionWorkflow(request, "regulation-profiles", "REG-CLOSED-0222", regulationRevision, "approve", { method_definition: methodDefinition });

    systemDefinition = conductedImmunitySystem();
    const invalidSystem = {
      ...systemDefinition,
      template_id: "SYS-INVALID-0222",
      edges: (systemDefinition.edges as Array<Record<string, unknown>>).filter((edge) => edge.edge_kind !== "feedback_measurement")
    };
    const invalidTopology = await request.post("/api/v1/measurement-system-templates", {
      data: {
        entity_id: "SYS-INVALID-0222",
        label: "Boucle sans chemin de retour",
        classification: "immunity_conducted",
        definition: invalidSystem,
        method_definition: methodDefinition,
        regulation_profiles: [regulationDefinition],
        ...context,
        operation_id: "op-0222-system-invalid"
      }
    });
    expect(invalidTopology.ok()).toBeFalsy();
    expect(JSON.stringify(await invalidTopology.json())).toContain("unreachable_regulation_feedback");

    const system = await createSystem(request, systemDefinition, methodDefinition, [regulationDefinition], "op-0222-system-conducted");
    systemRevision = system.revision;
    await transitionWorkflow(request, "measurement-system-templates", "SYS-CI-0222", systemRevision, "validate", { method_definition: methodDefinition, regulation_profiles: [regulationDefinition] });
    await transitionWorkflow(request, "measurement-system-templates", "SYS-CI-0222", systemRevision, "approve", { method_definition: methodDefinition, regulation_profiles: [regulationDefinition] });
    await createSystem(request, radiatedImmunitySystem(), methodDefinition, [regulationDefinition], "op-0222-system-radiated");
    await createSystem(request, conductedEmissionSystem(), undefined, [], "op-0222-system-emission");

    methodDefinition = {
      ...methodDefinition,
      measurement_system_templates: [revisionReference("SYS-CI-0222", systemRevision)],
      regulation_profiles: [revisionReference("REG-CLOSED-0222", regulationRevision)]
    };
    const saved = await putOk(request, "/api/v1/test-templates/METHOD-CI-0222/revisions/METHOD-CI-0222-rev-0001/definition", {
      expected_definition_checksum: methodRevision.definition_checksum,
      definition: methodDefinition,
      ...context,
      operation_id: "op-0222-method-reference-save"
    });
    methodRevision = saved.revision;
    methodRevision = (await transitionMethod(request, methodRevision, "submit-for-review", "op-0222-method-submit")).revision;
    methodRevision = (await transitionMethod(request, methodRevision, "approve", "op-0222-method-approve")).revision;
  });

  test("previews two bounded ranges and compiles an explainable plan", async ({ request }) => {
    const ranges = methodDefinition.sub_ranges as Array<Record<string, unknown>>;
    const previews = await Promise.all(ranges.map((subRange, index) => postOk(request, "/api/v1/sub-ranges/preview", {
      sub_range: subRange,
      maximum_points: 500,
      operation_id: `ignored-preview-${index}`
    })));
    expect(previews[0].preview.point_count).toBeGreaterThan(1);
    expect(previews[1].preview.point_count).toBeGreaterThan(1);

    const compiled = await postOk(request, "/api/v1/execution-plans/preview", {
      method_template_id: "METHOD-CI-0222",
      method_revision_id: methodRevision.revision_id,
      method_definition: methodDefinition,
      system_definition: systemDefinition,
      regulation_profiles: [regulationDefinition],
      ...context,
      operation_id: "op-0222-plan-preview"
    });
    expect(compiled.preview.blockers).toHaveLength(0);
    expect(compiled.preview.regulation_loops).toContain("Boucle de niveau injecté");
    expect(compiled.preview.unsupported_runtime_operations).toContain("FFT optionnelle");
    expect(compiled.preview.ordered_phases.some((phase: { maximum_iterations?: number }) => phase.maximum_iterations === 2)).toBe(true);
  });

  test("derives a dated configuration only from the authoritative preparation", async ({ request }) => {
    legacySuccessor = (await transitionMethod(request, legacySuccessor, "submit-for-review", "op-0222-legacy-submit")).revision;
    legacySuccessor = (await transitionMethod(request, legacySuccessor, "approve", "op-0222-legacy-approve")).revision;
    const legacyMethod = legacySuccessor.definition;
    const datedSystemDefinition = {
      definition_schema_version: "emc-locus.measurement-system-template-definition.v1",
      template_id: "SYS-DATED-0222",
      label: "Fonction de mesure datée",
      classification: "emission_conducted",
      nodes: [{
        node_id: "receiver",
        label: "Wattmètre RF",
        role_type: "measurement_receiver",
        method_role_id: "measurement_receiver",
        ports: [],
        notes: "Aucun exemplaire physique dans le modèle réutilisable."
      }],
      edges: [], correction_points: [], regulation_loops: [], notes: "Topologie logique."
    };
    const datedSystem = (await createSystem(request, datedSystemDefinition, legacyMethod, [], "op-0222-system-dated")).revision;
    await transitionWorkflow(request, "measurement-system-templates", "SYS-DATED-0222", datedSystem, "validate", { method_definition: legacyMethod });

    const schedule = await getOk(request, `/api/v1/projects/${preparationProjectCode}/schedule-items`);
    const item = schedule.schedule_items.find((candidate: { item_code: string }) => candidate.item_code === preparationScheduleCode);
    const setupResult = await getOk(request, `/api/v1/station-setups/${preparationSetupId}`);
    const setupRevision = setupResult.station_setup.current_ready_revision;
    const exactRequirement = setupRevision.definition.material_requirements.find((requirement: { requirement_id: string }) => requirement.requirement_id === "power_meter");
    expect(exactRequirement.selection_policy).toBe("exact_asset");
    expect(exactRequirement.exact_asset_id).toBe(preparationPowerMeterId);

    const assessment = await postOk(request, `/api/v1/projects/${preparationProjectCode}/schedule-items/${preparationScheduleCode}/preparation/assessments`, {
      expected_schedule_revision: item.revision,
      expected_current_revision_id: null,
      method_template_id: preparationMethodId,
      method_revision_id: legacySuccessor.revision_id,
      station_setup_id: preparationSetupId,
      station_setup_revision_id: setupRevision.revision_id,
      assignments: [{ slot_id: "measurement_receiver", binding_id: "power_meter" }],
      station_material_assignments: setupRevision.definition.material_assignments.map((assignment: { requirement_id: string; asset_id: string; selected_ports?: unknown[] }) => ({
        requirement_id: assignment.requirement_id,
        asset_id: assignment.asset_id,
        selected_ports: assignment.selected_ports ?? []
      })),
      ...context,
      operation_id: "op-0222-preparation-assess"
    });
    expect(assessment.preparation.current_state).toBe("ready");
    preparationRevision = assessment.preparation.current_revision;
    const station = preparationRevision.definition.station_setup;
    const preparedAssignment = preparationRevision.definition.station_material_assignments.find((assignment: { requirement_id: string }) => assignment.requirement_id === "power_meter");
    const physical = station.assets.find((asset: { asset_id: string }) => asset.asset_id === preparedAssignment.asset_id);

    const derived = await postOk(request, `/api/v1/projects/${preparationProjectCode}/schedule-items/${preparationScheduleCode}/execution-configuration`, {
      planned_preparation_revision_id: preparationRevision.revision_id,
      expected_current_revision_id: null,
      definition: {
        definition_schema_version: "emc-locus.execution-configuration.v1",
        configuration_id: "EXEC-CEM-0222",
        method_revision: revisionReference(preparationMethodId, legacySuccessor),
        measurement_system_template_revision: revisionReference("SYS-DATED-0222", datedSystem),
        parameter_profile_id: "default",
        parameter_values: { frequency_hz: 1_000_000 },
        selected_sub_range_ids: [],
        laboratory_location_id: station.laboratory_location_id,
        planned_use_on: "2026-07-16",
        eut_context: "Convertisseur Horizon HCU-4",
        station_setup_revision: {
          identity_id: preparationSetupId,
          revision_id: setupRevision.revision_id,
          definition_checksum: setupRevision.definition_checksum
        },
        assignments: [{
          role_id: "measurement_receiver",
          requirement_id: "power_meter",
          asset_id: physical.asset_id,
          asset_revision: physical.asset_revision,
          equipment_model_revision_id: physical.equipment_model_revision_id,
          equipment_model_checksum: physical.equipment_model_checksum
        }]
      },
      regulation_profiles: [],
      ...context,
      operation_id: "op-0222-execution-derive"
    });
    expect(derived.readiness.ready).toBe(true);

    const analyzer = await postOk(request, "/api/v1/equipment-models/from-preset", {
      preset_id: "spectrum_analyzer",
      equipment_model_id: "EQM-ALIAS-SPECTRUM-0222",
      manufacturer: "Locus Demo",
      model_name: "Analyseur à catégorie historique",
      is_demo: true,
      ...context,
      operation_id: "op-0222-alias-model-create"
    });
    await postOk(request, `/api/v1/equipment-models/EQM-ALIAS-SPECTRUM-0222/revisions/${analyzer.revision.revision_id}/transitions/submit-for-review`, { ...context, operation_id: "op-0222-alias-model-submit" });
    await postOk(request, `/api/v1/equipment-models/EQM-ALIAS-SPECTRUM-0222/revisions/${analyzer.revision.revision_id}/transitions/approve`, { ...context, operation_id: "op-0222-alias-model-approve" });
    const aliases = await getOk(request, "/api/v1/equipment-models?category_code=spectrum_analyzer&demo_mode=all");
    expect(aliases.equipment_models.some((model: { identity: { equipment_model_id: string } }) => model.identity.equipment_model_id === "EQM-ALIAS-SPECTRUM-0222")).toBe(true);
    const outbox = await getOk(request, "/api/v1/sync/outbox");
    expect(JSON.stringify(outbox)).toContain("execution_configuration_derived");

    await restartAgent(request);
    const persisted = await getOk(request, "/api/v1/execution-configurations/EXEC-CEM-0222");
    expect(persisted.definition.configuration_id).toBe("EXEC-CEM-0222");
    expect(persisted.definition.method_revision.revision_id).toBe(legacySuccessor.revision_id);
  });

  test("reviews guided method, topology, regulation and execution evidence", async ({ page }) => {
    await openMethodWorkflow(page);
    await page.getByRole("button", { name: /METHOD-CI-0222/ }).click();
    await capture(page, "guided-method-overview-1440x900.png", 1440, 900);

    await page.getByRole("button", { name: /Paramètres et variables/ }).click();
    await expect(page.getByRole("table", { name: "Variables de la méthode" })).toContainText("Fréquence d'essai");
    await capture(page, "parameters-and-variables-1440x900.png", 1440, 900);
    await page.getByRole("button", { name: /Fonctions instrumentales/ }).click();
    await expect(page.locator(".roleTile strong", { hasText: "Générateur de perturbation" })).toBeVisible();
    await capture(page, "functional-roles-1440x900.png", 1440, 900);
    await capture(page, "category-consolidation-1280x720.png", 1280, 720);

    await page.getByRole("button", { name: /Séquence et sous-plages/ }).click();
    const procedureTree = page.locator(".procedureTree").first();
    await expect(procedureTree).toContainText("États de l'objet");
    await expect(procedureTree).toContainText("Maximum 2");
    await capture(page, "sequence-hierarchy-1440x900.png", 1440, 900);
    await capture(page, "sub-range-editor-1280x720.png", 1280, 720);
    await page.getByRole("button", { name: /Traitements et résultats/ }).click();
    await expect(page.getByText(/élément\(s\) structuré\(s\)/)).toBeVisible();
    await capture(page, "post-processing-pipeline-1440x900.png", 1440, 900);

    await page.getByRole("button", { name: /Relecture et publication/ }).click();
    await page.getByRole("button", { name: "Compiler l'aperçu" }).click();
    await expect(page.getByText("Aperçu compilé")).toBeVisible();
    await capture(page, "execution-plan-preview-1440x900.png", 1440, 900);
  });

  test("reviews visual layers and remains usable at 1280 by 720 and by keyboard", async ({ page }) => {
    await openMethodWorkflow(page);
    await page.getByRole("button", { name: "Systèmes de mesure" }).click();
    await page.getByRole("button", { name: /Chaîne d'immunité conduite/ }).click();
    await expect(page.getByRole("table", { name: "Connexions logiques" })).toContainText("Retour de régulation");
    await capture(page, "conducted-immunity-topology-1440x900.png", 1440, 900);
    await page.getByLabel("Couches de la topologie").getByRole("button", { name: "Régulation" }).click();
    await expect(page.getByRole("table", { name: "Connexions logiques" })).toContainText("Retour de régulation");
    await capture(page, "regulation-loop-layer-1440x900.png", 1440, 900);
    await page.getByRole("button", { name: /Générateur/ }).click();
    await expect(page.getByRole("region", { name: /Fonction Générateur/ })).toBeVisible();
    await capture(page, "method-template-mapping-1440x900.png", 1440, 900);

    await page.getByLabel("Espace de conception").getByRole("button", { name: "Régulation" }).click();
    await page.getByRole("button", { name: /Régulation progressive/ }).click();
    await expect(page.getByRole("img", { name: "Approche progressive de la consigne" })).toBeVisible();
    await capture(page, "regulation-curve-editor-1440x900.png", 1440, 900);

    await page.setViewportSize({ width: 1280, height: 720 });
    await page.getByRole("button", { name: "Systèmes de mesure" }).focus();
    await page.keyboard.press("Enter");
    await page.getByRole("button", { name: /Chaîne d'immunité conduite/ }).focus();
    await page.keyboard.press("Enter");
    const firstNode = page.locator(".topologyNodes button").first();
    await firstNode.focus();
    await page.keyboard.press("ArrowRight");
    await expect(page.locator(".topologyNodes button").nth(1)).toBeFocused();
    await expect(page.locator("body")).not.toHaveCSS("overflow-x", "scroll");
    await capture(page, "workflow-1280x720.png", 1280, 720);
  });

  test("isolates a secondary hierarchy failure from readable methods", async ({ page }) => {
    await page.route("**/api/v1/method-hierarchy", (route) => route.fulfill({ status: 503, contentType: "application/json", body: JSON.stringify({ error: { code: "hierarchy_unavailable", message: "Classement temporairement indisponible" } }) }));
    await openMethodWorkflow(page);
    await expect(page.getByText("Classement indisponible")).toBeVisible();
    await expect(page.getByRole("button", { name: /METHOD-CI-0222/ })).toBeVisible();
  });
});

async function openMethodWorkflow(page: Page) {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/lab/");
  await page.getByRole("button", { name: "Méthodes d'essai" }).click();
  await expect(page.getByRole("heading", { name: "Conception des essais" })).toBeVisible();
}

async function capture(page: Page, filename: string, width: number, height: number) {
  if (!refreshScreenshots) return;
  await page.setViewportSize({ width, height });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(80);
  await mkdir(screenshotRoot, { recursive: true });
  await page.screenshot({ path: path.join(screenshotRoot, filename), animations: "disabled", fullPage: false });
}

function seedPreparationDemo() {
  const agentUrl = process.env.LAB_CONSOLE_E2E_BASE_URL ?? "http://127.0.0.1:8765";
  for (const script of ["seed-equipment-demo.ps1", "seed-planned-test-preparation-demo.ps1"]) {
    const args = ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", path.join(repoRoot, "scripts", script), "-AgentUrl", agentUrl];
    if (script === "seed-planned-test-preparation-demo.ps1") args.push("-FixtureSuffix", fixtureSuffix);
    execFileSync("powershell.exe", args, { cwd: repoRoot, encoding: "utf8", stdio: "pipe" });
  }
}

async function createHierarchyNode(request: APIRequestContext, nodeId: string, parent: string | undefined, kind: string, label: string, position: number) {
  return postOk(request, "/api/v1/method-hierarchy", { node: { node_id: nodeId, parent_node_id: parent, node_kind: kind, label, position, archived: false }, ...context, operation_id: `op-0222-hierarchy-${nodeId}` });
}

async function createSystem(request: APIRequestContext, definition: Record<string, unknown>, method: unknown, profiles: unknown[], operationId: string) {
  return postOk(request, "/api/v1/measurement-system-templates", { entity_id: definition.template_id, label: definition.label, classification: definition.classification, definition, method_definition: method, regulation_profiles: profiles, ...context, operation_id: operationId });
}

async function transitionWorkflow(request: APIRequestContext, collection: string, entityId: string, revision: Record<string, unknown>, transition: string, validation: Record<string, unknown>) {
  return postOk(request, `/api/v1/${collection}/${entityId}/revisions/${revision.revision_id}/transitions/${transition}`, { ...validation, ...context, operation_id: `op-0222-${entityId}-${transition}` });
}

async function transitionMethod(request: APIRequestContext, revision: Record<string, unknown>, transition: string, operationId: string) {
  return postOk(request, `/api/v1/test-templates/${revision.template_id}/revisions/${revision.revision_id}/transitions/${transition}`, { ...context, operation_id: operationId });
}

async function getOk(request: APIRequestContext, url: string) {
  const response = await request.get(url);
  expect(response.ok(), await response.text()).toBeTruthy();
  return response.json();
}

async function postOk(request: APIRequestContext, url: string, data: unknown) {
  const response = await request.post(url, { data });
  expect(response.ok(), await response.text()).toBeTruthy();
  return response.json();
}

async function putOk(request: APIRequestContext, url: string, data: unknown) {
  const response = await request.put(url, { data });
  expect(response.ok(), await response.text()).toBeTruthy();
  return response.json();
}

function revisionReference(identityId: string, revision: Record<string, unknown>) {
  return { identity_id: identityId, revision_id: revision.revision_id, definition_checksum: revision.definition_checksum };
}

function variable(variableId: string, label: string, semantic: string, dimension: string, unit: string, source: string) {
  return { variable_id: variableId, label, description: `Valeur ${label} traçable.`, semantic, value_type: "number", dimension, unit, required: true, source, availability_phase: semantic === "method_parameter" ? "definition" : "execution", consumers: [] };
}

function port(portId: string, label: string, directionality: string, signalDomain = "rf") {
  return { port_id: portId, label, directionality, signal_domain: signalDomain, connector: "N", impedance_ohm: 50, quantity_dimension: "power" };
}

function role(roleId: string, label: string, category: string, ports: unknown[], produces: string[] = []) {
  return { role_id: roleId, label, purpose: `Assurer la fonction ${label}.`, required: true, functional_category: category, capabilities: [{ capability_kind: category, frequency_range: { minimum: 0.15, maximum: 230, unit: "MHz" }, operating_modes: ["cw"], required_driver_action: roleId === "generator" ? "set_level" : undefined }], calibration_policy: "if_used", substitution_policy: "same_capabilities", assignment_stage: "planned_test_preparation", logical_ports: ports, consumes_variables: [], produces_variables: produces };
}

function range(subRangeId: string, label: string, start: number, stop: number) {
  return { sub_range_id: subRangeId, label, start_frequency: { value: start, unit: "MHz" }, stop_frequency: { value: stop, unit: "MHz" }, include_start: true, include_stop: true, progression: { kind: "points_per_decade", points: 10 }, direction: "increasing", sweep_mode: "stepped", dwell_seconds: 1, include_frequencies: [], exclude_frequencies: [], modulation_profile_id: "am_1khz", eut_state: "nominal", comments: "Sous-plage de démonstration, sans revendication normative." };
}

function conductedImmunityMethod() {
  return {
    definition_schema_version: "emc-locus.test-method-definition.v2",
    title: "Immunité conduite progressive 0.22.2",
    objective: "Appliquer une perturbation conduite et surveiller explicitement le niveau injecté.",
    scope: "Démonstration de workflow CEM, sans revendication de couverture normative complète.",
    classification_path: ["emc", "immunity", "conducted"],
    standard_references: ["METHODE-LAB-CI-DEMO"],
    variables: [
      variable("frequency", "Fréquence d'essai", "method_parameter", "frequency", "MHz", "laboratory_method"),
      variable("target_level", "Niveau cible", "setpoint", "ratio", "dB", "regulation_profile"),
      variable("observed_level", "Niveau injecté observé", "observed_signal", "ratio", "dB", "disturbance_monitor"),
      variable("corrected_level", "Niveau corrigé", "final_result", "ratio", "dB", "post_processing")
    ],
    lock_policy: [{ variable_id: "frequency", policy: "editable_until_execution" }],
    parameter_profiles: [{ profile_id: "default", label: "Profil nominal", values: { frequency: 1 } }],
    functional_roles: [
      role("generator", "Générateur de perturbation", "rf_signal_generator", [port("feedback", "Commande de niveau", "input", "ethernet"), port("rf_out", "Sortie RF", "output")]),
      role("amplifier", "Amplificateur RF", "rf_power_amplifier", [port("rf_in", "Entrée RF", "input"), port("rf_out", "Sortie RF", "output")]),
      role("injection", "Réseau d'injection", "coupling_decoupling_network", [port("rf_in", "Entrée RF", "input"), port("eut_out", "Sortie objet testé", "output"), port("monitor_out", "Mesure injectée", "output")]),
      role("monitor", "Mesure de perturbation", "rf_power_meter", [port("rf_in", "Entrée de mesure", "input"), port("feedback_out", "Retour mesuré", "output", "ethernet")], ["observed_level"]),
      role("eut", "Objet soumis à l'essai", "eut_interface", [port("rf_in", "Entrée perturbée", "input")])
    ],
    measurement_system_templates: [], regulation_profiles: [],
    modulation_profiles: [{ kind: "am", profile_id: "am_1khz", label: "AM 1 kHz", modulation_frequency: { value: 1, unit: "kHz" }, depth_percent: 80 }],
    sub_ranges: [range("range_low", "Sous-plage basse", 0.15, 30), range("range_high", "Sous-plage haute", 30, 230)],
    procedure: [{ node_id: "prepare", label: "Préparer et vérifier", purpose: "Vérifier le montage et les sécurités.", node_kind: "preparation", input_variables: [], output_variables: [], timeout_seconds: 600, failure_policy: "safe_shutdown", children: [{ node_id: "eut_states", label: "États de l'objet", purpose: "Parcourir deux états bornés.", node_kind: "loop", input_variables: [], output_variables: [], maximum_iterations: 2, failure_policy: "safe_shutdown", children: [{ node_id: "sweep", label: "Balayage régulé", purpose: "Parcourir les deux sous-plages.", node_kind: "sweep", input_variables: ["frequency", "target_level"], output_variables: ["observed_level"], maximum_iterations: 500, failure_policy: "safe_shutdown", children: [], audit_notes: "" }], audit_notes: "" }], audit_notes: "" }, { node_id: "shutdown", label: "Arrêt sûr", purpose: "Couper la sortie avant le verdict.", node_kind: "safe_shutdown", input_variables: [], output_variables: [], failure_policy: "stop", children: [], audit_notes: "" }],
    limits: [
      { limit_id: "safety_abort", label: "Protection du montage", classification: "safety", evaluated_variable_id: "observed_level", comparison: "less_than_or_equal", threshold: { value: 25, unit: "dB" }, phase: "execution", severity: "blocking", action: "safe_shutdown", contributes_to_verdict: false, explanation: "Arrêt technique indépendant du verdict objet." },
      { limit_id: "eut_verdict", label: "Critère de performance objet", classification: "eut_performance", evaluated_variable_id: "corrected_level", comparison: "less_than_or_equal", threshold: { value: 12, unit: "dB" }, phase: "verdict", severity: "major", action: "mark_nonconforming", contributes_to_verdict: true, explanation: "Verdict final séparé des sécurités." }
    ],
    post_processing: [
      { node_id: "apply_correction", label: "Appliquer les corrections", node_kind: "correction_application", inputs: ["observed_level"], outputs: ["corrected_level"], parameters: {} },
      { node_id: "fft_optional", label: "FFT optionnelle", node_kind: "fft_request", inputs: ["corrected_level"], outputs: ["spectrum_result"], parameters: { window: "hann" } },
      { node_id: "compare_limit", label: "Comparer au critère objet", node_kind: "limit_comparison", inputs: ["corrected_level"], outputs: ["final_verdict"], parameters: {}, limit_reference_id: "eut_verdict" }
    ],
    expected_output_variables: ["corrected_level"], migration_evidence: []
  };
}

function closedLoopRegulation() {
  return { definition_schema_version: "emc-locus.regulation-profile-definition.v1", profile_id: "REG-CLOSED-0222", label: "Régulation progressive fermée", regulated_quantity: "Niveau injecté", regulated_unit: "dB", target_expression: { kind: "variable", variable_id: "target_level" }, tolerance_band: { value: 1, unit: "dB" }, control_mode: "closed_loop", actuator_role_id: "generator", required_driver_action: "set_level", feedback_role_id: "monitor", feedback_signal_variable_id: "observed_level", monitoring_role_ids: ["monitor"], start_threshold: { value: -20, unit: "dB" }, fast_increasing_step: { value: 3, unit: "dB" }, slow_increasing_step: { value: 0.5, unit: "dB" }, decreasing_step: { value: 1, unit: "dB" }, dwell_t1_seconds: 0.1, dwell_t2_seconds: 0.2, dwell_t3_seconds: 1, regulation_start_criterion: "Démarrer sous la consigne", regulation_end_criterion: "Rester dans la tolérance", regulation_factor: 1, maximum_output: { value: 25, unit: "dB" }, maximum_forward_power: { value: 20, unit: "W" }, maximum_reflected_power: { value: 5, unit: "W" }, overshoot_limit: { value: 2, unit: "dB" }, retry_policy: { maximum_attempts: 3, on_exhaustion: "abort" }, abort_policy: "safe_shutdown", safe_state_policy: "generator_output_off", operator_notes: "Profil démonstratif à qualifier selon la méthode appliquée." };
}

function conductedImmunitySystem() {
  const node = (nodeId: string, label: string, roleType: string, ports: unknown[]) => ({ node_id: nodeId, label, role_type: roleType, method_role_id: nodeId, ports, notes: "" });
  return { definition_schema_version: "emc-locus.measurement-system-template-definition.v1", template_id: "SYS-CI-0222", label: "Chaîne d'immunité conduite", classification: "immunity_conducted", nodes: [node("generator", "Générateur", "generator", [port("feedback", "Commande", "input", "ethernet"), port("rf_out", "Sortie RF", "output")]), node("amplifier", "Amplificateur", "amplifier", [port("rf_in", "Entrée RF", "input"), port("rf_out", "Sortie RF", "output")]), node("injection", "Réseau d'injection", "injection_device", [port("rf_in", "Entrée RF", "input"), port("eut_out", "Sortie objet", "output"), port("monitor_out", "Prélèvement", "output")]), node("monitor", "Mesure de perturbation", "disturbance_monitor", [port("rf_in", "Entrée mesure", "input"), port("feedback_out", "Retour", "output", "ethernet")]), node("eut", "Objet soumis à l'essai", "eut_monitor", [port("rf_in", "Entrée perturbée", "input")])], edges: [{ edge_id: "generator_amp", label: "Générateur vers amplificateur", edge_kind: "physical_signal", from: { node_id: "generator", port_id: "rf_out" }, to: { node_id: "amplifier", port_id: "rf_in" } }, { edge_id: "amp_injection", label: "Puissance amplifiée", edge_kind: "excitation_or_power", from: { node_id: "amplifier", port_id: "rf_out" }, to: { node_id: "injection", port_id: "rf_in" } }, { edge_id: "injection_eut", label: "Perturbation appliquée", edge_kind: "physical_signal", from: { node_id: "injection", port_id: "eut_out" }, to: { node_id: "eut", port_id: "rf_in" } }, { edge_id: "monitor_path", label: "Prélèvement de niveau", edge_kind: "monitoring", from: { node_id: "injection", port_id: "monitor_out" }, to: { node_id: "monitor", port_id: "rf_in" } }, { edge_id: "level_feedback", label: "Retour de niveau", edge_kind: "feedback_measurement", from: { node_id: "monitor", port_id: "feedback_out" }, to: { node_id: "generator", port_id: "feedback" } }], correction_points: [{ correction_point_id: "level_correction", label: "Correction du niveau observé", node_id: "monitor", signal_variable_id: "observed_level", correction_kind: "frequency_response" }], regulation_loops: [{ loop_id: "level_loop", label: "Boucle de niveau injecté", regulation_profile_id: "REG-CLOSED-0222", actuator_node_id: "generator", feedback_node_id: "monitor", monitoring_node_ids: ["monitor"] }], notes: "Modèle réutilisable sans lieu, date ni numéro de série." };
}

function radiatedImmunitySystem() {
  return {
    definition_schema_version: "emc-locus.measurement-system-template-definition.v1",
    template_id: "SYS-RI-0222",
    label: "Chaîne d'immunité rayonnée",
    classification: "immunity_radiated",
    nodes: [
      { node_id: "generator", label: "Générateur", role_type: "generator", method_role_id: "generator", ports: [port("feedback", "Commande", "input", "ethernet"), port("rf_out", "Sortie RF", "output")], notes: "" },
      { node_id: "amplifier", label: "Amplificateur", role_type: "amplifier", method_role_id: "amplifier", ports: [port("rf_in", "Entrée RF", "input"), port("rf_out", "Sortie RF", "output")], notes: "" },
      { node_id: "antenna", label: "Antenne d'émission", role_type: "antenna", method_role_id: "injection", ports: [port("rf_in", "Entrée RF", "input"), port("field_out", "Champ émis", "output")], notes: "" },
      { node_id: "field_probe", label: "Sonde de champ", role_type: "sensor", method_role_id: "monitor", ports: [port("field_in", "Champ mesuré", "input"), port("feedback_out", "Retour de champ", "output", "ethernet")], notes: "" },
      { node_id: "eut", label: "Objet soumis à l'essai", role_type: "eut_monitor", method_role_id: "eut", ports: [port("field_in", "Champ appliqué", "input")], notes: "" }
    ],
    edges: [
      { edge_id: "generator_amp", label: "Signal d'excitation", edge_kind: "physical_signal", from: { node_id: "generator", port_id: "rf_out" }, to: { node_id: "amplifier", port_id: "rf_in" } },
      { edge_id: "amp_antenna", label: "Puissance vers antenne", edge_kind: "excitation_or_power", from: { node_id: "amplifier", port_id: "rf_out" }, to: { node_id: "antenna", port_id: "rf_in" } },
      { edge_id: "antenna_eut", label: "Champ appliqué", edge_kind: "physical_signal", from: { node_id: "antenna", port_id: "field_out" }, to: { node_id: "eut", port_id: "field_in" } },
      { edge_id: "antenna_probe", label: "Champ surveillé", edge_kind: "monitoring", from: { node_id: "antenna", port_id: "field_out" }, to: { node_id: "field_probe", port_id: "field_in" } },
      { edge_id: "field_feedback", label: "Retour de champ", edge_kind: "feedback_measurement", from: { node_id: "field_probe", port_id: "feedback_out" }, to: { node_id: "generator", port_id: "feedback" } }
    ],
    correction_points: [{ correction_point_id: "field_probe_correction", label: "Correction de la sonde", node_id: "field_probe", signal_variable_id: "observed_level", correction_kind: "frequency_response" }],
    regulation_loops: [{ loop_id: "field_loop", label: "Boucle de champ", regulation_profile_id: "REG-CLOSED-0222", actuator_node_id: "generator", feedback_node_id: "field_probe", monitoring_node_ids: ["field_probe"] }],
    notes: "Topologie rayonnée avec mesure de champ et retour de régulation explicites."
  };
}

function conductedEmissionSystem() {
  return { definition_schema_version: "emc-locus.measurement-system-template-definition.v1", template_id: "SYS-CE-0222", label: "Chaîne d'émissions conduites", classification: "emission_conducted", nodes: [{ node_id: "eut", label: "Objet soumis à l'essai", role_type: "eut_monitor", ports: [port("out", "Perturbation conduite", "output")], notes: "" }, { node_id: "lisn", label: "Réseau de stabilisation", role_type: "injection_device", ports: [port("in", "Entrée objet", "input"), port("out", "Sortie mesure", "output")], notes: "" }, { node_id: "receiver", label: "Récepteur de mesure", role_type: "measurement_receiver", ports: [port("in", "Entrée RF", "input")], notes: "" }], edges: [{ edge_id: "eut_lisn", label: "Objet vers réseau", edge_kind: "physical_signal", from: { node_id: "eut", port_id: "out" }, to: { node_id: "lisn", port_id: "in" } }, { edge_id: "lisn_receiver", label: "Réseau vers récepteur", edge_kind: "physical_signal", from: { node_id: "lisn", port_id: "out" }, to: { node_id: "receiver", port_id: "in" } }], correction_points: [], regulation_loops: [], notes: "Le contrat FFT temporel reste une demande de traitement non exécutée." };
}

async function restartAgent(request: APIRequestContext) {
  const executable = process.env.LAB_CONSOLE_E2E_AGENT_EXECUTABLE;
  const storageRelative = process.env.LAB_CONSOLE_E2E_STORAGE_RELATIVE;
  const bind = process.env.LAB_CONSOLE_E2E_AGENT_BIND;
  const pidFile = process.env.LAB_CONSOLE_E2E_RESTARTED_AGENT_PID_FILE;
  if (!executable || !storageRelative || !bind || !pidFile) throw new Error("Missing isolated restart metadata");
  const currentPid = readTrackedAgentPid(pidFile) || Number(process.env.LAB_CONSOLE_E2E_AGENT_PID);
  if (!currentPid) throw new Error("Missing isolated agent process identifier");
  stopTrackedAgent(currentPid);
  await new Promise((resolve) => setTimeout(resolve, 350));
  const restarted = spawn(executable, ["serve", "--storage-root", storageRelative, "--migrations-root", "storage/sqlite", "--bind", bind, "--lab-console-dist", "apps/lab-console/dist"], { cwd: repoRoot, detached: true, windowsHide: true, stdio: "ignore" });
  if (!restarted.pid) throw new Error("Restarted Local Agent has no PID");
  writeFileSync(pidFile, String(restarted.pid), "utf8");
  restarted.unref();
  for (let attempt = 0; attempt < 120; attempt += 1) {
    try { if ((await request.get("/api/v1/health", { timeout: 500 })).ok()) return; } catch { /* expected during restart */ }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error("Restarted Local Agent did not become ready");
}

function readTrackedAgentPid(pidFile: string) {
  try {
    return Number(readFileSync(pidFile, "utf8").trim());
  } catch {
    return 0;
  }
}

function stopTrackedAgent(pid: number) {
  try {
    process.kill(pid);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ESRCH") throw error;
  }
}
