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
fn sync_conflict_id_rejects_empty_and_unsafe_values() {
    assert_eq!(
        SyncConflictId::parse(" ").unwrap_err(),
        DomainError::EmptySyncConflictId
    );
    assert_eq!(
        SyncConflictId::parse("sync conflict").unwrap_err(),
        DomainError::InvalidSyncConflictId("sync conflict".to_owned())
    );

    assert_eq!(
        SyncConflictId::parse("conflict-001").unwrap().as_str(),
        "conflict-001"
    );
}

#[test]
fn sync_conflict_record_starts_open_with_snapshot_context() {
    let conflict = SyncConflictRecord::new(
        SyncConflictId::parse("conflict-001").unwrap(),
        RepositoryDomain::ProjectRecords,
        SyncConflictKind::ConcurrentUpdate,
        RepositorySnapshotId::parse("local-v1").unwrap(),
        RepositorySnapshotId::parse("reference-v2").unwrap(),
    );

    assert_eq!(conflict.id().as_str(), "conflict-001");
    assert_eq!(conflict.domain(), RepositoryDomain::ProjectRecords);
    assert_eq!(conflict.kind(), SyncConflictKind::ConcurrentUpdate);
    assert_eq!(conflict.local_snapshot().as_str(), "local-v1");
    assert_eq!(conflict.reference_snapshot().as_str(), "reference-v2");
    assert_eq!(conflict.status(), SyncConflictStatus::Open);
    assert_eq!(conflict.resolution(), None);
}

#[test]
fn sync_conflict_record_can_be_resolved_once() {
    let mut conflict = SyncConflictRecord::new(
        SyncConflictId::parse("conflict-001").unwrap(),
        RepositoryDomain::ProjectRecords,
        SyncConflictKind::ConcurrentUpdate,
        RepositorySnapshotId::parse("local-v1").unwrap(),
        RepositorySnapshotId::parse("reference-v2").unwrap(),
    );

    conflict
        .resolve(SyncConflictResolution::ManualMerge)
        .unwrap();
    let error = conflict
        .resolve(SyncConflictResolution::KeepLocal)
        .unwrap_err();

    assert_eq!(conflict.status(), SyncConflictStatus::Resolved);
    assert_eq!(
        conflict.resolution(),
        Some(SyncConflictResolution::ManualMerge)
    );
    assert_eq!(
        error,
        DomainError::SyncConflictAlreadyResolved("conflict-001".to_owned())
    );
}

#[test]
fn sync_conflict_record_can_be_deferred_for_later_review() {
    let mut conflict = SyncConflictRecord::new(
        SyncConflictId::parse("conflict-001").unwrap(),
        RepositoryDomain::MeasurementData,
        SyncConflictKind::ChecksumMismatch,
        RepositorySnapshotId::parse("local-v1").unwrap(),
        RepositorySnapshotId::parse("reference-v1").unwrap(),
    );

    conflict.resolve(SyncConflictResolution::Defer).unwrap();

    assert_eq!(conflict.status(), SyncConflictStatus::Deferred);
    assert_eq!(conflict.resolution(), Some(SyncConflictResolution::Defer));
}

#[test]
fn sync_conflict_action_plan_maps_resolution_to_action() {
    let conflict = sync_conflict("conflict-001", SyncConflictKind::ConcurrentUpdate);

    let plan = SyncConflictActionPlan::new(&conflict, SyncConflictResolution::ManualMerge).unwrap();

    assert_eq!(plan.conflict_id().as_str(), "conflict-001");
    assert_eq!(plan.domain(), RepositoryDomain::ProjectRecords);
    assert_eq!(plan.kind(), SyncConflictKind::ConcurrentUpdate);
    assert_eq!(plan.resolution(), SyncConflictResolution::ManualMerge);
    assert_eq!(plan.action(), SyncAction::ManualMerge);
    assert_eq!(plan.action().as_str(), "manual_merge");
    assert_eq!(plan.local_snapshot().as_str(), "local-v1");
    assert_eq!(plan.reference_snapshot().as_str(), "reference-v2");
    assert!(plan.requires_audit_event());
}

#[test]
fn sync_conflict_service_applies_resolution_and_updates_record() {
    let conflict_id = SyncConflictId::parse("conflict-001").unwrap();
    let mut service = SyncConflictService::new(vec![sync_conflict(
        conflict_id.as_str(),
        SyncConflictKind::ChecksumMismatch,
    )]);

    let plan = service
        .apply_resolution(&conflict_id, SyncConflictResolution::KeepReference)
        .unwrap();

    assert_eq!(plan.action(), SyncAction::PullReferenceSnapshot);
    assert_eq!(
        service.conflicts()[0].status(),
        SyncConflictStatus::Resolved
    );
    assert_eq!(
        service.conflicts()[0].resolution(),
        Some(SyncConflictResolution::KeepReference)
    );
    assert!(service.pending_conflicts().is_empty());
}

#[test]
fn sync_conflict_service_can_defer_conflicts_for_review() {
    let conflict_id = SyncConflictId::parse("conflict-001").unwrap();
    let mut service = SyncConflictService::new(vec![sync_conflict(
        conflict_id.as_str(),
        SyncConflictKind::SchemaMismatch,
    )]);

    let plan = service
        .apply_resolution(&conflict_id, SyncConflictResolution::Defer)
        .unwrap();

    assert_eq!(plan.action(), SyncAction::DeferForReview);
    assert_eq!(
        service.conflicts()[0].status(),
        SyncConflictStatus::Deferred
    );
    assert_eq!(service.pending_conflicts().len(), 1);
}

#[test]
fn sync_conflict_service_rejects_unknown_conflict() {
    let service = SyncConflictService::new(vec![sync_conflict(
        "conflict-001",
        SyncConflictKind::ChecksumMismatch,
    )]);
    let missing = SyncConflictId::parse("missing-conflict").unwrap();

    let error = service
        .plan_resolution(&missing, SyncConflictResolution::KeepLocal)
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::UnknownSyncConflict("missing-conflict".to_owned())
    );
}

