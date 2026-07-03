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

function Assert-DefinitionValid {
    param(
        [string]$Kind,
        [object]$Definition
    )
    $path = if ($Kind -eq "model") { "/api/v1/equipment-model-definitions/validate" } else { "/api/v1/driver-profile-definitions/validate" }
    $validation = Invoke-EmcApi -Method POST -Path $path -Body ([ordered]@{ definition = $Definition })
    if (-not $validation.valid) {
        $issues = ($validation.issues | ConvertTo-Json -Depth 20)
        throw "$Kind definition is invalid: $issues"
    }
    return $validation
}

function Get-EquipmentModelMap {
    $result = Invoke-EmcApi -Method GET -Path "/api/v1/equipment-models"
    $map = @{}
    foreach ($model in $result.equipment_models) {
        $map[$model.identity.equipment_model_id] = $model
    }
    return $map
}

function Get-DriverProfileMap {
    $result = Invoke-EmcApi -Method GET -Path "/api/v1/driver-profiles"
    $map = @{}
    foreach ($driver in $result.driver_profiles) {
        $map[$driver.identity.driver_profile_id] = $driver
    }
    return $map
}

function Approve-EquipmentRevision {
    param(
        [string]$ModelId,
        [object]$Revision
    )
    $revisionId = $Revision.revision_id
    if ($Revision.status -eq "draft") {
        Invoke-EmcApi -Method POST -Path "/api/v1/equipment-models/$ModelId/revisions/$revisionId/transitions/submit-for-review" -Body ([ordered]@{
            actor = "demo.equipment.reviewer"
            reason = "Equipment demo definition ready for review"
            operation_id = "seed-$ModelId-$revisionId-submit"
        }) | Out-Null
    }
    if ($Revision.status -eq "draft" -or $Revision.status -eq "under_review") {
        Invoke-EmcApi -Method POST -Path "/api/v1/equipment-models/$ModelId/revisions/$revisionId/transitions/approve" -Body ([ordered]@{
            actor = "demo.equipment.approver"
            reason = "Equipment demo definition approved"
            operation_id = "seed-$ModelId-$revisionId-approve"
        }) | Out-Null
    }
}

function Approve-DriverRevision {
    param(
        [string]$DriverId,
        [object]$Revision
    )
    $revisionId = $Revision.revision_id
    if ($Revision.status -eq "draft") {
        Invoke-EmcApi -Method POST -Path "/api/v1/driver-profiles/$DriverId/revisions/$revisionId/transitions/submit-for-review" -Body ([ordered]@{
            actor = "demo.driver.reviewer"
            reason = "Driver demo definition ready for review"
            operation_id = "seed-$DriverId-$revisionId-submit"
        }) | Out-Null
    }
    if ($Revision.status -eq "draft" -or $Revision.status -eq "under_review") {
        Invoke-EmcApi -Method POST -Path "/api/v1/driver-profiles/$DriverId/revisions/$revisionId/transitions/approve" -Body ([ordered]@{
            actor = "demo.driver.approver"
            reason = "Driver demo definition approved"
            operation_id = "seed-$DriverId-$revisionId-approve"
        }) | Out-Null
    }
}

function Ensure-ApprovedEquipmentModel {
    param(
        [string]$ModelId,
        [object]$Definition
    )
    $models = Get-EquipmentModelMap
    if ($models.ContainsKey($ModelId) -and $models[$ModelId].current_approved_revision) {
        Write-Host "Equipment model already approved: $ModelId"
        return $models[$ModelId]
    }

    if (-not $models.ContainsKey($ModelId)) {
        Assert-DefinitionValid -Kind "model" -Definition $Definition | Out-Null
        $created = Invoke-EmcApi -Method POST -Path "/api/v1/equipment-models" -Body ([ordered]@{
            equipment_model_id = $ModelId
            definition = $Definition
            actor = "demo.equipment.author"
            reason = "Seed equipment demo model"
            operation_id = "seed-$ModelId-create"
        })
        $revision = $created.revision
        Write-Host "Created equipment model: $ModelId"
    } else {
        $revision = $models[$ModelId].active_draft_revision
        if (-not $revision) {
            $revision = $models[$ModelId].latest_revision
        }
    }

    Approve-EquipmentRevision -ModelId $ModelId -Revision $revision
    $approved = Invoke-EmcApi -Method GET -Path "/api/v1/equipment-models/$ModelId"
    Write-Host "Approved equipment model: $ModelId"
    return $approved.equipment_model
}

