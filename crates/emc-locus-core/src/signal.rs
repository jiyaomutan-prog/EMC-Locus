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
