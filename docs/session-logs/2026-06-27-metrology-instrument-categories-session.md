# 2026-06-27 Metrology Instrument Categories Session

## Intent

Make metrology useful before expanding the rest of the application by adding a
revisioned instrument category taxonomy for electronics, EMC, thermal,
acoustic, shock/vibration, radio/RF, and data-monitoring equipment.

## Changes

- Added metrology migration v2 with 34 seeded instrument categories.
- Added category source provenance records and taxonomy metadata.
- Added nullable `instruments.category_code` so existing metrology v1 databases
  migrate without losing legacy assets.
- Added Python metrology repository APIs for category count, category listing,
  domain filtering, source listing, and instrument lookup by category/domain.
- Exposed category rows through GUI bootstrap payloads.
- Added a Qt console `Categories` table and category-count status metric.
- Added the category table to the static browser prototype bootstrap.
- Documented the first taxonomy and its public source set.
- Updated domain model, storage migration notes, roadmap, and changelog.

## Validation Evidence

- SQLite migration validation: passed with measurement_data=7, metrology=2,
  projects=1, sync=1, test_definitions=1, update_catalog=2.
- Direct in-memory metrology check: 34 categories, 7 domains, metadata category
  count 34.
- Targeted metrology and Qt model tests: 3 tests passed.
- Python unittest discovery: 23 tests passed.
- `python -m compileall -q python\emc_locus python\tests`: passed.
- `python -m py_compile python\emc_locus\sqlite_repositories.py
  python\emc_locus\gui_bootstrap.py python\emc_locus\qt_console_models.py
  apps\qt-console\main.py`: passed with bundled Python.
- `node --check apps\gui-shell\app.js`: passed with bundled Node.js.
- `cargo fmt --check`: passed with `C:\Users\gtrai\.cargo\bin\cargo.exe`.
- `cargo test -q`: 139 Rust tests passed.
- `git diff --check`: passed.

## Next Work

- Add CRUD/service actions for creating and updating instruments from the Qt
  console.
- Add category-specific calibration profiles and uncertainty templates.
- Connect category capabilities to instrument-driver selection and safety
  rules.
