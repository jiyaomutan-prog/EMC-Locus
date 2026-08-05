param(
    [string]$AgentUrl = "http://127.0.0.1:8765",
    [ValidatePattern("^[A-Za-z0-9-]*$")]
    [string]$FixtureSuffix = ""
)

$ErrorActionPreference = "Stop"
$IdentitySuffix = if ($FixtureSuffix) { "-$FixtureSuffix" } else { "" }
$MethodId = "METHOD-DEMO-RF-PREP$IdentitySuffix"
$SetupId = "SETUP-DEMO-RF-PREP$IdentitySuffix"
$ProjectCode = if ($FixtureSuffix) { "CEM-DEMO-PREP-$FixtureSuffix" } else { "CEM-DEMO-PREP-001" }
$ScheduleItemCode = if ($FixtureSuffix) { "PLAN-DEMO-PREP-$FixtureSuffix" } else { "PLAN-DEMO-PREP-001" }
$GeneratorAssetId = if ($FixtureSuffix) { "GEN-DEMO-RF-$FixtureSuffix" } else { "GEN-DEMO-RF-001" }
$PowerMeterAssetId = if ($FixtureSuffix) { "PM-DEMO-RF-$FixtureSuffix" } else { "PM-DEMO-RF-001" }
$LocationLabel = if ($FixtureSuffix) { "Poste CEM 1 - $FixtureSuffix" } else { "Poste CEM 1" }
$AssignedOperator = if ($FixtureSuffix) { "Alice Martin - $FixtureSuffix" } else { "Alice Martin" }

function Get-OperationId {
    param([string]$BaseId)
    if ($FixtureSuffix) {
        return "$BaseId-$FixtureSuffix"
    }
    return $BaseId
}

function Invoke-EmcApi {
    param(
        [ValidateSet("GET", "POST", "PUT")]
        [string]$Method,
        [string]$Path,
        [object]$Body = $null
    )

    $parameters = @{
        Uri = "$AgentUrl$Path"
        Method = $Method
        TimeoutSec = 20
    }
    if ($null -ne $Body) {
        $json = $Body | ConvertTo-Json -Depth 50
        $parameters.ContentType = "application/json; charset=utf-8"
        $parameters.Body = [System.Text.Encoding]::UTF8.GetBytes($json)
    }

    try {
        return Invoke-RestMethod @parameters
    } catch {
        $responseText = if ($_.ErrorDetails -and $_.ErrorDetails.Message) {
            $_.ErrorDetails.Message
        } else {
            $_.Exception.Message
        }
        throw "API $Method $Path failed. $responseText"
    }
}

function Get-ApprovedEquipmentModel {
    param([string]$ModelId)

    $models = (Invoke-EmcApi -Method GET -Path "/api/v1/equipment-models?demo_mode=all").equipment_models
    $model = $models | Where-Object { $_.identity.equipment_model_id -eq $ModelId } | Select-Object -First 1
    if (-not $model -or -not $model.current_approved_revision) {
        throw "Approved equipment model is required: $ModelId. Run seed-equipment-demo.ps1 first."
    }
    return $model
}

function Ensure-LaboratoryLocation {
    param([string]$Label)

    $locations = (Invoke-EmcApi -Method GET -Path "/api/v1/laboratory-locations?include_archived=true").locations
    $location = $locations | Where-Object { $_.label -eq $Label -and $_.status -eq "active" } | Select-Object -First 1
    if ($location) {
        return $location
    }
    return (Invoke-EmcApi -Method POST -Path "/api/v1/laboratory-locations" -Body ([ordered]@{
        label = $Label
        description = "Lieu de demonstration pour la preparation des essais"
        actor = "demo.laboratory.manager"
        reason = "Creer le lieu stable de la demonstration"
        operation_id = Get-OperationId "seed-planned-preparation-location-create"
    })).location
}

