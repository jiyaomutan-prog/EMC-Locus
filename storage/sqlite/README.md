# SQLite Migration Domains

EMC Locus starts with SQLite migrations because they are easy to inspect,
portable for field work, and suitable for deterministic tests.

Each subdirectory represents a separate local repository database. This keeps
metrology, project records, test definitions, measurement data, and update
metadata independently exportable and synchronizable.

```text
storage/sqlite/
  metrology/
  projects/
  test_definitions/
  measurement_data/
  update_catalog/
```

Migration filenames use:

```text
NNNN_short_description.sql
```

Each domain owns a `schema_migrations` table. Cross-domain links are stored as
stable text references instead of SQLite foreign keys, because the domains may
be exported, synchronized, or restored independently.
