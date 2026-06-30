const fallbackData = {
  lab_console_version: "ia-0.2",
  consoles: [
    {
      name: "LAB CONSOLE",
      technology: "Web application future",
      role: "Gestion du laboratoire, des objets metier et des decisions controlees.",
      owns: [
        "clients",
        "produits",
        "projets",
        "campagnes",
        "templates",
        "methodes",
        "documents",
        "roles",
        "competences",
        "metrologie",
        "planning",
        "rapports",
        "sync et audit",
      ],
      excludes: ["controle instrument", "acquisition", "faux run d'essai"],
    },
    {
      name: "TEST CONSOLE",
      technology: "Qt desktop local/offline",
      role: "Execution dense locale, instrumentation, acquisition et evidence d'essai.",
      owns: [
        "package campagne",
        "readiness",
        "chainage instrumental",
        "sequence",
        "monitoring",
        "acquisition",
        "deviations",
        "substitutions",
        "reruns",
        "publication evidence",
      ],
      excludes: ["creation template", "approbation methode", "edition metrologie source"],
    },
  ],
  shared_backbone: [
    "agent local",
    "audit",
    "outbox",
    "repositories SQLite",
    "sync offline future",
    "documents controles",
    "templates revisionnes",
    "methodes approuvees",
    "roles et competences",
    "metrologie",
    "readiness en contexte",
    "lineage resultats",
  ],
  guardrails: [
    "pas de faux controle instrument",
    "pas de fausse acquisition",
    "pas de workflows legacy comme source",
    "pas de CRUD disperse sans modele",
    "pas de mutation backend dans ce prototype",
    "pas de TEST CONSOLE dans le shell web",
  ],
  flow: [
    "Client",
    "Produit",
    "Version produit",
    "Projet",
    "Campagne",
    "Instance d'essai",
    "Package TEST",
    "Execution Qt",
    "Resultat valide",
    "Publication",
  ],
  nav_groups: [
    {
      title: "Clients et produits",
      spaces: ["clients", "products", "productVersions"],
    },
    {
      title: "Projets et campagnes",
      spaces: ["projects", "campaigns", "planning"],
    },
    {
      title: "Templates et methodes",
      spaces: ["projectTemplates", "campaignTemplates", "productTemplates", "testTemplates", "methods"],
    },
    {
      title: "Controle laboratoire",
      spaces: ["documents", "people", "metrology"],
    },
    {
      title: "Livraison et operations",
      spaces: ["reports", "sync"],
    },
  ],
  spaces: {
    clients: {
      title: "Clients",
      group: "Clients et produits",
      kind: "Organisation demandeuse",
      objective: "Maintenir les organisations, sites, contacts, contraintes de confidentialite et acces client.",
      handoff: "Point d'entree commercial. Les projets, produits et publications client doivent toujours revenir a cette identite.",
      guardrail: "Ne pas reduire le client a un champ texte dans un projet: les contacts, droits et publications dependent de cet objet.",
      objects: ["organisation", "site", "contact", "confidentialite", "acces client"],
      actions: ["creer client", "lier contact", "classer confidentialite", "preparer acces lecteur"],
      relations: ["possede produits", "demande projets", "recoit publications"],
      columns: ["Client", "Contacts", "Produits", "Projets actifs", "Contrainte"],
      records: [
        ["Rail Motion", "3", "Traction converter", "2", "NDA ferroviaire"],
        ["Aero Bench", "2", "Actuator bench", "1", "Diffusion restreinte"],
        ["Power Lab", "4", "Inverter platform", "3", "Client lecteur pilote"],
      ],
    },
    products: {
      title: "Produits",
      group: "Clients et produits",
      kind: "Famille d'equipement",
      objective: "Representer les familles d'equipements testes avant de figer une version precise pour projet.",
      handoff: "Le produit rassemble les exigences recurrentes; la version produit fige ce qui est vraiment teste.",
      guardrail: "Ne pas lancer une campagne sur un simple nom de produit: l'execution doit citer une version testee.",
      objects: ["produit", "famille", "proprietaire technique", "standards attendus", "preuves reutilisables"],
      actions: ["creer produit", "lier standards probables", "attacher documents generiques", "deriver version"],
      relations: ["appartient client", "contient versions produit", "alimente projets"],
      columns: ["Produit", "Client", "Famille", "Standards", "Versions"],
      records: [
        ["Traction converter", "Rail Motion", "Ferroviaire", "EN 50121, IEC 61000", "3"],
        ["Actuator bench", "Aero Bench", "Aerospace bench", "DO-160, IEC 61000", "2"],
        ["Inverter platform", "Power Lab", "Power electronics", "IEC 61800", "5"],
      ],
    },
    productVersions: {
      title: "Versions produit",
      group: "Clients et produits",
      kind: "Article teste",
      objective: "Figer la configuration effectivement testee, avec ses documents et variables critiques.",
      handoff: "La version produit est embarquee dans le package TEST pour identifier l'equipement sous test.",
      guardrail: "Ne pas modifier silencieusement la configuration apres execution; une nouvelle version ou deviation doit etre creee.",
      objects: ["version produit", "revision hardware", "revision software", "echantillon", "template produit teste"],
      actions: ["instancier template produit", "attacher fichiers client", "verrouiller configuration", "lier projet"],
      relations: ["vient produit", "utilisee par projet", "citee dans campagne et rapport"],
      columns: ["Version", "Produit", "Configuration", "Documents", "Etat"],
      records: [
        ["TC-INV-R3-FW18", "Traction converter", "HW R3 / FW 1.8", "drawing, firmware note", "prete"],
        ["ACT-BENCH-A2", "Actuator bench", "HW A2 / FPGA 4", "datasheet, wiring", "a completer"],
        ["INV-PLAT-M2", "Inverter platform", "Module M2", "bom, declaration", "verrouillee"],
      ],
    },
    projects: {
      title: "Projets",
      group: "Projets et campagnes",
      kind: "Enveloppe contractuelle",
      objective: "Piloter la prestation depuis la demande client jusqu'a la livraison du rapport.",
      handoff: "Le projet controle le mode qualite, les revues, les deviations et les campagnes qui seront executees.",
      guardrail: "Ne pas empiler des essais sans projet: les preuves doivent rester reliees au contrat, au client et au rapport.",
      objects: ["projet", "revue contrat", "mode qualite", "deviation", "campagne", "rapport"],
      actions: ["ouvrir depuis template", "completer revue contrat", "creer campagne", "approuver deviation", "archiver"],
      relations: ["appartient client", "reference version produit", "possede campagnes", "produit rapports"],
      columns: ["Projet", "Client", "Version produit", "Mode", "Etape"],
      records: [
        ["CEM-2026-001", "Rail Motion", "TC-INV-R3-FW18", "accredite", "measuring"],
        ["CEM-2026-002", "Aero Bench", "ACT-BENCH-A2", "non accredite", "contract review"],
        ["CEM-2026-003", "Power Lab", "INV-PLAT-M2", "investigation", "planning"],
      ],
    },
    campaigns: {
      title: "Campagnes",
      group: "Projets et campagnes",
      kind: "Plan d'essais",
      objective: "Regrouper les essais, ressources, methodes et preuves d'execution pour un projet.",
      handoff: "Une campagne fige un package lisible par TEST CONSOLE: essais, methodes, variables, documents et chaines instrumentales.",
      guardrail: "Ne pas laisser TEST CONSOLE inventer la campagne; elle execute une instance preparee et trace les ecarts.",
      objects: ["campagne", "test instance", "package offline", "conditions", "evidence retour"],
      actions: ["creer depuis template", "planifier essais", "figer package Qt", "recevoir evidence", "cloturer"],
      relations: ["appartient projet", "instancie templates essais", "alimente TEST CONSOLE", "recoit resultats"],
      columns: ["Campagne", "Projet", "Tests", "Package", "Etat"],
      records: [
        ["CMP-001-A", "CEM-2026-001", "7", "field-pack-001", "figee"],
        ["CMP-002-A", "CEM-2026-002", "3", "lab-pack-004", "brouillon"],
        ["CMP-003-I", "CEM-2026-003", "5", "investigation-pack", "en preparation"],
      ],
    },
    projectTemplates: {
      title: "Templates projet",
      group: "Templates et methodes",
      kind: "Definition generique",
      objective: "Standardiser l'ouverture de projets sans figer les instances deja executees.",
      handoff: "Instancie des projets et leurs campagnes par defaut; les mises a jour ne touchent que les elements non executes.",
      guardrail: "Ne pas confondre template et projet client: le template est une definition, pas une preuve de prestation.",
      objects: ["template projet", "revision", "checklist", "documents requis", "variables"],
      actions: ["dupliquer", "deriver", "approuver", "instancier projet", "mettre a jour non execute"],
      relations: ["cree projets", "inclut templates campagne", "requiert documents"],
      columns: ["Template", "Revision", "Usage", "Approbation", "Variables verrouillees"],
      records: [
        ["CEM accredited campaign", "PJT-REV-4", "EN 17025", "approved", "client, mode"],
        ["Fast investigation", "PJT-REV-1", "diagnostic", "draft", "none"],
        ["Non accredited service", "PJT-REV-2", "support client", "approved", "scope"],
      ],
    },
    campaignTemplates: {
      title: "Templates campagne",
      group: "Templates et methodes",
      kind: "Definition campagne",
      objective: "Composer des ensembles d'essais reutilisables et preparer les packages d'execution.",
      handoff: "Produit des campagnes planifiables puis exportables vers TEST CONSOLE apres gel controle.",
      guardrail: "Ne pas mettre des resultats ou des instruments reels dans le template; ce sont des donnees d'instance.",
      objects: ["template campagne", "revision", "liste essais", "sequence", "competences"],
      actions: ["creer revision", "lier templates essais", "approuver", "instancier campagne"],
      relations: ["utilise par projets", "reference templates essais", "definit paquet TEST CONSOLE"],
      columns: ["Template", "Revision", "Tests inclus", "Competence", "Etat"],
      records: [
        ["Railway EMC baseline", "CMP-T-3", "11", "ferroviaire CEM", "approved"],
        ["Conducted immunity pack", "CMP-T-1", "4", "immunite conduite", "draft"],
        ["Power electronics scan", "CMP-T-2", "6", "emission + inrush", "approved"],
      ],
    },
    productTemplates: {
      title: "Templates produit teste",
      group: "Templates et methodes",
      kind: "Definition article",
      objective: "Normaliser la description des equipements testes et de leurs pieces client.",
      handoff: "Instancie des versions produit avec variables critiques verrouillables avant campagne.",
      guardrail: "Ne pas faire porter a l'essai l'identite produit complete; cette identite est geree avant execution.",
      objects: ["template produit", "champs identification", "variables config", "documents requis", "lock policy"],
      actions: ["definir champs", "verrouiller identite", "instancier version", "deriver template"],
      relations: ["cree versions produit", "alimente projets", "contraint rapports"],
      columns: ["Template", "Champs critiques", "Documents", "Verrouillage", "Etat"],
      records: [
        ["Rail traction equipment", "serial, HW, FW", "drawing, cabling", "avant campagne", "approved"],
        ["Generic EUT", "serial, variant", "datasheet", "avant execution", "approved"],
        ["DAQ monitored system", "channels, sync", "channel map", "avant package", "draft"],
      ],
    },
    testTemplates: {
      title: "Templates essais",
      group: "Templates et methodes",
      kind: "Definition executable",
      objective: "Decrire chaine instrumentale, sequence, embranchements, limites, traitements et parametres modifiables.",
      handoff: "Cree des instances d'essai dans une campagne; TEST CONSOLE execute l'instance, pas le template.",
      guardrail: "Ne pas attacher une acquisition reelle au template; les donnees appartiennent a l'execution.",
      objects: ["test template", "slot instrumentation", "step", "branch rule", "limite", "post-processing"],
      actions: ["authorer", "lier methode", "instancier test", "deriver depuis execution", "controler mise a jour"],
      relations: ["cree test instances", "reference methodes", "requiert metrologie", "produit execution Qt"],
      columns: ["Template", "Methode", "Slots", "Sequence", "Resultats"],
      records: [
        ["Conducted emission sweep", "EN55032-CE-R2", "receiver, LISN", "12 steps", "level vs freq"],
        ["Railway harmonics", "RAIL-HARM-R1", "DAQ, voltage probe", "8 steps + branch", "FFT + harmonics"],
        ["Inrush capture", "INRUSH-R1", "DAQ, current probe", "triggered", "time peak"],
      ],
    },
    methods: {
      title: "Methodes",
      group: "Templates et methodes",
      kind: "Revision approuvee",
      objective: "Controler les revisions de methodes, leurs standards, validations et approbations.",
      handoff: "TEST CONSOLE lit une revision approuvee et ses regles; LAB CONSOLE gere l'auteur, la revue et l'approbation.",
      guardrail: "Ne pas autoriser une station a approuver une methode parce qu'un operateur a besoin de lancer un essai.",
      objects: ["methode", "revision", "approbation", "seconde approbation", "competence", "validation"],
      actions: ["creer revision", "demander revue", "approuver", "seconde approuver", "suspendre"],
      relations: ["reference standards", "autorise templates essais", "requiert competences"],
      columns: ["Methode", "Revision", "Statut", "Seconde approbation", "Competence"],
      records: [
        ["EN55032 conducted emission", "R2", "approved", "non", "emission conduite"],
        ["Railway harmonic analysis", "R1", "approved", "oui", "ferroviaire + signal"],
        ["Inrush current capture", "R1", "draft", "oui", "DAQ temporel"],
      ],
    },
    documents: {
      title: "Normes et documents",
      group: "Controle laboratoire",
      kind: "Objets controles",
      objective: "Gerer normes, PDFs, pieces client, certificats, scripts, worksheets et documents de rapport.",
      handoff: "Les documents sont references par LAB et TEST via identite, revision, checksum, applicabilite et confidentialite.",
      guardrail: "Ne pas stocker un fichier comme simple piece jointe orpheline: il doit avoir proprietaire, usage, revision et trace.",
      objects: ["standard", "document", "revision", "checksum", "classification", "applicabilite"],
      actions: ["enregistrer reference", "attacher fichier", "classer", "lier objet", "marquer supersede"],
      relations: ["cite par methodes", "fourni par client", "attache instruments", "alimente rapports"],
      columns: ["Document", "Type", "Revision", "Lien", "Etat"],
      records: [
        ["EN 50121 extract", "standard reference", "2024", "method RAIL-HARM", "applicable"],
        ["RX-001 certificate", "calibration certificate", "A", "instrument RX-001", "valid"],
        ["TC-INV wiring", "client drawing", "C", "version TC-INV-R3-FW18", "applicable"],
      ],
    },
    people: {
      title: "Personnel, competences, roles",
      group: "Controle laboratoire",
      kind: "Autorisations configurables",
      objective: "Modeler personnes, roles renommables, droits cumulables, competences et approbations.",
      handoff: "TEST CONSOLE consomme le contexte de role et competence; LAB CONSOLE gere les definitions, affectations et revocations.",
      guardrail: "Ne pas coder en dur les intitules admin ou operateur comme seule source d'autorisation.",
      objects: ["personne", "role", "permission", "competence", "delegation", "approbation"],
      actions: ["assigner role", "renommer role", "enregistrer competence", "approuver methode", "revoquer acces"],
      relations: ["autorise methodes", "qualifie operateurs", "signe validations", "trace audit"],
      columns: ["Personne", "Roles", "Competences", "Portee", "Etat"],
      records: [
        ["quality.lead", "responsable qualite", "audit, publication", "laboratoire", "actif"],
        ["technical.lead", "responsable technique", "methodes, deviations", "CEM", "actif"],
        ["operator.one", "operateur", "emission conduite", "campagne CMP-001-A", "actif"],
      ],
    },
    metrology: {
      title: "Metrologie",
      group: "Controle laboratoire",
      kind: "Inventaire et preuves",
      objective: "Maintenir instruments, categories, documents, calibration, serviceability, restrictions et reservations externes.",
      handoff: "LAB CONSOLE maintient les sources; TEST CONSOLE calcule la readiness sur l'instance d'essai et ses instruments lies.",
      guardrail: "Ne pas corriger une calibration depuis l'ecran d'execution; la station peut demander une substitution ou tracer un refus.",
      objects: ["asset", "categorie", "calibration", "certificat", "datasheet", "script", "contact externe"],
      actions: ["enregistrer asset", "attacher certificat", "changer serviceability", "reserver", "revoir due date"],
      relations: ["satisfait slots instrumentation", "alimente readiness", "contraint planning"],
      columns: ["Asset", "Categorie", "Calibration", "Service", "Documents"],
      records: [
        ["RX-001", "EMI receiver", "valid to 2027-01-01", "usable", "certificate, datasheet"],
        ["DAQ-OPEN-01", "DAQ chassis", "valid to 2027-03-18", "usable", "script, channel map"],
        ["EXT-ANT-9K", "external antenna", "reservation contact", "external", "provider certificate"],
      ],
    },
    planning: {
      title: "Planning",
      group: "Projets et campagnes",
      kind: "Ressources",
      objective: "Planifier essais, salles, bancs, instruments internes/externes et operateurs competents.",
      handoff: "Le planning produit des affectations et reservations que le package TEST doit afficher et verifier.",
      guardrail: "Ne pas confondre disponibilite planning et aptitude metrologique; les deux doivent rester visibles.",
      objects: ["creneau", "reservation", "operateur", "salle", "instrument", "conflit"],
      actions: ["reserver", "assigner", "detecter conflit", "figer package", "replanifier"],
      relations: ["planifie campagnes", "utilise metrologie", "assigne personnes", "alimente TEST CONSOLE"],
      columns: ["Creneau", "Campagne", "Ressource", "Operateur", "Etat"],
      records: [
        ["2026-07-01 09:00", "CMP-001-A", "Lab A + RX-001", "operator.one", "confirmed"],
        ["2026-07-02 13:00", "CMP-001-A", "Chambre + antenna", "operator.two", "planned"],
        ["2026-07-04 08:30", "CMP-003-I", "Field station", "operator.one", "package pending"],
      ],
    },
    reports: {
      title: "Rapports et publications",
      group: "Livraison et operations",
      kind: "Publications",
      objective: "Controler revues techniques, approbations, exports et livraisons client.",
      handoff: "Les resultats publies viennent d'evidences TEST validees; LAB controle le rapport et la livraison.",
      guardrail: "Ne pas publier un resultat brut ou calcule comme resultat client sans validation et contexte de rapport.",
      objects: ["rapport", "revision", "revue technique", "approbation", "publication", "export"],
      actions: ["preparer draft", "revoir", "approuver", "publier", "superseder"],
      relations: ["appartient projet", "cite campagnes", "publie resultats valides", "visible client"],
      columns: ["Rapport", "Projet", "Revision", "Statut", "Publication"],
      records: [
        ["RPT-CEM-001", "CEM-2026-001", "A", "technical review", "not published"],
        ["RPT-CEM-002", "CEM-2026-002", "draft", "draft", "not published"],
        ["RPT-CEM-003", "CEM-2026-003", "investigation note", "internal", "restricted"],
      ],
    },
    sync: {
      title: "Sync, audit, updates",
      group: "Livraison et operations",
      kind: "Controle offline",
      objective: "Superviser outbox, snapshots, conflits, audit et mises a jour signees.",
      handoff: "Les deux consoles produisent ou consomment des operations locales; LAB supervise reconciliation, audit et updates.",
      guardrail: "Ne pas supposer une base centrale verrouillee: le futur offline multi-operateur exige sync/merge explicite.",
      objects: ["sync operation", "outbox", "snapshot", "conflit", "audit event", "update package"],
      actions: ["inspecter outbox", "resoudre conflit", "verifier snapshot", "valider update", "bloquer live update"],
      relations: ["recoit evenements des consoles", "prepare offline", "trace revisions", "protege station"],
      columns: ["Objet", "Type", "Etat", "Source", "Action requise"],
      records: [
        ["OP-893", "outbox", "pending", "TEST CONSOLE", "sync package"],
        ["SNAP-2026-07", "snapshot", "signed", "LAB CONSOLE", "ready field"],
        ["driver-pack-visa", "update", "pending validation", "catalog", "quality review"],
      ],
    },
  },
};

