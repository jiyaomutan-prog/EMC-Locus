import {
  AlertCircle,
  CheckCircle2,
  ChevronRight,
  Link2,
  Plus,
  RefreshCw,
  Save,
  ShieldCheck,
  Trash2,
  X
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { equipmentApi, fleetApi, stationSetupApi, type OperationContext } from "../../api";
import { operatorCategoryPath, operatorModelName } from "../../operatorEquipmentLabels";
import type { EquipmentCategory } from "../../models/equipment";
import type { LaboratoryLocation, PhysicalAsset } from "../../models/fleet";
import type {
  StationMaterialAssignment,
  StationMaterialCandidate,
  StationMaterialRequirement,
  StationMaterialSelectionPolicy,
  StationMeasurementSetupDefinition,
  StationSetupAggregate,
  StationSetupReadiness
} from "../../models/stationSetup";

const operationContext: OperationContext = {
  actor: "station.technician",
  reason: "préparation d'un montage depuis LAB CONSOLE"
};

export function StationSetupWorkspace() {
  const [setups, setSetups] = useState<StationSetupAggregate[]>([]);
  const [locations, setLocations] = useState<LaboratoryLocation[]>([]);
  const [categories, setCategories] = useState<EquipmentCategory[]>([]);
  const [fleet, setFleet] = useState<PhysicalAsset[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [listError, setListError] = useState<string | null>(null);
  const [locationsError, setLocationsError] = useState<string | null>(null);
  const [referenceError, setReferenceError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  const loadSetups = useCallback(async () => {
    try {
      const response = await stationSetupApi.list();
      setSetups(response.station_setups);
      setSelectedId((current) => response.station_setups.some((setup) => setup.identity.setup_id === current)
        ? current
        : response.station_setups[0]?.identity.setup_id ?? "");
      setListError(null);
    } catch (error) {
      setListError(errorMessage(error));
    }
  }, []);

  const loadLocations = useCallback(async () => {
    try {
      const response = await fleetApi.listLocations();
      setLocations(response.locations);
      setLocationsError(null);
    } catch (error) {
      setLocationsError(errorMessage(error));
    }
  }, []);

  const loadReferences = useCallback(async () => {
    try {
      const [categoryResponse, fleetResponse] = await Promise.all([
        equipmentApi.categoryTree(),
        fleetApi.listAssets()
      ]);
      setCategories(categoryResponse.categories);
      setFleet(fleetResponse.assets);
      setReferenceError(null);
    } catch (error) {
      setReferenceError(errorMessage(error));
    }
  }, []);

  useEffect(() => {
    void loadSetups();
    void loadLocations();
    void loadReferences();
  }, [loadLocations, loadReferences, loadSetups]);

  const selected = setups.find((setup) => setup.identity.setup_id === selectedId) ?? null;

  async function createSetup(input: {
    label: string;
    locationId: string;
    plannedUseOn: string;
    executionMode: "accredited" | "non_accredited" | "investigation";
  }) {
    const location = locations.find((candidate) => candidate.location_id === input.locationId);
    if (!location) throw new Error("Sélectionnez un lieu actif du laboratoire.");
    const setupId = `SETUP-${crypto.randomUUID().slice(0, 8).toUpperCase()}`;
    const result = await stationSetupApi.create({
      setup_id: setupId,
      label: input.label,
      laboratory_location_id: location.location_id,
      planned_use_on: input.plannedUseOn,
      execution_mode: input.executionMode
    }, operationContext);
    setSetups((current) => [result.station_setup, ...current]);
    setSelectedId(result.station_setup.identity.setup_id);
    setCreating(false);
  }

  function replaceSetup(setup: StationSetupAggregate) {
    setSetups((current) => current.map((candidate) =>
      candidate.identity.setup_id === setup.identity.setup_id ? setup : candidate
    ));
  }

  return (
    <section className="stationWorkspace" aria-label="Montages de mesure">
      <header className="resourcePageHeader">
        <div>
          <p className="contextBanner">Vous définissez les rôles matériels avant de confirmer les exemplaires utilisables.</p>
          <h2>Montages de mesure</h2>
          <p>Besoins du montage, affectations physiques et aptitude opérationnelle restent distincts.</p>
        </div>
        <div className="headerActions">
          <button className="iconButton secondary" type="button" onClick={() => void loadSetups()} title="Rafraîchir les montages" aria-label="Rafraîchir les montages"><RefreshCw size={16} /></button>
          <button type="button" onClick={() => setCreating(true)} disabled={Boolean(locationsError) || locations.length === 0}><Plus size={16} /> Nouveau montage</button>
        </div>
      </header>
      {locationsError && <TargetedError title="Lieux du laboratoire indisponibles" detail="Les montages existants restent consultables. La création et le changement de lieu sont suspendus." />}
      {referenceError && <TargetedError title="Référentiel matériel partiellement indisponible" detail="Le montage reste lisible. La création de rôles et la recherche de candidats sont suspendues." />}
      {listError && <TargetedError title="Montages temporairement indisponibles" detail={listError} />}

      <div className="stationLayout">
        <aside className="stationList">
          <div className="listHeader"><h3>Montages enregistrés</h3><span>{setups.length}</span></div>
          {setups.length === 0 && !listError && <div className="compactEmpty"><strong>Aucun montage</strong><span>Créez le premier montage puis décrivez ses rôles matériels.</span></div>}
          {setups.map((setup) => {
            const revision = activeRevision(setup);
            return <button key={setup.identity.setup_id} type="button" className={setup.identity.setup_id === selectedId ? "active" : ""} onClick={() => setSelectedId(setup.identity.setup_id)}>
              <span><strong>{setup.identity.label}</strong><small>{revision.definition.laboratory_location_label} · {formatDate(revision.definition.planned_use_on)}</small></span>
              <span className={`status ${revision.status}`}>{revisionStatusLabel(revision.status)}</span>
              <ChevronRight size={15} />
            </button>;
          })}
        </aside>
        <StationSetupDetail
          setup={selected}
          locations={locations}
          categories={categories}
          fleet={fleet}
          locationsError={locationsError}
          referenceError={referenceError}
          onReplace={replaceSetup}
        />
      </div>

      {creating && <div className="modalBackdrop"><CreateStationDialog locations={locations} onCancel={() => setCreating(false)} onCreate={createSetup} /></div>}
    </section>
  );
}

function StationSetupDetail(props: {
  setup: StationSetupAggregate | null;
  locations: LaboratoryLocation[];
  categories: EquipmentCategory[];
  fleet: PhysicalAsset[];
  locationsError: string | null;
  referenceError: string | null;
  onReplace: (setup: StationSetupAggregate) => void;
}) {
  const revision = props.setup ? activeRevision(props.setup) : null;
  const [definition, setDefinition] = useState<StationMeasurementSetupDefinition | null>(revision?.definition ?? null);
  const [readiness, setReadiness] = useState<StationSetupReadiness | null>(revision?.readiness ?? null);
  const [roleEditorOpen, setRoleEditorOpen] = useState(false);
  const [selectedRequirementId, setSelectedRequirementId] = useState("");
  const [candidates, setCandidates] = useState<StationMaterialCandidate[]>([]);
  const [candidateContextKey, setCandidateContextKey] = useState("");
  const [candidatesLoading, setCandidatesLoading] = useState(false);
  const [candidatesError, setCandidatesError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const candidateSequence = useRef(0);

  useEffect(() => {
    setDefinition(revision?.definition ?? null);
    setReadiness(revision?.readiness ?? null);
    setSelectedRequirementId("");
    setCandidates([]);
    setCandidatesError(null);
    setError(null);
  }, [revision]);

  const requirements = definition?.material_requirements ?? [];
  const assignments = definition?.material_assignments ?? [];
  const isV3 = definition?.definition_schema_version.endsWith(".v3") ?? false;
  const dirty = Boolean(definition && revision && JSON.stringify(definition) !== JSON.stringify(revision.definition));
  const readOnly = revision?.status !== "draft";
  const candidateRequestKey = definition && revision && selectedRequirementId
    ? JSON.stringify({
      setup: definition.setup_id,
      revision: revision.revision_id,
      checksum: revision.definition_checksum,
      requirement: selectedRequirementId,
      date: definition.planned_use_on,
      mode: definition.execution_mode,
      location: definition.laboratory_location_id
    })
    : "";

  useEffect(() => {
    if (!definition || !revision || !selectedRequirementId || dirty || !definition.laboratory_location_id) {
      setCandidates([]);
      setCandidateContextKey("");
      return;
    }
    const sequence = ++candidateSequence.current;
    let cancelled = false;
    setCandidatesLoading(true);
    setCandidatesError(null);
    void stationSetupApi.materialCandidates(
      definition.setup_id,
      revision.revision_id,
      selectedRequirementId,
      {
        planned_use_on: definition.planned_use_on,
        execution_mode: definition.execution_mode,
        laboratory_location_id: definition.laboratory_location_id
      }
    ).then((response) => {
      if (cancelled || sequence !== candidateSequence.current) return;
      setCandidates(response.candidates);
      setCandidateContextKey(candidateRequestKey);
      setCandidatesLoading(false);
    }).catch((reason) => {
      if (cancelled || sequence !== candidateSequence.current) return;
      setCandidatesError(errorMessage(reason));
      setCandidatesLoading(false);
    });
    return () => { cancelled = true; };
  }, [candidateRequestKey, definition, dirty, revision, selectedRequirementId]);

  if (!props.setup || !revision || !definition) {
    return <article className="stationDetail empty"><strong>Aucun montage ouvert</strong><p>Sélectionnez un montage ou créez-en un nouveau.</p></article>;
  }

  async function run(operation: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await operation();
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  async function save() {
    const currentRevision = revision!;
    const currentDefinition = definition!;
    const result = await stationSetupApi.replaceDraft(
      props.setup!.identity.setup_id,
      currentRevision.revision_id,
      currentRevision.definition_checksum,
      currentDefinition,
      operationContext
    );
    props.onReplace(result.station_setup);
  }

  async function assess() {
    const result = await stationSetupApi.assess(props.setup!.identity.setup_id, revision!.revision_id);
    setReadiness(result.readiness);
  }

  async function qualify() {
    const currentRevision = revision!;
    const result = await stationSetupApi.markQualified(
      props.setup!.identity.setup_id,
      currentRevision.revision_id,
      currentRevision.definition_checksum,
      operationContext
    );
    props.onReplace(result.station_setup);
  }

  async function markReady() {
    const currentRevision = revision!;
    const result = await stationSetupApi.markReady(
      props.setup!.identity.setup_id,
      currentRevision.revision_id,
      currentRevision.definition_checksum,
      operationContext
    );
    props.onReplace(result.station_setup);
  }

  async function deriveV3() {
    const result = await stationSetupApi.deriveRevision(
      props.setup!.identity.setup_id,
      revision!.revision_id,
      true,
      { ...operationContext, reason: "conversion explicite du montage historique vers le contrat v3" }
    );
    props.onReplace(result.station_setup);
  }

  async function deriveQualifiedDraft() {
    const result = await stationSetupApi.deriveRevision(
      props.setup!.identity.setup_id,
      revision!.revision_id,
      false,
      {
        ...operationContext,
        reason: "finalisation des affectations physiques d'une définition validée"
      }
    );
    props.onReplace(result.station_setup);
  }

  function addRequirement(requirement: StationMaterialRequirement) {
    setDefinition((current) => current ? {
      ...current,
      material_requirements: [...(current.material_requirements ?? []), requirement]
    } : current);
    setRoleEditorOpen(false);
    setReadiness(null);
  }

  function removeRequirement(requirementId: string) {
    setDefinition((current) => current ? {
      ...current,
      material_requirements: (current.material_requirements ?? []).filter((item) => item.requirement_id !== requirementId),
      material_assignments: (current.material_assignments ?? []).filter((item) => item.requirement_id !== requirementId),
      logical_connections: (current.logical_connections ?? []).filter((connection) =>
        connection.from.requirement_id !== requirementId && connection.to.requirement_id !== requirementId)
    } : current);
    setReadiness(null);
  }

  function connectRoles() {
    const connected = requirements.slice(0, -1).map((requirement, index) => ({
      connection_id: `logical-link-${index + 1}`,
      label: `${requirement.role_label} vers ${requirements[index + 1].role_label}`,
      from: {
        requirement_id: requirement.requirement_id,
        logical_port_id: requirement.logical_ports[0]?.logical_port_id ?? "port"
      },
      to: {
        requirement_id: requirements[index + 1].requirement_id,
        logical_port_id: requirements[index + 1].logical_ports[0]?.logical_port_id ?? "port"
      }
    }));
    setDefinition((current) => current ? { ...current, logical_connections: connected } : current);
    setReadiness(null);
  }

  function assignCandidate(requirement: StationMaterialRequirement, candidate: StationMaterialCandidate) {
    if (!candidate.assignable || !candidate.asset.equipment_model_id || !candidate.asset.equipment_model_revision_id || !candidate.asset.equipment_model_checksum) return;
    const selectedPorts = requirement.logical_ports.map((port) => ({
      logical_port_id: port.logical_port_id,
      actual_port_id: candidate.logical_port_resolution_candidates[port.logical_port_id]?.[0] ?? ""
    })).filter((mapping) => mapping.actual_port_id);
    const assignment: StationMaterialAssignment = {
      requirement_id: requirement.requirement_id,
      asset_id: candidate.asset.asset_id,
      asset_revision: String(candidate.asset.revision),
      inventory_code: candidate.asset.inventory_code,
      serial_number: candidate.asset.serial_number ?? undefined,
      equipment_model_id: candidate.asset.equipment_model_id,
      equipment_model_revision_id: candidate.asset.equipment_model_revision_id,
      equipment_model_checksum: candidate.asset.equipment_model_checksum,
      selected_ports: selectedPorts,
      assignment_context: "setup_definition",
      assigned_on: definition!.planned_use_on
    };
    setDefinition((current) => current ? {
      ...current,
      material_assignments: [...assignments.filter((item) => item.requirement_id !== requirement.requirement_id), assignment]
    } : current);
    setReadiness(null);
  }

  return <article className="stationDetail">
    <header className="stationIdentityHeader">
      <div><p className="eyebrow">Montage de mesure</p><h2>{definition.label}</h2><p>{definition.laboratory_location_label} · utilisation prévue le {formatDate(definition.planned_use_on)}</p></div>
      <span className={`status ${revision.status}`}>{revisionStatusLongLabel(revision.status)}</span>
    </header>

    {!isV3 && <section className="stationSection legacyStationNotice">
      <div className="sectionTitleRow"><div><h3>Montage historique v2</h3><p>Cette version reste lisible et inchangée. Créez un brouillon v3 explicite pour définir des rôles matériels.</p></div></div>
      <button type="button" disabled={busy || revision.status === "draft"} onClick={() => void run(deriveV3)}><RefreshCw size={16} /> Créer un brouillon v3</button>
    </section>}

    <section className="stationSection">
      <div className="sectionTitleRow"><div><h3>Contexte d'utilisation</h3><p>Le lieu, la date et le mode déterminent l'aptitude opérationnelle, pas la compatibilité de conception.</p></div></div>
      <div className="formGrid">
        <label>Nom du montage <Required /><input value={definition.label} disabled={readOnly} onChange={(event) => setDefinition({ ...definition, label: event.target.value })} /></label>
        <label>Lieu du laboratoire <Required /><select value={definition.laboratory_location_id ?? ""} disabled={readOnly || Boolean(props.locationsError)} onChange={(event) => { const location = props.locations.find((candidate) => candidate.location_id === event.target.value); setDefinition({ ...definition, laboratory_location_id: location?.location_id ?? null, laboratory_location_label: location?.label ?? "" }); }}><option value="">Sélectionner...</option>{props.locations.map((location) => <option key={location.location_id} value={location.location_id}>{location.label}</option>)}</select></label>
        <label>Date d'utilisation <Required /><input type="date" value={definition.planned_use_on} disabled={readOnly} onChange={(event) => setDefinition({ ...definition, planned_use_on: event.target.value })} /></label>
        <label>Mode d'essai <Required /><select value={definition.execution_mode} disabled={readOnly} onChange={(event) => setDefinition({ ...definition, execution_mode: event.target.value as StationMeasurementSetupDefinition["execution_mode"] })}><option value="accredited">Sous accréditation</option><option value="non_accredited">Hors accréditation</option><option value="investigation">Investigation</option></select></label>
      </div>
    </section>

    {isV3 ? <>
      <section className="stationSection">
        <div className="sectionTitleRow">
          <div><h3>Rôles matériels du montage</h3><p>Décrivez ce dont le montage a besoin avant de choisir l'exemplaire réellement utilisable.</p></div>
          <div className="headerActions"><span className="countBadge">{requirements.length}</span>{!readOnly && <button type="button" disabled={Boolean(props.referenceError)} onClick={() => setRoleEditorOpen(true)}><Plus size={15} /> Ajouter un rôle</button>}</div>
        </div>
        {requirements.length === 0 && <div className="compactEmpty"><strong>Aucun rôle matériel</strong><span>Choisissez une catégorie, des aptitudes techniques ou un exemplaire imposé.</span></div>}
        <div className="stationRequirementList">
          {requirements.map((requirement) => {
            const assignment = assignments.find((item) => item.requirement_id === requirement.requirement_id);
            return <article key={requirement.requirement_id} className={selectedRequirementId === requirement.requirement_id ? "active" : ""}>
              <header><div><strong>{requirement.role_label}</strong><small>{requirement.required ? "Obligatoire" : "Optionnel"} · {selectionPolicyLabel(requirement.selection_policy)}</small></div>{!readOnly && <button className="iconButton secondary" type="button" aria-label={`Retirer ${requirement.role_label}`} onClick={() => removeRequirement(requirement.requirement_id)}><Trash2 size={15} /></button>}</header>
              <p>{requirementSummary(requirement, props.categories, props.fleet)}</p>
              <dl><dt>Affectation</dt><dd>{assignment ? `${assignment.inventory_code} · ${assignment.serial_number || "Sans numéro de série"}` : assignmentStageLabel(requirement.assignment_stage)}</dd><dt>Substitution</dt><dd>{substitutionLabel(requirement.substitution_policy)}</dd></dl>
              {requirement.selection_policy === "exact_asset" && <p className="exactAssetRequirement"><ShieldCheck size={15} /> Exemplaire imposé : {assetLabel(props.fleet.find((asset) => asset.asset_id === requirement.exact_asset_id))}</p>}
              <button className="secondary" type="button" disabled={dirty} onClick={() => setSelectedRequirementId(requirement.requirement_id)}><RefreshCw size={15} /> Vérifier les candidats</button>
            </article>;
          })}
        </div>
        {!readOnly && requirements.length >= 2 && <div className="stationTopologyAction"><button className="secondary" type="button" onClick={connectRoles}><Link2 size={15} /> Relier les rôles dans l'ordre</button><span>{definition.logical_connections?.length ?? 0} liaison(s) logique(s)</span></div>}
      </section>

      {selectedRequirementId && <CandidatePanel
        requirement={requirements.find((item) => item.requirement_id === selectedRequirementId) ?? null}
        candidates={candidateContextKey === candidateRequestKey ? candidates : []}
        loading={candidatesLoading}
        error={candidatesError}
        dirty={dirty}
        readOnly={readOnly}
        onAssign={assignCandidate}
      />}
    </> : <LegacyBindings definition={definition} fleet={props.fleet} />}

    <section className={`stationReadinessPanel ${readiness?.ready ? "ready" : "blocked"}`}>
      <div>{readiness?.ready ? <CheckCircle2 size={19} /> : <AlertCircle size={19} />}<div><h3>{readiness?.ready ? "Montage apte à être utilisé" : "Aptitude opérationnelle non acquise"}</h3><p>{revision.status === "qualified" ? "La définition logique est validée. Les affectations et contrôles opérationnels restent à terminer." : readiness ? `Contrôle du ${formatDate(readiness.checked_on)}.` : "Enregistrez les modifications avant le contrôle."}</p></div></div>
      {readiness && readiness.issues.length > 0 && <ul>{readiness.issues.map((issue) => <li key={`${issue.code}-${issue.binding_ids?.join("-") ?? "setup"}`}><strong>{readinessDimensionLabel(issue.dimension)}</strong> {issue.message}</li>)}</ul>}
    </section>

    {error && <TargetedError title="Opération refusée" detail={error} />}
    <div className="stationActions">
      {!readOnly && <button type="button" disabled={busy || !dirty || !definition.label.trim() || !definition.laboratory_location_id} onClick={() => void run(save)}><Save size={16} /> Enregistrer comme exigence du montage</button>}
      {!readOnly && !dirty && <button className="secondary" type="button" disabled={busy} onClick={() => void run(assess)}><RefreshCw size={16} /> Contrôler l'aptitude opérationnelle</button>}
      {!readOnly && isV3 && !dirty && <button type="button" disabled={busy || requirements.length < 2} onClick={() => void run(qualify)}><ShieldCheck size={16} /> Valider la définition</button>}
      {revision.status === "qualified" && <button className="secondary" type="button" disabled={busy} onClick={() => void run(deriveQualifiedDraft)}><RefreshCw size={16} /> Finaliser les affectations dans un brouillon</button>}
      {(revision.status === "qualified" || (!isV3 && revision.status === "draft")) && <button type="button" disabled={busy || !readiness?.ready} onClick={() => void run(markReady)}><CheckCircle2 size={16} /> Déclarer prêt à utiliser</button>}
    </div>
    {!readOnly && dirty && <p className="actionExplanation">Enregistrez le brouillon avant de demander les candidats ou de relancer les contrôles.</p>}
    <details><summary>Détails techniques</summary><dl><dt>Schéma</dt><dd>{definition.definition_schema_version}</dd><dt>Identifiant interne</dt><dd>{props.setup.identity.setup_id}</dd><dt>Révision</dt><dd>{revision.revision_id}</dd><dt>Empreinte</dt><dd className="technicalValue">{revision.definition_checksum}</dd></dl></details>

    {roleEditorOpen && <div className="modalBackdrop"><MaterialRoleDialog categories={props.categories} fleet={props.fleet} onCancel={() => setRoleEditorOpen(false)} onAdd={addRequirement} /></div>}
  </article>;
}

function MaterialRoleDialog(props: {
  categories: EquipmentCategory[];
  fleet: PhysicalAsset[];
  onCancel: () => void;
  onAdd: (requirement: StationMaterialRequirement) => void;
}) {
  const [policy, setPolicy] = useState<StationMaterialSelectionPolicy>("category_pool");
  const [label, setLabel] = useState("");
  const [description, setDescription] = useState("");
  const [required, setRequired] = useState(true);
  const [assignmentStage, setAssignmentStage] = useState<"setup_definition" | "planned_test_preparation">("planned_test_preparation");
  const [substitutionPolicy, setSubstitutionPolicy] = useState<StationMaterialRequirement["substitution_policy"]>("same_category");
  const [calibrationRequirement, setCalibrationRequirement] = useState<StationMaterialRequirement["calibration_requirement"]>("if_used");
  const [categoryId, setCategoryId] = useState("");
  const [acceptDescendants, setAcceptDescendants] = useState(true);
  const [capabilityKind, setCapabilityKind] = useState("measure_emission");
  const [frequencyMin, setFrequencyMin] = useState("9");
  const [frequencyMax, setFrequencyMax] = useState("1000");
  const [frequencyUnit, setFrequencyUnit] = useState("MHz");
  const [voltageMaximum, setVoltageMaximum] = useState("");
  const [currentMaximum, setCurrentMaximum] = useState("");
  const [powerMaximum, setPowerMaximum] = useState("");
  const [impedanceOhm, setImpedanceOhm] = useState("50");
  const [detectorModes, setDetectorModes] = useState("peak, quasi_peak");
  const [signalDomain, setSignalDomain] = useState("rf");
  const [connector, setConnector] = useState("N");
  const [driverAction, setDriverAction] = useState("");
  const [communicationCapability, setCommunicationCapability] = useState("");
  const [modelCapabilityId, setModelCapabilityId] = useState("");
  const [exactAssetId, setExactAssetId] = useState("");
  const [portLabel, setPortLabel] = useState("Port signal");
  const [directionality, setDirectionality] = useState<"input" | "output" | "bidirectional">("input");
  const labelRef = useRef<HTMLInputElement>(null);
  const missing = [
    !label.trim() && "le nom du rôle",
    policy === "category_pool" && !categoryId && "la catégorie",
    policy === "capability_match" && !capabilityKind.trim() && "l'aptitude",
    policy === "exact_asset" && !exactAssetId && "l'exemplaire imposé",
    !portLabel.trim() && "le port logique"
  ].filter(Boolean) as string[];

  function submit() {
    if (missing.length > 0) {
      labelRef.current?.focus();
      return;
    }
    const requirementId = `role-${crypto.randomUUID().slice(0, 8)}`;
    props.onAdd({
      requirement_id: requirementId,
      role_label: label.trim(),
      description: description.trim() || undefined,
      required,
      selection_policy: policy,
      assignment_stage: policy === "exact_asset" ? "setup_definition" : assignmentStage,
      substitution_policy: policy === "exact_asset" ? "no_substitution" : substitutionPolicy,
      calibration_requirement: calibrationRequirement,
      category_requirement: policy === "category_pool" ? { category_id: categoryId, accept_descendants: acceptDescendants } : undefined,
      capability_requirement: policy === "capability_match" ? {
        capability_kind: capabilityKind.trim(),
        model_capability_id: modelCapabilityId.trim() || undefined,
        frequency_range: frequencyMin || frequencyMax ? {
          minimum: optionalNumber(frequencyMin),
          maximum: optionalNumber(frequencyMax),
          unit: frequencyUnit
        } : undefined,
        voltage_range: voltageMaximum ? { maximum: optionalNumber(voltageMaximum), unit: "V" } : undefined,
        current_range: currentMaximum ? { maximum: optionalNumber(currentMaximum), unit: "A" } : undefined,
        power_range: powerMaximum ? { maximum: optionalNumber(powerMaximum), unit: "W" } : undefined,
        detector_modes: splitTokens(detectorModes),
        signal_domain: signalDomain || undefined,
        port_directionality: directionality,
        connector_requirement: connector.trim() || undefined,
        impedance_ohm: optionalNumber(impedanceOhm),
        communication_capability: communicationCapability.trim() || undefined,
        automated_control_required: Boolean(driverAction.trim()),
        required_driver_action: driverAction.trim() || undefined
      } : undefined,
      exact_asset_id: policy === "exact_asset" ? exactAssetId : undefined,
      logical_ports: [{
        logical_port_id: "signal",
        label: portLabel.trim(),
        directionality,
        signal_domain: signalDomain || "rf",
        connector_requirement: connector.trim() || undefined,
        impedance_ohm: optionalNumber(impedanceOhm),
        frequency_range: frequencyMin || frequencyMax ? {
          minimum: optionalNumber(frequencyMin),
          maximum: optionalNumber(frequencyMax),
          unit: frequencyUnit
        } : undefined,
        voltage_range: voltageMaximum ? { maximum: optionalNumber(voltageMaximum), unit: "V" } : undefined,
        current_range: currentMaximum ? { maximum: optionalNumber(currentMaximum), unit: "A" } : undefined,
        power_range: powerMaximum ? { maximum: optionalNumber(powerMaximum), unit: "W" } : undefined
      }]
    });
  }

  return <section className="modalCard stationRoleDialog" role="dialog" aria-modal="true" aria-labelledby="station-role-title">
    <header><div><p className="eyebrow">Rôles matériels du montage</p><h2 id="station-role-title">Ajouter un rôle</h2></div><button className="iconButton secondary" type="button" aria-label="Fermer" onClick={props.onCancel}><X size={16} /></button></header>
    <div className="segmentedControl" aria-label="Mode de sélection du matériel">
      {([
        ["category_pool", "Choisir plus tard dans une catégorie"],
        ["capability_match", "Exiger des aptitudes techniques"],
        ["exact_asset", "Imposer un exemplaire du parc"]
        ] as const).map(([value, text]) => <button key={value} type="button" className={policy === value ? "active" : ""} onClick={() => { setPolicy(value); setSubstitutionPolicy(value === "category_pool" ? "same_category" : value === "capability_match" ? "same_capabilities" : "no_substitution"); }}>{text}</button>)}
    </div>
    <div className="formGrid">
      <label>Nom du rôle <Required /><input ref={labelRef} autoFocus value={label} onChange={(event) => setLabel(event.target.value)} placeholder="Ex. Récepteur EMI" /></label>
      <label>Description <textarea value={description} onChange={(event) => setDescription(event.target.value)} /></label>
      <label className="checkboxLabel"><input type="checkbox" checked={required} onChange={(event) => setRequired(event.target.checked)} /> Rôle obligatoire</label>
      {policy !== "exact_asset" && <label>Moment du choix <select value={assignmentStage} onChange={(event) => setAssignmentStage(event.target.value as typeof assignmentStage)}><option value="planned_test_preparation">Lors de la préparation de l'essai</option><option value="setup_definition">Dans la définition du montage</option></select></label>}
      <label>Exigence d'étalonnage <select value={calibrationRequirement} onChange={(event) => setCalibrationRequirement(event.target.value as typeof calibrationRequirement)}><option value="required">Toujours requis</option><option value="if_used">Selon le matériel retenu</option><option value="not_required">Non requis pour ce rôle</option></select></label>
      {policy !== "exact_asset" && <label>Substitution autorisée <select value={substitutionPolicy} onChange={(event) => setSubstitutionPolicy(event.target.value as typeof substitutionPolicy)}><option value="no_substitution">Aucune</option><option value="same_exact_model">Même modèle exact</option><option value="same_category">Même catégorie</option><option value="same_capabilities">Mêmes aptitudes</option><option value="approved_equivalent">Équivalent approuvé</option></select></label>}
      {policy === "category_pool" && <>
        <label>Catégorie <Required /><select value={categoryId} onChange={(event) => setCategoryId(event.target.value)}><option value="">Sélectionner...</option>{flattenCategories(props.categories).map(({ category, depth }) => <option key={category.category_id} value={category.category_id}>{`${"— ".repeat(depth)}${category.label}`}</option>)}</select></label>
        <label className="checkboxLabel"><input type="checkbox" checked={acceptDescendants} onChange={(event) => setAcceptDescendants(event.target.checked)} /> Accepter les sous-catégories</label>
      </>}
      {policy === "capability_match" && <>
        <label>Aptitude stable <Required /><input value={capabilityKind} onChange={(event) => setCapabilityKind(event.target.value)} /></label>
        <label>Aptitude locale au modèle <input value={modelCapabilityId} onChange={(event) => setModelCapabilityId(event.target.value)} placeholder="Optionnelle, non portable" /></label>
        <label>Détecteurs ou modes <input value={detectorModes} onChange={(event) => setDetectorModes(event.target.value)} placeholder="peak, quasi_peak" /></label>
        <label>Fréquence minimale <input type="number" value={frequencyMin} onChange={(event) => setFrequencyMin(event.target.value)} /></label>
        <label>Fréquence maximale <input type="number" value={frequencyMax} onChange={(event) => setFrequencyMax(event.target.value)} /></label>
        <label>Unité <select value={frequencyUnit} onChange={(event) => setFrequencyUnit(event.target.value)}><option>Hz</option><option>kHz</option><option>MHz</option><option>GHz</option></select></label>
        <label>Action de driver requise <input value={driverAction} onChange={(event) => setDriverAction(event.target.value)} placeholder="Ex. measure_emission" /></label>
        <label>Communication requise <input value={communicationCapability} onChange={(event) => setCommunicationCapability(event.target.value)} placeholder="Ex. visa_tcpip" /></label>
        <label>Tension maximale requise (V)<input type="number" value={voltageMaximum} onChange={(event) => setVoltageMaximum(event.target.value)} /></label>
        <label>Courant maximal requis (A)<input type="number" value={currentMaximum} onChange={(event) => setCurrentMaximum(event.target.value)} /></label>
        <label>Puissance maximale requise (W)<input type="number" value={powerMaximum} onChange={(event) => setPowerMaximum(event.target.value)} /></label>
        <label>Impédance requise (Ω)<input type="number" value={impedanceOhm} onChange={(event) => setImpedanceOhm(event.target.value)} /></label>
      </>}
      {policy === "exact_asset" && <label>Exemplaire du parc <Required /><select value={exactAssetId} onChange={(event) => setExactAssetId(event.target.value)}><option value="">Sélectionner...</option>{props.fleet.map((asset) => <option key={asset.asset_id} value={asset.asset_id}>{assetLabel(asset)}</option>)}</select><small>Un exemplaire bloqué peut être imposé comme exigence, sans devenir apte à l'utilisation.</small></label>}
      <label>Port logique <Required /><input value={portLabel} onChange={(event) => setPortLabel(event.target.value)} /></label>
      <label>Direction <select value={directionality} onChange={(event) => setDirectionality(event.target.value as typeof directionality)}><option value="input">Entrée</option><option value="output">Sortie</option><option value="bidirectional">Bidirectionnel</option></select></label>
      <label>Domaine du signal <select value={signalDomain} onChange={(event) => setSignalDomain(event.target.value)}><option value="rf">Radiofréquence</option><option value="analog_voltage">Tension analogique</option><option value="analog_current">Courant analogique</option><option value="trigger">Déclenchement</option><option value="software">Logiciel</option></select></label>
      <label>Connecteur requis <input value={connector} onChange={(event) => setConnector(event.target.value)} /></label>
    </div>
    {missing.length > 0 && <p className="actionExplanation">Pour continuer, renseignez {new Intl.ListFormat("fr-FR", { type: "conjunction" }).format(missing)}.</p>}
    <footer><button className="secondary" type="button" onClick={props.onCancel}>Annuler</button><button type="button" disabled={missing.length > 0} onClick={submit}><Plus size={16} /> Ajouter au montage</button></footer>
  </section>;
}

function CandidatePanel(props: {
  requirement: StationMaterialRequirement | null;
  candidates: StationMaterialCandidate[];
  loading: boolean;
  error: string | null;
  dirty: boolean;
  readOnly: boolean;
  onAssign: (requirement: StationMaterialRequirement, candidate: StationMaterialCandidate) => void;
}) {
  if (!props.requirement) return null;
  return <section className="stationSection stationCandidatePanel">
    <div className="sectionTitleRow"><div><h3>Candidats pour {props.requirement.role_label}</h3><p>Compatibilité avec le besoin et aptitude pour l'utilisation prévue sont évaluées séparément.</p></div></div>
    {props.dirty && <p className="actionExplanation">Enregistrez d'abord l'exigence du montage.</p>}
    {props.loading && <p role="status">Recherche des exemplaires et contrôle des preuves...</p>}
    {props.error && <TargetedError title="Candidats temporairement indisponibles" detail={props.error} />}
    {!props.loading && !props.error && !props.dirty && props.candidates.length === 0 && <div className="compactEmpty"><strong>Aucun candidat</strong><span>Enregistrez ou rapprochez un exemplaire correspondant dans le parc matériel.</span></div>}
    <div className="candidateResultList">{props.candidates.map((candidate) => {
      const state = candidate.requirement_compatible
        ? candidate.operationally_eligible ? "compatible" : "blocked"
        : "incompatible";
      const title = candidate.requirement_compatible
        ? candidate.operationally_eligible ? "Compatible et disponible" : candidate.exact_asset_required ? "Exemplaire imposé mais non apte" : "Compatible mais indisponible"
        : candidate.compatibility_state === "indeterminate" ? "Compatibilité à confirmer" : "Incompatible avec les aptitudes requises";
      return <article key={candidate.asset.asset_id} className={state}>
        <header><div><strong>{assetLabel(candidate.asset)}</strong><small>{operatorCategoryPath(candidate.asset.category_code, candidate.asset.category_path).join(" › ")}</small></div><span className={`status ${state}`}>{title}</span></header>
        <div className="candidateEvidenceGrid" aria-label={`Aptitude de ${candidate.asset.inventory_code}`}>
          <div><span>Corrections</span><strong className={candidate.correction_readiness === "available" || candidate.correction_readiness === "not_required" ? "positive" : "negative"}>{correctionReadinessLabel(candidate)}</strong></div>
          <div><span>Étalonnage</span><strong className={candidate.asset.metrology.blocking ? "negative" : "positive"}>{metrologyCandidateLabel(candidate)}</strong></div>
          <div><span>Emplacement</span><strong className={candidate.asset.laboratory_location_id ? "positive" : "negative"}>{candidate.asset.laboratory_location_label ?? "Emplacement non défini"}</strong></div>
          <div><span>Aptitude globale</span><strong className={candidate.operationally_eligible ? "positive" : "negative"}>{candidate.operationally_eligible ? "Apte pour l'utilisation prévue" : "Non apte pour l'utilisation prévue"}</strong></div>
        </div>
        {candidate.compatibility_blockers.map((reason) => <p key={reason.code}>{reason.message} {reason.next_action && <span>{reason.next_action}</span>}</p>)}
        {candidate.operational_blockers.map((reason) => <p key={reason.code}>{reason.message} <span>{reason.next_action}</span></p>)}
        <button type="button" disabled={props.readOnly || !candidate.assignable} title={!candidate.assignable ? candidate.next_actions.join(" ") || title : undefined} onClick={() => props.onAssign(props.requirement!, candidate)}><CheckCircle2 size={15} /> Affecter pour l'utilisation prévue</button>
        {!candidate.assignable && <small className="actionExplanation">{candidate.next_actions[0] ?? title}</small>}
        <details><summary>Détails techniques</summary><dl><dt>Version de modèle</dt><dd>{candidate.asset.equipment_model_revision_id ?? "Non rapprochée"}</dd><dt>Ports compatibles</dt><dd>{Object.entries(candidate.logical_port_resolution_candidates).map(([logical, ports]) => `${logical}: ${ports.join(", ") || "aucun"}`).join(" · ")}</dd><dt>Driver</dt><dd>{candidate.driver_evidence.join(", ") || "Aucune action requise ou disponible"}</dd></dl></details>
      </article>;
    })}</div>
  </section>;
}

function LegacyBindings(props: { definition: StationMeasurementSetupDefinition; fleet: PhysicalAsset[] }) {
  return <section className="stationSection"><div className="sectionTitleRow"><div><h3>Matériels figés dans la version historique</h3><p>Ces affectations v2 restent immuables.</p></div></div><div className="stationBindingList">{props.definition.asset_bindings.map((binding) => <div key={binding.binding_id}><div><strong>{binding.role_label}</strong><span>{assetLabel(props.fleet.find((asset) => asset.asset_id === binding.asset_id))}</span></div></div>)}</div></section>;
}

function CreateStationDialog(props: {
  locations: LaboratoryLocation[];
  onCancel: () => void;
  onCreate: (input: { label: string; locationId: string; plannedUseOn: string; executionMode: "accredited" | "non_accredited" | "investigation" }) => Promise<void>;
}) {
  const [label, setLabel] = useState("");
  const [locationId, setLocationId] = useState("");
  const [plannedUseOn, setPlannedUseOn] = useState(today());
  const [executionMode, setExecutionMode] = useState<"accredited" | "non_accredited" | "investigation">("accredited");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const labelRef = useRef<HTMLInputElement>(null);
  const missing = [!label.trim() && "le nom du montage", !locationId && "le lieu", !plannedUseOn && "la date d'utilisation"].filter(Boolean) as string[];

  async function submit() {
    if (missing.length > 0) {
      if (!label.trim()) labelRef.current?.focus();
      else document.querySelector<HTMLSelectElement>("#station-location")?.focus();
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await props.onCreate({ label: label.trim(), locationId, plannedUseOn, executionMode });
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  return <section className="modalCard stationCreationDialog" role="dialog" aria-modal="true" aria-labelledby="create-station-title">
    <header><div><p className="eyebrow">Montages de mesure</p><h2 id="create-station-title">Préparer un montage</h2></div><button className="iconButton secondary" type="button" aria-label="Fermer" onClick={props.onCancel}><X size={16} /></button></header>
    <p className="contextBanner">Vous créez une définition datée. Les exemplaires pourront être choisis maintenant ou lors de la préparation de l'essai.</p>
    <div className="formGrid">
      <label>Nom du montage <Required /><input ref={labelRef} autoFocus value={label} onChange={(event) => setLabel(event.target.value)} placeholder="Ex. Banc d'émissions conduites" /></label>
      <label>Lieu du laboratoire <Required /><select id="station-location" value={locationId} onChange={(event) => setLocationId(event.target.value)}><option value="">Sélectionner...</option>{props.locations.map((location) => <option key={location.location_id} value={location.location_id}>{location.label}</option>)}</select></label>
      <label>Date d'utilisation <Required /><input type="date" value={plannedUseOn} onChange={(event) => setPlannedUseOn(event.target.value)} /></label>
      <label>Mode d'essai <Required /><select value={executionMode} onChange={(event) => setExecutionMode(event.target.value as typeof executionMode)}><option value="accredited">Sous accréditation</option><option value="non_accredited">Hors accréditation</option><option value="investigation">Investigation</option></select></label>
    </div>
    {missing.length > 0 && <p className="actionExplanation">Pour continuer, renseignez {new Intl.ListFormat("fr-FR", { type: "conjunction" }).format(missing)}.</p>}
    {error && <TargetedError title="Création refusée" detail={error} />}
    <footer><button className="secondary" type="button" onClick={props.onCancel}>Annuler</button><button type="button" disabled={busy || missing.length > 0} onClick={() => void submit()}><Plus size={16} /> {busy ? "Création..." : "Créer le brouillon"}</button></footer>
  </section>;
}

function activeRevision(setup: StationSetupAggregate) {
  return setup.active_draft_revision ?? setup.current_qualified_revision ?? setup.current_ready_revision ?? setup.latest_revision;
}
function revisionStatusLabel(status: string) { return ({ draft: "Brouillon", qualified: "Définition validée", ready: "Prêt", superseded: "Remplacé" })[status] ?? status; }
function revisionStatusLongLabel(status: string) { return ({ draft: "Brouillon", qualified: "Définition validée", ready: "Prêt à utiliser", superseded: "Version remplacée" })[status] ?? status; }
function selectionPolicyLabel(policy: StationMaterialSelectionPolicy) { return ({ category_pool: "Catégorie", capability_match: "Aptitudes techniques", exact_asset: "Exemplaire imposé" })[policy]; }
function assignmentStageLabel(stage: StationMaterialRequirement["assignment_stage"]) { return stage === "planned_test_preparation" ? "À choisir lors de la préparation de l'essai" : "À choisir dans le montage"; }
function substitutionLabel(policy: StationMaterialRequirement["substitution_policy"]) { return ({ no_substitution: "Aucune substitution", same_exact_model: "Même modèle exact", same_category: "Même catégorie", same_capabilities: "Mêmes aptitudes", approved_equivalent: "Équivalent approuvé" })[policy]; }
function requirementSummary(requirement: StationMaterialRequirement, categories: EquipmentCategory[], fleet: PhysicalAsset[]) {
  if (requirement.category_requirement) return `Catégorie : ${categoryName(categories, requirement.category_requirement.category_id)}${requirement.category_requirement.accept_descendants ? " et sous-catégories" : ""}`;
  if (requirement.capability_requirement) return `Aptitude : ${requirement.capability_requirement.capability_kind}${requirement.capability_requirement.frequency_range ? ` · ${rangeLabel(requirement.capability_requirement.frequency_range)}` : ""}`;
  return `Exemplaire : ${assetLabel(fleet.find((asset) => asset.asset_id === requirement.exact_asset_id))}`;
}
function categoryName(categories: EquipmentCategory[], categoryId: string): string { return flattenCategories(categories).find(({ category }) => category.category_id === categoryId)?.category.label ?? categoryId; }
function flattenCategories(categories: EquipmentCategory[], depth = 0): Array<{ category: EquipmentCategory; depth: number }> { return categories.flatMap((category) => [{ category, depth }, ...flattenCategories(category.children, depth + 1)]); }
function rangeLabel(range: { minimum?: number; maximum?: number; unit: string }) { return `${range.minimum ?? "…"} à ${range.maximum ?? "…"} ${range.unit}`; }
function correctionReadinessLabel(candidate: StationMaterialCandidate) { return ({ available: "Corrections requises disponibles", incomplete: "Corrections incomplètes", unavailable: "État des corrections indisponible", not_required: "Aucune correction requise" })[candidate.correction_readiness]; }
function metrologyCandidateLabel(candidate: StationMaterialCandidate) { return ({ valid: "Étalonnage valide", due_soon: "Étalonnage bientôt à échéance", expired: "Étalonnage expiré", missing: "Aucun étalonnage valide", not_required: "Étalonnage non requis", nonconforming: "Étalonnage non conforme", indeterminate: "Décision d'étalonnage indéterminée", unavailable: "État métrologique indisponible" })[candidate.asset.metrology.status] ?? candidate.asset.metrology.explanation; }
function assetLabel(asset?: PhysicalAsset) { return asset ? `${asset.inventory_code} · ${asset.manufacturer} ${operatorModelName(asset.category_code, asset.model_name, asset.manufacturer === "Demo")} · ${asset.serial_number || "Sans numéro de série"}` : "Exemplaire introuvable"; }
function readinessDimensionLabel(value: string) { return ({ structure: "Structure :", asset_identity: "Affectation :", serviceability: "État de service :", calibration_validity: "Étalonnage :", missing_evidence: "Preuve manquante :", nonconformance: "Non-conformité :", port_compatibility: "Ports :", correction_validity: "Corrections :" })[value] ?? "Aptitude :"; }
function optionalNumber(value: string) { const parsed = Number(value); return value.trim() && Number.isFinite(parsed) ? parsed : undefined; }
function splitTokens(value: string) { return value.split(",").map((token) => token.trim()).filter(Boolean); }
function Required() { return <span className="requiredBadge">Obligatoire</span>; }
function TargetedError(props: { title: string; detail: string }) { return <div className="targetedError"><AlertCircle size={17} /><div><strong>{props.title}</strong><p>{props.detail}</p></div></div>; }
function errorMessage(error: unknown) { return error instanceof Error ? error.message : "Erreur inattendue."; }
function today() { return new Date().toISOString().slice(0, 10); }
function formatDate(value: string) { const date = new Date(value.length === 10 ? `${value}T12:00:00` : value); return Number.isNaN(date.valueOf()) ? value : new Intl.DateTimeFormat("fr-FR", { dateStyle: "medium" }).format(date); }
