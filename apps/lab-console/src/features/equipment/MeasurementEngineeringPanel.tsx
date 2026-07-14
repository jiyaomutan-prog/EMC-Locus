import {
  CheckCircle2,
  Download,
  FileSpreadsheet,
  GitBranch,
  Plus,
  RefreshCw,
  Save,
  Send,
  ShieldCheck
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  ApiError,
  measurementEngineeringApi,
  operationAggregate,
  type MeasurementEngineeringConfig,
  type OperationContext
} from "../../api";
import type {
  EngineeringCurveEvaluation,
  MeasurementEngineeringAggregate,
  MeasurementEngineeringCollection,
  MeasurementEngineeringDefinition,
  MeasurementEngineeringRevision,
  EquipmentAuditEvent,
  EquipmentValidationResult,
  PhysicalQuantity,
  SignalDomain
} from "../../models/equipment";
import {
  AuditTable,
  EditorCard,
  Field,
  StateBlock,
  StructuredTable,
  ValidationPanel
} from "./EquipmentWorkspace";

type MeasurementSpace =
  | "sensors"
  | "scaling"
  | "curves"
  | "daq"
  | "recipes";

type StudioSection =
  | "general"
  | "physical"
  | "electrical"
  | "excitation"
  | "scaling"
  | "corrections"
  | "method"
  | "lookup"
  | "expression"
  | "limits"
  | "axes"
  | "table"
  | "evaluation"
  | "daq"
  | "chain"
  | "sampling"
  | "revisions"
  | "audit"
  | "json";

interface MeasurementStudioConfig extends MeasurementEngineeringConfig {
  key: MeasurementSpace;
  title: string;
  listTitle: string;
  createLabel: string;
  emptyDetail: string;
  sections: Array<[StudioSection, string]>;
}

const configs: Record<MeasurementSpace, MeasurementStudioConfig> = {
  sensors: {
    key: "sensors",
    collection: "sensor-definitions",
    validationCollection: "sensor-definition-definitions",
    operationPrefix: "sensor-definition",
    title: "Capteurs et transducteurs",
    listTitle: "Définitions de capteurs",
    createLabel: "Nouveau capteur",
    emptyDetail: "Sélectionnez ou créez une définition de capteur ou de transducteur.",
    sections: [
      ["general", "Identification"],
      ["physical", "Entrée mesurée"],
      ["electrical", "Sortie électrique"],
      ["excitation", "Alimentation / conditionnement"],
      ["scaling", "Conversion temporelle"],
      ["corrections", "Réponse fréquentielle"],
      ["revisions", "Révisions"],
      ["audit", "Audit"],
      ["json", "Diagnostic JSON"]
    ]
  },
  scaling: {
    key: "scaling",
    collection: "scaling-profiles",
    validationCollection: "scaling-profile-definitions",
    operationPrefix: "scaling-profile",
    title: "Conversions temporelles",
    listTitle: "Conversions des échantillons",
    createLabel: "Nouvelle conversion",
    emptyDetail: "Sélectionnez ou créez une conversion appliquée aux échantillons temporels.",
    sections: [
      ["general", "Identification"],
      ["physical", "Entrée / sortie"],
      ["method", "Gain et offset"],
      ["limits", "Surcharge / écrêtage"],
      ["lookup", "Table de conversion"],
      ["expression", "Expression"],
      ["revisions", "Révisions"],
      ["audit", "Audit"],
      ["json", "Diagnostic JSON"]
    ]
  },
  curves: {
    key: "curves",
    collection: "engineering-curves",
    validationCollection: "engineering-curve-definitions",
    operationPrefix: "engineering-curve",
    title: "Réponses fréquentielles",
    listTitle: "Corrections amplitude / phase",
    createLabel: "Nouvelle réponse",
    emptyDetail: "Sélectionnez ou créez une correction en fonction de la fréquence.",
    sections: [
      ["general", "Identification"],
      ["axes", "Grandeurs corrigées"],
      ["table", "Amplitude / phase"],
      ["evaluation", "Vérification ponctuelle"],
      ["revisions", "Révisions"],
      ["audit", "Audit"],
      ["json", "Diagnostic JSON"]
    ]
  },
  daq: {
    key: "daq",
    collection: "daq-channel-profiles",
    validationCollection: "daq-channel-profile-definitions",
    operationPrefix: "daq-channel-profile",
    title: "Entrées / sorties DAQ",
    listTitle: "Profils de voies DAQ",
    createLabel: "Nouvelle voie DAQ",
    emptyDetail: "Sélectionnez ou créez un profil d’entrée ou de sortie de numériseur.",
    sections: [
      ["general", "Identification"],
      ["daq", "Type de voie"],
      ["physical", "Plages"],
      ["sampling", "Echantillonnage"],
      ["excitation", "Alimentation capteur"],
      ["revisions", "Révisions"],
      ["audit", "Audit"],
      ["json", "Diagnostic JSON"]
    ]
  },
  recipes: {
    key: "recipes",
    collection: "acquisition-channel-recipes",
    validationCollection: "acquisition-channel-recipe-definitions",
    operationPrefix: "acquisition-channel-recipe",
    title: "Chaînes d'acquisition",
    listTitle: "Chaînes d'acquisition",
    createLabel: "Nouvelle chaîne",
    emptyDetail: "Sélectionnez ou créez une chaîne reliant capteur, voie DAQ et traitements.",
    sections: [
      ["general", "Identification"],
      ["chain", "Chaîne de mesure"],
      ["sampling", "Échantillonnage / plage"],
      ["revisions", "Révisions"],
      ["audit", "Audit"],
      ["json", "Diagnostic JSON"]
    ]
  }
};

const context: OperationContext = {
  actor: "measurement.engineering.author",
  reason: "operation LAB CONSOLE measurement engineering"
};

const curveTypes = [
  "antenna_factor",
  "cable_loss",
  "amplifier_gain",
  "attenuator_loss",
  "current_probe_transfer",
  "voltage_probe_transfer",
  "sensor_frequency_response",
  "phase_response",
  "linearity_correction",
  "uncertainty",
  "vswr",
  "s_parameter_magnitude",
  "site_characterization",
  "generic_correction"
];

const sensorFamilies = [
  "current_probe",
  "voltage_probe",
  "field_probe",
  "receiving_antenna",
  "transmitting_antenna",
  "accelerometer",
  "microphone",
  "thermocouple",
  "pressure_sensor",
  "photodiode",
  "strain_gauge",
  "generic_transducer",
  "manual_transducer"
];

const quantities: PhysicalQuantity[] = [
  "frequency",
  "time",
  "voltage",
  "current",
  "power",
  "electric_field",
  "magnetic_field",
  "velocity",
  "acceleration",
  "sound_pressure",
  "temperature",
  "pressure",
  "force",
  "torque",
  "strain",
  "electric_charge",
  "magnetic_flux_density",
  "humidity",
  "illuminance",
  "mass",
  "flow_rate",
  "dimensionless"
];

const signalDomains: SignalDomain[] = [
  "rf",
  "analog_voltage",
  "analog_current",
  "analog_charge",
  "trigger",
  "digital_logic",
  "can_bus",
  "mechanical",
  "environmental",
  "software"
];

const channelKinds = [
  "analog_input",
  "analog_output",
  "digital_input",
  "digital_output",
  "digital_bidirectional",
  "counter_input",
  "frequency_input",
  "trigger_input",
  "trigger_output",
  "can_bus_channel",
  "software_channel"
];

const inputModes = [
  "single_ended",
  "differential",
  "pseudo_differential",
  "current_loop",
  "charge",
  "iepe",
  "bridge_quarter",
  "bridge_half",
  "bridge_full",
  "thermocouple",
  "rtd"
];

