import {
  AlertTriangle,
  CheckCircle2,
  Copy,
  Cpu,
  GitBranch,
  Play,
  RefreshCw,
  Save,
  Send,
  ShieldCheck
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { ApiError, equipmentApi, type OperationContext } from "../../api";
import type {
  CommunicationProviderStatus,
  DriverActionDefinition,
  DriverProfileAggregate,
  DriverProfileDefinition,
  DriverProfileRevision,
  DriverScriptStep,
  DriverSimulationResult,
  EquipmentAuditEvent,
  EquipmentClass,
  EquipmentModelAggregate,
  EquipmentModelDefinition,
  EquipmentModelRevision,
  EquipmentValidationResult,
  FunctionalRole
} from "../../models/equipment";

type EquipmentSpace =
  | "catalog"
  | "drivers"
  | "metrology"
  | "fleet"
  | "connections"
  | "readiness";

type ModelSection =
  | "general"
  | "specifications"
  | "ports"
  | "interfaces"
  | "capabilities"
  | "revisions"
  | "audit"
  | "json";

type DriverSection =
  | "general"
  | "actions"
  | "script"
  | "simulation"
  | "revisions"
  | "audit"
  | "json";

const equipmentSpaces: Array<[EquipmentSpace, string]> = [
  ["catalog", "Model Catalog"],
  ["drivers", "Drivers and Actions"],
  ["metrology", "Metrology"],
  ["fleet", "Physical Fleet"],
  ["connections", "Connections"],
  ["readiness", "Readiness"]
];

const modelSections: Array<[ModelSection, string]> = [
  ["general", "General"],
  ["specifications", "Specifications"],
  ["ports", "Physical Ports"],
  ["interfaces", "Communication"],
  ["capabilities", "Capabilities"],
  ["revisions", "Revisions"],
  ["audit", "Audit"],
  ["json", "Advanced JSON"]
];

const driverSections: Array<[DriverSection, string]> = [
  ["general", "General"],
  ["actions", "Actions"],
  ["script", "Script"],
  ["simulation", "Simulation"],
  ["revisions", "Revisions"],
  ["audit", "Audit"],
  ["json", "Advanced JSON"]
];

const equipmentClasses: EquipmentClass[] = [
  "controllable_instrument",
  "daq_device",
  "acquisition_device",
  "converter",
  "sensor",
  "transducer",
  "passive_component",
  "switching_device",
  "motion_system",
  "facility",
  "software_adapter",
  "manual_equipment"
];

const functionalRoles: FunctionalRole[] = [
  "energy_source",
  "signal_source",
  "rf_network_element",
  "sensor",
  "actuator",
  "measurement_instrument",
  "acquisition_device",
  "converter",
  "control_system",
  "software_system",
  "facility",
  "manual_accessory"
];

const context: OperationContext = {
  actor: "equipment.author",
  reason: "operation LAB CONSOLE equipment"
};

export function EquipmentWorkspace() {
  const [space, setSpace] = useState<EquipmentSpace>("catalog");
  const [models, setModels] = useState<EquipmentModelAggregate[]>([]);
  const [drivers, setDrivers] = useState<DriverProfileAggregate[]>([]);
  const [providers, setProviders] = useState<CommunicationProviderStatus[]>([]);
  const [query, setQuery] = useState("");
  const [classFilter, setClassFilter] = useState("all");
  const [loadState, setLoadState] = useState<"loading" | "ready" | "error">("loading");
  const [operationError, setOperationError] = useState<string | null>(null);

  const [selectedModel, setSelectedModel] = useState<EquipmentModelAggregate | null>(null);
  const [selectedModelRevision, setSelectedModelRevision] = useState<EquipmentModelRevision | null>(null);
  const [modelDefinition, setModelDefinition] = useState<EquipmentModelDefinition | null>(null);
  const [modelChecksum, setModelChecksum] = useState("");
  const [modelSection, setModelSection] = useState<ModelSection>("general");
  const [modelValidation, setModelValidation] = useState<EquipmentValidationResult | null>(null);
  const [modelRevisions, setModelRevisions] = useState<EquipmentModelRevision[]>([]);
  const [modelAudit, setModelAudit] = useState<EquipmentAuditEvent[]>([]);
  const [modelJsonDraft, setModelJsonDraft] = useState("");

  const [selectedDriver, setSelectedDriver] = useState<DriverProfileAggregate | null>(null);
  const [selectedDriverRevision, setSelectedDriverRevision] = useState<DriverProfileRevision | null>(null);
  const [driverDefinition, setDriverDefinition] = useState<DriverProfileDefinition | null>(null);
  const [driverChecksum, setDriverChecksum] = useState("");
  const [driverSection, setDriverSection] = useState<DriverSection>("general");
  const [driverValidation, setDriverValidation] = useState<EquipmentValidationResult | null>(null);
  const [driverRevisions, setDriverRevisions] = useState<DriverProfileRevision[]>([]);
  const [driverAudit, setDriverAudit] = useState<EquipmentAuditEvent[]>([]);
  const [simulation, setSimulation] = useState<DriverSimulationResult | null>(null);
  const [driverJsonDraft, setDriverJsonDraft] = useState("");

  useEffect(() => {
    void refresh();
  }, []);

  const filteredModels = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return models.filter((model) => {
      const definition = model.latest_revision?.definition ?? model.current_approved_revision?.definition;
      const text = `${model.identity.manufacturer} ${model.identity.model_name} ${model.identity.variant ?? ""} ${model.identity.category_code} ${definition?.functional_role ?? ""} ${definition?.signal_domains?.join(" ") ?? ""} ${definition?.technology_tags?.join(" ") ?? ""}`.toLowerCase();
      return (
        (!normalized || text.includes(normalized)) &&
        (classFilter === "all" || model.identity.equipment_class === classFilter)
      );
    });
  }, [models, query, classFilter]);

  const approvedModels = models.filter((model) => model.current_approved_revision);
  const modelReadOnly = selectedModelRevision?.status !== "draft";
  const driverReadOnly = selectedDriverRevision?.status !== "draft";

  async function refresh() {
    setLoadState("loading");
    setOperationError(null);
    try {
      const [modelList, driverList, providerList] = await Promise.all([
        equipmentApi.listModels(),
        equipmentApi.listDrivers(),
        equipmentApi.providers()
      ]);
      setModels(modelList.equipment_models);
      setDrivers(driverList.driver_profiles);
      setProviders(providerList.providers);
      setLoadState("ready");
    } catch (error) {
      setLoadState("error");
      setOperationError(errorMessage(error));
    }
  }

  async function openModel(model: EquipmentModelAggregate, revision?: EquipmentModelRevision | null) {
    setOperationError(null);
    const target = revision ?? model.active_draft_revision ?? model.current_approved_revision ?? model.latest_revision;
    if (!target) return;
    try {
      const [detail, revisions, audit] = await Promise.all([
        equipmentApi.getModel(model.identity.equipment_model_id),
        equipmentApi.listModelRevisions(model.identity.equipment_model_id),
        equipmentApi.listModelAudit(model.identity.equipment_model_id)
      ]);
      const freshRevision =
        revisions.revisions.find((item) => item.revision_id === target.revision_id) ?? target;
      setSelectedModel(detail.equipment_model);
      setSelectedModelRevision(freshRevision);
      setModelDefinition(freshRevision.definition);
      setModelChecksum(freshRevision.definition_checksum);
      setModelJsonDraft(JSON.stringify(freshRevision.definition, null, 2));
      setModelRevisions(revisions.revisions);
      setModelAudit(audit.audit_events);
      setModelValidation(null);
      setModelSection("general");
      setSpace("catalog");
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  }

  async function openDriver(driver: DriverProfileAggregate, revision?: DriverProfileRevision | null) {
    setOperationError(null);
    const target = revision ?? driver.active_draft_revision ?? driver.current_approved_revision ?? driver.latest_revision;
    if (!target) return;
    try {
      const [revisions, audit] = await Promise.all([
        equipmentApi.listDriverRevisions(driver.identity.driver_profile_id),
        equipmentApi.listDriverAudit(driver.identity.driver_profile_id)
      ]);
      const freshRevision =
        revisions.revisions.find((item) => item.revision_id === target.revision_id) ?? target;
      setSelectedDriver(driver);
      setSelectedDriverRevision(freshRevision);
      setDriverDefinition(freshRevision.definition);
      setDriverChecksum(freshRevision.definition_checksum);
      setDriverJsonDraft(JSON.stringify(freshRevision.definition, null, 2));
      setDriverRevisions(revisions.revisions);
      setDriverAudit(audit.audit_events);
      setDriverValidation(null);
      setSimulation(null);
      setDriverSection("general");
      setSpace("drivers");
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  }

  async function createModel() {
    const id = `EQM-DEMO-${Date.now().toString(36).toUpperCase()}`;
    await runOperation(async () => {
      const result = await equipmentApi.createModel({
        equipment_model_id: id,
        definition: defaultEquipmentModelDefinition(id),
        ...context
      });
      await refresh();
      await openModel(result.aggregate, result.revision);
    });
  }

  async function cloneSelectedModel() {
    if (!selectedModel) return;
    const id = `${selectedModel.identity.equipment_model_id}-COPY-${Date.now().toString(36).toUpperCase()}`;
    await runOperation(async () => {
      const result = await equipmentApi.cloneModel(selectedModel.identity.equipment_model_id, {
        new_equipment_model_id: id,
        model_name: `${selectedModel.identity.model_name} copy`,
        ...context
      });
      await refresh();
      await openModel(result.aggregate, result.revision);
    });
  }

  async function validateModel() {
    if (!modelDefinition) return;
    await runOperation(async () => {
      setModelValidation(await equipmentApi.validateModelDefinition(modelDefinition));
    });
  }

  async function saveModelDraft() {
    if (!selectedModel || !selectedModelRevision || !modelDefinition || modelReadOnly) return;
    await runOperation(async () => {
      const result = await equipmentApi.saveModelDraft(
        selectedModel.identity.equipment_model_id,
        selectedModelRevision.revision_id,
        modelChecksum,
        modelDefinition,
        context
      );
      await refresh();
      await openModel(result.aggregate, result.revision);
    });
  }

  async function submitModel() {
    if (!selectedModel || !selectedModelRevision || modelReadOnly) return;
    await runOperation(async () => {
      const result = await equipmentApi.submitModel(
        selectedModel.identity.equipment_model_id,
        selectedModelRevision.revision_id,
        context
      );
      await refresh();
      await openModel(result.aggregate, result.revision);
    });
  }

  async function approveModel() {
    if (!selectedModel || !selectedModelRevision || selectedModelRevision.status !== "under_review") return;
    await runOperation(async () => {
      const result = await equipmentApi.approveModel(
        selectedModel.identity.equipment_model_id,
        selectedModelRevision.revision_id,
        { ...context, actor: "quality.approver", reason: "approbation catalogue equipement" }
      );
      await refresh();
      await openModel(result.aggregate, result.revision);
    });
  }

  async function deriveModel() {
    const approvedRevision = selectedModel?.current_approved_revision;
    if (!selectedModel || !approvedRevision) return;
    await runOperation(async () => {
      const result = await equipmentApi.deriveModelRevision(
        selectedModel.identity.equipment_model_id,
        approvedRevision.revision_id,
        context
      );
      await refresh();
      await openModel(result.aggregate, result.revision);
    });
  }

  async function createDriver() {
    const model = selectedModel?.current_approved_revision
      ? selectedModel
      : approvedModels[0] ?? null;
    if (!model?.current_approved_revision) {
      setOperationError("Aucun modele approuve disponible pour creer un driver.");
      return;
    }
    const id = `DRV-${model.identity.equipment_model_id}-${Date.now().toString(36).toUpperCase()}`;
    await runOperation(async () => {
      const result = await equipmentApi.createDriver({
        driver_profile_id: id,
        label: `${model.identity.manufacturer} ${model.identity.model_name} driver`,
        definition: defaultDriverProfileDefinition(model),
        ...context,
        actor: "driver.author",
        reason: "creation driver LAB CONSOLE"
      });
      await refresh();
      await openDriver(result.aggregate, result.revision);
    });
  }

  async function validateDriver() {
    if (!driverDefinition) return;
    await runOperation(async () => {
      setDriverValidation(await equipmentApi.validateDriverDefinition(driverDefinition));
    });
  }

  async function saveDriverDraft() {
    if (!selectedDriver || !selectedDriverRevision || !driverDefinition || driverReadOnly) return;
    await runOperation(async () => {
      const result = await equipmentApi.saveDriverDraft(
        selectedDriver.identity.driver_profile_id,
        selectedDriverRevision.revision_id,
        driverChecksum,
        driverDefinition,
        { ...context, actor: "driver.author", reason: "sauvegarde driver" }
      );
      await refresh();
      await openDriver(result.aggregate, result.revision);
    });
  }

  async function submitDriver() {
    if (!selectedDriver || !selectedDriverRevision || driverReadOnly) return;
    await runOperation(async () => {
      const result = await equipmentApi.submitDriver(
        selectedDriver.identity.driver_profile_id,
        selectedDriverRevision.revision_id,
        { ...context, actor: "driver.author", reason: "soumission driver" }
      );
      await refresh();
      await openDriver(result.aggregate, result.revision);
    });
  }

  async function approveDriver() {
    if (!selectedDriver || !selectedDriverRevision || selectedDriverRevision.status !== "under_review") return;
    await runOperation(async () => {
      const result = await equipmentApi.approveDriver(
        selectedDriver.identity.driver_profile_id,
        selectedDriverRevision.revision_id,
        { ...context, actor: "quality.approver", reason: "approbation driver" }
      );
      await refresh();
      await openDriver(result.aggregate, result.revision);
    });
  }

  async function deriveDriver() {
    const approvedRevision = selectedDriver?.current_approved_revision;
    if (!selectedDriver || !approvedRevision) return;
    await runOperation(async () => {
      const result = await equipmentApi.deriveDriverRevision(
        selectedDriver.identity.driver_profile_id,
        approvedRevision.revision_id,
        { ...context, actor: "driver.author", reason: "nouvelle revision driver" }
      );
      await refresh();
      await openDriver(result.aggregate, result.revision);
    });
  }

  async function simulateSelectedAction(action: DriverActionDefinition) {
    if (!selectedDriverRevision) return;
    const outputName = action.outputs[0]?.name ?? "value";
    await runOperation(async () => {
      const result = await equipmentApi.simulateDriver(
        selectedDriverRevision.driver_profile_id,
        action.action_id,
        {
          scenario_id: `scenario-${action.action_id}`,
          driver_revision_id: selectedDriverRevision.revision_id,
          action_id: action.action_id,
          input_values: {},
          expected_transport_operations: [],
          simulated_responses: ["-12.5"],
          expected_outputs: { [`result.${outputName}`]: -12.5 },
          expected_messages: [],
          expected_final_state: {}
        },
        selectedDriverRevision.revision_id
      );
      setSimulation(result.simulation);
      setDriverSection("simulation");
    });
  }

  async function runOperation(operation: () => Promise<void>) {
    setOperationError(null);
    try {
      await operation();
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  }

  function updateModel(next: EquipmentModelDefinition) {
    setModelDefinition(next);
    setModelJsonDraft(JSON.stringify(next, null, 2));
    setModelValidation(null);
  }

  function applyModelJson() {
    try {
      updateModel(JSON.parse(modelJsonDraft) as EquipmentModelDefinition);
    } catch {
      setOperationError("JSON modele invalide.");
    }
  }

  function updateDriver(next: DriverProfileDefinition) {
    setDriverDefinition(next);
    setDriverJsonDraft(JSON.stringify(next, null, 2));
    setDriverValidation(null);
    setSimulation(null);
  }

  function applyDriverJson() {
    try {
      updateDriver(JSON.parse(driverJsonDraft) as DriverProfileDefinition);
    } catch {
      setOperationError("JSON driver invalide.");
    }
  }

  return (
    <section className="equipmentWorkspace">
      <div className="toolbar equipmentToolbar">
        <label className="searchBox">
          <Cpu size={16} />
          <input
            aria-label="Recherche equipement"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Recherche modele, fabricant, categorie"
          />
        </label>
        <select
          aria-label="Filtre classe equipement"
          value={classFilter}
          onChange={(event) => setClassFilter(event.target.value)}
        >
          <option value="all">Toutes classes</option>
          {equipmentClasses.map((item) => (
            <option value={item} key={item}>
              {item}
            </option>
          ))}
        </select>
        <button onClick={() => void refresh()}>
          <RefreshCw size={16} /> Rafraichir
        </button>
        <button onClick={() => void createModel()}>
          <Cpu size={16} /> Nouveau modele
        </button>
        <button onClick={() => void createDriver()} disabled={approvedModels.length === 0}>
          <GitBranch size={16} /> Nouveau driver
        </button>
      </div>

      {operationError && (
        <div className="conflictBox">
          <AlertTriangle size={18} />
          <div>
            <strong>Operation refusee</strong>
            <p>{operationError}</p>
          </div>
        </div>
      )}

      <div className="equipmentTabs" role="tablist">
        {equipmentSpaces.map(([key, label]) => (
          <button
            key={key}
            className={space === key ? "active" : ""}
            onClick={() => setSpace(key)}
            disabled={!["catalog", "drivers"].includes(key)}
          >
            {label}
          </button>
        ))}
      </div>

      {loadState === "loading" && <StateBlock title="Chargement" detail="Lecture du catalogue equipement." />}
      {loadState === "error" && <StateBlock title="Erreur" detail={operationError ?? "Catalogue indisponible."} />}

      {space === "catalog" && loadState === "ready" && (
        <div className="equipmentLayout">
          <ModelCatalog
            models={filteredModels}
            selected={selectedModel}
            onOpen={(model) => void openModel(model)}
          />
          <ModelStudio
            model={selectedModel}
            revision={selectedModelRevision}
            definition={modelDefinition}
            readOnly={modelReadOnly}
            section={modelSection}
            revisions={modelRevisions}
            audit={modelAudit}
            validation={modelValidation}
            jsonDraft={modelJsonDraft}
            onSection={setModelSection}
            onDefinition={updateModel}
            onJsonDraft={setModelJsonDraft}
            onApplyJson={applyModelJson}
            onValidate={() => void validateModel()}
            onSave={() => void saveModelDraft()}
            onSubmit={() => void submitModel()}
            onApprove={() => void approveModel()}
            onDerive={() => void deriveModel()}
            onClone={() => void cloneSelectedModel()}
            onOpenRevision={(revision) => void openModel(selectedModel!, revision)}
          />
        </div>
      )}

      {space === "drivers" && loadState === "ready" && (
        <div className="equipmentLayout">
          <DriverTree
            models={models}
            drivers={drivers}
            selected={selectedDriver}
            onOpen={(driver) => void openDriver(driver)}
          />
          <DriverStudio
            driver={selectedDriver}
            revision={selectedDriverRevision}
            definition={driverDefinition}
            readOnly={driverReadOnly}
            section={driverSection}
            revisions={driverRevisions}
            audit={driverAudit}
            validation={driverValidation}
            providers={providers}
            simulation={simulation}
            jsonDraft={driverJsonDraft}
            onSection={setDriverSection}
            onDefinition={updateDriver}
            onJsonDraft={setDriverJsonDraft}
            onApplyJson={applyDriverJson}
            onValidate={() => void validateDriver()}
            onSave={() => void saveDriverDraft()}
            onSubmit={() => void submitDriver()}
            onApprove={() => void approveDriver()}
            onDerive={() => void deriveDriver()}
            onSimulate={(action) => void simulateSelectedAction(action)}
            onOpenRevision={(revision) => void openDriver(selectedDriver!, revision)}
          />
        </div>
      )}

      {!["catalog", "drivers"].includes(space) && (
        <StateBlock
          title="Non disponible en 0.11.0"
          detail="Cette sous-section restera liee a la flotte physique, aux connexions station et a la readiness dans une verticale ulterieure."
        />
      )}
    </section>
  );
}

function ModelCatalog(props: {
  models: EquipmentModelAggregate[];
  selected: EquipmentModelAggregate | null;
  onOpen: (model: EquipmentModelAggregate) => void;
}) {
  return (
    <aside className="equipmentList">
      <h2>Model Catalog</h2>
      {props.models.length === 0 && <p>Aucun modele equipement.</p>}
      {props.models.map((model) => (
        <button
          key={model.identity.equipment_model_id}
          className={props.selected?.identity.equipment_model_id === model.identity.equipment_model_id ? "active" : ""}
          onClick={() => props.onOpen(model)}
        >
          <strong>{model.identity.manufacturer} {model.identity.model_name}</strong>
          <span>{model.identity.variant ?? model.identity.category_code}</span>
          <small>
            {model.identity.equipment_class} | rev {model.latest_revision?.revision_number ?? "-"} | {model.latest_revision?.status ?? "no_revision"}
          </small>
        </button>
      ))}
    </aside>
  );
}

function ModelStudio(props: {
  model: EquipmentModelAggregate | null;
  revision: EquipmentModelRevision | null;
  definition: EquipmentModelDefinition | null;
  readOnly: boolean;
  section: ModelSection;
  revisions: EquipmentModelRevision[];
  audit: EquipmentAuditEvent[];
  validation: EquipmentValidationResult | null;
  jsonDraft: string;
  onSection: (section: ModelSection) => void;
  onDefinition: (definition: EquipmentModelDefinition) => void;
  onJsonDraft: (value: string) => void;
  onApplyJson: () => void;
  onValidate: () => void;
  onSave: () => void;
  onSubmit: () => void;
  onApprove: () => void;
  onDerive: () => void;
  onClone: () => void;
  onOpenRevision: (revision: EquipmentModelRevision) => void;
}) {
  if (!props.model || !props.revision || !props.definition) {
    return <StateBlock title="Aucun modele ouvert" detail="Selectionnez ou creez un modele equipement." />;
  }
  const definition = props.definition;
  return (
    <section className="equipmentStudio">
      <div className="studioHeader">
        <div>
          <p className="eyebrow">Equipment Model Definition</p>
          <h2>{props.model.identity.manufacturer} {props.model.identity.model_name}</h2>
          <p className="mono">{props.revision.revision_id} | {props.revision.status}</p>
        </div>
        <div className="headerActions">
          <button onClick={props.onValidate}><CheckCircle2 size={16} /> Valider</button>
          <button onClick={props.onSave} disabled={props.readOnly}><Save size={16} /> Sauvegarder</button>
          <button onClick={props.onSubmit} disabled={props.readOnly || props.revision.status !== "draft"}><Send size={16} /> Soumettre</button>
          <button onClick={props.onApprove} disabled={props.revision.status !== "under_review"}><ShieldCheck size={16} /> Approuver</button>
          <button onClick={props.onDerive} disabled={!props.model.current_approved_revision}><GitBranch size={16} /> Nouvelle revision</button>
          <button onClick={props.onClone}><Copy size={16} /> Cloner</button>
        </div>
      </div>

      <div className="studioLayout equipmentStudioLayout">
        <nav className="sectionNav">
          {modelSections.map(([key, label]) => (
            <button key={key} className={props.section === key ? "active" : ""} onClick={() => props.onSection(key)}>
              {label}
            </button>
          ))}
        </nav>
        <div className="editorPane">
          {props.section === "general" && (
            <EditorCard title="General">
              <Field label="Manufacturer" value={definition.manufacturer} disabled={props.readOnly} onChange={(manufacturer) => props.onDefinition({ ...definition, manufacturer })} />
              <Field label="Model name" value={definition.model_name} disabled={props.readOnly} onChange={(model_name) => props.onDefinition({ ...definition, model_name })} />
              <Field label="Variant" value={definition.variant ?? ""} disabled={props.readOnly} onChange={(variant) => props.onDefinition({ ...definition, variant: optionalString(variant) })} />
              <label>
                Equipment class
                <select disabled={props.readOnly} value={definition.equipment_class} onChange={(event) => props.onDefinition({ ...definition, equipment_class: event.target.value as EquipmentClass })}>
                  {equipmentClasses.map((item) => <option key={item} value={item}>{item}</option>)}
                </select>
              </label>
              <label>
                Functional role
                <select disabled={props.readOnly} value={definition.functional_role} onChange={(event) => props.onDefinition({ ...definition, functional_role: event.target.value as FunctionalRole })}>
                  {functionalRoles.map((item) => <option key={item} value={item}>{item}</option>)}
                </select>
              </label>
              <Field label="Category code" value={definition.category_code} disabled={props.readOnly} onChange={(category_code) => props.onDefinition({ ...definition, category_code })} />
              <Field label="Signal domains" value={definition.signal_domains.join(", ")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...definition, signal_domains: splitTokens(value) as EquipmentModelDefinition["signal_domains"] })} />
              <Field label="Technology tags" value={(definition.technology_tags ?? []).join(", ")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...definition, technology_tags: splitTokens(value) as EquipmentModelDefinition["technology_tags"] })} />
            </EditorCard>
          )}
          {props.section === "specifications" && (
            <EditorCard title="Specifications typées">
              <StructuredTable columns={["ID", "Label", "Quantity", "Unit", "Min", "Max"]}>
                {definition.specifications.map((spec, index) => (
                  <tr key={spec.specification_id}>
                    <td>{spec.specification_id}</td>
                    <td><input disabled={props.readOnly} value={spec.label} onChange={(event) => props.onDefinition({ ...definition, specifications: replaceAt(definition.specifications, index, { ...spec, label: event.target.value }) })} /></td>
                    <td>{spec.quantity}</td>
                    <td><input disabled={props.readOnly} value={spec.unit} onChange={(event) => props.onDefinition({ ...definition, specifications: replaceAt(definition.specifications, index, { ...spec, unit: event.target.value }) })} /></td>
                    <td>{spec.minimum ?? "-"}</td>
                    <td>{spec.maximum ?? "-"}</td>
                  </tr>
                ))}
              </StructuredTable>
              <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, specifications: [...definition.specifications, defaultSpecification(definition.specifications.length + 1)] })}>Ajouter une specification</button>
            </EditorCard>
          )}
          {props.section === "ports" && (
            <EditorCard title="Physical Ports">
              <StructuredTable columns={["ID", "Label", "Directionality", "Flow role", "Domain", "Quantity", "Unit"]}>
                {definition.signal_ports.map((port) => (
                  <tr key={port.port_id}><td>{port.port_id}</td><td>{port.label}</td><td>{port.directionality}</td><td>{port.flow_role}</td><td>{port.signal_domain}</td><td>{port.quantity}</td><td>{port.unit}</td></tr>
                ))}
              </StructuredTable>
              <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, signal_ports: [...definition.signal_ports, defaultPort(definition.signal_ports.length + 1)] })}>Ajouter un port RF</button>
            </EditorCard>
          )}
          {props.section === "interfaces" && (
            <EditorCard title="Communication Interfaces">
              <StructuredTable columns={["ID", "Transport", "Provider", "Protocol", "Default"]}>
                {definition.communication_interfaces.map((item) => (
                  <tr key={item.interface_id}><td>{item.interface_id}</td><td>{item.transport_kind}</td><td>{item.access_provider_kind}</td><td>{item.protocol_kind}</td><td>{item.default_interface ? "oui" : "non"}</td></tr>
                ))}
              </StructuredTable>
              <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, communication_interfaces: [...definition.communication_interfaces, defaultTcpInterface(definition.communication_interfaces.length + 1)] })}>Ajouter TCP SCPI</button>
              <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, communication_interfaces: [...definition.communication_interfaces, defaultCanInterface(definition.communication_interfaces.length + 1)] })}>Ajouter CAN bus simule</button>
            </EditorCard>
          )}
          {props.section === "capabilities" && (
            <EditorCard title="Capabilities">
              <StructuredTable columns={["ID", "Kind", "Safety", "Inputs", "Outputs"]}>
                {definition.capabilities.map((capability) => (
                  <tr key={capability.capability_id}><td>{capability.capability_id}</td><td>{capability.capability_kind}</td><td>{capability.safety_class}</td><td>{capability.inputs.length}</td><td>{capability.outputs.length}</td></tr>
                ))}
              </StructuredTable>
              <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, capabilities: [...definition.capabilities, defaultCapability(definition.capabilities.length + 1)] })}>Ajouter capability mesure</button>
            </EditorCard>
          )}
          {props.section === "revisions" && <RevisionTable revisions={props.revisions} onOpen={props.onOpenRevision} />}
          {props.section === "audit" && <AuditTable audit={props.audit} />}
          {props.section === "json" && (
            <EditorCard title="Advanced JSON">
              <textarea className="jsonPreview" value={props.jsonDraft} disabled={props.readOnly} onChange={(event) => props.onJsonDraft(event.target.value)} />
              <button disabled={props.readOnly} onClick={props.onApplyJson}>Appliquer JSON</button>
            </EditorCard>
          )}
        </div>
        <ValidationPanel validation={props.validation} />
      </div>
    </section>
  );
}

