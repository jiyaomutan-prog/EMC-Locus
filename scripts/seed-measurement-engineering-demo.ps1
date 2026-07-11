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
        TimeoutSec = 30
    }
    if ($null -ne $Body) {
        $parameters.ContentType = "application/json"
        $parameters.Body = ($Body | ConvertTo-Json -Depth 100)
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

function Assert-MeasurementDefinitionValid {
    param(
        [string]$ValidationPath,
        [object]$Definition
    )
    $validation = Invoke-EmcApi -Method POST -Path $ValidationPath -Body ([ordered]@{
        definition = $Definition
    })
    if (-not $validation.valid) {
        $issues = ($validation.issues | ConvertTo-Json -Depth 30)
        throw "Measurement engineering definition is invalid for $ValidationPath`: $issues"
    }
    return $validation
}

function Get-MeasurementMap {
    param([string]$Collection)
    $result = Invoke-EmcApi -Method GET -Path "/api/v1/$Collection"
    $map = @{}
    foreach ($item in $result.items) {
        $map[$item.identity.entity_id] = $item
    }
    return $map
}

function Approve-MeasurementRevision {
    param(
        [string]$Collection,
        [string]$EntityId,
        [object]$Revision
    )
    $revisionId = $Revision.revision_id
    if ($Revision.status -eq "draft") {
        Invoke-EmcApi -Method POST -Path "/api/v1/$Collection/$EntityId/revisions/$revisionId/transitions/submit-for-review" -Body ([ordered]@{
            actor = "demo.measurement.reviewer"
            reason = "Measurement engineering demo definition ready for review"
            operation_id = "seed-$EntityId-$revisionId-submit"
        }) | Out-Null
    }
    if ($Revision.status -eq "draft" -or $Revision.status -eq "under_review") {
        Invoke-EmcApi -Method POST -Path "/api/v1/$Collection/$EntityId/revisions/$revisionId/transitions/approve" -Body ([ordered]@{
            actor = "demo.measurement.approver"
            reason = "Measurement engineering demo definition approved"
            operation_id = "seed-$EntityId-$revisionId-approve"
        }) | Out-Null
    }
}

function Ensure-ApprovedMeasurementDefinition {
    param(
        [string]$Collection,
        [string]$ValidationPath,
        [string]$EntityId,
        [object]$Definition
    )
    $items = Get-MeasurementMap -Collection $Collection
    if ($items.ContainsKey($EntityId) -and $items[$EntityId].current_approved_revision) {
        Write-Host "Measurement engineering definition already approved: $EntityId"
        return $items[$EntityId].current_approved_revision
    }

    if (-not $items.ContainsKey($EntityId)) {
        Assert-MeasurementDefinitionValid -ValidationPath $ValidationPath -Definition $Definition | Out-Null
        $created = Invoke-EmcApi -Method POST -Path "/api/v1/$Collection" -Body ([ordered]@{
            entity_id = $EntityId
            definition = $Definition
            actor = "demo.measurement.author"
            reason = "Seed measurement engineering demo definition"
            operation_id = "seed-$EntityId-create"
        })
        $revision = $created.revision
        Write-Host "Created measurement engineering definition: $EntityId"
    } else {
        $revision = $items[$EntityId].active_draft_revision
        if (-not $revision) {
            $revision = $items[$EntityId].latest_revision
        }
    }

    Approve-MeasurementRevision -Collection $Collection -EntityId $EntityId -Revision $revision
    $approved = Invoke-EmcApi -Method GET -Path "/api/v1/$Collection/$EntityId"
    Write-Host "Approved measurement engineering definition: $EntityId"
    return $approved.item.current_approved_revision
}

function New-DefinitionReference {
    param(
        [string]$EntityId,
        [string]$RevisionId
    )
    return [ordered]@{
        entity_id = $EntityId
        revision_id = $RevisionId
        require_approved = $true
    }
}

function New-LinearScaling {
    param(
        [string]$Id,
        [string]$Label,
        [string]$InputQuantity,
        [string]$InputUnit,
        [string]$OutputQuantity,
        [string]$OutputUnit,
        [double]$Scale
    )
    return [ordered]@{
        definition_schema_version = "emc-locus.scaling-profile-definition.v1"
        scaling_profile_id = $Id
        label = $Label
        input_quantity = $InputQuantity
        input_unit = $InputUnit
        output_quantity = $OutputQuantity
        output_unit = $OutputUnit
        scaling_kind = "linear"
        parameters = [ordered]@{
            scale = $Scale
            offset = 0.0
        }
        validity_domain = [ordered]@{}
        source_reference = "demo:$Id"
        metadata = [ordered]@{ demo = $true }
    }
}

function New-FrequencyCurve {
    param(
        [string]$Id,
        [string]$CurveType,
        [string]$Label,
        [string]$ValueId,
        [string]$ValueUnit,
        [double[]]$Values
    )
    $frequencies = @(1000000.0, 10000000.0, 100000000.0, 1000000000.0)
    $points = @()
    for ($index = 0; $index -lt $frequencies.Count; $index++) {
        $points += [ordered]@{
            axis_values = [ordered]@{ frequency = $frequencies[$index] }
            values = [ordered]@{ $ValueId = $Values[$index] }
        }
    }
    return [ordered]@{
        definition_schema_version = "emc-locus.engineering-curve-definition.v1"
        curve_id = $Id
        curve_type = $CurveType
        label = $Label
        independent_axes = @(
            [ordered]@{ axis = "frequency"; quantity = "frequency"; unit = "Hz" }
        )
        dependent_values = @(
            [ordered]@{ value_id = $ValueId; quantity = "dimensionless"; unit = $ValueUnit }
        )
        units = [ordered]@{ frequency = "Hz"; $ValueId = $ValueUnit }
        points = $points
        interpolation = "log_x_linear_y"
        extrapolation_policy = "warn"
        validity_domain = [ordered]@{}
        conditions = [ordered]@{ temperature_c = 23.0 }
        source_document_reference = "demo:$Id"
        source_checksum = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        status = "demo"
        metadata = [ordered]@{ demo = $true }
    }
}

function New-CurrentProbeSensor {
    param(
        [object]$ScalingRevision,
        [object]$TransferCurveRevision
    )
    return [ordered]@{
        definition_schema_version = "emc-locus.sensor-definition.v1"
        sensor_definition_id = "SNS-DEMO-CURRENT-PROBE-10MV-A"
        manufacturer = "Demo"
        model_name = "Current Probe 10mV/A"
        variant = "wideband"
        sensor_family = "current_probe"
        physical_input_quantity = "current"
        engineering_output_quantity = "current"
        engineering_output_unit = "A"
        electrical_output_quantity = "voltage"
        electrical_output_unit = "V"
        signal_domain = "analog_voltage"
        technology_tags = @("voltage_input")
        required_excitation = [ordered]@{ excitation_kind = "none"; external_allowed = $false }
        input_mode_requirement = "differential"
        nominal_range = [ordered]@{ minimum = -100.0; maximum = 100.0; unit = "A" }
        safe_range = [ordered]@{ minimum = -200.0; maximum = 200.0; unit = "A" }
        orientation_axes = @()
        settling_time_ms = 1.0
        frequency_range = [ordered]@{ minimum_hz = 10.0; maximum_hz = 100000000.0 }
        scaling_profile_refs = @((New-DefinitionReference -EntityId $ScalingRevision.entity_id -RevisionId $ScalingRevision.revision_id))
        correction_curve_refs = @((New-DefinitionReference -EntityId $TransferCurveRevision.entity_id -RevisionId $TransferCurveRevision.revision_id))
        metadata = [ordered]@{ demo = $true }
    }
}

function New-ReceivingAntennaSensor {
    param([object]$AntennaCurveRevision)
    return [ordered]@{
        definition_schema_version = "emc-locus.sensor-definition.v1"
        sensor_definition_id = "SNS-DEMO-BICONICAL-ANTENNA"
        manufacturer = "Demo"
        model_name = "Biconical Antenna"
        variant = "30MHz-300MHz"
        sensor_family = "receiving_antenna"
        physical_input_quantity = "electric_field"
        engineering_output_quantity = "electric_field"
        engineering_output_unit = "V_per_meter"
        electrical_output_quantity = "voltage"
        electrical_output_unit = "V"
        signal_domain = "rf"
        technology_tags = @("rf_50_ohm")
        required_excitation = [ordered]@{ excitation_kind = "none"; external_allowed = $false }
        nominal_range = [ordered]@{ minimum = 0.001; maximum = 100.0; unit = "V_per_meter" }
        frequency_range = [ordered]@{ minimum_hz = 30000000.0; maximum_hz = 300000000.0 }
        scaling_profile_refs = @()
        correction_curve_refs = @((New-DefinitionReference -EntityId $AntennaCurveRevision.entity_id -RevisionId $AntennaCurveRevision.revision_id))
        metadata = [ordered]@{ demo = $true }
    }
}

function New-IepeAccelerometerSensor {
    param([object]$ScalingRevision)
    return [ordered]@{
        definition_schema_version = "emc-locus.sensor-definition.v1"
        sensor_definition_id = "SNS-DEMO-IEPE-ACCEL-100MV-G"
        manufacturer = "Demo"
        model_name = "IEPE Accelerometer 100mV/g"
        variant = "triax-ready"
        sensor_family = "accelerometer"
        physical_input_quantity = "acceleration"
        engineering_output_quantity = "acceleration"
        engineering_output_unit = "g"
        electrical_output_quantity = "voltage"
        electrical_output_unit = "V"
        signal_domain = "analog_voltage"
        technology_tags = @("voltage_input", "iepe")
        required_excitation = [ordered]@{ excitation_kind = "iepe"; nominal_value = 4.0; unit = "mA"; external_allowed = $false }
        input_mode_requirement = "iepe"
        nominal_range = [ordered]@{ minimum = -50.0; maximum = 50.0; unit = "g" }
        safe_range = [ordered]@{ minimum = -100.0; maximum = 100.0; unit = "g" }
        orientation_axes = @("x", "y", "z")
        settling_time_ms = 5.0
        frequency_range = [ordered]@{ minimum_hz = 0.5; maximum_hz = 10000.0 }
        scaling_profile_refs = @((New-DefinitionReference -EntityId $ScalingRevision.entity_id -RevisionId $ScalingRevision.revision_id))
        correction_curve_refs = @()
        metadata = [ordered]@{ demo = $true }
    }
}

function New-DaqAnalogInputProfile {
    return [ordered]@{
        definition_schema_version = "emc-locus.daq-channel-profile-definition.v1"
        daq_channel_profile_id = "DAQ-DEMO-AI-10V-1MS"
        label = "Demo DAQ AI +/-10 V 1 MS/s"
        channel_kind = "analog_input"
        signal_domain = "analog_voltage"
        input_quantity = "voltage"
        input_unit = "V"
        supported_ranges = @([ordered]@{ minimum = -10.0; maximum = 10.0; unit = "V" })
        resolution_bits = 16
        max_sampling_rate = 1000000.0
        min_sampling_rate = 1.0
        coupling_modes = @("dc", "ac")
        input_modes = @("single_ended", "differential", "iepe")
        anti_alias_filter = "available"
        excitation_capabilities = @(
            [ordered]@{ excitation_kind = "iepe"; nominal_value = 4.0; unit = "mA"; external_allowed = $false }
        )
        iepe_support = $true
        isolation = "channel_to_chassis"
        synchronization = "shared_sample_clock"
        triggering = "digital_start_trigger"
        metadata = [ordered]@{ demo = $true }
    }
}

function New-CurrentAcquisitionRecipe {
    param(
        [object]$DaqRevision,
        [object]$SensorRevision,
        [object]$ScalingRevision,
        [object]$CurveRevision
    )
    return [ordered]@{
        definition_schema_version = "emc-locus.acquisition-channel-recipe-definition.v1"
        recipe_id = "REC-DEMO-CURRENT-A"
        label = "current_A through demo current probe"
        output_channel_name = "current_A"
        output_quantity = "current"
        output_unit = "A"
        daq_channel_profile_ref = (New-DefinitionReference -EntityId $DaqRevision.entity_id -RevisionId $DaqRevision.revision_id)
        sensor_definition_ref = (New-DefinitionReference -EntityId $SensorRevision.entity_id -RevisionId $SensorRevision.revision_id)
        scaling_profile_ref = (New-DefinitionReference -EntityId $ScalingRevision.entity_id -RevisionId $ScalingRevision.revision_id)
        correction_curve_refs = @((New-DefinitionReference -EntityId $CurveRevision.entity_id -RevisionId $CurveRevision.revision_id))
        sample_rate = 1000000.0
        range = [ordered]@{ minimum = -10.0; maximum = 10.0; unit = "V" }
        coupling = "dc"
        input_mode = "differential"
        excitation = [ordered]@{ excitation_kind = "none"; external_allowed = $false }
        filtering = "anti_alias_on"
        triggering = "software"
        validation_rules = @("range_within_daq", "sample_rate_within_daq", "scaling_matches_sensor")
        metadata = [ordered]@{
            demo = $true
            chain_summary = "DAQ AI -> current probe voltage -> 10mV/A scaling -> current_A"
        }
    }
}

Invoke-EmcApi -Method GET -Path "/api/v1/health" | Out-Null

$currentScaling = Ensure-ApprovedMeasurementDefinition `
    -Collection "scaling-profiles" `
    -ValidationPath "/api/v1/scaling-profile-definitions/validate" `
    -EntityId "SCL-DEMO-CURRENT-10MV-A" `
    -Definition (New-LinearScaling -Id "SCL-DEMO-CURRENT-10MV-A" -Label "Current probe 10 mV/A to A" -InputQuantity "voltage" -InputUnit "V" -OutputQuantity "current" -OutputUnit "A" -Scale 100.0)

$accelerometerScaling = Ensure-ApprovedMeasurementDefinition `
    -Collection "scaling-profiles" `
    -ValidationPath "/api/v1/scaling-profile-definitions/validate" `
    -EntityId "SCL-DEMO-ACCEL-100MV-G" `
    -Definition (New-LinearScaling -Id "SCL-DEMO-ACCEL-100MV-G" -Label "IEPE accelerometer 100 mV/g" -InputQuantity "voltage" -InputUnit "V" -OutputQuantity "acceleration" -OutputUnit "g" -Scale 10.0)

$currentTransferCurve = Ensure-ApprovedMeasurementDefinition `
    -Collection "engineering-curves" `
    -ValidationPath "/api/v1/engineering-curve-definitions/validate" `
    -EntityId "CURVE-DEMO-CURRENT-PROBE-TRANSFER" `
    -Definition (New-FrequencyCurve -Id "CURVE-DEMO-CURRENT-PROBE-TRANSFER" -CurveType "current_probe_transfer" -Label "Demo current probe transfer" -ValueId "correction_db" -ValueUnit "dB" -Values @(0.0, 0.1, 0.4, 1.2))

$antennaFactorCurve = Ensure-ApprovedMeasurementDefinition `
    -Collection "engineering-curves" `
    -ValidationPath "/api/v1/engineering-curve-definitions/validate" `
    -EntityId "CURVE-DEMO-BICONICAL-ANTENNA-FACTOR" `
    -Definition (New-FrequencyCurve -Id "CURVE-DEMO-BICONICAL-ANTENNA-FACTOR" -CurveType "antenna_factor" -Label "Demo biconical antenna factor" -ValueId "antenna_factor_db_per_m" -ValueUnit "dB_per_meter" -Values @(14.0, 16.5, 21.0, 29.0))

$cableLossCurve = Ensure-ApprovedMeasurementDefinition `
    -Collection "engineering-curves" `
    -ValidationPath "/api/v1/engineering-curve-definitions/validate" `
    -EntityId "CURVE-DEMO-RF-CABLE-1M-LOSS" `
    -Definition (New-FrequencyCurve -Id "CURVE-DEMO-RF-CABLE-1M-LOSS" -CurveType "cable_loss" -Label "Demo RF cable 1m loss" -ValueId "correction_db" -ValueUnit "dB" -Values @(0.05, 0.15, 0.55, 1.8))

$amplifierGainCurve = Ensure-ApprovedMeasurementDefinition `
    -Collection "engineering-curves" `
    -ValidationPath "/api/v1/engineering-curve-definitions/validate" `
    -EntityId "CURVE-DEMO-RF-AMPLIFIER-GAIN" `
    -Definition (New-FrequencyCurve -Id "CURVE-DEMO-RF-AMPLIFIER-GAIN" -CurveType "amplifier_gain" -Label "Demo RF amplifier gain" -ValueId "gain_db" -ValueUnit "dB" -Values @(40.0, 39.5, 38.8, 36.0))

$currentSensor = Ensure-ApprovedMeasurementDefinition `
    -Collection "sensor-definitions" `
    -ValidationPath "/api/v1/sensor-definition-definitions/validate" `
    -EntityId "SNS-DEMO-CURRENT-PROBE-10MV-A" `
    -Definition (New-CurrentProbeSensor -ScalingRevision $currentScaling -TransferCurveRevision $currentTransferCurve)

Ensure-ApprovedMeasurementDefinition `
    -Collection "sensor-definitions" `
    -ValidationPath "/api/v1/sensor-definition-definitions/validate" `
    -EntityId "SNS-DEMO-BICONICAL-ANTENNA" `
    -Definition (New-ReceivingAntennaSensor -AntennaCurveRevision $antennaFactorCurve) | Out-Null

Ensure-ApprovedMeasurementDefinition `
    -Collection "sensor-definitions" `
    -ValidationPath "/api/v1/sensor-definition-definitions/validate" `
    -EntityId "SNS-DEMO-IEPE-ACCEL-100MV-G" `
    -Definition (New-IepeAccelerometerSensor -ScalingRevision $accelerometerScaling) | Out-Null

$daqProfile = Ensure-ApprovedMeasurementDefinition `
    -Collection "daq-channel-profiles" `
    -ValidationPath "/api/v1/daq-channel-profile-definitions/validate" `
    -EntityId "DAQ-DEMO-AI-10V-1MS" `
    -Definition (New-DaqAnalogInputProfile)

Ensure-ApprovedMeasurementDefinition `
    -Collection "acquisition-channel-recipes" `
    -ValidationPath "/api/v1/acquisition-channel-recipe-definitions/validate" `
    -EntityId "REC-DEMO-CURRENT-A" `
    -Definition (New-CurrentAcquisitionRecipe -DaqRevision $daqProfile -SensorRevision $currentSensor -ScalingRevision $currentScaling -CurveRevision $currentTransferCurve) | Out-Null

$evaluation = Invoke-EmcApi -Method POST -Path "/api/v1/engineering-curves/$($cableLossCurve.entity_id)/revisions/$($cableLossCurve.revision_id)/evaluate" -Body ([ordered]@{
    axis_values = [ordered]@{ frequency = 100000000.0 }
})

Write-Host "Measurement engineering demo seed complete via API at $AgentUrl"
Write-Host "Cable-loss evaluation at 100 MHz: $($evaluation.evaluation.values.correction_db) dB"
Write-Host "Seeded current recipe: REC-DEMO-CURRENT-A"