function Ensure-ApprovedDriverProfile {
    param(
        [string]$DriverId,
        [string]$Label,
        [object]$Definition
    )
    $drivers = Get-DriverProfileMap
    if ($drivers.ContainsKey($DriverId) -and $drivers[$DriverId].current_approved_revision) {
        Write-Host "Driver profile already approved: $DriverId"
        return $drivers[$DriverId]
    }

    if (-not $drivers.ContainsKey($DriverId)) {
        Assert-DefinitionValid -Kind "driver" -Definition $Definition | Out-Null
        $created = Invoke-EmcApi -Method POST -Path "/api/v1/driver-profiles" -Body ([ordered]@{
            driver_profile_id = $DriverId
            label = $Label
            definition = $Definition
            actor = "demo.driver.author"
            reason = "Seed equipment demo driver"
            operation_id = "seed-$DriverId-create"
        })
        $revision = $created.revision
        Write-Host "Created driver profile: $DriverId"
    } else {
        $revision = $drivers[$DriverId].active_draft_revision
        if (-not $revision) {
            $revision = $drivers[$DriverId].latest_revision
        }
    }

    Approve-DriverRevision -DriverId $DriverId -Revision $revision
    $approved = Invoke-EmcApi -Method GET -Path "/api/v1/driver-profiles/$DriverId"
    Write-Host "Approved driver profile: $DriverId"
    return $approved.driver_profile
}

function New-Value {
    param(
        [string]$Name,
        [string]$ValueType,
        [string]$Quantity,
        [string]$Unit,
        [bool]$Required = $true,
        [double]$Minimum = [double]::NaN,
        [double]$Maximum = [double]::NaN,
        [string[]]$EnumValues = @()
    )
    $value = [ordered]@{
        name = $Name
        value_type = $ValueType
        quantity = $Quantity
        unit = $Unit
        required = $Required
        enum_values = $EnumValues
    }
    if (-not [double]::IsNaN($Minimum)) { $value.minimum = $Minimum }
    if (-not [double]::IsNaN($Maximum)) { $value.maximum = $Maximum }
    return $value
}

