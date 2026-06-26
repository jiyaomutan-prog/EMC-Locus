use super::*;

#[test]
fn project_code_rejects_empty_values() {
    let error = ProjectCode::parse("   ").unwrap_err();
    assert_eq!(error, DomainError::EmptyProjectCode);
}

#[test]
fn project_code_accepts_lab_friendly_identifiers() {
    let code = ProjectCode::parse("CEM-2026_001.A").unwrap();
    assert_eq!(code.as_str(), "CEM-2026_001.A");
}

#[test]
fn audit_actor_rejects_empty_values() {
    let error = AuditActor::parse(" ").unwrap_err();
    assert_eq!(error, DomainError::EmptyAuditActor);
}

#[test]
fn audit_reason_rejects_empty_values() {
    let error = AuditReason::parse("\t").unwrap_err();
    assert_eq!(error, DomainError::EmptyAuditReason);
}

#[test]
fn project_stages_follow_the_campaign_lifecycle() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let mut project = Project::new(code, "Example Customer").unwrap();

    assert_eq!(project.stage(), ProjectStage::Quotation);
    project.advance_to(ProjectStage::ContractReview).unwrap();
    project.advance_to(ProjectStage::TestPlanning).unwrap();
    project.advance_to(ProjectStage::Measuring).unwrap();
    project.advance_to(ProjectStage::TechnicalReview).unwrap();
    project.advance_to(ProjectStage::ReportIssued).unwrap();
    project.advance_to(ProjectStage::Archived).unwrap();
}

#[test]
fn project_stages_reject_skipped_review_points() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let mut project = Project::new(code, "Example Customer").unwrap();

    let error = project.advance_to(ProjectStage::Measuring).unwrap_err();
    assert_eq!(
        error,
        DomainError::InvalidProjectTransition {
            from: ProjectStage::Quotation,
            to: ProjectStage::Measuring,
        }
    );
}

#[test]
fn project_record_opens_with_a_creation_audit_event() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let project = Project::new(code.clone(), "Example Customer").unwrap();
    let actor = AuditActor::parse("quality.manager").unwrap();

    let record = ProjectRecord::open(project, actor.clone());
    let event = &record.audit_events()[0];

    assert_eq!(record.audit_events().len(), 1);
    assert_eq!(event.sequence(), 1);
    assert_eq!(event.actor(), &actor);
    assert_eq!(event.project(), &code);
    assert_eq!(event.action(), &AuditAction::ProjectCreated);
    assert_eq!(event.reason(), None);
}

#[test]
fn project_record_records_stage_transition_audit_events() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let project = Project::new(code.clone(), "Example Customer").unwrap();
    let actor = AuditActor::parse("quality.manager").unwrap();
    let reason = AuditReason::parse("Contract review approved").unwrap();
    let mut record = ProjectRecord::open(project, actor.clone());

    let event = record
        .advance_to(ProjectStage::ContractReview, actor.clone(), reason.clone())
        .unwrap()
        .clone();

    assert_eq!(record.project().stage(), ProjectStage::ContractReview);
    assert_eq!(record.audit_events().len(), 2);
    assert_eq!(event.sequence(), 2);
    assert_eq!(event.actor(), &actor);
    assert_eq!(event.project(), &code);
    assert_eq!(
        event.action(),
        &AuditAction::ProjectStageAdvanced {
            from: ProjectStage::Quotation,
            to: ProjectStage::ContractReview,
        }
    );
    assert_eq!(event.reason(), Some(&reason));
}

#[test]
fn project_record_rejects_skipped_stages_without_audit_side_effects() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let project = Project::new(code, "Example Customer").unwrap();
    let actor = AuditActor::parse("quality.manager").unwrap();
    let reason = AuditReason::parse("Operator tried to skip planning").unwrap();
    let mut record = ProjectRecord::open(project, actor.clone());

    let error = record
        .advance_to(ProjectStage::Measuring, actor, reason)
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::InvalidProjectTransition {
            from: ProjectStage::Quotation,
            to: ProjectStage::Measuring,
        }
    );
    assert_eq!(record.project().stage(), ProjectStage::Quotation);
    assert_eq!(record.audit_events().len(), 1);
}

#[test]
fn contract_review_checklist_starts_with_all_items_missing() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let checklist = ContractReviewChecklist::new(code.clone());

    assert_eq!(checklist.project(), &code);
    assert!(!checklist.is_complete());
    assert_eq!(checklist.missing_items(), baseline_contract_review_items());
}

#[test]
fn contract_review_checklist_does_not_duplicate_completed_items() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let mut checklist = ContractReviewChecklist::new(code);

    checklist.mark_complete(ContractReviewItem::CustomerRequestDefined);
    checklist.mark_complete(ContractReviewItem::CustomerRequestDefined);

    assert_eq!(checklist.completed_items().len(), 1);
}

#[test]
fn contract_review_checklist_can_be_completed() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let mut checklist = ContractReviewChecklist::new(code);

    for item in baseline_contract_review_items() {
        checklist.mark_complete(item);
    }

    assert!(checklist.is_complete());
    assert!(checklist.missing_items().is_empty());
}

#[test]
fn test_planning_requires_complete_contract_review() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let project = Project::new(code.clone(), "Example Customer").unwrap();
    let actor = AuditActor::parse("quality.manager").unwrap();
    let mut record = ProjectRecord::open(project, actor.clone());
    record
        .advance_to(
            ProjectStage::ContractReview,
            actor.clone(),
            AuditReason::parse("Quote accepted").unwrap(),
        )
        .unwrap();
    let checklist = ContractReviewChecklist::new(code);
    let audit_count_before = record.audit_events().len();

    let error = record
        .advance_to_test_planning(
            &checklist,
            actor,
            AuditReason::parse("Planning requested").unwrap(),
            None,
        )
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::IncompleteContractReview {
            missing_items: baseline_contract_review_items(),
        }
    );
    assert_eq!(record.project().stage(), ProjectStage::ContractReview);
    assert_eq!(record.audit_events().len(), audit_count_before);
}

#[test]
fn complete_contract_review_allows_test_planning() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let project = Project::new(code.clone(), "Example Customer").unwrap();
    let actor = AuditActor::parse("quality.manager").unwrap();
    let mut record = ProjectRecord::open(project, actor.clone());
    record
        .advance_to(
            ProjectStage::ContractReview,
            actor.clone(),
            AuditReason::parse("Quote accepted").unwrap(),
        )
        .unwrap();
    let mut checklist = ContractReviewChecklist::new(code.clone());
    for item in baseline_contract_review_items() {
        checklist.mark_complete(item);
    }

    let event = record
        .advance_to_test_planning(
            &checklist,
            actor.clone(),
            AuditReason::parse("Contract review complete").unwrap(),
            None,
        )
        .unwrap()
        .clone();

    assert_eq!(record.project().stage(), ProjectStage::TestPlanning);
    assert_eq!(record.audit_events().len(), 3);
    assert_eq!(
        event.action(),
        &AuditAction::ProjectStageAdvanced {
            from: ProjectStage::ContractReview,
            to: ProjectStage::TestPlanning,
        }
    );
    assert_eq!(event.project(), &code);
    assert_eq!(event.actor(), &actor);
}