function DriverTree(props: {
  models: EquipmentModelAggregate[];
  drivers: DriverProfileAggregate[];
  selected: DriverProfileAggregate | null;
  onOpen: (driver: DriverProfileAggregate) => void;
}) {
  return (
    <aside className="equipmentList">
      <h2>Drivers and Actions</h2>
      {props.models.map((model) => {
        const modelDrivers = props.drivers.filter((driver) => driver.identity.equipment_model_id === model.identity.equipment_model_id);
        return (
          <div className="driverGroup" key={model.identity.equipment_model_id}>
            <strong>{model.identity.category_code}</strong>
            <span>{model.identity.manufacturer} {model.identity.model_name}</span>
            {modelDrivers.map((driver) => (
              <button key={driver.identity.driver_profile_id} className={props.selected?.identity.driver_profile_id === driver.identity.driver_profile_id ? "active" : ""} onClick={() => props.onOpen(driver)}>
                <span>{driver.identity.label}</span>
                <small>{driver.latest_revision?.status ?? "no_revision"} | {driver.latest_revision?.action_count ?? 0} actions</small>
              </button>
            ))}
          </div>
        );
      })}
    </aside>
  );
}

function DriverStudio(props: {
  driver: DriverProfileAggregate | null;
  revision: DriverProfileRevision | null;
  definition: DriverProfileDefinition | null;
  readOnly: boolean;
  section: DriverSection;
  revisions: DriverProfileRevision[];
  audit: EquipmentAuditEvent[];
  validation: EquipmentValidationResult | null;
  providers: CommunicationProviderStatus[];
  simulation: DriverSimulationResult | null;
  jsonDraft: string;
  onSection: (section: DriverSection) => void;
  onDefinition: (definition: DriverProfileDefinition) => void;
  onJsonDraft: (value: string) => void;
  onApplyJson: () => void;
  onValidate: () => void;
  onSave: () => void;
  onSubmit: () => void;
  onApprove: () => void;
  onDerive: () => void;
  onSimulate: (action: DriverActionDefinition) => void;
  onOpenRevision: (revision: DriverProfileRevision) => void;
}) {
  if (!props.driver || !props.revision || !props.definition) {
    return <StateBlock title="Aucun driver ouvert" detail="Creez un driver depuis un modele approuve ou selectionnez un driver existant." />;
  }
  const definition = props.definition;
  const firstAction = definition.actions[0] ?? null;
  return (
    <section className="equipmentStudio">
      <div className="studioHeader">
        <div>
          <p className="eyebrow">Driver Profile</p>
          <h2>{props.driver.identity.label}</h2>
          <p className="mono">{props.revision.revision_id} | {props.revision.status}</p>
        </div>
        <div className="headerActions">
          <button onClick={props.onValidate}><CheckCircle2 size={16} /> Valider</button>
          <button onClick={props.onSave} disabled={props.readOnly}><Save size={16} /> Sauvegarder</button>
          <button onClick={props.onSubmit} disabled={props.readOnly || props.revision.status !== "draft"}><Send size={16} /> Soumettre</button>
          <button onClick={props.onApprove} disabled={props.revision.status !== "under_review"}><ShieldCheck size={16} /> Approuver</button>
          <button onClick={props.onDerive} disabled={!props.driver.current_approved_revision}><GitBranch size={16} /> Nouvelle revision</button>
          <button onClick={() => firstAction && props.onSimulate(firstAction)} disabled={!firstAction}><Play size={16} /> Simuler</button>
        </div>
      </div>
      <div className="studioLayout equipmentStudioLayout">
        <nav className="sectionNav">
          {driverSections.map(([key, label]) => (
            <button key={key} className={props.section === key ? "active" : ""} onClick={() => props.onSection(key)}>{label}</button>
          ))}
        </nav>
        <div className="editorPane">
          {props.section === "general" && (
            <EditorCard title="Compatibility">
              <dl>
                <dt>Model</dt><dd>{definition.equipment_model_id}</dd>
                <dt>Model revision</dt><dd>{definition.supported_model_revision_id}</dd>
                <dt>Checksum</dt><dd><code>{definition.supported_model_definition_checksum}</code></dd>
                <dt>Interfaces</dt><dd>{definition.communication_profiles.join(", ") || "-"}</dd>
              </dl>
              <ProviderList providers={props.providers} />
            </EditorCard>
          )}
          {props.section === "actions" && (
            <EditorCard title="Actions">
              <StructuredTable columns={["ID", "Capability", "Safety", "Inputs", "Outputs", ""]}>
                {definition.actions.map((action) => (
                  <tr key={action.action_id}>
                    <td>{action.action_id}</td><td>{action.implements_capability_id}</td><td>{action.safety_class}</td><td>{action.inputs.length}</td><td>{action.outputs.length}</td>
                    <td><button onClick={() => props.onSimulate(action)}><Play size={14} /> Simuler</button></td>
                  </tr>
                ))}
              </StructuredTable>
              <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, actions: [...definition.actions, defaultAction(definition.actions.length + 1, definition.communication_profiles[0] ?? "tcp")] })}>Ajouter action</button>
            </EditorCard>
          )}
          {props.section === "script" && firstAction && (
            <EditorCard title={`Script AST - ${firstAction.action_id}`}>
              <ScriptSteps steps={firstAction.script.steps} />
              <div className="buttonRow">
                <button disabled={props.readOnly} onClick={() => props.onDefinition(replaceFirstAction(definition, { ...firstAction, script: { steps: [...firstAction.script.steps, defaultIoQueryStep(firstAction.script.steps.length + 1, definition.communication_profiles[0] ?? "tcp")] } }))}>Ajouter QUERY</button>
                <button disabled={props.readOnly} onClick={() => props.onDefinition(replaceFirstAction(definition, { ...firstAction, script: { steps: [...firstAction.script.steps, defaultCanStep(firstAction.script.steps.length + 1, definition.communication_profiles[0] ?? "can_bus")] } }))}>Ajouter CAN bus</button>
                <button disabled={props.readOnly} onClick={() => props.onDefinition(replaceFirstAction(definition, { ...firstAction, script: { steps: [...firstAction.script.steps, { step_id: `assert_${firstAction.script.steps.length + 1}`, step_type: "assert", expression: "${result.power_dbm} > -200" }] } }))}>Ajouter ASSERT</button>
              </div>
              <pre className="scriptPreview">{textualScript(firstAction.script.steps)}</pre>
            </EditorCard>
          )}
          {props.section === "simulation" && (
            <EditorCard title="Driver Test Console">
              {!props.simulation && <p>Aucune simulation executee.</p>}
              {props.simulation && (
                <>
                  <dl><dt>Status</dt><dd>{props.simulation.status}</dd><dt>Duree virtuelle</dt><dd>{props.simulation.virtual_duration_ms} ms</dd></dl>
                  <StructuredTable columns={["Step", "Type", "Operation", "Request", "Response", "Status"]}>
                    {props.simulation.trace.map((trace, index) => (
                      <tr key={index}><td>{String(trace.step_index ?? index)}</td><td>{String(trace.step_type ?? "-")}</td><td>{String(trace.operation ?? "-")}</td><td>{JSON.stringify(trace.request ?? "")}</td><td>{JSON.stringify(trace.response ?? "")}</td><td>{String(trace.status ?? "-")}</td></tr>
                    ))}
                  </StructuredTable>
                  <pre className="scriptPreview">{JSON.stringify(props.simulation.outputs, null, 2)}</pre>
                </>
              )}
            </EditorCard>
          )}
          {props.section === "revisions" && <RevisionTable revisions={props.revisions} onOpen={props.onOpenRevision} />}
          {props.section === "audit" && <AuditTable audit={props.audit} />}
          {props.section === "json" && (
            <EditorCard title="Advanced JSON">
              <textarea className="jsonPreview" value={props.jsonDraft} disabled={props.readOnly} onChange={(event) => props.onJsonDraft(event.target.value)} />
              <button disabled={props.readOnly} onClick={props.onApplyJson}>Appliquer JSON</button>
            </EditorCard>
          )}
        </div>
        <ValidationPanel validation={props.validation} />
      </div>
    </section>
  );
}

