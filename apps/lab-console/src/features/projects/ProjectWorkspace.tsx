import {
  AlertCircle,
  CalendarClock,
  Check,
  CheckCircle2,
  ChevronRight,
  CirclePlus,
  ClipboardCheck,
  Clock3,
  FolderKanban,
  History,
  MapPin,
  RefreshCw,
  Search,
  UserRound,
  X,
  XCircle
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { ApiError, projectApi } from "../../api";
import type {
  ContractReviewStatus,
  LaboratoryLocationOption,
  ProjectAuditEvent,
  ProjectExecutionMode,
  ProjectRecord,
  ServiceScheduleItem,
  ServiceScheduleStatus
} from "../../models/projects";

const stageLabels: Record<ProjectRecord["stage"], string> = {
  quotation: "Devis",
  contract_review: "Revue du besoin",
  test_planning: "Planification des essais",
  measuring: "Essais en cours",
  technical_review: "Revue technique",
  report_issued: "Rapport fourni",
  archived: "Archivé"
};

const modeLabels: Record<ProjectExecutionMode, string> = {
  accredited: "Accrédité",
  non_accredited: "Hors accréditation",
  investigation: "Investigation"
};

const reviewLabels: Record<string, string> = {
  customer_request_defined: "La demande du client est définie",
  test_method_selected: "La méthode d'essai est identifiée",
  laboratory_capability_confirmed: "La capacité du laboratoire est confirmée",
  equipment_availability_checked: "La disponibilité des moyens est vérifiée",
  calibration_status_reviewed: "L'état métrologique des moyens est vérifié",
  impartiality_risks_reviewed: "Les risques d'impartialité sont examinés",
  data_retention_agreed: "La conservation des données est convenue",
  report_requirements_agreed: "Les attentes du rapport sont convenues",
  deviations_recorded: "Les écarts et adaptations sont consignés"
};

const scheduleStatusLabels: Record<ServiceScheduleStatus, string> = {
  planned: "Prévu",
  confirmed: "Confirmé",
  in_progress: "En cours",
  completed: "Terminé",
  cancelled: "Annulé"
};

const auditLabels: Record<string, string> = {
  project_created: "Dossier créé",
  contract_review_item_completed: "Point de revue vérifié",
  contract_review_deviation_authorized: "Écart de revue autorisé",
  project_stage_advanced: "Dossier passé en planification",
  service_schedule_item_planned: "Créneau d'essai réservé",
  service_schedule_item_status_changed: "État du créneau modifié",
  service_schedule_item_rescheduled: "Créneau d'essai déplacé"
};

interface CreateProjectForm {
  code: string;
  customer_name: string;
  execution_mode: ProjectExecutionMode;
  actor: string;
}

interface ScheduleForm {
  title: string;
  date: string;
  start_time: string;
  end_time: string;
  assigned_operator: string;
  laboratory_location_id: string;
  equipment_under_test: string;
  notes: string;
}

export function ProjectWorkspace(props: { initialProjectCode?: string | null }) {
  const [projects, setProjects] = useState<ProjectRecord[]>([]);
  const [selectedCode, setSelectedCode] = useState<string | null>(null);
  const [selectedProject, setSelectedProject] = useState<ProjectRecord | null>(null);
  const [review, setReview] = useState<ContractReviewStatus | null>(null);
  const [schedule, setSchedule] = useState<ServiceScheduleItem[]>([]);
  const [locations, setLocations] = useState<LaboratoryLocationOption[]>([]);
  const [audit, setAudit] = useState<ProjectAuditEvent[]>([]);
  const [query, setQuery] = useState("");
  const [actor, setActor] = useState("Responsable laboratoire");
  const [loadState, setLoadState] = useState<"loading" | "ready" | "error">("loading");
  const [detailLoading, setDetailLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [showSchedule, setShowSchedule] = useState(false);
  const [reviewComment, setReviewComment] = useState("");
  const [createForm, setCreateForm] = useState<CreateProjectForm>(() => ({
    code: generatedProjectCode(),
    customer_name: "",
    execution_mode: "non_accredited",
    actor: "Responsable laboratoire"
  }));
  const [scheduleForm, setScheduleForm] = useState<ScheduleForm>(() => defaultScheduleForm(actor));

  const filteredProjects = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase("fr");
    if (!normalized) return projects;
    return projects.filter((project) =>
      `${project.code} ${project.customer_name}`.toLocaleLowerCase("fr").includes(normalized)
    );
  }, [projects, query]);

  useEffect(() => {
    let active = true;
    async function loadInitialProjects() {
      setLoadState("loading");
      setError(null);
      try {
        const [response, availableLocations] = await Promise.all([
          projectApi.listProjects(),
          projectApi.listLaboratoryLocations()
        ]);
        if (!active) return;
        setProjects(response.projects);
        setLocations(availableLocations);
        const preferredCode = response.projects.some(
          (project) => project.code === props.initialProjectCode
        )
          ? props.initialProjectCode ?? null
          : null;
        setSelectedCode(preferredCode ?? response.projects[0]?.code ?? null);
        setLoadState("ready");
      } catch (caught) {
        if (!active) return;
        setLoadState("error");
        setError(projectErrorMessage(caught));
      }
    }
    void loadInitialProjects();
    return () => {
      active = false;
    };
  }, [props.initialProjectCode]);

  useEffect(() => {
    if (!selectedCode) {
      setSelectedProject(null);
      setReview(null);
      setSchedule([]);
      setAudit([]);
      return;
    }
    void loadProject(selectedCode);
  }, [selectedCode]);

  async function refreshProjects(preferredCode?: string) {
    setLoadState("loading");
    setError(null);
    try {
      const [response, availableLocations] = await Promise.all([
        projectApi.listProjects(),
        projectApi.listLaboratoryLocations()
      ]);
      setProjects(response.projects);
      setLocations(availableLocations);
      const nextCode = preferredCode ?? selectedCode ?? response.projects[0]?.code ?? null;
      setSelectedCode(nextCode);
      setLoadState("ready");
      if (nextCode === selectedCode && nextCode) {
        await loadProject(nextCode);
      }
    } catch (caught) {
      setLoadState("error");
      setError(projectErrorMessage(caught));
    }
  }

  async function loadProject(projectCode: string) {
    setDetailLoading(true);
    setError(null);
    try {
      const [projectResponse, reviewResponse, scheduleResponse, auditResponse] = await Promise.all([
        projectApi.getProject(projectCode),
        projectApi.contractReview(projectCode),
        projectApi.listSchedule(projectCode),
        projectApi.auditEvents(projectCode)
      ]);
      setSelectedProject(projectResponse.project);
      setReview(reviewResponse.contract_review);
      setSchedule(scheduleResponse.schedule_items);
      setAudit(auditResponse.audit_events);
    } catch (caught) {
      setError(projectErrorMessage(caught));
    } finally {
      setDetailLoading(false);
    }
  }

  async function createProject() {
    setBusyAction("create-project");
    setError(null);
    try {
      const result = await projectApi.createProject({
        ...createForm,
        reason: "Ouverture du dossier d'essai"
      });
      setActor(createForm.actor.trim());
      setShowCreate(false);
      setCreateForm({
        code: generatedProjectCode(),
        customer_name: "",
        execution_mode: "non_accredited",
        actor: createForm.actor
      });
      await refreshProjects(result.project.code);
    } catch (caught) {
      setError(projectErrorMessage(caught));
    } finally {
      setBusyAction(null);
    }
  }

  async function completeReviewItem(item: string) {
    if (!selectedProject) return;
    setBusyAction(`review-${item}`);
    setError(null);
    try {
      await projectApi.completeReviewItem(
        selectedProject.code,
        item,
        actor,
        reviewComment.trim() || "Point vérifié depuis LAB CONSOLE"
      );
      setReviewComment("");
      await loadProject(selectedProject.code);
    } catch (caught) {
      setError(projectErrorMessage(caught));
    } finally {
      setBusyAction(null);
    }
  }

  async function advanceToPlanning() {
    if (!selectedProject) return;
    setBusyAction("advance-planning");
    setError(null);
    try {
      await projectApi.advanceToPlanning(
        selectedProject.code,
        actor,
        "Revue du besoin terminée"
      );
      await refreshProjects(selectedProject.code);
    } catch (caught) {
      setError(projectErrorMessage(caught));
    } finally {
      setBusyAction(null);
    }
  }

  async function createScheduleItem() {
    if (!selectedProject) return;
    const location = locations.find(
      (candidate) => candidate.laboratory_location_id === scheduleForm.laboratory_location_id
    );
    if (!location) {
      setError("Choisissez un poste de laboratoire prêt à câbler.");
      return;
    }
    setBusyAction("create-schedule");
    setError(null);
    try {
      const itemCode = generatedScheduleCode(selectedProject.code);
      await projectApi.createScheduleItem(selectedProject.code, {
        item_code: itemCode,
        title: scheduleForm.title,
        planned_start_at: `${scheduleForm.date}T${scheduleForm.start_time}`,
        planned_end_at: `${scheduleForm.date}T${scheduleForm.end_time}`,
        assigned_operator: scheduleForm.assigned_operator,
        laboratory_location_id: location.laboratory_location_id,
        laboratory_location_label: location.laboratory_location_label,
        equipment_under_test: scheduleForm.equipment_under_test,
        notes: scheduleForm.notes || undefined,
        actor,
        reason: "Réservation du créneau d'essai"
      });
      setShowSchedule(false);
      setScheduleForm(defaultScheduleForm(actor, locations[0]?.laboratory_location_id));
      await loadProject(selectedProject.code);
    } catch (caught) {
      setError(projectErrorMessage(caught));
    } finally {
      setBusyAction(null);
    }
  }

  async function transitionScheduleItem(
    item: ServiceScheduleItem,
    action: "confirm" | "start" | "complete" | "cancel"
  ) {
    if (!selectedProject) return;
    setBusyAction(`${action}-${item.item_code}`);
    setError(null);
    const reasons = {
      confirm: "Opérateur et lieu confirmés",
      start: "Essai démarré",
      complete: "Créneau d'essai terminé",
      cancel: "Créneau d'essai annulé"
    };
    try {
      await projectApi.transitionScheduleItem(
        selectedProject.code,
        item,
        action,
        actor,
        reasons[action]
      );
      await loadProject(selectedProject.code);
    } catch (caught) {
      setError(projectErrorMessage(caught));
    } finally {
      setBusyAction(null);
    }
  }

  function openScheduleForm() {
    setScheduleForm(defaultScheduleForm(actor, locations[0]?.laboratory_location_id));
    setShowSchedule(true);
  }

  return (
    <section className="projectWorkspace" aria-label="Dossiers d'essai">
      <header className="projectWorkspaceToolbar">
        <label className="searchBox projectSearch">
          <Search size={16} />
          <input
            aria-label="Rechercher un dossier"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Référence ou client"
          />
        </label>
        <button className="secondary iconButton" onClick={() => void refreshProjects()} title="Actualiser les dossiers" aria-label="Actualiser les dossiers">
          <RefreshCw size={16} />
        </button>
        <button onClick={() => setShowCreate(true)}>
          <CirclePlus size={17} /> Nouveau dossier
        </button>
      </header>

      {error && !showSchedule && (
        <div className="projectError" role="alert">
          <AlertCircle size={19} />
          <span>{error}</span>
          <button className="secondary iconButton" onClick={() => setError(null)} aria-label="Fermer le message">
            <X size={15} />
          </button>
        </div>
      )}

      <div className="projectWorkspaceLayout">
        <aside className="projectList" aria-label="Liste des dossiers">
          <div className="projectListHeading">
            <strong>Dossiers</strong>
            <span>{filteredProjects.length}</span>
          </div>
          {loadState === "loading" && <p className="projectListEmpty">Chargement des dossiers…</p>}
          {loadState === "error" && (
            <p className="projectListEmpty">Les dossiers ne sont pas disponibles.</p>
          )}
          {loadState === "ready" && filteredProjects.length === 0 && (
            <div className="projectListEmpty">
              <FolderKanban size={24} />
              <p>Aucun dossier d'essai.</p>
              <button onClick={() => setShowCreate(true)}>Créer le premier dossier</button>
            </div>
          )}
          {filteredProjects.map((project) => (
            <button
              key={project.code}
              className={`projectListItem${selectedCode === project.code ? " active" : ""}`}
              onClick={() => setSelectedCode(project.code)}
            >
              <span>
                <strong>{project.code}</strong>
                <small>{project.customer_name}</small>
              </span>
              <span className={`projectStage projectStage-${project.stage}`}>
                {stageLabels[project.stage]}
              </span>
              <ChevronRight size={16} aria-hidden="true" />
            </button>
          ))}
        </aside>

        <div className="projectDetail">
          {!selectedProject && !detailLoading && (
            <div className="projectWelcome">
              <FolderKanban size={38} />
              <h2>Préparer les essais à partir d'un dossier</h2>
              <p>
                Sélectionnez un dossier pour revoir le besoin, puis réserver son premier créneau
                de laboratoire.
              </p>
              <button onClick={() => setShowCreate(true)}>
                <CirclePlus size={17} /> Nouveau dossier
              </button>
            </div>
          )}
          {detailLoading && <div className="projectWelcome"><p>Ouverture du dossier…</p></div>}
          {selectedProject && review && !detailLoading && (
            <ProjectDetail
              project={selectedProject}
              review={review}
              schedule={schedule}
              audit={audit}
              actor={actor}
              reviewComment={reviewComment}
              busyAction={busyAction}
              onActorChange={setActor}
              onReviewCommentChange={setReviewComment}
              onCompleteReviewItem={(item) => void completeReviewItem(item)}
              onAdvanceToPlanning={() => void advanceToPlanning()}
              onOpenSchedule={openScheduleForm}
              onTransitionSchedule={(item, action) => void transitionScheduleItem(item, action)}
            />
          )}
        </div>
      </div>

      {showCreate && (
        <CreateProjectDialog
          form={createForm}
          busy={busyAction === "create-project"}
          onChange={setCreateForm}
          onClose={() => setShowCreate(false)}
          onSubmit={() => void createProject()}
        />
      )}
      {showSchedule && selectedProject && (
        <ScheduleDialog
          project={selectedProject}
          form={scheduleForm}
          locations={locations}
          error={error}
          busy={busyAction === "create-schedule"}
          onChange={setScheduleForm}
          onDismissError={() => setError(null)}
          onClose={() => setShowSchedule(false)}
          onSubmit={() => void createScheduleItem()}
        />
      )}
    </section>
  );
}

