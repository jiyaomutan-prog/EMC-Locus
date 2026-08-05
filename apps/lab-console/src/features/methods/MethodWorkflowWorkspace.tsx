import {
  AlertCircle,
  Archive,
  ArrowLeft,
  ArrowRight,
  BookOpenCheck,
  Check,
  ChevronDown,
  ChevronRight,
  CircleHelp,
  Eye,
  FileClock,
  Focus,
  GitBranch,
  ListTree,
  Network,
  Pencil,
  Plus,
  Redo2,
  RotateCcw,
  Save,
  Search,
  Settings2,
  SlidersHorizontal,
  Trash2,
  Undo2,
  X,
  ZoomIn,
  ZoomOut
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { ApiError, methodWorkflowApi, type OperationContext } from "../../api";
import type {
  ExecutionPlanPreview,
  FunctionalRole,
  MeasurementSystemDefinition,
  MethodHierarchyNode,
  MethodHierarchyNodeKind,
  MethodVariableV2,
  MethodWorkflowAggregate,
  MethodWorkflowRevision,
  RegulationProfileDefinition,
  SubRangeDefinition,
  SubRangePreview,
  TestMethodDefinitionV2,
  TopologyEdge,
  TopologyNode,
  WorkflowAggregate,
} from "../../models/methodWorkflow";
import { isMethodV2 } from "../../models/methodWorkflow";
import { InstrumentIcon } from "./InstrumentIcon";

type WorkflowSpace = "methods" | "systems" | "regulation";
type EditorMode = "guided" | "outline";
type TopologyLayer = "physical" | "regulation" | "control" | "all";

const context: OperationContext = {
  actor: "method.engineer",
  reason: "Conception depuis LAB CONSOLE"
};

const workflowSteps = [
  "Identité et classement",
  "Objectif et périmètre",
  "Paramètres et variables",
  "Fonctions instrumentales",
  "Système de mesure",
  "Séquence et sous-plages",
  "Régulation et surveillance",
  "Limites et verdicts",
  "Traitements et résultats",
  "Relecture et publication"
] as const;

const hierarchyLabels: Record<MethodHierarchyNodeKind, string> = {
  domain: "Domaine",
  test_family: "Famille d'essais",
  source_document: "Document source",
  applicable_edition: "Édition applicable",
  procedure: "Procédure",
  method_variant: "Variante de méthode",
  parameter_profile: "Profil de paramètres",
  sub_range_profile: "Profil de sous-plage"
};

const semanticLabels: Record<string, string> = {
  method_parameter: "Paramètre de méthode",
  project_eut_input: "Donnée projet ou objet testé",
  operator_input: "Saisie opérateur",
  derived_value: "Valeur calculée",
  setpoint: "Consigne",
  observed_signal: "Signal mesuré",
  monitoring_signal: "Signal de surveillance",
  intermediate_result: "Résultat intermédiaire",
  final_result: "Résultat final",
  verdict_output: "Verdict"
};

const roleLabels: Record<string, string> = {
  generator: "Générateur de signal",
  disturbance_generator: "Générateur de perturbation",
  amplifier: "Amplificateur RF",
  injection_device: "Dispositif d'injection",
  forward_power_monitor: "Mesure de puissance directe",
  reflected_power_monitor: "Mesure de puissance réfléchie",
  disturbance_monitor: "Mesure de la perturbation",
  measurement_receiver: "Récepteur de mesure",
  sensor: "Capteur ou sonde",
  daq: "Acquisition temporelle",
  eut_monitor: "Surveillance de l'objet testé",
  generic: "Fonction générique"
};

const directionLabels: Record<string, string> = {
  input: "Entrée",
  output: "Sortie",
  bidirectional: "Bidirectionnel"
};

const signalDomainLabels: Record<string, string> = {
  power_dc: "Alimentation continue",
  power_ac: "Alimentation alternative",
  rf: "Radiofréquence",
  analog_voltage: "Tension analogique",
  analog_current: "Courant analogique",
  analog_charge: "Charge analogique",
  digital_logic: "Logique numérique",
  trigger: "Déclenchement",
  pulse: "Impulsion",
  contact_dry: "Contact sec",
  relay: "Relais",
  can_bus: "Bus CAN",
  rs232: "RS-232",
  rs485: "RS-485",
  ethernet: "Ethernet / données",
  usb: "USB",
  gpib: "GPIB",
  optical: "Optique",
  mechanical: "Mécanique",
  environmental: "Environnement",
  software: "Logiciel"
};

const edgeLabels: Record<string, string> = {
  physical_signal: "Signal physique",
  excitation_or_power: "Excitation / puissance",
  control_command: "Commande",
  feedback_measurement: "Retour de régulation",
  monitoring: "Surveillance",
  trigger: "Déclenchement",
  synchronization: "Synchronisation",
  data_stream: "Données",
  eut_state: "État de l'objet testé"
};

function id(prefix: string) {
  return `${prefix}-${crypto.randomUUID().slice(0, 8)}`;
}

function message(error: unknown) {
  if (error instanceof ApiError) return `${error.message} (${error.code})`;
  return error instanceof Error ? error.message : String(error);
}

function latestRevision<T>(aggregate: WorkflowAggregate<T> | undefined) {
  return aggregate?.revisions[0] ?? null;
}

function selectedMethodRevision(method: MethodWorkflowAggregate | null) {
  return method?.active_draft_revision ?? method?.current_approved_revision ?? method?.latest_revision ?? null;
}

export function MethodWorkflowWorkspace(props: { onOpenPlanning?: () => void }) {
  const [space, setSpace] = useState<WorkflowSpace>("methods");
  const [nodes, setNodes] = useState<MethodHierarchyNode[]>([]);
  const [methods, setMethods] = useState<MethodWorkflowAggregate[]>([]);
  const [systems, setSystems] = useState<WorkflowAggregate<MeasurementSystemDefinition>[]>([]);
  const [profiles, setProfiles] = useState<WorkflowAggregate<RegulationProfileDefinition>[]>([]);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [selectedMethod, setSelectedMethod] = useState<MethodWorkflowAggregate | null>(null);
  const [selectedSystemId, setSelectedSystemId] = useState<string | null>(null);
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [includeArchived, setIncludeArchived] = useState(false);
  const [loading, setLoading] = useState(true);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [notice, setNotice] = useState<string | null>(null);

  async function load() {
    setLoading(true);
    const [hierarchy, methodList, systemList, profileList] = await Promise.allSettled([
      methodWorkflowApi.listHierarchy(),
      methodWorkflowApi.listMethods(),
      methodWorkflowApi.listSystems(),
      methodWorkflowApi.listRegulationProfiles()
    ]);
    const nextErrors: Record<string, string> = {};
    if (hierarchy.status === "fulfilled") setNodes(hierarchy.value.nodes);
    else nextErrors.hierarchy = message(hierarchy.reason);
    if (methodList.status === "fulfilled") setMethods(methodList.value.test_templates);
    else nextErrors.methods = message(methodList.reason);
    if (systemList.status === "fulfilled") setSystems(systemList.value.definitions);
    else nextErrors.systems = message(systemList.reason);
    if (profileList.status === "fulfilled") setProfiles(profileList.value.definitions);
    else nextErrors.profiles = message(profileList.reason);
    setErrors(nextErrors);
    setLoading(false);
  }

  useEffect(() => void load(), []);

  const methodCountByCategory = useMemo(() => {
    const counts = new Map<string, number>();
    for (const method of methods) {
      const key = method.identity.category_code;
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
    return counts;
  }, [methods]);

  return (
    <section className="methodWorkflow" aria-label="Conception des méthodes et systèmes de mesure">
      <div className="methodWorkflowBar">
        <div className="segmented" aria-label="Espace de conception">
          <button className={space === "methods" ? "active" : ""} onClick={() => setSpace("methods")}><BookOpenCheck size={16} /> Méthodes</button>
          <button className={space === "systems" ? "active" : ""} onClick={() => setSpace("systems")}><Network size={16} /> Systèmes de mesure</button>
          <button className={space === "regulation" ? "active" : ""} onClick={() => setSpace("regulation")}><SlidersHorizontal size={16} /> Régulation</button>
        </div>
        <button className="secondaryButton" onClick={() => void load()}><RotateCcw size={15} /> Actualiser</button>
      </div>
      {notice && <div className="workflowNotice" role="status"><Check size={16} />{notice}<button aria-label="Fermer le message" onClick={() => setNotice(null)}><X size={14} /></button></div>}
      {loading && <div className="workflowLoading">Chargement du référentiel local…</div>}
      {!loading && space === "methods" && (
        <div className="methodWorkflowLayout">
          <HierarchyPanel
            nodes={nodes}
            selectedId={selectedNodeId}
            query={query}
            includeArchived={includeArchived}
            counts={methodCountByCategory}
            error={errors.hierarchy}
            onQuery={setQuery}
            onIncludeArchived={setIncludeArchived}
            onSelect={setSelectedNodeId}
            onChanged={async (text) => { setNotice(text); await load(); }}
          />
          <MethodLibraryPanel
            methods={methods}
            query={query}
            selectedNode={nodes.find((node) => node.node_id === selectedNodeId) ?? null}
            selectedMethod={selectedMethod}
            error={errors.methods}
            systems={systems}
            profiles={profiles}
            onSelect={setSelectedMethod}
            onChanged={async (text, method) => { setNotice(text); await load(); if (method) setSelectedMethod(method); }}
            onOpenPlanning={props.onOpenPlanning}
          />
        </div>
      )}
      {!loading && space === "systems" && (
        <MeasurementSystemWorkspace
          systems={systems}
          profiles={profiles}
          methods={methods}
          selectedId={selectedSystemId}
          error={errors.systems}
          onSelect={setSelectedSystemId}
          onChanged={async (text) => { setNotice(text); await load(); }}
        />
      )}
      {!loading && space === "regulation" && (
        <RegulationWorkspace
          profiles={profiles}
          methods={methods}
          selectedId={selectedProfileId}
          error={errors.profiles}
          onSelect={setSelectedProfileId}
          onChanged={async (text) => { setNotice(text); await load(); }}
        />
      )}
    </section>
  );
}

function HierarchyPanel(props: {
  nodes: MethodHierarchyNode[];
  selectedId: string | null;
  query: string;
  includeArchived: boolean;
  counts: Map<string, number>;
  error?: string;
  onQuery: (value: string) => void;
  onIncludeArchived: (value: boolean) => void;
  onSelect: (id: string | null) => void;
  onChanged: (notice: string) => Promise<void>;
}) {
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [editing, setEditing] = useState<MethodHierarchyNode | "new" | null>(null);
  const [label, setLabel] = useState("");
  const [kind, setKind] = useState<MethodHierarchyNodeKind>("test_family");
  const [parentId, setParentId] = useState("");
  const [position, setPosition] = useState(0);
  const visible = props.nodes.filter((node) => props.includeArchived || !node.archived);
  const byParent = (parent: string | undefined) => visible.filter((node) => (node.parent_node_id ?? undefined) === parent).sort((a, b) => a.position - b.position);
  const selected = props.nodes.find((node) => node.node_id === props.selectedId);
  const breadcrumbs = selected ? hierarchyBreadcrumbs(props.nodes, selected) : [];

  async function save() {
    if (!label.trim()) return;
    if (editing === "new") {
      await methodWorkflowApi.createHierarchyNode({
        node_id: id("hierarchy"),
        parent_node_id: parentId || undefined,
        node_kind: kind,
        label: label.trim(),
        position,
        archived: false
      }, context);
      await props.onChanged("Élément ajouté à la hiérarchie.");
    } else if (editing) {
      await methodWorkflowApi.updateHierarchyNode({ ...editing, parent_node_id: parentId || undefined, label: label.trim(), node_kind: kind, position }, context);
      await props.onChanged("Classement de la hiérarchie mis à jour.");
    }
    setEditing(null);
  }

  function renderNode(node: MethodHierarchyNode, depth: number): React.ReactNode {
    const children = byParent(node.node_id);
    const open = expanded.has(node.node_id) || Boolean(props.query);
    const matches = !props.query || node.label.toLowerCase().includes(props.query.toLowerCase()) || children.some((child) => child.label.toLowerCase().includes(props.query.toLowerCase()));
    if (!matches) return null;
    return (
      <li key={node.node_id}>
        <div className={`hierarchyRow${props.selectedId === node.node_id ? " selected" : ""}`} style={{ paddingLeft: 8 + depth * 16 }}>
          <button className="treeToggle" aria-label={`${open ? "Replier" : "Déplier"} ${node.label}`} disabled={!children.length} onClick={() => setExpanded((current) => { const next = new Set(current); if (next.has(node.node_id)) next.delete(node.node_id); else next.add(node.node_id); return next; })}>
            {children.length ? (open ? <ChevronDown size={14} /> : <ChevronRight size={14} />) : <span />}
          </button>
          <button className="treeLabel" onClick={() => props.onSelect(node.node_id)}>
            <span>{node.label}</span><small>{hierarchyLabels[node.node_kind]}</small>
          </button>
          <span className="countBadge" title="Méthodes directement classées">{props.counts.get(node.node_id) ?? 0}</span>
        </div>
        {open && children.length > 0 && <ul>{children.map((child) => renderNode(child, depth + 1))}</ul>}
      </li>
    );
  }

  return (
    <aside className="hierarchyPanel" aria-label="Hiérarchie des méthodes">
      <div className="panelHeading"><div><strong>Classement</strong><small>{visible.length} éléments</small></div><button className="iconButton" title="Ajouter sous la sélection" aria-label="Ajouter un élément" onClick={() => { const nextParent = selected?.node_id ?? ""; setEditing("new"); setLabel(""); setParentId(nextParent); setPosition(byParent(nextParent || undefined).length); }}><Plus size={16} /></button></div>
      <label className="workflowSearch"><Search size={15} /><input aria-label="Rechercher dans les méthodes" value={props.query} onChange={(event) => props.onQuery(event.target.value)} placeholder="Rechercher" /></label>
      <label className="compactCheck"><input type="checkbox" checked={props.includeArchived} onChange={(event) => props.onIncludeArchived(event.target.checked)} />Afficher les éléments archivés</label>
      {breadcrumbs.length > 0 && <div className="breadcrumbs" aria-label="Fil d'Ariane">{breadcrumbs.map((item, index) => <span key={item.node_id}>{index > 0 && " / "}{item.label}</span>)}</div>}
      {props.error ? <TargetedError title="Classement indisponible" detail={props.error} /> : visible.length ? <ul className="hierarchyTree">{byParent(undefined).map((node) => renderNode(node, 0))}</ul> : <EmptyHint title="Aucun classement" detail="Créez un domaine, puis organisez les familles et procédures du laboratoire." action="Créer le premier domaine" onAction={() => { setEditing("new"); setKind("domain"); setParentId(""); setPosition(0); }} />}
      {selected && <div className="hierarchyActions"><button onClick={() => { setEditing(selected); setLabel(selected.label); setKind(selected.node_kind); setParentId(selected.parent_node_id ?? ""); setPosition(selected.position); }}><Pencil size={14} />Modifier</button><button onClick={() => void methodWorkflowApi.updateHierarchyNode({ ...selected, archived: !selected.archived }, context).then(() => props.onChanged(selected.archived ? "Élément restauré." : "Élément archivé."))}><Archive size={14} />{selected.archived ? "Restaurer" : "Archiver"}</button></div>}
      {editing && <div className="inlineEditor"><strong>{editing === "new" ? "Nouvel élément" : "Modifier le classement"}</strong><label>Libellé *<input autoFocus value={label} onChange={(event) => setLabel(event.target.value)} /></label><label>Type<select value={kind} onChange={(event) => setKind(event.target.value as MethodHierarchyNodeKind)}>{Object.entries(hierarchyLabels).map(([value, text]) => <option key={value} value={value}>{text}</option>)}</select></label><label>Parent<select aria-label="Parent dans la hiérarchie" value={parentId} onChange={(event) => setParentId(event.target.value)}><option value="">Racine</option>{props.nodes.filter((node) => node.node_id !== (editing === "new" ? "" : editing.node_id) && !node.archived).map((node) => <option key={node.node_id} value={node.node_id}>{node.label}</option>)}</select></label><label>Position<input aria-label="Position dans le parent" type="number" min="0" value={position} onChange={(event) => setPosition(Number(event.target.value))} /><small>Un nombre plus petit place l'élément plus haut.</small></label><div><button onClick={() => setEditing(null)}>Annuler</button><button className="primaryButton" disabled={!label.trim()} onClick={() => void save()}>Enregistrer</button></div></div>}
    </aside>
  );
}

function MethodLibraryPanel(props: {
  methods: MethodWorkflowAggregate[];
  query: string;
  selectedNode: MethodHierarchyNode | null;
  selectedMethod: MethodWorkflowAggregate | null;
  systems: WorkflowAggregate<MeasurementSystemDefinition>[];
  profiles: WorkflowAggregate<RegulationProfileDefinition>[];
  error?: string;
  onSelect: (method: MethodWorkflowAggregate | null) => void;
  onChanged: (notice: string, method?: MethodWorkflowAggregate) => Promise<void>;
  onOpenPlanning?: () => void;
}) {
  const [createOpen, setCreateOpen] = useState(false);
  const [title, setTitle] = useState("");
  const filtered = props.methods.filter((method) => !props.query || `${method.identity.title} ${method.identity.template_id}`.toLowerCase().includes(props.query.toLowerCase()));

  async function create() {
    const templateId = id("METHOD").toUpperCase();
    const result = await methodWorkflowApi.createMethodV2(templateId, title.trim(), props.selectedNode?.node_id ?? "unclassified", defaultMethod(title.trim(), props.selectedNode?.node_id), context);
    setCreateOpen(false);
    setTitle("");
    await props.onChanged("Brouillon de méthode créé.", result.test_template);
  }

  if (props.selectedMethod) {
    return <MethodEditor method={props.selectedMethod} systems={props.systems} profiles={props.profiles} onBack={() => props.onSelect(null)} onChanged={props.onChanged} onOpenPlanning={props.onOpenPlanning} />;
  }
  return (
    <section className="methodLibraryPanel">
      <div className="libraryHeading"><div><p className="eyebrow">Bibliothèque du laboratoire</p><h2>{props.selectedNode?.label ?? "Toutes les méthodes"}</h2><p>Définitions réutilisables. Les exemplaires réels et les dates restent dans la préparation d'essai.</p></div><button className="primaryButton" onClick={() => setCreateOpen(true)}><Plus size={16} />Nouvelle méthode</button></div>
      {props.error ? <TargetedError title="Méthodes indisponibles" detail={props.error} /> : filtered.length ? <div className="methodRows">{filtered.map((method) => {
        const revision = selectedMethodRevision(method);
        const legacy = revision?.definition_schema_version !== "emc-locus.test-method-definition.v2";
        return <button className="methodRow" key={method.identity.template_id} onClick={() => props.onSelect(method)}><span className="methodRowIcon"><BookOpenCheck size={19} /></span><span><strong>{method.identity.title}</strong><small>{method.identity.template_id} · {method.identity.category_code}</small></span><span className={`statusBadge ${revision?.status ?? "draft"}`}>{revision?.status === "approved" ? "Approuvée" : revision?.status === "under_review" ? "En revue" : "Brouillon"}</span>{legacy && <span className="legacyBadge">Workflow historique</span>}<ChevronRight size={18} /></button>;
      })}</div> : <EmptyHint title="Aucune méthode dans cette vue" detail="Créez une méthode guidée ou choisissez un autre élément du classement." action="Créer une méthode" onAction={() => setCreateOpen(true)} />}
      {createOpen && <div className="dialogBackdrop" role="presentation"><div className="compactDialog" role="dialog" aria-modal="true" aria-label="Créer une méthode"><div className="dialogTitle"><h2>Nouvelle méthode</h2><button aria-label="Fermer" onClick={() => setCreateOpen(false)}><X size={16} /></button></div><label>Nom de la méthode *<input autoFocus value={title} onChange={(event) => setTitle(event.target.value)} /></label><p>Le brouillon sera classé sous « {props.selectedNode?.label ?? "Non classé"} ».</p><div className="dialogActions"><button onClick={() => setCreateOpen(false)}>Annuler</button><button className="primaryButton" disabled={!title.trim()} onClick={() => void create()}>Créer le brouillon</button></div></div></div>}
    </section>
  );
}

function MethodEditor(props: {
  method: MethodWorkflowAggregate;
  systems: WorkflowAggregate<MeasurementSystemDefinition>[];
  profiles: WorkflowAggregate<RegulationProfileDefinition>[];
  onBack: () => void;
  onChanged: (notice: string, method?: MethodWorkflowAggregate) => Promise<void>;
  onOpenPlanning?: () => void;
}) {
  const sourceRevision = selectedMethodRevision(props.method);
  const [revision, setRevision] = useState<MethodWorkflowRevision | null>(sourceRevision);
  const [definition, setDefinition] = useState<TestMethodDefinitionV2 | null>(isMethodV2(sourceRevision?.definition) ? sourceRevision.definition : null);
  const [original, setOriginal] = useState(() => JSON.stringify(sourceRevision?.definition));
  const [step, setStep] = useState(0);
  const [mode, setMode] = useState<EditorMode>("guided");
  const [validation, setValidation] = useState<{ valid: boolean; issues: Array<{ code: string; path: string; message: string }> } | null>(null);
  const [plan, setPlan] = useState<ExecutionPlanPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dirty = definition ? JSON.stringify(definition) !== original : false;
  const readOnly = revision?.status !== "draft";

  useEffect(() => {
    const listener = (event: BeforeUnloadEvent) => { if (dirty) event.preventDefault(); };
    window.addEventListener("beforeunload", listener);
    return () => window.removeEventListener("beforeunload", listener);
  }, [dirty]);

  async function convertLegacy() {
    if (!sourceRevision) return;
    setBusy(true);
    setError(null);
    try {
      const result = await methodWorkflowApi.createMethodSuccessor(props.method.identity.template_id, sourceRevision.revision_id, context);
      setRevision(result.revision);
      if (isMethodV2(result.revision.definition)) {
        setDefinition(result.revision.definition);
        setOriginal(JSON.stringify(result.revision.definition));
      }
      await props.onChanged("Nouvelle version 0.22.2 créée en brouillon.", result.test_template);
    } catch (cause) { setError(message(cause)); } finally { setBusy(false); }
  }

  async function save() {
    if (!revision || !definition || readOnly) return;
    setBusy(true); setError(null);
    try {
      const result = await methodWorkflowApi.saveMethodDraft(props.method.identity.template_id, revision.revision_id, revision.definition_checksum, definition, context);
      setRevision(result.revision);
      if (isMethodV2(result.revision.definition)) {
        setDefinition(result.revision.definition);
        setOriginal(JSON.stringify(result.revision.definition));
      }
      await props.onChanged("Brouillon enregistré.", result.test_template);
    } catch (cause) { setError(message(cause)); } finally { setBusy(false); }
  }

  async function validate() {
    if (!definition) return;
    setBusy(true); setError(null);
    try { setValidation(await methodWorkflowApi.validateMethod(definition)); }
    catch (cause) { setError(message(cause)); }
    finally { setBusy(false); }
  }

  async function previewPlan() {
    if (!definition || !revision) return;
    const systemRef = definition.measurement_system_templates[0];
    const system = props.systems.find((item) => item.identity.entity_id === systemRef?.identity_id);
    const systemRevision = system?.revisions.find((item) => item.revision_id === systemRef?.revision_id) ?? latestRevision(system);
    if (!systemRevision) { setError("Associez d'abord un système de mesure réutilisable."); return; }
    const selectedProfiles = props.profiles.flatMap((aggregate) => aggregate.revisions.filter((candidate) => definition.regulation_profiles.some((ref) => ref.revision_id === candidate.revision_id)).map((candidate) => candidate.definition));
    setBusy(true); setError(null);
    try { setPlan((await methodWorkflowApi.previewExecutionPlan(props.method.identity.template_id, revision.revision_id, definition, systemRevision.definition, selectedProfiles, context)).preview); }
    catch (cause) { setError(message(cause)); }
    finally { setBusy(false); }
  }

  async function transition(transitionName: "submit-for-review" | "approve") {
    if (!revision) return;
    setBusy(true); setError(null);
    try {
      const result = await methodWorkflowApi.transitionMethod(props.method.identity.template_id, revision.revision_id, transitionName, context);
      setRevision(result.revision);
      await props.onChanged(transitionName === "approve" ? "Méthode approuvée." : "Méthode soumise à la revue.", result.test_template);
    } catch (cause) { setError(message(cause)); }
    finally { setBusy(false); }
  }

  if (!sourceRevision) return <EmptyHint title="Aucune révision" detail="Cette identité ne possède pas encore de définition exploitable." />;
  if (!definition) return (
    <section className="legacyMethodPanel">
      <button className="backButton" onClick={props.onBack}><ArrowLeft size={16} />Bibliothèque</button>
      <div className="legacyMethodMessage"><FileClock size={32} /><div><p className="eyebrow">Définition historique en lecture seule</p><h2>{props.method.identity.title}</h2><p>Cette révision reste intacte et lisible. Le nouveau workflow exige des fonctions instrumentales, une topologie explicite et des limites classifiées.</p><button className="primaryButton" disabled={busy} onClick={() => void convertLegacy()}>Créer une nouvelle version avec le workflow 0.22.2</button>{error && <TargetedError title="Conversion refusée" detail={error} />}</div></div>
      <details className="technicalDetails"><summary>Détails techniques</summary><pre>{JSON.stringify(sourceRevision.definition, null, 2)}</pre></details>
    </section>
  );

  const completion = methodStepCompletion(definition);
  return (
    <section className="methodEditor">
      <div className="editorHeader"><button className="backButton" onClick={props.onBack}><ArrowLeft size={16} />Bibliothèque</button><div><p className="eyebrow">Méthode · révision {revision?.revision_number}</p><h2>{definition.title}</h2></div><span className={`statusBadge ${revision?.status}`}>{readOnly ? "Lecture seule" : "Brouillon"}</span><span className={`saveIndicator ${dirty ? "dirty" : ""}`}>{dirty ? "Modifications non enregistrées" : "À jour"}</span><button className="primaryButton" disabled={!dirty || busy || readOnly} title={readOnly ? "Une révision soumise ou approuvée est immuable" : !dirty ? "Aucune modification à enregistrer" : "Enregistrer"} onClick={() => void save()}><Save size={15} />Enregistrer</button></div>
      <div className="editorMode"><div className="segmented"><button className={mode === "guided" ? "active" : ""} onClick={() => setMode("guided")}><CircleHelp size={15} />Parcours guidé</button><button className={mode === "outline" ? "active" : ""} onClick={() => setMode("outline")}><ListTree size={15} />Vue d'ensemble</button></div><span>{completion.filter(Boolean).length} / {workflowSteps.length} étapes renseignées</span></div>
      {error && <TargetedError title="Action impossible" detail={error} />}
      <div className={`methodEditorBody ${mode}`}>
        <nav className="methodStepRail" aria-label="Étapes de définition">{workflowSteps.map((label, index) => <button key={label} className={`${step === index ? "active" : ""} ${completion[index] ? "complete" : ""}`} onClick={() => setStep(index)}><span>{completion[index] ? <Check size={14} /> : index + 1}</span><strong>{label}</strong>{!completion[index] && <AlertCircle size={14} aria-label="À compléter" />}</button>)}</nav>
        <div className="methodStepContent">
          <div className="stepTitle"><div><p className="eyebrow">Étape {step + 1} sur {workflowSteps.length}</p><h3>{workflowSteps[step]}</h3></div>{!completion[step] && <span className="warningBadge">Informations manquantes</span>}</div>
          <MethodStep step={step} definition={definition} systems={props.systems} profiles={props.profiles} readOnly={readOnly} onChange={(next) => { setDefinition(next); setValidation(null); setPlan(null); }} onPreviewPlan={() => void previewPlan()} plan={plan} onOpenPlanning={props.onOpenPlanning} />
          {validation && <ValidationPanel result={validation} />}
          <div className="stepFooter"><button disabled={step === 0} onClick={() => setStep((value) => value - 1)}><ArrowLeft size={15} />Précédent</button><button onClick={() => void validate()} disabled={busy}>Vérifier la définition</button>{step < workflowSteps.length - 1 ? <button className="primaryButton" onClick={() => setStep((value) => value + 1)}>Étape suivante<ArrowRight size={15} /></button> : revision?.status === "under_review" ? <button className="primaryButton" disabled={busy} onClick={() => void transition("approve")}><Check size={15} />Approuver</button> : <button className="primaryButton" disabled={!validation?.valid || dirty || readOnly} onClick={() => void transition("submit-for-review")}><BookOpenCheck size={15} />Soumettre à la revue</button>}</div>
          {step === workflowSteps.length - 1 && revision?.status === "draft" && (!validation?.valid || dirty) && <p className="disabledExplanation">{dirty ? "Enregistrez les modifications avant de soumettre la méthode." : "Vérifiez la définition et corrigez les omissions bloquantes avant la revue."}</p>}
          <details className="technicalDetails"><summary><Settings2 size={14} />Détails techniques</summary><pre>{JSON.stringify(definition, null, 2)}</pre></details>
        </div>
      </div>
    </section>
  );
}

function MethodStep(props: {
  step: number;
  definition: TestMethodDefinitionV2;
  systems: WorkflowAggregate<MeasurementSystemDefinition>[];
  profiles: WorkflowAggregate<RegulationProfileDefinition>[];
  readOnly: boolean;
  onChange: (definition: TestMethodDefinitionV2) => void;
  onPreviewPlan: () => void;
  plan: ExecutionPlanPreview | null;
  onOpenPlanning?: () => void;
}) {
  const d = props.definition;
  if (props.step === 0) return <div className="formGrid"><Field label="Nom de la méthode" required value={d.title} disabled={props.readOnly} onChange={(title) => props.onChange({ ...d, title })} /><Field label="Chemin de classement" required value={d.classification_path.join(" / ")} disabled={props.readOnly} help="Domaine, famille, document et variante" onChange={(value) => props.onChange({ ...d, classification_path: value.split("/").map((item) => item.trim()).filter(Boolean) })} /><Field label="Références du laboratoire" value={d.standard_references.join(", ")} disabled={props.readOnly} help="Références documentaires, sans déclaration automatique de conformité" onChange={(value) => props.onChange({ ...d, standard_references: value.split(",").map((item) => item.trim()).filter(Boolean) })} /></div>;
  if (props.step === 1) return <div className="formGrid"><LongField label="Objectif de l'essai" required value={d.objective} disabled={props.readOnly} onChange={(objective) => props.onChange({ ...d, objective })} /><LongField label="Périmètre et exclusions" required value={d.scope} disabled={props.readOnly} onChange={(scope) => props.onChange({ ...d, scope })} /></div>;
  if (props.step === 2) return <VariableEditor variables={d.variables} readOnly={props.readOnly} onChange={(variables) => props.onChange({ ...d, variables })} />;
  if (props.step === 3) return <RoleEditor roles={d.functional_roles} readOnly={props.readOnly} onChange={(functional_roles) => props.onChange({ ...d, functional_roles })} />;
  if (props.step === 4) return <ReferencePicker title="Système de mesure associé" empty="Créez et approuvez d'abord un système de mesure réutilisable." items={props.systems} selected={d.measurement_system_templates} readOnly={props.readOnly} onSelect={(ref) => props.onChange({ ...d, measurement_system_templates: ref ? [ref] : [] })} />;
  if (props.step === 5) return <ProcedureEditor definition={d} readOnly={props.readOnly} onChange={props.onChange} />;
  if (props.step === 6) return <ReferencePicker title="Profils de régulation" empty="Les profils ouverts, fermés ou de surveillance sont gérés dans l'espace Régulation." items={props.profiles} selected={d.regulation_profiles} multiple readOnly={props.readOnly} onSelect={(ref) => ref && props.onChange({ ...d, regulation_profiles: [...d.regulation_profiles.filter((item) => item.identity_id !== ref.identity_id), ref] })} />;
  if (props.step === 7) return <SimpleCollection title="Limites classifiées" count={d.limits.length} empty="Définissez séparément sécurité, protection instrument, acceptation de procédure et verdict objet testé." />;
  if (props.step === 8) return <SimpleCollection title="Traitements demandés" count={d.post_processing.length} empty="Décrivez les entrées, sorties et dépendances. Les opérations restent des contrats non exécutés en 0.22.2." />;
  return <ReviewStep definition={d} systems={props.systems} profiles={props.profiles} onPreview={props.onPreviewPlan} preview={props.plan} onOpenPlanning={props.onOpenPlanning} />;
}

function VariableEditor(props: { variables: MethodVariableV2[]; readOnly: boolean; onChange: (items: MethodVariableV2[]) => void }) {
  const [adding, setAdding] = useState(false);
  const [label, setLabel] = useState("");
  const [semantic, setSemantic] = useState("method_parameter");
  const [unit, setUnit] = useState("MHz");
  function add() {
    const variable: MethodVariableV2 = { variable_id: id("var"), label: label.trim(), description: `Valeur ${label.trim()} définie par le laboratoire.`, semantic, value_type: "number", dimension: unit === "MHz" ? "frequency" : unit === "s" ? "time" : "dimensionless", unit: unit || undefined, required: true, source: "laboratory_method", availability_phase: "definition", consumers: [] };
    props.onChange([...props.variables, variable]); setAdding(false); setLabel("");
  }
  return <div><div className="sectionCommand"><div><h4>Variables typées</h4><p>Chaque valeur indique sa provenance, sa phase de disponibilité et ses consommateurs.</p></div><button disabled={props.readOnly} onClick={() => setAdding(true)}><Plus size={15} />Ajouter</button></div>{props.variables.length ? <div className="structuredTable" role="table" aria-label="Variables de la méthode">{props.variables.map((variable) => <div className="structuredRow" role="row" key={variable.variable_id}><span><strong>{variable.label}</strong><small>{variable.variable_id}</small></span><span>{semanticLabels[variable.semantic] ?? variable.semantic}</span><span>{variable.unit ?? "Sans unité"}</span><span>{variable.required ? "Obligatoire" : "Optionnelle"}</span><button className="iconButton" disabled={props.readOnly} aria-label={`Supprimer ${variable.label}`} onClick={() => props.onChange(props.variables.filter((item) => item.variable_id !== variable.variable_id))}><Trash2 size={15} /></button></div>)}</div> : <EmptyHint title="Aucune variable" detail="Commencez par les entrées indispensables, puis reliez les consignes et résultats." action={props.readOnly ? undefined : "Ajouter une variable"} onAction={() => setAdding(true)} />}{adding && <div className="inlineEditor horizontal"><Field label="Libellé" required value={label} onChange={setLabel} /><label>Sens métier<select value={semantic} onChange={(event) => setSemantic(event.target.value)}>{Object.entries(semanticLabels).map(([value, text]) => <option value={value} key={value}>{text}</option>)}</select></label><label>Unité<select value={unit} onChange={(event) => setUnit(event.target.value)}><option value="">Sans unité</option><option>Hz</option><option>kHz</option><option>MHz</option><option>GHz</option><option>V</option><option>A</option><option>W</option><option>s</option><option>dB</option></select></label><button className="primaryButton" disabled={!label.trim()} onClick={add}>Ajouter</button></div>}</div>;
}

function RoleEditor(props: { roles: FunctionalRole[]; readOnly: boolean; onChange: (items: FunctionalRole[]) => void }) {
  const [adding, setAdding] = useState(false);
  const [roleType, setRoleType] = useState("measurement_receiver");
  function add() {
    const roleId = id("role");
    const role: FunctionalRole = { role_id: roleId, label: roleLabels[roleType] ?? "Fonction instrumentale", purpose: "Assurer cette fonction dans le système de mesure.", required: true, functional_category: roleType, capabilities: [{ capability_kind: roleType === "measurement_receiver" ? "frequency_selective_measurement" : roleType }], calibration_policy: "required", substitution_policy: "same_capabilities", assignment_stage: "planned_test_preparation", logical_ports: [{ port_id: "signal", label: "Signal", directionality: roleType === "disturbance_generator" ? "output" : "input", signal_domain: "rf" }], consumes_variables: [], produces_variables: [] };
    props.onChange([...props.roles, role]); setAdding(false);
  }
  return <div><div className="sectionCommand"><div><h4>Fonctions nécessaires</h4><p>On décrit ce que le système doit faire. Les numéros de série seront affectés dans la préparation datée.</p></div><button disabled={props.readOnly} onClick={() => setAdding(true)}><Plus size={15} />Ajouter une fonction</button></div><div className="roleGrid">{props.roles.map((role) => <article className="roleTile" key={role.role_id}><InstrumentIcon roleType={role.functional_category} label={role.label} /><div><strong>{role.label}</strong><p>{role.purpose}</p><small>{role.required ? "Fonction obligatoire" : "Fonction optionnelle"} · {role.logical_ports.length} port(s)</small></div><button className="iconButton" disabled={props.readOnly} aria-label={`Supprimer ${role.label}`} onClick={() => props.onChange(props.roles.filter((item) => item.role_id !== role.role_id))}><Trash2 size={15} /></button></article>)}</div>{!props.roles.length && <EmptyHint title="Aucune fonction instrumentale" detail="Une méthode publiable doit demander au moins une fonction mesurable et vérifiable." action={props.readOnly ? undefined : "Ajouter une fonction"} onAction={() => setAdding(true)} />}{adding && <div className="inlineEditor horizontal"><label>Fonction<select value={roleType} onChange={(event) => setRoleType(event.target.value)}>{Object.entries(roleLabels).map(([value, text]) => <option key={value} value={value}>{text}</option>)}</select></label><button className="primaryButton" onClick={add}>Ajouter</button></div>}</div>;
}

function ReferencePicker<T>(props: { title: string; empty: string; items: WorkflowAggregate<T>[]; selected: Array<{ identity_id: string; revision_id: string; definition_checksum: string }>; multiple?: boolean; readOnly: boolean; onSelect: (ref: { identity_id: string; revision_id: string; definition_checksum: string } | null) => void }) {
  return <div><div className="sectionCommand"><div><h4>{props.title}</h4><p>La méthode enregistre une révision exacte et son empreinte, jamais une copie silencieusement divergente.</p></div></div>{props.items.length ? <div className="referenceChoices">{props.items.map((item) => { const revision = item.revisions.find((candidate) => candidate.status === "approved") ?? item.revisions[0]; const checked = props.selected.some((ref) => ref.revision_id === revision?.revision_id); return <label key={item.identity.entity_id} className={checked ? "selected" : ""}><input type={props.multiple ? "checkbox" : "radio"} name={props.title} checked={checked} disabled={props.readOnly || !revision} onChange={() => revision && props.onSelect(checked && props.multiple ? null : { identity_id: item.identity.entity_id, revision_id: revision.revision_id, definition_checksum: revision.definition_checksum })} /><Network size={20} /><span><strong>{item.identity.label}</strong><small>Révision {revision?.revision_number ?? "indisponible"} · {revision?.status ?? "sans révision"}</small></span></label>; })}</div> : <EmptyHint title="Prérequis manquant" detail={props.empty} />}</div>;
}

function ProcedureEditor(props: { definition: TestMethodDefinitionV2; readOnly: boolean; onChange: (definition: TestMethodDefinitionV2) => void }) {
  const d = props.definition;
  const range = d.sub_ranges[0];
  const [preview, setPreview] = useState<SubRangePreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  async function previewRange(candidate: SubRangeDefinition) {
    setError(null);
    try { setPreview((await methodWorkflowApi.previewSubRange(candidate, 200)).preview); }
    catch (cause) { setError(message(cause)); }
  }
  function ensureRange() {
    const next = defaultSubRange(); props.onChange({ ...d, sub_ranges: [next] }); void previewRange(next);
  }
  return <div className="procedureLayout"><section><div className="sectionCommand"><div><h4>Procédure hiérarchique</h4><p>Les boucles doivent toujours avoir une borne explicite.</p></div></div>{d.procedure.length ? <ProcedureTree nodes={d.procedure} /> : <EmptyHint title="Aucune phase" detail="Ajoutez une phase de préparation, de balayage, d'acquisition et d'arrêt sûr avant publication." />}</section><section><div className="sectionCommand"><div><h4>Sous-plages</h4><p>L'agent calcule le nombre de points sans dérouler une plage illimitée dans le navigateur.</p></div>{!range && <button disabled={props.readOnly} onClick={ensureRange}><Plus size={15} />Créer</button>}</div>{range && <div className="subRangeEditor"><Field label="Début" value={String(range.start_frequency.value)} disabled={props.readOnly} onChange={(value) => { const next = { ...range, start_frequency: { ...range.start_frequency, value: Number(value) } }; props.onChange({ ...d, sub_ranges: [next] }); }} /><Field label="Fin" value={String(range.stop_frequency.value)} disabled={props.readOnly} onChange={(value) => { const next = { ...range, stop_frequency: { ...range.stop_frequency, value: Number(value) } }; props.onChange({ ...d, sub_ranges: [next] }); }} /><Field label="Pas" value={range.progression.kind === "fixed_step" ? String(range.progression.step.value) : ""} disabled={props.readOnly || range.progression.kind !== "fixed_step"} onChange={(value) => { if (range.progression.kind !== "fixed_step") return; const next = { ...range, progression: { ...range.progression, step: { ...range.progression.step, value: Number(value) } } }; props.onChange({ ...d, sub_ranges: [next] }); }} /><button onClick={() => void previewRange(range)}><Eye size={15} />Prévisualiser</button>{preview && <div className="rangePreview"><strong>{preview.point_count.toLocaleString("fr-FR")} points</strong><small>{preview.truncated ? "Aperçu limité, définition conservée" : "Plage bornée et calculable"}</small></div>}{error && <TargetedError title="Sous-plage invalide" detail={error} />}</div>}</section></div>;
}

function ProcedureTree(props: { nodes: import("../../models/methodWorkflow").ProcedureNode[]; depth?: number }) {
  return <ol className="procedureTree">{props.nodes.map((node) => { const children = node.children ?? []; return <li key={node.node_id} style={{ marginLeft: (props.depth ?? 0) * 16 }}><span className="procedureKind">{node.node_kind.replaceAll("_", " ")}</span><strong>{node.label}</strong>{node.maximum_iterations && <small>Maximum {node.maximum_iterations} itérations</small>}{children.length > 0 && <ProcedureTree nodes={children} depth={(props.depth ?? 0) + 1} />}</li>; })}</ol>;
}

function ReviewStep(props: { definition: TestMethodDefinitionV2; systems: WorkflowAggregate<MeasurementSystemDefinition>[]; profiles: WorkflowAggregate<RegulationProfileDefinition>[]; onPreview: () => void; preview: ExecutionPlanPreview | null; onOpenPlanning?: () => void }) {
  const completion = methodStepCompletion(props.definition);
  return <div className="reviewLayout"><div className="reviewChecklist">{workflowSteps.map((label, index) => <div key={label} className={completion[index] ? "done" : "missing"}>{completion[index] ? <Check size={16} /> : <AlertCircle size={16} />}<span><strong>{label}</strong><small>{completion[index] ? "Renseigné" : "À compléter avant publication"}</small></span></div>)}</div><div className="executionPreview"><div className="sectionCommand"><div><h4>Aperçu du contrat d'exécution</h4><p>Compilation explicative uniquement. Aucun instrument ne sera commandé.</p></div><button className="primaryButton" onClick={props.onPreview}><Eye size={15} />Compiler l'aperçu</button></div>{props.preview ? <><PlanPreview preview={props.preview} />{props.onOpenPlanning && <div className="datedPreparationAction"><strong>Passer à une réalisation datée</strong><p>Le planning réutilise la préparation 0.22.1 pour affecter les exemplaires réels, les ports, le lieu et les preuves métrologiques.</p><button onClick={props.onOpenPlanning}>Ouvrir le planning du laboratoire<ArrowRight size={15} /></button></div>}</> : <EmptyHint title="Aucun aperçu compilé" detail="Associez une topologie puis demandez à l'agent d'expliquer les phases et les prérequis." />}</div></div>;
}

function PlanPreview({ preview }: { preview: ExecutionPlanPreview }) {
  return <div className="planPreview"><div className={preview.blockers.length ? "planVerdict blocked" : "planVerdict ready"}>{preview.blockers.length ? <AlertCircle size={20} /> : <Check size={20} />}<span><strong>{preview.blockers.length ? "Préparation incomplète" : "Contrat compilable"}</strong><small>{preview.blockers.length} blocage(s), {preview.warnings.length} avertissement(s)</small></span></div><ol>{preview.ordered_phases.map((phase) => <li key={phase.node_id} style={{ marginLeft: phase.depth * 12 }}><strong>{phase.label}</strong><small>{phase.node_kind.replaceAll("_", " ")}</small></li>)}</ol>{preview.blockers.map((blocker) => <div className="planBlocker" key={`${blocker.code}-${blocker.message}`}><strong>{blocker.message}</strong><span>{blocker.next_action}</span></div>)}{preview.unsupported_runtime_operations.length > 0 && <p className="runtimeBoundary">Définies mais non exécutées en 0.22.2 : {preview.unsupported_runtime_operations.join(", ")}.</p>}</div>;
}

function MeasurementSystemWorkspace(props: { systems: WorkflowAggregate<MeasurementSystemDefinition>[]; profiles: WorkflowAggregate<RegulationProfileDefinition>[]; methods: MethodWorkflowAggregate[]; selectedId: string | null; error?: string; onSelect: (id: string | null) => void; onChanged: (notice: string) => Promise<void> }) {
  const selected = props.systems.find((system) => system.identity.entity_id === props.selectedId);
  const revision = latestRevision(selected);
  const [draft, setDraft] = useState<MeasurementSystemDefinition | null>(revision?.definition ?? null);
  const [history, setHistory] = useState<MeasurementSystemDefinition[]>(revision ? [revision.definition] : []);
  const [historyIndex, setHistoryIndex] = useState(0);
  useEffect(() => { const next = latestRevision(selected)?.definition ?? null; setDraft(next); setHistory(next ? [next] : []); setHistoryIndex(0); }, [selected]);
  function change(next: MeasurementSystemDefinition) { setDraft(next); setHistory((current) => [...current.slice(0, historyIndex + 1), next]); setHistoryIndex((index) => index + 1); }
  async function create() { const entityId = id("SYSTEM").toUpperCase(); await methodWorkflowApi.createWorkflowDefinition("measurement-system-templates", entityId, "Nouveau système de mesure", "laboratory", defaultSystem(entityId), {}, context); await props.onChanged("Système de mesure créé en brouillon."); props.onSelect(entityId); }
  async function save() { if (!selected || !revision || !draft) return; await methodWorkflowApi.saveWorkflowDraft("measurement-system-templates", selected.identity.entity_id, revision.revision_id, revision.definition_checksum, draft, { method_definition: firstMethodV2(props.methods), regulation_profiles: props.profiles.map(latestRevision).filter(Boolean).map((item) => item!.definition) }, context); await props.onChanged("Topologie enregistrée et validée par l'agent."); }
  async function transition(target: "validate" | "approve") { if (!selected || !revision) return; await methodWorkflowApi.transitionWorkflow("measurement-system-templates", selected.identity.entity_id, revision.revision_id, target, { method_definition: firstMethodV2(props.methods), regulation_profiles: props.profiles.map(latestRevision).filter(Boolean).map((item) => item!.definition) }, context); await props.onChanged(target === "approve" ? "Système de mesure approuvé." : "Système de mesure validé."); }
  if (props.error) return <TargetedError title="Systèmes de mesure indisponibles" detail={props.error} />;
  return <div className="systemWorkspace"><aside className="systemList"><div className="panelHeading"><div><strong>Bibliothèque des systèmes</strong><small>Topologies sans date ni matériel réel</small></div><button className="iconButton" aria-label="Créer un système" onClick={() => void create()}><Plus size={16} /></button></div>{props.systems.map((system) => <button key={system.identity.entity_id} className={props.selectedId === system.identity.entity_id ? "selected" : ""} onClick={() => props.onSelect(system.identity.entity_id)}><Network size={18} /><span><strong>{system.identity.label}</strong><small>{system.identity.classification}</small></span><span className={`statusBadge ${latestRevision(system)?.status}`}>{latestRevision(system)?.status === "approved" ? "Approuvé" : latestRevision(system)?.status === "validated" ? "Validé" : "Brouillon"}</span></button>)}{!props.systems.length && <EmptyHint title="Aucun système réutilisable" detail="Créez une topologie logique avant de l'associer à une méthode." action="Créer un système" onAction={() => void create()} />}</aside><section className="topologyWorkspace">{draft && revision ? <><div className="topologyToolbar"><div><p className="eyebrow">Système réutilisable · révision {revision.revision_number}</p><input className="titleInput" aria-label="Nom du système" value={draft.label} onChange={(event) => change({ ...draft, label: event.target.value })} /></div><div><button className="iconButton" title="Annuler" aria-label="Annuler" disabled={historyIndex === 0} onClick={() => { const nextIndex = historyIndex - 1; setHistoryIndex(nextIndex); setDraft(history[nextIndex]); }}><Undo2 size={16} /></button><button className="iconButton" title="Rétablir" aria-label="Rétablir" disabled={historyIndex >= history.length - 1} onClick={() => { const nextIndex = historyIndex + 1; setHistoryIndex(nextIndex); setDraft(history[nextIndex]); }}><Redo2 size={16} /></button>{revision.status === "draft" && <button disabled={JSON.stringify(draft) !== JSON.stringify(revision.definition)} title={JSON.stringify(draft) !== JSON.stringify(revision.definition) ? "Enregistrez avant validation" : "Valider le contrat"} onClick={() => void transition("validate")}><Check size={15} />Valider</button>}{revision.status === "validated" && <button onClick={() => void transition("approve")}><BookOpenCheck size={15} />Approuver</button>}<button className="primaryButton" disabled={JSON.stringify(draft) === JSON.stringify(revision.definition) || revision.status !== "draft"} title={revision.status !== "draft" ? "Créez une nouvelle révision pour modifier ce système" : "Enregistrer la topologie"} onClick={() => void save()}><Save size={15} />Enregistrer</button></div></div><TopologyEditor definition={draft} onChange={change} readOnly={revision.status !== "draft"} /></> : <EmptyHint title="Sélectionnez un système" detail="La topologie décrit les rôles, ports, chemins physiques, commandes, retours et points de correction." />}</section></div>;
}

function TopologyEditor(props: { definition: MeasurementSystemDefinition; onChange: (next: MeasurementSystemDefinition) => void; readOnly: boolean }) {
  const [layer, setLayer] = useState<TopologyLayer>("all");
  const [zoom, setZoom] = useState(1);
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [connectionOpen, setConnectionOpen] = useState(false);
  const [connectionKind, setConnectionKind] = useState("physical_signal");
  const [connectionFrom, setConnectionFrom] = useState("");
  const [connectionTo, setConnectionTo] = useState("");
  const d = props.definition;
  const visibleEdges = d.edges.filter((edge) => layer === "all" || edgeLayer(edge.edge_kind) === layer);
  const inspectedNode = d.nodes.find((node) => node.node_id === selectedNode);
  const outputPorts = d.nodes.flatMap((node) => node.ports.filter((port) => port.directionality !== "input").map((port) => ({ node, port })));
  const inputPorts = d.nodes.flatMap((node) => node.ports.filter((port) => port.directionality !== "output").map((port) => ({ node, port })));
  function addNode() { const number = d.nodes.length + 1; const node: TopologyNode = { node_id: `node-${number}`, label: `Fonction ${number}`, role_type: "generic", ports: [{ port_id: "input", label: "Entrée", directionality: "input", signal_domain: "rf" }, { port_id: "output", label: "Sortie", directionality: "output", signal_domain: "rf" }], notes: "" }; props.onChange({ ...d, nodes: [...d.nodes, node] }); setSelectedNode(node.node_id); }
  function updateNode(next: TopologyNode) { props.onChange({ ...d, nodes: d.nodes.map((node) => node.node_id === next.node_id ? next : node) }); }
  function beginConnection() { const from = outputPorts[0]; const to = inputPorts.find((candidate) => candidate.node.node_id !== from?.node.node_id) ?? inputPorts[0]; setConnectionFrom(from ? portKey(from.node.node_id, from.port.port_id) : ""); setConnectionTo(to ? portKey(to.node.node_id, to.port.port_id) : ""); setConnectionOpen(true); }
  function addConnection() { const from = parsePortKey(connectionFrom); const to = parsePortKey(connectionTo); if (!from || !to) return; const fromNode = d.nodes.find((node) => node.node_id === from.node_id); const toNode = d.nodes.find((node) => node.node_id === to.node_id); if (!fromNode || !toNode) return; const edge: TopologyEdge = { edge_id: id("edge"), label: `${fromNode.label} vers ${toNode.label}`, edge_kind: connectionKind, from, to }; props.onChange({ ...d, edges: [...d.edges, edge] }); setConnectionOpen(false); }
  function nodePosition(index: number) { return d.nodes.length <= 1 ? 50 : 11 + (index * 78) / (d.nodes.length - 1); }
  return <div className="topologyEditor"><div className="topologyControls"><button disabled={props.readOnly} onClick={addNode}><Plus size={15} />Fonction</button><button disabled={props.readOnly || !outputPorts.length || !inputPorts.length} title={!outputPorts.length || !inputPorts.length ? "Ajoutez des ports de sortie et d'entrée" : "Choisir les ports et la nature de la liaison"} onClick={beginConnection}><GitBranch size={15} />Connexion</button><div className="segmented compact" aria-label="Couches de la topologie">{(["physical", "regulation", "control", "all"] as const).map((value) => <button className={layer === value ? "active" : ""} key={value} onClick={() => setLayer(value)}>{value === "physical" ? "Signaux physiques" : value === "regulation" ? "Régulation" : value === "control" ? "Pilotage et données" : "Toutes"}</button>)}</div><button className="iconButton" title="Réduire" aria-label="Réduire" onClick={() => setZoom((value) => Math.max(.65, value - .1))}><ZoomOut size={15} /></button><button className="iconButton" title="Agrandir" aria-label="Agrandir" onClick={() => setZoom((value) => Math.min(1.4, value + .1))}><ZoomIn size={15} /></button><button className="iconButton" title="Ajuster" aria-label="Ajuster la topologie" onClick={() => setZoom(1)}><Focus size={15} /></button></div>{connectionOpen && <div className="connectionEditor" role="dialog" aria-label="Nouvelle connexion"><label>Nature de la liaison<select aria-label="Nature de la liaison" value={connectionKind} onChange={(event) => setConnectionKind(event.target.value)}>{Object.entries(edgeLabels).map(([value, text]) => <option key={value} value={value}>{text}</option>)}</select></label><label>Port source<select aria-label="Port source" value={connectionFrom} onChange={(event) => setConnectionFrom(event.target.value)}>{outputPorts.map(({ node, port }) => <option key={portKey(node.node_id, port.port_id)} value={portKey(node.node_id, port.port_id)}>{node.label} · {port.label} · {signalDomainLabels[port.signal_domain] ?? port.signal_domain}</option>)}</select></label><ArrowRight size={16} /><label>Port destination<select aria-label="Port destination" value={connectionTo} onChange={(event) => setConnectionTo(event.target.value)}>{inputPorts.map(({ node, port }) => <option key={portKey(node.node_id, port.port_id)} value={portKey(node.node_id, port.port_id)}>{node.label} · {port.label} · {signalDomainLabels[port.signal_domain] ?? port.signal_domain}</option>)}</select></label><div><button onClick={() => setConnectionOpen(false)}>Annuler</button><button className="primaryButton" disabled={!connectionFrom || !connectionTo || connectionFrom.split("::")[0] === connectionTo.split("::")[0]} title={connectionFrom.split("::")[0] === connectionTo.split("::")[0] ? "La source et la destination doivent être deux fonctions distinctes" : "La compatibilité finale sera contrôlée par l'agent Rust"} onClick={addConnection}>Ajouter</button></div></div>}<div className="topologyCanvas" aria-label="Vue visuelle de la topologie"><svg className="topologyEdges" viewBox="0 0 100 260" preserveAspectRatio="none" aria-hidden="true">{visibleEdges.map((edge) => { const fromIndex = d.nodes.findIndex((node) => node.node_id === edge.from.node_id); const toIndex = d.nodes.findIndex((node) => node.node_id === edge.to.node_id); const fromX = nodePosition(fromIndex); const toX = nodePosition(toIndex); return <path key={edge.edge_id} className={`topologyEdge ${edgeLayer(edge.edge_kind)}`} vectorEffect="non-scaling-stroke" d={`M ${fromX} 118 C ${fromX + 4} 80, ${toX - 4} 80, ${toX} 118`}><title>{edgeLabels[edge.edge_kind] ?? edge.edge_kind}: {edge.label}</title></path>; })}</svg><div className="topologyNodes" style={{ transform: `scale(${zoom})` }}>{d.nodes.map((node) => <button key={node.node_id} className={selectedNode === node.node_id ? "selected" : ""} onClick={() => setSelectedNode(node.node_id)} onKeyDown={(event) => { if (event.key === "ArrowRight" || event.key === "ArrowLeft") { const index = d.nodes.findIndex((item) => item.node_id === node.node_id); const delta = event.key === "ArrowRight" ? 1 : -1; (event.currentTarget.parentElement?.children[Math.max(0, Math.min(d.nodes.length - 1, index + delta))] as HTMLElement | undefined)?.focus(); } }}><InstrumentIcon roleType={node.role_type} label={node.label} size={26} /><span><strong>{node.label}</strong><small>{roleLabels[node.role_type] ?? node.role_type}</small></span></button>)}</div></div>{inspectedNode && <NodeInspector node={inspectedNode} readOnly={props.readOnly} onChange={updateNode} onRemove={() => { props.onChange({ ...d, nodes: d.nodes.filter((node) => node.node_id !== inspectedNode.node_id), edges: d.edges.filter((edge) => edge.from.node_id !== inspectedNode.node_id && edge.to.node_id !== inspectedNode.node_id), correction_points: d.correction_points.filter((point) => point.node_id !== inspectedNode.node_id), regulation_loops: d.regulation_loops.filter((loop) => loop.actuator_node_id !== inspectedNode.node_id && loop.feedback_node_id !== inspectedNode.node_id && !loop.monitoring_node_ids.includes(inspectedNode.node_id)) }); setSelectedNode(null); }} />}<div className="topologyFallback"><div className="sectionCommand"><div><h4>Liste structurée équivalente</h4><p>Cette table expose les mêmes connexions que la vue graphique.</p></div></div>{visibleEdges.length ? <div className="structuredTable" role="table" aria-label="Connexions logiques">{visibleEdges.map((edge) => <div className="structuredRow topologyConnection" role="row" key={edge.edge_id}><span><strong>{edgeLabels[edge.edge_kind] ?? edge.edge_kind}</strong><small>{edge.label}</small></span><span>{nodeLabel(d.nodes, edge.from.node_id)} · {edge.from.port_id}</span><ArrowRight size={15} /><span>{nodeLabel(d.nodes, edge.to.node_id)} · {edge.to.port_id}</span><button className="iconButton" aria-label={`Supprimer ${edge.label}`} disabled={props.readOnly} onClick={() => props.onChange({ ...d, edges: d.edges.filter((item) => item.edge_id !== edge.edge_id) })}><Trash2 size={14} /></button></div>)}</div> : <EmptyHint title="Aucune connexion" detail="Reliez les ports pour rendre explicites les chemins de signal, de commande et de retour." />}</div></div>;
}

function NodeInspector(props: { node: TopologyNode; readOnly: boolean; onChange: (node: TopologyNode) => void; onRemove: () => void }) {
  const updatePort = (portId: string, patch: Partial<TopologyNode["ports"][number]>) => props.onChange({ ...props.node, ports: props.node.ports.map((port) => port.port_id === portId ? { ...port, ...patch } : port) });
  return <section className="nodeInspector" aria-label={`Fonction ${props.node.label}`}><div className="sectionCommand"><div><h4>Fonction sélectionnée</h4><p>Décrivez son rôle logique et les interfaces disponibles. Aucun matériel réel n'est affecté ici.</p></div><button className="iconButton" aria-label={`Supprimer la fonction ${props.node.label}`} disabled={props.readOnly} onClick={props.onRemove}><Trash2 size={15} /></button></div><div className="nodeFields"><Field label="Nom de la fonction" value={props.node.label} disabled={props.readOnly} required onChange={(label) => props.onChange({ ...props.node, label })} /><label className="workflowField"><span>Rôle logique</span><select disabled={props.readOnly} value={props.node.role_type} onChange={(event) => props.onChange({ ...props.node, role_type: event.target.value })}>{Object.entries(roleLabels).map(([value, text]) => <option key={value} value={value}>{text}</option>)}</select></label></div><div className="portList"><div className="sectionCommand"><div><h4>Ports logiques</h4><p>Les liaisons utilisent ces ports nommés et leur domaine de signal.</p></div><button disabled={props.readOnly} onClick={() => props.onChange({ ...props.node, ports: [...props.node.ports, { port_id: id("port"), label: "Nouveau port", directionality: "input", signal_domain: "rf" }] })}><Plus size={14} />Port</button></div>{props.node.ports.map((port) => <div className="portRow" key={port.port_id}><input aria-label={`Nom du port ${port.port_id}`} disabled={props.readOnly} value={port.label} onChange={(event) => updatePort(port.port_id, { label: event.target.value })} /><select aria-label={`Direction du port ${port.label}`} disabled={props.readOnly} value={port.directionality} onChange={(event) => updatePort(port.port_id, { directionality: event.target.value })}>{Object.entries(directionLabels).map(([value, text]) => <option key={value} value={value}>{text}</option>)}</select><select aria-label={`Domaine du port ${port.label}`} disabled={props.readOnly} value={port.signal_domain} onChange={(event) => updatePort(port.port_id, { signal_domain: event.target.value })}>{Object.entries(signalDomainLabels).map(([value, text]) => <option key={value} value={value}>{text}</option>)}</select><button className="iconButton" aria-label={`Supprimer le port ${port.label}`} disabled={props.readOnly} onClick={() => props.onChange({ ...props.node, ports: props.node.ports.filter((item) => item.port_id !== port.port_id) })}><Trash2 size={14} /></button></div>)}</div></section>;
}

function RegulationWorkspace(props: { profiles: WorkflowAggregate<RegulationProfileDefinition>[]; methods: MethodWorkflowAggregate[]; selectedId: string | null; error?: string; onSelect: (id: string | null) => void; onChanged: (notice: string) => Promise<void> }) {
  const selected = props.profiles.find((profile) => profile.identity.entity_id === props.selectedId);
  const revision = latestRevision(selected);
  const [draft, setDraft] = useState<RegulationProfileDefinition | null>(revision?.definition ?? null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => setDraft(latestRevision(selected)?.definition ?? null), [selected]);
  async function create() { const entityId = id("REG").toUpperCase(); const method = firstMethodV2(props.methods); if (!method) { setError("Créez d'abord une méthode 0.22.2 avec ses variables et ses fonctions."); return; } await methodWorkflowApi.createWorkflowDefinition("regulation-profiles", entityId, "Approche progressive", "laboratory", defaultRegulation(entityId, method), { method_definition: method }, context); await props.onChanged("Profil de régulation créé en brouillon."); props.onSelect(entityId); }
  async function save() { if (!selected || !revision || !draft) return; const method = firstMethodV2(props.methods); if (!method) return; try { await methodWorkflowApi.saveWorkflowDraft("regulation-profiles", selected.identity.entity_id, revision.revision_id, revision.definition_checksum, draft, { method_definition: method }, context); await props.onChanged("Profil enregistré et validé par l'agent."); } catch (cause) { setError(message(cause)); } }
  async function transition(target: "validate" | "approve") { if (!selected || !revision) return; const method = firstMethodV2(props.methods); if (!method) return; try { await methodWorkflowApi.transitionWorkflow("regulation-profiles", selected.identity.entity_id, revision.revision_id, target, { method_definition: method }, context); await props.onChanged(target === "approve" ? "Profil de régulation approuvé." : "Profil de régulation validé."); } catch (cause) { setError(message(cause)); } }
  if (props.error) return <TargetedError title="Profils de régulation indisponibles" detail={props.error} />;
  return <div className="regulationWorkspace"><aside className="systemList"><div className="panelHeading"><div><strong>Profils de régulation</strong><small>Stratégies versionnées du laboratoire</small></div><button className="iconButton" aria-label="Créer un profil" onClick={() => void create()}><Plus size={16} /></button></div>{props.profiles.map((profile) => <button key={profile.identity.entity_id} className={props.selectedId === profile.identity.entity_id ? "selected" : ""} onClick={() => props.onSelect(profile.identity.entity_id)}><SlidersHorizontal size={18} /><span><strong>{profile.identity.label}</strong><small>{controlModeLabel(latestRevision(profile)?.definition.control_mode)}</small></span></button>)}{!props.profiles.length && <EmptyHint title="Aucun profil" detail="Un profil formalise l'approche de consigne, la tolérance, les temporisations et l'arrêt sûr." action="Créer un profil" onAction={() => void create()} />}</aside><section className="regulationEditor">{error && <TargetedError title="Régulation incomplète" detail={error} />}{draft && revision ? <><div className="topologyToolbar"><div><p className="eyebrow">Profil · révision {revision.revision_number}</p><input className="titleInput" value={draft.label} aria-label="Nom du profil" onChange={(event) => setDraft({ ...draft, label: event.target.value })} /></div><div>{revision.status === "draft" && <button disabled={JSON.stringify(draft) !== JSON.stringify(revision.definition)} title={JSON.stringify(draft) !== JSON.stringify(revision.definition) ? "Enregistrez avant validation" : "Valider le profil"} onClick={() => void transition("validate")}><Check size={15} />Valider</button>}{revision.status === "validated" && <button onClick={() => void transition("approve")}><BookOpenCheck size={15} />Approuver</button>}<button className="primaryButton" disabled={revision.status !== "draft" || JSON.stringify(draft) === JSON.stringify(revision.definition)} onClick={() => void save()}><Save size={15} />Enregistrer</button></div></div><RegulationPreview profile={draft} /><div className="regulationTable"><NumberField label="Seuil de départ" value={draft.start_threshold.value} unit={draft.start_threshold.unit} onChange={(value) => setDraft({ ...draft, start_threshold: { ...draft.start_threshold, value } })} /><NumberField label="Pas rapide" value={draft.fast_increasing_step.value} unit={draft.fast_increasing_step.unit} onChange={(value) => setDraft({ ...draft, fast_increasing_step: { ...draft.fast_increasing_step, value } })} /><NumberField label="Pas lent" value={draft.slow_increasing_step.value} unit={draft.slow_increasing_step.unit} onChange={(value) => setDraft({ ...draft, slow_increasing_step: { ...draft.slow_increasing_step, value } })} /><NumberField label="Pas de réduction" value={draft.decreasing_step.value} unit={draft.decreasing_step.unit} onChange={(value) => setDraft({ ...draft, decreasing_step: { ...draft.decreasing_step, value } })} /><NumberField label="Attente T1" value={draft.dwell_t1_seconds} unit="s" onChange={(value) => setDraft({ ...draft, dwell_t1_seconds: value })} /><NumberField label="Attente T2" value={draft.dwell_t2_seconds} unit="s" onChange={(value) => setDraft({ ...draft, dwell_t2_seconds: value })} /><NumberField label="Attente T3" value={draft.dwell_t3_seconds} unit="s" onChange={(value) => setDraft({ ...draft, dwell_t3_seconds: value })} /></div><div className="regulationSummary"><strong>Résumé opérateur</strong><p>Partir à {draft.start_threshold.value} {draft.start_threshold.unit}, augmenter rapidement par pas de {draft.fast_increasing_step.value} {draft.fast_increasing_step.unit}, puis approcher la consigne par pas de {draft.slow_increasing_step.value} {draft.slow_increasing_step.unit}. Stabiliser dans une tolérance de ±{draft.tolerance_band.value} {draft.tolerance_band.unit} et réduire de {draft.decreasing_step.value} {draft.decreasing_step.unit} en cas de dépassement.</p></div><div className="validationSummary"><Check size={17} /><span><strong>Validation autoritaire à l'enregistrement</strong><small>Les rôles d'action, de retour, les unités et le chemin de boucle seront contrôlés par l'agent Rust.</small></span></div></> : <EmptyHint title="Sélectionnez un profil" detail="Les trois vues, graphique, table et résumé, restent synchronisées." />}</section></div>;
}

function RegulationPreview({ profile }: { profile: RegulationProfileDefinition }) {
  return <figure className="regulationPreview"><svg viewBox="0 0 760 230" role="img" aria-label="Approche progressive de la consigne"><title>Approche progressive de la consigne</title><line x1="55" y1="185" x2="730" y2="185"/><line x1="55" y1="185" x2="55" y2="25"/><rect x="500" y="54" width="205" height="34" className="toleranceBand"/><line x1="55" y1="71" x2="730" y2="71" className="targetLine"/><path d="M75 165 H145 V140 H215 V116 H285 V100 H355 V90 H425 V80 H500 V71 H580 V71 H640 V95 H690 V71" className="regulationPath"/><text x="75" y="211">Départ</text><text x="190" y="211">Approche rapide</text><text x="370" y="211">Approche lente</text><text x="535" y="211">Régulation</text><text x="650" y="211">Reprise</text><text x="590" y="48">Tolérance ±{profile.tolerance_band.value} {profile.tolerance_band.unit}</text></svg></figure>;
}

function ValidationPanel({ result }: { result: { valid: boolean; issues: Array<{ code: string; path: string; message: string }> } }) { return <div className={`validationPanel ${result.valid ? "valid" : "invalid"}`}><div>{result.valid ? <Check size={18} /> : <AlertCircle size={18} />}<strong>{result.valid ? "Définition cohérente" : `${result.issues.length} point(s) à corriger`}</strong></div>{result.issues.map((issue) => <p key={`${issue.code}-${issue.path}`}><code>{issue.path}</code>{issue.message}</p>)}</div>; }
function SimpleCollection(props: { title: string; count: number; empty: string }) { return <div><div className="sectionCommand"><div><h4>{props.title}</h4><p>{props.count ? `${props.count} élément(s) structuré(s).` : props.empty}</p></div></div>{!props.count && <EmptyHint title="Rien n'est encore défini" detail={props.empty} />}</div>; }
function TargetedError(props: { title: string; detail: string }) { return <div className="targetedError" role="alert"><AlertCircle size={18} /><span><strong>{props.title}</strong><small>{props.detail}</small></span></div>; }
function EmptyHint(props: { title: string; detail: string; action?: string; onAction?: () => void }) { return <div className="workflowEmpty"><CircleHelp size={22} /><span><strong>{props.title}</strong><small>{props.detail}</small></span>{props.action && props.onAction && <button onClick={props.onAction}>{props.action}</button>}</div>; }
function Field(props: { label: string; value: string; onChange: (value: string) => void; required?: boolean; help?: string; disabled?: boolean }) { return <label className="workflowField"><span>{props.label}{props.required && <b aria-label="obligatoire"> *</b>}</span><input value={props.value} disabled={props.disabled} required={props.required} onChange={(event) => props.onChange(event.target.value)} />{props.help && <small>{props.help}</small>}</label>; }
function LongField(props: { label: string; value: string; onChange: (value: string) => void; required?: boolean; disabled?: boolean }) { return <label className="workflowField full"><span>{props.label}{props.required && <b> *</b>}</span><textarea rows={5} value={props.value} disabled={props.disabled} required={props.required} onChange={(event) => props.onChange(event.target.value)} /></label>; }
function NumberField(props: { label: string; value: number; unit: string; onChange: (value: number) => void }) { return <label><span>{props.label}</span><span className="numberWithUnit"><input type="number" value={props.value} onChange={(event) => props.onChange(Number(event.target.value))} /><small>{props.unit}</small></span></label>; }

function hierarchyBreadcrumbs(nodes: MethodHierarchyNode[], selected: MethodHierarchyNode) { const result = [selected]; let parent = selected.parent_node_id; while (parent) { const node = nodes.find((item) => item.node_id === parent); if (!node) break; result.unshift(node); parent = node.parent_node_id; } return result; }
function methodStepCompletion(d: TestMethodDefinitionV2) { return [Boolean(d.title && d.classification_path.length), Boolean(d.objective && d.scope), d.variables.length > 0, d.functional_roles.length > 0, d.measurement_system_templates.length > 0, d.procedure.length > 0 && d.sub_ranges.length > 0, d.regulation_profiles.length > 0, d.limits.length > 0, d.post_processing.length > 0 && d.expected_output_variables.length > 0, false]; }
function edgeLayer(kind: string): TopologyLayer { if (["physical_signal", "excitation_or_power", "monitoring", "eut_state"].includes(kind)) return "physical"; if (kind === "feedback_measurement") return "regulation"; return "control"; }
function nodeLabel(nodes: TopologyNode[], idValue: string) { return nodes.find((node) => node.node_id === idValue)?.label ?? idValue; }
function portKey(nodeId: string, portId: string) { return `${nodeId}::${portId}`; }
function parsePortKey(value: string) { const separator = value.indexOf("::"); return separator < 1 ? null : { node_id: value.slice(0, separator), port_id: value.slice(separator + 2) }; }
function controlModeLabel(value?: string) { return value === "closed_loop" ? "Boucle fermée" : value === "monitor_only" ? "Surveillance seule" : "Boucle ouverte"; }
function firstMethodV2(methods: MethodWorkflowAggregate[]) { for (const method of methods) { const revision = selectedMethodRevision(method); if (isMethodV2(revision?.definition)) return revision.definition; } return undefined; }

function defaultMethod(title: string, classification?: string): TestMethodDefinitionV2 {
  return { definition_schema_version: "emc-locus.test-method-definition.v2", title, objective: "Définir l'objectif mesurable de l'essai.", scope: "Définir le périmètre, les exclusions et les conditions applicables.", classification_path: [classification ?? "unclassified"], standard_references: [], variables: [{ variable_id: "measured_level", label: "Niveau mesuré", description: "Grandeur observée utilisée pour le résultat.", semantic: "observed_signal", value_type: "number", dimension: "ratio", unit: "dB", required: true, source: "measurement_receiver", availability_phase: "execution", consumers: ["result"] }], lock_policy: [], parameter_profiles: [], functional_roles: [{ role_id: "receiver", label: "Récepteur de mesure", purpose: "Mesurer sélectivement le signal utile dans la bande définie.", required: true, functional_category: "frequency_selective_measurement_instruments", capabilities: [{ capability_kind: "frequency_selective_measurement" }], calibration_policy: "required", substitution_policy: "same_capabilities", assignment_stage: "planned_test_preparation", logical_ports: [{ port_id: "rf_input", label: "Entrée RF", directionality: "input", signal_domain: "rf", impedance_ohm: 50, quantity_dimension: "voltage" }], consumes_variables: [], produces_variables: ["measured_level"] }], measurement_system_templates: [], regulation_profiles: [], modulation_profiles: [], sub_ranges: [defaultSubRange()], procedure: [{ node_id: "phase-measurement", label: "Mesure", purpose: "Parcourir la sous-plage et acquérir le niveau.", node_kind: "phase", input_variables: [], output_variables: ["measured_level"], failure_policy: "safe_shutdown", children: [{ node_id: "sweep-main", label: "Balayage principal", purpose: "Appliquer la progression fréquentielle bornée.", node_kind: "sweep", input_variables: [], output_variables: ["measured_level"], failure_policy: "stop", maximum_iterations: 10000, children: [], audit_notes: "" }], audit_notes: "" }], limits: [], post_processing: [], expected_output_variables: ["measured_level"], migration_evidence: [] };
}
function defaultSubRange(): SubRangeDefinition { return { sub_range_id: "main-range", label: "Plage principale", start_frequency: { value: .15, unit: "MHz" }, stop_frequency: { value: 30, unit: "MHz" }, include_start: true, include_stop: true, progression: { kind: "percentage", percentage: 1 }, direction: "increasing", sweep_mode: "stepped", dwell_seconds: .05, include_frequencies: [], exclude_frequencies: [], comments: "" }; }
function defaultSystem(templateId: string): MeasurementSystemDefinition { return { definition_schema_version: "emc-locus.measurement-system-template-definition.v1", template_id: templateId, label: "Nouveau système de mesure", classification: "laboratory", nodes: [], edges: [], correction_points: [], regulation_loops: [], notes: "Topologie logique sans matériel daté." }; }
function defaultRegulation(profileId: string, method: TestMethodDefinitionV2): RegulationProfileDefinition { const role = method.functional_roles[0]?.role_id; const variable = method.variables[0]?.variable_id; return { definition_schema_version: "emc-locus.regulation-profile-definition.v1", profile_id: profileId, label: "Approche progressive", regulated_quantity: "Niveau appliqué", regulated_unit: "dB", target_expression: { kind: "literal", value: 10, unit: "dB" }, tolerance_band: { value: 1, unit: "dB" }, control_mode: "open_loop", actuator_role_id: role, required_driver_action: "set_level", feedback_role_id: undefined, feedback_signal_variable_id: variable, monitoring_role_ids: [], start_threshold: { value: -12, unit: "dB" }, fast_increasing_step: { value: 3, unit: "dB" }, slow_increasing_step: { value: 1, unit: "dB" }, decreasing_step: { value: 1, unit: "dB" }, dwell_t1_seconds: 1, dwell_t2_seconds: 2, dwell_t3_seconds: 1, regulation_start_criterion: "Approcher la consigne", regulation_end_criterion: "Tolérance atteinte", regulation_factor: 1, retry_policy: { maximum_attempts: 3, on_exhaustion: "operator_decision" }, abort_policy: "safe_shutdown", safe_state_policy: "set_output_off", operator_notes: "Profil de laboratoire à revoir avant approbation." }; }