function ProviderList(props: { providers: CommunicationProviderStatus[] }) {
  return (
    <div className="providerGrid">
      {props.providers.map((provider) => (
        <span key={provider.provider} className={provider.available ? "provider ok" : "provider unavailable"}>
          {provider.provider}: {provider.available ? "available" : provider.reason ?? "not installed"}
        </span>
      ))}
    </div>
  );
}

function ValidationPanel(props: { validation: EquipmentValidationResult | null }) {
  return (
    <aside className="validationPanel">
      <h2>Validation</h2>
      {!props.validation && <p>Aucun verdict serveur courant.</p>}
      {props.validation?.valid && <p className="validationOk"><CheckCircle2 size={16} /> Definition valide</p>}
      {props.validation && !props.validation.valid && (
        <ul>
          {props.validation.issues.map((issue) => (
            <li key={`${issue.code}-${issue.path}`}><strong>{issue.code}</strong><span>{issue.path}</span><p>{issue.message}</p></li>
          ))}
        </ul>
      )}
    </aside>
  );
}

function RevisionTable<T extends EquipmentModelRevision | DriverProfileRevision>(props: {
  revisions: T[];
  onOpen: (revision: T) => void;
}) {
  return (
    <EditorCard title="Revisions">
      <StructuredTable columns={["No", "Revision", "Status", "Parent", "Checksum", "Created", "Submitted", "Approved", ""]}>
        {props.revisions.map((revision) => (
          <tr key={revision.revision_id}>
            <td>{revision.revision_number}</td><td className="mono">{revision.revision_id}</td><td>{revision.status}</td><td>{revision.parent_revision_id ?? "-"}</td><td><code>{revision.definition_checksum}</code></td><td>{formatDate(revision.created_at)}</td><td>{formatDate(revision.submitted_at)}</td><td>{formatDate(revision.approved_at)}</td><td><button onClick={() => props.onOpen(revision)}>Ouvrir</button></td>
          </tr>
        ))}
      </StructuredTable>
    </EditorCard>
  );
}

