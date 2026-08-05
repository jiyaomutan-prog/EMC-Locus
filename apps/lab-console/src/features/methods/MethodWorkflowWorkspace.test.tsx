import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import type {
  MeasurementSystemDefinition,
  MethodWorkflowAggregate,
  TestMethodDefinitionV2,
  WorkflowAggregate
} from "../../models/methodWorkflow";
import { jsonResponse } from "../../test/fixtures";
import { MethodWorkflowWorkspace } from "./MethodWorkflowWorkspace";

const fetchMock = vi.fn();
const checksum = `sha256:${"a".repeat(64)}`;

beforeEach(() => vi.stubGlobal("fetch", fetchMock));
afterEach(() => {
  vi.restoreAllMocks();
  fetchMock.mockReset();
});

describe("workflow méthodes 0.22.2", () => {
  test("isole une panne de hiérarchie sans masquer les méthodes", async () => {
    mockWorkflowApi({ hierarchyFails: true });
    render(<MethodWorkflowWorkspace />);

    expect(await screen.findByText("Classement indisponible")).toBeInTheDocument();
    expect(screen.getByText("Mesure conduite guidée")).toBeInTheDocument();
  });

  test("convertit explicitement une méthode historique sans masquer sa source", async () => {
    mockWorkflowApi({ legacy: true });
    const user = userEvent.setup();
    render(<MethodWorkflowWorkspace />);

    await user.click(await screen.findByRole("button", { name: /Méthode historique/ }));
    expect(screen.getByText("Définition historique en lecture seule")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Créer une nouvelle version avec le workflow 0.22.2" }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(
      "/api/v1/test-templates/METHOD-LEGACY/revisions/successor-0.22.2",
      expect.objectContaining({ method: "POST" })
    ));
  });

  test("présente les dix étapes avec du vocabulaire opérateur", async () => {
    mockWorkflowApi({});
    const user = userEvent.setup();
    render(<MethodWorkflowWorkspace />);

    await user.click(await screen.findByRole("button", { name: /Mesure conduite guidée/ }));
    expect(screen.getByRole("button", { name: /Identité et classement/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Relecture et publication/ })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Fonctions instrumentales/ }));
    expect(screen.getAllByText("Récepteur de mesure").length).toBeGreaterThan(0);
    expect(screen.queryByText("measurement_receiver")).not.toBeInTheDocument();
    expect(screen.getByText(/numéros de série seront affectés dans la préparation datée/)).toBeInTheDocument();
  });

  test("expose la topologie graphique sous forme de table accessible", async () => {
    mockWorkflowApi({});
    const user = userEvent.setup();
    render(<MethodWorkflowWorkspace />);

    await user.click(await screen.findByRole("button", { name: /Systèmes de mesure/ }));
    await user.click(screen.getByRole("button", { name: /Chaîne conduite/ }));
    expect(screen.getByRole("img", { name: "Générateur" })).toBeInTheDocument();
    expect(screen.getByRole("table", { name: "Connexions logiques" })).toHaveTextContent("Signal physique");
    expect(screen.getByRole("button", { name: "Ajuster la topologie" })).toBeInTheDocument();
  });

  test("édite les ports et demande une liaison explicite", async () => {
    mockWorkflowApi({});
    const user = userEvent.setup();
    render(<MethodWorkflowWorkspace />);

    await user.click(await screen.findByRole("button", { name: /Systèmes de mesure/ }));
    await user.click(screen.getByRole("button", { name: /Chaîne conduite/ }));
    await user.click(screen.getByRole("button", { name: /Générateur/ }));
    expect(screen.getByRole("region", { name: "Fonction Générateur" })).toHaveTextContent(
      "Aucun matériel réel n'est affecté ici"
    );

    await user.click(screen.getByRole("button", { name: "Connexion" }));
    expect(screen.getByRole("dialog", { name: "Nouvelle connexion" })).toBeInTheDocument();
    expect(screen.getByLabelText("Port source")).toHaveTextContent("Générateur · Sortie RF");
    expect(screen.getByLabelText("Port destination")).toHaveTextContent("Amplificateur · Entrée RF");
    await user.selectOptions(screen.getByLabelText("Nature de la liaison"), "feedback_measurement");
    await user.click(screen.getByRole("button", { name: "Ajouter" }));
    expect(screen.getByRole("table", { name: "Connexions logiques" })).toHaveTextContent(
      "Retour de régulation"
    );
  });
});

function mockWorkflowApi(options: { hierarchyFails?: boolean; legacy?: boolean }) {
  const method = options.legacy ? legacyMethodAggregate() : methodAggregate();
  fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
    const path = String(input);
    if (path === "/api/v1/method-hierarchy") {
      if (options.hierarchyFails) return jsonResponse({ error: { code: "offline", message: "Référentiel de classement indisponible" } }, 503);
      return jsonResponse({ nodes: [{ node_id: "emissions", node_kind: "domain", label: "Émissions", position: 0, archived: false, revision: 1 }] });
    }
    if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [method] });
    if (path === "/api/v1/measurement-system-templates") return jsonResponse({ definitions: [systemAggregate()] });
    if (path === "/api/v1/regulation-profiles") return jsonResponse({ definitions: [] });
    if (path.endsWith("/successor-0.22.2") && init?.method === "POST") {
      const next = methodAggregate();
      return jsonResponse({ operation: "test_method_v2_successor_created", operation_id: "op", replayed: false, test_template: next, revision: next.active_draft_revision });
    }
    return jsonResponse({ error: { code: "unexpected", message: path } }, 500);
  });
}