const bootstrap = window.EMC_LOCUS_BOOTSTRAP || {};
const data =
  bootstrap.lab_console_version === fallbackData.lab_console_version
    ? { ...fallbackData, ...bootstrap, spaces: { ...fallbackData.spaces, ...(bootstrap.spaces || {}) } }
    : fallbackData;

const state = {
  space: "overview",
  search: "",
};

const selectors = {
  nav: document.querySelector("#nav-list"),
  statusStrip: document.querySelector("#status-strip"),
  title: document.querySelector("#view-title"),
  summary: document.querySelector("#view-summary"),
  search: document.querySelector("#search-input"),
};

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function matchesSearch(values) {
  if (!state.search) return true;
  return values.join(" ").toLowerCase().includes(state.search);
}

function tag(text, tone = "") {
  return `<span class="tag ${tone}">${escapeHtml(text)}</span>`;
}

function renderNav() {
  const overviewButton = `
    <button class="nav-item" data-space="overview" type="button">Carte LAB/TEST</button>`;
  const groups = data.nav_groups
    .map((group) => {
      const items = group.spaces
        .map((spaceKey) => {
          const space = data.spaces[spaceKey];
          return `<button class="nav-item" data-space="${escapeHtml(spaceKey)}" type="button">${escapeHtml(
            space.title
          )}</button>`;
        })
        .join("");
      return `
        <section class="nav-section">
          <h2>${escapeHtml(group.title)}</h2>
          ${items}
        </section>`;
    })
    .join("");

  selectors.nav.innerHTML = `${overviewButton}${groups}`;
  selectors.nav.querySelectorAll(".nav-item").forEach((item) => {
    item.addEventListener("click", () => {
      state.space = item.dataset.space;
      render();
    });
  });
}

