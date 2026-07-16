import type {
  ApiErrorBody,
  AuditEvent,
  BranchRule,
  ExecutionSequenceStep,
  HealthReport,
  InstrumentationChainSlot,
  LimitDefinition,
  PostProcessingDefinition,
  StorageStatus,
  TestTemplateAggregate,
  TestTemplateDefinition,
  TestTemplateRevision,
  ValidationResult
} from "./types";
import type {
  CommunicationProviderStatus,
  DriverProfileAggregate,
  DriverProfileDefinition,
  DriverProfileRevision,
  DriverSimulationResult,
  DriverSimulationScenario,
  EquipmentAuditEvent,
  EquipmentCategory,
  EquipmentCategoryFieldRule,
  EquipmentClassificationPreset,
  EquipmentEffectiveTemplate,
  EquipmentFieldDefinition,
  EquipmentFileReference,
  EquipmentRegistries,
  EquipmentModelAggregate,
  EquipmentModelDefinition,
  EquipmentModelRevision,
  EquipmentOperationResult,
  EquipmentValidationResult,
  EngineeringCurveEvaluation,
  MeasurementEngineeringAggregate,
  MeasurementEngineeringCollection,
  MeasurementEngineeringDefinition,
  MeasurementEngineeringOperationResult,
  MeasurementEngineeringRevision
} from "./models/equipment";
import type {
  AssetCharacterization,
  AssetCorrectionAssignmentEnvelope,
  AssetCorrectionResolutionReport,
  MetrologyAuditEvent,
  MetrologyInstrument,
  RecordAssetCharacterizationInput,
  RegisterMetrologyInstrumentInput
} from "./models/metrology";
import type {
  ContractReviewOperationResult,
  ContractReviewStatus,
  LaboratoryLocationOption,
  LaboratoryWeekSchedule,
  ProjectAuditEvent,
  ProjectExecutionMode,
  ProjectOperationResult,
  ProjectRecord,
  PlannedTestPreparationAggregate,
  PlannedTestPreparationOperationResult,
  PlannedTestPreparationOptions,
  PlannedTestPreparationRevision,
  ServiceScheduleItem,
  ServiceScheduleOperationResult,
  StationSetupLocationSource
} from "./models/projects";

export class ApiError extends Error {
  readonly code: string;
  readonly status: number;
  readonly details?: Record<string, unknown>;

  constructor(status: number, body: ApiErrorBody) {
    super(body.error.message);
    this.name = "ApiError";
    this.status = status;
    this.code = body.error.code;
    this.details = body.error.details;
  }
}

type RequestBody = Record<string, unknown>;

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(path, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...options.headers
    }
  });
  const text = await response.text();
  const body = text ? (JSON.parse(text) as T | ApiErrorBody) : ({} as T);
  if (!response.ok) {
    throw new ApiError(response.status, body as ApiErrorBody);
  }
  return body as T;
}

function post<T>(path: string, body: RequestBody): Promise<T> {
  return request<T>(path, { method: "POST", body: JSON.stringify(body) });
}

function put<T>(path: string, body: RequestBody): Promise<T> {
  return request<T>(path, { method: "PUT", body: JSON.stringify(body) });
}

export interface OperationContext {
  actor: string;
  reason: string;
}

export interface TemplateOperationResult {
  operation: string;
  operation_id: string;
  replayed: boolean;
  test_template: TestTemplateAggregate;
  revision: TestTemplateRevision;
}

export type EquipmentModelOperationResult = EquipmentOperationResult<
  EquipmentModelAggregate,
  EquipmentModelRevision
>;

export type DriverProfileOperationResult = EquipmentOperationResult<
  DriverProfileAggregate,
  DriverProfileRevision
>;

function operationAggregate<TAggregate>(
  result: { aggregate?: TAggregate; item?: TAggregate }
): TAggregate {
  const aggregate = result.aggregate ?? result.item;
  if (!aggregate) {
    throw new Error("agent operation result is missing aggregate/item");
  }
  return aggregate;
}

function normalizeDefinition(definition: TestTemplateDefinition): TestTemplateDefinition {
  return {
    ...definition,
    description: definition.description ?? "",
    standard_references: definition.standard_references ?? [],
    variables: (definition.variables ?? []).map((variable) => ({
      ...variable,
      constraints: {
        ...variable.constraints,
        required: variable.constraints?.required ?? false,
        enum_values: variable.constraints?.enum_values ?? []
      }
    })),
    lock_policy: definition.lock_policy ?? [],
    instrumentation_chain: (definition.instrumentation_chain ?? []).map(
      (slot: InstrumentationChainSlot) => ({
        ...slot,
        depends_on_slots: slot.depends_on_slots ?? []
      })
    ),
    sequence: (definition.sequence ?? []).map((step: ExecutionSequenceStep) => ({
      ...step,
      required_slots: step.required_slots ?? [],
      branches: (step.branches ?? []).map((branch: BranchRule) => ({
        ...branch,
        allow_cycle: branch.allow_cycle ?? false
      }))
    })),
    limits: (definition.limits ?? []).map((limit: LimitDefinition) => ({
      ...limit,
      variable_refs: limit.variable_refs ?? []
    })),
    post_processing: (definition.post_processing ?? []).map(
      (operation: PostProcessingDefinition) => ({
        ...operation,
        inputs: operation.inputs ?? [],
        outputs: operation.outputs ?? [],
        parameters: operation.parameters ?? {}
      })
    ),
    method_parameters: definition.method_parameters ?? {}
  };
}

