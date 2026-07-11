PRAGMA foreign_keys = ON;

INSERT OR IGNORE INTO equipment_class_registry(class_code, label, driver_profile_allowed)
VALUES
    ('acquisition_device', 'Acquisition device', 1),
    ('converter', 'Converter', 1);

INSERT OR IGNORE INTO equipment_unit_registry(unit_code, quantity_code, scale_to_base, logarithmic)
VALUES
    ('Pa', 'pressure', 1.0, 0);

CREATE TABLE equipment_functional_role_registry (
    role_code TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    recommended_equipment_classes TEXT NOT NULL DEFAULT '[]',
    deprecated INTEGER NOT NULL DEFAULT 0 CHECK (deprecated IN (0, 1))
);

CREATE TABLE equipment_signal_domain_registry (
    domain_code TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    recommended_functional_roles TEXT NOT NULL DEFAULT '[]',
    deprecated INTEGER NOT NULL DEFAULT 0 CHECK (deprecated IN (0, 1))
);

CREATE TABLE equipment_port_directionality_registry (
    directionality_code TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    deprecated INTEGER NOT NULL DEFAULT 0 CHECK (deprecated IN (0, 1))
);

CREATE TABLE equipment_flow_role_registry (
    flow_role_code TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    compatible_directionalities TEXT NOT NULL DEFAULT '[]',
    compatible_signal_domains TEXT NOT NULL DEFAULT '[]',
    deprecated INTEGER NOT NULL DEFAULT 0 CHECK (deprecated IN (0, 1))
);

CREATE TABLE equipment_technology_tag_registry (
    tag_code TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    compatible_signal_domains TEXT NOT NULL DEFAULT '[]',
    recommended_functional_roles TEXT NOT NULL DEFAULT '[]',
    deprecated INTEGER NOT NULL DEFAULT 0 CHECK (deprecated IN (0, 1))
);

CREATE TABLE equipment_classification_presets (
    preset_id TEXT PRIMARY KEY,
    category_label TEXT NOT NULL,
    function_description TEXT NOT NULL,
    example_label TEXT NOT NULL,
    default_equipment_class TEXT NOT NULL REFERENCES equipment_class_registry(class_code),
    default_functional_role TEXT NOT NULL REFERENCES equipment_functional_role_registry(role_code),
    default_signal_domains TEXT NOT NULL,
    default_technology_tags TEXT NOT NULL,
    notes TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    deprecated INTEGER NOT NULL DEFAULT 0 CHECK (deprecated IN (0, 1))
);

CREATE TABLE equipment_classification_preset_ports (
    preset_id TEXT NOT NULL REFERENCES equipment_classification_presets(preset_id) ON DELETE CASCADE,
    port_order INTEGER NOT NULL,
    port_id TEXT NOT NULL,
    label TEXT NOT NULL,
    directionality TEXT NOT NULL REFERENCES equipment_port_directionality_registry(directionality_code),
    flow_role TEXT NOT NULL REFERENCES equipment_flow_role_registry(flow_role_code),
    signal_domain TEXT NOT NULL REFERENCES equipment_signal_domain_registry(domain_code),
    connector_type TEXT,
    technology_tags TEXT NOT NULL DEFAULT '[]',
    quantity TEXT NOT NULL DEFAULT 'dimensionless',
    unit TEXT NOT NULL DEFAULT 'dimensionless',
    impedance REAL,
    frequency_min REAL,
    frequency_max REAL,
    voltage_max REAL,
    current_max REAL,
    power_max REAL,
    required INTEGER NOT NULL DEFAULT 1 CHECK (required IN (0, 1)),
    comment TEXT,
    PRIMARY KEY(preset_id, port_order),
    UNIQUE(preset_id, port_id)
);