function ProjectDetail(props: {
  project: ProjectRecord;
  review: ContractReviewStatus;
  schedule: ServiceScheduleItem[];
  audit: ProjectAuditEvent[];
  actor: string;
  reviewComment: string;
  busyAction: string | null;
  onActorChange: (value: string) => void;
  onReviewCommentChange: (value: string) => void;
  onCompleteReviewItem: (item: string) => void;
  onAdvanceToPlanning: () => void;
  onOpenSchedule: () => void;
  onTransitionSchedule: (
    item: ServiceScheduleItem,
    action: "confirm" | "start" | "complete" | "cancel"
  ) => void;
}) {
  const completed = new Map(props.review.completed_items.map((item) => [item.item, item]));
  const firstOpenSchedule = props.schedule.find((item) => !["completed", "cancelled"].includes(item.status));
  const stageIndex = props.project.stage === "contract_review" ? 1 : props.project.stage === "quotation" ? 0 : 2;

  return (
    <>
      <header className="projectIdentity">
        <div>
          <p className="eyebrow">Dossier d'essai</p>
          <h2>{props.project.code}</h2>
          <p>{props.project.customer_name}</p>
        </div>
        <div className="projectIdentityMeta">
          <span className="projectMode">{modeLabels[props.project.execution_mode]}</span>
          <label>
            Action enregistrée au nom de
            <input
              aria-label="Responsable des actions"
              value={props.actor}
              onChange={(event) => props.onActorChange(event.target.value)}
            />
          </label>
        </div>
      </header>

      <ol className="projectStageRail" aria-label="Avancement du dossier">
        {["Demande reçue", "Revue du besoin", "Planification des essais"].map((label, index) => (
          <li key={label} className={index < stageIndex ? "done" : index === stageIndex ? "current" : ""}>
            <span>{index < stageIndex ? <Check size={15} /> : index + 1}</span>
            <strong>{label}</strong>
          </li>
        ))}
      </ol>

      <NextAction
        project={props.project}
        review={props.review}
        firstOpenSchedule={firstOpenSchedule}
        busyAction={props.busyAction}
        onAdvanceToPlanning={props.onAdvanceToPlanning}
        onOpenSchedule={props.onOpenSchedule}
        onTransitionSchedule={props.onTransitionSchedule}
      />

      <section
        className={`projectSection${
          props.review.complete && props.project.stage !== "contract_review" ? " reviewComplete" : ""
        }`}
        id="contract-review-section"
        aria-labelledby="contract-review-title"
      >
        <div className="projectSectionHeading">
          <div>
            <p className="eyebrow">Avant de réserver le laboratoire</p>
            <h3 id="contract-review-title">Revue du besoin</h3>
          </div>
          <strong>{props.review.required_items.length - props.review.missing_items.length} / {props.review.required_items.length}</strong>
        </div>
        <p className="sectionIntro">
          Vérifiez que la demande est réalisable et suffisamment définie pour ce cadre de
          prestation.
        </p>
        {props.project.stage === "contract_review" && !props.review.complete && (
          <label className="reviewComment">
            Commentaire pour le prochain point vérifié
            <input
              value={props.reviewComment}
              onChange={(event) => props.onReviewCommentChange(event.target.value)}
              placeholder="Facultatif"
            />
          </label>
        )}
        <ul className="reviewChecklist">
          {props.review.required_items.map((item) => {
            const evidence = completed.get(item);
            return (
              <li key={item} className={evidence ? "complete" : ""}>
                <label>
                  <input
                    type="checkbox"
                    checked={Boolean(evidence)}
                    disabled={
                      Boolean(evidence) ||
                      props.project.stage !== "contract_review" ||
                      props.busyAction !== null
                    }
                    onChange={(event) => {
                      if (event.target.checked) props.onCompleteReviewItem(item);
                    }}
                  />
                  <span>
                    <strong>{reviewLabels[item] ?? humanize(item)}</strong>
                    {evidence && (
                      <small>
                        Vérifié par {evidence.completed_by ?? "le responsable"}
                        {evidence.comment ? ` · ${evidence.comment}` : ""}
                      </small>
                    )}
                  </span>
                </label>
              </li>
            );
          })}
        </ul>
      </section>

      <section className="projectSection" id="planning-section" aria-labelledby="planning-title">
        <div className="projectSectionHeading">
          <div>
            <p className="eyebrow">Ressources du laboratoire</p>
            <h3 id="planning-title">Créneaux d'essai</h3>
          </div>
          {props.project.stage === "test_planning" && (
            <button onClick={props.onOpenSchedule}>
              <CirclePlus size={16} /> Planifier un essai
            </button>
          )}
        </div>
        {props.project.stage === "contract_review" && (
          <div className="planningEmpty blocked">
            <ClipboardCheck size={26} />
            <div>
              <strong>La revue doit être terminée avant de réserver un créneau.</strong>
              <p>Les points manquants sont indiqués juste au-dessus.</p>
            </div>
          </div>
        )}
        {props.project.stage === "test_planning" && props.schedule.length === 0 && (
          <div className="planningEmpty">
            <CalendarClock size={28} />
            <div>
              <strong>Aucun essai n'est encore planifié pour ce dossier.</strong>
              <p>Réservez l'opérateur et le lieu du premier essai.</p>
            </div>
            <button onClick={props.onOpenSchedule}>Planifier le premier essai</button>
          </div>
        )}
        {props.schedule.length > 0 && (
          <div className="scheduleList">
            {props.schedule.map((item) => (
              <ScheduleRow
                key={item.item_code}
                item={item}
                busyAction={props.busyAction}
                onTransition={(action) => props.onTransitionSchedule(item, action)}
              />
            ))}
          </div>
        )}
      </section>

      <details className="projectHistory">
        <summary>
          <History size={17} /> Historique du dossier <span>{props.audit.length}</span>
        </summary>
        {props.audit.length === 0 ? (
          <p>Aucune décision enregistrée.</p>
        ) : (
          <ol>
            {[...props.audit].reverse().map((event) => (
              <li key={event.sequence}>
                <span className="historyMarker" />
                <div>
                  <strong>{auditLabels[event.action] ?? humanize(event.action)}</strong>
                  <p>{event.reason || "Décision enregistrée"}</p>
                  <small>{formatAuditDate(event.occurred_at)} · {event.actor}</small>
                </div>
              </li>
            ))}
          </ol>
        )}
      </details>
    </>
  );
}

