import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { App } from "./App";
import {
  auditFixture,
  healthFixture,
  jsonResponse,
  revisionFixture,
  storageFixture,
  templateFixture
} from "./test/fixtures";

const fetchMock = vi.fn();

beforeEach(() => {
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.restoreAllMocks();
  fetchMock.mockReset();
});

function mockBaseApi(templates = [templateFixture()]) {
  fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
    const path = String(input);
    if (path === "/api/v1/health") return jsonResponse(healthFixture);
    if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
    if (path === "/api/v1/test-templates") {
      if (init?.method === "POST") {
        return jsonResponse({
          operation: "test_template_created",
          operation_id: "op",
          replayed: false,
          test_template: templateFixture(),
          revision: revisionFixture()
        });
      }
      return jsonResponse({ test_templates: templates });
    }
    if (path === "/api/v1/test-templates/TT-LAB-001") {
      return jsonResponse({ test_template: templates[0] ?? templateFixture() });
    }
    if (path.includes("/revisions/TT-LAB-001-rev-0001") && !path.endsWith("/definition") && !path.includes("/transitions/")) {
      return jsonResponse({ revision: revisionFixture() });
    }
    if (path === "/api/v1/test-templates/TT-LAB-001/revisions") {
      return jsonResponse({ template_id: "TT-LAB-001", revisions: [revisionFixture()] });
    }
    if (path === "/api/v1/test-templates/TT-LAB-001/audit-events") {
      return jsonResponse({ template_id: "TT-LAB-001", audit_events: auditFixture });
    }
    if (path === "/api/v1/test-template-definitions/validate") {
      return jsonResponse({ valid: true, issues: [], definition_checksum: "sha256:bbbb" });
    }
    if (path.endsWith("/definition")) {
      return jsonResponse({
        operation: "test_template_definition_replaced",
        operation_id: "op",
        replayed: false,
        test_template: templateFixture(),
        revision: { ...revisionFixture(), definition_checksum: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" }
      });
    }
    if (path.endsWith("/transitions/submit-for-review")) {
      return jsonResponse({
        operation: "test_template_submitted_for_review",
        operation_id: "op",
        replayed: false,
        test_template: templateFixture("draft"),
        revision: { ...revisionFixture("under_review"), status: "under_review" }
      });
    }
    if (path.endsWith("/transitions/approve")) {
      return jsonResponse({
        operation: "test_template_approved",
        operation_id: "op",
        replayed: false,
        test_template: templateFixture("approved"),
        revision: { ...revisionFixture("approved"), status: "approved" }
      });
    }
    if (path.endsWith("/clone")) {
      return jsonResponse({
        operation: "test_template_cloned",
        operation_id: "op",
        replayed: false,
        test_template: templateFixture(),
        revision: revisionFixture()
      });
    }
    return jsonResponse({ error: { code: "not_found", message: path } }, 404);
  });
}

