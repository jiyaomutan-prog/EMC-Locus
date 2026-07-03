param(
    [string]$AgentUrl = "http://127.0.0.1:8765",
    [switch]$Recreate,
    [switch]$IncludeEquipment
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
        $responseText = ""
        if ($_.ErrorDetails -and $_.ErrorDetails.Message) {
            $responseText = $_.ErrorDetails.Message
        } elseif ($_.Exception.Response -and $_.Exception.Response.GetResponseStream()) {
            $reader = [System.IO.StreamReader]::new($_.Exception.Response.GetResponseStream())
            $responseText = $reader.ReadToEnd()
        }
        throw "API $Method $Path failed. $responseText"
    }
}

function Get-TemplateMap {
    $result = Invoke-EmcApi -Method GET -Path "/api/v1/test-templates"
    $map = @{}
    foreach ($template in $result.test_templates) {
        $map[$template.identity.template_id] = $template
    }
    return $map
}

function New-DemoDefinition {
    param(
        [string]$Title,
        [string]$Axis = "time_series",
        [switch]$Rich
    )

    $variables = @(
        [ordered]@{
            variable_id = "repeat_count"
            label = "Nombre de repetitions"
            value_type = "integer"
            default_value = 3
            constraints = [ordered]@{
                required = $true
                dimensionless = $true
                minimum = 1
                maximum = 10
                enum_values = @()
            }
            description = "Compteur sans dimension pour demonstrations."
        }
    )
    if ($Rich) {
        $variables += [ordered]@{
            variable_id = "sample_rate_hz"
            label = "Frequence echantillonnage"
            value_type = "integer"
            default_value = 100000
            constraints = [ordered]@{
                required = $true
                dimensionless = $false
                unit = "Hz"
                minimum = 1000
                maximum = 1000000
                enum_values = @()
            }
            description = "Taux DAQ pour capture temporelle."
        }
    }

    $slots = @(
        [ordered]@{
            slot_id = "measurement_receiver"
            label = "Recepteur ou DAQ"
            required_category = "daq_chassis"
            required_capability = "time_series_capture"
            required = $true
            calibration_requirement = "if_used"
            substitution_policy = "same_capability"
            depends_on_slots = @()
        }
    )
    if ($Rich) {
        $slots += [ordered]@{
            slot_id = "current_probe"
            label = "Sonde courant"
            required_category = "current_probe"
            required_capability = "transient_current"
            required = $true
            calibration_requirement = "required"
            substitution_policy = "same_category"
            depends_on_slots = @("measurement_receiver")
        }
    }

    $captureRequiredSlots = @("measurement_receiver")
    $limitVariableRefs = @()
    if ($Rich) {
        $captureRequiredSlots = @("measurement_receiver", "current_probe")
        $limitVariableRefs = @("sample_rate_hz")
    }

    $sequence = @(
        [ordered]@{
            step_id = "prepare"
            order = 10
            kind = "prepare"
            label = "Preparation"
            instruction = "Verifier la configuration et les raccordements."
            required_slots = @("measurement_receiver")
            branches = @()
        },
        [ordered]@{
            step_id = "capture"
            order = 20
            kind = "acquire"
            label = "Capture"
            instruction = "Capturer le signal temporel."
            required_slots = $captureRequiredSlots
            branches = @(
                [ordered]@{
                    rule_id = "signal_absent"
                    condition = "Condition textuelle provisoire: signal absent"
                    destination_step_id = "finish"
                    allow_cycle = $false
                }
            )
        },
        [ordered]@{
            step_id = "finish"
            order = 30
            kind = "finish"
            label = "Cloture"
            instruction = "Clore la sequence de demonstration."
            required_slots = @()
            branches = @()
        }
    )

    return [ordered]@{
        definition_schema_version = "emc-locus.test-template-definition.v1"
        title = $Title
        description = "Definition de demonstration LAB CONSOLE creee par API."
        measurement_axis = $Axis
        standard_references = @("DEMO-METHOD-0.10.0")
        variables = $variables
        lock_policy = @(
            [ordered]@{
                variable_id = "repeat_count"
                policy = "editable_until_execution"
            }
        )
        instrumentation_chain = $slots
        entry_step_id = "prepare"
        sequence = $sequence
        limits = @(
            [ordered]@{
                limit_id = "demo_threshold"
                kind = "scalar_threshold"
                axis = $Axis
                unit = if ($Rich) { "A" } else { "V" }
                application_domain = "demo"
                source_reference = "method:LAB-DEMO:0.10.0"
                threshold = if ($Rich) { 30.0 } else { 1.0 }
                attention_rule = "warn_above_threshold"
                variable_refs = $limitVariableRefs
            }
        )
        post_processing = @(
            [ordered]@{
                operation_id = "peak"
                order = 10
                operation_type = "peak"
                inputs = @("raw.signal")
                outputs = @("calculated.peak")
                parameters = [ordered]@{
                    absolute = $true
                }
            }
        )
        method_parameters = [ordered]@{}
    }
}