function normalizeRevision(revision: TestTemplateRevision): TestTemplateRevision {
  return {
    ...revision,
    definition: normalizeDefinition(revision.definition)
  };
}

function normalizeAggregate(template: TestTemplateAggregate): TestTemplateAggregate {
  return {
    ...template,
    current_approved_revision: template.current_approved_revision
      ? normalizeRevision(template.current_approved_revision)
      : null,
    latest_revision: template.latest_revision ? normalizeRevision(template.latest_revision) : null,
    active_draft_revision: template.active_draft_revision
      ? normalizeRevision(template.active_draft_revision)
      : null
  };
}

function normalizeOperationResult(result: TemplateOperationResult): TemplateOperationResult {
  return {
    ...result,
    test_template: normalizeAggregate(result.test_template),
    revision: normalizeRevision(result.revision)
  };
}

export const api = {
  health: () => request<HealthReport>("/api/v1/health"),
  storageStatus: () => request<StorageStatus>("/api/v1/storage/status"),
  listTemplates: () =>
    request<{ test_templates: TestTemplateAggregate[] }>("/api/v1/test-templates").then((body) => ({
      test_templates: body.test_templates.map(normalizeAggregate)
    })),
  getTemplate: (templateId: string) =>
    request<{ test_template: TestTemplateAggregate }>(
      `/api/v1/test-templates/${encodeURIComponent(templateId)}`
    ).then((body) => ({ test_template: normalizeAggregate(body.test_template) })),
  getRevision: (templateId: string, revisionId: string) =>
    request<{ revision: TestTemplateRevision }>(
      `/api/v1/test-templates/${encodeURIComponent(templateId)}/revisions/${encodeURIComponent(
        revisionId
      )}`
    ).then((body) => ({ revision: normalizeRevision(body.revision) })),
  listRevisions: (templateId: string) =>
    request<{ template_id: string; revisions: TestTemplateRevision[] }>(
      `/api/v1/test-templates/${encodeURIComponent(templateId)}/revisions`
    ).then((body) => ({
      ...body,
      revisions: body.revisions.map(normalizeRevision)
    })),
  listAudit: (templateId: string) =>
    request<{ template_id: string; audit_events: AuditEvent[] }>(
      `/api/v1/test-templates/${encodeURIComponent(templateId)}/audit-events`
    ),
  validateDefinition: (definition: TestTemplateDefinition) =>
    post<ValidationResult>("/api/v1/test-template-definitions/validate", { definition }),
  createTemplate: (
    input: {
      template_id: string;
      title: string;
      category_code: string;
      definition: TestTemplateDefinition;
    } & OperationContext
  ) =>
    post<TemplateOperationResult>("/api/v1/test-templates", {
      ...input,
      operation_id: operationId("template-create", input.template_id)
    }).then(normalizeOperationResult),
  cloneTemplate: (
    sourceTemplateId: string,
    input: {
      source_revision_id: string;
      new_template_id: string;
      title: string;
      category_code?: string;
    } & OperationContext
  ) =>
    post<TemplateOperationResult>(`/api/v1/test-templates/${encodeURIComponent(sourceTemplateId)}/clone`, {
      ...input,
      operation_id: operationId("template-clone", input.new_template_id)
    }).then(normalizeOperationResult),
  saveDraft: (
    templateId: string,
    revisionId: string,
    expectedChecksum: string,
    definition: TestTemplateDefinition,
    context: OperationContext
  ) =>
    put<TemplateOperationResult>(
      `/api/v1/test-templates/${encodeURIComponent(templateId)}/revisions/${encodeURIComponent(
        revisionId
      )}/definition`,
      {
        expected_definition_checksum: expectedChecksum,
        definition,
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId("template-save", `${templateId}-${revisionId}`)
      }
    ).then(normalizeOperationResult),
  submitRevision: (templateId: string, revisionId: string, context: OperationContext) =>
    post<TemplateOperationResult>(
      `/api/v1/test-templates/${encodeURIComponent(templateId)}/revisions/${encodeURIComponent(
        revisionId
      )}/transitions/submit-for-review`,
      {
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId("template-submit", `${templateId}-${revisionId}`)
      }
    ).then(normalizeOperationResult),
  approveRevision: (templateId: string, revisionId: string, context: OperationContext) =>
    post<TemplateOperationResult>(
      `/api/v1/test-templates/${encodeURIComponent(templateId)}/revisions/${encodeURIComponent(
        revisionId
      )}/transitions/approve`,
      {
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId("template-approve", `${templateId}-${revisionId}`)
      }
    ).then(normalizeOperationResult),
  deriveRevision: (templateId: string, sourceRevisionId: string, context: OperationContext) =>
    post<TemplateOperationResult>(
      `/api/v1/test-templates/${encodeURIComponent(templateId)}/revisions`,
      {
        source_revision_id: sourceRevisionId,
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId("template-derive", `${templateId}-${sourceRevisionId}`)
      }
    ).then(normalizeOperationResult)
};

