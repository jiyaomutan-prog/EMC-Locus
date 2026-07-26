param(
    [string]$AgentUrl = "http://127.0.0.1:8765"
)

$ErrorActionPreference = "Stop"

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
        $parameters.ContentType = "application/json"
        $parameters.Body = ($Body | ConvertTo-Json -Depth 50)
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

function Ensure-ApprovedMethod {
    $templateId = "METHOD-DEMO-RF-PREP"
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
        operation_id = "seed-planned-preparation-method-create"
    })
    $revisionId = $created.revision.revision_id
    Invoke-EmcApi -Method POST -Path "/api/v1/test-templates/$templateId/revisions/$revisionId/transitions/submit-for-review" -Body ([ordered]@{
        actor = "demo.method.reviewer"
        reason = "Verifier la methode de demonstration"
        operation_id = "seed-planned-preparation-method-submit"
    }) | Out-Null
    Invoke-EmcApi -Method POST -Path "/api/v1/test-templates/$templateId/revisions/$revisionId/transitions/approve" -Body ([ordered]@{
        actor = "demo.method.approver"
        reason = "Approuver la methode de demonstration"
        operation_id = "seed-planned-preparation-method-approve"
    }) | Out-Null
    return (Invoke-EmcApi -Method GET -Path "/api/v1/test-templates/$templateId").test_template
}

function Ensure-Instrument {
    param(
        [string]$AssetId,
        [string]$Family,
        [string]$CategoryCode,
        [string]$SerialNumber,
        [object]$Model
    )

    $instruments = (Invoke-EmcApi -Method GET -Path "/api/v1/metrology/instruments").instruments
    $instrument = $instruments | Where-Object { $_.asset_id -eq $AssetId } | Select-Object -First 1
    if ($instrument) {
        return $instrument
    }

    $revision = $Model.current_approved_revision
    $registered = Invoke-EmcApi -Method POST -Path "/api/v1/metrology/instruments" -Body ([ordered]@{
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
        operation_id = "seed-planned-preparation-register-$AssetId"
    })
    return $registered.instrument
}

function New-StationBinding {
    param(
        [string]$BindingId,
        [string]$RoleLabel,
        [object]$Instrument,
        [object]$Model
    )

    return [ordered]@{
        binding_id = $BindingId
        role_label = $RoleLabel
        asset_id = $Instrument.asset_id
        asset_revision = $Instrument.revision
        equipment_model_id = $Model.identity.equipment_model_id
        equipment_model_revision_id = $Model.current_approved_revision.revision_id
        equipment_model_checksum = $Model.current_approved_revision.definition_checksum
    }
}