function New-ScpiPowerMeterModel {
    return [ordered]@{
        definition_schema_version = "emc-locus.equipment-model-definition.v1"
        manufacturer = "R&S"
        model_name = "NRP6AN"
        variant = "FWD"
        equipment_class = "controllable_instrument"
        category_code = "power_meter"
        specifications = @(
            [ordered]@{ specification_id = "frequency_range"; label = "Frequency range"; quantity = "frequency"; unit = "GHz"; minimum = 0.009; maximum = 6.0 },
            [ordered]@{ specification_id = "power_range"; label = "Power range"; quantity = "power"; unit = "dBm"; minimum = -70.0; maximum = 23.0 }
        )
        signal_ports = @(
            [ordered]@{ port_id = "rf_input"; label = "RF INPUT 50 ohm"; direction = "input"; signal_domain = "rf"; connector_type = "N"; quantity = "power"; unit = "dBm"; impedance = 50.0; frequency_min = 9000.0; frequency_max = 6000000000.0 }
        )
        communication_interfaces = @(
            [ordered]@{
                interface_id = "visa_usbtmc"
                label = "VISA USBTMC"
                transport_kind = "usb"
                access_provider_kind = "visa"
                protocol_kind = "scpi"
                required = $false
                default_interface = $false
                default_configuration = [ordered]@{ resource_pattern = "USB0::*::INSTR"; open_timeout_ms = 3000; io_timeout_ms = 2000; write_terminator = "\n"; read_terminator = "\n" }
                identification_strategy = [ordered]@{ strategy_id = "scpi_idn"; strategy_type = "scpi_idn"; query = "*IDN?"; response_regex = "^Rohde&Schwarz,NRP6AN," }
            },
            [ordered]@{
                interface_id = "visa_tcpip"
                label = "VISA TCPIP"
                transport_kind = "ethernet_tcp"
                access_provider_kind = "visa"
                protocol_kind = "scpi"
                required = $false
                default_interface = $false
                default_configuration = [ordered]@{ resource_pattern = "TCPIP0::*::inst0::INSTR"; open_timeout_ms = 3000; io_timeout_ms = 2000; write_terminator = "\n"; read_terminator = "\n" }
                identification_strategy = [ordered]@{ strategy_id = "scpi_idn"; strategy_type = "scpi_idn"; query = "*IDN?"; response_regex = "^Rohde&Schwarz,NRP6AN," }
            },
            [ordered]@{
                interface_id = "tcp_scpi"
                label = "Native TCP SCPI"
                transport_kind = "ethernet_tcp"
                access_provider_kind = "native_tcp"
                protocol_kind = "scpi"
                required = $true
                default_interface = $true
                default_configuration = [ordered]@{ host = "127.0.0.1"; port = 5025; connect_timeout_ms = 1000; read_timeout_ms = 1000; write_timeout_ms = 1000; write_terminator = "\n"; read_terminator = "\n" }
                identification_strategy = [ordered]@{ strategy_id = "scpi_idn"; strategy_type = "scpi_idn"; query = "*IDN?"; response_regex = "^Rohde&Schwarz,NRP6AN," }
            }
        )
        capabilities = @(
            [ordered]@{ capability_id = "initialize"; label = "Initialize"; description = "Prepare SCPI session."; capability_kind = "initialize"; inputs = @(); outputs = @(); safety_class = "read_only" },
            [ordered]@{ capability_id = "terminate"; label = "Terminate"; description = "Close SCPI session."; capability_kind = "terminate"; inputs = @(); outputs = @(); safety_class = "read_only" },
            [ordered]@{ capability_id = "set_frequency"; label = "Set frequency"; description = "Set measurement frequency."; capability_kind = "set_frequency"; inputs = @((New-Value -Name "frequency_hz" -ValueType "number" -Quantity "frequency" -Unit "Hz" -Minimum 9000 -Maximum 6000000000)); outputs = @((New-Value -Name "applied_frequency_hz" -ValueType "number" -Quantity "frequency" -Unit "Hz")); safety_class = "configuration_change" },
            [ordered]@{ capability_id = "measure_powers"; label = "Measure powers"; description = "Measure forward and reverse RF power."; capability_kind = "measure_power"; inputs = @(); outputs = @((New-Value -Name "forward_power_dbm" -ValueType "number" -Quantity "power" -Unit "dBm"), (New-Value -Name "reverse_power_dbm" -ValueType "number" -Quantity "power" -Unit "dBm")); required_signal_ports = @("rf_input"); safety_class = "read_only" },
            [ordered]@{ capability_id = "read_errors"; label = "Read errors"; description = "Read instrument error queue."; capability_kind = "read_errors"; inputs = @(); outputs = @((New-Value -Name "error_text" -ValueType "text" -Quantity "text" -Unit "dimensionless")); safety_class = "read_only" }
        )
        metadata = [ordered]@{ demo = $true }
    }
}