function AuditTable(props: { audit: EquipmentAuditEvent[] }) {
  return (
    <EditorCard title="Audit">
      <StructuredTable columns={["Date", "Action", "Actor", "Reason", "Operation", "Checksum"]}>
        {props.audit.map((event) => (
          <tr key={event.audit_id}><td>{formatDate(event.occurred_at)}</td><td>{event.action}</td><td>{event.actor}</td><td>{event.reason}</td><td>{event.operation_id}</td><td><code>{event.new_definition_checksum ?? "-"}</code></td></tr>
        ))}
      </StructuredTable>
    </EditorCard>
  );
}

function ScriptSteps(props: { steps: DriverScriptStep[] }) {
  return (
    <StructuredTable columns={["ID", "Type", "Interface", "Payload", "Binding", "Expression"]}>
      {props.steps.map((step) => (
        <tr key={step.step_id}><td>{step.step_id}</td><td>{step.step_type}</td><td>{step.interface_id ?? "-"}</td><td>{step.payload ?? "-"}</td><td>{step.response_binding ?? step.variable ?? "-"}</td><td>{step.expression ?? "-"}</td></tr>
      ))}
    </StructuredTable>
  );
}

function EditorCard(props: { title: string; children: React.ReactNode }) {
  return <section className="editorCard"><h2>{props.title}</h2>{props.children}</section>;
}

