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
import { metrologyStatusGroup, metrologyStatusLabel } from "../../metrologyStatus";
import type { EquipmentCategory, EquipmentModelAggregate } from "../../models/equipment";
import type {
  AdministrativeAvailability,
  CreatePhysicalAssetInput,
  LaboratoryLocation,
  ModelReconciliationCandidate,
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

  async function updateAssetIdentification(asset: PhysicalAsset, input: Parameters<typeof fleetApi.updateAssetIdentification>[1]) {
    const response = await fleetApi.updateAssetIdentification(asset, input, context);
    replaceAsset(response.asset);
  }

  async function moveAsset(asset: PhysicalAsset, destinationLocationId: string | undefined, reason: string) {
    const response = await fleetApi.moveAsset(asset, destinationLocationId, { ...context, reason });
    replaceAsset(response.asset);
  }

  async function transitionServiceState(asset: PhysicalAsset, state: ServiceState, reason: string) {
    const response = await fleetApi.transitionServiceState(asset, state, reason, context);
    replaceAsset(response.asset);
  }

  async function transitionAdministrativeAvailability(
    asset: PhysicalAsset,
    state: AdministrativeAvailability,
    reason: string
  ) {
    const response = await fleetApi.transitionAdministrativeAvailability(
      asset,
      state,
      reason,
      context
    );
    replaceAsset(response.asset);
  }

  async function reconcileModel(
    asset: PhysicalAsset,
    candidate: ModelReconciliationCandidate
  ) {
    const response = await fleetApi.reconcileModel(asset, candidate, context);
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
          onUpdateIdentification={updateAssetIdentification}
          onMove={moveAsset}
          onTransitionService={transitionServiceState}
          onTransitionAdministrativeAvailability={transitionAdministrativeAvailability}
          onReconcileModel={reconcileModel}
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
          <small>{locationDisplay(asset)}</small>
          <span className="assetRowStates"><span className={`status ${asset.service_state}`}>{serviceStateLabel(asset.service_state)}</span><span>{operationalUsageLabel(asset.availability_state)}</span><span>{metrologyLabel(asset)}</span></span>
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
  onUpdateIdentification: (asset: PhysicalAsset, input: Parameters<typeof fleetApi.updateAssetIdentification>[1]) => Promise<void>;
  onMove: (asset: PhysicalAsset, destinationLocationId: string | undefined, reason: string) => Promise<void>;
  onTransitionService: (asset: PhysicalAsset, state: ServiceState, reason: string) => Promise<void>;
  onTransitionAdministrativeAvailability: (
    asset: PhysicalAsset,
    state: AdministrativeAvailability,
    reason: string
  ) => Promise<void>;
  onReconcileModel: (
    asset: PhysicalAsset,
    candidate: ModelReconciliationCandidate
  ) => Promise<void>;
  onOpenPinnedModel: (modelId: string, revisionId: string) => void;
  onOpenMetrology: (assetId: string) => void;
}) {
  const [tab, setTab] = useState("summary");
  const [reconciliationOpen, setReconciliationOpen] = useState(false);
  if (!props.asset) return <div className="assetDetail empty"><strong>Aucun exemplaire ouvert</strong><p>Sélectionnez un exemplaire dans la hiérarchie du parc.</p></div>;
  const asset = props.asset;
  return (
    <article className="assetDetail">
      <header className="assetIdentityHeader">
        <div><p className="eyebrow">Exemplaire du parc</p><h2>{asset.inventory_code}</h2><p>{asset.manufacturer} {asset.model_name}{asset.variant ? ` ${asset.variant}` : ""}</p></div>
        <div className="identityFacts">
          <IdentityFact label="Numéro de série" value={asset.serial_number || "Sans numéro de série"} />
          <IdentityFact label="Catégorie" value={categoryPath(asset)} />
          <IdentityFact label="Emplacement" value={locationDisplay(asset)} />
          <IdentityFact label="État de service" value={serviceStateLabel(asset.service_state)} />
          <IdentityFact label="Disponibilité administrative" value={administrativeAvailabilityLabel(asset.administrative_availability)} />
          <IdentityFact label="Utilisation calculée" value={operationalUsageLabel(asset.availability_state)} />
          <IdentityFact label="Métrologie" value={metrologyLabel(asset)} />
        </div>
      </header>
      <nav className="detailTabs" aria-label="Détail de l'exemplaire">
        {[["summary", "Résumé"], ["identification", "Identification"], ["location", "Emplacement et disponibilité"], ["metrology", "Métrologie"], ["history", "Historique"], ["technical", "Détails techniques"]].map(([key, label]) => (
          <button key={key} type="button" className={tab === key ? "active" : ""} onClick={() => setTab(key)}>{label}</button>
        ))}
      </nav>
      {tab === "summary" && <div className="detailSection"><h3>Utilisation au laboratoire</h3><p>{asset.notes || "Aucune note d'utilisation."}</p><p><strong>État technique :</strong> {serviceStateLabel(asset.service_state)}</p><p><strong>Disponibilité administrative :</strong> {administrativeAvailabilityLabel(asset.administrative_availability)}</p><p><strong>Utilisation calculée :</strong> {operationalUsageLabel(asset.operational_usage.state)}</p>{asset.operational_usage.evidence.length > 0 && <ul className="usageEvidenceList">{asset.operational_usage.evidence.map((item) => <li key={`${item.source_kind}-${item.source_identifier}-${item.reason}`}><strong>{item.source_label}</strong><span>{item.reason}</span>{item.relevant_start_at && <small>{formatUsageInterval(item.relevant_start_at, item.relevant_end_at)}</small>}</li>)}</ul>}</div>}
      {tab === "identification" && <AssetIdentificationEditor key={`${asset.asset_id}-${asset.revision}`} asset={asset} onUpdate={props.onUpdateIdentification} />}
      {tab === "location" && <AssetOperationalEditor key={`${asset.asset_id}-${asset.revision}`} asset={asset} locations={props.locations} locationsError={props.locationsError} onMove={props.onMove} onTransitionService={props.onTransitionService} onTransitionAdministrativeAvailability={props.onTransitionAdministrativeAvailability} />}
      {tab === "metrology" && <div className="detailSection"><h3>Métrologie de cet exemplaire</h3><p>{metrologyLabel(asset)}</p><button type="button" onClick={() => props.onOpenMetrology(asset.asset_id)}><Wrench size={16} /> Ouvrir la métrologie</button></div>}
      {tab === "history" && <div className="detailSection"><p>Créé le {formatDate(asset.created_at)} · mis à jour le {formatDate(asset.updated_at)}.</p></div>}
      {tab === "technical" && <details open><summary>Identifiants et preuve de version</summary><dl><dt>Identifiant interne</dt><dd>{asset.asset_id}</dd><dt>Révision</dt><dd>{asset.revision}</dd><dt>Version du modèle</dt><dd>{asset.equipment_model_revision_id || "Lien à rapprocher"}</dd><dt>Empreinte du modèle</dt><dd className="technicalValue">{asset.equipment_model_checksum || "Indisponible"}</dd></dl></details>}
      {asset.equipment_model_id && asset.equipment_model_revision_id && <button className="secondary" type="button" onClick={() => props.onOpenPinnedModel(asset.equipment_model_id!, asset.equipment_model_revision_id!)}>Ouvrir le modèle constructeur</button>}
      {asset.model_link_state === "migration_review_required" && (
        <section className="modelReconciliationCallout">
          <div>
            <h3>Modèle constructeur à rapprocher</h3>
            <p>Cet exemplaire provient de l’ancien registre métrologique. Sélectionnez la version exacte de son modèle avant de l’utiliser dans un montage ou un essai.</p>
          </div>
          <button type="button" onClick={() => setReconciliationOpen(true)}>
            Rapprocher avec un modèle constructeur
          </button>
        </section>
      )}
      {reconciliationOpen && (
        <div className="modalBackdrop">
          <ModelReconciliationDialog
            asset={asset}
            onCancel={() => setReconciliationOpen(false)}
            onReconcile={async (candidate) => {
              await props.onReconcileModel(asset, candidate);
              setReconciliationOpen(false);
            }}
          />
        </div>
      )}
    </article>
  );
}

