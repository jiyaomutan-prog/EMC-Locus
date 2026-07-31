import {
  Activity,
  AlertTriangle,
  CalendarCheck,
  CheckCircle2,
  FileCheck2,
  FileUp,
  PackagePlus,
  Plus,
  Radio,
  RefreshCw,
  Send,
  ShieldCheck,
  X
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { ApiError, metrologyApi } from "../../api";
import type {
  EquipmentCategory,
  CorrectionRequirementDefinition,
  EquipmentFileReference,
  EquipmentModelAggregate,
  MeasurementEngineeringAggregate
} from "../../models/equipment";
import type {
  AssetCharacterization,
  AssetCharacterizationDefinition,
  AssetCorrectionAssignmentEnvelope,
  AssetCorrectionResolutionReport,
  CalibrationEvent,
  CalibrationStatus,
  CalibrationUncertaintySummary,
  MetrologyAuditEvent,
  MetrologyInstrument,
  RegisterMetrologyInstrumentInput
} from "../../models/metrology";

interface PhysicalAssetMetrologyPanelProps {
  instruments: MetrologyInstrument[];
  approvedModels: EquipmentModelAggregate[];
  nominalCorrections: MeasurementEngineeringAggregate[];
  categories: EquipmentCategory[];
  onRegister: (input: RegisterMetrologyInstrumentInput) => Promise<void>;
  onRefreshInstruments?: () => Promise<void>;
  onOpenCatalog: () => void;
  initialSelectedAssetId?: string | null;
  allowRegistration?: boolean;
}

type CharacterizationKind = "time_conversion" | "frequency_response";

interface CalibrationEvidencePrefill {
  certificateReference: string;
  provider: string;
  calibratedAt: string;
  dueAt: string;
  documentManifest?: EquipmentFileReference;
}

const quantityChoices = [
  ["voltage", "Tension", "V"],
  ["current", "Courant", "A"],
  ["charge", "Charge", "C"],
  ["electric_field", "Champ électrique", "V_per_meter"],
  ["magnetic_field_strength", "Champ magnétique", "A_per_meter"],
  ["acceleration", "Accélération", "m_per_s2"],
  ["temperature", "Température", "degC"],
  ["sound_pressure", "Pression acoustique", "Pa"],
  ["dimensionless", "Sans dimension", "dimensionless"]
] as const;

export function PhysicalAssetMetrologyPanel(props: PhysicalAssetMetrologyPanelProps) {
  const [selectedAssetId, setSelectedAssetId] = useState("");
  const [registering, setRegistering] = useState(false);
  const [characterizations, setCharacterizations] = useState<AssetCharacterization[]>([]);
  const [correctionAssignments, setCorrectionAssignments] = useState<AssetCorrectionAssignmentEnvelope[]>([]);
  const [correctionResolution, setCorrectionResolution] = useState<AssetCorrectionResolutionReport | null>(null);
  const [reviewQueue, setReviewQueue] = useState<AssetCorrectionAssignmentEnvelope[]>([]);
  const [selectedCharacterizationId, setSelectedCharacterizationId] = useState("");
  const [audit, setAudit] = useState<MetrologyAuditEvent[]>([]);
  const [creatingCharacterization, setCreatingCharacterization] = useState(false);
  const [creatingCalibration, setCreatingCalibration] = useState(false);
  const [calibrationPrefill, setCalibrationPrefill] = useState<CalibrationEvidencePrefill | null>(null);
  const [creatingForRequirement, setCreatingForRequirement] = useState<CorrectionRequirementDefinition | null>(null);
  const [loadingCharacterizations, setLoadingCharacterizations] = useState(false);
  const [calibrations, setCalibrations] = useState<CalibrationEvent[]>([]);
  const [calibrationStatus, setCalibrationStatus] = useState<CalibrationStatus | null>(null);
  const [calibrationLoading, setCalibrationLoading] = useState(false);
  const [calibrationError, setCalibrationError] = useState<string | null>(null);
  const [characterizationLoadError, setCharacterizationLoadError] = useState<string | null>(null);
  const [correctionLoadError, setCorrectionLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (props.initialSelectedAssetId && props.instruments.some((instrument) => instrument.asset_id === props.initialSelectedAssetId)) {
      setSelectedAssetId(props.initialSelectedAssetId);
      setRegistering(false);
      return;
    }
    if (!selectedAssetId && props.instruments.length > 0) {
      setSelectedAssetId(props.instruments[0].asset_id);
      setRegistering(false);
    }
  }, [props.initialSelectedAssetId, props.instruments, selectedAssetId]);

  useEffect(() => {
    if (!selectedAssetId) {
      setCharacterizations([]);
      setCorrectionAssignments([]);
      setCorrectionResolution(null);
      setSelectedCharacterizationId("");
      setCalibrations([]);
      setCalibrationStatus(null);
      return;
    }
    let cancelled = false;
    setLoadingCharacterizations(true);
    setCharacterizationLoadError(null);
    setCorrectionLoadError(null);
    metrologyApi.listCharacterizations(selectedAssetId)
      .then((response) => {
        if (cancelled) return;
        setCharacterizations(response.characterizations);
        setSelectedCharacterizationId((current) =>
          response.characterizations.some((item) => item.characterization_id === current)
            ? current
            : response.characterizations[0]?.characterization_id ?? ""
        );
      })
      .catch((reason) => {
        if (!cancelled) setCharacterizationLoadError(characterizationErrorMessage(reason));
      })
      .finally(() => {
        if (!cancelled) setLoadingCharacterizations(false);
      });
    metrologyApi.listCorrections(selectedAssetId)
      .then((response) => { if (!cancelled) setCorrectionAssignments(response.assignments); })
      .catch((reason) => { if (!cancelled) setCorrectionLoadError(characterizationErrorMessage(reason)); });
    metrologyApi.correctionReviewQueue()
      .then((response) => { if (!cancelled) setReviewQueue(response.assignments); })
      .catch((reason) => { if (!cancelled) setCorrectionLoadError(characterizationErrorMessage(reason)); });
    metrologyApi.resolveCorrections(selectedAssetId, todayIso(), "accredited")
      .then((response) => { if (!cancelled) setCorrectionResolution(response.report); })
      .catch((reason) => { if (!cancelled) setCorrectionLoadError(characterizationErrorMessage(reason)); });
    return () => {
      cancelled = true;
    };
  }, [selectedAssetId]);

  useEffect(() => {
    if (!selectedAssetId) return;
    let cancelled = false;
    setCalibrationLoading(true);
    setCalibrationError(null);
    Promise.allSettled([
      metrologyApi.listCalibrations(selectedAssetId),
      metrologyApi.calibrationStatus(selectedAssetId, todayIso())
    ]).then(([history, status]) => {
      if (cancelled) return;
      const errors: string[] = [];
      if (history.status === "fulfilled") setCalibrations(history.value.calibration_events);
      else errors.push(characterizationErrorMessage(history.reason));
      if (status.status === "fulfilled") setCalibrationStatus(status.value);
      else errors.push(characterizationErrorMessage(status.reason));
      setCalibrationError(errors.length > 0 ? errors.join(" ") : null);
      setCalibrationLoading(false);
    });
    return () => { cancelled = true; };
  }, [selectedAssetId]);

  useEffect(() => {
    if (!selectedAssetId || !selectedCharacterizationId) {
      setAudit([]);
      return;
    }
    let cancelled = false;
    metrologyApi
      .characterizationAudit(selectedAssetId, selectedCharacterizationId)
      .then((response) => {
        if (!cancelled) setAudit(response.audit_events);
      })
      .catch(() => {
        if (!cancelled) setAudit([]);
      });
    return () => {
      cancelled = true;
    };
  }, [selectedAssetId, selectedCharacterizationId]);

  const selectedAsset = props.instruments.find((item) => item.asset_id === selectedAssetId);
  const selectedModel = props.approvedModels.find(
    (item) => item.identity.equipment_model_id === selectedAsset?.equipment_model_id
  );
  const correctionRequirements = (selectedModel?.current_approved_revision?.definition.signal_paths ?? [])
    .flatMap((path) => path.correction_requirements ?? []);
  const selectedCharacterization = characterizations.find(
    (item) => item.characterization_id === selectedCharacterizationId
  );

  async function refreshCalibrationDossier(assetId: string): Promise<string[]> {
    const [history, status] = await Promise.allSettled([
      metrologyApi.listCalibrations(assetId),
      metrologyApi.calibrationStatus(assetId, todayIso())
    ]);
    const errors: string[] = [];
    if (history.status === "fulfilled") setCalibrations(history.value.calibration_events);
    else errors.push(`Historique : ${characterizationErrorMessage(history.reason)}`);
    if (status.status === "fulfilled") setCalibrationStatus(status.value);
    else errors.push(`Statut daté : ${characterizationErrorMessage(status.reason)}`);
    return errors;
  }

  async function recordCalibration(input: Parameters<typeof metrologyApi.recordCalibration>[1]) {
    if (!selectedAsset) return;
    setError(null);
    const response = await metrologyApi.recordCalibration(selectedAsset.asset_id, input);
    setCalibrations((current) => [
      response.calibration_event,
      ...current.filter((event) => event.event_id !== response.calibration_event.event_id)
    ]);
    const secondaryErrors = await refreshCalibrationDossier(selectedAsset.asset_id);
    if (props.onRefreshInstruments) {
      try {
        await props.onRefreshInstruments();
      } catch (reason) {
        secondaryErrors.push(`Parc : ${characterizationErrorMessage(reason)}`);
      }
    }
    setCalibrationError(secondaryErrors.length > 0
      ? `L'étalonnage est enregistré. Certains états n'ont pas pu être actualisés. ${secondaryErrors.join(" ")}`
      : null);
    setCreatingCalibration(false);
  }

  async function recordCharacterization(input: Parameters<typeof metrologyApi.recordCharacterization>[1]) {
    if (!selectedAsset) return;
    setError(null);
    try {
      const response = await metrologyApi.recordCharacterization(selectedAsset.asset_id, input);
      if (creatingForRequirement) {
        await metrologyApi.createCorrection(selectedAsset.asset_id, {
          assignment_id: newCorrectionAssignmentId(selectedAsset.asset_id, creatingForRequirement.requirement_id),
          signal_path_id: creatingForRequirement.signal_path_id,
          requirement_id: creatingForRequirement.requirement_id,
          source_event_id: response.characterization.characterization_id,
          conditions: creatingForRequirement.conditions ?? {},
          actor: "metrology.operator",
          reason: "liaison de la caractérisation à l'exigence du modèle"
        });
      }
      await refreshCorrectionDossier(selectedAsset.asset_id);
      setSelectedCharacterizationId(response.characterization.characterization_id);
      setCreatingCharacterization(false);
      setCreatingForRequirement(null);
    } catch (reason) {
      setError(characterizationErrorMessage(reason));
      throw reason;
    }
  }

  async function refreshCorrectionDossier(assetId: string) {
    const [characterizationResponse, correctionResponse, queueResponse, resolutionResponse] = await Promise.allSettled([
      metrologyApi.listCharacterizations(assetId),
      metrologyApi.listCorrections(assetId),
      metrologyApi.correctionReviewQueue(),
      metrologyApi.resolveCorrections(assetId, todayIso(), "accredited")
    ]);
    const secondaryErrors: string[] = [];
    if (characterizationResponse.status === "fulfilled") setCharacterizations(characterizationResponse.value.characterizations);
    else secondaryErrors.push(`Caractérisations : ${characterizationErrorMessage(characterizationResponse.reason)}`);
    if (correctionResponse.status === "fulfilled") setCorrectionAssignments(correctionResponse.value.assignments);
    else secondaryErrors.push(`Affectations : ${characterizationErrorMessage(correctionResponse.reason)}`);
    if (queueResponse.status === "fulfilled") setReviewQueue(queueResponse.value.assignments);
    else secondaryErrors.push(`File de revue : ${characterizationErrorMessage(queueResponse.reason)}`);
    if (resolutionResponse.status === "fulfilled") setCorrectionResolution(resolutionResponse.value.report);
    else secondaryErrors.push(`Résolution : ${characterizationErrorMessage(resolutionResponse.reason)}`);
    setCorrectionLoadError(secondaryErrors.length > 0 ? secondaryErrors.join(" ") : null);
  }

  async function createCorrectionAssignment(requirement: CorrectionRequirementDefinition, sourceEventId: string) {
    if (!selectedAsset) return;
    setError(null);
    try {
      await metrologyApi.createCorrection(selectedAsset.asset_id, {
        assignment_id: newCorrectionAssignmentId(selectedAsset.asset_id, requirement.requirement_id),
        signal_path_id: requirement.signal_path_id,
        requirement_id: requirement.requirement_id,
        source_event_id: sourceEventId,
        conditions: requirement.conditions ?? {},
        actor: "metrology.operator",
        reason: "liaison d'une preuve à l'exigence du modèle"
      });
      await refreshCorrectionDossier(selectedAsset.asset_id);
    } catch (reason) {
      setError(characterizationErrorMessage(reason));
    }
  }

  async function transitionCorrection(
    envelope: AssetCorrectionAssignmentEnvelope,
    transition: "submit-for-review" | "approve-and-activate" | "reject" | "request-changes"
  ) {
    if (!selectedAsset) return;
    setError(null);
    try {
      await metrologyApi.transitionCorrection(
        selectedAsset.asset_id,
        envelope.assignment.assignment_id,
        transition,
        envelope.revision,
        transition === "submit-for-review" ? "metrology.operator" : "metrology.reviewer",
        transition === "submit-for-review"
          ? "correction prête pour revue"
          : transition === "approve-and-activate"
            ? "preuve et domaine de validité vérifiés"
            : transition === "reject"
              ? "preuve refusée après revue métrologique"
              : "correction à compléter"
      );
      await refreshCorrectionDossier(selectedAsset.asset_id);
    } catch (reason) {
      setError(characterizationErrorMessage(reason));
    }
  }

  return (
    <div className="equipmentLayout physicalAssetsLayout">
      <aside className="equipmentList physicalAssetList">
        <div className="listHeader">
          <h2>Exemplaires du parc</h2>
          <span>{props.instruments.length}</span>
        </div>
        {props.allowRegistration !== false && <button
          type="button"
          className="secondary fullWidthAction"
          onClick={() => {
            setRegistering(true);
            setCreatingCharacterization(false);
          }}
        >
          <PackagePlus size={16} /> Enregistrer un matériel
        </button>}
        {props.instruments.length === 0 && (
          <div className="compactEmpty">
            <strong>Aucun matériel enregistré</strong>
            <span>Créez le premier exemplaire depuis un modèle approuvé.</span>
          </div>
        )}
        {props.instruments.map((instrument) => {
          const calibration = instrument.latest_calibration_event ?? instrument.latest_calibration;
          return (
            <button
              type="button"
              className={`physicalAssetRow ${!registering && selectedAssetId === instrument.asset_id ? "active" : ""}`}
              key={instrument.asset_id}
              onClick={() => {
                setSelectedAssetId(instrument.asset_id);
                setRegistering(false);
                setCreatingCharacterization(false);
              }}
            >
              <strong>{instrument.inventory_code}</strong>
              <span>{instrument.manufacturer} {instrument.model}</span>
              <small>N° de série {instrument.serial_number}</small>
              <span className="listItemMeta">
                <span className={`status ${instrument.serviceability_status}`}>
                  {serviceabilityLabel(instrument.serviceability_status)}
                </span>
                <small>{calibration ? `Échéance ${formatDate(calibration.due_at)}` : "Aucun étalonnage"}</small>
              </span>
            </button>
          );
        })}
      </aside>

      <section className="equipmentStudio">
        {error && <p className="operationError"><AlertTriangle size={16} /> {error}</p>}
        {props.allowRegistration !== false && (registering || !selectedAsset) ? (
          <RegisterPhysicalAssetForm
            {...props}
            onRegistered={(assetId) => {
              setSelectedAssetId(assetId);
              setRegistering(false);
            }}
          />
        ) : selectedAsset ? (
          <>
            <header className="studioHeader assetRecordHeader">
              <div>
                <p className="eyebrow">Dossier métrologique</p>
                <h2>{selectedAsset.inventory_code}</h2>
                <p>{selectedAsset.manufacturer} {selectedAsset.model} · {selectedAsset.serial_number ? `N° de série ${selectedAsset.serial_number}` : "Sans numéro de série"}</p>
              </div>
              <div className="headerActions">
                {!creatingCalibration && <button type="button" onClick={() => { setCreatingCharacterization(false); setCreatingCalibration(true); }}>
                  <CalendarCheck size={16} /> Enregistrer un étalonnage
                </button>}
                {!creatingCharacterization && <button className="secondary" type="button" onClick={() => { setCreatingCalibration(false); setCalibrationPrefill(null); setCreatingForRequirement(null); setCreatingCharacterization(true); }}>
                  <Plus size={16} /> Ajouter une caractérisation
                </button>}
              </div>
            </header>

            <AssetSummary asset={selectedAsset} />

            {creatingCalibration ? <CalibrationForm
              asset={selectedAsset}
              onCancel={() => setCreatingCalibration(false)}
              onRecord={recordCalibration}
            /> : <CalibrationHistoryPanel
              asset={selectedAsset}
              status={calibrationStatus}
              events={calibrations}
              loading={calibrationLoading}
              error={calibrationError}
              onRetry={() => void refreshCalibrationDossier(selectedAsset.asset_id).then((errors) => setCalibrationError(errors.join(" ") || null))}
              onAddValues={(event) => {
                setCalibrationPrefill(calibrationEvidencePrefill(event));
                setCreatingForRequirement(null);
                setCreatingCharacterization(true);
              }}
              onRecord={() => setCreatingCalibration(true)}
            />}

            {correctionLoadError && <div className="targetedError" role="alert"><AlertTriangle size={17} /><div><strong>État des corrections partiellement indisponible</strong><p>{correctionLoadError}</p><button className="secondary" type="button" onClick={() => void refreshCorrectionDossier(selectedAsset.asset_id)}><RefreshCw size={15} /> Réessayer</button></div></div>}
            <CorrectionRequirementsPanel
              requirements={correctionRequirements}
              assignments={correctionAssignments}
              resolution={correctionResolution}
              characterizations={characterizations}
              reviewQueue={reviewQueue}
              nominalCorrections={props.nominalCorrections}
              selectedAssetId={selectedAsset.asset_id}
              onCreateAssignment={createCorrectionAssignment}
              onTransition={transitionCorrection}
              onMeasure={(requirement) => {
                setCreatingForRequirement(requirement);
                setCreatingCharacterization(true);
              }}
              onOpenCatalog={props.onOpenCatalog}
            />

            {creatingCharacterization ? (
              <CharacterizationForm
                asset={selectedAsset}
                initialKind={creatingForRequirement?.correction_kind === "raw_signal_conversion" ? "time_conversion" : "frequency_response"}
                requirementName={creatingForRequirement?.display_name}
                initialEvidence={calibrationPrefill}
                hasCalibrationEvent={calibrations.length > 0}
                onRecordCalibration={() => { setCreatingCharacterization(false); setCreatingCalibration(true); }}
                onCancel={() => setCreatingCharacterization(false)}
                onRecord={recordCharacterization}
              />
            ) : (
              <section className="editorCard assetCharacterizationSection">
                <div className="sectionTitleRow">
                  <div>
                    <h2>Caractérisations propres à ce matériel</h2>
                    <p>Ces valeurs appartiennent au numéro de série {selectedAsset.serial_number}, pas au modèle générique.</p>
                  </div>
                  <span className="countBadge">{characterizations.length}</span>
                </div>
                {loadingCharacterizations && <p>Chargement des caractérisations…</p>}
                {characterizationLoadError && <div className="targetedError" role="alert"><AlertTriangle size={17} /><div><strong>Historique des caractérisations indisponible</strong><p>{characterizationLoadError}</p><button className="secondary" type="button" onClick={() => void refreshCorrectionDossier(selectedAsset.asset_id)}><RefreshCw size={15} /> Réessayer</button></div></div>}
                {!loadingCharacterizations && characterizations.length === 0 && (
                  <div className="workflowEmpty">
                    <Activity size={24} />
                    <div>
                      <strong>Aucune correction propre à ce matériel</strong>
                      <p>Ajoutez une caractérisation issue d’un certificat ou d’une mesure interne.</p>
                    </div>
                    <button type="button" onClick={() => { setCreatingForRequirement(null); setCreatingCharacterization(true); }}>
                      Ajouter la première
                    </button>
                  </div>
                )}
                {characterizations.length > 0 && (
                  <div className="assetCharacterizationLayout">
                    <div className="characterizationList" aria-label="Historique des caractérisations">
                      {characterizations.map((item) => (
                        <button
                          type="button"
                          key={item.characterization_id}
                          className={selectedCharacterizationId === item.characterization_id ? "active" : ""}
                          onClick={() => setSelectedCharacterizationId(item.characterization_id)}
                        >
                          {item.characterization_kind === "frequency_response" ? <Radio size={17} /> : <Activity size={17} />}
                          <span>
                            <strong>{item.label}</strong>
                            <small>{formatDate(item.performed_on)} · {characterizationStatus(item)}</small>
                          </span>
                        </button>
                      ))}
                    </div>
                    {selectedCharacterization && (
                      <CharacterizationDetail characterization={selectedCharacterization} audit={audit} />
                    )}
                  </div>
                )}
              </section>
            )}
          </>
        ) : (
          <div className="compactEmpty">
            <strong>Aucun exemplaire avec dossier métrologique</strong>
            <span>Créez d'abord un exemplaire dans le Parc matériel.</span>
          </div>
        )}
      </section>
    </div>
  );
}

