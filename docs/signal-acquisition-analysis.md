# Signal Acquisition and Analysis

EMC Locus must support CEM campaigns that are not naturally represented as
level-versus-frequency sweeps. Many useful CEM measurements start as synchronized
time-domain signals and require signal processing before the final result can be
reviewed.

## Scope

This architecture covers:

- time-series acquisition;
- event-triggered acquisition;
- mixed time/frequency workflows;
- FFT and temporal processing;
- mathematical operations between channels;
- synchronized multi-DAQ tests;
- raw-data lineage for processed results.

## Example CEM Test Families

### Railway Harmonics

Railway-oriented measurements may require long time captures, harmonic
extraction, RMS or peak calculations, and comparison against method-specific
limits.

### Axle-Counter Measurements

Axle-counter related measurements can require precise edge timing, event
counting, threshold crossings, pulse spacing, and correlation between analog and
digital channels.

### Inrush Measurements

Inrush current measurements require trigger strategy, high sample-rate capture,
peak/RMS extraction, envelope or decay analysis, and repeatability tracking.

### Transient and Pulsed Disturbance Capture

Transient capture needs pre-trigger data, post-trigger data, timestamp quality,
and traceability to the acquisition setup.

## DAQ Integration Strategy

openDAQ is the preferred generic DAQ integration path because it is public,
open-source, cross-platform, and designed for interoperable DAQ devices.

EMC Locus should still support:

- vendor SDK bridges;
- USB DAQs;
- Ethernet DAQs;
- EtherCAT DAQs;
- PCIe DAQs;
- sound-card style inputs for low-risk exploratory work;
- VISA-connected digitizers;
- file replay;
- simulated DAQ sources.

## Synchronization Strategy

The synchronization method is part of measurement evidence. EMC Locus should
record which method was used and expose whether the method is suitable for the
selected test.

Baseline methods:

- shared sample clock;
- external trigger;
- start trigger;
- PTP/IEEE 1588;
- GPS/GNSS;
- IRIG-B;
- EtherCAT distributed clocks;
- hardware timestamps;
- software timestamps;
- cross-correlation post-alignment.

## Signal Processing Graph

Signal processing should be represented as a graph:

```text
Raw signal(s)
  -> synchronization/alignment
  -> conditioning/filtering
  -> FFT or temporal analysis
  -> channel math or event extraction
  -> result dataset
  -> report table/plot
```

Every node should preserve:

- input signal references;
- processing parameters;
- software version;
- operator or procedure source;
- output checksum or dataset identifier.

## Baseline Operations

EMC Locus should support:

- FFT;
- windowed FFT;
- inverse FFT;
- time-domain filters;
- channel arithmetic;
- mathematical expressions;
- harmonic analysis;
- inrush analysis;
- event counting;
- edge timing;
- resampling;
- cross-correlation;
- RMS;
- peak;
- envelope.

## Data Retention

Processed results are not enough. The system must retain:

- raw samples or a controlled raw-data export;
- acquisition metadata;
- synchronization metadata;
- processing graph definition;
- generated result datasets;
- report linkage.

## First Implementation Slice

Implemented in the Rust core:

1. simulated openDAQ-style DAQ source;
2. deterministic inrush time-series fixture;
3. synchronized signal dataset with channel metadata;
4. processing graph nodes for FFT and channel arithmetic vocabulary;
5. raw lineage lookup from derived outputs back to acquired inputs;
6. deterministic channel-sum execution;
7. temporal peak extraction;
8. deterministic DFT magnitude fixture for FFT-oriented workflows;
9. Hann and rectangular windowing;
10. deterministic downsampling.

Not yet implemented:

- optimized FFT execution;
- richer window families;
- interpolation-based resampling;
- persistence of graph instances;
- real DAQ adapters.