#[test]
fn sync_conflict_action_plan_rejects_invalid_resolution_for_kind() {
    let conflict = sync_conflict("conflict-001", SyncConflictKind::ConcurrentUpdate);

    let error =
        SyncConflictActionPlan::new(&conflict, SyncConflictResolution::AcceptDeletion).unwrap_err();

    assert_eq!(
        error,
        DomainError::InvalidSyncConflictResolution {
            conflict: "conflict-001".to_owned(),
            kind: "concurrent_update".to_owned(),
            resolution: "accept_deletion".to_owned(),
        }
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
fn transport_endpoint_rejects_empty_addresses() {
    let error = InstrumentTransportEndpoint::new(InstrumentTransport::TcpIp, " ").unwrap_err();

    assert_eq!(error, DomainError::EmptyTransportEndpointAddress);

    let endpoint =
        InstrumentTransportEndpoint::new(InstrumentTransport::TcpIp, "TCPIP::192.0.2.10").unwrap();
    assert_eq!(endpoint.transport(), InstrumentTransport::TcpIp);
    assert_eq!(endpoint.address(), "TCPIP::192.0.2.10");
}

#[test]
fn transport_timeout_policy_rejects_zero_timeouts() {
    let error = TransportTimeoutPolicy::new(0, 5_000, 1).unwrap_err();

    assert_eq!(
        error,
        DomainError::InvalidTransportTimeoutPolicy {
            connect_timeout_ms: 0,
            response_timeout_ms: 5_000,
            max_retries: 1,
        }
    );

    let policy = TransportTimeoutPolicy::laboratory_default();
    assert_eq!(policy.connect_timeout_ms(), 2_000);
    assert_eq!(policy.response_timeout_ms(), 5_000);
    assert_eq!(policy.max_retries(), 1);
}

#[test]
fn serial_endpoint_settings_parse_default_and_explicit_framing() {
    let default = SerialEndpointSettings::parse("COM3:115200").unwrap();
    assert_eq!(default.port(), "COM3");
    assert_eq!(default.baud_rate(), 115_200);
    assert_eq!(default.data_bits(), 8);
    assert_eq!(default.parity(), SerialParity::None);
    assert_eq!(default.parity().as_str(), "none");
    assert_eq!(default.stop_bits(), SerialStopBits::One);
    assert_eq!(default.stop_bits().value(), 1);

    let explicit = SerialEndpointSettings::parse("COM4:9600:7E2").unwrap();
    assert_eq!(explicit.port(), "COM4");
    assert_eq!(explicit.baud_rate(), 9_600);
    assert_eq!(explicit.data_bits(), 7);
    assert_eq!(explicit.parity(), SerialParity::Even);
    assert_eq!(explicit.stop_bits(), SerialStopBits::Two);

    let linux_port = SerialEndpointSettings::parse("/dev/ttyUSB0:115200:8N1").unwrap();
    assert_eq!(linux_port.port(), "/dev/ttyUSB0");
    assert_eq!(linux_port.baud_rate(), 115_200);
}

#[test]
fn serial_endpoint_settings_reject_invalid_addresses() {
    assert_eq!(
        SerialEndpointSettings::parse("COM3").unwrap_err(),
        DomainError::InvalidSerialEndpointAddress("COM3".to_owned())
    );
    assert_eq!(
        SerialEndpointSettings::parse("COM3:0").unwrap_err(),
        DomainError::InvalidSerialEndpointAddress("COM3:0".to_owned())
    );
    assert_eq!(
        SerialEndpointSettings::parse("COM3:115200:9N1").unwrap_err(),
        DomainError::InvalidSerialEndpointAddress("COM3:115200:9N1".to_owned())
    );
    assert_eq!(
        SerialEndpointSettings::parse("COM3:115200:8X1").unwrap_err(),
        DomainError::InvalidSerialEndpointAddress("COM3:115200:8X1".to_owned())
    );
    assert_eq!(
        SerialEndpointSettings::parse("COM 3:115200").unwrap_err(),
        DomainError::InvalidSerialEndpointAddress("COM 3:115200".to_owned())
    );
    assert_eq!(
        SerialEndpointSettings::parse("TCPIP0:115200").unwrap_err(),
        DomainError::InvalidSerialEndpointAddress("TCPIP0:115200".to_owned())
    );
    assert_eq!(
        SerialEndpointSettings::parse("GPIB0:115200").unwrap_err(),
        DomainError::InvalidSerialEndpointAddress("GPIB0:115200".to_owned())
    );
    assert_eq!(
        SerialEndpointSettings::parse("USB0:115200").unwrap_err(),
        DomainError::InvalidSerialEndpointAddress("USB0:115200".to_owned())
    );
    assert_eq!(
        SerialEndpointSettings::parse("ASRL3:115200").unwrap_err(),
        DomainError::InvalidSerialEndpointAddress("ASRL3:115200".to_owned())
    );
}

#[test]
fn visa_resource_address_parses_common_interfaces() {
    let tcpip = VisaResourceAddress::parse("TCPIP0::192.0.2.10::inst0::INSTR").unwrap();
    assert_eq!(tcpip.raw(), "TCPIP0::192.0.2.10::inst0::INSTR");
    assert_eq!(tcpip.interface(), VisaInterface::TcpIp);
    assert_eq!(tcpip.interface().as_str(), "tcp_ip");
    assert_eq!(tcpip.resource_class(), "INSTR");

    let tcp_socket = VisaResourceAddress::parse("TCPIP0::192.0.2.10::5025::SOCKET").unwrap();
    assert_eq!(tcp_socket.interface(), VisaInterface::TcpIp);
    assert_eq!(tcp_socket.resource_class(), "SOCKET");

    let gpib = VisaResourceAddress::parse("GPIB0::12::INSTR").unwrap();
    assert_eq!(gpib.interface(), VisaInterface::Gpib);

    let gpib_secondary = VisaResourceAddress::parse("GPIB0::12::1::INSTR").unwrap();
    assert_eq!(gpib_secondary.interface(), VisaInterface::Gpib);

    let serial = VisaResourceAddress::parse("ASRL3::INSTR").unwrap();
    assert_eq!(serial.interface(), VisaInterface::Serial);
}

#[test]
fn visa_resource_address_rejects_unknown_or_incomplete_resources() {
    assert_eq!(
        VisaResourceAddress::parse("192.0.2.10").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("192.0.2.10".to_owned())
    );
    assert_eq!(
        VisaResourceAddress::parse("PXI0::1::INSTR").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("PXI0::1::INSTR".to_owned())
    );
    assert_eq!(
        VisaResourceAddress::parse("TCPIP0::192.0.2.10::RAW").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("TCPIP0::192.0.2.10::RAW".to_owned())
    );
    assert_eq!(
        VisaResourceAddress::parse("TCPIP0::192.0.2.10::SOCKET").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("TCPIP0::192.0.2.10::SOCKET".to_owned())
    );
    assert_eq!(
        VisaResourceAddress::parse("TCPIP0::192.0.2.10::0::SOCKET").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("TCPIP0::192.0.2.10::0::SOCKET".to_owned())
    );
    assert_eq!(
        VisaResourceAddress::parse("GPIB0::12::SOCKET").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("GPIB0::12::SOCKET".to_owned())
    );
    assert_eq!(
        VisaResourceAddress::parse("GPIB0::primary::INSTR").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("GPIB0::primary::INSTR".to_owned())
    );
    assert_eq!(
        VisaResourceAddress::parse("GPIB0::31::INSTR").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("GPIB0::31::INSTR".to_owned())
    );
    assert_eq!(
        VisaResourceAddress::parse("GPIB0::12::31::INSTR").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("GPIB0::12::31::INSTR".to_owned())
    );
    assert_eq!(
        VisaResourceAddress::parse("ASRL::INSTR").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("ASRL::INSTR".to_owned())
    );
    assert_eq!(
        VisaResourceAddress::parse("TCPIPX::192.0.2.10::INSTR").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("TCPIPX::192.0.2.10::INSTR".to_owned())
    );
}

#[test]
fn concrete_transport_adapters_validate_endpoint_transport() {
    let endpoint =
        InstrumentTransportEndpoint::new(InstrumentTransport::TcpIp, "TCPIP::192.0.2.10").unwrap();

    let error = VisaTransportAdapter::new(endpoint, TransportTimeoutPolicy::laboratory_default())
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::TransportAdapterMismatch {
            expected: "visa".to_owned(),
            actual: "tcp_ip".to_owned(),
        }
    );
}

#[test]
fn visa_transport_adapter_exposes_validated_resource() {
    let endpoint = InstrumentTransportEndpoint::new(
        InstrumentTransport::Visa,
        "USB0::0x1234::0x5678::SN001::INSTR",
    )
    .unwrap();
    let adapter =
        VisaTransportAdapter::new(endpoint, TransportTimeoutPolicy::laboratory_default()).unwrap();

    assert_eq!(adapter.resource().interface(), VisaInterface::Usb);
    assert_eq!(adapter.resource().resource_class(), "INSTR");
    assert_eq!(
        adapter.resource().raw(),
        "USB0::0x1234::0x5678::SN001::INSTR"
    );
}

#[test]
fn visa_transport_adapter_exchanges_tcpip_socket_resources() {
    use std::io::{Read, Write};

    let code = InstrumentCode::parse("RX-001").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0_u8; 64];
        let read = stream.read(&mut buffer).unwrap();
        assert_eq!(std::str::from_utf8(&buffer[..read]).unwrap(), "*IDN?\n");
        stream.write_all(b"EMC LOCUS,VISA TCPIP FIXTURE\n").unwrap();
    });

    let endpoint = InstrumentTransportEndpoint::new(
        InstrumentTransport::Visa,
        format!("TCPIP0::{}::{}::SOCKET", address.ip(), address.port()),
    )
    .unwrap();
    let timeout_policy = TransportTimeoutPolicy::new(100, 1_000, 0).unwrap();
    let mut adapter = VisaTransportAdapter::new(endpoint, timeout_policy).unwrap();

    let response = adapter
        .exchange(&InstrumentCommand::new(
            code,
            InstrumentTransport::Visa,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap();

    assert_eq!(response.as_str(), "EMC LOCUS,VISA TCPIP FIXTURE");
    assert_eq!(adapter.last_exchange_attempt_count(), 1);
    handle.join().unwrap();
}

#[test]
fn visa_transport_adapter_keeps_non_tcpip_io_unavailable() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let endpoint = InstrumentTransportEndpoint::new(
        InstrumentTransport::Visa,
        "USB0::0x1234::0x5678::SN001::INSTR",
    )
    .unwrap();
    let mut adapter =
        VisaTransportAdapter::new(endpoint, TransportTimeoutPolicy::laboratory_default()).unwrap();

    let error = adapter
        .exchange(&InstrumentCommand::new(
            code,
            InstrumentTransport::Visa,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap_err();

    assert_eq!(adapter.last_exchange_attempt_count(), 1);
    assert_eq!(
        error,
        DomainError::ExternalTransportExchangeUnavailable {
            transport: "visa".to_owned(),
            address: "USB0::0x1234::0x5678::SN001::INSTR".to_owned(),
        }
    );
}

#[test]
fn tcp_ip_transport_adapter_reports_external_exchange_unavailable() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let endpoint =
        InstrumentTransportEndpoint::new(InstrumentTransport::TcpIp, "TCPIP::192.0.2.10").unwrap();
    let timeout_policy = TransportTimeoutPolicy::new(10, 10, 0).unwrap();
    let mut adapter = TcpIpTransportAdapter::new(endpoint, timeout_policy).unwrap();

    let error = adapter
        .exchange(&InstrumentCommand::new(
            code,
            InstrumentTransport::TcpIp,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap_err();

    assert_eq!(adapter.transport(), InstrumentTransport::TcpIp);
    assert_eq!(adapter.last_exchange_attempt_count(), 1);
    assert_eq!(
        error,
        DomainError::ExternalTransportExchangeUnavailable {
            transport: "tcp_ip".to_owned(),
            address: "TCPIP::192.0.2.10".to_owned(),
        }
    );
}

#[test]
fn tcp_ip_transport_adapter_exchanges_with_local_socket() {
    use std::io::{Read, Write};

    let code = InstrumentCode::parse("RX-001").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0_u8; 64];
        let read = stream.read(&mut buffer).unwrap();
        assert_eq!(std::str::from_utf8(&buffer[..read]).unwrap(), "*IDN?\n");
        stream.write_all(b"EMC LOCUS,TCP FIXTURE\n").unwrap();
    });

    let endpoint = InstrumentTransportEndpoint::new(
        InstrumentTransport::TcpIp,
        format!("TCPIP::{}::{}", address.ip(), address.port()),
    )
    .unwrap();
    let timeout_policy = TransportTimeoutPolicy::new(100, 1_000, 0).unwrap();
    let mut adapter = TcpIpTransportAdapter::new(endpoint, timeout_policy).unwrap();

    let response = adapter
        .exchange(&InstrumentCommand::new(
            code,
            InstrumentTransport::TcpIp,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap();

    assert_eq!(response.as_str(), "EMC LOCUS,TCP FIXTURE");
    assert_eq!(adapter.last_exchange_attempt_count(), 1);
    handle.join().unwrap();
}

#[test]
fn tcp_ip_transport_adapter_accepts_visa_socket_resource() {
    use std::io::{Read, Write};

    let code = InstrumentCode::parse("RX-001").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0_u8; 64];
        let read = stream.read(&mut buffer).unwrap();
        assert_eq!(std::str::from_utf8(&buffer[..read]).unwrap(), "SYST:ERR?\n");
        stream.write_all(b"0,No error\n").unwrap();
    });

    let endpoint = InstrumentTransportEndpoint::new(
        InstrumentTransport::TcpIp,
        format!("TCPIP0::{}::{}::SOCKET", address.ip(), address.port()),
    )
    .unwrap();
    let timeout_policy = TransportTimeoutPolicy::new(100, 1_000, 0).unwrap();
    let mut adapter = TcpIpTransportAdapter::new(endpoint, timeout_policy).unwrap();

    let response = adapter
        .exchange(&InstrumentCommand::new(
            code,
            InstrumentTransport::TcpIp,
            InstrumentCommandMessage::parse("SYST:ERR?").unwrap(),
        ))
        .unwrap();

    assert_eq!(response.as_str(), "0,No error");
    assert_eq!(adapter.last_exchange_attempt_count(), 1);
    handle.join().unwrap();
}