function Ensure-ApprovedMethod {
    $templateId = $script:MethodId
    $templates = (Invoke-EmcApi -Method GET -Path "/api/v1/test-templates").test_templates
    $template = $templates | Where-Object { $_.identity.template_id -eq $templateId } | Select-Object -First 1
    if ($template -and $template.current_approved_revision) {
        return $template
    }
    if ($template) {
        throw "Demo method exists without an approved revision: $templateId"
    }

    $definition = [ordered]@{
        definition_schema_version = "emc-locus.test-template-definition.v1"
        title = "Verification RF planifiee"
        description = "Methode de demonstration pour preparer une chaine generateur vers wattmetre."
        measurement_axis = "frequency_sweep"
        standard_references = @("METHODE INTERNE DEMO-RF-01")
        variables = @(
            [ordered]@{
                variable_id = "frequency_hz"
                label = "Frequence de verification"
                value_type = "integer"
                default_value = 1000000
                constraints = [ordered]@{
                    required = $true
                    dimensionless = $false
                    unit = "Hz"
                    minimum = 100000
                    maximum = 1000000000
                    enum_values = @()
                }
                description = "Frequence nominale appliquee a la chaine RF."
            }
        )
        lock_policy = @(
            [ordered]@{
                variable_id = "frequency_hz"
                policy = "editable_until_execution"
            }
        )
        instrumentation_chain = @(
            [ordered]@{
                slot_id = "measurement_receiver"
                label = "Wattmetre RF"
                required_category = "power_meter"
                required = $true
                calibration_requirement = "not_required"
                substitution_policy = "same_category"
                depends_on_slots = @()
            }
        )
        entry_step_id = "finish"
        sequence = @(
            [ordered]@{
                step_id = "finish"
                order = 10
                kind = "finish"
                label = "Cloturer la verification"
                required_slots = @()
                branches = @()
            }
        )
        limits = @()
        post_processing = @()
        method_parameters = [ordered]@{}
    }
    $created = Invoke-EmcApi -Method POST -Path "/api/v1/test-templates" -Body ([ordered]@{
        template_id = $templateId
        title = "Verification RF planifiee"
        category_code = "emission_conducted"
        definition = $definition
        actor = "demo.method.author"
        reason = "Creer la methode de demonstration de preparation"
        operation_id = Get-OperationId "seed-planned-preparation-method-create"
    })
    $revisionId = $created.revision.revision_id
    Invoke-EmcApi -Method POST -Path "/api/v1/test-templates/$templateId/revisions/$revisionId/transitions/submit-for-review" -Body ([ordered]@{
        actor = "demo.method.reviewer"
        reason = "Verifier la methode de demonstration"
        operation_id = Get-OperationId "seed-planned-preparation-method-submit"
    }) | Out-Null
    Invoke-EmcApi -Method POST -Path "/api/v1/test-templates/$templateId/revisions/$revisionId/transitions/approve" -Body ([ordered]@{
        actor = "demo.method.approver"
        reason = "Approuver la methode de demonstration"
        operation_id = Get-OperationId "seed-planned-preparation-method-approve"
    }) | Out-Null
    return (Invoke-EmcApi -Method GET -Path "/api/v1/test-templates/$templateId").test_template
}

function Ensure-Instrument {
    param(
        [string]$AssetId,
        [string]$Family,
        [string]$CategoryCode,
        [string]$SerialNumber,
        [object]$Model,
        [object]$Location
    )

    $instruments = (Invoke-EmcApi -Method GET -Path "/api/v1/metrology/instruments").instruments
    $instrument = $instruments | Where-Object { $_.asset_id -eq $AssetId } | Select-Object -First 1
    if (-not $instrument) {
        $revision = $Model.current_approved_revision
        Invoke-EmcApi -Method POST -Path "/api/v1/metrology/instruments" -Body ([ordered]@{
            asset_id = $AssetId
            family = $Family
            category_code = $CategoryCode
            equipment_model_id = $Model.identity.equipment_model_id
            equipment_model_revision_id = $revision.revision_id
            equipment_model_checksum = $revision.definition_checksum
            manufacturer = $revision.definition.manufacturer
            model = $revision.definition.model_name
            serial_number = $SerialNumber
            part_number = $revision.definition.model_name
            calibration_requirement = "not_required"
            serviceability_status = "usable"
            serviceability_reason = "Materiel de demonstration verifie"
            capabilities = [ordered]@{}
            metrology_notes = "Jeu de demonstration du controle de preparation planifiee."
            actor = "demo.metrology"
            reason = "Enregistrer le materiel de demonstration"
            operation_id = Get-OperationId "seed-planned-preparation-register-$AssetId"
        }) | Out-Null
    }

    $asset = (Invoke-EmcApi -Method GET -Path "/api/v1/fleet/assets/$AssetId").asset
    if ($asset.laboratory_location_id -ne $Location.location_id) {
        $moved = Invoke-EmcApi -Method POST -Path "/api/v1/fleet/assets/$AssetId/transitions/move" -Body ([ordered]@{
            expected_revision = $asset.revision
            destination_location_id = $Location.location_id
            actor = "demo.metrology"
            reason = "Affecter le materiel de demonstration au poste CEM"
            operation_id = Get-OperationId "seed-planned-preparation-move-$AssetId"
        })
        $asset = $moved.asset
    }
    return $asset
}

