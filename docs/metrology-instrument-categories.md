# Metrology Instrument Categories

This document records the first EMC Locus metrology taxonomy. It is intentionally
broader than classic BAT-EMC level-versus-frequency equipment because EMC
campaigns also involve time-domain acquisition, DAQ synchronization, thermal
state, acoustic evidence, shock/vibration monitoring, and radio/RF checks.

The canonical seed data lives in:

```text
storage/sqlite/metrology/0002_instrument_categories.sql
```

Taxonomy revision: `2026-06-27-v1`.

## Domains

### Electronics

Bench and system instruments used to stimulate or observe electrical behavior:
oscilloscopes, digital multimeters, LCR/impedance meters, DC power supplies,
electronic loads, source-measure units, and function/arbitrary waveform
generators.

These categories cover voltage, current, impedance, timing, transient, and
source/load behavior. They are usually driven through VISA/SCPI over USB,
Ethernet, GPIB, or serial links, with manual entries still allowed for simple
bench equipment.

### EMC

Emission and immunity chain equipment: EMI receivers, LISN/AMN networks,
antennas and probes, RF power amplifiers, coupling/decoupling networks, and ESD
generators.

This domain must support accredited pre-run readiness checks because wrong
calibration, wrong network factor, or an out-of-service amplifier can invalidate
an entire campaign.

### Thermal

Thermal cameras, temperature/humidity loggers, climatic chambers, and
temperature calibrators. These instruments are needed when environmental state
or component temperature is part of the test evidence.

### Acoustic

Sound level meters, measurement microphones, acoustic calibrators, and noise
dosimeters. These categories support acoustic evidence, monitoring, and
calibrator-before/after workflows.

### Shock And Vibration

Accelerometers, vibration controllers, impact hammers, and shock/vibration
loggers. These categories make EMC Locus ready for campaigns where mechanical
state, transport shock, shaker excitation, or vibration monitoring must be
correlated with electrical behavior.

### Radio And RF

Spectrum/signal analyzers, RF signal generators, vector network analyzers, RF
power meters, and radio communication testers. These categories cover RF
source/measurement chains, antenna and cable verification, amplifier level
traceability, and radio-oriented checks.

### Data Monitoring

DAQ chassis/modules, data loggers, condition-monitoring units, and
timing/synchronization units. This domain is central for future openDAQ-style
time-series acquisition, multi-DAQ synchronization, triggered capture, and
offline field work.

## Storage Behavior

The migration adds:

- `instrument_categories`, seeded with 34 active categories;
- `instrument_category_sources`, keeping public-source provenance;
- nullable `instruments.category_code`, so existing v1 metrology databases
  migrate without losing legacy instruments;
- repository metadata keys for taxonomy revision and category count.

`MetrologyRepository` exposes category listing, domain filtering, source
listing, and instrument lookup by category/domain. The Qt console now receives a
dedicated `instrument_categories` bootstrap table.

## Operator Registration Action

The local action layer can create a metrology asset, link it to a controlled
category, optionally record the first calibration certificate, and regenerate
the browser/Qt bootstrap payload:

```text
$env:PYTHONPATH='python'
python -m emc_locus.actions_cli register-instrument `
  --metrology-db local/metrology.sqlite `
  --asset-id RX-001 `
  --family Receiver `
  --manufacturer "Rohde Schwarz" `
  --model ESW `
  --serial-number 100001 `
  --part-number ESW44 `
  --category-code emi_receiver `
  --calibration-period-months 12 `
  --certificate-reference CERT-RX-001 `
  --calibrated-at 2026-06-01 `
  --provider "Accredited Lab" `
  --bootstrap-output apps/gui-shell/bootstrap.js
```

If `--calibration-requirement` is omitted, the action takes the default from the
selected category. If one certificate field is supplied, the certificate
reference, calibration date, and provider are required. If `--due-at` is not
supplied, EMC Locus computes it from `--calibrated-at` and
`--calibration-period-months`. The instrument and certificate are written in a
single SQLite transaction.

A later certificate can be added without recreating the instrument:

```text
$env:PYTHONPATH='python'
python -m emc_locus.actions_cli record-calibration `
  --metrology-db local/metrology.sqlite `
  --asset-id RX-001 `
  --certificate-reference CERT-RX-002 `
  --calibrated-at 2027-05-20 `
  --due-at 2028-05-20 `
  --provider "Accredited Lab" `
  --uncertainty-json '{\"level_db\":0.5}' `
  --bootstrap-output apps/gui-shell/bootstrap.js
```

The GUI bootstrap displays the latest calibration record for each instrument,
while older calibration rows remain in the metrology database for history and
audit review.

The operational status can be changed without modifying calibration history:

```text
$env:PYTHONPATH='python'
python -m emc_locus.actions_cli set-instrument-availability `
  --metrology-db local/metrology.sqlite `
  --asset-id RX-001 `
  --availability out_of_service `
  --bootstrap-output apps/gui-shell/bootstrap.js
```

Allowed status values are `available`, `reserved`, and `out_of_service`.
Out-of-service instruments remain blocking in readiness checks.

Instrument capabilities can be updated as controlled JSON. Typical content is
category-specific: RF range, channel count, supported transports, sensor type,
maximum voltage/current, or synchronization support.

```text
$env:PYTHONPATH='python'
python -m emc_locus.actions_cli set-instrument-capabilities `
  --metrology-db local/metrology.sqlite `
  --asset-id DAQ-001 `
  --capabilities-json '{\"channels\":8,\"transports\":[\"opendaq\",\"ethernet\"]}' `
  --bootstrap-output apps/gui-shell/bootstrap.js
```

Documents can be attached to the asset lifecycle. Supported document kinds are:
`certificate`, `datasheet`, `transducer_calculation`, `script`, `manual`,
`photo`, and `other`.

```text
$env:PYTHONPATH='python'
python -m emc_locus.actions_cli attach-instrument-document `
  --metrology-db local/metrology.sqlite `
  --asset-id RX-001 `
  --document-kind datasheet `
  --title "ESW datasheet" `
  --file-reference metrology/RX-001/datasheet.pdf `
  --uploaded-by metrology.admin `
  --applies-to-function "receiver correction"
```

## Public Sources Used

- Keysight public electronic test equipment guide:
  https://www.keysight.com/used/cz/en/knowledge/guides/top-10-electronic-test-equipment-for-engineers
- Rohde and Schwarz EMC test equipment overview:
  https://www.rohde-schwarz.com/us/products/test-and-measurement/emc-test-equipment_105350.html
- NI data acquisition overview:
  https://www.ni.com/en/shop/data-acquisition.html
- NTi Audio sound level meter overview:
  https://www.nti-audio.com/en/support/know-how/what-is-a-sound-level-meter
- Larson Davis sound and vibration product scope:
  https://www.larsondavis.com/
- PCB Piezotronics accelerometer reference:
  https://www.pcb.com/sensors-for-test-measurement/accelerometers
- Dewesoft vibration analysis and testing reference:
  https://dewesoft.com/applications/vibration-analysis
- Fluke thermal camera reference:
  https://www.fluke.com/en-us/products/thermal-cameras

The source list is not a vendor selection. It is a starting reference set used
to avoid modeling the metrology database around only one CEM software family.