function StructuredTable(props: { columns: string[]; children: React.ReactNode }) {
  return (
    <div className="tableWrap"><table><thead><tr>{props.columns.map((column) => <th key={column}>{column}</th>)}</tr></thead><tbody>{props.children}</tbody></table></div>
  );
}

function Field(props: { label: string; value: string; disabled?: boolean; onChange: (value: string) => void }) {
  return <label>{props.label}<input value={props.value} disabled={props.disabled} onChange={(event) => props.onChange(event.target.value)} /></label>;
}

function StateBlock(props: { title: string; detail: string }) {
  return <div className="stateBlock"><h2>{props.title}</h2><p>{props.detail}</p></div>;
}

function defaultEquipmentModelDefinition(id: string): EquipmentModelDefinition {
  return {
    definition_schema_version: "emc-locus.equipment-model-definition.v2",
    manufacturer: "Demo Instruments",
    model_name: id,
    equipment_class: "controllable_instrument",
    functional_role: "measurement_instrument",
    category_code: "power_meter",
    signal_domains: ["rf", "ethernet"],
    technology_tags: ["rf_50_ohm", "ethernet", "raw_tcp", "scpi"],
    specifications: [defaultSpecification(1)],
    signal_ports: [defaultPort(1)],
    communication_interfaces: [defaultTcpInterface(1)],
    capabilities: [defaultCapability(1)],
    metadata: { created_from: "lab_console" }
  };
}