export const equipmentApi = {
  listModels: (filters: Record<string, string> = {}) => {
    const query = new URLSearchParams(
      Object.entries(filters).filter(([, value]) => value && value !== "all")
    );
    return request<{ equipment_models: EquipmentModelAggregate[] }>(
      `/api/v1/equipment-models${query.size ? `?${query.toString()}` : ""}`
    );
  },
  registries: () => request<EquipmentRegistries>("/api/v1/equipment/registries"),
  listCategories: (includeInactive = false) =>
    request<{ categories: EquipmentCategory[] }>(
      `/api/v1/equipment/categories${includeInactive ? "?include_inactive=true" : ""}`
    ),
  categoryTree: (includeInactive = false) =>
    request<{ categories: EquipmentCategory[] }>(
      `/api/v1/equipment/categories/tree${includeInactive ? "?include_inactive=true" : ""}`
    ),
  createCategory: (input: {
    category_id: string;
    parent_category_id: string;
    label: string;
    description?: string;
    sort_order?: number;
  }) => post<{ category: EquipmentCategory }>("/api/v1/equipment/categories", input),
  updateCategory: (
    categoryId: string,
    input: { label: string; description?: string; sort_order?: number; active?: boolean }
  ) => put<{ category: EquipmentCategory }>(`/api/v1/equipment/categories/${encodeURIComponent(categoryId)}`, input),
  archiveCategory: (categoryId: string) =>
    post<{ category: EquipmentCategory }>(
      `/api/v1/equipment/categories/${encodeURIComponent(categoryId)}/archive`,
      {}
    ),
  moveCategory: (categoryId: string, input: { parent_category_id: string; sort_order?: number }) =>
    post<{ category: EquipmentCategory }>(
      `/api/v1/equipment/categories/${encodeURIComponent(categoryId)}/move`,
      input
    ),
  listFieldDefinitions: (scope = "equipment_model", includeInactive = false) => {
    const query = new URLSearchParams({ scope });
    if (includeInactive) query.set("include_inactive", "true");
    return request<{ field_definitions: EquipmentFieldDefinition[] }>(
      `/api/v1/equipment/field-definitions?${query.toString()}`
    );
  },
  createFieldDefinition: (input: Partial<EquipmentFieldDefinition>) =>
    post<{ field_definition: EquipmentFieldDefinition }>("/api/v1/equipment/field-definitions", input),
  updateFieldDefinition: (fieldId: string, input: Partial<EquipmentFieldDefinition>) =>
    put<{ field_definition: EquipmentFieldDefinition }>(
      `/api/v1/equipment/field-definitions/${encodeURIComponent(fieldId)}`,
      input
    ),
  archiveFieldDefinition: (fieldId: string) =>
    post<{ field_definition: EquipmentFieldDefinition }>(
      `/api/v1/equipment/field-definitions/${encodeURIComponent(fieldId)}/archive`,
      {}
    ),
  categoryFieldRules: (categoryId: string) =>
    request<{ category_id: string; rules: EquipmentCategoryFieldRule[] }>(
      `/api/v1/equipment/categories/${encodeURIComponent(categoryId)}/field-rules`
    ),
  replaceCategoryFieldRules: (categoryId: string, rules: EquipmentCategoryFieldRule[]) =>
    put<{ category_id: string; rules: EquipmentCategoryFieldRule[] }>(
      `/api/v1/equipment/categories/${encodeURIComponent(categoryId)}/field-rules`,
      { rules }
    ),
  effectiveTemplate: (categoryId: string) =>
    request<{ effective_template: EquipmentEffectiveTemplate }>(
      `/api/v1/equipment/categories/${encodeURIComponent(categoryId)}/effective-template`
    ),
  uploadFile: async (file: File) =>
    post<{ file: EquipmentFileReference }>("/api/v1/equipment/files", {
      original_filename: file.name,
      mime_type: file.type || "application/octet-stream",
      content_base64: await fileToBase64(file)
    }),
  listClassificationPresets: () =>
    request<{ presets: EquipmentClassificationPreset[] }>("/api/v1/equipment/classification-presets"),
  getClassificationPreset: (presetId: string) =>
    request<{ preset: EquipmentClassificationPreset }>(
      `/api/v1/equipment/classification-presets/${encodeURIComponent(presetId)}`
    ),
  getModel: (modelId: string) =>
    request<{ equipment_model: EquipmentModelAggregate }>(
      `/api/v1/equipment-models/${encodeURIComponent(modelId)}`
    ),
  listModelRevisions: (modelId: string) =>
    request<{ equipment_model_id: string; revisions: EquipmentModelRevision[] }>(
      `/api/v1/equipment-models/${encodeURIComponent(modelId)}/revisions`
    ),
  listModelAudit: (modelId: string) =>
    request<{ aggregate_kind: string; entity_id: string; audit_events: EquipmentAuditEvent[] }>(
      `/api/v1/equipment-models/${encodeURIComponent(modelId)}/audit-events`
    ),
  validateModelDefinition: (definition: EquipmentModelDefinition) =>
    post<EquipmentValidationResult>("/api/v1/equipment-model-definitions/validate", { definition }),
  createModel: (
    input: {
      equipment_model_id: string;
      definition: EquipmentModelDefinition;
    } & OperationContext
  ) =>
    post<EquipmentModelOperationResult>("/api/v1/equipment-models", {
      ...input,
      operation_id: operationId("equipment-model-create", input.equipment_model_id)
    }),
  createModelFromPreset: (
    input: {
      preset_id: string;
      equipment_model_id: string;
      manufacturer: string;
      model_name: string;
      variant?: string;
      is_demo?: boolean;
    } & OperationContext
  ) =>
    post<EquipmentModelOperationResult>("/api/v1/equipment-models/from-preset", {
      ...input,
      operation_id: operationId("equipment-model-from-preset", `${input.preset_id}-${input.equipment_model_id}`)
    }),
  createModelFromCategoryTemplate: (
    input: {
      category_id: string;
      equipment_model_id?: string;
      field_values: Record<string, unknown>;
      is_demo?: boolean;
    } & OperationContext
  ) =>
    post<EquipmentModelOperationResult>("/api/v1/equipment-models/from-category-template", {
      ...input,
      operation_id: operationId(
        "equipment-model-from-category-template",
        `${input.category_id}-${String(input.equipment_model_id ?? input.field_values.model_name ?? "model")}`
      )
    }),
  cloneModel: (
    sourceModelId: string,
    input: {
      new_equipment_model_id: string;
      source_revision_id?: string;
      manufacturer?: string;
      model_name?: string;
      variant?: string;
    } & OperationContext
  ) =>
    post<EquipmentModelOperationResult>(
      `/api/v1/equipment-models/${encodeURIComponent(sourceModelId)}/clone`,
      {
        ...input,
        operation_id: operationId("equipment-model-clone", input.new_equipment_model_id)
      }
    ),
  saveModelDraft: (
    modelId: string,
    revisionId: string,
    expectedChecksum: string,
    definition: EquipmentModelDefinition,
    context: OperationContext
  ) =>
    put<EquipmentModelOperationResult>(
      `/api/v1/equipment-models/${encodeURIComponent(modelId)}/revisions/${encodeURIComponent(
        revisionId
      )}/definition`,
      {
        expected_definition_checksum: expectedChecksum,
        definition,
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId("equipment-model-save", `${modelId}-${revisionId}`)
      }
    ),
  submitModel: (modelId: string, revisionId: string, context: OperationContext) =>
    post<EquipmentModelOperationResult>(
      `/api/v1/equipment-models/${encodeURIComponent(modelId)}/revisions/${encodeURIComponent(
        revisionId
      )}/transitions/submit-for-review`,
      {
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId("equipment-model-submit", `${modelId}-${revisionId}`)
      }
    ),
  approveModel: (modelId: string, revisionId: string, context: OperationContext) =>
    post<EquipmentModelOperationResult>(
      `/api/v1/equipment-models/${encodeURIComponent(modelId)}/revisions/${encodeURIComponent(
        revisionId
      )}/transitions/approve`,
      {
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId("equipment-model-approve", `${modelId}-${revisionId}`)
      }
    ),
  deriveModelRevision: (modelId: string, sourceRevisionId: string, context: OperationContext) =>
    post<EquipmentModelOperationResult>(
      `/api/v1/equipment-models/${encodeURIComponent(modelId)}/revisions`,
      {
        source_revision_id: sourceRevisionId,
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId("equipment-model-derive", `${modelId}-${sourceRevisionId}`)
      }
    ),
  listDrivers: () =>
    request<{ driver_profiles: DriverProfileAggregate[] }>("/api/v1/driver-profiles"),
  listDriverRevisions: (driverId: string) =>
    request<{ driver_profile_id: string; revisions: DriverProfileRevision[] }>(
      `/api/v1/driver-profiles/${encodeURIComponent(driverId)}/revisions`
    ),
  listDriverAudit: (driverId: string) =>
    request<{ aggregate_kind: string; entity_id: string; audit_events: EquipmentAuditEvent[] }>(
      `/api/v1/driver-profiles/${encodeURIComponent(driverId)}/audit-events`
    ),
  validateDriverDefinition: (definition: DriverProfileDefinition) =>
    post<EquipmentValidationResult>("/api/v1/driver-profile-definitions/validate", { definition }),
  createDriver: (
    input: {
      driver_profile_id: string;
      label: string;
      definition: DriverProfileDefinition;
    } & OperationContext
  ) =>
    post<DriverProfileOperationResult>("/api/v1/driver-profiles", {
      ...input,
      operation_id: operationId("driver-profile-create", input.driver_profile_id)
    }),
  saveDriverDraft: (
    driverId: string,
    revisionId: string,
    expectedChecksum: string,
    definition: DriverProfileDefinition,
    context: OperationContext
  ) =>
    put<DriverProfileOperationResult>(
      `/api/v1/driver-profiles/${encodeURIComponent(driverId)}/revisions/${encodeURIComponent(
        revisionId
      )}/definition`,
      {
        expected_definition_checksum: expectedChecksum,
        definition,
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId("driver-profile-save", `${driverId}-${revisionId}`)
      }
    ),
  submitDriver: (driverId: string, revisionId: string, context: OperationContext) =>
    post<DriverProfileOperationResult>(
      `/api/v1/driver-profiles/${encodeURIComponent(driverId)}/revisions/${encodeURIComponent(
        revisionId
      )}/transitions/submit-for-review`,
      {
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId("driver-profile-submit", `${driverId}-${revisionId}`)
      }
    ),
  approveDriver: (driverId: string, revisionId: string, context: OperationContext) =>
    post<DriverProfileOperationResult>(
      `/api/v1/driver-profiles/${encodeURIComponent(driverId)}/revisions/${encodeURIComponent(
        revisionId
      )}/transitions/approve`,
      {
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId("driver-profile-approve", `${driverId}-${revisionId}`)
      }
    ),
  deriveDriverRevision: (driverId: string, sourceRevisionId: string, context: OperationContext) =>
    post<DriverProfileOperationResult>(
      `/api/v1/driver-profiles/${encodeURIComponent(driverId)}/revisions`,
      {
        source_revision_id: sourceRevisionId,
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId("driver-profile-derive", `${driverId}-${sourceRevisionId}`)
      }
    ),
  simulateDriver: (
    driver_profile_id: string,
    action_id: string,
    scenario: DriverSimulationScenario,
    revision_id?: string
  ) =>
    post<{ simulation: DriverSimulationResult }>("/api/v1/driver-profile-simulations", {
      driver_profile_id,
      revision_id,
      action_id,
      scenario
    }),
  providers: () =>
    request<{ providers: CommunicationProviderStatus[] }>(
      "/api/v1/equipment/communication-providers"
    )
};