function methodDefinition(): TestMethodDefinitionV2 {
  return {
    definition_schema_version: "emc-locus.test-method-definition.v2",
    title: "Mesure conduite guidée",
    objective: "Mesurer les perturbations conduites.",
    scope: "Objet alimenté sur réseau alternatif.",
    classification_path: ["emissions"],
    standard_references: [],
    variables: [{
      variable_id: "level",
      label: "Niveau mesuré",
      description: "Niveau corrigé retenu.",
      semantic: "observed_signal",
      value_type: "number",
      dimension: "ratio",
      unit: "dB",
      required: true,
      source: "receiver",
      availability_phase: "execution",
      consumers: []
    }],
    lock_policy: [],
    parameter_profiles: [],
    functional_roles: [{
      role_id: "receiver",
      label: "Récepteur de mesure",
      purpose: "Mesure sélective du signal.",
      required: true,
      functional_category: "measurement_receiver",
      capabilities: [{ capability_kind: "frequency_selective_measurement" }],
      calibration_policy: "required",
      substitution_policy: "same_capabilities",
      assignment_stage: "planned_test_preparation",
      logical_ports: [{ port_id: "rf", label: "Entrée RF", directionality: "input", signal_domain: "rf" }],
      consumes_variables: [],
      produces_variables: ["level"]
    }],
    measurement_system_templates: [{ identity_id: "SYS-CONDUCTED", revision_id: "SYS-CONDUCTED-rev-0001", definition_checksum: checksum }],
    regulation_profiles: [],
    modulation_profiles: [],
    sub_ranges: [],
    procedure: [],
    limits: [],
    post_processing: [],
    expected_output_variables: ["level"],
    migration_evidence: []
  };
}

function methodAggregate(): MethodWorkflowAggregate {
  const revision = {
    revision_id: "METHOD-GUIDED-rev-0001",
    template_id: "METHOD-GUIDED",
    revision_number: 1,
    parent_revision_id: null,
    status: "draft",
    definition_schema_version: "emc-locus.test-method-definition.v2",
    definition: methodDefinition(),
    definition_checksum: checksum,
    created_by: "author",
    created_at: "2026-08-05T10:00:00Z",
    updated_at: "2026-08-05T10:00:00Z",
    submitted_at: null,
    approved_at: null
  };
  return {
    identity: { template_id: "METHOD-GUIDED", title: "Mesure conduite guidée", category_code: "emissions", current_approved_revision_id: null, created_by: "author", created_at: revision.created_at, updated_at: revision.updated_at },
    current_approved_revision: null,
    latest_revision: revision,
    active_draft_revision: revision
  };
}

function legacyMethodAggregate(): MethodWorkflowAggregate {
  const aggregate = methodAggregate();
  const revision = {
    ...aggregate.active_draft_revision!,
    revision_id: "METHOD-LEGACY-rev-0001",
    template_id: "METHOD-LEGACY",
    definition_schema_version: "emc-locus.test-template-definition.v1",
    definition: { definition_schema_version: "emc-locus.test-template-definition.v1", title: "Méthode historique" }
  };
  return { ...aggregate, identity: { ...aggregate.identity, template_id: "METHOD-LEGACY", title: "Méthode historique" }, latest_revision: revision, active_draft_revision: revision };
}

function systemAggregate(): WorkflowAggregate<MeasurementSystemDefinition> {
  const definition: MeasurementSystemDefinition = {
    definition_schema_version: "emc-locus.measurement-system-template-definition.v1",
    template_id: "SYS-CONDUCTED",
    label: "Chaîne conduite",
    classification: "immunity",
    nodes: [
      { node_id: "gen", label: "Générateur", role_type: "generator", ports: [{ port_id: "out", label: "Sortie RF", directionality: "output", signal_domain: "rf" }], notes: "" },
      { node_id: "amp", label: "Amplificateur", role_type: "amplifier", ports: [{ port_id: "in", label: "Entrée RF", directionality: "input", signal_domain: "rf" }], notes: "" }
    ],
    edges: [{ edge_id: "rf-path", label: "Liaison RF", edge_kind: "physical_signal", from: { node_id: "gen", port_id: "out" }, to: { node_id: "amp", port_id: "in" } }],
    correction_points: [],
    regulation_loops: [],
    notes: ""
  };
  const revision = { revision_id: "SYS-CONDUCTED-rev-0001", entity_id: "SYS-CONDUCTED", revision_number: 1, parent_revision_id: null, status: "draft" as const, definition_schema_version: definition.definition_schema_version, definition, definition_checksum: checksum, created_by: "author", created_at: "2026-08-05T10:00:00Z", updated_at: "2026-08-05T10:00:00Z", validated_at: null, approved_at: null };
  return { identity: { aggregate_kind: "measurement_system_template", entity_id: "SYS-CONDUCTED", label: "Chaîne conduite", classification: "immunity", current_approved_revision_id: null, created_by: "author", created_at: revision.created_at, updated_at: revision.updated_at }, revisions: [revision] };
}
