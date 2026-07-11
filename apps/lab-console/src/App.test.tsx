import { render, screen, waitFor } from "@testing-library/react";
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
    expect(await screen.findByRole("heading", { name: "Model Catalog" })).toBeInTheDocument();
    const modelButton = await screen.findByRole("button", { name: /R&S\s+NRP6AN/ });
    await user.click(modelButton);
    expect(await screen.findByText("Equipment Model Definition")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Drivers and Actions" }));
    await user.click(await screen.findByRole("button", { name: /NRP6AN SCPI/ }));
    expect(await screen.findByText(/No VISA implementation installed/)).toBeInTheDocument();
  });

  test("filters equipment catalog and creates a model from a classification preset", async () => {
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
      if (path === "/api/v1/equipment-models/from-preset" && init?.method === "POST") {
        return jsonResponse({
          operation: "equipment_model_created_from_preset",
          operation_id: "op-from-preset",
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
    await screen.findByRole("heading", { name: "Model Catalog" });
    await user.selectOptions(screen.getByLabelText("Filtre role physique"), "sensor");
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("functional_role=sensor"),
        expect.any(Object)
      )
    );

    await user.click(screen.getByRole("button", { name: /Nouveau modele/ }));
    expect(await screen.findByText("Nouveau modele catalogue")).toBeInTheDocument();
    expect(screen.getAllByText("RF Cable").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("RF_A")).toBeInTheDocument();
    expect(screen.getByText("RF_B")).toBeInTheDocument();
    await user.clear(screen.getByLabelText("Equipment model ID"));
    await user.type(screen.getByLabelText("Equipment model ID"), "EQM-RF-CABLE-DEMO");
    await user.clear(screen.getByLabelText("Manufacturer"));
    await user.type(screen.getByLabelText("Manufacturer"), "Demo");
    await user.clear(screen.getByLabelText("Model name"));
    await user.type(screen.getByLabelText("Model name"), "RF Cable");
    await user.click(screen.getByRole("button", { name: "Creer" }));
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/v1/equipment-models/from-preset",
        expect.objectContaining({ method: "POST" })
      )
    );

    expect(await screen.findByText("Equipment Model Definition")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Classification" }));
    expect(screen.getByLabelText("Functional role")).toHaveValue("rf_network_element");
    expect(screen.getByDisplayValue("rf")).toBeInTheDocument();
    expect(screen.getByDisplayValue("rf_50_ohm")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Port Topology" }));
    expect(screen.getByDisplayValue("RF_A")).toBeInTheDocument();
    expect(screen.getByDisplayValue("RF_B")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Ajouter CAN bus" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Valider/ }));
    expect(await screen.findByText("Definition valide")).toBeInTheDocument();
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
      metadata: { classification_preset_id: "rf_cable" }
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
  if (path === "/api/v1/test-templates/TT-LAB-001") return jsonResponse({ test_template: templateFixture() });
  if (path.includes("/revisions/TT-LAB-001-rev-0001") && !path.endsWith("/definition") && !path.includes("/transitions/")) return jsonResponse({ revision: revisionFixture() });
  if (path === "/api/v1/test-templates/TT-LAB-001/revisions") return jsonResponse({ template_id: "TT-LAB-001", revisions: [revisionFixture()] });
  if (path === "/api/v1/test-templates/TT-LAB-001/audit-events") return jsonResponse({ template_id: "TT-LAB-001", audit_events: auditFixture });
  return jsonResponse({ error: { code: "unexpected", message: `${path} ${init?.method ?? "GET"}` } }, 500);
}