#[test]
fn authorized_deviation_allows_incomplete_contract_review_to_reach_planning() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let project = Project::new(code.clone(), "Example Customer").unwrap();
    let actor = AuditActor::parse("quality.manager").unwrap();
    let mut record = ProjectRecord::open(project, actor.clone());
    record
        .advance_to(
            ProjectStage::ContractReview,
            actor.clone(),
            AuditReason::parse("Quote accepted").unwrap(),
        )
        .unwrap();
    let checklist = ContractReviewChecklist::new(code.clone());
    let deviation_reason =
        AuditReason::parse("Quality manager accepted documented planning risk").unwrap();
    let deviation = AuthorizedDeviation::new(actor.clone(), deviation_reason.clone());

    let event = record
        .advance_to_test_planning(
            &checklist,
            actor.clone(),
            AuditReason::parse("Planning authorized with deviation").unwrap(),
            Some(deviation),
        )
        .unwrap()
        .clone();

    assert_eq!(record.project().stage(), ProjectStage::TestPlanning);
    assert_eq!(record.audit_events().len(), 4);

    let deviation_event = &record.audit_events()[2];
    assert_eq!(deviation_event.actor(), &actor);
    assert_eq!(deviation_event.reason(), Some(&deviation_reason));
    assert_eq!(
        deviation_event.action(),
        &AuditAction::ContractReviewDeviationAuthorized {
            missing_items: baseline_contract_review_items(),
        }
    );
    assert_eq!(
        event.action(),
        &AuditAction::ProjectStageAdvanced {
            from: ProjectStage::ContractReview,
            to: ProjectStage::TestPlanning,
        }
    );
}

#[test]
fn contract_review_gate_rejects_checklists_for_another_project() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let other_code = ProjectCode::parse("CEM-2026-002").unwrap();
    let project = Project::new(code.clone(), "Example Customer").unwrap();
    let actor = AuditActor::parse("quality.manager").unwrap();
    let mut record = ProjectRecord::open(project, actor.clone());
    record
        .advance_to(
            ProjectStage::ContractReview,
            actor.clone(),
            AuditReason::parse("Quote accepted").unwrap(),
        )
        .unwrap();
    let checklist = ContractReviewChecklist::new(other_code.clone());

    let error = record
        .advance_to_test_planning(
            &checklist,
            actor,
            AuditReason::parse("Planning requested").unwrap(),
            None,
        )
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::ChecklistProjectMismatch {
            project: code,
            checklist_project: other_code,
        }
    );
}

#[test]
fn contract_review_gate_rejects_invalid_source_stage_before_checklist_checks() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let project = Project::new(code.clone(), "Example Customer").unwrap();
    let actor = AuditActor::parse("quality.manager").unwrap();
    let mut record = ProjectRecord::open(project, actor.clone());
    let checklist = ContractReviewChecklist::new(code);

    let error = record
        .advance_to_test_planning(
            &checklist,
            actor,
            AuditReason::parse("Planning requested").unwrap(),
            None,
        )
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::InvalidProjectTransition {
            from: ProjectStage::Quotation,
            to: ProjectStage::TestPlanning,
        }
    );
    assert_eq!(record.project().stage(), ProjectStage::Quotation);
    assert_eq!(record.audit_events().len(), 1);
}

#[test]
fn accredited_mode_keeps_strict_quality_constraints() {
    let profile = ExecutionMode::Accredited.constraint_profile();

    assert_eq!(profile.mode(), ExecutionMode::Accredited);
    assert!(profile.stage_gates_required());
    assert!(profile.valid_calibration_required());
    assert!(profile.controlled_method_required());
    assert!(profile.report_approval_required());
    assert!(profile.deviations_allowed());
    assert!(!profile.exploratory_steps_allowed());
}

#[test]
fn non_accredited_mode_relaxes_metrology_and_report_constraints() {
    let profile = ExecutionMode::NonAccredited.constraint_profile();

    assert_eq!(profile.mode(), ExecutionMode::NonAccredited);
    assert!(profile.stage_gates_required());
    assert!(!profile.valid_calibration_required());
    assert!(profile.controlled_method_required());
    assert!(!profile.report_approval_required());
    assert!(profile.deviations_allowed());
    assert!(!profile.exploratory_steps_allowed());
}

#[test]
fn investigation_mode_allows_exploratory_work() {
    let profile = ExecutionMode::Investigation.constraint_profile();

    assert_eq!(profile.mode(), ExecutionMode::Investigation);
    assert!(!profile.stage_gates_required());
    assert!(!profile.valid_calibration_required());
    assert!(!profile.controlled_method_required());
    assert!(!profile.report_approval_required());
    assert!(profile.deviations_allowed());
    assert!(profile.exploratory_steps_allowed());
}

#[test]
fn offline_field_mode_requires_local_references_but_allows_acquisition() {
    let mode = ConnectivityMode::OfflineField;

    assert!(mode.requires_local_references());
    assert!(mode.allows_measurement_acquisition());
    assert!(!mode.can_require_remote_login());
}

#[test]
fn repository_policy_keeps_core_references_available_offline() {
    let domains = baseline_repository_domains();

    assert!(domains.contains(&RepositoryDomain::Metrology));
    assert!(domains.contains(&RepositoryDomain::TestDefinitions));
    assert!(domains.contains(&RepositoryDomain::InstrumentDrivers));
    assert!(domains.contains(&RepositoryDomain::ProjectRecords));
    assert!(domains.contains(&RepositoryDomain::MeasurementData));

    let policy = RepositoryPolicy::new(RepositoryDomain::Metrology, ConnectivityMode::OfflineField);
    assert_eq!(policy.domain(), RepositoryDomain::Metrology);
    assert_eq!(policy.sync_direction(), SyncDirection::PullFromReference);
    assert!(policy.local_snapshot_required());
}

#[test]
fn offline_field_snapshot_requirements_cover_every_repository_domain() {
    let requirements = offline_field_snapshot_requirements();

    assert_eq!(requirements.len(), baseline_repository_domains().len());
    assert!(requirements
        .iter()
        .any(|requirement| requirement.domain() == RepositoryDomain::Metrology));
    assert!(requirements
        .iter()
        .any(|requirement| requirement.domain() == RepositoryDomain::MeasurementData));
    assert!(requirements
        .iter()
        .all(RepositorySnapshotRequirement::signature_required));
    assert!(requirements
        .iter()
        .all(|requirement| requirement.minimum_schema_version() == 1));
}