function CalibrationHistoryPanel(props: {
  asset: MetrologyInstrument;
  status: CalibrationStatus | null;
  events: CalibrationEvent[];
  loading: boolean;
  error: string | null;
  onRetry: () => void;
  onRecord: () => void;
  onAddValues: (event: CalibrationEvent) => void;
}) {
  return <section className="editorCard calibrationHistorySection">
    <div className="sectionTitleRow">
      <div><p className="eyebrow">Décision métrologique globale</p><h2>Étalonnages</h2><p>Validité et décision applicables au matériel physique {props.asset.inventory_code}.</p></div>
      <span className={`readinessBadge ${calibrationStatusTone(props.status?.calibration_status)}`}>
        {props.status?.calibration_status === "valid" || props.status?.calibration_status === "not_required" ? <CheckCircle2 size={16} /> : <AlertTriangle size={16} />}
        {calibrationStatusBadgeLabel(props.status?.calibration_status, props.asset.calibration_requirement)}
      </span>
    </div>
    <div className="metrologyStateGrid">
      <div><span>Exigence d'étalonnage</span><strong>{calibrationRequirementLabel(props.asset.calibration_requirement)}</strong></div>
      <div><span>État de service</span><strong>{serviceabilityLabel(props.status?.serviceability_status ?? props.asset.serviceability_status)}</strong></div>
      <div><span>Statut au {formatDate(props.status?.checked_on ?? todayIso())}</span><strong>{calibrationStatusLabel(props.status?.calibration_status, props.asset.calibration_requirement)}</strong></div>
    </div>
    {props.status?.reasons.map((reason) => <p className="calibrationReason" key={reason}>{reason}</p>)}
    {props.error && <div className="targetedError" role="alert"><AlertTriangle size={17} /><div><strong>État métrologique partiellement indisponible</strong><p>{props.error}</p><button className="secondary" type="button" onClick={props.onRetry}><RefreshCw size={15} /> Réessayer</button></div></div>}
    {props.loading && <p role="status">Chargement de l'historique d'étalonnage…</p>}
    {!props.loading && props.events.length === 0 && <div className="workflowEmpty compactWorkflowEmpty"><CalendarCheck size={22} /><div><strong>Aucun événement d'étalonnage</strong><p>Une caractérisation issue d'un certificat ne remplace pas cette décision globale.</p></div><button type="button" onClick={props.onRecord}>Enregistrer l'étalonnage</button></div>}
    <div className="calibrationEventList">
      {props.events.map((event) => {
        const uncertainty = parseJsonObject<CalibrationUncertaintySummary>(event.uncertainty_summary_json);
        const document = parseJsonObject<EquipmentFileReference>(event.document_manifest_json);
        return <article key={event.event_id}>
          <header><div><strong>{event.certificate_reference}</strong><span>{event.provider} · étalonné le {formatDate(event.calibrated_at)}</span></div><span className={`status ${event.decision}`}>{decisionLabel(event.decision)}</span></header>
          <dl className="businessSummary compact">
            <dt>Échéance</dt><dd>{formatDate(event.due_at)}</dd>
            <dt>État constaté</dt><dd>{event.as_found_status ? decisionLabel(event.as_found_status) : "Non renseigné"}</dd>
            <dt>État laissé</dt><dd>{event.as_left_status ? decisionLabel(event.as_left_status) : "Non renseigné"}</dd>
            <dt>Réglage</dt><dd>{event.adjustment_performed ? "Effectué" : "Non effectué"}</dd>
            {uncertainty && <><dt>Incertitude</dt><dd>{uncertaintySummaryLabel(uncertainty)}</dd></>}
            {event.traceability_reference && <><dt>Traçabilité</dt><dd>{event.traceability_reference}</dd></>}
            {document && <><dt>Document</dt><dd><FileCheck2 size={14} /> {document.original_filename}</dd></>}
          </dl>
          {event.comment && <p>{event.comment}</p>}
          <div className="headerActions"><button className="secondary" type="button" onClick={() => props.onAddValues(event)}><Plus size={15} /> Ajouter des valeurs de correction issues de ce certificat</button></div>
          <details><summary>Détails techniques</summary><dl className="businessSummary compact"><dt>Enregistré par</dt><dd>{event.recorded_by}</dd><dt>Enregistré le</dt><dd>{formatDateTime(event.recorded_at)}</dd><dt>Événement</dt><dd className="mono">{event.event_id}</dd><dt>Révision</dt><dd className="mono">{event.revision}</dd>{document && <><dt>Empreinte du document</dt><dd className="mono">{document.sha256}</dd></>}</dl></details>
        </article>;
      })}
    </div>
  </section>;
}

function CalibrationForm(props: {
  asset: MetrologyInstrument;
  onCancel: () => void;
  onRecord: (input: Parameters<typeof metrologyApi.recordCalibration>[1]) => Promise<void>;
}) {
  const currentDate = todayIso();
  const [calibratedAt, setCalibratedAt] = useState(currentDate);
  const [dueAt, setDueAt] = useState(oneYearAfter(currentDate));
  const [certificateReference, setCertificateReference] = useState("");
  const [provider, setProvider] = useState("");
  const [decision, setDecision] = useState<CalibrationEvent["decision"]>("conforming");
  const [asFoundStatus, setAsFoundStatus] = useState<CalibrationEvent["decision"] | "">("");
  const [asLeftStatus, setAsLeftStatus] = useState<CalibrationEvent["decision"] | "">("");
  const [adjustmentPerformed, setAdjustmentPerformed] = useState(false);
  const [uncertainty, setUncertainty] = useState("");
  const [uncertaintyUnit, setUncertaintyUnit] = useState("");
  const [coverageFactor, setCoverageFactor] = useState("2");
  const [uncertaintyStatement, setUncertaintyStatement] = useState("");
  const [traceabilityReference, setTraceabilityReference] = useState("");
  const [comment, setComment] = useState("");
  const [recordedBy, setRecordedBy] = useState("metrology.operator");
  const [proofFile, setProofFile] = useState<File | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFormError(null);
    if (!certificateReference.trim() || !provider.trim() || !recordedBy.trim()) {
      setFormError("La référence du certificat, le laboratoire ou prestataire et l'auteur de saisie sont obligatoires.");
      return;
    }
    if (!calibratedAt || !dueAt || dueAt < calibratedAt) {
      setFormError("L'échéance doit être identique ou postérieure à la date d'étalonnage.");
      return;
    }
    let uncertaintySummary: CalibrationUncertaintySummary = {};
    try {
      const expanded = optionalFiniteNumber(uncertainty, "L'incertitude");
      uncertaintySummary = {
        ...(expanded !== undefined ? {
          expanded_uncertainty: expanded,
          unit: uncertaintyUnit.trim(),
          coverage_factor: requiredFiniteNumber(coverageFactor, "Le facteur d'élargissement")
        } : {}),
        ...(uncertaintyStatement.trim() ? { statement: uncertaintyStatement.trim() } : {})
      };
      if (expanded !== undefined && !uncertaintyUnit.trim()) throw new Error("L'unité de l'incertitude est obligatoire lorsqu'une valeur est renseignée.");
    } catch (reason) {
      setFormError(reason instanceof Error ? reason.message : "L'incertitude est invalide.");
      return;
    }
    setSubmitting(true);
    try {
      const documentManifest = proofFile ? (await metrologyApi.uploadFile(proofFile)).file : undefined;
      await props.onRecord({
        event_id: newCalibrationEventId(props.asset.asset_id),
        certificate_reference: certificateReference.trim(),
        calibrated_at: calibratedAt,
        due_at: dueAt,
        provider: provider.trim(),
        decision,
        as_found_status: asFoundStatus || undefined,
        as_left_status: asLeftStatus || undefined,
        adjustment_performed: adjustmentPerformed,
        uncertainty_summary: uncertaintySummary,
        traceability_reference: traceabilityReference.trim() || undefined,
        comment: comment.trim() || undefined,
        document_manifest: documentManifest,
        recorded_by: recordedBy.trim(),
        actor: recordedBy.trim(),
        reason: "enregistrement de l'événement d'étalonnage global du matériel"
      });
    } catch (reason) {
      setFormError(calibrationErrorMessage(reason));
    } finally {
      setSubmitting(false);
    }
  }

  return <form className="characterizationForm calibrationForm" onSubmit={submit}>
    <header className="sectionTitleRow"><div><p className="eyebrow">Nouvel événement métrologique global</p><h2>Enregistrer un étalonnage</h2><p>Cette décision met à jour le statut d'étalonnage de {props.asset.inventory_code}. Elle ne crée aucune valeur de correction.</p></div><button className="iconButton" type="button" title="Fermer" onClick={props.onCancel}><X size={18} /></button></header>
    {formError && <p className="errorText"><AlertTriangle size={16} /> {formError}</p>}
    <section className="editorCard"><h3>Certificat et décision</h3><div className="formGrid">
      <label><FieldCaption label="Date d'étalonnage" required /><input type="date" value={calibratedAt} onChange={(event) => setCalibratedAt(event.target.value)} /></label>
      <label><FieldCaption label="Prochaine échéance" required /><input type="date" value={dueAt} onChange={(event) => setDueAt(event.target.value)} /></label>
      <label><FieldCaption label="Référence du certificat" required /><input value={certificateReference} onChange={(event) => setCertificateReference(event.target.value)} /></label>
      <label><FieldCaption label="Laboratoire ou prestataire" required /><input value={provider} onChange={(event) => setProvider(event.target.value)} /></label>
      <label><FieldCaption label="Décision globale" required /><CalibrationDecisionSelect value={decision} onChange={setDecision} /></label>
      <label>État constaté avant intervention<CalibrationDecisionSelect optional value={asFoundStatus} onChange={setAsFoundStatus} /></label>
      <label>État laissé après intervention<CalibrationDecisionSelect optional value={asLeftStatus} onChange={setAsLeftStatus} /></label>
      <label className="checkboxField"><input type="checkbox" checked={adjustmentPerformed} onChange={(event) => setAdjustmentPerformed(event.target.checked)} /> Un réglage a été effectué</label>
    </div></section>
    <section className="editorCard"><h3>Incertitude et traçabilité</h3><div className="formGrid">
      <label>Incertitude élargie<input type="number" min="0" step="any" value={uncertainty} onChange={(event) => setUncertainty(event.target.value)} /></label>
      <label>Unité<input value={uncertaintyUnit} onChange={(event) => setUncertaintyUnit(event.target.value)} /></label>
      <label>Facteur d'élargissement<input type="number" min="0.01" step="any" value={coverageFactor} onChange={(event) => setCoverageFactor(event.target.value)} /></label>
      <label>Résumé de l'incertitude<textarea value={uncertaintyStatement} onChange={(event) => setUncertaintyStatement(event.target.value)} /></label>
      <label>Référence de traçabilité<input value={traceabilityReference} onChange={(event) => setTraceabilityReference(event.target.value)} /></label>
      <label className="fileField"><span>Document de preuve</span><input type="file" onChange={(event) => setProofFile(event.target.files?.[0] ?? null)} /><small>{proofFile ? proofFile.name : "Certificat PDF, feuille de calcul ou document associé"}</small></label>
      <label><FieldCaption label="Enregistré par" required /><input value={recordedBy} onChange={(event) => setRecordedBy(event.target.value)} /></label>
      <label>Commentaire<textarea value={comment} onChange={(event) => setComment(event.target.value)} /></label>
    </div></section>
    <div className="buttonRow stickyActions"><button className="secondary" type="button" onClick={props.onCancel}>Annuler</button><button type="submit" disabled={submitting}><CheckCircle2 size={16} /> {submitting ? "Enregistrement…" : "Enregistrer l'étalonnage"}</button></div>
  </form>;
}

