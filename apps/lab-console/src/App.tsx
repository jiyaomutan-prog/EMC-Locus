import {
  AlertTriangle,
  BookOpenText,
  CheckCircle2,
  ClipboardCheck,
  Copy,
  Cpu,
  Database,
  GitBranch,
  History,
  PanelLeftClose,
  PanelLeftOpen,
  RefreshCw,
  Save,
  Search,
  Send,
  ShieldCheck
} from "lucide-react";
import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import { ApiError, api, type OperationContext } from "./api";
import { defaultTemplateDefinition } from "./defaultDefinition";
import { EquipmentWorkspace } from "./features/equipment/EquipmentWorkspace";
import { APP_VERSION } from "./version";
import type {
  AuditEvent,
  HealthReport,
  InstrumentationChainSlot,
  LimitDefinition,
  PostProcessingDefinition,
  RevisionStatus,
  SaveState,
  StorageStatus,
  TestTemplateAggregate,
  TestTemplateDefinition,
  TestTemplateRevision,
  ValidationResult,
  VariableDefinition,
  VariableLockPolicy
} from "./types";

type ActiveView = "library" | "studio" | "equipment" | "system";
type StudioSection =
  | "general"
  | "variables"
  | "locks"
  | "instrumentation"
  | "sequence"
  | "limits"
  | "post"
  | "revisions"
  | "audit"
  | "advanced";

const upcomingModules = ["Métrologie", "Planification", "Campagnes", "Rapports"];

const statusLabels: Record<RevisionStatus, string> = {
  draft: "Brouillon",
  under_review: "En revue",
  approved: "Approuve",
  superseded: "Supersede",
  suspended: "Suspendu",
  retired: "Retire"
};

const statusOrder: RevisionStatus[] = [
  "draft",
  "under_review",
  "approved",
  "superseded",
  "suspended",
  "retired"
];

const sectionLabels: Array<[StudioSection, string]> = [
  ["general", "General"],
  ["variables", "Variables"],
  ["locks", "Politiques"],
  ["instrumentation", "Instrumentation"],
  ["sequence", "Sequence"],
  ["limits", "Limites"],
  ["post", "Post-traitement"],
  ["revisions", "Revisions"],
  ["audit", "Audit"],
  ["advanced", "JSON"]
];

const emptyContext: OperationContext = {
  actor: "method.author",
  reason: "operation LAB CONSOLE"
};

interface CreateFormState {
  template_id: string;
  title: string;
  category_code: string;
  actor: string;
  reason: string;
}

interface CloneFormState {
  source_template_id: string;
  source_revision_id: string;
  new_template_id: string;
  title: string;
  category_code: string;
  actor: string;
  reason: string;
}