export function MeasurementEngineeringPanel(props: { initialSpace: MeasurementSpace }) {
  const activeSpace = props.initialSpace;
  const activeConfig = configs[activeSpace];
  const [itemsByCollection, setItemsByCollection] = useState<
    Partial<Record<MeasurementEngineeringCollection, MeasurementEngineeringAggregate[]>>
  >({});
  const [loadState, setLoadState] = useState<"loading" | "ready" | "error">("loading");
  const [operationError, setOperationError] = useState<string | null>(null);
  const [selected, setSelected] = useState<MeasurementEngineeringAggregate | null>(null);
  const [revision, setRevision] = useState<MeasurementEngineeringRevision | null>(null);
  const [definition, setDefinition] = useState<MeasurementEngineeringDefinition | null>(null);
  const [definitionChecksum, setDefinitionChecksum] = useState("");
  const [revisions, setRevisions] = useState<MeasurementEngineeringRevision[]>([]);
  const [audit, setAudit] = useState<EquipmentAuditEvent[]>([]);
  const [validation, setValidation] = useState<EquipmentValidationResult | null>(null);
  const [section, setSection] = useState<StudioSection>("general");
  const [jsonDraft, setJsonDraft] = useState("");
  const [newEntityId, setNewEntityId] = useState("");
  const [newLabel, setNewLabel] = useState("");
  const [curveCsv, setCurveCsv] = useState("");
  const [lookupCsv, setLookupCsv] = useState("");
  const [evaluationFrequency, setEvaluationFrequency] = useState("100000000");
  const [curveEvaluation, setCurveEvaluation] = useState<EngineeringCurveEvaluation | null>(null);

  const refreshLists = useCallback(async () => {
    setLoadState("loading");
    setOperationError(null);
    try {
      const entries = await Promise.all(
        Object.values(configs).map(async (config) => {
          const response = await measurementEngineeringApi.list(config);
          return [config.collection, response.items] as const;
        })
      );
      setItemsByCollection(Object.fromEntries(entries));
      setLoadState("ready");
    } catch (error) {
      setLoadState("error");
      setOperationError(errorMessage(error));
    }
  }, []);

  useEffect(() => {
    void refreshLists();
  }, [refreshLists]);

  useEffect(() => {
    setSelected(null);
    setRevision(null);
    setDefinition(null);
    setDefinitionChecksum("");
    setRevisions([]);
    setAudit([]);
    setJsonDraft("");
    setSection("general");
    setValidation(null);
    setCurveEvaluation(null);
  }, [activeSpace]);

  const items = itemsByCollection[activeConfig.collection] ?? [];
  const readOnly = revision?.status !== "draft";
  const approvedOptions = useMemo(
    () => ({
      sensors: approvedRefs(itemsByCollection["sensor-definitions"] ?? []),
      scalings: approvedRefs(itemsByCollection["scaling-profiles"] ?? []),
      curves: approvedRefs(itemsByCollection["engineering-curves"] ?? []),
      daq: approvedRefs(itemsByCollection["daq-channel-profiles"] ?? [])
    }),
    [itemsByCollection]
  );

  async function openItem(
    config: MeasurementStudioConfig,
    item: MeasurementEngineeringAggregate,
    targetRevision?: MeasurementEngineeringRevision | null
  ) {
    setOperationError(null);
    const target =
      targetRevision ?? item.active_draft_revision ?? item.current_approved_revision ?? item.latest_revision;
    if (!target) return;
    try {
      const [detail, revisionList, auditList] = await Promise.all([
        measurementEngineeringApi.get(config, item.identity.entity_id),
        measurementEngineeringApi.listRevisions(config, item.identity.entity_id),
        measurementEngineeringApi.listAudit(config, item.identity.entity_id)
      ]);
      const freshRevision =
        revisionList.revisions.find((candidate) => candidate.revision_id === target.revision_id) ??
        target;
      setSelected(detail.item);
      setRevision(freshRevision);
      setDefinition(freshRevision.definition);
      setDefinitionChecksum(freshRevision.definition_checksum);
      setRevisions(revisionList.revisions);
      setAudit(auditList.audit_events);
      setJsonDraft(JSON.stringify(freshRevision.definition, null, 2));
      setValidation(null);
      setCurveEvaluation(null);
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  }

  async function runOperation(operation: () => Promise<void>) {
    setOperationError(null);
    try {
      await operation();
    } catch (error) {
      setOperationError(errorMessage(error));
    }
  }

  function updateDefinition(next: MeasurementEngineeringDefinition) {
    setDefinition(next);
    setJsonDraft(JSON.stringify(next, null, 2));
    setValidation(null);
    setCurveEvaluation(null);
  }

  async function createDraft() {
    const entityId = newEntityId.trim() || generatedEntityId(activeSpace);
    const label = newLabel.trim() || defaultLabel(activeSpace, entityId);
    const draft = defaultMeasurementDefinition(activeSpace, entityId, label, approvedOptions);
    await runOperation(async () => {
      const result = await measurementEngineeringApi.create(activeConfig, {
        entity_id: entityId,
        definition: draft,
        ...context
      });
      await refreshLists();
      await openItem(activeConfig, operationAggregate(result), result.revision);
      setNewEntityId("");
      setNewLabel("");
    });
  }

  async function validateDefinition() {
    if (!definition) return;
    await runOperation(async () => {
      setValidation(await measurementEngineeringApi.validateDefinition(activeConfig, definition));
    });
  }

  async function saveDraft() {
    if (!selected || !revision || !definition || readOnly) return;
    await runOperation(async () => {
      const result = await measurementEngineeringApi.saveDraft(
        activeConfig,
        selected.identity.entity_id,
        revision.revision_id,
        definitionChecksum,
        definition,
        context
      );
      await refreshLists();
      await openItem(activeConfig, operationAggregate(result), result.revision);
    });
  }

  async function submitRevision() {
    if (!selected || !revision || readOnly) return;
    await runOperation(async () => {
      const result = await measurementEngineeringApi.submit(
        activeConfig,
        selected.identity.entity_id,
        revision.revision_id,
        { ...context, reason: "soumission definition mesure" }
      );
      await refreshLists();
      await openItem(activeConfig, operationAggregate(result), result.revision);
    });
  }

  async function approveRevision() {
    if (!selected || !revision || revision.status !== "under_review") return;
    await runOperation(async () => {
      const result = await measurementEngineeringApi.approve(
        activeConfig,
        selected.identity.entity_id,
        revision.revision_id,
        {
          actor: "measurement.engineering.approver",
          reason: "approbation definition mesure"
        }
      );
      await refreshLists();
      await openItem(activeConfig, operationAggregate(result), result.revision);
    });
  }

  async function deriveRevision() {
    if (!selected?.current_approved_revision) return;
    await runOperation(async () => {
      const result = await measurementEngineeringApi.deriveRevision(
        activeConfig,
        selected.identity.entity_id,
        selected.current_approved_revision!.revision_id,
        { ...context, reason: "nouvelle revision definition mesure" }
      );
      await refreshLists();
      await openItem(activeConfig, operationAggregate(result), result.revision);
    });
  }

  function applyJson() {
    try {
      updateDefinition(JSON.parse(jsonDraft) as MeasurementEngineeringDefinition);
    } catch {
      setOperationError("JSON definition invalide.");
    }
  }

  async function evaluateCurve() {
    if (!selected || !revision || activeSpace !== "curves") return;
    const frequency = Number(evaluationFrequency);
    if (!Number.isFinite(frequency) || frequency <= 0) {
      setOperationError("La frequence d'evaluation doit etre positive.");
      return;
    }
    await runOperation(async () => {
      const result = await measurementEngineeringApi.evaluateCurve(
        selected.identity.entity_id,
        revision.revision_id,
        { frequency }
      );
      setCurveEvaluation(result.evaluation);
    });
  }

  return (
    <section className="measurementPanel">
      <div className="measurementHeader">
        <div>
          <p className="eyebrow">Signaux et corrections</p>
          <h2>{activeConfig.title}</h2>
          <p className="workspaceMeta">{items.length} définition{items.length > 1 ? "s" : ""}</p>
        </div>
        <button className="iconButton secondary" onClick={() => void refreshLists()} title="Rafraichir les definitions" aria-label="Rafraichir les definitions">
          <RefreshCw size={16} />
        </button>
      </div>

      <div className="toolbar measurementCreateBar">
        <Field label="Libellé / modèle" value={newLabel} onChange={setNewLabel} />
        <details className="advancedOptions compactAdvanced">
          <summary>Identifiant personnalisé</summary>
          <Field label="Nouvel ID" value={newEntityId} onChange={setNewEntityId} />
        </details>
        <button onClick={() => void createDraft()}>
          <Plus size={16} /> {activeConfig.createLabel}
        </button>
      </div>

      {operationError && (
        <div className="conflictBox">
          <strong>Opération refusée</strong>
          <p>{operationError}</p>
        </div>
      )}

      {loadState === "loading" && <StateBlock title="Chargement" detail="Lecture des définitions de signaux et de corrections." />}
      {loadState === "error" && <StateBlock title="Erreur" detail={operationError ?? "Domaine indisponible."} />}

      {loadState === "ready" && (
        <div className="equipmentLayout measurementLayout">
          <MeasurementList
            title={activeConfig.listTitle}
            items={items}
            selected={selected}
            onOpen={(item) => void openItem(activeConfig, item)}
          />
          <section className="equipmentStudio">
            {!selected || !revision || !definition ? (
              <StateBlock title="Aucune définition ouverte" detail={activeConfig.emptyDetail} />
            ) : (
              <>
                <div className="studioHeader">
                  <div>
                    <p className="eyebrow">{activeConfig.title}</p>
                    <h2>{revision.label}</h2>
                    <div className="studioTitleMeta">
                      <span className={"status " + revision.status}>{measurementStatusLabel(revision.status)}</span>
                      <span>Révision {revision.revision_number}</span>
                    </div>
                  </div>
                  <div className="headerActions">
                    <button className="secondary" onClick={() => void validateDefinition()}>
                      <CheckCircle2 size={16} /> Valider
                    </button>
                    {revision.status === "draft" && (
                      <>
                        <button onClick={() => void saveDraft()}>
                          <Save size={16} /> Sauvegarder
                        </button>
                        <button className="secondary" onClick={() => void submitRevision()}>
                          <Send size={16} /> Soumettre
                        </button>
                      </>
                    )}
                    {revision.status === "under_review" && (
                      <button onClick={() => void approveRevision()}>
                        <ShieldCheck size={16} /> Approuver
                      </button>
                    )}
                    {revision.status === "approved" && selected.current_approved_revision && (
                      <button onClick={() => void deriveRevision()}>
                        <GitBranch size={16} /> Nouvelle révision
                      </button>
                    )}
                  </div>
                </div>

                <div className="studioLayout equipmentStudioLayout">
                  <nav className="sectionNav">
                    {activeConfig.sections.map(([key, label]) => (
                      <button
                        key={key}
                        className={section === key ? "active" : ""}
                        onClick={() => setSection(key)}
                      >
                        {label}
                      </button>
                    ))}
                  </nav>
                  <div className="editorPane">
                    <StudioSectionRenderer
                      space={activeSpace}
                      section={section}
                      definition={definition}
                      readOnly={readOnly}
                      revisions={revisions}
                      audit={audit}
                      jsonDraft={jsonDraft}
                      lookupCsv={lookupCsv}
                      curveCsv={curveCsv}
                      evaluationFrequency={evaluationFrequency}
                      curveEvaluation={curveEvaluation}
                      approvedOptions={approvedOptions}
                      onDefinition={updateDefinition}
                      onJsonDraft={setJsonDraft}
                      onApplyJson={applyJson}
                      onLookupCsv={setLookupCsv}
                      onApplyLookupCsv={() => applyLookupCsv(definition, lookupCsv, updateDefinition, setOperationError)}
                      onExportLookupCsv={() => setLookupCsv(exportLookupCsv(definition))}
                      onCurveCsv={setCurveCsv}
                      onApplyCurveCsv={() => applyCurveCsv(definition, curveCsv, updateDefinition, setOperationError)}
                      onExportCurveCsv={() => setCurveCsv(exportCurveCsv(definition))}
                      onImportCurveFile={(file) => void readFile(file).then((text) => {
                        setCurveCsv(text);
                        applyCurveCsv(definition, text, updateDefinition, setOperationError);
                      })}
                      onEvaluationFrequency={setEvaluationFrequency}
                      onEvaluateCurve={() => void evaluateCurve()}
                      onOpenRevision={(nextRevision) => void openItem(activeConfig, selected, nextRevision)}
                    />
                  </div>
                  <ValidationPanel validation={validation} />
                </div>
              </>
            )}
          </section>
        </div>
      )}
    </section>
  );
}

function MeasurementList(props: {
  title: string;
  items: MeasurementEngineeringAggregate[];
  selected: MeasurementEngineeringAggregate | null;
  onOpen: (item: MeasurementEngineeringAggregate) => void;
}) {
  return (
    <aside className="equipmentList">
      <h2>{props.title}</h2>
      {props.items.length === 0 && <p>Aucune definition.</p>}
      {props.items.map((item) => {
        const revision = item.latest_revision ?? item.current_approved_revision;
        return (
          <button
            key={item.identity.entity_id}
            className={props.selected?.identity.entity_id === item.identity.entity_id ? "active" : ""}
            onClick={() => props.onOpen(item)}
          >
            <strong>{item.identity.label}</strong>
            <span className="listItemMeta">
              <span className={"status " + (revision?.status ?? "")}>{measurementStatusLabel(revision?.status)}</span>
              <small>Révision {revision?.revision_number ?? "-"}</small>
            </span>
          </button>
        );
      })}
    </aside>
  );
}

function measurementStatusLabel(status?: string) {
  if (status === "draft") return "Brouillon";
  if (status === "under_review") return "En revue";
  if (status === "approved") return "Approuve";
  if (status === "superseded") return "Remplace";
  if (status === "suspended") return "Suspendu";
  if (status === "retired") return "Retire";
  return "Sans revision";
}

function StudioSectionRenderer(props: {
  space: MeasurementSpace;
  section: StudioSection;
  definition: MeasurementEngineeringDefinition;
  readOnly: boolean;
  revisions: MeasurementEngineeringRevision[];
  audit: EquipmentAuditEvent[];
  jsonDraft: string;
  lookupCsv: string;
  curveCsv: string;
  evaluationFrequency: string;
  curveEvaluation: EngineeringCurveEvaluation | null;
  approvedOptions: ApprovedOptions;
  onDefinition: (definition: MeasurementEngineeringDefinition) => void;
  onJsonDraft: (value: string) => void;
  onApplyJson: () => void;
  onLookupCsv: (value: string) => void;
  onApplyLookupCsv: () => void;
  onExportLookupCsv: () => void;
  onCurveCsv: (value: string) => void;
  onApplyCurveCsv: () => void;
  onExportCurveCsv: () => void;
  onImportCurveFile: (file: File) => void;
  onEvaluationFrequency: (value: string) => void;
  onEvaluateCurve: () => void;
  onOpenRevision: (revision: MeasurementEngineeringRevision) => void;
}) {
  const definition = props.definition;
  if (props.section === "revisions") {
    return <MeasurementRevisionTable revisions={props.revisions} onOpen={props.onOpenRevision} />;
  }
  if (props.section === "audit") {
    return <AuditTable audit={props.audit} />;
  }
  if (props.section === "json") {
    return (
      <EditorCard title="Diagnostic JSON">
        <textarea
          className="jsonPreview"
          value={props.jsonDraft}
          disabled={props.readOnly}
          onChange={(event) => props.onJsonDraft(event.target.value)}
        />
        <button disabled={props.readOnly} onClick={props.onApplyJson}>
          Appliquer JSON
        </button>
      </EditorCard>
    );
  }
  if (props.space === "sensors") {
    return (
      <SensorSections
        section={props.section}
        definition={definition}
        readOnly={props.readOnly}
        approvedOptions={props.approvedOptions}
        onDefinition={props.onDefinition}
      />
    );
  }
  if (props.space === "scaling") {
    return (
      <ScalingSections
        section={props.section}
        definition={definition}
        readOnly={props.readOnly}
        lookupCsv={props.lookupCsv}
        onDefinition={props.onDefinition}
        onLookupCsv={props.onLookupCsv}
        onApplyLookupCsv={props.onApplyLookupCsv}
        onExportLookupCsv={props.onExportLookupCsv}
      />
    );
  }
  if (props.space === "curves") {
    return (
      <CurveSections
        section={props.section}
        definition={definition}
        readOnly={props.readOnly}
        curveCsv={props.curveCsv}
        evaluationFrequency={props.evaluationFrequency}
        curveEvaluation={props.curveEvaluation}
        onDefinition={props.onDefinition}
        onCurveCsv={props.onCurveCsv}
        onApplyCurveCsv={props.onApplyCurveCsv}
        onExportCurveCsv={props.onExportCurveCsv}
        onImportCurveFile={props.onImportCurveFile}
        onEvaluationFrequency={props.onEvaluationFrequency}
        onEvaluateCurve={props.onEvaluateCurve}
      />
    );
  }
  if (props.space === "daq") {
    return (
      <DaqSections
        section={props.section}
        definition={definition}
        readOnly={props.readOnly}
        onDefinition={props.onDefinition}
      />
    );
  }
  return (
    <RecipeSections
      section={props.section}
      definition={definition}
      readOnly={props.readOnly}
      approvedOptions={props.approvedOptions}
      onDefinition={props.onDefinition}
    />
  );
}

function SensorSections(props: SectionProps & { approvedOptions: ApprovedOptions }) {
  const d = props.definition;
  if (props.section === "general") {
    return (
      <EditorCard title="Identification">
        <Field label="Identifiant interne" value={s(d.sensor_definition_id)} disabled onChange={() => undefined} />
        <Field label="Fabricant" value={s(d.manufacturer)} disabled={props.readOnly} onChange={(manufacturer) => props.onDefinition({ ...d, manufacturer })} />
        <Field label="Modèle" value={s(d.model_name)} disabled={props.readOnly} onChange={(model_name) => props.onDefinition({ ...d, model_name })} />
        <label>
          Famille de capteur
          <select disabled={props.readOnly} value={s(d.sensor_family)} onChange={(event) => props.onDefinition({ ...d, sensor_family: event.target.value })}>
            {sensorFamilies.map((family) => <option key={family} value={family}>{humanMeasurementLabel(family)}</option>)}
          </select>
        </label>
        <Field label="Technologies" value={stringArray(d.technology_tags).join(", ")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, technology_tags: splitTokens(value) })} />
      </EditorCard>
    );
  }
  if (props.section === "physical") {
    return (
      <EditorCard title="Grandeur mesurée">
        <QuantitySelect label="Grandeur physique à l’entrée" value={s(d.physical_input_quantity)} disabled={props.readOnly} onChange={(physical_input_quantity) => props.onDefinition({ ...d, physical_input_quantity })} />
        <QuantitySelect label="Grandeur restituée" value={s(d.engineering_output_quantity)} disabled={props.readOnly} onChange={(engineering_output_quantity) => props.onDefinition({ ...d, engineering_output_quantity })} />
        <Field label="Unité restituée" value={s(d.engineering_output_unit)} disabled={props.readOnly} onChange={(engineering_output_unit) => props.onDefinition({ ...d, engineering_output_unit })} />
        <RangeFields title="Plage nominale" range={objectValue(d.nominal_range)} unitFallback={s(d.engineering_output_unit)} readOnly={props.readOnly} onRange={(nominal_range) => props.onDefinition({ ...d, nominal_range })} />
        <FrequencyRangeFields range={frequencyRange(d.frequency_range)} readOnly={props.readOnly} onRange={(frequency_range) => props.onDefinition({ ...d, frequency_range })} />
      </EditorCard>
    );
  }
  if (props.section === "electrical") {
    return (
      <EditorCard title="Signal électrique délivré">
        <QuantitySelect label="Grandeur électrique" value={s(d.electrical_output_quantity)} disabled={props.readOnly} onChange={(electrical_output_quantity) => props.onDefinition({ ...d, electrical_output_quantity })} />
        <Field label="Unité électrique" value={s(d.electrical_output_unit)} disabled={props.readOnly} onChange={(electrical_output_unit) => props.onDefinition({ ...d, electrical_output_unit })} />
        <label>
          Domaine du signal
          <select disabled={props.readOnly} value={s(d.signal_domain)} onChange={(event) => props.onDefinition({ ...d, signal_domain: event.target.value })}>
            {signalDomains.map((domain) => <option key={domain} value={domain}>{humanMeasurementLabel(domain)}</option>)}
          </select>
        </label>
        <label>
          Mode d’entrée requis
          <select disabled={props.readOnly} value={s(d.input_mode_requirement)} onChange={(event) => props.onDefinition({ ...d, input_mode_requirement: optionalString(event.target.value) })}>
            <option value="">Aucun</option>
            {inputModes.map((mode) => <option key={mode} value={mode}>{humanMeasurementLabel(mode)}</option>)}
          </select>
        </label>
      </EditorCard>
    );
  }
  if (props.section === "excitation") {
    return <ExcitationEditor definition={d} readOnly={props.readOnly} onDefinition={props.onDefinition} field="required_excitation" />;
  }
  if (props.section === "scaling") {
    return (
      <ReferenceEditor
        title="Conversion des échantillons"
        refs={refs(d.scaling_profile_refs)}
        options={props.approvedOptions.scalings}
        readOnly={props.readOnly}
        onRefs={(scaling_profile_refs) => props.onDefinition({ ...d, scaling_profile_refs })}
      />
    );
  }
  return (
    <ReferenceEditor
      title="Réponse fréquentielle"
      refs={refs(d.correction_curve_refs)}
      options={props.approvedOptions.curves}
      readOnly={props.readOnly}
      onRefs={(correction_curve_refs) => props.onDefinition({ ...d, correction_curve_refs })}
    />
  );
}