function Ensure-ApprovedTemplate {
    param(
        [hashtable]$TemplateMap,
        [string]$TemplateId,
        [string]$Title,
        [string]$Category,
        [object]$Definition
    )

    if ($TemplateMap.ContainsKey($TemplateId) -and -not $Recreate) {
        Write-Host "Demo template already exists: $TemplateId"
        return $TemplateMap[$TemplateId]
    }

    if ($TemplateMap.ContainsKey($TemplateId) -and $Recreate) {
        Write-Host "Demo template exists and cannot be deleted through the public API: $TemplateId"
        return $TemplateMap[$TemplateId]
    }

    $create = Invoke-EmcApi -Method POST -Path "/api/v1/test-templates" -Body ([ordered]@{
        template_id = $TemplateId
        title = $Title
        category_code = $Category
        definition = $Definition
        actor = "demo.seed"
        reason = "LAB CONSOLE demo seed"
        operation_id = "seed-$TemplateId-create"
    })
    $revisionId = $create.revision.revision_id
    Invoke-EmcApi -Method POST -Path "/api/v1/test-templates/$TemplateId/revisions/$revisionId/transitions/submit-for-review" -Body ([ordered]@{
        actor = "demo.reviewer"
        reason = "Demo template ready for approval"
        operation_id = "seed-$TemplateId-submit"
    }) | Out-Null
    $approved = Invoke-EmcApi -Method POST -Path "/api/v1/test-templates/$TemplateId/revisions/$revisionId/transitions/approve" -Body ([ordered]@{
        actor = "demo.approver"
        reason = "Demo approval"
        operation_id = "seed-$TemplateId-approve"
    })
    Write-Host "Approved demo template: $TemplateId"
    return $approved.test_template
}

function Ensure-DraftFromApproved {
    param(
        [string]$TemplateId,
        [object]$Template
    )

    if ($Template.active_draft_revision) {
        Write-Host "Demo draft already exists: $TemplateId -> $($Template.active_draft_revision.revision_id)"
        return
    }
    if (-not $Template.current_approved_revision) {
        Write-Host "Demo template has no approved revision to derive: $TemplateId"
        return
    }

    Invoke-EmcApi -Method POST -Path "/api/v1/test-templates/$TemplateId/revisions" -Body ([ordered]@{
        source_revision_id = $Template.current_approved_revision.revision_id
        actor = "demo.author"
        reason = "Prepare next demo draft"
        operation_id = "seed-$TemplateId-derive"
    }) | Out-Null
    Write-Host "Derived demo draft: $TemplateId"
}

Invoke-EmcApi -Method GET -Path "/api/v1/health" | Out-Null
$templates = Get-TemplateMap

Ensure-ApprovedTemplate `
    -TemplateMap $templates `
    -TemplateId "DEMO-APPROVED-001" `
    -Title "Demo approved conducted emission" `
    -Category "emission_conducted" `
    -Definition (New-DemoDefinition -Title "Demo approved conducted emission") | Out-Null

$templates = Get-TemplateMap
$draftBase = Ensure-ApprovedTemplate `
    -TemplateMap $templates `
    -TemplateId "DEMO-DRAFT-001" `
    -Title "Demo approved with active draft" `
    -Category "emission_transient_time_domain" `
    -Definition (New-DemoDefinition -Title "Demo approved with active draft")
Ensure-DraftFromApproved -TemplateId "DEMO-DRAFT-001" -Template $draftBase

$templates = Get-TemplateMap
Ensure-ApprovedTemplate `
    -TemplateMap $templates `
    -TemplateId "DEMO-RICH-001" `
    -Title "Demo rich time-domain EMC method" `
    -Category "emission_transient_time_domain" `
    -Definition (New-DemoDefinition -Title "Demo rich time-domain EMC method" -Rich) | Out-Null

Write-Host "LAB demo seed complete via API at $AgentUrl"

if ($IncludeEquipment) {
    & (Join-Path $PSScriptRoot "seed-equipment-demo.ps1") -AgentUrl $AgentUrl
}