#[test]
fn repository_snapshot_identity_and_schema_are_validated() {
    assert_eq!(
        RepositorySnapshotId::parse(" ").unwrap_err(),
        DomainError::EmptyRepositorySnapshotId
    );
    assert_eq!(
        RepositorySnapshotId::parse("metrology v1").unwrap_err(),
        DomainError::InvalidRepositorySnapshotId("metrology v1".to_owned())
    );
    assert_eq!(
        SnapshotChecksum::parse("").unwrap_err(),
        DomainError::EmptySnapshotChecksum
    );
    assert_eq!(
        RepositorySnapshot::new(
            RepositoryDomain::Metrology,
            RepositorySnapshotId::parse("metrology-v0").unwrap(),
            0,
            SnapshotChecksum::parse("sha256:metrology").unwrap(),
            true,
        )
        .unwrap_err(),
        DomainError::InvalidRepositorySchemaVersion(0)
    );
}

#[test]
fn field_repository_package_rejects_duplicate_domain_snapshots() {
    let snapshot = signed_snapshot(RepositoryDomain::Metrology);

    let error = FieldRepositoryPackage::new(vec![snapshot.clone(), snapshot]).unwrap_err();

    assert_eq!(
        error,
        DomainError::DuplicateRepositorySnapshot("metrology".to_owned())
    );
}

#[test]
fn field_repository_package_validation_rejects_missing_snapshots() {
    let package =
        FieldRepositoryPackage::new(vec![signed_snapshot(RepositoryDomain::Metrology)]).unwrap();

    let error = package
        .validate(&offline_field_snapshot_requirements())
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::MissingRepositorySnapshot("test_definitions".to_owned())
    );
}

#[test]
fn field_repository_package_validation_rejects_unsigned_snapshots() {
    let snapshots: Vec<_> = baseline_repository_domains()
        .into_iter()
        .map(|domain| snapshot_with_signature(domain, domain != RepositoryDomain::UpdateCatalog))
        .collect();
    let package = FieldRepositoryPackage::new(snapshots).unwrap();

    let error = package
        .validate(&offline_field_snapshot_requirements())
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::UnsignedRepositorySnapshot("update_catalog".to_owned())
    );
}

#[test]
fn field_repository_package_validation_rejects_incompatible_schema() {
    let package =
        FieldRepositoryPackage::new(vec![signed_snapshot(RepositoryDomain::Metrology)]).unwrap();
    let requirements =
        vec![RepositorySnapshotRequirement::new(RepositoryDomain::Metrology, 2, true).unwrap()];

    let error = package.validate(&requirements).unwrap_err();

    assert_eq!(
        error,
        DomainError::IncompatibleRepositorySnapshot {
            domain: "metrology".to_owned(),
            minimum_schema_version: 2,
            actual_schema_version: 1,
        }
    );
}

#[test]
fn signed_field_repository_package_validates_for_offline_work() {
    let snapshots = baseline_repository_domains()
        .into_iter()
        .map(signed_snapshot)
        .collect();
    let package = FieldRepositoryPackage::new(snapshots).unwrap();

    package
        .validate(&offline_field_snapshot_requirements())
        .unwrap();

    assert_eq!(
        package
            .snapshot_for(RepositoryDomain::MeasurementData)
            .unwrap()
            .domain(),
        RepositoryDomain::MeasurementData
    );
}

#[test]
fn instrument_transport_baseline_covers_common_lab_communications() {
    let transports = baseline_instrument_transports();

    assert!(transports.contains(&InstrumentTransport::Visa));
    assert!(transports.contains(&InstrumentTransport::Gpib));
    assert!(transports.contains(&InstrumentTransport::Serial));
    assert!(transports.contains(&InstrumentTransport::TcpIp));
    assert!(transports.contains(&InstrumentTransport::UsbTmc));
    assert!(transports.contains(&InstrumentTransport::Can));
    assert!(transports.contains(&InstrumentTransport::Rest));
    assert!(transports.contains(&InstrumentTransport::VendorSdk));
    assert!(transports.contains(&InstrumentTransport::Simulated));
}

#[test]
fn instrument_transport_slugs_are_stable_for_logs_and_adapters() {
    assert_eq!(InstrumentTransport::Visa.as_str(), "visa");
    assert_eq!(InstrumentTransport::UsbTmc.as_str(), "usb_tmc");
    assert_eq!(InstrumentTransport::VendorSdk.as_str(), "vendor_sdk");
    assert_eq!(InstrumentTransport::Simulated.as_str(), "simulated");
}

#[test]
fn instrument_command_message_rejects_empty_values() {
    assert_eq!(
        InstrumentCommandMessage::parse(" ").unwrap_err(),
        DomainError::EmptyInstrumentCommandMessage
    );

    assert_eq!(
        InstrumentCommandMessage::parse("*IDN?").unwrap().as_str(),
        "*IDN?"
    );
}

#[test]
fn simulated_instrument_runtime_records_ordered_observations() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let mut runtime = SimulatedInstrumentRuntime::new(
        code.clone(),
        vec![InstrumentTransport::Simulated, InstrumentTransport::Visa],
    );

    let first = runtime
        .execute(InstrumentCommand::new(
            code.clone(),
            InstrumentTransport::Simulated,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap()
        .clone();
    let second = runtime
        .execute(InstrumentCommand::new(
            code.clone(),
            InstrumentTransport::Visa,
            InstrumentCommandMessage::parse("FREQ 1000000").unwrap(),
        ))
        .unwrap()
        .clone();

    assert_eq!(runtime.instrument(), &code);
    assert_eq!(runtime.observations().len(), 2);
    assert_eq!(first.sequence(), 1);
    assert_eq!(first.response().as_str(), "SIM:*IDN?=0");
    assert!(first.success());
    assert_eq!(second.sequence(), 2);
    assert_eq!(second.response().as_str(), "OK:FREQ 1000000");
    assert_eq!(second.command().transport(), InstrumentTransport::Visa);
}

#[test]
fn simulated_instrument_runtime_rejects_wrong_target() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let other = InstrumentCode::parse("GEN-001").unwrap();
    let mut runtime =
        SimulatedInstrumentRuntime::new(code.clone(), vec![InstrumentTransport::Simulated]);

    let error = runtime
        .execute(InstrumentCommand::new(
            other,
            InstrumentTransport::Simulated,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::InstrumentCommandTargetMismatch {
            expected: code.as_str().to_owned(),
            actual: "GEN-001".to_owned(),
        }
    );
    assert!(runtime.observations().is_empty());
}

#[test]
fn simulated_instrument_runtime_rejects_unsupported_transport() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let mut runtime =
        SimulatedInstrumentRuntime::new(code.clone(), vec![InstrumentTransport::Simulated]);

    let error = runtime
        .execute(InstrumentCommand::new(
            code,
            InstrumentTransport::TcpIp,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::UnsupportedInstrumentTransport("tcp_ip".to_owned())
    );
    assert!(runtime.observations().is_empty());
}

#[test]
fn update_policy_requires_signed_packages_and_blocks_live_measurement_updates() {
    let policy = UpdatePolicy::laboratory_default();

    assert!(policy.signed_packages_required());
    assert!(policy.offline_install_allowed());
    assert!(!policy.apply_during_measurement_allowed());
}

#[test]
fn instrument_code_rejects_empty_and_unsafe_values() {
    assert_eq!(
        InstrumentCode::parse(" ").unwrap_err(),
        DomainError::EmptyInstrumentCode
    );
    assert_eq!(
        InstrumentCode::parse("RX 01").unwrap_err(),
        DomainError::InvalidInstrumentCode("RX 01".to_owned())
    );

    let code = InstrumentCode::parse("RX-2026_001.A").unwrap();
    assert_eq!(code.as_str(), "RX-2026_001.A");
}

#[test]
fn metrology_date_validates_calendar_boundaries() {
    assert!(MetrologyDate::new(2024, 2, 29).is_ok());
    assert_eq!(
        MetrologyDate::new(2026, 2, 29).unwrap_err(),
        DomainError::InvalidMetrologyDate {
            year: 2026,
            month: 2,
            day: 29,
        }
    );
    assert_eq!(
        MetrologyDate::new(1899, 12, 31).unwrap_err(),
        DomainError::InvalidMetrologyDate {
            year: 1899,
            month: 12,
            day: 31,
        }
    );
}

#[test]
fn metrology_registry_rejects_duplicate_instruments() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let instrument = InstrumentRecord::new(
        code.clone(),
        InstrumentFamily::Receiver,
        "Nexio Lab",
        "Reference Receiver",
        "SN-001",
        CalibrationRequirement::Required,
    )
    .unwrap();
    let mut registry = MetrologyRegistry::new();

    registry.register_instrument(instrument.clone()).unwrap();
    let error = registry.register_instrument(instrument).unwrap_err();

    assert_eq!(
        error,
        DomainError::DuplicateInstrumentCode(code.as_str().to_owned())
    );
}