function defaultDriverProfileDefinition(model: EquipmentModelAggregate): DriverProfileDefinition {
  const revision = model.current_approved_revision!;
  const interfaceId = revision.definition.communication_interfaces[0]?.interface_id ?? "tcp";
  const capability = revision.definition.capabilities[0]?.capability_id ?? "measure_power";
  return {
    definition_schema_version: "emc-locus.driver-profile-definition.v1",
    equipment_model_id: model.identity.equipment_model_id,
    supported_model_revision_id: revision.revision_id,
    supported_model_definition_checksum: revision.definition_checksum,
    supported_firmware_ranges: ["*"],
    communication_profiles: [interfaceId],
    actions: [defaultAction(1, interfaceId, capability)],
    metadata: { created_from: "lab_console" }
  };
}

function defaultSpecification(index: number) {
  return {
    specification_id: `frequency_range_${index}`,
    label: "Frequency range",
    quantity: "frequency" as const,
    unit: "Hz",
    minimum: 9000,
    maximum: 1000000000
  };
}

function defaultPort(index: number) {
  return {
    port_id: `rf_input_${index}`,
    label: `RF Input ${index}`,
    directionality: "input" as const,
    flow_role: "measurement_port" as const,
    signal_domain: "rf" as const,
    connector_type: "N",
    quantity: "power" as const,
    unit: "dBm",
    impedance: 50,
    frequency_min: 9000,
    frequency_max: 1000000000,
    power_max: 30
  };
}

