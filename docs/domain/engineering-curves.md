# Engineering Curves

Release `0.13.0` introduces `EngineeringCurveDefinition` for reusable
correction artifacts such as antenna factor, cable loss, amplifier gain,
attenuator loss, current-probe transfer, voltage-probe transfer, sensor
frequency response, phase response, uncertainty, VSWR, S-parameter magnitude,
site characterization, and generic correction curves.

An engineering curve is not a metrology certificate. The certificate is source
evidence that may justify the curve and provide a source reference/checksum.
The curve is the controlled, machine-readable correction artifact used by later
templates, recipes, or execution packages.

## RF Concepts

- Antenna factor converts between received voltage and electric-field strength;
  EMC workflows often express it as `dB_per_meter`.
- Cable loss represents frequency-dependent attenuation and is commonly stored
  as `dB` versus frequency.
- Amplifier gain represents frequency-dependent gain and is commonly stored as
  `dB` versus frequency.
- Current-probe transfer represents probe frequency response or correction,
  often expressed as a `dB` correction or transfer quantity depending on method.

## CSV Curve Format

For 1D frequency curves, LAB CONSOLE supports CSV paste/import/export such as:

```text
frequency_hz,correction_db
10000000,0.2
100000000,1.2
1000000000,2.2
```

Other supported value headers include method-specific names such as
`antenna_factor_db_per_m`, `cable_loss_db`, or `gain_db`. Units are carried in
definition metadata and dependent-value definitions, not repeated in each cell.

## Evaluation

The local agent can deterministically evaluate 1D curves for draft or approved
revisions. Supported interpolation modes are `linear_x_linear_y`,
`log_x_linear_y`, `linear_x_log_y`, `nearest`, `step_previous`, and
`step_next`. Extrapolation policies are `forbidden`, `clamp`, `warn`, and
`allow`.

Validation rejects `log_x_linear_y` curves with non-positive x values and
`linear_x_log_y` curves with non-positive dependent values, so logarithmic
interpolation inputs are controlled before a revision can be submitted.

Evaluation returns the computed values, axis values, interpolation mode,
whether extrapolation occurred, optional warning, source revision id, and source
checksum. It does not acquire live spectrum, time-domain, or DAQ data.
