# 2026-07-11 - Equipment Physics Classification Session

## Objective

Extend the `0.11.x` equipment model so it is not only organized around
commercial equipment families. Equipment definitions must describe their role in
a real measurement chain: energy source, signal source, RF network element,
sensor, actuator, measurement instrument, acquisition device, converter, control
system, software system, facility, or manual accessory.

## Work Completed

- Added `functional_role`, `signal_domains`, port `directionality`, port
  `flow_role`, and `technology_tags` to the typed Rust equipment definition.
- Added explicit `adc_converter`, `dac_converter`, and `can_bus` vocabulary to
  avoid ambiguity between French ADC/CAN wording and Controller Area Network.
- Renamed CAN-bus communication contracts from `can`/`can_frames` and `can_*`
  script steps to `can_bus`/`can_bus_frames` and `can_bus_*`.
- Added static core validation for RF impedance, communication ports used as
  measurement ports, through-port topology, sensor input/output shape,
  signal-source outputs, measurement-instrument inputs, software-system RF port
  exceptions, and ambiguous `can`/`adc`/`dac` category codes.
- Updated LAB CONSOLE model defaults, local search, fixtures and seed
  equipment examples to use the physics-based classification.
- Updated equipment domain and communication documentation.

## Validation Notes

Executed checks:

- `cargo fmt --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test -p emc-locus-core equipment`: passed, 19 targeted tests.
- `cargo test -p emc-locus-agent equipment`: passed, 2 targeted tests.
- `cargo test --workspace`: passed, including 46 agent tests and 173 core
  tests.
- `py -m compileall python\emc_locus`: passed.
- `PYTHONPATH=python py -m unittest discover -s python\tests`: passed, 164
  tests.
- LAB CONSOLE `tsc --noEmit`: passed through the local `node_modules` binary.
- LAB CONSOLE `vitest run`: passed, 6 tests.
- LAB CONSOLE `eslint .`: passed.
- LAB CONSOLE production build: passed.

Note: `pnpm run typecheck` was not used because the runtime wrapper attempted a
fresh install and stopped on the existing `esbuild` build-script approval gate.
The same TypeScript compiler was executed directly from `node_modules`.

## Limits

This pass prepares future instrumentation-chain reasoning but does not implement
the full chain builder. It validates a single equipment model statically; it
does not yet connect two equipment models or compute end-to-end compatibility.
