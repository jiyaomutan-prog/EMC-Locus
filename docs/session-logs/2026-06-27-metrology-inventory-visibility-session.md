# 2026-06-27 Metrology Inventory Visibility Session

## Intent

Make the newly maintained metrology data visible to operators by surfacing
instrument category and capability summaries in the inventory views.

## Changes

- Extended GUI bootstrap instrument rows with category label and capability
  preview columns while keeping the existing alert tone at index 5.
- Added category lookup and compact capability preview generation in the
  bootstrap adapter.
- Extended the Qt metrology table to show `Categorie` and `Capacites`.
- Extended the browser prototype metrology table with the same columns.
- Updated static fixture data and `apps/gui-shell/bootstrap.js`.
- Added tests covering the expanded Qt table contract and SQLite-to-bootstrap
  category/capability mapping.
- Updated changelog.

## Validation Evidence

- Targeted Qt/bootstrap tests: 2 tests passed.
- SQLite migration validation: passed with measurement_data=7, metrology=2,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- Python unittest discovery: 31 tests passed.
- `python -m compileall -q python\emc_locus python\tests`: passed.
- `python -m py_compile python\emc_locus\sqlite_repositories.py
  python\emc_locus\gui_actions.py python\emc_locus\actions_cli.py
  python\emc_locus\__init__.py python\emc_locus\gui_bootstrap.py
  python\emc_locus\qt_console_models.py apps\qt-console\main.py`: passed.
- `node --check apps\gui-shell\app.js`: passed with bundled Node.js.
- `cargo fmt --check`: passed with `C:\Users\gtrai\.cargo\bin\cargo.exe`.
- `cargo test -q`: 139 Rust tests passed.
- `git diff --check`: passed.

## Next Work

- Add a Qt-facing metrology action panel that calls the local Python action
  layer rather than only displaying refreshed bootstrap data.
- Add category-specific capability templates.
- Link capabilities and categories to method-equipment compatibility checks.
