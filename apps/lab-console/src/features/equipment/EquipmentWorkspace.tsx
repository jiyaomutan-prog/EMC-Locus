import {
  AlertTriangle,
  Boxes,
  ChevronDown,
  ChevronRight,
  CheckCircle2,
  Copy,
  Cpu,
  Folder,
  FolderOpen,
  GitBranch,
  MoreHorizontal,
  Plus,
  Play,
  RefreshCw,
  Save,
  Search,
  Send,
  Settings,
  SlidersHorizontal,
  ShieldCheck
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
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
  EquipmentCategory,
  EquipmentEffectiveTemplate,
  EquipmentFieldDataType,
  EquipmentFieldDefinition,
  EquipmentClass,
  EquipmentRegistries,
  EquipmentModelAggregate,
  EquipmentModelDefinition,
  EquipmentModelRevision,
  EquipmentValidationResult,
  FunctionalRole,
  SignalDomain,
  TechnologyTag
} from "../../models/equipment";
import { MeasurementEngineeringPanel } from "./MeasurementEngineeringPanel";

type EquipmentSpace =
  | "admin"
  | "catalog"
  | "drivers"
  | "sensors"
  | "scaling"
  | "curves"
  | "daq"
  | "recipes";

type ModelSection =
  | "summary"
  | "identification"
  | "category_template"
  | "characteristics"
  | "ports_connections"
  | "measurement_corrections"
  | "control_drivers"
  | "documents"
  | "revisions_audit"
  | "advanced_diagnostics";

type DriverSection =
  | "general"
  | "actions"
  | "script"
  | "simulation"
  | "revisions"
  | "audit"
  | "json";

const measurementSpaces: Array<[EquipmentSpace, string]> = [
  ["sensors", "Capteurs / transducteurs"],
  ["scaling", "Profils de scaling"],
  ["curves", "Courbes d'ingénierie"],
  ["daq", "Voies DAQ"],
  ["recipes", "Recettes d'acquisition"]
];

const modelSections: Array<[ModelSection, string]> = [
  ["summary", "Synthese"],
  ["identification", "Identification"],
  ["category_template", "Categorie et formulaire"],
  ["characteristics", "Caracteristiques"],
  ["ports_connections", "Ports et connexions"],
  ["measurement_corrections", "Mesure / corrections"],
  ["control_drivers", "Pilotage / drivers"],
  ["documents", "Documents"],
  ["revisions_audit", "Revisions et audit"],
  ["advanced_diagnostics", "Diagnostic avance"]
];