function NextAction(props: {
  project: ProjectRecord;
  review: ContractReviewStatus;
  firstOpenSchedule?: ServiceScheduleItem;
  busyAction: string | null;
  onAdvanceToPlanning: () => void;
  onOpenSchedule: () => void;
  onTransitionSchedule: (
    item: ServiceScheduleItem,
    action: "confirm" | "start" | "complete" | "cancel"
  ) => void;
}) {
  if (props.project.stage === "contract_review" && !props.review.complete) {
    const next = props.review.missing_items[0];
    return (
      <div className="nextAction attention">
        <ClipboardCheck size={23} />
        <div>
          <span>Prochaine action</span>
          <strong>Vérifier : {reviewLabels[next] ?? humanize(next)}</strong>
          <p>{props.review.missing_items.length} point(s) restent avant la planification.</p>
        </div>
        <button
          className="secondary"
          onClick={() => document.getElementById("contract-review-section")?.scrollIntoView({ behavior: "smooth" })}
        >
          Continuer la revue
        </button>
      </div>
    );
  }
  if (props.project.stage === "contract_review") {
    return (
      <div className="nextAction ready">
        <CheckCircle2 size={23} />
        <div>
          <span>Revue terminée</span>
          <strong>Le dossier peut passer en planification.</strong>
          <p>Cette décision sera enregistrée dans l'historique.</p>
        </div>
        <button disabled={props.busyAction !== null} onClick={props.onAdvanceToPlanning}>
          <CalendarClock size={17} /> Passer à la planification
        </button>
      </div>
    );
  }
  if (props.project.stage === "test_planning" && !props.firstOpenSchedule) {
    return (
      <div className="nextAction ready">
        <CalendarClock size={23} />
        <div>
          <span>Prochaine action</span>
          <strong>Réserver le premier créneau d'essai.</strong>
          <p>Choisissez un opérateur, un lieu et l'équipement à tester.</p>
        </div>
        <button onClick={props.onOpenSchedule}>
          <CirclePlus size={17} /> Planifier un essai
        </button>
      </div>
    );
  }
  if (props.firstOpenSchedule?.status === "planned") {
    return (
      <div className="nextAction attention">
        <Clock3 size={23} />
        <div>
          <span>Prochaine action</span>
          <strong>Confirmer le créneau du {formatScheduleDate(props.firstOpenSchedule)}.</strong>
          <p>{props.firstOpenSchedule.assigned_operator} · {props.firstOpenSchedule.laboratory_location_label}</p>
        </div>
        <button
          disabled={props.busyAction !== null}
          onClick={() => props.onTransitionSchedule(props.firstOpenSchedule!, "confirm")}
        >
          <Check size={17} /> Confirmer le créneau
        </button>
      </div>
    );
  }
  return (
    <div className="nextAction ready">
      <CheckCircle2 size={23} />
      <div>
        <span>Planning à jour</span>
        <strong>Le prochain créneau est {props.firstOpenSchedule ? scheduleStatusLabels[props.firstOpenSchedule.status].toLocaleLowerCase("fr") : "terminé"}.</strong>
        <p>Les actions disponibles sont présentées dans le planning.</p>
      </div>
    </div>
  );
}