function New-StationRequirement {
    param(
        [string]$RequirementId,
        [string]$RoleLabel,
        [object]$Instrument,
        [string]$PortId,
        [string]$PortLabel,
        [ValidateSet("input", "output")]
        [string]$Directionality
    )

    return [ordered]@{
        requirement_id = $RequirementId
        role_label = $RoleLabel
        required = $true
        selection_policy = "exact_asset"
        assignment_stage = "setup_definition"
        substitution_policy = "no_substitution"
        calibration_requirement = "not_required"
        exact_asset_id = $Instrument.asset_id
        logical_ports = @(
            [ordered]@{
                logical_port_id = $PortId
                label = $PortLabel
                directionality = $Directionality
                signal_domain = "rf"
            }
        )
    }
}

function New-StationAssignment {
    param(
        [string]$RequirementId,
        [object]$Instrument,
        [object]$Model,
        [string]$PortId
    )

    return [ordered]@{
        requirement_id = $RequirementId
        asset_id = $Instrument.asset_id
        asset_revision = [string]$Instrument.revision
        inventory_code = $Instrument.inventory_code
        serial_number = $Instrument.serial_number
        equipment_model_id = $Model.identity.equipment_model_id
        equipment_model_revision_id = $Model.current_approved_revision.revision_id
        equipment_model_checksum = $Model.current_approved_revision.definition_checksum
        selected_ports = @(
            [ordered]@{
                logical_port_id = $PortId
                actual_port_id = $PortId
            }
        )
        assignment_context = "setup_definition"
        assigned_on = "2026-07-16"
    }
}

