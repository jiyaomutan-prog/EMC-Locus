const state = {
  view: "dashboard",
  search: "",
  offline: true,
  qualityMode: "Accredite",
  selectedProject: "CEM-2026-001",
};

const fallbackData = {
  projects: [
    {
      code: "CEM-2026-001",
      customer: "Rail Motion",
      stage: "Measuring",
      mode: "Accredite",
      blocker: "Calibration due soon",
      run: "RUN-001",
      method: "Railway harmonics",
    },
    {
      code: "CEM-2026-002",
      customer: "Aero Bench",
      stage: "Contract review",
      mode: "Non accredite",
      blocker: "Aucun",
      run: "RUN-004",
      method: "Conducted immunity",
    },
    {
      code: "CEM-2026-003",
      customer: "Power Lab",
      stage: "Investigation",
      mode: "Investigation",
      blocker: "Mode relaxe",
      run: "RUN-007",
      method: "Inrush current",
    },
  ],
  instruments: [
    ["RX-001", "Receiver", "Available", "CERT-2026-001", "2027-01-01", "ok", "EMI test receiver", "detectors"],
    ["GEN-002", "Generator", "Reserved", "CERT-2025-044", "2026-07-12", "warn", "RF signal generator", "scpi"],
    ["DAQ-OPEN-01", "DAQ", "Available", "CERT-2026-112", "2027-03-18", "ok", "DAQ chassis and modules", "8 channels"],
    ["AMP-004", "Amplifier", "Out of service", "CERT-2024-090", "2025-12-04", "danger", "RF power amplifier", "interlock"],
  ],
  instrument_categories: [
    ["emi_receiver", "emc", "EMI test receiver", "required", "rf"],
    ["line_impedance_stabilization_network", "emc", "LISN and AMN", "required", "rf"],
    ["oscilloscope", "electronics", "Oscilloscope", "required", "electrical"],
    ["thermal_camera", "thermal", "Thermal camera", "conditional", "thermal"],
    ["sound_level_meter", "acoustic", "Sound level meter", "required", "acoustic"],
    ["accelerometer", "shock_vibration", "Accelerometer", "required", "mechanical"],
    ["spectrum_analyzer", "radio_rf", "Spectrum analyzer", "required", "rf"],
    ["daq_chassis", "data_monitoring", "DAQ chassis and modules", "required", "data_acquisition"],
  ],
  methods: [
    ["EN61000-4-6-CS", "Conducted immunity", "frequency_sweep", "approved", "sha256:methodA"],
    ["RAIL-HARM-01", "Railway harmonics", "mixed_time_frequency", "approved", "sha256:railH"],
    ["INRUSH-DAQ-01", "Inrush current", "time_series", "draft", "sha256:inrushD"],
    ["AXLE-COUNT-01", "Axle counter", "event_triggered", "approved", "sha256:axle"],
  ],
  datasets: [
    ["RUN-001", "raw_signal", "data/RUN-001/raw.opendata", "sha256:raw001", "Immutable"],
    ["RUN-001", "processed_signal", "data/RUN-001/current_fft.csv", "sha256:fft001", "Linked"],
    ["RUN-004", "raw_sweep", "data/RUN-004/sweep.csv", "sha256:sweep004", "Immutable"],
  ],
  updates: [
    ["emc-locus-core", "0.2.0", "Signed", "Compatible", "offline_bundle"],
    ["driver-pack-visa", "0.1.0", "Signed", "Pending validation", "online_catalog"],
    ["report-template-fr", "0.1.1", "Signed", "Installed", "offline_bundle"],
  ],
};

const data = window.EMC_LOCUS_BOOTSTRAP || fallbackData;

const titles = {
  dashboard: "Tableau",
  projects: "Projets",
  metrology: "Metrologie",
  methods: "Definitions d'essais",
  data: "Donnees de mesure",
  updates: "Mises a jour",
};

const selectors = {
  statusStrip: document.querySelector("#status-strip"),
  title: document.querySelector("#view-title"),
  search: document.querySelector("#search-input"),
  offlineToggle: document.querySelector("#offline-toggle"),
  qualityMode: document.querySelector("#quality-mode"),
};

function matchesSearch(values) {
  if (!state.search) return true;
  return values.join(" ").toLowerCase().includes(state.search);
}