function ScheduleRow(props: {
  item: ServiceScheduleItem;
  busyAction: string | null;
  onTransition: (action: "confirm" | "start" | "complete" | "cancel") => void;
}) {
  const positiveAction = schedulePositiveAction(props.item.status);
  const cancelled = props.item.status === "cancelled";
  return (
    <article className={`scheduleRow${cancelled ? " cancelled" : ""}`}>
      <time dateTime={props.item.planned_start_at}>
        <strong>{formatDay(props.item.planned_start_at)}</strong>
        <span>{formatTimeRange(props.item)}</span>
      </time>
      <div className="scheduleMain">
        <div className="scheduleTitle">
          <strong>{props.item.title}</strong>
          <span className={`scheduleStatus scheduleStatus-${props.item.status}`}>
            {scheduleStatusLabels[props.item.status]}
          </span>
        </div>
        <p>{props.item.equipment_under_test}</p>
        <div className="scheduleResources">
          <span><UserRound size={14} /> {props.item.assigned_operator}</span>
          <span><MapPin size={14} /> {props.item.laboratory_location_label}</span>
        </div>
      </div>
      <div className="scheduleActions">
        {positiveAction && props.item.available_transitions.includes(positiveAction.target) && (
          <button
            disabled={props.busyAction !== null}
            onClick={() => props.onTransition(positiveAction.action)}
          >
            <Check size={15} />
            {positiveAction.label}
          </button>
        )}
        {props.item.status === "confirmed" && (
          <small>Préparez l'essai depuis le planning du laboratoire.</small>
        )}
        {props.item.available_transitions.includes("cancelled") && (
          <button
            className="secondary"
            disabled={props.busyAction !== null}
            onClick={() => props.onTransition("cancel")}
          >
            <XCircle size={15} /> Annuler
          </button>
        )}
      </div>
    </article>
  );
}