function ScalingSections(props: SectionProps & {
  lookupCsv: string;
  onLookupCsv: (value: string) => void;
  onApplyLookupCsv: () => void;
  onExportLookupCsv: () => void;
}) {
  const d = props.definition;
  const parameters = objectValue(d.parameters);
  const inputLimits = objectValue(d.input_limits);
  if (props.section === "general") {
    return (
      <EditorCard title="Identification">
        <div className="domainBanner">
          <strong>Signal temporel échantillonné</strong>
          <span>Conversion d’une valeur brute vers une grandeur physique.</span>
        </div>
        <Field label="Identifiant interne" value={s(d.scaling_profile_id)} disabled onChange={() => undefined} />
        <Field label="Nom de la conversion" value={s(d.label)} disabled={props.readOnly} onChange={(label) => props.onDefinition({ ...d, label })} />
        <Field label="Source métrologique ou documentaire" value={s(d.source_reference)} disabled={props.readOnly} onChange={(source_reference) => props.onDefinition({ ...d, source_reference: optionalString(source_reference) })} />
      </EditorCard>
    );
  }
  if (props.section === "physical") {
    return (
      <EditorCard title="Grandeur brute et grandeur restituée">
        <QuantitySelect label="Grandeur d’entrée" value={s(d.input_quantity)} disabled={props.readOnly} onChange={(input_quantity) => props.onDefinition({ ...d, input_quantity })} />
        <Field label="Unité d’entrée" value={s(d.input_unit)} disabled={props.readOnly} onChange={(input_unit) => props.onDefinition({ ...d, input_unit })} />
        <QuantitySelect label="Grandeur de sortie" value={s(d.output_quantity)} disabled={props.readOnly} onChange={(output_quantity) => props.onDefinition({ ...d, output_quantity })} />
        <Field label="Unité de sortie" value={s(d.output_unit)} disabled={props.readOnly} onChange={(output_unit) => props.onDefinition({ ...d, output_unit })} />
      </EditorCard>
    );
  }
  if (props.section === "method") {
    return (
      <EditorCard title="Loi de conversion">
        <div className="formulaStrip">Valeur physique = gain × échantillon + offset</div>
        <label>
          Type de conversion
          <select disabled={props.readOnly} value={s(d.scaling_kind)} onChange={(event) => props.onDefinition({ ...d, scaling_kind: event.target.value })}>
            {["identity", "linear", "two_point", "polynomial", "lookup_table", "piecewise_linear", "expression"].map((kind) => <option key={kind} value={kind}>{scalingKindLabel(kind)}</option>)}
          </select>
        </label>
        <Field label="Gain / facteur" value={s(parameters.scale)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, scale: optionalNumber(value) } })} />
        <Field label="Offset" value={s(parameters.offset ?? 0)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, offset: optionalNumber(value) ?? 0 } })} />
        <div className="measurementFourGrid">
          <Field label="Input point 1" value={s(parameters.input_point_1)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, input_point_1: optionalNumber(value) } })} />
          <Field label="Output point 1" value={s(parameters.output_point_1)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, output_point_1: optionalNumber(value) } })} />
          <Field label="Input point 2" value={s(parameters.input_point_2)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, input_point_2: optionalNumber(value) } })} />
          <Field label="Output point 2" value={s(parameters.output_point_2)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, output_point_2: optionalNumber(value) } })} />
        </div>
        <Field label="Coefficients polynomiaux" value={numberArray(parameters.coefficients).join(", ")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, coefficients: numberList(value) } })} />
      </EditorCard>
    );
  }
  if (props.section === "limits") {
    return (
      <EditorCard title="Limites du signal d’entrée">
        <div className="domainBanner warning">
          <strong>Détection de surcharge</strong>
          <span>Ces bornes décrivent la plage exploitable avant saturation ou écrêtage de l’entrée.</span>
        </div>
        <div className="measurementThreeGrid">
          <Field label={`Minimum (${s(d.input_unit)})`} value={s(inputLimits.minimum)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, input_limits: { ...inputLimits, minimum: numberOrZero(value), maximum: Number(inputLimits.maximum ?? 10), handling: s(inputLimits.handling || "warn") } })} />
          <Field label={`Maximum (${s(d.input_unit)})`} value={s(inputLimits.maximum)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, input_limits: { ...inputLimits, minimum: Number(inputLimits.minimum ?? -10), maximum: numberOrZero(value), handling: s(inputLimits.handling || "warn") } })} />
          <label>
            Traitement hors plage
            <select disabled={props.readOnly} value={s(inputLimits.handling || "warn")} onChange={(event) => props.onDefinition({ ...d, input_limits: { minimum: Number(inputLimits.minimum ?? -10), maximum: Number(inputLimits.maximum ?? 10), handling: event.target.value } })}>
              <option value="warn">Signaler une surcharge</option>
              <option value="reject">Refuser la valeur</option>
              <option value="mark_clipped">Marquer comme écrêtée</option>
            </select>
          </label>
        </div>
        <button className="secondary" disabled={props.readOnly || !d.input_limits} onClick={() => props.onDefinition({ ...d, input_limits: undefined })}>
          Retirer les limites
        </button>
      </EditorCard>
    );
  }
  if (props.section === "lookup") {
    return (
      <EditorCard title="Table de conversion">
        <div className="buttonRow">
          <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...d, parameters: { ...parameters, points: [...scalingPoints(parameters.points), { input: 0, output: 0 }] } })}>Ajouter point</button>
          <button onClick={props.onExportLookupCsv}><Download size={16} /> Exporter CSV</button>
          <button disabled={props.readOnly} onClick={props.onApplyLookupCsv}><FileSpreadsheet size={16} /> Importer CSV</button>
        </div>
        <textarea value={props.lookupCsv} onChange={(event) => props.onLookupCsv(event.target.value)} placeholder="input,output" />
        <StructuredTable columns={["Entrée", "Sortie"]}>
          {scalingPoints(parameters.points).map((point, index) => (
            <tr key={index}>
              <td><input disabled={props.readOnly} value={point.input} onChange={(event) => props.onDefinition({ ...d, parameters: { ...parameters, points: replaceAt(scalingPoints(parameters.points), index, { ...point, input: numberOrZero(event.target.value) }) } })} /></td>
              <td><input disabled={props.readOnly} value={point.output} onChange={(event) => props.onDefinition({ ...d, parameters: { ...parameters, points: replaceAt(scalingPoints(parameters.points), index, { ...point, output: numberOrZero(event.target.value) }) } })} /></td>
            </tr>
          ))}
        </StructuredTable>
      </EditorCard>
    );
  }
  return (
    <EditorCard title="Expression">
      <Field label="Expression contrôlée" value={s(parameters.expression)} disabled={props.readOnly} onChange={(expression) => props.onDefinition({ ...d, parameters: { ...parameters, expression: optionalString(expression) } })} />
      <p className="notice">Variables autorisées : x, input, temperature, frequency. Fonctions : pow, sqrt, log10, ln, abs, min, max.</p>
    </EditorCard>
  );
}