function CalibrationDecisionSelect<T extends CalibrationEvent["decision"] | "">(props: {
  value: T;
  optional?: boolean;
  onChange: (value: T) => void;
}) {
  return <select value={props.value} onChange={(event) => props.onChange(event.target.value as T)}>{props.optional && <option value="">Non renseigné</option>}<option value="conforming">Conforme</option><option value="nonconforming">Non conforme</option><option value="indeterminate">Indéterminé</option><option value="not_assessed">Non évalué</option></select>;
}

function CorrectionRequirementsPanel(props: {
  requirements: CorrectionRequirementDefinition[];
  assignments: AssetCorrectionAssignmentEnvelope[];
  resolution: AssetCorrectionResolutionReport | null;
  characterizations: AssetCharacterization[];
  reviewQueue: AssetCorrectionAssignmentEnvelope[];
  nominalCorrections: MeasurementEngineeringAggregate[];
  selectedAssetId: string;
  onCreateAssignment: (requirement: CorrectionRequirementDefinition, sourceEventId: string) => Promise<void>;
  onTransition: (
    assignment: AssetCorrectionAssignmentEnvelope,
    transition: "submit-for-review" | "approve-and-activate" | "reject" | "request-changes"
  ) => Promise<void>;
  onMeasure: (requirement: CorrectionRequirementDefinition) => void;
  onOpenCatalog: () => void;
}) {
  const [selectedSources, setSelectedSources] = useState<Record<string, string>>({});
  const selectedReviewCount = props.reviewQueue.filter(
    (item) => item.assignment.asset_id === props.selectedAssetId
  ).length;

  return (
    <section className="editorCard correctionReadinessSection">
      <div className="sectionTitleRow">
        <div>
          <p className="eyebrow">Aptitude du signal</p>
          <h2>Corrections requises par le modèle</h2>
          <p>Le verdict est calculé pour ce numéro de série, aujourd’hui, en contexte accrédité.</p>
        </div>
        <span className={`readinessBadge ${props.resolution?.ready ? "ready" : "blocked"}`}>
          {props.resolution?.ready ? <CheckCircle2 size={16} /> : <AlertTriangle size={16} />}
          {props.resolution?.ready ? "Corrections requises disponibles" : "Corrections incomplètes"}
        </span>
      </div>

      {selectedReviewCount > 0 && (
        <div className="correctionReviewNotice">
          <ShieldCheck size={18} />
          <div><strong>File de revue métrologique</strong><span>{selectedReviewCount} correction{selectedReviewCount > 1 ? "s" : ""} de ce matériel attend{selectedReviewCount > 1 ? "ent" : ""} une décision.</span></div>
        </div>
      )}

      {props.requirements.length === 0 && (
        <div className="workflowEmpty compactWorkflowEmpty">
          <CheckCircle2 size={22} />
          <div><strong>Aucune correction exigée par le modèle approuvé</strong><p>Révisez le modèle dans le catalogue si son chemin de signal nécessite une compensation.</p></div>
          <button type="button" className="secondary" onClick={props.onOpenCatalog}>Ouvrir le modèle</button>
        </div>
      )}

      <div className="assetCorrectionTable" role="table" aria-label="Corrections requises">
        {props.requirements.map((requirement) => {
          const resolution = props.resolution?.resolutions.find(
            (item) => item.requirement_id === requirement.requirement_id
          );
          const assignment = props.assignments.find((item) =>
            item.assignment.requirement_id === requirement.requirement_id
            && !["superseded", "rejected", "expired"].includes(item.assignment.status)
          );
          const compatibleSources = props.characterizations.filter((item) =>
            item.decision === "conforming"
            && (requirement.correction_kind === "raw_signal_conversion"
              ? item.characterization_kind === "time_conversion"
              : item.characterization_kind === "frequency_response")
          );
          const selectedSource = selectedSources[requirement.requirement_id]
            ?? compatibleSources[0]?.characterization_id
            ?? "";
          const assignedCharacterization = props.characterizations.find(
            (item) => item.characterization_id === assignment?.assignment.source_event_id
          );
          const nominalSummary = nominalCorrectionSummary(requirement, props.nominalCorrections);
          const state = resolution?.reason === "asset_correction_expired"
            ? "expired"
            : assignment?.assignment.status
              ?? (resolution?.selected_source === "model_nominal"
              ? "model_nominal"
              : "missing");
          return (
            <div className="assetCorrectionRow" role="row" key={requirement.requirement_id}>
              <div className="assetCorrectionIdentity" role="cell">
                {requirement.correction_kind === "frequency_dependent_correction" ? <Radio size={18} /> : <Activity size={18} />}
                <div>
                  <strong>{requirement.display_name}</strong>
                  <span>{requirement.physical_purpose}</span>
                  <small>{requirement.operation === "add" ? "Addition" : requirement.operation === "subtract" ? "Soustraction" : requirement.operation === "multiply" ? "Multiplication" : "Division"} · {operatorUnitLabel(requirement.expected_unit)}</small>
                  {Object.keys(requirement.conditions ?? {}).length > 0 && (
                    <small>{friendlyConditions(requirement.conditions ?? {})}</small>
                  )}
                </div>
              </div>
              <div className="assetCorrectionState" role="cell">
                <span className={`status correction-${state}`}>{correctionAssignmentStatusLabel(state)}</span>
                {resolution?.warning && <small>{resolution.warning}</small>}
                {assignment?.assignment.valid_until && <small>Valide jusqu’au {formatDate(assignment.assignment.valid_until)}</small>}
              </div>
              <div className="assetCorrectionAction" role="cell">
                {!assignment && resolution?.selected_source !== "model_nominal" && compatibleSources.length > 0 && (
                  <>
                    <select aria-label={`Preuve pour ${requirement.display_name}`} value={selectedSource} onChange={(event) => setSelectedSources((current) => ({ ...current, [requirement.requirement_id]: event.target.value }))}>
                      {compatibleSources.map((item) => <option key={item.characterization_id} value={item.characterization_id}>{item.label} · {formatDate(item.performed_on)}</option>)}
                    </select>
                    <button type="button" onClick={() => void props.onCreateAssignment(requirement, selectedSource)}>Lier cette preuve</button>
                  </>
                )}
                {!assignment && resolution?.selected_source !== "model_nominal" && compatibleSources.length === 0 && (
                  <button type="button" onClick={() => props.onMeasure(requirement)}><Plus size={15} /> Mesurer cette correction</button>
                )}
                {assignment?.assignment.status === "draft" && (
                  <button type="button" onClick={() => void props.onTransition(assignment, "submit-for-review")}><Send size={15} /> Soumettre pour revue</button>
                )}
                {assignment?.assignment.status === "waiting_for_review" && (
                  <div className="buttonRow">
                    <button type="button" onClick={() => void props.onTransition(assignment, "approve-and-activate")}><ShieldCheck size={15} /> Approuver et activer</button>
                    <button type="button" className="secondary" onClick={() => void props.onTransition(assignment, "request-changes")}>Demander une correction</button>
                    <button type="button" className="danger" onClick={() => void props.onTransition(assignment, "reject")}>Refuser</button>
                  </div>
                )}
                {state === "expired" && (
                  <button type="button" onClick={() => props.onMeasure(requirement)}>
                    <Plus size={15} /> Renouveler cette correction
                  </button>
                )}
                {assignment?.assignment.status === "active" && state !== "expired" && (
                  <div className="correctionSourceComparison">
                    <span className="activeCorrectionSource">
                      <CheckCircle2 size={16} />
                      <span>
                        <strong>Valeur propre à ce matériel</strong>
                        <small>{correctionValueSummary(assignedCharacterization)}</small>
                        <small>{assignedCharacterization?.certificate_reference || assignedCharacterization?.label || "Preuve métrologique approuvée"}</small>
                      </span>
                    </span>
                    {requirement.model_default_reference && (
                      <span className="nominalCorrectionSource">
                        <strong>Valeur nominale du modèle</strong>
                        <small>{nominalSummary} · non sélectionnée</small>
                      </span>
                    )}
                  </div>
                )}
                {!assignment && resolution?.selected_source === "model_nominal" && (
                  <span className="nominalCorrectionSource">Valeur nominale du modèle · usage de secours autorisé</span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}

function AssetSummary({ asset }: { asset: MetrologyInstrument }) {
  const calibration = asset.latest_calibration_event ?? asset.latest_calibration;
  return (
    <section className="assetSummaryBand" aria-label="État du matériel">
      <div>
        <span>État de service</span>
        <strong>{serviceabilityLabel(asset.serviceability_status)}</strong>
        {asset.serviceability_reason && <small>{asset.serviceability_reason}</small>}
      </div>
      <div>
        <span>Étalonnage</span>
        <strong>{calibration ? `Valide jusqu’au ${formatDate(calibration.due_at)}` : "Aucun étalonnage"}</strong>
        <small>{calibration?.certificate_reference ?? calibrationRequirementLabel(asset.calibration_requirement)}</small>
      </div>
      <div>
        <span>Modèle de référence</span>
        <strong>{asset.manufacturer} {asset.model}</strong>
        <small>{asset.equipment_model_id ? "Modèle approuvé lié" : "Sans lien de catalogue"}</small>
      </div>
    </section>
  );
}

interface CharacterizationFormProps {
  asset: MetrologyInstrument;
  initialKind?: CharacterizationKind;
  requirementName?: string;
  initialEvidence?: CalibrationEvidencePrefill | null;
  hasCalibrationEvent: boolean;
  onRecordCalibration: () => void;
  onCancel: () => void;
  onRecord: (input: Parameters<typeof metrologyApi.recordCharacterization>[1]) => Promise<void>;
}

function CharacterizationForm({ asset, initialKind = "frequency_response", requirementName, initialEvidence, hasCalibrationEvent, onRecordCalibration, onCancel, onRecord }: CharacterizationFormProps) {
  const today = todayIso();
  const [kind, setKind] = useState<CharacterizationKind>(initialKind);
  const [label, setLabel] = useState(requirementName ?? (initialKind === "frequency_response" ? "Pertes mesurées" : "Sensibilité mesurée"));
  const [performedOn, setPerformedOn] = useState(initialEvidence?.calibratedAt ?? today);
  const [validFrom, setValidFrom] = useState(initialEvidence?.calibratedAt ?? today);
  const [validUntil, setValidUntil] = useState(initialEvidence?.dueAt ?? oneYearAfter(today));
  const [sourceKind, setSourceKind] = useState<AssetCharacterization["source_kind"]>(initialEvidence ? "calibration" : "characterization");
  const [provider, setProvider] = useState(initialEvidence?.provider ?? "");
  const [methodReference, setMethodReference] = useState("");
  const [decision, setDecision] = useState<AssetCharacterization["decision"]>("conforming");
  const [certificateReference, setCertificateReference] = useState(initialEvidence?.certificateReference ?? "");
  const [proofFile, setProofFile] = useState<File | null>(null);
  const [comment, setComment] = useState("");
  const [uncertainty, setUncertainty] = useState("");
  const [uncertaintyUnit, setUncertaintyUnit] = useState("dB");
  const [coverageFactor, setCoverageFactor] = useState("2");
  const [confidenceLevel, setConfidenceLevel] = useState("95");
  const [temperatureC, setTemperatureC] = useState("");
  const [humidityPercent, setHumidityPercent] = useState("");
  const [asFound, setAsFound] = useState("");
  const [asLeft, setAsLeft] = useState("");
  const [adjustmentPerformed, setAdjustmentPerformed] = useState(false);
  const [inputQuantity, setInputQuantity] = useState("voltage");
  const [inputUnit, setInputUnit] = useState("V");
  const [outputQuantity, setOutputQuantity] = useState("current");
  const [outputUnit, setOutputUnit] = useState("A");
  const [scale, setScale] = useState("1");
  const [offset, setOffset] = useState("0");
  const [minimum, setMinimum] = useState("");
  const [maximum, setMaximum] = useState("");
  const [limitHandling, setLimitHandling] = useState("warn");
  const [curveType, setCurveType] = useState("cable_loss");
  const [operation, setOperation] = useState("add");
  const [phaseIncluded, setPhaseIncluded] = useState(false);
  const [frequencyCsv, setFrequencyCsv] = useState(
    "frequence_hz,amplitude_db\n1000000,0.2\n100000000,1.0\n1000000000,3.0"
  );
  const [submitting, setSubmitting] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const preview = useMemo(() => {
    if (kind !== "frequency_response") return null;
    try {
      return parseFrequencyCsv(frequencyCsv, phaseIncluded);
    } catch {
      return null;
    }
  }, [frequencyCsv, kind, phaseIncluded]);

  function changeQuantity(value: string, target: "input" | "output") {
    const choice = quantityChoices.find(([quantity]) => quantity === value);
    if (!choice) return;
    if (target === "input") {
      setInputQuantity(choice[0]);
      setInputUnit(choice[2]);
    } else {
      setOutputQuantity(choice[0]);
      setOutputUnit(choice[2]);
    }
  }

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFormError(null);
    if (!label.trim() || !provider.trim() || !methodReference.trim()) {
      setFormError("Le nom, le laboratoire ou prestataire et la méthode sont obligatoires.");
      return;
    }
    if (validFrom < performedOn || validUntil < validFrom) {
      setFormError("La validité doit commencer au plus tôt à la date de mesure et se terminer après son début.");
      return;
    }

    const characterizationId = newCharacterizationId(asset.asset_id);
    let correction: AssetCharacterizationDefinition["correction"];
    try {
      if (kind === "time_conversion") {
        const numericScale = requiredFiniteNumber(scale, "Le facteur de conversion");
        const numericOffset = requiredFiniteNumber(offset, "L’offset");
        const lower = optionalFiniteNumber(minimum, "La limite basse");
        const upper = optionalFiniteNumber(maximum, "La limite haute");
        if ((lower === undefined) !== (upper === undefined)) {
          throw new Error("Renseignez les deux limites, ou laissez les deux champs vides.");
        }
        if (lower !== undefined && upper !== undefined && upper <= lower) {
          throw new Error("La limite haute doit être supérieure à la limite basse.");
        }
        correction = {
          correction_kind: "time_conversion",
          correction: {
            definition_schema_version: "emc-locus.scaling-profile-definition.v1",
            scaling_profile_id: characterizationId,
            label: label.trim(),
            input_quantity: inputQuantity,
            input_unit: inputUnit,
            output_quantity: outputQuantity,
            output_unit: outputUnit,
            signal_representation: "time_domain_samples",
            scaling_kind: "linear",
            parameters: { scale: numericScale, offset: numericOffset },
            ...(lower !== undefined && upper !== undefined
              ? { input_limits: { minimum: lower, maximum: upper, handling: limitHandling } }
              : {}),
            validity_domain: {},
            metadata: {}
          }
        };
      } else {
        const points = parseFrequencyCsv(frequencyCsv, phaseIncluded);
        correction = {
          correction_kind: "frequency_response",
          correction: {
            definition_schema_version: "emc-locus.engineering-curve-definition.v1",
            curve_id: characterizationId,
            curve_type: curveType,
            label: label.trim(),
            signal_representation: "frequency_domain_spectrum",
            independent_axes: [{ axis: "frequency", quantity: "frequency", unit: "Hz" }],
            dependent_values: [
              {
                value_id: "amplitude",
                quantity: "dimensionless",
                unit: "dB",
                component: "amplitude",
                operation
              },
              ...(phaseIncluded
                ? [{ value_id: "phase", quantity: "angle", unit: "deg", component: "phase", operation: "add" }]
                : [])
            ],
            units: { frequency: "Hz", amplitude: "dB", ...(phaseIncluded ? { phase: "deg" } : {}) },
            points,
            interpolation: "log_x_linear_y",
            extrapolation_policy: "forbidden",
            validity_domain: {},
            conditions: {},
            metadata: {}
          }
        };
      }
    } catch (reason) {
      setFormError(reason instanceof Error ? reason.message : "Les valeurs de correction sont invalides.");
      return;
    }

    let definition: AssetCharacterizationDefinition;
    try {
      const uncertaintyValue = optionalFiniteNumber(uncertainty, "L’incertitude");
      definition = {
        definition_schema_version: "emc-locus.asset-characterization-definition.v1",
        characterization_id: characterizationId,
        asset_id: asset.asset_id,
        label: label.trim(),
        correction,
        ...(uncertaintyValue !== undefined
          ? {
              uncertainty: {
                expanded_uncertainty: uncertaintyValue,
                unit: uncertaintyUnit.trim(),
                coverage_factor: requiredFiniteNumber(coverageFactor, "Le facteur d’élargissement"),
                confidence_level_percent: requiredFiniteNumber(confidenceLevel, "Le niveau de confiance")
              }
            }
          : {}),
        conditions: {}
      };
    } catch (reason) {
      setFormError(reason instanceof Error ? reason.message : "L’incertitude est invalide.");
      return;
    }

    setSubmitting(true);
    try {
      let documentManifest: EquipmentFileReference | undefined = initialEvidence?.documentManifest;
      if (proofFile) {
        documentManifest = (await metrologyApi.uploadFile(proofFile)).file;
      }
      await onRecord({
        characterization_id: characterizationId,
        performed_on: performedOn,
        valid_from: validFrom,
        valid_until: validUntil,
        source_kind: sourceKind,
        provider: provider.trim(),
        method_reference: methodReference.trim(),
        decision,
        definition,
        certificate_reference: certificateReference.trim() || undefined,
        document_manifest: documentManifest,
        comment: comment.trim() || undefined,
        environmental_conditions: {
          ...(temperatureC.trim() ? { temperature_c: requiredFiniteNumber(temperatureC, "La température") } : {}),
          ...(humidityPercent.trim() ? { relative_humidity_percent: requiredFiniteNumber(humidityPercent, "L’humidité") } : {})
        },
        as_found: asFound.trim() ? { summary: asFound.trim() } : undefined,
        as_left: asLeft.trim() ? { summary: asLeft.trim() } : undefined,
        adjustment_performed: adjustmentPerformed,
        recorded_by: "metrology.operator",
        actor: "metrology.operator",
        reason: "enregistrement d’une caractérisation propre au matériel"
      });
    } catch (reason) {
      setFormError(characterizationErrorMessage(reason));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <form className="characterizationForm" onSubmit={submit}>
      <header className="sectionTitleRow">
        <div>
          <p className="eyebrow">Nouvel événement métrologique</p>
          <h2>{requirementName ? `Mesurer : ${requirementName}` : "Ajouter une caractérisation"}</h2>
          <p>La mesure sera enregistrée pour {asset.asset_id}, N° de série {asset.serial_number}.</p>
        </div>
        <button type="button" className="iconButton" title="Fermer" onClick={onCancel}><X size={18} /></button>
      </header>

      {formError && <p className="errorText"><AlertTriangle size={16} /> {formError}</p>}

      <ol className="correctionWorkflowSteps" aria-label="Étapes d'enregistrement de la correction">
        <li><span>1</span><strong>Contexte</strong><small>Matériel et correction attendue</small></li>
        <li><span>2</span><strong>Valeurs</strong><small>Conversion ou points mesurés</small></li>
        <li><span>3</span><strong>Preuves</strong><small>Validité, incertitude et document</small></li>
        <li><span>4</span><strong>Revue</strong><small>Soumission après enregistrement</small></li>
      </ol>

      <section className="editorCard">
        <p className="stepEyebrow">Étape 1 · Contexte</p>
        <h2>Quel résultat a été mesuré pour ce matériel ?</h2>
        <div className="signalModeControl" role="group" aria-label="Type de caractérisation">
          <button type="button" className={kind === "time_conversion" ? "active" : ""} onClick={() => { setKind("time_conversion"); setLabel("Sensibilité mesurée"); setUncertaintyUnit(outputUnit); }}>
            <Activity size={18} />
            <span><strong>Conversion du signal brut</strong><small>Facteur, offset et limites propres au matériel</small></span>
          </button>
          <button type="button" className={kind === "frequency_response" ? "active" : ""} onClick={() => { setKind("frequency_response"); setLabel("Pertes mesurées"); setUncertaintyUnit("dB"); }}>
            <Radio size={18} />
            <span><strong>Correction selon la fréquence</strong><small>Amplitude et phase optionnelle selon la fréquence</small></span>
          </button>
        </div>
      </section>

      <section className="editorCard">
        <p className="stepEyebrow">Étape 3 · Preuves</p>
        <h2>Origine et validité</h2>
        <div className="formGrid">
          <label><FieldCaption label="Nom de la caractérisation" required /><input value={label} onChange={(event) => setLabel(event.target.value)} placeholder="ex. Pertes du câble RF après contrôle" /></label>
          <label><FieldCaption label="Origine des valeurs mesurées" required /><select value={sourceKind} onChange={(event) => setSourceKind(event.target.value as typeof sourceKind)}><option value="calibration">Valeurs issues d’un certificat d’étalonnage</option><option value="characterization">Rapport de caractérisation</option><option value="internal_measurement">Mesure interne</option><option value="manufacturer_certificate">Certificat fabricant</option><option value="verification">Vérification</option></select></label>
          <label><FieldCaption label="Date de mesure" required /><input type="date" value={performedOn} onChange={(event) => setPerformedOn(event.target.value)} /></label>
          <label><FieldCaption label="Valide à partir du" required /><input type="date" value={validFrom} onChange={(event) => setValidFrom(event.target.value)} /></label>
          <label><FieldCaption label="Valide jusqu’au" required /><input type="date" value={validUntil} onChange={(event) => setValidUntil(event.target.value)} /></label>
          <label><FieldCaption label="Laboratoire ou prestataire" required /><input value={provider} onChange={(event) => setProvider(event.target.value)} /></label>
          <label><FieldCaption label="Méthode utilisée" required /><input value={methodReference} onChange={(event) => setMethodReference(event.target.value)} placeholder="ex. MET-RF-CABLE-001" /></label>
          <label><FieldCaption label="Décision" required /><select value={decision} onChange={(event) => setDecision(event.target.value as typeof decision)}><option value="conforming">Conforme</option><option value="nonconforming">Non conforme</option><option value="indeterminate">Indéterminée</option><option value="not_assessed">Non évaluée</option></select></label>
        </div>
        {sourceKind === "calibration" && <div className="evidenceBoundaryNotice"><AlertTriangle size={17} /><div><strong>Valeurs mesurées uniquement</strong><p>Cette caractérisation enregistre les valeurs mesurées. Elle ne crée pas l'événement d'étalonnage global du matériel.</p>{!hasCalibrationEvent && <button className="secondary" type="button" onClick={onRecordCalibration}>Enregistrer d'abord l'étalonnage</button>}</div></div>}
      </section>

      {kind === "time_conversion" ? (
        <section className="editorCard">
          <p className="stepEyebrow">Étape 2 · Valeurs</p>
          <h2>Facteur, offset et limites</h2>
          <p className="contextHint">Résultat = valeur brute × facteur + offset.</p>
          <div className="formGrid">
            <label>Grandeur brute<select value={inputQuantity} onChange={(event) => changeQuantity(event.target.value, "input")}>{quantityChoices.map(([value, text]) => <option key={value} value={value}>{text}</option>)}</select></label>
            <label>Unité brute<input value={inputUnit} readOnly /></label>
            <label>Grandeur du résultat<select value={outputQuantity} onChange={(event) => changeQuantity(event.target.value, "output")}>{quantityChoices.map(([value, text]) => <option key={value} value={value}>{text}</option>)}</select></label>
            <label>Unité du résultat<input value={outputUnit} onChange={(event) => setOutputUnit(event.target.value)} /></label>
            <label><FieldCaption label="Facteur de conversion" required /><input type="number" step="any" value={scale} onChange={(event) => setScale(event.target.value)} /></label>
            <label><FieldCaption label="Offset" required /><input type="number" step="any" value={offset} onChange={(event) => setOffset(event.target.value)} /></label>
            <label>Limite basse<input type="number" step="any" value={minimum} onChange={(event) => setMinimum(event.target.value)} placeholder="Optionnelle" /></label>
            <label>Limite haute<input type="number" step="any" value={maximum} onChange={(event) => setMaximum(event.target.value)} placeholder="Optionnelle" /></label>
            <label>En cas de dépassement<select value={limitHandling} onChange={(event) => setLimitHandling(event.target.value)}><option value="warn">Avertir</option><option value="reject">Refuser la valeur</option><option value="mark_clipped">Marquer comme écrêtée</option></select></label>
          </div>
        </section>
      ) : (
        <section className="editorCard">
          <p className="stepEyebrow">Étape 2 · Valeurs</p>
          <h2>Correction d’amplitude et de phase</h2>
          <div className="formGrid compactGrid">
            <label>Type de réponse<select value={curveType} onChange={(event) => setCurveType(event.target.value)}><option value="cable_loss">Pertes de câble</option><option value="amplifier_gain">Gain d’amplificateur</option><option value="attenuator_loss">Pertes d’atténuateur</option><option value="sensor_frequency_response">Réponse d’un capteur</option><option value="generic_correction">Autre correction</option></select></label>
            <label>Application de l’amplitude<select value={operation} onChange={(event) => setOperation(event.target.value)}><option value="add">Ajouter la valeur en dB</option><option value="subtract">Soustraire la valeur en dB</option><option value="multiply">Multiplier</option><option value="divide">Diviser</option></select></label>
            <label className="checkboxField"><input type="checkbox" checked={phaseIncluded} onChange={(event) => setPhaseIncluded(event.target.checked)} /> Inclure une correction de phase</label>
          </div>
          <label>
            <FieldCaption label="Tableau mesuré" required />
            <textarea className="csvInput" value={frequencyCsv} onChange={(event) => setFrequencyCsv(event.target.value)} />
          </label>
          <p className="contextHint">En-têtes attendus : <code>frequence_hz,amplitude_db{phaseIncluded ? ",phase_deg" : ""}</code>. Au moins deux fréquences strictement croissantes.</p>
          {preview && (
            <div className="frequencyPreviewLayout">
              <FrequencyCorrectionChart points={preview} />
              <div className="frequencyPreview">
                <strong>Aperçu de la correction</strong>
                <span>{formatFrequency(preview[0].axis_values.frequency)} à {formatFrequency(preview[preview.length - 1].axis_values.frequency)}</span>
                <small>{preview.length} points · interpolation entre les points · aucune extrapolation</small>
              </div>
            </div>
          )}
        </section>
      )}

      <section className="editorCard">
        <p className="stepEyebrow">Étape 3 · Preuves</p>
        <h2>Incertitude, conditions et document</h2>
        <div className="formGrid">
          <label>Incertitude élargie<input type="number" min="0" step="any" value={uncertainty} onChange={(event) => setUncertainty(event.target.value)} placeholder="Optionnelle" /></label>
          <label>Unité<input value={uncertaintyUnit} onChange={(event) => setUncertaintyUnit(event.target.value)} /></label>
          <label>Facteur d’élargissement<input type="number" min="0.01" step="any" value={coverageFactor} onChange={(event) => setCoverageFactor(event.target.value)} /></label>
          <label>Niveau de confiance (%)<input type="number" min="0" max="100" step="any" value={confidenceLevel} onChange={(event) => setConfidenceLevel(event.target.value)} /></label>
          <label>Référence du certificat ou feuillet<input value={certificateReference} onChange={(event) => setCertificateReference(event.target.value)} /></label>
          <label>Température (°C)<input type="number" step="any" value={temperatureC} onChange={(event) => setTemperatureC(event.target.value)} placeholder="Optionnelle" /></label>
          <label>Humidité relative (%)<input type="number" min="0" max="100" step="any" value={humidityPercent} onChange={(event) => setHumidityPercent(event.target.value)} placeholder="Optionnelle" /></label>
          <label className="fileField"><span>Document de preuve</span><input type="file" onChange={(event) => setProofFile(event.target.files?.[0] ?? null)} /><small>{proofFile ? proofFile.name : initialEvidence?.documentManifest?.original_filename ?? "PDF, feuille de calcul ou document associé"}</small></label>
        </div>
        <div className="formGrid">
          <label>État constaté avant intervention<textarea value={asFound} onChange={(event) => setAsFound(event.target.value)} placeholder="Valeurs ou observation avant réglage" /></label>
          <label>État laissé après intervention<textarea value={asLeft} onChange={(event) => setAsLeft(event.target.value)} placeholder="Valeurs ou observation après réglage" /></label>
          <label className="checkboxField"><input type="checkbox" checked={adjustmentPerformed} onChange={(event) => setAdjustmentPerformed(event.target.checked)} /> Un réglage a été effectué</label>
        </div>
        <label>Commentaire<textarea value={comment} onChange={(event) => setComment(event.target.value)} /></label>
      </section>

      <div className="buttonRow stickyActions">
        <button type="button" className="secondary" onClick={onCancel}>Annuler</button>
        <button type="submit" disabled={submitting}><CheckCircle2 size={16} /> {submitting ? "Enregistrement…" : "Enregistrer puis préparer la revue"}</button>
      </div>
    </form>
  );
}

function CharacterizationDetail({ characterization, audit }: { characterization: AssetCharacterization; audit: MetrologyAuditEvent[] }) {
  const correction = characterization.definition.correction.correction;
  const frequencyPoints = characterization.characterization_kind === "frequency_response"
    ? arrayOfObjects(correction.points)
    : [];
  const uncertainty = characterization.definition.uncertainty;
  return (
    <article className="characterizationDetail">
      <header>
        <div>
          <p className="eyebrow">{characterizationKindLabel(characterization.characterization_kind)}</p>
          <h3>{characterization.label}</h3>
        </div>
        <span className={`status ${characterizationStatusClass(characterization)}`}>
          {characterizationStatus(characterization)}
        </span>
      </header>
      <dl className="businessSummary">
        <dt>Mesurée le</dt><dd>{formatDate(characterization.performed_on)}</dd>
        <dt>Valide à partir du</dt><dd>{formatDate(characterization.valid_from)}</dd>
        <dt>Valide jusqu’au</dt><dd>{formatDate(characterization.valid_until)}</dd>
        <dt>Source</dt><dd>{sourceKindLabel(characterization.source_kind)}</dd>
        <dt>Origine</dt><dd>{characterization.provider}</dd>
        <dt>Méthode</dt><dd>{characterization.method_reference}</dd>
        <dt>Décision</dt><dd>{decisionLabel(characterization.decision)}</dd>
        {characterization.certificate_reference && <><dt>Preuve</dt><dd>{characterization.certificate_reference}</dd></>}
      </dl>
      {characterization.characterization_kind === "time_conversion" ? (
        <div className="correctionResultSummary">
          <strong>Conversion appliquée aux échantillons</strong>
          <span>Résultat = valeur brute × {numberValue(objectValue(correction.parameters).scale)} + {numberValue(objectValue(correction.parameters).offset)}</span>
          {objectValue(correction.input_limits).minimum !== undefined && (
            <small>Plage brute : {numberValue(objectValue(correction.input_limits).minimum)} à {numberValue(objectValue(correction.input_limits).maximum)} {String(correction.input_unit ?? "")}</small>
          )}
        </div>
      ) : (
        <div className="correctionResultSummary">
          <strong>Réponse mesurée en fréquence</strong>
          <span>{frequencyPoints.length} points, de {formatFrequency(pointFrequency(frequencyPoints[0]))} à {formatFrequency(pointFrequency(frequencyPoints[frequencyPoints.length - 1]))}</span>
          <small>Amplitude en dB{hasPhase(correction) ? " et phase en degrés" : ""} · hors domaine interdit</small>
        </div>
      )}
      {uncertainty?.expanded_uncertainty !== undefined && (
        <p className="uncertaintyStatement">Incertitude élargie : {uncertainty.expanded_uncertainty} {uncertainty.unit} (k = {uncertainty.coverage_factor ?? "-"}, confiance {uncertainty.confidence_level_percent ?? "-"} %)</p>
      )}
      {Object.keys(characterization.environmental_conditions).length > 0 && (
        <p className="uncertaintyStatement">Conditions : {friendlyConditions(characterization.environmental_conditions)}</p>
      )}
      {(characterization.as_found || characterization.as_left || characterization.adjustment_performed) && (
        <div className="asFoundAsLeftSummary">
          {characterization.as_found && <p><strong>Avant intervention</strong><span>{String(characterization.as_found.summary ?? "Valeurs enregistrées")}</span></p>}
          {characterization.as_left && <p><strong>Après intervention</strong><span>{String(characterization.as_left.summary ?? "Valeurs enregistrées")}</span></p>}
          <small>{characterization.adjustment_performed ? "Un réglage a été effectué." : "Aucun réglage déclaré."}</small>
        </div>
      )}
      {characterization.document_manifest && (
        <p className="documentEvidence"><FileUp size={16} /> {characterization.document_manifest.original_filename}</p>
      )}
      {characterization.comment && <p>{characterization.comment}</p>}
      <details>
        <summary>Traçabilité technique</summary>
        <dl className="businessSummary compact">
          <dt>Enregistré par</dt><dd>{characterization.recorded_by}</dd>
          <dt>Enregistré le</dt><dd>{formatDateTime(characterization.recorded_at)}</dd>
          <dt>Empreinte</dt><dd className="mono">{characterization.definition_checksum}</dd>
        </dl>
        <h4>Journal</h4>
        {audit.map((event) => <p key={event.sequence}>{formatDateTime(event.occurred_at)} · {event.actor} · {event.reason}</p>)}
      </details>
    </article>
  );
}

function FrequencyCorrectionChart({ points }: { points: ReturnType<typeof parseFrequencyCsv> }) {
  const amplitudes = points.map((point) => Number(point.values.amplitude));
  const minimum = Math.min(...amplitudes);
  const maximum = Math.max(...amplitudes);
  const range = maximum - minimum || 1;
  const coordinates = amplitudes.map((amplitude, index) => {
    const x = points.length === 1 ? 50 : (index / (points.length - 1)) * 100;
    const y = 36 - ((amplitude - minimum) / range) * 30;
    return `${x},${y}`;
  }).join(" ");
  return (
    <figure className="frequencyCorrectionChart" aria-label="Aperçu graphique de la correction d'amplitude">
      <svg viewBox="0 0 100 42" role="img" aria-label={`Correction de ${minimum} à ${maximum} dB`}>
        <line x1="0" y1="36" x2="100" y2="36" />
        <polyline points={coordinates} />
        {coordinates.split(" ").map((coordinate) => {
          const [cx, cy] = coordinate.split(",");
          return <circle key={coordinate} cx={cx} cy={cy} r="1.7" />;
        })}
      </svg>
      <figcaption>Amplitude (dB) selon la fréquence</figcaption>
    </figure>
  );
}

function RegisterPhysicalAssetForm(props: PhysicalAssetMetrologyPanelProps & { onRegistered: (assetId: string) => void }) {
  const [modelId, setModelId] = useState(props.approvedModels[0]?.identity.equipment_model_id ?? "");
  const [assetId, setAssetId] = useState("");
  const [serialNumber, setSerialNumber] = useState("");
  const [partNumber, setPartNumber] = useState("");
  const [calibrationRequirement, setCalibrationRequirement] = useState<MetrologyInstrument["calibration_requirement"]>("required");
  const [calibrationPeriod, setCalibrationPeriod] = useState("12");
  const [warningDays, setWarningDays] = useState("45");
  const [serviceability, setServiceability] = useState<MetrologyInstrument["serviceability_status"]>("usable");
  const [notes, setNotes] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const selectedModel = props.approvedModels.find((model) => model.identity.equipment_model_id === modelId);
  const approvedRevision = selectedModel?.current_approved_revision;
  const calibrationApplies = calibrationRequirement !== "not_required";

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFormError(null);
    if (!selectedModel || !approvedRevision) {
      setFormError("Choisissez un modèle approuvé.");
      return;
    }
    if (!assetId.trim() || !serialNumber.trim()) {
      setFormError("Le numéro d’inventaire et le numéro de série sont obligatoires.");
      return;
    }
    try {
      await props.onRegister({
        asset_id: assetId.trim(),
        family: selectedModel.identity.root_category_id ?? selectedModel.identity.category_code,
        equipment_model_id: selectedModel.identity.equipment_model_id,
        equipment_model_revision_id: approvedRevision.revision_id,
        equipment_model_checksum: approvedRevision.definition_checksum,
        manufacturer: selectedModel.identity.manufacturer,
        model: selectedModel.identity.model_name,
        serial_number: serialNumber.trim(),
        part_number: partNumber.trim() || undefined,
        calibration_requirement: calibrationRequirement,
        calibration_period_months: calibrationApplies ? optionalPositiveInteger(calibrationPeriod) : undefined,
        calibration_due_warning_days: calibrationApplies ? optionalPositiveInteger(warningDays) : undefined,
        serviceability_status: serviceability,
        serviceability_reason: "Enregistrement initial depuis LAB CONSOLE",
        capabilities: approvedRevision.definition.capabilities,
        metrology_notes: notes.trim(),
        actor: "metrology.operator",
        reason: "enregistrement d’un matériel réel depuis le catalogue"
      });
      props.onRegistered(assetId.trim());
    } catch (reason) {
      setFormError(characterizationErrorMessage(reason));
    }
  }

  return (
    <>
      <div className="studioHeader">
        <div><p className="eyebrow">Registre métrologique</p><h2>Enregistrer un matériel réel</h2></div>
      </div>
      <form className="physicalAssetForm" onSubmit={submit}>
        {formError && <p className="errorText">{formError}</p>}
        {props.approvedModels.length === 0 && (
          <div className="workflowNotice">
            <div><strong>Aucun modèle approuvé n’est disponible</strong><p>Un matériel réel doit être rattaché à un modèle commun approuvé.</p></div>
            <button type="button" onClick={props.onOpenCatalog}>Ouvrir le catalogue</button>
          </div>
        )}
        <section className="editorCard">
          <h2>Modèle et identification</h2>
          <div className="formGrid">
            <label><FieldCaption label="Modèle constructeur" required /><select value={modelId} onChange={(event) => setModelId(event.target.value)}><option value="">Choisir un modèle approuvé</option>{props.approvedModels.map((model) => <option key={model.identity.equipment_model_id} value={model.identity.equipment_model_id}>{model.identity.manufacturer} {model.identity.model_name} · {categoryLabel(props.categories, model.identity.category_code)}</option>)}</select></label>
            <label><FieldCaption label="Code inventaire" required /><input value={assetId} onChange={(event) => setAssetId(event.target.value)} placeholder="ex. SA-001" /></label>
            <label>Numéro de série <span className="fieldHint">Facultatif</span><input value={serialNumber} onChange={(event) => setSerialNumber(event.target.value)} /></label>
            <label>Référence fabricant <span className="fieldHint">Facultatif</span><input value={partNumber} onChange={(event) => setPartNumber(event.target.value)} /></label>
          </div>
        </section>
        <section className="editorCard">
          <h2>Aptitude et étalonnage</h2>
          <div className="formGrid">
            <label><FieldCaption label="Exigence d’étalonnage" required /><select value={calibrationRequirement} onChange={(event) => setCalibrationRequirement(event.target.value as typeof calibrationRequirement)}><option value="required">Requis</option><option value="conditional">Selon utilisation</option><option value="not_required">Non applicable</option></select></label>
            {calibrationApplies && <label><FieldCaption label="Périodicité (mois)" required /><input type="number" min="1" value={calibrationPeriod} onChange={(event) => setCalibrationPeriod(event.target.value)} /></label>}
            {calibrationApplies && <label><FieldCaption label="Alerte avant échéance (jours)" required /><input type="number" min="1" value={warningDays} onChange={(event) => setWarningDays(event.target.value)} /></label>}
            <label><FieldCaption label="État de service" required /><select value={serviceability} onChange={(event) => setServiceability(event.target.value as typeof serviceability)}><option value="usable">Utilisable</option><option value="restricted">Utilisation restreinte</option><option value="out_of_service">Hors service</option><option value="retired">Retiré</option></select></label>
          </div>
          <label>Notes métrologiques<textarea value={notes} onChange={(event) => setNotes(event.target.value)} /></label>
        </section>
        <div className="buttonRow"><button type="submit" disabled={!selectedModel}><PackagePlus size={16} /> Enregistrer le matériel</button></div>
      </form>
    </>
  );
}

function parseFrequencyCsv(csv: string, phaseIncluded: boolean) {
  const lines = csv.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  if (lines.length < 3) throw new Error("Le tableau doit contenir un en-tête et au moins deux mesures.");
  const separator = lines[0].includes(";") ? ";" : ",";
  const headers = lines[0].split(separator).map((value) => value.trim().toLowerCase());
  const frequencyIndex = headers.indexOf("frequence_hz");
  const amplitudeIndex = headers.indexOf("amplitude_db");
  const phaseIndex = headers.indexOf("phase_deg");
  if (frequencyIndex < 0 || amplitudeIndex < 0 || (phaseIncluded && phaseIndex < 0)) {
    throw new Error(`Utilisez les en-têtes frequence_hz,amplitude_db${phaseIncluded ? ",phase_deg" : ""}.`);
  }
  const points = lines.slice(1).map((line, index) => {
    const values = line.split(separator).map((value) => value.trim());
    const frequency = Number(values[frequencyIndex]);
    const amplitude = Number(values[amplitudeIndex]);
    const phase = phaseIncluded ? Number(values[phaseIndex]) : undefined;
    if (!Number.isFinite(frequency) || frequency <= 0 || !Number.isFinite(amplitude) || (phaseIncluded && !Number.isFinite(phase))) {
      throw new Error(`La ligne ${index + 2} contient une valeur invalide.`);
    }
    return {
      axis_values: { frequency },
      values: { amplitude, ...(phaseIncluded ? { phase } : {}) }
    };
  }).sort((left, right) => left.axis_values.frequency - right.axis_values.frequency);
  if (points.some((point, index) => index > 0 && point.axis_values.frequency <= points[index - 1].axis_values.frequency)) {
    throw new Error("Chaque fréquence doit être unique.");
  }
  return points;
}

function newCharacterizationId(assetId: string) {
  const safeAsset = assetId.replace(/[^A-Za-z0-9_-]/g, "-");
  return `CHAR-${safeAsset}-${crypto.randomUUID().replace(/-/g, "").slice(0, 10)}`;
}

function newCalibrationEventId(assetId: string) {
  const safeAsset = assetId.replace(/[^A-Za-z0-9_-]/g, "-");
  return `CAL-${safeAsset}-${crypto.randomUUID().replace(/-/g, "").slice(0, 10)}`;
}

function newCorrectionAssignmentId(assetId: string, requirementId: string) {
  const safeAsset = assetId.replace(/[^A-Za-z0-9_-]/g, "-");
  const safeRequirement = requirementId.replace(/[^A-Za-z0-9_-]/g, "-");
  return `CORR-${safeAsset}-${safeRequirement}-${crypto.randomUUID().replace(/-/g, "").slice(0, 8)}`;
}

function requiredFiniteNumber(value: string, label: string) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) throw new Error(`${label} doit être un nombre.`);
  return parsed;
}

function optionalFiniteNumber(value: string, label: string) {
  if (!value.trim()) return undefined;
  return requiredFiniteNumber(value, label);
}

function optionalPositiveInteger(value: string) {
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : undefined;
}

function FieldCaption({ label, required }: { label: string; required?: boolean }) {
  return <span className="fieldCaption">{label}{required && <span className="requiredMark">Obligatoire</span>}</span>;
}

function characterizationErrorMessage(error: unknown) {
  if (error instanceof ApiError) {
    if (error.code === "invalid_asset_characterization") return "La caractérisation contient une valeur incohérente. Vérifiez la correction, les unités et les dates.";
    if (error.code === "operation_replay_mismatch") return "Cette opération a déjà été utilisée avec d’autres valeurs. Relancez l’enregistrement.";
    if (error.code === "metrology_instrument_not_found") return "Le matériel n’existe plus dans le registre métrologique.";
    if (error.code === "asset_characterization_already_exists") return "Cette caractérisation est déjà enregistrée.";
    if (error.code === "asset_correction_revision_conflict") return "Cette correction a été modifiée depuis son ouverture. Le dossier a été rafraîchi.";
    if (error.code === "asset_correction_source_not_conforming") return "La preuve n'est pas conforme et ne peut pas être activée.";
    if (error.code === "asset_correction_model_pin_changed") return "Le modèle lié au matériel a changé. Préparez une nouvelle affectation.";
    return error.message;
  }
  return error instanceof Error ? error.message : "L’opération métrologique a échoué.";
}

function calibrationErrorMessage(error: unknown) {
  if (error instanceof ApiError) {
    if (error.code === "invalid_metrology_calibration") return "Les dates ou la décision d'étalonnage sont incohérentes.";
    if (error.code === "metrology_calibration_already_exists") return "Ce certificat d'étalonnage est déjà enregistré pour ce matériel.";
    if (error.code === "operation_replay_mismatch") return "Cette opération a déjà été utilisée avec d'autres valeurs. Relancez l'enregistrement.";
    if (error.code === "metrology_instrument_not_found") return "Le matériel n'existe plus dans le registre métrologique.";
    return error.message;
  }
  return error instanceof Error ? error.message : "L'étalonnage n'a pas pu être enregistré.";
}

function correctionAssignmentStatusLabel(status: string) {
  return {
    draft: "Brouillon à soumettre",
    waiting_for_review: "En attente de revue",
    approved: "Approuvée",
    active: "Active pour ce matériel",
    expired: "Expirée",
    superseded: "Remplacée",
    rejected: "Rejetée",
    model_nominal: "Valeur nominale du modèle",
    missing: "Correction manquante"
  }[status] ?? "État à vérifier";
}

function characterizationStatus(item: AssetCharacterization) {
  if (item.decision === "nonconforming") return "Non conforme";
  if (item.decision !== "conforming") return "À examiner";
  const remaining = daysBetween(todayIso(), item.valid_until);
  if (remaining < 0) return "Expirée";
  if (remaining <= 30) return "À renouveler bientôt";
  return "Applicable";
}

function characterizationStatusClass(item: AssetCharacterization) {
  const status = characterizationStatus(item);
  if (status === "Applicable") return "approved";
  if (status === "À renouveler bientôt") return "warning";
  return "retired";
}

function characterizationKindLabel(kind: AssetCharacterization["characterization_kind"]) {
  return kind === "frequency_response" ? "Correction selon la fréquence" : "Conversion du signal brut";
}

function sourceKindLabel(kind: AssetCharacterization["source_kind"]) {
  return {
    calibration: "Valeurs issues d’un certificat d’étalonnage",
    characterization: "Rapport de caractérisation",
    verification: "Vérification",
    manufacturer_certificate: "Certificat fabricant",
    internal_measurement: "Mesure interne"
  }[kind];
}

function nominalCorrectionSummary(
  requirement: CorrectionRequirementDefinition,
  corrections: MeasurementEngineeringAggregate[]
) {
  const reference = requirement.model_default_reference;
  if (!reference) return "Aucune valeur nominale";
  const item = corrections.find((candidate) => candidate.identity.entity_id === reference.definition_id);
  const revision = item?.current_approved_revision?.revision_id === reference.revision_id
    ? item.current_approved_revision
    : item?.latest_revision?.revision_id === reference.revision_id
      ? item.latest_revision
      : null;
  return revision?.label || item?.identity.label || "Valeur documentée pour le modèle";
}

function correctionValueSummary(characterization?: AssetCharacterization) {
  if (!characterization) return "Preuve métrologique approuvée";
  const correction = characterization.definition.correction.correction;
  if (characterization.characterization_kind === "time_conversion") {
    const parameters = objectValue(correction.parameters);
    return `Facteur ${numberValue(parameters.scale)} · offset ${numberValue(parameters.offset)}`;
  }
  return `${arrayOfObjects(correction.points).length} points mesurés`;
}

function friendlyConditions(conditions: Record<string, unknown>) {
  const labels: Record<string, string> = {
    polarization: "Polarisation",
    orientation: "Orientation",
    channel: "Voie",
    distance: "Distance",
    temperature_c: "Température",
    relative_humidity_percent: "Humidité relative"
  };
  return Object.entries(conditions)
    .map(([key, value]) => {
      const suffix = key === "temperature_c" ? " °C" : key === "relative_humidity_percent" ? " %" : "";
      return `${labels[key] ?? key.replaceAll("_", " ")}: ${String(value)}${suffix}`;
    })
    .join(" · ");
}

function decisionLabel(decision: AssetCharacterization["decision"]) {
  return { conforming: "Conforme", nonconforming: "Non conforme", indeterminate: "Indéterminée", not_assessed: "Non évaluée" }[decision];
}

function serviceabilityLabel(status: MetrologyInstrument["serviceability_status"]) {
  return { usable: "Utilisable", restricted: "Usage restreint", out_of_service: "Hors service", retired: "Retiré" }[status];
}

function calibrationRequirementLabel(value: MetrologyInstrument["calibration_requirement"]) {
  return { required: "Étalonnage requis", conditional: "Selon l’utilisation", not_required: "Non applicable" }[value];
}

function calibrationStatusLabel(
  status: CalibrationStatus["calibration_status"] | undefined,
  requirement: MetrologyInstrument["calibration_requirement"]
) {
  if (!status) return requirement === "not_required" ? "Étalonnage non applicable" : "Statut à actualiser";
  return {
    valid: "Étalonnage valide",
    due_soon: "Étalonnage à renouveler bientôt",
    expired: "Étalonnage expiré",
    missing: "Aucun étalonnage valide",
    not_required: "Étalonnage non applicable",
    nonconforming: "Dernier étalonnage non conforme"
  }[status];
}

function calibrationStatusTone(status: CalibrationStatus["calibration_status"] | undefined) {
  if (status === "valid" || status === "not_required") return "ready";
  if (status === "due_soon") return "warning";
  return "blocked";
}

function calibrationStatusBadgeLabel(
  status: CalibrationStatus["calibration_status"] | undefined,
  requirement: MetrologyInstrument["calibration_requirement"]
) {
  if (!status) return requirement === "not_required" ? "Non applicable" : "À actualiser";
  return {
    valid: "Valide",
    due_soon: "À renouveler bientôt",
    expired: "Expiré",
    missing: "À enregistrer",
    not_required: "Non applicable",
    nonconforming: "Non conforme"
  }[status];
}

function calibrationEvidencePrefill(event: CalibrationEvent): CalibrationEvidencePrefill {
  return {
    certificateReference: event.certificate_reference,
    provider: event.provider,
    calibratedAt: event.calibrated_at,
    dueAt: event.due_at,
    documentManifest: parseJsonObject<EquipmentFileReference>(event.document_manifest_json) ?? undefined
  };
}

function uncertaintySummaryLabel(uncertainty: CalibrationUncertaintySummary) {
  const numeric = uncertainty.expanded_uncertainty !== undefined
    ? `${uncertainty.expanded_uncertainty} ${uncertainty.unit ?? ""}${uncertainty.coverage_factor !== undefined ? ` (k = ${uncertainty.coverage_factor})` : ""}`
    : "";
  return [numeric, uncertainty.statement].filter(Boolean).join(" · ") || "Résumé non renseigné";
}

function parseJsonObject<T>(value: string | null): T | null {
  if (!value) return null;
  try {
    const parsed = JSON.parse(value) as unknown;
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed as T : null;
  } catch {
    return null;
  }
}

function categoryLabel(categories: EquipmentCategory[], categoryId: string) {
  return categories.find((category) => category.category_id === categoryId)?.label ?? categoryId;
}

function formatDate(value: string) {
  const [year, month, day] = value.slice(0, 10).split("-");
  return year && month && day ? `${day}/${month}/${year}` : value;
}

function formatDateTime(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("fr-FR");
}

function formatFrequency(value: number) {
  if (!Number.isFinite(value)) return "-";
  if (value >= 1e9) return `${value / 1e9} GHz`;
  if (value >= 1e6) return `${value / 1e6} MHz`;
  if (value >= 1e3) return `${value / 1e3} kHz`;
  return `${value} Hz`;
}

function operatorUnitLabel(unit: string) {
  return {
    m_per_s2: "m/s²",
    meter_per_second_squared: "m/s²",
    degree: "°",
    dimensionless: "1"
  }[unit] ?? unit.replaceAll("_per_", "/").replaceAll("_", " ");
}

function todayIso() {
  const now = new Date();
  const localTime = new Date(now.getTime() - now.getTimezoneOffset() * 60_000);
  return localTime.toISOString().slice(0, 10);
}

function oneYearAfter(value: string) {
  const date = new Date(`${value}T00:00:00Z`);
  date.setUTCFullYear(date.getUTCFullYear() + 1);
  return date.toISOString().slice(0, 10);
}

function daysBetween(from: string, to: string) {
  return Math.floor((Date.parse(`${to}T00:00:00Z`) - Date.parse(`${from}T00:00:00Z`)) / 86_400_000);
}

function objectValue(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
}

function arrayOfObjects(value: unknown) {
  return Array.isArray(value) ? value.map(objectValue) : [];
}

function numberValue(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

function pointFrequency(point: Record<string, unknown> | undefined) {
  return numberValue(objectValue(point?.axis_values).frequency);
}

function hasPhase(correction: Record<string, unknown>) {
  return arrayOfObjects(correction.dependent_values).some((value) => value.component === "phase");
}
