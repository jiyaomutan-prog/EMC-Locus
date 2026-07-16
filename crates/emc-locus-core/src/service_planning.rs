use crate::ProjectCode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceScheduleStatus {
    Planned,
    Confirmed,
    InProgress,
    Completed,
    Cancelled,
}

impl ServiceScheduleStatus {
    pub fn parse(value: &str) -> Result<Self, PlanningValidationIssue> {
        match value.trim() {
            "planned" => Ok(Self::Planned),
            "confirmed" => Ok(Self::Confirmed),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(PlanningValidationIssue::new(
                "unknown_schedule_status",
                "status",
                "unknown service schedule status",
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Confirmed => "confirmed",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }

    pub fn can_reschedule(self) -> bool {
        matches!(self, Self::Planned | Self::Confirmed)
    }

    pub fn allowed_targets(self) -> &'static [Self] {
        match self {
            Self::Planned => &[Self::Confirmed, Self::Cancelled],
            Self::Confirmed => &[Self::InProgress, Self::Cancelled],
            Self::InProgress => &[Self::Completed, Self::Cancelled],
            Self::Completed | Self::Cancelled => &[],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleResourceConflictKind {
    Operator,
    UnresolvedLocationIdentity,
    Location,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningValidationIssue {
    pub code: String,
    pub field: String,
    pub message: String,
}

impl PlanningValidationIssue {
    fn new(code: &str, field: &str, message: &str) -> Self {
        Self {
            code: code.to_owned(),
            field: field.to_owned(),
            message: message.to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScheduleLocalDateTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    canonical: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScheduleLocalDate {
    year: u16,
    month: u8,
    day: u8,
    canonical: String,
}

impl ScheduleLocalDate {
    fn parse(value: &str, field: &str) -> Result<Self, PlanningValidationIssue> {
        let value = value.trim();
        let bytes = value.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(invalid_date(field));
        }
        let year = parse_digits(bytes, 0, 4).ok_or_else(|| invalid_date(field))? as u16;
        let month = parse_digits(bytes, 5, 2).ok_or_else(|| invalid_date(field))? as u8;
        let day = parse_digits(bytes, 8, 2).ok_or_else(|| invalid_date(field))? as u8;
        if year < 2000 || !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month)
        {
            return Err(invalid_date(field));
        }
        Ok(Self {
            year,
            month,
            day,
            canonical: value.to_owned(),
        })
    }

    fn checked_add_days(&self, count: u8) -> Option<Self> {
        let (mut year, mut month, mut day) = (self.year, self.month, self.day);
        for _ in 0..count {
            if day < days_in_month(year, month) {
                day += 1;
            } else if month < 12 {
                month += 1;
                day = 1;
            } else {
                year = year.checked_add(1)?;
                if year > 9999 {
                    return None;
                }
                month = 1;
                day = 1;
            }
        }
        Some(Self {
            year,
            month,
            day,
            canonical: format!("{year:04}-{month:02}-{day:02}"),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceScheduleWeek {
    start: ScheduleLocalDate,
    end: ScheduleLocalDate,
    next_start: ScheduleLocalDate,
}

impl ServiceScheduleWeek {
    pub fn parse(week_start: &str) -> Result<Self, PlanningValidationIssue> {
        let start = ScheduleLocalDate::parse(week_start, "week_start")?;
        if weekday(start.year, start.month, start.day) != 1 {
            return Err(PlanningValidationIssue::new(
                "schedule_week_must_start_on_monday",
                "week_start",
                "a laboratory schedule week must start on a Monday",
            ));
        }
        let end = start.checked_add_days(4).ok_or_else(|| {
            PlanningValidationIssue::new(
                "schedule_week_out_of_range",
                "week_start",
                "the requested laboratory schedule week is outside the supported date range",
            )
        })?;
        let next_start = start.checked_add_days(7).ok_or_else(|| {
            PlanningValidationIssue::new(
                "schedule_week_out_of_range",
                "week_start",
                "the requested laboratory schedule week is outside the supported date range",
            )
        })?;
        Ok(Self {
            start,
            end,
            next_start,
        })
    }

    pub fn start_date(&self) -> &str {
        &self.start.canonical
    }

    pub fn end_date(&self) -> &str {
        &self.end.canonical
    }

    pub fn query_start_at(&self) -> String {
        format!("{}T00:00", self.start.canonical)
    }

    pub fn query_end_at_exclusive(&self) -> String {
        format!("{}T00:00", self.next_start.canonical)
    }
}

impl ScheduleLocalDateTime {
    pub fn parse(value: &str, field: &str) -> Result<Self, PlanningValidationIssue> {
        let value = value.trim();
        let bytes = value.as_bytes();
        if bytes.len() != 16
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || bytes[10] != b'T'
            || bytes[13] != b':'
        {
            return Err(invalid_datetime(field));
        }
        let year = parse_digits(bytes, 0, 4).ok_or_else(|| invalid_datetime(field))? as u16;
        let month = parse_digits(bytes, 5, 2).ok_or_else(|| invalid_datetime(field))? as u8;
        let day = parse_digits(bytes, 8, 2).ok_or_else(|| invalid_datetime(field))? as u8;
        let hour = parse_digits(bytes, 11, 2).ok_or_else(|| invalid_datetime(field))? as u8;
        let minute = parse_digits(bytes, 14, 2).ok_or_else(|| invalid_datetime(field))? as u8;
        if year < 2000
            || !(1..=12).contains(&month)
            || day == 0
            || day > days_in_month(year, month)
            || hour > 23
            || minute > 59
        {
            return Err(invalid_datetime(field));
        }
        Ok(Self {
            year,
            month,
            day,
            hour,
            minute,
            canonical: value.to_owned(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.canonical
    }

    pub fn same_date(&self, other: &Self) -> bool {
        (self.year, self.month, self.day) == (other.year, other.month, other.day)
    }

    pub fn is_business_day(&self) -> bool {
        !matches!(weekday(self.year, self.month, self.day), 0 | 6)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceScheduleItemInput {
    pub item_code: String,
    pub project_code: ProjectCode,
    pub title: String,
    pub planned_start_at: String,
    pub planned_end_at: String,
    pub assigned_operator: String,
    pub laboratory_location_id: Option<String>,
    pub laboratory_location_label: String,
    pub equipment_under_test: String,
    pub test_category_code: Option<String>,
    pub test_method_code: Option<String>,
    pub status: ServiceScheduleStatus,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceScheduleRescheduleInput {
    pub planned_start_at: String,
    pub planned_end_at: String,
    pub assigned_operator: String,
    pub laboratory_location_id: String,
    pub laboratory_location_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceScheduleItem {
    item_code: String,
    project_code: ProjectCode,
    title: String,
    planned_start_at: ScheduleLocalDateTime,
    planned_end_at: ScheduleLocalDateTime,
    assigned_operator: String,
    laboratory_location_id: Option<String>,
    laboratory_location_label: String,
    equipment_under_test: String,
    test_category_code: Option<String>,
    test_method_code: Option<String>,
    status: ServiceScheduleStatus,
    notes: String,
}

impl ServiceScheduleItem {
    pub fn create(input: ServiceScheduleItemInput) -> Result<Self, PlanningValidationIssue> {
        if input.status != ServiceScheduleStatus::Planned {
            return Err(PlanningValidationIssue::new(
                "schedule_initial_status_must_be_planned",
                "status",
                "a new service schedule item must start as planned",
            ));
        }
        if input.laboratory_location_id.is_none() {
            return Err(PlanningValidationIssue::new(
                "schedule_location_identity_required",
                "laboratory_location_id",
                "a new schedule item requires a stable laboratory location identifier",
            ));
        }
        Self::restore(input)
    }

    pub fn restore(input: ServiceScheduleItemInput) -> Result<Self, PlanningValidationIssue> {
        let item_code = require_identifier(input.item_code, "item_code")?;
        let title = require_text(input.title, "title")?;
        let assigned_operator = require_text(input.assigned_operator, "assigned_operator")?;
        let laboratory_location_id = input
            .laboratory_location_id
            .map(|value| require_identifier(value, "laboratory_location_id"))
            .transpose()?;
        let laboratory_location_label =
            require_text(input.laboratory_location_label, "laboratory_location_label")?;
        let equipment_under_test =
            require_text(input.equipment_under_test, "equipment_under_test")?;
        let planned_start_at =
            ScheduleLocalDateTime::parse(&input.planned_start_at, "planned_start_at")?;
        let planned_end_at = ScheduleLocalDateTime::parse(&input.planned_end_at, "planned_end_at")?;
        if !planned_start_at.same_date(&planned_end_at) {
            return Err(PlanningValidationIssue::new(
                "schedule_spans_multiple_days",
                "planned_end_at",
                "a service schedule item must stay within one day",
            ));
        }
        if !planned_start_at.is_business_day() {
            return Err(PlanningValidationIssue::new(
                "schedule_outside_business_day",
                "planned_start_at",
                "a service schedule item must be planned on a business day",
            ));
        }
        if planned_end_at <= planned_start_at {
            return Err(PlanningValidationIssue::new(
                "schedule_end_not_after_start",
                "planned_end_at",
                "the planned end must be after the planned start",
            ));
        }
        Ok(Self {
            item_code,
            project_code: input.project_code,
            title,
            planned_start_at,
            planned_end_at,
            assigned_operator,
            laboratory_location_id,
            laboratory_location_label,
            equipment_under_test,
            test_category_code: optional_text(input.test_category_code),
            test_method_code: optional_text(input.test_method_code),
            status: input.status,
            notes: input
                .notes
                .map_or_else(String::new, |value| value.trim().to_owned()),
        })
    }

    pub fn transition_to(
        &mut self,
        target: ServiceScheduleStatus,
    ) -> Result<(), PlanningValidationIssue> {
        if !self.status.allowed_targets().contains(&target) {
            return Err(PlanningValidationIssue::new(
                "invalid_schedule_status_transition",
                "status",
                "the requested service schedule status transition is not allowed",
            ));
        }
        self.status = target;
        Ok(())
    }

    pub fn rescheduled(
        &self,
        input: ServiceScheduleRescheduleInput,
    ) -> Result<Self, PlanningValidationIssue> {
        if !self.status.can_reschedule() {
            return Err(PlanningValidationIssue::new(
                "schedule_status_not_reschedulable",
                "status",
                "a service schedule item can only be moved while planned or confirmed",
            ));
        }
        Self::restore(ServiceScheduleItemInput {
            item_code: self.item_code.clone(),
            project_code: self.project_code.clone(),
            title: self.title.clone(),
            planned_start_at: input.planned_start_at,
            planned_end_at: input.planned_end_at,
            assigned_operator: input.assigned_operator,
            laboratory_location_id: Some(input.laboratory_location_id),
            laboratory_location_label: input.laboratory_location_label,
            equipment_under_test: self.equipment_under_test.clone(),
            test_category_code: self.test_category_code.clone(),
            test_method_code: self.test_method_code.clone(),
            status: self.status,
            notes: Some(self.notes.clone()),
        })
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.planned_start_at < other.planned_end_at && self.planned_end_at > other.planned_start_at
    }

    pub fn resource_conflict(&self, other: &Self) -> Option<ScheduleResourceConflictKind> {
        if self.status.is_terminal() || other.status.is_terminal() || !self.overlaps(other) {
            return None;
        }
        if self.assigned_operator == other.assigned_operator {
            return Some(ScheduleResourceConflictKind::Operator);
        }
        if self.laboratory_location_id.is_none() || other.laboratory_location_id.is_none() {
            return Some(ScheduleResourceConflictKind::UnresolvedLocationIdentity);
        }
        if self.laboratory_location_id.is_some()
            && self.laboratory_location_id == other.laboratory_location_id
        {
            return Some(ScheduleResourceConflictKind::Location);
        }
        None
    }

    pub fn item_code(&self) -> &str {
        &self.item_code
    }

    pub fn project_code(&self) -> &ProjectCode {
        &self.project_code
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn planned_start_at(&self) -> &str {
        self.planned_start_at.as_str()
    }

    pub fn planned_end_at(&self) -> &str {
        self.planned_end_at.as_str()
    }

    pub fn assigned_operator(&self) -> &str {
        &self.assigned_operator
    }

    pub fn laboratory_location_id(&self) -> Option<&str> {
        self.laboratory_location_id.as_deref()
    }

    pub fn laboratory_location_label(&self) -> &str {
        &self.laboratory_location_label
    }

    pub fn equipment_under_test(&self) -> &str {
        &self.equipment_under_test
    }

    pub fn test_category_code(&self) -> Option<&str> {
        self.test_category_code.as_deref()
    }

    pub fn test_method_code(&self) -> Option<&str> {
        self.test_method_code.as_deref()
    }

    pub fn status(&self) -> ServiceScheduleStatus {
        self.status
    }

    pub fn notes(&self) -> &str {
        &self.notes
    }
}

fn require_identifier(value: String, field: &str) -> Result<String, PlanningValidationIssue> {
    let value = require_text(value, field)?;
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
    {
        return Err(PlanningValidationIssue::new(
            "invalid_schedule_identifier",
            field,
            "a schedule identifier may only contain letters, numbers, dots, dashes and underscores",
        ));
    }
    Ok(value)
}

fn require_text(value: String, field: &str) -> Result<String, PlanningValidationIssue> {
    let value = value.trim();
    if value.is_empty() {
        return Err(PlanningValidationIssue::new(
            "planning_required_field_missing",
            field,
            "a required service schedule field is empty",
        ));
    }
    Ok(value.to_owned())
}

fn optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

fn invalid_datetime(field: &str) -> PlanningValidationIssue {
    PlanningValidationIssue::new(
        "invalid_schedule_datetime",
        field,
        "a schedule date-time must use YYYY-MM-DDTHH:MM and contain a valid local date and time",
    )
}

fn invalid_date(field: &str) -> PlanningValidationIssue {
    PlanningValidationIssue::new(
        "invalid_schedule_date",
        field,
        "a schedule date must use YYYY-MM-DD and contain a valid local date",
    )
}

fn parse_digits(bytes: &[u8], start: usize, length: usize) -> Option<u32> {
    bytes
        .get(start..start + length)?
        .iter()
        .try_fold(0_u32, |value, digit| {
            digit
                .is_ascii_digit()
                .then(|| value * 10 + u32::from(*digit - b'0'))
        })
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn weekday(year: u16, month: u8, day: u8) -> u8 {
    const OFFSETS: [u16; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let adjusted_year = if month < 3 { year - 1 } else { year };
    ((adjusted_year + adjusted_year / 4 - adjusted_year / 100
        + adjusted_year / 400
        + OFFSETS[usize::from(month - 1)]
        + u16::from(day))
        % 7) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(
        code: &str,
        start: &str,
        end: &str,
        operator: &str,
        location: &str,
    ) -> ServiceScheduleItem {
        ServiceScheduleItem::create(ServiceScheduleItemInput {
            item_code: code.to_owned(),
            project_code: ProjectCode::parse("CEM-2026-001").unwrap(),
            title: "Émission conduite".to_owned(),
            planned_start_at: start.to_owned(),
            planned_end_at: end.to_owned(),
            assigned_operator: operator.to_owned(),
            laboratory_location_id: Some(format!(
                "LOCATION-{}",
                location.trim().replace(' ', "-").to_ascii_uppercase()
            )),
            laboratory_location_label: location.to_owned(),
            equipment_under_test: "Convertisseur ferroviaire".to_owned(),
            test_category_code: None,
            test_method_code: None,
            status: ServiceScheduleStatus::Planned,
            notes: None,
        })
        .unwrap()
    }

    #[test]
    fn creates_a_normalized_business_day_schedule_item() {
        let item = item(
            " PLAN-001 ",
            "2026-07-15T09:00",
            "2026-07-15T12:00",
            " Alice Martin ",
            " Labo CEM 1 ",
        );

        assert_eq!(item.item_code(), "PLAN-001");
        assert_eq!(item.assigned_operator(), "Alice Martin");
        assert_eq!(item.laboratory_location_id(), Some("LOCATION-LABO-CEM-1"));
        assert_eq!(item.laboratory_location_label(), "Labo CEM 1");
        assert_eq!(item.status(), ServiceScheduleStatus::Planned);
    }

    #[test]
    fn rejects_invalid_or_weekend_schedule_dates() {
        let invalid = ScheduleLocalDateTime::parse("2026-02-30T09:00", "start").unwrap_err();
        assert_eq!(invalid.code, "invalid_schedule_datetime");

        let weekend = ServiceScheduleItem::create(ServiceScheduleItemInput {
            item_code: "PLAN-WEEKEND".to_owned(),
            project_code: ProjectCode::parse("CEM-2026-001").unwrap(),
            title: "Essai".to_owned(),
            planned_start_at: "2026-07-18T09:00".to_owned(),
            planned_end_at: "2026-07-18T10:00".to_owned(),
            assigned_operator: "Alice".to_owned(),
            laboratory_location_id: Some("LOCATION-LABO-1".to_owned()),
            laboratory_location_label: "Labo 1".to_owned(),
            equipment_under_test: "EUT".to_owned(),
            test_category_code: None,
            test_method_code: None,
            status: ServiceScheduleStatus::Planned,
            notes: None,
        })
        .unwrap_err();
        assert_eq!(weekend.code, "schedule_outside_business_day");
    }

    #[test]
    fn rejects_multi_day_and_non_positive_blocks() {
        let mut input = ServiceScheduleItemInput {
            item_code: "PLAN-BAD".to_owned(),
            project_code: ProjectCode::parse("CEM-2026-001").unwrap(),
            title: "Essai".to_owned(),
            planned_start_at: "2026-07-15T09:00".to_owned(),
            planned_end_at: "2026-07-16T10:00".to_owned(),
            assigned_operator: "Alice".to_owned(),
            laboratory_location_id: Some("LOCATION-LABO-1".to_owned()),
            laboratory_location_label: "Labo 1".to_owned(),
            equipment_under_test: "EUT".to_owned(),
            test_category_code: None,
            test_method_code: None,
            status: ServiceScheduleStatus::Planned,
            notes: None,
        };
        assert_eq!(
            ServiceScheduleItem::create(input.clone()).unwrap_err().code,
            "schedule_spans_multiple_days"
        );
        input.planned_end_at = input.planned_start_at.clone();
        assert_eq!(
            ServiceScheduleItem::create(input).unwrap_err().code,
            "schedule_end_not_after_start"
        );
    }

    #[test]
    fn adjacent_blocks_do_not_overlap() {
        let morning = item(
            "PLAN-AM",
            "2026-07-15T09:00",
            "2026-07-15T12:00",
            "Alice",
            "Labo 1",
        );
        let afternoon = item(
            "PLAN-PM",
            "2026-07-15T12:00",
            "2026-07-15T15:00",
            "Alice",
            "Labo 1",
        );

        assert!(!morning.overlaps(&afternoon));
        assert_eq!(morning.resource_conflict(&afternoon), None);
    }

    #[test]
    fn reports_operator_before_location_conflicts() {
        let first = item(
            "PLAN-001",
            "2026-07-15T09:00",
            "2026-07-15T12:00",
            "Alice",
            "Labo 1",
        );
        let same_operator = item(
            "PLAN-002",
            "2026-07-15T10:00",
            "2026-07-15T11:00",
            "Alice",
            "Labo 2",
        );
        let same_location = item(
            "PLAN-003",
            "2026-07-15T10:00",
            "2026-07-15T11:00",
            "Bob",
            "Labo 1",
        );

        assert_eq!(
            first.resource_conflict(&same_operator),
            Some(ScheduleResourceConflictKind::Operator)
        );
        assert_eq!(
            first.resource_conflict(&same_location),
            Some(ScheduleResourceConflictKind::Location)
        );
    }

    #[test]
    fn location_conflicts_use_stable_identity_instead_of_label() {
        let first = item(
            "PLAN-LOCATION-1",
            "2026-07-15T09:00",
            "2026-07-15T12:00",
            "Alice",
            "Ancien nom",
        );
        let mut renamed_same_location = item(
            "PLAN-LOCATION-2",
            "2026-07-15T10:00",
            "2026-07-15T11:00",
            "Bob",
            "Nouveau nom",
        );
        renamed_same_location.laboratory_location_id = first.laboratory_location_id.clone();
        let mut same_label_other_location = renamed_same_location.clone();
        same_label_other_location.item_code = "PLAN-LOCATION-3".to_owned();
        same_label_other_location.laboratory_location_label =
            first.laboratory_location_label.clone();
        same_label_other_location.laboratory_location_id =
            Some("LAB-LOCATION-DIFFERENT".to_owned());

        assert_eq!(
            first.resource_conflict(&renamed_same_location),
            Some(ScheduleResourceConflictKind::Location)
        );
        assert_eq!(first.resource_conflict(&same_label_other_location), None);
    }

    #[test]
    fn unresolved_location_identity_blocks_without_guessing_from_the_label() {
        let candidate = item(
            "PLAN-CANDIDATE",
            "2026-07-15T09:00",
            "2026-07-15T12:00",
            "Bob",
            "Poste CEM 1",
        );
        let historical = ServiceScheduleItem::restore(ServiceScheduleItemInput {
            item_code: "PLAN-HISTORICAL".to_owned(),
            project_code: ProjectCode::parse("CEM-2026-001").unwrap(),
            title: "Essai historique".to_owned(),
            planned_start_at: "2026-07-15T10:00".to_owned(),
            planned_end_at: "2026-07-15T11:00".to_owned(),
            assigned_operator: "Alice".to_owned(),
            laboratory_location_id: None,
            laboratory_location_label: "Poste CEM 1".to_owned(),
            equipment_under_test: "EUT".to_owned(),
            test_category_code: None,
            test_method_code: None,
            status: ServiceScheduleStatus::Confirmed,
            notes: None,
        })
        .unwrap();

        assert_eq!(historical.laboratory_location_id(), None);
        assert_eq!(
            candidate.resource_conflict(&historical),
            Some(ScheduleResourceConflictKind::UnresolvedLocationIdentity)
        );
    }

    #[test]
    fn operator_conflict_has_priority_over_unresolved_location_identity() {
        let candidate = item(
            "PLAN-CANDIDATE",
            "2026-07-15T09:00",
            "2026-07-15T12:00",
            "Alice",
            "Poste CEM 2",
        );
        let historical = ServiceScheduleItem::restore(ServiceScheduleItemInput {
            item_code: "PLAN-HISTORICAL".to_owned(),
            project_code: ProjectCode::parse("CEM-2026-001").unwrap(),
            title: "Essai historique".to_owned(),
            planned_start_at: "2026-07-15T10:00".to_owned(),
            planned_end_at: "2026-07-15T11:00".to_owned(),
            assigned_operator: "Alice".to_owned(),
            laboratory_location_id: None,
            laboratory_location_label: "Ancien libellé".to_owned(),
            equipment_under_test: "EUT".to_owned(),
            test_category_code: None,
            test_method_code: None,
            status: ServiceScheduleStatus::Planned,
            notes: None,
        })
        .unwrap();

        assert_eq!(
            candidate.resource_conflict(&historical),
            Some(ScheduleResourceConflictKind::Operator)
        );
    }

    #[test]
    fn terminal_items_no_longer_reserve_resources() {
        let mut completed = item(
            "PLAN-001",
            "2026-07-15T09:00",
            "2026-07-15T12:00",
            "Alice",
            "Labo 1",
        );
        completed
            .transition_to(ServiceScheduleStatus::Confirmed)
            .unwrap();
        completed
            .transition_to(ServiceScheduleStatus::InProgress)
            .unwrap();
        completed
            .transition_to(ServiceScheduleStatus::Completed)
            .unwrap();
        let replacement = item(
            "PLAN-002",
            "2026-07-15T10:00",
            "2026-07-15T11:00",
            "Alice",
            "Labo 1",
        );

        assert_eq!(completed.resource_conflict(&replacement), None);
    }

    #[test]
    fn enforces_sequential_status_transitions() {
        let mut item = item(
            "PLAN-001",
            "2026-07-15T09:00",
            "2026-07-15T12:00",
            "Alice",
            "Labo 1",
        );
        assert_eq!(
            item.transition_to(ServiceScheduleStatus::Completed)
                .unwrap_err()
                .code,
            "invalid_schedule_status_transition"
        );
        item.transition_to(ServiceScheduleStatus::Confirmed)
            .unwrap();
        item.transition_to(ServiceScheduleStatus::InProgress)
            .unwrap();
        item.transition_to(ServiceScheduleStatus::Completed)
            .unwrap();
        assert!(item.status().is_terminal());
    }

    #[test]
    fn allows_cancellation_from_any_open_state() {
        for status in [
            ServiceScheduleStatus::Planned,
            ServiceScheduleStatus::Confirmed,
            ServiceScheduleStatus::InProgress,
        ] {
            let mut item = item(
                "PLAN-001",
                "2026-07-15T09:00",
                "2026-07-15T12:00",
                "Alice",
                "Labo 1",
            );
            if status != ServiceScheduleStatus::Planned {
                item.transition_to(ServiceScheduleStatus::Confirmed)
                    .unwrap();
            }
            if status == ServiceScheduleStatus::InProgress {
                item.transition_to(ServiceScheduleStatus::InProgress)
                    .unwrap();
            }
            item.transition_to(ServiceScheduleStatus::Cancelled)
                .unwrap();
            assert_eq!(item.status(), ServiceScheduleStatus::Cancelled);
        }
    }

    #[test]
    fn builds_a_monday_to_friday_week_across_month_boundaries() {
        let week = ServiceScheduleWeek::parse("2026-08-31").unwrap();

        assert_eq!(week.start_date(), "2026-08-31");
        assert_eq!(week.end_date(), "2026-09-04");
        assert_eq!(week.query_start_at(), "2026-08-31T00:00");
        assert_eq!(week.query_end_at_exclusive(), "2026-09-07T00:00");
    }

    #[test]
    fn rejects_a_week_that_does_not_start_on_monday() {
        let error = ServiceScheduleWeek::parse("2026-07-15").unwrap_err();

        assert_eq!(error.code, "schedule_week_must_start_on_monday");
        assert_eq!(error.field, "week_start");
    }

    #[test]
    fn reschedules_planned_and_confirmed_items_without_changing_their_state() {
        for status in [
            ServiceScheduleStatus::Planned,
            ServiceScheduleStatus::Confirmed,
        ] {
            let mut current = item(
                "PLAN-001",
                "2026-07-15T09:00",
                "2026-07-15T12:00",
                "Alice",
                "Labo 1",
            );
            if status == ServiceScheduleStatus::Confirmed {
                current
                    .transition_to(ServiceScheduleStatus::Confirmed)
                    .unwrap();
            }

            let moved = current
                .rescheduled(ServiceScheduleRescheduleInput {
                    planned_start_at: "2026-07-16T13:00".to_owned(),
                    planned_end_at: "2026-07-16T16:00".to_owned(),
                    assigned_operator: "Bob".to_owned(),
                    laboratory_location_id: "LOCATION-LABO-2".to_owned(),
                    laboratory_location_label: "Labo 2".to_owned(),
                })
                .unwrap();

            assert_eq!(moved.status(), status);
            assert_eq!(moved.planned_start_at(), "2026-07-16T13:00");
            assert_eq!(moved.assigned_operator(), "Bob");
            assert_eq!(moved.equipment_under_test(), current.equipment_under_test());
        }
    }

    #[test]
    fn rejects_rescheduling_after_a_test_has_started() {
        let mut current = item(
            "PLAN-001",
            "2026-07-15T09:00",
            "2026-07-15T12:00",
            "Alice",
            "Labo 1",
        );
        current
            .transition_to(ServiceScheduleStatus::Confirmed)
            .unwrap();
        current
            .transition_to(ServiceScheduleStatus::InProgress)
            .unwrap();

        let error = current
            .rescheduled(ServiceScheduleRescheduleInput {
                planned_start_at: "2026-07-16T13:00".to_owned(),
                planned_end_at: "2026-07-16T16:00".to_owned(),
                assigned_operator: "Alice".to_owned(),
                laboratory_location_id: "LOCATION-LABO-1".to_owned(),
                laboratory_location_label: "Labo 1".to_owned(),
            })
            .unwrap_err();

        assert_eq!(error.code, "schedule_status_not_reschedulable");
    }
}
