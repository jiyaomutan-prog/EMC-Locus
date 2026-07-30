import { AlertTriangle, Archive, MapPin, Pencil, Plus, RefreshCw, Save, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { fleetApi, type OperationContext } from "../../api";
import type { LaboratoryLocation } from "../../models/fleet";

const context: OperationContext = {
  actor: "laboratory.admin",
  reason: "gestion des lieux depuis LAB CONSOLE"
};

export function LaboratoryLocationsPanel() {
  const [locations, setLocations] = useState<LaboratoryLocation[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [editing, setEditing] = useState(false);
  const [creating, setCreating] = useState(false);
  const [label, setLabel] = useState("");
  const [description, setDescription] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const selected = locations.find((location) => location.location_id === selectedId) ?? null;

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const response = await fleetApi.listLocations(true);
      setLocations(response.locations);
      setSelectedId((current) => response.locations.some((location) => location.location_id === current) ? current : response.locations[0]?.location_id ?? "");
      setError(null);
    } catch (reason) {
      setError(message(reason));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);
  useEffect(() => {
    if (selected && !editing && !creating) {
      setLabel(selected.label);
      setDescription(selected.description);
    }
  }, [selected, editing, creating]);

  async function save() {
    if (!label.trim()) return;
    try {
      if (creating) await fleetApi.createLocation(label.trim(), description.trim(), context);
      else if (selected) await fleetApi.updateLocation(selected, label.trim(), description.trim(), context);
      setCreating(false);
      setEditing(false);
      await refresh();
    } catch (reason) {
      setError(message(reason));
    }
  }

  async function archive() {
    if (!selected) return;
    try {
      await fleetApi.archiveLocation(selected, context);
      await refresh();
    } catch (reason) {
      setError(message(reason));
    }
  }

  return (
    <section className="locationsWorkspace">
      <header className="resourcePageHeader">
        <div><p className="contextBanner">Registre des emplacements stables utilisés par le parc, les montages et le planning.</p><h2>Lieux du laboratoire</h2><p>Un changement de nom conserve l'identité du lieu et l'historique associé.</p></div>
        <div className="headerActions"><button className="iconButton secondary" type="button" onClick={() => void refresh()} title="Rafraîchir" aria-label="Rafraîchir"><RefreshCw size={16} /></button><button type="button" onClick={() => { setCreating(true); setEditing(false); setLabel(""); setDescription(""); }}><Plus size={16} /> Ajouter un lieu</button></div>
      </header>
      {error && <div className="targetedError"><AlertTriangle size={17} /><div><strong>Registre des lieux indisponible</strong><p>{error}</p></div></div>}
      <div className="locationsLayout">
        <aside className="locationList"><div className="listHeader"><h3>Emplacements</h3><span>{locations.length}</span></div>{loading && locations.length === 0 && <p>Chargement...</p>}{locations.map((location) => <button key={location.location_id} type="button" className={location.location_id === selectedId ? "active" : ""} onClick={() => { setSelectedId(location.location_id); setCreating(false); setEditing(false); }}><MapPin size={16} /><span><strong>{location.label}</strong><small>{location.status === "active" ? "Actif" : "Archivé"}</small></span></button>)}</aside>
        <article className="locationDetail">
          {(creating || (selected && editing)) ? (
            <><header><div><p className="eyebrow">{creating ? "Nouveau lieu" : "Modification"}</p><h3>{creating ? "Ajouter un emplacement" : selected?.label}</h3></div><button className="iconButton secondary" type="button" aria-label="Annuler" onClick={() => { setCreating(false); setEditing(false); }}><X size={16} /></button></header><label>Nom du lieu <span className="requiredBadge">Obligatoire</span><input autoFocus value={label} onChange={(event) => setLabel(event.target.value)} /></label><label>Description<textarea value={description} onChange={(event) => setDescription(event.target.value)} /></label>{!label.trim() && <p className="actionExplanation">Renseignez un nom lisible pour enregistrer le lieu.</p>}<button type="button" disabled={!label.trim()} onClick={() => void save()}><Save size={16} /> Enregistrer</button></>
          ) : selected ? (
            <><header><div><p className="eyebrow">Lieu du laboratoire</p><h3>{selected.label}</h3></div><span className={`status ${selected.status}`}>{selected.status === "active" ? "Actif" : "Archivé"}</span></header><p>{selected.description || "Aucune description."}</p><div className="headerActions"><button className="secondary" type="button" onClick={() => setEditing(true)} disabled={selected.status === "archived"}><Pencil size={16} /> Renommer ou décrire</button><button className="danger" type="button" onClick={() => void archive()} disabled={selected.status === "archived"}><Archive size={16} /> Archiver</button></div>{selected.status === "archived" && <p className="actionExplanation">Ce lieu reste lisible dans l'historique, mais ne peut plus recevoir de nouvelle affectation.</p>}<details><summary>Détails techniques</summary><p>Identifiant interne : {selected.location_id}</p><p>Révision : {selected.revision}</p></details></>
          ) : <div className="compactEmpty"><strong>Aucun lieu sélectionné</strong><span>Ajoutez le premier lieu du laboratoire.</span></div>}
        </article>
      </div>
    </section>
  );
}

function message(error: unknown) { return error instanceof Error ? error.message : "Erreur inattendue."; }
