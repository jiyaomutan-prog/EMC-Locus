import {
  AlertTriangle,
  ChevronDown,
  ChevronRight,
  PackagePlus,
  RefreshCw,
  Search,
  Wrench,
  X
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { fleetApi, type OperationContext } from "../../api";
import type { EquipmentCategory, EquipmentModelAggregate } from "../../models/equipment";
import type {
  AvailabilityState,
  CreatePhysicalAssetInput,
  LaboratoryLocation,
  OwnershipSource,
  PhysicalAsset,
  ServiceState
} from "../../models/fleet";

type GroupingMode = "category" | "location" | "service" | "metrology";

interface FleetWorkspaceProps {
  models: EquipmentModelAggregate[];
  categories: EquipmentCategory[];
  modelLoadError?: string | null;
  initialModelId?: string | null;
  initialViewModelId?: string | null;
  onInitialModelHandled?: () => void;
  onInitialViewModelHandled?: () => void;
  onOpenPinnedModel: (modelId: string, revisionId: string) => void;
  onOpenMetrology: (assetId: string) => void;
}

const context: OperationContext = {
  actor: "fleet.operator",
  reason: "gestion du parc depuis LAB CONSOLE"
};

export function FleetWorkspace(props: FleetWorkspaceProps) {
  const {
    initialModelId,
    initialViewModelId,
    onInitialModelHandled,
    onInitialViewModelHandled
  } = props;
  const [assets, setAssets] = useState<PhysicalAsset[]>([]);
  const [locations, setLocations] = useState<LaboratoryLocation[]>([]);
  const [assetsLoading, setAssetsLoading] = useState(true);
  const [assetsError, setAssetsError] = useState<string | null>(null);
  const [locationsError, setLocationsError] = useState<string | null>(null);
  const [selectedAssetId, setSelectedAssetId] = useState("");
  const [grouping, setGrouping] = useState<GroupingMode>("category");
  const [query, setQuery] = useState("");
  const [modelFilterId, setModelFilterId] = useState(initialViewModelId ?? "");
  const [creationOpen, setCreationOpen] = useState(Boolean(initialModelId));
  const approvedModels = props.models.filter((model) => model.current_approved_revision);

  const loadAssets = useCallback(async () => {
    setAssetsLoading(true);
    try {
      const response = await fleetApi.listAssets();
      setAssets(response.assets);
      setAssetsError(null);
      setSelectedAssetId((current) =>
        response.assets.some((asset) => asset.asset_id === current)
          ? current
          : response.assets[0]?.asset_id ?? ""
      );
    } catch (error) {
      setAssetsError(errorMessage(error));
    } finally {
      setAssetsLoading(false);
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
    void loadAssets();
    void loadLocations();
  }, [loadAssets, loadLocations]);

  useEffect(() => {
    if (initialModelId) {
      setCreationOpen(true);
      onInitialModelHandled?.();
    }
  }, [initialModelId, onInitialModelHandled]);

  useEffect(() => {
    if (initialViewModelId) {
      setModelFilterId(initialViewModelId);
      onInitialViewModelHandled?.();
    }
  }, [initialViewModelId, onInitialViewModelHandled]);

  const filteredAssets = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase("fr");
    return assets.filter((asset) =>
      (!modelFilterId || asset.equipment_model_id === modelFilterId)
      && (!normalized || searchText(asset).includes(normalized))
    );
  }, [assets, modelFilterId, query]);
  useEffect(() => {
    if (!filteredAssets.some((asset) => asset.asset_id === selectedAssetId)) {
      setSelectedAssetId(filteredAssets[0]?.asset_id ?? "");
    }
  }, [filteredAssets, selectedAssetId]);
  const selectedAsset = filteredAssets.find((asset) => asset.asset_id === selectedAssetId) ?? null;

  async function createAsset(input: CreatePhysicalAssetInput) {
    try {
      const response = await fleetApi.createAsset(input, context);
      setCreationOpen(false);
      await loadAssets();
      setSelectedAssetId(response.asset.asset_id);
    } catch (error) {
      throw new Error(errorMessage(error));
    }
  }

  function replaceAsset(asset: PhysicalAsset) {
    setAssets((current) => current.map((candidate) => candidate.asset_id === asset.asset_id ? asset : candidate));
  }

  async function updateAsset(asset: PhysicalAsset, input: Parameters<typeof fleetApi.updateAsset>[1]) {
    const response = await fleetApi.updateAsset(asset, input, context);
    replaceAsset(response.asset);
  }

  async function transitionServiceState(asset: PhysicalAsset, state: ServiceState, reason: string) {
    const response = await fleetApi.transitionServiceState(asset, state, reason, context);
    replaceAsset(response.asset);
  }

  async function transitionAvailability(asset: PhysicalAsset, state: AvailabilityState) {
    const response = await fleetApi.transitionAvailability(asset, state, context);
    replaceAsset(response.asset);
  }

  return (
    <section className="fleetWorkspace" aria-label="Parc matériel">
      <header className="resourcePageHeader">
        <div>
          <p className="contextBanner">Vous consultez un exemplaire du parc.</p>
          <h2>Parc matériel</h2>
          <p>Exemplaires physiques ou logiciels réellement disponibles dans le laboratoire.</p>
        </div>
        <div className="headerActions">
          <button className="iconButton secondary" type="button" onClick={() => void loadAssets()} title="Rafraîchir le parc" aria-label="Rafraîchir le parc">
            <RefreshCw size={16} />
          </button>
          <button type="button" onClick={() => setCreationOpen(true)} disabled={approvedModels.length === 0}>
            <PackagePlus size={16} /> Ajouter un exemplaire
          </button>
        </div>
      </header>

      {approvedModels.length === 0 && (
        <p className="actionExplanation">Approuvez un modèle constructeur avant d'ajouter un exemplaire au parc.</p>
      )}
      {props.modelLoadError && <TargetedError title="Catalogue des modèles indisponible" detail={props.modelLoadError} />}
      {assetsError && <TargetedError title="Parc temporairement indisponible" detail={assetsError} />}

      <div className="fleetToolbar">
        <label className="searchBox">
          <Search size={16} />
          <input aria-label="Rechercher dans le parc" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Code inventaire, série, modèle ou lieu" />
        </label>
        <div className="segmentedControl" role="group" aria-label="Regrouper le parc">
          {([
            ["category", "Par catégorie"],
            ["location", "Par emplacement"],
            ["service", "Par état de service"],
            ["metrology", "Par échéance métrologique"]
          ] as Array<[GroupingMode, string]>).map(([mode, label]) => (
            <button key={mode} type="button" className={grouping === mode ? "active" : ""} onClick={() => setGrouping(mode)}>{label}</button>
          ))}
        </div>
      </div>
      {modelFilterId && (
        <div className="activeFilterBanner">
          <span>Exemplaires du modèle <strong>{modelLabel(props.models, modelFilterId)}</strong></span>
          <button className="textButton" type="button" onClick={() => setModelFilterId("")}>Voir tout le parc</button>
        </div>
      )}

      {locationsError && (
        <TargetedError title="Emplacements indisponibles" detail={`Les exemplaires restent consultables. ${locationsError}`} />
      )}
      <div className="fleetLayout">
        <aside className="fleetTreePanel">
          <div className="listHeader"><h3>Exemplaires</h3><span>{filteredAssets.length}</span></div>
          {assetsLoading && assets.length === 0 ? (
            <p className="muted">Lecture du parc...</p>
          ) : filteredAssets.length === 0 ? (
            <div className="compactEmpty"><strong>Aucun exemplaire</strong><span>Ajoutez un exemplaire depuis un modèle approuvé.</span></div>
          ) : (
            <FleetTree assets={filteredAssets} grouping={grouping} selectedAssetId={selectedAssetId} onSelect={setSelectedAssetId} />
          )}
        </aside>
        <AssetDetail
          asset={selectedAsset}
          locations={locations}
          locationsError={locationsError}
          onUpdate={updateAsset}
          onTransitionService={transitionServiceState}
          onTransitionAvailability={transitionAvailability}
          onOpenPinnedModel={props.onOpenPinnedModel}
          onOpenMetrology={props.onOpenMetrology}
        />
      </div>

      {creationOpen && (
        <div className="modalBackdrop">
          <CreateAssetDialog
            models={approvedModels}
            locations={locations}
            locationsError={locationsError}
            initialModelId={initialModelId}
            onCancel={() => setCreationOpen(false)}
            onCreate={createAsset}
          />
        </div>
      )}
    </section>
  );
}