function badge(text, tone = "") {
  return `<span class="badge ${tone}">${text}</span>`;
}

function statusTone(value) {
  const lower = value.toLowerCase();
  if (lower.includes("out") || lower.includes("expired")) return "danger";
  if (lower.includes("due") || lower.includes("pending") || lower.includes("relaxe")) return "warn";
  return "ok";
}

function renderStatus() {
  const activeProjects = data.projects.length;
  const readyInstruments = data.instruments.filter((item) => item[5] === "ok").length;
  const approvedMethods = data.methods.filter((item) => item[3] === "approved").length;
  const immutableDatasets = data.datasets.filter((item) => item[4] === "Immutable").length;
  const instrumentCategories = (data.instrument_categories || []).length;
  const updateGate = data.updates.every((item) => item[2] === "Signed") ? "Strict" : "Review";

  selectors.statusStrip.innerHTML = [
    ["Projets", activeProjects],
    ["Mode qualite", state.qualityMode],
    ["Instruments prets", readyInstruments],
    ["Categories metro", instrumentCategories],
    ["Methodes approuvees", approvedMethods],
    ["Datasets immutables", immutableDatasets],
    ["Update gate", updateGate],
  ]
    .map(([label, value]) => `<article class="metric"><span>${label}</span><strong>${value}</strong></article>`)
    .join("");
}

function renderDashboard() {
  document.querySelector("#project-count").textContent = `${data.projects.length} ouverts`;
  document.querySelector("#dashboard-projects").innerHTML = data.projects
    .filter((project) => matchesSearch(Object.values(project)))
    .map(
      (project) => `
        <tr data-project="${project.code}">
          <td>${project.code}</td>
          <td>${project.customer}</td>
          <td>${project.stage}</td>
          <td>${project.mode}</td>
          <td>${badge(project.blocker, statusTone(project.blocker))}</td>
        </tr>`
    )
    .join("");

  const readiness = data.instruments
    .filter((item) => item[5] !== "ok")
    .map(
      (item) => `
      <div class="queue-item">
        <strong>${item[0]} ${badge(item[2], item[5])}</strong>
        <span>${item[1]} - ${item[3]} - ${item[4]}</span>
      </div>`
    );
  document.querySelector("#readiness-count").textContent = `${readiness.length} points`;
  document.querySelector("#readiness-list").innerHTML = readiness.join("");
  document.querySelector("#signal-facts").innerHTML = `
    <dt>Run</dt><dd>RUN-001</dd>
    <dt>Source</dt><dd>DAQ-OPEN-01</dd>
    <dt>Backend</dt><dd>reference_dft</dd>
    <dt>Lineage</dt><dd>raw001</dd>`;
}

function renderProjects() {
  const rows = data.projects
    .filter((project) => matchesSearch(Object.values(project)))
    .map(
      (project) => `
        <tr class="${project.code === state.selectedProject ? "selected" : ""}" data-project="${project.code}">
          <td>${project.code}</td>
          <td>${project.customer}</td>
          <td>${project.stage}</td>
          <td>${project.mode}</td>
        </tr>`
    )
    .join("");
  document.querySelector("#projects-table").innerHTML = rows;
  renderProjectDetail();
}

function renderProjectDetail() {
  const project = data.projects.find((item) => item.code === state.selectedProject) || data.projects[0];
  document.querySelector("#project-detail").innerHTML = `
    <h3>${project.code}</h3>
    <dl class="facts">
      <dt>Client</dt><dd>${project.customer}</dd>
      <dt>Etape</dt><dd>${project.stage}</dd>
      <dt>Mode</dt><dd>${project.mode}</dd>
      <dt>Run</dt><dd>${project.run}</dd>
      <dt>Methode</dt><dd>${project.method}</dd>
      <dt>Blocage</dt><dd>${project.blocker}</dd>
    </dl>`;
}

