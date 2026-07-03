# Measurement Capabilities

Measurement capabilities describe what equipment can do independently from the
driver syntax used to do it.

Examples:

- `set_frequency`;
- `measure_power`;
- `measure_forward_power`;
- `measure_reverse_power`;
- `acquire_waveform`;
- `generate_signal`;
- `read_status`;
- `read_errors`;
- `activate_rf`;
- `deactivate_rf`;
- `move_position`.

Each capability declares typed inputs, outputs, optional engineering
constraints, required signal ports and a safety class. A future test template or
instrumentation chain should be able to request a category and capability
without hard-coding a manufacturer or driver.

## Units

`0.11.0` validates the minimal unit registry required for the equipment catalog:

```text
Hz kHz MHz GHz s ms us ns V mV uV A mA uA W mW dBm dBuV
dBuV_per_m dB dB_per_m ohm m cm mm deg rad Celsius percent dimensionless
```

The registry validates unit family compatibility and simple SI-prefix
conversion. It distinguishes logarithmic values from linear values. Text,
boolean and binary values use the explicit `dimensionless` unit.
