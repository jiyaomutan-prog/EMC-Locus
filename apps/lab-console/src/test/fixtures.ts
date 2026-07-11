import { defaultTemplateDefinition } from "../defaultDefinition";
import type {
  AuditEvent,
  HealthReport,
  StorageStatus,
  TestTemplateAggregate,
  TestTemplateRevision
} from "../types";

export const healthFixture: HealthReport = {
  agent: "emc-locus-agent",
  version: "0.13.0",
  storage_root: "data\\local-agent",
  storage_root_exists: true,
  domains: ["metrology", "project_records", "test_definitions"]
};

export const storageFixture: StorageStatus = {
  action: "status",
  storage_root: "data\\local-agent",
  migrations_root: "storage\\sqlite",
  domains: [
    {
      domain: "test_definitions",
      database_path: "data\\local-agent\\test_definitions.sqlite",
      exists: true,
      schema_version: 5,
      latest_migration: 5,
      status: "current",
      journal_mode: "delete"
    }
  ]
};

export function revisionFixture(status: TestTemplateRevision["status"] = "draft"): TestTemplateRevision {
  return {
    revision_id: "TT-LAB-001-rev-0001",
    template_id: "TT-LAB-001",
    revision_number: 1,
    parent_revision_id: null,
    status,
    definition_schema_version: "emc-locus.test-template-definition.v1",
    definition: defaultTemplateDefinition("Inrush current template"),
    definition_checksum: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    created_by: "method.author",
    created_at: "2026-07-01T10:00:00Z",
    updated_at: "2026-07-01T10:00:00Z",
    submitted_at: status === "draft" ? null : "2026-07-01T11:00:00Z",
    approved_at: status === "approved" ? "2026-07-01T12:00:00Z" : null
  };
}

export function templateFixture(status: TestTemplateRevision["status"] = "draft"): TestTemplateAggregate {
  const revision = revisionFixture(status);
  return {
    identity: {
      template_id: "TT-LAB-001",
      title: "Inrush current template",
      category_code: "emission_transient_time_domain",
      current_approved_revision_id: status === "approved" ? revision.revision_id : null,
      created_by: "method.author",
      created_at: "2026-07-01T10:00:00Z",
      updated_at: "2026-07-01T10:00:00Z"
    },
    current_approved_revision: status === "approved" ? revision : null,
    latest_revision: revision,
    active_draft_revision: status === "draft" ? revision : null
  };
}

export const auditFixture: AuditEvent[] = [
  {
    audit_id: 1,
    template_id: "TT-LAB-001",
    revision_id: "TT-LAB-001-rev-0001",
    action: "test_template_created",
    actor: "method.author",
    reason: "creation",
    old_revision_id: null,
    new_revision_id: "TT-LAB-001-rev-0001",
    old_definition_checksum: null,
    new_definition_checksum: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    operation_id: "op-create",
    correlation_id: "op-create",
    device_id: "local-agent",
    payload_json: "{}",
    occurred_at: "2026-07-01T10:00:00Z"
  }
];

export function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" }
  });
}
