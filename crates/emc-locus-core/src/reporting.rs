use crate::{
    datasets::{DatasetChecksum, DatasetFileReference},
    identifiers::{AuditActor, ProjectCode},
    quality::ExecutionMode,
    DomainError,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportNumber(String);

impl ReportNumber {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyReportNumber);
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportRevision(String);

impl ReportRevision {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyReportRevision);
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportStatus {
    Draft,
    TechnicalReview,
    TechnicallyReviewed,
    Approved,
    Issued,
    Voided,
}

impl ReportStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::TechnicalReview => "technical_review",
            Self::TechnicallyReviewed => "technically_reviewed",
            Self::Approved => "approved",
            Self::Issued => "issued",
            Self::Voided => "voided",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportPackage {
    project: ProjectCode,
    number: ReportNumber,
    revision: ReportRevision,
    mode: ExecutionMode,
    status: ReportStatus,
    reviewed_by: Option<AuditActor>,
    approved_by: Option<AuditActor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportExportFormat {
    Pdf,
    Docx,
    Zip,
    Json,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportExportBundle {
    project: ProjectCode,
    number: ReportNumber,
    revision: ReportRevision,
    format: ReportExportFormat,
    file_reference: DatasetFileReference,
    checksum: DatasetChecksum,
    reviewed_by: Option<AuditActor>,
    approved_by: Option<AuditActor>,
}

impl ReportExportBundle {
    pub fn from_issued_report(
        report: &ReportPackage,
        format: ReportExportFormat,
        file_reference: DatasetFileReference,
        checksum: DatasetChecksum,
    ) -> Result<Self, DomainError> {
        if report.status() != ReportStatus::Issued {
            return Err(DomainError::ReportMustBeIssuedBeforeExport);
        }

        Ok(Self {
            project: report.project().clone(),
            number: report.number().clone(),
            revision: report.revision().clone(),
            format,
            file_reference,
            checksum,
            reviewed_by: report.reviewed_by().cloned(),
            approved_by: report.approved_by().cloned(),
        })
    }

    pub fn project(&self) -> &ProjectCode {
        &self.project
    }

    pub fn number(&self) -> &ReportNumber {
        &self.number
    }

    pub fn revision(&self) -> &ReportRevision {
        &self.revision
    }

    pub fn format(&self) -> ReportExportFormat {
        self.format
    }

    pub fn file_reference(&self) -> &DatasetFileReference {
        &self.file_reference
    }

    pub fn checksum(&self) -> &DatasetChecksum {
        &self.checksum
    }

    pub fn reviewed_by(&self) -> Option<&AuditActor> {
        self.reviewed_by.as_ref()
    }

    pub fn approved_by(&self) -> Option<&AuditActor> {
        self.approved_by.as_ref()
    }
}

impl ReportPackage {
    pub fn new(
        project: ProjectCode,
        number: ReportNumber,
        revision: ReportRevision,
        mode: ExecutionMode,
    ) -> Self {
        Self {
            project,
            number,
            revision,
            mode,
            status: ReportStatus::Draft,
            reviewed_by: None,
            approved_by: None,
        }
    }

    pub fn project(&self) -> &ProjectCode {
        &self.project
    }

    pub fn number(&self) -> &ReportNumber {
        &self.number
    }

    pub fn revision(&self) -> &ReportRevision {
        &self.revision
    }

    pub fn mode(&self) -> ExecutionMode {
        self.mode
    }

    pub fn status(&self) -> ReportStatus {
        self.status
    }

    pub fn reviewed_by(&self) -> Option<&AuditActor> {
        self.reviewed_by.as_ref()
    }

    pub fn approved_by(&self) -> Option<&AuditActor> {
        self.approved_by.as_ref()
    }

    pub fn submit_for_technical_review(&mut self) -> Result<(), DomainError> {
        self.transition(ReportStatus::Draft, ReportStatus::TechnicalReview)
    }

    pub fn complete_technical_review(&mut self, reviewer: AuditActor) -> Result<(), DomainError> {
        self.transition(
            ReportStatus::TechnicalReview,
            ReportStatus::TechnicallyReviewed,
        )?;
        self.reviewed_by = Some(reviewer);
        Ok(())
    }

    pub fn approve(&mut self, approver: AuditActor) -> Result<(), DomainError> {
        if self.mode.constraint_profile().report_approval_required()
            && self.status != ReportStatus::TechnicallyReviewed
        {
            return Err(DomainError::ReportTechnicalReviewRequired);
        }

        if matches!(self.status, ReportStatus::Issued | ReportStatus::Voided) {
            return Err(DomainError::InvalidReportTransition {
                from: self.status.as_str().to_owned(),
                to: ReportStatus::Approved.as_str().to_owned(),
            });
        }

        self.status = ReportStatus::Approved;
        self.approved_by = Some(approver);
        Ok(())
    }

    pub fn issue(&mut self) -> Result<(), DomainError> {
        if self.mode.constraint_profile().report_approval_required()
            && self.status != ReportStatus::Approved
        {
            return Err(DomainError::ReportApprovalRequired);
        }

        if matches!(self.status, ReportStatus::Issued | ReportStatus::Voided) {
            return Err(DomainError::InvalidReportTransition {
                from: self.status.as_str().to_owned(),
                to: ReportStatus::Issued.as_str().to_owned(),
            });
        }

        self.status = ReportStatus::Issued;
        Ok(())
    }

    fn transition(&mut self, from: ReportStatus, to: ReportStatus) -> Result<(), DomainError> {
        if self.status != from {
            return Err(DomainError::InvalidReportTransition {
                from: self.status.as_str().to_owned(),
                to: to.as_str().to_owned(),
            });
        }

        self.status = to;
        Ok(())
    }
}
