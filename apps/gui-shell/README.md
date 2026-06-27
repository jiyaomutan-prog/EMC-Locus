# EMC Locus GUI Shell

This is the first static operator-facing shell for EMC Locus.

Open `index.html` directly in a browser.

The console reads `bootstrap.js` when present and falls back to embedded fixture
data. Regenerate the local bootstrap file with:

```text
$env:PYTHONPATH='python'; py -m emc_locus.gui_bootstrap apps\gui-shell\bootstrap.js
```

Pass `--projects-db`, `--metrology-db`, `--test-definitions-db`,
`--measurement-data-db`, or `--update-catalog-db` to export data from local
SQLite repositories.