function CreateProjectDialog(props: {
  form: CreateProjectForm;
  busy: boolean;
  onChange: (value: CreateProjectForm) => void;
  onClose: () => void;
  onSubmit: () => void;
}) {
  return (
    <div className="modalBackdrop" role="presentation">
      <section className="wizardPanel projectDialog" role="dialog" aria-modal="true" aria-labelledby="create-project-title">
        <header className="creationHeader">
          <div>
            <p className="eyebrow">Locus Lab Management</p>
            <h2 id="create-project-title">Nouveau dossier d'essai</h2>
          </div>
          <button className="secondary iconButton" onClick={props.onClose} aria-label="Fermer">
            <X size={17} />
          </button>
        </header>
        <div className="wizardBody projectDialogBody">
          <p className="sectionIntro">
            Ouvrez le dossier qui portera la revue du besoin et la planification des essais.
          </p>
          <label>
            Référence du dossier
            <input
              value={props.form.code}
              onChange={(event) => props.onChange({ ...props.form, code: event.target.value })}
              required
            />
          </label>
          <label>
            Client
            <input
              autoFocus
              value={props.form.customer_name}
              onChange={(event) => props.onChange({ ...props.form, customer_name: event.target.value })}
              placeholder="Nom du client ou de l'entité demandeuse"
              required
            />
          </label>
          <fieldset className="modeChoice">
            <legend>Cadre de réalisation</legend>
            {([
              ["accredited", "Accrédité", "Revue complète et exigences qualité renforcées"],
              ["non_accredited", "Hors accréditation", "Exigences adaptées à la prestation"],
              ["investigation", "Investigation", "Objectif exploratoire et contraintes desserrées"]
            ] as const).map(([value, label, help]) => (
              <label key={value} className={props.form.execution_mode === value ? "selected" : ""}>
                <input
                  type="radio"
                  name="execution-mode"
                  value={value}
                  checked={props.form.execution_mode === value}
                  onChange={() => props.onChange({ ...props.form, execution_mode: value })}
                />
                <span><strong>{label}</strong><small>{help}</small></span>
              </label>
            ))}
          </fieldset>
          <label>
            Responsable du dossier
            <input
              value={props.form.actor}
              onChange={(event) => props.onChange({ ...props.form, actor: event.target.value })}
              required
            />
          </label>
        </div>
        <footer className="wizardFooter">
          <button className="secondary" onClick={props.onClose}>Annuler</button>
          <button
            disabled={
              props.busy ||
              !props.form.code.trim() ||
              !props.form.customer_name.trim() ||
              !props.form.actor.trim()
            }
            onClick={props.onSubmit}
          >
            <FolderKanban size={17} /> Ouvrir le dossier
          </button>
        </footer>
      </section>
    </div>
  );
}

