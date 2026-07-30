const categoryLabels: Record<string, string> = {
  adc_converter: "Convertisseurs A/N",
  antenna: "Antennes de réception",
  can_bus_power_unit: "Alimentations de bus CAN",
  daq_card: "Cartes d'acquisition",
  dc_power_supply: "Alimentations continues",
  oscilloscope: "Oscilloscopes",
  power_meter: "Wattmètres RF",
  receiving_antenna: "Antennes de réception",
  rf_amplifier: "Amplificateurs RF",
  rf_cable: "Câbles RF",
  rf_generator: "Générateurs RF",
  rf_load: "Charges RF",
  rf_power_meter: "Wattmètres RF",
  rf_signal_generator: "Générateurs RF",
  test_acquisition_software: "Logiciels d'acquisition d'essai",
  transmitting_antenna: "Antennes d'émission"
};

const demoModelNames: Record<string, string> = {
  adc_converter: "Convertisseur A/N",
  antenna: "Antenne de réception",
  daq_card: "Carte d'acquisition",
  dc_power_supply: "Alimentation continue",
  receiving_antenna: "Antenne de réception",
  rf_cable: "Câble RF",
  rf_generator: "Générateur RF",
  rf_load: "Charge RF",
  test_acquisition_software: "Logiciel d'acquisition d'essai",
  transmitting_antenna: "Antenne d'émission"
};

const demoVariants: Record<string, string> = {
  "16 bit": "16 bits",
  "4CH": "4 voies",
  "6GHz": "6 GHz",
  "50 ohm": "50 ohms",
  "60V": "60 V",
  broadband: "large bande",
  local: "locale"
};

const requirementLabels: Record<string, string> = {
  adc_converter: "un convertisseur A/N",
  daq_card: "une carte d'acquisition",
  power_meter: "un wattmètre RF",
  receiving_antenna: "une antenne de réception",
  rf_generator: "un générateur RF",
  rf_power_meter: "un wattmètre RF",
  rf_signal_generator: "un générateur RF",
  transmitting_antenna: "une antenne d'émission"
};

export function operatorCategoryLabel(categoryCode: string, fallback: string) {
  return categoryLabels[categoryCode] ?? fallback;
}

export function operatorCategoryPath(categoryCode: string, path: string[]) {
  if (path.length === 0) return [operatorCategoryLabel(categoryCode, categoryCode)];
  return path.map((segment, index) =>
    index === path.length - 1 ? operatorCategoryLabel(categoryCode, segment) : segment
  );
}

export function operatorModelName(
  categoryCode: string,
  modelName: string,
  isDemo: boolean
) {
  return isDemo ? demoModelNames[categoryCode] ?? modelName : modelName;
}

export function operatorModelVariant(variant: string | null | undefined, isDemo: boolean) {
  if (!variant) return "";
  return isDemo ? demoVariants[variant] ?? variant : variant;
}

export function operatorRequirementLabel(value: string) {
  return requirementLabels[value] ?? value.replaceAll("_", " ");
}