function Ensure-ReadyStation {
    param(
        [object]$Generator,
        [object]$PowerMeter,
        [object]$GeneratorModel,
        [object]$PowerMeterModel,
        [object]$Location
    )

    $setupId = $script:SetupId
    $setups = (Invoke-EmcApi -Method GET -Path "/api/v1/station-setups").station_setups
    $setup = $setups | Where-Object { $_.identity.setup_id -eq $setupId } | Select-Object -First 1
    if (-not $setup) {
        $created = Invoke-EmcApi -Method POST -Path "/api/v1/station-setups" -Body ([ordered]@{
            setup_id = $setupId
            label = "Chaine RF de verification"
            laboratory_location_id = $Location.location_id
            laboratory_location_label = $Location.label
            planned_use_on = "2026-07-16"
            execution_mode = "investigation"
            actor = "demo.technician"
            reason = "Creer le montage de demonstration"
            operation_id = Get-OperationId "seed-planned-preparation-station-create"
        })
        $setup = $created.station_setup
    }
    if ($setup.current_ready_revision) {
        return $setup
    }
    if (-not $setup.active_draft_revision) {
        throw "Demo station exists without a ready or editable revision: $setupId"
    }

    $draft = $setup.active_draft_revision
    $definition = [ordered]@{
        definition_schema_version = "emc-locus.station-measurement-setup-definition.v3"
        setup_id = $setupId
        label = "Chaine RF de verification"
        laboratory_location_id = $Location.location_id
        laboratory_location_label = $Location.label
        planned_use_on = "2026-07-16"
        execution_mode = "investigation"
        asset_bindings = @()
        connections = @()
        correction_selections = @()
        material_requirements = @(
            (New-StationRequirement -RequirementId "rf_generator" -RoleLabel "Generateur RF" -Instrument $Generator -PortId "RF_OUT" -PortLabel "Sortie RF" -Directionality "output"),
            (New-StationRequirement -RequirementId "power_meter" -RoleLabel "Wattmetre RF" -Instrument $PowerMeter -PortId "rf_input" -PortLabel "Entree RF" -Directionality "input")
        )
        material_assignments = @(
            (New-StationAssignment -RequirementId "rf_generator" -Instrument $Generator -Model $GeneratorModel -PortId "RF_OUT"),
            (New-StationAssignment -RequirementId "power_meter" -Instrument $PowerMeter -Model $PowerMeterModel -PortId "rf_input")
        )
        logical_connections = @(
            [ordered]@{
                connection_id = "rf_verification_path"
                label = "Sortie generateur vers entree wattmetre"
                from = [ordered]@{ requirement_id = "rf_generator"; logical_port_id = "RF_OUT" }
                to = [ordered]@{ requirement_id = "power_meter"; logical_port_id = "rf_input" }
            }
        )
        notes = [ordered]@{ purpose = "Démonstration du contrôle d’aptitude du montage" }
    }
    $saved = Invoke-EmcApi -Method PUT -Path "/api/v1/station-setups/$setupId/revisions/$($draft.revision_id)/definition" -Body ([ordered]@{
        expected_definition_checksum = $draft.definition_checksum
        definition = $definition
        actor = "demo.technician"
        reason = "Affecter les materiels et raccorder la chaine RF"
        operation_id = Get-OperationId "seed-planned-preparation-station-save"
    })
    $savedDraft = $saved.station_setup.active_draft_revision
    $readiness = Invoke-EmcApi -Method GET -Path "/api/v1/station-setups/$setupId/revisions/$($savedDraft.revision_id)/readiness"
    if (-not $readiness.readiness.ready) {
        throw "Demo station readiness is blocked: $($readiness | ConvertTo-Json -Depth 20 -Compress)"
    }
    Invoke-EmcApi -Method POST -Path "/api/v1/station-setups/$setupId/revisions/$($savedDraft.revision_id)/transitions/qualified" -Body ([ordered]@{
        expected_definition_checksum = $savedDraft.definition_checksum
        actor = "demo.technician"
        reason = "Valider la definition logique de demonstration"
        operation_id = Get-OperationId "seed-planned-preparation-station-qualified"
    }) | Out-Null
    Invoke-EmcApi -Method POST -Path "/api/v1/station-setups/$setupId/revisions/$($savedDraft.revision_id)/transitions/ready" -Body ([ordered]@{
        expected_definition_checksum = $savedDraft.definition_checksum
        actor = "demo.technician"
        reason = "Valider la chaine physique de demonstration"
        operation_id = Get-OperationId "seed-planned-preparation-station-ready"
    }) | Out-Null
    return (Invoke-EmcApi -Method GET -Path "/api/v1/station-setups/$setupId").station_setup
}

