use std::f64::consts::PI;

use crate::{
    datasets::{DatasetChecksum, DatasetFileReference, DatasetKind, DatasetReference},
    identifiers::AuditActor,
    DomainError,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementAxis {
    FrequencySweep,
    TimeSeries,
    EventTriggered,
    MixedTimeFrequency,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaqInterface {
    OpenDaq,
    VendorSdk,
    Usb,
    Ethernet,
    EtherCat,
    Pcie,
    SoundCard,
    VisaDigitizer,
    FileReplay,
    Simulated,
}

impl DaqInterface {
    pub fn preferred_generic() -> Self {
        Self::OpenDaq
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalSourceKind {
    AnalogVoltage,
    AnalogCurrent,
    DigitalInput,
    Counter,
    Encoder,
    BusFrame,
    VideoFrame,
    DerivedSignal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynchronizationMethod {
    SharedSampleClock,
    ExternalTrigger,
    StartTrigger,
    PtpIeee1588,
    GpsGnss,
    IrigB,
    EtherCatDistributedClock,
    HardwareTimestamp,
    SoftwareTimestamp,
    CrossCorrelationPostAlignment,
}

pub fn baseline_synchronization_methods() -> Vec<SynchronizationMethod> {
    use SynchronizationMethod::*;

    vec![
        SharedSampleClock,
        ExternalTrigger,
        StartTrigger,
        PtpIeee1588,
        GpsGnss,
        IrigB,
        EtherCatDistributedClock,
        HardwareTimestamp,
        SoftwareTimestamp,
        CrossCorrelationPostAlignment,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalProcessingOperation {
    Fft,
    WindowedFft,
    Ifft,
    TimeDomainFilter,
    ChannelArithmetic,
    MathExpression,
    HarmonicAnalysis,
    InrushAnalysis,
    EventCounting,
    EdgeTiming,
    Resampling,
    CrossCorrelation,
    Rms,
    Peak,
    Envelope,
}

pub fn baseline_signal_processing_operations() -> Vec<SignalProcessingOperation> {
    use SignalProcessingOperation::*;

    vec![
        Fft,
        WindowedFft,
        Ifft,
        TimeDomainFilter,
        ChannelArithmetic,
        MathExpression,
        HarmonicAnalysis,
        InrushAnalysis,
        EventCounting,
        EdgeTiming,
        Resampling,
        CrossCorrelation,
        Rms,
        Peak,
        Envelope,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CemTimeDomainTestFamily {
    RailwayHarmonics,
    AxleCounter,
    InrushCurrent,
    TransientCapture,
    PowerQuality,
    PulsedDisturbance,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalReference(String);

impl SignalReference {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptySignalReference);
        }

        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(DomainError::InvalidSignalReference(trimmed.to_owned()));
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessingGraphReference(String);

impl ProcessingGraphReference {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyProcessingGraphReference);
        }

        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(DomainError::InvalidProcessingGraphReference(
                trimmed.to_owned(),
            ));
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessingGraphRevision(String);

impl ProcessingGraphRevision {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyProcessingGraphRevision);
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalUnit(String);

impl SignalUnit {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptySignalUnit);
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SampleRateHz(u32);

impl SampleRateHz {
    pub fn new(value: u32) -> Result<Self, DomainError> {
        if value == 0 {
            return Err(DomainError::InvalidSampleRateHz(value));
        }

        Ok(Self(value))
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcquiredSignalChannel {
    reference: SignalReference,
    source_kind: SignalSourceKind,
    unit: SignalUnit,
    sample_rate: SampleRateHz,
    samples: Vec<i64>,
}

impl AcquiredSignalChannel {
    pub fn new(
        reference: SignalReference,
        source_kind: SignalSourceKind,
        unit: SignalUnit,
        sample_rate: SampleRateHz,
        samples: Vec<i64>,
    ) -> Self {
        Self {
            reference,
            source_kind,
            unit,
            sample_rate,
            samples,
        }
    }

    pub fn reference(&self) -> &SignalReference {
        &self.reference
    }

    pub fn source_kind(&self) -> SignalSourceKind {
        self.source_kind
    }

    pub fn unit(&self) -> &SignalUnit {
        &self.unit
    }

    pub fn sample_rate(&self) -> SampleRateHz {
        self.sample_rate
    }

    pub fn samples(&self) -> &[i64] {
        &self.samples
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalDataset {
    daq_interface: DaqInterface,
    synchronization_method: SynchronizationMethod,
    channels: Vec<AcquiredSignalChannel>,
}

impl SignalDataset {
    pub fn new(
        daq_interface: DaqInterface,
        synchronization_method: SynchronizationMethod,
        channels: Vec<AcquiredSignalChannel>,
    ) -> Result<Self, DomainError> {
        if channels.is_empty() {
            return Err(DomainError::EmptySignalDataset);
        }

        Ok(Self {
            daq_interface,
            synchronization_method,
            channels,
        })
    }

    pub fn daq_interface(&self) -> DaqInterface {
        self.daq_interface
    }

    pub fn synchronization_method(&self) -> SynchronizationMethod {
        self.synchronization_method
    }

    pub fn channels(&self) -> &[AcquiredSignalChannel] {
        &self.channels
    }

    pub fn channel(&self, reference: &SignalReference) -> Option<&AcquiredSignalChannel> {
        self.channels
            .iter()
            .find(|channel| channel.reference() == reference)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulatedDaqSource {
    interface: DaqInterface,
    synchronization_method: SynchronizationMethod,
}

impl SimulatedDaqSource {
    pub fn open_daq() -> Self {
        Self {
            interface: DaqInterface::OpenDaq,
            synchronization_method: SynchronizationMethod::SharedSampleClock,
        }
    }

    pub fn interface(&self) -> DaqInterface {
        self.interface
    }

    pub fn synchronization_method(&self) -> SynchronizationMethod {
        self.synchronization_method
    }

    pub fn acquire_inrush_fixture(&self) -> Result<SignalDataset, DomainError> {
        let sample_rate = SampleRateHz::new(10_000)?;
        let voltage = AcquiredSignalChannel::new(
            SignalReference::parse("voltage_l1")?,
            SignalSourceKind::AnalogVoltage,
            SignalUnit::parse("mV")?,
            sample_rate,
            vec![0, 100, 260, 520, 260, 100, 0, -100],
        );
        let current = AcquiredSignalChannel::new(
            SignalReference::parse("current_l1")?,
            SignalSourceKind::AnalogCurrent,
            SignalUnit::parse("mA")?,
            sample_rate,
            vec![0, 20, 60, 180, 120, 40, 5, 0],
        );

        SignalDataset::new(
            self.interface,
            self.synchronization_method,
            vec![voltage, current],
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalProcessingNode {
    id: SignalReference,
    operation: SignalProcessingOperation,
    inputs: Vec<SignalReference>,
    output: SignalReference,
}

impl SignalProcessingNode {
    pub fn new(
        id: SignalReference,
        operation: SignalProcessingOperation,
        inputs: Vec<SignalReference>,
        output: SignalReference,
    ) -> Result<Self, DomainError> {
        if inputs.is_empty() {
            return Err(DomainError::EmptyProcessingNodeInputs);
        }

        Ok(Self {
            id,
            operation,
            inputs,
            output,
        })
    }

    pub fn id(&self) -> &SignalReference {
        &self.id
    }

    pub fn operation(&self) -> SignalProcessingOperation {
        self.operation
    }

    pub fn inputs(&self) -> &[SignalReference] {
        &self.inputs
    }

    pub fn output(&self) -> &SignalReference {
        &self.output
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalProcessingGraph {
    source_signals: Vec<SignalReference>,
    nodes: Vec<SignalProcessingNode>,
}

impl SignalProcessingGraph {
    pub fn from_dataset(dataset: &SignalDataset) -> Self {
        Self {
            source_signals: dataset
                .channels()
                .iter()
                .map(|channel| channel.reference().clone())
                .collect(),
            nodes: Vec::new(),
        }
    }

    pub fn source_signals(&self) -> &[SignalReference] {
        &self.source_signals
    }

    pub fn nodes(&self) -> &[SignalProcessingNode] {
        &self.nodes
    }

    pub fn add_node(&mut self, node: SignalProcessingNode) -> Result<(), DomainError> {
        if self.nodes.iter().any(|existing| existing.id() == node.id()) {
            return Err(DomainError::DuplicateProcessingNode(
                node.id().as_str().to_owned(),
            ));
        }

        for input in node.inputs() {
            if !self.knows_signal(input) {
                return Err(DomainError::UnknownSignalReference(
                    input.as_str().to_owned(),
                ));
            }
        }

        self.nodes.push(node);
        Ok(())
    }

    pub fn contains_operation(&self, operation: SignalProcessingOperation) -> bool {
        self.nodes.iter().any(|node| node.operation() == operation)
    }

    pub fn has_nodes(&self) -> bool {
        !self.nodes.is_empty()
    }

    pub fn raw_lineage_for(
        &self,
        signal: &SignalReference,
    ) -> Result<Vec<SignalReference>, DomainError> {
        if !self.knows_signal(signal) {
            return Err(DomainError::UnknownSignalReference(
                signal.as_str().to_owned(),
            ));
        }

        let mut lineage = Vec::new();
        self.collect_raw_lineage(signal, &mut lineage);
        Ok(lineage)
    }

    fn knows_signal(&self, signal: &SignalReference) -> bool {
        self.source_signals.contains(signal)
            || self.nodes.iter().any(|node| node.output() == signal)
    }

    fn collect_raw_lineage(&self, signal: &SignalReference, lineage: &mut Vec<SignalReference>) {
        if self.source_signals.contains(signal) {
            if !lineage.contains(signal) {
                lineage.push(signal.clone());
            }
            return;
        }

        if let Some(node) = self.nodes.iter().find(|node| node.output() == signal) {
            for input in node.inputs() {
                self.collect_raw_lineage(input, lineage);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessingGraphInstance {
    reference: ProcessingGraphReference,
    revision: ProcessingGraphRevision,
    source_dataset: DatasetReference,
    source_dataset_checksum: DatasetChecksum,
    graph: SignalProcessingGraph,
    definition_checksum: DatasetChecksum,
    created_by: AuditActor,
    software_version: String,
}

impl ProcessingGraphInstance {
    pub fn new(
        reference: ProcessingGraphReference,
        revision: ProcessingGraphRevision,
        source_dataset: DatasetReference,
        source_dataset_checksum: DatasetChecksum,
        graph: SignalProcessingGraph,
        definition_checksum: DatasetChecksum,
        created_by: AuditActor,
        software_version: impl Into<String>,
    ) -> Result<Self, DomainError> {
        if !graph.has_nodes() {
            return Err(DomainError::EmptyProcessingGraphDefinition(
                reference.as_str().to_owned(),
            ));
        }

        let software_version = software_version.into();
        let software_version = software_version.trim();
        if software_version.is_empty() {
            return Err(DomainError::EmptyProcessingGraphSoftwareVersion);
        }

        Ok(Self {
            reference,
            revision,
            source_dataset,
            source_dataset_checksum,
            graph,
            definition_checksum,
            created_by,
            software_version: software_version.to_owned(),
        })
    }

    pub fn reference(&self) -> &ProcessingGraphReference {
        &self.reference
    }

    pub fn revision(&self) -> &ProcessingGraphRevision {
        &self.revision
    }

    pub fn source_dataset(&self) -> &DatasetReference {
        &self.source_dataset
    }

    pub fn source_dataset_checksum(&self) -> &DatasetChecksum {
        &self.source_dataset_checksum
    }

    pub fn graph(&self) -> &SignalProcessingGraph {
        &self.graph
    }

    pub fn definition_checksum(&self) -> &DatasetChecksum {
        &self.definition_checksum
    }

    pub fn created_by(&self) -> &AuditActor {
        &self.created_by
    }

    pub fn software_version(&self) -> &str {
        &self.software_version
    }

    pub fn contains_operation(&self, operation: SignalProcessingOperation) -> bool {
        self.graph.contains_operation(operation)
    }

    pub fn raw_lineage_for(
        &self,
        signal: &SignalReference,
    ) -> Result<Vec<SignalReference>, DomainError> {
        self.graph.raw_lineage_for(signal)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessingGraphResultArtifact {
    graph_reference: ProcessingGraphReference,
    graph_revision: ProcessingGraphRevision,
    output_signal: SignalReference,
    kind: DatasetKind,
    file_reference: DatasetFileReference,
    checksum: DatasetChecksum,
    raw_lineage: Vec<SignalReference>,
}

impl ProcessingGraphResultArtifact {
    pub fn from_instance(
        instance: &ProcessingGraphInstance,
        output_signal: SignalReference,
        kind: DatasetKind,
        file_reference: DatasetFileReference,
        checksum: DatasetChecksum,
    ) -> Result<Self, DomainError> {
        if !matches!(
            kind,
            DatasetKind::ProcessedSignal | DatasetKind::ResultTable
        ) {
            return Err(DomainError::InvalidProcessingGraphArtifactKind(
                dataset_kind_slug(kind).to_owned(),
            ));
        }

        let raw_lineage = instance.raw_lineage_for(&output_signal)?;

        Ok(Self {
            graph_reference: instance.reference().clone(),
            graph_revision: instance.revision().clone(),
            output_signal,
            kind,
            file_reference,
            checksum,
            raw_lineage,
        })
    }

    pub fn graph_reference(&self) -> &ProcessingGraphReference {
        &self.graph_reference
    }

    pub fn graph_revision(&self) -> &ProcessingGraphRevision {
        &self.graph_revision
    }

    pub fn output_signal(&self) -> &SignalReference {
        &self.output_signal
    }

    pub fn kind(&self) -> DatasetKind {
        self.kind
    }

    pub fn file_reference(&self) -> &DatasetFileReference {
        &self.file_reference
    }

    pub fn checksum(&self) -> &DatasetChecksum {
        &self.checksum
    }

    pub fn raw_lineage(&self) -> &[SignalReference] {
        &self.raw_lineage
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SignalSeriesResult {
    output: SignalReference,
    operation: SignalProcessingOperation,
    unit: SignalUnit,
    samples: Vec<i64>,
    raw_lineage: Vec<SignalReference>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SignalFloatSeriesResult {
    output: SignalReference,
    operation: SignalProcessingOperation,
    unit: SignalUnit,
    sample_rate: SampleRateHz,
    samples: Vec<f64>,
    raw_lineage: Vec<SignalReference>,
}

impl SignalFloatSeriesResult {
    fn new(
        output: SignalReference,
        operation: SignalProcessingOperation,
        unit: SignalUnit,
        sample_rate: SampleRateHz,
        samples: Vec<f64>,
        raw_lineage: Vec<SignalReference>,
    ) -> Self {
        Self {
            output,
            operation,
            unit,
            sample_rate,
            samples,
            raw_lineage,
        }
    }

    pub fn output(&self) -> &SignalReference {
        &self.output
    }

    pub fn operation(&self) -> SignalProcessingOperation {
        self.operation
    }

    pub fn unit(&self) -> &SignalUnit {
        &self.unit
    }

    pub fn sample_rate(&self) -> SampleRateHz {
        self.sample_rate
    }

    pub fn samples(&self) -> &[f64] {
        &self.samples
    }

    pub fn raw_lineage(&self) -> &[SignalReference] {
        &self.raw_lineage
    }
}

impl SignalSeriesResult {
    fn new(
        output: SignalReference,
        operation: SignalProcessingOperation,
        unit: SignalUnit,
        samples: Vec<i64>,
        raw_lineage: Vec<SignalReference>,
    ) -> Self {
        Self {
            output,
            operation,
            unit,
            samples,
            raw_lineage,
        }
    }

    pub fn output(&self) -> &SignalReference {
        &self.output
    }

    pub fn operation(&self) -> SignalProcessingOperation {
        self.operation
    }

    pub fn unit(&self) -> &SignalUnit {
        &self.unit
    }

    pub fn samples(&self) -> &[i64] {
        &self.samples
    }

    pub fn raw_lineage(&self) -> &[SignalReference] {
        &self.raw_lineage
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SignalScalarResult {
    output: SignalReference,
    operation: SignalProcessingOperation,
    unit: SignalUnit,
    value: f64,
    raw_lineage: Vec<SignalReference>,
}

impl SignalScalarResult {
    fn new(
        output: SignalReference,
        operation: SignalProcessingOperation,
        unit: SignalUnit,
        value: f64,
        raw_lineage: Vec<SignalReference>,
    ) -> Self {
        Self {
            output,
            operation,
            unit,
            value,
            raw_lineage,
        }
    }

    pub fn output(&self) -> &SignalReference {
        &self.output
    }

    pub fn operation(&self) -> SignalProcessingOperation {
        self.operation
    }

    pub fn unit(&self) -> &SignalUnit {
        &self.unit
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn raw_lineage(&self) -> &[SignalReference] {
        &self.raw_lineage
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SignalSpectrumResult {
    output: SignalReference,
    operation: SignalProcessingOperation,
    backend: FrequencyTransformBackend,
    magnitudes: Vec<f64>,
    raw_lineage: Vec<SignalReference>,
}

impl SignalSpectrumResult {
    fn new(
        output: SignalReference,
        operation: SignalProcessingOperation,
        backend: FrequencyTransformBackend,
        magnitudes: Vec<f64>,
        raw_lineage: Vec<SignalReference>,
    ) -> Self {
        Self {
            output,
            operation,
            backend,
            magnitudes,
            raw_lineage,
        }
    }

    pub fn output(&self) -> &SignalReference {
        &self.output
    }

    pub fn operation(&self) -> SignalProcessingOperation {
        self.operation
    }

    pub fn backend(&self) -> FrequencyTransformBackend {
        self.backend
    }

    pub fn magnitudes(&self) -> &[f64] {
        &self.magnitudes
    }

    pub fn raw_lineage(&self) -> &[SignalReference] {
        &self.raw_lineage
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrequencyTransformBackend {
    ReferenceDft,
    OptimizedFftCompatible,
}

impl FrequencyTransformBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReferenceDft => "reference_dft",
            Self::OptimizedFftCompatible => "optimized_fft_compatible",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowFunction {
    Rectangular,
    Hann,
    Hamming,
    Blackman,
    FlatTop,
}

impl WindowFunction {
    pub fn coefficient(self, index: usize, sample_count: usize) -> f64 {
        if sample_count <= 1 {
            return 1.0;
        }

        let angle = 2.0 * PI * index as f64 / (sample_count - 1) as f64;

        match self {
            Self::Rectangular => 1.0,
            Self::Hann => 0.5 - 0.5 * angle.cos(),
            Self::Hamming => 0.54 - 0.46 * angle.cos(),
            Self::Blackman => 0.42 - 0.5 * angle.cos() + 0.08 * (2.0 * angle).cos(),
            Self::FlatTop => {
                0.215_578_95 - 0.416_631_58 * angle.cos() + 0.277_263_158 * (2.0 * angle).cos()
                    - 0.083_578_947 * (3.0 * angle).cos()
                    + 0.006_947_368 * (4.0 * angle).cos()
            }
        }
    }
}

pub struct SignalExecutionEngine;

impl SignalExecutionEngine {
    pub fn channel_sum(
        dataset: &SignalDataset,
        left: &SignalReference,
        right: &SignalReference,
        output: SignalReference,
        output_unit: SignalUnit,
    ) -> Result<SignalSeriesResult, DomainError> {
        let left_channel = required_channel(dataset, left)?;
        let right_channel = required_channel(dataset, right)?;

        validate_sample_compatibility(left_channel, right_channel)?;

        let samples = left_channel
            .samples()
            .iter()
            .zip(right_channel.samples())
            .map(|(left, right)| left + right)
            .collect();

        Ok(SignalSeriesResult::new(
            output,
            SignalProcessingOperation::ChannelArithmetic,
            output_unit,
            samples,
            raw_lineage(vec![left.clone(), right.clone()]),
        ))
    }

    pub fn apply_window(
        dataset: &SignalDataset,
        source: &SignalReference,
        output: SignalReference,
        window: WindowFunction,
    ) -> Result<SignalFloatSeriesResult, DomainError> {
        let channel = required_channel(dataset, source)?;
        if channel.samples().is_empty() {
            return Err(DomainError::EmptySignalSamples(source.as_str().to_owned()));
        }

        let sample_count = channel.samples().len();
        let samples = channel
            .samples()
            .iter()
            .enumerate()
            .map(|(index, sample)| *sample as f64 * window.coefficient(index, sample_count))
            .collect();

        Ok(SignalFloatSeriesResult::new(
            output,
            SignalProcessingOperation::TimeDomainFilter,
            channel.unit().clone(),
            channel.sample_rate(),
            samples,
            vec![source.clone()],
        ))
    }

    pub fn resample_linear(
        dataset: &SignalDataset,
        source: &SignalReference,
        output: SignalReference,
        target_sample_rate: SampleRateHz,
    ) -> Result<SignalFloatSeriesResult, DomainError> {
        let channel = required_channel(dataset, source)?;
        if channel.samples().is_empty() {
            return Err(DomainError::EmptySignalSamples(source.as_str().to_owned()));
        }

        let samples = linear_resample(channel.samples(), channel.sample_rate(), target_sample_rate);

        Ok(SignalFloatSeriesResult::new(
            output,
            SignalProcessingOperation::Resampling,
            channel.unit().clone(),
            target_sample_rate,
            samples,
            vec![source.clone()],
        ))
    }

    pub fn downsample(
        dataset: &SignalDataset,
        source: &SignalReference,
        output: SignalReference,
        factor: usize,
    ) -> Result<SignalSeriesResult, DomainError> {
        if factor == 0 {
            return Err(DomainError::InvalidResamplingFactor(factor));
        }

        let channel = required_channel(dataset, source)?;
        if channel.samples().is_empty() {
            return Err(DomainError::EmptySignalSamples(source.as_str().to_owned()));
        }

        let samples = channel.samples().iter().step_by(factor).copied().collect();

        Ok(SignalSeriesResult::new(
            output,
            SignalProcessingOperation::Resampling,
            channel.unit().clone(),
            samples,
            vec![source.clone()],
        ))
    }

    pub fn peak(
        dataset: &SignalDataset,
        source: &SignalReference,
        output: SignalReference,
    ) -> Result<SignalScalarResult, DomainError> {
        let channel = required_channel(dataset, source)?;
        if channel.samples().is_empty() {
            return Err(DomainError::EmptySignalSamples(source.as_str().to_owned()));
        }

        let value = channel
            .samples()
            .iter()
            .map(|sample| sample.abs() as f64)
            .fold(0.0, f64::max);

        Ok(SignalScalarResult::new(
            output,
            SignalProcessingOperation::Peak,
            channel.unit().clone(),
            value,
            vec![source.clone()],
        ))
    }

    pub fn dft_magnitude(
        dataset: &SignalDataset,
        source: &SignalReference,
        output: SignalReference,
    ) -> Result<SignalSpectrumResult, DomainError> {
        Self::spectrum_magnitude_with_backend(
            dataset,
            source,
            output,
            FrequencyTransformBackend::ReferenceDft,
        )
    }

    pub fn spectrum_magnitude_with_backend(
        dataset: &SignalDataset,
        source: &SignalReference,
        output: SignalReference,
        backend: FrequencyTransformBackend,
    ) -> Result<SignalSpectrumResult, DomainError> {
        let channel = required_channel(dataset, source)?;
        if channel.samples().is_empty() {
            return Err(DomainError::EmptySignalSamples(source.as_str().to_owned()));
        }

        let samples: Vec<f64> = channel
            .samples()
            .iter()
            .map(|sample| *sample as f64)
            .collect();
        let magnitudes = match backend {
            FrequencyTransformBackend::ReferenceDft => reference_dft_magnitudes(&samples),
            FrequencyTransformBackend::OptimizedFftCompatible => optimized_fft_magnitudes(&samples),
        };

        Ok(SignalSpectrumResult::new(
            output,
            SignalProcessingOperation::Fft,
            backend,
            magnitudes,
            vec![source.clone()],
        ))
    }
}

fn linear_resample(
    samples: &[i64],
    source_sample_rate: SampleRateHz,
    target_sample_rate: SampleRateHz,
) -> Vec<f64> {
    if samples.len() == 1 {
        return vec![samples[0] as f64];
    }

    let source_hz = source_sample_rate.value() as f64;
    let target_hz = target_sample_rate.value() as f64;
    let last_source_index = samples.len() - 1;
    let output_count = ((last_source_index as f64 * target_hz / source_hz).floor() as usize) + 1;

    (0..output_count)
        .map(|target_index| {
            let source_position = target_index as f64 * source_hz / target_hz;
            if source_position >= last_source_index as f64 {
                return samples[last_source_index] as f64;
            }

            let lower_index = source_position.floor() as usize;
            let upper_index = lower_index + 1;
            let fraction = source_position - lower_index as f64;
            let lower = samples[lower_index] as f64;
            let upper = samples[upper_index] as f64;

            lower + (upper - lower) * fraction
        })
        .collect()
}

fn reference_dft_magnitudes(samples: &[f64]) -> Vec<f64> {
    let count = samples.len();
    let mut magnitudes = Vec::with_capacity(count);

    for bin in 0..count {
        let mut real = 0.0;
        let mut imaginary = 0.0;
        for (index, sample) in samples.iter().enumerate() {
            let angle = -2.0 * PI * bin as f64 * index as f64 / count as f64;
            real += sample * angle.cos();
            imaginary += sample * angle.sin();
        }
        magnitudes.push((real.powi(2) + imaginary.powi(2)).sqrt());
    }

    magnitudes
}

#[derive(Clone, Copy, Debug)]
struct ComplexSample {
    real: f64,
    imaginary: f64,
}

impl ComplexSample {
    fn new(real: f64, imaginary: f64) -> Self {
        Self { real, imaginary }
    }

    fn magnitude(self) -> f64 {
        (self.real.powi(2) + self.imaginary.powi(2)).sqrt()
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.real + other.real, self.imaginary + other.imaginary)
    }

    fn sub(self, other: Self) -> Self {
        Self::new(self.real - other.real, self.imaginary - other.imaginary)
    }

    fn mul(self, other: Self) -> Self {
        Self::new(
            self.real * other.real - self.imaginary * other.imaginary,
            self.real * other.imaginary + self.imaginary * other.real,
        )
    }
}

fn optimized_fft_magnitudes(samples: &[f64]) -> Vec<f64> {
    if !samples.len().is_power_of_two() {
        return reference_dft_magnitudes(samples);
    }

    let count = samples.len();
    let bit_count = count.trailing_zeros();
    let mut values = vec![ComplexSample::new(0.0, 0.0); count];

    for (index, sample) in samples.iter().enumerate() {
        let reversed = reverse_bits(index, bit_count);
        values[reversed] = ComplexSample::new(*sample, 0.0);
    }

    let mut width = 2;
    while width <= count {
        let half_width = width / 2;
        let angle_step = -2.0 * PI / width as f64;

        for start in (0..count).step_by(width) {
            for offset in 0..half_width {
                let angle = angle_step * offset as f64;
                let twiddle = ComplexSample::new(angle.cos(), angle.sin());
                let even = values[start + offset];
                let odd = values[start + offset + half_width].mul(twiddle);

                values[start + offset] = even.add(odd);
                values[start + offset + half_width] = even.sub(odd);
            }
        }

        width *= 2;
    }

    values.into_iter().map(ComplexSample::magnitude).collect()
}

fn reverse_bits(value: usize, bit_count: u32) -> usize {
    let mut reversed = 0_usize;

    for bit in 0..bit_count {
        if value & (1_usize << bit) != 0 {
            reversed |= 1_usize << (bit_count - 1 - bit);
        }
    }

    reversed
}

fn required_channel<'a>(
    dataset: &'a SignalDataset,
    reference: &SignalReference,
) -> Result<&'a AcquiredSignalChannel, DomainError> {
    dataset
        .channel(reference)
        .ok_or_else(|| DomainError::UnknownSignalReference(reference.as_str().to_owned()))
}

fn validate_sample_compatibility(
    left: &AcquiredSignalChannel,
    right: &AcquiredSignalChannel,
) -> Result<(), DomainError> {
    if left.sample_rate() != right.sample_rate() {
        return Err(DomainError::SignalSampleRateMismatch {
            left_hz: left.sample_rate().value(),
            right_hz: right.sample_rate().value(),
        });
    }

    if left.samples().len() != right.samples().len() {
        return Err(DomainError::SignalSampleCountMismatch {
            left_count: left.samples().len(),
            right_count: right.samples().len(),
        });
    }

    Ok(())
}

fn dataset_kind_slug(kind: DatasetKind) -> &'static str {
    match kind {
        DatasetKind::RawSignal => "raw_signal",
        DatasetKind::RawSweep => "raw_sweep",
        DatasetKind::CommandLog => "command_log",
        DatasetKind::ProcessedSignal => "processed_signal",
        DatasetKind::ResultTable => "result_table",
        DatasetKind::ReportExport => "report_export",
    }
}

fn raw_lineage(inputs: Vec<SignalReference>) -> Vec<SignalReference> {
    let mut lineage = Vec::new();
    for input in inputs {
        if !lineage.contains(&input) {
            lineage.push(input);
        }
    }
    lineage
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalWorkflowProfile {
    axis: MeasurementAxis,
    preferred_daq_interface: DaqInterface,
    synchronization_required: bool,
    operations: Vec<SignalProcessingOperation>,
}

impl SignalWorkflowProfile {
    pub fn cem_time_domain_default() -> Self {
        Self {
            axis: MeasurementAxis::MixedTimeFrequency,
            preferred_daq_interface: DaqInterface::preferred_generic(),
            synchronization_required: true,
            operations: vec![
                SignalProcessingOperation::TimeDomainFilter,
                SignalProcessingOperation::Fft,
                SignalProcessingOperation::WindowedFft,
                SignalProcessingOperation::ChannelArithmetic,
                SignalProcessingOperation::MathExpression,
                SignalProcessingOperation::HarmonicAnalysis,
                SignalProcessingOperation::InrushAnalysis,
                SignalProcessingOperation::EventCounting,
                SignalProcessingOperation::EdgeTiming,
                SignalProcessingOperation::CrossCorrelation,
            ],
        }
    }

    pub fn axis(&self) -> MeasurementAxis {
        self.axis
    }

    pub fn preferred_daq_interface(&self) -> DaqInterface {
        self.preferred_daq_interface
    }

    pub fn synchronization_required(&self) -> bool {
        self.synchronization_required
    }

    pub fn operations(&self) -> &[SignalProcessingOperation] {
        &self.operations
    }
}
