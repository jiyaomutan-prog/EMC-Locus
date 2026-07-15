import {
  AlertCircle,
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  Clock3,
  FolderKanban,
  MapPin,
  PencilLine,
  RefreshCw,
  UserRound,
  X
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { ApiError, projectApi } from "../../api";
import type {
  LaboratoryScheduleItem,
  LaboratoryWeekSchedule,
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
  const [editing, setEditing] = useState(false);
  const [form, setForm] = useState<RescheduleForm>(() => rescheduleForm(props.item));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const valid =
    form.date &&
    form.start_time &&
    form.end_time &&
    form.assigned_operator.trim() &&
    form.location.trim() &&
    form.reason.trim();

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
      setEditing(false);
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
            <h2 id="planning-detail-title">{editing ? "Déplacer le créneau" : props.item.title}</h2>
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

        {!editing ? (
          <div className="wizardBody planningDetailBody">
            <div className="planningDetailStatus">
              <span className={`scheduleStatus scheduleStatus-${props.item.status}`}>
                {statusLabels[props.item.status]}
              </span>
              {!props.item.can_reschedule && <small>Ce créneau ne peut plus être déplacé.</small>}
            </div>
            <dl className="planningDetailFacts">
              <div><dt><Clock3 size={15} /> Horaire</dt><dd>{formatFullDateTime(props.item)}</dd></div>
              <div><dt><UserRound size={15} /> Opérateur</dt><dd>{props.item.assigned_operator}</dd></div>
              <div><dt><MapPin size={15} /> Lieu</dt><dd>{props.item.location}</dd></div>
              <div><dt>Équipement à tester</dt><dd>{props.item.equipment_under_test}</dd></div>
            </dl>
            {props.item.notes && <p className="planningDetailNote">{props.item.notes}</p>}
          </div>
        ) : (
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
        )}

        <footer className="wizardFooter planningDetailFooter">
          {!editing ? (
            <>
              <button className="secondary" onClick={props.onOpenProject}>
                <FolderKanban size={16} /> Ouvrir le dossier
              </button>
              {props.item.can_reschedule && (
                <button onClick={() => setEditing(true)}>
                  <PencilLine size={16} /> Déplacer
                </button>
              )}
            </>
          ) : (
            <>
              <button className="secondary" onClick={() => { setEditing(false); setError(null); }}>
                Annuler
              </button>
              <button disabled={busy || !valid} onClick={() => void submit()}>
                <CalendarDays size={16} /> Enregistrer le déplacement
              </button>
            </>
          )}
        </footer>
      </section>
    </div>
  );
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
  const messages: Record<string, string> = {
    service_schedule_concurrent_update:
      "Ce créneau a changé sur un autre écran. La semaine a été actualisée ; votre saisie est conservée.",
    service_schedule_item_not_reschedulable:
      "Ce créneau a déjà démarré ou est terminé. Il ne peut plus être déplacé.",
    invalid_service_schedule_request:
      "Le nouvel horaire n'est pas valide. Vérifiez la date, le début et la fin.",
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