function CurveSections(props: SectionProps & {
  curveCsv: string;
  evaluationFrequency: string;
  curveEvaluation: EngineeringCurveEvaluation | null;
  onCurveCsv: (value: string) => void;
  onApplyCurveCsv: () => void;
  onExportCurveCsv: () => void;
  onImportCurveFile: (file: File) => void;
  onEvaluationFrequency: (value: string) => void;
  onEvaluateCurve: () => void;
}) {
  const d = props.definition;
  if (props.section === "general") {
    return (
      <EditorCard title="Identification">
        <div className="domainBanner">
          <strong>Spectre fréquentiel</strong>
          <span>Compensation d’amplitude et, si nécessaire, de phase en fonction de la fréquence.</span>
        </div>
        <Field label="Identifiant interne" value={s(d.curve_id)} disabled onChange={() => undefined} />
        <Field label="Nom de la réponse" value={s(d.label)} disabled={props.readOnly} onChange={(label) => props.onDefinition({ ...d, label })} />
        <label>
          Type de réponse
          <select disabled={props.readOnly} value={s(d.curve_type)} onChange={(event) => props.onDefinition({ ...d, curve_type: event.target.value })}>
            {curveTypes.map((type) => <option key={type} value={type}>{curveTypeLabel(type)}</option>)}
          </select>
        </label>
        <Field label="Document source" value={s(d.source_document_reference)} disabled={props.readOnly} onChange={(source_document_reference) => props.onDefinition({ ...d, source_document_reference: optionalString(source_document_reference) })} />
        <Field label="Empreinte SHA-256 de la source" value={s(d.source_checksum)} disabled={props.readOnly} onChange={(source_checksum) => props.onDefinition({ ...d, source_checksum: optionalString(source_checksum) })} />
      </EditorCard>
    );
  }
  if (props.section === "axes") {
    const axis = curveAxes(d)[0] ?? { axis: "frequency", quantity: "frequency", unit: "Hz" };
    const values = curveValues(d);
    const amplitude = values.find((value) => s(value.component || "amplitude") === "amplitude") ?? { value_id: "amplitude_correction_db", quantity: "dimensionless", unit: "dB", component: "amplitude", operation: "add" };
    const phase = values.find((value) => s(value.component) === "phase");
    const amplitudeValueId = s(amplitude.value_id || "amplitude_correction_db");
    return (
      <EditorCard title="Composantes de la réponse">
        <div className="measurementThreeGrid">
          <Field label="Axe" value="Fréquence" disabled onChange={() => undefined} />
          <Field label="Unité de fréquence" value={s(axis.unit)} disabled={props.readOnly} onChange={(unit) => props.onDefinition({ ...d, independent_axes: [{ axis: "frequency", quantity: "frequency", unit }] })} />
          <Field label="ID amplitude" value={amplitudeValueId} disabled={props.readOnly} onChange={(value_id) => props.onDefinition({ ...d, dependent_values: [{ ...amplitude, value_id }, ...(phase ? [phase] : [])], units: { frequency: s(axis.unit || "Hz"), [value_id]: amplitude.unit, ...(phase ? { [s(phase.value_id)]: phase.unit } : {}) } })} />
          <Field label="Unité d’amplitude" value={s(amplitude.unit)} disabled={props.readOnly} onChange={(unit) => props.onDefinition({ ...d, dependent_values: [{ ...amplitude, unit }, ...(phase ? [phase] : [])], units: { ...objectValue(d.units), [amplitudeValueId]: unit } })} />
          <label>
            Opération sur l’amplitude
            <select disabled={props.readOnly} value={s(amplitude.operation || "add")} onChange={(event) => props.onDefinition({ ...d, dependent_values: [{ ...amplitude, operation: event.target.value }, ...(phase ? [phase] : [])] })}>
              {["add", "subtract", "multiply", "divide"].map((operation) => <option key={operation} value={operation}>{correctionOperationLabel(operation)}</option>)}
            </select>
          </label>
          <div className="inlineFieldAction">
            {phase
              ? <button className="secondary" disabled={props.readOnly} onClick={() => props.onDefinition({ ...d, dependent_values: [amplitude], points: curvePoints(d.points).map((point) => ({ ...point, values: { [amplitudeValueId]: valueNumber(point, amplitudeValueId) } })) })}>Retirer la phase</button>
              : <button className="secondary" disabled={props.readOnly} onClick={() => props.onDefinition({ ...d, dependent_values: [amplitude, { value_id: "phase_correction_deg", quantity: "angle", unit: "deg", component: "phase", operation: "add" }], units: { ...objectValue(d.units), phase_correction_deg: "deg" }, points: curvePoints(d.points).map((point) => ({ ...point, values: { ...objectValue(point.values), phase_correction_deg: 0 } })) })}>Ajouter la phase</button>}
          </div>
        </div>
        {phase && (
          <div className="measurementThreeGrid">
            <Field label="ID phase" value={s(phase.value_id)} disabled={props.readOnly} onChange={(value_id) => props.onDefinition({ ...d, dependent_values: [amplitude, { ...phase, value_id }] })} />
            <Field label="Unité de phase" value={s(phase.unit)} disabled={props.readOnly} onChange={(unit) => props.onDefinition({ ...d, dependent_values: [amplitude, { ...phase, unit }], units: { ...objectValue(d.units), [s(phase.value_id)]: unit } })} />
            <label>
              Opération sur la phase
              <select disabled={props.readOnly} value={s(phase.operation || "add")} onChange={(event) => props.onDefinition({ ...d, dependent_values: [amplitude, { ...phase, operation: event.target.value }] })}>
                <option value="add">Ajouter</option>
                <option value="subtract">Soustraire</option>
              </select>
            </label>
          </div>
        )}
        <label>
          Interpolation
          <select disabled={props.readOnly} value={s(d.interpolation)} onChange={(event) => props.onDefinition({ ...d, interpolation: event.target.value })}>
            {["linear_x_linear_y", "log_x_linear_y", "linear_x_log_y", "nearest", "step_previous", "step_next"].map((mode) => <option key={mode} value={mode}>{interpolationLabel(mode)}</option>)}
          </select>
        </label>
        <label>
          Extrapolation
          <select disabled={props.readOnly} value={s(d.extrapolation_policy)} onChange={(event) => props.onDefinition({ ...d, extrapolation_policy: event.target.value })}>
            {["forbidden", "clamp", "warn", "allow"].map((mode) => <option key={mode} value={mode}>{extrapolationLabel(mode)}</option>)}
          </select>
        </label>
      </EditorCard>
    );
  }
  if (props.section === "table") {
    const values = curveValues(d);
    const valueIds = values.map((value) => s(value.value_id));
    return (
      <EditorCard title="Réponse en fréquence">
        <FrequencyCoverage definition={d} />
        <div className="buttonRow">
          <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...d, points: [...curvePoints(d.points), newCurvePoint(valueIds)] })}>Ajouter un point</button>
          <button onClick={props.onExportCurveCsv}><Download size={16} /> Exporter CSV</button>
          <button disabled={props.readOnly} onClick={props.onApplyCurveCsv}><FileSpreadsheet size={16} /> Importer CSV</button>
          <label className="fileButton">
            Fichier CSV
            <input type="file" accept=".csv,text/csv" disabled={props.readOnly} onChange={(event) => event.target.files?.[0] && props.onImportCurveFile(event.target.files[0])} />
          </label>
        </div>
        <textarea value={props.curveCsv} onChange={(event) => props.onCurveCsv(event.target.value)} placeholder={`frequency_hz,${valueIds.join(",")}`} />
        <CurvePlot definition={d} />
        <StructuredTable columns={["Fréquence (Hz)", ...valueIds]}>
          {curvePoints(d.points).map((point, index) => (
            <tr key={index}>
              <td><input disabled={props.readOnly} value={axisNumber(point, "frequency")} onChange={(event) => props.onDefinition({ ...d, points: replaceAt(curvePoints(d.points), index, { ...point, axis_values: { ...objectValue(point.axis_values), frequency: numberOrZero(event.target.value) } }) })} /></td>
              {valueIds.map((valueId) => <td key={valueId}><input disabled={props.readOnly} value={valueNumber(point, valueId)} onChange={(event) => props.onDefinition({ ...d, points: replaceAt(curvePoints(d.points), index, { ...point, values: { ...objectValue(point.values), [valueId]: numberOrZero(event.target.value) } }) })} /></td>)}
            </tr>
          ))}
        </StructuredTable>
      </EditorCard>
    );
  }
  return (
    <EditorCard title="Vérification ponctuelle">
      <Field label="Fréquence (Hz)" value={props.evaluationFrequency} onChange={props.onEvaluationFrequency} />
      <button onClick={props.onEvaluateCurve}><CheckCircle2 size={16} /> Calculer la correction</button>
      {props.curveEvaluation && (
        <dl>
          <dt>Valeurs</dt><dd>{JSON.stringify(props.curveEvaluation.values)}</dd>
          <dt>Interpolation</dt><dd>{props.curveEvaluation.interpolation}</dd>
          <dt>Extrapolation</dt><dd>{props.curveEvaluation.extrapolated ? "Oui" : "Non"}</dd>
          <dt>Source</dt><dd className="mono">{props.curveEvaluation.source_revision_id}</dd>
          <dt>Checksum</dt><dd><code>{props.curveEvaluation.source_checksum}</code></dd>
        </dl>
      )}
    </EditorCard>
  );
}

