# Communication Adapters

Equipment communication is modeled with three independent layers:

- physical transport: `serial`, `gpib`, `ethernet_tcp`, `ethernet_udp`, `can_bus`,
  `usb`, `none`;
- access provider: `native_serial`, `native_tcp`, `native_udp`, `visa`,
  `socketcan`, `pcan`, `vector_can`, `usbtmc`, `hid`, `custom_adapter`,
  `simulation`;
- application protocol: `scpi`, `raw_ascii`, `raw_binary`, `can_bus_frames`,
  `modbus_rtu`, `modbus_tcp`, `custom_protocol`, `manual`.

VISA is an access layer, not a physical transport. SCPI may run through VISA,
native TCP or serial when the model declares that combination.

## Provider Status

The local agent exposes:

```text
GET /api/v1/equipment/communication-providers
```

`0.12.0` continues to report simulation, native TCP, native UDP and native
serial as available in the local runtime contract. CI does not require a
physical COM port. VISA, vendor CAN bus, USBTMC and HID are modeled and
simulated but reported as unavailable when no provider is installed.

CAN bus wording is deliberately explicit. A model using
`protocol_kind=can_bus_frames` must also expose `transport_kind=can_bus`, a
`can_bus` signal domain, and a CAN bus communication port. ADC and DAC
converters are not inferred as CAN bus devices.

## Safety Position

No release code executes arbitrary DLLs, shell commands, Python snippets or
unregistered plugins from a driver script. Future external adapters must be
registered with identity, version, schema, safety class and checksum before use.