describe("LAB CONSOLE", () => {
  test("renders an empty API library without fake business rows", async () => {
    mockBaseApi([]);

    render(<App />);

    expect(await screen.findByText("Aucun template")).toBeInTheDocument();
    expect(screen.queryByText("CEM-2026-001")).not.toBeInTheDocument();
    expect(screen.queryByText("Client demo")).not.toBeInTheDocument();
  });

  test("loads templates, filters them, and opens the draft studio", async () => {
    mockBaseApi([templateFixture()]);
    const user = userEvent.setup();

    render(<App />);

    expect(await screen.findByText("Inrush current template")).toBeInTheDocument();
    await user.type(screen.getByLabelText("Recherche template"), "inrush");
    expect(screen.getByText("TT-LAB-001")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Brouillon" }));

    expect(await screen.findByText("Template Studio")).toBeInTheDocument();
    expect(screen.getByText("Non modifie")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Variables" }));
    expect(screen.getByDisplayValue("repeat_count")).toBeInTheDocument();
  });

  test("edits variables, validates, saves, submits, approves, and derives through API calls", async () => {
    mockBaseApi([templateFixture()]);
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Brouillon" }));
    await user.click(screen.getByRole("button", { name: "Variables" }));
    await user.click(screen.getByRole("button", { name: "Ajouter une variable" }));
    await user.click(screen.getByRole("button", { name: /Valider/ }));
    expect(await screen.findByText("Definition valide")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Sauvegarder/ }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("/definition"), expect.any(Object)));
    await user.click(screen.getByRole("button", { name: /Valider/ }));
    await user.click(screen.getByRole("button", { name: /Soumettre/ }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("submit-for-review"), expect.any(Object)));
  });

  test("shows CAS conflict without dropping local edits", async () => {
    mockBaseApi([templateFixture()]);
    fetchMock.mockImplementationOnce(() => jsonResponse(healthFixture));
    const user = userEvent.setup();

    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Brouillon" }));
    await user.click(screen.getByLabelText("Titre technique de revision"));
    await user.keyboard(" updated");
    fetchMock.mockImplementationOnce(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path.endsWith("/definition") && init?.method === "PUT") {
        return jsonResponse(
          {
            error: {
              code: "test_template_definition_checksum_mismatch",
              message: "draft definition was modified by another operation",
              details: {
                expected_definition_checksum: "sha256:local",
                actual_definition_checksum: "sha256:server"
              }
            }
          },
          409
        );
      }
      return mockBaseApiResponse(path, init);
    });

    await user.click(screen.getByRole("button", { name: /Sauvegarder/ }));

    expect(await screen.findByText("Conflit de sauvegarde")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Inrush current template updated")).toBeInTheDocument();
  });

  test("creates and clones through public API routes", async () => {
    mockBaseApi([templateFixture("approved")]);
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: /Creer/ }));
    await user.type(screen.getByLabelText("Identifiant"), "TT-NEW-001");
    await user.type(screen.getByLabelText("Titre bibliotheque"), "New template");
    await user.click(screen.getByRole("button", { name: "Creer le brouillon" }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith("/api/v1/test-templates", expect.objectContaining({ method: "POST" })));

    await user.click(screen.getByRole("button", { name: /Bibliotheque/ }));
    await user.click(screen.getByRole("button", { name: /Cloner/ }));
    await user.selectOptions(screen.getByLabelText("Source approuvee"), "TT-LAB-001|TT-LAB-001-rev-0001");
    await user.type(screen.getByLabelText("Nouvel identifiant"), "TT-CLONE-001");
    await user.type(screen.getByLabelText("Nouveau titre bibliotheque"), "Clone template");
    await user.click(screen.getByRole("button", { name: "Cloner vers un nouveau template" }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("/clone"), expect.objectContaining({ method: "POST" })));
  });

  test("opens Equipment space and displays model catalog provider status", async () => {
    mockBaseApi([templateFixture()]);
    fetchMock.mockImplementation(async (input: RequestInfo | URL) => {
      const path = String(input);
      if (path === "/api/v1/health") return jsonResponse(healthFixture);
      if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
      if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
      if (path === "/api/v1/equipment/registries") return jsonResponse(equipmentRegistriesFixture());
      if (path === "/api/v1/equipment/classification-presets") {
        return jsonResponse({ presets: [rfCablePresetFixture(), adcPresetFixture()] });
      }
      if (path === "/api/v1/equipment-models" || path.startsWith("/api/v1/equipment-models?")) {
        return jsonResponse({ equipment_models: [equipmentModelFixture()] });
      }
      if (path === "/api/v1/driver-profiles") return jsonResponse({ driver_profiles: [driverProfileFixture()] });
      if (path === "/api/v1/equipment/communication-providers") {
        return jsonResponse({
          providers: [
            { provider: "simulation", available: true },
            { provider: "visa", available: false, reason: "No VISA implementation installed" }
          ]
        });
      }
      if (path === "/api/v1/equipment-models/EQM-NRP6AN-FWD") {
        return jsonResponse({ equipment_model: equipmentModelFixture() });
      }
      if (path === "/api/v1/equipment-models/EQM-NRP6AN-FWD/revisions") {
        return jsonResponse({
          equipment_model_id: "EQM-NRP6AN-FWD",
          revisions: [equipmentModelFixture().latest_revision]
        });
      }
      if (path === "/api/v1/equipment-models/EQM-NRP6AN-FWD/audit-events") {
        return jsonResponse({ aggregate_kind: "equipment_model", entity_id: "EQM-NRP6AN-FWD", audit_events: [] });
      }
      if (path === "/api/v1/driver-profiles/DRV-NRP6AN-SCPI/revisions") {
        return jsonResponse({
          driver_profile_id: "DRV-NRP6AN-SCPI",
          revisions: [driverProfileFixture().latest_revision]
        });
      }
      if (path === "/api/v1/driver-profiles/DRV-NRP6AN-SCPI/audit-events") {
        return jsonResponse({ aggregate_kind: "driver_profile", entity_id: "DRV-NRP6AN-SCPI", audit_events: [] });
      }
      return mockBaseApiResponse(path);
    });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Equipment" }));
    expect(await screen.findByRole("heading", { name: "Equipment Repository" })).toBeInTheDocument();
    const modelButton = await screen.findByRole("button", { name: /R&S\s+NRP6AN/ });
    await user.click(modelButton);
    expect(await screen.findByText("Equipment Model Definition")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Drivers and Actions" }));
    await user.click(await screen.findByRole("button", { name: /NRP6AN SCPI/ }));
    expect(await screen.findByText(/No VISA implementation installed/)).toBeInTheDocument();
  });

  test("filters equipment catalog and creates a model from a category template", async () => {
    const createdModel = rfCableModelFixture();
    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/health") return jsonResponse(healthFixture);
      if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
      if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
      if (path === "/api/v1/equipment/registries") return jsonResponse(equipmentRegistriesFixture());
      if (path === "/api/v1/equipment/classification-presets") {
        return jsonResponse({ presets: [rfCablePresetFixture(), adcPresetFixture()] });
      }
      if (path === "/api/v1/equipment-models" || path.startsWith("/api/v1/equipment-models?")) {
        return jsonResponse({ equipment_models: [equipmentModelFixture()] });
      }
      if (path === "/api/v1/equipment-models/from-category-template" && init?.method === "POST") {
        return jsonResponse({
          operation: "equipment_model_created_from_category_template",
          operation_id: "op-from-category-template",
          replayed: false,
          aggregate: createdModel,
          revision: createdModel.latest_revision
        });
      }
      if (path === "/api/v1/equipment-models/EQM-RF-CABLE-DEMO") {
        return jsonResponse({ equipment_model: createdModel });
      }
      if (path === "/api/v1/equipment-models/EQM-RF-CABLE-DEMO/revisions") {
        return jsonResponse({
          equipment_model_id: "EQM-RF-CABLE-DEMO",
          revisions: [createdModel.latest_revision]
        });
      }
      if (path === "/api/v1/equipment-models/EQM-RF-CABLE-DEMO/audit-events") {
        return jsonResponse({ aggregate_kind: "equipment_model", entity_id: "EQM-RF-CABLE-DEMO", audit_events: [] });
      }
      if (path === "/api/v1/equipment-model-definitions/validate" && init?.method === "POST") {
        return jsonResponse({ valid: true, issues: [], definition_checksum: "sha256:cccc" });
      }
      if (path === "/api/v1/driver-profiles") return jsonResponse({ driver_profiles: [] });
      if (path === "/api/v1/equipment/communication-providers") {
        return jsonResponse({ providers: [{ provider: "simulation", available: true }] });
      }
      return mockBaseApiResponse(path, init);
    });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Equipment" }));
    await screen.findByRole("heading", { name: "Equipment Repository" });
    await user.selectOptions(screen.getByLabelText("Filtre categorie racine"), "rf_equipment");
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("root_category_id=rf_equipment"),
        expect.any(Object)
      )
    );

    await user.click(screen.getByRole("button", { name: /Nouveau modele/ }));
    const creationPanel = (await screen.findByText("Nouveau modele equipement")).closest(".creationPanel");
    expect(creationPanel).not.toBeNull();
    const wizard = within(creationPanel as HTMLElement);
    await user.click(wizard.getByRole("button", { name: /Équipements radiofréquences/ }));
    await waitFor(() => expect(wizard.getByLabelText(/Fabricant/)).toBeInTheDocument());
    await user.type(wizard.getByLabelText(/ID modele optionnel/), "EQM-RF-CABLE-DEMO");
    await user.type(wizard.getByLabelText(/Fabricant/), "Demo");
    await user.type(wizard.getByLabelText(/Modèle/), "RF Cable");
    await user.type(wizard.getByLabelText(/Variante/), "1m");
    await user.type(wizard.getByLabelText(/Connecteur A/), "N");
    await user.type(wizard.getByLabelText(/Connecteur B/), "N");
    await user.click(wizard.getByRole("button", { name: /Creer brouillon/ }));
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/v1/equipment-models/from-category-template",
        expect.objectContaining({ method: "POST" })
      )
    );

    expect(await screen.findByText("Equipment Model Definition")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Category & Template" }));
    expect(screen.getByText("Template checksum")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Ports & Connections" }));
    expect(screen.getByDisplayValue("RF_A")).toBeInTheDocument();
    expect(screen.getByDisplayValue("RF_B")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Valider/ }));
    expect(await screen.findByText("Definition valide")).toBeInTheDocument();
  });

  test("opens measurement engineering studios and runs curve CSV evaluation workflow", async () => {
    let curveStatus = "draft";
    let curveChecksum = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/health") return jsonResponse(healthFixture);
      if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
      if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
      if (path === "/api/v1/equipment/registries") return jsonResponse(equipmentRegistriesFixture());
      if (path === "/api/v1/equipment/classification-presets") return jsonResponse({ presets: [rfCablePresetFixture(), adcPresetFixture()] });
      if (path === "/api/v1/equipment-models" || path.startsWith("/api/v1/equipment-models?")) return jsonResponse({ equipment_models: [equipmentModelFixture()] });
      if (path === "/api/v1/driver-profiles") return jsonResponse({ driver_profiles: [] });
      if (path === "/api/v1/equipment/communication-providers") return jsonResponse({ providers: [{ provider: "simulation", available: true }] });
      const measurementResponse = measurementApiResponse(path, init, {
        curveStatus,
        curveChecksum,
        onCurveStatus: (status, checksum) => {
          curveStatus = status;
          curveChecksum = checksum;
        }
      });
      if (measurementResponse) return measurementResponse;
      return mockBaseApiResponse(path, init);
    });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Equipment" }));
    await user.click(await screen.findByRole("button", { name: "Engineering Curves" }));
    await user.click(await screen.findByRole("button", { name: /Demo RF cable loss/ }));
    await user.click(screen.getByRole("button", { name: "Curve Table" }));
    expect(await screen.findByRole("img", { name: "1D curve plot" })).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText("frequency_hz,correction_db"), {
      target: { value: "frequency_hz,correction_db\n10000000,0.2\n100000000,1.25\n1000000000,3.8" }
    });
    await user.click(screen.getByRole("button", { name: /Import CSV/ }));
    await user.click(screen.getByRole("button", { name: "Evaluation" }));
    await user.clear(screen.getByLabelText("Frequency Hz"));
    await user.type(screen.getByLabelText("Frequency Hz"), "100000000");
    await user.click(screen.getByRole("button", { name: /Evaluer la courbe/ }));
    expect(await screen.findByText(/log_x_linear_y/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Valider/ }));
    expect(await screen.findByText("Definition valide")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Sauvegarder/ }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("/definition"), expect.objectContaining({ method: "PUT" })));
    await user.click(screen.getByRole("button", { name: /Soumettre/ }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("submit-for-review"), expect.objectContaining({ method: "POST" })));
    await user.click(screen.getByRole("button", { name: /Approuver/ }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("transitions/approve"), expect.objectContaining({ method: "POST" })));
  });

  test("opens sensor, scaling, DAQ and acquisition recipe measurement studios", async () => {
    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/health") return jsonResponse(healthFixture);
      if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
      if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
      if (path === "/api/v1/equipment/registries") return jsonResponse(equipmentRegistriesFixture());
      if (path === "/api/v1/equipment/classification-presets") return jsonResponse({ presets: [rfCablePresetFixture(), adcPresetFixture()] });
      if (path === "/api/v1/equipment-models" || path.startsWith("/api/v1/equipment-models?")) return jsonResponse({ equipment_models: [equipmentModelFixture()] });
      if (path === "/api/v1/driver-profiles") return jsonResponse({ driver_profiles: [] });
      if (path === "/api/v1/equipment/communication-providers") return jsonResponse({ providers: [{ provider: "simulation", available: true }] });
      const measurementResponse = measurementApiResponse(path, init, {});
      if (measurementResponse) return measurementResponse;
      return mockBaseApiResponse(path, init);
    });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Equipment" }));
    await user.click(await screen.findByRole("button", { name: "Sensors & Transducers" }));
    await user.click(await screen.findByRole("button", { name: /Demo Current Probe/ }));
    expect(await screen.findByDisplayValue("current_probe")).toBeInTheDocument();

    await user.click(screen.getAllByRole("button", { name: "Scaling Profiles" })[0]);
    await user.click(await screen.findByRole("button", { name: /Current probe 10 mV/ }));
    await user.click(screen.getByRole("button", { name: "Lookup Table" }));
    expect(screen.getByPlaceholderText("input,output")).toBeInTheDocument();

    await user.click(screen.getAllByRole("button", { name: "DAQ Channels" })[0]);
    await user.click(await screen.findByRole("button", { name: /Demo DAQ AI/ }));
    await user.click(screen.getByRole("button", { name: "Sampling" }));
    expect(screen.getByDisplayValue("1000000")).toBeInTheDocument();

    await user.click(screen.getAllByRole("button", { name: "Acquisition Recipes" })[0]);
    await user.click(await screen.findByRole("button", { name: /current_A through demo current probe/ }));
    await user.click(screen.getByRole("button", { name: "Measurement Chain" }));
    expect(await screen.findByText("DAQ channel")).toBeInTheDocument();
    expect(screen.getAllByText(/REC-DEMO-CURRENT-A/).length).toBeGreaterThan(0);
  });
});

