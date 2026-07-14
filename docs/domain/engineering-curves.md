# Frequency-Response Corrections

The operator-facing term is **réponse fréquentielle**. The Rust aggregate keeps
the technical name `EngineeringCurveDefinition`, but its domain is explicit:
it describes how a measured spectrum is compensated as a function of
frequency. It declares `signal_representation = frequency_domain_spectrum`.

This differs from a time-domain conversion. A conversion applies gain and
offset to each sampled value. A frequency response applies an amplitude value,
and optionally a phase value, at each spectral frequency.

## Components And Operations

Every definition has one amplitude component. A phase component is optional.
Each component declares the operation applied by a future runtime:

- `add` or `subtract`, typically for logarithmic dB values or phase;
- `multiply` or `divide`, typically for linear ratios.

The operation is part of the controlled contract. It must not be guessed from
the curve label or from whether stored values happen to be positive or
negative. Amplitude is dimensionless and may be expressed as `dB` or a linear
ratio. Phase uses an angle unit such as `deg` or `rad`.

## RF Cable Example

An RF cable model exposes two RF through ports and a declared operating band
`Fmin` to `Fmax`. Its controlled loss response can be recorded as:

```text
frequency_hz,cable_loss_db
10000000,0.2
100000000,1.2
1000000000,2.2
```

For a method where measured level is compensated by cable loss, the amplitude
component uses `operation = add`. Another laboratory method may deliberately
choose a different operation; the revision makes that choice auditable. An
optional `phase_correction_deg` column can be added when phase traceability is
required.

The point range provides the effective frequency coverage. Interpolation modes
are `linear_x_linear_y`, `log_x_linear_y`, `linear_x_log_y`, `nearest`,
`step_previous`, and `step_next`. Extrapolation is explicitly `forbidden`,
`clamp`, `warn`, or `allow`.

## Traceability Boundary

The response is a controlled machine-readable artifact, not a metrology
certificate. Its source reference and checksum can point to the certificate or
calculation sheet maintained by the metrologist. Equipment paths pin reusable
model-level responses by identity, approved revision, and definition checksum.
A `0.17.0` physical station setup instead pins the chosen characterization id
and checksum when the response was measured for one serial-numbered asset.

Validation requires a frequency axis, a unique amplitude component, at most one
phase component, coherent units, complete point values, and evaluable
interpolation. This release can evaluate the stored curve deterministically and
can retain the selected serial-specific response in a ready station setup. It
does not acquire a spectrum, perform an FFT, or apply the correction to live
data.