#[test]
fn tcp_ip_socket_target_accepts_visa_instr_and_plain_forms() {
    assert_eq!(
        tcp_socket_target("TCPIP0::192.0.2.10::inst0::INSTR").unwrap(),
        "192.0.2.10:5025"
    );
    assert_eq!(
        tcp_socket_target("TCPIP0::192.0.2.10::5026::SOCKET").unwrap(),
        "192.0.2.10:5026"
    );
    assert_eq!(
        tcp_socket_target("TCPIP::192.0.2.11::5026").unwrap(),
        "192.0.2.11:5026"
    );
    assert_eq!(tcp_socket_target("192.0.2.12").unwrap(), "192.0.2.12:5025");
}

#[test]
fn tcp_ip_socket_target_rejects_malformed_visa_resources() {
    assert_eq!(
        tcp_socket_target("TCPIP0::::5025::SOCKET").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("TCPIP0::::5025::SOCKET".to_owned())
    );
    assert_eq!(
        tcp_socket_target("TCPIP0::192.0.2.10::inst0::SOCKET").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("TCPIP0::192.0.2.10::inst0::SOCKET".to_owned())
    );
    assert_eq!(
        tcp_socket_target("TCPIP0::192.0.2.10::0::SOCKET").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("TCPIP0::192.0.2.10::0::SOCKET".to_owned())
    );
    assert_eq!(
        tcp_socket_target("TCPIP0::192.0.2.10::RAW").unwrap_err(),
        DomainError::InvalidVisaResourceAddress("TCPIP0::192.0.2.10::RAW".to_owned())
    );
}

