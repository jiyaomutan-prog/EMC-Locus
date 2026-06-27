PRAGMA foreign_keys = ON;

CREATE TABLE instrument_categories (
    code TEXT PRIMARY KEY,
    domain TEXT NOT NULL CHECK (
        domain IN (
            'electronics',
            'emc',
            'thermal',
            'acoustic',
            'shock_vibration',
            'radio_rf',
            'data_monitoring'
        )
    ),
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    typical_instruments_json TEXT NOT NULL DEFAULT '[]',
    measured_quantities_json TEXT NOT NULL DEFAULT '[]',
    typical_transports_json TEXT NOT NULL DEFAULT '[]',
    calibration_profile TEXT NOT NULL CHECK (
        calibration_profile IN (
            'electrical',
            'rf',
            'thermal',
            'acoustic',
            'mechanical',
            'data_acquisition',
            'environmental'
        )
    ),
    default_calibration_requirement TEXT NOT NULL CHECK (
        default_calibration_requirement IN ('required', 'conditional', 'not_required')
    ),
    taxonomy_revision TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX instrument_categories_domain_idx
    ON instrument_categories(domain, label);

CREATE TABLE instrument_category_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category_code TEXT NOT NULL REFERENCES instrument_categories(code),
    source_name TEXT NOT NULL,
    source_url TEXT NOT NULL,
    source_note TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    UNIQUE(category_code, source_url)
);

CREATE INDEX instrument_category_sources_category_idx
    ON instrument_category_sources(category_code);

ALTER TABLE instruments
ADD COLUMN category_code TEXT REFERENCES instrument_categories(code);

