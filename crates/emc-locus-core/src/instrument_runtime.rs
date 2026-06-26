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
pub struct InstrumentCommand {
    target: InstrumentCode,
    transport: InstrumentTransport,
    message: InstrumentCommandMessage,
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
pub struct SimulatedInstrumentRuntime {
    instrument: InstrumentCode,
    supported_transports: Vec<InstrumentTransport>,
    observations: Vec<InstrumentObservation>,
    next_sequence: u64,
}

impl SimulatedInstrumentRuntime {
    pub fn new(instrument: InstrumentCode, supported_transports: Vec<InstrumentTransport>) -> Self {
        Self {
            instrument,
            supported_transports,
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

    pub fn observations(&self) -> &[InstrumentObservation] {
        &self.observations
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

fn deterministic_response(message: &InstrumentCommandMessage) -> InstrumentResponse {
    if message.as_str().ends_with('?') {
        InstrumentResponse::simulated(format!("SIM:{}=0", message.as_str()))
    } else {
        InstrumentResponse::simulated(format!("OK:{}", message.as_str()))
    }
}