#[test]
fn tcp_ip_transport_adapter_tracks_failed_retry_attempts() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let endpoint = InstrumentTransportEndpoint::new(
        InstrumentTransport::TcpIp,
        format!("TCPIP::{}::{}", address.ip(), address.port()),
    )
    .unwrap();
    let timeout_policy = TransportTimeoutPolicy::new(10, 10, 2).unwrap();
    let mut adapter = TcpIpTransportAdapter::new(endpoint, timeout_policy).unwrap();

    let error = adapter
        .exchange(&InstrumentCommand::new(
            code,
            InstrumentTransport::TcpIp,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap_err();

    assert_eq!(adapter.last_exchange_attempt_count(), 3);
    assert_eq!(
        error,
        DomainError::ExternalTransportExchangeUnavailable {
            transport: "tcp_ip".to_owned(),
            address: format!("TCPIP::{}::{}", address.ip(), address.port()),
        }
    );
}

#[test]
fn transport_adapter_runtime_does_not_fake_concrete_io() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let endpoint =
        InstrumentTransportEndpoint::new(InstrumentTransport::Serial, "COM3:115200").unwrap();
    let adapter =
        SerialTransportAdapter::new(endpoint, TransportTimeoutPolicy::laboratory_default())
            .unwrap();
    let mut runtime = TransportAdapterRuntime::new(code.clone(), adapter);

    let error = runtime
        .execute(InstrumentCommand::new(
            code,
            InstrumentTransport::Serial,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::ExternalTransportExchangeUnavailable {
            transport: "serial".to_owned(),
            address: "COM3:115200".to_owned(),
        }
    );
    assert!(runtime.observations().is_empty());
}

#[test]
fn serial_transport_adapter_exposes_validated_settings() {
    let endpoint =
        InstrumentTransportEndpoint::new(InstrumentTransport::Serial, "COM7:57600:8O1").unwrap();
    let adapter =
        SerialTransportAdapter::new(endpoint, TransportTimeoutPolicy::laboratory_default())
            .unwrap();

    assert_eq!(adapter.settings().port(), "COM7");
    assert_eq!(adapter.settings().baud_rate(), 57_600);
    assert_eq!(adapter.settings().data_bits(), 8);
    assert_eq!(adapter.settings().parity(), SerialParity::Odd);
    assert_eq!(adapter.settings().stop_bits(), SerialStopBits::One);
}

#[test]
fn simulated_transport_adapter_returns_deterministic_exchange() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let endpoint =
        InstrumentTransportEndpoint::new(InstrumentTransport::Simulated, "SIM::RX-001").unwrap();
    let mut adapter = SimulatedTransportAdapter::new(endpoint);

    let response = adapter
        .exchange(&InstrumentCommand::new(
            code,
            InstrumentTransport::Simulated,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap();

    assert_eq!(adapter.transport(), InstrumentTransport::Simulated);
    assert_eq!(adapter.endpoint().address(), "SIM::RX-001");
    assert_eq!(response.as_str(), "SIM:*IDN?=0");
}

#[test]
fn transport_adapter_runtime_records_observations() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let endpoint =
        InstrumentTransportEndpoint::new(InstrumentTransport::TcpIp, "TCPIP::192.0.2.10").unwrap();
    let adapter = SimulatedTransportAdapter::new(endpoint);
    let mut runtime = TransportAdapterRuntime::new(code.clone(), adapter);

    let observation = runtime
        .execute(InstrumentCommand::new(
            code.clone(),
            InstrumentTransport::TcpIp,
            InstrumentCommandMessage::parse("FREQ 1000000").unwrap(),
        ))
        .unwrap()
        .clone();

    assert_eq!(runtime.instrument(), &code);
    assert_eq!(runtime.adapter().endpoint().address(), "TCPIP::192.0.2.10");
    assert_eq!(runtime.observations().len(), 1);
    assert_eq!(observation.sequence(), 1);
    assert_eq!(observation.exchange_attempts(), 1);
    assert_eq!(
        observation.command().transport(),
        InstrumentTransport::TcpIp
    );
    assert_eq!(observation.response().as_str(), "OK:FREQ 1000000");
}

#[test]
fn transport_adapter_runtime_rejects_transport_mismatch() {
    let code = InstrumentCode::parse("RX-001").unwrap();
    let endpoint =
        InstrumentTransportEndpoint::new(InstrumentTransport::Simulated, "SIM::RX-001").unwrap();
    let adapter = SimulatedTransportAdapter::new(endpoint);
    let mut runtime = TransportAdapterRuntime::new(code.clone(), adapter);

    let error = runtime
        .execute(InstrumentCommand::new(
            code,
            InstrumentTransport::TcpIp,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::TransportAdapterMismatch {
            expected: "simulated".to_owned(),
            actual: "tcp_ip".to_owned(),
        }
    );
    assert!(runtime.observations().is_empty());
}

#[test]
fn transport_adapter_runtime_applies_safety_limits_before_exchange() {
    let code = InstrumentCode::parse("GEN-001").unwrap();
    let endpoint =
        InstrumentTransportEndpoint::new(InstrumentTransport::TcpIp, "TCPIP::192.0.2.20").unwrap();
    let adapter = SimulatedTransportAdapter::new(endpoint);
    let mut runtime = TransportAdapterRuntime::new(code.clone(), adapter);
    runtime.add_safety_limit(
        InstrumentSafetyLimit::new(InstrumentQuantity::LevelDbm, -120, 10).unwrap(),
    );

    let error = runtime
        .execute(InstrumentCommand::with_setpoint(
            code,
            InstrumentTransport::TcpIp,
            InstrumentCommandMessage::parse("POW 20").unwrap(),
            InstrumentSetpoint::new(InstrumentQuantity::LevelDbm, 20),
        ))
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::InstrumentSetpointOutOfRange {
            quantity: "level_dbm".to_owned(),
            value: 20,
            minimum: -120,
            maximum: 10,
        }
    );
    assert_eq!(runtime.safety_limits().len(), 1);
    assert!(runtime.observations().is_empty());
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
    assert_eq!(first.exchange_attempts(), 1);
    assert_eq!(second.sequence(), 2);
    assert_eq!(second.response().as_str(), "OK:FREQ 1000000");
    assert_eq!(second.command().transport(), InstrumentTransport::Visa);
    assert_eq!(second.exchange_attempts(), 1);
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
fn instrument_safety_limit_rejects_inverted_ranges() {
    let error = InstrumentSafetyLimit::new(InstrumentQuantity::LevelDbm, 10, -10).unwrap_err();

    assert_eq!(
        error,
        DomainError::InvalidInstrumentSafetyLimit {
            quantity: "level_dbm".to_owned(),
            minimum: 10,
            maximum: -10,
        }
    );
}

#[test]
fn simulated_instrument_runtime_allows_setpoints_inside_known_limits() {
    let code = InstrumentCode::parse("GEN-001").unwrap();
    let mut runtime =
        SimulatedInstrumentRuntime::new(code.clone(), vec![InstrumentTransport::Simulated]);
    runtime.add_safety_limit(
        InstrumentSafetyLimit::new(InstrumentQuantity::LevelDbm, -120, 10).unwrap(),
    );

    let observation = runtime
        .execute(InstrumentCommand::with_setpoint(
            code,
            InstrumentTransport::Simulated,
            InstrumentCommandMessage::parse("POW -20").unwrap(),
            InstrumentSetpoint::new(InstrumentQuantity::LevelDbm, -20),
        ))
        .unwrap()
        .clone();

    assert_eq!(runtime.safety_limits().len(), 1);
    assert_eq!(observation.response().as_str(), "OK:POW -20");
    assert_eq!(runtime.observations().len(), 1);
}

#[test]
fn simulated_instrument_runtime_blocks_setpoints_outside_known_limits() {
    let code = InstrumentCode::parse("GEN-001").unwrap();
    let mut runtime =
        SimulatedInstrumentRuntime::new(code.clone(), vec![InstrumentTransport::Simulated]);
    runtime.add_safety_limit(
        InstrumentSafetyLimit::new(InstrumentQuantity::LevelDbm, -120, 10).unwrap(),
    );

    let error = runtime
        .execute(InstrumentCommand::with_setpoint(
            code,
            InstrumentTransport::Simulated,
            InstrumentCommandMessage::parse("POW 20").unwrap(),
            InstrumentSetpoint::new(InstrumentQuantity::LevelDbm, 20),
        ))
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::InstrumentSetpointOutOfRange {
            quantity: "level_dbm".to_owned(),
            value: 20,
            minimum: -120,
            maximum: 10,
        }
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
fn update_package_identity_signature_and_compatibility_are_validated() {
    assert_eq!(
        UpdatePackageName::parse(" ").unwrap_err(),
        DomainError::EmptyUpdatePackageName
    );
    assert_eq!(
        UpdatePackageName::parse("emc core").unwrap_err(),
        DomainError::InvalidUpdatePackageName("emc core".to_owned())
    );
    assert_eq!(
        UpdateSignature::parse("\t").unwrap_err(),
        DomainError::EmptyUpdateSignature
    );
    assert_eq!(
        RollbackReference::parse("").unwrap_err(),
        DomainError::EmptyRollbackReference
    );

    let error = VersionCompatibilityRange::new(
        SoftwareVersion::new(0, 2, 0),
        Some(SoftwareVersion::new(0, 1, 9)),
    )
    .unwrap_err();

    assert_eq!(
        error,
        DomainError::InvalidUpdateCompatibilityRange {
            minimum_version: "0.2.0".to_owned(),
            maximum_version: "0.1.9".to_owned(),
        }
    );
}

#[test]
fn signed_update_bundle_exposes_compatibility_and_rollback_metadata() {
    let bundle = sample_update_bundle(Some(UpdateSignature::parse("sig:core-020").unwrap()), true);
    let components = baseline_update_components();

    assert_eq!(bundle.name().as_str(), "emc-locus-core");
    assert_eq!(bundle.package_version(), SoftwareVersion::new(0, 2, 0));
    assert_eq!(bundle.component(), UpdateComponent::CoreApplication);
    assert_eq!(bundle.component().as_str(), "core_application");
    assert!(components.contains(&UpdateComponent::InstrumentDriver));
    assert!(components.contains(&UpdateComponent::SignalProcessingEngine));
    assert!(components.contains(&UpdateComponent::DatabaseMigration));
    assert!(bundle.signed());
    assert!(bundle.offline_install_allowed());
    assert_eq!(bundle.signature().unwrap().as_str(), "sig:core-020");
    assert_eq!(
        bundle.rollback_reference().unwrap().as_str(),
        "emc-locus-core-0.1.0"
    );
    assert!(bundle.is_compatible_with(&SoftwareVersion::new(0, 1, 0)));
    assert!(!bundle.is_compatible_with(&SoftwareVersion::new(0, 2, 0)));
}

#[test]
fn update_install_plan_accepts_signed_offline_bundle() {
    let bundle = sample_update_bundle(Some(UpdateSignature::parse("sig:core-020").unwrap()), true);

    let plan = UpdateInstallPlan::prepare(
        bundle,
        UpdatePolicy::laboratory_default(),
        SoftwareVersion::new(0, 1, 0),
        UpdateSource::OfflineBundle,
        false,
    )
    .unwrap();

    assert_eq!(plan.bundle().name().as_str(), "emc-locus-core");
    assert_eq!(plan.bundle().component(), UpdateComponent::CoreApplication);
    assert_eq!(plan.source(), UpdateSource::OfflineBundle);
    assert_eq!(plan.source().as_str(), "offline_bundle");
    assert_eq!(plan.installed_version(), SoftwareVersion::new(0, 1, 0));
    assert_eq!(
        plan.rollback_reference().unwrap().as_str(),
        "emc-locus-core-0.1.0"
    );
}

#[test]
fn update_install_plan_rejects_unsigned_required_package() {
    let bundle = sample_update_bundle(None, true);

    let error = UpdateInstallPlan::prepare(
        bundle,
        UpdatePolicy::laboratory_default(),
        SoftwareVersion::new(0, 1, 0),
        UpdateSource::OfflineBundle,
        false,
    )
    .unwrap_err();

    assert_eq!(
        error,
        DomainError::UnsignedUpdatePackage("emc-locus-core".to_owned())
    );
}

#[test]
fn update_install_plan_rejects_offline_bundle_when_catalog_disallows_it() {
    let bundle = sample_update_bundle(Some(UpdateSignature::parse("sig:core-020").unwrap()), false);

    let error = UpdateInstallPlan::prepare(
        bundle,
        UpdatePolicy::laboratory_default(),
        SoftwareVersion::new(0, 1, 0),
        UpdateSource::OfflineBundle,
        false,
    )
    .unwrap_err();

    assert_eq!(
        error,
        DomainError::OfflineUpdateInstallNotAllowed("emc-locus-core".to_owned())
    );
}

#[test]
fn update_install_plan_rejects_incompatible_installed_version() {
    let bundle = sample_update_bundle(Some(UpdateSignature::parse("sig:core-020").unwrap()), true);

    let error = UpdateInstallPlan::prepare(
        bundle,
        UpdatePolicy::laboratory_default(),
        SoftwareVersion::new(0, 2, 0),
        UpdateSource::OnlineCatalog,
        false,
    )
    .unwrap_err();

    assert_eq!(
        error,
        DomainError::IncompatibleUpdatePackage {
            package: "emc-locus-core".to_owned(),
            minimum_version: "0.1.0".to_owned(),
            maximum_version: Some("0.1.9".to_owned()),
            actual_version: "0.2.0".to_owned(),
        }
    );
}

#[test]
fn update_install_plan_blocks_live_measurement_updates() {
    let bundle = sample_update_bundle(Some(UpdateSignature::parse("sig:core-020").unwrap()), true);

    let error = UpdateInstallPlan::prepare(
        bundle,
        UpdatePolicy::laboratory_default(),
        SoftwareVersion::new(0, 1, 0),
        UpdateSource::OnlineCatalog,
        true,
    )
    .unwrap_err();

    assert_eq!(
        error,
        DomainError::UpdateDuringMeasurementBlocked("emc-locus-core".to_owned())
    );
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
fn dataset_retention_record_starts_retained_for_raw_data() {
    let dataset = raw_dataset_for_run("RUN-001");
    let retention = DatasetRetentionRecord::for_raw_dataset(&dataset);

    assert_eq!(retention.dataset().as_str(), "raw-signal-001");
    assert_eq!(retention.checksum().as_str(), "sha256:abc123");
    assert!(retention.immutable());
    assert_eq!(retention.status(), DatasetRetentionStatus::Retained);
    assert!(retention.events().is_empty());
}

#[test]
fn immutable_dataset_deletion_requires_reviewed_retention_transition() {
    let dataset = raw_dataset_for_run("RUN-001");
    let mut retention = DatasetRetentionRecord::for_raw_dataset(&dataset);

    let error = retention
        .mark_deleted(
            AuditActor::parse("data.manager").unwrap(),
            AuditReason::parse("Free disk space").unwrap(),
        )
        .unwrap_err();

    assert_eq!(
        error,
        DomainError::InvalidDatasetRetentionTransition {
            dataset: "raw-signal-001".to_owned(),
            from: "retained".to_owned(),
            to: "deleted".to_owned(),
        }
    );
    assert_eq!(retention.status(), DatasetRetentionStatus::Retained);
    assert!(retention.events().is_empty());
}

#[test]
fn dataset_retention_records_reviewed_deletion_workflow() {
    let dataset = raw_dataset_for_run("RUN-001");
    let mut retention = DatasetRetentionRecord::for_raw_dataset(&dataset);

    retention
        .request_deletion(
            AuditActor::parse("data.manager").unwrap(),
            AuditReason::parse("Retention period expired").unwrap(),
        )
        .unwrap();
    retention
        .approve_deletion(
            AuditActor::parse("quality.manager").unwrap(),
            AuditReason::parse("Reviewed raw-data lineage and backup").unwrap(),
        )
        .unwrap();
    retention
        .mark_deleted(
            AuditActor::parse("data.manager").unwrap(),
            AuditReason::parse("Approved deletion executed").unwrap(),
        )
        .unwrap();

    assert_eq!(retention.status(), DatasetRetentionStatus::Deleted);
    assert_eq!(retention.events().len(), 3);
    assert_eq!(
        retention.events()[0].status(),
        DatasetRetentionStatus::DeletionRequested
    );
    assert_eq!(retention.events()[0].actor().as_str(), "data.manager");
    assert_eq!(
        retention.events()[1].status(),
        DatasetRetentionStatus::DeletionApproved
    );
    assert_eq!(
        retention.events()[2].reason().as_str(),
        "Approved deletion executed"
    );
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
fn measurement_execution_session_rejects_unplanned_runtime_instrument() {
    let plan = accepted_measurement_plan("RUN-001");
    let runtime = SimulatedInstrumentRuntime::new(
        InstrumentCode::parse("GEN-001").unwrap(),
        vec![InstrumentTransport::Simulated],
    );

    let error = MeasurementExecutionSession::new(plan, runtime).unwrap_err();

    assert_eq!(
        error,
        DomainError::ExecutionInstrumentNotPlanned("GEN-001".to_owned())
    );
}

#[test]
fn measurement_execution_session_records_runtime_observations() {
    let plan = accepted_measurement_plan("RUN-001");
    let instrument = plan.equipment()[0].clone();
    let runtime =
        SimulatedInstrumentRuntime::new(instrument.clone(), vec![InstrumentTransport::Simulated]);
    let mut session = MeasurementExecutionSession::new(plan, runtime).unwrap();

    let observation = session
        .execute_command(InstrumentCommand::new(
            instrument,
            InstrumentTransport::Simulated,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap()
        .clone();

    assert_eq!(observation.sequence(), 1);
    assert_eq!(session.runtime().observations().len(), 1);
    assert_eq!(session.evidence().observations().len(), 1);
}

#[test]
fn measurement_execution_finish_requires_raw_data() {
    let plan = accepted_measurement_plan("RUN-001");
    let instrument = plan.equipment()[0].clone();
    let runtime = SimulatedInstrumentRuntime::new(instrument, vec![InstrumentTransport::Simulated]);
    let session = MeasurementExecutionSession::new(plan, runtime).unwrap();

    let error = session.finish().unwrap_err();

    assert_eq!(error, DomainError::MeasurementRunMissingRawData);
}

#[test]
fn measurement_execution_finish_returns_complete_evidence() {
    let plan = accepted_measurement_plan("RUN-001");
    let instrument = plan.equipment()[0].clone();
    let runtime =
        SimulatedInstrumentRuntime::new(instrument.clone(), vec![InstrumentTransport::Simulated]);
    let mut session = MeasurementExecutionSession::new(plan, runtime).unwrap();

    session
        .execute_command(InstrumentCommand::new(
            instrument,
            InstrumentTransport::Simulated,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap();
    session
        .record_raw_dataset(raw_dataset_for_run("RUN-001"))
        .unwrap();

    let evidence = session.finish().unwrap();

    assert_eq!(evidence.observations().len(), 1);
    assert_eq!(evidence.raw_datasets().len(), 1);
    assert!(evidence.has_raw_data());
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
fn report_export_bundle_requires_issued_report() {
    let project = ProjectCode::parse("CEM-2026-001").unwrap();
    let report = ReportPackage::new(
        project,
        ReportNumber::parse("RPT-001").unwrap(),
        ReportRevision::parse("A").unwrap(),
        ExecutionMode::Accredited,
    );

    let error = ReportExportBundle::from_issued_report(
        &report,
        ReportExportFormat::Pdf,
        DatasetFileReference::parse("reports/RPT-001-A.pdf").unwrap(),
        DatasetChecksum::parse("sha256:report123").unwrap(),
    )
    .unwrap_err();

    assert_eq!(error, DomainError::ReportMustBeIssuedBeforeExport);
}

#[test]
fn report_export_bundle_preserves_accredited_review_and_approval_evidence() {
    let project = ProjectCode::parse("CEM-2026-001").unwrap();
    let reviewer = AuditActor::parse("technical.reviewer").unwrap();
    let approver = AuditActor::parse("quality.manager").unwrap();
    let report = issued_accredited_report(project.clone(), reviewer.clone(), approver.clone());

    let bundle = ReportExportBundle::from_issued_report(
        &report,
        ReportExportFormat::Pdf,
        DatasetFileReference::parse("reports/RPT-001-A.pdf").unwrap(),
        DatasetChecksum::parse("sha256:report123").unwrap(),
    )
    .unwrap();

    assert_eq!(bundle.project(), &project);
    assert_eq!(bundle.number().as_str(), "RPT-001");
    assert_eq!(bundle.revision().as_str(), "A");
    assert_eq!(bundle.format(), ReportExportFormat::Pdf);
    assert_eq!(bundle.file_reference().as_str(), "reports/RPT-001-A.pdf");
    assert_eq!(bundle.checksum().as_str(), "sha256:report123");
    assert_eq!(bundle.reviewed_by(), Some(&reviewer));
    assert_eq!(bundle.approved_by(), Some(&approver));
}

#[test]
fn traceability_report_view_links_report_export_to_run_evidence() {
    let reviewer = AuditActor::parse("technical.reviewer").unwrap();
    let approver = AuditActor::parse("quality.manager").unwrap();
    let report = issued_accredited_report(
        ProjectCode::parse("CEM-2026-001").unwrap(),
        reviewer.clone(),
        approver.clone(),
    );
    let bundle = ReportExportBundle::from_issued_report(
        &report,
        ReportExportFormat::Pdf,
        DatasetFileReference::parse("reports/RPT-001-A.pdf").unwrap(),
        DatasetChecksum::parse("sha256:report123").unwrap(),
    )
    .unwrap();
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
        .record_raw_dataset(raw_dataset_for_run("RUN-001"))
        .unwrap();

    let view = TraceabilityReportView::from_export_bundle(&bundle, &[evidence]).unwrap();

    assert_eq!(view.project().as_str(), "CEM-2026-001");
    assert_eq!(view.report_number().as_str(), "RPT-001");
    assert_eq!(view.report_revision().as_str(), "A");
    assert_eq!(view.export_checksum().as_str(), "sha256:report123");
    assert_eq!(view.reviewed_by(), Some(&reviewer));
    assert_eq!(view.approved_by(), Some(&approver));
    assert!(view.has_technical_review());
    assert!(view.has_report_approval());
    assert!(view.has_raw_data_lineage());
    assert_eq!(view.requirements().len(), 11);
    assert_eq!(view.runs().len(), 1);
    assert_eq!(view.runs()[0].run().as_str(), "RUN-001");
    assert_eq!(view.runs()[0].method().as_str(), "EN61000-4-6");
    assert_eq!(view.runs()[0].equipment()[0].as_str(), "RX-001");
    assert_eq!(view.runs()[0].observation_count(), 1);
    assert_eq!(view.runs()[0].total_exchange_attempts(), 1);
    assert_eq!(view.runs()[0].max_exchange_attempts(), 1);
    assert_eq!(
        view.runs()[0].raw_datasets()[0].checksum().as_str(),
        "sha256:abc123"
    );
}

#[test]
fn traceability_report_view_summarizes_exchange_attempts() {
    #[derive(Clone, Debug)]
    struct AttemptFixtureAdapter {
        endpoint: InstrumentTransportEndpoint,
        attempts: u16,
    }

    impl InstrumentTransportAdapter for AttemptFixtureAdapter {
        fn endpoint(&self) -> &InstrumentTransportEndpoint {
            &self.endpoint
        }

        fn exchange(
            &mut self,
            _command: &InstrumentCommand,
        ) -> Result<InstrumentResponse, DomainError> {
            Ok(InstrumentResponse::received("OK".to_owned()))
        }

        fn last_exchange_attempt_count(&self) -> u16 {
            self.attempts
        }
    }

    let report = issued_accredited_report(
        ProjectCode::parse("CEM-2026-001").unwrap(),
        AuditActor::parse("technical.reviewer").unwrap(),
        AuditActor::parse("quality.manager").unwrap(),
    );
    let bundle = ReportExportBundle::from_issued_report(
        &report,
        ReportExportFormat::Pdf,
        DatasetFileReference::parse("reports/RPT-001-A.pdf").unwrap(),
        DatasetChecksum::parse("sha256:report123").unwrap(),
    )
    .unwrap();
    let plan = accepted_measurement_plan("RUN-001");
    let instrument = plan.equipment()[0].clone();
    let endpoint =
        InstrumentTransportEndpoint::new(InstrumentTransport::TcpIp, "TCPIP::127.0.0.1").unwrap();
    let adapter = AttemptFixtureAdapter {
        endpoint,
        attempts: 3,
    };
    let mut runtime = TransportAdapterRuntime::new(instrument.clone(), adapter);
    let observation = runtime
        .execute(InstrumentCommand::new(
            instrument,
            InstrumentTransport::TcpIp,
            InstrumentCommandMessage::parse("*IDN?").unwrap(),
        ))
        .unwrap()
        .clone();
    let mut evidence = MeasurementRunEvidence::new(plan);
    evidence.record_observation(observation);

    let view = TraceabilityReportView::from_export_bundle(&bundle, &[evidence]).unwrap();

    assert_eq!(view.runs()[0].observation_count(), 1);
    assert_eq!(view.runs()[0].total_exchange_attempts(), 3);
    assert_eq!(view.runs()[0].max_exchange_attempts(), 3);
}

#[test]
fn traceability_report_view_rejects_run_evidence_for_another_project() {
    let report = issued_accredited_report(
        ProjectCode::parse("CEM-OTHER").unwrap(),
        AuditActor::parse("technical.reviewer").unwrap(),
        AuditActor::parse("quality.manager").unwrap(),
    );
    let bundle = ReportExportBundle::from_issued_report(
        &report,
        ReportExportFormat::Pdf,
        DatasetFileReference::parse("reports/RPT-001-A.pdf").unwrap(),
        DatasetChecksum::parse("sha256:report123").unwrap(),
    )
    .unwrap();
    let evidence = MeasurementRunEvidence::new(accepted_measurement_plan("RUN-001"));

    let error = TraceabilityReportView::from_export_bundle(&bundle, &[evidence]).unwrap_err();

    assert_eq!(
        error,
        DomainError::TraceabilityProjectMismatch {
            expected: "CEM-OTHER".to_owned(),
            actual: "CEM-2026-001".to_owned(),
        }
    );
}

#[test]
fn report_export_bundle_allows_non_accredited_issue_without_approval() {
    let project = ProjectCode::parse("CEM-2026-001").unwrap();
    let mut report = ReportPackage::new(
        project,
        ReportNumber::parse("RPT-001").unwrap(),
        ReportRevision::parse("A").unwrap(),
        ExecutionMode::NonAccredited,
    );
    report.issue().unwrap();

    let bundle = ReportExportBundle::from_issued_report(
        &report,
        ReportExportFormat::Zip,
        DatasetFileReference::parse("reports/RPT-001-A.zip").unwrap(),
        DatasetChecksum::parse("sha256:reportzip").unwrap(),
    )
    .unwrap();

    assert_eq!(bundle.format(), ReportExportFormat::Zip);
    assert_eq!(bundle.approved_by(), None);
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
fn processing_graph_instance_preserves_revision_dataset_and_lineage() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let current_fft = SignalReference::parse("current_l1_fft").unwrap();
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

    let instance = ProcessingGraphInstance::new(
        ProcessingGraphReference::parse("inrush-analysis").unwrap(),
        ProcessingGraphRevision::parse("A").unwrap(),
        DatasetReference::parse("dataset-raw-inrush").unwrap(),
        DatasetChecksum::parse("sha256:rawinrush001").unwrap(),
        graph,
        DatasetChecksum::parse("sha256:graphinrush001").unwrap(),
        AuditActor::parse("signal.engineer").unwrap(),
        "0.1.0",
    )
    .unwrap();

    assert_eq!(instance.reference().as_str(), "inrush-analysis");
    assert_eq!(instance.revision().as_str(), "A");
    assert_eq!(instance.source_dataset().as_str(), "dataset-raw-inrush");
    assert_eq!(
        instance.source_dataset_checksum().as_str(),
        "sha256:rawinrush001"
    );
    assert_eq!(
        instance.definition_checksum().as_str(),
        "sha256:graphinrush001"
    );
    assert_eq!(instance.created_by().as_str(), "signal.engineer");
    assert_eq!(instance.software_version(), "0.1.0");
    assert!(instance.contains_operation(SignalProcessingOperation::Fft));
    assert_eq!(
        instance.raw_lineage_for(&current_fft).unwrap(),
        vec![current]
    );
}

#[test]
fn processing_graph_instance_rejects_empty_definition_and_software_version() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let empty_graph = SignalProcessingGraph::from_dataset(&dataset);

    let empty_error = ProcessingGraphInstance::new(
        ProcessingGraphReference::parse("empty-graph").unwrap(),
        ProcessingGraphRevision::parse("A").unwrap(),
        DatasetReference::parse("dataset-raw-inrush").unwrap(),
        DatasetChecksum::parse("sha256:rawinrush001").unwrap(),
        empty_graph,
        DatasetChecksum::parse("sha256:graphinrush001").unwrap(),
        AuditActor::parse("signal.engineer").unwrap(),
        "0.1.0",
    )
    .unwrap_err();

    assert_eq!(
        empty_error,
        DomainError::EmptyProcessingGraphDefinition("empty-graph".to_owned())
    );
    assert_eq!(
        ProcessingGraphReference::parse("bad reference").unwrap_err(),
        DomainError::InvalidProcessingGraphReference("bad reference".to_owned())
    );
    assert_eq!(
        ProcessingGraphRevision::parse(" ").unwrap_err(),
        DomainError::EmptyProcessingGraphRevision
    );

    let mut graph = SignalProcessingGraph::from_dataset(&dataset);
    let current = SignalReference::parse("current_l1").unwrap();
    graph
        .add_node(
            SignalProcessingNode::new(
                SignalReference::parse("fft_current").unwrap(),
                SignalProcessingOperation::Fft,
                vec![current],
                SignalReference::parse("current_l1_fft").unwrap(),
            )
            .unwrap(),
        )
        .unwrap();

    let software_error = ProcessingGraphInstance::new(
        ProcessingGraphReference::parse("inrush-analysis").unwrap(),
        ProcessingGraphRevision::parse("A").unwrap(),
        DatasetReference::parse("dataset-raw-inrush").unwrap(),
        DatasetChecksum::parse("sha256:rawinrush001").unwrap(),
        graph,
        DatasetChecksum::parse("sha256:graphinrush001").unwrap(),
        AuditActor::parse("signal.engineer").unwrap(),
        " ",
    )
    .unwrap_err();

    assert_eq!(
        software_error,
        DomainError::EmptyProcessingGraphSoftwareVersion
    );
}

#[test]
fn processing_graph_result_artifact_links_output_to_instance_lineage() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let current_fft = SignalReference::parse("current_l1_fft").unwrap();
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
    let instance = ProcessingGraphInstance::new(
        ProcessingGraphReference::parse("inrush-analysis").unwrap(),
        ProcessingGraphRevision::parse("A").unwrap(),
        DatasetReference::parse("dataset-raw-inrush").unwrap(),
        DatasetChecksum::parse("sha256:rawinrush001").unwrap(),
        graph,
        DatasetChecksum::parse("sha256:graphinrush001").unwrap(),
        AuditActor::parse("signal.engineer").unwrap(),
        "0.1.0",
    )
    .unwrap();

    let artifact = ProcessingGraphResultArtifact::from_instance(
        &instance,
        current_fft.clone(),
        DatasetKind::ProcessedSignal,
        DatasetFileReference::parse("data/RUN-INRUSH/current_l1_fft.csv").unwrap(),
        DatasetChecksum::parse("sha256:currentfft001").unwrap(),
    )
    .unwrap();

    assert_eq!(artifact.graph_reference().as_str(), "inrush-analysis");
    assert_eq!(artifact.graph_revision().as_str(), "A");
    assert_eq!(artifact.output_signal(), &current_fft);
    assert_eq!(artifact.kind(), DatasetKind::ProcessedSignal);
    assert_eq!(
        artifact.file_reference().as_str(),
        "data/RUN-INRUSH/current_l1_fft.csv"
    );
    assert_eq!(artifact.checksum().as_str(), "sha256:currentfft001");
    assert_eq!(artifact.raw_lineage(), &[current]);
}

#[test]
fn processing_graph_result_artifact_rejects_invalid_kind_and_unknown_output() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let current_fft = SignalReference::parse("current_l1_fft").unwrap();
    let mut graph = SignalProcessingGraph::from_dataset(&dataset);
    graph
        .add_node(
            SignalProcessingNode::new(
                SignalReference::parse("fft_current").unwrap(),
                SignalProcessingOperation::Fft,
                vec![current],
                current_fft.clone(),
            )
            .unwrap(),
        )
        .unwrap();
    let instance = ProcessingGraphInstance::new(
        ProcessingGraphReference::parse("inrush-analysis").unwrap(),
        ProcessingGraphRevision::parse("A").unwrap(),
        DatasetReference::parse("dataset-raw-inrush").unwrap(),
        DatasetChecksum::parse("sha256:rawinrush001").unwrap(),
        graph,
        DatasetChecksum::parse("sha256:graphinrush001").unwrap(),
        AuditActor::parse("signal.engineer").unwrap(),
        "0.1.0",
    )
    .unwrap();

    let kind_error = ProcessingGraphResultArtifact::from_instance(
        &instance,
        current_fft,
        DatasetKind::RawSignal,
        DatasetFileReference::parse("data/RUN-INRUSH/raw.opendata").unwrap(),
        DatasetChecksum::parse("sha256:rawagain001").unwrap(),
    )
    .unwrap_err();

    assert_eq!(
        kind_error,
        DomainError::InvalidProcessingGraphArtifactKind("raw_signal".to_owned())
    );

    let unknown_error = ProcessingGraphResultArtifact::from_instance(
        &instance,
        SignalReference::parse("missing_output").unwrap(),
        DatasetKind::ResultTable,
        DatasetFileReference::parse("data/RUN-INRUSH/missing.csv").unwrap(),
        DatasetChecksum::parse("sha256:missing001").unwrap(),
    )
    .unwrap_err();

    assert_eq!(
        unknown_error,
        DomainError::UnknownSignalReference("missing_output".to_owned())
    );
}

#[test]
fn processing_graph_execution_record_links_instance_and_artifacts() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let current_fft = SignalReference::parse("current_l1_fft").unwrap();
    let mut graph = SignalProcessingGraph::from_dataset(&dataset);
    graph
        .add_node(
            SignalProcessingNode::new(
                SignalReference::parse("fft_current").unwrap(),
                SignalProcessingOperation::Fft,
                vec![current],
                current_fft.clone(),
            )
            .unwrap(),
        )
        .unwrap();
    let instance = ProcessingGraphInstance::new(
        ProcessingGraphReference::parse("inrush-analysis").unwrap(),
        ProcessingGraphRevision::parse("A").unwrap(),
        DatasetReference::parse("dataset-raw-inrush").unwrap(),
        DatasetChecksum::parse("sha256:rawinrush001").unwrap(),
        graph,
        DatasetChecksum::parse("sha256:graphinrush001").unwrap(),
        AuditActor::parse("signal.engineer").unwrap(),
        "0.1.0",
    )
    .unwrap();
    let artifact = ProcessingGraphResultArtifact::from_instance(
        &instance,
        current_fft,
        DatasetKind::ProcessedSignal,
        DatasetFileReference::parse("data/RUN-INRUSH/current_l1_fft.csv").unwrap(),
        DatasetChecksum::parse("sha256:currentfft001").unwrap(),
    )
    .unwrap();

    let record = ProcessingGraphExecutionRecord::from_instance(
        ProcessingExecutionReference::parse("exec-inrush-001").unwrap(),
        &instance,
        AuditActor::parse("signal.engine").unwrap(),
        "0.1.0",
        ProcessingGraphExecutionStatus::Completed,
        &[artifact],
    )
    .unwrap();

    assert_eq!(record.execution().as_str(), "exec-inrush-001");
    assert_eq!(record.graph_reference().as_str(), "inrush-analysis");
    assert_eq!(record.graph_revision().as_str(), "A");
    assert_eq!(record.source_dataset().as_str(), "dataset-raw-inrush");
    assert_eq!(
        record.source_dataset_checksum().as_str(),
        "sha256:rawinrush001"
    );
    assert_eq!(record.executed_by().as_str(), "signal.engine");
    assert_eq!(record.software_version(), "0.1.0");
    assert_eq!(record.status(), ProcessingGraphExecutionStatus::Completed);
    assert_eq!(record.status().as_str(), "completed");
    assert_eq!(record.output_artifact_count(), 1);
}

#[test]
fn processing_graph_execution_record_rejects_completed_without_artifacts() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let mut graph = SignalProcessingGraph::from_dataset(&dataset);
    graph
        .add_node(
            SignalProcessingNode::new(
                SignalReference::parse("fft_current").unwrap(),
                SignalProcessingOperation::Fft,
                vec![current],
                SignalReference::parse("current_l1_fft").unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
    let instance = ProcessingGraphInstance::new(
        ProcessingGraphReference::parse("inrush-analysis").unwrap(),
        ProcessingGraphRevision::parse("A").unwrap(),
        DatasetReference::parse("dataset-raw-inrush").unwrap(),
        DatasetChecksum::parse("sha256:rawinrush001").unwrap(),
        graph,
        DatasetChecksum::parse("sha256:graphinrush001").unwrap(),
        AuditActor::parse("signal.engineer").unwrap(),
        "0.1.0",
    )
    .unwrap();

    let error = ProcessingGraphExecutionRecord::from_instance(
        ProcessingExecutionReference::parse("exec-inrush-empty").unwrap(),
        &instance,
        AuditActor::parse("signal.engine").unwrap(),
        "0.1.0",
        ProcessingGraphExecutionStatus::Completed,
        &[],
    )
    .unwrap_err();

    assert_eq!(
        error,
        DomainError::ProcessingGraphExecutionMissingArtifacts("exec-inrush-empty".to_owned())
    );

    let failed = ProcessingGraphExecutionRecord::from_instance(
        ProcessingExecutionReference::parse("exec-inrush-failed").unwrap(),
        &instance,
        AuditActor::parse("signal.engine").unwrap(),
        "0.1.0",
        ProcessingGraphExecutionStatus::Failed,
        &[],
    )
    .unwrap();
    assert_eq!(failed.status().as_str(), "failed");
    assert_eq!(failed.output_artifact_count(), 0);
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
    assert_eq!(result.backend(), FrequencyTransformBackend::ReferenceDft);
    assert_eq!(result.window(), None);
    assert_eq!(result.magnitudes().len(), 8);
    assert!((result.magnitudes()[0] - 425.0).abs() < 1e-9);
    assert_eq!(result.raw_lineage(), &[current]);
}

#[test]
fn signal_execution_engine_records_fft_backend_boundary() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let output = SignalReference::parse("current_l1_fft_optimized").unwrap();

    let result = SignalExecutionEngine::spectrum_magnitude_with_backend(
        &dataset,
        &current,
        output,
        FrequencyTransformBackend::OptimizedFftCompatible,
    )
    .unwrap();

    assert_eq!(
        result.backend(),
        FrequencyTransformBackend::OptimizedFftCompatible
    );
    assert_eq!(result.backend().as_str(), "optimized_fft_compatible");
    assert_eq!(result.operation(), SignalProcessingOperation::Fft);
    assert!((result.magnitudes()[0] - 425.0).abs() < 1e-9);
}

#[test]
fn optimized_fft_backend_matches_reference_dft_for_power_of_two_fixture() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let reference_output = SignalReference::parse("current_l1_reference_fft").unwrap();
    let optimized_output = SignalReference::parse("current_l1_optimized_fft").unwrap();

    let reference = SignalExecutionEngine::spectrum_magnitude_with_backend(
        &dataset,
        &current,
        reference_output,
        FrequencyTransformBackend::ReferenceDft,
    )
    .unwrap();
    let optimized = SignalExecutionEngine::spectrum_magnitude_with_backend(
        &dataset,
        &current,
        optimized_output,
        FrequencyTransformBackend::OptimizedFftCompatible,
    )
    .unwrap();

    assert_eq!(
        optimized.backend(),
        FrequencyTransformBackend::OptimizedFftCompatible
    );
    assert_eq!(optimized.magnitudes().len(), reference.magnitudes().len());
    for (actual, expected) in optimized.magnitudes().iter().zip(reference.magnitudes()) {
        assert!((actual - expected).abs() < 1e-9);
    }
}

#[test]
fn windowed_fft_records_window_and_matches_optimized_backend() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let reference_output = SignalReference::parse("current_l1_hamming_fft_ref").unwrap();
    let optimized_output = SignalReference::parse("current_l1_hamming_fft_opt").unwrap();

    let reference = SignalExecutionEngine::windowed_spectrum_magnitude_with_backend(
        &dataset,
        &current,
        reference_output,
        WindowFunction::Hamming,
        FrequencyTransformBackend::ReferenceDft,
    )
    .unwrap();
    let optimized = SignalExecutionEngine::windowed_spectrum_magnitude_with_backend(
        &dataset,
        &current,
        optimized_output,
        WindowFunction::Hamming,
        FrequencyTransformBackend::OptimizedFftCompatible,
    )
    .unwrap();

    assert_eq!(
        reference.operation(),
        SignalProcessingOperation::WindowedFft
    );
    assert_eq!(reference.window(), Some(WindowFunction::Hamming));
    assert_eq!(reference.raw_lineage(), &[current]);
    assert_eq!(optimized.window(), Some(WindowFunction::Hamming));
    assert_eq!(optimized.magnitudes().len(), reference.magnitudes().len());
    for (actual, expected) in optimized.magnitudes().iter().zip(reference.magnitudes()) {
        assert!((actual - expected).abs() < 1e-9);
    }
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
fn signal_execution_engine_applies_hann_window() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let voltage = SignalReference::parse("voltage_l1").unwrap();
    let output = SignalReference::parse("voltage_l1_hann").unwrap();

    let result = SignalExecutionEngine::apply_window(
        &dataset,
        &voltage,
        output.clone(),
        WindowFunction::Hann,
    )
    .unwrap();

    assert_eq!(result.output(), &output);
    assert_eq!(
        result.operation(),
        SignalProcessingOperation::TimeDomainFilter
    );
    assert_eq!(result.unit().as_str(), "mV");
    assert_eq!(result.sample_rate(), SampleRateHz::new(10_000).unwrap());
    assert_eq!(result.samples().len(), 8);
    assert!(result.samples()[0].abs() < 1e-9);
    assert!(result.samples()[7].abs() < 1e-9);
    assert_eq!(result.raw_lineage(), &[voltage]);
}

#[test]
fn window_functions_expose_deterministic_coefficients() {
    assert_eq!(WindowFunction::Rectangular.coefficient(2, 5), 1.0);
    assert!((WindowFunction::Hann.coefficient(2, 5) - 1.0).abs() < 1e-12);
    assert!((WindowFunction::Hamming.coefficient(0, 5) - 0.08).abs() < 1e-12);
    assert!(WindowFunction::Blackman.coefficient(0, 5).abs() < 1e-12);
    assert!((WindowFunction::Blackman.coefficient(2, 5) - 1.0).abs() < 1e-12);
    assert!((WindowFunction::FlatTop.coefficient(2, 5) - 1.0).abs() < 1e-8);
    assert!(WindowFunction::FlatTop.coefficient(0, 5).abs() < 0.001);
    assert_eq!(WindowFunction::FlatTop.coefficient(0, 1), 1.0);
}

#[test]
fn signal_execution_engine_resamples_linearly() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let output = SignalReference::parse("current_l1_20khz").unwrap();

    let result = SignalExecutionEngine::resample_linear(
        &dataset,
        &current,
        output.clone(),
        SampleRateHz::new(20_000).unwrap(),
    )
    .unwrap();

    assert_eq!(result.output(), &output);
    assert_eq!(result.operation(), SignalProcessingOperation::Resampling);
    assert_eq!(result.unit().as_str(), "mA");
    assert_eq!(result.sample_rate(), SampleRateHz::new(20_000).unwrap());
    assert_eq!(result.samples().len(), 15);
    assert_eq!(
        &result.samples()[0..7],
        &[0.0, 10.0, 20.0, 40.0, 60.0, 120.0, 180.0]
    );
    assert_eq!(result.raw_lineage(), &[current]);
}

#[test]
fn signal_execution_engine_downsamples_deterministically() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();
    let current = SignalReference::parse("current_l1").unwrap();
    let output = SignalReference::parse("current_l1_downsampled").unwrap();

    let result = SignalExecutionEngine::downsample(&dataset, &current, output.clone(), 2).unwrap();

    assert_eq!(result.output(), &output);
    assert_eq!(result.operation(), SignalProcessingOperation::Resampling);
    assert_eq!(result.unit().as_str(), "mA");
    assert_eq!(result.samples(), &[0, 60, 120, 5]);
    assert_eq!(result.raw_lineage(), &[current]);
}

#[test]
fn signal_execution_engine_rejects_invalid_resampling_factor() {
    let dataset = SimulatedDaqSource::open_daq()
        .acquire_inrush_fixture()
        .unwrap();

    let error = SignalExecutionEngine::downsample(
        &dataset,
        &SignalReference::parse("current_l1").unwrap(),
        SignalReference::parse("downsampled").unwrap(),
        0,
    )
    .unwrap_err();

    assert_eq!(error, DomainError::InvalidResamplingFactor(0));
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

fn sync_conflict(id: &str, kind: SyncConflictKind) -> SyncConflictRecord {
    SyncConflictRecord::new(
        SyncConflictId::parse(id).unwrap(),
        RepositoryDomain::ProjectRecords,
        kind,
        RepositorySnapshotId::parse("local-v1").unwrap(),
        RepositorySnapshotId::parse("reference-v2").unwrap(),
    )
}

fn sample_update_bundle(
    signature: Option<UpdateSignature>,
    offline_install_allowed: bool,
) -> UpdateBundle {
    UpdateBundle::new(
        UpdatePackageName::parse("emc-locus-core").unwrap(),
        SoftwareVersion::new(0, 2, 0),
        UpdateComponent::CoreApplication,
        VersionCompatibilityRange::new(
            SoftwareVersion::new(0, 1, 0),
            Some(SoftwareVersion::new(0, 1, 9)),
        )
        .unwrap(),
        SnapshotChecksum::parse("sha256:emc-locus-core-020").unwrap(),
        signature,
        offline_install_allowed,
        Some(RollbackReference::parse("emc-locus-core-0.1.0").unwrap()),
    )
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

fn raw_dataset_for_run(run_reference: &str) -> RawDatasetRecord {
    RawDatasetRecord::new(
        MeasurementRunReference::parse(run_reference).unwrap(),
        DatasetReference::parse("raw-signal-001").unwrap(),
        DatasetKind::RawSignal,
        DatasetFileReference::parse("data/RUN-001/raw-signal-001.opendata").unwrap(),
        DatasetChecksum::parse("sha256:abc123").unwrap(),
    )
}

fn issued_accredited_report(
    project: ProjectCode,
    reviewer: AuditActor,
    approver: AuditActor,
) -> ReportPackage {
    let mut report = ReportPackage::new(
        project,
        ReportNumber::parse("RPT-001").unwrap(),
        ReportRevision::parse("A").unwrap(),
        ExecutionMode::Accredited,
    );
    report.submit_for_technical_review().unwrap();
    report.complete_technical_review(reviewer).unwrap();
    report.approve(approver).unwrap();
    report.issue().unwrap();
    report
}