function DaqSections(props: SectionProps) {
  const d = props.definition;
  if (props.section === "general") {
    return (
      <EditorCard title="Identification">
        <Field label="Identifiant interne" value={s(d.daq_channel_profile_id)} disabled onChange={() => undefined} />
        <Field label="Nom de la voie" value={s(d.label)} disabled={props.readOnly} onChange={(label) => props.onDefinition({ ...d, label })} />
        <label>
          Type de voie
          <select disabled={props.readOnly} value={s(d.channel_kind)} onChange={(event) => props.onDefinition({ ...d, channel_kind: event.target.value })}>
            {channelKinds.map((kind) => <option key={kind} value={kind}>{humanMeasurementLabel(kind)}</option>)}
          </select>
        </label>
        <label>
          Domaine du signal
          <select disabled={props.readOnly} value={s(d.signal_domain)} onChange={(event) => props.onDefinition({ ...d, signal_domain: event.target.value })}>
            {signalDomains.map((domain) => <option key={domain} value={domain}>{humanMeasurementLabel(domain)}</option>)}
          </select>
        </label>
      </EditorCard>
    );
  }
  if (props.section === "daq") {
    return (
      <EditorCard title="Mode d’entrée / sortie">
        <QuantitySelect label="Grandeur d’entrée" value={s(d.input_quantity)} disabled={props.readOnly} onChange={(input_quantity) => props.onDefinition({ ...d, input_quantity })} />
        <Field label="Unité d’entrée" value={s(d.input_unit)} disabled={props.readOnly} onChange={(input_unit) => props.onDefinition({ ...d, input_unit })} />
        <Field label="Modes d’entrée" value={stringArray(d.input_modes).join(", ")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, input_modes: splitTokens(value) })} />
        <Field label="Couplages" value={stringArray(d.coupling_modes).join(", ")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, coupling_modes: splitTokens(value) })} />
      </EditorCard>
    );
  }
  if (props.section === "physical") {
    return (
      <EditorCard title="Plages électriques">
        <SupportedRangeTable ranges={supportedRanges(d.supported_ranges)} readOnly={props.readOnly} onRanges={(supported_ranges) => props.onDefinition({ ...d, supported_ranges })} />
      </EditorCard>
    );
  }
  if (props.section === "sampling") {
    return (
      <EditorCard title="Échantillonnage">
        <Field label="Résolution (bits)" value={s(d.resolution_bits)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, resolution_bits: optionalInteger(value) })} />
        <Field label="Fréquence minimale (éch/s)" value={s(d.min_sampling_rate)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, min_sampling_rate: optionalNumber(value) })} />
        <Field label="Fréquence maximale (éch/s)" value={s(d.max_sampling_rate)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, max_sampling_rate: optionalNumber(value) })} />
        <Field label="Synchronisation" value={s(d.synchronization)} disabled={props.readOnly} onChange={(synchronization) => props.onDefinition({ ...d, synchronization: optionalString(synchronization) })} />
        <Field label="Déclenchement" value={s(d.triggering)} disabled={props.readOnly} onChange={(triggering) => props.onDefinition({ ...d, triggering: optionalString(triggering) })} />
      </EditorCard>
    );
  }
  return <ExcitationEditor definition={d} readOnly={props.readOnly} onDefinition={props.onDefinition} field="excitation_capabilities" list />;
}

function RecipeSections(props: SectionProps & { approvedOptions: ApprovedOptions }) {
  const d = props.definition;
  if (props.section === "general") {
    return (
      <EditorCard title="Identification">
        <Field label="Identifiant interne" value={s(d.recipe_id)} disabled onChange={() => undefined} />
        <Field label="Nom de la chaîne" value={s(d.label)} disabled={props.readOnly} onChange={(label) => props.onDefinition({ ...d, label })} />
        <Field label="Nom du canal résultat" value={s(d.output_channel_name)} disabled={props.readOnly} onChange={(output_channel_name) => props.onDefinition({ ...d, output_channel_name })} />
        <QuantitySelect label="Grandeur de sortie" value={s(d.output_quantity)} disabled={props.readOnly} onChange={(output_quantity) => props.onDefinition({ ...d, output_quantity })} />
        <Field label="Unité de sortie" value={s(d.output_unit)} disabled={props.readOnly} onChange={(output_unit) => props.onDefinition({ ...d, output_unit })} />
      </EditorCard>
    );
  }
  if (props.section === "chain") {
    return (
      <EditorCard title="Chaîne de mesure">
        <ChainSummary definition={d} />
        <ReferenceSelect label="Voie DAQ" value={refText(d.daq_channel_profile_ref)} options={props.approvedOptions.daq} readOnly={props.readOnly} onValue={(value) => props.onDefinition({ ...d, daq_channel_profile_ref: refFromText(value) })} />
        <ReferenceSelect label="Capteur / transducteur" value={refText(d.sensor_definition_ref)} options={props.approvedOptions.sensors} readOnly={props.readOnly} onValue={(value) => props.onDefinition({ ...d, sensor_definition_ref: value ? refFromText(value) : undefined })} />
        <ReferenceSelect label="Conversion temporelle" value={refText(d.scaling_profile_ref)} options={props.approvedOptions.scalings} readOnly={props.readOnly} onValue={(value) => props.onDefinition({ ...d, scaling_profile_ref: value ? refFromText(value) : undefined })} />
        <ReferenceEditor title="Réponses fréquentielles" refs={refs(d.correction_curve_refs)} options={props.approvedOptions.curves} readOnly={props.readOnly} onRefs={(correction_curve_refs) => props.onDefinition({ ...d, correction_curve_refs })} />
      </EditorCard>
    );
  }
  return (
    <EditorCard title="Échantillonnage et plage">
      <Field label="Fréquence d’échantillonnage (éch/s)" value={s(d.sample_rate)} disabled={props.readOnly} onChange={(sample_rate) => props.onDefinition({ ...d, sample_rate: numberOrZero(sample_rate) })} />
      <SupportedRangeTable ranges={[supportedRange(d.range)]} readOnly={props.readOnly} onRanges={(ranges) => props.onDefinition({ ...d, range: ranges[0] })} />
      <label>
        Coupling
        <select disabled={props.readOnly} value={s(d.coupling)} onChange={(event) => props.onDefinition({ ...d, coupling: event.target.value })}>
          {["dc", "ac", "gnd"].map((mode) => <option key={mode} value={mode}>{mode}</option>)}
        </select>
      </label>
      <label>
        Input mode
        <select disabled={props.readOnly} value={s(d.input_mode)} onChange={(event) => props.onDefinition({ ...d, input_mode: event.target.value })}>
          {inputModes.map((mode) => <option key={mode} value={mode}>{mode}</option>)}
        </select>
      </label>
    </EditorCard>
  );
}

