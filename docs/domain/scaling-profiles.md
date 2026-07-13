# Scaling Profiles

Release `0.13.0` introduces `ScalingProfileDefinition` for reusable signal to
engineering-value transformations. A scaling profile describes the mathematical
conversion, for example volts to amperes for a current probe, volts to
acceleration for an accelerometer, or a lookup table for a transducer sheet.

A scaling profile is not a calibration event. A calibration event records that
a physical asset was calibrated at a date, by a provider, with evidence and
uncertainty. A scaling profile is a controlled transformation definition that
may be referenced by many sensors or channel recipes.

## Supported Kinds

- `identity`;
- `linear`, using `output = scale * input + offset`;
- `two_point`;
- `polynomial`;
- `lookup_table`;
- `piecewise_linear`;
- `expression`.

The expression mode uses a limited DSL only. Allowed variables are `x`,
`input`, `temperature`, and `frequency`; allowed functions include `pow`,
`sqrt`, `log10`, `ln`, `abs`, `min`, and `max`. There is no unsafe eval.

## CSV Lookup Format

Lookup-table and piecewise scaling data use a simple two-column CSV:

```text
input,output
0.0,0.0
0.01,1.0
```

Decimal points use `.`, whitespace is trimmed, non-numeric values are rejected,
and duplicate input values are rejected unless a future explicit step policy
allows them.

## Validation Boundary

Validation checks quantities, units, numeric finite parameters, monotonic
lookup input where required, explicit interpolation/extrapolation policies, and
expression safety. Evaluation inputs and computed outputs must remain finite;
non-finite scaled engineering values are rejected instead of being returned as
traceability evidence. It does not execute a live DAQ channel and does not
replace metrology evidence for a serialized transducer.