function ScheduleDialog(props: {
  project: ProjectRecord;
  form: ScheduleForm;
  locations: LaboratoryLocationOption[];
  error: string | null;
  busy: boolean;
  onChange: (value: ScheduleForm) => void;
  onDismissError: () => void;
  onClose: () => void;
  onSubmit: () => void;
}) {
  const valid =
    props.form.title.trim() &&
    props.form.date &&
    props.form.start_time &&
    props.form.end_time &&
    props.form.assigned_operator.trim() &&
    props.form.laboratory_location_id &&
    props.form.equipment_under_test.trim();
  return (
    <div className="modalBackdrop" role="presentation">
      <section className="wizardPanel projectDialog" role="dialog" aria-modal="true" aria-labelledby="schedule-title">
        <header className="creationHeader">
          <div>
            <p className="eyebrow">{props.project.code} · {props.project.customer_name}</p>
            <h2 id="schedule-title">Planifier un essai</h2>
          </div>
          <button className="secondary iconButton" onClick={props.onClose} aria-label="Fermer">
            <X size={17} />
          </button>
        </header>
        {props.error && (
          <div className="projectDialogError" role="alert">
            <AlertCircle size={18} />
            <span>{props.error}</span>
            <button
              className="secondary iconButton"
              onClick={props.onDismissError}
              aria-label="Fermer le message"
            >
              <X size={15} />
            </button>
          </div>
        )}
        <div className="wizardBody projectDialogBody">
          <p className="sectionIntro">
            Ce créneau réservera l'opérateur et le lieu jusqu'à sa fin ou son annulation.
          </p>
          <label>
            Essai prévu
            <input
              autoFocus
              value={props.form.title}
              onChange={(event) => props.onChange({ ...props.form, title: event.target.value })}
              placeholder="Ex. Émission conduite"
            />
          </label>
          <div className="scheduleDateGrid">
            <label>
              Date
              <input
                type="date"
                value={props.form.date}
                onChange={(event) => props.onChange({ ...props.form, date: event.target.value })}
              />
            </label>
            <label>
              Début
              <input
                type="time"
                value={props.form.start_time}
                onChange={(event) => props.onChange({ ...props.form, start_time: event.target.value })}
              />
            </label>
            <label>
              Fin
              <input
                type="time"
                value={props.form.end_time}
                onChange={(event) => props.onChange({ ...props.form, end_time: event.target.value })}
              />
            </label>
          </div>
          <label>
            Opérateur
            <input
              value={props.form.assigned_operator}
              onChange={(event) => props.onChange({ ...props.form, assigned_operator: event.target.value })}
              placeholder="Nom de l'opérateur"
            />
          </label>
          <label>
            Lieu
            <select
              value={props.form.laboratory_location_id}
              onChange={(event) =>
                props.onChange({ ...props.form, laboratory_location_id: event.target.value })
              }
            >
              <option value="">Sélectionner un poste prêt à câbler</option>
              {props.locations.map((location) => (
                <option
                  key={location.laboratory_location_id}
                  value={location.laboratory_location_id}
                >
                  {location.laboratory_location_label}
                </option>
              ))}
            </select>
            {props.locations.length === 0 && (
              <small>Créez et validez d'abord un montage dans Test Station.</small>
            )}
          </label>
          <label>
            Équipement à tester
            <input
              value={props.form.equipment_under_test}
              onChange={(event) => props.onChange({ ...props.form, equipment_under_test: event.target.value })}
              placeholder="Produit, prototype ou sous-ensemble"
            />
          </label>
          <label>
            Note
            <textarea
              value={props.form.notes}
              onChange={(event) => props.onChange({ ...props.form, notes: event.target.value })}
              placeholder="Préparation ou contrainte particulière"
            />
          </label>
        </div>
        <footer className="wizardFooter">
          <button className="secondary" onClick={props.onClose}>Annuler</button>
          <button disabled={props.busy || !valid} onClick={props.onSubmit}>
            <CalendarClock size={17} /> Réserver le créneau
          </button>
        </footer>
      </section>
    </div>
  );
}