INSERT INTO instrument_categories (
    code,
    domain,
    label,
    description,
    typical_instruments_json,
    measured_quantities_json,
    typical_transports_json,
    calibration_profile,
    default_calibration_requirement,
    taxonomy_revision,
    created_at,
    updated_at
)
VALUES
    (
        'oscilloscope',
        'electronics',
        'Oscilloscope',
        'Time-domain electrical waveform instrument used for voltage, timing, transient, and protocol-adjacent checks.',
        '["digital oscilloscope","mixed signal oscilloscope","high-voltage probe"]',
        '["voltage","time","frequency","rise_time","pulse_width"]',
        '["visa","scpi","usb","ethernet","gpib"]',
        'electrical',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'digital_multimeter',
        'electronics',
        'Digital multimeter',
        'General-purpose electrical measurement category for voltage, current, resistance, continuity, and frequency checks.',
        '["bench multimeter","handheld multimeter","system DMM"]',
        '["voltage","current","resistance","frequency","continuity"]',
        '["manual","visa","scpi","usb","ethernet"]',
        'electrical',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'lcr_impedance_meter',
        'electronics',
        'LCR and impedance meter',
        'Impedance measurement category for passive components, LISN checks, fixtures, and bench characterization.',
        '["LCR meter","impedance analyzer","precision bridge"]',
        '["capacitance","inductance","resistance","impedance","phase"]',
        '["visa","scpi","usb","ethernet","gpib"]',
        'electrical',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'dc_power_supply',
        'electronics',
        'DC power supply',
        'Programmable or manual source used to bias equipment under test and exercise supply-dependent EMC behavior.',
        '["bench supply","programmable DC supply","source module"]',
        '["voltage","current","power","limit_state"]',
        '["manual","visa","scpi","usb","ethernet","serial"]',
        'electrical',
        'conditional',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'electronic_load',
        'electronics',
        'Electronic load',
        'Controlled load used for supply, immunity, inrush, and endurance conditions.',
        '["programmable DC load","AC load","load bank"]',
        '["current","voltage","power","resistance","load_profile"]',
        '["manual","visa","scpi","usb","ethernet","serial"]',
        'electrical',
        'conditional',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'source_measure_unit',
        'electronics',
        'Source measure unit',
        'Precision source and measurement instrument for low-level electrical characterization.',
        '["SMU","parametric source measure unit"]',
        '["voltage","current","resistance","iv_curve"]',
        '["visa","scpi","usb","ethernet","gpib"]',
        'electrical',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'function_generator',
        'electronics',
        'Function and arbitrary waveform generator',
        'Low-frequency signal source for bench excitation, timing checks, and simulated sensor signals.',
        '["function generator","arbitrary waveform generator","pulse generator"]',
        '["voltage","frequency","waveform","phase","duty_cycle"]',
        '["visa","scpi","usb","ethernet","gpib"]',
        'electrical',
        'conditional',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'emi_receiver',
        'emc',
        'EMI test receiver',
        'CEM emission measurement receiver category for detector-based conducted and radiated emission measurements.',
        '["EMI receiver","EMC receiver","measuring receiver"]',
        '["frequency","level","detector_level","bandwidth","emission_margin"]',
        '["visa","scpi","ethernet","gpib","usb"]',
        'rf',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'line_impedance_stabilization_network',
        'emc',
        'LISN and AMN',
        'Line impedance stabilization and artificial mains network category used in conducted-emission setups.',
        '["LISN","AMN","DC network","telecom network"]',
        '["impedance","insertion_loss","voltage","current","frequency"]',
        '["manual","switch_matrix","relay_io"]',
        'rf',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'emc_antennas_probes',
        'emc',
        'EMC antennas and probes',
        'Radiated emission, immunity, near-field, current, and voltage pickup category.',
        '["biconical antenna","log-periodic antenna","horn antenna","near-field probe","current probe"]',
        '["field_strength","antenna_factor","current","voltage","frequency"]',
        '["manual","switch_matrix","positioner","antenna_mast"]',
        'rf',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'rf_power_amplifier',
        'emc',
        'RF power amplifier',
        'Amplifier category for radiated and conducted immunity chains where gain, compression, and safety status must be tracked.',
        '["RF amplifier","power amplifier","broadband amplifier"]',
        '["forward_power","reflected_power","gain","frequency","temperature"]',
        '["manual","ethernet","serial","interlock_io","visa"]',
        'rf',
        'conditional',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'coupling_decoupling_network',
        'emc',
        'CDN and coupling network',
        'Conducted immunity coupling or decoupling network category for injecting disturbances into lines or ports.',
        '["CDN","coupling clamp","decoupling network","injection probe"]',
        '["insertion_loss","coupling_factor","frequency","level","impedance"]',
        '["manual","relay_io","switch_matrix"]',
        'rf',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'esd_generator',
        'emc',
        'ESD generator',
        'Electrostatic discharge generator category for contact and air discharge immunity tests.',
        '["ESD gun","ESD simulator","discharge network"]',
        '["voltage","discharge_mode","polarity","repetition","current_waveform"]',
        '["manual","usb","ethernet","vendor_sdk"]',
        'electrical',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'thermal_camera',
        'thermal',
        'Thermal camera',
        'Infrared imaging category for temperature maps, hotspots, and thermal behavior during EMC or endurance tests.',
        '["infrared camera","thermal imaging camera","radiometric camera"]',
        '["temperature","thermal_image","emissivity","spot_temperature","delta_temperature"]',
        '["manual","usb","ethernet","vendor_sdk"]',
        'thermal',
        'conditional',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'temperature_humidity_logger',
        'thermal',
        'Temperature and humidity logger',
        'Environmental monitoring category for lab or field conditions surrounding a measurement campaign.',
        '["temperature logger","humidity logger","thermocouple logger"]',
        '["temperature","humidity","dew_point","sample_time"]',
        '["manual","usb","ethernet","serial","bluetooth"]',
        'environmental',
        'conditional',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'climatic_chamber',
        'thermal',
        'Climatic chamber',
        'Controlled environment category for thermal, humidity, and combined-stress campaigns.',
        '["temperature chamber","climatic chamber","thermal shock chamber"]',
        '["temperature","humidity","ramp_rate","soak_time","chamber_state"]',
        '["ethernet","serial","modbus","vendor_sdk"]',
        'environmental',
        'conditional',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'temperature_calibrator',
        'thermal',
        'Temperature calibrator',
        'Reference or simulator category used to verify temperature sensors and acquisition chains.',
        '["dry block calibrator","temperature simulator","reference thermometer"]',
        '["temperature","reference_temperature","stability","sensor_error"]',
        '["manual","usb","serial","ethernet"]',
        'thermal',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'sound_level_meter',
        'acoustic',
        'Sound level meter',
        'Acoustic measurement category for sound pressure level, spectral noise, and environmental noise evidence.',
        '["integrating sound level meter","class 1 sound level meter","sound analyzer"]',
        '["sound_pressure_level","frequency_weighting","octave_band","time_weighting","leq"]',
        '["manual","usb","ethernet","memory_card"]',
        'acoustic',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'measurement_microphone',
        'acoustic',
        'Measurement microphone',
        'Microphone and preamplifier category for acoustic pressure acquisition and array measurements.',
        '["measurement microphone","microphone preamplifier","microphone array"]',
        '["sound_pressure","sensitivity","frequency_response","phase"]',
        '["analog_input","iepe","ccp","daq"]',
        'acoustic',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'acoustic_calibrator',
        'acoustic',
        'Acoustic calibrator',
        'Reference acoustic source category used before and after sound measurements.',
        '["sound calibrator","pistonphone","microphone calibrator"]',
        '["sound_pressure_level","frequency","stability"]',
        '["manual","usb"]',
        'acoustic',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'noise_dosimeter',
        'acoustic',
        'Noise dosimeter',
        'Portable acoustic monitoring category for long-duration exposure or site surveys.',
        '["noise dosimeter","wearable noise logger"]',
        '["dose","leq","peak_level","sample_time"]',
        '["manual","usb","bluetooth"]',
        'acoustic',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'accelerometer',
        'shock_vibration',
        'Accelerometer',
        'Vibration and shock sensor category for acceleration, velocity, displacement, and modal measurements.',
        '["IEPE accelerometer","triaxial accelerometer","charge accelerometer"]',
        '["acceleration","velocity","displacement","frequency","shock_pulse"]',
        '["iepe","charge_input","analog_input","daq"]',
        'mechanical',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'vibration_controller',
        'shock_vibration',
        'Vibration controller',
        'Control category for shaker tests, closed-loop vibration profiles, and mechanical stress campaigns.',
        '["vibration controller","shaker controller","signal conditioner"]',
        '["acceleration","profile_level","control_error","frequency","notching"]',
        '["ethernet","usb","analog_output","vendor_sdk"]',
        'mechanical',
        'conditional',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'impact_hammer',
        'shock_vibration',
        'Impact hammer',
        'Instrumented excitation category for modal testing and shock response characterization.',
        '["instrumented hammer","modal hammer","force transducer"]',
        '["force","impulse","frequency_response","trigger_time"]',
        '["iepe","charge_input","analog_input","daq"]',
        'mechanical',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'shock_vibration_logger',
        'shock_vibration',
        'Shock and vibration logger',
        'Portable monitoring category for transport, field, or endurance campaigns.',
        '["shock logger","vibration recorder","transport logger"]',
        '["acceleration","shock_event","rms_vibration","temperature","sample_time"]',
        '["usb","memory_card","bluetooth","ethernet"]',
        'mechanical',
        'conditional',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'spectrum_analyzer',
        'radio_rf',
        'Spectrum analyzer',
        'Frequency-domain RF measurement category for signal search, emission pre-scan, modulation, and spectrum occupancy.',
        '["spectrum analyzer","signal analyzer","real-time spectrum analyzer"]',
        '["frequency","level","bandwidth","modulation","occupied_bandwidth"]',
        '["visa","scpi","ethernet","usb","gpib"]',
        'rf',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'rf_signal_generator',
        'radio_rf',
        'RF signal generator',
        'RF source category for immunity chains, receiver checks, radio tests, and calibrated excitation.',
        '["RF signal generator","vector signal generator","microwave source"]',
        '["frequency","level","modulation","phase","output_power"]',
        '["visa","scpi","ethernet","usb","gpib"]',
        'rf',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'vector_network_analyzer',
        'radio_rf',
        'Vector network analyzer',
        'RF network measurement category for S-parameters, cable/fixture verification, and transfer functions.',
        '["VNA","cable analyzer","network analyzer"]',
        '["s_parameter","return_loss","insertion_loss","phase","frequency"]',
        '["visa","scpi","ethernet","usb","gpib"]',
        'rf',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'rf_power_meter',
        'radio_rf',
        'RF power meter',
        'RF power reference category for forward/reflected power, amplifier verification, and level traceability.',
        '["RF power meter","power sensor","directional power meter"]',
        '["power","frequency","crest_factor","average_power","peak_power"]',
        '["visa","scpi","usb","ethernet"]',
        'rf',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'radio_communication_tester',
        'radio_rf',
        'Radio communication tester',
        'Radio test set category for protocol-adjacent RF performance and monitored communication checks.',
        '["radio test set","communication analyzer","base-station tester"]',
        '["frequency","power","modulation_quality","receiver_sensitivity","packet_error_rate"]',
        '["ethernet","usb","visa","scpi","vendor_sdk"]',
        'rf',
        'conditional',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'daq_chassis',
        'data_monitoring',
        'DAQ chassis and modules',
        'Modular acquisition category for synchronized voltage, current, sensor, acoustic, vibration, and temperature channels.',
        '["DAQ chassis","analog input module","digital IO module","openDAQ device"]',
        '["voltage","current","temperature","strain","sound","vibration","digital_state"]',
        '["opendaq","usb","ethernet","ethercat","pcie","vendor_sdk"]',
        'data_acquisition',
        'required',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'data_logger',
        'data_monitoring',
        'Data logger',
        'Standalone or portable logging category for field campaigns, environmental data, and long-duration monitoring.',
        '["data logger","standalone recorder","portable recorder"]',
        '["voltage","current","temperature","humidity","event_state","sample_time"]',
        '["manual","usb","ethernet","serial","memory_card","wireless"]',
        'data_acquisition',
        'conditional',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'condition_monitoring_unit',
        'data_monitoring',
        'Condition monitoring unit',
        'Monitoring category for long-running assets, rotating machinery, environment, or EUT state capture.',
        '["condition monitor","edge logger","industrial monitoring node"]',
        '["vibration","temperature","current","status","trend"]',
        '["ethernet","modbus","opc_ua","mqtt","can","wireless"]',
        'data_acquisition',
        'conditional',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    ),
    (
        'timing_sync_unit',
        'data_monitoring',
        'Timing and synchronization unit',
        'Timing category used to synchronize distributed DAQ, cameras, RF instruments, and trigger systems.',
        '["GPS timebase","PTP grandmaster","trigger distribution unit","IRIG time source"]',
        '["time","trigger","phase","clock_drift","sync_status"]',
        '["ptp","gps","irig","pps","ethernet","bnc"]',
        'data_acquisition',
        'conditional',
        '2026-06-27-v1',
        '2026-06-27T00:00:00Z',
        '2026-06-27T00:00:00Z'
    );

