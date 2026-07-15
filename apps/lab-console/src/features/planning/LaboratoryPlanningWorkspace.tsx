import {
  AlertCircle,
  CalendarDays,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  ClipboardCheck,
  Clock3,
  FolderKanban,
  History,
  MapPin,
  PencilLine,
  Play,
  RefreshCw,
  ShieldAlert,
  UserRound,
  Wrench,
  X
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { ApiError, projectApi } from "../../api";
import type {
  LaboratoryScheduleItem,
  LaboratoryWeekSchedule,
  PlannedTestMethodSnapshot,
  PlannedTestPreparationAggregate,
  PlannedTestPreparationIssue,
  PlannedTestPreparationOptions,
  PlannedTestPreparationRevision,
  PlannedStationSetupSnapshot,
  ServiceScheduleStatus
} from "../../models/projects";

const statusLabels: Record<ServiceScheduleStatus, string> = {
  planned: "Prévu",
  confirmed: "Confirmé",
  in_progress: "En cours",
  completed: "Terminé",
  cancelled: "Annulé"
};

const weekdayLabels = ["Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi"];

interface RescheduleForm {
  date: string;
  start_time: string;
  end_time: string;
  assigned_operator: string;
  location: string;
  reason: string;
}

export function LaboratoryPlanningWorkspace(props: {
  onOpenProject: (projectCode: string) => void;
}) {
  const [weekStart, setWeekStart] = useState(() => mondayFor(new Date()));
  const [schedule, setSchedule] = useState<LaboratoryWeekSchedule | null>(null);
  const [selectedItem, setSelectedItem] = useState<LaboratoryScheduleItem | null>(null);
  const [operatorFilter, setOperatorFilter] = useState("all");
  const [locationFilter, setLocationFilter] = useState("all");
  const [statusFilter, setStatusFilter] = useState<"all" | ServiceScheduleStatus>("all");
  const [loadState, setLoadState] = useState<"loading" | "ready" | "error">("loading");
  const [error, setError] = useState<string | null>(null);

  const loadWeek = useCallback(
    async (preserveSelected = false, silent = false) => {
      if (!silent) setLoadState("loading");
      setError(null);
      try {
        const response = await projectApi.listLaboratoryWeek(weekStart);
        setSchedule(response);
        setLoadState("ready");
        if (!preserveSelected) {
          setSelectedItem((current) =>
            current
              ? response.schedule_items.find((item) => item.item_code === current.item_code) ?? null
              : null
          );
        }
      } catch (caught) {
        setLoadState("error");
        setError(planningErrorMessage(caught));
      }
    },
    [weekStart]
  );

  useEffect(() => {
    void loadWeek();
  }, [loadWeek]);

  const operators = useMemo(
    () => uniqueSorted(schedule?.schedule_items.map((item) => item.assigned_operator) ?? []),
    [schedule]
  );
  const locations = useMemo(
    () => uniqueSorted(schedule?.schedule_items.map((item) => item.location) ?? []),
    [schedule]
  );
  const filteredItems = useMemo(
    () =>
      (schedule?.schedule_items ?? []).filter(
        (item) =>
          (operatorFilter === "all" || item.assigned_operator === operatorFilter) &&
          (locationFilter === "all" || item.location === locationFilter) &&
          (statusFilter === "all" || item.status === statusFilter)
      ),
    [locationFilter, operatorFilter, schedule, statusFilter]
  );
  const days = useMemo(() => buildWeekDays(schedule?.week_start ?? weekStart), [schedule, weekStart]);
  const filtersActive =
    operatorFilter !== "all" || locationFilter !== "all" || statusFilter !== "all";

  function changeWeek(offset: number) {
    setSelectedItem(null);
    setWeekStart(addDays(weekStart, offset * 7));
  }

  function reachWeek(date: string) {
    if (!date) return;
    setSelectedItem(null);
    setWeekStart(mondayFor(parseIsoDate(date)));
  }

  function clearFilters() {
    setOperatorFilter("all");
    setLocationFilter("all");
    setStatusFilter("all");
  }

  function updateMovedItem(item: LaboratoryScheduleItem) {
    setSchedule((current) =>
      current
        ? {
            ...current,
            schedule_items: current.schedule_items
              .map((candidate) => (candidate.item_code === item.item_code ? item : candidate))
              .sort(compareScheduleItems)
          }
        : current
    );
    setSelectedItem(item);
  }

  return (
    <section className="laboratoryPlanningWorkspace">
      <div className="planningWeekToolbar">
        <div className="weekNavigation" aria-label="Navigation entre les semaines">
          <button
            className="secondary iconButton"
            onClick={() => changeWeek(-1)}
            aria-label="Semaine précédente"
            title="Semaine précédente"
          >
            <ChevronLeft size={17} />
          </button>
          <div className="weekTitle">
            <strong>{formatWeekRange(schedule?.week_start ?? weekStart)}</strong>
            <span>{filteredItems.length} créneau{filteredItems.length === 1 ? "" : "x"}</span>
          </div>
          <button
            className="secondary iconButton"
            onClick={() => changeWeek(1)}
            aria-label="Semaine suivante"
            title="Semaine suivante"
          >
            <ChevronRight size={17} />
          </button>
          <button className="secondary" onClick={() => setWeekStart(mondayFor(new Date()))}>
            <CalendarDays size={16} /> Cette semaine
          </button>
        </div>
        <label className="weekDatePicker">
          Atteindre une date
          <input type="date" value={weekStart} onChange={(event) => reachWeek(event.target.value)} />
        </label>
        <button
          className="secondary iconButton"
          onClick={() => void loadWeek()}
          aria-label="Actualiser le planning"
          title="Actualiser le planning"
        >
          <RefreshCw size={16} />
        </button>
      </div>

      <div className="planningFilters" aria-label="Filtres du planning">
        <label>
          Opérateur
          <select value={operatorFilter} onChange={(event) => setOperatorFilter(event.target.value)}>
            <option value="all">Tous les opérateurs</option>
            {operators.map((operator) => <option key={operator}>{operator}</option>)}
          </select>
        </label>
        <label>
          Lieu
          <select value={locationFilter} onChange={(event) => setLocationFilter(event.target.value)}>
            <option value="all">Tous les lieux</option>
            {locations.map((location) => <option key={location}>{location}</option>)}
          </select>
        </label>
        <label>
          État
          <select
            value={statusFilter}
            onChange={(event) => setStatusFilter(event.target.value as "all" | ServiceScheduleStatus)}
          >
            <option value="all">Tous les états</option>
            {(Object.entries(statusLabels) as Array<[ServiceScheduleStatus, string]>).map(
              ([value, label]) => <option key={value} value={value}>{label}</option>
            )}
          </select>
        </label>
        {filtersActive && (
          <button className="secondary" onClick={clearFilters}>
            <X size={15} /> Effacer les filtres
          </button>
        )}
      </div>

      {error && (
        <div className="planningPageError" role="alert">
          <AlertCircle size={19} />
          <span>{error}</span>
          <button className="secondary" onClick={() => void loadWeek()}>Réessayer</button>
        </div>
      )}

      {loadState === "loading" && !schedule && (
        <div className="planningLoading"><CalendarDays size={30} /><p>Chargement du planning…</p></div>
      )}

      {schedule && schedule.schedule_items.length > 0 && (
        <div className="laboratoryWeek" aria-label={`Planning du ${schedule.week_start} au ${schedule.week_end}`}>
          {days.map((day, index) => {
            const items = filteredItems.filter((item) => item.planned_start_at.startsWith(day));
            return (
              <section className="planningDay" key={day} aria-labelledby={`planning-day-${day}`}>
                <header>
                  <span>{weekdayLabels[index]}</span>
                  <strong id={`planning-day-${day}`}>{formatShortDate(day)}</strong>
                  <small>{items.length || "—"}</small>
                </header>
                <div className="planningDaySlots">
                  {items.map((item) => (
                    <button
                      key={item.item_code}
                      className={`planningSlot planningSlot-${item.status}`}
                      onClick={() => setSelectedItem(item)}
                      aria-label={`Ouvrir ${item.title}, dossier ${item.project_code}`}
                    >
                      <span className="planningSlotTime">{formatTimeRange(item)}</span>
                      <span className="planningSlotTitle">{item.title}</span>
                      <span className="planningSlotProject">{item.project_code} · {item.customer_name}</span>
                      <span><UserRound size={13} /> {item.assigned_operator}</span>
                      <span><MapPin size={13} /> {item.location}</span>
                      <span className={`scheduleStatus scheduleStatus-${item.status}`}>
                        {statusLabels[item.status]}
                      </span>
                    </button>
                  ))}
                  {items.length === 0 && (
                    <p className="planningDayEmpty">
                      {filtersActive ? "Aucun résultat" : "Aucun essai prévu"}
                    </p>
                  )}
                </div>
              </section>
            );
          })}
        </div>
      )}

      {schedule && schedule.schedule_items.length === 0 && loadState === "ready" && (
        <div className="planningEmptyWeek">
          <CalendarDays size={30} />
          <div>
            <strong>Aucun essai planifié cette semaine</strong>
            <p>Les créneaux réservés depuis les dossiers apparaîtront ici.</p>
          </div>
          <button className="secondary" onClick={() => props.onOpenProject("")}>
            <FolderKanban size={16} /> Ouvrir les dossiers
          </button>
        </div>
      )}

      {selectedItem && (
        <ScheduleDetailDialog
          key={`${selectedItem.item_code}-${selectedItem.revision}`}
          item={selectedItem}
          onClose={() => setSelectedItem(null)}
          onOpenProject={() => props.onOpenProject(selectedItem.project_code)}
          onMoved={updateMovedItem}
          onConcurrentRefresh={() => loadWeek(true, true)}
        />
      )}
    </section>
  );
}

function ScheduleDetailDialog(props: {
  item: LaboratoryScheduleItem;
  onClose: () => void;
  onOpenProject: () => void;
  onMoved: (item: LaboratoryScheduleItem) => void;
  onConcurrentRefresh: () => Promise<void>;
}) {
  const [mode, setMode] = useState<"details" | "move" | "prepare">("details");
  const [form, setForm] = useState<RescheduleForm>(() => rescheduleForm(props.item));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [preparation, setPreparation] = useState<PlannedTestPreparationAggregate | null>(null);
  const [preparationLoading, setPreparationLoading] = useState(true);
  const valid =
    form.date &&
    form.start_time &&
    form.end_time &&
    form.assigned_operator.trim() &&
    form.location.trim() &&
    form.reason.trim();

  const loadPreparation = useCallback(async () => {
    setPreparationLoading(true);
    try {
      const response = await projectApi.getPlannedTestPreparation(
        props.item.project_code,
        props.item.item_code
      );
      setPreparation(response.preparation);
    } catch (caught) {
      setError(planningErrorMessage(caught));
    } finally {
      setPreparationLoading(false);
    }
  }, [props.item.item_code, props.item.project_code]);

  useEffect(() => {
    void loadPreparation();
  }, [loadPreparation]);

  async function submit() {
    setBusy(true);
    setError(null);
    try {
      const result = await projectApi.rescheduleItem(props.item, {
        planned_start_at: `${form.date}T${form.start_time}`,
        planned_end_at: `${form.date}T${form.end_time}`,
        assigned_operator: form.assigned_operator,
        location: form.location,
        actor: "Responsable laboratoire",
        reason: form.reason
      });
      props.onMoved({
        ...props.item,
        ...result.schedule_item,
        customer_name: props.item.customer_name,
        project_stage: props.item.project_stage
      });
      setMode("details");
      setPreparation((current) =>
        current
          ? { ...current, current_state: "stale", can_start: false }
          : current
      );
    } catch (caught) {
      setError(planningErrorMessage(caught));
      if (caught instanceof ApiError && caught.code === "service_schedule_concurrent_update") {
        await props.onConcurrentRefresh();
      }
    } finally {
      setBusy(false);
    }
  }

  async function transition(action: "confirm" | "start") {
    setBusy(true);
    setError(null);
    try {
      const result = await projectApi.transitionScheduleItem(
        props.item.project_code,
        props.item,
        action,
        "Opérateur CEM",
        action === "confirm"
          ? "Créneau et ressources confirmés"
          : "Préparation vérifiée avant démarrage"
      );
      props.onMoved({
        ...props.item,
        ...result.schedule_item,
        customer_name: props.item.customer_name,
        project_stage: props.item.project_stage
      });
      if (action === "confirm") {
        setPreparation((current) =>
          current ? { ...current, current_state: "stale", can_start: false } : current
        );
      }
    } catch (caught) {
      setError(planningErrorMessage(caught));
      if (caught instanceof ApiError && caught.code === "service_schedule_concurrent_update") {
        await props.onConcurrentRefresh();
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="modalBackdrop" role="presentation">
      <section
        className="wizardPanel planningDetailDialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="planning-detail-title"
      >
        <header className="creationHeader">
          <div>
            <p className="eyebrow">{props.item.project_code} · {props.item.customer_name}</p>
            <h2 id="planning-detail-title">
              {mode === "move"
                ? "Déplacer le créneau"
                : mode === "prepare"
                  ? "Préparer l'essai"
                  : props.item.title}
            </h2>
          </div>
          <button className="secondary iconButton" onClick={props.onClose} aria-label="Fermer">
            <X size={17} />
          </button>
        </header>

        {error && (
          <div className="projectDialogError" role="alert">
            <AlertCircle size={18} />
            <span>{error}</span>
            <button
              className="secondary iconButton"
              onClick={() => setError(null)}
              aria-label="Fermer le message"
            >
              <X size={15} />
            </button>
          </div>
        )}

        {mode === "details" ? (
          <div className="wizardBody planningDetailBody">
            <div className="planningDetailStatus">
              <span className={`scheduleStatus scheduleStatus-${props.item.status}`}>
                {statusLabels[props.item.status]}
              </span>
              {matchesPreparationStatus(props.item.status) && !preparationLoading && (
                <PreparationStateBadge state={preparation?.current_state ?? "missing"} />
              )}
              {!props.item.can_reschedule && <small>Ce créneau ne peut plus être déplacé.</small>}
            </div>
            <dl className="planningDetailFacts">
              <div><dt><Clock3 size={15} /> Horaire</dt><dd>{formatFullDateTime(props.item)}</dd></div>
              <div><dt><UserRound size={15} /> Opérateur</dt><dd>{props.item.assigned_operator}</dd></div>
              <div><dt><MapPin size={15} /> Lieu</dt><dd>{props.item.location}</dd></div>
              <div><dt>Équipement à tester</dt><dd>{props.item.equipment_under_test}</dd></div>
            </dl>
            {props.item.notes && <p className="planningDetailNote">{props.item.notes}</p>}
            {matchesPreparationStatus(props.item.status) && !preparationLoading && (
              <PreparationSummary preparation={preparation} />
            )}
          </div>
        ) : mode === "move" ? (
          <div className="wizardBody projectDialogBody">
            <p className="sectionIntro">
              L'essai et l'équipement restent rattachés au même dossier. Le nouvel horaire sera
              contrôlé avant enregistrement.
            </p>
            <div className="scheduleDateGrid">
              <label>
                Date
                <input
                  type="date"
                  value={form.date}
                  onChange={(event) => setForm({ ...form, date: event.target.value })}
                />
              </label>
              <label>
                Début
                <input
                  type="time"
                  value={form.start_time}
                  onChange={(event) => setForm({ ...form, start_time: event.target.value })}
                />
              </label>
              <label>
                Fin
                <input
                  type="time"
                  value={form.end_time}
                  onChange={(event) => setForm({ ...form, end_time: event.target.value })}
                />
              </label>
            </div>
            <label>
              Opérateur
              <input
                value={form.assigned_operator}
                onChange={(event) => setForm({ ...form, assigned_operator: event.target.value })}
              />
            </label>
            <label>
              Lieu
              <input
                value={form.location}
                onChange={(event) => setForm({ ...form, location: event.target.value })}
              />
            </label>
            <label>
              Raison du changement
              <textarea
                value={form.reason}
                onChange={(event) => setForm({ ...form, reason: event.target.value })}
                placeholder="Ex. disponibilité du laboratoire confirmée avec le client"
                autoFocus
              />
            </label>
          </div>
        ) : (
          <PreparationWorkspace
            item={props.item}
            preparation={preparation}
            onPreparation={setPreparation}
            onError={setError}
          />
        )}

        <footer className="wizardFooter planningDetailFooter">
          {mode === "details" ? (
            <>
              <button className="secondary" onClick={props.onOpenProject}>
                <FolderKanban size={16} /> Ouvrir le dossier
              </button>
              {props.item.can_reschedule && (
                <button className="secondary" onClick={() => setMode("move")}>
                  <PencilLine size={16} /> Déplacer
                </button>
              )}
              {matchesPreparationStatus(props.item.status) && (
                <button className="secondary" onClick={() => setMode("prepare")}>
                  <ClipboardCheck size={16} /> Préparer l'essai
                </button>
              )}
              {props.item.status === "planned" && (
                <button disabled={busy} onClick={() => void transition("confirm")}>
                  <CheckCircle2 size={16} /> Confirmer
                </button>
              )}
              {props.item.status === "confirmed" && (
                <button
                  disabled={busy || !preparation?.can_start}
                  onClick={() => void transition("start")}
                  title={
                    preparation?.can_start
                      ? "Démarrer l'essai"
                      : "La préparation doit être prête avant le démarrage"
                  }
                >
                  <Play size={16} /> Démarrer l'essai
                </button>
              )}
            </>
          ) : mode === "move" ? (
            <>
              <button className="secondary" onClick={() => { setMode("details"); setError(null); }}>
                Annuler
              </button>
              <button disabled={busy || !valid} onClick={() => void submit()}>
                <CalendarDays size={16} /> Enregistrer le déplacement
              </button>
            </>
          ) : (
            <button className="secondary" onClick={() => setMode("details")}>
              Retour au créneau
            </button>
          )}
        </footer>
      </section>
    </div>
  );
}

function PreparationSummary(props: {
  preparation: PlannedTestPreparationAggregate | null;
}) {
  const current = props.preparation?.current_revision;
  const state = props.preparation?.current_state ?? "missing";
  const blocking = current?.definition.verdict.issues.filter(
    (issue) => issue.severity === "blocking"
  ).length ?? 0;
  return (
    <section className={`preparationSummary preparationSummary-${state}`}>
      <div className="preparationSummaryIcon">
        {state === "ready" ? <CheckCircle2 size={22} /> : <ShieldAlert size={22} />}
      </div>
      <div>
        <strong>{preparationStateTitle(state)}</strong>
        <p>
          {state === "ready"
            ? `${current?.definition.method.title} avec ${current?.definition.station_setup.label}`
            : state === "blocked"
              ? `${blocking} point${blocking === 1 ? "" : "s"} à corriger avant le démarrage.`
              : state === "stale"
                ? "Le créneau a changé depuis le dernier contrôle. Une nouvelle vérification est requise."
                : "Choisissez la méthode, le montage et les matériels avant de lancer l'essai."}
        </p>
      </div>
    </section>
  );
}

function PreparationWorkspace(props: {
  item: LaboratoryScheduleItem;
  preparation: PlannedTestPreparationAggregate | null;
  onPreparation: (preparation: PlannedTestPreparationAggregate) => void;
  onError: (message: string | null) => void;
}) {
  const projectCode = props.item.project_code;
  const itemCode = props.item.item_code;
  const reportError = props.onError;
  const [options, setOptions] = useState<PlannedTestPreparationOptions | null>(null);
  const [history, setHistory] = useState<PlannedTestPreparationRevision[]>([]);
  const [methodRevisionId, setMethodRevisionId] = useState(
    props.preparation?.current_revision?.definition.method.revision_id ?? ""
  );
  const [setupRevisionId, setSetupRevisionId] = useState(
    props.preparation?.current_revision?.definition.station_setup.revision_id ?? ""
  );
  const [assignments, setAssignments] = useState<Record<string, string>>(() =>
    Object.fromEntries(
      props.preparation?.current_revision?.definition.assignments.map((assignment) => [
        assignment.slot_id,
        assignment.binding_id
      ]) ?? []
    )
  );
  const [reason, setReason] = useState("Vérification avant essai");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    reportError(null);
    try {
      const [available, revisions] = await Promise.all([
        projectApi.plannedTestPreparationOptions(projectCode, itemCode),
        projectApi.plannedTestPreparationRevisions(projectCode, itemCode)
      ]);
      setOptions(available);
      setHistory(revisions.revisions);
      setMethodRevisionId((current) => current || available.methods[0]?.revision_id || "");
      setSetupRevisionId(
        (current) => current || available.station_setups[0]?.station_setup.revision_id || ""
      );
    } catch (caught) {
      reportError(planningErrorMessage(caught));
    } finally {
      setLoading(false);
    }
  }, [itemCode, projectCode, reportError]);

  useEffect(() => {
    void load();
  }, [load]);

  const method = options?.methods.find((candidate) => candidate.revision_id === methodRevisionId);
  const stationOption = options?.station_setups.find(
    (candidate) => candidate.station_setup.revision_id === setupRevisionId
  );

  async function assess() {
    if (!method || !stationOption) return;
    setBusy(true);
    props.onError(null);
    try {
      const result = await projectApi.assessPlannedTestPreparation(props.item, {
        expected_current_revision_id: props.preparation?.current_revision?.revision_id ?? null,
        method_template_id: method.template_id,
        method_revision_id: method.revision_id,
        station_setup_id: stationOption.station_setup.setup_id,
        station_setup_revision_id: stationOption.station_setup.revision_id,
        assignments: method.instrumentation_chain
          .map((slot) => ({ slot_id: slot.slot_id, binding_id: assignments[slot.slot_id] ?? "" }))
          .filter((assignment) => assignment.binding_id),
        actor: "Opérateur CEM",
        reason
      });
      props.onPreparation(result.preparation);
      const revisions = await projectApi.plannedTestPreparationRevisions(
        props.item.project_code,
        props.item.item_code
      );
      setHistory(revisions.revisions);
    } catch (caught) {
      props.onError(planningErrorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  if (loading) {
    return <div className="wizardBody preparationLoading"><RefreshCw size={20} /> Chargement de la préparation…</div>;
  }
  if (!options || options.methods.length === 0 || options.station_setups.length === 0) {
    return (
      <div className="wizardBody preparationEmpty">
        <ShieldAlert size={24} />
        <div>
          <strong>Préparation impossible pour le moment</strong>
          <p>
            Il faut au moins une méthode approuvée et un montage marqué « Prêt à câbler ».
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="wizardBody preparationWorkspace">
      <PreparationVerdict preparation={props.preparation} />

      <section className="preparationSection">
        <div className="preparationSectionTitle">
          <span>1</span><div><strong>Méthode d'essai</strong><small>Version approuvée applicable au créneau</small></div>
        </div>
        <label>
          Méthode
          <select value={methodRevisionId} onChange={(event) => setMethodRevisionId(event.target.value)}>
            {options.methods.map((candidate) => (
              <option key={candidate.revision_id} value={candidate.revision_id}>
                {methodOptionLabel(candidate)}
              </option>
            ))}
          </select>
        </label>
        {method && (
          <p className="preparationContextLine">
            {measurementAxisLabel(method.measurement_axis)}
            {method.standard_references?.length ? ` · ${method.standard_references.join(", ")}` : ""}
          </p>
        )}
      </section>

      <section className="preparationSection">
        <div className="preparationSectionTitle">
          <span>2</span><div><strong>Montage de mesure</strong><small>Matériels et corrections figés dans Test Station</small></div>
        </div>
        <label>
          Montage
          <select value={setupRevisionId} onChange={(event) => setSetupRevisionId(event.target.value)}>
            {options.station_setups.map((candidate) => (
              <option key={candidate.station_setup.revision_id} value={candidate.station_setup.revision_id}>
                {candidate.station_setup.label} · {candidate.station_setup.station_label}
              </option>
            ))}
          </select>
        </label>
        {stationOption && (
          <div className={`stationReadiness stationReadiness-${stationOption.readiness.ready ? "ready" : "blocked"}`}>
            {stationOption.readiness.ready ? <CheckCircle2 size={16} /> : <AlertCircle size={16} />}
            <span>
              {stationOption.readiness.ready
                ? "Montage contrôlé pour la date prévue"
                : `${stationOption.readiness.issues.filter((issue) => issue.severity === "blocking").length} point(s) bloquant(s) sur le montage`}
            </span>
          </div>
        )}
      </section>

      {method && stationOption && (
        <section className="preparationSection">
          <div className="preparationSectionTitle">
            <span>3</span><div><strong>Affectation des matériels</strong><small>Un matériel physique pour chaque rôle de la méthode</small></div>
          </div>
          <div className="instrumentAssignmentList">
            {method.instrumentation_chain.map((slot) => (
              <label key={slot.slot_id}>
                <span className="assignmentLabel">
                  {slot.label}
                  <small>{slot.required ? "Obligatoire" : "Optionnel"}</small>
                </span>
                <select
                  aria-label={`Matériel pour ${slot.label}`}
                  value={assignments[slot.slot_id] ?? ""}
                  onChange={(event) =>
                    setAssignments({ ...assignments, [slot.slot_id]: event.target.value })
                  }
                >
                  <option value="">Non affecté</option>
                  {stationOption.station_setup.assets.map((asset) => (
                    <option key={asset.binding_id} value={asset.binding_id}>
                      {assetOptionLabel(asset)}
                    </option>
                  ))}
                </select>
              </label>
            ))}
          </div>
        </section>
      )}

      <section className="preparationAssessmentBar">
        <label>
          Motif du contrôle
          <input value={reason} onChange={(event) => setReason(event.target.value)} />
        </label>
        <button disabled={busy || !method || !stationOption || !reason.trim()} onClick={() => void assess()}>
          <ClipboardCheck size={16} /> Vérifier la préparation
        </button>
      </section>

      {history.length > 0 && <PreparationHistory revisions={history} />}
    </div>
  );
}

function PreparationVerdict(props: { preparation: PlannedTestPreparationAggregate | null }) {
  const state = props.preparation?.current_state ?? "missing";
  const revision = props.preparation?.current_revision;
  if (!revision) {
    return (
      <section className="preparationVerdict preparationVerdict-missing">
        <ClipboardCheck size={22} />
        <div><strong>Aucun contrôle enregistré</strong><p>Complétez les trois étapes puis vérifiez la préparation.</p></div>
      </section>
    );
  }
  return (
    <section className={`preparationVerdict preparationVerdict-${state}`}>
      {state === "ready" ? <CheckCircle2 size={23} /> : <ShieldAlert size={23} />}
      <div className="preparationVerdictContent">
        <div><strong>{preparationStateTitle(state)}</strong><small>Contrôle n° {revision.revision_number} · {formatAuditDate(revision.created_at)}</small></div>
        {revision.definition.verdict.issues.length > 0 ? (
          <div className="preparationIssues">
            {revision.definition.verdict.issues.map((issue, index) => (
              <PreparationIssueRow key={`${issue.code}-${index}`} issue={issue} />
            ))}
          </div>
        ) : (
          <p>La méthode, le montage et les matériels sont aptes pour ce créneau.</p>
        )}
      </div>
    </section>
  );
}

function PreparationIssueRow(props: { issue: PlannedTestPreparationIssue }) {
  return (
    <div className={`preparationIssue preparationIssue-${props.issue.severity}`}>
      {props.issue.severity === "blocking" ? <AlertCircle size={16} /> : <Wrench size={16} />}
      <div><strong>{preparationDimensionLabel(props.issue.dimension)}</strong><p>{props.issue.message}</p><small>{props.issue.next_action}</small></div>
    </div>
  );
}

function PreparationHistory(props: { revisions: PlannedTestPreparationRevision[] }) {
  return (
    <section className="preparationHistory">
      <header><History size={16} /><strong>Historique des contrôles</strong></header>
      <div>
        {props.revisions.map((revision) => (
          <div key={revision.revision_id}>
            <span className={`preparationHistoryState preparationHistoryState-${revision.recorded_state}`}>
              {revision.recorded_state === "ready" ? <CheckCircle2 size={14} /> : <AlertCircle size={14} />}
            </span>
            <span><strong>Contrôle n° {revision.revision_number}</strong><small>{formatAuditDate(revision.created_at)} · {revision.actor}</small></span>
            <span>{revision.recorded_state === "ready" ? "Prêt" : "Bloqué"}</span>
          </div>
        ))}
      </div>
    </section>
  );
}

function PreparationStateBadge(props: { state: PlannedTestPreparationAggregate["current_state"] }) {
  return <span className={`preparationState preparationState-${props.state}`}>{preparationStateTitle(props.state)}</span>;
}

function matchesPreparationStatus(status: ServiceScheduleStatus) {
  return status === "planned" || status === "confirmed";
}

function preparationStateTitle(state: PlannedTestPreparationAggregate["current_state"]) {
  return {
    missing: "À préparer",
    blocked: "Préparation bloquée",
    ready: "Prêt à démarrer",
    stale: "À revérifier"
  }[state];
}

function methodOptionLabel(method: PlannedTestMethodSnapshot) {
  const reference = method.method_code ? ` · ${method.method_code}` : "";
  return `${method.title}${reference} · version ${method.revision_number}`;
}

function assetOptionLabel(asset: PlannedStationSetupSnapshot["assets"][number]) {
  return `${asset.role_label} · ${asset.manufacturer} ${asset.model_name} · n° série ${asset.serial_number}`;
}

function measurementAxisLabel(axis: string) {
  const labels: Record<string, string> = {
    frequency_sweep: "Mesure fréquentielle",
    time_series: "Mesure temporelle",
    scalar: "Mesure scalaire",
    hybrid: "Mesure hybride"
  };
  return labels[axis] ?? "Méthode de mesure";
}

function preparationDimensionLabel(dimension: PlannedTestPreparationIssue["dimension"]) {
  const labels: Record<PlannedTestPreparationIssue["dimension"], string> = {
    schedule_context: "Créneau",
    test_method: "Méthode",
    station_setup: "Montage",
    instrument_assignment: "Affectation des matériels",
    serviceability: "État de service",
    calibration_validity: "Étalonnage",
    missing_evidence: "Preuve métrologique",
    nonconformance: "Non-conformité",
    correction_validity: "Correction de mesure"
  };
  return labels[dimension];
}

function formatAuditDate(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat("fr-FR", { dateStyle: "short", timeStyle: "short" }).format(date);
}

function rescheduleForm(item: LaboratoryScheduleItem): RescheduleForm {
  return {
    date: item.planned_start_at.slice(0, 10),
    start_time: item.planned_start_at.slice(11, 16),
    end_time: item.planned_end_at.slice(11, 16),
    assigned_operator: item.assigned_operator,
    location: item.location,
    reason: ""
  };
}

function planningErrorMessage(caught: unknown): string {
  if (!(caught instanceof ApiError)) {
    return caught instanceof Error ? caught.message : "Une erreur inattendue est survenue.";
  }
  const conflict = caught.details?.conflicting_item as Record<string, unknown> | undefined;
  if (caught.code === "service_schedule_operator_conflict" && conflict) {
    return `${String(caught.details?.value)} est déjà affecté à « ${String(conflict.title)} » du dossier ${String(conflict.project_code)}, de ${formatApiDateTime(String(conflict.planned_start_at))} à ${String(conflict.planned_end_at).slice(11, 16)}.`;
  }
  if (caught.code === "service_schedule_location_conflict" && conflict) {
    return `${String(caught.details?.value)} est déjà réservé pour « ${String(conflict.title)} » du dossier ${String(conflict.project_code)}, de ${formatApiDateTime(String(conflict.planned_start_at))} à ${String(conflict.planned_end_at).slice(11, 16)}.`;
  }
  if (caught.code === "planned_test_preparation_not_ready") {
    const issues = caught.details?.issues as
      | Array<{ message?: unknown; next_action?: unknown }>
      | undefined;
    const first = issues?.[0];
    if (first?.message) {
      const nextAction = first.next_action ? ` ${String(first.next_action)}` : "";
      return `${String(first.message)}${nextAction}`;
    }
  }
  const messages: Record<string, string> = {
    service_schedule_concurrent_update:
      "Ce créneau a changé sur un autre écran. La semaine a été actualisée ; votre saisie est conservée.",
    service_schedule_item_not_reschedulable:
      "Ce créneau a déjà démarré ou est terminé. Il ne peut plus être déplacé.",
    invalid_service_schedule_request:
      "Le nouvel horaire n'est pas valide. Vérifiez la date, le début et la fin.",
    planned_test_preparation_required:
      "Préparez la méthode, le montage et les matériels avant de démarrer l'essai.",
    planned_test_preparation_stale:
      "Le créneau a changé depuis le dernier contrôle. Vérifiez à nouveau la préparation.",
    planned_test_preparation_concurrent_update:
      "Une autre vérification a été enregistrée. Fermez puis rouvrez la préparation avant de continuer.",
    planned_test_schedule_concurrent_update:
      "Le créneau a changé pendant la vérification. Actualisez le planning puis recommencez.",
    planned_test_schedule_not_preparable:
      "Cet essai a déjà démarré ou est terminé. Sa préparation ne peut plus être modifiée.",
    planned_test_method_not_approved:
      "La méthode choisie n'est plus approuvée. Sélectionnez une méthode disponible.",
    planned_test_station_setup_not_ready:
      "Le montage choisi n'est plus marqué « Prêt à câbler ». Sélectionnez un autre montage.",
    storage_not_initialized: "Le stockage local doit être initialisé avant d'ouvrir le planning."
  };
  return messages[caught.code] ?? caught.message;
}

function buildWeekDays(weekStart: string): string[] {
  return Array.from({ length: 5 }, (_, index) => addDays(weekStart, index));
}

function compareScheduleItems(left: LaboratoryScheduleItem, right: LaboratoryScheduleItem) {
  return (
    left.planned_start_at.localeCompare(right.planned_start_at) ||
    left.planned_end_at.localeCompare(right.planned_end_at) ||
    left.project_code.localeCompare(right.project_code) ||
    left.item_code.localeCompare(right.item_code)
  );
}

function uniqueSorted(values: string[]): string[] {
  return [...new Set(values)].sort((left, right) => left.localeCompare(right, "fr"));
}

function parseIsoDate(value: string): Date {
  const [year, month, day] = value.split("-").map(Number);
  return new Date(year, month - 1, day, 12);
}

function isoDate(date: Date): string {
  return [
    date.getFullYear(),
    String(date.getMonth() + 1).padStart(2, "0"),
    String(date.getDate()).padStart(2, "0")
  ].join("-");
}

function mondayFor(date: Date): string {
  const monday = new Date(date.getFullYear(), date.getMonth(), date.getDate(), 12);
  const day = monday.getDay();
  monday.setDate(monday.getDate() - (day === 0 ? 6 : day - 1));
  return isoDate(monday);
}

function addDays(value: string, count: number): string {
  const date = parseIsoDate(value);
  date.setDate(date.getDate() + count);
  return isoDate(date);
}

function formatShortDate(value: string): string {
  return new Intl.DateTimeFormat("fr-FR", { day: "numeric", month: "short" })
    .format(parseIsoDate(value))
    .replace(".", "");
}

function formatWeekRange(weekStart: string): string {
  return `Semaine du ${formatShortDate(weekStart)} au ${formatShortDate(addDays(weekStart, 4))}`;
}

function formatTimeRange(item: LaboratoryScheduleItem): string {
  return `${item.planned_start_at.slice(11, 16)}–${item.planned_end_at.slice(11, 16)}`;
}

function formatFullDateTime(item: LaboratoryScheduleItem): string {
  return `${weekdayLabels[parseIsoDate(item.planned_start_at.slice(0, 10)).getDay() - 1]} ${formatShortDate(item.planned_start_at.slice(0, 10))}, ${formatTimeRange(item)}`;
}

function formatApiDateTime(value: string): string {
  return `${formatShortDate(value.slice(0, 10))} à ${value.slice(11, 16)}`;
}
