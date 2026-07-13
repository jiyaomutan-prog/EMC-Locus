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
    title: "Capteurs / transducteurs",
    listTitle: "Definitions capteurs",
    createLabel: "Creer capteur",
    emptyDetail: "Selectionnez ou creez une definition de capteur/transducteur.",
    sections: [
      ["general", "General"],
      ["physical", "Entree physique"],
      ["electrical", "Sortie electrique"],
      ["excitation", "Excitation"],
      ["scaling", "Scaling"],
      ["corrections", "Courbes de correction"],
      ["revisions", "Revisions"],
      ["audit", "Audit"],
      ["json", "Diagnostic JSON"]
    ]
  },
  scaling: {
    key: "scaling",
    collection: "scaling-profiles",
    validationCollection: "scaling-profile-definitions",
    operationPrefix: "scaling-profile",
    title: "Profils de scaling",
    listTitle: "Profils de scaling",
    createLabel: "Creer scaling",
    emptyDetail: "Selectionnez ou creez un profil de mise a l'echelle.",
    sections: [
      ["general", "General"],
      ["physical", "Entree / sortie"],
      ["method", "Methode de scaling"],
      ["lookup", "Lookup Table"],
      ["expression", "Expression"],
      ["revisions", "Revisions"],
      ["audit", "Audit"],
      ["json", "Diagnostic JSON"]
    ]
  },
  curves: {
    key: "curves",
    collection: "engineering-curves",
    validationCollection: "engineering-curve-definitions",
    operationPrefix: "engineering-curve",
    title: "Courbes d'ingenierie",
    listTitle: "Courbes d'ingenierie",
    createLabel: "Creer courbe",
    emptyDetail: "Selectionnez ou creez une courbe de correction.",
    sections: [
      ["general", "General"],
      ["axes", "Axes"],
      ["table", "Table courbe"],
      ["evaluation", "Evaluation"],
      ["revisions", "Revisions"],
      ["audit", "Audit"],
      ["json", "Diagnostic JSON"]
    ]
  },
  daq: {
    key: "daq",
    collection: "daq-channel-profiles",
    validationCollection: "daq-channel-profile-definitions",
    operationPrefix: "daq-channel-profile",
    title: "Voies DAQ",
    listTitle: "Profils de voies DAQ",
    createLabel: "Creer profil DAQ",
    emptyDetail: "Selectionnez ou creez un profil de voie DAQ.",
    sections: [
      ["general", "General"],
      ["daq", "Type de voie"],
      ["physical", "Plages"],
      ["sampling", "Echantillonnage"],
      ["excitation", "Excitation"],
      ["revisions", "Revisions"],
      ["audit", "Audit"],
      ["json", "Diagnostic JSON"]
    ]
  },
  recipes: {
    key: "recipes",
    collection: "acquisition-channel-recipes",
    validationCollection: "acquisition-channel-recipe-definitions",
    operationPrefix: "acquisition-channel-recipe",
    title: "Recettes d'acquisition",
    listTitle: "Recettes d'acquisition",
    createLabel: "Creer recette",
    emptyDetail: "Selectionnez ou creez une recette de voie logique.",
    sections: [
      ["general", "General"],
      ["chain", "Chaine de mesure"],
      ["sampling", "Echantillonnage / plage"],
      ["revisions", "Revisions"],
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
  const [activeSpace, setActiveSpace] = useState<MeasurementSpace>(props.initialSpace);
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

  useEffect(() => {
    setActiveSpace(props.initialSpace);
  }, [props.initialSpace]);

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
          <p className="eyebrow">Measurement engineering</p>
          <h2>{activeConfig.title}</h2>
        </div>
        <div className="segmented">
          {(Object.values(configs) as MeasurementStudioConfig[]).map((config) => (
            <button
              key={config.key}
              className={activeSpace === config.key ? "active" : ""}
              onClick={() => setActiveSpace(config.key)}
            >
              {config.title}
            </button>
          ))}
        </div>
      </div>

      <div className="toolbar measurementCreateBar">
        <Field label="Nouvel ID" value={newEntityId} onChange={setNewEntityId} />
        <Field label="Libelle / modele" value={newLabel} onChange={setNewLabel} />
        <button onClick={() => void createDraft()}>
          <Plus size={16} /> {activeConfig.createLabel}
        </button>
        <button onClick={() => void refreshLists()}>
          <RefreshCw size={16} /> Rafraichir
        </button>
      </div>

      {operationError && (
        <div className="conflictBox">
          <strong>Operation refusee</strong>
          <p>{operationError}</p>
        </div>
      )}

      {loadState === "loading" && <StateBlock title="Chargement" detail="Lecture du domaine measurement engineering." />}
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
              <StateBlock title="Aucune definition ouverte" detail={activeConfig.emptyDetail} />
            ) : (
              <>
                <div className="studioHeader">
                  <div>
                    <p className="eyebrow">{activeConfig.title}</p>
                    <h2>{revision.label}</h2>
                    <p className="mono">
                      {revision.revision_id} | {revision.status} | {revision.definition_checksum}
                    </p>
                  </div>
                  <div className="headerActions">
                    <button onClick={() => void validateDefinition()}>
                      <CheckCircle2 size={16} /> Valider
                    </button>
                    <button onClick={() => void saveDraft()} disabled={readOnly}>
                      <Save size={16} /> Sauvegarder
                    </button>
                    <button onClick={() => void submitRevision()} disabled={readOnly || revision.status !== "draft"}>
                      <Send size={16} /> Soumettre
                    </button>
                    <button onClick={() => void approveRevision()} disabled={revision.status !== "under_review"}>
                      <ShieldCheck size={16} /> Approuver
                    </button>
                    <button onClick={() => void deriveRevision()} disabled={!selected.current_approved_revision}>
                      <GitBranch size={16} /> Nouvelle revision
                    </button>
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
            <span className="mono">{item.identity.entity_id}</span>
            <small>{item.identity.summary_kind} | {revision?.status ?? "no_revision"}</small>
            <small>rev {revision?.revision_number ?? "-"} | {revision?.definition_checksum?.slice(0, 18) ?? "-"}</small>
          </button>
        );
      })}
    </aside>
  );
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
      <EditorCard title="General">
        <Field label="Sensor definition ID" value={s(d.sensor_definition_id)} disabled onChange={() => undefined} />
        <Field label="Manufacturer" value={s(d.manufacturer)} disabled={props.readOnly} onChange={(manufacturer) => props.onDefinition({ ...d, manufacturer })} />
        <Field label="Model name" value={s(d.model_name)} disabled={props.readOnly} onChange={(model_name) => props.onDefinition({ ...d, model_name })} />
        <Field label="Variant" value={s(d.variant)} disabled={props.readOnly} onChange={(variant) => props.onDefinition({ ...d, variant: optionalString(variant) })} />
        <label>
          Sensor family
          <select disabled={props.readOnly} value={s(d.sensor_family)} onChange={(event) => props.onDefinition({ ...d, sensor_family: event.target.value })}>
            {sensorFamilies.map((family) => <option key={family} value={family}>{family}</option>)}
          </select>
        </label>
        <Field label="Technology tags" value={stringArray(d.technology_tags).join(", ")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, technology_tags: splitTokens(value) })} />
      </EditorCard>
    );
  }
  if (props.section === "physical") {
    return (
      <EditorCard title="Physical Input">
        <QuantitySelect label="Physical input quantity" value={s(d.physical_input_quantity)} disabled={props.readOnly} onChange={(physical_input_quantity) => props.onDefinition({ ...d, physical_input_quantity })} />
        <QuantitySelect label="Engineering output quantity" value={s(d.engineering_output_quantity)} disabled={props.readOnly} onChange={(engineering_output_quantity) => props.onDefinition({ ...d, engineering_output_quantity })} />
        <Field label="Engineering output unit" value={s(d.engineering_output_unit)} disabled={props.readOnly} onChange={(engineering_output_unit) => props.onDefinition({ ...d, engineering_output_unit })} />
        <RangeFields title="Nominal range" range={objectValue(d.nominal_range)} unitFallback={s(d.engineering_output_unit)} readOnly={props.readOnly} onRange={(nominal_range) => props.onDefinition({ ...d, nominal_range })} />
        <FrequencyRangeFields range={frequencyRange(d.frequency_range)} readOnly={props.readOnly} onRange={(frequency_range) => props.onDefinition({ ...d, frequency_range })} />
      </EditorCard>
    );
  }
  if (props.section === "electrical") {
    return (
      <EditorCard title="Electrical Output">
        <QuantitySelect label="Electrical output quantity" value={s(d.electrical_output_quantity)} disabled={props.readOnly} onChange={(electrical_output_quantity) => props.onDefinition({ ...d, electrical_output_quantity })} />
        <Field label="Electrical output unit" value={s(d.electrical_output_unit)} disabled={props.readOnly} onChange={(electrical_output_unit) => props.onDefinition({ ...d, electrical_output_unit })} />
        <label>
          Signal domain
          <select disabled={props.readOnly} value={s(d.signal_domain)} onChange={(event) => props.onDefinition({ ...d, signal_domain: event.target.value })}>
            {signalDomains.map((domain) => <option key={domain} value={domain}>{domain}</option>)}
          </select>
        </label>
        <label>
          Input mode requirement
          <select disabled={props.readOnly} value={s(d.input_mode_requirement)} onChange={(event) => props.onDefinition({ ...d, input_mode_requirement: optionalString(event.target.value) })}>
            <option value="">none</option>
            {inputModes.map((mode) => <option key={mode} value={mode}>{mode}</option>)}
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
        title="Scaling"
        refs={refs(d.scaling_profile_refs)}
        options={props.approvedOptions.scalings}
        readOnly={props.readOnly}
        onRefs={(scaling_profile_refs) => props.onDefinition({ ...d, scaling_profile_refs })}
      />
    );
  }
  return (
    <ReferenceEditor
      title="Correction Curves"
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
  if (props.section === "general") {
    return (
      <EditorCard title="General">
        <Field label="Scaling profile ID" value={s(d.scaling_profile_id)} disabled onChange={() => undefined} />
        <Field label="Label" value={s(d.label)} disabled={props.readOnly} onChange={(label) => props.onDefinition({ ...d, label })} />
        <Field label="Source reference" value={s(d.source_reference)} disabled={props.readOnly} onChange={(source_reference) => props.onDefinition({ ...d, source_reference: optionalString(source_reference) })} />
      </EditorCard>
    );
  }
  if (props.section === "physical") {
    return (
      <EditorCard title="Input / Output">
        <QuantitySelect label="Input quantity" value={s(d.input_quantity)} disabled={props.readOnly} onChange={(input_quantity) => props.onDefinition({ ...d, input_quantity })} />
        <Field label="Input unit" value={s(d.input_unit)} disabled={props.readOnly} onChange={(input_unit) => props.onDefinition({ ...d, input_unit })} />
        <QuantitySelect label="Output quantity" value={s(d.output_quantity)} disabled={props.readOnly} onChange={(output_quantity) => props.onDefinition({ ...d, output_quantity })} />
        <Field label="Output unit" value={s(d.output_unit)} disabled={props.readOnly} onChange={(output_unit) => props.onDefinition({ ...d, output_unit })} />
      </EditorCard>
    );
  }
  if (props.section === "method") {
    return (
      <EditorCard title="Scaling Method">
        <label>
          Scaling kind
          <select disabled={props.readOnly} value={s(d.scaling_kind)} onChange={(event) => props.onDefinition({ ...d, scaling_kind: event.target.value })}>
            {["identity", "linear", "two_point", "polynomial", "lookup_table", "piecewise_linear", "expression"].map((kind) => <option key={kind} value={kind}>{kind}</option>)}
          </select>
        </label>
        <Field label="Scale" value={s(parameters.scale)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, scale: optionalNumber(value) } })} />
        <Field label="Offset" value={s(parameters.offset ?? 0)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, offset: optionalNumber(value) ?? 0 } })} />
        <div className="measurementFourGrid">
          <Field label="Input point 1" value={s(parameters.input_point_1)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, input_point_1: optionalNumber(value) } })} />
          <Field label="Output point 1" value={s(parameters.output_point_1)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, output_point_1: optionalNumber(value) } })} />
          <Field label="Input point 2" value={s(parameters.input_point_2)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, input_point_2: optionalNumber(value) } })} />
          <Field label="Output point 2" value={s(parameters.output_point_2)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, output_point_2: optionalNumber(value) } })} />
        </div>
        <Field label="Polynomial coefficients" value={numberArray(parameters.coefficients).join(", ")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, parameters: { ...parameters, coefficients: numberList(value) } })} />
      </EditorCard>
    );
  }
  if (props.section === "lookup") {
    return (
      <EditorCard title="Lookup Table">
        <div className="buttonRow">
          <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...d, parameters: { ...parameters, points: [...scalingPoints(parameters.points), { input: 0, output: 0 }] } })}>Ajouter point</button>
          <button onClick={props.onExportLookupCsv}><Download size={16} /> Export CSV</button>
          <button disabled={props.readOnly} onClick={props.onApplyLookupCsv}><FileSpreadsheet size={16} /> Import CSV</button>
        </div>
        <textarea value={props.lookupCsv} onChange={(event) => props.onLookupCsv(event.target.value)} placeholder="input,output" />
        <StructuredTable columns={["Input", "Output"]}>
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
      <Field label="Expression DSL" value={s(parameters.expression)} disabled={props.readOnly} onChange={(expression) => props.onDefinition({ ...d, parameters: { ...parameters, expression: optionalString(expression) } })} />
      <p className="notice">Allowed identifiers: x, input, temperature, frequency. Allowed functions: pow, sqrt, log10, ln, abs, min, max.</p>
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
      <EditorCard title="General">
        <Field label="Curve ID" value={s(d.curve_id)} disabled onChange={() => undefined} />
        <Field label="Label" value={s(d.label)} disabled={props.readOnly} onChange={(label) => props.onDefinition({ ...d, label })} />
        <label>
          Curve type
          <select disabled={props.readOnly} value={s(d.curve_type)} onChange={(event) => props.onDefinition({ ...d, curve_type: event.target.value })}>
            {curveTypes.map((type) => <option key={type} value={type}>{type}</option>)}
          </select>
        </label>
        <Field label="Source document reference" value={s(d.source_document_reference)} disabled={props.readOnly} onChange={(source_document_reference) => props.onDefinition({ ...d, source_document_reference: optionalString(source_document_reference) })} />
        <Field label="Source checksum" value={s(d.source_checksum)} disabled={props.readOnly} onChange={(source_checksum) => props.onDefinition({ ...d, source_checksum: optionalString(source_checksum) })} />
      </EditorCard>
    );
  }
  if (props.section === "axes") {
    const axis = curveAxes(d)[0] ?? { axis: "frequency", quantity: "frequency", unit: "Hz" };
    const dependent = curveValues(d)[0] ?? { value_id: "correction_db", quantity: "dimensionless", unit: "dB" };
    const dependentValueId = s(dependent.value_id || "correction_db");
    return (
      <EditorCard title="Axes And Values">
        <Field label="Independent axis" value={s(axis.axis)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, independent_axes: [{ ...axis, axis: value }] })} />
        <QuantitySelect label="Axis quantity" value={s(axis.quantity)} disabled={props.readOnly} onChange={(quantity) => props.onDefinition({ ...d, independent_axes: [{ ...axis, quantity }] })} />
        <Field label="Axis unit" value={s(axis.unit)} disabled={props.readOnly} onChange={(unit) => props.onDefinition({ ...d, independent_axes: [{ ...axis, unit }] })} />
        <Field label="Dependent value ID" value={dependentValueId} disabled={props.readOnly} onChange={(value_id) => props.onDefinition({ ...d, dependent_values: [{ ...dependent, value_id }], units: { ...objectValue(d.units), [value_id]: dependent.unit } })} />
        <QuantitySelect label="Dependent quantity" value={s(dependent.quantity)} disabled={props.readOnly} onChange={(quantity) => props.onDefinition({ ...d, dependent_values: [{ ...dependent, quantity }] })} />
        <Field label="Dependent unit" value={s(dependent.unit)} disabled={props.readOnly} onChange={(unit) => props.onDefinition({ ...d, dependent_values: [{ ...dependent, unit }], units: { frequency: "Hz", [dependentValueId]: unit } })} />
        <label>
          Interpolation
          <select disabled={props.readOnly} value={s(d.interpolation)} onChange={(event) => props.onDefinition({ ...d, interpolation: event.target.value })}>
            {["linear_x_linear_y", "log_x_linear_y", "linear_x_log_y", "nearest", "step_previous", "step_next"].map((mode) => <option key={mode} value={mode}>{mode}</option>)}
          </select>
        </label>
        <label>
          Extrapolation
          <select disabled={props.readOnly} value={s(d.extrapolation_policy)} onChange={(event) => props.onDefinition({ ...d, extrapolation_policy: event.target.value })}>
            {["forbidden", "clamp", "warn", "allow"].map((mode) => <option key={mode} value={mode}>{mode}</option>)}
          </select>
        </label>
      </EditorCard>
    );
  }
  if (props.section === "table") {
    const valueId = s(curveValues(d)[0]?.value_id ?? "correction_db");
    return (
      <EditorCard title="Curve Table">
        <div className="buttonRow">
          <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...d, points: [...curvePoints(d.points), newCurvePoint(valueId)] })}>Ajouter point</button>
          <button onClick={props.onExportCurveCsv}><Download size={16} /> Export CSV</button>
          <button disabled={props.readOnly} onClick={props.onApplyCurveCsv}><FileSpreadsheet size={16} /> Import CSV</button>
          <label className="fileButton">
            CSV file
            <input type="file" accept=".csv,text/csv" disabled={props.readOnly} onChange={(event) => event.target.files?.[0] && props.onImportCurveFile(event.target.files[0])} />
          </label>
        </div>
        <textarea value={props.curveCsv} onChange={(event) => props.onCurveCsv(event.target.value)} placeholder="frequency_hz,correction_db" />
        <CurvePlot definition={d} />
        <StructuredTable columns={["Frequency Hz", valueId]}>
          {curvePoints(d.points).map((point, index) => (
            <tr key={index}>
              <td><input disabled={props.readOnly} value={axisNumber(point, "frequency")} onChange={(event) => props.onDefinition({ ...d, points: replaceAt(curvePoints(d.points), index, { ...point, axis_values: { ...objectValue(point.axis_values), frequency: numberOrZero(event.target.value) } }) })} /></td>
              <td><input disabled={props.readOnly} value={valueNumber(point, valueId)} onChange={(event) => props.onDefinition({ ...d, points: replaceAt(curvePoints(d.points), index, { ...point, values: { ...objectValue(point.values), [valueId]: numberOrZero(event.target.value) } }) })} /></td>
            </tr>
          ))}
        </StructuredTable>
      </EditorCard>
    );
  }
  return (
    <EditorCard title="Evaluation">
      <Field label="Frequency Hz" value={props.evaluationFrequency} onChange={props.onEvaluationFrequency} />
      <button onClick={props.onEvaluateCurve}><CheckCircle2 size={16} /> Evaluer la courbe</button>
      {props.curveEvaluation && (
        <dl>
          <dt>Values</dt><dd>{JSON.stringify(props.curveEvaluation.values)}</dd>
          <dt>Interpolation</dt><dd>{props.curveEvaluation.interpolation}</dd>
          <dt>Extrapolated</dt><dd>{props.curveEvaluation.extrapolated ? "yes" : "no"}</dd>
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
      <EditorCard title="General">
        <Field label="DAQ profile ID" value={s(d.daq_channel_profile_id)} disabled onChange={() => undefined} />
        <Field label="Label" value={s(d.label)} disabled={props.readOnly} onChange={(label) => props.onDefinition({ ...d, label })} />
        <label>
          Channel kind
          <select disabled={props.readOnly} value={s(d.channel_kind)} onChange={(event) => props.onDefinition({ ...d, channel_kind: event.target.value })}>
            {channelKinds.map((kind) => <option key={kind} value={kind}>{kind}</option>)}
          </select>
        </label>
        <label>
          Signal domain
          <select disabled={props.readOnly} value={s(d.signal_domain)} onChange={(event) => props.onDefinition({ ...d, signal_domain: event.target.value })}>
            {signalDomains.map((domain) => <option key={domain} value={domain}>{domain}</option>)}
          </select>
        </label>
      </EditorCard>
    );
  }
  if (props.section === "daq") {
    return (
      <EditorCard title="Input / Output Mode">
        <QuantitySelect label="Input quantity" value={s(d.input_quantity)} disabled={props.readOnly} onChange={(input_quantity) => props.onDefinition({ ...d, input_quantity })} />
        <Field label="Input unit" value={s(d.input_unit)} disabled={props.readOnly} onChange={(input_unit) => props.onDefinition({ ...d, input_unit })} />
        <Field label="Input modes" value={stringArray(d.input_modes).join(", ")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, input_modes: splitTokens(value) })} />
        <Field label="Coupling modes" value={stringArray(d.coupling_modes).join(", ")} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, coupling_modes: splitTokens(value) })} />
      </EditorCard>
    );
  }
  if (props.section === "physical") {
    return (
      <EditorCard title="Ranges">
        <SupportedRangeTable ranges={supportedRanges(d.supported_ranges)} readOnly={props.readOnly} onRanges={(supported_ranges) => props.onDefinition({ ...d, supported_ranges })} />
      </EditorCard>
    );
  }
  if (props.section === "sampling") {
    return (
      <EditorCard title="Sampling">
        <Field label="Resolution bits" value={s(d.resolution_bits)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, resolution_bits: optionalInteger(value) })} />
        <Field label="Min sampling rate" value={s(d.min_sampling_rate)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, min_sampling_rate: optionalNumber(value) })} />
        <Field label="Max sampling rate" value={s(d.max_sampling_rate)} disabled={props.readOnly} onChange={(value) => props.onDefinition({ ...d, max_sampling_rate: optionalNumber(value) })} />
        <Field label="Synchronization" value={s(d.synchronization)} disabled={props.readOnly} onChange={(synchronization) => props.onDefinition({ ...d, synchronization: optionalString(synchronization) })} />
        <Field label="Triggering" value={s(d.triggering)} disabled={props.readOnly} onChange={(triggering) => props.onDefinition({ ...d, triggering: optionalString(triggering) })} />
      </EditorCard>
    );
  }
  return <ExcitationEditor definition={d} readOnly={props.readOnly} onDefinition={props.onDefinition} field="excitation_capabilities" list />;
}

