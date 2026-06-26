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
fn update_policy_requires_signed_packages_and_blocks_live_measurement_updates() {
    let policy = UpdatePolicy::laboratory_default();

    assert!(policy.signed_packages_required());
    assert!(policy.offline_install_allowed());
    assert!(!policy.apply_during_measurement_allowed());
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