function schedulePositiveAction(status: ServiceScheduleStatus): {
  target: ServiceScheduleStatus;
  action: "confirm" | "complete";
  label: string;
} | null {
  if (status === "planned") return { target: "confirmed", action: "confirm", label: "Confirmer" };
  if (status === "in_progress") return { target: "completed", action: "complete", label: "Terminer" };
  return null;
}

function defaultScheduleForm(actor: string, locationId = ""): ScheduleForm {
  return {
    title: "",
    date: nextBusinessDate(),
    start_time: "09:00",
    end_time: "12:00",
    assigned_operator: actor,
    laboratory_location_id: locationId,
    equipment_under_test: "",
    notes: ""
  };
}

function generatedProjectCode(): string {
  return `CEM-${new Date().getFullYear()}-${crypto.randomUUID().slice(0, 4).toUpperCase()}`;
}

function generatedScheduleCode(projectCode: string): string {
  return `PLAN-${projectCode}-${crypto.randomUUID().slice(0, 6).toUpperCase()}`;
}

function nextBusinessDate(): string {
  const date = new Date();
  while (date.getDay() === 0 || date.getDay() === 6) {
    date.setDate(date.getDate() + 1);
  }
  return [
    date.getFullYear(),
    String(date.getMonth() + 1).padStart(2, "0"),
    String(date.getDate()).padStart(2, "0")
  ].join("-");
}