const driverSections: Array<[DriverSection, string]> = [
  ["general", "General"],
  ["actions", "Actions"],
  ["script", "Script"],
  ["simulation", "Simulation"],
  ["revisions", "Revisions"],
  ["audit", "Audit"],
  ["json", "Diagnostic avance"]
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
  const [registries, setRegistries] = useState<EquipmentRegistries | null>(null);
  const [categories, setCategories] = useState<EquipmentCategory[]>([]);
  const [categoryTree, setCategoryTree] = useState<EquipmentCategory[]>([]);
  const [fieldDefinitions, setFieldDefinitions] = useState<EquipmentFieldDefinition[]>([]);
  const [query, setQuery] = useState("");
  const [rootFilter, setRootFilter] = useState("all");
  const [categoryFilter, setCategoryFilter] = useState("all");
  const [demoMode, setDemoMode] = useState<"hide" | "show" | "only">("hide");
  const [classFilter, setClassFilter] = useState("all");
  const [roleFilter, setRoleFilter] = useState("all");
  const [domainFilter, setDomainFilter] = useState("all");
  const [tagFilter, setTagFilter] = useState("all");
  const [manufacturerFilter, setManufacturerFilter] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [loadState, setLoadState] = useState<"loading" | "ready" | "error">("loading");
  const [operationError, setOperationError] = useState<string | null>(null);
  const [creationOpen, setCreationOpen] = useState(false);
  const [selectedModel, setSelectedModel] = useState<EquipmentModelAggregate | null>(null);
  const [selectedModelRevision, setSelectedModelRevision] = useState<EquipmentModelRevision | null>(null);
  const [modelDefinition, setModelDefinition] = useState<EquipmentModelDefinition | null>(null);
  const [modelChecksum, setModelChecksum] = useState("");
  const [modelSection, setModelSection] = useState<ModelSection>("summary");
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

  const refresh = useCallback(async () => {
    setLoadState((current) => current === "ready" ? "ready" : "loading");
    setOperationError(null);
    try {
      const [modelList, driverList, providerList, categoryList, treeList, fieldsList] = await Promise.all([
        equipmentApi.listModels({
          q: query.trim(),
          manufacturer: manufacturerFilter.trim(),
          root_category_id: rootFilter,
          category_code: categoryFilter,
          demo_mode: demoMode,
          equipment_class: classFilter,
          functional_role: roleFilter,
          signal_domain: domainFilter,
          technology_tag: tagFilter,
          status: statusFilter
        }),
        equipmentApi.listDrivers(),
        equipmentApi.providers(),
        equipmentApi.listCategories(true),
        equipmentApi.categoryTree(true),
        equipmentApi.listFieldDefinitions("equipment_model", true)
      ]);
      const registryList = await equipmentApi.registries();
      setModels(modelList.equipment_models);
      setDrivers(driverList.driver_profiles);
      setProviders(providerList.providers);
      setCategories(categoryList.categories);
      setCategoryTree(treeList.categories);
      setFieldDefinitions(fieldsList.field_definitions);
      setRegistries(registryList);
      setLoadState("ready");
    } catch (error) {
      setLoadState("error");
      setOperationError(errorMessage(error));
    }
  }, [
    query,
    manufacturerFilter,
    rootFilter,
    categoryFilter,
    demoMode,
    classFilter,
    roleFilter,
    domainFilter,
    tagFilter,
    statusFilter
  ]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const approvedModels = models.filter((model) => model.current_approved_revision);
  const modelReadOnly = selectedModelRevision?.status !== "draft";
  const driverReadOnly = selectedDriverRevision?.status !== "draft";
  const measurementSpaceActive = measurementSpaces.some(([key]) => key === space);
  const activeCatalogFilterCount = [
    query.trim(),
    manufacturerFilter.trim(),
    rootFilter !== "all" ? rootFilter : "",
    categoryFilter !== "all" ? categoryFilter : "",
    demoMode !== "hide" ? demoMode : "",
    classFilter !== "all" ? classFilter : "",
    roleFilter !== "all" ? roleFilter : "",
    domainFilter !== "all" ? domainFilter : "",
    tagFilter !== "all" ? tagFilter : "",
    statusFilter !== "all" ? statusFilter : ""
  ].filter(Boolean).length;

  function resetCatalogFilters() {
    setQuery("");
    setManufacturerFilter("");
    setRootFilter("all");
    setCategoryFilter("all");
    setDemoMode("hide");
    setClassFilter("all");
    setRoleFilter("all");
    setDomainFilter("all");
    setTagFilter("all");
    setStatusFilter("all");
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
      setModelSection("summary");
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

  async function createModelFromTemplate(
    categoryId: string,
    fieldValues: Record<string, unknown>,
    equipmentModelId?: string
  ) {
    await runOperation(async () => {
      const result = await equipmentApi.createModelFromCategoryTemplate({
        category_id: categoryId,
        equipment_model_id: optionalString(equipmentModelId ?? ""),
        field_values: fieldValues,
        ...context
      });
      setCreationOpen(false);
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
        manufacturer: selectedModel.identity.manufacturer,
        model_name: `${selectedModel.identity.model_name} copy`,
        variant: selectedModel.identity.variant ?? undefined,
        ...context
      });
      setCreationOpen(false);
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
      <div className="equipmentNavigation">
        <nav className="equipmentTabs" aria-label="Navigation equipements">
          <button
            className={space === "catalog" ? "active" : ""}
            aria-current={space === "catalog" ? "page" : undefined}
            onClick={() => setSpace("catalog")}
          >
            <Boxes size={17} /> Catalogue équipements
          </button>
          <button
            className={measurementSpaceActive ? "active" : ""}
            aria-current={measurementSpaceActive ? "page" : undefined}
            onClick={() => setSpace("sensors")}
          >
            <Cpu size={17} /> Ingénierie de mesure
          </button>
          <button
            className={space === "drivers" ? "active" : ""}
            aria-current={space === "drivers" ? "page" : undefined}
            onClick={() => setSpace("drivers")}
          >
            <GitBranch size={17} /> Drivers et actions
          </button>
          <button
            className={"equipmentAdminTab" + (space === "admin" ? " active" : "")}
            aria-current={space === "admin" ? "page" : undefined}
            onClick={() => setSpace("admin")}
          >
            <Settings size={17} /> Administration du référentiel
          </button>
        </nav>
        {measurementSpaceActive && (
          <nav className="equipmentSubnav" aria-label="Domaines d'ingenierie de mesure">
            {measurementSpaces.map(([key, label]) => (
              <button key={key} className={space === key ? "active" : ""} onClick={() => setSpace(key)}>
                {label}
              </button>
            ))}
          </nav>
        )}
      </div>

      {space === "catalog" && (
        <div className="catalogCommandBar">
          <label className="searchBox catalogSearch">
            <Search size={16} />
            <input
              aria-label="Recherche equipement"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Modèle, fabricant ou catégorie"
            />
          </label>
          <select aria-label="Filtre categorie racine" value={rootFilter} onChange={(event) => setRootFilter(event.target.value)}>
            <option value="all">Toutes les familles</option>
            {categoryTree.map((category) => (
              <option key={category.category_id} value={category.category_id}>{category.label}</option>
            ))}
          </select>
          <select aria-label="Filtre statut" value={statusFilter} onChange={(event) => setStatusFilter(event.target.value)}>
            <option value="all">Tous les statuts</option>
            <option value="draft">{humanStatus("draft")}</option>
            <option value="under_review">{humanStatus("under_review")}</option>
            <option value="approved">{humanStatus("approved")}</option>
            <option value="superseded">{humanStatus("superseded")}</option>
          </select>
          <details className="advancedFilters">
            <summary>
              <SlidersHorizontal size={15} />
              Filtres{activeCatalogFilterCount > 0 ? " (" + activeCatalogFilterCount + ")" : ""}
            </summary>
            <div className="filterPopover">
              <label>
                Fabricant
                <input
                  aria-label="Filtre fabricant"
                  value={manufacturerFilter}
                  onChange={(event) => setManufacturerFilter(event.target.value)}
                  placeholder="Tous les fabricants"
                />
              </label>
              <label>
                Sous-categorie
                <select aria-label="Filtre sous categorie" value={categoryFilter} onChange={(event) => setCategoryFilter(event.target.value)}>
                  <option value="all">Toutes les sous-categories</option>
                  {categories.filter((category) => category.parent_category_id).map((category) => (
                    <option key={category.category_id} value={category.category_id}>{categoryPathLabel(categories, category.category_id)}</option>
                  ))}
                </select>
              </label>
              <label>
                Donnees de demonstration
                <select aria-label="Filtre donnees demo" value={demoMode} onChange={(event) => setDemoMode(event.target.value as typeof demoMode)}>
                  <option value="hide">Masquer</option>
                  <option value="show">Afficher</option>
                  <option value="only">Demonstration uniquement</option>
                </select>
              </label>
              <label>
                Classe d'equipement
                <select aria-label="Filtre classe equipement" value={classFilter} onChange={(event) => setClassFilter(event.target.value)}>
                  <option value="all">Toutes les classes</option>
                  {equipmentClasses.map((item) => <option value={item} key={item}>{humanLabel(item)}</option>)}
                </select>
              </label>
              <label>
                Role physique
                <select aria-label="Filtre role physique" value={roleFilter} onChange={(event) => setRoleFilter(event.target.value)}>
                  <option value="all">Tous les roles</option>
                  {(registries?.functional_roles ?? functionalRoles.map((code) => ({ code, label: humanLabel(code) }))).map((item) => (
                    <option value={item.code} key={item.code}>{item.label || humanLabel(item.code)}</option>
                  ))}
                </select>
              </label>
              <label>
                Domaine de signal
                <select aria-label="Filtre domaine signal" value={domainFilter} onChange={(event) => setDomainFilter(event.target.value)}>
                  <option value="all">Tous les domaines</option>
                  {(registries?.signal_domains ?? []).map((item) => (
                    <option value={item.code} key={item.code}>{item.label || humanLabel(item.code)}</option>
                  ))}
                </select>
              </label>
              <label>
                Technologie
                <select aria-label="Filtre technologie" value={tagFilter} onChange={(event) => setTagFilter(event.target.value)}>
                  <option value="all">Toutes les technologies</option>
                  {(registries?.technology_tags ?? []).map((item) => (
                    <option value={item.code} key={item.code}>{item.label || humanLabel(item.code)}</option>
                  ))}
                </select>
              </label>
            </div>
          </details>
          {activeCatalogFilterCount > 0 && (
            <button className="textButton" type="button" onClick={resetCatalogFilters}>Effacer les filtres</button>
          )}
          <span className="commandBarSpacer" />
          <button className="iconButton secondary" onClick={() => void refresh()} title="Rafraichir le catalogue" aria-label="Rafraichir le catalogue">
            <RefreshCw size={16} />
          </button>
          <button onClick={() => setCreationOpen(true)}>
            <Plus size={16} /> Nouveau modèle
          </button>
        </div>
      )}

      {space === "drivers" && (
        <div className="contextCommandBar">
          <strong>Profils de pilotage</strong>
          <span className="commandBarSpacer" />
          <button className="iconButton secondary" onClick={() => void refresh()} title="Rafraichir les drivers" aria-label="Rafraichir les drivers">
            <RefreshCw size={16} />
          </button>
          <button onClick={() => void createDriver()} disabled={approvedModels.length === 0}>
            <Plus size={16} /> Nouveau driver
          </button>
        </div>
      )}

      {operationError && (
        <div className="conflictBox">
          <AlertTriangle size={18} />
          <div>
            <strong>Operation refusee</strong>
            <p>{operationError}</p>
          </div>
        </div>
      )}

      {creationOpen && (
        <div className="modalBackdrop">
          <EquipmentModelWizard
            roots={categoryTree}
            categories={categories}
            onCancel={() => setCreationOpen(false)}
            onCreate={(categoryId, values, modelId) => void createModelFromTemplate(categoryId, values, modelId)}
          />
        </div>
      )}

      {loadState === "loading" && <StateBlock title="Chargement" detail="Lecture du catalogue equipement." />}
      {loadState === "error" && <StateBlock title="Erreur" detail={operationError ?? "Catalogue indisponible."} />}

      {space === "admin" && loadState === "ready" && (
        <EquipmentRepositoryAdmin
          categories={categories}
          categoryTree={categoryTree}
          fieldDefinitions={fieldDefinitions}
          onRefresh={() => void refresh()}
          onError={setOperationError}
        />
      )}

      {space === "catalog" && loadState === "ready" && (
        <div className="equipmentLayout">
          <ModelCatalog
            models={models}
            selected={selectedModel}
            categories={categories}
            categoryTree={categoryTree}
            demoMode={demoMode}
            onCategory={(categoryId) => {
              const category = categories.find((item) => item.category_id === categoryId);
              if (category?.parent_category_id) {
                setCategoryFilter(categoryId);
                setRootFilter("all");
              } else {
                setRootFilter(categoryId);
                setCategoryFilter("all");
              }
            }}
            onOpen={(model) => void openModel(model)}
          />
          <ModelStudio
            model={selectedModel}
            revision={selectedModelRevision}
            definition={modelDefinition}
            readOnly={modelReadOnly}
            registries={registries}
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

      {["sensors", "scaling", "curves", "daq", "recipes"].includes(space) && (
        <MeasurementEngineeringPanel
          initialSpace={
            space === "sensors"
              ? "sensors"
              : space === "scaling"
                ? "scaling"
                : space === "curves"
                  ? "curves"
                  : space === "daq"
                    ? "daq"
                    : "recipes"
          }
        />
      )}

    </section>
  );
}

function EquipmentRepositoryAdmin(props: {
  categories: EquipmentCategory[];
  categoryTree: EquipmentCategory[];
  fieldDefinitions: EquipmentFieldDefinition[];
  onRefresh: () => void;
  onError: (message: string | null) => void;
}) {
  type AdminTab = "information" | "children" | "form" | "preview" | "diagnostics";
  type CategoryAction = "add_child" | "rename" | "move" | "archive" | "edit_form" | "preview" | "diagnostics";
  const [selectedCategoryId, setSelectedCategoryId] = useState(() => props.categoryTree[0]?.category_id ?? "");
  const [adminTab, setAdminTab] = useState<AdminTab>("information");
  const [newCategoryLabel, setNewCategoryLabel] = useState("");
  const [newCategoryDescription, setNewCategoryDescription] = useState("");
  const [categoryCodeOverride, setCategoryCodeOverride] = useState("");
  const [editCategoryLabel, setEditCategoryLabel] = useState("");
  const [editCategoryDescription, setEditCategoryDescription] = useState("");
  const [moveParentId, setMoveParentId] = useState("");
  const [newFieldLabel, setNewFieldLabel] = useState("");
  const [newFieldDescription, setNewFieldDescription] = useState("");
  const [newFieldType, setNewFieldType] = useState<EquipmentFieldDataType>("choice");
  const [fieldCodeOverride, setFieldCodeOverride] = useState("");
  const [newFieldChoices, setNewFieldChoices] = useState(["Faible", "Normale", "Haute", "Critique"]);
  const [newChoiceValue, setNewChoiceValue] = useState("");
  const [newFieldUnits, setNewFieldUnits] = useState(["Hz", "kHz", "MHz", "GHz"]);
  const [newUnitValue, setNewUnitValue] = useState("");
  const [newFieldGroup, setNewFieldGroup] = useState("Identification");
  const [selectedFieldId, setSelectedFieldId] = useState("");
  const [fieldRequired, setFieldRequired] = useState(false);
  const [fieldVisible, setFieldVisible] = useState(true);
  const [template, setTemplate] = useState<EquipmentEffectiveTemplate | null>(null);
  const [directRules, setDirectRules] = useState<string[]>([]);
  const selectedCategory = props.categories.find((category) => category.category_id === selectedCategoryId) ?? props.categories[0];
  const selectedChildren = props.categories.filter((category) => category.parent_category_id === selectedCategory?.category_id);
  const generatedCategoryId = uniqueGeneratedCode(
    categoryCodeOverride || newCategoryLabel,
    new Set(props.categories.map((category) => category.category_id))
  );
  const generatedFieldCode = uniqueGeneratedCode(
    fieldCodeOverride || newFieldLabel,
    new Set(props.fieldDefinitions.map((field) => field.field_code))
  );
  const selectedCategoryDescendants = selectedCategory ? descendantCategoryIds(props.categories, selectedCategory.category_id) : new Set<string>();
  const movableParents = props.categories.filter((category) =>
    selectedCategory
      && category.category_id !== selectedCategory.category_id
      && !descendantCategoryIds(props.categories, selectedCategory.category_id).has(category.category_id)
  );
  const onError = props.onError;

  useEffect(() => {
    if (!selectedCategoryId && props.categories.length > 0) {
      setSelectedCategoryId(props.categories[0].category_id);
    }
  }, [selectedCategoryId, props.categories]);

  useEffect(() => {
    if (!selectedCategory) return;
    setEditCategoryLabel(selectedCategory.label);
    setEditCategoryDescription(selectedCategory.description ?? "");
    setMoveParentId(selectedCategory.parent_category_id ?? "");
  }, [selectedCategory]);

  useEffect(() => {
    if (!selectedCategoryId) return;
    let cancelled = false;
    Promise.all([
      equipmentApi.effectiveTemplate(selectedCategoryId),
      equipmentApi.categoryFieldRules(selectedCategoryId)
    ])
      .then((result) => {
        if (cancelled) return;
        setTemplate(result[0].effective_template);
        setDirectRules(result[1].rules.map((rule) => rule.field_id));
      })
      .catch((error) => onError(errorMessage(error)));
    return () => {
      cancelled = true;
    };
  }, [selectedCategoryId, onError]);

  function selectCategory(categoryId: string) {
    setSelectedCategoryId(categoryId);
    props.onError(null);
  }

  function handleCategoryAction(categoryId: string, action: CategoryAction) {
    selectCategory(categoryId);
    if (action === "add_child") setAdminTab("children");
    if (action === "rename" || action === "move" || action === "archive") setAdminTab("information");
    if (action === "edit_form") setAdminTab("form");
    if (action === "preview") setAdminTab("preview");
    if (action === "diagnostics") setAdminTab("diagnostics");
  }

  async function createSubcategory() {
    if (!selectedCategory || !newCategoryLabel.trim()) return;
    try {
      await equipmentApi.createCategory({
        category_id: generatedCategoryId,
        parent_category_id: selectedCategory.category_id,
        label: newCategoryLabel.trim(),
        description: newCategoryDescription.trim(),
        sort_order: 100
      });
      setNewCategoryLabel("");
      setNewCategoryDescription("");
      setCategoryCodeOverride("");
      props.onRefresh();
    } catch (error) {
      props.onError(errorMessage(error));
    }
  }

  async function createField() {
    if (!newFieldLabel.trim()) return;
    try {
      const result = await equipmentApi.createFieldDefinition({
        field_code: generatedFieldCode,
        label: newFieldLabel.trim(),
        description: newFieldDescription.trim(),
        data_type: newFieldType,
        scope: "equipment_model",
        required_by_default: false,
        visible_by_default: true,
        unique_value: false,
        option_values: newFieldType === "choice" || newFieldType === "multi_choice" ? newFieldChoices.filter(Boolean) : [],
        allowed_units: newFieldType === "number_with_unit" ? newFieldUnits.filter(Boolean) : [],
        display_group: newFieldGroup,
        display_order: 650,
        active: true
      });
      setSelectedFieldId(result.field_definition.field_id);
      setNewFieldLabel("");
      setNewFieldDescription("");
      setFieldCodeOverride("");
      props.onRefresh();
    } catch (error) {
      props.onError(errorMessage(error));
    }
  }

  async function addFieldToTemplate() {
    if (!selectedCategory || !selectedFieldId) return;
    try {
      const current = await equipmentApi.categoryFieldRules(selectedCategory.category_id);
      const existing = current.rules.filter((rule) => rule.field_id !== selectedFieldId);
      await equipmentApi.replaceCategoryFieldRules(selectedCategory.category_id, [
        ...existing,
        {
          category_id: selectedCategory.category_id,
          field_id: selectedFieldId,
          required: fieldRequired,
          visible: fieldVisible,
          display_group: newFieldGroup,
          display_order: 650
        }
      ]);
      const preview = await equipmentApi.effectiveTemplate(selectedCategory.category_id);
      setTemplate(preview.effective_template);
      props.onRefresh();
    } catch (error) {
      props.onError(errorMessage(error));
    }
  }

  async function updateSelectedCategory() {
    if (!selectedCategory) return;
    try {
      await equipmentApi.updateCategory(selectedCategory.category_id, {
        label: editCategoryLabel.trim(),
        description: editCategoryDescription.trim(),
        active: selectedCategory.active
      });
      props.onRefresh();
    } catch (error) {
      props.onError(errorMessage(error));
    }
  }

  async function moveSelectedCategory() {
    if (!selectedCategory || !moveParentId) return;
    try {
      await equipmentApi.moveCategory(selectedCategory.category_id, { parent_category_id: moveParentId });
      props.onRefresh();
    } catch (error) {
      props.onError(errorMessage(error));
    }
  }

  async function archiveSelectedCategory() {
    if (!selectedCategory) return;
    try {
      await equipmentApi.archiveCategory(selectedCategory.category_id);
      props.onRefresh();
    } catch (error) {
      props.onError(errorMessage(error));
    }
  }

  async function removeFieldFromTemplate(fieldId: string) {
    if (!selectedCategory) return;
    try {
      const current = await equipmentApi.categoryFieldRules(selectedCategory.category_id);
      await equipmentApi.replaceCategoryFieldRules(
        selectedCategory.category_id,
        current.rules.filter((rule) => rule.field_id !== fieldId)
      );
      const preview = await equipmentApi.effectiveTemplate(selectedCategory.category_id);
      const refreshedRules = await equipmentApi.categoryFieldRules(selectedCategory.category_id);
      setTemplate(preview.effective_template);
      setDirectRules(refreshedRules.rules.map((rule) => rule.field_id));
      props.onRefresh();
    } catch (error) {
      props.onError(errorMessage(error));
    }
  }

  function addChoiceValue() {
    if (!newChoiceValue.trim()) return;
    setNewFieldChoices((current) => [...current, newChoiceValue.trim()]);
    setNewChoiceValue("");
  }

  function addUnitValue() {
    if (!newUnitValue.trim()) return;
    setNewFieldUnits((current) => [...current, newUnitValue.trim()]);
    setNewUnitValue("");
  }

  return (
    <div className="equipmentLayout adminWorkbench">
      <aside className="equipmentList categoryPanel">
        <div className="listHeader">
          <h2>Catégories</h2>
          <span>{props.categories.length}</span>
        </div>
        <CategoryTree
          categories={props.categoryTree}
          selectedId={selectedCategoryId}
          onSelect={selectCategory}
          actions={[
            ["add_child", "Ajouter une sous-categorie"],
            ["rename", "Renommer"],
            ["move", "Deplacer"],
            ["archive", "Archiver"],
            ["edit_form", "Modifier le formulaire"],
            ["preview", "Previsualiser le formulaire"],
            ["diagnostics", "Diagnostic avance"]
          ]}
          onAction={(categoryId, action) => handleCategoryAction(categoryId, action as CategoryAction)}
        />
      </aside>
      <section className="equipmentStudio">
        <div className="studioHeader">
          <div>
            <p className="eyebrow">Administration du référentiel</p>
            <h2>{selectedCategory ? categoryPathLabel(props.categories, selectedCategory.category_id) : "Categorie"}</h2>
          </div>
          {selectedCategory && (
            <div className="headerActions">
              <button onClick={() => setAdminTab("children")}><Plus size={16} /> Ajouter une sous-categorie</button>
              <button onClick={() => setAdminTab("form")}><Settings size={16} /> Modifier le formulaire</button>
            </div>
          )}
        </div>
        <nav className="adminTabs" aria-label="Administration de la categorie">
          {[
            ["information", "Informations"],
            ["children", "Sous-categories"],
            ["form", "Formulaire"],
            ["preview", "Previsualisation"],
            ["diagnostics", "Diagnostic avance"]
          ].map(([key, label]) => (
            <button key={key} className={adminTab === key ? "active" : ""} onClick={() => setAdminTab(key as AdminTab)}>
              {label}
            </button>
          ))}
        </nav>

        {adminTab === "information" && selectedCategory && (
          <EditorCard title="Informations">
            <Field label="Nom de la categorie" value={editCategoryLabel} onChange={setEditCategoryLabel} />
            <Field label="Description" value={editCategoryDescription} onChange={setEditCategoryDescription} />
            <dl className="businessSummary">
              <dt>Categorie parente</dt><dd>{selectedCategory.parent_category_id ? categoryPathLabel(props.categories, selectedCategory.parent_category_id) : "Categorie racine systeme"}</dd>
              <dt>Etat</dt><dd>{selectedCategory.active ? "Active" : "Archivee"}</dd>
              <dt>Sous-categories</dt><dd>{selectedChildren.length}</dd>
            </dl>
            <div className="buttonRow">
              <button onClick={() => void updateSelectedCategory()}><Save size={16} /> Sauvegarder</button>
              {!selectedCategory.system_defined && (
                <>
                  <label>Nouvelle categorie parente
                    <select value={moveParentId} onChange={(event) => setMoveParentId(event.target.value)}>
                      <option value="">Choisir une categorie</option>
                      {movableParents.map((category) => (
                        <option key={category.category_id} value={category.category_id}>{categoryPathLabel(props.categories, category.category_id)}</option>
                      ))}
                    </select>
                  </label>
                  <button onClick={() => void moveSelectedCategory()}>Deplacer</button>
                  <button onClick={() => void archiveSelectedCategory()}>Archiver</button>
                </>
              )}
            </div>
          </EditorCard>
        )}

        {adminTab === "children" && selectedCategory && (
          <EditorCard title="Sous-categories">
            <div className="childList">
              {selectedChildren.length === 0 && <p>Aucune sous-categorie directe.</p>}
              {selectedChildren.map((category) => (
                <button key={category.category_id} onClick={() => selectCategory(category.category_id)}>
                  <Folder size={16} /> <span>{category.label}</span>
                </button>
              ))}
            </div>
            <div className="formGrid">
              <Field label="Nom de la sous-categorie" value={newCategoryLabel} onChange={setNewCategoryLabel} />
              <Field label="Description" value={newCategoryDescription} onChange={setNewCategoryDescription} />
            </div>
            <details className="advancedOptions">
              <summary>Options avancees</summary>
              <Field label="Identifiant interne" value={categoryCodeOverride || generatedCategoryId} onChange={setCategoryCodeOverride} />
            </details>
            <button onClick={() => void createSubcategory()} disabled={!newCategoryLabel.trim()}>
              <Plus size={16} /> Creer la sous-categorie
            </button>
          </EditorCard>
        )}

        {adminTab === "form" && selectedCategory && (
          <>
            <EditorCard title="Champs disponibles">
              <div className="formGrid">
                <Field label="Nom du champ" value={newFieldLabel} onChange={setNewFieldLabel} />
                <Field label="Description / aide" value={newFieldDescription} onChange={setNewFieldDescription} />
                <label>Type de champ
                  <select value={newFieldType} onChange={(event) => setNewFieldType(event.target.value as EquipmentFieldDataType)}>
                    {fieldTypes.map((type) => <option key={type} value={type}>{fieldTypeLabel(type)}</option>)}
                  </select>
                </label>
                <Field label="Groupe d'affichage" value={newFieldGroup} onChange={setNewFieldGroup} />
              </div>
              {(newFieldType === "choice" || newFieldType === "multi_choice") && (
                <ChoiceListEditor
                  title="Valeurs proposees"
                  values={newFieldChoices}
                  draft={newChoiceValue}
                  onDraft={setNewChoiceValue}
                  onAdd={addChoiceValue}
                  onRemove={(index) => setNewFieldChoices((current) => current.filter((_, itemIndex) => itemIndex !== index))}
                />
              )}
              {newFieldType === "number_with_unit" && (
                <ChoiceListEditor
                  title="Unites autorisees"
                  values={newFieldUnits}
                  draft={newUnitValue}
                  onDraft={setNewUnitValue}
                  onAdd={addUnitValue}
                  onRemove={(index) => setNewFieldUnits((current) => current.filter((_, itemIndex) => itemIndex !== index))}
                />
              )}
              <details className="advancedOptions">
                <summary>Options avancees</summary>
                <Field label="Nom technique genere" value={fieldCodeOverride || generatedFieldCode} onChange={setFieldCodeOverride} />
                <p className="hint">Le nom technique reste reserve au diagnostic, aux exports et aux API.</p>
              </details>
              <button onClick={() => void createField()} disabled={!newFieldLabel.trim()}><Plus size={16} /> Creer le champ</button>
            </EditorCard>
            <EditorCard title="Formulaire de la categorie">
              <div className="formGrid">
                <label>Champ du formulaire
                  <select value={selectedFieldId} onChange={(event) => setSelectedFieldId(event.target.value)}>
                    <option value="">Choisir un champ</option>
                    {props.fieldDefinitions.filter((field) => field.active).map((field) => (
                      <option value={field.field_id} key={field.field_id}>{field.label}</option>
                    ))}
                  </select>
                </label>
                <label>Groupe
                  <input value={newFieldGroup} onChange={(event) => setNewFieldGroup(event.target.value)} />
                </label>
                <label className="checkboxLine"><input type="checkbox" checked={fieldRequired} onChange={(event) => setFieldRequired(event.target.checked)} />Champ obligatoire</label>
                <label className="checkboxLine"><input type="checkbox" checked={fieldVisible} onChange={(event) => setFieldVisible(event.target.checked)} />Visible dans le formulaire</label>
              </div>
              <button onClick={() => void addFieldToTemplate()} disabled={!selectedFieldId}><Settings size={16} /> Ajouter au formulaire</button>
              <StructuredTable columns={["Champ", "Type", "Obligatoire", "Visible", "Groupe", "Origine", "Action"]}>
                {(template?.fields ?? []).map((field) => {
                  const direct = directRules.includes(field.field.field_id);
                  return (
                    <tr key={field.field.field_id}>
                      <td>{field.field.label}</td>
                      <td>{fieldTypeLabel(field.field.data_type)}</td>
                      <td>{field.required ? "Oui" : "Non"}</td>
                      <td>{field.visible ? "Oui" : "Non"}</td>
                      <td>{field.display_group}</td>
                      <td>{direct ? "Cette categorie" : "Herite"}</td>
                      <td>{direct && <button onClick={() => void removeFieldFromTemplate(field.field.field_id)}>Retirer</button>}</td>
                    </tr>
                  );
                })}
              </StructuredTable>
            </EditorCard>
          </>
        )}

        {adminTab === "preview" && (
          <EditorCard title="Previsualisation du formulaire">
            <p>Voici le formulaire que verra un technicien pour cette categorie.</p>
            <TemplatePreview template={template} />
          </EditorCard>
        )}

        {adminTab === "diagnostics" && selectedCategory && (
          <EditorCard title="Diagnostic avance">
            <dl className="businessSummary">
              <dt>Identifiant interne</dt><dd className="mono">{selectedCategory.category_id}</dd>
              <dt>Racine interne</dt><dd className="mono">{selectedCategory.root_category_id}</dd>
              <dt>Descendants</dt><dd>{selectedCategoryDescendants.size}</dd>
              <dt>Checksum du formulaire</dt><dd className="mono">{template?.template_checksum ?? "-"}</dd>
            </dl>
            <TemplatePreview template={template} showDiagnostics />
          </EditorCard>
        )}
      </section>
    </div>
  );
}

function EquipmentModelWizard(props: {
  roots: EquipmentCategory[];
  categories: EquipmentCategory[];
  onCancel: () => void;
  onCreate: (categoryId: string, values: Record<string, unknown>, modelId?: string) => void;
}) {
  const [step, setStep] = useState(1);
  const [rootId, setRootId] = useState("");
  const root = props.roots.find((item) => item.category_id === rootId);
  const [categoryId, setCategoryId] = useState("");
  const [template, setTemplate] = useState<EquipmentEffectiveTemplate | null>(null);
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [modelId, setModelId] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!rootId && props.roots.length > 0) setRootId(props.roots[0].category_id);
  }, [rootId, props.roots]);

  useEffect(() => {
    if (!categoryId) {
      setTemplate(null);
      return;
    }
    let cancelled = false;
    equipmentApi.effectiveTemplate(categoryId)
      .then((result) => {
        if (cancelled) return;
        setTemplate(result.effective_template);
        const defaults: Record<string, unknown> = {};
        result.effective_template.fields.forEach((field) => {
          if (field.default_value !== undefined && field.default_value !== null) {
            defaults[field.field.field_code] = field.default_value;
          }
        });
        setValues(defaults);
      })
      .catch((reason) => setError(errorMessage(reason)));
    return () => {
      cancelled = true;
    };
  }, [categoryId]);

  function create() {
    if (!template) return;
    const missing = template.fields.find((field) => field.visible && field.required && isEmptyField(values[field.field.field_code]));
    if (missing) {
      setError(`Le champ "${missing.field.label}" est obligatoire pour cette categorie.`);
      return;
    }
    props.onCreate(categoryId, values, optionalString(modelId));
  }

  function continueFromStep() {
    setError(null);
    if (step === 1 && !rootId) {
      setError("Choisissez une famille d'equipement.");
      return;
    }
    if (step === 2 && !categoryId) {
      setError("Choisissez une sous-categorie.");
      return;
    }
    if (step === 3) {
      const missing = template?.fields.find((field) => field.visible && field.required && isEmptyField(values[field.field.field_code]));
      if (missing) {
        setError(`Le champ "${missing.field.label}" est obligatoire pour cette categorie.`);
        return;
      }
    }
    setStep((current) => Math.min(4, current + 1));
  }

  const rootSubtree = root ? [root] : [];
  const visibleFields = template?.fields.filter((field) => field.visible) ?? [];
  const optionalFilled = visibleFields.filter((field) => !field.required && !isEmptyField(values[field.field.field_code])).length;
  const requiredComplete = visibleFields.every((field) => !field.required || !isEmptyField(values[field.field.field_code]));

  return (
    <div className="creationPanel wizardPanel" role="dialog" aria-modal="true" aria-labelledby="equipment-wizard-title">
      <div className="creationHeader">
        <div>
          <p className="eyebrow">Assistant de creation</p>
          <h2 id="equipment-wizard-title">Nouveau modèle équipement</h2>
        </div>
        <button className="secondary" onClick={props.onCancel}>Annuler</button>
      </div>
      <ol className="wizardStepper">
        {["Categorie", "Sous-categorie", "Identification", "Verification"].map((label, index) => {
          const number = index + 1;
          return <li key={label} className={step === number ? "active" : step > number ? "done" : ""}>Etape {number} - {label}</li>;
        })}
      </ol>
      {error && <p className="errorText">{error}</p>}
      <div className="wizardBody">
        {step === 1 && (
          <EditorCard title="Famille d'equipement">
            <div className="choiceList" role="radiogroup" aria-label="Famille d'equipement">
              {props.roots.map((category) => (
                <label key={category.category_id} className={rootId === category.category_id ? "choiceRow selected" : "choiceRow"}>
                  <input
                    type="radio"
                    name="equipment-root-category"
                    value={category.category_id}
                    checked={rootId === category.category_id}
                    onChange={() => {
                      setRootId(category.category_id);
                      setCategoryId("");
                      setTemplate(null);
                    }}
                  />
                  <span>{category.label}</span>
                </label>
              ))}
            </div>
          </EditorCard>
        )}
        {step === 2 && (
          <EditorCard title="Sous-categorie">
            <p>{root ? root.label : "Choisissez d'abord une famille d'equipement."}</p>
            {!categoryId && <p className="hint">Choisissez une sous-categorie.</p>}
            <CategoryTree categories={rootSubtree} selectedId={categoryId} onSelect={(id) => setCategoryId(id)} />
            {categoryId && <p className="selectedPath">Categorie : {categoryPathLabel(props.categories, categoryId)}</p>}
          </EditorCard>
        )}
        {step === 3 && (
          <EditorCard title="Identification">
            <p className="selectedPath">{categoryId ? categoryPathLabel(props.categories, categoryId) : ""}</p>
          {template?.fields.filter((field) => field.visible).map((field) => (
            <TemplateFieldInput
              key={field.field.field_id}
              field={field}
              value={values[field.field.field_code]}
              onChange={(value) => setValues((current) => ({ ...current, [field.field.field_code]: value }))}
            />
          ))}
          </EditorCard>
        )}
        {step === 4 && (
          <EditorCard title="Verification">
            <Field label="ID modele optionnel" value={modelId} onChange={setModelId} />
            <dl className="businessSummary">
              <dt>Categorie</dt><dd>{categoryId ? categoryPathLabel(props.categories, categoryId) : "-"}</dd>
              <dt>Champs obligatoires</dt><dd>{requiredComplete ? "Complets" : "Incomplets"}</dd>
              <dt>Champs optionnels renseignes</dt><dd>{optionalFilled}</dd>
            </dl>
            <TemplatePreview template={template} values={values} />
          </EditorCard>
        )}
      </div>
      <div className="buttonRow wizardFooter">
        <button className="secondary" onClick={() => setStep((current) => Math.max(1, current - 1))} disabled={step === 1}>Retour</button>
        {step < 4 && <button onClick={continueFromStep}>Continuer</button>}
        {step === 4 && <button onClick={create}><Cpu size={16} /> Creer brouillon</button>}
      </div>
    </div>
  );
}

function TemplateFieldInput(props: {
  field: EquipmentEffectiveTemplate["fields"][number];
  value: unknown;
  onChange: (value: unknown) => void;
}) {
  const field = props.field.field;
  const required = props.field.required ? " *" : "";
  if (field.data_type === "choice") {
    return <label>{field.label}{required}<select value={String(props.value ?? "")} onChange={(event) => props.onChange(event.target.value)}><option value="">-</option>{field.option_values.map((item) => <option key={item} value={item}>{item}</option>)}</select></label>;
  }
  if (field.data_type === "multi_choice") {
    return <label>{field.label}{required}<input value={Array.isArray(props.value) ? props.value.join(", ") : ""} onChange={(event) => props.onChange(splitTokens(event.target.value))} /></label>;
  }
  if (field.data_type === "number") {
    return <label>{field.label}{required}<input type="number" value={String(props.value ?? "")} onChange={(event) => props.onChange(optionalNumber(event.target.value) ?? 0)} /></label>;
  }
  if (field.data_type === "number_with_unit") {
    const current = typeof props.value === "object" && props.value ? props.value as { value?: number; unit?: string } : {};
    return <label>{field.label}{required}<span className="unitField"><input type="number" value={String(current.value ?? "")} onChange={(event) => props.onChange({ value: optionalNumber(event.target.value) ?? 0, unit: current.unit ?? field.allowed_units[0] ?? "" })} /><select value={current.unit ?? field.allowed_units[0] ?? ""} onChange={(event) => props.onChange({ value: current.value ?? 0, unit: event.target.value })}>{field.allowed_units.map((unit) => <option key={unit} value={unit}>{unit}</option>)}</select></span></label>;
  }
  if (field.data_type === "boolean") {
    return <label className="checkboxLine"><input type="checkbox" checked={Boolean(props.value)} onChange={(event) => props.onChange(event.target.checked)} />{field.label}{required}</label>;
  }
  return <Field label={`${field.label}${required}`} value={String(props.value ?? "")} onChange={props.onChange} />;
}

function ChoiceListEditor(props: {
  title: string;
  values: string[];
  draft: string;
  onDraft: (value: string) => void;
  onAdd: () => void;
  onRemove: (index: number) => void;
}) {
  return (
    <div className="choiceEditor">
      <strong>{props.title}</strong>
      <ul>
        {props.values.map((value, index) => (
          <li key={`${value}-${index}`}>
            <span>{value}</span>
            <button type="button" onClick={() => props.onRemove(index)}>Retirer</button>
          </li>
        ))}
      </ul>
      <div className="inlineEditor">
        <input value={props.draft} onChange={(event) => props.onDraft(event.target.value)} placeholder="Nouvelle valeur" />
        <button type="button" onClick={props.onAdd}>Ajouter une valeur</button>
      </div>
    </div>
  );
}

function TemplatePreview(props: { template: EquipmentEffectiveTemplate | null; values?: Record<string, unknown>; showDiagnostics?: boolean }) {
  if (!props.template) return <p>Aucun template charge.</p>;
  return (
    <div className="templatePreview">
      <strong>{props.template.category_path.map((category) => category.label).join(" > ")}</strong>
      <small>{props.template.fields.filter((field) => field.visible).length} champs visibles</small>
      {props.template.fields.filter((field) => field.visible).map((field) => (
        <span key={field.field.field_id}>
          {field.field.label}{field.required ? " *" : ""}
          {props.values ? `: ${displayTemplateValue(props.values[field.field.field_code]) || "-"}` : ""}
        </span>
      ))}
      {props.showDiagnostics && <code>{props.template.template_checksum}</code>}
    </div>
  );
}

function CategoryTree(props: {
  categories: EquipmentCategory[];
  selectedId: string;
  onSelect: (categoryId: string) => void;
  actions?: Array<[string, string]>;
  onAction?: (categoryId: string, action: string) => void;
}) {
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set(flattenCategories(props.categories).filter((category) => category.children.length > 0).map((category) => category.category_id)));
  const [menuCategoryId, setMenuCategoryId] = useState<string | null>(null);
  useEffect(() => {
    setExpanded((current) => {
      const next = new Set(current);
      flattenCategories(props.categories).forEach((category) => {
        if (category.children.length > 0) next.add(category.category_id);
      });
      return next;
    });
  }, [props.categories]);
  function toggle(categoryId: string) {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(categoryId)) next.delete(categoryId); else next.add(categoryId);
      return next;
    });
  }
  function renderNode(category: EquipmentCategory, depth: number): React.ReactNode {
    const hasChildren = category.children.length > 0;
    const isExpanded = expanded.has(category.category_id);
    const isSelected = props.selectedId === category.category_id;
    return (
      <div key={category.category_id} role="treeitem" aria-expanded={hasChildren ? isExpanded : undefined} aria-selected={isSelected}>
        <div
          className={[
            "categoryTreeRow",
            isSelected ? "selected" : "",
            menuCategoryId === category.category_id ? "menuOpen" : ""
          ].filter(Boolean).join(" ")}
          style={{ paddingLeft: `${depth * 18 + 6}px` }}
          data-category-id={category.category_id}
          tabIndex={0}
          onClick={() => props.onSelect(category.category_id)}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              props.onSelect(category.category_id);
            }
            if (event.key === "ArrowRight" && hasChildren) toggle(category.category_id);
            if (event.key === "ArrowLeft" && hasChildren) toggle(category.category_id);
          }}
        >
          <button className="treeDisclosure" type="button" aria-label={isExpanded ? "Replier" : "Deplier"} onClick={(event) => { event.stopPropagation(); if (hasChildren) toggle(category.category_id); }}>
            {hasChildren ? (isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />) : <span />}
          </button>
          {hasChildren && isExpanded ? <FolderOpen size={15} /> : <Folder size={15} />}
          <span>{category.label}</span>
          {props.actions && props.onAction && (
            <span className="treeMenuWrap">
              <button
                className="treeMenuButton"
                type="button"
                aria-label={`Actions ${category.label}`}
                onClick={(event) => {
                  event.stopPropagation();
                  setMenuCategoryId((current) => current === category.category_id ? null : category.category_id);
                }}
              >
                <MoreHorizontal size={15} />
              </button>
              {menuCategoryId === category.category_id && (
                <div className="treeActionMenu">
                  {props.actions.map(([action, label]) => (
                    <button key={action} type="button" onClick={(event) => { event.stopPropagation(); setMenuCategoryId(null); props.onAction?.(category.category_id, action); }}>
                      {label}
                    </button>
                  ))}
                </div>
              )}
            </span>
          )}
        </div>
        {hasChildren && isExpanded && category.children.map((child) => renderNode(child, depth + 1))}
      </div>
    );
  }
  return (
    <div className="categoryTree" role="tree">
      {props.categories.map((category) => renderNode(category, 0))}
    </div>
  );
}