function renderMetrology() {
  document.querySelector("#metrology-table").innerHTML = data.instruments
    .filter((item) => matchesSearch(item))
    .map(
      (item) => `
      <tr>
        <td>${item[0]}</td>
        <td>${item[1]}</td>
        <td>${badge(item[2], item[5])}</td>
        <td>${item[3]}</td>
        <td>${item[4]}</td>
        <td>${item[6] || item[1]}</td>
        <td>${item[7] || "none"}</td>
      </tr>`
    )
    .join("");
  document.querySelector("#metrology-categories-table").innerHTML = (data.instrument_categories || [])
    .filter((item) => matchesSearch(item))
    .map(
      (item) => `
      <tr>
        <td>${item[0]}</td>
        <td>${item[1]}</td>
        <td>${item[2]}</td>
        <td>${badge(item[3], item[3] === "required" ? "ok" : "warn")}</td>
        <td>${item[4]}</td>
      </tr>`
    )
    .join("");
}

function renderMethods() {
  document.querySelector("#method-grid").innerHTML = data.methods
    .filter((item) => matchesSearch(item))
    .map(
      (item) => `
      <article class="method-card">
        <strong>${item[0]}</strong>
        <span class="tag">${item[1]}</span>
        <dl class="facts">
          <dt>Axe</dt><dd>${item[2]}</dd>
          <dt>Statut</dt><dd>${badge(item[3], item[3] === "approved" ? "ok" : "warn")}</dd>
          <dt>Checksum</dt><dd>${item[4]}</dd>
        </dl>
      </article>`
    )
    .join("");
}

function renderData() {
  document.querySelector("#data-table").innerHTML = data.datasets
    .filter((item) => matchesSearch(item))
    .map(
      (item) => `
      <tr>
        <td>${item[0]}</td>
        <td>${item[1]}</td>
        <td>${item[2]}</td>
        <td>${item[3]}</td>
        <td>${badge(item[4], item[4] === "Immutable" ? "ok" : "warn")}</td>
      </tr>`
    )
    .join("");
}

function renderUpdates() {
  document.querySelector("#update-gate").textContent = state.offline ? "Offline bundle" : "Online catalog";
  document.querySelector("#update-list").innerHTML = data.updates
    .filter((item) => matchesSearch(item))
    .map(
      (item) => `
      <article class="update-item">
        <strong>${item[0]} ${item[1]}</strong>
        <span>${badge(item[2], item[2] === "Signed" ? "ok" : "danger")} ${badge(item[3], statusTone(item[3]))} ${item[4]}</span>
      </article>`
    )
    .join("");
}

function render() {
  selectors.title.textContent = titles[state.view];
  document.querySelectorAll(".view").forEach((view) => view.classList.remove("active"));
  document.querySelector(`#${state.view}-view`).classList.add("active");
  document.querySelectorAll(".nav-item").forEach((item) => {
    item.classList.toggle("active", item.dataset.view === state.view);
  });
  selectors.offlineToggle.textContent = state.offline ? "Local" : "Reference";
  selectors.offlineToggle.classList.toggle("active", state.offline);
  selectors.offlineToggle.setAttribute("aria-pressed", String(state.offline));
  selectors.qualityMode.value = state.qualityMode;
  renderStatus();
  renderDashboard();
  renderProjects();
  renderMetrology();
  renderMethods();
  renderData();
  renderUpdates();
}

document.querySelectorAll(".nav-item").forEach((item) => {
  item.addEventListener("click", () => {
    state.view = item.dataset.view;
    render();
  });
});

selectors.search.addEventListener("input", (event) => {
  state.search = event.target.value.trim().toLowerCase();
  render();
});

selectors.offlineToggle.addEventListener("click", () => {
  state.offline = !state.offline;
  render();
});

selectors.qualityMode.addEventListener("change", (event) => {
  state.qualityMode = event.target.value;
  render();
});

document.querySelector("#projects-table").addEventListener("click", (event) => {
  const row = event.target.closest("tr[data-project]");
  if (!row) return;
  state.selectedProject = row.dataset.project;
  renderProjects();
});

document.querySelector("#dashboard-projects").addEventListener("click", (event) => {
  const row = event.target.closest("tr[data-project]");
  if (!row) return;
  state.selectedProject = row.dataset.project;
  state.view = "projects";
  render();
});

document.querySelector("#advance-project").addEventListener("click", () => {
  const project = data.projects.find((item) => item.code === state.selectedProject);
  if (!project) return;
  const flow = ["Contract review", "Measuring", "Technical review", "Report issued", "Archived"];
  const index = flow.indexOf(project.stage);
  project.stage = flow[Math.min(index + 1, flow.length - 1)];
  render();
});

render();