CREATE TABLE equipment_model_classification_summaries (
    equipment_model_id TEXT PRIMARY KEY REFERENCES equipment_model_identities(equipment_model_id) ON DELETE CASCADE,
    revision_id TEXT NOT NULL REFERENCES equipment_model_revisions(revision_id) ON DELETE CASCADE,
    revision_number INTEGER NOT NULL CHECK (revision_number > 0),
    status TEXT NOT NULL,
    manufacturer TEXT NOT NULL,
    equipment_class TEXT NOT NULL,
    category_code TEXT NOT NULL,
    functional_role TEXT NOT NULL REFERENCES equipment_functional_role_registry(role_code),
    definition_checksum TEXT NOT NULL,
    signal_domains_json TEXT NOT NULL,
    technology_tags_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE equipment_model_signal_domain_summaries (
    equipment_model_id TEXT NOT NULL REFERENCES equipment_model_classification_summaries(equipment_model_id) ON DELETE CASCADE,
    signal_domain TEXT NOT NULL REFERENCES equipment_signal_domain_registry(domain_code),
    revision_id TEXT NOT NULL,
    PRIMARY KEY(equipment_model_id, signal_domain)
);

CREATE TABLE equipment_model_technology_tag_summaries (
    equipment_model_id TEXT NOT NULL REFERENCES equipment_model_classification_summaries(equipment_model_id) ON DELETE CASCADE,
    technology_tag TEXT NOT NULL REFERENCES equipment_technology_tag_registry(tag_code),
    revision_id TEXT NOT NULL,
    PRIMARY KEY(equipment_model_id, technology_tag)
);

CREATE INDEX equipment_model_classification_role_idx
    ON equipment_model_classification_summaries(functional_role, status, manufacturer);

CREATE INDEX equipment_model_classification_class_idx
    ON equipment_model_classification_summaries(equipment_class, status, manufacturer);

CREATE INDEX equipment_model_classification_manufacturer_idx
    ON equipment_model_classification_summaries(manufacturer, status);

CREATE INDEX equipment_model_signal_domain_filter_idx
    ON equipment_model_signal_domain_summaries(signal_domain, equipment_model_id);

CREATE INDEX equipment_model_technology_tag_filter_idx
    ON equipment_model_technology_tag_summaries(technology_tag, equipment_model_id);

INSERT INTO equipment_functional_role_registry(role_code, label, description, recommended_equipment_classes)
VALUES
    ('energy_source', 'Energy source', 'Provides electrical, RF, thermal or other energy to a test setup.', '["controllable_instrument","manual_equipment"]'),
    ('signal_source', 'Signal source', 'Generates a controlled electrical, RF, pulse or transient signal.', '["controllable_instrument","converter"]'),
    ('rf_network_element', 'RF network element', 'Passes, adapts, attenuates, combines, splits, isolates or terminates RF energy.', '["passive_component","controllable_instrument","manual_equipment"]'),
    ('sensor', 'Sensor', 'Converts a physical field or quantity into a measurable output.', '["sensor","transducer","manual_equipment"]'),
    ('actuator', 'Actuator', 'Acts on the physical setup, energizes an output, moves a mechanism or emits a field.', '["controllable_instrument","switching_device","motion_system"]'),
    ('measurement_instrument', 'Measurement instrument', 'Measures a physical or electrical quantity and exposes a reading.', '["controllable_instrument","daq_device","acquisition_device"]'),
    ('acquisition_device', 'Acquisition device', 'Acquires one or more measured channels for later processing.', '["daq_device","acquisition_device"]'),
    ('converter', 'Converter', 'Converts between analog, digital, RF, optical or other signal forms.', '["converter","daq_device"]'),
    ('control_system', 'Control system', 'Coordinates equipment, control loops, PLC logic or field buses.', '["controllable_instrument","software_adapter"]'),
    ('software_system', 'Software system', 'Software-only controller, acquisition or analysis component.', '["software_adapter"]'),
    ('facility', 'Facility', 'Laboratory infrastructure used by a setup.', '["facility"]'),
    ('manual_accessory', 'Manual accessory', 'Manual accessory or fixture requiring operator handling.', '["manual_equipment","passive_component"]');

INSERT INTO equipment_signal_domain_registry(domain_code, label, description, recommended_functional_roles)
VALUES
    ('power_dc', 'DC power', 'Direct-current power or energy path.', '["energy_source","control_system"]'),
    ('power_ac', 'AC power', 'Alternating-current power or mains-like energy path.', '["energy_source"]'),
    ('rf', 'RF', 'Radio-frequency conducted or radiated signal path.', '["signal_source","rf_network_element","sensor","actuator","measurement_instrument"]'),
    ('analog_voltage', 'Analog voltage', 'Analog voltage signal.', '["sensor","measurement_instrument","acquisition_device","converter"]'),
    ('analog_current', 'Analog current', 'Analog current signal.', '["sensor","measurement_instrument","acquisition_device","converter"]'),
    ('analog_charge', 'Analog charge', 'Charge-mode sensor signal.', '["sensor","acquisition_device","converter"]'),
    ('digital_logic', 'Digital logic', 'Digital logic or discrete data signal.', '["converter","acquisition_device","control_system"]'),
    ('trigger', 'Trigger', 'Trigger, sync or timing signal.', '["measurement_instrument","acquisition_device","control_system"]'),
    ('pulse', 'Pulse', 'Pulse or transient stimulus signal.', '["signal_source","measurement_instrument"]'),
    ('contact_dry', 'Dry contact', 'Dry contact signal.', '["sensor","actuator","control_system"]'),
    ('relay', 'Relay', 'Relay contact or switched path.', '["actuator","control_system"]'),
    ('can_bus', 'CAN bus', 'Controller Area Network communication bus.', '["control_system","software_system"]'),
    ('rs232', 'RS-232', 'RS-232 communication signal.', '["control_system","software_system"]'),
    ('rs485', 'RS-485', 'RS-485 communication signal.', '["control_system","software_system"]'),
    ('ethernet', 'Ethernet', 'Ethernet communication signal.', '["control_system","software_system","measurement_instrument"]'),
    ('usb', 'USB', 'USB communication signal.', '["control_system","software_system","measurement_instrument"]'),
    ('gpib', 'GPIB', 'IEEE-488/GPIB communication signal.', '["measurement_instrument","control_system"]'),
    ('optical', 'Optical', 'Optical signal or optical sensor path.', '["sensor","converter"]'),
    ('mechanical', 'Mechanical', 'Mechanical motion, shock, vibration or force path.', '["sensor","actuator"]'),
    ('environmental', 'Environmental', 'Temperature, humidity, acoustic, field or ambient physical domain.', '["sensor","facility"]'),
    ('software', 'Software', 'Software-only data/control domain.', '["software_system"]');

INSERT INTO equipment_port_directionality_registry(directionality_code, label, description)
VALUES
    ('input', 'Input', 'Receives energy, signal or measured quantity.'),
    ('output', 'Output', 'Provides energy, signal or measured quantity.'),
    ('bidirectional', 'Bidirectional', 'Can both receive and provide signal.'),
    ('through', 'Through', 'One side of a pass-through path.'),
    ('control', 'Control', 'Control path rather than measured signal path.'),
    ('communication', 'Communication', 'Communication-only path.');

INSERT INTO equipment_flow_role_registry(flow_role_code, label, description, compatible_directionalities, compatible_signal_domains)
VALUES
    ('source_port', 'Source port', 'Source of energy or signal.', '["output","bidirectional"]', '[]'),
    ('sink_port', 'Sink port', 'Terminating or receiving sink.', '["input","bidirectional"]', '[]'),
    ('through_port', 'Through port', 'One side of a through-path network element.', '["through","bidirectional"]', '["rf","analog_voltage","analog_current","power_dc","power_ac"]'),
    ('measurement_port', 'Measurement port', 'Input used to measure a physical/electrical quantity.', '["input","bidirectional"]', '[]'),
    ('control_port', 'Control port', 'Control or timing path.', '["control","input","output","bidirectional"]', '["trigger","pulse","digital_logic","contact_dry","relay"]'),
    ('communication_port', 'Communication port', 'Communication-only port.', '["communication","bidirectional"]', '["can_bus","rs232","rs485","ethernet","usb","gpib","software"]'),
    ('field_side_port', 'Field-side port', 'Physical field side of a transducer or antenna.', '["input","output","bidirectional"]', '["rf","mechanical","environmental","optical"]'),
    ('transducer_output_port', 'Transducer output port', 'Electrical/RF/digital output of a sensor/transducer.', '["output","bidirectional"]', '[]');

INSERT INTO equipment_technology_tag_registry(tag_code, label, description, compatible_signal_domains, recommended_functional_roles)
VALUES
    ('adc_converter', 'ADC converter', 'Analog-to-digital converter.', '["analog_voltage","analog_current","digital_logic"]', '["converter","acquisition_device"]'),
    ('dac_converter', 'DAC converter', 'Digital-to-analog converter.', '["digital_logic","analog_voltage","analog_current"]', '["converter","signal_source"]'),
    ('rf_50_ohm', 'RF 50 ohm', '50 ohm RF path.', '["rf"]', '["rf_network_element","signal_source","measurement_instrument","sensor","actuator"]'),
    ('rf_75_ohm', 'RF 75 ohm', '75 ohm RF path.', '["rf"]', '["rf_network_element","signal_source","measurement_instrument","sensor","actuator"]'),
    ('ttl', 'TTL', 'TTL digital logic.', '["digital_logic","trigger"]', '["control_system","acquisition_device"]'),
    ('cmos', 'CMOS', 'CMOS digital logic.', '["digital_logic","trigger"]', '["control_system","acquisition_device"]'),
    ('trigger', 'Trigger', 'Trigger or sync path.', '["trigger","pulse"]', '["measurement_instrument","acquisition_device"]'),
    ('dry_contact', 'Dry contact', 'Dry contact input or output.', '["contact_dry"]', '["sensor","actuator","control_system"]'),
    ('relay_contact', 'Relay contact', 'Relay switched contact.', '["relay"]', '["actuator","control_system"]'),
    ('voltage_input', 'Voltage input', 'Analog voltage input.', '["analog_voltage"]', '["measurement_instrument","acquisition_device","converter"]'),
    ('current_input', 'Current input', 'Analog current input.', '["analog_current"]', '["measurement_instrument","acquisition_device","converter"]'),
    ('charge_input', 'Charge input', 'Charge-mode input.', '["analog_charge"]', '["measurement_instrument","acquisition_device","converter"]'),
    ('iepe', 'IEPE', 'IEPE sensor interface.', '["analog_voltage","analog_current"]', '["sensor","acquisition_device"]'),
    ('bridge', 'Bridge', 'Bridge sensor interface.', '["analog_voltage"]', '["sensor","acquisition_device"]'),
    ('usb', 'USB', 'USB communication.', '["usb"]', '["measurement_instrument","software_system","control_system"]'),
    ('ethernet', 'Ethernet', 'Ethernet communication.', '["ethernet"]', '["measurement_instrument","software_system","control_system"]'),
    ('gpib', 'GPIB', 'GPIB communication.', '["gpib"]', '["measurement_instrument","control_system"]'),
    ('rs232', 'RS-232', 'RS-232 communication.', '["rs232"]', '["control_system"]'),
    ('rs485', 'RS-485', 'RS-485 communication.', '["rs485"]', '["control_system"]'),
    ('can_bus', 'CAN bus', 'Controller Area Network communication.', '["can_bus"]', '["control_system","software_system"]'),
    ('visa', 'VISA', 'VISA access layer.', '["usb","ethernet","gpib","rs232"]', '["measurement_instrument","control_system"]'),
    ('raw_tcp', 'Raw TCP', 'Raw TCP socket protocol.', '["ethernet"]', '["measurement_instrument","control_system"]'),
    ('serial_text', 'Serial text', 'Text protocol over serial line.', '["rs232","rs485"]', '["control_system","measurement_instrument"]'),
    ('scpi', 'SCPI', 'SCPI command protocol.', '["ethernet","usb","gpib","rs232"]', '["measurement_instrument","signal_source"]');

INSERT INTO equipment_classification_presets(
    preset_id, category_label, function_description, example_label,
    default_equipment_class, default_functional_role, default_signal_domains,
    default_technology_tags, notes, sort_order
)
VALUES
    ('dc_power_supply', 'Energy sources', 'Provides controlled DC power.', 'DC power supply', 'controllable_instrument', 'energy_source', '["power_dc","ethernet","usb"]', '["voltage_input","ethernet","usb","scpi"]', '', 10),
    ('ac_power_source', 'Energy sources', 'Provides controlled AC power.', 'AC power source', 'controllable_instrument', 'energy_source', '["power_ac","ethernet"]', '["ethernet","scpi"]', '', 20),
    ('battery', 'Energy sources', 'Portable DC energy source.', 'Battery', 'manual_equipment', 'energy_source', '["power_dc"]', '[]', '', 30),
    ('rf_generator', 'Signal sources', 'Generates RF signal.', 'RF generator', 'controllable_instrument', 'signal_source', '["rf","ethernet","usb","gpib"]', '["rf_50_ohm","ethernet","usb","gpib","scpi"]', '', 100),
    ('vector_signal_generator', 'Signal sources', 'Generates modulated RF signal.', 'Vector signal generator', 'controllable_instrument', 'signal_source', '["rf","ethernet","usb"]', '["rf_50_ohm","ethernet","usb","scpi"]', '', 110),
    ('awg', 'Signal sources', 'Generates arbitrary analog waveforms.', 'AWG', 'controllable_instrument', 'signal_source', '["analog_voltage","trigger","ethernet","usb"]', '["voltage_input","trigger","ethernet","usb","scpi"]', '', 120),
    ('pulse_generator', 'Signal sources', 'Generates pulse signals.', 'Pulse generator', 'controllable_instrument', 'signal_source', '["pulse","trigger","ethernet"]', '["trigger","ethernet","scpi"]', '', 130),
    ('esd_generator', 'Signal sources', 'Generates ESD immunity pulses.', 'ESD generator', 'controllable_instrument', 'signal_source', '["pulse","contact_dry"]', '["trigger"]', '', 140),
    ('eft_burst_generator', 'Signal sources', 'Generates EFT/burst immunity transients.', 'EFT/burst generator', 'controllable_instrument', 'signal_source', '["pulse","power_ac"]', '["trigger"]', '', 150),
    ('surge_generator', 'Signal sources', 'Generates surge immunity transients.', 'Surge generator', 'controllable_instrument', 'signal_source', '["pulse","power_ac"]', '["trigger"]', '', 160),
    ('rf_cable', 'RF equipment', 'Passes RF signal between two ports.', 'RF cable', 'passive_component', 'rf_network_element', '["rf"]', '["rf_50_ohm"]', '', 200),
    ('rf_amplifier', 'RF equipment', 'Amplifies RF signal.', 'RF amplifier', 'controllable_instrument', 'actuator', '["rf","rs232"]', '["rf_50_ohm","rs232","serial_text"]', '', 210),
    ('attenuator', 'RF equipment', 'Attenuates RF signal.', 'Attenuator', 'passive_component', 'rf_network_element', '["rf"]', '["rf_50_ohm"]', '', 220),
    ('filter', 'RF equipment', 'Filters RF signal.', 'Filter', 'passive_component', 'rf_network_element', '["rf"]', '["rf_50_ohm"]', '', 230),
    ('coupler', 'RF equipment', 'Samples or couples RF signal.', 'Coupler', 'passive_component', 'rf_network_element', '["rf"]', '["rf_50_ohm"]', '', 240),
    ('combiner', 'RF equipment', 'Combines RF paths.', 'Combiner', 'passive_component', 'rf_network_element', '["rf"]', '["rf_50_ohm"]', '', 250),
    ('divider', 'RF equipment', 'Divides RF paths.', 'Divider', 'passive_component', 'rf_network_element', '["rf"]', '["rf_50_ohm"]', '', 260),
    ('isolator', 'RF equipment', 'Provides directional isolation.', 'Isolator', 'passive_component', 'rf_network_element', '["rf"]', '["rf_50_ohm"]', '', 270),
    ('circulator', 'RF equipment', 'Routes RF signal between ports.', 'Circulator', 'passive_component', 'rf_network_element', '["rf"]', '["rf_50_ohm"]', '', 280),
    ('impedance_adapter', 'RF equipment', 'Adapts RF impedance.', 'Impedance adapter', 'passive_component', 'rf_network_element', '["rf"]', '["rf_50_ohm"]', '', 290),
    ('rf_load', 'RF equipment', 'Terminates RF signal.', 'RF load', 'passive_component', 'rf_network_element', '["rf"]', '["rf_50_ohm"]', '', 300),
    ('receiving_antenna', 'Sensors', 'Converts RF field to RF output.', 'Receiving antenna', 'sensor', 'sensor', '["rf","environmental"]', '["rf_50_ohm"]', '', 400),
    ('transmitting_antenna', 'Actuators', 'Converts RF input to radiated field.', 'Transmitting antenna', 'manual_equipment', 'actuator', '["rf","environmental"]', '["rf_50_ohm"]', '', 410),
    ('field_probe', 'Sensors', 'Measures electric or magnetic field.', 'Field probe', 'sensor', 'sensor', '["environmental","analog_voltage"]', '["voltage_input"]', '', 420),
    ('current_probe', 'Sensors', 'Converts current to voltage/RF output.', 'Current probe', 'sensor', 'sensor', '["analog_current","analog_voltage"]', '["current_input","voltage_input"]', '', 430),
    ('temperature_sensor', 'Sensors', 'Measures temperature.', 'Temperature sensor', 'sensor', 'sensor', '["environmental","analog_voltage"]', '["voltage_input"]', '', 440),
    ('accelerometer', 'Sensors', 'Measures acceleration or vibration.', 'Accelerometer', 'sensor', 'sensor', '["mechanical","analog_voltage"]', '["iepe","voltage_input"]', '', 450),
    ('photodiode', 'Sensors', 'Converts light to current or voltage.', 'Photodiode', 'sensor', 'sensor', '["optical","analog_current"]', '["current_input"]', '', 460),
    ('microphone', 'Sensors', 'Converts acoustic pressure to voltage.', 'Microphone', 'sensor', 'sensor', '["environmental","analog_voltage"]', '["voltage_input"]', '', 470),
    ('motor', 'Actuators', 'Converts electrical power to motion.', 'Motor', 'motion_system', 'actuator', '["power_dc","mechanical"]', '[]', '', 500),
    ('relay', 'Actuators', 'Switches electrical contacts.', 'Relay', 'switching_device', 'actuator', '["relay","contact_dry"]', '["relay_contact","dry_contact"]', '', 510),
    ('valve', 'Actuators', 'Controls a fluid or pneumatic path.', 'Valve', 'motion_system', 'actuator', '["mechanical","power_dc"]', '[]', '', 520),
    ('heater', 'Actuators', 'Provides heat.', 'Heater', 'controllable_instrument', 'actuator', '["power_ac","environmental"]', '[]', '', 530),
    ('speaker', 'Actuators', 'Converts electrical signal to acoustic output.', 'Speaker', 'manual_equipment', 'actuator', '["analog_voltage","environmental"]', '[]', '', 540),
    ('spectrum_analyzer', 'Measurement instruments', 'Measures RF spectrum.', 'Spectrum analyzer', 'controllable_instrument', 'measurement_instrument', '["rf","ethernet","usb","gpib"]', '["rf_50_ohm","ethernet","usb","gpib","scpi"]', '', 600),
    ('emi_receiver', 'Measurement instruments', 'Measures EMI levels.', 'EMI receiver', 'controllable_instrument', 'measurement_instrument', '["rf","ethernet","usb","gpib"]', '["rf_50_ohm","ethernet","usb","gpib","scpi"]', '', 610),
    ('oscilloscope', 'Measurement instruments', 'Measures voltage versus time.', 'Oscilloscope', 'controllable_instrument', 'measurement_instrument', '["analog_voltage","trigger","ethernet","usb"]', '["voltage_input","trigger","ethernet","usb"]', '', 620),
    ('vna', 'Measurement instruments', 'Measures network parameters.', 'VNA', 'controllable_instrument', 'measurement_instrument', '["rf","ethernet","usb","gpib"]', '["rf_50_ohm","ethernet","usb","gpib","scpi"]', '', 630),
    ('rf_power_meter', 'Measurement instruments', 'Measures RF power.', 'RF power meter', 'controllable_instrument', 'measurement_instrument', '["rf","ethernet","usb"]', '["rf_50_ohm","ethernet","usb","scpi"]', '', 640),
    ('power_sensor', 'Measurement instruments', 'Senses RF power.', 'Power sensor', 'sensor', 'sensor', '["rf","analog_voltage","usb"]', '["rf_50_ohm","voltage_input","usb"]', '', 650),
    ('frequency_counter', 'Measurement instruments', 'Measures frequency.', 'Frequency counter', 'controllable_instrument', 'measurement_instrument', '["rf","analog_voltage","ethernet"]', '["rf_50_ohm","voltage_input","ethernet","scpi"]', '', 660),
    ('adc_converter', 'Converters and acquisition', 'Converts analog signal to digital data.', 'ADC converter', 'daq_device', 'converter', '["analog_voltage","digital_logic"]', '["adc_converter","voltage_input"]', '', 700),
    ('dac_converter', 'Converters and acquisition', 'Converts digital data to analog signal.', 'DAC converter', 'converter', 'converter', '["digital_logic","analog_voltage"]', '["dac_converter"]', '', 710),
    ('daq_card', 'Converters and acquisition', 'Acquires multiple analog/digital channels.', 'DAQ card', 'daq_device', 'acquisition_device', '["analog_voltage","analog_current","digital_logic","trigger","usb","ethernet"]', '["voltage_input","current_input","trigger","usb","ethernet"]', '', 720),
    ('plc', 'Processing and control systems', 'Industrial control system.', 'PLC', 'controllable_instrument', 'control_system', '["digital_logic","relay","rs485","ethernet"]', '["relay_contact","rs485","ethernet"]', '', 800),
    ('bench_controller', 'Processing and control systems', 'Controls bench instruments.', 'Bench controller', 'controllable_instrument', 'control_system', '["ethernet","usb","gpib","rs232"]', '["ethernet","usb","gpib","rs232","visa"]', '', 810),
    ('can_bus_controlled_unit', 'Processing and control systems', 'Controls power or functions through Controller Area Network.', 'CAN bus controlled unit', 'controllable_instrument', 'control_system', '["can_bus","power_dc"]', '["can_bus"]', '', 820),
    ('test_acquisition_software', 'Software systems', 'Software-only test and acquisition controller.', 'Test and acquisition software', 'software_adapter', 'software_system', '["software","ethernet","usb"]', '["ethernet","usb"]', '', 900);

INSERT INTO equipment_classification_preset_ports(
    preset_id, port_order, port_id, label, directionality, flow_role, signal_domain,
    connector_type, technology_tags, quantity, unit, impedance, frequency_min, frequency_max,
    voltage_max, current_max, power_max, required, comment
)
VALUES
    ('dc_power_supply', 1, 'DC_OUT', 'DC output', 'output', 'source_port', 'power_dc', 'terminal', '[]', 'voltage', 'V', NULL, NULL, NULL, 60.0, NULL, NULL, 1, NULL),
    ('ac_power_source', 1, 'AC_OUT', 'AC output', 'output', 'source_port', 'power_ac', 'socket', '[]', 'voltage', 'V', NULL, NULL, NULL, 300.0, NULL, NULL, 1, NULL),
    ('battery', 1, 'DC_OUT', 'Battery output', 'output', 'source_port', 'power_dc', 'terminal', '[]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('rf_generator', 1, 'RF_OUT', 'RF output', 'output', 'source_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, 9000.0, 6000000000.0, NULL, NULL, NULL, 1, NULL),
    ('vector_signal_generator', 1, 'RF_OUT', 'RF output', 'output', 'source_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, 9000.0, 6000000000.0, NULL, NULL, NULL, 1, NULL),
    ('awg', 1, 'ANALOG_OUT', 'Analog output', 'output', 'source_port', 'analog_voltage', 'BNC', '[]', 'voltage', 'V', NULL, NULL, NULL, 10.0, NULL, NULL, 1, NULL),
    ('awg', 2, 'TRIG_OUT', 'Trigger output', 'output', 'control_port', 'trigger', 'BNC', '["trigger"]', 'dimensionless', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL),
    ('pulse_generator', 1, 'PULSE_OUT', 'Pulse output', 'output', 'source_port', 'pulse', 'BNC', '["trigger"]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('esd_generator', 1, 'DISCHARGE', 'Discharge output', 'output', 'source_port', 'pulse', NULL, '[]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('eft_burst_generator', 1, 'BURST_OUT', 'Burst output', 'output', 'source_port', 'pulse', 'coupling_network', '[]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('surge_generator', 1, 'SURGE_OUT', 'Surge output', 'output', 'source_port', 'pulse', 'coupling_network', '[]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('rf_cable', 1, 'RF_A', 'RF side A', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('rf_cable', 2, 'RF_B', 'RF side B', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('rf_amplifier', 1, 'RF_IN', 'RF input', 'input', 'sink_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('rf_amplifier', 2, 'RF_OUT', 'RF output', 'output', 'source_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'W', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('attenuator', 1, 'RF_A', 'RF side A', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('attenuator', 2, 'RF_B', 'RF side B', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('filter', 1, 'RF_IN', 'RF input', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('filter', 2, 'RF_OUT', 'RF output', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('coupler', 1, 'RF_IN', 'RF input', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('coupler', 2, 'RF_OUT', 'RF output', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('coupler', 3, 'COUPLED', 'Coupled output', 'output', 'source_port', 'rf', 'SMA', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 0, NULL),
    ('combiner', 1, 'RF_A', 'RF input A', 'input', 'sink_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('combiner', 2, 'RF_B', 'RF input B', 'input', 'sink_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('combiner', 3, 'RF_OUT', 'RF combined output', 'output', 'source_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('divider', 1, 'RF_IN', 'RF input', 'input', 'sink_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('divider', 2, 'RF_A', 'RF output A', 'output', 'source_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('divider', 3, 'RF_B', 'RF output B', 'output', 'source_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('isolator', 1, 'RF_IN', 'RF input', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('isolator', 2, 'RF_OUT', 'RF output', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('circulator', 1, 'PORT1', 'Port 1', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('circulator', 2, 'PORT2', 'Port 2', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('circulator', 3, 'PORT3', 'Port 3', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('impedance_adapter', 1, 'RF_A', 'RF side A', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('impedance_adapter', 2, 'RF_B', 'RF side B', 'through', 'through_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('rf_load', 1, 'RF_IN', 'RF input', 'input', 'sink_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'W', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('receiving_antenna', 1, 'FIELD', 'RF field input', 'input', 'field_side_port', 'environmental', NULL, '[]', 'electric_field', 'dBuV_per_m', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('receiving_antenna', 2, 'RF_OUT', 'RF output', 'output', 'transducer_output_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('transmitting_antenna', 1, 'RF_IN', 'RF input', 'input', 'sink_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('transmitting_antenna', 2, 'FIELD', 'RF field output', 'output', 'field_side_port', 'environmental', NULL, '[]', 'electric_field', 'dBuV_per_m', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('field_probe', 1, 'FIELD', 'Field input', 'input', 'field_side_port', 'environmental', NULL, '[]', 'electric_field', 'dBuV_per_m', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('field_probe', 2, 'V_OUT', 'Voltage output', 'output', 'transducer_output_port', 'analog_voltage', 'BNC', '["voltage_input"]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('current_probe', 1, 'CURRENT_PATH', 'Current path', 'input', 'field_side_port', 'analog_current', NULL, '["current_input"]', 'current', 'A', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('current_probe', 2, 'V_OUT', 'Voltage output', 'output', 'transducer_output_port', 'analog_voltage', 'BNC', '["voltage_input"]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('temperature_sensor', 1, 'TEMP_FIELD', 'Temperature input', 'input', 'field_side_port', 'environmental', NULL, '[]', 'temperature', 'Celsius', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('temperature_sensor', 2, 'V_OUT', 'Voltage output', 'output', 'transducer_output_port', 'analog_voltage', 'BNC', '["voltage_input"]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('accelerometer', 1, 'MECH_IN', 'Mechanical input', 'input', 'field_side_port', 'mechanical', NULL, '["iepe"]', 'dimensionless', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('accelerometer', 2, 'V_OUT', 'Voltage output', 'output', 'transducer_output_port', 'analog_voltage', 'BNC', '["voltage_input"]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('photodiode', 1, 'OPTICAL_IN', 'Optical input', 'input', 'field_side_port', 'optical', NULL, '[]', 'dimensionless', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('photodiode', 2, 'I_OUT', 'Current output', 'output', 'transducer_output_port', 'analog_current', 'BNC', '["current_input"]', 'current', 'A', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('microphone', 1, 'ACOUSTIC_IN', 'Acoustic input', 'input', 'field_side_port', 'environmental', NULL, '[]', 'pressure', 'Pa', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('microphone', 2, 'V_OUT', 'Voltage output', 'output', 'transducer_output_port', 'analog_voltage', 'BNC', '["voltage_input"]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('motor', 1, 'POWER_IN', 'Power input', 'input', 'sink_port', 'power_dc', 'terminal', '[]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('motor', 2, 'SHAFT', 'Mechanical output', 'output', 'source_port', 'mechanical', NULL, '[]', 'dimensionless', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('relay', 1, 'COIL', 'Coil control', 'control', 'control_port', 'relay', 'terminal', '["relay_contact"]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('relay', 2, 'CONTACT', 'Switched contact', 'bidirectional', 'through_port', 'contact_dry', 'terminal', '["dry_contact"]', 'dimensionless', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('relay', 3, 'CONTACT_RETURN', 'Switched contact return', 'bidirectional', 'through_port', 'contact_dry', 'terminal', '["dry_contact"]', 'dimensionless', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('valve', 1, 'CONTROL', 'Valve control', 'control', 'control_port', 'power_dc', 'terminal', '[]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('valve', 2, 'MECH_OUT', 'Mechanical output', 'output', 'source_port', 'mechanical', NULL, '[]', 'dimensionless', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('heater', 1, 'POWER_IN', 'Heater power input', 'input', 'sink_port', 'power_ac', 'terminal', '[]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('heater', 2, 'HEAT_FIELD', 'Heat output', 'output', 'field_side_port', 'environmental', NULL, '[]', 'temperature', 'Celsius', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('speaker', 1, 'AUDIO_IN', 'Audio input', 'input', 'sink_port', 'analog_voltage', 'terminal', '[]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('speaker', 2, 'ACOUSTIC_OUT', 'Acoustic output', 'output', 'field_side_port', 'environmental', NULL, '[]', 'dimensionless', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('spectrum_analyzer', 1, 'RF_IN', 'RF input', 'input', 'measurement_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, 9000.0, 6000000000.0, NULL, NULL, 30.0, 1, NULL),
    ('emi_receiver', 1, 'RF_IN', 'RF input', 'input', 'measurement_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, 9000.0, 6000000000.0, NULL, NULL, 30.0, 1, NULL),
    ('oscilloscope', 1, 'CH1', 'Channel 1', 'input', 'measurement_port', 'analog_voltage', 'BNC', '["voltage_input"]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('oscilloscope', 2, 'TRIG_IN', 'Trigger input', 'input', 'control_port', 'trigger', 'BNC', '["trigger"]', 'dimensionless', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL),
    ('oscilloscope', 3, 'LAN', 'LAN', 'communication', 'communication_port', 'ethernet', 'RJ45', '["ethernet"]', 'binary', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL),
    ('vna', 1, 'PORT1', 'RF port 1', 'bidirectional', 'measurement_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, 9000.0, 6000000000.0, NULL, NULL, NULL, 1, NULL),
    ('vna', 2, 'PORT2', 'RF port 2', 'bidirectional', 'measurement_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, 9000.0, 6000000000.0, NULL, NULL, NULL, 1, NULL),
    ('rf_power_meter', 1, 'RF_IN', 'RF input', 'input', 'measurement_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, 9000.0, 6000000000.0, NULL, NULL, 30.0, 1, NULL),
    ('power_sensor', 1, 'RF_IN', 'RF input', 'input', 'field_side_port', 'rf', 'N', '["rf_50_ohm"]', 'power', 'dBm', 50.0, 9000.0, 6000000000.0, NULL, NULL, 30.0, 1, NULL),
    ('power_sensor', 2, 'V_OUT', 'Voltage output', 'output', 'transducer_output_port', 'analog_voltage', 'BNC', '["voltage_input"]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('power_sensor', 3, 'USB', 'USB output', 'communication', 'communication_port', 'usb', 'USB', '["usb"]', 'binary', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL),
    ('frequency_counter', 1, 'SIGNAL_IN', 'Signal input', 'input', 'measurement_port', 'rf', 'BNC', '["rf_50_ohm"]', 'frequency', 'Hz', 50.0, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('adc_converter', 1, 'ANALOG_IN', 'Analog input', 'input', 'measurement_port', 'analog_voltage', 'BNC', '["adc_converter","voltage_input"]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, 'ADC converter analog input; no CAN bus implied.'),
    ('adc_converter', 2, 'DIGITAL_OUT', 'Digital output', 'output', 'transducer_output_port', 'digital_logic', NULL, '["adc_converter"]', 'binary', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('dac_converter', 1, 'DIGITAL_IN', 'Digital input', 'input', 'sink_port', 'digital_logic', NULL, '["dac_converter"]', 'binary', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('dac_converter', 2, 'ANALOG_OUT', 'Analog output', 'output', 'source_port', 'analog_voltage', 'BNC', '["dac_converter"]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, 'DAC converter analog output; no CAN bus implied.'),
    ('daq_card', 1, 'AI0', 'Analog input 0', 'input', 'measurement_port', 'analog_voltage', 'terminal', '["voltage_input"]', 'voltage', 'V', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('daq_card', 2, 'TRIG_IN', 'Trigger input', 'input', 'control_port', 'trigger', 'terminal', '["trigger"]', 'dimensionless', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL),
    ('daq_card', 3, 'USB', 'USB communication', 'communication', 'communication_port', 'usb', 'USB', '["usb"]', 'binary', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('plc', 1, 'DIGITAL_IO', 'Digital I/O', 'bidirectional', 'control_port', 'digital_logic', 'terminal', '["ttl"]', 'binary', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('plc', 2, 'ETHERNET', 'Ethernet', 'communication', 'communication_port', 'ethernet', 'RJ45', '["ethernet"]', 'binary', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('bench_controller', 1, 'LAN', 'LAN', 'communication', 'communication_port', 'ethernet', 'RJ45', '["ethernet","raw_tcp"]', 'binary', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('bench_controller', 2, 'USB', 'USB', 'communication', 'communication_port', 'usb', 'USB', '["usb"]', 'binary', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL),
    ('can_bus_controlled_unit', 1, 'CAN_BUS', 'CAN bus', 'communication', 'communication_port', 'can_bus', 'D-Sub', '["can_bus"]', 'binary', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, 'Controller Area Network communication port.'),
    ('can_bus_controlled_unit', 2, 'POWER_OUT', 'Power output', 'output', 'source_port', 'power_dc', 'terminal', '[]', 'voltage', 'V', NULL, NULL, NULL, 60.0, NULL, NULL, 1, NULL),
    ('test_acquisition_software', 1, 'SOFTWARE_API', 'Software API', 'communication', 'communication_port', 'software', NULL, '[]', 'binary', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 1, NULL),
    ('test_acquisition_software', 2, 'LAN', 'LAN', 'communication', 'communication_port', 'ethernet', 'RJ45', '["ethernet"]', 'binary', 'dimensionless', NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL);

UPDATE repository_metadata
SET value = '2026-07-11-v2', updated_at = '2026-07-11T00:00:00Z'
WHERE key = 'equipment_catalog_schema';

UPDATE repository_metadata
SET value = '0.12.0', updated_at = '2026-07-11T00:00:00Z'
WHERE key = 'equipment_catalog_release';

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (2, 'physical_classification', '2026-07-11T00:00:00Z');