INSERT INTO instrument_category_sources (
    category_code,
    source_name,
    source_url,
    source_note,
    captured_at
)
SELECT code,
       'Keysight electronic test equipment guide',
       'https://www.keysight.com/used/cz/en/knowledge/guides/top-10-electronic-test-equipment-for-engineers',
       'Public category reference for common electronic test equipment.',
       '2026-06-27T00:00:00Z'
FROM instrument_categories
WHERE code IN (
    'oscilloscope',
    'digital_multimeter',
    'lcr_impedance_meter',
    'dc_power_supply',
    'electronic_load',
    'source_measure_unit',
    'function_generator',
    'spectrum_analyzer',
    'rf_signal_generator',
    'vector_network_analyzer',
    'rf_power_meter'
);

INSERT INTO instrument_category_sources (
    category_code,
    source_name,
    source_url,
    source_note,
    captured_at
)
SELECT code,
       'Rohde and Schwarz EMC test equipment',
       'https://www.rohde-schwarz.com/us/products/test-and-measurement/emc-test-equipment_105350.html',
       'Public EMC equipment reference for emission and immunity chains.',
       '2026-06-27T00:00:00Z'
FROM instrument_categories
WHERE code IN (
    'emi_receiver',
    'line_impedance_stabilization_network',
    'emc_antennas_probes',
    'rf_power_amplifier',
    'coupling_decoupling_network',
    'esd_generator',
    'spectrum_analyzer',
    'rf_signal_generator'
);