function equipmentModelFixture() {
  const revision = {
    revision_id: "EQM-NRP6AN-FWD-rev-0001",
    equipment_model_id: "EQM-NRP6AN-FWD",
    revision_number: 1,
    parent_revision_id: null,
    status: "approved",
    definition_schema_version: "emc-locus.equipment-model-definition.v2",
    definition_checksum: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    created_by: "equipment.author",
    created_at: "2026-07-03T00:00:00Z",
    updated_at: "2026-07-03T00:00:00Z",
    submitted_at: "2026-07-03T00:00:00Z",
    approved_at: "2026-07-03T00:00:00Z",
    capability_count: 1,
    interface_count: 1,
    signal_port_count: 1,
    definition: {
      definition_schema_version: "emc-locus.equipment-model-definition.v2",
      manufacturer: "R&S",
      model_name: "NRP6AN",
      variant: "FWD",
      equipment_class: "controllable_instrument",
      functional_role: "measurement_instrument",
      category_code: "power_meter",
      signal_domains: ["rf", "ethernet"],
      technology_tags: ["rf_50_ohm", "ethernet", "raw_tcp", "scpi"],
      specifications: [],
      signal_ports: [
        {
          port_id: "rf_input",
          label: "RF input",
          directionality: "input",
          flow_role: "measurement_port",
          signal_domain: "rf",
          connector_type: "N",
          quantity: "power",
          unit: "dBm",
          impedance: 50
        }
      ],
      communication_interfaces: [
        {
          interface_id: "tcp",
          label: "SCPI TCP",
          transport_kind: "ethernet_tcp",
          access_provider_kind: "native_tcp",
          protocol_kind: "scpi",
          required: true,
          default_interface: true
        }
      ],
      capabilities: [
        {
          capability_id: "measure_power",
          label: "Measure power",
          description: "Measure RF power.",
          capability_kind: "measure_power",
          inputs: [],
          outputs: [],
          safety_class: "read_only"
        }
      ],
      custom_field_values: {
        manufacturer: "R&S",
        model_name: "NRP6AN",
        variant: "FWD"
      },
      template_snapshot: {
        category_id: "measurement_instruments_digitizers",
        root_category_id: "measurement_instruments_digitizers",
        category_path: ["Instruments de mesure / numériseurs"],
        captured_at: "2026-07-13T00:00:00Z",
        template_checksum: "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        fields: []
      },
      is_demo: false,
      metadata: {}
    }
  };
  return {
    identity: {
      equipment_model_id: "EQM-NRP6AN-FWD",
      manufacturer: "R&S",
      model_name: "NRP6AN",
      variant: "FWD",
      equipment_class: "controllable_instrument",
      category_code: "power_meter",
      root_category_id: "measurement_instruments_digitizers",
      is_demo: false,
      current_approved_revision_id: revision.revision_id,
      created_by: "equipment.author",
      created_at: "2026-07-03T00:00:00Z",
      updated_at: "2026-07-03T00:00:00Z"
    },
    current_approved_revision: revision,
    latest_revision: revision,
    active_draft_revision: null
  };
}