export const metrologyApi = {
  listInstruments: () =>
    request<{ instruments: MetrologyInstrument[] }>("/api/v1/metrology/instruments"),
  registerInstrument: (input: RegisterMetrologyInstrumentInput) =>
    post<{ instrument: MetrologyInstrument }>("/api/v1/metrology/instruments", {
      ...input,
      operation_id: operationId("metrology-register", input.asset_id)
    }),
  listCharacterizations: (assetId: string) =>
    request<{ asset_id: string; characterizations: AssetCharacterization[] }>(
      `/api/v1/metrology/instruments/${encodeURIComponent(assetId)}/characterizations`
    ),
  getCharacterization: (assetId: string, characterizationId: string) =>
    request<{ characterization: AssetCharacterization }>(
      `/api/v1/metrology/instruments/${encodeURIComponent(assetId)}/characterizations/${encodeURIComponent(characterizationId)}`
    ),
  recordCharacterization: (assetId: string, input: RecordAssetCharacterizationInput) =>
    post<{ characterization: AssetCharacterization }>(
      `/api/v1/metrology/instruments/${encodeURIComponent(assetId)}/characterizations`,
      {
        ...input,
        operation_id: operationId("asset-characterization", input.characterization_id)
      }
    ),
  characterizationAudit: (assetId: string, characterizationId: string) =>
    request<{ audit_events: MetrologyAuditEvent[] }>(
      `/api/v1/metrology/instruments/${encodeURIComponent(assetId)}/characterizations/${encodeURIComponent(characterizationId)}/audit-events`
    ),
  listCorrections: (assetId: string) =>
    request<{ assignments: AssetCorrectionAssignmentEnvelope[] }>(
      `/api/v1/metrology/instruments/${encodeURIComponent(assetId)}/corrections`
    ),
  correctionReviewQueue: () =>
    request<{ assignments: AssetCorrectionAssignmentEnvelope[] }>(
      "/api/v1/metrology/corrections/review-queue"
    ),
  createCorrection: (
    assetId: string,
    input: {
      assignment_id: string;
      signal_path_id: string;
      requirement_id: string;
      source_event_id: string;
      valid_from?: string;
      valid_until?: string;
      conditions?: Record<string, string>;
      actor: string;
      reason: string;
    }
  ) =>
    post<AssetCorrectionAssignmentEnvelope>(
      `/api/v1/metrology/instruments/${encodeURIComponent(assetId)}/corrections`,
      {
        ...input,
        operation_id: operationId("asset-correction-create", input.assignment_id)
      }
    ),
  transitionCorrection: (
    assetId: string,
    assignmentId: string,
    transition: "submit-for-review" | "approve-and-activate" | "reject" | "request-changes",
    expectedRevision: string,
    actor: string,
    reason: string
  ) =>
    post<AssetCorrectionAssignmentEnvelope>(
      `/api/v1/metrology/instruments/${encodeURIComponent(assetId)}/corrections/${encodeURIComponent(assignmentId)}/transitions/${transition}`,
      {
        expected_revision: expectedRevision,
        actor,
        reason,
        operation_id: operationId("asset-correction-transition", `${assignmentId}-${transition}`)
      }
    ),
  resolveCorrections: (
    assetId: string,
    intendedUseOn: string,
    executionContext: "accredited" | "non_accredited" | "investigation" | "simulation",
    conditions: Record<string, string> = {}
  ) =>
    post<{
      asset_id: string;
      equipment_model_id: string;
      equipment_model_revision_id: string;
      equipment_model_checksum: string;
      report: AssetCorrectionResolutionReport;
    }>(`/api/v1/metrology/instruments/${encodeURIComponent(assetId)}/corrections/resolve`, {
      intended_use_on: intendedUseOn,
      execution_context: executionContext,
      conditions
    }),
  uploadFile: async (file: File) =>
    post<{ file: EquipmentFileReference }>("/api/v1/metrology/files", {
      original_filename: file.name,
      mime_type: file.type || "application/octet-stream",
      content_base64: await fileToBase64(file)
    })
};