function New-SerialAmplifierModel {
    return [ordered]@{
        definition_schema_version = "emc-locus.equipment-model-definition.v1"
        manufacturer = "Demo"
        model_name = "RF Amplifier"
        variant = "Serial"
        equipment_class = "controllable_instrument"
        category_code = "rf_amplifier"
        specifications = @(
            [ordered]@{ specification_id = "frequency_range"; label = "Frequency range"; quantity = "frequency"; unit = "MHz"; minimum = 80.0; maximum = 1000.0 },
            [ordered]@{ specification_id = "output_power_max"; label = "Output power max"; quantity = "power"; unit = "W"; maximum = 1000.0 }
        )
        signal_ports = @(
            [ordered]@{ port_id = "rf_input"; label = "RF input"; direction = "input"; signal_domain = "rf"; connector_type = "N"; quantity = "power"; unit = "dBm"; impedance = 50.0 },
            [ordered]@{ port_id = "rf_output"; label = "RF output"; direction = "output"; signal_domain = "rf"; connector_type = "N"; quantity = "power"; unit = "W"; impedance = 50.0 }
        )
        communication_interfaces = @(
            [ordered]@{
                interface_id = "serial_ascii"
                label = "RS-232 ASCII"
                transport_kind = "serial"
                access_provider_kind = "native_serial"
                protocol_kind = "raw_ascii"
                required = $true
                default_interface = $true
                default_configuration = [ordered]@{ port_name = "COM1"; baud_rate = 9600; data_bits = 8; stop_bits = 1; parity = "none"; flow_control = "none"; write_terminator = "\r"; read_terminator = "\r"; timeout_ms = 1000; inter_command_delay_ms = 50 }
                identification_strategy = [ordered]@{ strategy_id = "text_id"; strategy_type = "text_query"; query = "ID"; response_regex = "^DEMO-AMP" }
            }
        )
        capabilities = @(
            [ordered]@{ capability_id = "read_fault"; label = "Read fault"; description = "Read amplifier fault status."; capability_kind = "read_fault"; inputs = @(); outputs = @((New-Value -Name "fault_status" -ValueType "text" -Quantity "text" -Unit "dimensionless")); safety_class = "read_only" },
            [ordered]@{ capability_id = "activate_rf"; label = "Activate RF"; description = "Enable amplifier output."; capability_kind = "activate_rf"; inputs = @(); outputs = @(); safety_class = "energizes_output" },
            [ordered]@{ capability_id = "deactivate_rf"; label = "Deactivate RF"; description = "Disable amplifier output."; capability_kind = "deenergizes_output"; inputs = @(); outputs = @(); safety_class = "deenergizes_output" },
            [ordered]@{ capability_id = "initialize"; label = "Initialize"; description = "Check and activate amplifier."; capability_kind = "initialize"; inputs = @(); outputs = @(); safety_class = "energizes_output" },
            [ordered]@{ capability_id = "terminate"; label = "Terminate"; description = "Return amplifier to safe state."; capability_kind = "terminate"; inputs = @(); outputs = @(); safety_class = "deenergizes_output" }
        )
        metadata = [ordered]@{ demo = $true }
    }
}

function New-CanPowerUnitModel {
    return [ordered]@{
        definition_schema_version = "emc-locus.equipment-model-definition.v1"
        manufacturer = "Demo"
        model_name = "CAN Controlled Power Unit"
        variant = "48V"
        equipment_class = "controllable_instrument"
        category_code = "can_power_unit"
        specifications = @(
            [ordered]@{ specification_id = "dc_voltage"; label = "DC bus voltage"; quantity = "voltage"; unit = "V"; nominal = 48.0; maximum = 60.0 }
        )
        signal_ports = @(
            [ordered]@{ port_id = "dc_output"; label = "DC output"; direction = "output"; signal_domain = "analog_electrical"; connector_type = "terminal"; quantity = "voltage"; unit = "V"; voltage_max = 60.0 }
        )
        communication_interfaces = @(
            [ordered]@{
                interface_id = "can0"
                label = "CAN control bus"
                transport_kind = "can"
                access_provider_kind = "socketcan"
                protocol_kind = "can_frames"
                required = $true
                default_interface = $true
                default_configuration = [ordered]@{ provider = "socketcan"; channel = "can0"; bitrate = 500000; fd_enabled = $false; extended_id = $false; timeout_ms = 1000 }
                identification_strategy = [ordered]@{ strategy_id = "can_handshake"; strategy_type = "can_handshake"; parameters = [ordered]@{ request_id = 256; response_id = 257 } }
            }
        )
        capabilities = @(
            [ordered]@{ capability_id = "read_status"; label = "Read status"; description = "Read CAN status word."; capability_kind = "read_status"; inputs = @(); outputs = @((New-Value -Name "status_word" -ValueType "integer" -Quantity "dimensionless" -Unit "dimensionless")); safety_class = "read_only" },
            [ordered]@{ capability_id = "set_mode"; label = "Set mode"; description = "Set operating mode."; capability_kind = "set_mode"; inputs = @((New-Value -Name "mode" -ValueType "text" -Quantity "text" -Unit "dimensionless" -EnumValues @("standby", "test", "run"))); outputs = @(); safety_class = "configuration_change" },
            [ordered]@{ capability_id = "enable_output"; label = "Enable output"; description = "Enable power output."; capability_kind = "activate_rf"; inputs = @(); outputs = @(); required_signal_ports = @("dc_output"); safety_class = "energizes_output" },
            [ordered]@{ capability_id = "disable_output"; label = "Disable output"; description = "Disable power output."; capability_kind = "deactivate_rf"; inputs = @(); outputs = @(); required_signal_ports = @("dc_output"); safety_class = "deenergizes_output" }
        )
        metadata = [ordered]@{ demo = $true }
    }
}

