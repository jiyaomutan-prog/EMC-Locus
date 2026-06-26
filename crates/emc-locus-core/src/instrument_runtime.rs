use crate::{instrument::InstrumentTransport, metrology::InstrumentCode, DomainError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentCommandMessage(String);

impl InstrumentCommandMessage {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyInstrumentCommandMessage);
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentResponse(String);

impl InstrumentResponse {
    pub fn simulated(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentTransportEndpoint {
    transport: InstrumentTransport,
    address: String,
}

impl InstrumentTransportEndpoint {
    pub fn new(
        transport: InstrumentTransport,
        address: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let address = address.into();
        let trimmed = address.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyTransportEndpointAddress);
        }

        Ok(Self {
            transport,
            address: trimmed.to_owned(),
        })
    }

    pub fn transport(&self) -> InstrumentTransport {
        self.transport
    }

    pub fn address(&self) -> &str {
        &self.address
    }
}

pub trait InstrumentTransportAdapter {
    fn endpoint(&self) -> &InstrumentTransportEndpoint;

    fn exchange(&mut self, command: &InstrumentCommand) -> Result<InstrumentResponse, DomainError>;

    fn transport(&self) -> InstrumentTransport {
        self.endpoint().transport()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulatedTransportAdapter {
    endpoint: InstrumentTransportEndpoint,
}

impl SimulatedTransportAdapter {
    pub fn new(endpoint: InstrumentTransportEndpoint) -> Self {
        Self { endpoint }
    }
}

impl InstrumentTransportAdapter for SimulatedTransportAdapter {
    fn endpoint(&self) -> &InstrumentTransportEndpoint {
        &self.endpoint
    }

    fn exchange(&mut self, command: &InstrumentCommand) -> Result<InstrumentResponse, DomainError> {
        if command.transport() != self.transport() {
            return Err(DomainError::TransportAdapterMismatch {
                expected: self.transport().as_str().to_owned(),
                actual: command.transport().as_str().to_owned(),
            });
        }

        Ok(deterministic_response(command.message()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentCommand {
    target: InstrumentCode,
    transport: InstrumentTransport,
    message: InstrumentCommandMessage,
    setpoint: Option<InstrumentSetpoint>,
}

impl InstrumentCommand {
    pub fn new(
        target: InstrumentCode,
        transport: InstrumentTransport,
        message: InstrumentCommandMessage,
    ) -> Self {
        Self {
            target,
            transport,
            message,
            setpoint: None,
        }
    }

    pub fn with_setpoint(
        target: InstrumentCode,
        transport: InstrumentTransport,
        message: InstrumentCommandMessage,
        setpoint: InstrumentSetpoint,
    ) -> Self {
        Self {
            target,
            transport,
            message,
            setpoint: Some(setpoint),
        }
    }

    pub fn target(&self) -> &InstrumentCode {
        &self.target
    }

    pub fn transport(&self) -> InstrumentTransport {
        self.transport
    }

    pub fn message(&self) -> &InstrumentCommandMessage {
        &self.message
    }

    pub fn setpoint(&self) -> Option<InstrumentSetpoint> {
        self.setpoint
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentQuantity {
    FrequencyHz,
    LevelDbm,
    VoltageMv,
    CurrentMa,
}

impl InstrumentQuantity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FrequencyHz => "frequency_hz",
            Self::LevelDbm => "level_dbm",
            Self::VoltageMv => "voltage_mv",
            Self::CurrentMa => "current_ma",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstrumentSetpoint {
    quantity: InstrumentQuantity,
    value: i64,
}

impl InstrumentSetpoint {
    pub fn new(quantity: InstrumentQuantity, value: i64) -> Self {
        Self { quantity, value }
    }

    pub fn quantity(&self) -> InstrumentQuantity {
        self.quantity
    }

    pub fn value(&self) -> i64 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstrumentSafetyLimit {
    quantity: InstrumentQuantity,
    minimum: i64,
    maximum: i64,
}

impl InstrumentSafetyLimit {
    pub fn new(
        quantity: InstrumentQuantity,
        minimum: i64,
        maximum: i64,
    ) -> Result<Self, DomainError> {
        if minimum > maximum {
            return Err(DomainError::InvalidInstrumentSafetyLimit {
                quantity: quantity.as_str().to_owned(),
                minimum,
                maximum,
            });
        }

        Ok(Self {
            quantity,
            minimum,
            maximum,
        })
    }

    pub fn quantity(&self) -> InstrumentQuantity {
        self.quantity
    }

    pub fn minimum(&self) -> i64 {
        self.minimum
    }

    pub fn maximum(&self) -> i64 {
        self.maximum
    }

    pub fn contains(&self, setpoint: InstrumentSetpoint) -> bool {
        self.quantity == setpoint.quantity()
            && setpoint.value() >= self.minimum
            && setpoint.value() <= self.maximum
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentObservation {
    sequence: u64,
    command: InstrumentCommand,
    response: InstrumentResponse,
    success: bool,
}

impl InstrumentObservation {
    fn new(
        sequence: u64,
        command: InstrumentCommand,
        response: InstrumentResponse,
        success: bool,
    ) -> Self {
        Self {
            sequence,
            command,
            response,
            success,
        }
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn command(&self) -> &InstrumentCommand {
        &self.command
    }

    pub fn response(&self) -> &InstrumentResponse {
        &self.response
    }

    pub fn success(&self) -> bool {
        self.success
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransportAdapterRuntime<A>
where
    A: InstrumentTransportAdapter,
{
    instrument: InstrumentCode,
    adapter: A,
    safety_limits: Vec<InstrumentSafetyLimit>,
    observations: Vec<InstrumentObservation>,
    next_sequence: u64,
}

impl<A> TransportAdapterRuntime<A>
where
    A: InstrumentTransportAdapter,
{
    pub fn new(instrument: InstrumentCode, adapter: A) -> Self {
        Self {
            instrument,
            adapter,
            safety_limits: Vec::new(),
            observations: Vec::new(),
            next_sequence: 1,
        }
    }

    pub fn instrument(&self) -> &InstrumentCode {
        &self.instrument
    }

    pub fn adapter(&self) -> &A {
        &self.adapter
    }

    pub fn safety_limits(&self) -> &[InstrumentSafetyLimit] {
        &self.safety_limits
    }

    pub fn observations(&self) -> &[InstrumentObservation] {
        &self.observations
    }

    pub fn add_safety_limit(&mut self, limit: InstrumentSafetyLimit) {
        self.safety_limits.push(limit);
    }

    pub fn execute(
        &mut self,
        command: InstrumentCommand,
    ) -> Result<&InstrumentObservation, DomainError> {
        if command.target() != &self.instrument {
            return Err(DomainError::InstrumentCommandTargetMismatch {
                expected: self.instrument.as_str().to_owned(),
                actual: command.target().as_str().to_owned(),
            });
        }

        if command.transport() != self.adapter.transport() {
            return Err(DomainError::TransportAdapterMismatch {
                expected: self.adapter.transport().as_str().to_owned(),
                actual: command.transport().as_str().to_owned(),
            });
        }

        if let Some(setpoint) = command.setpoint() {
            validate_setpoint_against_limits(&self.safety_limits, setpoint)?;
        }

        let response = self.adapter.exchange(&command)?;
        let observation = InstrumentObservation::new(self.next_sequence, command, response, true);
        self.next_sequence += 1;
        self.observations.push(observation);

        Ok(self
            .observations
            .last()
            .expect("observation was just appended"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulatedInstrumentRuntime {
    instrument: InstrumentCode,
    supported_transports: Vec<InstrumentTransport>,
    safety_limits: Vec<InstrumentSafetyLimit>,
    observations: Vec<InstrumentObservation>,
    next_sequence: u64,
}

impl SimulatedInstrumentRuntime {
    pub fn new(instrument: InstrumentCode, supported_transports: Vec<InstrumentTransport>) -> Self {
        Self {
            instrument,
            supported_transports,
            safety_limits: Vec::new(),
            observations: Vec::new(),
            next_sequence: 1,
        }
    }

    pub fn instrument(&self) -> &InstrumentCode {
        &self.instrument
    }

    pub fn supported_transports(&self) -> &[InstrumentTransport] {
        &self.supported_transports
    }

    pub fn safety_limits(&self) -> &[InstrumentSafetyLimit] {
        &self.safety_limits
    }

    pub fn observations(&self) -> &[InstrumentObservation] {
        &self.observations
    }

    pub fn add_safety_limit(&mut self, limit: InstrumentSafetyLimit) {
        self.safety_limits.push(limit);
    }

    pub fn execute(
        &mut self,
        command: InstrumentCommand,
    ) -> Result<&InstrumentObservation, DomainError> {
        if command.target() != &self.instrument {
            return Err(DomainError::InstrumentCommandTargetMismatch {
                expected: self.instrument.as_str().to_owned(),
                actual: command.target().as_str().to_owned(),
            });
        }

        if !self.supported_transports.contains(&command.transport()) {
            return Err(DomainError::UnsupportedInstrumentTransport(
                command.transport().as_str().to_owned(),
            ));
        }

        if let Some(setpoint) = command.setpoint() {
            validate_setpoint_against_limits(&self.safety_limits, setpoint)?;
        }

        let response = deterministic_response(command.message());
        let observation = InstrumentObservation::new(self.next_sequence, command, response, true);
        self.next_sequence += 1;
        self.observations.push(observation);

        Ok(self
            .observations
            .last()
            .expect("observation was just appended"))
    }
}

fn validate_setpoint_against_limits(
    limits: &[InstrumentSafetyLimit],
    setpoint: InstrumentSetpoint,
) -> Result<(), DomainError> {
    let Some(limit) = limits
        .iter()
        .find(|limit| limit.quantity() == setpoint.quantity())
    else {
        return Ok(());
    };

    if !limit.contains(setpoint) {
        return Err(DomainError::InstrumentSetpointOutOfRange {
            quantity: setpoint.quantity().as_str().to_owned(),
            value: setpoint.value(),
            minimum: limit.minimum(),
            maximum: limit.maximum(),
        });
    }

    Ok(())
}

fn deterministic_response(message: &InstrumentCommandMessage) -> InstrumentResponse {
    if message.as_str().ends_with('?') {
        InstrumentResponse::simulated(format!("SIM:{}=0", message.as_str()))
    } else {
        InstrumentResponse::simulated(format!("OK:{}", message.as_str()))
    }
}