function renderStatus() {
  const spaceCount = Object.keys(data.spaces).length;
  const methodCount = data.spaces.methods.records.length;
  const templateCount =
    data.spaces.projectTemplates.records.length +
    data.spaces.campaignTemplates.records.length +
    data.spaces.productTemplates.records.length +
    data.spaces.testTemplates.records.length;

  selectors.statusStrip.innerHTML = [
    ["Consoles", "2"],
    ["Espaces LAB", spaceCount],
    ["Templates", templateCount],
    ["Methodes", methodCount],
    ["Documents", data.spaces.documents.records.length],
    ["Roles", data.spaces.people.records.length],
    ["Metrologie", data.spaces.metrology.records.length],
    ["Runtime", "Qt"],
  ]
    .map(([label, value]) => `<article class="metric"><span>${label}</span><strong>${value}</strong></article>`)
    .join("");
}

function renderOverview() {
  selectors.title.textContent = "Architecture LAB";
  selectors.summary.textContent =
    "Prototype statique de l'architecture d'information LAB CONSOLE, avec frontiere explicite vers TEST CONSOLE Qt.";

  document.querySelector("#console-grid").innerHTML = data.consoles
    .map(
      (consoleSurface) => `
        <article class="console-card">
          <div>
            <h3>${escapeHtml(consoleSurface.name)}</h3>
            <strong>${escapeHtml(consoleSurface.technology)}</strong>
            <p>${escapeHtml(consoleSurface.role)}</p>
          </div>
          <div>
            <span class="mini-label">Appartient ici</span>
            <div class="tag-list">${consoleSurface.owns.map((item) => tag(item)).join("")}</div>
          </div>
          <div>
            <span class="mini-label">Exclu</span>
            <div class="tag-list">${consoleSurface.excludes.map((item) => tag(item, "danger")).join("")}</div>
          </div>
        </article>`
    )
    .join("");

  document.querySelector("#shared-backbone").innerHTML = data.shared_backbone.map((item) => tag(item, "ok")).join("");
  document.querySelector("#static-guardrails").innerHTML = data.guardrails
    .map((item) => `<div class="guard-item">${escapeHtml(item)}</div>`)
    .join("");
  document.querySelector("#relationship-flow").innerHTML = data.flow
    .map((item, index) => `<span>${escapeHtml(item)}</span>${index < data.flow.length - 1 ? "<b></b>" : ""}`)
    .join("");

  document.querySelector("#lab-domain-map").innerHTML = data.nav_groups
    .map(
      (group) => `
        <article class="domain-column">
          <h3>${escapeHtml(group.title)}</h3>
          ${group.spaces
            .map((spaceKey) => {
              const space = data.spaces[spaceKey];
              return `<button class="domain-link" data-space="${escapeHtml(spaceKey)}" type="button">
                <strong>${escapeHtml(space.title)}</strong>
                <span>${escapeHtml(space.kind)}</span>
              </button>`;
            })
            .join("")}
        </article>`
    )
    .join("");

  document.querySelectorAll(".domain-link").forEach((item) => {
    item.addEventListener("click", () => {
      state.space = item.dataset.space;
      render();
    });
  });
}