function Ensure-ProjectAndSchedule {
    param([object]$Location)
    $projectCode = $script:ProjectCode
    $itemCode = $script:ScheduleItemCode
    $projects = (Invoke-EmcApi -Method GET -Path "/api/v1/projects").projects
    $project = $projects | Where-Object { $_.code -eq $projectCode } | Select-Object -First 1
    if (-not $project) {
        $created = Invoke-EmcApi -Method POST -Path "/api/v1/projects" -Body ([ordered]@{
            code = $projectCode
            customer_name = "Atelier Horizon"
            execution_mode = "investigation"
            actor = "demo.project.lead"
            reason = "Creer le dossier de demonstration de preparation"
            operation_id = Get-OperationId "seed-planned-preparation-project-create"
        })
        $project = $created.project
    }
    if ($project.stage -eq "contract_review") {
        $review = (Invoke-EmcApi -Method GET -Path "/api/v1/projects/$projectCode/contract-review").contract_review
        foreach ($item in $review.required_items) {
            if ($review.completed_items.item -notcontains $item) {
                Invoke-EmcApi -Method POST -Path "/api/v1/projects/$projectCode/contract-review/items/$item/complete" -Body ([ordered]@{
                    actor = "demo.project.lead"
                    comment = "Point verifie pour la demonstration"
                    operation_id = Get-OperationId "seed-planned-preparation-review-$item"
                }) | Out-Null
            }
        }
        Invoke-EmcApi -Method POST -Path "/api/v1/projects/$projectCode/transitions/to-test-planning" -Body ([ordered]@{
            actor = "demo.project.lead"
            reason = "Revue terminee pour planifier la demonstration"
            operation_id = Get-OperationId "seed-planned-preparation-project-plan"
        }) | Out-Null
    }

    $schedule = (Invoke-EmcApi -Method GET -Path "/api/v1/projects/$projectCode/schedule-items").schedule_items
    $item = $schedule | Where-Object { $_.item_code -eq $itemCode } | Select-Object -First 1
    if (-not $item) {
        $created = Invoke-EmcApi -Method POST -Path "/api/v1/projects/$projectCode/schedule-items" -Body ([ordered]@{
            item_code = $itemCode
            title = "Verification RF du convertisseur Horizon"
            planned_start_at = "2026-07-16T09:00"
            planned_end_at = "2026-07-16T12:00"
            assigned_operator = $script:AssignedOperator
            laboratory_location_id = $Location.location_id
            laboratory_location_label = $Location.label
            equipment_under_test = "Convertisseur Horizon HCU-4"
            notes = "Preparation metrologique requise avant demarrage."
            actor = "demo.project.lead"
            reason = "Planifier la verification RF de demonstration"
            operation_id = Get-OperationId "seed-planned-preparation-schedule-create"
        })
        $item = $created.schedule_item
    }
    if ($item.status -eq "planned") {
        Invoke-EmcApi -Method POST -Path "/api/v1/projects/$projectCode/schedule-items/$itemCode/transitions/confirm" -Body ([ordered]@{
            expected_revision = $item.revision
            actor = "demo.project.lead"
            reason = "Confirmer l'operateur et le poste CEM"
            operation_id = Get-OperationId "seed-planned-preparation-schedule-confirm"
        }) | Out-Null
    }
}

Invoke-EmcApi -Method GET -Path "/api/v1/health" | Out-Null
$method = Ensure-ApprovedMethod
$generatorModel = Get-ApprovedEquipmentModel -ModelId "EQM-PRESET-RF-GENERATOR"
$powerMeterModel = Get-ApprovedEquipmentModel -ModelId "EQM-DEMO-NRP6AN-FWD"
$location = Ensure-LaboratoryLocation -Label $LocationLabel
$generator = Ensure-Instrument -AssetId $GeneratorAssetId -Family "Generateur RF" -CategoryCode "rf_signal_generator" -SerialNumber "GEN-RF-2026$IdentitySuffix" -Model $generatorModel -Location $location
$powerMeter = Ensure-Instrument -AssetId $PowerMeterAssetId -Family "Wattmetre RF" -CategoryCode "rf_power_meter" -SerialNumber "PM-RF-2026$IdentitySuffix" -Model $powerMeterModel -Location $location
$station = Ensure-ReadyStation -Generator $generator -PowerMeter $powerMeter -GeneratorModel $generatorModel -PowerMeterModel $powerMeterModel -Location $location
Ensure-ProjectAndSchedule -Location $location
$schedule = (Invoke-EmcApi -Method GET -Path "/api/v1/projects/$ProjectCode/schedule-items").schedule_items |
    Where-Object { $_.item_code -eq $ScheduleItemCode } |
    Select-Object -First 1
$preparation = (Invoke-EmcApi -Method GET -Path "/api/v1/projects/$ProjectCode/schedule-items/$ScheduleItemCode/preparation").preparation

Write-Host "Planned test preparation demo is ready via API at $AgentUrl"
Write-Host "  Method: $($method.identity.title)"
Write-Host "  Station: $($station.identity.label)"
Write-Host "  Project: $ProjectCode"
Write-Host "  Slot: $ScheduleItemCode ($($schedule.status), preparation $($preparation.current_state))"