function defaultTcpInterface(index: number) {
  return {
    interface_id: `tcp_${index}`,
    label: `SCPI TCP ${index}`,
    transport_kind: "ethernet_tcp" as const,
    access_provider_kind: "native_tcp" as const,
    protocol_kind: "scpi" as const,
    required: true,
    default_interface: index === 1,
    configuration_schema: { host: { type: "text" }, port: { type: "integer" } },
    default_configuration: { host: "127.0.0.1", port: 5025 },
    framing: "lf"
  };
}

function defaultCanInterface(index: number) {
  return {
    interface_id: `can_${index}`,
    label: `CAN bus ${index}`,
    transport_kind: "can_bus" as const,
    access_provider_kind: "simulation" as const,
    protocol_kind: "can_bus_frames" as const,
    required: false,
    default_interface: false,
    configuration_schema: { bitrate: { type: "integer" } },
    default_configuration: { bitrate: 500000 }
  };
}

function defaultCapability(index: number) {
  return {
    capability_id: `measure_power_${index}`,
    label: "Measure power",
    description: "Measure RF power.",
    capability_kind: "measure_power",
    inputs: [],
    outputs: [
      {
        name: "power_dbm",
        value_type: "number" as const,
        quantity: "power" as const,
        unit: "dBm",
        required: true
      }
    ],
    required_signal_ports: [],
    safety_class: "read_only" as const
  };
}