#[test]
fn calibration_records_must_belong_to_a_registered_instrument() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let record = CalibrationRecord::new(
        code.clone(),
        "CERT-001",
        MetrologyDate::new(2026, 1, 1).unwrap(),
        MetrologyDate::new(2027, 1, 1).unwrap(),
        "Accredited Provider",
    )
    .unwrap();
    let mut registry = MetrologyRegistry::new();

    let error = registry.record_calibration(record).unwrap_err();

    assert_eq!(
        error,
        DomainError::UnknownInstrumentCode(code.as_str().to_owned())
    );
}

#[test]
fn accredited_equipment_readiness_requires_valid_calibration() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let mut registry = MetrologyRegistry::new();
    registry
        .register_instrument(reference_receiver(code.clone()))
        .unwrap();

    let report = registry.assess_equipment_readiness(
        &[code.clone()],
        ExecutionMode::Accredited,
        MetrologyDate::new(2026, 6, 27).unwrap(),
    );

    assert!(!report.is_ready());
    assert_eq!(report.mode(), ExecutionMode::Accredited);
    assert_eq!(
        report.checked_on(),
        MetrologyDate::new(2026, 6, 27).unwrap()
    );
    assert_eq!(report.issues().len(), 1);
    assert_eq!(report.issues()[0].instrument(), &code);
    assert_eq!(
        report.issues()[0].kind(),
        EquipmentIssueKind::CalibrationMissing
    );
    assert!(report.issues()[0].is_blocking());
}

#[test]
fn non_accredited_equipment_readiness_flags_expired_calibration_without_blocking() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let mut registry = MetrologyRegistry::new();
    registry
        .register_instrument(reference_receiver(code.clone()))
        .unwrap();
    registry
        .record_calibration(
            CalibrationRecord::new(
                code.clone(),
                "CERT-2025-001",
                MetrologyDate::new(2025, 1, 1).unwrap(),
                MetrologyDate::new(2026, 1, 1).unwrap(),
                "Accredited Provider",
            )
            .unwrap(),
        )
        .unwrap();

    let report = registry.assess_equipment_readiness(
        &[code],
        ExecutionMode::NonAccredited,
        MetrologyDate::new(2026, 6, 27).unwrap(),
    );

    assert!(report.is_ready());
    assert_eq!(report.issues().len(), 1);
    assert_eq!(
        report.issues()[0].kind(),
        EquipmentIssueKind::CalibrationExpired
    );
    assert!(!report.issues()[0].is_blocking());
}

#[test]
fn valid_calibrated_equipment_is_ready_for_accredited_work() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let mut registry = MetrologyRegistry::new();
    registry
        .register_instrument(reference_receiver(code.clone()))
        .unwrap();
    registry
        .record_calibration(
            CalibrationRecord::new(
                code.clone(),
                "CERT-2026-001",
                MetrologyDate::new(2026, 1, 1).unwrap(),
                MetrologyDate::new(2027, 1, 1).unwrap(),
                "Accredited Provider",
            )
            .unwrap(),
        )
        .unwrap();

    let report = registry.assess_equipment_readiness(
        &[code.clone()],
        ExecutionMode::Accredited,
        MetrologyDate::new(2026, 6, 27).unwrap(),
    );

    assert!(report.is_ready());
    assert!(report.issues().is_empty());
    assert_eq!(
        registry
            .latest_calibration_for(&code)
            .unwrap()
            .certificate_reference(),
        "CERT-2026-001"
    );
}

#[test]
fn equipment_due_soon_is_reported_as_non_blocking_attention_point() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let mut registry = MetrologyRegistry::new();
    registry
        .register_instrument(reference_receiver(code.clone()))
        .unwrap();
    registry
        .record_calibration(
            CalibrationRecord::new(
                code.clone(),
                "CERT-2026-001",
                MetrologyDate::new(2026, 1, 1).unwrap(),
                MetrologyDate::new(2026, 7, 15).unwrap(),
                "Accredited Provider",
            )
            .unwrap(),
        )
        .unwrap();

    let report = registry.assess_equipment_readiness(
        &[code],
        ExecutionMode::Accredited,
        MetrologyDate::new(2026, 6, 27).unwrap(),
    );

    assert!(report.is_ready());
    assert_eq!(report.issues().len(), 1);
    assert_eq!(
        report.issues()[0].kind(),
        EquipmentIssueKind::CalibrationDueSoon
    );
    assert!(!report.issues()[0].is_blocking());
}

#[test]
fn out_of_service_equipment_blocks_every_execution_mode() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let mut receiver = reference_receiver(code.clone());
    receiver.set_availability(InstrumentAvailability::OutOfService);
    let mut registry = MetrologyRegistry::new();
    registry.register_instrument(receiver).unwrap();

    let report = registry.assess_equipment_readiness(
        &[code],
        ExecutionMode::Investigation,
        MetrologyDate::new(2026, 6, 27).unwrap(),
    );

    assert!(!report.is_ready());
    assert_eq!(report.issues()[0].kind(), EquipmentIssueKind::OutOfService);
    assert!(report.issues()[0].is_blocking());
}