function projectErrorMessage(caught: unknown): string {
  if (!(caught instanceof ApiError)) {
    return caught instanceof Error ? caught.message : "Une erreur inattendue est survenue.";
  }
  const conflict = caught.details?.conflicting_item as Record<string, unknown> | undefined;
  if (caught.code === "service_schedule_operator_conflict" && conflict) {
    return `${String(conflict.assigned_operator)} est déjà affecté au créneau « ${String(conflict.title)} » le ${formatConflictSchedule(conflict)}.`;
  }
  if (caught.code === "service_schedule_location_conflict" && conflict) {
    return `${String(conflict.laboratory_location_label)} est déjà réservé pour « ${String(conflict.title)} » le ${formatConflictSchedule(conflict)}.`;
  }
  if (caught.code === "service_schedule_legacy_location_identity_required" && conflict) {
    return `Un créneau existant utilise encore un lieu non identifié. Identifiez son lieu avant de réserver un autre créneau sur cette période. Créneau concerné : « ${String(conflict.title)} » du dossier ${String(conflict.project_code)}, le ${formatConflictSchedule(conflict)}, libellé historique « ${String(conflict.laboratory_location_label)} », état ${scheduleStatusLabel(String(conflict.status))}.`;
  }
  const messages: Record<string, string> = {
    contract_review_incomplete: "La revue du besoin n'est pas encore terminée.",
    project_not_ready_for_scheduling: "Terminez la revue du besoin avant de planifier un essai.",
    service_schedule_concurrent_update: "Le planning a changé. Le dossier a été actualisé ; recommencez l'action.",
    invalid_service_schedule_transition: "Cette action n'est plus disponible pour ce créneau.",
    invalid_service_schedule_request: "Le créneau n'est pas valide. Vérifiez la date, les horaires et les informations demandées.",
    local_agent_unavailable: "L'agent local ne répond pas. Vérifiez qu'il est démarré."
  };
  return messages[caught.code] ?? caught.message;
}

function scheduleStatusLabel(status: string): string {
  return {
    planned: "Prévu",
    confirmed: "Confirmé",
    in_progress: "En cours",
    completed: "Terminé",
    cancelled: "Annulé"
  }[status] ?? status;
}

function formatConflictSchedule(conflict: Record<string, unknown>): string {
  const start = String(conflict.planned_start_at);
  const end = String(conflict.planned_end_at);
  return `${formatDay(start)}, ${start.slice(11, 16)}–${end.slice(11, 16)}`;
}

function humanize(value: string | undefined): string {
  if (!value) return "Étape à vérifier";
  const text = value.replaceAll("_", " ");
  return text.charAt(0).toLocaleUpperCase("fr") + text.slice(1);
}

function formatDay(value: string): string {
  const [year, month, day] = value.slice(0, 10).split("-").map(Number);
  return new Intl.DateTimeFormat("fr-FR", { weekday: "short", day: "2-digit", month: "short" }).format(
    new Date(year, month - 1, day)
  );
}

function formatTimeRange(item: ServiceScheduleItem): string {
  return `${item.planned_start_at.slice(11)} – ${item.planned_end_at.slice(11)}`;
}

function formatScheduleDate(item: ServiceScheduleItem): string {
  return `${formatDay(item.planned_start_at)} · ${formatTimeRange(item)}`;
}

function formatAuditDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("fr-FR", {
    dateStyle: "medium",
    timeStyle: "short"
  }).format(date);
}