function ModelReconciliationDialog(props: {
  asset: PhysicalAsset;
  onCancel: () => void;
  onReconcile: (candidate: ModelReconciliationCandidate) => Promise<void>;
}) {
  const [candidates, setCandidates] = useState<ModelReconciliationCandidate[]>([]);
  const [selectedRevisionId, setSelectedRevisionId] = useState("");
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void fleetApi.listModelReconciliationCandidates()
      .then((response) => {
        if (!active) return;
        setCandidates(response.candidates);
        setSelectedRevisionId(response.candidates[0]?.equipment_model_revision_id ?? "");
        setError(null);
      })
      .catch((reason: unknown) => {
        if (active) setError(errorMessage(reason));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);

  const selected = candidates.find(
    (candidate) => candidate.equipment_model_revision_id === selectedRevisionId
  );

  async function reconcile() {
    if (!selected) return;
    setSubmitting(true);
    setError(null);
    try {
      await props.onReconcile(selected);
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <section className="modalCard reconciliationDialog" role="dialog" aria-modal="true" aria-labelledby="reconciliation-title">
      <header>
        <div>
          <p className="eyebrow">Exemplaire migré · {props.asset.inventory_code}</p>
          <h2 id="reconciliation-title">Rapprocher avec un modèle constructeur</h2>
        </div>
        <button className="iconButton secondary" type="button" onClick={props.onCancel} aria-label="Fermer"><X size={17} /></button>
      </header>
      <p>Sélectionnez la version exacte correspondant à cet exemplaire. Ce choix est définitif pour préserver la traçabilité.</p>
      {loading ? (
        <p className="muted">Lecture des versions contrôlées...</p>
      ) : candidates.length === 0 ? (
        <p className="actionExplanation">Aucune version approuvée ou remplacée n’est disponible. Faites approuver le modèle constructeur attendu avant de poursuivre.</p>
      ) : (
        <label>Modèle et version <Required />
          <select value={selectedRevisionId} onChange={(event) => setSelectedRevisionId(event.target.value)}>
            {candidates.map((candidate) => (
              <option key={candidate.equipment_model_revision_id} value={candidate.equipment_model_revision_id}>
                {reconciliationCandidateLabel(candidate)}
              </option>
            ))}
          </select>
        </label>
      )}
      {selected && (
        <div className="selectedModelSummary">
          <strong>{selected.manufacturer} {selected.model_name}{selected.variant ? ` ${selected.variant}` : ""}</strong>
          <span>{selected.category_path.join(" > ")} · Version {selected.revision_number} · {revisionStatusLabel(selected.lifecycle_status)}{selected.approved_at ? ` le ${formatDate(selected.approved_at)}` : ""}</span>
        </div>
      )}
      {error && <TargetedError title="Rapprochement indisponible" detail={error} />}
      <footer>
        <button className="secondary" type="button" onClick={props.onCancel}>Annuler</button>
        <button type="button" disabled={submitting || !selected} onClick={() => void reconcile()}>
          {submitting ? "Rapprochement..." : "Rapprocher cette version"}
        </button>
      </footer>
    </section>
  );
}

function AssetIdentificationEditor(props: {
  asset: PhysicalAsset;
  onUpdate: (asset: PhysicalAsset, input: Parameters<typeof fleetApi.updateAssetIdentification>[1]) => Promise<void>;
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
        <label>Référence fabricant <span className="fieldHint">Facultatif</span><input value={partNumber} onChange={(event) => setPartNumber(event.target.value)} /></label>
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
  onMove: (asset: PhysicalAsset, destinationLocationId: string | undefined, reason: string) => Promise<void>;
  onTransitionService: (asset: PhysicalAsset, state: ServiceState, reason: string) => Promise<void>;
  onTransitionAdministrativeAvailability: (
    asset: PhysicalAsset,
    state: AdministrativeAvailability,
    reason: string
  ) => Promise<void>;
}) {
  const [locationId, setLocationId] = useState(props.asset.laboratory_location_id ?? "");
  const [movementReason, setMovementReason] = useState("");
  const [serviceState, setServiceState] = useState<ServiceState>(props.asset.service_state);
  const [serviceReason, setServiceReason] = useState(props.asset.service_state_reason);
  const [administrativeAvailability, setAdministrativeAvailability] = useState<AdministrativeAvailability>(props.asset.administrative_availability);
  const [administrativeReason, setAdministrativeReason] = useState(props.asset.administrative_unavailability_reason);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeLocations = props.locations.filter((location) => location.status === "active");
  const serviceChanged = serviceState !== props.asset.service_state;
  const availabilityChanged = administrativeAvailability !== props.asset.administrative_availability
    || administrativeReason.trim() !== props.asset.administrative_unavailability_reason;
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
        <div><h3>Déplacer l'exemplaire</h3><p>Le lieu courant reste autorisé s'il a été archivé. Toute nouvelle destination doit être active.</p></div>
        {props.asset.laboratory_location_status === "archived" && <p className="actionExplanation"><strong>Emplacement actuel archivé.</strong> L'identification reste modifiable. Déplacez l'exemplaire vers un lieu actif ou retirez son affectation.</p>}
        <label>Nouvel emplacement <select value={locationId} disabled={Boolean(props.locationsError)} onChange={(event) => setLocationId(event.target.value)}><option value="">Emplacement non défini</option>{props.asset.laboratory_location_status === "archived" && props.asset.laboratory_location_id && <option value={props.asset.laboratory_location_id} disabled>{props.asset.laboratory_location_label} (emplacement actuel archivé)</option>}{activeLocations.map((location) => <option key={location.location_id} value={location.location_id}>{location.label}</option>)}</select></label>
        <label>Motif du déplacement <Required /><input value={movementReason} onChange={(event) => setMovementReason(event.target.value)} placeholder="Réaffectation, retour de prêt ou mise en attente" /></label>
        {props.locationsError && <TargetedError title="Registre des lieux indisponible" detail="L'identité de l'exemplaire reste consultable. Le déplacement est suspendu jusqu'au retour du registre." />}
        {!locationChanged && <p className="actionExplanation">Sélectionnez un autre emplacement pour déplacer cet exemplaire.</p>}
        {locationChanged && !movementReason.trim() && <p className="actionExplanation">Renseignez le motif du déplacement.</p>}
        <button type="button" disabled={busy || !locationChanged || !movementReason.trim() || Boolean(props.locationsError)} onClick={() => void run(() => props.onMove(props.asset, locationId || undefined, movementReason.trim()))}>Enregistrer le déplacement</button>
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
        <div><h3>Disponibilité administrative</h3><p>Décision manuelle du parc. Les réservations, montages et essais en cours sont calculés depuis leurs workflows.</p></div>
        <label>Disponibilité administrative <Required /><select value={administrativeAvailability} onChange={(event) => { const value = event.target.value as AdministrativeAvailability; setAdministrativeAvailability(value); if (value === "available") setAdministrativeReason(""); }}>{administrativeAvailabilityChoices.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
        {administrativeAvailability === "unavailable" && <label>Motif d'indisponibilité <Required /><input value={administrativeReason} onChange={(event) => setAdministrativeReason(event.target.value)} placeholder="Prêt externe, quarantaine ou décision administrative" /></label>}
        {!availabilityChanged && <p className="actionExplanation">Modifiez la disponibilité administrative ou son motif pour enregistrer une décision.</p>}
        {administrativeAvailability === "unavailable" && !administrativeReason.trim() && <p className="actionExplanation">Renseignez le motif de l'indisponibilité administrative.</p>}
        <button type="button" disabled={busy || !availabilityChanged || (administrativeAvailability === "unavailable" && !administrativeReason.trim())} onClick={() => void run(() => props.onTransitionAdministrativeAvailability(props.asset, administrativeAvailability, administrativeReason.trim()))}>Enregistrer la disponibilité administrative</button>
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
  const [serviceStateReason, setServiceStateReason] = useState("");
  const [administrativeAvailability, setAdministrativeAvailability] = useState<AdministrativeAvailability>("available");
  const [administrativeReason, setAdministrativeReason] = useState("");
  const [calibrationRequirement, setCalibrationRequirement] = useState("required");
  const [periodMonths, setPeriodMonths] = useState("12");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const selectedModel = props.models.find((model) => model.identity.equipment_model_id === modelId);
  const serviceReasonRequired = serviceState !== "usable";
  const availabilityForced = serviceStateForcesUnavailability(serviceState);
  const missing = [
    !modelId && "le modèle constructeur",
    !inventoryCode.trim() && "le code inventaire",
    serviceReasonRequired && !serviceStateReason.trim() && "le motif de l'état de service",
    administrativeAvailability === "unavailable" && !availabilityForced && !administrativeReason.trim() && "le motif de l'indisponibilité administrative",
    calibrationRequirement !== "not_required" && (!Number.isInteger(Number(periodMonths)) || Number(periodMonths) < 1) && "une périodicité d'étalonnage valide"
  ].filter(Boolean) as string[];

  async function submit() {
    if (missing.length > 0) {
      const target = !modelId
        ? "#fleet-model"
        : !inventoryCode.trim()
          ? "#fleet-inventory"
          : serviceReasonRequired && !serviceStateReason.trim()
            ? "#fleet-service-reason"
            : administrativeAvailability === "unavailable" && !availabilityForced && !administrativeReason.trim()
              ? "#fleet-administrative-reason"
              : "#fleet-calibration-period";
      document.querySelector<HTMLInputElement | HTMLSelectElement>(target)?.focus();
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
        laboratory_location_id: locationId || undefined,
        ownership_source: ownership,
        service_state: serviceState,
        service_state_reason: serviceStateReason.trim() || undefined,
        administrative_availability: availabilityForced ? "unavailable" : administrativeAvailability,
        administrative_unavailability_reason: (availabilityForced ? serviceStateReason : administrativeReason).trim() || undefined,
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
        <label>Référence fabricant <span className="fieldHint">Facultatif</span><input value={partNumber} onChange={(event) => setPartNumber(event.target.value)} /></label>
        <label>Propriété / source <Required /><select value={ownership} onChange={(event) => setOwnership(event.target.value as OwnershipSource)}>{ownershipChoices.map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select></label>
        <label>Emplacement <span className="fieldHint">Facultatif</span><select id="fleet-location" value={locationId} onChange={(event) => setLocationId(event.target.value)} disabled={Boolean(props.locationsError)}><option value="">Emplacement non défini</option>{props.locations.map((location) => <option value={location.location_id} key={location.location_id}>{location.label}</option>)}</select></label>
        <label>État de service <Required /><select value={serviceState} onChange={(event) => { const value = event.target.value as ServiceState; if (serviceStateForcesUnavailability(value)) setAdministrativeAvailability("unavailable"); else if (serviceStateForcesUnavailability(serviceState)) setAdministrativeAvailability("available"); setServiceState(value); }}>{serviceStateChoices.map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select></label>
        {serviceReasonRequired && <label>Motif de l'état de service <Required /><input id="fleet-service-reason" value={serviceStateReason} onChange={(event) => setServiceStateReason(event.target.value)} placeholder="Restriction, maintenance ou décision de retrait" /><small>Ce motif explique pourquoi l'exemplaire n'est pas déclaré pleinement utilisable.</small></label>}
        <label>Disponibilité administrative <Required /><select value={availabilityForced ? "unavailable" : administrativeAvailability} disabled={availabilityForced} onChange={(event) => { const value = event.target.value as AdministrativeAvailability; setAdministrativeAvailability(value); if (value === "available") setAdministrativeReason(""); }}>{administrativeAvailabilityChoices.map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select></label>
        {availabilityForced && <p className="actionExplanation formGridExplanation">Indisponible automatiquement car cet état technique interdit l'utilisation.</p>}
        {administrativeAvailability === "unavailable" && !availabilityForced && <label>Motif d'indisponibilité <Required /><input id="fleet-administrative-reason" value={administrativeReason} onChange={(event) => setAdministrativeReason(event.target.value)} /></label>}
        <label>Exigence métrologique <Required /><select value={calibrationRequirement} onChange={(event) => setCalibrationRequirement(event.target.value)}><option value="required">Étalonnage requis</option><option value="conditional">Selon l'utilisation</option><option value="not_required">Non requis</option></select></label>
        {calibrationRequirement !== "not_required" && <label>Périodicité (mois) <Required /><input id="fleet-calibration-period" type="number" min="1" value={periodMonths} onChange={(event) => setPeriodMonths(event.target.value)} /></label>}
        <label className="wideField">Notes<textarea value={notes} onChange={(event) => setNotes(event.target.value)} /></label>
      </div>
      {selectedModel && <p className="selectedModelSummary">Modèle sélectionné : <strong>{selectedModel.identity.manufacturer} {selectedModel.identity.model_name}</strong>. La version approuvée exacte sera figée par l'agent local.</p>}
      {props.locationsError && <TargetedError title="Registre des lieux indisponible" detail="Vous pouvez enregistrer l'exemplaire sans emplacement et l'affecter plus tard." />}
      {error && <TargetedError title="Enregistrement refusé" detail={error} />}
      {missing.length > 0 && <p className="actionExplanation">Pour enregistrer : renseignez {formatList(missing)}.</p>}
      <footer><button className="secondary" type="button" onClick={props.onCancel}>Annuler</button><button type="button" onClick={() => void submit()} disabled={submitting} aria-disabled={missing.length > 0}><PackagePlus size={16} /> {submitting ? "Enregistrement..." : "Enregistrer l'exemplaire"}</button></footer>
    </section>
  );
}

function buildGroups(assets: PhysicalAsset[], grouping: GroupingMode) {
  const grouped = groupBy(assets, (asset) => {
    if (grouping === "location") return locationDisplay(asset);
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
function reconciliationCandidateLabel(candidate: ModelReconciliationCandidate) { return `${candidate.manufacturer} ${candidate.model_name}${candidate.variant ? ` ${candidate.variant}` : ""} · ${candidate.category_path.join(" > ")} · Version ${candidate.revision_number} · ${revisionStatusLabel(candidate.lifecycle_status)}${candidate.approved_at ? ` · ${formatDate(candidate.approved_at)}` : ""}`; }
function revisionStatusLabel(value: ModelReconciliationCandidate["lifecycle_status"]) { return value === "approved" ? "Approuvée" : "Remplacée"; }
function metrologyGroup(asset: PhysicalAsset) { return metrologyStatusGroup(asset.metrology); }
function metrologyLabel(asset: PhysicalAsset) { return metrologyStatusLabel(asset.metrology); }
function serviceStateLabel(value: ServiceState) { return serviceStateChoices.find(([key]) => key === value)?.[1] ?? value; }
function serviceStateForcesUnavailability(value: ServiceState) { return value === "in_maintenance" || value === "out_of_service" || value === "retired"; }
function locationDisplay(asset: PhysicalAsset) { if (!asset.laboratory_location_label) return "Emplacement non défini"; return asset.laboratory_location_status === "archived" ? `${asset.laboratory_location_label} (archivé)` : asset.laboratory_location_label; }
function administrativeAvailabilityLabel(value: AdministrativeAvailability) { return administrativeAvailabilityChoices.find(([key]) => key === value)?.[1] ?? value; }
function operationalUsageLabel(value: PhysicalAsset["availability_state"]) { return operationalUsageChoices.find(([key]) => key === value)?.[1] ?? value; }
function formatUsageInterval(startAt: string, endAt: string | null) { return `${formatDateTime(startAt)}${endAt ? ` au ${formatDateTime(endAt)}` : ""}`; }
function formatDateTime(value: string) { return new Intl.DateTimeFormat("fr-FR", { dateStyle: "medium", timeStyle: "short" }).format(new Date(value)); }
function errorMessage(error: unknown) { return error instanceof Error ? error.message : "Erreur inattendue."; }
function modelLabel(models: EquipmentModelAggregate[], modelId: string) { const model = models.find((candidate) => candidate.identity.equipment_model_id === modelId); return model ? `${model.identity.manufacturer} ${model.identity.model_name}${model.identity.variant ? ` ${model.identity.variant}` : ""}` : "sélectionné"; }

const serviceStateChoices: Array<[ServiceState, string]> = [["usable", "Utilisable"], ["restricted", "Utilisation restreinte"], ["in_maintenance", "En maintenance"], ["out_of_service", "Hors service"], ["retired", "Retiré du parc"]];
const administrativeAvailabilityChoices: Array<[AdministrativeAvailability, string]> = [["available", "Disponible"], ["unavailable", "Indisponible"]];
const operationalUsageChoices: Array<[PhysicalAsset["availability_state"], string]> = [["available", "Disponible"], ["reserved", "Réservé"], ["assigned_to_setup", "Référencé par un montage"], ["in_test", "Utilisé en essai"], ["unavailable", "Indisponible"]];
const ownershipChoices: Array<[OwnershipSource, string]> = [["laboratory_owned", "Propriété du laboratoire"], ["customer_supplied", "Fourni par le client"], ["rented", "Loué"], ["borrowed", "Emprunté"], ["external", "Externe"], ["software_license", "Licence logicielle"], ["installed_facility", "Installation fixe"]];