#[test]
fn measurement_run_reference_and_method_reference_validate_values() {
    assert_eq!(
        MeasurementRunReference::parse(" ").unwrap_err(),
        DomainError::EmptyMeasurementRunReference
    );
    assert_eq!(
        MeasurementRunReference::parse("RUN 001").unwrap_err(),
        DomainError::InvalidMeasurementRunReference("RUN 001".to_owned())
    );
    assert_eq!(
        TestMethodReference::parse("").unwrap_err(),
        DomainError::EmptyTestMethodReference
    );
    assert_eq!(
        TestMethodReference::parse("EN 61000").unwrap_err(),
        DomainError::InvalidTestMethodReference("EN 61000".to_owned())
    );

    assert_eq!(
        MeasurementRunReference::parse("RUN-001").unwrap().as_str(),
        "RUN-001"
    );
    assert_eq!(
        TestMethodReference::parse("EN61000-4-6").unwrap().as_str(),
        "EN61000-4-6"
    );
}

#[test]
fn measurement_run_plan_requires_equipment_selection() {
    let project = ProjectCode::parse("CEM-2026-001").unwrap();
    let registry = MetrologyRegistry::new();

    let error = MeasurementRunPlan::plan(
        project,
        MeasurementRunReference::parse("RUN-001").unwrap(),
        TestMethodReference::parse("EN61000-4-6").unwrap(),
        ExecutionMode::Accredited,
        Vec::new(),
        &registry,
        MetrologyDate::new(2026, 6, 27).unwrap(),
    )
    .unwrap_err();

    assert_eq!(error, DomainError::EmptyEquipmentSelection);
}

#[test]
fn accredited_measurement_run_plan_blocks_when_required_calibration_is_missing() {
    let project = ProjectCode::parse("CEM-2026-001").unwrap();
    let code = InstrumentCode::parse("RX-001").unwrap();
    let mut registry = MetrologyRegistry::new();
    registry
        .register_instrument(reference_receiver(code.clone()))
        .unwrap();

    let error = MeasurementRunPlan::plan(
        project,
        MeasurementRunReference::parse("RUN-001").unwrap(),
        TestMethodReference::parse("EN61000-4-6").unwrap(),
        ExecutionMode::Accredited,
        vec![code],
        &registry,
        MetrologyDate::new(2026, 6, 27).unwrap(),
    )
    .unwrap_err();

    assert_eq!(
        error,
        DomainError::EquipmentReadinessBlocked {
            blocking_issue_count: 1,
        }
    );
}

#[test]
fn non_accredited_measurement_run_plan_keeps_non_blocking_readiness_warnings() {
    let project = ProjectCode::parse("CEM-2026-001").unwrap();
    let code = InstrumentCode::parse("RX-001").unwrap();
    let mut registry = MetrologyRegistry::new();
    registry
        .register_instrument(reference_receiver(code.clone()))
        .unwrap();
    registry
        .record_calibration(
            CalibrationRecord::new(
                code.clone(),
                "CERT-2025-001",
                MetrologyDate::new(2025, 1, 1).unwrap(),
                MetrologyDate::new(2026, 1, 1).unwrap(),
                "Accredited Provider",
            )
            .unwrap(),
        )
        .unwrap();

    let plan = MeasurementRunPlan::plan(
        project.clone(),
        MeasurementRunReference::parse("RUN-001").unwrap(),
        TestMethodReference::parse("EN61000-4-6").unwrap(),
        ExecutionMode::NonAccredited,
        vec![code.clone()],
        &registry,
        MetrologyDate::new(2026, 6, 27).unwrap(),
    )
    .unwrap();

    assert_eq!(plan.project(), &project);
    assert_eq!(plan.mode(), ExecutionMode::NonAccredited);
    assert_eq!(plan.equipment(), &[code]);
    assert!(plan.readiness_report().is_ready());
    assert_eq!(plan.readiness_report().issues().len(), 1);
    assert_eq!(
        plan.readiness_report().issues()[0].kind(),
        EquipmentIssueKind::CalibrationExpired
    );
    assert!(!plan.readiness_report().issues()[0].is_blocking());
}

#[test]
fn accredited_measurement_run_plan_accepts_valid_calibrated_equipment() {
    let project = ProjectCode::parse("CEM-2026-001").unwrap();
    let code = InstrumentCode::parse("RX-001").unwrap();
    let mut registry = MetrologyRegistry::new();
    registry
        .register_instrument(reference_receiver(code.clone()))
        .unwrap();
    registry
        .record_calibration(
            CalibrationRecord::new(
                code.clone(),
                "CERT-2026-001",
                MetrologyDate::new(2026, 1, 1).unwrap(),
                MetrologyDate::new(2027, 1, 1).unwrap(),
                "Accredited Provider",
            )
            .unwrap(),
        )
        .unwrap();

    let plan = MeasurementRunPlan::plan(
        project,
        MeasurementRunReference::parse("RUN-001").unwrap(),
        TestMethodReference::parse("EN61000-4-6").unwrap(),
        ExecutionMode::Accredited,
        vec![code],
        &registry,
        MetrologyDate::new(2026, 6, 27).unwrap(),
    )
    .unwrap();

    assert_eq!(plan.reference().as_str(), "RUN-001");
    assert_eq!(plan.method().as_str(), "EN61000-4-6");
    assert!(plan.readiness_report().is_ready());
    assert!(plan.readiness_report().issues().is_empty());
}

#[test]
fn dataset_references_and_checksums_validate_values() {
    assert_eq!(
        DatasetReference::parse(" ").unwrap_err(),
        DomainError::EmptyDatasetReference
    );
    assert_eq!(
        DatasetReference::parse("raw signal 1").unwrap_err(),
        DomainError::InvalidDatasetReference("raw signal 1".to_owned())
    );
    assert_eq!(
        DatasetFileReference::parse("").unwrap_err(),
        DomainError::EmptyDatasetFileReference
    );
    assert_eq!(
        DatasetChecksum::parse("").unwrap_err(),
        DomainError::EmptyDatasetChecksum
    );
    assert_eq!(
        DatasetChecksum::parse("abc123").unwrap_err(),
        DomainError::InvalidDatasetChecksum("abc123".to_owned())
    );

    assert_eq!(
        DatasetReference::parse("raw-signal-001").unwrap().as_str(),
        "raw-signal-001"
    );
    assert_eq!(
        DatasetChecksum::parse("sha256:abc123").unwrap().as_str(),
        "sha256:abc123"
    );
}

#[test]
fn raw_dataset_record_is_immutable_and_linked_to_a_run() {
    let run = MeasurementRunReference::parse("RUN-001").unwrap();
    let record = RawDatasetRecord::new(
        run.clone(),
        DatasetReference::parse("raw-signal-001").unwrap(),
        DatasetKind::RawSignal,
        DatasetFileReference::parse("data/RUN-001/raw-signal-001.opendata").unwrap(),
        DatasetChecksum::parse("sha256:abc123").unwrap(),
    );

    assert_eq!(record.run(), &run);
    assert_eq!(record.reference().as_str(), "raw-signal-001");
    assert_eq!(record.kind(), DatasetKind::RawSignal);
    assert_eq!(
        record.file_reference().as_str(),
        "data/RUN-001/raw-signal-001.opendata"
    );
    assert_eq!(record.checksum().as_str(), "sha256:abc123");
    assert!(record.immutable());
}