function New-ManualAntennaModel {
    return [ordered]@{
        definition_schema_version = "emc-locus.equipment-model-definition.v1"
        manufacturer = "Demo"
        model_name = "Manual Antenna"
        variant = "Broadband"
        equipment_class = "manual_equipment"
        category_code = "antenna"
        specifications = @(
            [ordered]@{ specification_id = "frequency_range"; label = "Frequency range"; quantity = "frequency"; unit = "MHz"; minimum = 30.0; maximum = 1000.0 }
        )
        signal_ports = @(
            [ordered]@{ port_id = "rf_output"; label = "RF output"; direction = "output"; signal_domain = "rf"; connector_type = "N"; quantity = "electric_field"; unit = "dBuV_per_m" }
        )
        communication_interfaces = @()
        capabilities = @(
            [ordered]@{ capability_id = "receive_rf_signal"; label = "Receive RF signal"; description = "Manual antenna receives RF field."; capability_kind = "receive_rf_signal"; inputs = @(); outputs = @((New-Value -Name "received_level_dbuv_m" -ValueType "number" -Quantity "electric_field" -Unit "dBuV_per_m")); required_signal_ports = @("rf_output"); safety_class = "read_only" },
            [ordered]@{ capability_id = "provide_antenna_factor"; label = "Provide antenna factor"; description = "References a future engineering curve revision."; capability_kind = "provide_antenna_factor"; inputs = @((New-Value -Name "frequency_hz" -ValueType "number" -Quantity "frequency" -Unit "Hz")); outputs = @((New-Value -Name "antenna_factor_db" -ValueType "number" -Quantity "dimensionless" -Unit "dB")); safety_class = "read_only" }
        )
        metadata = [ordered]@{ future_engineering_curve_reference = "EngineeringCurveRevision:antenna-factor"; demo = $true }
    }
}

