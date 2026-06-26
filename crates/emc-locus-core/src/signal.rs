use crate::DomainError;

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