function defaultAction(index: number, interfaceId: string, capabilityId = "measure_power_1"): DriverActionDefinition {
  return {
    action_id: `measure_power_${index}`,
    label: "Measure power",
    description: "Query power measurement.",
    implements_capability_id: capabilityId,
    inputs: [],
    outputs: [
      {
        name: "power_dbm",
        value_type: "number",
        quantity: "power",
        unit: "dBm",
        required: true
      }
    ],
    safety_class: "read_only",
    default_timeout_ms: 1000,
    script: { steps: [defaultIoQueryStep(1, interfaceId), { step_id: "return_1", step_type: "return" }] },
    safe_to_retry: true,
    idempotent: true
  };
}

function defaultIoQueryStep(index: number, interfaceId: string): DriverScriptStep {
  return {
    step_id: `query_${index}`,
    step_type: "io_query",
    interface_id: interfaceId,
    payload_format: "text",
    payload: "MEAS:POW?",
    response_binding: "${result.power_dbm}",
    timeout_ms: 1000
  };
}

function defaultCanStep(index: number, interfaceId: string): DriverScriptStep {
  return {
    step_id: `can_bus_request_${index}`,
    step_type: "can_bus_request_response",
    interface_id: interfaceId,
    frame: { arbitration_id: 0x120, extended: false, data: [1, 0], dlc: 2 },
    response_binding: "${state.can_bus_response}",
    timeout_ms: 100
  };
}

function replaceFirstAction(definition: DriverProfileDefinition, action: DriverActionDefinition): DriverProfileDefinition {
  return { ...definition, actions: definition.actions.map((item, index) => (index === 0 ? action : item)) };
}

function textualScript(steps: DriverScriptStep[]) {
  return steps.map((step) => `${step.step_type.toUpperCase()} ${step.payload ?? step.expression ?? step.response_binding ?? ""}`).join("\n");
}

function replaceAt<T>(items: T[], index: number, value: T) {
  return items.map((item, itemIndex) => (itemIndex === index ? value : item));
}

function optionalString(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

function splitTokens(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function formatDate(value?: string | null) {
  return value ? value.replace("T", " ").replace("Z", "") : "-";
}

function errorMessage(error: unknown) {
  if (error instanceof ApiError) {
    return `${error.code}: ${error.message}`;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
