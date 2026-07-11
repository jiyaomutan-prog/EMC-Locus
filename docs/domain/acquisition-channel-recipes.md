# Acquisition Channel Recipes

Release `0.13.0` introduces `AcquisitionChannelRecipeDefinition` as a reusable
logical channel setup. It describes how a physical quantity should be produced
from an approved DAQ channel profile, optional sensor definition, optional
scaling profile, and optional correction curves.

An acquisition recipe is not a campaign execution package. It is not bound to a
specific serialized DAQ card, station connector, campaign, operator, or runtime
dataset. A future execution package will freeze approved definitions, physical
assets, station connections, environmental conditions, and run evidence.

## Example Chain

```text
DAQ analog input +/-10 V
-> current-probe electrical voltage
-> 10 mV/A scaling profile
-> current-probe transfer correction
-> engineering output current_A [A]
```

## Core Fields

- recipe id, label, output channel name, output quantity/unit;
- references to DAQ channel profile, sensor definition, scaling profile, and
  correction curves;
- sample rate, range, coupling, input mode, excitation;
- filtering, triggering, validation rules, and metadata.

## Validation Boundary

Validation checks identifier shape, approved references, DAQ range compatibility
with sensor electrical output where modeled, scaling output quantity/unit,
sample-rate limit, sensor excitation versus DAQ capabilities or external
marking, correction-stage coherence, and ADC/CAN bus ambiguity.

It does not reserve physical resources, start acquisition, stream samples,
perform FFT, or generate reports.