export function App() {
  const [activeView, setActiveView] = useState<ActiveView>("library");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [templates, setTemplates] = useState<TestTemplateAggregate[]>([]);
  const [health, setHealth] = useState<HealthReport | null>(null);
  const [storage, setStorage] = useState<StorageStatus | null>(null);
  const [loadState, setLoadState] = useState<"loading" | "ready" | "empty" | "error">("loading");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [categoryFilter, setCategoryFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState("all");
  const [sort, setSort] = useState("updated_desc");
  const [creationMode, setCreationMode] = useState<"none" | "create" | "clone">("none");
  const [createForm, setCreateForm] = useState({
    template_id: "",
    title: "",
    category_code: "emission_transient_time_domain",
    actor: "method.author",
    reason: "creation template LAB"
  });
  const [cloneForm, setCloneForm] = useState({
    source_template_id: "",
    source_revision_id: "",
    new_template_id: "",
    title: "",
    category_code: "",
    actor: "method.author",
    reason: "clonage depuis revision approuvee"
  });
  const [selectedTemplate, setSelectedTemplate] = useState<TestTemplateAggregate | null>(null);
  const [selectedRevision, setSelectedRevision] = useState<TestTemplateRevision | null>(null);
  const [definition, setDefinition] = useState<TestTemplateDefinition | null>(null);
  const [originalDefinitionJson, setOriginalDefinitionJson] = useState("");
  const [expectedChecksum, setExpectedChecksum] = useState("");
  const [saveState, setSaveState] = useState<SaveState>("clean");
  const [saveError, setSaveError] = useState<string | null>(null);
  const [conflict, setConflict] = useState<{ expected?: string; actual?: string } | null>(null);
  const [validation, setValidation] = useState<ValidationResult | null>(null);
  const [section, setSection] = useState<StudioSection>("general");
  const [revisions, setRevisions] = useState<TestTemplateRevision[]>([]);
  const [audit, setAudit] = useState<AuditEvent[]>([]);
  const [context, setContext] = useState<OperationContext>(emptyContext);
  const [operationError, setOperationError] = useState<string | null>(null);

  const dirty = definition !== null && stableStringify(definition) !== originalDefinitionJson;
  const readOnly = selectedRevision?.status !== "draft";

  useEffect(() => {
    void refreshAll();
  }, []);

  useEffect(() => {
    const page = activeView === "equipment"
      ? "Équipements"
      : activeView === "system"
        ? "Système local"
        : activeView === "studio"
          ? "Éditeur de méthode"
          : "Méthodes d'essai";
    document.title = `${page} | EMC Locus`;
  }, [activeView]);

  useEffect(() => {
    setSaveState((current) => {
      if (current === "saving" || current === "conflict" || current === "error") {
        return current;
      }
      return dirty ? "dirty" : "clean";
    });
  }, [dirty]);

  useEffect(() => {
    const listener = (event: BeforeUnloadEvent) => {
      if (dirty) {
        event.preventDefault();
        event.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", listener);
    return () => window.removeEventListener("beforeunload", listener);
  }, [dirty]);

  const categories = useMemo(
    () => Array.from(new Set(templates.map((template) => template.identity.category_code))).sort(),
    [templates]
  );

  const filteredTemplates = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return templates
      .filter((template) => {
        const latestStatus = template.latest_revision?.status ?? "draft";
        const text = `${template.identity.template_id} ${template.identity.title} ${template.identity.category_code}`.toLowerCase();
        return (
          (!normalizedQuery || text.includes(normalizedQuery)) &&
          (categoryFilter === "all" || template.identity.category_code === categoryFilter) &&
          (statusFilter === "all" || latestStatus === statusFilter)
        );
      })
      .sort((left, right) => compareTemplates(left, right, sort));
  }, [templates, query, categoryFilter, statusFilter, sort]);

  async function refreshAll() {
    setLoadState("loading");
    setLoadError(null);
    try {
      const [healthReport, storageReport, list] = await Promise.all([
        api.health(),
        api.storageStatus(),
        api.listTemplates()
      ]);
      setHealth(healthReport);
      setStorage(storageReport);
      setTemplates(list.test_templates);
      setLoadState(list.test_templates.length === 0 ? "empty" : "ready");
    } catch (error) {
      setLoadState("error");
      setLoadError(errorMessage(error));
    }
  }

  async function openRevision(template: TestTemplateAggregate, revision: TestTemplateRevision | null) {
    const target = revision ?? template.active_draft_revision ?? template.current_approved_revision ?? template.latest_revision;
    if (!target) {
      return;
    }
    const [templateDetail, revisionDetail, revisionList, auditList] = await Promise.all([
      api.getTemplate(template.identity.template_id),
      api.getRevision(template.identity.template_id, target.revision_id),
      api.listRevisions(template.identity.template_id),
      api.listAudit(template.identity.template_id)
    ]);
    setSelectedTemplate(templateDetail.test_template);
    setSelectedRevision(revisionDetail.revision);
    setDefinition(revisionDetail.revision.definition);
    setExpectedChecksum(revisionDetail.revision.definition_checksum);
    setOriginalDefinitionJson(stableStringify(revisionDetail.revision.definition));
    setSaveState("clean");
    setSaveError(null);
    setConflict(null);
    setValidation(null);
    setRevisions(revisionList.revisions);
    setAudit(auditList.audit_events);
    setSection("general");
    setActiveView("studio");
  }

  async function reloadSelectedTemplate() {
    if (!selectedTemplate || !selectedRevision) {
      return;
    }
    const detail = await api.getTemplate(selectedTemplate.identity.template_id);
    const currentRevision = await api.getRevision(
      selectedTemplate.identity.template_id,
      selectedRevision.revision_id
    );
    const revisionList = await api.listRevisions(selectedTemplate.identity.template_id);
    const auditList = await api.listAudit(selectedTemplate.identity.template_id);
    setSelectedTemplate(detail.test_template);
    setSelectedRevision(currentRevision.revision);
    setDefinition(currentRevision.revision.definition);
    setExpectedChecksum(currentRevision.revision.definition_checksum);
    setOriginalDefinitionJson(stableStringify(currentRevision.revision.definition));
    setRevisions(revisionList.revisions);
    setAudit(auditList.audit_events);
    setConflict(null);
    setSaveState("clean");
  }

  async function validateCurrentDefinition() {
    if (!definition) {
      return;
    }
    setOperationError(null);
    try {
      const result = await api.validateDefinition(definition);
      setValidation(result);
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  }

  async function saveDraft() {
    if (!selectedTemplate || !selectedRevision || !definition || readOnly) {
      return;
    }
    setSaveState("saving");
    setSaveError(null);
    try {
      const result = await api.saveDraft(
        selectedTemplate.identity.template_id,
        selectedRevision.revision_id,
        expectedChecksum,
        definition,
        context
      );
      setSelectedTemplate(result.test_template);
      setSelectedRevision(result.revision);
      setDefinition(result.revision.definition);
      setExpectedChecksum(result.revision.definition_checksum);
      setOriginalDefinitionJson(stableStringify(result.revision.definition));
      setSaveState("saved");
      await refreshAll();
    } catch (error) {
      if (error instanceof ApiError && error.code === "test_template_definition_checksum_mismatch") {
        setConflict({
          expected: String(error.details?.expected_definition_checksum ?? expectedChecksum),
          actual: String(error.details?.actual_definition_checksum ?? "")
        });
        setSaveState("conflict");
        return;
      }
      setSaveState("error");
      setSaveError(errorMessage(error));
    }
  }

  async function submitForReview() {
    if (!selectedTemplate || !selectedRevision || !validation?.valid || dirty || readOnly) {
      return;
    }
    setOperationError(null);
    try {
      const result = await api.submitRevision(
        selectedTemplate.identity.template_id,
        selectedRevision.revision_id,
        context
      );
      await afterTransition(result.revision);
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  }

  async function approveRevision() {
    if (!selectedTemplate || !selectedRevision || selectedRevision.status !== "under_review") {
      return;
    }
    setOperationError(null);
    try {
      const result = await api.approveRevision(
        selectedTemplate.identity.template_id,
        selectedRevision.revision_id,
        context
      );
      await afterTransition(result.revision);
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  }

  async function deriveNewRevision() {
    if (!selectedTemplate?.current_approved_revision) {
      return;
    }
    setOperationError(null);
    try {
      const result = await api.deriveRevision(
        selectedTemplate.identity.template_id,
        selectedTemplate.current_approved_revision.revision_id,
        context
      );
      await openRevision(result.test_template, result.revision);
      await refreshAll();
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  }

  async function afterTransition(revision: TestTemplateRevision) {
    if (!selectedTemplate) {
      return;
    }
    const detail = await api.getTemplate(selectedTemplate.identity.template_id);
    await openRevision(detail.test_template, revision);
    await refreshAll();
  }

  async function createTemplate() {
    setOperationError(null);
    try {
      const definitionForCreate = defaultTemplateDefinition(createForm.title || createForm.template_id);
      const result = await api.createTemplate({
        template_id: createForm.template_id,
        title: createForm.title,
        category_code: createForm.category_code,
        definition: definitionForCreate,
        actor: createForm.actor,
        reason: createForm.reason
      });
      setCreationMode("none");
      await refreshAll();
      await openRevision(result.test_template, result.revision);
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  }

  async function cloneTemplate() {
    setOperationError(null);
    try {
      const result = await api.cloneTemplate(cloneForm.source_template_id, {
        source_revision_id: cloneForm.source_revision_id,
        new_template_id: cloneForm.new_template_id,
        title: cloneForm.title,
        category_code: cloneForm.category_code || undefined,
        actor: cloneForm.actor,
        reason: cloneForm.reason
      });
      setCreationMode("none");
      await refreshAll();
      await openRevision(result.test_template, result.revision);
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  }

  function updateDefinition(next: TestTemplateDefinition) {
    setDefinition(next);
    setValidation(null);
    setSaveState("dirty");
  }

  return (
    <div className={`shell${sidebarCollapsed ? " sidebarCollapsed" : ""}`}>
      <aside className="sidebar" aria-label="Navigation de l'application">
        <div className="brand">
          <span className="brandMark" aria-hidden="true">EL</span>
          <span className="brandCopy">
            <strong>EMC Locus</strong>
            <small>LAB CONSOLE {APP_VERSION}</small>
          </span>
          <button
            className="sidebarToggle"
            type="button"
            onClick={() => setSidebarCollapsed((collapsed) => !collapsed)}
            aria-label={sidebarCollapsed ? "Déployer la navigation" : "Réduire la navigation"}
            title={sidebarCollapsed ? "Déployer la navigation" : "Réduire la navigation"}
          >
            {sidebarCollapsed ? <PanelLeftOpen size={17} /> : <PanelLeftClose size={17} />}
          </button>
        </div>
        <nav className="primaryNav" aria-label="Espaces de travail">
          <p className="navLabel">Conception laboratoire</p>
          <button
            className={activeView === "library" || activeView === "studio" ? "active" : ""}
            onClick={() => setActiveView("library")}
            title="Méthodes d'essai"
          >
            <BookOpenText size={18} />
            <span>Méthodes d'essai</span>
          </button>
          <button
            className={activeView === "equipment" ? "active" : ""}
            onClick={() => setActiveView("equipment")}
            title="Équipements"
          >
            <Cpu size={18} />
            <span>Équipements</span>
          </button>
        </nav>

        <div className="upcomingNav" aria-label="Prochaines verticales">
          <p className="navLabel">Prochaines verticales</p>
          <ul>
            {upcomingModules.map((module) => (
              <li key={module}><span>{module}</span><small>À venir</small></li>
            ))}
          </ul>
        </div>

        <nav className="systemNav" aria-label="État de l'application">
          <button
            className={activeView === "system" ? "active" : ""}
            onClick={() => setActiveView("system")}
            title="Système local"
          >
            <Database size={18} />
            <span>Système local</span>
          </button>
        </nav>
      </aside>

      <main className="workspace">
        <header className="topbar">
          <div>
            <p className="eyebrow">
              {activeView === "equipment"
                ? "Référentiel, signaux et corrections"
                : activeView === "system"
                  ? "Diagnostic local"
                  : "Définitions et révisions"}
            </p>
            <h1>
              {activeView === "studio"
                ? "Éditeur de méthode"
                : activeView === "equipment"
                  ? "Équipements"
                  : activeView === "system"
                    ? "Système local"
                    : "Méthodes d'essai"}
            </h1>
          </div>
          <div className="connection">
            <span className="connectionStatus">
              <span className={health ? "dot ok" : "dot warn"} />
              <span>{health ? "Agent local" : "Agent indisponible"}</span>
              {health && <strong>{health.version}</strong>}
            </span>
            <button
              className="iconButton"
              onClick={() => void refreshAll()}
              title="Rafraîchir l'état local"
              aria-label="Rafraîchir l'état local"
            >
              <RefreshCw size={16} />
            </button>
          </div>
        </header>

        {activeView === "system" && <SystemView health={health} storage={storage} />}
        {activeView === "equipment" && <EquipmentWorkspace />}
        {activeView === "library" && (
          <LibraryView
            templates={filteredTemplates}
            allTemplates={templates}
            categories={categories}
            loadState={loadState}
            loadError={loadError}
            query={query}
            categoryFilter={categoryFilter}
            statusFilter={statusFilter}
            sort={sort}
            creationMode={creationMode}
            operationError={operationError}
            createForm={createForm}
            cloneForm={cloneForm}
            onQueryChange={setQuery}
            onCategoryChange={setCategoryFilter}
            onStatusChange={setStatusFilter}
            onSortChange={setSort}
            onRefresh={() => void refreshAll()}
            onOpen={(template, revision) => void openRevision(template, revision)}
            onCreateMode={setCreationMode}
            onCreateFormChange={setCreateForm}
            onCloneFormChange={setCloneForm}
            onCreate={() => void createTemplate()}
            onClone={() => void cloneTemplate()}
          />
        )}
        {activeView === "studio" && selectedTemplate && selectedRevision && definition && (
          <StudioView
            template={selectedTemplate}
            revision={selectedRevision}
            definition={definition}
            readOnly={readOnly}
            dirty={dirty}
            saveState={saveState}
            saveError={saveError}
            operationError={operationError}
            conflict={conflict}
            validation={validation}
            section={section}
            revisions={revisions}
            audit={audit}
            context={context}
            expectedChecksum={expectedChecksum}
            onBack={() => setActiveView("library")}
            onSectionChange={setSection}
            onDefinitionChange={updateDefinition}
            onContextChange={setContext}
            onValidate={() => void validateCurrentDefinition()}
            onSave={() => void saveDraft()}
            onSubmit={() => void submitForReview()}
            onApprove={() => void approveRevision()}
            onDerive={() => void deriveNewRevision()}
            onReload={() => void reloadSelectedTemplate()}
            onOpenRevision={(revision) => void openRevision(selectedTemplate, revision)}
            onKeepLocal={() => {
              setConflict(null);
              setSaveState("dirty");
            }}
          />
        )}
      </main>
    </div>
  );
}

function LibraryView(props: {
  templates: TestTemplateAggregate[];
  allTemplates: TestTemplateAggregate[];
  categories: string[];
  loadState: "loading" | "ready" | "empty" | "error";
  loadError: string | null;
  query: string;
  categoryFilter: string;
  statusFilter: string;
  sort: string;
  creationMode: "none" | "create" | "clone";
  operationError: string | null;
  createForm: CreateFormState;
  cloneForm: CloneFormState;
  onQueryChange: (value: string) => void;
  onCategoryChange: (value: string) => void;
  onStatusChange: (value: string) => void;
  onSortChange: (value: string) => void;
  onRefresh: () => void;
  onOpen: (template: TestTemplateAggregate, revision: TestTemplateRevision | null) => void;
  onCreateMode: (mode: "none" | "create" | "clone") => void;
  onCreateFormChange: (value: CreateFormState) => void;
  onCloneFormChange: (value: CloneFormState) => void;
  onCreate: () => void;
  onClone: () => void;
}) {
  const cloneSources = props.allTemplates.filter((template) => template.current_approved_revision);
  return (
    <section className="library">
      <div className="toolbar">
        <label className="searchBox">
          <Search size={16} />
          <input
            aria-label="Recherche template"
            value={props.query}
            onChange={(event) => props.onQueryChange(event.target.value)}
            placeholder="Recherche"
          />
        </label>
        <select
          aria-label="Filtre categorie"
          value={props.categoryFilter}
          onChange={(event) => props.onCategoryChange(event.target.value)}
        >
          <option value="all">Toutes categories</option>
          {props.categories.map((category) => (
            <option key={category} value={category}>
              {category}
            </option>
          ))}
        </select>
        <select
          aria-label="Filtre statut"
          value={props.statusFilter}
          onChange={(event) => props.onStatusChange(event.target.value)}
        >
          <option value="all">Tous statuts</option>
          {statusOrder.map((status) => (
            <option key={status} value={status}>
              {statusLabels[status]}
            </option>
          ))}
        </select>
        <select aria-label="Tri" value={props.sort} onChange={(event) => props.onSortChange(event.target.value)}>
          <option value="updated_desc">Derniere modification</option>
          <option value="title_asc">Titre bibliotheque</option>
          <option value="category_asc">Categorie</option>
          <option value="revision_desc">Revision</option>
        </select>
        <button onClick={props.onRefresh}>
          <RefreshCw size={16} /> Rafraichir
        </button>
        <button onClick={() => props.onCreateMode("create")}>
          <ClipboardCheck size={16} /> Creer
        </button>
        <button onClick={() => props.onCreateMode("clone")} disabled={cloneSources.length === 0}>
          <Copy size={16} /> Cloner
        </button>
      </div>
      {props.operationError && <StateBlock title="Erreur operation" detail={props.operationError} tone="bad" />}

      {props.creationMode === "create" && (
        <div className="actionPanel">
          <h2>Creation vide</h2>
          <TextInput label="Identifiant" value={props.createForm.template_id} onChange={(template_id) => props.onCreateFormChange({ ...props.createForm, template_id })} />
          <TextInput label="Titre bibliotheque" value={props.createForm.title} onChange={(title) => props.onCreateFormChange({ ...props.createForm, title })} />
          <TextInput label="Categorie" value={props.createForm.category_code} onChange={(category_code) => props.onCreateFormChange({ ...props.createForm, category_code })} />
          <TextInput label="Acteur saisi manuellement" value={props.createForm.actor} onChange={(actor) => props.onCreateFormChange({ ...props.createForm, actor })} />
          <TextInput label="Raison" value={props.createForm.reason} onChange={(reason) => props.onCreateFormChange({ ...props.createForm, reason })} />
          <div className="buttonRow">
            <button onClick={props.onCreate}>Creer le brouillon</button>
            <button className="secondary" onClick={() => props.onCreateMode("none")}>Annuler</button>
          </div>
        </div>
      )}

      {props.creationMode === "clone" && (
        <div className="actionPanel">
          <h2>Clonage serveur</h2>
          <label>
            Source approuvee
            <select
              value={`${props.cloneForm.source_template_id}|${props.cloneForm.source_revision_id}`}
              onChange={(event) => {
                const [source_template_id, source_revision_id] = event.target.value.split("|");
                props.onCloneFormChange({ ...props.cloneForm, source_template_id, source_revision_id });
              }}
            >
              <option value="|">Selectionner</option>
              {cloneSources.map((template) => (
                <option
                  key={template.identity.template_id}
                  value={`${template.identity.template_id}|${template.current_approved_revision?.revision_id ?? ""}`}
                >
                  {template.identity.template_id} - {template.current_approved_revision?.revision_id}
                </option>
              ))}
            </select>
          </label>
          <TextInput label="Nouvel identifiant" value={props.cloneForm.new_template_id} onChange={(new_template_id) => props.onCloneFormChange({ ...props.cloneForm, new_template_id })} />
          <TextInput label="Nouveau titre bibliotheque" value={props.cloneForm.title} onChange={(title) => props.onCloneFormChange({ ...props.cloneForm, title })} />
          <TextInput label="Nouvelle categorie optionnelle" value={props.cloneForm.category_code} onChange={(category_code) => props.onCloneFormChange({ ...props.cloneForm, category_code })} />
          <TextInput label="Acteur saisi manuellement" value={props.cloneForm.actor} onChange={(actor) => props.onCloneFormChange({ ...props.cloneForm, actor })} />
          <TextInput label="Raison" value={props.cloneForm.reason} onChange={(reason) => props.onCloneFormChange({ ...props.cloneForm, reason })} />
          <div className="buttonRow">
            <button onClick={props.onClone}>Cloner vers un nouveau template</button>
            <button className="secondary" onClick={() => props.onCreateMode("none")}>Annuler</button>
          </div>
        </div>
      )}

      {props.loadState === "loading" && <StateBlock title="Chargement" detail="Lecture de l'API locale." />}
      {props.loadState === "error" && <StateBlock title="Erreur reseau" detail={props.loadError ?? "API indisponible."} tone="bad" />}
      {props.loadState === "empty" && <StateBlock title="Aucun template" detail="La bibliotheque API ne contient aucun template." />}

      {props.templates.length > 0 && (
        <div className="templateGrid">
          {props.templates.map((template) => {
            const visibleRevision = template.active_draft_revision ?? template.current_approved_revision ?? template.latest_revision;
            return (
              <article key={template.identity.template_id} className="templateCard">
                <div className="templateCardHeading">
                  <div>
                    <p className="eyebrow">Méthode d'essai</p>
                    <h2>{template.identity.title}</h2>
                  </div>
                  {visibleRevision && <StatusBadge status={visibleRevision.status} />}
                </div>
                <span className="pill templateCategory">{humanizeCode(template.identity.category_code)}</span>
                <dl className="templateSummary">
                  <dt>Révision</dt>
                  <dd>{visibleRevision ? visibleRevision.revision_number : "-"}</dd>
                  <dt>Dernière modification</dt>
                  <dd>{formatDate(template.identity.updated_at)}</dd>
                </dl>
                <details className="templateTechnicalDetails">
                  <summary>Référence technique</summary>
                  <code>{template.identity.template_id}</code>
                </details>
                <div className="buttonRow">
                  <button onClick={() => props.onOpen(template, visibleRevision)} disabled={!visibleRevision}>
                    {template.active_draft_revision ? "Continuer le brouillon" : "Consulter"}
                  </button>
                  {template.active_draft_revision && template.current_approved_revision && (
                    <button className="secondary" onClick={() => props.onOpen(template, template.current_approved_revision)}>
                      Consulter l'approuvée
                    </button>
                  )}
                </div>
              </article>
            );
          })}
        </div>
      )}
    </section>
  );
}

function StudioView(props: {
  template: TestTemplateAggregate;
  revision: TestTemplateRevision;
  definition: TestTemplateDefinition;
  readOnly: boolean;
  dirty: boolean;
  saveState: SaveState;
  saveError: string | null;
  operationError: string | null;
  conflict: { expected?: string; actual?: string } | null;
  validation: ValidationResult | null;
  section: StudioSection;
  revisions: TestTemplateRevision[];
  audit: AuditEvent[];
  context: OperationContext;
  expectedChecksum: string;
  onBack: () => void;
  onSectionChange: (section: StudioSection) => void;
  onDefinitionChange: (definition: TestTemplateDefinition) => void;
  onContextChange: (context: OperationContext) => void;
  onValidate: () => void;
  onSave: () => void;
  onSubmit: () => void;
  onApprove: () => void;
  onDerive: () => void;
  onReload: () => void;
  onOpenRevision: (revision: TestTemplateRevision) => void;
  onKeepLocal: () => void;
}) {
  const canSubmit = props.revision.status === "draft" && !props.dirty && props.validation?.valid === true;
  return (
    <section className="studio">
      <div className="studioHeader">
        <button className="secondary" onClick={props.onBack}>Bibliothèque</button>
        <div className="studioHeading">
          <p className="eyebrow">Méthode d'essai</p>
          <h2>{props.template.identity.title}</h2>
          <div className="studioTitleMeta">
            <StatusBadge status={props.revision.status} />
            <span>Révision {props.revision.revision_number}</span>
            <span className={"saveState " + props.saveState}>{saveStateLabel(props.saveState)}</span>
          </div>
        </div>
        <div className="headerActions">
          <button className="secondary" onClick={props.onValidate}>
            <CheckCircle2 size={16} /> Valider
          </button>
          {props.revision.status === "draft" && (
            <>
              <button onClick={props.onSave} disabled={props.saveState === "saving" || !props.dirty}>
                <Save size={16} /> Sauvegarder
              </button>
              <button className="secondary" onClick={props.onSubmit} disabled={!canSubmit}>
                <Send size={16} /> Soumettre
              </button>
            </>
          )}
          {props.revision.status === "under_review" && (
            <button onClick={props.onApprove}>
              <ShieldCheck size={16} /> Approuver
            </button>
          )}
          {props.revision.status === "approved" && props.template.current_approved_revision && !props.template.active_draft_revision && (
            <button onClick={props.onDerive}>
              <GitBranch size={16} /> Dériver
            </button>
          )}
        </div>
      </div>

      <details className="traceabilityDetails">
        <summary>Contexte de traçabilité</summary>
        <div className="contextBar">
          <TextInput label="Acteur saisi manuellement" value={props.context.actor} onChange={(actor) => props.onContextChange({ ...props.context, actor })} />
          <TextInput label="Raison" value={props.context.reason} onChange={(reason) => props.onContextChange({ ...props.context, reason })} />
          <div>
            <small>Checksum ouvert</small>
            <code>{props.expectedChecksum}</code>
          </div>
        </div>
      </details>

      {props.conflict && (
        <div className="conflictBox" role="alert">
          <AlertTriangle size={18} />
          <div>
            <strong>Conflit de sauvegarde</strong>
            <p>Une autre modification a ete enregistree. La fusion silencieuse est bloquee.</p>
            <p>Checksum local: <code>{props.conflict.expected}</code></p>
            <p>Checksum serveur: <code>{props.conflict.actual}</code></p>
            <div className="buttonRow">
              <button onClick={props.onReload}>Recharger la version serveur</button>
              <button className="secondary" onClick={props.onKeepLocal}>Conserver la copie locale</button>
            </div>
          </div>
        </div>
      )}

      {props.saveError && <StateBlock title="Erreur de sauvegarde" detail={props.saveError} tone="bad" />}
      {props.operationError && <StateBlock title="Erreur operation" detail={props.operationError} tone="bad" />}

      <div className="studioLayout">
        <aside className="sectionNav">
          {sectionLabels.map(([id, label]) => (
            <button key={id} className={props.section === id ? "active" : ""} onClick={() => props.onSectionChange(id)}>
              {label}
            </button>
          ))}
        </aside>
        <div className="editorPane">
          {renderEditorSection(props)}
        </div>
        <ValidationPanel validation={props.validation} readOnly={props.readOnly} dirty={props.dirty} />
      </div>
      <HistoryPanel audit={props.audit} />
    </section>
  );
}

function renderEditorSection(props: Parameters<typeof StudioView>[0]) {
  const definition = props.definition;
  const update = props.onDefinitionChange;
  switch (props.section) {
    case "general":
      return <GeneralEditor definition={definition} readOnly={props.readOnly} onChange={update} templateId={props.template.identity.template_id} />;
    case "variables":
      return <VariablesEditor definition={definition} readOnly={props.readOnly} onChange={update} />;
    case "locks":
      return <LockPolicyEditor definition={definition} readOnly={props.readOnly} onChange={update} />;
    case "instrumentation":
      return <InstrumentationEditor definition={definition} readOnly={props.readOnly} onChange={update} />;
    case "sequence":
      return <SequenceEditor definition={definition} readOnly={props.readOnly} onChange={update} />;
    case "limits":
      return <LimitsEditor definition={definition} readOnly={props.readOnly} onChange={update} />;
    case "post":
      return <PostProcessingEditor definition={definition} readOnly={props.readOnly} onChange={update} />;
    case "revisions":
      return <RevisionsView revisions={props.revisions} onOpen={props.onOpenRevision} />;
    case "audit":
      return <AuditView audit={props.audit} />;
    case "advanced":
      return <AdvancedJson definition={definition} validation={props.validation} />;
  }
}

function GeneralEditor(props: {
  definition: TestTemplateDefinition;
  readOnly: boolean;
  templateId: string;
  onChange: (definition: TestTemplateDefinition) => void;
}) {
  const d = props.definition;
  return (
    <EditorCard title="General">
      <TextInput label="Identifiant stable" value={props.templateId} disabled />
      <TextInput label="Titre technique de revision" value={d.title} disabled={props.readOnly} onChange={(title) => props.onChange({ ...d, title })} />
      <TextArea label="Description" value={d.description} disabled={props.readOnly} onChange={(description) => props.onChange({ ...d, description })} />
      <label>
        Axe de mesure
        <select value={d.measurement_axis} disabled={props.readOnly} onChange={(event) => props.onChange({ ...d, measurement_axis: event.target.value as TestTemplateDefinition["measurement_axis"] })}>
          <option value="frequency_sweep">Balayage frequence</option>
          <option value="time_series">Serie temporelle</option>
          <option value="event_triggered">Evenementiel</option>
          <option value="mixed_time_frequency">Temps/frequence</option>
        </select>
      </label>
      <TextInput label="Version schema" value={d.definition_schema_version} disabled />
      <TextInput label="Code methode" value={d.method_code ?? ""} disabled={props.readOnly} onChange={(method_code) => props.onChange({ ...d, method_code: optionalValue(method_code) })} />
      <TextInput label="Revision methode" value={d.method_revision ?? ""} disabled={props.readOnly} onChange={(method_revision) => props.onChange({ ...d, method_revision: optionalValue(method_revision) })} />
      <TextInput label="References" value={d.standard_references.join(", ")} disabled={props.readOnly} onChange={(value) => props.onChange({ ...d, standard_references: splitList(value) })} />
    </EditorCard>
  );
}

function VariablesEditor(props: {
  definition: TestTemplateDefinition;
  readOnly: boolean;
  onChange: (definition: TestTemplateDefinition) => void;
}) {
  const variables = props.definition.variables;
  const update = (index: number, variable: VariableDefinition) => {
    props.onChange({ ...props.definition, variables: replaceAt(variables, index, variable) });
  };
  return (
    <EditorCard title="Variables">
      <StructuredTable columns={["ID", "Type", "Unite", "Defaut", "Min", "Max", "Enum", "Obligatoire", "Dimensionless", "Description", ""]}>
        {variables.map((variable, index) => (
          <tr key={`${variable.variable_id}-${index}`}>
            <td>
              <input
                aria-label={`Variable ${index + 1} ID`}
                value={variable.variable_id}
                disabled={props.readOnly}
                onChange={(event) => update(index, { ...variable, variable_id: event.target.value })}
              />
            </td>
            <td>
              <select value={variable.value_type} disabled={props.readOnly} onChange={(event) => update(index, { ...variable, value_type: event.target.value as VariableDefinition["value_type"] })}>
                <option value="number">number</option>
                <option value="integer">integer</option>
                <option value="boolean">boolean</option>
                <option value="text">text</option>
                <option value="enum">enum</option>
              </select>
            </td>
            <td><input value={variable.constraints.unit ?? ""} disabled={props.readOnly || Boolean(variable.constraints.dimensionless)} onChange={(event) => update(index, { ...variable, constraints: { ...variable.constraints, unit: optionalValue(event.target.value) } })} /></td>
            <td><input value={String(variable.default_value ?? "")} disabled={props.readOnly} onChange={(event) => update(index, { ...variable, default_value: parseDefaultValue(variable.value_type, event.target.value) })} /></td>
            <td><input value={variable.constraints.minimum ?? ""} disabled={props.readOnly} onChange={(event) => update(index, { ...variable, constraints: { ...variable.constraints, minimum: optionalNumber(event.target.value) } })} /></td>
            <td><input value={variable.constraints.maximum ?? ""} disabled={props.readOnly} onChange={(event) => update(index, { ...variable, constraints: { ...variable.constraints, maximum: optionalNumber(event.target.value) } })} /></td>
            <td><input value={(variable.constraints.enum_values ?? []).join(", ")} disabled={props.readOnly} onChange={(event) => update(index, { ...variable, constraints: { ...variable.constraints, enum_values: splitList(event.target.value) } })} /></td>
            <td><input type="checkbox" checked={variable.constraints.required} disabled={props.readOnly} onChange={(event) => update(index, { ...variable, constraints: { ...variable.constraints, required: event.target.checked } })} /></td>
            <td><input type="checkbox" checked={Boolean(variable.constraints.dimensionless)} disabled={props.readOnly} onChange={(event) => update(index, { ...variable, constraints: { ...variable.constraints, dimensionless: event.target.checked, unit: event.target.checked ? undefined : variable.constraints.unit } })} /></td>
            <td><input value={variable.description ?? ""} disabled={props.readOnly} onChange={(event) => update(index, { ...variable, description: optionalValue(event.target.value) })} /></td>
            <td><button className="danger" disabled={props.readOnly || variables.length <= 1} onClick={() => props.onChange({ ...props.definition, variables: variables.filter((_, itemIndex) => itemIndex !== index) })}>Supprimer</button></td>
          </tr>
        ))}
      </StructuredTable>
      <button disabled={props.readOnly} onClick={() => props.onChange({ ...props.definition, variables: [...variables, newVariable(variables.length + 1)] })}>Ajouter une variable</button>
    </EditorCard>
  );
}

function LockPolicyEditor(props: {
  definition: TestTemplateDefinition;
  readOnly: boolean;
  onChange: (definition: TestTemplateDefinition) => void;
}) {
  const policies = props.definition.lock_policy;
  const variables = props.definition.variables;
  return (
    <EditorCard title="Politiques de verrouillage">
      <StructuredTable columns={["Variable", "Politique", "Justification", "Consequence accreditee", ""]}>
        {policies.map((policy, index) => (
          <tr key={`${policy.variable_id}-${index}`}>
            <td>
              <select value={policy.variable_id} disabled={props.readOnly} onChange={(event) => props.onChange({ ...props.definition, lock_policy: replaceAt(policies, index, { ...policy, variable_id: event.target.value }) })}>
                {variables.map((variable) => <option key={variable.variable_id} value={variable.variable_id}>{variable.variable_id}</option>)}
              </select>
            </td>
            <td>
              <select value={policy.policy} disabled={props.readOnly} onChange={(event) => props.onChange({ ...props.definition, lock_policy: replaceAt(policies, index, { ...policy, policy: event.target.value as VariableLockPolicy["policy"] }) })}>
                <option value="editable_until_campaign_freeze">editable_until_campaign_freeze</option>
                <option value="editable_until_execution">editable_until_execution</option>
                <option value="admin_only">admin_only</option>
                <option value="investigation_only">investigation_only</option>
                <option value="immutable">immutable</option>
              </select>
            </td>
            <td>Modification normale, avec justification, ou interdite selon le contexte qualite.</td>
            <td>RBAC et regles configurables prevus dans le domaine personnes/competences.</td>
            <td><button className="danger" disabled={props.readOnly} onClick={() => props.onChange({ ...props.definition, lock_policy: policies.filter((_, itemIndex) => itemIndex !== index) })}>Supprimer</button></td>
          </tr>
        ))}
      </StructuredTable>
      <button disabled={props.readOnly || variables.length === 0} onClick={() => props.onChange({ ...props.definition, lock_policy: [...policies, { variable_id: variables[0]?.variable_id ?? "", policy: "editable_until_execution" }] })}>Ajouter une politique</button>
    </EditorCard>
  );
}

function InstrumentationEditor(props: {
  definition: TestTemplateDefinition;
  readOnly: boolean;
  onChange: (definition: TestTemplateDefinition) => void;
}) {
  const slots = props.definition.instrumentation_chain;
  const update = (index: number, slot: InstrumentationChainSlot) => props.onChange({ ...props.definition, instrumentation_chain: replaceAt(slots, index, slot) });
  return (
    <EditorCard title="Chaine d'instrumentation">
      <p className="notice">Vue graphique prevue dans une version ulterieure</p>
      <div className="cardList">
        {slots.map((slot, index) => (
          <div className="slotCard" key={`${slot.slot_id}-${index}`}>
            <TextInput label="Identifiant logique" value={slot.slot_id} disabled={props.readOnly} onChange={(slot_id) => update(index, { ...slot, slot_id })} />
            <TextInput label="Role" value={slot.label} disabled={props.readOnly} onChange={(label) => update(index, { ...slot, label })} />
            <TextInput label="Categorie attendue" value={slot.required_category ?? ""} disabled={props.readOnly} onChange={(required_category) => update(index, { ...slot, required_category: optionalValue(required_category) })} />
            <TextInput label="Capacite attendue" value={slot.required_capability ?? ""} disabled={props.readOnly} onChange={(required_capability) => update(index, { ...slot, required_capability: optionalValue(required_capability) })} />
            <TextInput label="Contraintes slots" value={(slot.depends_on_slots ?? []).join(", ")} disabled={props.readOnly} onChange={(value) => update(index, { ...slot, depends_on_slots: splitList(value) })} />
            <label><input type="checkbox" checked={slot.required} disabled={props.readOnly} onChange={(event) => update(index, { ...slot, required: event.target.checked })} /> Obligatoire</label>
            <label>Calibration<select value={slot.calibration_requirement} disabled={props.readOnly} onChange={(event) => update(index, { ...slot, calibration_requirement: event.target.value as InstrumentationChainSlot["calibration_requirement"] })}><option value="required">required</option><option value="not_required">not_required</option><option value="if_used">if_used</option></select></label>
            <label>Substitution<select value={slot.substitution_policy} disabled={props.readOnly} onChange={(event) => update(index, { ...slot, substitution_policy: event.target.value as InstrumentationChainSlot["substitution_policy"] })}><option value="no_substitution">no_substitution</option><option value="same_category">same_category</option><option value="same_capability">same_capability</option><option value="approved_equivalent">approved_equivalent</option></select></label>
            <button className="danger" disabled={props.readOnly} onClick={() => props.onChange({ ...props.definition, instrumentation_chain: slots.filter((_, itemIndex) => itemIndex !== index) })}>Supprimer</button>
          </div>
        ))}
      </div>
      <button disabled={props.readOnly} onClick={() => props.onChange({ ...props.definition, instrumentation_chain: [...slots, newSlot(slots.length + 1)] })}>Ajouter un emplacement</button>
    </EditorCard>
  );
}

function SequenceEditor(props: {
  definition: TestTemplateDefinition;
  readOnly: boolean;
  onChange: (definition: TestTemplateDefinition) => void;
}) {
  const steps = [...props.definition.sequence].sort((a, b) => a.order - b.order);
  const update = (index: number, step: TestTemplateDefinition["sequence"][number]) => props.onChange({ ...props.definition, sequence: replaceAt(steps, index, step) });
  return (
    <EditorCard title="Sequence d'execution">
      <StructuredTable columns={["Ordre", "ID", "Type", "Titre", "Description", "Slots", "Branches", ""]}>
        {steps.map((step, index) => (
          <tr key={`${step.step_id}-${index}`}>
            <td><input value={step.order} disabled={props.readOnly} onChange={(event) => update(index, { ...step, order: Number(event.target.value) || 0 })} /></td>
            <td><input value={step.step_id} disabled={props.readOnly} onChange={(event) => update(index, { ...step, step_id: event.target.value })} /></td>
            <td><select value={step.kind} disabled={props.readOnly} onChange={(event) => update(index, { ...step, kind: event.target.value as TestTemplateDefinition["sequence"][number]["kind"] })}><option value="prepare">prepare</option><option value="configure_instrument">configure_instrument</option><option value="acquire">acquire</option><option value="operator_decision">operator_decision</option><option value="post_process">post_process</option><option value="verify">verify</option><option value="finish">finish</option></select></td>
            <td><input value={step.label} disabled={props.readOnly} onChange={(event) => update(index, { ...step, label: event.target.value })} /></td>
            <td><input value={step.instruction ?? ""} disabled={props.readOnly} onChange={(event) => update(index, { ...step, instruction: optionalValue(event.target.value) })} /></td>
            <td><input value={(step.required_slots ?? []).join(", ")} disabled={props.readOnly} onChange={(event) => update(index, { ...step, required_slots: splitList(event.target.value) })} /></td>
            <td>
              <textarea
                value={(step.branches ?? []).map((branch) => `${branch.rule_id}:${branch.condition}->${branch.destination_step_id}${branch.allow_cycle ? ":cycle" : ""}`).join("\n")}
                disabled={props.readOnly}
                onChange={(event) => update(index, { ...step, branches: parseBranches(event.target.value) })}
              />
              <small>Condition textuelle provisoire</small>
            </td>
            <td><button className="danger" disabled={props.readOnly || steps.length <= 1} onClick={() => props.onChange({ ...props.definition, sequence: steps.filter((_, itemIndex) => itemIndex !== index) })}>Supprimer</button></td>
          </tr>
        ))}
      </StructuredTable>
      <button disabled={props.readOnly} onClick={() => props.onChange({ ...props.definition, sequence: [...steps, newStep(steps.length + 1)] })}>Ajouter une etape</button>
    </EditorCard>
  );
}

function LimitsEditor(props: {
  definition: TestTemplateDefinition;
  readOnly: boolean;
  onChange: (definition: TestTemplateDefinition) => void;
}) {
  const limits = props.definition.limits;
  const update = (index: number, limit: LimitDefinition) => props.onChange({ ...props.definition, limits: replaceAt(limits, index, limit) });
  return (
    <EditorCard title="Limites">
      <p className="notice">Les courbes multi-segments, masques, detecteurs et regles avancees seront ajoutes dans une version ulterieure.</p>
      <StructuredTable columns={["ID", "Type", "Axe", "Unite", "Domaine", "Source", "Seuil", "Attention", "Variables", ""]}>
        {limits.map((limit, index) => (
          <tr key={`${limit.limit_id}-${index}`}>
            <td><input value={limit.limit_id} disabled={props.readOnly} onChange={(event) => update(index, { ...limit, limit_id: event.target.value })} /></td>
            <td><select value={limit.kind} disabled={props.readOnly} onChange={(event) => update(index, { ...limit, kind: event.target.value as LimitDefinition["kind"] })}><option value="time_limit">time_limit</option><option value="frequency_limit">frequency_limit</option><option value="scalar_threshold">scalar_threshold</option></select></td>
            <td><select value={limit.axis} disabled={props.readOnly} onChange={(event) => update(index, { ...limit, axis: event.target.value as LimitDefinition["axis"] })}><option value="frequency_sweep">frequency_sweep</option><option value="time_series">time_series</option><option value="event_triggered">event_triggered</option><option value="mixed_time_frequency">mixed_time_frequency</option></select></td>
            <td><input value={limit.unit} disabled={props.readOnly} onChange={(event) => update(index, { ...limit, unit: event.target.value })} /></td>
            <td><input value={limit.application_domain} disabled={props.readOnly} onChange={(event) => update(index, { ...limit, application_domain: event.target.value })} /></td>
            <td><input value={limit.source_reference} disabled={props.readOnly} onChange={(event) => update(index, { ...limit, source_reference: event.target.value })} /></td>
            <td><input value={limit.threshold ?? ""} disabled={props.readOnly} onChange={(event) => update(index, { ...limit, threshold: optionalNumber(event.target.value) })} /></td>
            <td><input value={limit.attention_rule ?? ""} disabled={props.readOnly} onChange={(event) => update(index, { ...limit, attention_rule: optionalValue(event.target.value) })} /></td>
            <td><input value={(limit.variable_refs ?? []).join(", ")} disabled={props.readOnly} onChange={(event) => update(index, { ...limit, variable_refs: splitList(event.target.value) })} /></td>
            <td><button className="danger" disabled={props.readOnly} onClick={() => props.onChange({ ...props.definition, limits: limits.filter((_, itemIndex) => itemIndex !== index) })}>Supprimer</button></td>
          </tr>
        ))}
      </StructuredTable>
      <button disabled={props.readOnly} onClick={() => props.onChange({ ...props.definition, limits: [...limits, newLimit(limits.length + 1)] })}>Ajouter une limite</button>
    </EditorCard>
  );
}

function PostProcessingEditor(props: {
  definition: TestTemplateDefinition;
  readOnly: boolean;
  onChange: (definition: TestTemplateDefinition) => void;
}) {
  const operations = props.definition.post_processing;
  const update = (index: number, operation: PostProcessingDefinition) => props.onChange({ ...props.definition, post_processing: replaceAt(operations, index, operation) });
  return (
    <EditorCard title="Post-traitement">
      <StructuredTable columns={["Ordre", "ID", "Operation", "Entrees", "Sorties", "Parametres JSON", ""]}>
        {operations.map((operation, index) => (
          <tr key={`${operation.operation_id}-${index}`}>
            <td><input value={operation.order} disabled={props.readOnly} onChange={(event) => update(index, { ...operation, order: Number(event.target.value) || 0 })} /></td>
            <td><input value={operation.operation_id} disabled={props.readOnly} onChange={(event) => update(index, { ...operation, operation_id: event.target.value })} /></td>
            <td><select value={operation.operation_type} disabled={props.readOnly} onChange={(event) => update(index, { ...operation, operation_type: event.target.value as PostProcessingDefinition["operation_type"] })}><option value="correction">correction</option><option value="fft">fft</option><option value="windowing">windowing</option><option value="resampling">resampling</option><option value="harmonic_calculation">harmonic_calculation</option><option value="event_counting">event_counting</option><option value="channel_math">channel_math</option><option value="peak">peak</option><option value="custom">custom</option></select></td>
            <td><input value={operation.inputs.join(", ")} disabled={props.readOnly} onChange={(event) => update(index, { ...operation, inputs: splitList(event.target.value) })} /></td>
            <td><input value={operation.outputs.join(", ")} disabled={props.readOnly} onChange={(event) => update(index, { ...operation, outputs: splitList(event.target.value) })} /></td>
            <td><textarea value={JSON.stringify(operation.parameters)} disabled={props.readOnly} onChange={(event) => update(index, { ...operation, parameters: parseJsonObject(event.target.value) })} /></td>
            <td><button className="danger" disabled={props.readOnly} onClick={() => props.onChange({ ...props.definition, post_processing: operations.filter((_, itemIndex) => itemIndex !== index) })}>Supprimer</button></td>
          </tr>
        ))}
      </StructuredTable>
      <button disabled={props.readOnly} onClick={() => props.onChange({ ...props.definition, post_processing: [...operations, newPost(operations.length + 1)] })}>Ajouter une operation</button>
    </EditorCard>
  );
}

function RevisionsView(props: { revisions: TestTemplateRevision[]; onOpen: (revision: TestTemplateRevision) => void }) {
  return (
    <EditorCard title="Historique des revisions">
      <StructuredTable columns={["Numero", "Revision", "Statut", "Parent", "Checksum", "Auteur", "Creation", "Soumission", "Approbation", ""]}>
        {props.revisions.map((revision) => (
          <tr key={revision.revision_id}>
            <td>{revision.revision_number}</td>
            <td className="mono">{revision.revision_id}</td>
            <td><StatusBadge status={revision.status} /></td>
            <td>{revision.parent_revision_id ?? "-"}</td>
            <td><code>{revision.definition_checksum}</code></td>
            <td>{revision.created_by}</td>
            <td>{formatDate(revision.created_at)}</td>
            <td>{formatDate(revision.submitted_at)}</td>
            <td>{formatDate(revision.approved_at)}</td>
            <td><button className="secondary" onClick={() => props.onOpen(revision)}>Ouvrir</button></td>
          </tr>
        ))}
      </StructuredTable>
    </EditorCard>
  );
}

function AuditView(props: { audit: AuditEvent[] }) {
  return (
    <EditorCard title="Audit">
      <StructuredTable columns={["Date", "Action", "Acteur", "Raison", "Ancienne revision", "Nouvelle revision", "Ancien checksum", "Nouveau checksum", "Operation", "Correlation"]}>
        {props.audit.map((event) => (
          <tr key={event.audit_id}>
            <td>{formatDate(event.occurred_at)}</td>
            <td>{event.action}</td>
            <td>{event.actor}</td>
            <td>{event.reason}</td>
            <td>{event.old_revision_id ?? "-"}</td>
            <td>{event.new_revision_id ?? "-"}</td>
            <td><code>{event.old_definition_checksum ?? "-"}</code></td>
            <td><code>{event.new_definition_checksum ?? "-"}</code></td>
            <td>{event.operation_id}</td>
            <td>{event.correlation_id}</td>
          </tr>
        ))}
      </StructuredTable>
    </EditorCard>
  );
}

function AdvancedJson(props: { definition: TestTemplateDefinition; validation: ValidationResult | null }) {
  return (
    <EditorCard title="JSON canonique">
      <textarea className="jsonPreview" readOnly value={props.validation?.canonical_json ?? JSON.stringify(props.definition, null, 2)} />
    </EditorCard>
  );
}

function ValidationPanel(props: { validation: ValidationResult | null; readOnly: boolean; dirty: boolean }) {
  return (
    <aside className="validationPanel">
      <h2>Validation</h2>
      {props.readOnly && <p className="notice">Revision en lecture seule.</p>}
      {props.dirty && <p className="notice">Modifications non sauvegardees.</p>}
      {!props.validation && <p>Aucun verdict serveur courant.</p>}
      {props.validation?.valid && (
        <p className="validationOk"><CheckCircle2 size={16} /> Definition valide</p>
      )}
      {props.validation && !props.validation.valid && (
        <ul>
          {props.validation.issues.map((issue) => (
            <li key={`${issue.code}-${issue.path}`}>
              <strong>{issue.code}</strong>
              <span>{issue.path}</span>
              <p>{issue.message}</p>
            </li>
          ))}
        </ul>
      )}
    </aside>
  );
}

function HistoryPanel(props: { audit: AuditEvent[] }) {
  return (
    <footer className="bottomPanel">
      <History size={16} />
      <span>{props.audit.length} evenements audit charges</span>
    </footer>
  );
}

function SystemView(props: { health: HealthReport | null; storage: StorageStatus | null }) {
  return (
    <section className="systemView">
      <EditorCard title="Systeme local">
        <dl className="systemGrid">
          <dt>Version logiciel</dt>
          <dd>{APP_VERSION}</dd>
          <dt>Connexion agent</dt>
          <dd>{props.health ? "Connecte" : "Indisponible"}</dd>
          <dt>URL agent</dt>
          <dd>{window.location.origin}</dd>
          <dt>Stockage local</dt>
          <dd>{props.health?.storage_root ?? props.storage?.storage_root ?? "-"}</dd>
          <dt>Mode</dt>
          <dd>Local</dd>
        </dl>
      </EditorCard>
      {props.storage && (
        <EditorCard title="Domaines SQLite">
          <StructuredTable columns={["Domaine", "Schema", "Derniere migration", "Statut", "Base", "Journal"]}>
            {props.storage.domains.map((domain) => (
              <tr key={domain.domain}>
                <td>{domain.domain}</td>
                <td>{domain.schema_version ?? "-"}</td>
                <td>{domain.latest_migration}</td>
                <td>{domain.status}</td>
                <td>{domain.database_path}</td>
                <td>{domain.journal_mode ?? "-"}</td>
              </tr>
            ))}
          </StructuredTable>
        </EditorCard>
      )}
    </section>
  );
}

function EditorCard(props: { title: string; children: ReactNode }) {
  return (
    <section className="editorCard">
      <h2>{props.title}</h2>
      {props.children}
    </section>
  );
}

function StructuredTable(props: { columns: string[]; children: ReactNode }) {
  return (
    <div className="tableWrap">
      <table>
        <thead>
          <tr>{props.columns.map((column) => <th key={column}>{column}</th>)}</tr>
        </thead>
        <tbody>{props.children}</tbody>
      </table>
    </div>
  );
}

function TextInput(props: { label: string; value: string; disabled?: boolean; onChange?: (value: string) => void }) {
  return (
    <label>
      {props.label}
      <input value={props.value} disabled={props.disabled} onChange={(event) => props.onChange?.(event.target.value)} />
    </label>
  );
}

function TextArea(props: { label: string; value: string; disabled?: boolean; onChange: (value: string) => void }) {
  return (
    <label>
      {props.label}
      <textarea value={props.value} disabled={props.disabled} onChange={(event) => props.onChange(event.target.value)} />
    </label>
  );
}

function StatusBadge(props: { status: RevisionStatus }) {
  return <span className={`status ${props.status}`}>{statusLabels[props.status]}</span>;
}

function StateBlock(props: { title: string; detail: string; tone?: "bad" | "neutral" }) {
  return (
    <div className={`stateBlock ${props.tone ?? "neutral"}`}>
      <h2>{props.title}</h2>
      <p>{props.detail}</p>
    </div>
  );
}

function compareTemplates(left: TestTemplateAggregate, right: TestTemplateAggregate, sort: string) {
  if (sort === "title_asc") {
    return left.identity.title.localeCompare(right.identity.title);
  }
  if (sort === "category_asc") {
    return left.identity.category_code.localeCompare(right.identity.category_code);
  }
  if (sort === "revision_desc") {
    return (right.latest_revision?.revision_number ?? 0) - (left.latest_revision?.revision_number ?? 0);
  }
  return right.identity.updated_at.localeCompare(left.identity.updated_at);
}

function formatDate(value?: string | null) {
  if (!value) {
    return "-";
  }
  const normalized = value.endsWith("Z") ? value : `${value}Z`;
  const parsed = new Date(normalized);
  if (Number.isNaN(parsed.getTime())) {
    return value.replace("T", " ").replace("Z", "");
  }
  return new Intl.DateTimeFormat("fr-FR", {
    dateStyle: "medium",
    timeStyle: "short"
  }).format(parsed);
}

function humanizeCode(value: string) {
  const normalized = value.replaceAll("_", " ").trim();
  return normalized ? normalized.charAt(0).toLocaleUpperCase("fr-FR") + normalized.slice(1) : "Sans catégorie";
}

function errorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

function stableStringify(value: unknown) {
  return JSON.stringify(value);
}

function optionalValue(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

function splitList(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function optionalNumber(value: string) {
  const trimmed = String(value).trim();
  if (!trimmed) {
    return undefined;
  }
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function parseDefaultValue(type: VariableDefinition["value_type"], value: string) {
  if (!value.trim()) {
    return undefined;
  }
  if (type === "boolean") {
    return value === "true" || value === "1" || value.toLowerCase() === "oui";
  }
  if (type === "number" || type === "integer") {
    return Number(value);
  }
  return value;
}

function parseBranches(value: string) {
  return value
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const [left, cycleFlag] = line.split(":cycle");
      const [ruleAndCondition, destination] = left.split("->");
      const [rule_id, condition] = ruleAndCondition.split(":");
      return {
        rule_id: rule_id?.trim() || "branch",
        condition: condition?.trim() || "condition_textuelle",
        destination_step_id: destination?.trim() || "",
        allow_cycle: Boolean(cycleFlag)
      };
    });
}

function parseJsonObject(value: string) {
  try {
    const parsed = JSON.parse(value);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>;
    }
  } catch {
    return {};
  }
  return {};
}

function replaceAt<T>(items: T[], index: number, value: T) {
  return items.map((item, itemIndex) => (itemIndex === index ? value : item));
}

function newVariable(index: number): VariableDefinition {
  return {
    variable_id: `variable_${index}`,
    label: `Variable ${index}`,
    value_type: "number",
    constraints: { required: false, dimensionless: true, enum_values: [] },
    description: ""
  };
}

function newSlot(index: number): InstrumentationChainSlot {
  return {
    slot_id: `slot_${index}`,
    label: `Emplacement ${index}`,
    required_category: "instrument",
    required: true,
    calibration_requirement: "if_used",
    substitution_policy: "same_capability",
    depends_on_slots: []
  };
}

function newStep(index: number): TestTemplateDefinition["sequence"][number] {
  return {
    step_id: `step_${index}`,
    order: index * 10,
    kind: "prepare",
    label: `Etape ${index}`,
    instruction: "",
    required_slots: [],
    branches: []
  };
}

function newLimit(index: number): LimitDefinition {
  return {
    limit_id: `limit_${index}`,
    kind: "scalar_threshold",
    axis: "time_series",
    unit: "V",
    application_domain: "investigation",
    source_reference: "internal-method",
    threshold: 1,
    attention_rule: "",
    variable_refs: []
  };
}

function newPost(index: number): PostProcessingDefinition {
  return {
    operation_id: `operation_${index}`,
    order: index * 10,
    operation_type: "custom",
    inputs: ["raw.signal"],
    outputs: [`calculated.output_${index}`],
    parameters: {}
  };
}

function saveStateLabel(state: SaveState) {
  switch (state) {
    case "clean":
      return "Non modifie";
    case "dirty":
      return "Modifications non sauvegardees";
    case "saving":
      return "Sauvegarde en cours";
    case "saved":
      return "Sauvegarde";
    case "conflict":
      return "Conflit";
    case "error":
      return "Erreur de sauvegarde";
  }
}