INSERT INTO instrument_category_sources (
    category_code,
    source_name,
    source_url,
    source_note,
    captured_at
)
SELECT code,
       'NI data acquisition overview',
       'https://www.ni.com/en/shop/data-acquisition.html',
       'Public DAQ reference for electrical and physical measurement categories.',
       '2026-06-27T00:00:00Z'
FROM instrument_categories
WHERE code IN (
    'daq_chassis',
    'data_logger',
    'condition_monitoring_unit',
    'timing_sync_unit',
    'measurement_microphone',
    'accelerometer'
);

INSERT INTO instrument_category_sources (
    category_code,
    source_name,
    source_url,
    source_note,
    captured_at
)
SELECT code,
       'NTi Audio acoustic measurement overview',
       'https://www.nti-audio.com/en/support/know-how/what-is-a-sound-level-meter',
       'Public acoustic measurement reference for meters, microphones, and calibration.',
       '2026-06-27T00:00:00Z'
FROM instrument_categories
WHERE code IN (
    'sound_level_meter',
    'measurement_microphone',
    'acoustic_calibrator'
);

INSERT INTO instrument_category_sources (
    category_code,
    source_name,
    source_url,
    source_note,
    captured_at
)
SELECT code,
       'Larson Davis sound and vibration products',
       'https://www.larsondavis.com/',
       'Public reference for sound level meters, dosimeters, human vibration, and monitoring systems.',
       '2026-06-27T00:00:00Z'