function New-ScpiPowerMeterDriver {
    param([object]$ModelRevision)
    return [ordered]@{
        definition_schema_version = "emc-locus.driver-profile-definition.v1"
        equipment_model_id = "EQM-DEMO-NRP6AN-FWD"
        supported_model_revision_id = $ModelRevision.revision_id
        supported_model_definition_checksum = $ModelRevision.definition_checksum
        supported_firmware_ranges = @("*")
        communication_profiles = @("tcp_scpi", "visa_usbtmc", "visa_tcpip")
        actions = @(
            [ordered]@{ action_id = "initialize"; label = "Initialize"; description = "Clear and identify instrument."; implements_capability_id = "initialize"; inputs = @(); outputs = @(); safety_class = "read_only"; default_timeout_ms = 2000; script = [ordered]@{ steps = @([ordered]@{ step_id = "clear"; step_type = "io_write"; interface_id = "tcp_scpi"; payload_format = "text"; payload = "*CLS" }, [ordered]@{ step_id = "identify"; step_type = "io_query"; interface_id = "tcp_scpi"; payload_format = "text"; payload = "*IDN?"; response_binding = '${state.idn}' }) } },
            [ordered]@{ action_id = "terminate"; label = "Terminate"; description = "Return to local state."; implements_capability_id = "terminate"; inputs = @(); outputs = @(); safety_class = "read_only"; default_timeout_ms = 1000; script = [ordered]@{ steps = @([ordered]@{ step_id = "local"; step_type = "io_write"; interface_id = "tcp_scpi"; payload_format = "text"; payload = "SYST:LOC" }) } },
            [ordered]@{ action_id = "set_frequency"; label = "Set frequency"; description = "Set the measurement frequency."; implements_capability_id = "set_frequency"; inputs = @((New-Value -Name "frequency_hz" -ValueType "number" -Quantity "frequency" -Unit "Hz" -Minimum 9000 -Maximum 6000000000)); outputs = @((New-Value -Name "applied_frequency_hz" -ValueType "number" -Quantity "frequency" -Unit "Hz")); safety_class = "configuration_change"; default_timeout_ms = 1000; script = [ordered]@{ steps = @([ordered]@{ step_id = "write_frequency"; step_type = "io_write"; interface_id = "tcp_scpi"; payload_format = "text"; payload = "SENS:FREQ ${input.frequency_hz}" }, [ordered]@{ step_id = "record_applied"; step_type = "set_variable"; variable = '${result.applied_frequency_hz}'; value = 1000000 }) } },
            [ordered]@{ action_id = "measure_powers"; label = "Measure powers"; description = "Measure forward and reverse powers."; implements_capability_id = "measure_powers"; inputs = @(); outputs = @((New-Value -Name "forward_power_dbm" -ValueType "number" -Quantity "power" -Unit "dBm"), (New-Value -Name "reverse_power_dbm" -ValueType "number" -Quantity "power" -Unit "dBm")); safety_class = "read_only"; default_timeout_ms = 1500; script = [ordered]@{ steps = @([ordered]@{ step_id = "query_forward"; step_type = "io_query"; interface_id = "tcp_scpi"; payload_format = "text"; payload = "MEAS:POW:FORW?"; response_binding = '${result.forward_power_dbm}' }, [ordered]@{ step_id = "query_reverse"; step_type = "io_query"; interface_id = "tcp_scpi"; payload_format = "text"; payload = "MEAS:POW:REV?"; response_binding = '${result.reverse_power_dbm}' }) } },
            [ordered]@{ action_id = "read_errors"; label = "Read errors"; description = "Read SCPI error queue."; implements_capability_id = "read_errors"; inputs = @(); outputs = @((New-Value -Name "error_text" -ValueType "text" -Quantity "text" -Unit "dimensionless")); safety_class = "read_only"; default_timeout_ms = 1000; script = [ordered]@{ steps = @([ordered]@{ step_id = "query_error"; step_type = "io_query"; interface_id = "tcp_scpi"; payload_format = "text"; payload = "SYST:ERR?"; response_binding = '${result.error_text}' }) } }
        )
        error_check_action_id = "read_errors"
        metadata = [ordered]@{ demo = $true }
    }
}

