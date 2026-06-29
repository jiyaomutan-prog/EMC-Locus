use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use crate::{instrument::InstrumentTransport, metrology::InstrumentCode, DomainError};

const DEFAULT_SCPI_TCP_PORT: u16 = 5025;
const MAX_GPIB_ADDRESS: u8 = 30;

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

    pub fn received(value: impl Into<String>) -> Self {
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

    fn last_exchange_attempt_count(&self) -> u16 {
        1
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransportTimeoutPolicy {
    connect_timeout_ms: u32,
    response_timeout_ms: u32,
    max_retries: u8,
}

impl TransportTimeoutPolicy {
    pub fn new(
        connect_timeout_ms: u32,
        response_timeout_ms: u32,
        max_retries: u8,
    ) -> Result<Self, DomainError> {
        if connect_timeout_ms == 0 || response_timeout_ms == 0 {
            return Err(DomainError::InvalidTransportTimeoutPolicy {
                connect_timeout_ms,
                response_timeout_ms,
                max_retries,
            });
        }

        Ok(Self {
            connect_timeout_ms,
            response_timeout_ms,
            max_retries,
        })
    }

    pub fn laboratory_default() -> Self {
        Self {
            connect_timeout_ms: 2_000,
            response_timeout_ms: 5_000,
            max_retries: 1,
        }
    }

    pub fn connect_timeout_ms(&self) -> u32 {
        self.connect_timeout_ms
    }

    pub fn response_timeout_ms(&self) -> u32 {
        self.response_timeout_ms
    }

    pub fn max_retries(&self) -> u8 {
        self.max_retries
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SerialParity {
    None,
    Even,
    Odd,
}

impl SerialParity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Even => "even",
            Self::Odd => "odd",
        }
    }

    fn parse(value: char) -> Option<Self> {
        match value.to_ascii_uppercase() {
            'N' => Some(Self::None),
            'E' => Some(Self::Even),
            'O' => Some(Self::Odd),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SerialStopBits {
    One,
    Two,
}

impl SerialStopBits {
    pub fn value(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
        }
    }

    fn parse(value: char) -> Option<Self> {
        match value {
            '1' => Some(Self::One),
            '2' => Some(Self::Two),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerialEndpointSettings {
    port: String,
    baud_rate: u32,
    data_bits: u8,
    parity: SerialParity,
    stop_bits: SerialStopBits,
}

impl SerialEndpointSettings {
    pub fn parse(address: &str) -> Result<Self, DomainError> {
        let parts: Vec<&str> = address.split(':').collect();
        if parts.len() < 2 || parts.len() > 3 {
            return Err(invalid_serial_endpoint(address));
        }

        let port = parts[0].trim();
        if !is_valid_serial_port_name(port) {
            return Err(invalid_serial_endpoint(address));
        }

        let baud_rate = parts[1]
            .trim()
            .parse::<u32>()
            .map_err(|_| invalid_serial_endpoint(address))?;
        if baud_rate == 0 {
            return Err(invalid_serial_endpoint(address));
        }

        let (data_bits, parity, stop_bits) = match parts.get(2).map(|value| value.trim()) {
            None | Some("") => (8, SerialParity::None, SerialStopBits::One),
            Some(framing) => parse_serial_framing(framing, address)?,
        };

        Ok(Self {
            port: port.to_owned(),
            baud_rate,
            data_bits,
            parity,
            stop_bits,
        })
    }

    pub fn port(&self) -> &str {
        &self.port
    }

    pub fn baud_rate(&self) -> u32 {
        self.baud_rate
    }

    pub fn data_bits(&self) -> u8 {
        self.data_bits
    }

    pub fn parity(&self) -> SerialParity {
        self.parity
    }

    pub fn stop_bits(&self) -> SerialStopBits {
        self.stop_bits
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisaInterface {
    TcpIp,
    Usb,
    Gpib,
    Serial,
}

impl VisaInterface {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TcpIp => "tcp_ip",
            Self::Usb => "usb",
            Self::Gpib => "gpib",
            Self::Serial => "serial",
        }
    }

    fn parse(prefix: &str) -> Option<Self> {
        let prefix = prefix.to_ascii_uppercase();
        if visa_prefix_matches(&prefix, "TCPIP", false) {
            Some(Self::TcpIp)
        } else if visa_prefix_matches(&prefix, "USB", false) {
            Some(Self::Usb)
        } else if visa_prefix_matches(&prefix, "GPIB", false) {
            Some(Self::Gpib)
        } else if visa_prefix_matches(&prefix, "ASRL", true) {
            Some(Self::Serial)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisaResourceAddress {
    raw: String,
    interface: VisaInterface,
    resource_class: String,
}

impl VisaResourceAddress {
    pub fn parse(address: &str) -> Result<Self, DomainError> {
        let trimmed = address.trim();
        let parts: Vec<&str> = trimmed.split("::").collect();
        if parts.len() < 2 || parts.iter().any(|part| part.trim().is_empty()) {
            return Err(invalid_visa_resource(address));
        }

        let interface =
            VisaInterface::parse(parts[0]).ok_or_else(|| invalid_visa_resource(address))?;
        let resource_class = parts
            .last()
            .map(|value| value.trim().to_ascii_uppercase())
            .ok_or_else(|| invalid_visa_resource(address))?;
        if !matches!(resource_class.as_str(), "INSTR" | "SOCKET") {
            return Err(invalid_visa_resource(address));
        }
        validate_visa_resource_shape(interface, &parts, &resource_class, address)?;

        Ok(Self {
            raw: trimmed.to_owned(),
            interface,
            resource_class,
        })
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn interface(&self) -> VisaInterface {
        self.interface
    }

    pub fn resource_class(&self) -> &str {
        &self.resource_class
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisaTransportAdapter {
    endpoint: InstrumentTransportEndpoint,
    timeout_policy: TransportTimeoutPolicy,
    resource: VisaResourceAddress,
    last_exchange_attempt_count: u16,
}

impl VisaTransportAdapter {
    pub fn new(
        endpoint: InstrumentTransportEndpoint,
        timeout_policy: TransportTimeoutPolicy,
    ) -> Result<Self, DomainError> {
        validate_adapter_endpoint(&endpoint, InstrumentTransport::Visa)?;
        let resource = VisaResourceAddress::parse(endpoint.address())?;
        Ok(Self {
            endpoint,
            timeout_policy,
            resource,
            last_exchange_attempt_count: 0,
        })
    }

    pub fn timeout_policy(&self) -> TransportTimeoutPolicy {
        self.timeout_policy
    }

    pub fn resource(&self) -> &VisaResourceAddress {
        &self.resource
    }
}

impl InstrumentTransportAdapter for VisaTransportAdapter {
    fn endpoint(&self) -> &InstrumentTransportEndpoint {
        &self.endpoint
    }

    fn exchange(&mut self, command: &InstrumentCommand) -> Result<InstrumentResponse, DomainError> {
        self.last_exchange_attempt_count = 0;
        validate_command_transport(self.endpoint(), command)?;
        if self.resource.interface() == VisaInterface::TcpIp {
            let (result, attempt_count) =
                exchange_tcp_ip(self.endpoint(), self.timeout_policy(), command);
            self.last_exchange_attempt_count = attempt_count;
            result
        } else {
            self.last_exchange_attempt_count = 1;
            Err(external_exchange_unavailable(self.endpoint()))
        }
    }

    fn last_exchange_attempt_count(&self) -> u16 {
        self.last_exchange_attempt_count
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TcpIpTransportAdapter {
    endpoint: InstrumentTransportEndpoint,
    timeout_policy: TransportTimeoutPolicy,
    last_exchange_attempt_count: u16,
}

impl TcpIpTransportAdapter {
    pub fn new(
        endpoint: InstrumentTransportEndpoint,
        timeout_policy: TransportTimeoutPolicy,
    ) -> Result<Self, DomainError> {
        validate_adapter_endpoint(&endpoint, InstrumentTransport::TcpIp)?;
        Ok(Self {
            endpoint,
            timeout_policy,
            last_exchange_attempt_count: 0,
        })
    }

    pub fn timeout_policy(&self) -> TransportTimeoutPolicy {
        self.timeout_policy
    }
}

impl InstrumentTransportAdapter for TcpIpTransportAdapter {
    fn endpoint(&self) -> &InstrumentTransportEndpoint {
        &self.endpoint
    }

    fn exchange(&mut self, command: &InstrumentCommand) -> Result<InstrumentResponse, DomainError> {
        self.last_exchange_attempt_count = 0;
        validate_command_transport(self.endpoint(), command)?;
        let (result, attempt_count) =
            exchange_tcp_ip(self.endpoint(), self.timeout_policy(), command);
        self.last_exchange_attempt_count = attempt_count;
        result
    }

    fn last_exchange_attempt_count(&self) -> u16 {
        self.last_exchange_attempt_count
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerialTransportAdapter {
    endpoint: InstrumentTransportEndpoint,
    timeout_policy: TransportTimeoutPolicy,
    settings: SerialEndpointSettings,
}

impl SerialTransportAdapter {
    pub fn new(
        endpoint: InstrumentTransportEndpoint,
        timeout_policy: TransportTimeoutPolicy,
    ) -> Result<Self, DomainError> {
        validate_adapter_endpoint(&endpoint, InstrumentTransport::Serial)?;
        let settings = SerialEndpointSettings::parse(endpoint.address())?;
        Ok(Self {
            endpoint,
            timeout_policy,
            settings,
        })
    }

    pub fn timeout_policy(&self) -> TransportTimeoutPolicy {
        self.timeout_policy
    }

    pub fn settings(&self) -> &SerialEndpointSettings {
        &self.settings
    }
}

impl InstrumentTransportAdapter for SerialTransportAdapter {
    fn endpoint(&self) -> &InstrumentTransportEndpoint {
        &self.endpoint
    }

    fn exchange(&mut self, command: &InstrumentCommand) -> Result<InstrumentResponse, DomainError> {
        validate_command_transport(self.endpoint(), command)?;
        Err(external_exchange_unavailable(self.endpoint()))
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
        validate_command_transport(self.endpoint(), command)?;

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
    exchange_attempts: u16,
}

impl InstrumentObservation {
    fn new(
        sequence: u64,
        command: InstrumentCommand,
        response: InstrumentResponse,
        success: bool,
        exchange_attempts: u16,
    ) -> Self {
        Self {
            sequence,
            command,
            response,
            success,
            exchange_attempts,
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

    pub fn exchange_attempts(&self) -> u16 {
        self.exchange_attempts
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
        let exchange_attempts = self.adapter.last_exchange_attempt_count();
        let observation = InstrumentObservation::new(
            self.next_sequence,
            command,
            response,
            true,
            exchange_attempts,
        );
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
        let observation =
            InstrumentObservation::new(self.next_sequence, command, response, true, 1);
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

fn validate_adapter_endpoint(
    endpoint: &InstrumentTransportEndpoint,
    expected: InstrumentTransport,
) -> Result<(), DomainError> {
    if endpoint.transport() != expected {
        return Err(DomainError::TransportAdapterMismatch {
            expected: expected.as_str().to_owned(),
            actual: endpoint.transport().as_str().to_owned(),
        });
    }

    Ok(())
}

fn validate_command_transport(
    endpoint: &InstrumentTransportEndpoint,
    command: &InstrumentCommand,
) -> Result<(), DomainError> {
    if command.transport() != endpoint.transport() {
        return Err(DomainError::TransportAdapterMismatch {
            expected: endpoint.transport().as_str().to_owned(),
            actual: command.transport().as_str().to_owned(),
        });
    }

    Ok(())
}

fn parse_serial_framing(
    framing: &str,
    original_address: &str,
) -> Result<(u8, SerialParity, SerialStopBits), DomainError> {
    let mut chars = framing.chars();
    let data_bits = chars
        .next()
        .and_then(|value| value.to_digit(10))
        .map(|value| value as u8)
        .ok_or_else(|| invalid_serial_endpoint(original_address))?;
    let parity = chars
        .next()
        .and_then(SerialParity::parse)
        .ok_or_else(|| invalid_serial_endpoint(original_address))?;
    let stop_bits = chars
        .next()
        .and_then(SerialStopBits::parse)
        .ok_or_else(|| invalid_serial_endpoint(original_address))?;

    if chars.next().is_some() || !(5..=8).contains(&data_bits) {
        return Err(invalid_serial_endpoint(original_address));
    }

    Ok((data_bits, parity, stop_bits))
}

fn is_valid_serial_port_name(port: &str) -> bool {
    if port.is_empty()
        || port.chars().any(char::is_whitespace)
        || port.contains("::")
        || is_reserved_transport_prefix(port)
    {
        return false;
    }

    port.chars()
        .all(|value| value.is_ascii_alphanumeric() || matches!(value, '/' | '\\' | '.' | '_' | '-'))
}

fn is_reserved_transport_prefix(port: &str) -> bool {
    let uppercase = port.to_ascii_uppercase();
    ["TCPIP", "GPIB", "USB", "ASRL"]
        .iter()
        .any(|prefix| uppercase.starts_with(prefix))
}

fn visa_prefix_matches(prefix: &str, expected: &str, require_index: bool) -> bool {
    let Some(index) = prefix.strip_prefix(expected) else {
        return false;
    };

    (!require_index || !index.is_empty()) && index.chars().all(|value| value.is_ascii_digit())
}

fn validate_visa_resource_shape(
    interface: VisaInterface,
    parts: &[&str],
    resource_class: &str,
    original_address: &str,
) -> Result<(), DomainError> {
    match (interface, resource_class) {
        (VisaInterface::TcpIp, "SOCKET") => {
            if parts.len() == 4 && parse_tcp_port(parts[2]).is_some() {
                Ok(())
            } else {
                Err(invalid_visa_resource(original_address))
            }
        }
        (_, "SOCKET") => Err(invalid_visa_resource(original_address)),
        (VisaInterface::TcpIp, "INSTR") => {
            if matches!(parts.len(), 3 | 4) {
                Ok(())
            } else {
                Err(invalid_visa_resource(original_address))
            }
        }
        (VisaInterface::Usb, "INSTR") => {
            if matches!(parts.len(), 5 | 6) {
                Ok(())
            } else {
                Err(invalid_visa_resource(original_address))
            }
        }
        (VisaInterface::Gpib, "INSTR") => validate_gpib_resource_parts(parts, original_address),
        (VisaInterface::Serial, "INSTR") => {
            if parts.len() == 2 {
                Ok(())
            } else {
                Err(invalid_visa_resource(original_address))
            }
        }
        _ => Err(invalid_visa_resource(original_address)),
    }
}

fn validate_gpib_resource_parts(parts: &[&str], original_address: &str) -> Result<(), DomainError> {
    if !matches!(parts.len(), 3 | 4) || parse_gpib_address(parts[1]).is_none() {
        return Err(invalid_visa_resource(original_address));
    }
    if parts.len() == 4 && parse_gpib_address(parts[2]).is_none() {
        return Err(invalid_visa_resource(original_address));
    }

    Ok(())
}

fn parse_gpib_address(value: &str) -> Option<u8> {
    value
        .trim()
        .parse::<u8>()
        .ok()
        .filter(|address| *address <= MAX_GPIB_ADDRESS)
}

fn invalid_serial_endpoint(address: &str) -> DomainError {
    DomainError::InvalidSerialEndpointAddress(address.to_owned())
}

fn invalid_visa_resource(address: &str) -> DomainError {
    DomainError::InvalidVisaResourceAddress(address.to_owned())
}

fn external_exchange_unavailable(endpoint: &InstrumentTransportEndpoint) -> DomainError {
    DomainError::ExternalTransportExchangeUnavailable {
        transport: endpoint.transport().as_str().to_owned(),
        address: endpoint.address().to_owned(),
    }
}

fn exchange_tcp_ip(
    endpoint: &InstrumentTransportEndpoint,
    timeout_policy: TransportTimeoutPolicy,
    command: &InstrumentCommand,
) -> (Result<InstrumentResponse, DomainError>, u16) {
    let target = match tcp_socket_target(endpoint.address()) {
        Ok(target) => target,
        Err(error) => return (Err(error), 0),
    };
    let connect_timeout = Duration::from_millis(u64::from(timeout_policy.connect_timeout_ms()));
    let response_timeout = Duration::from_millis(u64::from(timeout_policy.response_timeout_ms()));
    let attempts = u16::from(timeout_policy.max_retries()) + 1;
    let mut attempt_count = 0;

    for _ in 0..attempts {
        attempt_count += 1;
        if let Ok(mut stream) = connect_tcp_stream(&target, connect_timeout) {
            if stream.set_read_timeout(Some(response_timeout)).is_err() {
                return (Err(external_exchange_unavailable(endpoint)), attempt_count);
            }
            if stream.set_write_timeout(Some(response_timeout)).is_err() {
                return (Err(external_exchange_unavailable(endpoint)), attempt_count);
            }

            let outbound = format!("{}\n", command.message().as_str());
            if stream.write_all(outbound.as_bytes()).is_err() {
                return (Err(external_exchange_unavailable(endpoint)), attempt_count);
            }
            if stream.flush().is_err() {
                return (Err(external_exchange_unavailable(endpoint)), attempt_count);
            }

            let response = match read_tcp_response(&mut stream) {
                Ok(response) => response,
                Err(_) => return (Err(external_exchange_unavailable(endpoint)), attempt_count),
            };
            return (Ok(InstrumentResponse::received(response)), attempt_count);
        }
    }

    (Err(external_exchange_unavailable(endpoint)), attempt_count)
}

fn connect_tcp_stream(target: &str, timeout: Duration) -> std::io::Result<TcpStream> {
    let mut last_error = None;

    for address in target.to_socket_addrs()? {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "no socket address")
    }))
}

fn read_tcp_response(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut response = Vec::new();
    let mut buffer = [0_u8; 256];

    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        response.extend_from_slice(&buffer[..read]);
        if response.contains(&b'\n') {
            break;
        }
    }

    Ok(String::from_utf8_lossy(&response).trim().to_owned())
}

pub(crate) fn tcp_socket_target(address: &str) -> Result<String, DomainError> {
    let trimmed = address.trim();
    if trimmed.is_empty() {
        return Err(DomainError::EmptyTransportEndpointAddress);
    }

    if let Some(target) = visa_tcp_socket_target(trimmed)? {
        return Ok(target);
    }

    if trimmed.contains(':') {
        return Ok(trimmed.to_owned());
    }

    Ok(format!("{trimmed}:{DEFAULT_SCPI_TCP_PORT}"))
}

fn visa_tcp_socket_target(address: &str) -> Result<Option<String>, DomainError> {
    let parts: Vec<&str> = address.split("::").collect();
    let Some(interface) = parts.first().map(|part| part.trim()) else {
        return Ok(None);
    };
    if !interface.to_ascii_uppercase().starts_with("TCPIP") || parts.len() < 2 {
        return Ok(None);
    }

    let host = parts[1].trim();
    if host.is_empty() {
        return Err(invalid_visa_resource(address));
    }

    let explicit_port = match parts.as_slice() {
        [_, _] => None,
        [_, _, value] if value.trim().eq_ignore_ascii_case("INSTR") => None,
        [_, _, port] => Some(parse_tcp_port(port).ok_or_else(|| invalid_visa_resource(address))?),
        [_, _, port, resource_class] if resource_class.trim().eq_ignore_ascii_case("SOCKET") => {
            Some(parse_tcp_port(port).ok_or_else(|| invalid_visa_resource(address))?)
        }
        [_, _, _, resource_class] if resource_class.trim().eq_ignore_ascii_case("INSTR") => None,
        _ => return Err(invalid_visa_resource(address)),
    };
    Ok(Some(format!(
        "{}:{}",
        host,
        explicit_port.unwrap_or(DEFAULT_SCPI_TCP_PORT)
    )))
}

fn deterministic_response(message: &InstrumentCommandMessage) -> InstrumentResponse {
    if message.as_str().ends_with('?') {
        InstrumentResponse::simulated(format!("SIM:{}=0", message.as_str()))
    } else {
        InstrumentResponse::simulated(format!("OK:{}", message.as_str()))
    }
}

fn parse_tcp_port(value: &str) -> Option<u16> {
    value.trim().parse::<u16>().ok().filter(|port| *port > 0)
}