FROM instrument_categories
WHERE code IN (
    'sound_level_meter',
    'noise_dosimeter',
    'shock_vibration_logger'
);

INSERT INTO instrument_category_sources (
    category_code,
    source_name,
    source_url,
    source_note,
    captured_at
)
SELECT code,
       'PCB Piezotronics accelerometers',
       'https://www.pcb.com/sensors-for-test-measurement/accelerometers',
       'Public accelerometer reference for vibration, shock, acceleration, and motion measurement.',
       '2026-06-27T00:00:00Z'
FROM instrument_categories
WHERE code IN (
    'accelerometer',
    'impact_hammer'
);

INSERT INTO instrument_category_sources (
    category_code,
    source_name,
    source_url,
    source_note,
    captured_at
)
SELECT code,
       'Dewesoft vibration analysis and testing',
       'https://dewesoft.com/applications/vibration-analysis',
       'Public reference for vibration sensors, FFT, harmonic, order, and monitoring workflows.',
       '2026-06-27T00:00:00Z'
FROM instrument_categories
WHERE code IN (
    'vibration_controller',
    'shock_vibration_logger',
    'condition_monitoring_unit',
    'daq_chassis'
);

INSERT INTO instrument_category_sources (
    category_code,
    source_name,
    source_url,
    source_note,
    captured_at
)
SELECT code,
       'Fluke temperature and thermal tools',
       'https://www.fluke.com/en-us/products/thermal-cameras',
       'Public temperature and thermal measurement reference.',
       '2026-06-27T00:00:00Z'
FROM instrument_categories
WHERE code IN (
    'thermal_camera',
    'temperature_humidity_logger',
    'climatic_chamber',
    'temperature_calibrator'
);

INSERT OR REPLACE INTO repository_metadata(key, value, updated_at)
VALUES
    ('instrument_taxonomy_revision', '2026-06-27-v1', '2026-06-27T00:00:00Z'),
    ('instrument_taxonomy_category_count', '34', '2026-06-27T00:00:00Z');

INSERT INTO schema_migrations(version, name, applied_at)
VALUES (2, 'instrument_categories', '2026-06-27T00:00:00Z');