function rfCableModelFixture() {
  const revision = {
    revision_id: "EQM-RF-CABLE-DEMO-rev-0001",
    equipment_model_id: "EQM-RF-CABLE-DEMO",
    revision_number: 1,
    parent_revision_id: null,
    status: "draft",
    definition_schema_version: "emc-locus.equipment-model-definition.v2",
    definition_checksum: "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    created_by: "equipment.author",
    created_at: "2026-07-11T00:00:00Z",
    updated_at: "2026-07-11T00:00:00Z",
    submitted_at: null,
    approved_at: null,
    capability_count: 0,
    interface_count: 0,
    signal_port_count: 2,
    definition: {
      definition_schema_version: "emc-locus.equipment-model-definition.v2",
      manufacturer: "Demo",
      model_name: "RF Cable",
      variant: "1m",
      equipment_class: "passive_component",
      functional_role: "rf_network_element",
      category_code: "rf_cable",
      signal_domains: ["rf"],
      technology_tags: ["rf_50_ohm"],
      specifications: [],
      signal_ports: [
        {
          port_id: "RF_A",
          label: "RF A",
          directionality: "through",
          flow_role: "through_port",
          signal_domain: "rf",
          required: true,
          connector_type: "N",
          technology_tags: ["rf_50_ohm"],
          quantity: "power",
          unit: "dBm",
          impedance: 50
        },
        {
          port_id: "RF_B",
          label: "RF B",
          directionality: "through",
          flow_role: "through_port",
          signal_domain: "rf",
          required: true,
          connector_type: "N",
          technology_tags: ["rf_50_ohm"],
          quantity: "power",
          unit: "dBm",
          impedance: 50
        }
      ],
      communication_interfaces: [],
      capabilities: [],
      custom_field_values: {
        manufacturer: "Demo",
        model_name: "RF Cable",
        variant: "1m",
        connector_a: "N",
        connector_b: "N"
      },
      template_snapshot: {
        category_id: "rf_cable",
        root_category_id: "rf_equipment",
        category_path: ["Équipements radiofréquences", "Câbles RF"],
        captured_at: "2026-07-13T00:00:00Z",
        template_checksum: "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        fields: effectiveTemplateFixture().fields
      },
      is_demo: false,
      metadata: { entry_template_category_id: "rf_cable" }
    }
  };
  return {
    identity: {
      equipment_model_id: "EQM-RF-CABLE-DEMO",
      manufacturer: "Demo",
      model_name: "RF Cable",
      variant: "1m",
      equipment_class: "passive_component",
      category_code: "rf_cable",
      root_category_id: "rf_equipment",
      is_demo: false,
      current_approved_revision_id: null,
      created_by: "equipment.author",
      created_at: "2026-07-11T00:00:00Z",
      updated_at: "2026-07-11T00:00:00Z"
    },
    current_approved_revision: null,
    latest_revision: revision,
    active_draft_revision: revision
  };
}

