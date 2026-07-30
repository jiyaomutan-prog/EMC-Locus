import type { PhysicalAssetMetrologySummary } from "./models/fleet";

type MetrologyDisplay = Pick<PhysicalAssetMetrologySummary, "status" | "due_at">;

export function metrologyStatusLabel(summary: MetrologyDisplay): string {
  switch (summary.status) {
    case "valid":
      return summary.due_at ? `Étalonnage valide jusqu’au ${formatDate(summary.due_at)}` : "Décision métrologique indéterminée";
    case "due_soon":
      return summary.due_at ? `Échéance d’étalonnage proche : ${formatDate(summary.due_at)}` : "Décision métrologique indéterminée";
    case "expired":
      return summary.due_at ? `Étalonnage expiré depuis le ${formatDate(summary.due_at)}` : "Aucun étalonnage valide";
    case "missing": return "Aucun étalonnage valide";
    case "not_required": return "Étalonnage non requis";
    case "nonconforming": return "Dernier étalonnage non conforme";
    case "indeterminate": return "Décision métrologique indéterminée";
    case "unavailable": return "Métrologie temporairement indisponible";
  }
}

export function metrologyStatusGroup(summary: MetrologyDisplay): string {
  return ({
    valid: "Étalonnage valide",
    due_soon: "Échéance proche",
    expired: "Étalonnage expiré",
    missing: "Étalonnage manquant",
    not_required: "Étalonnage non requis",
    nonconforming: "Étalonnage non conforme",
    indeterminate: "Décision indéterminée",
    unavailable: "Métrologie indisponible"
  } as const)[summary.status];
}

function formatDate(value: string): string {
  const date = new Date(`${value}T12:00:00`);
  return Number.isNaN(date.valueOf()) ? value : new Intl.DateTimeFormat("fr-FR", { dateStyle: "medium" }).format(date);
}
