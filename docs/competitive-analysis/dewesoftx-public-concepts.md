# DewesoftX Public Concepts for EMC Locus

DewesoftX is a strong reference point for time-domain acquisition and signal
processing workflows. EMC Locus should learn from its public concepts without
copying proprietary UI, code, data formats, algorithms, or licensed assets.

## Public Sources Reviewed

- DewesoftX product page:
  https://dewesoft.com/products/dewesoftx
- openDAQ SDK:
  https://opendaq.com/
- openDAQ solutions:
  https://opendaq.com/solutions
- openDAQ GitHub repository:
  https://github.com/openDAQ/openDAQ
- openDAQ architecture documentation:
  https://docs.opendaq.com/manual/opendaq/3.30/explanations/opendaq_architecture.html
- openDAQ streaming documentation:
  https://docs.opendaq.com/manual/opendaq/3.30/explanations/streaming.html

## Publicly Observed DewesoftX Concepts

The public DewesoftX product page presents it as data acquisition and digital
signal-processing software, with broad test-and-measurement coverage. Relevant
concepts for EMC Locus include:

- synchronized recording from multiple data interfaces;
- analog, digital, video, GNSS, vehicle bus, aerospace bus, industrial bus, and
  serial inputs;
- signal processing and math modules, including formulas, statistics, filters,
  FFT, power analysis, acoustics, and other application-specific modules;
- data visualization and reporting;
- automation through sequencing;
- extensibility through API and scripting;
- offline installer availability and frequent product updates.

## Publicly Observed openDAQ Concepts

openDAQ is described publicly as an open-source SDK by Dewesoft and HBK for
discovering, configuring, and collecting data from compatible DAQ devices. It
supports C++, C#, and Python workflows, runs across major desktop operating
systems, and provides generic APIs for DAQ integration.

The public openDAQ documentation and repository describe concepts directly
useful to EMC Locus:

- devices exposing signals;
- subscribers receiving signal data;
- streaming for continuous real-time data reception;
- property/configuration access;
- custom signal-processing blocks;
- OPC UA for structure/properties and WebSocket streaming for data in the
  current SDK architecture;
- support for varied data types and high data rates.

## EMC Locus Product Response

### 1. Frequency Sweeps Are Not Enough

BAT-EMC-like workflows focus heavily on level versus frequency. EMC Locus must
also cover CEM tests where the source data is a time series:

- railway harmonic measurements;
- axle-counter related measurements;
- inrush current measurements;
- transient capture;
- pulsed disturbances;
- power-quality-like CEM investigations;
- custom multi-signal investigations.

### 2. Time-Domain Acquisition Must Be Native

EMC Locus should model time-domain acquisitions as first-class campaigns, not as
attachments to a frequency sweep. A time-domain campaign needs:

- DAQ source definitions;
- channel definitions;
- sampling rate and timebase;
- trigger strategy;
- synchronization strategy;
- raw time-series retention;
- processing graph;
- report traceability back to raw signals.

### 3. openDAQ Preferred, Not Exclusive

openDAQ should be the preferred generic DAQ integration path because it is open,
cross-platform, and designed for interoperable data acquisition. EMC Locus must
still support vendor SDK bridges and file replay when a DAQ is not openDAQ
compatible.

### 4. Synchronization Is a Product Feature

Multi-DAQ tests can require:

- shared sample clocks;
- external triggers;
- start triggers;
- PTP/IEEE 1588;
- GNSS/GPS time;
- IRIG-B;
- EtherCAT distributed clocks;
- hardware timestamps;
- software timestamps;
- post-alignment by cross-correlation.

The selected synchronization method must be recorded in the measurement run.

### 5. Signal Processing Should Be a Graph

The processing layer should support:

- FFT and windowed FFT;
- inverse FFT when needed;
- temporal filters;
- channel arithmetic;
- mathematical expressions;
- harmonic analysis;
- inrush analysis;
- event counting and edge timing;
- resampling;
- cross-correlation;
- RMS, peak, and envelope computations.

Every processed result should retain lineage to the raw signal inputs and
processing parameters.