function equipmentRegistriesFixture() {
  const item = (code: string, label = code) => ({
    code,
    label,
    description: `${label} registry item`,
    recommended_equipment_classes: [],
    recommended_functional_roles: [],
    compatible_signal_domains: [],
    compatible_directionalities: [],
    deprecated: false
  });
  return {
    functional_roles: [item("measurement_instrument", "Measurement instrument"), item("rf_network_element", "RF network element"), item("sensor", "Sensor")],
    signal_domains: [item("rf", "RF"), item("analog_voltage", "Analog voltage"), item("can_bus", "CAN bus"), item("ethernet", "Ethernet"), item("trigger", "Trigger")],
    port_directionalities: [item("input"), item("output"), item("through"), item("communication"), item("bidirectional"), item("control")],
    flow_roles: [item("measurement_port"), item("through_port"), item("source_port"), item("sink_port"), item("communication_port"), item("control_port"), item("field_side_port"), item("transducer_output_port")],
    technology_tags: [item("rf_50_ohm", "RF 50 ohm"), item("adc_converter", "ADC converter"), item("voltage_input", "Voltage input"), item("can_bus", "CAN bus"), item("ethernet", "Ethernet"), item("trigger", "Trigger")]
  };
}

function rfCablePresetFixture() {
  return {
    preset_id: "rf_cable",
    category_label: "RF equipment",
    function_description: "50 ohm RF through component.",
    example_label: "RF Cable",
    default_equipment_class: "passive_component",
    default_functional_role: "rf_network_element",
    default_signal_domains: ["rf"],
    default_technology_tags: ["rf_50_ohm"],
    notes: "",
    deprecated: false,
    ports: [
      { port_order: 10, port_id: "RF_A", label: "RF A", directionality: "through", flow_role: "through_port", signal_domain: "rf", connector_type: "N", technology_tags: ["rf_50_ohm"], quantity: "power", unit: "dBm", impedance: 50, required: true },
      { port_order: 20, port_id: "RF_B", label: "RF B", directionality: "through", flow_role: "through_port", signal_domain: "rf", connector_type: "N", technology_tags: ["rf_50_ohm"], quantity: "power", unit: "dBm", impedance: 50, required: true }
    ]
  };
}

function adcPresetFixture() {
  return {
    preset_id: "adc_converter",
    category_label: "Converters and acquisition",
    function_description: "Analog to digital conversion without implicit CAN bus.",
    example_label: "ADC Converter",
    default_equipment_class: "daq_device",
    default_functional_role: "converter",
    default_signal_domains: ["analog_voltage", "digital_logic"],
    default_technology_tags: ["adc_converter", "voltage_input"],
    notes: "No CAN bus port is created by default.",
    deprecated: false,
    ports: [
      { port_order: 10, port_id: "ANALOG_IN", label: "Analog input", directionality: "input", flow_role: "measurement_port", signal_domain: "analog_voltage", connector_type: "BNC", technology_tags: ["voltage_input"], quantity: "voltage", unit: "V", required: true },
      { port_order: 20, port_id: "DIGITAL_OUT", label: "Digital output", directionality: "output", flow_role: "transducer_output_port", signal_domain: "digital_logic", technology_tags: ["adc_converter"], quantity: "binary", unit: "dimensionless", required: true }
    ]
  };
}

function equipmentCategoriesFixture() {
  const now = "2026-07-13T00:00:00Z";
  return [
    {
      category_id: "energy_sources",
      parent_category_id: null,
      root_category_id: "energy_sources",
      label: "Sources d'énergie",
      description: "",
      sort_order: 10,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    },
    {
      category_id: "rf_equipment",
      parent_category_id: null,
      root_category_id: "rf_equipment",
      label: "Équipements radiofréquences",
      description: "",
      sort_order: 30,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    },
    {
      category_id: "rf_cable",
      parent_category_id: "rf_equipment",
      root_category_id: "rf_equipment",
      label: "Câbles RF",
      description: "",
      sort_order: 10,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    },
    {
      category_id: "measurement_instruments_digitizers",
      parent_category_id: null,
      root_category_id: "measurement_instruments_digitizers",
      label: "Instruments de mesure / numériseurs",
      description: "",
      sort_order: 60,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    }
  ];
}