function RecipeSections(props: SectionProps & { approvedOptions: ApprovedOptions }) {
  const d = props.definition;
  if (props.section === "general") {
    return (
      <EditorCard title="General">
        <Field label="Recipe ID" value={s(d.recipe_id)} disabled onChange={() => undefined} />
        <Field label="Label" value={s(d.label)} disabled={props.readOnly} onChange={(label) => props.onDefinition({ ...d, label })} />
        <Field label="Output channel name" value={s(d.output_channel_name)} disabled={props.readOnly} onChange={(output_channel_name) => props.onDefinition({ ...d, output_channel_name })} />
        <QuantitySelect label="Output quantity" value={s(d.output_quantity)} disabled={props.readOnly} onChange={(output_quantity) => props.onDefinition({ ...d, output_quantity })} />
        <Field label="Output unit" value={s(d.output_unit)} disabled={props.readOnly} onChange={(output_unit) => props.onDefinition({ ...d, output_unit })} />
      </EditorCard>
    );
  }
  if (props.section === "chain") {
    return (
      <EditorCard title="Measurement Chain">
        <ChainSummary definition={d} />
        <ReferenceSelect label="DAQ channel profile" value={refText(d.daq_channel_profile_ref)} options={props.approvedOptions.daq} readOnly={props.readOnly} onValue={(value) => props.onDefinition({ ...d, daq_channel_profile_ref: refFromText(value) })} />
        <ReferenceSelect label="Sensor definition" value={refText(d.sensor_definition_ref)} options={props.approvedOptions.sensors} readOnly={props.readOnly} onValue={(value) => props.onDefinition({ ...d, sensor_definition_ref: value ? refFromText(value) : undefined })} />
        <ReferenceSelect label="Scaling profile" value={refText(d.scaling_profile_ref)} options={props.approvedOptions.scalings} readOnly={props.readOnly} onValue={(value) => props.onDefinition({ ...d, scaling_profile_ref: value ? refFromText(value) : undefined })} />
        <ReferenceEditor title="Correction curves" refs={refs(d.correction_curve_refs)} options={props.approvedOptions.curves} readOnly={props.readOnly} onRefs={(correction_curve_refs) => props.onDefinition({ ...d, correction_curve_refs })} />
      </EditorCard>
    );
  }
  return (
    <EditorCard title="Sampling / Range">
      <Field label="Sample rate" value={s(d.sample_rate)} disabled={props.readOnly} onChange={(sample_rate) => props.onDefinition({ ...d, sample_rate: numberOrZero(sample_rate) })} />
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
        {quantities.map((quantity) => <option key={quantity} value={quantity}>{quantity}</option>)}
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
      <Field label="Unit" value={s(props.range.unit ?? props.unitFallback)} disabled={props.readOnly} onChange={(unit) => props.onRange({ ...props.range, unit })} />
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
      <legend>Frequency range Hz</legend>
      <Field label="Minimum Hz" value={s(props.range.minimum_hz)} disabled={props.readOnly} onChange={(value) => props.onRange({ ...props.range, minimum_hz: numberOrZero(value) })} />
      <Field label="Maximum Hz" value={s(props.range.maximum_hz)} disabled={props.readOnly} onChange={(value) => props.onRange({ ...props.range, maximum_hz: numberOrZero(value) })} />
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
      <button disabled={props.readOnly} onClick={() => props.onRanges([...props.ranges, { minimum: -10, maximum: 10, unit: "V" }])}>Ajouter range</button>
      <StructuredTable columns={["Minimum", "Maximum", "Unit"]}>
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
    <EditorCard title="Excitation">
      {props.list && <button disabled={props.readOnly} onClick={() => props.onDefinition({ ...props.definition, [props.field]: [...excitations, defaultExcitation("iepe")] })}>Ajouter excitation</button>}
      {excitations.map((excitation, index) => (
        <fieldset className="measurementFieldset" key={index}>
          <legend>{props.list ? `Excitation ${index + 1}` : "Required excitation"}</legend>
          <label>
            Kind
            <select disabled={props.readOnly} value={s(excitation.excitation_kind ?? "none")} onChange={(event) => update({ ...excitation, excitation_kind: event.target.value }, index)}>
              {["none", "external", "voltage", "current", "iepe", "bridge", "charge"].map((kind) => <option key={kind} value={kind}>{kind}</option>)}
            </select>
          </label>
          <Field label="Nominal value" value={s(excitation.nominal_value)} disabled={props.readOnly} onChange={(nominal_value) => update({ ...excitation, nominal_value: optionalNumber(nominal_value) }, index)} />
          <Field label="Unit" value={s(excitation.unit)} disabled={props.readOnly} onChange={(unit) => update({ ...excitation, unit: optionalString(unit) }, index)} />
          <label className="checkboxLabel">
            <input type="checkbox" disabled={props.readOnly} checked={Boolean(excitation.external_allowed)} onChange={(event) => update({ ...excitation, external_allowed: event.target.checked }, index)} />
            External allowed
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
          <option value="">Approved definition...</option>
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
      <StructuredTable columns={["Entity", "Revision", "Approved", ""]}>
        {props.refs.map((ref, index) => (
          <tr key={`${ref.entity_id}-${index}`}>
            <td>{ref.entity_id}</td>
            <td>{ref.revision_id ?? "-"}</td>
            <td>{ref.require_approved ? "yes" : "no"}</td>
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
        <option value="">none</option>
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
      <span>DAQ channel<br /><strong>{refText(props.definition.daq_channel_profile_ref)}</strong></span>
      <span>sensor electrical signal<br /><strong>{refText(props.definition.sensor_definition_ref)}</strong></span>
      <span>scaling profile<br /><strong>{refText(props.definition.scaling_profile_ref)}</strong></span>
      <span>correction curve<br /><strong>{refs(props.definition.correction_curve_refs).map(refText).join(", ") || "-"}</strong></span>
      <span>engineering output<br /><strong>{s(props.definition.output_channel_name)} [{s(props.definition.output_unit)}]</strong></span>
    </div>
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
    variant: "lab-console",
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
    scaling_kind: "linear",
    parameters: { scale: 100, offset: 0 },
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
    independent_axes: [{ axis: "frequency", quantity: "frequency", unit: "Hz" }],
    dependent_values: [{ value_id: "correction_db", quantity: "dimensionless", unit: "dB" }],
    units: { frequency: "Hz", correction_db: "dB" },
    points: [
      { axis_values: { frequency: 10000000 }, values: { correction_db: 0.2 } },
      { axis_values: { frequency: 100000000 }, values: { correction_db: 1.0 } },
      { axis_values: { frequency: 1000000000 }, values: { correction_db: 3.0 } }
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
    const valueId = s(curveValues(definition)[0]?.value_id ?? "correction_db");
    const points = parseTwoColumnCsv(csv, "frequency_hz", valueId).map(([frequency, value]) => ({
      axis_values: { frequency },
      values: { [valueId]: value }
    }));
    onDefinition({ ...definition, points: points.sort((a, b) => a.axis_values.frequency - b.axis_values.frequency) });
  } catch (error) {
    onError(errorMessage(error));
  }
}

function exportCurveCsv(definition: MeasurementEngineeringDefinition) {
  const valueId = s(curveValues(definition)[0]?.value_id ?? "correction_db");
  return [
    `frequency_hz,${valueId}`,
    ...curvePoints(definition.points)
      .sort((a, b) => axisNumber(a, "frequency") - axisNumber(b, "frequency"))
      .map((point) => `${axisNumber(point, "frequency")},${valueNumber(point, valueId)}`)
  ].join("\n");
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

function newCurvePoint(valueId: string): CurvePoint {
  return { axis_values: { frequency: 1000000 }, values: { [valueId]: 0 } };
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
    sensors: "Current Probe 10mV/A",
    scaling: "Current probe 10 mV/A",
    curves: "Cable loss curve",
    daq: "Analog input +/-10 V",
    recipes: "current_A logical channel"
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
