import {
  AlertCircle,
  CheckCircle2,
  ChevronRight,
  Plus,
  RefreshCw,
  Save,
  X
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { fleetApi, stationSetupApi, type OperationContext } from "../../api";
import { metrologyStatusLabel } from "../../metrologyStatus";
import { operatorCategoryPath, operatorModelName, operatorModelVariant } from "../../operatorEquipmentLabels";
import type {
  ExecutablePhysicalAssetOption,
  LaboratoryLocation,
  PhysicalAsset
} from "../../models/fleet";
import type {
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
  const [selectedId, setSelectedId] = useState("");
  const [listError, setListError] = useState<string | null>(null);
  const [locationsError, setLocationsError] = useState<string | null>(null);
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

  useEffect(() => {
    void loadSetups();
    void loadLocations();
  }, [loadLocations, loadSetups]);

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
          <p className="contextBanner">Vous préparez une chaîne de mesure avec des exemplaires du parc.</p>
          <h2>Montages de mesure</h2>
          <p>Matériels du laboratoire assemblés pour une date, un lieu et un mode d'essai précis.</p>
        </div>
        <div className="headerActions">
          <button className="iconButton secondary" type="button" onClick={() => void loadSetups()} title="Rafraîchir les montages" aria-label="Rafraîchir les montages"><RefreshCw size={16} /></button>
          <button type="button" onClick={() => setCreating(true)} disabled={Boolean(locationsError) || locations.length === 0}><Plus size={16} /> Nouveau montage</button>
        </div>
      </header>
      {locationsError && <TargetedError title="Lieux du laboratoire indisponibles" detail="Les montages existants restent consultables. La création et le changement de lieu sont suspendus." />}
      {!locationsError && locations.length === 0 && <p className="actionExplanation">Créez un lieu du laboratoire avant de préparer un montage.</p>}
      {listError && <TargetedError title="Montages temporairement indisponibles" detail={listError} />}

      <div className="stationLayout">
        <aside className="stationList">
          <div className="listHeader"><h3>Montages enregistrés</h3><span>{setups.length}</span></div>
          {setups.length === 0 && !listError && <div className="compactEmpty"><strong>Aucun montage</strong><span>Créez le premier montage à partir du parc matériel.</span></div>}
          {setups.map((setup) => {
            const revision = setup.active_draft_revision ?? setup.current_ready_revision ?? setup.latest_revision;
            return <button key={setup.identity.setup_id} type="button" className={setup.identity.setup_id === selectedId ? "active" : ""} onClick={() => setSelectedId(setup.identity.setup_id)}>
              <span><strong>{setup.identity.label}</strong><small>{revision.definition.laboratory_location_label} · {formatDate(revision.definition.planned_use_on)}</small></span>
              <span className={`status ${revision.status}`}>{revision.status === "ready" ? "Prêt" : revision.status === "draft" ? "Brouillon" : "Remplacé"}</span>
              <ChevronRight size={15} />
            </button>;
          })}
        </aside>
        <StationSetupDetail
          setup={selected}
          locations={locations}
          locationsError={locationsError}
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
  locationsError: string | null;
  onReplace: (setup: StationSetupAggregate) => void;
}) {
  const revision = props.setup?.active_draft_revision ?? props.setup?.current_ready_revision ?? props.setup?.latest_revision ?? null;
  const [definition, setDefinition] = useState<StationMeasurementSetupDefinition | null>(revision?.definition ?? null);
  const [readiness, setReadiness] = useState<StationSetupReadiness | null>(revision?.readiness ?? null);
  const [assetOptions, setAssetOptions] = useState<ExecutablePhysicalAssetOption[]>([]);
  const [loadedAssetOptionsContextKey, setLoadedAssetOptionsContextKey] = useState("");
  const [assetsLoading, setAssetsLoading] = useState(false);
  const [assetsError, setAssetsError] = useState<string | null>(null);
  const [roleLabel, setRoleLabel] = useState("");
  const [assetId, setAssetId] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const roleRef = useRef<HTMLInputElement>(null);
  const assetOptionsRequestSequence = useRef(0);
  const knownAssets = useRef(new Map<string, PhysicalAsset>());

  useEffect(() => {
    setDefinition(revision?.definition ?? null);
    setReadiness(revision?.readiness ?? null);
    setError(null);
  }, [revision]);

  const plannedUseOn = definition?.planned_use_on ?? "";
  const executionMode = definition?.execution_mode ?? "accredited";
  const laboratoryLocationId = definition?.laboratory_location_id ?? "";
  const assetOptionsContextKey = stationAssetOptionsContextKey(
    plannedUseOn,
    executionMode,
    laboratoryLocationId
  );

  useEffect(() => {
    const requestSequence = ++assetOptionsRequestSequence.current;
    let cancelled = false;
    setAssetId("");
    setAssetsError(null);
    setLoadedAssetOptionsContextKey("");
    if (!plannedUseOn || !laboratoryLocationId) {
      setAssetOptions([]);
      setAssetsLoading(false);
      return () => { cancelled = true; };
    }
    setAssetsLoading(true);
    void stationSetupApi.assetOptions({
      planned_use_on: plannedUseOn,
      execution_mode: executionMode,
      laboratory_location_id: laboratoryLocationId
    }).then((response) => {
      if (cancelled || requestSequence !== assetOptionsRequestSequence.current) return;
      response.assets.forEach((option) => knownAssets.current.set(option.asset.asset_id, option.asset));
      setAssetOptions(response.assets);
      setLoadedAssetOptionsContextKey(assetOptionsContextKey);
      setAssetsLoading(false);
      setAssetsError(null);
    }).catch((reason) => {
      if (cancelled || requestSequence !== assetOptionsRequestSequence.current) return;
      setAssetsLoading(false);
      setAssetsError(errorMessage(reason));
    });
    return () => { cancelled = true; };
  }, [assetOptionsContextKey, executionMode, laboratoryLocationId, plannedUseOn]);

  const dirty = Boolean(definition && revision && JSON.stringify(definition) !== JSON.stringify(revision.definition));
  const selectedAssetIds = new Set(definition?.asset_bindings.map((binding) => binding.asset_id) ?? []);
  const assetOptionsAreCurrent = Boolean(assetOptionsContextKey && loadedAssetOptionsContextKey === assetOptionsContextKey);
  const currentAssetOptions = assetOptionsAreCurrent ? assetOptions : [];
  const availableOptions = currentAssetOptions.filter((option) => !selectedAssetIds.has(option.asset.asset_id));
  const selectedAssetOption = currentAssetOptions.find((option) => option.asset.asset_id === assetId);
  const selectedAsset = selectedAssetOption?.asset;
  const canAdd = Boolean(roleLabel.trim() && selectedAssetOption?.eligible);

  if (!props.setup || !revision || !definition) {
    return <article className="stationDetail empty"><strong>Aucun montage ouvert</strong><p>Sélectionnez un montage ou créez-en un nouveau.</p></article>;
  }

  const readOnly = revision.status !== "draft";

  function addAsset() {
    if (!canAdd || !selectedAsset?.equipment_model_id || !selectedAsset.equipment_model_revision_id || !selectedAsset.equipment_model_checksum) {
      roleRef.current?.focus();
      return;
    }
    const equipmentModelId = selectedAsset.equipment_model_id;
    const equipmentModelRevisionId = selectedAsset.equipment_model_revision_id;
    const equipmentModelChecksum = selectedAsset.equipment_model_checksum;
    setDefinition((current) => current ? {
      ...current,
      asset_bindings: [...current.asset_bindings, {
        binding_id: `material-${crypto.randomUUID().slice(0, 8)}`,
        role_label: roleLabel.trim(),
        asset_id: selectedAsset.asset_id,
        asset_revision: String(selectedAsset.revision),
        equipment_model_id: equipmentModelId,
        equipment_model_revision_id: equipmentModelRevisionId,
        equipment_model_checksum: equipmentModelChecksum
      }]
    } : current);
    setRoleLabel("");
    setAssetId("");
    setReadiness(null);
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
    if (!props.setup || !revision || !definition) return;
    const result = await stationSetupApi.replaceDraft(
      props.setup.identity.setup_id,
      revision.revision_id,
      revision.definition_checksum,
      definition,
      operationContext
    );
    props.onReplace(result.station_setup);
  }

  async function assess() {
    if (!props.setup || !revision) return;
    const result = await stationSetupApi.assess(props.setup.identity.setup_id, revision.revision_id);
    setReadiness(result.readiness);
  }

  async function markReady() {
    if (!props.setup || !revision) return;
    const result = await stationSetupApi.markReady(
      props.setup.identity.setup_id,
      revision.revision_id,
      revision.definition_checksum,
      operationContext
    );
    props.onReplace(result.station_setup);
  }

  return <article className="stationDetail">
    <header className="stationIdentityHeader">
      <div><p className="eyebrow">Montage de mesure</p><h2>{definition.label}</h2><p>{definition.laboratory_location_label} · utilisation prévue le {formatDate(definition.planned_use_on)}</p></div>
      <span className={`status ${revision.status}`}>{revision.status === "ready" ? "Prêt à utiliser" : revision.status === "draft" ? "À préparer" : "Version remplacée"}</span>
    </header>

    {assetsError && <TargetedError title="Aptitude du parc temporairement indisponible" detail="Le montage et ses affectations restent consultables. L'ajout d'un exemplaire est suspendu jusqu'au prochain contrôle." />}

    <section className="stationSection">
      <div className="sectionTitleRow"><div><h3>Contexte d'utilisation</h3><p>Le lieu et la date servent au contrôle métrologique du montage.</p></div></div>
      <div className="formGrid">
        <label>Nom du montage <Required /><input value={definition.label} disabled={readOnly} onChange={(event) => setDefinition({ ...definition, label: event.target.value })} /></label>
        <label>Lieu du laboratoire <Required /><select value={definition.laboratory_location_id ?? ""} disabled={readOnly || Boolean(props.locationsError)} onChange={(event) => { const location = props.locations.find((candidate) => candidate.location_id === event.target.value); setDefinition({ ...definition, laboratory_location_id: location?.location_id ?? null, laboratory_location_label: location?.label ?? "" }); }}><option value="">Sélectionner...</option>{props.locations.map((location) => <option key={location.location_id} value={location.location_id}>{location.label}</option>)}</select></label>
        <label>Date d'utilisation <Required /><input type="date" value={definition.planned_use_on} disabled={readOnly} onChange={(event) => setDefinition({ ...definition, planned_use_on: event.target.value })} /></label>
        <label>Mode d'essai <Required /><select value={definition.execution_mode} disabled={readOnly} onChange={(event) => setDefinition({ ...definition, execution_mode: event.target.value as StationMeasurementSetupDefinition["execution_mode"] })}><option value="accredited">Sous accréditation</option><option value="non_accredited">Hors accréditation</option><option value="investigation">Investigation</option></select></label>
      </div>
    </section>

    <section className="stationSection">
      <div className="sectionTitleRow"><div><h3>Matériels du laboratoire</h3><p>Seuls les exemplaires réels du parc peuvent être affectés au montage.</p></div><span className="countBadge">{definition.asset_bindings.length}</span></div>
      {definition.asset_bindings.length === 0 && <div className="compactEmpty"><strong>Aucun matériel affecté</strong><span>Choisissez le rôle tenu dans la chaîne puis un exemplaire du parc.</span></div>}
      <div className="stationBindingList">{definition.asset_bindings.map((binding) => {
        const asset = currentAssetOptions.find((candidate) => candidate.asset.asset_id === binding.asset_id)?.asset
          ?? knownAssets.current.get(binding.asset_id);
        return <div key={binding.binding_id}>
          <div><strong>{binding.role_label}</strong><span>{asset ? assetOperatorLabel(asset) : "Exemplaire momentanément introuvable dans le parc"}</span></div>
          {!readOnly && <button className="iconButton secondary" type="button" aria-label={`Retirer ${binding.role_label}`} onClick={() => { setDefinition({ ...definition, asset_bindings: definition.asset_bindings.filter((candidate) => candidate.binding_id !== binding.binding_id) }); setReadiness(null); }}><X size={15} /></button>}
        </div>;
      })}</div>

      {!readOnly && <div className="stationAssetPicker">
        <label>Rôle dans le montage <Required /><input ref={roleRef} value={roleLabel} onChange={(event) => setRoleLabel(event.target.value)} placeholder="Ex. Récepteur EMI" /></label>
        <label>Exemplaire du parc <Required /><select value={assetId} disabled={assetsLoading || Boolean(assetsError) || !assetOptionsAreCurrent} onChange={(event) => setAssetId(event.target.value)}><option value="">Sélectionner...</option>{assetOptionGroups(availableOptions)}</select></label>
        <button type="button" onClick={addAsset} disabled={!canAdd || assetsLoading || Boolean(assetsError) || !assetOptionsAreCurrent}><Plus size={15} /> Affecter au montage</button>
      </div>}
      {!readOnly && assetsLoading && <p className="actionExplanation" role="status">Actualisation des exemplaires disponibles pour ce lieu, cette date et ce mode d'essai...</p>}
      {!readOnly && !assetsLoading && (!plannedUseOn || !laboratoryLocationId) && <p className="actionExplanation">Renseignez le lieu et la date d'utilisation pour contrôler les exemplaires disponibles.</p>}
      {!readOnly && selectedAssetOption && selectedAssetOption.warnings.length > 0 && <SelectionReasons title="Points d'attention" reasons={selectedAssetOption.warnings} />}
      {!readOnly && !assetsLoading && assetOptionsAreCurrent && !canAdd && <p className="actionExplanation">Renseignez le rôle et choisissez un exemplaire déclaré disponible pour ce lieu, cette date et ce mode d'essai.</p>}
      {!readOnly && !assetsLoading && assetOptionsAreCurrent && !assetsError && availableOptions.length === 0 && <p className="actionExplanation">Aucun autre exemplaire n'est enregistré dans le parc. Ajoutez l'exemplaire requis depuis un modèle constructeur approuvé.</p>}
      {!readOnly && availableOptions.some((option) => !option.eligible) && <UnavailableAssetExplanations options={availableOptions.filter((option) => !option.eligible)} />}
    </section>

    <section className={`stationReadinessPanel ${readiness?.ready ? "ready" : "blocked"}`}>
      <div>{readiness?.ready ? <CheckCircle2 size={19} /> : <AlertCircle size={19} />}<div><h3>{readiness?.ready ? "Montage apte à être utilisé" : "Points à corriger avant utilisation"}</h3><p>{readiness ? `Contrôle d’aptitude du ${formatDate(readiness.checked_on)}.` : "Enregistrez les modifications avant de contrôler l’aptitude."}</p></div></div>
      {readiness && readiness.issues.length > 0 && <ul>{readiness.issues.map((issue) => <li key={`${issue.code}-${issue.binding_ids?.join("-") ?? "setup"}`}><strong>{readinessDimensionLabel(issue.dimension)}</strong> {issue.message}</li>)}</ul>}
    </section>

    {error && <TargetedError title="Opération refusée" detail={`${error} Corrigez le montage puis contrôlez de nouveau son aptitude.`} />}
    <div className="stationActions">
      {!readOnly && <button type="button" disabled={busy || !dirty || !definition.label.trim() || !definition.laboratory_location_id} onClick={() => void run(save)}><Save size={16} /> Enregistrer le brouillon</button>}
      {!readOnly && !dirty && <button className="secondary" type="button" disabled={busy} onClick={() => void run(assess)}><RefreshCw size={16} /> Contrôler l’aptitude du montage</button>}
      {!readOnly && <button type="button" disabled={busy || dirty || !readiness?.ready} onClick={() => void run(markReady)}><CheckCircle2 size={16} /> Déclarer le montage prêt</button>}
    </div>
    {!readOnly && dirty && <p className="actionExplanation">Enregistrez le brouillon avant de relancer le contrôle d'aptitude.</p>}
    {!readOnly && !dirty && !readiness?.ready && <p className="actionExplanation">Corrigez chaque point bloquant avant de déclarer le montage prêt.</p>}
    <details><summary>Détails techniques</summary><dl><dt>Identifiant interne</dt><dd>{props.setup.identity.setup_id}</dd><dt>Révision</dt><dd>{revision.revision_id}</dd><dt>Empreinte</dt><dd className="technicalValue">{revision.definition_checksum}</dd></dl></details>
  </article>;
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
    <p className="contextBanner">Vous créez une configuration datée utilisant des exemplaires du parc.</p>
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

function assetOptionGroups(options: ExecutablePhysicalAssetOption[]) {
  const groups = new Map<string, PhysicalAsset[]>();
  for (const { asset, eligible } of options) {
    if (!eligible) continue;
    const demo = asset.manufacturer === "Demo";
    const variant = operatorModelVariant(asset.variant, demo);
    const key = `${categoryPath(asset)} · ${asset.manufacturer} ${operatorModelName(asset.category_code, asset.model_name, demo)}${variant ? ` ${variant}` : ""}`;
    groups.set(key, [...(groups.get(key) ?? []), asset]);
  }
  const eligibleGroups = Array.from(groups.entries()).sort(([left], [right]) => left.localeCompare(right, "fr")).map(([label, rows]) =>
    <optgroup key={label} label={label}>{rows.map((asset) => <option key={asset.asset_id} value={asset.asset_id}>{assetOperatorLabel(asset)}</option>)}</optgroup>
  );
  const unavailable = options.filter((option) => !option.eligible);
  return <>{eligibleGroups}{unavailable.length > 0 && <optgroup label="Matériels non disponibles">{unavailable.map((option) => <option key={option.asset.asset_id} value={option.asset.asset_id} disabled>{categoryPath(option.asset)} · {option.asset.manufacturer} {operatorModelName(option.asset.category_code, option.asset.model_name, option.asset.manufacturer === "Demo")} · {option.asset.inventory_code} · {option.blocking_reasons[0]?.message ?? "Non disponible"}</option>)}</optgroup>}</>;
}

function assetOperatorLabel(asset: PhysicalAsset) {
  return `${asset.inventory_code} · ${asset.serial_number || "Sans numéro de série"} · ${asset.laboratory_location_label || "Sans emplacement"} · ${serviceLabel(asset.service_state)} · ${availabilityLabel(asset.availability_state)} · ${metrologyLabel(asset)}`;
}

function categoryPath(asset: PhysicalAsset) {
  return operatorCategoryPath(asset.category_code, asset.category_path).join(" > ");
}

function serviceLabel(value: PhysicalAsset["service_state"]) {
  return ({ usable: "Utilisable", restricted: "Utilisation restreinte", in_maintenance: "En maintenance", out_of_service: "Hors service", retired: "Retiré du parc" })[value];
}

function availabilityLabel(value: PhysicalAsset["availability_state"]) {
  return ({ available: "Disponible", reserved: "Réservé", assigned_to_setup: "Affecté à un montage", in_test: "Utilisé en essai", unavailable: "Indisponible" })[value];
}

function metrologyLabel(asset: PhysicalAsset) {
  return metrologyStatusLabel(asset.metrology);
}

function readinessDimensionLabel(value: string) {
  return ({ structure: "Structure :", asset_identity: "Identité :", serviceability: "État de service :", calibration_validity: "Étalonnage :", missing_evidence: "Preuve manquante :", nonconformance: "Non-conformité :", port_compatibility: "Connexions :", correction_validity: "Correction :" })[value] ?? "Contrôle d’aptitude :";
}

function SelectionReasons(props: { title: string; reasons: ExecutablePhysicalAssetOption["warnings"] }) {
  return <div className="selectionReasonPanel"><strong>{props.title}</strong><ul>{props.reasons.map((reason) => <li key={reason.code}>{reason.message} <span>{reason.next_action}</span></li>)}</ul></div>;
}

function UnavailableAssetExplanations(props: { options: ExecutablePhysicalAssetOption[] }) {
  const visible = props.options.slice(0, 20);
  return <details className="unavailableAssetExplanations"><summary>Matériels non disponibles ({props.options.length})</summary><div>{visible.map((option) => <article key={option.asset.asset_id}><strong>{option.asset.inventory_code} · {option.asset.manufacturer} {operatorModelName(option.asset.category_code, option.asset.model_name, option.asset.manufacturer === "Demo")}</strong>{option.blocking_reasons.map((reason) => <p key={reason.code}>{reason.message} <span>{reason.next_action}</span></p>)}</article>)}</div>{props.options.length > visible.length && <p>{props.options.length - visible.length} autre(s) exemplaire(s) restent visibles dans la liste de sélection.</p>}</details>;
}

function Required() { return <span className="requiredBadge">Obligatoire</span>; }
function TargetedError(props: { title: string; detail: string }) { return <div className="targetedError"><AlertCircle size={17} /><div><strong>{props.title}</strong><p>{props.detail}</p></div></div>; }
function errorMessage(error: unknown) { return error instanceof Error ? error.message : "Erreur inattendue."; }
function stationAssetOptionsContextKey(plannedUseOn: string, executionMode: string, laboratoryLocationId: string) {
  return JSON.stringify({ planned_use_on: plannedUseOn, execution_mode: executionMode, laboratory_location_id: laboratoryLocationId });
}
function today() { return new Date().toISOString().slice(0, 10); }
function formatDate(value: string) { const date = new Date(value.length === 10 ? `${value}T12:00:00` : value); return Number.isNaN(date.valueOf()) ? value : new Intl.DateTimeFormat("fr-FR", { dateStyle: "medium" }).format(date); }