function renderSpace(spaceKey) {
  const space = data.spaces[spaceKey];
  if (!space) return;

  selectors.title.textContent = space.title;
  selectors.summary.textContent = space.objective;
  document.querySelector("#space-kind").textContent = space.kind;
  document.querySelector("#space-group").textContent = space.group;
  document.querySelector("#space-objective").textContent = space.objective;
  document.querySelector("#space-handoff").textContent = space.handoff;
  document.querySelector("#space-guardrail").textContent = space.guardrail;
  document.querySelector("#space-object-count").textContent = `${space.objects.length} objets`;
  document.querySelector("#space-objects").innerHTML = space.objects.map((item) => tag(item)).join("");
  document.querySelector("#space-actions").innerHTML = space.actions
    .map((item) => `<div class="action-item">${escapeHtml(item)}</div>`)
    .join("");
  document.querySelector("#space-relations").innerHTML = space.relations
    .map((item) => `<div class="relation-item">${escapeHtml(item)}</div>`)
    .join("");

  const rows = space.records.filter((record) => matchesSearch(record));
  document.querySelector("#space-record-count").textContent = `${rows.length} lignes`;
  document.querySelector("#space-table").innerHTML = `
    <thead>
      <tr>${space.columns.map((column) => `<th>${escapeHtml(column)}</th>`).join("")}</tr>
    </thead>
    <tbody>
      ${rows
        .map((record) => `<tr>${record.map((cell) => `<td>${escapeHtml(cell)}</td>`).join("")}</tr>`)
        .join("")}
    </tbody>`;
}

function render() {
  const isOverview = state.space === "overview";
  document.querySelector("#overview-view").classList.toggle("active", isOverview);
  document.querySelector("#space-view").classList.toggle("active", !isOverview);
  document.querySelectorAll(".nav-item").forEach((item) => {
    item.classList.toggle("active", item.dataset.space === state.space);
  });
  renderStatus();
  if (isOverview) {
    renderOverview();
  } else {
    renderSpace(state.space);
  }
}

selectors.search.addEventListener("input", (event) => {
  state.search = event.target.value.trim().toLowerCase();
  render();
});

renderNav();
render();
