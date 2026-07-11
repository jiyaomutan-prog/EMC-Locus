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
  EquipmentClassificationPreset,
  EquipmentRegistries,
  EquipmentModelAggregate,
  EquipmentModelDefinition,
  EquipmentModelRevision,
  EquipmentOperationResult,
  EquipmentValidationResult
} from "./models/equipment";

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
    } & OperationContext
  ) =>
    post<EquipmentModelOperationResult>("/api/v1/equipment-models/from-preset", {
      ...input,
      operation_id: operationId("equipment-model-from-preset", `${input.preset_id}-${input.equipment_model_id}`)
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

export function operationId(prefix: string, key: string): string {
  const normalized = key.replace(/[^a-zA-Z0-9_.:-]+/g, "-").slice(0, 48);
  return `${prefix}-${normalized}-${Date.now().toString(36)}`;
}
