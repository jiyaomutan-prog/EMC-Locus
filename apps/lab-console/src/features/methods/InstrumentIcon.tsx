import spriteUrl from "./instrument-icons.svg";

const iconByRole: Record<string, string> = {
  eut: "eut",
  generator: "generator",
  disturbance_generator: "generator",
  amplifier: "amplifier",
  receiver: "receiver",
  measurement_receiver: "receiver",
  spectrum_analyzer: "receiver",
  oscilloscope: "oscilloscope",
  daq: "daq",
  power_meter: "power-meter",
  wattmeter: "power-meter",
  voltage_probe: "voltage-probe",
  current_probe: "current-probe",
  field_probe: "field-probe",
  antenna: "antenna",
  lisn: "lisn",
  cdn: "cdn",
  coupler: "coupler",
  attenuator: "attenuator",
  filter: "filter",
  rf_switch: "rf-switch",
  cable: "cable",
  controller: "controller",
  computer: "controller",
  trigger: "trigger",
  correction: "correction"
};

export function InstrumentIcon(props: { roleType: string; label: string; size?: number }) {
  const symbol = iconByRole[props.roleType] ?? "generic";
  const size = props.size ?? 32;
  return (
    <svg
      className="instrumentIcon"
      role="img"
      aria-label={props.label}
      width={size}
      height={size}
      viewBox="0 0 32 32"
    >
      <title>{props.label}</title>
      <use href={`${spriteUrl}#${symbol}`} />
    </svg>
  );
}