#[test]
fn measurement_run_evidence_records_observations_and_raw_datasets() {
    let plan = accepted_measurement_plan("RUN-001");
    let instrument = plan.equipment()[0].clone();
    let mut runtime =
        SimulatedInstrumentRuntime::new(instrument.clone(), vec![InstrumentTransport::Simulated]);
    let observation = runtime
        .execute(InstrumentCommand::new(
            instrument,
            InstrumentTransport::Simulated,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap()
        .clone();
    let mut evidence = MeasurementRunEvidence::new(plan);

    evidence.record_observation(observation);
    evidence
        .record_raw_dataset(RawDatasetRecord::new(
            MeasurementRunReference::parse("RUN-001").unwrap(),
            DatasetReference::parse("raw-signal-001").unwrap(),
            DatasetKind::RawSignal,
            DatasetFileReference::parse("data/RUN-001/raw-signal-001.opendata").unwrap(),
            DatasetChecksum::parse("sha256:abc123").unwrap(),
        ))
        .unwrap();

    assert_eq!(evidence.observations().len(), 1);
    assert_eq!(evidence.raw_datasets().len(), 1);
    assert!(evidence.has_raw_data());
}

#[test]
fn measurement_run_evidence_rejects_dataset_from_another_run() {
    let plan = accepted_measurement_plan("RUN-001");
    let mut evidence = MeasurementRunEvidence::new(plan);
    let dataset = RawDatasetRecord::new(
        MeasurementRunReference::parse("RUN-002").unwrap(),
        DatasetReference::parse("raw-signal-001").unwrap(),
        DatasetKind::RawSignal,
        DatasetFileReference::parse("data/RUN-002/raw-signal-001.opendata").unwrap(),
        DatasetChecksum::parse("sha256:abc123").unwrap(),
    );

    let error = evidence.record_raw_dataset(dataset).unwrap_err();

    assert_eq!(
        error,
        DomainError::DatasetRunMismatch {
            expected: "RUN-001".to_owned(),
            actual: "RUN-002".to_owned(),
        }
    );
    assert!(!evidence.has_raw_data());
}

#[test]
fn report_number_and_revision_reject_empty_values() {
    assert_eq!(
        ReportNumber::parse(" ").unwrap_err(),
        DomainError::EmptyReportNumber
    );
    assert_eq!(
        ReportRevision::parse("").unwrap_err(),
        DomainError::EmptyReportRevision
    );

    assert_eq!(ReportNumber::parse("RPT-001").unwrap().as_str(), "RPT-001");
    assert_eq!(ReportRevision::parse("A").unwrap().as_str(), "A");
}

#[test]
fn accredited_report_requires_technical_review_before_approval() {
    let project = ProjectCode::parse("CEM-2026-001").unwrap();
    let approver = AuditActor::parse("quality.manager").unwrap();
    let mut report = ReportPackage::new(
        project,
        ReportNumber::parse("RPT-001").unwrap(),
        ReportRevision::parse("A").unwrap(),
        ExecutionMode::Accredited,
    );

    let error = report.approve(approver).unwrap_err();

    assert_eq!(error, DomainError::ReportTechnicalReviewRequired);
    assert_eq!(report.status(), ReportStatus::Draft);
}

#[test]
fn accredited_report_requires_approval_before_issue() {
    let project = ProjectCode::parse("CEM-2026-001").unwrap();
    let mut report = ReportPackage::new(
        project,
        ReportNumber::parse("RPT-001").unwrap(),
        ReportRevision::parse("A").unwrap(),
        ExecutionMode::Accredited,
    );

    let error = report.issue().unwrap_err();

    assert_eq!(error, DomainError::ReportApprovalRequired);
    assert_eq!(report.status(), ReportStatus::Draft);
}

#[test]
fn accredited_report_follows_review_approval_issue_flow() {
    let project = ProjectCode::parse("CEM-2026-001").unwrap();
    let reviewer = AuditActor::parse("technical.reviewer").unwrap();
    let approver = AuditActor::parse("quality.manager").unwrap();
    let mut report = ReportPackage::new(
        project.clone(),
        ReportNumber::parse("RPT-001").unwrap(),
        ReportRevision::parse("A").unwrap(),
        ExecutionMode::Accredited,
    );

    report.submit_for_technical_review().unwrap();
    report.complete_technical_review(reviewer.clone()).unwrap();
    report.approve(approver.clone()).unwrap();
    report.issue().unwrap();

    assert_eq!(report.project(), &project);
    assert_eq!(report.number().as_str(), "RPT-001");
    assert_eq!(report.revision().as_str(), "A");
    assert_eq!(report.mode(), ExecutionMode::Accredited);
    assert_eq!(report.status(), ReportStatus::Issued);
    assert_eq!(report.reviewed_by(), Some(&reviewer));
    assert_eq!(report.approved_by(), Some(&approver));
}

#[test]
fn non_accredited_report_can_be_issued_without_formal_approval() {
    let project = ProjectCode::parse("CEM-2026-001").unwrap();
    let mut report = ReportPackage::new(
        project,
        ReportNumber::parse("RPT-001").unwrap(),
        ReportRevision::parse("A").unwrap(),
        ExecutionMode::NonAccredited,
    );

    report.issue().unwrap();

    assert_eq!(report.status(), ReportStatus::Issued);
    assert_eq!(report.approved_by(), None);
}

#[test]
fn report_workflow_rejects_invalid_review_transition() {
    let project = ProjectCode::parse("CEM-2026-001").unwrap();
    let reviewer = AuditActor::parse("technical.reviewer").unwrap();
    let mut report = ReportPackage::new(
        project,
        ReportNumber::parse("RPT-001").unwrap(),
        ReportRevision::parse("A").unwrap(),
        ExecutionMode::Accredited,
    );

    let error = report.complete_technical_review(reviewer).unwrap_err();

    assert_eq!(
        error,
        DomainError::InvalidReportTransition {
            from: "draft".to_owned(),
            to: "technically_reviewed".to_owned(),
        }
    );
}

#[test]
fn cem_time_domain_workflow_prefers_opendaq_and_mixed_signal_processing() {
    let profile = SignalWorkflowProfile::cem_time_domain_default();

    assert_eq!(profile.axis(), MeasurementAxis::MixedTimeFrequency);
    assert_eq!(profile.preferred_daq_interface(), DaqInterface::OpenDaq);
    assert!(profile.synchronization_required());
    assert!(profile
        .operations()
        .contains(&SignalProcessingOperation::Fft));
    assert!(profile
        .operations()
        .contains(&SignalProcessingOperation::TimeDomainFilter));
    assert!(profile
        .operations()
        .contains(&SignalProcessingOperation::ChannelArithmetic));
    assert!(profile
        .operations()
        .contains(&SignalProcessingOperation::HarmonicAnalysis));
    assert!(profile
        .operations()
        .contains(&SignalProcessingOperation::InrushAnalysis));
}

#[test]
fn signal_reference_and_sample_rate_reject_invalid_values() {
    assert_eq!(
        SignalReference::parse(" ").unwrap_err(),
        DomainError::EmptySignalReference
    );
    assert_eq!(
        SignalReference::parse("current l1").unwrap_err(),
        DomainError::InvalidSignalReference("current l1".to_owned())
    );
    assert_eq!(
        SampleRateHz::new(0).unwrap_err(),
        DomainError::InvalidSampleRateHz(0)
    );

    let reference = SignalReference::parse("current_l1").unwrap();
    let sample_rate = SampleRateHz::new(10_000).unwrap();

    assert_eq!(reference.as_str(), "current_l1");
    assert_eq!(sample_rate.value(), 10_000);
}

#[test]
fn simulated_daq_inrush_fixture_produces_synchronized_channels() {
    let source = SimulatedDaqSource::open_daq();
    let dataset = source.acquire_inrush_fixture().unwrap();
    let voltage = SignalReference::parse("voltage_l1").unwrap();
    let current = SignalReference::parse("current_l1").unwrap();

    assert_eq!(source.interface(), DaqInterface::OpenDaq);
    assert_eq!(
        source.synchronization_method(),
        SynchronizationMethod::SharedSampleClock
    );
    assert_eq!(dataset.daq_interface(), DaqInterface::OpenDaq);
    assert_eq!(
        dataset.synchronization_method(),
        SynchronizationMethod::SharedSampleClock
    );
    assert_eq!(dataset.channels().len(), 2);
    assert_eq!(dataset.channel(&voltage).unwrap().samples()[3], 520);
    assert_eq!(dataset.channel(&current).unwrap().samples()[3], 180);
    assert_eq!(
        dataset.channel(&current).unwrap().sample_rate(),
        SampleRateHz::new(10_000).unwrap()
    );
}

#[test]
fn empty_signal_dataset_is_rejected() {
    let error = SignalDataset::new(
        DaqInterface::Simulated,
        SynchronizationMethod::SoftwareTimestamp,
        Vec::new(),
    )
    .unwrap_err();

    assert_eq!(error, DomainError::EmptySignalDataset);
}

#[test]
fn signal_processing_graph_tracks_fft_and_channel_math_lineage() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let voltage = SignalReference::parse("voltage_l1").unwrap();
    let current_fft = SignalReference::parse("current_l1_fft").unwrap();
    let apparent_power = SignalReference::parse("apparent_power").unwrap();
    let mut graph = SignalProcessingGraph::from_dataset(&dataset);

    graph
        .add_node(
            SignalProcessingNode::new(
                SignalReference::parse("fft_current").unwrap(),
                SignalProcessingOperation::Fft,
                vec![current.clone()],
                current_fft.clone(),
            )
            .unwrap(),
        )
        .unwrap();
    graph
        .add_node(
            SignalProcessingNode::new(
                SignalReference::parse("math_power").unwrap(),
                SignalProcessingOperation::ChannelArithmetic,
                vec![voltage.clone(), current.clone()],
                apparent_power.clone(),
            )
            .unwrap(),
        )
        .unwrap();

    assert_eq!(graph.source_signals().len(), 2);
    assert_eq!(graph.nodes().len(), 2);
    assert!(graph.contains_operation(SignalProcessingOperation::Fft));
    assert!(graph.contains_operation(SignalProcessingOperation::ChannelArithmetic));
    assert_eq!(graph.raw_lineage_for(&current_fft).unwrap(), vec![current]);
    assert_eq!(
        graph.raw_lineage_for(&apparent_power).unwrap(),
        vec![voltage, SignalReference::parse("current_l1").unwrap()]
    );
}

#[test]
fn signal_processing_graph_rejects_unknown_inputs_and_duplicate_nodes() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let mut graph = SignalProcessingGraph::from_dataset(&dataset);
    let unknown = SignalReference::parse("unknown_channel").unwrap();
    let current = SignalReference::parse("current_l1").unwrap();

    let error = graph
        .add_node(
            SignalProcessingNode::new(
                SignalReference::parse("fft_current").unwrap(),
                SignalProcessingOperation::Fft,
                vec![unknown.clone()],
                SignalReference::parse("current_l1_fft").unwrap(),
            )
            .unwrap(),
        )
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::UnknownSignalReference(unknown.as_str().to_owned())
    );

    graph
        .add_node(
            SignalProcessingNode::new(
                SignalReference::parse("fft_current").unwrap(),
                SignalProcessingOperation::Fft,
                vec![current.clone()],
                SignalReference::parse("current_l1_fft").unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
    let duplicate_error = graph
        .add_node(
            SignalProcessingNode::new(
                SignalReference::parse("fft_current").unwrap(),
                SignalProcessingOperation::WindowedFft,
                vec![current],
                SignalReference::parse("current_l1_windowed_fft").unwrap(),
            )
            .unwrap(),
        )
        .unwrap_err();

    assert_eq!(
        duplicate_error,
        DomainError::DuplicateProcessingNode("fft_current".to_owned())
    );
}

#[test]
fn signal_processing_node_requires_inputs() {
    let error = SignalProcessingNode::new(
        SignalReference::parse("fft_current").unwrap(),
        SignalProcessingOperation::Fft,
        Vec::new(),
        SignalReference::parse("current_l1_fft").unwrap(),
    )
    .unwrap_err();

    assert_eq!(error, DomainError::EmptyProcessingNodeInputs);
}

#[test]
fn signal_execution_engine_channel_sum_preserves_samples_and_lineage() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let voltage = SignalReference::parse("voltage_l1").unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let output = SignalReference::parse("voltage_plus_current").unwrap();

    let result = SignalExecutionEngine::channel_sum(
        &dataset,
        &voltage,
        &current,
        output.clone(),
        SignalUnit::parse("derived").unwrap(),
    )
    .unwrap();

    assert_eq!(result.output(), &output);
    assert_eq!(
        result.operation(),
        SignalProcessingOperation::ChannelArithmetic
    );
    assert_eq!(result.unit().as_str(), "derived");
    assert_eq!(result.samples()[3], 700);
    assert_eq!(result.raw_lineage(), &[voltage, current]);
}