function FleetTree(props: { assets: PhysicalAsset[]; grouping: GroupingMode; selectedAssetId: string; onSelect: (assetId: string) => void }) {
  const [expanded, setExpanded] = useState<Set<string>>(() => {
    try {
      return new Set(JSON.parse(localStorage.getItem("emc-locus.fleet-expanded") ?? "[]") as string[]);
    } catch {
      return new Set();
    }
  });
  const groups = buildGroups(props.assets, props.grouping);
  const categoryHierarchy = buildFleetCategoryHierarchy(props.assets);

  function toggle(key: string) {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key); else next.add(key);
      localStorage.setItem("emc-locus.fleet-expanded", JSON.stringify(Array.from(next)));
      return next;
    });
  }

  return (
    <div className="fleetTree" role="tree" aria-label="Hiérarchie du parc">
      {props.grouping === "category" && categoryHierarchy.map((branch) => renderFleetCategoryBranch(
        branch,
        categoryHierarchy.length === 1,
        expanded,
        toggle,
        props.selectedAssetId,
        props.onSelect
      ))}
      {props.grouping !== "category" && groups.map((group) => {
        const open = expanded.has(group.key) || groups.length === 1;
        return (
          <div key={group.key}>
            <button className="fleetGroup" type="button" role="treeitem" aria-expanded={open} onClick={() => toggle(group.key)}>
              {open ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
              <span>{group.label}</span><small>{group.assets.length}</small>
            </button>
            {open && <div role="group">{renderModelGroups(group.assets, expanded, toggle, props.selectedAssetId, props.onSelect, group.key)}</div>}
          </div>
        );
      })}
    </div>
  );
}