function Ensure-ReadyStation {
    param(
        [object]$Generator,
        [object]$PowerMeter,
        [object]$GeneratorModel,
        [object]$PowerMeterModel
    )

    $setupId = "SETUP-DEMO-RF-PREP"
    $setups = (Invoke-EmcApi -Method GET -Path "/api/v1/station-setups").station_setups
    $setup = $setups | Where-Object { $_.identity.setup_id -eq $setupId } | Select-Object -First 1
    if (-not $setup) {
        $created = Invoke-EmcApi -Method POST -Path "/api/v1/station-setups" -Body ([ordered]@{
            setup_id = $setupId
            label = "Chaine RF de verification"
            laboratory_location_id = "LAB-LOCATION-DEMO-CEM-1"
            laboratory_location_label = "Poste CEM 1"
            planned_use_on = "2026-07-16"
            execution_mode = "investigation"
            actor = "demo.technician"
            reason = "Creer le montage de demonstration"
            operation_id = "seed-planned-preparation-station-create"
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
        definition_schema_version = "emc-locus.station-measurement-setup-definition.v2"
        setup_id = $setupId
        label = "Chaine RF de verification"
        laboratory_location_id = "LAB-LOCATION-DEMO-CEM-1"
        laboratory_location_label = "Poste CEM 1"
        planned_use_on = "2026-07-16"
        execution_mode = "investigation"
        asset_bindings = @(
            (New-StationBinding -BindingId "rf_generator" -RoleLabel "Generateur RF" -Instrument $Generator -Model $GeneratorModel),
            (New-StationBinding -BindingId "power_meter" -RoleLabel "Wattmetre RF" -Instrument $PowerMeter -Model $PowerMeterModel)
        )
        connections = @(
            [ordered]@{
                connection_id = "rf_verification_path"
                label = "Sortie generateur vers entree wattmetre"
                from = [ordered]@{ binding_id = "rf_generator"; port_id = "RF_OUT" }
                to = [ordered]@{ binding_id = "power_meter"; port_id = "rf_input" }
            }
        )
        correction_selections = @()
        notes = [ordered]@{ purpose = "Demonstration du pre-vol operateur" }
    }
    $saved = Invoke-EmcApi -Method PUT -Path "/api/v1/station-setups/$setupId/revisions/$($draft.revision_id)/definition" -Body ([ordered]@{
        expected_definition_checksum = $draft.definition_checksum
        definition = $definition
        actor = "demo.technician"
        reason = "Affecter les materiels et raccorder la chaine RF"
        operation_id = "seed-planned-preparation-station-save"
    })
    $savedDraft = $saved.station_setup.active_draft_revision
    $readiness = Invoke-EmcApi -Method GET -Path "/api/v1/station-setups/$setupId/revisions/$($savedDraft.revision_id)/readiness"
    if (-not $readiness.readiness.ready) {
        throw "Demo station readiness is blocked: $($readiness | ConvertTo-Json -Depth 20 -Compress)"
    }
    Invoke-EmcApi -Method POST -Path "/api/v1/station-setups/$setupId/revisions/$($savedDraft.revision_id)/transitions/ready" -Body ([ordered]@{
        expected_definition_checksum = $savedDraft.definition_checksum
        actor = "demo.technician"
        reason = "Valider la chaine physique de demonstration"
        operation_id = "seed-planned-preparation-station-ready"
    }) | Out-Null
    return (Invoke-EmcApi -Method GET -Path "/api/v1/station-setups/$setupId").station_setup
}

function Ensure-ProjectAndSchedule {
    $projectCode = "CEM-DEMO-PREP-001"
    $itemCode = "PLAN-DEMO-PREP-001"
    $projects = (Invoke-EmcApi -Method GET -Path "/api/v1/projects").projects
    $project = $projects | Where-Object { $_.code -eq $projectCode } | Select-Object -First 1
    if (-not $project) {
        $created = Invoke-EmcApi -Method POST -Path "/api/v1/projects" -Body ([ordered]@{
            code = $projectCode
            customer_name = "Atelier Horizon"
            execution_mode = "investigation"
            actor = "demo.project.lead"
            reason = "Creer le dossier de demonstration de preparation"
            operation_id = "seed-planned-preparation-project-create"
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
                    operation_id = "seed-planned-preparation-review-$item"
                }) | Out-Null
            }
        }
        Invoke-EmcApi -Method POST -Path "/api/v1/projects/$projectCode/transitions/to-test-planning" -Body ([ordered]@{
            actor = "demo.project.lead"
            reason = "Revue terminee pour planifier la demonstration"
            operation_id = "seed-planned-preparation-project-plan"
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
            assigned_operator = "Alice Martin"
            laboratory_location_id = "LAB-LOCATION-DEMO-CEM-1"
            laboratory_location_label = "Poste CEM 1"
            equipment_under_test = "Convertisseur Horizon HCU-4"
            notes = "Preparation metrologique requise avant demarrage."
            actor = "demo.project.lead"
            reason = "Planifier la verification RF de demonstration"
            operation_id = "seed-planned-preparation-schedule-create"
        })
        $item = $created.schedule_item
    }
    if ($item.status -eq "planned") {
        Invoke-EmcApi -Method POST -Path "/api/v1/projects/$projectCode/schedule-items/$itemCode/transitions/confirm" -Body ([ordered]@{
            expected_revision = $item.revision
            actor = "demo.project.lead"
            reason = "Confirmer l'operateur et le poste CEM"
            operation_id = "seed-planned-preparation-schedule-confirm"
        }) | Out-Null
    }
}

Invoke-EmcApi -Method GET -Path "/api/v1/health" | Out-Null
$method = Ensure-ApprovedMethod
$generatorModel = Get-ApprovedEquipmentModel -ModelId "EQM-PRESET-RF-GENERATOR"
$powerMeterModel = Get-ApprovedEquipmentModel -ModelId "EQM-DEMO-NRP6AN-FWD"
$generator = Ensure-Instrument -AssetId "GEN-DEMO-RF-001" -Family "Generateur RF" -CategoryCode "rf_signal_generator" -SerialNumber "GEN-RF-2026-001" -Model $generatorModel
$powerMeter = Ensure-Instrument -AssetId "PM-DEMO-RF-001" -Family "Wattmetre RF" -CategoryCode "rf_power_meter" -SerialNumber "PM-RF-2026-001" -Model $powerMeterModel
$station = Ensure-ReadyStation -Generator $generator -PowerMeter $powerMeter -GeneratorModel $generatorModel -PowerMeterModel $powerMeterModel
Ensure-ProjectAndSchedule
$schedule = (Invoke-EmcApi -Method GET -Path "/api/v1/projects/CEM-DEMO-PREP-001/schedule-items").schedule_items |
    Where-Object { $_.item_code -eq "PLAN-DEMO-PREP-001" } |
    Select-Object -First 1
$preparation = (Invoke-EmcApi -Method GET -Path "/api/v1/projects/CEM-DEMO-PREP-001/schedule-items/PLAN-DEMO-PREP-001/preparation").preparation

Write-Host "Planned test preparation demo is ready via API at $AgentUrl"
Write-Host "  Method: $($method.identity.title)"
Write-Host "  Station: $($station.identity.label)"
Write-Host "  Project: CEM-DEMO-PREP-001"
Write-Host "  Slot: PLAN-DEMO-PREP-001 ($($schedule.status), preparation $($preparation.current_state))"