function ModelCatalog(props: {
  models: EquipmentModelAggregate[];
  selected: EquipmentModelAggregate | null;
  categories: EquipmentCategory[];
  categoryTree: EquipmentCategory[];
  demoMode: "hide" | "show" | "only";
  onCategory: (categoryId: string) => void;
  onOpen: (model: EquipmentModelAggregate) => void;
}) {
  const demoCount = props.models.filter((model) => model.identity.is_demo || model.latest_revision?.definition.is_demo).length;
  return (
    <aside className="equipmentList">
      <div className="listHeader">
        <h2>Modèles</h2>
        <span>{props.models.length}</span>
      </div>
      <details className="catalogTreeFilter">
        <summary>Par catégorie</summary>
        <CategoryTree categories={props.categoryTree} selectedId="" onSelect={props.onCategory} />
      </details>
      {demoCount > 0 && <div className="demoBanner">Donnees de demonstration visibles ({props.demoMode})</div>}
      {props.models.length === 0 && (
        <div className="compactEmpty">
          <strong>Aucun modele trouve</strong>
          <span>Modifiez les filtres ou creez un nouveau modele.</span>
        </div>
      )}
      {props.models.map((model) => {
        const revision = model.latest_revision ?? model.current_approved_revision;
        const definition = revision?.definition;
        const categoryLabel = categoryPathLabel(props.categories, model.identity.category_code);
        const isDemo = model.identity.is_demo || definition?.is_demo;
        return (
          <button
            key={model.identity.equipment_model_id}
            className={props.selected?.identity.equipment_model_id === model.identity.equipment_model_id ? "active" : ""}
            onClick={() => props.onOpen(model)}
          >
            <strong>{model.identity.manufacturer} {model.identity.model_name}</strong>
            <span>{isDemo ? "[DEMO] " : ""}{model.identity.variant ?? categoryLabel}</span>
            <small>{categoryLabel || humanLabel(model.identity.root_category_id ?? model.identity.category_code)}</small>
            <span className="listItemMeta">
              <span className={"status " + (revision?.status ?? "")}>{humanStatus(revision?.status)}</span>
              <small>Révision {revision?.revision_number ?? "-"}</small>
            </span>
          </button>
        );
      })}
    </aside>
  );
}