function MeasurementRevisionTable(props: {
  revisions: MeasurementEngineeringRevision[];
  onOpen: (revision: MeasurementEngineeringRevision) => void;
}) {
  return (
    <EditorCard title="Revisions">
      <StructuredTable columns={["No", "Revision", "Status", "Parent", "Checksum", "Created", "Submitted", "Approved", ""]}>
        {props.revisions.map((revision) => (
          <tr key={revision.revision_id}>
            <td>{revision.revision_number}</td>
            <td className="mono">{revision.revision_id}</td>
            <td>{revision.status}</td>
            <td>{revision.parent_revision_id ?? "-"}</td>
            <td><code>{revision.definition_checksum}</code></td>
            <td>{formatDate(revision.created_at)}</td>
            <td>{formatDate(revision.submitted_at)}</td>
            <td>{formatDate(revision.approved_at)}</td>
            <td><button onClick={() => props.onOpen(revision)}>Ouvrir</button></td>
          </tr>
        ))}
      </StructuredTable>
    </EditorCard>
  );
}

interface SectionProps {
  section: StudioSection;
  definition: MeasurementEngineeringDefinition;
  readOnly: boolean;
  onDefinition: (definition: MeasurementEngineeringDefinition) => void;
}

interface ApprovedOptions {
  sensors: DefinitionOption[];
  scalings: DefinitionOption[];
  curves: DefinitionOption[];
  daq: DefinitionOption[];
}

interface DefinitionOption {
  entity_id: string;
  revision_id: string;
  label: string;
}

function QuantitySelect(props: {
  label: string;
  value: string;
  disabled?: boolean;
  onChange: (value: string) => void;
}) {
  return (
    <label>
      {props.label}
      <select disabled={props.disabled} value={props.value} onChange={(event) => props.onChange(event.target.value)}>
        {quantities.map((quantity) => <option key={quantity} value={quantity}>{humanMeasurementLabel(quantity)}</option>)}
      </select>
    </label>
  );
}

function RangeFields(props: {
  title: string;
  range: Record<string, unknown>;
  unitFallback?: string;
  readOnly: boolean;
  onRange: (range: Record<string, unknown>) => void;
}) {
  return (
    <fieldset className="measurementFieldset">
      <legend>{props.title}</legend>
      <Field label="Minimum" value={s(props.range.minimum)} disabled={props.readOnly} onChange={(value) => props.onRange({ ...props.range, minimum: optionalNumber(value) })} />
      <Field label="Maximum" value={s(props.range.maximum)} disabled={props.readOnly} onChange={(value) => props.onRange({ ...props.range, maximum: optionalNumber(value) })} />
      <Field label="Unité" value={s(props.range.unit ?? props.unitFallback)} disabled={props.readOnly} onChange={(unit) => props.onRange({ ...props.range, unit })} />
    </fieldset>
  );
}

function FrequencyRangeFields(props: {
  range: { minimum_hz: number; maximum_hz: number };
  readOnly: boolean;
  onRange: (range: { minimum_hz: number; maximum_hz: number }) => void;
}) {
  return (
    <fieldset className="measurementFieldset">
      <legend>Domaine fréquentiel</legend>
      <Field label="Fmin (Hz)" value={s(props.range.minimum_hz)} disabled={props.readOnly} onChange={(value) => props.onRange({ ...props.range, minimum_hz: numberOrZero(value) })} />
      <Field label="Fmax (Hz)" value={s(props.range.maximum_hz)} disabled={props.readOnly} onChange={(value) => props.onRange({ ...props.range, maximum_hz: numberOrZero(value) })} />
    </fieldset>
  );
}

function SupportedRangeTable(props: {
  ranges: SupportedRange[];
  readOnly: boolean;
  onRanges: (ranges: SupportedRange[]) => void;
}) {
  return (
    <>
      <button disabled={props.readOnly} onClick={() => props.onRanges([...props.ranges, { minimum: -10, maximum: 10, unit: "V" }])}>Ajouter une plage</button>
      <StructuredTable columns={["Minimum", "Maximum", "Unité"]}>
        {props.ranges.map((range, index) => (
          <tr key={index}>
            <td><input disabled={props.readOnly} value={range.minimum} onChange={(event) => props.onRanges(replaceAt(props.ranges, index, { ...range, minimum: numberOrZero(event.target.value) }))} /></td>
            <td><input disabled={props.readOnly} value={range.maximum} onChange={(event) => props.onRanges(replaceAt(props.ranges, index, { ...range, maximum: numberOrZero(event.target.value) }))} /></td>
            <td><input disabled={props.readOnly} value={range.unit} onChange={(event) => props.onRanges(replaceAt(props.ranges, index, { ...range, unit: event.target.value }))} /></td>
          </tr>
        ))}
      </StructuredTable>
    </>
  );
}

function ExcitationEditor(props: {
  definition: MeasurementEngineeringDefinition;
  readOnly: boolean;
  field: "required_excitation" | "excitation_capabilities";
  list?: boolean;
  onDefinition: (definition: MeasurementEngineeringDefinition) => void;
}) {
  const excitations = props.list
    ? excitationList(props.definition[props.field])
    : [objectValue(props.definition[props.field])];
  const update = (next: Record<string, unknown>, index: number) => {
    if (props.list) {
      props.onDefinition({
        ...props.definition,
        [props.field]: replaceAt(excitations, index, next)
      });
    } else {
      props.onDefinition({ ...props.definition, [props.field]: next });
    }
  };
  return (
    <EditorCard title="Alimentation et conditionnement du capteur">
      <div className="domainBanner">
        <strong>Ce n’est pas le signal d’essai</strong>
        <span>Il s’agit de l’énergie ou du conditionnement nécessaire au capteur : courant IEPE, tension de pont ou amplificateur de charge.</span>
      </div>
      {props.list && <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...props.definition, [props.field]: [...excitations, defaultExcitation("iepe")] })}>Ajouter une alimentation</button>}
      {excitations.map((excitation, index) => (
        <fieldset className="measurementFieldset" key={index}>
          <legend>{props.list ? `Alimentation ${index + 1}` : "Besoin du capteur"}</legend>
          <label>
            Type
            <select disabled={props.readOnly} value={s(excitation.excitation_kind ?? "none")} onChange={(event) => update({ ...excitation, excitation_kind: event.target.value }, index)}>
              {["none", "external", "voltage", "current", "iepe", "bridge", "charge"].map((kind) => <option key={kind} value={kind}>{excitationKindLabel(kind)}</option>)}
            </select>
          </label>
          <Field label="Valeur nominale" value={s(excitation.nominal_value)} disabled={props.readOnly} onChange={(nominal_value) => update({ ...excitation, nominal_value: optionalNumber(nominal_value) }, index)} />
          <Field label="Unité" value={s(excitation.unit)} disabled={props.readOnly} onChange={(unit) => update({ ...excitation, unit: optionalString(unit) }, index)} />
          <label className="checkboxLabel">
            <input type="checkbox" disabled={props.readOnly} checked={Boolean(excitation.external_allowed)} onChange={(event) => update({ ...excitation, external_allowed: event.target.checked }, index)} />
            Une source externe est autorisée
          </label>
        </fieldset>
      ))}
    </EditorCard>
  );
}

function ReferenceEditor(props: {
  title: string;
  refs: DefinitionReference[];
  options: DefinitionOption[];
  readOnly: boolean;
  onRefs: (refs: DefinitionReference[]) => void;
}) {
  const [selected, setSelected] = useState("");
  return (
    <EditorCard title={props.title}>
      <div className="buttonRow">
        <select disabled={props.readOnly} value={selected} onChange={(event) => setSelected(event.target.value)}>
          <option value="">Définition approuvée...</option>
          {props.options.map((option) => (
            <option key={`${option.entity_id}:${option.revision_id}`} value={refText(option)}>
              {option.label} | {option.entity_id}
            </option>
          ))}
        </select>
        <button disabled={props.readOnly || !selected} onClick={() => {
          props.onRefs([...props.refs, refFromText(selected)]);
          setSelected("");
        }}>Ajouter</button>
      </div>
      <StructuredTable columns={["Définition", "Révision", "Approbation requise", ""]}>
        {props.refs.map((ref, index) => (
          <tr key={`${ref.entity_id}-${index}`}>
            <td>{ref.entity_id}</td>
            <td>{ref.revision_id ?? "-"}</td>
            <td>{ref.require_approved ? "Oui" : "Non"}</td>
            <td><button disabled={props.readOnly} onClick={() => props.onRefs(props.refs.filter((_, itemIndex) => itemIndex !== index))}>Retirer</button></td>
          </tr>
        ))}
      </StructuredTable>
    </EditorCard>
  );
}

function ReferenceSelect(props: {
  label: string;
  value: string;
  options: DefinitionOption[];
  readOnly: boolean;
  onValue: (value: string) => void;
}) {
  return (
    <label>
      {props.label}
      <select disabled={props.readOnly} value={props.value} onChange={(event) => props.onValue(event.target.value)}>
        <option value="">Aucune</option>
        {props.options.map((option) => (
          <option value={refText(option)} key={`${option.entity_id}:${option.revision_id}`}>
            {option.label} | {option.entity_id}
          </option>
        ))}
      </select>
    </label>
  );
}

function ChainSummary(props: { definition: MeasurementEngineeringDefinition }) {
  return (
    <div className="chainSummary">
      <span>Voie DAQ<br /><strong>{refText(props.definition.daq_channel_profile_ref)}</strong></span>
      <span>Signal électrique du capteur<br /><strong>{refText(props.definition.sensor_definition_ref)}</strong></span>
      <span>Conversion temporelle<br /><strong>{refText(props.definition.scaling_profile_ref)}</strong></span>
      <span>Réponse fréquentielle<br /><strong>{refs(props.definition.correction_curve_refs).map(refText).join(", ") || "-"}</strong></span>
      <span>Résultat physique<br /><strong>{s(props.definition.output_channel_name)} [{s(props.definition.output_unit)}]</strong></span>
    </div>
  );
}

function FrequencyCoverage(props: { definition: MeasurementEngineeringDefinition }) {
  const points = curvePoints(props.definition.points);
  const frequencies = points.map((point) => axisNumber(point, "frequency")).filter(Number.isFinite);
  const values = curveValues(props.definition);
  const amplitude = values.find((value) => s(value.component || "amplitude") === "amplitude");
  const phase = values.find((value) => s(value.component) === "phase");
  if (frequencies.length === 0) return null;
  return (
    <dl className="frequencyCoverage">
      <div><dt>Fmin</dt><dd>{Math.min(...frequencies).toLocaleString("fr-FR")} Hz</dd></div>
      <div><dt>Fmax</dt><dd>{Math.max(...frequencies).toLocaleString("fr-FR")} Hz</dd></div>
      <div><dt>Amplitude</dt><dd>{amplitude ? correctionOperationLabel(s(amplitude.operation || "add")) : "Non définie"}</dd></div>
      <div><dt>Phase</dt><dd>{phase ? "Compensée" : "Non utilisée"}</dd></div>
    </dl>
  );
}