function New-SerialAmplifierDriver {
    param([object]$ModelRevision)
    return [ordered]@{
        definition_schema_version = "emc-locus.driver-profile-definition.v1"
        equipment_model_id = "EQM-DEMO-SERIAL-AMP"
        supported_model_revision_id = $ModelRevision.revision_id
        supported_model_definition_checksum = $ModelRevision.definition_checksum
        supported_firmware_ranges = @("*")
        communication_profiles = @("serial_ascii")
        actions = @(
            [ordered]@{ action_id = "read_fault"; label = "Read fault"; description = "Read amplifier fault state."; implements_capability_id = "read_fault"; inputs = @(); outputs = @((New-Value -Name "fault_status" -ValueType "text" -Quantity "text" -Unit "dimensionless")); safety_class = "read_only"; default_timeout_ms = 1000; script = [ordered]@{ steps = @([ordered]@{ step_id = "query_fault"; step_type = "io_query"; interface_id = "serial_ascii"; payload_format = "text"; payload = "FLT?"; response_binding = '${result.fault_status}' }) } },
            [ordered]@{ action_id = "deactivate"; label = "Deactivate"; description = "Disable amplifier output."; implements_capability_id = "deactivate_rf"; inputs = @(); outputs = @(); safety_class = "deenergizes_output"; default_timeout_ms = 1000; script = [ordered]@{ steps = @([ordered]@{ step_id = "rf_off"; step_type = "io_write"; interface_id = "serial_ascii"; payload_format = "text"; payload = "RF OFF" }) } },
            [ordered]@{ action_id = "activate"; label = "Activate"; description = "Enable amplifier output."; implements_capability_id = "activate_rf"; inputs = @(); outputs = @(); safety_class = "energizes_output"; default_timeout_ms = 1000; safe_state_action_id = "deactivate"; requires_operator_confirmation = $true; script = [ordered]@{ steps = @([ordered]@{ step_id = "rf_on"; step_type = "io_write"; interface_id = "serial_ascii"; payload_format = "text"; payload = "RF ON" }) } },
            [ordered]@{ action_id = "initialize"; label = "Initialize"; description = "Status read, condition, operator message, bounded wait, activation and verification."; implements_capability_id = "initialize"; inputs = @(); outputs = @(); safety_class = "energizes_output"; default_timeout_ms = 5000; safe_state_action_id = "deactivate"; script = [ordered]@{ steps = @([ordered]@{ step_id = "read_initial_fault"; step_type = "call_action"; action_id = "read_fault" }, [ordered]@{ step_id = "mirror_initial_fault"; step_type = "set_variable"; variable = '${state.fault_status}'; value = "0" }, [ordered]@{ step_id = "fault_message"; step_type = "if"; expression = '${state.fault_status} != "0"'; steps = @([ordered]@{ step_id = "operator_fault_warning"; step_type = "operator_message"; message = "Amplifier reports a fault before activation." }) }, [ordered]@{ step_id = "bounded_wait_ready"; step_type = "loop_until"; expression = '${state.fault_status} == "0"'; max_iterations = 3; steps = @([ordered]@{ step_id = "poll_fault"; step_type = "call_action"; action_id = "read_fault" }, [ordered]@{ step_id = "mirror_polled_fault"; step_type = "set_variable"; variable = '${state.fault_status}'; value = "0" }, [ordered]@{ step_id = "wait_100ms"; step_type = "delay"; duration_ms = 100 }) }, [ordered]@{ step_id = "activate_output"; step_type = "call_action"; action_id = "activate" }, [ordered]@{ step_id = "verify_fault"; step_type = "call_action"; action_id = "read_fault" }) } },
            [ordered]@{ action_id = "terminate"; label = "Terminate"; description = "Disable amplifier output."; implements_capability_id = "terminate"; inputs = @(); outputs = @(); safety_class = "deenergizes_output"; default_timeout_ms = 1000; safe_state_action_id = "deactivate"; script = [ordered]@{ steps = @([ordered]@{ step_id = "call_deactivate"; step_type = "call_action"; action_id = "deactivate" }) } }
        )
        safe_state_action_id = "deactivate"
        metadata = [ordered]@{ demo = $true }
    }
}

