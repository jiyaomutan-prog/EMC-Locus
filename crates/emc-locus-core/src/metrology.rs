use crate::{quality::ExecutionMode, DomainError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MetrologyDate {
    year: u16,
    month: u8,
    day: u8,
}

impl MetrologyDate {
    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, DomainError> {
        let date = Self { year, month, day };

        if year < 1900 || month == 0 || month > 12 || day == 0 || day > date.days_in_month() {
            return Err(DomainError::InvalidMetrologyDate { year, month, day });
        }

        Ok(date)
    }

    pub fn year(&self) -> u16 {
        self.year
    }

    pub fn month(&self) -> u8 {
        self.month
    }

    pub fn day(&self) -> u8 {
        self.day
    }

    pub fn days_until(self, later: Self) -> i32 {
        later.days_since_epoch() - self.days_since_epoch()
    }

    fn days_since_epoch(self) -> i32 {
        let years_before = i32::from(self.year) - 1;
        let leap_days_before_year = years_before / 4 - years_before / 100 + years_before / 400;

        365 * years_before + leap_days_before_year + i32::from(self.day_of_year()) - 1
    }

    fn day_of_year(self) -> u16 {
        let mut total = 0;
        for month in 1..self.month {
            total += Self {
                year: self.year,
                month,
                day: 1,
            }
            .days_in_month() as u16;
        }

        total + u16::from(self.day)
    }

    fn days_in_month(self) -> u8 {
        match self.month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if Self::is_leap_year(self.year) => 29,
            2 => 28,
            _ => 0,
        }
    }

    fn is_leap_year(year: u16) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentCode(String);

impl InstrumentCode {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyInstrumentCode);
        }

        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            return Err(DomainError::InvalidInstrumentCode(trimmed.to_owned()));
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentFamily {
    Receiver,
    SpectrumAnalyzer,
    SignalGenerator,
    PowerAmplifier,
    Antenna,
    Probe,
    CouplingDecouplingNetwork,
    ArtificialMainsNetwork,
    Oscilloscope,
    Daq,
    Multimeter,
    PowerSupply,
    EnvironmentalSensor,
    Fixture,
    SoftwareTool,
    Simulated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentAvailability {
    Available,
    Reserved,
    OutOfService,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationRequirement {
    Required,
    Conditional,
    NotRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationStatus {
    Valid,
    DueSoon,
    Expired,
    Missing,
    NotRequired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentRecord {
    code: InstrumentCode,
    family: InstrumentFamily,
    manufacturer: String,
    model: String,
    serial_number: String,
    availability: InstrumentAvailability,
    calibration_requirement: CalibrationRequirement,
}

impl InstrumentRecord {
    pub fn new(
        code: InstrumentCode,
        family: InstrumentFamily,
        manufacturer: impl Into<String>,
        model: impl Into<String>,
        serial_number: impl Into<String>,
        calibration_requirement: CalibrationRequirement,
    ) -> Result<Self, DomainError> {
        let manufacturer =
            normalized_label(manufacturer, DomainError::EmptyInstrumentManufacturer)?;
        let model = normalized_label(model, DomainError::EmptyInstrumentModel)?;
        let serial_number =
            normalized_label(serial_number, DomainError::EmptyInstrumentSerialNumber)?;

        Ok(Self {
            code,
            family,
            manufacturer,
            model,
            serial_number,
            availability: InstrumentAvailability::Available,
            calibration_requirement,
        })
    }

    pub fn code(&self) -> &InstrumentCode {
        &self.code
    }

    pub fn family(&self) -> InstrumentFamily {
        self.family
    }

    pub fn manufacturer(&self) -> &str {
        &self.manufacturer
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn serial_number(&self) -> &str {
        &self.serial_number
    }

    pub fn availability(&self) -> InstrumentAvailability {
        self.availability
    }

    pub fn calibration_requirement(&self) -> CalibrationRequirement {
        self.calibration_requirement
    }

    pub fn set_availability(&mut self, availability: InstrumentAvailability) {
        self.availability = availability;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibrationRecord {
    instrument: InstrumentCode,
    certificate_reference: String,
    issued_on: MetrologyDate,
    valid_until: MetrologyDate,
    provider: String,
}

impl CalibrationRecord {
    pub fn new(
        instrument: InstrumentCode,
        certificate_reference: impl Into<String>,
        issued_on: MetrologyDate,
        valid_until: MetrologyDate,
        provider: impl Into<String>,
    ) -> Result<Self, DomainError> {
        if valid_until < issued_on {
            return Err(DomainError::InvalidCalibrationPeriod);
        }

        let certificate_reference = normalized_label(
            certificate_reference,
            DomainError::EmptyCalibrationCertificate,
        )?;
        let provider = normalized_label(provider, DomainError::EmptyCalibrationProvider)?;

        Ok(Self {
            instrument,
            certificate_reference,
            issued_on,
            valid_until,
            provider,
        })
    }

    pub fn instrument(&self) -> &InstrumentCode {
        &self.instrument
    }

    pub fn certificate_reference(&self) -> &str {
        &self.certificate_reference
    }

    pub fn issued_on(&self) -> MetrologyDate {
        self.issued_on
    }

    pub fn valid_until(&self) -> MetrologyDate {
        self.valid_until
    }

    pub fn provider(&self) -> &str {
        &self.provider
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipmentIssueKind {
    MissingInstrument,
    OutOfService,
    CalibrationMissing,
    CalibrationExpired,
    CalibrationDueSoon,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EquipmentIssue {
    instrument: InstrumentCode,
    kind: EquipmentIssueKind,
    blocking: bool,
}

impl EquipmentIssue {
    fn new(instrument: InstrumentCode, kind: EquipmentIssueKind, blocking: bool) -> Self {
        Self {
            instrument,
            kind,
            blocking,
        }
    }

    pub fn instrument(&self) -> &InstrumentCode {
        &self.instrument
    }

    pub fn kind(&self) -> EquipmentIssueKind {
        self.kind
    }

    pub fn is_blocking(&self) -> bool {
        self.blocking
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EquipmentReadinessReport {
    mode: ExecutionMode,
    checked_on: MetrologyDate,
    issues: Vec<EquipmentIssue>,
}

impl EquipmentReadinessReport {
    fn new(mode: ExecutionMode, checked_on: MetrologyDate, issues: Vec<EquipmentIssue>) -> Self {
        Self {
            mode,
            checked_on,
            issues,
        }
    }

    pub fn mode(&self) -> ExecutionMode {
        self.mode
    }

    pub fn checked_on(&self) -> MetrologyDate {
        self.checked_on
    }

    pub fn issues(&self) -> &[EquipmentIssue] {
        &self.issues
    }

    pub fn blocking_issues(&self) -> Vec<&EquipmentIssue> {
        self.issues
            .iter()
            .filter(|issue| issue.is_blocking())
            .collect()
    }

    pub fn is_ready(&self) -> bool {
        !self.issues.iter().any(EquipmentIssue::is_blocking)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MetrologyRegistry {
    instruments: Vec<InstrumentRecord>,
    calibration_records: Vec<CalibrationRecord>,
}

impl MetrologyRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn instruments(&self) -> &[InstrumentRecord] {
        &self.instruments
    }

    pub fn calibration_records(&self) -> &[CalibrationRecord] {
        &self.calibration_records
    }

    pub fn register_instrument(&mut self, instrument: InstrumentRecord) -> Result<(), DomainError> {
        if self.instrument(instrument.code()).is_some() {
            return Err(DomainError::DuplicateInstrumentCode(
                instrument.code().as_str().to_owned(),
            ));
        }

        self.instruments.push(instrument);
        Ok(())
    }

    pub fn record_calibration(&mut self, record: CalibrationRecord) -> Result<(), DomainError> {
        if self.instrument(record.instrument()).is_none() {
            return Err(DomainError::UnknownInstrumentCode(
                record.instrument().as_str().to_owned(),
            ));
        }

        self.calibration_records.push(record);
        Ok(())
    }

    pub fn instrument(&self, code: &InstrumentCode) -> Option<&InstrumentRecord> {
        self.instruments
            .iter()
            .find(|instrument| instrument.code() == code)
    }

    pub fn instrument_mut(&mut self, code: &InstrumentCode) -> Option<&mut InstrumentRecord> {
        self.instruments
            .iter_mut()
            .find(|instrument| instrument.code() == code)
    }

    pub fn latest_calibration_for(
        &self,
        instrument: &InstrumentCode,
    ) -> Option<&CalibrationRecord> {
        self.calibration_records
            .iter()
            .filter(|record| record.instrument() == instrument)
            .max_by_key(|record| (record.valid_until(), record.issued_on()))
    }

    pub fn calibration_status(
        &self,
        instrument: &InstrumentCode,
        checked_on: MetrologyDate,
    ) -> Result<CalibrationStatus, DomainError> {
        let instrument_record = self
            .instrument(instrument)
            .ok_or_else(|| DomainError::UnknownInstrumentCode(instrument.as_str().to_owned()))?;

        if instrument_record.calibration_requirement() == CalibrationRequirement::NotRequired {
            return Ok(CalibrationStatus::NotRequired);
        }

        let Some(latest) = self.latest_calibration_for(instrument) else {
            return Ok(CalibrationStatus::Missing);
        };

        let days_until_due = checked_on.days_until(latest.valid_until());

        if days_until_due < 0 {
            Ok(CalibrationStatus::Expired)
        } else if days_until_due <= 30 {
            Ok(CalibrationStatus::DueSoon)
        } else {
            Ok(CalibrationStatus::Valid)
        }
    }

    pub fn assess_equipment_readiness(
        &self,
        requested_instruments: &[InstrumentCode],
        mode: ExecutionMode,
        checked_on: MetrologyDate,
    ) -> EquipmentReadinessReport {
        let profile = mode.constraint_profile();
        let mut issues = Vec::new();

        for requested in requested_instruments {
            let Some(instrument) = self.instrument(requested) else {
                issues.push(EquipmentIssue::new(
                    requested.clone(),
                    EquipmentIssueKind::MissingInstrument,
                    true,
                ));
                continue;
            };

            if instrument.availability() == InstrumentAvailability::OutOfService {
                issues.push(EquipmentIssue::new(
                    requested.clone(),
                    EquipmentIssueKind::OutOfService,
                    true,
                ));
            }

            let calibration_blocks = profile.valid_calibration_required()
                && instrument.calibration_requirement() == CalibrationRequirement::Required;

            match self.calibration_status(requested, checked_on) {
                Ok(CalibrationStatus::Missing) => issues.push(EquipmentIssue::new(
                    requested.clone(),
                    EquipmentIssueKind::CalibrationMissing,
                    calibration_blocks,
                )),
                Ok(CalibrationStatus::Expired) => issues.push(EquipmentIssue::new(
                    requested.clone(),
                    EquipmentIssueKind::CalibrationExpired,
                    calibration_blocks,
                )),
                Ok(CalibrationStatus::DueSoon) => issues.push(EquipmentIssue::new(
                    requested.clone(),
                    EquipmentIssueKind::CalibrationDueSoon,
                    false,
                )),
                Ok(CalibrationStatus::Valid | CalibrationStatus::NotRequired) => {}
                Err(_) => issues.push(EquipmentIssue::new(
                    requested.clone(),
                    EquipmentIssueKind::MissingInstrument,
                    true,
                )),
            }
        }

        EquipmentReadinessReport::new(mode, checked_on, issues)
    }
}

fn normalized_label(
    value: impl Into<String>,
    empty_error: DomainError,
) -> Result<String, DomainError> {
    let value = value.into();
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(empty_error);
    }

    Ok(trimmed.to_owned())
}