function CurvePlot(props: { definition: MeasurementEngineeringDefinition }) {
  const points = curvePoints(props.definition.points);
  const valueId = s(curveValues(props.definition)[0]?.value_id ?? "correction_db");
  if (points.length < 2) {
    return <div className="notice">At least two points are needed for the 1D plot.</div>;
  }
  const xs = points.map((point) => axisNumber(point, "frequency"));
  const ys = points.map((point) => valueNumber(point, valueId));
  const minX = Math.min(...xs);
  const maxX = Math.max(...xs);
  const minY = Math.min(...ys);
  const maxY = Math.max(...ys);
  const path = points
    .map((point) => {
      const x = scale(axisNumber(point, "frequency"), minX, maxX, 10, 390);
      const y = scale(valueNumber(point, valueId), minY, maxY, 110, 10);
      return `${x},${y}`;
    })
    .join(" ");
  return (
    <svg className="curvePlot" viewBox="0 0 400 120" role="img" aria-label="1D curve plot">
      <line x1="10" y1="110" x2="390" y2="110" />
      <line x1="10" y1="10" x2="10" y2="110" />
      <polyline points={path} />
    </svg>
  );
}

function defaultMeasurementDefinition(
  space: MeasurementSpace,
  entityId: string,
  label: string,
  approved: ApprovedOptions
): MeasurementEngineeringDefinition {
  if (space === "sensors") return defaultSensor(entityId, label, approved);
  if (space === "scaling") return defaultScaling(entityId, label);
  if (space === "curves") return defaultCurve(entityId, label);
  if (space === "daq") return defaultDaq(entityId, label);
  return defaultRecipe(entityId, label, approved);
}

function defaultSensor(entityId: string, label: string, approved: ApprovedOptions): MeasurementEngineeringDefinition {
  const scaling = approved.scalings[0];
  const curve = approved.curves.find((item) => item.entity_id.includes("CURRENT")) ?? approved.curves[0];
  return {
    definition_schema_version: "emc-locus.sensor-definition.v1",
    sensor_definition_id: entityId,
    manufacturer: "Demo",
    model_name: label,
    sensor_family: "current_probe",
    physical_input_quantity: "current",
    engineering_output_quantity: "current",
    engineering_output_unit: "A",
    electrical_output_quantity: "voltage",
    electrical_output_unit: "V",
    signal_domain: "analog_voltage",
    technology_tags: ["voltage_input"],
    required_excitation: defaultExcitation("none"),
    input_mode_requirement: "differential",
    nominal_range: { minimum: -100, maximum: 100, unit: "A" },
    safe_range: { minimum: -200, maximum: 200, unit: "A" },
    orientation_axes: [],
    settling_time_ms: 1,
    frequency_range: { minimum_hz: 10, maximum_hz: 100000000 },
    scaling_profile_refs: scaling ? [refFromOption(scaling)] : [],
    correction_curve_refs: curve ? [refFromOption(curve)] : [],
    metadata: { created_from: "lab_console" }
  };
}

function defaultScaling(entityId: string, label: string): MeasurementEngineeringDefinition {
  return {
    definition_schema_version: "emc-locus.scaling-profile-definition.v1",
    scaling_profile_id: entityId,
    label,
    input_quantity: "voltage",
    input_unit: "V",
    output_quantity: "current",
    output_unit: "A",
    signal_representation: "time_domain_samples",
    scaling_kind: "linear",
    parameters: { scale: 100, offset: 0 },
    input_limits: { minimum: -10, maximum: 10, handling: "warn" },
    validity_domain: {},
    source_reference: "lab-console",
    metadata: { created_from: "lab_console" }
  };
}

function defaultCurve(entityId: string, label: string): MeasurementEngineeringDefinition {
  return {
    definition_schema_version: "emc-locus.engineering-curve-definition.v1",
    curve_id: entityId,
    curve_type: "cable_loss",
    label,
    signal_representation: "frequency_domain_spectrum",
    independent_axes: [{ axis: "frequency", quantity: "frequency", unit: "Hz" }],
    dependent_values: [{ value_id: "amplitude_correction_db", quantity: "dimensionless", unit: "dB", component: "amplitude", operation: "add" }],
    units: { frequency: "Hz", amplitude_correction_db: "dB" },
    points: [
      { axis_values: { frequency: 10000000 }, values: { amplitude_correction_db: 0.2 } },
      { axis_values: { frequency: 100000000 }, values: { amplitude_correction_db: 1.0 } },
      { axis_values: { frequency: 1000000000 }, values: { amplitude_correction_db: 3.0 } }
    ],
    interpolation: "log_x_linear_y",
    extrapolation_policy: "warn",
    validity_domain: {},
    conditions: {},
    source_document_reference: "lab-console",
    source_checksum: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    metadata: { created_from: "lab_console" }
  };
}

function defaultDaq(entityId: string, label: string): MeasurementEngineeringDefinition {
  return {
    definition_schema_version: "emc-locus.daq-channel-profile-definition.v1",
    daq_channel_profile_id: entityId,
    label,
    channel_kind: "analog_input",
    signal_domain: "analog_voltage",
    input_quantity: "voltage",
    input_unit: "V",
    supported_ranges: [{ minimum: -10, maximum: 10, unit: "V" }],
    resolution_bits: 16,
    max_sampling_rate: 1000000,
    min_sampling_rate: 1,
    coupling_modes: ["dc", "ac"],
    input_modes: ["single_ended", "differential", "iepe"],
    anti_alias_filter: "available",
    excitation_capabilities: [defaultExcitation("iepe", 4, "mA")],
    iepe_support: true,
    synchronization: "shared_sample_clock",
    triggering: "digital_start_trigger",
    metadata: { created_from: "lab_console" }
  };
}

function defaultRecipe(entityId: string, label: string, approved: ApprovedOptions): MeasurementEngineeringDefinition {
  const daq = approved.daq[0];
  const sensor = approved.sensors[0];
  const scaling = approved.scalings[0];
  return {
    definition_schema_version: "emc-locus.acquisition-channel-recipe-definition.v1",
    recipe_id: entityId,
    label,
    output_channel_name: "current_A",
    output_quantity: "current",
    output_unit: "A",
    daq_channel_profile_ref: daq ? refFromOption(daq) : { entity_id: "", require_approved: true },
    sensor_definition_ref: sensor ? refFromOption(sensor) : undefined,
    scaling_profile_ref: scaling ? refFromOption(scaling) : undefined,
    correction_curve_refs: [],
    sample_rate: 1000000,
    range: { minimum: -10, maximum: 10, unit: "V" },
    coupling: "dc",
    input_mode: "differential",
    excitation: defaultExcitation("none"),
    filtering: "anti_alias_on",
    triggering: "software",
    validation_rules: ["range_within_daq", "sample_rate_within_daq"],
    metadata: { created_from: "lab_console" }
  };
}

function defaultExcitation(kind: string, nominal_value?: number, unit?: string) {
  return { excitation_kind: kind, nominal_value, unit, external_allowed: false };
}

function approvedRefs(items: MeasurementEngineeringAggregate[]): DefinitionOption[] {
  return items
    .map((item) => item.current_approved_revision)
    .filter((revision): revision is MeasurementEngineeringRevision => Boolean(revision))
    .map((revision) => ({
      entity_id: revision.entity_id,
      revision_id: revision.revision_id,
      label: revision.label
    }));
}

function applyLookupCsv(
  definition: MeasurementEngineeringDefinition,
  csv: string,
  onDefinition: (definition: MeasurementEngineeringDefinition) => void,
  onError: (message: string | null) => void
) {
  try {
    const points = parseTwoColumnCsv(csv, "input", "output").map(([input, output]) => ({ input, output }));
    onDefinition({
      ...definition,
      scaling_kind: "lookup_table",
      parameters: {
        ...objectValue(definition.parameters),
        points,
        interpolation: "linear",
        extrapolation_policy: "warn"
      }
    });
  } catch (error) {
    onError(errorMessage(error));
  }
}

function exportLookupCsv(definition: MeasurementEngineeringDefinition) {
  return [
    "input,output",
    ...scalingPoints(objectValue(definition.parameters).points)
      .sort((a, b) => a.input - b.input)
      .map((point) => `${point.input},${point.output}`)
  ].join("\n");
}

function applyCurveCsv(
  definition: MeasurementEngineeringDefinition,
  csv: string,
  onDefinition: (definition: MeasurementEngineeringDefinition) => void,
  onError: (message: string | null) => void
) {
  try {
    const valueIds = curveValues(definition).map((value) => s(value.value_id));
    const points = parseFrequencyResponseCsv(csv, valueIds);
    onDefinition({ ...definition, points: points.sort((a, b) => a.axis_values.frequency - b.axis_values.frequency) });
  } catch (error) {
    onError(errorMessage(error));
  }
}

function exportCurveCsv(definition: MeasurementEngineeringDefinition) {
  const valueIds = curveValues(definition).map((value) => s(value.value_id));
  return [
    `frequency_hz,${valueIds.join(",")}`,
    ...curvePoints(definition.points)
      .sort((a, b) => axisNumber(a, "frequency") - axisNumber(b, "frequency"))
      .map((point) => [axisNumber(point, "frequency"), ...valueIds.map((valueId) => valueNumber(point, valueId))].join(","))
  ].join("\n");
}

function parseFrequencyResponseCsv(csv: string, valueIds: string[]): CurvePoint[] {
  const lines = csv
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  if (lines.length < 2) throw new Error("Le CSV doit contenir un en-tête et au moins une ligne.");
  const headers = lines[0].split(",").map((item) => item.trim());
  const frequencyIndex = headers.indexOf("frequency_hz");
  const valueIndexes = valueIds.map((valueId) => headers.indexOf(valueId));
  if (frequencyIndex < 0 || valueIndexes.some((index) => index < 0)) {
    throw new Error(`L’en-tête CSV doit contenir frequency_hz et ${valueIds.join(", ")}.`);
  }
  const seen = new Set<number>();
  return lines.slice(1).map((line, rowIndex) => {
    const cells = line.split(",").map((item) => item.trim());
    const frequency = Number(cells[frequencyIndex]);
    const values = Object.fromEntries(valueIds.map((valueId, index) => [valueId, Number(cells[valueIndexes[index]])]));
    if (!Number.isFinite(frequency) || Object.values(values).some((value) => !Number.isFinite(value))) {
      throw new Error(`La ligne CSV ${rowIndex + 2} contient une valeur non numérique.`);
    }
    if (seen.has(frequency)) throw new Error(`La ligne CSV ${rowIndex + 2} duplique la fréquence ${frequency} Hz.`);
    seen.add(frequency);
    return { axis_values: { frequency }, values };
  });
}

