window.EMC_LOCUS_BOOTSTRAP = {
  "datasets": [
    [
      "RUN-001",
      "raw_signal",
      "data/RUN-001/raw.opendata",
      "sha256:raw001",
      "Immutable"
    ],
    [
      "RUN-001",
      "processed_signal",
      "data/RUN-001/current_fft.csv",
      "sha256:fft001",
      "Linked"
    ],
    [
      "RUN-004",
      "raw_sweep",
      "data/RUN-004/sweep.csv",
      "sha256:sweep004",
      "Immutable"
    ]
  ],
  "instrument_categories": [
    [
      "emi_receiver",
      "emc",
      "EMI test receiver",
      "required",
      "rf"
    ],
    [
      "line_impedance_stabilization_network",
      "emc",
      "LISN and AMN",
      "required",
      "rf"
    ],
    [
      "oscilloscope",
      "electronics",
      "Oscilloscope",
      "required",
      "electrical"
    ],
    [
      "thermal_camera",
      "thermal",
      "Thermal camera",
      "conditional",
      "thermal"
    ],
    [
      "sound_level_meter",
      "acoustic",
      "Sound level meter",
      "required",
      "acoustic"
    ],
    [
      "accelerometer",
      "shock_vibration",
      "Accelerometer",
      "required",
      "mechanical"
    ],
    [
      "spectrum_analyzer",
      "radio_rf",
      "Spectrum analyzer",
      "required",
      "rf"
    ],
    [
      "daq_chassis",
      "data_monitoring",
      "DAQ chassis and modules",
      "required",
      "data_acquisition"
    ]
  ],
  "instrument_documents": [
    [
      "RX-001",
      "certificate",
      "Certificat 2026",
      "metrology/RX-001/cert-2026.pdf",
      "A",
      "receiver calibration"
    ],
    [
      "RX-001",
      "datasheet",
      "Datasheet ESW",
      "metrology/RX-001/datasheet.pdf",
      "A",
      "technical data"
    ],
    [
      "DAQ-OPEN-01",
      "script",
      "openDAQ init",
      "scripts/daq/opendaq_init.py",
      "A",
      "measurement setup"
    ]
  ],
  "instruments": [
    [
      "RX-001",
      "Receiver",
      "Available",
      "CERT-2026-001",
      "2027-01-01",
      "ok",
      "EMI test receiver",
      "detectors",
      "Rohde Schwarz",
      "ESW",
      "100001",
      "ESW44",
      "2026-01-01",
      "12",
      "2"
    ],
    [
      "GEN-002",
      "Generator",
      "Reserved",
      "CERT-2025-044",
      "2026-07-12",
      "warn",
      "RF signal generator",
      "scpi",
      "Keysight",
      "N5183B",
      "100002",
      "N5183B-540",
      "2025-07-12",
      "12",
      "1"
    ],
    [
      "DAQ-OPEN-01",
      "DAQ",
      "Available",
      "CERT-2026-112",
      "2027-03-18",
      "ok",
      "DAQ chassis and modules",
      "8 channels",
      "openDAQ",
      "Reference DAQ",
      "DAQ001",
      "ODAQ-8",
      "2026-03-18",
      "12",
      "3"
    ],
    [
      "AMP-004",
      "Amplifier",
      "Out of service",
      "CERT-2024-090",
      "2025-12-04",
      "danger",
      "RF power amplifier",
      "interlock",
      "RF Lab",
      "AMP-250",
      "AMP004",
      "AMP-250",
      "2024-12-04",
      "12",
      "1"
    ]
  ],
  "methods": [
    [
      "EN61000-4-6-CS",
      "Conducted immunity",
      "frequency_sweep",
      "approved",
      "sha256:methodA"
    ],
    [
      "RAIL-HARM-01",
      "Railway harmonics",
      "mixed_time_frequency",
      "approved",
      "sha256:railH"
    ],
    [
      "INRUSH-DAQ-01",
      "Inrush current",
      "time_series",
      "draft",
      "sha256:inrushD"
    ],
    [
      "AXLE-COUNT-01",
      "Axle counter",
      "event_triggered",
      "approved",
      "sha256:axle"
    ]
  ],
  "projects": [
    {
      "blocker": "Calibration due soon",
      "code": "CEM-2026-001",
      "customer": "Rail Motion",
      "method": "Railway harmonics",
      "mode": "Accredite",
      "run": "RUN-001",
      "stage": "Measuring"
    },
    {
      "blocker": "Aucun",
      "code": "CEM-2026-002",
      "customer": "Aero Bench",
      "method": "Conducted immunity",
      "mode": "Non accredite",
      "run": "RUN-004",
      "stage": "Contract review"
    },
    {
      "blocker": "Mode relaxe",
      "code": "CEM-2026-003",
      "customer": "Power Lab",
      "method": "Inrush current",
      "mode": "Investigation",
      "run": "RUN-007",
      "stage": "Investigation"
    }
  ],
  "schedule": [
    [
      "PLAN-001",
      "CEM-2026-001",
      "Pre-scan emission conduite",
      "emission_conducted",
      "2026-07-01T09:00",
      "2026-07-01T12:00",
      "operator.one",
      "Lab A",
      "planned"
    ],
    [
      "PLAN-002",
      "CEM-2026-001",
      "Immunite rayonnee",
      "immunity_radiated",
      "2026-07-02T13:00",
      "2026-07-02T17:00",
      "operator.two",
      "Chambre",
      "confirmed"
    ]
  ],
  "test_categories": [
    [
      "emission",
      "",
      "Emission",
      "active"
    ],
    [
      "emission_conducted",
      "emission",
      "Emission conduite",
      "active"
    ],
    [
      "emission_radiated",
      "emission",
      "Emission rayonnee",
      "active"
    ],
    [
      "immunity",
      "",
      "Immunite",
      "active"
    ],
    [
      "immunity_conducted",
      "immunity",
      "Immunite conduite",
      "active"
    ],
    [
      "immunity_radiated",
      "immunity",
      "Immunite rayonnee",
      "active"
    ]
  ],
  "updates": [
    [
      "emc-locus-core",
      "0.2.0",
      "Signed",
      "Compatible",
      "offline_bundle"
    ],
    [
      "driver-pack-visa",
      "0.1.0",
      "Signed",
      "Pending validation",
      "online_catalog"
    ],
    [
      "report-template-fr",
      "0.1.1",
      "Signed",
      "Installed",
      "offline_bundle"
    ]
  ]
};