function ModelStudio(props: {
  model: EquipmentModelAggregate | null;
  revision: EquipmentModelRevision | null;
  definition: EquipmentModelDefinition | null;
  readOnly: boolean;
  registries: EquipmentRegistries | null;
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
          <p className="eyebrow">Fiche modèle équipement</p>
          <h2>{props.model.identity.manufacturer} {props.model.identity.model_name}</h2>
          <div className="studioTitleMeta">
            <span className={"status " + props.revision.status}>{humanStatus(props.revision.status)}</span>
            <span>Révision {props.revision.revision_number}</span>
          </div>
        </div>
        <div className="headerActions">
          <button className="secondary" onClick={props.onValidate}><CheckCircle2 size={16} /> Valider</button>
          {props.revision.status === "draft" && (
            <>
              <button onClick={props.onSave}><Save size={16} /> Sauvegarder</button>
              <button className="secondary" onClick={props.onSubmit}><Send size={16} /> Soumettre</button>
            </>
          )}
          {props.revision.status === "under_review" && (
            <button onClick={props.onApprove}><ShieldCheck size={16} /> Approuver</button>
          )}
          {props.revision.status === "approved" && props.model.current_approved_revision && (
            <button onClick={props.onDerive}><GitBranch size={16} /> Nouvelle revision</button>
          )}
          <button className="secondary" onClick={props.onClone}><Copy size={16} /> Cloner</button>
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
          {props.section === "summary" && (
            <EditorCard title="Synthese">
              <Field label="Fabricant" value={definition.manufacturer} disabled={props.readOnly} onChange={(manufacturer) => props.onDefinition({ ...definition, manufacturer })} />
              <Field label="Modele" value={definition.model_name} disabled={props.readOnly} onChange={(model_name) => props.onDefinition({ ...definition, model_name })} />
              <Field label="Variante" value={definition.variant ?? ""} disabled={props.readOnly} onChange={(variant) => props.onDefinition({ ...definition, variant: optionalString(variant) })} />
              <dl>
                <dt>Categorie</dt><dd>{definition.template_snapshot?.category_path?.join(" > ") || humanLabel(definition.category_code)}</dd>
                <dt>Statut</dt><dd>{humanStatus(props.revision.status)}</dd>
                <dt>Champs renseignes</dt><dd>{Object.keys(definition.custom_field_values ?? {}).length}</dd>
                <dt>Ports</dt><dd>{props.revision.signal_port_count}</dd>
                <dt>Interfaces</dt><dd>{props.revision.interface_count}</dd>
              </dl>
            </EditorCard>
          )}
          {props.section === "identification" && (
            <EditorCard title="Identification">
              {(definition.template_snapshot?.fields ?? []).filter((field) => field.visible).map((field) => (
                <TemplateFieldInput
                  key={field.field.field_id}
                  field={field}
                  value={definition.custom_field_values?.[field.field.field_code]}
                  onChange={(value) => props.onDefinition({ ...definition, custom_field_values: { ...(definition.custom_field_values ?? {}), [field.field.field_code]: value } })}
                />
              ))}
              {!definition.template_snapshot && <p>Ce modele n'a pas encore ete cree depuis un template de categorie.</p>}
            </EditorCard>
          )}
          {props.section === "category_template" && (
            <EditorCard title="Categorie et formulaire">
              <dl>
                <dt>Famille</dt><dd>{humanLabel(definition.template_snapshot?.root_category_id ?? props.model.identity.root_category_id ?? "")}</dd>
                <dt>Categorie</dt><dd>{definition.template_snapshot?.category_path?.join(" > ") || humanLabel(definition.category_code)}</dd>
                <dt>Formulaire utilise</dt><dd>{(definition.template_snapshot?.fields ?? []).filter((field) => field.visible).length} champs visibles</dd>
              </dl>
            </EditorCard>
          )}
          {props.section === "advanced_diagnostics" && (
            <EditorCard title="Diagnostic classification">
              <label>
                Functional role
                <select disabled={props.readOnly} value={definition.functional_role} onChange={(event) => props.onDefinition({ ...definition, functional_role: event.target.value as FunctionalRole })}>
                  {(props.registries?.functional_roles ?? functionalRoles.map((code) => ({ code, label: code }))).map((item) => <option key={item.code} value={item.code}>{item.label}</option>)}
                </select>
              </label>
              <Field label="Signal domains" value={definition.signal_domains.join(", ")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...definition, signal_domains: splitTokens(value) as SignalDomain[] })} />
              <Field label="Technology tags" value={(definition.technology_tags ?? []).join(", ")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...definition, technology_tags: splitTokens(value) as TechnologyTag[] })} />
              <Field label="Preset reference" value={String(definition.metadata?.classification_preset_id ?? "")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...definition, metadata: { ...(definition.metadata ?? {}), classification_preset_id: optionalString(value) } })} />
              <Field label="Classification notes" value={String(definition.metadata?.classification_notes ?? "")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...definition, metadata: { ...(definition.metadata ?? {}), classification_notes: value } })} />
              <dl className="businessSummary">
                <dt>Checksum du formulaire</dt><dd className="mono">{definition.template_snapshot?.template_checksum ?? "-"}</dd>
                <dt>Categorie interne</dt><dd className="mono">{definition.category_code}</dd>
              </dl>
            </EditorCard>
          )}
          {props.section === "characteristics" && (
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
          {props.section === "ports_connections" && (
            <EditorCard title="Port Topology">
              <StructuredTable columns={["ID", "Label", "Direction", "Flow", "Domain", "Tags", "Req.", "Connector", "Quantity", "Unit", "Impedance", "Fmin", "Fmax", "Vmax", "Imax", "Pmax", "Comment"]}>
                {definition.signal_ports.map((port, index) => (
                  <tr key={port.port_id}>
                    <td><input disabled={props.readOnly} value={port.port_id} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, port_id: event.target.value }) })} /></td>
                    <td><input disabled={props.readOnly} value={port.label} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, label: event.target.value }) })} /></td>
                    <td>
                      <select disabled={props.readOnly} value={port.directionality} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, directionality: event.target.value as typeof port.directionality }) })}>
                        {(props.registries?.port_directionalities ?? []).map((item) => <option key={item.code} value={item.code}>{item.code}</option>)}
                      </select>
                    </td>
                    <td>
                      <select disabled={props.readOnly} value={port.flow_role} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, flow_role: event.target.value as typeof port.flow_role }) })}>
                        {(props.registries?.flow_roles ?? []).map((item) => <option key={item.code} value={item.code}>{item.code}</option>)}
                      </select>
                    </td>
                    <td>
                      <select disabled={props.readOnly} value={port.signal_domain} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, signal_domain: event.target.value as SignalDomain }) })}>
                        {(props.registries?.signal_domains ?? []).map((item) => <option key={item.code} value={item.code}>{item.code}</option>)}
                      </select>
                    </td>
                    <td><input disabled={props.readOnly} value={(port.technology_tags ?? []).join(", ")} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, technology_tags: splitTokens(event.target.value) as TechnologyTag[] }) })} /></td>
                    <td><input type="checkbox" disabled={props.readOnly} checked={port.required ?? true} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, required: event.target.checked }) })} /></td>
                    <td><input disabled={props.readOnly} value={port.connector_type ?? ""} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, connector_type: optionalString(event.target.value) }) })} /></td>
                    <td><input disabled={props.readOnly} value={port.quantity} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, quantity: event.target.value as typeof port.quantity }) })} /></td>
                    <td><input disabled={props.readOnly} value={port.unit} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, unit: event.target.value }) })} /></td>
                    <td><input disabled={props.readOnly} value={port.impedance ?? ""} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, impedance: optionalNumber(event.target.value) }) })} /></td>
                    <td><input disabled={props.readOnly} value={port.frequency_min ?? ""} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, frequency_min: optionalNumber(event.target.value) }) })} /></td>
                    <td><input disabled={props.readOnly} value={port.frequency_max ?? ""} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, frequency_max: optionalNumber(event.target.value) }) })} /></td>
                    <td><input disabled={props.readOnly} value={port.voltage_max ?? ""} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, voltage_max: optionalNumber(event.target.value) }) })} /></td>
                    <td><input disabled={props.readOnly} value={port.current_max ?? ""} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, current_max: optionalNumber(event.target.value) }) })} /></td>
                    <td><input disabled={props.readOnly} value={port.power_max ?? ""} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, power_max: optionalNumber(event.target.value) }) })} /></td>
                    <td><input disabled={props.readOnly} value={port.comment ?? ""} onChange={(event) => props.onDefinition({ ...definition, signal_ports: replaceAt(definition.signal_ports, index, { ...port, comment: optionalString(event.target.value) }) })} /></td>
                  </tr>
                ))}
              </StructuredTable>
              <div className="buttonRow">
                <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, signal_ports: [...definition.signal_ports, ...defaultRfThroughPair(definition.signal_ports.length + 1)], signal_domains: mergeTokens(definition.signal_domains, ["rf"]) as SignalDomain[], technology_tags: mergeTokens(definition.technology_tags ?? [], ["rf_50_ohm"]) as TechnologyTag[] })}>Ajouter RF through pair</button>
                <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, signal_ports: [...definition.signal_ports, defaultRfSourceOutput(definition.signal_ports.length + 1)], signal_domains: mergeTokens(definition.signal_domains, ["rf"]) as SignalDomain[], technology_tags: mergeTokens(definition.technology_tags ?? [], ["rf_50_ohm"]) as TechnologyTag[] })}>Ajouter RF source output</button>
                <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, signal_ports: [...definition.signal_ports, defaultRfSinkInput(definition.signal_ports.length + 1)], signal_domains: mergeTokens(definition.signal_domains, ["rf"]) as SignalDomain[], technology_tags: mergeTokens(definition.technology_tags ?? [], ["rf_50_ohm"]) as TechnologyTag[] })}>Ajouter RF sink input</button>
                <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, signal_ports: [...definition.signal_ports, defaultCommunicationPort(definition.signal_ports.length + 1)], signal_domains: mergeTokens(definition.signal_domains, ["ethernet"]) as SignalDomain[], technology_tags: mergeTokens(definition.technology_tags ?? [], ["ethernet"]) as TechnologyTag[] })}>Ajouter communication port</button>
                <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, signal_ports: [...definition.signal_ports, defaultAnalogInputPort(definition.signal_ports.length + 1)], signal_domains: mergeTokens(definition.signal_domains, ["analog_voltage"]) as SignalDomain[], technology_tags: mergeTokens(definition.technology_tags ?? [], ["voltage_input"]) as TechnologyTag[] })}>Ajouter measurement input</button>
                <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, signal_ports: [...definition.signal_ports, defaultTriggerInput(definition.signal_ports.length + 1)], signal_domains: mergeTokens(definition.signal_domains, ["trigger"]) as SignalDomain[], technology_tags: mergeTokens(definition.technology_tags ?? [], ["trigger"]) as TechnologyTag[] })}>Ajouter trigger input</button>
                <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, signal_ports: [...definition.signal_ports, defaultFieldSidePort(definition.signal_ports.length + 1)], signal_domains: mergeTokens(definition.signal_domains, ["environmental"]) as SignalDomain[] })}>Ajouter field-side port</button>
                <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, signal_ports: [...definition.signal_ports, defaultCanBusPort(definition.signal_ports.length + 1)], signal_domains: mergeTokens(definition.signal_domains, ["can_bus"]) as SignalDomain[], technology_tags: mergeTokens(definition.technology_tags ?? [], ["can_bus"]) as TechnologyTag[] })}>Ajouter CAN bus</button>
              </div>
            </EditorCard>
          )}
          {props.section === "control_drivers" && (
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
          {props.section === "control_drivers" && (
            <EditorCard title="Capabilities">
              <StructuredTable columns={["ID", "Kind", "Safety", "Inputs", "Outputs"]}>
                {definition.capabilities.map((capability) => (
                  <tr key={capability.capability_id}><td>{capability.capability_id}</td><td>{capability.capability_kind}</td><td>{capability.safety_class}</td><td>{capability.inputs.length}</td><td>{capability.outputs.length}</td></tr>
                ))}
              </StructuredTable>
              <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...definition, capabilities: [...definition.capabilities, defaultCapability(definition.capabilities.length + 1)] })}>Ajouter capability mesure</button>
            </EditorCard>
          )}
          {props.section === "measurement_corrections" && (
            <EditorCard title="Mesure / corrections">
              <p>Les capteurs, profils de scaling et courbes d'ingenierie restent geres dans les espaces Capteurs, Profils de scaling et Courbes d'ingenierie.</p>
            </EditorCard>
          )}
          {props.section === "documents" && (
            <EditorCard title="Documents">
              <p>Les certificats, datasheets et scripts lies au modele seront attaches via le domaine documents. Cette release prepare l'emplacement sans upload fichier.</p>
            </EditorCard>
          )}
          {props.section === "revisions_audit" && <RevisionTable revisions={props.revisions} onOpen={props.onOpenRevision} />}
          {props.section === "revisions_audit" && <AuditTable audit={props.audit} />}
          {props.section === "advanced_diagnostics" && (
            <EditorCard title="JSON de diagnostic">
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
      <h2>Drivers et actions</h2>
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
            <EditorCard title="Diagnostic JSON">
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

export function ValidationPanel(props: { validation: EquipmentValidationResult | null }) {
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

export function RevisionTable<T extends EquipmentModelRevision | DriverProfileRevision>(props: {
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

export function AuditTable(props: { audit: EquipmentAuditEvent[] }) {
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

export function EditorCard(props: { title: string; children: React.ReactNode }) {
  return <section className="editorCard"><h2>{props.title}</h2>{props.children}</section>;
}

export function StructuredTable(props: { columns: string[]; children: React.ReactNode }) {
  return (
    <div className="tableWrap"><table><thead><tr>{props.columns.map((column) => <th key={column}>{column}</th>)}</tr></thead><tbody>{props.children}</tbody></table></div>
  );
}

export function Field(props: { label: string; value: string; disabled?: boolean; onChange: (value: string) => void }) {
  return <label>{props.label}<input value={props.value} disabled={props.disabled} onChange={(event) => props.onChange(event.target.value)} /></label>;
}

export function StateBlock(props: { title: string; detail: string }) {
  return <div className="stateBlock"><h2>{props.title}</h2><p>{props.detail}</p></div>;
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

const fieldTypes: EquipmentFieldDataType[] = [
  "short_text",
  "long_text",
  "number",
  "number_with_unit",
  "date",
  "boolean",
  "choice",
  "multi_choice",
  "url",
  "file_reference",
  "object_reference"
];

const humanLabels: Record<string, string> = {
  energy_sources: "Sources d'énergie",
  signal_sources: "Sources de signaux",
  rf_equipment: "Équipements radiofréquences",
  sensors_transducers: "Capteurs / transducteurs",
  actuators_emitters: "Actionneurs / Émetteurs",
  measurement_instruments_digitizers: "Instruments de mesure / numériseurs",
  processing_control_systems: "Systèmes de traitement et de contrôle",
  controllable_instrument: "Instrument pilotable",
  daq_device: "DAQ / numériseur",
  acquisition_device: "Système d'acquisition",
  converter: "Convertisseur",
  sensor: "Capteur",
  transducer: "Transducteur",
  passive_component: "Composant passif",
  switching_device: "Commutation",
  motion_system: "Système de mouvement",
  facility: "Infrastructure",
  software_adapter: "Adaptateur logiciel",
  manual_equipment: "Équipement manuel",
  energy_source: "Source d'énergie",
  signal_source: "Source de signal",
  measurement_instrument: "Instrument de mesure",
  acquisition_device_role: "Acquisition",
  rf_network_element: "Élément RF",
  actuator: "Actionneur",
  control_system: "Système de contrôle",
  software_system: "Système logiciel",
  manual_accessory: "Accessoire manuel",
  draft: "Brouillon",
  under_review: "En revue",
  approved: "Approuvé",
  superseded: "Remplacé",
  no_revision: "Sans révision",
  short_text: "Texte court",
  long_text: "Texte long",
  number: "Nombre",
  number_with_unit: "Nombre avec unité",
  date: "Date",
  boolean: "Oui / non",
  choice: "Choix",
  multi_choice: "Choix multiples",
  url: "URL",
  file_reference: "Référence fichier",
  object_reference: "Référence objet",
  rf_50_ohm: "RF 50 ohm",
  can_bus: "CAN bus"
};

function humanLabel(value: string | null | undefined) {
  if (!value) return "-";
  return humanLabels[value] ?? value.replace(/_/g, " ");
}

function humanStatus(value: string | undefined) {
  return humanLabel(value ?? "no_revision");
}

function fieldTypeLabel(type: EquipmentFieldDataType) {
  return humanLabel(type);
}

function categoryPathLabel(categories: EquipmentCategory[], categoryId: string) {
  const byId = new Map(categories.map((category) => [category.category_id, category]));
  const path: string[] = [];
  let current = byId.get(categoryId);
  let guard = 0;
  while (current && guard < categories.length + 1) {
    path.unshift(current.label);
    current = current.parent_category_id ? byId.get(current.parent_category_id) : undefined;
    guard += 1;
  }
  return path.join(" > ") || humanLabel(categoryId);
}

function flattenCategories(categories: EquipmentCategory[]): EquipmentCategory[] {
  return categories.flatMap((category) => [category, ...flattenCategories(category.children)]);
}

function descendantCategoryIds(categories: EquipmentCategory[], categoryId: string): Set<string> {
  const byParent = new Map<string, EquipmentCategory[]>();
  categories.forEach((category) => {
    if (!category.parent_category_id) return;
    const children = byParent.get(category.parent_category_id) ?? [];
    children.push(category);
    byParent.set(category.parent_category_id, children);
  });
  const output = new Set<string>();
  const visit = (id: string) => {
    for (const child of byParent.get(id) ?? []) {
      output.add(child.category_id);
      visit(child.category_id);
    }
  };
  visit(categoryId);
  return output;
}

function slugifyLabel(label: string): string {
  const normalized = label
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
  return normalized || "element";
}

function uniqueGeneratedCode(label: string, existing: Set<string>): string {
  const base = slugifyLabel(label);
  let candidate = base;
  let index = 2;
  while (existing.has(candidate)) {
    candidate = `${base}_${index}`;
    index += 1;
  }
  return candidate;
}

function displayTemplateValue(value: unknown): string {
  if (value === null || value === undefined || value === "") return "";
  if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) return value.map(displayTemplateValue).join(", ");
  if (typeof value === "object") {
    const maybeUnit = value as { value?: unknown; unit?: unknown };
    if (maybeUnit.value !== undefined && maybeUnit.unit !== undefined) {
      return `${displayTemplateValue(maybeUnit.value)} ${displayTemplateValue(maybeUnit.unit)}`;
    }
  }
  return JSON.stringify(value);
}

function isEmptyField(value: unknown) {
  return value === undefined || value === null || value === "" || (Array.isArray(value) && value.length === 0);
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

function defaultRfSinkInput(index: number) {
  return {
    port_id: `rf_input_${index}`,
    label: `RF sink input ${index}`,
    directionality: "input" as const,
    flow_role: "sink_port" as const,
    signal_domain: "rf" as const,
    required: true,
    connector_type: "N",
    technology_tags: ["rf_50_ohm" as const],
    quantity: "power" as const,
    unit: "dBm",
    impedance: 50,
    frequency_min: 9000,
    frequency_max: 1000000000,
    power_max: 30
  };
}

function defaultRfSourceOutput(index: number) {
  return {
    port_id: `rf_out_${index}`,
    label: `RF source output ${index}`,
    directionality: "output" as const,
    flow_role: "source_port" as const,
    signal_domain: "rf" as const,
    required: true,
    connector_type: "N",
    technology_tags: ["rf_50_ohm" as const],
    quantity: "power" as const,
    unit: "dBm",
    impedance: 50,
    frequency_min: 9000,
    frequency_max: 1000000000,
    power_max: 30
  };
}

function defaultRfThroughPair(startIndex: number) {
  return [
    {
      port_id: `rf_a_${startIndex}`,
      label: `RF A ${startIndex}`,
      directionality: "through" as const,
      flow_role: "through_port" as const,
      signal_domain: "rf" as const,
      required: true,
      connector_type: "N",
      technology_tags: ["rf_50_ohm" as const],
      quantity: "power" as const,
      unit: "dBm",
      impedance: 50
    },
    {
      port_id: `rf_b_${startIndex + 1}`,
      label: `RF B ${startIndex + 1}`,
      directionality: "through" as const,
      flow_role: "through_port" as const,
      signal_domain: "rf" as const,
      required: true,
      connector_type: "N",
      technology_tags: ["rf_50_ohm" as const],
      quantity: "power" as const,
      unit: "dBm",
      impedance: 50
    }
  ];
}

function defaultAnalogInputPort(index: number) {
  return {
    port_id: `ai_${index}`,
    label: `Measurement input ${index}`,
    directionality: "input" as const,
    flow_role: "measurement_port" as const,
    signal_domain: "analog_voltage" as const,
    required: true,
    connector_type: "BNC",
    technology_tags: ["voltage_input" as const],
    quantity: "voltage" as const,
    unit: "V"
  };
}

function defaultCommunicationPort(index: number) {
  return {
    port_id: `lan_${index}`,
    label: `Ethernet communication ${index}`,
    directionality: "communication" as const,
    flow_role: "communication_port" as const,
    signal_domain: "ethernet" as const,
    required: false,
    connector_type: "RJ45",
    technology_tags: ["ethernet" as const],
    quantity: "binary" as const,
    unit: "dimensionless"
  };
}

function defaultTriggerInput(index: number) {
  return {
    port_id: `trig_in_${index}`,
    label: `Trigger input ${index}`,
    directionality: "input" as const,
    flow_role: "control_port" as const,
    signal_domain: "trigger" as const,
    required: false,
    connector_type: "BNC",
    technology_tags: ["trigger" as const],
    quantity: "voltage" as const,
    unit: "V"
  };
}

function defaultFieldSidePort(index: number) {
  return {
    port_id: `field_${index}`,
    label: `Field-side port ${index}`,
    directionality: "input" as const,
    flow_role: "field_side_port" as const,
    signal_domain: "environmental" as const,
    required: true,
    connector_type: undefined,
    technology_tags: [],
    quantity: "electric_field" as const,
    unit: "dBuV_per_m"
  };
}

function defaultCanBusPort(index: number) {
  return {
    port_id: `can_bus_${index}`,
    label: `CAN bus ${index}`,
    directionality: "communication" as const,
    flow_role: "communication_port" as const,
    signal_domain: "can_bus" as const,
    required: true,
    connector_type: "D-Sub",
    technology_tags: ["can_bus" as const],
    quantity: "binary" as const,
    unit: "dimensionless"
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

function optionalNumber(value: string) {
  const trimmed = value.trim();
  if (!trimmed) return undefined;
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function mergeTokens<T extends string>(existing: T[], additions: T[]) {
  return Array.from(new Set([...existing, ...additions]));
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
    const issues = Array.isArray(error.details?.issues) ? error.details.issues as Array<{ message?: string }> : [];
    const firstIssue = issues.find((issue) => issue.message)?.message;
    if (firstIssue) return firstIssue;
    const readable: Record<string, string> = {
      invalid_equipment_field_definition: "Le champ n'est pas valide. Verifiez son nom, son type et ses valeurs possibles.",
      invalid_equipment_model_definition: "Le modele equipement n'est pas valide. Verifiez les champs obligatoires et les valeurs saisies.",
      equipment_template_required_field_missing: "Un champ obligatoire du formulaire n'est pas renseigne.",
      invalid_id: "Le nom technique genere existe deja ou contient un caractere non autorise. Modifiez le libelle ou l'identifiant avance.",
      equipment_category_in_use: "Cette categorie est utilisee par des modeles. Archivez-la plutot que de la supprimer.",
      equipment_category_system_root_immutable: "Les familles racines systeme ne peuvent pas etre archivees."
    };
    return readable[error.code] ?? error.message;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