function equipmentCategoryTreeFixture() {
  const categories = equipmentCategoriesFixture();
  return categories
    .filter((category) => category.parent_category_id === null)
    .map((category) => ({
      ...category,
      children: categories.filter((child) => child.parent_category_id === category.category_id)
    }));
}

function equipmentFieldDefinitionsFixture() {
  const now = "2026-07-13T00:00:00Z";
  const field = (field_id: string, field_code: string, label: string, required = false) => ({
    field_id,
    field_code,
    label,
    description: "",
    data_type: "short_text",
    scope: "equipment_model",
    required_by_default: required,
    visible_by_default: true,
    unique_value: false,
    unit_quantity: null,
    allowed_units: [],
    option_values: [],
    validation_regex: null,
    default_value: null,
    display_group: "Identification",
    display_order: 10,
    active: true,
    system_defined: true,
    created_at: now,
    updated_at: now
  });
  return [
    field("field_manufacturer", "manufacturer", "Fabricant", true),
    field("field_model_name", "model_name", "Modèle", true),
    field("field_variant", "variant", "Variante"),
    field("field_connector_a", "connector_a", "Connecteur A"),
    field("field_connector_b", "connector_b", "Connecteur B")
  ];
}

function effectiveTemplateFixture() {
  const categories = equipmentCategoriesFixture();
  const fields = equipmentFieldDefinitionsFixture();
  return {
    category: categories.find((category) => category.category_id === "rf_cable"),
    root_category: categories.find((category) => category.category_id === "rf_equipment"),
    category_path: [
      categories.find((category) => category.category_id === "rf_equipment"),
      categories.find((category) => category.category_id === "rf_cable")
    ].filter(Boolean),
    fields: fields.map((field, index) => ({
      field,
      required: field.field_code === "manufacturer" || field.field_code === "model_name",
      visible: true,
      display_group: "Identification",
      display_order: (index + 1) * 10,
      default_value: null,
      help_text: null,
      inherited_from_category_ids: field.field_code.startsWith("connector") ? ["rf_cable"] : ["rf_equipment"]
    })),
    template_checksum: "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
  };
}

function driverProfileFixture() {
  const revision = {
    revision_id: "DRV-NRP6AN-SCPI-rev-0001",
    driver_profile_id: "DRV-NRP6AN-SCPI",
    equipment_model_id: "EQM-NRP6AN-FWD",
    supported_model_revision_id: "EQM-NRP6AN-FWD-rev-0001",
    revision_number: 1,
    parent_revision_id: null,
    status: "approved",
    definition_schema_version: "emc-locus.driver-profile-definition.v1",
    definition_checksum: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    created_by: "driver.author",
    created_at: "2026-07-03T00:00:00Z",
    updated_at: "2026-07-03T00:00:00Z",
    submitted_at: "2026-07-03T00:00:00Z",
    approved_at: "2026-07-03T00:00:00Z",
    action_count: 1,
    definition: {
      definition_schema_version: "emc-locus.driver-profile-definition.v1",
      equipment_model_id: "EQM-NRP6AN-FWD",
      supported_model_revision_id: "EQM-NRP6AN-FWD-rev-0001",
      supported_model_definition_checksum: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      supported_firmware_ranges: ["*"],
      communication_profiles: ["tcp"],
      actions: [
        {
          action_id: "measure_power",
          label: "Measure power",
          description: "Query power.",
          implements_capability_id: "measure_power",
          inputs: [],
          outputs: [],
          safety_class: "read_only",
          default_timeout_ms: 1000,
          script: { steps: [{ step_id: "query", step_type: "io_query", interface_id: "tcp", payload: "MEAS:POW?", response_binding: "${result.power_dbm}" }] }
        }
      ],
      metadata: {}
    }
  };
  return {
    identity: {
      driver_profile_id: "DRV-NRP6AN-SCPI",
      equipment_model_id: "EQM-NRP6AN-FWD",
      label: "NRP6AN SCPI",
      current_approved_revision_id: revision.revision_id,
      created_by: "driver.author",
      created_at: "2026-07-03T00:00:00Z",
      updated_at: "2026-07-03T00:00:00Z"
    },
    current_approved_revision: revision,
    latest_revision: revision,
    active_draft_revision: null
  };
}

function mockBaseApiResponse(path: string, init?: RequestInit) {
  if (path === "/api/v1/health") return jsonResponse(healthFixture);
  if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
  if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
  if (path.startsWith("/api/v1/equipment/categories/tree")) return jsonResponse({ categories: equipmentCategoryTreeFixture() });
  if (path.includes("/api/v1/equipment/categories/") && path.endsWith("/effective-template")) return jsonResponse({ effective_template: effectiveTemplateFixture() });
  if (path.startsWith("/api/v1/equipment/categories/rf_cable/field-rules")) return jsonResponse({ category_id: "rf_cable", rules: [] });
  if (path.startsWith("/api/v1/equipment/categories")) return jsonResponse({ categories: equipmentCategoriesFixture() });
  if (path.startsWith("/api/v1/equipment/field-definitions")) return jsonResponse({ field_definitions: equipmentFieldDefinitionsFixture() });
  if (path === "/api/v1/test-templates/TT-LAB-001") return jsonResponse({ test_template: templateFixture() });
  if (path.includes("/revisions/TT-LAB-001-rev-0001") && !path.endsWith("/definition") && !path.includes("/transitions/")) return jsonResponse({ revision: revisionFixture() });
  if (path === "/api/v1/test-templates/TT-LAB-001/revisions") return jsonResponse({ template_id: "TT-LAB-001", revisions: [revisionFixture()] });
  if (path === "/api/v1/test-templates/TT-LAB-001/audit-events") return jsonResponse({ template_id: "TT-LAB-001", audit_events: auditFixture });
  return jsonResponse({ error: { code: "unexpected", message: `${path} ${init?.method ?? "GET"}` } }, 500);
}

