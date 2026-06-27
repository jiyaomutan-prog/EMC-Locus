# EMC Locus GUI Shell

This is the first static operator-facing shell for EMC Locus.

Open `index.html` directly in a browser.

The console reads `bootstrap.js` when present and falls back to embedded fixture
data. Regenerate the local bootstrap file with:

```text
$env:PYTHONPATH='python'; py -m emc_locus.gui_actions refresh-bootstrap --output apps\gui-shell\bootstrap.js
```

Pass `--projects-db`, `--metrology-db`, `--test-definitions-db`,
`--measurement-data-db`, or `--update-catalog-db` to export data from local
SQLite repositories.

Advance a project locally and refresh the console data with:

```text
$env:PYTHONPATH='python'; py -m emc_locus.gui_actions advance-project --projects-db data\projects.sqlite --code CEM-2026-001 --actor operator.one --reason "Contract review ready" --bootstrap-output apps\gui-shell\bootstrap.js
```

Record a dataset retention action and refresh the console data with:

```text
$env:PYTHONPATH='python'; py -m emc_locus.gui_actions dataset-retention --measurement-data-db data\measurement_data.sqlite --dataset-id 1 --action request-deletion --actor data.manager --reason "Retention period expired" --bootstrap-output apps\gui-shell\bootstrap.js
```

Validate and record an update install with:

```text
$env:PYTHONPATH='python'; py -m emc_locus.gui_actions validate-update --update-catalog-db data\update_catalog.sqlite --package-name driver-pack-visa --package-version 0.2.0 --component instrument_driver --installed-version 0.1.0 --source offline_bundle --compatibility-minimum-version 0.1.0 --compatibility-maximum-version 0.1.9 --validated-by qa.lead --bootstrap-output apps\gui-shell\bootstrap.js
$env:PYTHONPATH='python'; py -m emc_locus.gui_actions install-update --update-catalog-db data\update_catalog.sqlite --package-name driver-pack-visa --package-version 0.2.0 --component instrument_driver --installed-by qa.lead --source offline_bundle --rollback-reference driver-pack-visa-0.1.0 --validation-evidence-id 1 --bootstrap-output apps\gui-shell\bootstrap.js
```
