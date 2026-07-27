# Migrated Asset Model Reconciliation

## Purpose

Release 0.22.0 imports physical identities from the read-only 0.21.1
metrology register into `equipment.sqlite/physical_assets`. When a legacy row
does not contain a complete trusted model pin, the imported asset remains
visible with:

```text
model_link_state = migration_review_required
```

This state is deliberately non-executable. It is not a guessed model link and
does not authorize the asset for a station setup or test preparation.

## Explicit Command

LAB CONSOLE exposes **Rapprocher avec un modèle constructeur**. The operator
selects a readable immutable version showing manufacturer, model, variant,
complete category path, revision number, lifecycle status and approval date.
Internal model IDs, revision IDs and checksums are not entered manually.

The corresponding agent command requires:

- the physical asset ID;
- its expected numeric revision for optimistic concurrency;
- the exact model identity and revision selected by the UI;
- actor, reason, operation ID, device ID and correlation ID.

Only `approved` and `superseded` model revisions are eligible. Both are
immutable. A superseded revision remains selectable because it may be the
exact historical definition of an older physical item.

## Server Validation

Inside one immediate SQLite write transaction, the agent:

1. verifies operation replay semantics;
2. compares the expected asset revision;
3. requires `migration_review_required`;
4. loads the exact model revision;
5. parses and validates the typed `EquipmentModelDefinition`;
6. canonicalizes the definition and checks its stored SHA-256 checksum;
7. derives manufacturer, model, variant, category code and category path;
8. changes the link state to `resolved` and increments the asset revision;
9. writes audit, operation and sync-outbox evidence;
10. commits only when all writes succeed.

No successful command automatically selects the latest revision. A resolved
asset cannot be repointed through this command. A checksum mismatch,
non-immutable revision, concurrency conflict or outbox failure leaves the
asset and all command evidence unchanged.

## Preserved Evidence

`migration_evidence_json` is not rewritten. It continues to identify the
legacy source, original identifiers and original availability/model fields.
The reconciliation audit event records the newly selected exact pin without
erasing that source evidence.

Safe identification, notes, service-state and active-location changes may be
recorded while the link is unresolved. They do not alter the model fields and
do not make the asset executable. Executable selector eligibility remains a
separate backend decision.