interface FleetCategoryBranch {
  key: string;
  label: string;
  path: string[];
  assets: PhysicalAsset[];
  children: FleetCategoryBranch[];
}

function buildFleetCategoryHierarchy(assets: PhysicalAsset[]): FleetCategoryBranch[] {
  interface MutableBranch {
    key: string;
    label: string;
    path: string[];
    assets: PhysicalAsset[];
    children: Map<string, MutableBranch>;
  }
  const roots = new Map<string, MutableBranch>();
  for (const asset of assets) {
    const categorySegments = asset.category_path.filter((segment) => segment && segment !== "Général");
    const segments = categorySegments.length > 0
      ? categorySegments
      : [asset.category_code || "Sans catégorie"];
    let siblings = roots;
    let branch: MutableBranch | null = null;
    const path: string[] = [];
    for (const segment of segments) {
      path.push(segment);
      branch = siblings.get(segment) ?? {
        key: `category:${path.join("/")}`,
        label: segment,
        path: [...path],
        assets: [],
        children: new Map()
      };
      siblings.set(segment, branch);
      siblings = branch.children;
    }
    if (branch) branch.assets.push(asset);
  }

  function materialize(branches: Map<string, MutableBranch>): FleetCategoryBranch[] {
    return Array.from(branches.values())
      .sort((left, right) => left.label.localeCompare(right.label, "fr"))
      .map((branch) => ({
        key: branch.key,
        label: branch.label,
        path: branch.path,
        assets: branch.assets,
        children: materialize(branch.children)
      }));
  }
  return materialize(roots);
}