export const projectApi = {
  listLaboratoryLocations: async (): Promise<LaboratoryLocationOption[]> => {
    const response = await request<{ station_setups: StationSetupLocationSource[] }>(
      "/api/v1/station-setups"
    );
    const locations = new Map<string, string>();
    for (const aggregate of response.station_setups) {
      const definition = aggregate.current_ready_revision?.definition;
      const locationId = definition?.laboratory_location_id?.trim();
      const locationLabel = definition?.laboratory_location_label?.trim();
      if (locationId && locationLabel) locations.set(locationId, locationLabel);
    }
    return Array.from(locations, ([laboratory_location_id, laboratory_location_label]) => ({
      laboratory_location_id,
      laboratory_location_label
    })).sort((left, right) =>
      left.laboratory_location_label.localeCompare(right.laboratory_location_label, "fr")
    );
  },
  listProjects: () => request<{ projects: ProjectRecord[] }>("/api/v1/projects"),
  getProject: (projectCode: string) =>
    request<{ project: ProjectRecord }>(`/api/v1/projects/${encodeURIComponent(projectCode)}`),
  createProject: (input: {
    code: string;
    customer_name: string;
    execution_mode: ProjectExecutionMode;
    actor: string;
    reason: string;
  }) =>
    post<ProjectOperationResult>("/api/v1/projects", {
      ...input,
      operation_id: operationId("project-create", input.code)
    }),
  contractReview: (projectCode: string) =>
    request<{ contract_review: ContractReviewStatus }>(
      `/api/v1/projects/${encodeURIComponent(projectCode)}/contract-review`
    ),
  completeReviewItem: (
    projectCode: string,
    item: string,
    actor: string,
    comment?: string
  ) =>
    post<ContractReviewOperationResult>(
      `/api/v1/projects/${encodeURIComponent(projectCode)}/contract-review/items/${encodeURIComponent(item)}/complete`,
      {
        actor,
        comment,
        operation_id: operationId("contract-review-complete", `${projectCode}-${item}`)
      }
    ),
  advanceToPlanning: (projectCode: string, actor: string, reason: string) =>
    post<ProjectOperationResult>(
      `/api/v1/projects/${encodeURIComponent(projectCode)}/transitions/to-test-planning`,
      {
        actor,
        reason,
        operation_id: operationId("project-to-planning", projectCode)
      }
    ),
  listSchedule: (projectCode: string) =>
    request<{ project_code: string; schedule_items: ServiceScheduleItem[] }>(
      `/api/v1/projects/${encodeURIComponent(projectCode)}/schedule-items`
    ),
  getPlannedTestPreparation: (projectCode: string, itemCode: string) =>
    request<{ preparation: PlannedTestPreparationAggregate }>(
      `/api/v1/projects/${encodeURIComponent(projectCode)}/schedule-items/${encodeURIComponent(itemCode)}/preparation`
    ),
  plannedTestPreparationOptions: (projectCode: string, itemCode: string) =>
    request<PlannedTestPreparationOptions>(
      `/api/v1/projects/${encodeURIComponent(projectCode)}/schedule-items/${encodeURIComponent(itemCode)}/preparation/options`
    ),
  plannedTestPreparationRevisions: (projectCode: string, itemCode: string) =>
    request<{
      project_code: string;
      schedule_item_code: string;
      revisions: PlannedTestPreparationRevision[];
    }>(
      `/api/v1/projects/${encodeURIComponent(projectCode)}/schedule-items/${encodeURIComponent(itemCode)}/preparation/revisions`
    ),
  getPlannedTestPreparationRevision: (
    projectCode: string,
    itemCode: string,
    revisionId: string
  ) =>
    request<{ revision: PlannedTestPreparationRevision }>(
      `/api/v1/projects/${encodeURIComponent(projectCode)}/schedule-items/${encodeURIComponent(itemCode)}/preparation/revisions/${encodeURIComponent(revisionId)}`
    ),
  assessPlannedTestPreparation: (
    item: ServiceScheduleItem,
    input: {
      expected_current_revision_id: string | null;
      method_template_id: string;
      method_revision_id: string;
      station_setup_id: string;
      station_setup_revision_id: string;
      assignments: Array<{ slot_id: string; binding_id: string }>;
      actor: string;
      reason: string;
    }
  ) =>
    post<PlannedTestPreparationOperationResult>(
      `/api/v1/projects/${encodeURIComponent(item.project_code)}/schedule-items/${encodeURIComponent(item.item_code)}/preparation/assessments`,
      {
        ...input,
        expected_schedule_revision: item.revision,
        operation_id: operationId(
          "planned-test-preparation",
          `${item.project_code}-${item.item_code}-${item.revision}-${input.expected_current_revision_id ?? "initial"}`
        )
      }
    ),
  listLaboratoryWeek: (weekStart: string) =>
    request<LaboratoryWeekSchedule>(
      `/api/v1/service-schedule?week_start=${encodeURIComponent(weekStart)}`
    ),
  createScheduleItem: (
    projectCode: string,
    input: {
      item_code: string;
      title: string;
      planned_start_at: string;
      planned_end_at: string;
      assigned_operator: string;
      laboratory_location_id: string;
      laboratory_location_label: string;
      equipment_under_test: string;
      notes?: string;
      actor: string;
      reason: string;
    }
  ) =>
    post<ServiceScheduleOperationResult>(
      `/api/v1/projects/${encodeURIComponent(projectCode)}/schedule-items`,
      {
        ...input,
        operation_id: operationId("service-schedule-create", input.item_code)
      }
    ),
  transitionScheduleItem: (
    projectCode: string,
    item: ServiceScheduleItem,
    action: "confirm" | "start" | "complete" | "cancel",
    actor: string,
    reason: string,
    preparation?: { revision_id: string; definition_checksum: string }
  ) =>
    post<ServiceScheduleOperationResult>(
      `/api/v1/projects/${encodeURIComponent(projectCode)}/schedule-items/${encodeURIComponent(item.item_code)}/transitions/${action}`,
      {
        expected_revision: item.revision,
        expected_preparation_revision_id: preparation?.revision_id,
        expected_preparation_checksum: preparation?.definition_checksum,
        actor,
        reason,
        operation_id: operationId(
          `service-schedule-${action}`,
          `${projectCode}-${item.item_code}-${item.revision}`
        )
      }
    ),
  rescheduleItem: (
    item: ServiceScheduleItem,
    input: {
      planned_start_at: string;
      planned_end_at: string;
      assigned_operator: string;
      laboratory_location_id: string;
      laboratory_location_label: string;
      actor: string;
      reason: string;
    }
  ) =>
    post<ServiceScheduleOperationResult>(
      `/api/v1/projects/${encodeURIComponent(item.project_code)}/schedule-items/${encodeURIComponent(item.item_code)}/reschedule`,
      {
        ...input,
        expected_revision: item.revision,
        operation_id: operationId(
          "service-schedule-reschedule",
          `${item.project_code}-${item.item_code}-${item.revision}`
        )
      }
    ),
  auditEvents: (projectCode: string) =>
    request<{ project_code: string; audit_events: ProjectAuditEvent[] }>(
      `/api/v1/projects/${encodeURIComponent(projectCode)}/audit-events`
    )
};