function parseTwoColumnCsv(csv: string, expectedFirst: string, expectedSecond: string): Array<[number, number]> {
  const lines = csv
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  if (lines.length < 2) throw new Error("CSV must include a header and at least one row.");
  const headers = lines[0].split(",").map((item) => item.trim());
  const firstIndex = headers.indexOf(expectedFirst);
  const secondIndex = headers.indexOf(expectedSecond);
  if (firstIndex < 0 || secondIndex < 0) {
    throw new Error(`CSV header must include ${expectedFirst} and ${expectedSecond}.`);
  }
  const seen = new Set<number>();
  return lines.slice(1).map((line, rowIndex) => {
    const cells = line.split(",").map((item) => item.trim());
    const first = Number(cells[firstIndex]);
    const second = Number(cells[secondIndex]);
    if (!Number.isFinite(first) || !Number.isFinite(second)) {
      throw new Error(`CSV row ${rowIndex + 2} contains a non-numeric value.`);
    }
    if (seen.has(first)) {
      throw new Error(`CSV row ${rowIndex + 2} duplicates x=${first}.`);
    }
    seen.add(first);
    return [first, second];
  });
}

function curveAxes(definition: MeasurementEngineeringDefinition) {
  return Array.isArray(definition.independent_axes) ? (definition.independent_axes as Array<Record<string, unknown>>) : [];
}

function curveValues(definition: MeasurementEngineeringDefinition) {
  return Array.isArray(definition.dependent_values) ? (definition.dependent_values as Array<Record<string, unknown>>) : [];
}

function curvePoints(value: unknown): CurvePoint[] {
  return Array.isArray(value) ? (value as CurvePoint[]) : [];
}

function scalingPoints(value: unknown): ScalingPoint[] {
  return Array.isArray(value) ? (value as ScalingPoint[]) : [];
}

function supportedRanges(value: unknown): SupportedRange[] {
  return Array.isArray(value) ? (value as SupportedRange[]) : [];
}

function supportedRange(value: unknown): SupportedRange {
  const range = objectValue(value);
  return {
    minimum: Number(range.minimum ?? -10),
    maximum: Number(range.maximum ?? 10),
    unit: s(range.unit || "V")
  };
}

function frequencyRange(value: unknown) {
  const range = objectValue(value);
  return {
    minimum_hz: Number(range.minimum_hz ?? 0),
    maximum_hz: Number(range.maximum_hz ?? 1000000)
  };
}

function axisNumber(point: CurvePoint, axis: string) {
  return Number(objectValue(point.axis_values)[axis] ?? 0);
}

function valueNumber(point: CurvePoint, valueId: string) {
  return Number(objectValue(point.values)[valueId] ?? 0);
}

function newCurvePoint(valueIds: string[]): CurvePoint {
  return {
    axis_values: { frequency: 1000000 },
    values: Object.fromEntries(valueIds.map((valueId) => [valueId, 0]))
  };
}

function refs(value: unknown): DefinitionReference[] {
  if (!Array.isArray(value)) return [];
  return value as DefinitionReference[];
}

function refFromOption(option: DefinitionOption): DefinitionReference {
  return { entity_id: option.entity_id, revision_id: option.revision_id, require_approved: true };
}

function refText(value: unknown): string {
  if (!value) return "";
  const ref = value as Partial<DefinitionReference & DefinitionOption>;
  if (!ref.entity_id) return "";
  return `${ref.entity_id}@${ref.revision_id ?? ""}`;
}

function refFromText(value: string): DefinitionReference {
  const [entity_id, revision_id] = value.split("@");
  return { entity_id, revision_id: optionalString(revision_id), require_approved: true };
}

function excitationList(value: unknown): Array<Record<string, unknown>> {
  return Array.isArray(value) ? (value as Array<Record<string, unknown>>) : [];
}

function objectValue(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
}

function numberArray(value: unknown): number[] {
  return Array.isArray(value) ? value.map(Number).filter(Number.isFinite) : [];
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.map(String) : [];
}

function numberList(value: string): number[] {
  return splitTokens(value).map(Number).filter(Number.isFinite);
}

function splitTokens(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function optionalString(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

function optionalNumber(value: string) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function optionalInteger(value: string) {
  const parsed = Number(value);
  return Number.isInteger(parsed) ? parsed : undefined;
}

function numberOrZero(value: string) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function replaceAt<T>(items: T[], index: number, value: T) {
  return items.map((item, itemIndex) => (itemIndex === index ? value : item));
}

function s(value: unknown): string {
  if (value === null || typeof value === "undefined") return "";
  return String(value);
}

function humanMeasurementLabel(value: string) {
  const labels: Record<string, string> = {
    current_probe: "Pince / sonde de courant",
    voltage_probe: "Sonde de tension",
    field_probe: "Sonde de champ",
    receiving_antenna: "Antenne de réception",
    transmitting_antenna: "Antenne d’émission",
    accelerometer: "Accéléromètre",
    microphone: "Microphone",
    thermocouple: "Thermocouple",
    pressure_sensor: "Capteur de pression",
    photodiode: "Photodiode",
    strain_gauge: "Jauge de contrainte",
    generic_transducer: "Transducteur générique",
    manual_transducer: "Transducteur manuel",
    analog_input: "Entrée analogique",
    analog_output: "Sortie analogique",
    digital_input: "Entrée numérique",
    digital_output: "Sortie numérique",
    digital_bidirectional: "Entrée / sortie numérique",
    counter_input: "Entrée compteur",
    frequency_input: "Entrée fréquencemètre",
    trigger_input: "Entrée de déclenchement",
    trigger_output: "Sortie de déclenchement",
    can_bus_channel: "Voie bus CAN",
    software_channel: "Canal logiciel",
    single_ended: "Asymétrique",
    differential: "Différentiel",
    pseudo_differential: "Pseudo-différentiel",
    current_loop: "Boucle de courant",
    bridge_quarter: "Quart de pont",
    bridge_half: "Demi-pont",
    bridge_full: "Pont complet",
    analog_voltage: "Tension analogique",
    analog_current: "Courant analogique",
    analog_charge: "Charge analogique",
    digital_logic: "Logique numérique",
    mechanical: "Mécanique",
    environmental: "Environnement",
    software: "Données logicielles"
  };
  return labels[value] ?? value.replaceAll("_", " ");
}

function scalingKindLabel(value: string) {
  return {
    identity: "Identité",
    linear: "Gain et offset",
    two_point: "Étalonnage en deux points",
    polynomial: "Polynôme",
    lookup_table: "Table de correspondance",
    piecewise_linear: "Linéaire par morceaux",
    expression: "Expression contrôlée"
  }[value] ?? humanMeasurementLabel(value);
}

function curveTypeLabel(value: string) {
  return {
    antenna_factor: "Facteur d’antenne",
    cable_loss: "Pertes de câble",
    amplifier_gain: "Gain d’amplificateur",
    attenuator_loss: "Pertes d’atténuateur",
    current_probe_transfer: "Transfert de sonde de courant",
    voltage_probe_transfer: "Transfert de sonde de tension",
    sensor_frequency_response: "Réponse fréquentielle de capteur",
    phase_response: "Réponse de phase",
    linearity_correction: "Correction de linéarité",
    uncertainty: "Incertitude",
    vswr: "ROS / VSWR",
    s_parameter_magnitude: "Module de paramètre S",
    site_characterization: "Caractérisation de site",
    generic_correction: "Correction générique"
  }[value] ?? humanMeasurementLabel(value);
}

function correctionOperationLabel(value: string) {
  return {
    add: "Ajouter à la mesure",
    subtract: "Soustraire de la mesure",
    multiply: "Multiplier la mesure",
    divide: "Diviser la mesure"
  }[value] ?? humanMeasurementLabel(value);
}

function interpolationLabel(value: string) {
  return {
    linear_x_linear_y: "Linéaire en fréquence et en valeur",
    log_x_linear_y: "Logarithmique en fréquence",
    linear_x_log_y: "Logarithmique en valeur",
    nearest: "Point le plus proche",
    step_previous: "Palier précédent",
    step_next: "Palier suivant"
  }[value] ?? humanMeasurementLabel(value);
}

function extrapolationLabel(value: string) {
  return {
    forbidden: "Interdite hors bande",
    clamp: "Borner à Fmin / Fmax",
    warn: "Autoriser avec avertissement",
    allow: "Autoriser"
  }[value] ?? humanMeasurementLabel(value);
}

function excitationKindLabel(value: string) {
  return {
    none: "Aucune alimentation",
    external: "Conditionnement externe",
    voltage: "Tension constante",
    current: "Courant constant",
    iepe: "Courant IEPE / ICP",
    bridge: "Alimentation de pont",
    charge: "Amplificateur de charge"
  }[value] ?? humanMeasurementLabel(value);
}

function generatedEntityId(space: MeasurementSpace) {
  const prefix = {
    sensors: "SNS",
    scaling: "SCL",
    curves: "CURVE",
    daq: "DAQ",
    recipes: "REC"
  }[space];
  return `${prefix}-LAB-${Date.now().toString(36).toUpperCase()}`;
}

function defaultLabel(space: MeasurementSpace, entityId: string) {
  const label = {
    sensors: "Pince de courant 10 mV/A",
    scaling: "Conversion 10 mV/A",
    curves: "Pertes du câble RF",
    daq: "Entrée analogique +/-10 V",
    recipes: "Chaîne courant_A"
  }[space];
  return `${label} ${entityId.slice(-4)}`;
}

function formatDate(value?: string | null) {
  return value ? value.replace("T", " ").replace("Z", "") : "-";
}

function scale(value: number, inMin: number, inMax: number, outMin: number, outMax: number) {
  if (inMax === inMin) return (outMin + outMax) / 2;
  return outMin + ((value - inMin) / (inMax - inMin)) * (outMax - outMin);
}

async function readFile(file: File): Promise<string> {
  return file.text();
}

function errorMessage(error: unknown) {
  if (error instanceof ApiError) {
    return `${error.code}: ${error.message}`;
  }
  if (error instanceof Error) return error.message;
  return String(error);
}

interface DefinitionReference {
  entity_id: string;
  revision_id?: string;
  require_approved: boolean;
}

interface SupportedRange {
  minimum: number;
  maximum: number;
  unit: string;
}

interface ScalingPoint {
  input: number;
  output: number;
}

interface CurvePoint {
  axis_values: Record<string, number>;
  values: Record<string, number>;
}