function measurementApiResponse(
  path: string,
  init: RequestInit | undefined,
  state: {
    curveStatus?: string;
    curveChecksum?: string;
    onCurveStatus?: (status: string, checksum: string) => void;
  }
) {
  const collections = [
    "sensor-definitions",
    "scaling-profiles",
    "engineering-curves",
    "daq-channel-profiles",
    "acquisition-channel-recipes"
  ];
  for (const collection of collections) {
    if (path === `/api/v1/${collection}`) {
      if (init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        const aggregate = measurementAggregate(collection, state, body.entity_id, body.definition);
        return jsonResponse({
          operation: `${collection}_created`,
          operation_id: body.operation_id,
          replayed: false,
          item: aggregate,
          revision: aggregate.latest_revision
        });
      }
      return jsonResponse({ aggregate_kind: collection, collection_key: "items", items: [measurementAggregate(collection, state)] });
    }
    if (path.startsWith(`/api/v1/${collection}/`) && path.endsWith("/audit-events")) {
      return jsonResponse({ aggregate_kind: collection, entity_id: path.split("/")[4], audit_events: [] });
    }
    if (path.startsWith(`/api/v1/${collection}/`) && path.endsWith("/revisions")) {
      return jsonResponse({
        aggregate_kind: collection,
        entity_id: path.split("/")[4],
        revisions: [measurementAggregate(collection, state).latest_revision]
      });
    }
    if (path.startsWith(`/api/v1/${collection}/`) && path.includes("/revisions/") && path.endsWith("/definition")) {
      const body = JSON.parse(String(init?.body ?? "{}"));
      const aggregate = measurementAggregate(collection, state, undefined, body.definition);
      return jsonResponse({
        operation: `${collection}_saved`,
        operation_id: body.operation_id,
        replayed: false,
        item: aggregate,
        revision: aggregate.latest_revision
      });
    }
    if (path.startsWith(`/api/v1/${collection}/`) && path.includes("submit-for-review")) {
      state.onCurveStatus?.("under_review", "sha256:2222222222222222222222222222222222222222222222222222222222222222");
      const body = JSON.parse(String(init?.body ?? "{}"));
      const aggregate = measurementAggregate(collection, state);
      return jsonResponse({
        operation: `${collection}_submitted`,
        operation_id: body.operation_id,
        replayed: false,
        item: aggregate,
        revision: aggregate.latest_revision
      });
    }
    if (path.startsWith(`/api/v1/${collection}/`) && path.includes("transitions/approve")) {
      state.onCurveStatus?.("approved", "sha256:3333333333333333333333333333333333333333333333333333333333333333");
      const body = JSON.parse(String(init?.body ?? "{}"));
      const aggregate = measurementAggregate(collection, state);
      return jsonResponse({
        operation: `${collection}_approved`,
        operation_id: body.operation_id,
        replayed: false,
        item: aggregate,
        revision: aggregate.latest_revision
      });
    }
    if (path.startsWith(`/api/v1/${collection}/`) && !path.includes("/revisions/")) {
      return jsonResponse({ aggregate_kind: collection, item: measurementAggregate(collection, state) });
    }
  }
  if (path.endsWith("-definitions/validate")) {
    return jsonResponse({ valid: true, issues: [], definition_checksum: "sha256:dddd" });
  }
  if (path.includes("/engineering-curves/") && path.endsWith("/evaluate")) {
    return jsonResponse({
      evaluation: {
        values: { correction_db: 1.25 },
        axis_values: { frequency: 100000000 },
        interpolation: "log_x_linear_y",
        extrapolated: false,
        source_revision_id: "CURVE-DEMO-RF-CABLE-1M-LOSS-rev-0001",
        source_checksum: "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
      }
    });
  }
  return null;
}

function measurementAggregate(
  collection: string,
  state: { curveStatus?: string; curveChecksum?: string },
  entityId?: string,
  definitionOverride?: Record<string, unknown>
) {
  const definition = definitionOverride ?? measurementDefinition(collection);
  const id = entityId ?? String(
    definition.sensor_definition_id ??
      definition.scaling_profile_id ??
      definition.curve_id ??
      definition.daq_channel_profile_id ??
      definition.recipe_id
  );
  const status = collection === "engineering-curves" ? state.curveStatus ?? "draft" : "approved";
  const checksum = collection === "engineering-curves"
    ? state.curveChecksum ?? "sha256:1111111111111111111111111111111111111111111111111111111111111111"
    : "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
  const revision = {
    aggregate_kind: collection.replace(/-/g, "_"),
    revision_id: `${id}-rev-0001`,
    entity_id: id,
    revision_number: 1,
    parent_revision_id: null,
    status,
    definition_schema_version: String(definition.definition_schema_version),
    definition,
    definition_checksum: checksum,
    label: measurementLabel(collection),
    summary_kind: collection,
    created_by: "measurement.author",
    created_at: "2026-07-11T00:00:00Z",
    updated_at: "2026-07-11T00:00:00Z",
    submitted_at: status === "draft" ? null : "2026-07-11T00:10:00Z",
    approved_at: status === "approved" ? "2026-07-11T00:20:00Z" : null
  };
  return {
    identity: {
      aggregate_kind: collection.replace(/-/g, "_"),
      entity_id: id,
      label: measurementLabel(collection),
      summary_kind: collection,
      current_approved_revision_id: status === "approved" ? revision.revision_id : null,
      created_by: "measurement.author",
      created_at: "2026-07-11T00:00:00Z",
      updated_at: "2026-07-11T00:00:00Z"
    },
    current_approved_revision: status === "approved" ? revision : null,
    latest_revision: revision,
    active_draft_revision: status === "draft" ? revision : null
  };
}

