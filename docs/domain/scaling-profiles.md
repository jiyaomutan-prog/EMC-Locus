# Time-Domain Sample Conversions

The operator-facing term is **conversion temporelle**. The Rust aggregate keeps
the technical name `ScalingProfileDefinition`, but its scope is deliberately
narrow: it converts one time-domain sample from the electrical or raw quantity
into the physical quantity used by the laboratory.

The usual linear law is:

```text
physical_value = gain * input_sample + offset
```

Examples include volts to amperes for a current probe, volts to acceleration
for an accelerometer, or ADC counts to volts. The definition declares
`signal_representation = time_domain_samples`; spectrum correction does not
belong here.

## Conversion Methods

- `identity`;
- `linear`, with explicit gain and offset;
- `two_point`;
- `polynomial`;
- `lookup_table`;
- `piecewise_linear`;
- `expression` using the controlled expression subset.

The expression subset allows `x`, `input`, `temperature`, and `frequency`, and
the functions `pow`, `sqrt`, `log10`, `ln`, `abs`, `min`, and `max`. It does not
use unrestricted evaluation.

## Overload And Clipping

`input_limits` optionally defines the finite minimum and maximum raw input for
which the conversion is usable. `handling` records the future runtime policy:

- `warn`: keep the value and report overload;
- `reject`: refuse the out-of-range value;
- `mark_clipped`: retain it with explicit clipping evidence.

These limits describe the input chain, such as a DAQ range of `-10 V` to
`+10 V`. They are not frequency coverage and do not replace the safe operating
limits of the physical equipment.

## Traceability Boundary

A conversion definition is revisioned, approved, and checksum-addressed. It is
not a calibration event. Calibration records the state of a serialized asset,
date, provider, uncertainty, and evidence. A transducer sheet or certificate
may justify the conversion, while the controlled conversion remains the
machine-readable contract referenced by an equipment signal path. When a
conversion is measured for one serialized transducer, a `0.17.0` station setup
can pin that characterization id and checksum for the physical mounting.

Validation checks quantities, units, finite parameters, ordered lookup inputs,
conversion expression safety, and coherent overload bounds. This release does
not execute a live DAQ channel or apply the conversion to acquired samples.
