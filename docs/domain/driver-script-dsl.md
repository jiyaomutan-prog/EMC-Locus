# Driver Script DSL

Driver scripts are stored as structured JSON ASTs, not as flat BAT-style text
macros. LAB CONSOLE may display a readable textual representation, but the AST
is the source of truth.

## Step Families

`0.11.0` models:

- generic I/O: `io_write`, `io_read`, `io_query`;
- CAN bus I/O: `can_bus_send`, `can_bus_receive`, `can_bus_request_response`;
- processing: `set_variable`, `parse_number`, `parse_text`, `parse_csv`,
  `parse_regex`, `convert_unit`, `calculate`, `assert`;
- control flow: `if`, `loop_until`, `repeat`, `call_action`, `return`;
- operator interaction: `operator_message`, `operator_confirmation`,
  `operator_input`;
- timing: `delay`, `wait_until`;
- controlled extension point: `call_registered_adapter`.

Every loop must be bounded by `max_iterations` and/or `timeout_ms`.

## Variables

Scripts use one explicit variable syntax:

```text
${input.frequency_hz}
${state.answer}
${result.forward_power_dbm}
${context.instrument_id}
```

Validation detects unknown variables in expressions, duplicate step ids,
missing payloads, missing interfaces, unknown called actions, declared outputs
that are never produced, and CAN bus steps targeting non-CAN-bus contracts.

## Simulation

The deterministic simulator executes the same AST shape intended for the future
runtime. Delays are virtualized. The trace records step index, step type,
virtual time, request/response, variable changes, assertion result, virtual
duration and status.