function renderFleetCategoryBranch(
  branch: FleetCategoryBranch,
  onlySibling: boolean,
  expanded: Set<string>,
  toggle: (key: string) => void,
  selectedAssetId: string,
  onSelect: (assetId: string) => void
) {
  const open = expanded.has(branch.key) || onlySibling;
  const count = branch.assets.length + branch.children.reduce((total, child) => total + fleetBranchCount(child), 0);
  return <div key={branch.key} className="fleetCategoryBranch">
    <button className="fleetGroup category" type="button" role="treeitem" aria-expanded={open} onClick={() => toggle(branch.key)}>
      {open ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
      <span>{branch.label}</span><small>{count}</small>
    </button>
    {open && <div role="group">
      {branch.children.map((child) => renderFleetCategoryBranch(
        child,
        branch.children.length === 1 && branch.assets.length === 0,
        expanded,
        toggle,
        selectedAssetId,
        onSelect
      ))}
      {renderModelGroups(branch.assets, expanded, toggle, selectedAssetId, onSelect, branch.key)}
    </div>}
  </div>;
}

function fleetBranchCount(branch: FleetCategoryBranch): number {
  return branch.assets.length + branch.children.reduce((total, child) => total + fleetBranchCount(child), 0);
}

function renderModelGroups(
  assets: PhysicalAsset[],
  expanded: Set<string>,
  toggle: (key: string) => void,
  selectedAssetId: string,
  onSelect: (assetId: string) => void,
  parentKey: string
) {
  const models = groupBy(assets, (asset) => `${asset.manufacturer} ${asset.model_name}${asset.variant ? ` ${asset.variant}` : ""}`);
  return Array.from(models.entries()).sort(([left], [right]) => left.localeCompare(right, "fr")).map(([model, modelAssets]) => {
    const modelKey = `${parentKey}:model:${model}`;
    const open = expanded.has(modelKey) || models.size === 1;
    return (
    <div className="fleetModelGroup" key={modelKey}>
      <button className="fleetModelNode" type="button" role="treeitem" aria-expanded={open} onClick={() => toggle(modelKey)}>
        {open ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        <strong>{model}</strong><small>{modelAssets.length}</small>
      </button>
      {open && <div className="fleetAssetNodes" role="group">{modelAssets.map((asset) => (
        <button key={asset.asset_id} type="button" role="treeitem" className={asset.asset_id === selectedAssetId ? "active" : ""} onClick={() => onSelect(asset.asset_id)}>
          <span><b>{asset.inventory_code}</b> · {asset.serial_number || "Sans numéro de série"}</span>
          <small>{asset.laboratory_location_label || "Emplacement non défini"}</small>
          <span className="assetRowStates"><span className={`status ${asset.service_state}`}>{serviceStateLabel(asset.service_state)}</span><span>{availabilityLabel(asset.availability_state)}</span><span>{metrologyLabel(asset)}</span></span>
        </button>
      ))}</div>}
    </div>
  );
  });
}

function AssetDetail(props: {
  asset: PhysicalAsset | null;
  locations: LaboratoryLocation[];
  locationsError: string | null;
  onUpdate: (asset: PhysicalAsset, input: Parameters<typeof fleetApi.updateAsset>[1]) => Promise<void>;
  onTransitionService: (asset: PhysicalAsset, state: ServiceState, reason: string) => Promise<void>;
  onTransitionAvailability: (asset: PhysicalAsset, state: AvailabilityState) => Promise<void>;
  onOpenPinnedModel: (modelId: string, revisionId: string) => void;
  onOpenMetrology: (assetId: string) => void;
}) {
  const [tab, setTab] = useState("summary");
  if (!props.asset) return <div className="assetDetail empty"><strong>Aucun exemplaire ouvert</strong><p>Sélectionnez un exemplaire dans la hiérarchie du parc.</p></div>;
  const asset = props.asset;
  return (
    <article className="assetDetail">
      <header className="assetIdentityHeader">
        <div><p className="eyebrow">Exemplaire du parc</p><h2>{asset.inventory_code}</h2><p>{asset.manufacturer} {asset.model_name}{asset.variant ? ` ${asset.variant}` : ""}</p></div>
        <div className="identityFacts">
          <IdentityFact label="Numéro de série" value={asset.serial_number || "Sans numéro de série"} />
          <IdentityFact label="Catégorie" value={categoryPath(asset)} />
          <IdentityFact label="Emplacement" value={asset.laboratory_location_label || "Non défini"} />
          <IdentityFact label="État de service" value={serviceStateLabel(asset.service_state)} />
          <IdentityFact label="Disponibilité" value={availabilityLabel(asset.availability_state)} />
          <IdentityFact label="Métrologie" value={metrologyLabel(asset)} />
        </div>
      </header>
      <nav className="detailTabs" aria-label="Détail de l'exemplaire">
        {[["summary", "Résumé"], ["identification", "Identification"], ["location", "Emplacement et disponibilité"], ["metrology", "Métrologie"], ["history", "Historique"], ["technical", "Détails techniques"]].map(([key, label]) => (
          <button key={key} type="button" className={tab === key ? "active" : ""} onClick={() => setTab(key)}>{label}</button>
        ))}
      </nav>
      {tab === "summary" && <div className="detailSection"><h3>Utilisation au laboratoire</h3><p>{asset.notes || "Aucune note d'utilisation."}</p><p><strong>État actuel :</strong> {serviceStateLabel(asset.service_state)} · {availabilityLabel(asset.availability_state)}</p></div>}
      {tab === "identification" && <AssetIdentificationEditor key={`${asset.asset_id}-${asset.revision}`} asset={asset} onUpdate={props.onUpdate} />}
      {tab === "location" && <AssetOperationalEditor key={`${asset.asset_id}-${asset.revision}`} asset={asset} locations={props.locations} locationsError={props.locationsError} onUpdate={props.onUpdate} onTransitionService={props.onTransitionService} onTransitionAvailability={props.onTransitionAvailability} />}
      {tab === "metrology" && <div className="detailSection"><h3>Métrologie de cet exemplaire</h3><p>{metrologyLabel(asset)}</p><button type="button" onClick={() => props.onOpenMetrology(asset.asset_id)}><Wrench size={16} /> Ouvrir la métrologie</button></div>}
      {tab === "history" && <div className="detailSection"><p>Créé le {formatDate(asset.created_at)} · mis à jour le {formatDate(asset.updated_at)}.</p></div>}
      {tab === "technical" && <details open><summary>Identifiants et preuve de version</summary><dl><dt>Identifiant interne</dt><dd>{asset.asset_id}</dd><dt>Révision</dt><dd>{asset.revision}</dd><dt>Version du modèle</dt><dd>{asset.equipment_model_revision_id || "Lien à rapprocher"}</dd><dt>Empreinte du modèle</dt><dd className="technicalValue">{asset.equipment_model_checksum || "Indisponible"}</dd></dl></details>}
      {asset.equipment_model_id && asset.equipment_model_revision_id && <button className="secondary" type="button" onClick={() => props.onOpenPinnedModel(asset.equipment_model_id!, asset.equipment_model_revision_id!)}>Ouvrir le modèle constructeur</button>}
      {(!asset.equipment_model_id || !asset.equipment_model_revision_id) && <p className="actionExplanation">Le lien exact vers le modèle doit être rapproché par un administrateur du référentiel.</p>}
    </article>
  );
}

function AssetIdentificationEditor(props: {
  asset: PhysicalAsset;
  onUpdate: (asset: PhysicalAsset, input: Parameters<typeof fleetApi.updateAsset>[1]) => Promise<void>;
}) {
  const [inventoryCode, setInventoryCode] = useState(props.asset.inventory_code);
  const [serialNumber, setSerialNumber] = useState(props.asset.serial_number ?? "");
  const [partNumber, setPartNumber] = useState(props.asset.part_number ?? "");
  const [ownership, setOwnership] = useState<OwnershipSource>(props.asset.ownership_source);
  const [notes, setNotes] = useState(props.asset.notes);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function save() {
    if (!inventoryCode.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await props.onUpdate(props.asset, {
        inventory_code: inventoryCode.trim(),
        serial_number: serialNumber.trim() || undefined,
        part_number: partNumber.trim() || undefined,
        laboratory_location_id: props.asset.laboratory_location_id ?? undefined,
        ownership_source: ownership,
        notes: notes.trim()
      });
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="detailSection assetEditSection">
      <div className="sectionTitleRow"><div><h3>Identification de l'exemplaire</h3><p>Le code inventaire est l'identité lisible dans les montages et les essais.</p></div></div>
      <div className="formGrid">
        <label>Code inventaire <Required /><input value={inventoryCode} onChange={(event) => setInventoryCode(event.target.value)} /></label>
        <label>Numéro de série <span className="fieldHint">Facultatif</span><input value={serialNumber} onChange={(event) => setSerialNumber(event.target.value)} /></label>
        <label>Part number <span className="fieldHint">Facultatif</span><input value={partNumber} onChange={(event) => setPartNumber(event.target.value)} /></label>
        <label>Propriété / source <Required /><select value={ownership} onChange={(event) => setOwnership(event.target.value as OwnershipSource)}>{ownershipChoices.map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select></label>
        <label className="wideField">Notes<textarea value={notes} onChange={(event) => setNotes(event.target.value)} /></label>
      </div>
      {!inventoryCode.trim() && <p className="actionExplanation">Renseignez le code inventaire pour enregistrer l'identification.</p>}
      {error && <TargetedError title="Modification refusée" detail={`${error} Actualisez la fiche puis vérifiez les informations saisies.`} />}
      <button type="button" disabled={busy || !inventoryCode.trim()} onClick={() => void save()}><RefreshCw size={16} /> {busy ? "Enregistrement..." : "Enregistrer l'identification"}</button>
    </section>
  );
}

function AssetOperationalEditor(props: {
  asset: PhysicalAsset;
  locations: LaboratoryLocation[];
  locationsError: string | null;
  onUpdate: (asset: PhysicalAsset, input: Parameters<typeof fleetApi.updateAsset>[1]) => Promise<void>;
  onTransitionService: (asset: PhysicalAsset, state: ServiceState, reason: string) => Promise<void>;
  onTransitionAvailability: (asset: PhysicalAsset, state: AvailabilityState) => Promise<void>;
}) {
  const [locationId, setLocationId] = useState(props.asset.laboratory_location_id ?? "");
  const [serviceState, setServiceState] = useState<ServiceState>(props.asset.service_state);
  const [serviceReason, setServiceReason] = useState(props.asset.service_state_reason);
  const [availability, setAvailability] = useState<AvailabilityState>(props.asset.availability_state);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeLocations = props.locations.filter((location) => location.status === "active");
  const serviceChanged = serviceState !== props.asset.service_state;
  const availabilityChanged = availability !== props.asset.availability_state;
  const locationChanged = locationId !== (props.asset.laboratory_location_id ?? "");

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

  return (
    <section className="detailSection assetOperationalSection">
      <div className="operationalEditBlock">
        <div><h3>Emplacement</h3><p>Le lieu stable utilisé par le parc, les montages et le planning.</p></div>
        <label>Lieu du laboratoire <Required /><select value={locationId} disabled={Boolean(props.locationsError)} onChange={(event) => setLocationId(event.target.value)}><option value="">Non défini</option>{activeLocations.map((location) => <option key={location.location_id} value={location.location_id}>{location.label}</option>)}</select></label>
        {props.locationsError && <TargetedError title="Registre des lieux indisponible" detail="L'identité de l'exemplaire reste consultable. Le déplacement est suspendu jusqu'au retour du registre." />}
        {!locationChanged && <p className="actionExplanation">Sélectionnez un autre emplacement pour déplacer cet exemplaire.</p>}
        <button type="button" disabled={busy || !locationChanged || Boolean(props.locationsError)} onClick={() => void run(() => props.onUpdate(props.asset, { inventory_code: props.asset.inventory_code, serial_number: props.asset.serial_number ?? undefined, part_number: props.asset.part_number ?? undefined, laboratory_location_id: locationId || undefined, ownership_source: props.asset.ownership_source, notes: props.asset.notes }))}>Enregistrer l'emplacement</button>
      </div>
      <div className="operationalEditBlock">
        <div><h3>État de service</h3><p>Indique si l'exemplaire peut techniquement être utilisé.</p></div>
        <label>État <Required /><select value={serviceState} onChange={(event) => setServiceState(event.target.value as ServiceState)}>{serviceStateChoices.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
        <label>Motif du changement <Required /><input value={serviceReason} onChange={(event) => setServiceReason(event.target.value)} placeholder="Contrôle, maintenance ou restriction" /></label>
        {!serviceChanged && <p className="actionExplanation">Choisissez un nouvel état de service pour appliquer une transition.</p>}
        {serviceChanged && !serviceReason.trim() && <p className="actionExplanation">Renseignez le motif du changement d'état.</p>}
        <button type="button" disabled={busy || !serviceChanged || !serviceReason.trim()} onClick={() => void run(() => props.onTransitionService(props.asset, serviceState, serviceReason.trim()))}>Appliquer l'état de service</button>
      </div>
      <div className="operationalEditBlock">
        <div><h3>Disponibilité</h3><p>Indique si l'exemplaire peut être réservé ou affecté maintenant.</p></div>
        <label>Disponibilité <Required /><select value={availability} onChange={(event) => setAvailability(event.target.value as AvailabilityState)}>{availabilityChoices.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
        {!availabilityChanged && <p className="actionExplanation">Choisissez une nouvelle disponibilité pour appliquer une transition.</p>}
        <button type="button" disabled={busy || !availabilityChanged} onClick={() => void run(() => props.onTransitionAvailability(props.asset, availability))}>Appliquer la disponibilité</button>
      </div>
      {error && <TargetedError title="Transition refusée" detail={`${error} Vérifiez l'état actuel et choisissez une transition autorisée.`} />}
    </section>
  );
}

function CreateAssetDialog(props: {
  models: EquipmentModelAggregate[];
  locations: LaboratoryLocation[];
  locationsError: string | null;
  initialModelId?: string | null;
  onCancel: () => void;
  onCreate: (input: CreatePhysicalAssetInput) => Promise<void>;
}) {
  const [modelId, setModelId] = useState(props.initialModelId ?? props.models[0]?.identity.equipment_model_id ?? "");
  const [inventoryCode, setInventoryCode] = useState("");
  const [serialNumber, setSerialNumber] = useState("");
  const [partNumber, setPartNumber] = useState("");
  const [locationId, setLocationId] = useState("");
  const [ownership, setOwnership] = useState<OwnershipSource>("laboratory_owned");
  const [serviceState, setServiceState] = useState<ServiceState>("usable");
  const [availability, setAvailability] = useState<AvailabilityState>("available");
  const [calibrationRequirement, setCalibrationRequirement] = useState("required");
  const [periodMonths, setPeriodMonths] = useState("12");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const selectedModel = props.models.find((model) => model.identity.equipment_model_id === modelId);
  const missing = [!modelId && "le modèle constructeur", !inventoryCode.trim() && "le code inventaire", !locationId && "l'emplacement"].filter(Boolean) as string[];

  async function submit() {
    if (missing.length > 0) {
      document.querySelector<HTMLInputElement | HTMLSelectElement>(!modelId ? "#fleet-model" : !inventoryCode.trim() ? "#fleet-inventory" : "#fleet-location")?.focus();
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      await props.onCreate({
        inventory_code: inventoryCode.trim(),
        serial_number: serialNumber.trim() || undefined,
        part_number: partNumber.trim() || undefined,
        equipment_model_id: modelId,
        laboratory_location_id: locationId,
        ownership_source: ownership,
        service_state: serviceState,
        availability_state: availability,
        notes: notes.trim(),
        calibration_requirement: calibrationRequirement,
        calibration_period_months: calibrationRequirement === "not_required" ? undefined : Number(periodMonths),
        calibration_due_warning_days: 30
      });
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <section className="modalCard assetCreationDialog" role="dialog" aria-modal="true" aria-labelledby="create-asset-title">
      <header><div><p className="eyebrow">Parc matériel</p><h2 id="create-asset-title">Ajouter un exemplaire</h2></div><button className="iconButton secondary" type="button" onClick={props.onCancel} aria-label="Fermer"><X size={17} /></button></header>
      <p className="contextBanner">Vous ajoutez un exemplaire réellement utilisé par le laboratoire.</p>
      <div className="formGrid">
        <label>Modèle constructeur <Required /><select id="fleet-model" value={modelId} onChange={(event) => setModelId(event.target.value)}><option value="">Sélectionner...</option>{props.models.map((model) => <option key={model.identity.equipment_model_id} value={model.identity.equipment_model_id}>{model.identity.manufacturer} {model.identity.model_name}{model.identity.variant ? ` ${model.identity.variant}` : ""}</option>)}</select></label>
        <label>Code inventaire <Required /><input id="fleet-inventory" value={inventoryCode} onChange={(event) => setInventoryCode(event.target.value)} placeholder="INV-0042" /></label>
        <label>Numéro de série <span className="fieldHint">Facultatif</span><input value={serialNumber} onChange={(event) => setSerialNumber(event.target.value)} /></label>
        <label>Part number <span className="fieldHint">Facultatif</span><input value={partNumber} onChange={(event) => setPartNumber(event.target.value)} /></label>
        <label>Propriété / source <Required /><select value={ownership} onChange={(event) => setOwnership(event.target.value as OwnershipSource)}>{ownershipChoices.map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select></label>
        <label>Emplacement <Required /><select id="fleet-location" value={locationId} onChange={(event) => setLocationId(event.target.value)} disabled={Boolean(props.locationsError)}><option value="">Sélectionner...</option>{props.locations.map((location) => <option value={location.location_id} key={location.location_id}>{location.label}</option>)}</select></label>
        <label>État de service <Required /><select value={serviceState} onChange={(event) => setServiceState(event.target.value as ServiceState)}>{serviceStateChoices.map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select></label>
        <label>Disponibilité <Required /><select value={availability} onChange={(event) => setAvailability(event.target.value as AvailabilityState)}>{availabilityChoices.map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select></label>
        <label>Exigence métrologique <Required /><select value={calibrationRequirement} onChange={(event) => setCalibrationRequirement(event.target.value)}><option value="required">Étalonnage requis</option><option value="conditional">Selon l'utilisation</option><option value="not_required">Non requis</option></select></label>
        {calibrationRequirement !== "not_required" && <label>Périodicité (mois) <Required /><input type="number" min="1" value={periodMonths} onChange={(event) => setPeriodMonths(event.target.value)} /></label>}
        <label className="wideField">Notes<textarea value={notes} onChange={(event) => setNotes(event.target.value)} /></label>
      </div>
      {selectedModel && <p className="selectedModelSummary">Modèle sélectionné : <strong>{selectedModel.identity.manufacturer} {selectedModel.identity.model_name}</strong>. La version approuvée exacte sera figée par l'agent local.</p>}
      {props.locationsError && <TargetedError title="Choix de l'emplacement indisponible" detail="La création est suspendue jusqu'au retour du registre des lieux." />}
      {error && <TargetedError title="Enregistrement refusé" detail={error} />}
      {missing.length > 0 && <p className="actionExplanation">Pour enregistrer : renseignez {formatList(missing)}.</p>}
      <footer><button className="secondary" type="button" onClick={props.onCancel}>Annuler</button><button type="button" onClick={() => void submit()} disabled={submitting || missing.length > 0 || Boolean(props.locationsError)}><PackagePlus size={16} /> {submitting ? "Enregistrement..." : "Enregistrer l'exemplaire"}</button></footer>
    </section>
  );
}

function buildGroups(assets: PhysicalAsset[], grouping: GroupingMode) {
  const grouped = groupBy(assets, (asset) => {
    if (grouping === "location") return asset.laboratory_location_label || "Emplacement non défini";
    if (grouping === "service") return serviceStateLabel(asset.service_state);
    if (grouping === "metrology") return metrologyGroup(asset);
    return categoryPath(asset);
  });
  return Array.from(grouped.entries()).sort(([left], [right]) => left.localeCompare(right, "fr")).map(([label, rows]) => ({ key: `${grouping}:${label}`, label, assets: rows }));
}

function groupBy<T>(items: T[], keyFor: (item: T) => string) {
  const result = new Map<string, T[]>();
  for (const item of items) result.set(keyFor(item), [...(result.get(keyFor(item)) ?? []), item]);
  return result;
}

function IdentityFact(props: { label: string; value: string }) { return <div><span>{props.label}</span><strong>{props.value}</strong></div>; }
function Required() { return <span className="requiredBadge">Obligatoire</span>; }
function TargetedError(props: { title: string; detail: string }) { return <div className="targetedError"><AlertTriangle size={17} /><div><strong>{props.title}</strong><p>{props.detail}</p></div></div>; }
function categoryPath(asset: PhysicalAsset) { return asset.category_path.length > 0 ? asset.category_path.join(" > ") : asset.category_code; }
function searchText(asset: PhysicalAsset) { return [categoryPath(asset), asset.manufacturer, asset.model_name, asset.variant, asset.inventory_code, asset.serial_number, asset.laboratory_location_label].filter(Boolean).join(" ").toLocaleLowerCase("fr"); }
function formatDate(value: string) { return new Intl.DateTimeFormat("fr-FR", { dateStyle: "medium" }).format(new Date(value)); }
function formatList(items: string[]) { return new Intl.ListFormat("fr-FR", { style: "long", type: "conjunction" }).format(items); }
function metrologyGroup(asset: PhysicalAsset) { if (!asset.metrology || asset.metrology.calibration_requirement === "not_required") return "Étalonnage non requis"; if (!asset.metrology.latest_due_at) return "Étalonnage à planifier"; return new Date(asset.metrology.latest_due_at) < new Date() ? "Échéance dépassée" : "Étalonnage valide"; }
function metrologyLabel(asset: PhysicalAsset) { if (!asset.metrology) return "Dossier métrologique indisponible"; if (asset.metrology.calibration_requirement === "not_required") return "Étalonnage non requis"; if (!asset.metrology.latest_due_at) return "Étalonnage à planifier"; return `Valide jusqu'au ${formatDate(asset.metrology.latest_due_at)}`; }
function serviceStateLabel(value: ServiceState) { return serviceStateChoices.find(([key]) => key === value)?.[1] ?? value; }
function availabilityLabel(value: AvailabilityState) { return availabilityChoices.find(([key]) => key === value)?.[1] ?? value; }
function errorMessage(error: unknown) { return error instanceof Error ? error.message : "Erreur inattendue."; }
function modelLabel(models: EquipmentModelAggregate[], modelId: string) { const model = models.find((candidate) => candidate.identity.equipment_model_id === modelId); return model ? `${model.identity.manufacturer} ${model.identity.model_name}${model.identity.variant ? ` ${model.identity.variant}` : ""}` : "sélectionné"; }

const serviceStateChoices: Array<[ServiceState, string]> = [["usable", "Utilisable"], ["restricted", "Utilisation restreinte"], ["in_maintenance", "En maintenance"], ["out_of_service", "Hors service"], ["retired", "Retiré du parc"]];
const availabilityChoices: Array<[AvailabilityState, string]> = [["available", "Disponible"], ["reserved", "Réservé"], ["assigned_to_setup", "Affecté à un montage"], ["in_test", "Utilisé en essai"], ["unavailable", "Indisponible"]];
const ownershipChoices: Array<[OwnershipSource, string]> = [["laboratory_owned", "Propriété du laboratoire"], ["customer_supplied", "Fourni par le client"], ["rented", "Loué"], ["borrowed", "Emprunté"], ["external", "Externe"], ["software_license", "Licence logicielle"], ["installed_facility", "Installation fixe"]];