#[test]
fn signal_execution_engine_peak_reports_absolute_peak() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let output = SignalReference::parse("current_peak").unwrap();

    let result = SignalExecutionEngine::peak(&dataset, &current, output.clone()).unwrap();

    assert_eq!(result.output(), &output);
    assert_eq!(result.operation(), SignalProcessingOperation::Peak);
    assert_eq!(result.unit().as_str(), "mA");
    assert_eq!(result.value(), 180.0);
    assert_eq!(result.raw_lineage(), &[current]);
}

#[test]
fn signal_execution_engine_dft_magnitude_returns_deterministic_bins() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let output = SignalReference::parse("current_fft").unwrap();

    let result = SignalExecutionEngine::dft_magnitude(&dataset, &current, output.clone()).unwrap();

    assert_eq!(result.output(), &output);
    assert_eq!(result.operation(), SignalProcessingOperation::Fft);
    assert_eq!(result.magnitudes().len(), 8);
    assert!((result.magnitudes()[0] - 425.0).abs() < 1e-9);
    assert_eq!(result.raw_lineage(), &[current]);
}

#[test]
fn signal_execution_engine_rejects_unknown_inputs() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let unknown = SignalReference::parse("missing").unwrap();

    let error = SignalExecutionEngine::peak(
        &dataset,
        &unknown,
        SignalReference::parse("missing_peak").unwrap(),
    )
    .unwrap_err();

    assert_eq!(
        error,
        DomainError::UnknownSignalReference("missing".to_owned())
    );
}

