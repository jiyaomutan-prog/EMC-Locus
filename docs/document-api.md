# Attached Document API

The attached-document API is the first shared document registry behind the
Locus Local Agent. It is intentionally metadata-only: it does not upload,
download, parse, or store file bytes.

Files and scientific payloads are referenced by storage URI and content
checksum. The current local slice stores metadata in `projects.sqlite` and
emits outbox operations under domain `project_records`, entity type
`attached_document`. A later slice may promote documents to a dedicated local
repository when central PostgreSQL and object storage contracts are introduced.

## Routes

```text
POST /api/v1/documents
GET  /api/v1/documents
GET  /api/v1/documents?owner_domain=locus_lab_management&owner_entity_type=project&owner_entity_id=CEM-2026-001
GET  /api/v1/documents/{document_id}
GET  /api/v1/documents/{document_id}/audit-events
```

## Register Document

```json
{
  "document_id": "DOC-CEM-2026-001-REQ-A",
  "classification": "client_document",
  "title": "Customer EMC requirements",
  "owner_domain": "locus_lab_management",
  "owner_entity_type": "project",
  "owner_entity_id": "CEM-2026-001",
  "storage_backend": "object_store",
  "storage_uri": "objects/projects/CEM-2026-001/requirements-A.pdf",
  "original_filename": "requirements.pdf",
  "mime_type": "application/pdf",
  "size_bytes": 12345,
  "sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
  "revision": "A",
  "applicability": "applicable",
  "confidentiality": "customer_visible",
  "actor": "project.manager",
  "reason": "customer requirement received",
  "operation_id": "op-doc-register"
}
```

Allowed owner domains:

- `locus_metrology`;
- `locus_lab_management`;
- `locus_test_station`;
- `shared`.

Allowed classifications:

- `client_document`;
- `standard_reference`;
- `calibration_certificate`;
- `datasheet`;
- `worksheet`;
- `script`;
- `report`;
- `photo`;
- `drawing`;
- `contract`;
- `dataset_manifest`;
- `other`.

Allowed storage backends:

- `object_store`;
- `local_path`;
- `external_reference`.

Allowed applicability states:

- `draft`;
- `applicable`;
- `superseded`;
- `archival`.

Allowed confidentiality states:

- `internal`;
- `customer_visible`;
- `restricted`.

When the owner is a Locus Lab Management project
(`owner_domain=locus_lab_management`, `owner_entity_type=project`), the project
must already exist.

## Idempotence

`operation_id` is required. Replaying the same canonical operation returns the
stored document with `replayed=true`. Reusing the same `operation_id` for a
different payload returns HTTP `409` with `operation_replay_mismatch`.

## Audit And Outbox

Each successful registration writes:

- one `attached_documents` row;
- one `document_audit_events` row with action
  `attached_document_registered`;
- one pending `sync_operations` row with entity type `attached_document`.

The document registry is shared by Locus Metrology, Locus Lab Management, and
Locus Test Station. The current slice proves common document identity and
traceability before adding real object upload or document permission workflows.