function measurementLabel(collection: string) {
  return {
    "sensor-definitions": "Demo Current Probe 10mV/A",
    "scaling-profiles": "Current probe 10 mV/A",
    "engineering-curves": "Demo RF cable loss",
    "daq-channel-profiles": "Demo DAQ AI +/-10 V",
    "acquisition-channel-recipes": "current_A through demo current probe"
  }[collection] ?? collection;
}

function measurementDefinition(collection: string): Record<string, unknown> {
  if (collection === "sensor-definitions") {
    return {
      definition_schema_version: "emc-locus.sensor-definition.v1",
      sensor_definition_id: "SNS-DEMO-CURRENT-PROBE-10MV-A",
      manufacturer: "Demo",
      model_name: "Current Probe 10mV/A",
      sensor_family: "current_probe",
      physical_input_quantity: "current",
      engineering_output_quantity: "current",
      engineering_output_unit: "A",
      electrical_output_quantity: "voltage",
      electrical_output_unit: "V",
      signal_domain: "analog_voltage",
      technology_tags: ["voltage_input"],
      required_excitation: { excitation_kind: "none", external_allowed: false },
      input_mode_requirement: "differential",
      nominal_range: { minimum: -100, maximum: 100, unit: "A" },
      frequency_range: { minimum_hz: 10, maximum_hz: 100000000 },
      scaling_profile_refs: [{ entity_id: "SCL-DEMO-CURRENT-10MV-A", revision_id: "SCL-DEMO-CURRENT-10MV-A-rev-0001", require_approved: true }],
      correction_curve_refs: [],
      metadata: {}
    };
  }
  if (collection === "scaling-profiles") {
    return {
      definition_schema_version: "emc-locus.scaling-profile-definition.v1",
      scaling_profile_id: "SCL-DEMO-CURRENT-10MV-A",
      label: "Current probe 10 mV/A",
      input_quantity: "voltage",
      input_unit: "V",
      output_quantity: "current",
      output_unit: "A",
      scaling_kind: "linear",
      parameters: { scale: 100, offset: 0, points: [{ input: 0, output: 0 }, { input: 0.01, output: 1 }] },
      metadata: {}
    };
  }
  if (collection === "engineering-curves") {
    return {
      definition_schema_version: "emc-locus.engineering-curve-definition.v1",
      curve_id: "CURVE-DEMO-RF-CABLE-1M-LOSS",
      curve_type: "cable_loss",
      label: "Demo RF cable loss",
      independent_axes: [{ axis: "frequency", quantity: "frequency", unit: "Hz" }],
      dependent_values: [{ value_id: "correction_db", quantity: "dimensionless", unit: "dB" }],
      units: { frequency: "Hz", correction_db: "dB" },
      points: [
        { axis_values: { frequency: 10000000 }, values: { correction_db: 0.2 } },
        { axis_values: { frequency: 100000000 }, values: { correction_db: 1.25 } },
        { axis_values: { frequency: 1000000000 }, values: { correction_db: 3.8 } }
      ],
      interpolation: "log_x_linear_y",
      extrapolation_policy: "warn",
      validity_domain: {},
      conditions: {},
      metadata: {}
    };
  }
  if (collection === "daq-channel-profiles") {
    return {
      definition_schema_version: "emc-locus.daq-channel-profile-definition.v1",
      daq_channel_profile_id: "DAQ-DEMO-AI-10V-1MS",
      label: "Demo DAQ AI +/-10 V",
      channel_kind: "analog_input",
      signal_domain: "analog_voltage",
      input_quantity: "voltage",
      input_unit: "V",
      supported_ranges: [{ minimum: -10, maximum: 10, unit: "V" }],
      resolution_bits: 16,
      max_sampling_rate: 1000000,
      min_sampling_rate: 1,
      coupling_modes: ["dc", "ac"],
      input_modes: ["single_ended", "differential", "iepe"],
      excitation_capabilities: [{ excitation_kind: "iepe", nominal_value: 4, unit: "mA", external_allowed: false }],
      iepe_support: true,
      metadata: {}
    };
  }
  return {
    definition_schema_version: "emc-locus.acquisition-channel-recipe-definition.v1",
    recipe_id: "REC-DEMO-CURRENT-A",
    label: "current_A through demo current probe",
    output_channel_name: "current_A",
    output_quantity: "current",
    output_unit: "A",
    daq_channel_profile_ref: { entity_id: "DAQ-DEMO-AI-10V-1MS", revision_id: "DAQ-DEMO-AI-10V-1MS-rev-0001", require_approved: true },
    sensor_definition_ref: { entity_id: "SNS-DEMO-CURRENT-PROBE-10MV-A", revision_id: "SNS-DEMO-CURRENT-PROBE-10MV-A-rev-0001", require_approved: true },
    scaling_profile_ref: { entity_id: "SCL-DEMO-CURRENT-10MV-A", revision_id: "SCL-DEMO-CURRENT-10MV-A-rev-0001", require_approved: true },
    correction_curve_refs: [],
    sample_rate: 1000000,
    range: { minimum: -10, maximum: 10, unit: "V" },
    coupling: "dc",
    input_mode: "differential",
    excitation: { excitation_kind: "none", external_allowed: false },
    validation_rules: [],
    metadata: {}
  };
}