function New-CanPowerUnitDriver {
    param([object]$ModelRevision)
    return [ordered]@{
        definition_schema_version = "emc-locus.driver-profile-definition.v1"
        equipment_model_id = "EQM-DEMO-CAN-POWER"
        supported_model_revision_id = $ModelRevision.revision_id
        supported_model_definition_checksum = $ModelRevision.definition_checksum
        supported_firmware_ranges = @("*")
        communication_profiles = @("can0")
        actions = @(
            [ordered]@{ action_id = "read_status"; label = "Read status"; description = "Read status word from CAN."; implements_capability_id = "read_status"; inputs = @(); outputs = @((New-Value -Name "status_word" -ValueType "integer" -Quantity "dimensionless" -Unit "dimensionless")); safety_class = "read_only"; default_timeout_ms = 1000; script = [ordered]@{ steps = @([ordered]@{ step_id = "request_status"; step_type = "can_request_response"; interface_id = "can0"; frame = [ordered]@{ arbitration_id = 256; extended = $false; remote_frame = $false; data = @(1); dlc = 1 }; response_binding = '${result.status_word}' }) } },
            [ordered]@{ action_id = "set_mode"; label = "Set mode"; description = "Set CAN operating mode."; implements_capability_id = "set_mode"; inputs = @((New-Value -Name "mode" -ValueType "text" -Quantity "text" -Unit "dimensionless" -EnumValues @("standby", "test", "run"))); outputs = @(); safety_class = "configuration_change"; default_timeout_ms = 1000; script = [ordered]@{ steps = @([ordered]@{ step_id = "send_mode"; step_type = "can_send"; interface_id = "can0"; frame = [ordered]@{ arbitration_id = 258; extended = $false; remote_frame = $false; data = @(2, 1); dlc = 2 } }) } },
            [ordered]@{ action_id = "disable_output"; label = "Disable output"; description = "Disable DC output."; implements_capability_id = "disable_output"; inputs = @(); outputs = @(); safety_class = "deenergizes_output"; default_timeout_ms = 1000; script = [ordered]@{ steps = @([ordered]@{ step_id = "send_disable"; step_type = "can_send"; interface_id = "can0"; frame = [ordered]@{ arbitration_id = 259; extended = $false; remote_frame = $false; data = @(0); dlc = 1 } }) } },
            [ordered]@{ action_id = "enable_output"; label = "Enable output"; description = "Enable DC output."; implements_capability_id = "enable_output"; inputs = @(); outputs = @(); safety_class = "energizes_output"; default_timeout_ms = 1000; safe_state_action_id = "disable_output"; requires_operator_confirmation = $true; script = [ordered]@{ steps = @([ordered]@{ step_id = "send_enable"; step_type = "can_send"; interface_id = "can0"; frame = [ordered]@{ arbitration_id = 259; extended = $false; remote_frame = $false; data = @(1); dlc = 1 } }) } }
        )
        safe_state_action_id = "disable_output"
        metadata = [ordered]@{ demo = $true }
    }
}

Invoke-EmcApi -Method GET -Path "/api/v1/health" | Out-Null

$powerMeter = Ensure-ApprovedEquipmentModel -ModelId "EQM-DEMO-NRP6AN-FWD" -Definition (New-ScpiPowerMeterModel)
$amplifier = Ensure-ApprovedEquipmentModel -ModelId "EQM-DEMO-SERIAL-AMP" -Definition (New-SerialAmplifierModel)
$canUnit = Ensure-ApprovedEquipmentModel -ModelId "EQM-DEMO-CAN-POWER" -Definition (New-CanPowerUnitModel)
Ensure-ApprovedEquipmentModel -ModelId "EQM-DEMO-MANUAL-ANTENNA" -Definition (New-ManualAntennaModel) | Out-Null

Ensure-ApprovedDriverProfile -DriverId "DRV-DEMO-NRP6AN-SCPI" -Label "R&S NRP6AN SCPI" -Definition (New-ScpiPowerMeterDriver -ModelRevision $powerMeter.current_approved_revision) | Out-Null
Ensure-ApprovedDriverProfile -DriverId "DRV-DEMO-SERIAL-AMP" -Label "Demo serial amplifier" -Definition (New-SerialAmplifierDriver -ModelRevision $amplifier.current_approved_revision) | Out-Null
Ensure-ApprovedDriverProfile -DriverId "DRV-DEMO-CAN-POWER" -Label "Demo CAN power unit" -Definition (New-CanPowerUnitDriver -ModelRevision $canUnit.current_approved_revision) | Out-Null

$providers = Invoke-EmcApi -Method GET -Path "/api/v1/equipment/communication-providers"
Write-Host "Equipment demo seed complete via API at $AgentUrl"
Write-Host "Provider status:"
foreach ($provider in $providers.providers) {
    $status = if ($provider.available) { "available" } else { "unavailable" }
    $reason = if ($provider.reason) { " - $($provider.reason)" } else { "" }
    Write-Host "  $($provider.provider): $status$reason"
}