async function fileToBase64(file: File): Promise<string> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const chunks: string[] = [];
  const chunkSize = 0x8000;
  for (let offset = 0; offset < bytes.length; offset += chunkSize) {
    chunks.push(String.fromCharCode(...bytes.subarray(offset, offset + chunkSize)));
  }
  return btoa(chunks.join(""));
}

export interface MeasurementEngineeringConfig {
  collection: MeasurementEngineeringCollection;
  validationCollection: string;
  operationPrefix: string;
}

export const measurementEngineeringApi = {
  list: (config: MeasurementEngineeringConfig) =>
    request<{
      aggregate_kind: string;
      collection_key: string;
      items: MeasurementEngineeringAggregate[];
    }>(`/api/v1/${config.collection}`),
  get: (config: MeasurementEngineeringConfig, entityId: string) =>
    request<{
      aggregate_kind: string;
      item: MeasurementEngineeringAggregate;
    }>(`/api/v1/${config.collection}/${encodeURIComponent(entityId)}`),
  listRevisions: (config: MeasurementEngineeringConfig, entityId: string) =>
    request<{
      aggregate_kind: string;
      entity_id: string;
      revisions: MeasurementEngineeringRevision[];
    }>(`/api/v1/${config.collection}/${encodeURIComponent(entityId)}/revisions`),
  listAudit: (config: MeasurementEngineeringConfig, entityId: string) =>
    request<{ aggregate_kind: string; entity_id: string; audit_events: EquipmentAuditEvent[] }>(
      `/api/v1/${config.collection}/${encodeURIComponent(entityId)}/audit-events`
    ),
  validateDefinition: (
    config: MeasurementEngineeringConfig,
    definition: MeasurementEngineeringDefinition
  ) =>
    post<EquipmentValidationResult>(`/api/v1/${config.validationCollection}/validate`, {
      definition
    }),
  create: (
    config: MeasurementEngineeringConfig,
    input: {
      entity_id: string;
      definition: MeasurementEngineeringDefinition;
    } & OperationContext
  ) =>
    post<MeasurementEngineeringOperationResult>(`/api/v1/${config.collection}`, {
      ...input,
      operation_id: operationId(`${config.operationPrefix}-create`, input.entity_id)
    }),
  saveDraft: (
    config: MeasurementEngineeringConfig,
    entityId: string,
    revisionId: string,
    expectedChecksum: string,
    definition: MeasurementEngineeringDefinition,
    context: OperationContext
  ) =>
    put<MeasurementEngineeringOperationResult>(
      `/api/v1/${config.collection}/${encodeURIComponent(entityId)}/revisions/${encodeURIComponent(
        revisionId
      )}/definition`,
      {
        expected_definition_checksum: expectedChecksum,
        definition,
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId(`${config.operationPrefix}-save`, `${entityId}-${revisionId}`)
      }
    ),
  submit: (
    config: MeasurementEngineeringConfig,
    entityId: string,
    revisionId: string,
    context: OperationContext
  ) =>
    post<MeasurementEngineeringOperationResult>(
      `/api/v1/${config.collection}/${encodeURIComponent(entityId)}/revisions/${encodeURIComponent(
        revisionId
      )}/transitions/submit-for-review`,
      {
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId(`${config.operationPrefix}-submit`, `${entityId}-${revisionId}`)
      }
    ),
  approve: (
    config: MeasurementEngineeringConfig,
    entityId: string,
    revisionId: string,
    context: OperationContext
  ) =>
    post<MeasurementEngineeringOperationResult>(
      `/api/v1/${config.collection}/${encodeURIComponent(entityId)}/revisions/${encodeURIComponent(
        revisionId
      )}/transitions/approve`,
      {
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId(`${config.operationPrefix}-approve`, `${entityId}-${revisionId}`)
      }
    ),
  deriveRevision: (
    config: MeasurementEngineeringConfig,
    entityId: string,
    sourceRevisionId: string,
    context: OperationContext
  ) =>
    post<MeasurementEngineeringOperationResult>(
      `/api/v1/${config.collection}/${encodeURIComponent(entityId)}/revisions`,
      {
        source_revision_id: sourceRevisionId,
        actor: context.actor,
        reason: context.reason,
        operation_id: operationId(`${config.operationPrefix}-derive`, `${entityId}-${sourceRevisionId}`)
      }
    ),
  evaluateCurve: (curveId: string, revisionId: string, axisValues: Record<string, number>) =>
    post<{ evaluation: EngineeringCurveEvaluation }>(
      `/api/v1/engineering-curves/${encodeURIComponent(curveId)}/revisions/${encodeURIComponent(
        revisionId
      )}/evaluate`,
      { axis_values: axisValues }
    )
};

export { operationAggregate };

export function operationId(prefix: string, key: string): string {
  const normalized = key.replace(/[^a-zA-Z0-9_.:-]+/g, "-").slice(0, 48);
  return `${prefix}-${normalized}-${Date.now().toString(36)}`;
}