#[test]
fn signal_execution_engine_rejects_sample_count_mismatch() {
    let sample_rate = SampleRateHz::new(10_000).unwrap();
    let left = AcquiredSignalChannel::new(
        SignalReference::parse("left").unwrap(),
        SignalSourceKind::AnalogVoltage,
        SignalUnit::parse("mV").unwrap(),
        sample_rate,
        vec![1, 2, 3],
    );
    let right = AcquiredSignalChannel::new(
        SignalReference::parse("right").unwrap(),
        SignalSourceKind::AnalogVoltage,
        SignalUnit::parse("mV").unwrap(),
        sample_rate,
        vec![1, 2],
    );
    let dataset = SignalDataset::new(
        DaqInterface::Simulated,
        SynchronizationMethod::SharedSampleClock,
        vec![left],
    )
    .unwrap();
    let dataset = SignalDataset::new(
        dataset.daq_interface(),
        dataset.synchronization_method(),
        vec![
            dataset
                .channel(&SignalReference::parse("left").unwrap())
                .unwrap()
                .clone(),
            right,
        ],
    )
    .unwrap();

    let error = SignalExecutionEngine::channel_sum(
        &dataset,
        &SignalReference::parse("left").unwrap(),
        &SignalReference::parse("right").unwrap(),
        SignalReference::parse("sum").unwrap(),
        SignalUnit::parse("mV").unwrap(),
    )
    .unwrap_err();

    assert_eq!(
        error,
        DomainError::SignalSampleCountMismatch {
            left_count: 3,
            right_count: 2,
        }
    );
}

#[test]
fn synchronization_baseline_covers_multi_daq_alignment_methods() {
    let methods = baseline_synchronization_methods();

    assert!(methods.contains(&SynchronizationMethod::SharedSampleClock));
    assert!(methods.contains(&SynchronizationMethod::ExternalTrigger));
    assert!(methods.contains(&SynchronizationMethod::PtpIeee1588));
    assert!(methods.contains(&SynchronizationMethod::GpsGnss));
    assert!(methods.contains(&SynchronizationMethod::IrigB));
    assert!(methods.contains(&SynchronizationMethod::EtherCatDistributedClock));
    assert!(methods.contains(&SynchronizationMethod::CrossCorrelationPostAlignment));
}

#[test]
fn signal_processing_baseline_covers_fft_temporal_math_and_events() {
    let operations = baseline_signal_processing_operations();

    assert!(operations.contains(&SignalProcessingOperation::Fft));
    assert!(operations.contains(&SignalProcessingOperation::Ifft));
    assert!(operations.contains(&SignalProcessingOperation::TimeDomainFilter));
    assert!(operations.contains(&SignalProcessingOperation::ChannelArithmetic));
    assert!(operations.contains(&SignalProcessingOperation::MathExpression));
    assert!(operations.contains(&SignalProcessingOperation::HarmonicAnalysis));
    assert!(operations.contains(&SignalProcessingOperation::InrushAnalysis));
    assert!(operations.contains(&SignalProcessingOperation::EventCounting));
    assert!(operations.contains(&SignalProcessingOperation::EdgeTiming));
}

#[test]
fn cem_time_domain_test_families_include_railway_axle_counter_and_inrush() {
    let families = [
        CemTimeDomainTestFamily::RailwayHarmonics,
        CemTimeDomainTestFamily::AxleCounter,
        CemTimeDomainTestFamily::InrushCurrent,
    ];

    assert!(families.contains(&CemTimeDomainTestFamily::RailwayHarmonics));
    assert!(families.contains(&CemTimeDomainTestFamily::AxleCounter));
    assert!(families.contains(&CemTimeDomainTestFamily::InrushCurrent));
}

#[test]
fn campaign_trace_starts_with_the_baseline_requirements() {
    let code = ProjectCode::parse("CEM-2026-001").unwrap();
    let trace = CampaignTrace::new(code);

    assert!(trace.is_baseline_complete());
    assert_eq!(trace.requirements().len(), 11);
}

fn reference_receiver(code: InstrumentCode) -> InstrumentRecord {
    InstrumentRecord::new(
        code,
        InstrumentFamily::Receiver,
        "Nexio Lab",
        "Reference Receiver",
        "SN-001",
        CalibrationRequirement::Required,
    )
    .unwrap()
}

fn signed_snapshot(domain: RepositoryDomain) -> RepositorySnapshot {
    snapshot_with_signature(domain, true)
}

fn snapshot_with_signature(domain: RepositoryDomain, signed: bool) -> RepositorySnapshot {
    RepositorySnapshot::new(
        domain,
        RepositorySnapshotId::parse(format!("{}-v1", domain.as_str())).unwrap(),
        1,
        SnapshotChecksum::parse(format!("sha256:{}", domain.as_str())).unwrap(),
        signed,
    )
    .unwrap()
}

fn accepted_measurement_plan(run_reference: &str) -> MeasurementRunPlan {
    let project = ProjectCode::parse("CEM-2026-001").unwrap();
    let code = InstrumentCode::parse("RX-001").unwrap();
    let mut registry = MetrologyRegistry::new();
    registry
        .register_instrument(reference_receiver(code.clone()))
        .unwrap();
    registry
        .record_calibration(
            CalibrationRecord::new(
                code.clone(),
                "CERT-2026-001",
                MetrologyDate::new(2026, 1, 1).unwrap(),
                MetrologyDate::new(2027, 1, 1).unwrap(),
                "Accredited Provider",
            )
            .unwrap(),
        )
        .unwrap();

    MeasurementRunPlan::plan(
        project,
        MeasurementRunReference::parse(run_reference).unwrap(),
        TestMethodReference::parse("EN61000-4-6").unwrap(),
        ExecutionMode::Accredited,
        vec![code],
        &registry,
        MetrologyDate::new(2026, 6, 27).unwrap(),
    )
    .unwrap()
}
