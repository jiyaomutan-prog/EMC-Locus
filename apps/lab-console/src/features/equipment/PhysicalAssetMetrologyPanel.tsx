import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  FileUp,
  PackagePlus,
  Plus,
  Radio,
  X
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { ApiError, metrologyApi } from "../../api";
import type {
  EquipmentCategory,
  EquipmentFileReference,
  EquipmentModelAggregate
} from "../../models/equipment";
import type {
  AssetCharacterization,
  AssetCharacterizationDefinition,
  MetrologyAuditEvent,
  MetrologyInstrument,
  RegisterMetrologyInstrumentInput
} from "../../models/metrology";

interface PhysicalAssetMetrologyPanelProps {
  instruments: MetrologyInstrument[];
  approvedModels: EquipmentModelAggregate[];
  categories: EquipmentCategory[];
  onRegister: (input: RegisterMetrologyInstrumentInput) => Promise<void>;
  onOpenCatalog: () => void;
}

type CharacterizationKind = "time_conversion" | "frequency_response";

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
  const [selectedCharacterizationId, setSelectedCharacterizationId] = useState("");
  const [audit, setAudit] = useState<MetrologyAuditEvent[]>([]);
  const [creatingCharacterization, setCreatingCharacterization] = useState(false);
  const [loadingCharacterizations, setLoadingCharacterizations] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!selectedAssetId && props.instruments.length > 0) {
      setSelectedAssetId(props.instruments[0].asset_id);
      setRegistering(false);
    }
  }, [props.instruments, selectedAssetId]);

  useEffect(() => {
    if (!selectedAssetId) {
      setCharacterizations([]);
      setSelectedCharacterizationId("");
      return;
    }
    let cancelled = false;
    setLoadingCharacterizations(true);
    setError(null);
    metrologyApi
      .listCharacterizations(selectedAssetId)
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
        if (!cancelled) setError(characterizationErrorMessage(reason));
      })
      .finally(() => {
        if (!cancelled) setLoadingCharacterizations(false);
      });
    return () => {
      cancelled = true;
    };
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
  const selectedCharacterization = characterizations.find(
    (item) => item.characterization_id === selectedCharacterizationId
  );

  async function recordCharacterization(input: Parameters<typeof metrologyApi.recordCharacterization>[1]) {
    if (!selectedAsset) return;
    setError(null);
    try {
      const response = await metrologyApi.recordCharacterization(selectedAsset.asset_id, input);
      const refreshed = await metrologyApi.listCharacterizations(selectedAsset.asset_id);
      setCharacterizations(refreshed.characterizations);
      setSelectedCharacterizationId(response.characterization.characterization_id);
      setCreatingCharacterization(false);
    } catch (reason) {
      setError(characterizationErrorMessage(reason));
      throw reason;
    }
  }

  return (
    <div className="equipmentLayout physicalAssetsLayout">
      <aside className="equipmentList physicalAssetList">
        <div className="listHeader">
          <h2>Matériels</h2>
          <span>{props.instruments.length}</span>
        </div>
        <button
          type="button"
          className="secondary fullWidthAction"
          onClick={() => {
            setRegistering(true);
            setCreatingCharacterization(false);
          }}
        >
          <PackagePlus size={16} /> Enregistrer un matériel
        </button>
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
              <strong>{instrument.asset_id}</strong>
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
        {registering || !selectedAsset ? (
          <RegisterPhysicalAssetForm {...props} onRegistered={() => setRegistering(false)} />
        ) : (
          <>
            <header className="studioHeader assetRecordHeader">
              <div>
                <p className="eyebrow">Dossier métrologique</p>
                <h2>{selectedAsset.asset_id}</h2>
                <p>{selectedAsset.manufacturer} {selectedAsset.model} · N° de série {selectedAsset.serial_number}</p>
              </div>
              {!creatingCharacterization && (
                <button type="button" onClick={() => setCreatingCharacterization(true)}>
                  <Plus size={16} /> Ajouter une caractérisation
                </button>
              )}
            </header>

            <AssetSummary asset={selectedAsset} />

            {creatingCharacterization ? (
              <CharacterizationForm
                asset={selectedAsset}
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
                {!loadingCharacterizations && characterizations.length === 0 && (
                  <div className="workflowEmpty">
                    <Activity size={24} />
                    <div>
                      <strong>Aucune correction propre à ce matériel</strong>
                      <p>Ajoutez une caractérisation issue d’un certificat ou d’une mesure interne.</p>
                    </div>
                    <button type="button" onClick={() => setCreatingCharacterization(true)}>
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
        )}
      </section>
    </div>
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
  onCancel: () => void;
  onRecord: (input: Parameters<typeof metrologyApi.recordCharacterization>[1]) => Promise<void>;
}

function CharacterizationForm({ asset, onCancel, onRecord }: CharacterizationFormProps) {
  const today = todayIso();
  const [kind, setKind] = useState<CharacterizationKind>("frequency_response");
  const [label, setLabel] = useState("Pertes mesurées");
  const [performedOn, setPerformedOn] = useState(today);
  const [validUntil, setValidUntil] = useState(oneYearAfter(today));
  const [provider, setProvider] = useState("");
  const [methodReference, setMethodReference] = useState("");
  const [decision, setDecision] = useState<AssetCharacterization["decision"]>("conforming");
  const [certificateReference, setCertificateReference] = useState("");
  const [proofFile, setProofFile] = useState<File | null>(null);
  const [comment, setComment] = useState("");
  const [uncertainty, setUncertainty] = useState("");
  const [uncertaintyUnit, setUncertaintyUnit] = useState("dB");
  const [coverageFactor, setCoverageFactor] = useState("2");
  const [confidenceLevel, setConfidenceLevel] = useState("95");
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
    if (validUntil < performedOn) {
      setFormError("La date de validité doit être postérieure à la date de mesure.");
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
      let documentManifest: EquipmentFileReference | undefined;
      if (proofFile) {
        documentManifest = (await metrologyApi.uploadFile(proofFile)).file;
      }
      await onRecord({
        characterization_id: characterizationId,
        performed_on: performedOn,
        valid_until: validUntil,
        provider: provider.trim(),
        method_reference: methodReference.trim(),
        decision,
        definition,
        certificate_reference: certificateReference.trim() || undefined,
        document_manifest: documentManifest,
        comment: comment.trim() || undefined,
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
          <h2>Ajouter une caractérisation</h2>
          <p>La mesure sera enregistrée pour {asset.asset_id}, N° de série {asset.serial_number}.</p>
        </div>
        <button type="button" className="iconButton" title="Fermer" onClick={onCancel}><X size={18} /></button>
      </header>

      {formError && <p className="errorText"><AlertTriangle size={16} /> {formError}</p>}

      <section className="editorCard">
        <h2>Quel résultat a été mesuré ?</h2>
        <div className="signalModeControl" role="group" aria-label="Type de caractérisation">
          <button type="button" className={kind === "time_conversion" ? "active" : ""} onClick={() => { setKind("time_conversion"); setLabel("Sensibilité mesurée"); setUncertaintyUnit(outputUnit); }}>
            <Activity size={18} />
            <span><strong>Conversion temporelle</strong><small>Facteur, offset et limites propres au matériel</small></span>
          </button>
          <button type="button" className={kind === "frequency_response" ? "active" : ""} onClick={() => { setKind("frequency_response"); setLabel("Pertes mesurées"); setUncertaintyUnit("dB"); }}>
            <Radio size={18} />
            <span><strong>Réponse fréquentielle</strong><small>Amplitude et phase optionnelle selon la fréquence</small></span>
          </button>
        </div>
      </section>

      <section className="editorCard">
        <h2>Origine et validité</h2>
        <div className="formGrid">
          <label><FieldCaption label="Nom de la caractérisation" required /><input value={label} onChange={(event) => setLabel(event.target.value)} placeholder="ex. Pertes du câble RF après contrôle" /></label>
          <label><FieldCaption label="Date de mesure" required /><input type="date" value={performedOn} onChange={(event) => setPerformedOn(event.target.value)} /></label>
          <label><FieldCaption label="Valide jusqu’au" required /><input type="date" value={validUntil} onChange={(event) => setValidUntil(event.target.value)} /></label>
          <label><FieldCaption label="Laboratoire ou prestataire" required /><input value={provider} onChange={(event) => setProvider(event.target.value)} /></label>
          <label><FieldCaption label="Méthode utilisée" required /><input value={methodReference} onChange={(event) => setMethodReference(event.target.value)} placeholder="ex. MET-RF-CABLE-001" /></label>
          <label><FieldCaption label="Décision" required /><select value={decision} onChange={(event) => setDecision(event.target.value as typeof decision)}><option value="conforming">Conforme</option><option value="nonconforming">Non conforme</option><option value="indeterminate">Indéterminée</option><option value="not_assessed">Non évaluée</option></select></label>
        </div>
      </section>

      {kind === "time_conversion" ? (
        <section className="editorCard">
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
            <div className="frequencyPreview">
              <strong>Domaine mesuré</strong>
              <span>{formatFrequency(preview[0].axis_values.frequency)} à {formatFrequency(preview[preview.length - 1].axis_values.frequency)}</span>
              <small>{preview.length} points · extrapolation interdite</small>
            </div>
          )}
        </section>
      )}

      <section className="editorCard">
        <h2>Incertitude et preuve</h2>
        <div className="formGrid">
          <label>Incertitude élargie<input type="number" min="0" step="any" value={uncertainty} onChange={(event) => setUncertainty(event.target.value)} placeholder="Optionnelle" /></label>
          <label>Unité<input value={uncertaintyUnit} onChange={(event) => setUncertaintyUnit(event.target.value)} /></label>
          <label>Facteur d’élargissement<input type="number" min="0.01" step="any" value={coverageFactor} onChange={(event) => setCoverageFactor(event.target.value)} /></label>
          <label>Niveau de confiance (%)<input type="number" min="0" max="100" step="any" value={confidenceLevel} onChange={(event) => setConfidenceLevel(event.target.value)} /></label>
          <label>Référence du certificat ou feuillet<input value={certificateReference} onChange={(event) => setCertificateReference(event.target.value)} /></label>
          <label className="fileField"><span>Document de preuve</span><input type="file" onChange={(event) => setProofFile(event.target.files?.[0] ?? null)} /><small>{proofFile ? proofFile.name : "PDF, feuille de calcul ou document associé"}</small></label>
        </div>
        <label>Commentaire<textarea value={comment} onChange={(event) => setComment(event.target.value)} /></label>
      </section>

      <div className="buttonRow stickyActions">
        <button type="button" className="secondary" onClick={onCancel}>Annuler</button>
        <button type="submit" disabled={submitting}><CheckCircle2 size={16} /> {submitting ? "Enregistrement…" : "Enregistrer la caractérisation"}</button>
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
        <dt>Valide jusqu’au</dt><dd>{formatDate(characterization.valid_until)}</dd>
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

function RegisterPhysicalAssetForm(props: PhysicalAssetMetrologyPanelProps & { onRegistered: () => void }) {
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
      props.onRegistered();
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
            <label><FieldCaption label="Modèle d’équipement" required /><select value={modelId} onChange={(event) => setModelId(event.target.value)}><option value="">Choisir un modèle approuvé</option>{props.approvedModels.map((model) => <option key={model.identity.equipment_model_id} value={model.identity.equipment_model_id}>{model.identity.manufacturer} {model.identity.model_name} · {categoryLabel(props.categories, model.identity.category_code)}</option>)}</select></label>
            <label><FieldCaption label="Numéro d’inventaire" required /><input value={assetId} onChange={(event) => setAssetId(event.target.value)} placeholder="ex. SA-001" /></label>
            <label><FieldCaption label="Numéro de série" required /><input value={serialNumber} onChange={(event) => setSerialNumber(event.target.value)} /></label>
            <label>Part number<input value={partNumber} onChange={(event) => setPartNumber(event.target.value)} /></label>
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
    return error.message;
  }
  return error instanceof Error ? error.message : "L’opération métrologique a échoué.";
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
  return kind === "frequency_response" ? "Réponse fréquentielle" : "Conversion temporelle";
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
