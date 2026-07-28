import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { App } from "./App";
import { FleetWorkspace } from "./features/equipment/FleetWorkspace";
import { StationSetupWorkspace } from "./features/equipment/StationSetupWorkspace";
import { retainCompatibleAssignments } from "./features/planning/LaboratoryPlanningWorkspace";
import type { EquipmentModelAggregate, EquipmentModelDefinition } from "./models/equipment";
import type { AssetCorrectionAssignment } from "./models/metrology";
import type { PhysicalAsset, PhysicalAssetMetrologySummary } from "./models/fleet";
import type {
  CompletedContractReviewItem,
  LaboratoryScheduleItem,
  PlannedTestPreparationAggregate,
  PlannedTestPreparationOptions,
  PlannedTestPreparationRevision,
  ProjectAuditEvent,
  ProjectRecord,
  ServiceScheduleItem
} from "./models/projects";
import {
  auditFixture,
  healthFixture,
  jsonResponse,
  revisionFixture,
  storageFixture,
  templateFixture
} from "./test/fixtures";

const fetchMock = vi.fn();

function canonicalChecksum(hexDigit: string): string {
  return `sha256:${hexDigit.repeat(64)}`;
}

beforeEach(() => {
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.restoreAllMocks();
  fetchMock.mockReset();
});

function mockBaseApi(templates = [templateFixture()]) {
  fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
    const path = String(input);
    if (path === "/api/v1/health") return jsonResponse(healthFixture);
    if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
    if (path === "/api/v1/test-templates") {
      if (init?.method === "POST") {
        return jsonResponse({
          operation: "test_template_created",
          operation_id: "op",
          replayed: false,
          test_template: templateFixture(),
          revision: revisionFixture()
        });
      }
      return jsonResponse({ test_templates: templates });
    }
    if (path === "/api/v1/test-templates/TT-LAB-001") {
      return jsonResponse({ test_template: templates[0] ?? templateFixture() });
    }
    if (path.includes("/revisions/TT-LAB-001-rev-0001") && !path.endsWith("/definition") && !path.includes("/transitions/")) {
      return jsonResponse({ revision: revisionFixture() });
    }
    if (path === "/api/v1/test-templates/TT-LAB-001/revisions") {
      return jsonResponse({ template_id: "TT-LAB-001", revisions: [revisionFixture()] });
    }
    if (path === "/api/v1/test-templates/TT-LAB-001/audit-events") {
      return jsonResponse({ template_id: "TT-LAB-001", audit_events: auditFixture });
    }
    if (path === "/api/v1/test-template-definitions/validate") {
      return jsonResponse({ valid: true, issues: [], definition_checksum: canonicalChecksum("b") });
    }
    if (path.endsWith("/definition")) {
      return jsonResponse({
        operation: "test_template_definition_replaced",
        operation_id: "op",
        replayed: false,
        test_template: templateFixture(),
        revision: { ...revisionFixture(), definition_checksum: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" }
      });
    }
    if (path.endsWith("/transitions/submit-for-review")) {
      return jsonResponse({
        operation: "test_template_submitted_for_review",
        operation_id: "op",
        replayed: false,
        test_template: templateFixture("draft"),
        revision: { ...revisionFixture("under_review"), status: "under_review" }
      });
    }
    if (path.endsWith("/transitions/approve")) {
      return jsonResponse({
        operation: "test_template_approved",
        operation_id: "op",
        replayed: false,
        test_template: templateFixture("approved"),
        revision: { ...revisionFixture("approved"), status: "approved" }
      });
    }
    if (path.endsWith("/clone")) {
      return jsonResponse({
        operation: "test_template_cloned",
        operation_id: "op",
        replayed: false,
        test_template: templateFixture(),
        revision: revisionFixture()
      });
    }
    return jsonResponse({ error: { code: "not_found", message: path } }, 404);
  });
}

describe("LAB CONSOLE", () => {
  test("retains only assignments compatible with the newly selected method and setup", () => {
    const current = {
      receiver: "receiver-binding",
      obsoleteRole: "old-binding"
    };
    const retained = retainCompatibleAssignments(current, ["receiver", "generator"], [
      {
        slot_id: "receiver",
        binding_id: "receiver-binding",
        compatible: true
      },
      {
        slot_id: "generator",
        binding_id: "generator-binding",
        compatible: false,
        reason: "La catégorie ne correspond pas."
      }
    ]);

    expect(retained).toEqual({ receiver: "receiver-binding" });
    expect(retainCompatibleAssignments(retained, ["receiver"], [
      { slot_id: "receiver", binding_id: "receiver-binding", compatible: true }
    ])).toBe(retained);
  });

  test("renders an empty API library without fake business rows", async () => {
    mockBaseApi([]);

    render(<App />);

    expect(await screen.findByText("Aucune méthode d’essai")).toBeInTheDocument();
    expect(screen.queryByText("CEM-2026-001")).not.toBeInTheDocument();
    expect(screen.queryByText("Client demo")).not.toBeInTheDocument();
  });

  test("keeps navigation focused on active work and catalog controls contextual", async () => {
    mockBaseApi([]);
    const user = userEvent.setup();

    render(<App />);

    await screen.findByText("Aucune méthode d’essai");
    expect(screen.queryByText("Métrologie")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Réduire la navigation" }));
    expect(screen.getByRole("button", { name: "Déployer la navigation" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Catalogue des modèles" }));
    expect(await screen.findByLabelText("Rechercher un modèle")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Administration du référentiel" }));
    expect(screen.queryByLabelText("Rechercher un modèle")).not.toBeInTheDocument();
  });

  test("moves a dossier from contract review to a confirmed laboratory slot", async () => {
    mockProjectWorkflowApi();
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Dossiers d'essai" }));
    expect(await screen.findByRole("heading", { name: "CEM-UX-001" })).toBeInTheDocument();
    expect(screen.queryByText("Prochaines verticales")).not.toBeInTheDocument();
    expect(screen.queryByText("À venir")).not.toBeInTheDocument();

    const requestDefined = screen.getByRole("checkbox", {
      name: "La demande du client est définie"
    });
    await user.click(requestDefined);
    await waitFor(() => expect(requestDefined).toBeChecked());

    const deviationsRecorded = screen.getByRole("checkbox", {
      name: "Les écarts et adaptations sont consignés"
    });
    await user.click(deviationsRecorded);
    await waitFor(() => expect(deviationsRecorded).toBeChecked());

    await user.click(await screen.findByRole("button", { name: "Passer à la planification" }));
    const planningButtons = await screen.findAllByRole("button", { name: "Planifier un essai" });
    await user.click(planningButtons[0]);

    const reservationButton = screen.getByRole("button", { name: "Réserver le créneau" });
    expect(screen.getByLabelText("Essai prévu")).toBeRequired();
    expect(screen.getByLabelText("Date")).toBeRequired();
    expect(screen.getByLabelText("Début")).toBeRequired();
    expect(screen.getByLabelText("Fin")).toBeRequired();
    expect(screen.getByLabelText("Opérateur")).toBeRequired();
    expect(screen.getByLabelText("Lieu")).toBeRequired();
    expect(screen.getByLabelText("Objet soumis à l’essai")).toBeRequired();
    expect(screen.getByText("Produit, prototype ou sous-ensemble du client")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent(
      "À renseigner avant de réserver : Essai prévu, Objet soumis à l’essai."
    );
    expect(reservationButton).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Compléter les champs" }));
    expect(screen.getByLabelText("Essai prévu")).toHaveFocus();
    await user.type(screen.getByLabelText("Essai prévu"), "Émission conduite");
    await user.click(screen.getByRole("button", { name: "Compléter les champs" }));
    expect(screen.getByLabelText("Objet soumis à l’essai")).toHaveFocus();
    await user.selectOptions(screen.getByLabelText("Lieu"), "LAB-LOCATION-CEM-1");
    await user.type(screen.getByLabelText("Objet soumis à l’essai"), "Convertisseur prototype");
    expect(reservationButton).toBeEnabled();
    await user.click(reservationButton);

    expect(await screen.findByText("Émission conduite")).toBeInTheDocument();
    expect(screen.getByText("Prévu")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Confirmer le créneau" }));

    expect(await screen.findByText("Planning à jour")).toBeInTheDocument();
    expect(screen.getByText("Confirmé")).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining("/schedule-items/PLAN-CEM-UX-001-"),
      expect.objectContaining({ method: "POST" })
    );
  });

  test("keeps a project schedule readable when laboratory locations fail to load", async () => {
    mockProjectWorkflowApi({ existingSchedule: true, locationFailure: true });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Dossiers d'essai" }));
    expect(await screen.findByRole("heading", { name: "CEM-UX-001" })).toBeInTheDocument();
    expect(screen.getByText("Émission conduite déjà planifiée")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Les lieux du laboratoire sont temporairement indisponibles"
    );
    for (const button of screen.getAllByRole("button", { name: /Planifier un essai|Planifier le premier essai/ })) {
      expect(button).toBeDisabled();
    }
  });

  test("filters the laboratory week and keeps reschedule values after a conflict", async () => {
    mockLaboratoryPlanningApi();
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Planning du laboratoire" }));
    expect(await screen.findByText("CEM-LAB-001 · Industries Atlas")).toBeInTheDocument();
    expect(screen.getByText("CEM-LAB-002 · Mobilités Boréal")).toBeInTheDocument();
    expect(screen.queryByText(/LAB-LOCATION-/)).not.toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("Lieu"), "LAB-LOCATION-CEM-1");
    expect(screen.getByText("CEM-LAB-001 · Industries Atlas")).toBeInTheDocument();
    expect(screen.queryByText("CEM-LAB-002 · Mobilités Boréal")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Effacer les filtres" }));

    await user.click(
      screen.getByRole("button", {
        name: "Ouvrir Émission conduite, dossier CEM-LAB-001"
      })
    );
    await user.click(screen.getByRole("button", { name: "Déplacer" }));
    await user.type(
      screen.getByLabelText("Raison du changement"),
      "Réorganisation avec le client"
    );
    await user.click(screen.getByRole("button", { name: "Enregistrer le déplacement" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Alice Martin est déjà affecté à « Immunité rayonnée » du dossier CEM-LAB-002"
    );
    expect(screen.getByLabelText("Raison du changement")).toHaveValue(
      "Réorganisation avec le client"
    );
    expect(screen.getByLabelText("Date")).toHaveValue("2026-07-15");

    fireEvent.change(screen.getByLabelText("Date"), { target: { value: "2026-07-17" } });
    await user.click(screen.getByRole("button", { name: "Enregistrer le déplacement" }));

    expect(await screen.findByText(/Vendredi 17 juil/i)).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining("/schedule-items/PLAN-LAB-001/reschedule"),
      expect.objectContaining({ method: "POST" })
    );
  });

  test("keeps the laboratory week readable when location choices fail to load", async () => {
    mockLaboratoryPlanningApi({ locationFailure: true });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Planning du laboratoire" }));
    expect(await screen.findByText("CEM-LAB-001 · Industries Atlas")).toBeInTheDocument();
    expect(screen.getByText("CEM-LAB-002 · Mobilités Boréal")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Les lieux du laboratoire sont temporairement indisponibles"
    );
    await user.click(
      screen.getByRole("button", { name: "Ouvrir Émission conduite, dossier CEM-LAB-001" })
    );
    expect(screen.getByText("Convertisseur prototype")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Déplacer" })).not.toBeInTheDocument();
  });

  test("identifies a historical planning location without exposing its stable id", async () => {
    mockLaboratoryPlanningApi({ legacyLocation: true });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Planning du laboratoire" }));
    expect((await screen.findAllByText("Lieu à identifier")).length).toBeGreaterThan(0);
    expect(screen.queryByText("LAB-LOCATION-CEM-1")).not.toBeInTheDocument();
    await user.click(
      screen.getByRole("button", {
        name: "Ouvrir Émission conduite, dossier CEM-LAB-001"
      })
    );

    expect(screen.getByText("Libellé historique : Poste CEM historique")).toBeInTheDocument();
    expect(screen.getByText(/créé avant l’identification stable des lieux/i)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Identifier le lieu" }));

    const save = screen.getByRole("button", { name: "Enregistrer le lieu" });
    expect(save).toBeDisabled();
    await user.selectOptions(screen.getByLabelText("Lieu réel"), "LAB-LOCATION-CEM-1");
    expect(save).toBeDisabled();
    await user.type(
      screen.getByLabelText("Motif de l’identification"),
      "Vérification du plan d’implantation"
    );
    expect(save).toBeEnabled();
    await user.click(save);

    expect((await screen.findAllByText("Labo CEM 1")).length).toBeGreaterThan(0);
    expect(screen.queryByText("Libellé historique : Poste CEM historique")).not.toBeInTheDocument();
    expect(screen.queryByText("LAB-LOCATION-CEM-1")).not.toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining("/schedule-items/PLAN-LAB-001/location-identification"),
      expect.objectContaining({ method: "POST" })
    );
  });

  test("refreshes laboratory planning after concurrent historical location identification", async () => {
    mockLaboratoryPlanningApi({ legacyLocation: true, identificationConcurrency: true });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Planning du laboratoire" }));
    await user.click(
      screen.getByRole("button", {
        name: "Ouvrir Émission conduite, dossier CEM-LAB-001"
      })
    );
    await user.click(screen.getByRole("button", { name: "Identifier le lieu" }));
    await user.selectOptions(screen.getByLabelText("Lieu réel"), "LAB-LOCATION-CEM-1");
    await user.type(screen.getByLabelText("Motif de l’identification"), "Contrôle concurrent");
    await user.click(screen.getByRole("button", { name: "Enregistrer le lieu" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Ce créneau a changé sur un autre écran"
    );
    await waitFor(() =>
      expect(
        fetchMock.mock.calls.filter(([path]) => String(path).startsWith("/api/v1/service-schedule?"))
      ).toHaveLength(2)
    );
  });

  test("explains a blocking historical location reservation with booking details", async () => {
    mockLaboratoryPlanningApi({ legacyConflict: true });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Planning du laboratoire" }));
    await user.click(
      screen.getByRole("button", {
        name: "Ouvrir Émission conduite, dossier CEM-LAB-001"
      })
    );
    await user.click(screen.getByRole("button", { name: "Déplacer" }));
    await user.type(screen.getByLabelText("Raison du changement"), "Déplacement demandé");
    await user.click(screen.getByRole("button", { name: "Enregistrer le déplacement" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(
      "Un créneau existant utilise encore un lieu non identifié. Identifiez son lieu avant de réserver un autre créneau sur cette période."
    );
    expect(alert).toHaveTextContent("« Essai historique » du dossier CEM-HIST-001");
    expect(alert).toHaveTextContent("Poste CEM historique");
    expect(alert).toHaveTextContent("Confirmé");
  });

  test("requires confirmation before a planned slot can be prepared", async () => {
    mockLaboratoryPlanningApi();
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Planning du laboratoire" }));
    await user.click(
      screen.getByRole("button", {
        name: "Ouvrir Émission conduite, dossier CEM-LAB-001"
      })
    );

    expect((await screen.findAllByText("À confirmer", { exact: true })).length).toBeGreaterThan(0);
    expect(
      screen.getByText("Confirmez le créneau avant de préparer l'essai.")
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Préparer l'essai" })).not.toBeInTheDocument();
    expect(
      fetchMock.mock.calls.some(([path]) => String(path).includes("PLAN-LAB-001/preparation/options"))
    ).toBe(false);

    await user.click(screen.getByRole("button", { name: "Confirmer le créneau" }));

    expect((await screen.findAllByText("À préparer", { exact: true })).length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Préparer l'essai" })).toBeInTheDocument();
  });

  test("blocks then authorizes a confirmed slot from its preparation workflow", async () => {
    mockLaboratoryPlanningApi();
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Planning du laboratoire" }));
    await user.click(
      screen.getByRole("button", {
        name: "Ouvrir Immunité rayonnée, dossier CEM-LAB-002"
      })
    );
    await user.click(await screen.findByRole("button", { name: "Préparer l'essai" }));

    expect(await screen.findByText("Aucun contrôle enregistré")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Vérifier la préparation" }));
    expect(await screen.findByText("Préparation bloquée")).toBeInTheDocument();
    expect(screen.getAllByText("Affectation des matériels").length).toBeGreaterThan(0);

    const materialSelect = screen.getByLabelText("Matériel pour Récepteur de mesure");
    expect(within(materialSelect).queryByRole("option", { name: /SMW200A/ })).not.toBeInTheDocument();
    await user.selectOptions(materialSelect, "receiver-binding");
    await user.click(screen.getByRole("button", { name: "Vérifier la préparation" }));
    expect(await screen.findByText("Prêt à démarrer")).toBeInTheDocument();
    expect(screen.getAllByText("Contrôle n° 1").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Contrôle n° 2").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "Retour au créneau" }));
    await user.click(screen.getByRole("button", { name: "Démarrer l'essai" }));
    expect((await screen.findAllByText("En cours")).length).toBeGreaterThan(0);
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining("/preparation/assessments"),
      expect.objectContaining({ method: "POST" })
    );
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining("/transitions/start"),
      expect.objectContaining({ method: "POST" })
    );
  });

  test("keeps project and schedule context visible when preparation options fail", async () => {
    mockLaboratoryPlanningApi({ preparationOptionsFailure: true });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Planning du laboratoire" }));
    await user.click(
      await screen.findByRole("button", {
        name: "Ouvrir Immunité rayonnée, dossier CEM-LAB-002"
      })
    );
    await user.click(await screen.findByRole("button", { name: "Préparer l'essai" }));

    expect(await screen.findByText("Choix de préparation temporairement indisponibles")).toBeInTheDocument();
    expect(screen.getAllByText("CEM-LAB-002 · Mobilités Boréal").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Immunité rayonnée").length).toBeGreaterThan(0);
    expect(screen.getByText("Objet soumis à l’essai")).toBeInTheDocument();
    expect(screen.getByText("Calculateur de bord")).toBeInTheDocument();
    expect(screen.queryByText("options de préparation indisponibles", { exact: true })).not.toBeInTheDocument();
  });

  test("explains when a method role has no compatible material in the selected setup", async () => {
    mockLaboratoryPlanningApi({ noCompatibleMaterials: true });
    const user = userEvent.setup();

    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Planning du laboratoire" }));
    await user.click(
      screen.getByRole("button", {
        name: "Ouvrir Immunité rayonnée, dossier CEM-LAB-002"
      })
    );
    await user.click(await screen.findByRole("button", { name: "Préparer l'essai" }));

    expect(await screen.findByText("Aucun matériel compatible dans ce montage.")).toBeInTheDocument();
    expect(screen.getByLabelText("Matériel pour Récepteur de mesure")).toBeDisabled();
    expect(screen.getByText("Le modèle ne fournit pas la capacité spectrale requise.")).toBeInTheDocument();
  });

  test("loads templates, filters them, and opens the draft studio", async () => {
    mockBaseApi([templateFixture()]);
    const user = userEvent.setup();

    render(<App />);

    expect(await screen.findByText("Inrush current template")).toBeInTheDocument();
    await user.type(screen.getByLabelText("Rechercher une méthode"), "inrush");
    expect(screen.getByText("TT-LAB-001")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Continuer le brouillon" }));

    expect(await screen.findByText("Éditeur de méthode")).toBeInTheDocument();
    expect(screen.getByText("Non modifie")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Variables" }));
    expect(screen.getByDisplayValue("repeat_count")).toBeInTheDocument();
  });

  test("edits variables, validates, saves, submits, approves, and derives through API calls", async () => {
    mockBaseApi([templateFixture()]);
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Continuer le brouillon" }));
    await user.click(screen.getByRole("button", { name: "Variables" }));
    await user.click(screen.getByRole("button", { name: "Ajouter une variable" }));
    await user.click(screen.getByRole("button", { name: /Vérifier la définition/ }));
    expect(await screen.findByText("Définition prête à être soumise")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Sauvegarder/ }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("/definition"), expect.any(Object)));
    await user.click(screen.getByRole("button", { name: /Vérifier la définition/ }));
    await user.click(screen.getByRole("button", { name: /Soumettre/ }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("submit-for-review"), expect.any(Object)));
  });

  test("shows CAS conflict without dropping local edits", async () => {
    mockBaseApi([templateFixture()]);
    fetchMock.mockImplementationOnce(() => jsonResponse(healthFixture));
    const user = userEvent.setup();

    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Continuer le brouillon" }));
    await user.click(screen.getByLabelText("Titre technique de revision"));
    await user.keyboard(" updated");
    fetchMock.mockImplementationOnce(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path.endsWith("/definition") && init?.method === "PUT") {
        return jsonResponse(
          {
            error: {
              code: "test_template_definition_checksum_mismatch",
              message: "draft definition was modified by another operation",
              details: {
                expected_definition_checksum: canonicalChecksum("e"),
                actual_definition_checksum: canonicalChecksum("f")
              }
            }
          },
          409
        );
      }
      return mockBaseApiResponse(path, init);
    });

    await user.click(screen.getByRole("button", { name: /Sauvegarder/ }));

    expect(await screen.findByText("Conflit de sauvegarde")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Inrush current template updated")).toBeInTheDocument();
  });

  test("creates and clones through public API routes", async () => {
    mockBaseApi([templateFixture("approved")]);
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: /Créer une méthode/ }));
    expect(screen.queryByLabelText("Identifiant")).not.toBeInTheDocument();
    await user.type(screen.getByLabelText("Nom de la méthode"), "New template");
    await user.click(screen.getByRole("button", { name: "Créer le brouillon" }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith("/api/v1/test-templates", expect.objectContaining({ method: "POST" })));

    await user.click(screen.getByRole("button", { name: /Bibliothèque/ }));
    await user.click(screen.getByRole("button", { name: /Dupliquer/ }));
    await user.selectOptions(screen.getByLabelText("Méthode source"), "TT-LAB-001|TT-LAB-001-rev-0001");
    await user.type(screen.getByLabelText("Nom de la copie"), "Clone template");
    await user.click(screen.getByRole("button", { name: "Créer la copie" }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("/clone"), expect.objectContaining({ method: "POST" })));
  });

  test("opens Equipment space and displays model catalog provider status", async () => {
    mockBaseApi([templateFixture()]);
    fetchMock.mockImplementation(async (input: RequestInfo | URL) => {
      const path = String(input);
      if (path === "/api/v1/health") return jsonResponse(healthFixture);
      if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
      if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
      if (path === "/api/v1/equipment/registries") return jsonResponse(equipmentRegistriesFixture());
      if (path === "/api/v1/equipment/classification-presets") {
        return jsonResponse({ presets: [rfCablePresetFixture(), adcPresetFixture()] });
      }
      if (path === "/api/v1/equipment-models" || path.startsWith("/api/v1/equipment-models?")) {
        return jsonResponse({ equipment_models: [equipmentModelFixture()] });
      }
      if (path === "/api/v1/driver-profiles") return jsonResponse({ driver_profiles: [driverProfileFixture()] });
      if (path === "/api/v1/equipment/communication-providers") {
        return jsonResponse({
          providers: [
            { provider: "simulation", available: true },
            { provider: "visa", available: false, reason: "No VISA implementation installed" }
          ]
        });
      }
      if (path === "/api/v1/equipment-models/EQM-NRP6AN-FWD") {
        return jsonResponse({ equipment_model: equipmentModelFixture() });
      }
      if (path === "/api/v1/equipment-models/EQM-NRP6AN-FWD/revisions") {
        return jsonResponse({
          equipment_model_id: "EQM-NRP6AN-FWD",
          revisions: [equipmentModelFixture().latest_revision]
        });
      }
      if (path === "/api/v1/equipment-models/EQM-NRP6AN-FWD/audit-events") {
        return jsonResponse({ aggregate_kind: "equipment_model", entity_id: "EQM-NRP6AN-FWD", audit_events: [] });
      }
      if (path === "/api/v1/driver-profiles/DRV-NRP6AN-SCPI/revisions") {
        return jsonResponse({
          driver_profile_id: "DRV-NRP6AN-SCPI",
          revisions: [driverProfileFixture().latest_revision]
        });
      }
      if (path === "/api/v1/driver-profiles/DRV-NRP6AN-SCPI/audit-events") {
        return jsonResponse({ aggregate_kind: "driver_profile", entity_id: "DRV-NRP6AN-SCPI", audit_events: [] });
      }
      return mockBaseApiResponse(path);
    });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Catalogue des modèles" }));
    expect(await screen.findByRole("heading", { level: 1, name: "Catalogue des modèles" })).toBeInTheDocument();
    const modelButton = await screen.findByRole("treeitem", { name: /NRP6AN/ });
    await user.click(modelButton);
    expect(await screen.findByText("Vous consultez un modèle générique.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Entrées et sorties" }));
    expect(screen.getByText("Vous consultez un modèle générique.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Profils de pilotage" }));
    await user.click(await screen.findByRole("button", { name: /NRP6AN SCPI/ }));
    expect(await screen.findByText(/Aucune implémentation VISA n’est installée/)).toBeInTheDocument();
  });

  test("registers a physical asset from an approved equipment model", async () => {
    const assets: Array<Record<string, unknown>> = [];
    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/health") return jsonResponse(healthFixture);
      if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
      if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
      if (path === "/api/v1/equipment/registries") return jsonResponse(equipmentRegistriesFixture());
      if (path === "/api/v1/equipment/classification-presets") return jsonResponse({ presets: [] });
      if (path === "/api/v1/equipment-models" || path.startsWith("/api/v1/equipment-models?")) {
        return jsonResponse({ equipment_models: [equipmentModelFixture()] });
      }
      if (path === "/api/v1/driver-profiles") return jsonResponse({ driver_profiles: [] });
      if (path === "/api/v1/equipment/communication-providers") return jsonResponse({ providers: [] });
      if (path === "/api/v1/fleet/assets" && init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        if (body.inventory_code === "SA-REJECT") {
          return jsonResponse({ error: { code: "physical_asset_inventory_code_conflict", message: "Code inventaire déjà utilisé" } }, 409);
        }
        const asset = {
          asset_id: `ASSET-${body.inventory_code}`,
          inventory_code: body.inventory_code,
          serial_number: body.serial_number ?? null,
          part_number: body.part_number ?? null,
          equipment_model_id: body.equipment_model_id,
          equipment_model_revision_id: "EQM-NRP6AN-FWD-rev-0001",
          equipment_model_checksum: equipmentModelFixture().current_approved_revision?.definition_checksum,
          manufacturer: "R&S",
          model_name: "NRP6AN",
          variant: null,
          category_code: "rf_power_sensor",
          category_path: ["Mesure RF", "Wattmètres"],
          laboratory_location_id: body.laboratory_location_id,
          laboratory_location_label: body.laboratory_location_id ? "Labo CEM 1" : null,
          laboratory_location_status: body.laboratory_location_id ? "active" : null,
          ownership_source: body.ownership_source,
          service_state: body.service_state,
          administrative_availability: body.administrative_availability,
          administrative_unavailability_reason: body.administrative_unavailability_reason ?? "",
          operational_usage: {
            state: body.administrative_availability,
            assessed_at: "2026-07-14T00:00:00Z",
            evidence: []
          },
          availability_state: body.administrative_availability,
          service_state_reason: body.service_state_reason ?? "",
          notes: body.notes ?? "",
          revision: 1,
          model_link_state: "controlled",
          migrated_from_metrology: false,
          metrology: metrologySummary({
            status: body.calibration_requirement === "not_required" ? "not_required" : "missing",
            calibration_requirement: body.calibration_requirement,
            calibration_period_months: body.calibration_period_months
          }),
          created_at: "2026-07-14T00:00:00Z",
          updated_at: "2026-07-14T00:00:00Z"
        };
        assets.push(asset);
        return jsonResponse({ asset, replayed: false });
      }
      if (path === "/api/v1/fleet/assets") return jsonResponse({ assets });
      if (path === "/api/v1/metrology/instruments") return jsonResponse({ instruments: [] });
      return mockBaseApiResponse(path, init);
    });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Parc matériel" }));
    await user.click(await screen.findByRole("button", { name: "Ajouter un exemplaire" }));
    await user.type(screen.getByLabelText(/Code inventaire/), "SA-LAB-001");
    await user.type(screen.getByLabelText(/Numéro de série/), "SN-7788");
    await user.type(screen.getByLabelText(/Référence fabricant/), "PN-NRP6AN");
    await user.selectOptions(screen.getByLabelText(/Emplacement/), "LAB-LOCATION-CEM-1");
    await user.click(screen.getByRole("button", { name: "Enregistrer l'exemplaire" }));

    await waitFor(() => expect(assets).toHaveLength(1));
    const request = fetchMock.mock.calls.find(([path, options]) =>
      String(path) === "/api/v1/fleet/assets" && (options as RequestInit | undefined)?.method === "POST"
    );
    const body = JSON.parse(String((request?.[1] as RequestInit).body));
    expect(body.serial_number).toBe("SN-7788");
    expect(body.equipment_model_id).toBe("EQM-NRP6AN-FWD");
    expect(body.equipment_model_revision_id).toBeUndefined();
    expect(body.equipment_model_checksum).toBeUndefined();
    expect(body.administrative_availability).toBe("available");

    expect(screen.getByText("Vous consultez un exemplaire du parc.")).toBeInTheDocument();
    expect(screen.getByText("SN-7788")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Ajouter un exemplaire" }));
    await user.type(screen.getByLabelText(/Code inventaire/), "SA-LAB-002");
    await user.click(screen.getByRole("button", { name: "Enregistrer l'exemplaire" }));
    expect(await screen.findByText("Sans numéro de série")).toBeInTheDocument();
    expect(screen.getAllByText("Emplacement non défini").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "Ajouter un exemplaire" }));
    await user.type(screen.getByLabelText(/Code inventaire/), "SA-LAB-003");
    await user.selectOptions(screen.getByLabelText(/État de service/), "in_maintenance");
    const serviceReason = screen.getByLabelText(/Motif de l'état de service/);
    const administrativeAvailability = screen.getByLabelText(/Disponibilité administrative/);
    expect(administrativeAvailability).toHaveValue("unavailable");
    expect(administrativeAvailability).toBeDisabled();
    const saveUnavailable = screen.getByRole("button", { name: "Enregistrer l'exemplaire" });
    expect(saveUnavailable).toHaveAttribute("aria-disabled", "true");
    await user.click(saveUnavailable);
    expect(serviceReason).toHaveFocus();
    await user.type(serviceReason, "Maintenance préventive");
    await user.click(saveUnavailable);
    await waitFor(() => expect(assets).toHaveLength(3));
    const createRequests = fetchMock.mock.calls.filter(([path, options]) =>
      String(path) === "/api/v1/fleet/assets" && (options as RequestInit | undefined)?.method === "POST"
    );
    const unavailableBody = JSON.parse(String((createRequests.at(-1)?.[1] as RequestInit).body));
    expect(unavailableBody.service_state).toBe("in_maintenance");
    expect(unavailableBody.service_state_reason).toBe("Maintenance préventive");
    expect(unavailableBody.administrative_availability).toBe("unavailable");
    expect(unavailableBody.administrative_unavailability_reason).toBe("Maintenance préventive");
    expect(unavailableBody.laboratory_location_id).toBeUndefined();

    await user.click(screen.getByRole("button", { name: "Ajouter un exemplaire" }));
    await user.type(screen.getByLabelText(/Code inventaire/), "SA-REJECT");
    await user.type(screen.getByLabelText(/Numéro de série/), "SN-A-CONSERVER");
    await user.click(screen.getByRole("button", { name: "Enregistrer l'exemplaire" }));
    expect(await screen.findByText("Code inventaire déjà utilisé")).toBeInTheDocument();
    expect(screen.getByLabelText(/Code inventaire/)).toHaveValue("SA-REJECT");
    expect(screen.getByLabelText(/Numéro de série/)).toHaveValue("SN-A-CONSERVER");
    expect(assets).toHaveLength(3);
  });

  test("keeps catalogue and fleet identity visible when secondary services fail", async () => {
    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/equipment/registries") return jsonResponse(equipmentRegistriesFixture());
      if (path === "/api/v1/equipment/classification-presets") return jsonResponse({ presets: [] });
      if (path === "/api/v1/equipment-models" || path.startsWith("/api/v1/equipment-models?")) {
        return jsonResponse({ equipment_models: [equipmentModelFixture()] });
      }
      if (path === "/api/v1/fleet/assets") return jsonResponse({ assets: [physicalAssetFixture()] });
      if (path === "/api/v1/driver-profiles") {
        return jsonResponse({ error: { code: "drivers_unavailable", message: "service de pilotage indisponible" } }, 503);
      }
      if (path === "/api/v1/metrology/instruments") {
        return jsonResponse({ error: { code: "metrology_unavailable", message: "service métrologique indisponible" } }, 503);
      }
      if (path === "/api/v1/equipment/communication-providers") return jsonResponse({ providers: [] });
      if (path === "/api/v1/equipment-models/EQM-NRP6AN-FWD") {
        return jsonResponse({ equipment_model: equipmentModelFixture() });
      }
      if (path === "/api/v1/equipment-models/EQM-NRP6AN-FWD/revisions") {
        return jsonResponse({ error: { code: "revisions_unavailable", message: "versions temporairement indisponibles" } }, 503);
      }
      if (path === "/api/v1/equipment-models/EQM-NRP6AN-FWD/audit-events") {
        return jsonResponse({ error: { code: "audit_unavailable", message: "journal temporairement indisponible" } }, 503);
      }
      return mockBaseApiResponse(path, init);
    });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Catalogue des modèles" }));
    expect(await screen.findByText("Pilotage temporairement indisponible")).toBeInTheDocument();
    expect(screen.getByRole("treeitem", { name: /NRP6AN/ })).toBeInTheDocument();
    expect(screen.getByText(/Le catalogue reste consultable\./)).toBeInTheDocument();
    await user.click(screen.getByRole("treeitem", { name: /NRP6AN/ }));
    expect(await screen.findByText("Vous consultez un modèle générique.")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "R&S NRP6AN" })).toBeInTheDocument();
    expect(await screen.findByText("Historique des versions indisponible")).toBeInTheDocument();
    expect(await screen.findByText("Historique des modifications indisponible")).toBeInTheDocument();
    expect(screen.getByText(/La fiche du modèle reste consultable\./)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Parc matériel" }));
    expect(await screen.findByText("Métrologie temporairement indisponible")).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "INV-0042" })).toBeInTheDocument();
    expect(screen.getByText(/L’identité et la disponibilité des exemplaires restent consultables\./)).toBeInTheDocument();
  });

  test("renders the fleet hierarchy and opens the exact pinned model revision", async () => {
    const first = physicalAssetFixture();
    const second = physicalAssetFixture({
      asset_id: "ASSET-NRP6AN-002",
      inventory_code: "INV-0043",
      serial_number: null
    });
    fetchMock.mockImplementation(async (input: RequestInfo | URL) => {
      const path = String(input);
      if (path === "/api/v1/fleet/assets") return jsonResponse({ assets: [first, second] });
      if (path === "/api/v1/laboratory-locations") return mockBaseApiResponse(path);
      return jsonResponse({ error: { code: "unexpected", message: path } }, 500);
    });
    const openPinnedModel = vi.fn();
    const user = userEvent.setup();

    render(<FleetWorkspace
      models={[equipmentModelFixture() as EquipmentModelAggregate]}
      categories={equipmentCategoriesFixture()}
      onOpenPinnedModel={openPinnedModel}
      onOpenMetrology={vi.fn()}
    />);

    expect(await screen.findByRole("treeitem", { name: /Mesure RF/ })).toBeInTheDocument();
    expect(screen.getByRole("treeitem", { name: /Wattmètres/ })).toBeInTheDocument();
    expect(screen.getByRole("treeitem", { name: /R&S NRP6AN/ })).toBeInTheDocument();
    await user.click(screen.getByRole("treeitem", { name: /INV-0043/ }));
    expect(screen.getAllByText("Sans numéro de série").length).toBeGreaterThan(0);
    expect(screen.getByText("Vous consultez un exemplaire du parc.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Ouvrir le modèle constructeur" }));
    expect(openPinnedModel).toHaveBeenCalledWith("EQM-NRP6AN-FWD", "EQM-NRP6AN-FWD-rev-0001");
    await user.click(screen.getByRole("button", { name: "Par emplacement" }));
    expect(screen.getAllByRole("treeitem", { name: /Labo CEM 1/ }).some((item) => item.classList.contains("fleetGroup"))).toBe(true);
    await user.click(screen.getByRole("button", { name: "Historique" }));
    expect(await screen.findByText("Historique temporairement indisponible")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "INV-0043" })).toBeInTheDocument();
  });

  test("renders real fleet audit events without coupling asset identity to audit availability", async () => {
    const asset = physicalAssetFixture();
    const auditEvents = [
      {
        sequence: 1,
        action: "physical_asset_identification_updated",
        actor: "fleet.operator",
        reason: "Alignement avec l'étiquette du laboratoire",
        old_revision: 1,
        new_revision: 2,
        operation_id: "op-identification-1",
        device_id: "lab-console",
        correlation_id: "corr-identification-1",
        payload: {
          from: { inventory_code: "INV-0041", serial_number: "SN-0042" },
          to: { inventory_code: "INV-0042", serial_number: "SN-0042" }
        },
        occurred_at: "2026-07-27T09:15:00Z"
      },
      {
        sequence: 2,
        action: "physical_asset_moved",
        actor: "Opérateur CEM",
        reason: "Affectation au poste conduit",
        old_revision: 2,
        new_revision: 3,
        operation_id: "op-move-1",
        device_id: "lab-console",
        correlation_id: "corr-move-1",
        payload: {
          from: { laboratory_location_label: "Magasin métrologie" },
          to: { laboratory_location_label: "Labo CEM 1" }
        },
        occurred_at: "2026-07-27T10:30:00Z"
      }
    ];
    fetchMock.mockImplementation(async (input: RequestInfo | URL) => {
      const path = String(input);
      if (path === "/api/v1/fleet/assets") return jsonResponse({ assets: [asset] });
      if (path === `/api/v1/fleet/assets/${asset.asset_id}/audit-events`) {
        return jsonResponse({ entity_id: asset.asset_id, audit_events: auditEvents });
      }
      return mockBaseApiResponse(path);
    });
    const user = userEvent.setup();

    render(<FleetWorkspace
      models={[equipmentModelFixture() as EquipmentModelAggregate]}
      categories={equipmentCategoriesFixture()}
      onOpenPinnedModel={vi.fn()}
      onOpenMetrology={vi.fn()}
    />);

    expect(await screen.findByRole("heading", { name: "INV-0042" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Historique" }));
    expect(await screen.findByRole("heading", { name: "Déplacement de l’exemplaire" })).toBeInTheDocument();
    expect(screen.getByText("Affectation au poste conduit", { exact: false })).toBeInTheDocument();
    expect(screen.getByText("Emplacement : Magasin métrologie → Labo CEM 1.")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Modification du code inventaire et de l’identification" })).toBeInTheDocument();
    expect(screen.getByText("Code inventaire : INV-0041 → INV-0042.")).toBeInTheDocument();
    expect(screen.getByText("Opérateur du parc", { exact: false })).toBeInTheDocument();
    expect(screen.getAllByText("Détails techniques").filter((element) => element.tagName === "SUMMARY")).toHaveLength(2);
  });

  test("separates administrative availability from computed operational usage", async () => {
    let asset = physicalAssetFixture({
      availability_state: "reserved",
      operational_usage: {
        state: "reserved",
        assessed_at: "2026-07-27T10:00:00Z",
        evidence: [{
          source_kind: "planned_test_reservation",
          source_identifier: "PLAN-CEM-042",
          source_label: "Essai d'immunité conduit",
          relevant_start_at: "2026-07-27T09:00:00Z",
          relevant_end_at: "2026-07-27T12:00:00Z",
          reason: "Réservé pour un essai planifié",
          blocks_selection: true
        }]
      }
    }) as PhysicalAsset;
    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/fleet/assets") return jsonResponse({ assets: [asset] });
      if (path === "/api/v1/laboratory-locations") return mockBaseApiResponse(path);
      if (path.endsWith("/transitions/administrative-availability") && init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        asset = {
          ...asset,
          administrative_availability: body.administrative_availability,
          administrative_unavailability_reason: body.administrative_unavailability_reason,
          availability_state: "unavailable",
          operational_usage: {
            state: "unavailable",
            assessed_at: "2026-07-27T10:00:00Z",
            evidence: [{
              source_kind: "administrative_availability",
              source_identifier: asset.asset_id,
              source_label: "Disponibilité administrative",
              relevant_start_at: null,
              relevant_end_at: null,
              reason: body.administrative_unavailability_reason,
              blocks_selection: true
            }]
          },
          revision: Number(asset.revision) + 1
        };
        return jsonResponse({ asset, replayed: false });
      }
      return jsonResponse({ error: { code: "unexpected", message: path } }, 500);
    });
    const user = userEvent.setup();

    render(<FleetWorkspace
      models={[equipmentModelFixture() as EquipmentModelAggregate]}
      categories={equipmentCategoriesFixture()}
      onOpenPinnedModel={vi.fn()}
      onOpenMetrology={vi.fn()}
    />);

    expect(await screen.findByText("Essai d'immunité conduit")).toBeInTheDocument();
    expect(screen.getByText("Réservé pour un essai planifié")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Emplacement et disponibilité" }));
    const availabilitySelect = screen.getByLabelText(/Disponibilité administrative/);
    expect(within(availabilitySelect).getAllByRole("option").map((option) => option.textContent)).toEqual([
      "Disponible",
      "Indisponible"
    ]);
    expect(within(availabilitySelect).queryByRole("option", { name: "Réservé" })).not.toBeInTheDocument();
    expect(within(availabilitySelect).queryByRole("option", { name: "Utilisé en essai" })).not.toBeInTheDocument();

    await user.selectOptions(availabilitySelect, "unavailable");
    const save = screen.getByRole("button", { name: "Enregistrer la disponibilité administrative" });
    expect(save).toBeDisabled();
    await user.type(screen.getByLabelText(/Motif d'indisponibilité/), "Prêt externe");
    expect(save).toBeEnabled();
    await user.click(save);

    await waitFor(() => expect(fetchMock.mock.calls.some(([path]) =>
      String(path).endsWith("/transitions/administrative-availability")
    )).toBe(true));
    expect(await screen.findByLabelText(/Motif d'indisponibilité/)).toHaveValue("Prêt externe");
  });

  test("edits identity at an archived location and moves through a dedicated command", async () => {
    let asset = physicalAssetFixture({
      laboratory_location_id: "LAB-LOCATION-ARCHIVED",
      laboratory_location_label: "Ancien laboratoire",
      laboratory_location_status: "archived"
    }) as PhysicalAsset;
    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/fleet/assets") return jsonResponse({ assets: [asset] });
      if (path === "/api/v1/laboratory-locations") return mockBaseApiResponse(path);
      if (path.endsWith("/identification") && init?.method === "PUT") {
        const body = JSON.parse(String(init.body));
        expect(body.laboratory_location_id).toBeUndefined();
        asset = { ...asset, ...body, revision: asset.revision + 1 };
        return jsonResponse({ asset, replayed: false });
      }
      if (path.endsWith("/transitions/move") && init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        asset = {
          ...asset,
          laboratory_location_id: body.destination_location_id,
          laboratory_location_label: "Chambre semi-anéchoïque",
          laboratory_location_status: "active",
          revision: asset.revision + 1
        };
        return jsonResponse({ asset, replayed: false });
      }
      return jsonResponse({ error: { code: "unexpected", message: path } }, 500);
    });
    const user = userEvent.setup();

    render(<FleetWorkspace
      models={[equipmentModelFixture() as EquipmentModelAggregate]}
      categories={equipmentCategoriesFixture()}
      onOpenPinnedModel={vi.fn()}
      onOpenMetrology={vi.fn()}
    />);

    expect((await screen.findAllByText("Ancien laboratoire (archivé)")).length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: "Identification" }));
    const notes = screen.getByLabelText("Notes");
    await user.clear(notes);
    await user.type(notes, "Inventaire vérifié sur place");
    await user.click(screen.getByRole("button", { name: "Enregistrer l'identification" }));
    await waitFor(() => expect(fetchMock.mock.calls.some(([path]) => String(path).endsWith("/identification"))).toBe(true));
    expect(screen.getAllByText("Ancien laboratoire (archivé)").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "Emplacement et disponibilité" }));
    expect(await screen.findByText(/Emplacement actuel archivé/)).toBeInTheDocument();
    const location = screen.getByLabelText("Nouvel emplacement");
    expect(within(location).getByRole("option", { name: /Ancien laboratoire.*archivé/ })).toBeDisabled();
    await user.selectOptions(location, "LAB-LOCATION-ANECHOIC");
    await user.type(screen.getByLabelText(/Motif du déplacement/), "Transfert vers la chambre rayonnée");
    await user.click(screen.getByRole("button", { name: "Enregistrer le déplacement" }));

    await waitFor(() => expect(fetchMock.mock.calls.some(([path]) => String(path).endsWith("/transitions/move"))).toBe(true));
    expect((await screen.findAllByText("Chambre semi-anéchoïque")).length).toBeGreaterThan(0);
    const moveRequest = fetchMock.mock.calls.find(([path]) => String(path).endsWith("/transitions/move"));
    const moveBody = JSON.parse(String((moveRequest?.[1] as RequestInit).body));
    expect(moveBody.destination_location_id).toBe("LAB-LOCATION-ANECHOIC");
    expect(moveBody.reason).toBe("Transfert vers la chambre rayonnée");
  });

  test("reconciles a migrated asset through a readable exact model revision", async () => {
    const unresolved = physicalAssetFixture({
      asset_id: "LEGACY-SCOPE-001",
      inventory_code: "LEGACY-SCOPE-001",
      equipment_model_id: null,
      equipment_model_revision_id: null,
      equipment_model_checksum: null,
      manufacturer: "Ancien fabricant",
      model_name: "Modèle à rapprocher",
      variant: null,
      category_code: "legacy_metrology",
      category_path: ["Ancien registre"],
      revision: 1,
      model_link_state: "migration_review_required",
      migrated_from_metrology: true
    });
    const candidate = {
      equipment_model_id: "EQM-NRP6AN-FWD",
      equipment_model_revision_id: "EQM-NRP6AN-FWD-rev-0001",
      revision_number: 1,
      lifecycle_status: "superseded",
      approved_at: "2026-07-03T00:00:00Z",
      manufacturer: "R&S",
      model_name: "NRP6AN",
      variant: "FWD",
      category_path: ["Mesure RF", "Wattmètres"]
    };
    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/fleet/assets") return jsonResponse({ assets: [unresolved] });
      if (path === "/api/v1/laboratory-locations") return mockBaseApiResponse(path);
      if (path === "/api/v1/fleet/model-reconciliation-candidates") {
        return jsonResponse({ candidates: [candidate] });
      }
      if (path.endsWith("/transitions/reconcile-model") && init?.method === "POST") {
        return jsonResponse({
          replayed: false,
          asset: {
            ...unresolved,
            equipment_model_id: candidate.equipment_model_id,
            equipment_model_revision_id: candidate.equipment_model_revision_id,
            equipment_model_checksum: canonicalChecksum("a"),
            manufacturer: candidate.manufacturer,
            model_name: candidate.model_name,
            variant: candidate.variant,
            category_code: "power_meter",
            category_path: candidate.category_path,
            revision: 2,
            model_link_state: "resolved"
          }
        });
      }
      return jsonResponse({ error: { code: "unexpected", message: path } }, 500);
    });
    const user = userEvent.setup();

    render(<FleetWorkspace
      models={[equipmentModelFixture() as EquipmentModelAggregate]}
      categories={equipmentCategoriesFixture()}
      onOpenPinnedModel={vi.fn()}
      onOpenMetrology={vi.fn()}
    />);

    expect(await screen.findByRole("heading", { name: "Modèle constructeur à rapprocher" })).toBeInTheDocument();
    expect(screen.getByText(/provient de l’ancien registre métrologique/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Rapprocher avec un modèle constructeur" }));
    expect(await screen.findByRole("option", { name: /R&S NRP6AN FWD.*Mesure RF > Wattmètres.*Version 1.*Remplacée/ })).toBeInTheDocument();
    expect(screen.queryByText(candidate.equipment_model_id)).not.toBeInTheDocument();
    expect(screen.queryByText(candidate.equipment_model_revision_id)).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Rapprocher cette version" }));

    expect(await screen.findByRole("button", { name: "Ouvrir le modèle constructeur" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Modèle constructeur à rapprocher" })).not.toBeInTheDocument();
    const request = fetchMock.mock.calls.find(([path]) => String(path).endsWith("/transitions/reconcile-model"));
    const body = JSON.parse(String((request?.[1] as RequestInit).body));
    expect(body.expected_revision).toBe(1);
    expect(body.equipment_model_id).toBe(candidate.equipment_model_id);
    expect(body.equipment_model_revision_id).toBe(candidate.equipment_model_revision_id);
    expect(body.equipment_model_checksum).toBeUndefined();
  });

  test("uses only physical fleet assets in a station setup and isolates fleet failure", async () => {
    const setup = stationSetupFixture();
    let failFleet = false;
    fetchMock.mockImplementation(async (input: RequestInfo | URL) => {
      const path = String(input);
      if (path === "/api/v1/station-setups") return jsonResponse({ station_setups: [setup] });
      if (path.startsWith("/api/v1/station-setups/asset-options?")) {
        return failFleet
          ? jsonResponse({ error: { code: "fleet_unavailable", message: "parc indisponible" } }, 503)
          : jsonResponse({
            assessed_at: "2026-07-30T12:00:00Z",
            checked_on: "2026-07-30",
            execution_mode: "accredited",
            laboratory_location_id: "LAB-LOCATION-CEM-1",
            assets: [
              { asset: physicalAssetFixture(), eligible: true, blocking_reasons: [], warnings: [] },
              {
                asset: physicalAssetFixture({
                  asset_id: "ASSET-RESTRICTED-001",
                  inventory_code: "INV-RESTRICTED",
                  serial_number: null,
                  service_state: "restricted"
                }),
                eligible: true,
                blocking_reasons: [],
                warnings: [{
                  code: "service_restricted",
                  message: "Cet exemplaire comporte une restriction d'utilisation.",
                  next_action: "Consultez son dossier avant de confirmer son utilisation."
                }]
              },
              {
                asset: physicalAssetFixture({
                  asset_id: "ASSET-OOS-001",
                  inventory_code: "INV-OOS",
                  serial_number: null,
                  service_state: "out_of_service",
                  availability_state: "unavailable"
                }),
                eligible: false,
                blocking_reasons: [{
                  code: "service_out_of_service",
                  message: "Cet exemplaire est hors service.",
                  next_action: "Faites rétablir son état de service ou choisissez un autre exemplaire."
                }],
                warnings: []
              }
            ]
          });
      }
      if (path === "/api/v1/laboratory-locations") return mockBaseApiResponse(path);
      return jsonResponse({ error: { code: "unexpected", message: path } }, 500);
    });
    const user = userEvent.setup();
    const view = render(<StationSetupWorkspace />);

    expect(await screen.findByRole("heading", { name: "Chaîne d'émissions conduites" })).toBeInTheDocument();
    expect(screen.queryByText(/pré-vol/i)).not.toBeInTheDocument();
    const selector = screen.getByLabelText(/Exemplaire du parc/);
    const assetOption = await within(selector).findByRole("option", { name: /INV-0042.*SN 103456.*Labo CEM 1.*Utilisable.*Disponible.*Étalonnage valide/ });
    expect(assetOption).toBeInTheDocument();
    expect(within(selector).getByRole("option", { name: /INV-OOS.*hors service/ })).toBeDisabled();
    expect(screen.getByText("Matériels non disponibles (1)")).toBeInTheDocument();
    await user.selectOptions(selector, "ASSET-RESTRICTED-001");
    expect(screen.getByText("Cet exemplaire comporte une restriction d'utilisation.")).toBeInTheDocument();
    expect(screen.queryByText("EQM-NRP6AN-FWD-rev-0001")).not.toBeInTheDocument();
    await user.type(screen.getByLabelText(/Rôle dans le montage/), "Récepteur EMI");
    await user.selectOptions(selector, "ASSET-NRP6AN-001");
    await user.click(screen.getByRole("button", { name: "Affecter au montage" }));
    expect(screen.getByText(/INV-0042.*SN 103456/)).toBeInTheDocument();

    failFleet = true;
    view.unmount();
    render(<StationSetupWorkspace />);
    expect(await screen.findByRole("heading", { name: "Chaîne d'émissions conduites" })).toBeInTheDocument();
    expect(await screen.findByText("Aptitude du parc temporairement indisponible")).toBeInTheDocument();
    expect(screen.getByText(/Labo CEM 1.*utilisation prévue/)).toBeInTheDocument();
  });

  test("invalidates stale station asset options when the use context changes", async () => {
    const setup = stationSetupFixture();
    const locationAAsset = physicalAssetFixture({
      asset_id: "ASSET-LOCATION-A",
      inventory_code: "INV-LOCATION-A"
    });
    const locationBAsset = physicalAssetFixture({
      asset_id: "ASSET-LOCATION-B",
      inventory_code: "INV-LOCATION-B",
      laboratory_location_id: "LAB-LOCATION-ANECHOIC",
      laboratory_location_label: "Chambre semi-anéchoïque"
    });
    let resolveLocationA!: (response: Response) => void;
    let resolveLocationB!: (response: Response) => void;
    const locationAResponse = new Promise<Response>((resolve) => { resolveLocationA = resolve; });
    const locationBResponse = new Promise<Response>((resolve) => { resolveLocationB = resolve; });
    let locationBRequests = 0;

    fetchMock.mockImplementation(async (input: RequestInfo | URL) => {
      const path = String(input);
      if (path === "/api/v1/station-setups") return jsonResponse({ station_setups: [setup] });
      if (path === "/api/v1/laboratory-locations") return mockBaseApiResponse(path);
      if (path.startsWith("/api/v1/station-setups/asset-options?")) {
        const query = new URL(path, "http://localhost").searchParams;
        if (query.get("laboratory_location_id") === "LAB-LOCATION-CEM-1") {
          return locationAResponse;
        }
        locationBRequests += 1;
        if (locationBRequests === 1) return locationBResponse;
        return jsonResponse({ error: { code: "fleet_unavailable", message: "parc indisponible" } }, 503);
      }
      return jsonResponse({ error: { code: "unexpected", message: path } }, 500);
    });
    const user = userEvent.setup();
    render(<StationSetupWorkspace />);

    expect(await screen.findByRole("heading", { name: "Chaîne d'émissions conduites" })).toBeInTheDocument();
    expect(await screen.findByRole("status")).toHaveTextContent(/Actualisation des exemplaires/);
    const locationSelector = screen.getByLabelText(/Lieu du laboratoire/);
    const assetSelector = screen.getByLabelText(/Exemplaire du parc/);
    const assignButton = screen.getByRole("button", { name: "Affecter au montage" });

    await user.selectOptions(locationSelector, "LAB-LOCATION-ANECHOIC");
    expect(assetSelector).toBeDisabled();
    expect(assignButton).toBeDisabled();
    expect(within(assetSelector).queryByRole("option", { name: /INV-LOCATION-A/ })).not.toBeInTheDocument();
    await waitFor(() => expect(fetchMock.mock.calls.some(([request]) =>
      String(request).includes("laboratory_location_id=LAB-LOCATION-ANECHOIC")
    )).toBe(true));

    resolveLocationA(stationAssetOptionsResponse("LAB-LOCATION-CEM-1", locationAAsset));
    await waitFor(() => expect(within(assetSelector).queryByRole("option", { name: /INV-LOCATION-A/ })).not.toBeInTheDocument());
    expect(assetSelector).toBeDisabled();

    resolveLocationB(stationAssetOptionsResponse("LAB-LOCATION-ANECHOIC", locationBAsset));
    const locationBOption = await within(assetSelector).findByRole("option", { name: /INV-LOCATION-B/ });
    expect(locationBOption).toBeEnabled();
    await user.type(screen.getByLabelText(/Rôle dans le montage/), "Récepteur EMI");
    await user.selectOptions(assetSelector, "ASSET-LOCATION-B");
    expect(assignButton).toBeEnabled();
    await user.click(assignButton);
    expect(screen.getByText(/INV-LOCATION-B.*Chambre semi-anéchoïque/)).toBeInTheDocument();

    await user.selectOptions(locationSelector, "");
    expect(assetSelector).toBeDisabled();
    expect(within(assetSelector).queryByRole("option", { name: /INV-LOCATION-B/ })).not.toBeInTheDocument();
    expect(screen.getByText(/Renseignez le lieu et la date d'utilisation/)).toBeInTheDocument();
    expect(screen.getByText(/INV-LOCATION-B.*Chambre semi-anéchoïque/)).toBeInTheDocument();

    await user.selectOptions(locationSelector, "LAB-LOCATION-ANECHOIC");
    expect(await screen.findByText("Aptitude du parc temporairement indisponible")).toBeInTheDocument();
    expect(assignButton).toBeDisabled();
    expect(screen.getByRole("heading", { name: "Chaîne d'émissions conduites" })).toBeInTheDocument();
    expect(screen.getByText(/INV-LOCATION-B.*Chambre semi-anéchoïque/)).toBeInTheDocument();
  });

  test("records and displays a frequency response for a physical asset", async () => {
    const instrument = metrologyInstrumentFixture();
    const characterizations: Array<Record<string, unknown>> = [];
    const collectionPath = `/api/v1/metrology/instruments/${instrument.asset_id}/characterizations`;
    const correctionPath = `/api/v1/metrology/instruments/${instrument.asset_id}/corrections`;
    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/health") return jsonResponse(healthFixture);
      if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
      if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
      if (path === "/api/v1/equipment/registries") return jsonResponse(equipmentRegistriesFixture());
      if (path === "/api/v1/equipment/classification-presets") return jsonResponse({ presets: [] });
      if (path === "/api/v1/equipment-models" || path.startsWith("/api/v1/equipment-models?")) {
        return jsonResponse({ equipment_models: [equipmentModelFixture()] });
      }
      if (path === "/api/v1/driver-profiles") return jsonResponse({ driver_profiles: [] });
      if (path === "/api/v1/equipment/communication-providers") return jsonResponse({ providers: [] });
      if (path === "/api/v1/metrology/instruments") return jsonResponse({ instruments: [instrument] });
      if (path === "/api/v1/metrology/corrections/review-queue") return jsonResponse({ assignments: [] });
      if (path === correctionPath) return jsonResponse({ assignments: [] });
      if (path === `${correctionPath}/resolve`) return jsonResponse({
        asset_id: instrument.asset_id,
        equipment_model_id: instrument.equipment_model_id,
        equipment_model_revision_id: instrument.equipment_model_revision_id,
        equipment_model_checksum: instrument.equipment_model_checksum,
        report: {
          asset_id: instrument.asset_id,
          intended_use_on: "2026-07-14",
          execution_context: "accredited",
          ready: true,
          resolutions: []
        }
      });
      if (path === collectionPath && init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        const characterization = {
          characterization_id: body.characterization_id,
          asset_id: instrument.asset_id,
          characterization_kind: body.definition.correction.correction_kind,
          label: body.definition.label,
          performed_on: body.performed_on,
          valid_from: body.valid_from ?? body.performed_on,
          valid_until: body.valid_until,
          source_kind: body.source_kind ?? "characterization",
          provider: body.provider,
          method_reference: body.method_reference,
          decision: body.decision,
          definition_schema_version: body.definition.definition_schema_version,
          definition: body.definition,
          definition_checksum: `sha256:${"c".repeat(64)}`,
          certificate_reference: body.certificate_reference ?? null,
          document_manifest: body.document_manifest ?? null,
          comment: body.comment ?? "",
          recorded_at: "2026-07-14T20:00:00Z",
          recorded_by: body.recorded_by,
          revision: "rev-0001",
          environmental_conditions: body.environmental_conditions ?? {},
          as_found: body.as_found ?? null,
          as_left: body.as_left ?? null,
          adjustment_performed: body.adjustment_performed ?? false
        };
        characterizations.push(characterization);
        return jsonResponse({ characterization });
      }
      if (path === collectionPath) {
        return jsonResponse({ asset_id: instrument.asset_id, characterizations });
      }
      if (path.startsWith(`${collectionPath}/`) && path.endsWith("/audit-events")) {
        return jsonResponse({ audit_events: [] });
      }
      return mockBaseApiResponse(path, init);
    });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Métrologie du parc" }));
    expect(await screen.findByRole("heading", { name: instrument.inventory_code })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Ajouter une caractérisation" }));
    expect(await screen.findByRole("heading", { name: "Ajouter une caractérisation" })).toBeInTheDocument();
    await user.type(screen.getByLabelText(/Laboratoire ou prestataire/), "Laboratoire interne");
    await user.type(screen.getByLabelText(/Méthode utilisée/), "MET-RF-CABLE-001");
    await user.clear(screen.getByLabelText(/Tableau mesuré/));
    await user.type(
      screen.getByLabelText(/Tableau mesuré/),
      "frequence_hz,amplitude_db\n1000000,0.15\n1000000000,2.8"
    );
    await user.click(screen.getByRole("button", { name: "Enregistrer puis préparer la revue" }));

    expect(await screen.findByRole("heading", { name: "Pertes mesurées" })).toBeInTheDocument();
    expect(screen.getByText("2 points, de 1 MHz à 1 GHz")).toBeInTheDocument();
    const request = fetchMock.mock.calls.find(([path, options]) =>
      String(path) === collectionPath && (options as RequestInit | undefined)?.method === "POST"
    );
    const body = JSON.parse(String((request?.[1] as RequestInit).body));
    expect(body.definition.correction.correction_kind).toBe("frequency_response");
    expect(body.definition.correction.correction.points).toHaveLength(2);
    expect(body.provider).toBe("Laboratoire interne");
  });

  test("filters equipment catalog and creates a model from a category template", async () => {
    const createdModel = rfCableModelFixture();
    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/health") return jsonResponse(healthFixture);
      if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
      if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
      if (path === "/api/v1/equipment/registries") return jsonResponse(equipmentRegistriesFixture());
      if (path === "/api/v1/equipment/classification-presets") {
        return jsonResponse({ presets: [rfCablePresetFixture(), adcPresetFixture()] });
      }
      if (path === "/api/v1/equipment-models" || path.startsWith("/api/v1/equipment-models?")) {
        return jsonResponse({ equipment_models: [equipmentModelFixture()] });
      }
      if (path === "/api/v1/equipment-models/from-category-template" && init?.method === "POST") {
        return jsonResponse({
          operation: "equipment_model_created_from_category_template",
          operation_id: "op-from-category-template",
          replayed: false,
          aggregate: createdModel,
          revision: createdModel.latest_revision
        });
      }
      if (path === "/api/v1/equipment-models/EQM-RF-CABLE-DEMO") {
        return jsonResponse({ equipment_model: createdModel });
      }
      if (path === "/api/v1/equipment-models/EQM-RF-CABLE-DEMO/revisions") {
        return jsonResponse({
          equipment_model_id: "EQM-RF-CABLE-DEMO",
          revisions: [createdModel.latest_revision]
        });
      }
      if (path === "/api/v1/equipment-models/EQM-RF-CABLE-DEMO/audit-events") {
        return jsonResponse({ aggregate_kind: "equipment_model", entity_id: "EQM-RF-CABLE-DEMO", audit_events: [] });
      }
      if (path === "/api/v1/equipment-model-definitions/validate" && init?.method === "POST") {
        return jsonResponse({ valid: true, issues: [], definition_checksum: canonicalChecksum("c") });
      }
      if (path === "/api/v1/driver-profiles") return jsonResponse({ driver_profiles: [] });
      if (path === "/api/v1/equipment/communication-providers") {
        return jsonResponse({ providers: [{ provider: "simulation", available: true }] });
      }
      return mockBaseApiResponse(path, init);
    });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Catalogue des modèles" }));
    await screen.findByRole("heading", { level: 1, name: "Catalogue des modèles" });
    await user.selectOptions(screen.getByLabelText("Filtrer par famille"), "rf_equipment");
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("root_category_id=rf_equipment"),
        expect.any(Object)
      )
    );

    await user.click(screen.getByRole("button", { name: /Nouveau modèle/ }));
    const creationPanel = (await screen.findByText("Nouveau modèle constructeur")).closest(".creationPanel");
    expect(creationPanel).not.toBeNull();
    const wizard = within(creationPanel as HTMLElement);
    expect(wizard.queryByRole("button", { name: /radiofr/i })).not.toBeInTheDocument();
    await user.click(wizard.getByLabelText(/radiofr/i));
    await user.click(wizard.getByRole("button", { name: "Continuer" }));
    await user.click(wizard.getByText(/Câbles RF/));
    await user.click(wizard.getByRole("button", { name: "Continuer" }));
    await waitFor(() => expect(wizard.getByLabelText(/Fabricant/)).toBeInTheDocument());
    await user.type(wizard.getByLabelText(/Fabricant/), "Demo");
    await user.type(wizard.getByLabelText(/Modèle/), "RF Cable");
    await user.type(wizard.getByLabelText(/Connecteur A/), "N");
    await user.type(wizard.getByLabelText(/Connecteur B/), "N");
    await user.click(wizard.getByRole("button", { name: "Continuer" }));
    expect(wizard.queryByLabelText(/identifiant interne/i)).not.toBeInTheDocument();
    await user.click(wizard.getByRole("button", { name: /Créer le brouillon/ }));
    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        "/api/v1/equipment-models/from-category-template",
        expect.objectContaining({ method: "POST" })
      )
    );

    expect(await screen.findByText("Vous consultez un modèle générique.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Catégorie et champs" }));
    expect(screen.getByText("Formulaire utilisé")).toBeInTheDocument();
    expect(screen.queryByText("Template checksum")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Entrées et sorties" }));
    expect(screen.getByDisplayValue("RF_A")).toBeInTheDocument();
    expect(screen.getByDisplayValue("RF_B")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Vérifier la fiche/ }));
    expect(await screen.findByText("Définition prête à être soumise")).toBeInTheDocument();
  });

  test("administers nested categories with contextual actions and generated field codes", async () => {
    let categories = equipmentCategoriesFixture();
    let fields = equipmentFieldDefinitionsFixture();
    const rulesByCategory: Record<string, Array<{
      category_id: string;
      field_id: string;
      required: boolean;
      visible: boolean;
      display_group: string;
      display_order: number;
      default_value?: unknown;
      help_text_override?: string | null;
    }>> = {
      rf_equipment: [
        { category_id: "rf_equipment", field_id: "field_manufacturer", required: true, visible: true, display_group: "Identification", display_order: 10 },
        { category_id: "rf_equipment", field_id: "field_model_name", required: true, visible: true, display_group: "Identification", display_order: 20 }
      ],
      rf_amplifier: []
    };

    function categoryPath(categoryId: string) {
      const path = [];
      let current = categories.find((category) => category.category_id === categoryId);
      while (current) {
        path.unshift(current);
        current = current.parent_category_id ? categories.find((category) => category.category_id === current?.parent_category_id) : undefined;
      }
      return path;
    }

    function effectiveTemplate(categoryId: string) {
      const path = categoryPath(categoryId);
      const effectiveFields = path.flatMap((category) => rulesByCategory[category.category_id] ?? []).map((rule) => {
        const field = fields.find((candidate) => candidate.field_id === rule.field_id)!;
        return {
          field,
          required: rule.required,
          visible: rule.visible,
          display_group: rule.display_group,
          display_order: rule.display_order,
          default_value: rule.default_value ?? null,
          help_text: rule.help_text_override ?? null,
          inherited_from_category_ids: [rule.category_id]
        };
      });
      return {
        category: path[path.length - 1],
        root_category: path[0],
        category_path: path,
        fields: effectiveFields,
        template_checksum: canonicalChecksum("9")
      };
    }

    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/health") return jsonResponse(healthFixture);
      if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
      if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
      if (path === "/api/v1/equipment/registries") return jsonResponse(equipmentRegistriesFixture());
      if (path === "/api/v1/equipment/classification-presets") return jsonResponse({ presets: [] });
      if (path === "/api/v1/equipment-models" || path.startsWith("/api/v1/equipment-models?")) return jsonResponse({ equipment_models: [] });
      if (path === "/api/v1/driver-profiles") return jsonResponse({ driver_profiles: [] });
      if (path === "/api/v1/equipment/communication-providers") return jsonResponse({ providers: [{ provider: "simulation", available: true }] });
      if (path.startsWith("/api/v1/equipment/categories/tree")) return jsonResponse({ categories: buildCategoryTree(categories) });
      if (path.includes("/api/v1/equipment/categories/") && path.endsWith("/effective-template")) {
        const categoryId = decodeURIComponent(path.split("/api/v1/equipment/categories/")[1].split("/")[0]);
        return jsonResponse({ effective_template: effectiveTemplate(categoryId) });
      }
      if (path.includes("/api/v1/equipment/categories/") && path.endsWith("/field-rules")) {
        const categoryId = decodeURIComponent(path.split("/api/v1/equipment/categories/")[1].split("/")[0]);
        if (init?.method === "PUT") {
          const body = JSON.parse(String(init.body));
          rulesByCategory[categoryId] = body.rules.map((rule: Record<string, unknown>) => ({
            category_id: categoryId,
            field_id: String(rule.field_id),
            required: Boolean(rule.required),
            visible: Boolean(rule.visible),
            display_group: String(rule.display_group ?? "Identification"),
            display_order: Number(rule.display_order ?? 650),
            default_value: rule.default_value,
            help_text_override: typeof rule.help_text_override === "string" ? rule.help_text_override : null
          }));
        }
        return jsonResponse({ category_id: categoryId, rules: rulesByCategory[categoryId] ?? [] });
      }
      if (path === "/api/v1/equipment/categories" && init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        const parent = categories.find((category) => category.category_id === body.parent_category_id);
        const createdAt = "2026-07-13T00:00:00Z";
        const category = {
          category_id: String(body.category_id),
          parent_category_id: String(body.parent_category_id),
          root_category_id: parent?.root_category_id ?? String(body.category_id),
          label: String(body.label),
          description: String(body.description ?? ""),
          sort_order: Number(body.sort_order ?? 100),
          active: true,
          system_defined: false,
          created_at: createdAt,
          updated_at: createdAt,
          children: []
        };
        categories = [...categories, category];
        rulesByCategory[category.category_id] = [];
        return jsonResponse({ category });
      }
      if (path.startsWith("/api/v1/equipment/categories")) return jsonResponse({ categories });
      if (path === "/api/v1/equipment/field-definitions" && init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        const createdAt = "2026-07-13T00:00:00Z";
        const field = {
          field_id: `field_${body.field_code}`,
          field_code: String(body.field_code),
          label: String(body.label),
          description: String(body.description ?? ""),
          data_type: String(body.data_type),
          scope: "equipment_model",
          required_by_default: false,
          visible_by_default: true,
          unique_value: false,
          unit_quantity: null,
          allowed_units: body.allowed_units ?? [],
          option_values: body.option_values ?? [],
          validation_regex: null,
          default_value: null,
          display_group: String(body.display_group ?? "Identification"),
          display_order: Number(body.display_order ?? 650),
          active: true,
          system_defined: false,
          created_at: createdAt,
          updated_at: createdAt
        };
        fields = [...fields, field];
        return jsonResponse({ field_definition: field });
      }
      if (path.startsWith("/api/v1/equipment/field-definitions/") && init?.method === "PUT") {
        const fieldId = decodeURIComponent(path.split("/api/v1/equipment/field-definitions/")[1]);
        const body = JSON.parse(String(init.body));
        fields = fields.map((field) => field.field_id === fieldId ? { ...field, ...body, field_code: field.field_code } : field);
        return jsonResponse({ field_definition: fields.find((field) => field.field_id === fieldId) });
      }
      if (path.endsWith("/archive") && path.includes("/api/v1/equipment/field-definitions/") && init?.method === "POST") {
        const fieldId = decodeURIComponent(path.split("/api/v1/equipment/field-definitions/")[1].split("/")[0]);
        fields = fields.map((field) => field.field_id === fieldId ? { ...field, active: false } : field);
        return jsonResponse({ field_definition: fields.find((field) => field.field_id === fieldId) });
      }
      if (path.startsWith("/api/v1/equipment/field-definitions")) return jsonResponse({ field_definitions: fields });
      return mockBaseApiResponse(path, init);
    });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Catalogue des modèles" }));
    await user.click(await screen.findByRole("button", { name: "Administration du référentiel" }));
    const rfActions = await screen.findByRole("button", { name: /Actions .*radiofr/i });
    const rfActionMenu = within(rfActions.closest(".treeMenuWrap") as HTMLElement);
    await user.click(rfActions);
    expect(rfActionMenu.getByRole("button", { name: "Modifier le formulaire" })).toBeInTheDocument();
    await user.click(rfActionMenu.getByRole("button", { name: "Modifier le formulaire" }));
    expect(screen.getByRole("button", { name: "Formulaire" })).toHaveClass("active");

    await user.click(screen.getByText("Amplificateurs"));
    await user.click(screen.getByRole("button", { name: "Sous-catégories" }));
    await user.type(screen.getByLabelText(/Nom de la sous-catégorie/), "Amplificateurs faible bruit");
    expect(screen.getByLabelText(/Identifiant interne/)).not.toBeVisible();
    await user.click(screen.getByRole("button", { name: /Créer la sous-catégorie/ }));
    await waitFor(() => expect(document.querySelector('[data-category-id="amplificateurs_faible_bruit"]')).not.toBeNull());

    await user.click(document.querySelector('[data-category-id="amplificateurs_faible_bruit"]') as HTMLElement);
    await user.click(screen.getByRole("button", { name: "Formulaire" }));
    await user.type(screen.getByLabelText("Nom du champ"), "Criticite terrain");
    expect(screen.getByLabelText("Nom technique")).not.toBeVisible();
    await user.type(screen.getByPlaceholderText("Nouvelle valeur"), "Surveillance");
    await user.click(screen.getByRole("button", { name: "Ajouter une valeur" }));
    expect(screen.getByText("Surveillance")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Créer le champ/ }));
    await waitFor(() => expect(fields.some((field) => field.field_code === "criticite_terrain")).toBe(true));
    await user.click(screen.getByRole("button", { name: "Modifier Criticite terrain" }));
    await user.clear(screen.getByLabelText("Nom du champ"));
    await user.type(screen.getByLabelText("Nom du champ"), "Criticite mission");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(fields.some((field) => field.label === "Criticite mission")).toBe(true));
    await user.click(screen.getByRole("button", { name: /Ajouter au formulaire/ }));

    await user.click(screen.getByRole("button", { name: "Aperçu" }));
    expect(await screen.findByText("Criticite mission")).toBeInTheDocument();
    expect(screen.getByText(canonicalChecksum("9"))).not.toBeVisible();
    await user.click(screen.getByText("Informations techniques"));
    expect(screen.getAllByText(canonicalChecksum("9")).length).toBeGreaterThanOrEqual(1);
  });

  test("opens measurement engineering studios and runs curve CSV evaluation workflow", async () => {
    let curveStatus = "draft";
    let curveChecksum = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/health") return jsonResponse(healthFixture);
      if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
      if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
      if (path === "/api/v1/equipment/registries") return jsonResponse(equipmentRegistriesFixture());
      if (path === "/api/v1/equipment/classification-presets") return jsonResponse({ presets: [rfCablePresetFixture(), adcPresetFixture()] });
      if (path === "/api/v1/equipment-models" || path.startsWith("/api/v1/equipment-models?")) return jsonResponse({ equipment_models: [equipmentModelFixture()] });
      if (path === "/api/v1/driver-profiles") return jsonResponse({ driver_profiles: [] });
      if (path === "/api/v1/equipment/communication-providers") return jsonResponse({ providers: [{ provider: "simulation", available: true }] });
      const measurementResponse = measurementApiResponse(path, init, {
        curveStatus,
        curveChecksum,
        onCurveStatus: (status, checksum) => {
          curveStatus = status;
          curveChecksum = checksum;
        }
      });
      if (measurementResponse) return measurementResponse;
      return mockBaseApiResponse(path, init);
    });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Catalogue des modèles" }));
    await user.click(await screen.findByTitle("Signaux et corrections"));
    expect(await screen.findByRole("heading", { name: "Comment le signal est-il exploité ?" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /échantillons temporels/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /spectre en fréquence/ })).toBeInTheDocument();
    await user.click(await screen.findByRole("button", { name: "Corrections selon la fréquence" }));
    await user.click(await screen.findByRole("button", { name: /Demo RF cable loss/ }));
    expect(screen.getByText("Spectre fréquentiel")).toBeInTheDocument();
    expect(screen.getByText(/Compensation d.amplitude/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Amplitude / phase" }));
    expect(await screen.findByRole("img", { name: "1D curve plot" })).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText("frequence_hz,amplitude_db"), {
      target: { value: "frequence_hz,amplitude_db\n10000000,0.2\n100000000,1.25\n1000000000,3.8" }
    });
    await user.click(screen.getByRole("button", { name: /Importer CSV/ }));
    await user.click(screen.getByRole("button", { name: "Vérification ponctuelle" }));
    await user.clear(screen.getByLabelText("Fréquence (Hz)"));
    await user.type(screen.getByLabelText("Fréquence (Hz)"), "100000000");
    await user.click(screen.getByRole("button", { name: /Calculer la correction/ }));
    expect(await screen.findByText("Logarithmique en fréquence")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Vérifier la définition/ }));
    expect(await screen.findByText("Définition prête à être soumise")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Sauvegarder/ }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("/definition"), expect.objectContaining({ method: "PUT" })));
    await user.click(screen.getByRole("button", { name: /Soumettre/ }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("submit-for-review"), expect.objectContaining({ method: "POST" })));
    await user.click(screen.getByRole("button", { name: /Approuver/ }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("transitions/approve"), expect.objectContaining({ method: "POST" })));
  });

  test("opens sensor, scaling, DAQ and acquisition recipe measurement studios", async () => {
    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/health") return jsonResponse(healthFixture);
      if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
      if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
      if (path === "/api/v1/equipment/registries") return jsonResponse(equipmentRegistriesFixture());
      if (path === "/api/v1/equipment/classification-presets") return jsonResponse({ presets: [rfCablePresetFixture(), adcPresetFixture()] });
      if (path === "/api/v1/equipment-models" || path.startsWith("/api/v1/equipment-models?")) return jsonResponse({ equipment_models: [equipmentModelFixture()] });
      if (path === "/api/v1/driver-profiles") return jsonResponse({ driver_profiles: [] });
      if (path === "/api/v1/equipment/communication-providers") return jsonResponse({ providers: [{ provider: "simulation", available: true }] });
      const measurementResponse = measurementApiResponse(path, init, {});
      if (measurementResponse) return measurementResponse;
      return mockBaseApiResponse(path, init);
    });
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByTitle("Signaux et corrections"));
    await user.click(await screen.findByRole("button", { name: "Capteurs / transducteurs" }));
    await user.click(await screen.findByRole("button", { name: /Demo Current Probe/ }));
    expect(await screen.findByRole("combobox", { name: "Famille de capteur" })).toHaveValue("current_probe");

    await user.click(screen.getAllByRole("button", { name: "Conversions du signal brut" })[0]);
    await user.click(await screen.findByRole("button", { name: /Current probe 10 mV/ }));
    expect(screen.getByText("Signal temporel échantillonné")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Gain et offset" }));
    expect(screen.getByText(/gain × échantillon \+ offset/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Surcharge / écrêtage" }));
    expect(screen.getByText(/plage exploitable avant saturation/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Table de conversion" }));
    expect(screen.getByPlaceholderText("valeur_brute,valeur_physique")).toBeInTheDocument();

    await user.click(screen.getAllByRole("button", { name: "Corrections selon la fréquence" })[0]);
    expect(await screen.findByText("Aucune définition ouverte")).toBeInTheDocument();
    expect(screen.queryByPlaceholderText("valeur_brute,valeur_physique")).not.toBeInTheDocument();

    await user.click(screen.getAllByRole("button", { name: "Voies DAQ" })[0]);
    await user.click(await screen.findByRole("button", { name: /Demo DAQ AI/ }));
    await user.click(screen.getByRole("button", { name: "Echantillonnage" }));
    expect(screen.getByDisplayValue("1000000")).toBeInTheDocument();

    await user.click(screen.getAllByRole("button", { name: "Chaînes d'acquisition" })[0]);
    await user.click(await screen.findByRole("button", { name: /current_A through demo current probe/ }));
    await user.click(screen.getByRole("button", { name: "Chaîne de mesure" }));
    expect((await screen.findAllByText("Voie DAQ")).length).toBeGreaterThan(0);
    expect(screen.getByText("current_A [A]")).toBeInTheDocument();
  });

  test("reviews and activates a serial-specific correction from the material record", async () => {
    const instrument = metrologyInstrumentFixture();
    const model = equipmentModelFixture();
    instrument.equipment_model_id = model.identity.equipment_model_id;
    instrument.equipment_model_revision_id = model.current_approved_revision!.revision_id;
    instrument.equipment_model_checksum = model.current_approved_revision!.definition_checksum;
    const definition = model.current_approved_revision!.definition as EquipmentModelDefinition;
    definition.signal_ports.push({
      port_id: "measurement_result",
      label: "Measured result",
      directionality: "output",
      flow_role: "measurement_port",
      signal_domain: "software",
      quantity: "power",
      unit: "dBm"
    });
    definition.signal_paths = [{
      path_id: "RF_MEASUREMENT",
      label: "RF measurement",
      input_port_id: "rf_input",
      output_port_id: "measurement_result",
      transformations: [],
      correction_requirements: [{
        requirement_id: "rf_input_loss",
        display_name: "Pertes du chemin RF",
        description: "Compensation d'amplitude",
        signal_path_id: "RF_MEASUREMENT",
        correction_kind: "frequency_dependent_correction",
        physical_purpose: "Compenser les pertes entre le connecteur et le plan de référence.",
        operation: "add",
        input_quantity: "power",
        output_quantity: "power",
        expected_unit: "dB",
        required_for_use: true,
        asset_specific_policy: "asset_preferred",
        model_default_reference: {
          correction_kind: "frequency_dependent_correction",
          definition_id: "CURVE-NOMINAL-RF-LOSS",
          revision_id: "CURVE-NOMINAL-RF-LOSS-rev-0001",
          definition_checksum: `sha256:${"d".repeat(64)}`,
          quality: "manufacturer_nominal"
        },
        conditions: {}
      }]
    }];
    const characterization = {
      characterization_id: "CHAR-SA-RF-001",
      asset_id: instrument.asset_id,
      characterization_kind: "frequency_response",
      label: "Pertes mesurées du chemin RF",
      performed_on: "2026-07-01",
      valid_from: "2026-07-01",
      valid_until: "2027-07-01",
      source_kind: "characterization",
      provider: "Laboratoire interne",
      method_reference: "MET-RF-001",
      decision: "conforming",
      definition_schema_version: "emc-locus.asset-characterization-definition.v1",
      definition: {
        definition_schema_version: "emc-locus.asset-characterization-definition.v1",
        characterization_id: "CHAR-SA-RF-001",
        asset_id: instrument.asset_id,
        label: "Pertes mesurées du chemin RF",
        correction: { correction_kind: "frequency_response", correction: { points: [] } },
        conditions: {}
      },
      definition_checksum: `sha256:${"c".repeat(64)}`,
      certificate_reference: "CERT-RF-001",
      document_manifest: null,
      comment: "",
      recorded_at: "2026-07-14T20:00:00Z",
      recorded_by: "metrology.operator",
      revision: "rev-characterization",
      environmental_conditions: {},
      as_found: null,
      as_left: null,
      adjustment_performed: false
    };
    let assignment: AssetCorrectionAssignment | null = null;
    let revision = "rev-draft";
    const correctionPath = `/api/v1/metrology/instruments/${instrument.asset_id}/corrections`;

    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      if (path === "/api/v1/health") return jsonResponse(healthFixture);
      if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
      if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
      if (path === "/api/v1/equipment/registries") return jsonResponse(equipmentRegistriesFixture());
      if (path === "/api/v1/equipment/classification-presets") return jsonResponse({ presets: [] });
      if (path === "/api/v1/equipment-models" || path.startsWith("/api/v1/equipment-models?")) return jsonResponse({ equipment_models: [model] });
      if (path === "/api/v1/driver-profiles") return jsonResponse({ driver_profiles: [] });
      if (path === "/api/v1/equipment/communication-providers") return jsonResponse({ providers: [] });
      if (path === "/api/v1/metrology/instruments") return jsonResponse({ instruments: [instrument] });
      if (path.endsWith("/characterizations")) return jsonResponse({ asset_id: instrument.asset_id, characterizations: [characterization] });
      if (path.endsWith("/characterizations/CHAR-SA-RF-001/audit-events")) return jsonResponse({ audit_events: [] });
      if (path === "/api/v1/metrology/corrections/review-queue") return jsonResponse({ assignments: assignment?.status === "waiting_for_review" ? [{ assignment, revision }] : [] });
      if (path === correctionPath && init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        assignment = {
          assignment_id: body.assignment_id,
          asset_id: instrument.asset_id,
          equipment_model_id: instrument.equipment_model_id,
          equipment_model_revision_id: instrument.equipment_model_revision_id,
          equipment_model_checksum: instrument.equipment_model_checksum,
          signal_path_id: body.signal_path_id,
          requirement_id: body.requirement_id,
          correction_definition_id: characterization.characterization_id,
          correction_revision_id: characterization.revision,
          correction_checksum: characterization.definition_checksum,
          source_event_id: characterization.characterization_id,
          source_kind: "characterization",
          valid_from: characterization.valid_from,
          valid_until: characterization.valid_until,
          status: "draft",
          conditions: {},
          assigned_at: "2026-07-14T20:00:00Z",
          assigned_by: "metrology.operator"
        };
        revision = "rev-draft";
        return jsonResponse({ assignment, revision });
      }
      if (path === correctionPath) return jsonResponse({ assignments: assignment ? [{ assignment, revision }] : [] });
      if (path.includes("/transitions/submit-for-review")) {
        assignment!.status = "waiting_for_review";
        assignment!.submitted_at = "2026-07-14T20:10:00Z";
        revision = "rev-review";
        return jsonResponse({ assignment, revision });
      }
      if (path.includes("/transitions/approve-and-activate")) {
        assignment!.status = "active";
        assignment!.approved_at = "2026-07-14T20:20:00Z";
        assignment!.approved_by = "metrology.reviewer";
        revision = "rev-active";
        return jsonResponse({ assignment, revision });
      }
      if (path === `${correctionPath}/resolve`) {
        const active = assignment?.status === "active";
        return jsonResponse({ report: {
          asset_id: instrument.asset_id,
          intended_use_on: "2026-07-14",
          execution_context: "accredited",
          ready: active,
          resolutions: [{
            requirement_id: "rf_input_loss",
            display_name: "Pertes du chemin RF",
            signal_path_id: "RF_MEASUREMENT",
            selected_source: active ? "asset_specific" : "none",
            reason: active ? "active_asset_correction" : "asset_correction_missing",
            fallback_used: false,
            blocking: !active
          }]
        } });
      }
      return mockBaseApiResponse(path, init);
    });

    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Métrologie du parc" }));
    expect(await screen.findByText("Correction manquante")).toBeInTheDocument();
    await user.click(await screen.findByRole("button", { name: "Lier cette preuve" }));
    await user.click(await screen.findByRole("button", { name: "Soumettre pour revue" }));
    expect(await screen.findByText(/attend une décision/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Approuver et activer" }));
    expect(await screen.findByText("Active pour ce matériel")).toBeInTheDocument();
    expect(screen.getByText("Prêt pour un essai")).toBeInTheDocument();
    expect(screen.getByText("Valeur propre à ce matériel")).toBeInTheDocument();
    expect(screen.getByText("Valeur nominale du modèle")).toBeInTheDocument();
    expect(screen.getByText(/non sélectionnée/)).toBeInTheDocument();
    const operatorText = document.querySelector(".physicalAssetsLayout")?.textContent ?? "";
    for (const forbidden of [
      "EngineeringCurve",
      "ScalingProfile",
      "asset_correction_assignment",
      "definition_checksum",
      "entity_id"
    ]) {
      expect(operatorText).not.toContain(forbidden);
    }
  });
});

function equipmentModelFixture() {
  const revision = {
    revision_id: "EQM-NRP6AN-FWD-rev-0001",
    equipment_model_id: "EQM-NRP6AN-FWD",
    revision_number: 1,
    parent_revision_id: null,
    status: "approved",
    definition_schema_version: "emc-locus.equipment-model-definition.v2",
    definition_checksum: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    created_by: "equipment.author",
    created_at: "2026-07-03T00:00:00Z",
    updated_at: "2026-07-03T00:00:00Z",
    submitted_at: "2026-07-03T00:00:00Z",
    approved_at: "2026-07-03T00:00:00Z",
    capability_count: 1,
    interface_count: 1,
    signal_port_count: 1,
    definition: {
      definition_schema_version: "emc-locus.equipment-model-definition.v2",
      manufacturer: "R&S",
      model_name: "NRP6AN",
      variant: "FWD",
      equipment_class: "controllable_instrument",
      functional_role: "measurement_instrument",
      category_code: "power_meter",
      signal_domains: ["rf", "ethernet"],
      technology_tags: ["rf_50_ohm", "ethernet", "raw_tcp", "scpi"],
      specifications: [],
      signal_ports: [
        {
          port_id: "rf_input",
          label: "RF input",
          directionality: "input",
          flow_role: "measurement_port",
          signal_domain: "rf",
          connector_type: "N",
          quantity: "power",
          unit: "dBm",
          impedance: 50
        }
      ],
      communication_interfaces: [
        {
          interface_id: "tcp",
          label: "SCPI TCP",
          transport_kind: "ethernet_tcp",
          access_provider_kind: "native_tcp",
          protocol_kind: "scpi",
          required: true,
          default_interface: true
        }
      ],
      capabilities: [
        {
          capability_id: "measure_power",
          label: "Measure power",
          description: "Measure RF power.",
          capability_kind: "measure_power",
          inputs: [],
          outputs: [],
          safety_class: "read_only"
        }
      ],
      custom_field_values: {
        manufacturer: "R&S",
        model_name: "NRP6AN",
        variant: "FWD"
      },
      template_snapshot: {
        category_id: "measurement_instruments_digitizers",
        root_category_id: "measurement_instruments_digitizers",
        category_path: ["Instruments de mesure / numériseurs"],
        captured_at: "2026-07-13T00:00:00Z",
        template_checksum: "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        fields: []
      },
      is_demo: false,
      metadata: {}
    }
  };
  return {
    identity: {
      equipment_model_id: "EQM-NRP6AN-FWD",
      manufacturer: "R&S",
      model_name: "NRP6AN",
      variant: "FWD",
      equipment_class: "controllable_instrument",
      category_code: "power_meter",
      root_category_id: "measurement_instruments_digitizers",
      is_demo: false,
      current_approved_revision_id: revision.revision_id,
      created_by: "equipment.author",
      created_at: "2026-07-03T00:00:00Z",
      updated_at: "2026-07-03T00:00:00Z"
    },
    current_approved_revision: revision,
    latest_revision: revision,
    active_draft_revision: null
  };
}

function metrologySummary(
  overrides: Partial<PhysicalAssetMetrologySummary> = {}
): PhysicalAssetMetrologySummary {
  return {
    status: "valid",
    checked_on: "2026-07-27",
    calibration_requirement: "required",
    calibration_period_months: 12,
    latest_calibration_decision: "conforming",
    calibrated_at: "2026-04-30",
    due_at: "2027-04-30",
    warning_threshold_days: 30,
    blocking: false,
    explanation: "Étalonnage valide jusqu’au 2027-04-30.",
    reasons: ["calibration_valid"],
    latest_calibration_event_id: "CAL-EVENT-001",
    latest_calibration_revision: "rev-0001",
    ...overrides
  };
}

function physicalAssetFixture(overrides: Record<string, unknown> = {}) {
  return {
    asset_id: "ASSET-NRP6AN-001",
    inventory_code: "INV-0042",
    serial_number: "SN 103456",
    part_number: "NRP6AN",
    equipment_model_id: "EQM-NRP6AN-FWD",
    equipment_model_revision_id: "EQM-NRP6AN-FWD-rev-0001",
    equipment_model_checksum: canonicalChecksum("a"),
    manufacturer: "R&S",
    model_name: "NRP6AN",
    variant: "FWD",
    category_code: "power_meter",
    category_path: ["Mesure RF", "Wattmètres"],
    laboratory_location_id: "LAB-LOCATION-CEM-1",
    laboratory_location_label: "Labo CEM 1",
    laboratory_location_status: "active",
    ownership_source: "laboratory_owned",
    service_state: "usable",
    administrative_availability: "available",
    administrative_unavailability_reason: "",
    operational_usage: {
      state: "available",
      assessed_at: "2026-07-27T08:00:00Z",
      evidence: []
    },
    availability_state: "available",
    service_state_reason: "Contrôle avant utilisation",
    notes: "Capteur de puissance RF",
    revision: 3,
    model_link_state: "controlled",
    migrated_from_metrology: false,
    metrology: metrologySummary(),
    created_at: "2026-07-14T08:00:00Z",
    updated_at: "2026-07-15T08:00:00Z",
    ...overrides
  };
}

function stationSetupFixture() {
  const definition = {
    definition_schema_version: "emc-locus.station-measurement-setup-definition.v2",
    setup_id: "SETUP-CONDUCTED-001",
    label: "Chaîne d'émissions conduites",
    laboratory_location_id: "LAB-LOCATION-CEM-1",
    laboratory_location_label: "Labo CEM 1",
    planned_use_on: "2026-07-30",
    execution_mode: "accredited" as const,
    asset_bindings: [],
    connections: [],
    correction_selections: [],
    notes: {}
  };
  const revision = {
    revision_id: "SETUP-CONDUCTED-001-rev-0001",
    setup_id: definition.setup_id,
    revision_number: 1,
    parent_revision_id: null,
    status: "draft" as const,
    definition_schema_version: definition.definition_schema_version,
    definition,
    definition_checksum: canonicalChecksum("b"),
    readiness: {
      ready: false,
      checked_on: "2026-07-26",
      issues: [{ code: "station_setup_requires_two_materials", severity: "blocking" as const, dimension: "structure", message: "Ajoutez les matériels de la chaîne." }]
    },
    created_by: "station.technician",
    created_at: "2026-07-26T08:00:00Z",
    updated_at: "2026-07-26T08:00:00Z",
    ready_at: null
  };
  return {
    identity: {
      setup_id: definition.setup_id,
      label: definition.label,
      current_ready_revision_id: null,
      created_by: "station.technician",
      created_at: "2026-07-26T08:00:00Z",
      updated_at: "2026-07-26T08:00:00Z"
    },
    active_draft_revision: revision,
    current_ready_revision: null,
    latest_revision: revision
  };
}

function stationAssetOptionsResponse(laboratoryLocationId: string, asset: ReturnType<typeof physicalAssetFixture>) {
  return jsonResponse({
    assessed_at: "2026-07-30T12:00:00Z",
    checked_on: "2026-07-30",
    execution_mode: "accredited",
    laboratory_location_id: laboratoryLocationId,
    assets: [{ asset, eligible: true, blocking_reasons: [], warnings: [] }]
  });
}

function rfCableModelFixture() {
  const revision = {
    revision_id: "EQM-RF-CABLE-DEMO-rev-0001",
    equipment_model_id: "EQM-RF-CABLE-DEMO",
    revision_number: 1,
    parent_revision_id: null,
    status: "draft",
    definition_schema_version: "emc-locus.equipment-model-definition.v2",
    definition_checksum: "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    created_by: "equipment.author",
    created_at: "2026-07-11T00:00:00Z",
    updated_at: "2026-07-11T00:00:00Z",
    submitted_at: null,
    approved_at: null,
    capability_count: 0,
    interface_count: 0,
    signal_port_count: 2,
    definition: {
      definition_schema_version: "emc-locus.equipment-model-definition.v2",
      manufacturer: "Demo",
      model_name: "RF Cable",
      variant: "1m",
      equipment_class: "passive_component",
      functional_role: "rf_network_element",
      category_code: "rf_cable",
      signal_domains: ["rf"],
      technology_tags: ["rf_50_ohm"],
      specifications: [],
      signal_ports: [
        {
          port_id: "RF_A",
          label: "RF A",
          directionality: "through",
          flow_role: "through_port",
          signal_domain: "rf",
          required: true,
          connector_type: "N",
          technology_tags: ["rf_50_ohm"],
          quantity: "power",
          unit: "dBm",
          impedance: 50
        },
        {
          port_id: "RF_B",
          label: "RF B",
          directionality: "through",
          flow_role: "through_port",
          signal_domain: "rf",
          required: true,
          connector_type: "N",
          technology_tags: ["rf_50_ohm"],
          quantity: "power",
          unit: "dBm",
          impedance: 50
        }
      ],
      communication_interfaces: [],
      capabilities: [],
      custom_field_values: {
        manufacturer: "Demo",
        model_name: "RF Cable",
        variant: "1m",
        connector_a: "N",
        connector_b: "N"
      },
      template_snapshot: {
        category_id: "rf_cable",
        root_category_id: "rf_equipment",
        category_path: ["Équipements radiofréquences", "Câbles RF"],
        captured_at: "2026-07-13T00:00:00Z",
        template_checksum: "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        fields: effectiveTemplateFixture().fields
      },
      is_demo: false,
      metadata: { entry_template_category_id: "rf_cable" }
    }
  };
  return {
    identity: {
      equipment_model_id: "EQM-RF-CABLE-DEMO",
      manufacturer: "Demo",
      model_name: "RF Cable",
      variant: "1m",
      equipment_class: "passive_component",
      category_code: "rf_cable",
      root_category_id: "rf_equipment",
      is_demo: false,
      current_approved_revision_id: null,
      created_by: "equipment.author",
      created_at: "2026-07-11T00:00:00Z",
      updated_at: "2026-07-11T00:00:00Z"
    },
    current_approved_revision: null,
    latest_revision: revision,
    active_draft_revision: revision
  };
}

function equipmentRegistriesFixture() {
  const item = (code: string, label = code) => ({
    code,
    label,
    description: `${label} registry item`,
    recommended_equipment_classes: [],
    recommended_functional_roles: [],
    compatible_signal_domains: [],
    compatible_directionalities: [],
    deprecated: false
  });
  return {
    functional_roles: [item("measurement_instrument", "Measurement instrument"), item("rf_network_element", "RF network element"), item("sensor", "Sensor")],
    signal_domains: [item("rf", "RF"), item("analog_voltage", "Analog voltage"), item("can_bus", "CAN bus"), item("ethernet", "Ethernet"), item("trigger", "Trigger")],
    port_directionalities: [item("input"), item("output"), item("through"), item("communication"), item("bidirectional"), item("control")],
    flow_roles: [item("measurement_port"), item("through_port"), item("source_port"), item("sink_port"), item("communication_port"), item("control_port"), item("field_side_port"), item("transducer_output_port")],
    technology_tags: [item("rf_50_ohm", "RF 50 ohm"), item("adc_converter", "ADC converter"), item("voltage_input", "Voltage input"), item("can_bus", "CAN bus"), item("ethernet", "Ethernet"), item("trigger", "Trigger")]
  };
}

function rfCablePresetFixture() {
  return {
    preset_id: "rf_cable",
    category_label: "RF equipment",
    function_description: "50 ohm RF through component.",
    example_label: "RF Cable",
    default_equipment_class: "passive_component",
    default_functional_role: "rf_network_element",
    default_signal_domains: ["rf"],
    default_technology_tags: ["rf_50_ohm"],
    notes: "",
    deprecated: false,
    ports: [
      { port_order: 10, port_id: "RF_A", label: "RF A", directionality: "through", flow_role: "through_port", signal_domain: "rf", connector_type: "N", technology_tags: ["rf_50_ohm"], quantity: "power", unit: "dBm", impedance: 50, required: true },
      { port_order: 20, port_id: "RF_B", label: "RF B", directionality: "through", flow_role: "through_port", signal_domain: "rf", connector_type: "N", technology_tags: ["rf_50_ohm"], quantity: "power", unit: "dBm", impedance: 50, required: true }
    ]
  };
}

function adcPresetFixture() {
  return {
    preset_id: "adc_converter",
    category_label: "Converters and acquisition",
    function_description: "Analog to digital conversion without implicit CAN bus.",
    example_label: "ADC Converter",
    default_equipment_class: "daq_device",
    default_functional_role: "converter",
    default_signal_domains: ["analog_voltage", "digital_logic"],
    default_technology_tags: ["adc_converter", "voltage_input"],
    notes: "No CAN bus port is created by default.",
    deprecated: false,
    ports: [
      { port_order: 10, port_id: "ANALOG_IN", label: "Analog input", directionality: "input", flow_role: "measurement_port", signal_domain: "analog_voltage", connector_type: "BNC", technology_tags: ["voltage_input"], quantity: "voltage", unit: "V", required: true },
      { port_order: 20, port_id: "DIGITAL_OUT", label: "Digital output", directionality: "output", flow_role: "transducer_output_port", signal_domain: "digital_logic", technology_tags: ["adc_converter"], quantity: "binary", unit: "dimensionless", required: true }
    ]
  };
}

type EquipmentCategoryFixture = {
  category_id: string;
  parent_category_id: string | null;
  root_category_id: string;
  label: string;
  description: string;
  sort_order: number;
  active: boolean;
  system_defined: boolean;
  created_at: string;
  updated_at: string;
  children: EquipmentCategoryFixture[];
};

function equipmentCategoriesFixture(): EquipmentCategoryFixture[] {
  const now = "2026-07-13T00:00:00Z";
  return [
    {
      category_id: "general_equipment",
      parent_category_id: null,
      root_category_id: "general_equipment",
      label: "Général",
      description: "Champs communs",
      sort_order: 0,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    },
    {
      category_id: "energy_sources",
      parent_category_id: "general_equipment",
      root_category_id: "energy_sources",
      label: "Sources d'énergie",
      description: "",
      sort_order: 10,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    },
    {
      category_id: "signal_sources",
      parent_category_id: "general_equipment",
      root_category_id: "signal_sources",
      label: "Sources de signaux",
      description: "",
      sort_order: 20,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    },
    {
      category_id: "rf_equipment",
      parent_category_id: "general_equipment",
      root_category_id: "rf_equipment",
      label: "Équipements radiofréquences",
      description: "",
      sort_order: 30,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    },
    {
      category_id: "sensors_transducers",
      parent_category_id: "general_equipment",
      root_category_id: "sensors_transducers",
      label: "Capteurs / transducteurs",
      description: "",
      sort_order: 40,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    },
    {
      category_id: "actuators_emitters",
      parent_category_id: "general_equipment",
      root_category_id: "actuators_emitters",
      label: "Actionneurs / Emetteurs",
      description: "",
      sort_order: 50,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    },
    {
      category_id: "measurement_instruments_digitizers",
      parent_category_id: "general_equipment",
      root_category_id: "measurement_instruments_digitizers",
      label: "Instruments de mesure / numériseurs",
      description: "",
      sort_order: 60,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    },
    {
      category_id: "processing_control_systems",
      parent_category_id: "general_equipment",
      root_category_id: "processing_control_systems",
      label: "Systèmes de traitement et de contrôle",
      description: "",
      sort_order: 70,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    },
    {
      category_id: "rf_cable",
      parent_category_id: "rf_equipment",
      root_category_id: "rf_equipment",
      label: "Câbles RF",
      description: "",
      sort_order: 10,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    },
    {
      category_id: "rf_amplifier",
      parent_category_id: "rf_equipment",
      root_category_id: "rf_equipment",
      label: "Amplificateurs",
      description: "",
      sort_order: 40,
      active: true,
      system_defined: true,
      created_at: now,
      updated_at: now,
      children: []
    }
  ];
}

function equipmentCategoryTreeFixture() {
  return buildCategoryTree(equipmentCategoriesFixture());
}

function buildCategoryTree(categories: EquipmentCategoryFixture[]): EquipmentCategoryFixture[] {
  const byParent = new Map<string | null, EquipmentCategoryFixture[]>();
  categories.forEach((category) => {
    const siblings = byParent.get(category.parent_category_id) ?? [];
    siblings.push(category);
    byParent.set(category.parent_category_id, siblings);
  });
  const build = (category: EquipmentCategoryFixture): EquipmentCategoryFixture => ({
    ...category,
    children: (byParent.get(category.category_id) ?? []).map(build)
  });
  return (byParent.get(null) ?? []).map(build);
}

function equipmentFieldDefinitionsFixture() {
  const now = "2026-07-13T00:00:00Z";
  const field = (field_id: string, field_code: string, label: string, required = false) => ({
    field_id,
    field_code,
    label,
    description: "",
    data_type: "short_text",
    scope: "equipment_model",
    required_by_default: required,
    visible_by_default: true,
    unique_value: false,
    unit_quantity: null,
    allowed_units: [],
    option_values: [],
    validation_regex: null,
    default_value: null,
    display_group: "Identification",
    display_order: 10,
    active: true,
    system_defined: true,
    created_at: now,
    updated_at: now
  });
  return [
    field("field_manufacturer", "manufacturer", "Fabricant", true),
    field("field_model_name", "model_name", "Modèle", true),
    field("field_connector_a", "connector_a", "Connecteur A"),
    field("field_connector_b", "connector_b", "Connecteur B"),
    {
      ...field("field_documentation_file", "documentation", "Documentation"),
      data_type: "file_reference",
      display_group: "Documents",
      display_order: 900
    }
  ];
}

function effectiveTemplateFixture() {
  const categories = equipmentCategoriesFixture();
  const fields = equipmentFieldDefinitionsFixture();
  return {
    category: categories.find((category) => category.category_id === "rf_cable"),
    root_category: categories.find((category) => category.category_id === "rf_equipment"),
    category_path: [
      categories.find((category) => category.category_id === "general_equipment"),
      categories.find((category) => category.category_id === "rf_equipment"),
      categories.find((category) => category.category_id === "rf_cable")
    ].filter(Boolean),
    fields: fields.map((field, index) => ({
      field,
      required: field.field_code === "manufacturer" || field.field_code === "model_name",
      visible: true,
      display_group: "Identification",
      display_order: (index + 1) * 10,
      default_value: null,
      help_text: null,
      inherited_from_category_ids: field.field_code.startsWith("connector") ? ["rf_cable"] : ["general_equipment"]
    })),
    template_checksum: "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
  };
}

function driverProfileFixture() {
  const revision = {
    revision_id: "DRV-NRP6AN-SCPI-rev-0001",
    driver_profile_id: "DRV-NRP6AN-SCPI",
    equipment_model_id: "EQM-NRP6AN-FWD",
    supported_model_revision_id: "EQM-NRP6AN-FWD-rev-0001",
    revision_number: 1,
    parent_revision_id: null,
    status: "approved",
    definition_schema_version: "emc-locus.driver-profile-definition.v1",
    definition_checksum: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    created_by: "driver.author",
    created_at: "2026-07-03T00:00:00Z",
    updated_at: "2026-07-03T00:00:00Z",
    submitted_at: "2026-07-03T00:00:00Z",
    approved_at: "2026-07-03T00:00:00Z",
    action_count: 1,
    definition: {
      definition_schema_version: "emc-locus.driver-profile-definition.v1",
      equipment_model_id: "EQM-NRP6AN-FWD",
      supported_model_revision_id: "EQM-NRP6AN-FWD-rev-0001",
      supported_model_definition_checksum: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      supported_firmware_ranges: ["*"],
      communication_profiles: ["tcp"],
      actions: [
        {
          action_id: "measure_power",
          label: "Measure power",
          description: "Query power.",
          implements_capability_id: "measure_power",
          inputs: [],
          outputs: [],
          safety_class: "read_only",
          default_timeout_ms: 1000,
          script: { steps: [{ step_id: "query", step_type: "io_query", interface_id: "tcp", payload: "MEAS:POW?", response_binding: "${result.power_dbm}" }] }
        }
      ],
      metadata: {}
    }
  };
  return {
    identity: {
      driver_profile_id: "DRV-NRP6AN-SCPI",
      equipment_model_id: "EQM-NRP6AN-FWD",
      label: "NRP6AN SCPI",
      current_approved_revision_id: revision.revision_id,
      created_by: "driver.author",
      created_at: "2026-07-03T00:00:00Z",
      updated_at: "2026-07-03T00:00:00Z"
    },
    current_approved_revision: revision,
    latest_revision: revision,
    active_draft_revision: null
  };
}

function metrologyInstrumentFixture() {
  return {
    asset_id: "RF-CABLE-001",
    inventory_code: "INV-RF-CABLE-001",
    family: "passive_rf_component",
    category_code: "rf_cable",
    equipment_model_id: "EQM-RF-CABLE-DEMO",
    equipment_model_revision_id: "EQM-RF-CABLE-DEMO-rev-0001",
    equipment_model_checksum: `sha256:${"a".repeat(64)}`,
    manufacturer: "Huber+Suhner",
    model: "Sucoflex 104",
    serial_number: "RF-24017",
    part_number: "22510146",
    availability: "available",
    legacy_availability: null,
    serviceability_status: "usable",
    serviceability_reason: "Contrôle visuel conforme",
    serviceability_updated_at: "2026-07-14T00:00:00Z",
    calibration_requirement: "required",
    calibration_period_months: 12,
    calibration_due_warning_days: 45,
    capabilities_json: "{}",
    metrology_notes: "Câble de référence",
    created_at: "2026-07-14T00:00:00Z",
    updated_at: "2026-07-14T00:00:00Z",
    revision: "rev-0001",
    latest_calibration: {
      calibrated_at: "2026-07-01",
      due_at: "2027-07-01",
      certificate_reference: "CERT-RF-2026-017",
      revision: "rev-0001"
    },
    latest_calibration_event: null
  };
}

function mockProjectWorkflowApi(settings: {
  existingSchedule?: boolean;
  locationFailure?: boolean;
} = {}) {
  let project: ProjectRecord = {
    code: "CEM-UX-001",
    customer_name: "Industries Atlas",
    stage: settings.existingSchedule ? "test_planning" : "contract_review",
    execution_mode: "investigation",
    created_at: "2026-07-15T08:00:00Z",
    archived_at: null,
    revision: "rev-0001"
  };
  const requiredItems = ["customer_request_defined", "deviations_recorded"];
  const completedItems: CompletedContractReviewItem[] = [];
  const schedule: ServiceScheduleItem[] = settings.existingSchedule
    ? [
        {
          item_code: "PLAN-CEM-UX-001-EXISTING",
          project_code: "CEM-UX-001",
          title: "Émission conduite déjà planifiée",
          test_category_code: null,
          test_method_code: null,
          planned_start_at: "2026-07-16T09:00",
          planned_end_at: "2026-07-16T12:00",
          assigned_operator: "Alice Martin",
          laboratory_location_id: "LAB-LOCATION-CEM-1",
          laboratory_location_label: "Labo CEM 1",
          equipment_under_test: "Convertisseur prototype",
          status: "planned",
          notes: "",
          revision: 1,
          created_by: "Responsable laboratoire",
          updated_by: "Responsable laboratoire",
          created_at: "2026-07-15T08:15:00Z",
          updated_at: "2026-07-15T08:15:00Z",
          available_transitions: ["confirmed", "cancelled"],
          can_reschedule: true
        }
      ]
    : [];
  const audit: ProjectAuditEvent[] = [
    {
      sequence: 1,
      actor: "Responsable laboratoire",
      action: "project_created",
      reason: "Ouverture du dossier d'essai",
      payload_json: "{}",
      occurred_at: "2026-07-15T08:00:00Z"
    }
  ];

  function contractReview() {
    const completedNames = new Set(completedItems.map((item) => item.item));
    const missingItems = requiredItems.filter((item) => !completedNames.has(item));
    return {
      project_code: project.code,
      execution_mode: project.execution_mode,
      required_items: requiredItems,
      completed_items: completedItems,
      missing_items: missingItems,
      complete: missingItems.length === 0
    };
  }

  fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
    const path = String(input);
    const method = init?.method ?? "GET";
    if (path === "/api/v1/projects" && method === "GET") {
      return jsonResponse({ projects: [project] });
    }
    if (path === `/api/v1/projects/${project.code}` && method === "GET") {
      return jsonResponse({ project });
    }
    if (path === `/api/v1/projects/${project.code}/contract-review` && method === "GET") {
      return jsonResponse({ contract_review: contractReview() });
    }
    const completedItemMatch = path.match(
      new RegExp(`^/api/v1/projects/${project.code}/contract-review/items/([^/]+)/complete$`)
    );
    if (completedItemMatch && method === "POST") {
      const item = decodeURIComponent(completedItemMatch[1]);
      if (!completedItems.some((entry) => entry.item === item)) {
        completedItems.push({
          item,
          completed_by: "Responsable laboratoire",
          completed_at: "2026-07-15T08:05:00Z",
          comment: "Point vérifié depuis LAB CONSOLE"
        });
      }
      audit.push({
        sequence: audit.length + 1,
        actor: "Responsable laboratoire",
        action: "contract_review_item_completed",
        reason: "Point vérifié depuis LAB CONSOLE",
        payload_json: "{}",
        occurred_at: "2026-07-15T08:05:00Z"
      });
      return jsonResponse({
        operation: "contract_review_item_completed",
        operation_id: "op-review",
        replayed: false,
        already_completed: false,
        resulting_revision: `rev-${String(completedItems.length + 1).padStart(4, "0")}`,
        contract_review: contractReview()
      });
    }
    if (
      path === `/api/v1/projects/${project.code}/transitions/to-test-planning` &&
      method === "POST"
    ) {
      project = { ...project, stage: "test_planning", revision: "rev-0004" };
      audit.push({
        sequence: audit.length + 1,
        actor: "Responsable laboratoire",
        action: "project_stage_advanced",
        reason: "Revue du besoin terminée",
        payload_json: "{}",
        occurred_at: "2026-07-15T08:10:00Z"
      });
      return jsonResponse({
        operation: "project_stage_advanced",
        operation_id: "op-planning",
        replayed: false,
        project
      });
    }
    if (path === `/api/v1/projects/${project.code}/schedule-items` && method === "GET") {
      return jsonResponse({ project_code: project.code, schedule_items: schedule });
    }
    if (path === `/api/v1/projects/${project.code}/schedule-items` && method === "POST") {
      const body = JSON.parse(String(init?.body)) as Record<string, string>;
      const item: ServiceScheduleItem = {
        item_code: body.item_code,
        project_code: project.code,
        title: body.title,
        test_category_code: null,
        test_method_code: null,
        planned_start_at: body.planned_start_at,
        planned_end_at: body.planned_end_at,
        assigned_operator: body.assigned_operator,
        laboratory_location_id: body.laboratory_location_id,
        laboratory_location_label: body.laboratory_location_label,
        equipment_under_test: body.equipment_under_test,
        status: "planned",
        notes: body.notes ?? "",
        revision: 1,
        created_by: body.actor,
        updated_by: body.actor,
        created_at: "2026-07-15T08:15:00Z",
        updated_at: "2026-07-15T08:15:00Z",
        available_transitions: ["confirmed", "cancelled"],
        can_reschedule: true
      };
      schedule.push(item);
      return jsonResponse({
        operation: "service_schedule_item_planned",
        operation_id: "op-schedule",
        replayed: false,
        schedule_item: item
      });
    }
    const transitionMatch = path.match(
      new RegExp(`^/api/v1/projects/${project.code}/schedule-items/([^/]+)/transitions/confirm$`)
    );
    if (transitionMatch && method === "POST") {
      const item = schedule.find(
        (entry) => entry.item_code === decodeURIComponent(transitionMatch[1])
      );
      if (!item) return jsonResponse({ error: { code: "not_found", message: path } }, 404);
      item.status = "confirmed";
      item.revision = 2;
      item.updated_at = "2026-07-15T08:20:00Z";
      item.available_transitions = ["in_progress", "cancelled"];
      return jsonResponse({
        operation: "service_schedule_item_status_changed",
        operation_id: "op-confirm",
        replayed: false,
        schedule_item: item
      });
    }
    if (path === `/api/v1/projects/${project.code}/audit-events` && method === "GET") {
      return jsonResponse({ project_code: project.code, audit_events: audit });
    }
    if (path === "/api/v1/laboratory-locations" && settings.locationFailure) {
      return jsonResponse(
          { error: { code: "laboratory_location_unavailable", message: "location source unavailable" } },
        503
      );
    }
    return mockBaseApiResponse(path, init);
  });
}

function mockLaboratoryPlanningApi(settings: {
  noCompatibleMaterials?: boolean;
  legacyLocation?: boolean;
  identificationConcurrency?: boolean;
  legacyConflict?: boolean;
  locationFailure?: boolean;
  preparationOptionsFailure?: boolean;
} = {}) {
  let rescheduleAttempts = 0;
  const first: LaboratoryScheduleItem = {
    item_code: "PLAN-LAB-001",
    project_code: "CEM-LAB-001",
    customer_name: "Industries Atlas",
    project_stage: "test_planning",
    title: "Émission conduite",
    test_category_code: null,
    test_method_code: null,
    planned_start_at: "2026-07-15T09:00",
    planned_end_at: "2026-07-15T12:00",
    assigned_operator: "Alice Martin",
    laboratory_location_id: "LAB-LOCATION-CEM-1",
    laboratory_location_label: "Labo CEM 1",
    equipment_under_test: "Convertisseur prototype",
    status: "planned",
    notes: "Préparer le réseau de stabilisation",
    revision: 1,
    created_by: "Responsable laboratoire",
    updated_by: "Responsable laboratoire",
    created_at: "2026-07-15T07:00:00Z",
    updated_at: "2026-07-15T07:00:00Z",
    available_transitions: ["confirmed", "cancelled"],
    can_reschedule: true
  };
  const second: LaboratoryScheduleItem = {
    ...first,
    item_code: "PLAN-LAB-002",
    project_code: "CEM-LAB-002",
    customer_name: "Mobilités Boréal",
    title: "Immunité rayonnée",
    planned_start_at: "2026-07-16T09:00",
    planned_end_at: "2026-07-16T12:00",
    assigned_operator: "Alice Martin",
    laboratory_location_id: "LAB-LOCATION-ANECHOIC",
    laboratory_location_label: "Chambre semi-anéchoïque",
    equipment_under_test: "Calculateur de bord",
    status: "confirmed",
    revision: 2,
    available_transitions: ["in_progress", "cancelled"]
  };
  if (settings.legacyLocation) {
    first.laboratory_location_id = null;
    first.laboratory_location_label = "Poste CEM historique";
  }
  const method = {
    template_id: "METHOD-RI-001",
    revision_id: "METHOD-RI-001-rev-0002",
    revision_number: 2,
    revision_status: "approved" as const,
    definition_checksum: canonicalChecksum("a"),
    title: "Immunité rayonnée",
    measurement_axis: "frequency_sweep",
    method_code: "RI-SIM",
    method_revision: "B",
    standard_references: ["IEC 61000-4-3"],
    instrumentation_chain: [
      {
        slot_id: "receiver",
        label: "Récepteur de mesure",
        required_category: "emi_receiver",
        required: true,
        calibration_requirement: "required" as const,
        substitution_policy: "same_category" as const,
        depends_on_slots: []
      }
    ]
  };
  const station = {
    setup_id: "SETUP-RI-001",
    revision_id: "SETUP-RI-001-rev-0003",
    revision_number: 3,
    revision_status: "ready" as const,
    definition_checksum: canonicalChecksum("b"),
    label: "Chaîne immunité rayonnée",
    laboratory_location_id: "LAB-LOCATION-ANECHOIC",
    laboratory_location_label: "Chambre semi-anéchoïque",
    planned_use_on: "2026-07-16",
    execution_mode: "investigation" as const,
    assets: [
      {
        binding_id: "receiver-binding",
        role_label: "Récepteur de mesure",
        asset_id: "ASSET-RX-001",
        asset_revision: "asset-revision-001",
        inventory_code: "INV-RX-001",
        serial_number: "SN-ESW-101",
        manufacturer: "Rohde & Schwarz",
        model_name: "ESW",
        equipment_model_id: "MODEL-ESW",
        equipment_model_revision_id: "MODEL-ESW-rev-0002",
        equipment_model_checksum: canonicalChecksum("c"),
        category_code: "emi_receiver",
        metrology: metrologySummary(),
        capabilities: []
      },
      {
        binding_id: "generator-binding",
        role_label: "Générateur RF",
        asset_id: "ASSET-GEN-001",
        asset_revision: "asset-revision-002",
        inventory_code: "INV-GEN-001",
        serial_number: "SN-SMW-101",
        manufacturer: "Rohde & Schwarz",
        model_name: "SMW200A",
        equipment_model_id: "MODEL-SMW200A",
        equipment_model_revision_id: "MODEL-SMW200A-rev-0001",
        equipment_model_checksum: canonicalChecksum("f"),
        category_code: "rf_signal_generator",
        metrology: metrologySummary(),
        capabilities: []
      }
    ],
    corrections: []
  };
  const preparationOptions: PlannedTestPreparationOptions = {
    project_code: second.project_code,
    schedule_item_code: second.item_code,
    methods: [method],
    station_setups: [
      {
        station_setup: station,
        eligible: true,
        blocking_reasons: [],
        warnings: [],
        asset_options: station.assets.map((asset) => ({
          asset: physicalAssetFixture({
            asset_id: asset.asset_id,
            inventory_code: asset.inventory_code,
            serial_number: asset.serial_number,
            equipment_model_id: asset.equipment_model_id,
            equipment_model_revision_id: asset.equipment_model_revision_id,
            equipment_model_checksum: asset.equipment_model_checksum,
            manufacturer: asset.manufacturer,
            model_name: asset.model_name,
            category_code: asset.category_code,
            category_path: [],
            laboratory_location_label: "Labo CEM 1",
            service_state: "usable",
            availability_state: "available",
            metrology: asset.metrology
          }) as PhysicalAsset,
          eligible: true,
          blocking_reasons: [],
          warnings: []
        })),
        readiness: { ready: true, checked_on: "2026-07-16", issues: [] }
      }
    ],
    material_compatibility: [
      {
        method_revision_id: method.revision_id,
        station_setup_revision_id: station.revision_id,
        materials: [
          {
            slot_id: "receiver",
            binding_id: "receiver-binding",
            compatible: !settings.noCompatibleMaterials,
            reason: settings.noCompatibleMaterials
              ? "Le modèle ne fournit pas la capacité spectrale requise."
              : undefined,
            next_action: settings.noCompatibleMaterials
              ? "Choisissez un récepteur déclaré pour cette mesure."
              : undefined
          },
          {
            slot_id: "receiver",
            binding_id: "generator-binding",
            compatible: false,
            reason: "Le générateur ne respecte pas la catégorie du rôle.",
            next_action: "Choisissez un récepteur compatible."
          }
        ]
      }
    ]
  };
  let preparation: PlannedTestPreparationAggregate = {
    project_code: second.project_code,
    schedule_item_code: second.item_code,
    current_state: "missing",
    can_start: false,
    current_revision: null,
    revision_count: 0
  };
  const preparationRevisions: PlannedTestPreparationRevision[] = [];

  fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
    const path = String(input);
    if (path.startsWith("/api/v1/service-schedule?")) {
      return jsonResponse({
        week_start: "2026-07-13",
        week_end: "2026-07-17",
        schedule_items: [first, second]
      });
    }
    const preparationBase = `/api/v1/projects/${second.project_code}/schedule-items/${second.item_code}/preparation`;
    if (path === preparationBase && (!init?.method || init.method === "GET")) {
      return jsonResponse({ preparation });
    }
    if (
      path ===
      `/api/v1/projects/${first.project_code}/schedule-items/${first.item_code}/preparation`
    ) {
      return jsonResponse({
        preparation: {
          project_code: first.project_code,
          schedule_item_code: first.item_code,
          current_state: "missing",
          can_start: false,
          current_revision: null,
          revision_count: 0
        }
      });
    }
    if (path === `${preparationBase}/options`) {
      if (settings.preparationOptionsFailure) {
        return jsonResponse({ error: { code: "preparation_options_unavailable", message: "options de préparation indisponibles" } }, 503);
      }
      return jsonResponse(preparationOptions);
    }
    if (path === `${preparationBase}/revisions`) {
      return jsonResponse({
        project_code: second.project_code,
        schedule_item_code: second.item_code,
        revisions: preparationRevisions
      });
    }
    if (path === `${preparationBase}/assessments` && init?.method === "POST") {
      const body = JSON.parse(String(init.body)) as {
        assignments: Array<{ slot_id: string; binding_id: string }>;
      };
      const ready = body.assignments.some(
        (assignment) =>
          assignment.slot_id === "receiver" && assignment.binding_id === "receiver-binding"
      );
      const revisionNumber = preparationRevisions.length + 1;
      const revision: PlannedTestPreparationRevision = {
        revision_id: `PLAN-LAB-002-prep-rev-${String(revisionNumber).padStart(4, "0")}`,
        revision_number: revisionNumber,
        parent_revision_id: preparation.current_revision?.revision_id ?? null,
        recorded_state: ready ? "ready" : "blocked",
        effective_state: ready ? "ready" : "blocked",
        is_current: true,
        definition: {
          definition_schema_version: "emc-locus.planned-test-preparation.v1",
          schedule: {
            project_code: second.project_code,
            item_code: second.item_code,
            revision: second.revision,
            title: second.title,
            planned_start_at: second.planned_start_at,
            planned_end_at: second.planned_end_at,
            assigned_operator: second.assigned_operator,
            laboratory_location_id: second.laboratory_location_id,
            laboratory_location_label: second.laboratory_location_label,
            equipment_under_test: second.equipment_under_test,
            execution_mode: "investigation",
            status: second.status
          },
          method,
          station_setup: station,
          assignments: body.assignments,
          verdict: {
            ready,
            checked_on: "2026-07-16",
            issues: ready
              ? []
              : [
                  {
                    code: "planned_test_required_role_unassigned",
                    severity: "blocking",
                    dimension: "instrument_assignment",
                    message: "Le récepteur requis n'est pas affecté.",
                    next_action: "Choisissez un matériel du montage pour ce rôle.",
                    method_slot_ids: ["receiver"]
                  }
                ]
          }
        },
        definition_checksum: canonicalChecksum(ready ? "e" : "d"),
        actor: "Opérateur CEM",
        reason: "Vérification avant essai",
        operation_id: `op-prep-${revisionNumber}`,
        device_id: "lab-console",
        correlation_id: `corr-prep-${revisionNumber}`,
        created_at: `2026-07-15T0${revisionNumber + 8}:00:00Z`
      };
      for (const existing of preparationRevisions) existing.is_current = false;
      preparationRevisions.unshift(revision);
      preparation = {
        ...preparation,
        current_state: ready ? "ready" : "blocked",
        can_start: ready,
        current_revision: revision,
        revision_count: revisionNumber
      };
      return jsonResponse({
        operation: "planned_test_preparation_assessed",
        operation_id: revision.operation_id,
        replayed: false,
        preparation
      });
    }
    if (
      path.endsWith("/schedule-items/PLAN-LAB-002/transitions/start") &&
      init?.method === "POST"
    ) {
      second.status = "in_progress";
      second.revision += 1;
      second.available_transitions = ["completed", "cancelled"];
      second.can_reschedule = false;
      return jsonResponse({
        operation: "service_schedule_item_status_changed",
        operation_id: "op-start",
        replayed: false,
        schedule_item: second
      });
    }
    if (
      path.endsWith("/schedule-items/PLAN-LAB-001/transitions/confirm") &&
      init?.method === "POST"
    ) {
      first.status = "confirmed";
      first.revision += 1;
      first.available_transitions = ["in_progress", "cancelled"];
      return jsonResponse({
        operation: "service_schedule_item_status_changed",
        operation_id: "op-confirm",
        replayed: false,
        schedule_item: first
      });
    }
    if (path.endsWith("/schedule-items/PLAN-LAB-001/reschedule") && init?.method === "POST") {
      rescheduleAttempts += 1;
      if (settings.legacyConflict) {
        return jsonResponse(
          {
            error: {
              code: "service_schedule_legacy_location_identity_required",
              message: "Un créneau existant utilise encore un lieu non identifié.",
              details: {
                resource: "unresolved_location_identity",
                value: "Poste CEM historique",
                conflicting_item: {
                  item_code: "PLAN-HIST-001",
                  project_code: "CEM-HIST-001",
                  title: "Essai historique",
                  planned_start_at: "2026-07-15T10:00",
                  planned_end_at: "2026-07-15T11:00",
                  assigned_operator: "Bob Martin",
                  laboratory_location_id: null,
                  laboratory_location_label: "Poste CEM historique",
                  status: "confirmed"
                },
                next_action: "identify_service_schedule_location"
              }
            }
          },
          409
        );
      }
      if (rescheduleAttempts === 1) {
        return jsonResponse(
          {
            error: {
              code: "service_schedule_operator_conflict",
              message: "operator conflict",
              details: {
                resource: "operator",
                value: "Alice Martin",
                conflicting_item: {
                  item_code: second.item_code,
                  project_code: second.project_code,
                  title: second.title,
                  planned_start_at: second.planned_start_at,
                  planned_end_at: second.planned_end_at,
                  assigned_operator: second.assigned_operator,
                  laboratory_location_id: second.laboratory_location_id,
                  laboratory_location_label: second.laboratory_location_label
                }
              }
            }
          },
          409
        );
      }
      const body = JSON.parse(String(init.body)) as Record<string, string>;
      first.planned_start_at = body.planned_start_at;
      first.planned_end_at = body.planned_end_at;
      first.assigned_operator = body.assigned_operator;
      first.laboratory_location_id = body.laboratory_location_id;
      first.laboratory_location_label = body.laboratory_location_label;
      first.revision += 1;
      return jsonResponse({
        operation: "service_schedule_item_rescheduled",
        operation_id: "op-reschedule",
        replayed: false,
        schedule_item: first
      });
    }
    if (
      path.endsWith("/schedule-items/PLAN-LAB-001/location-identification")
      && init?.method === "POST"
    ) {
      if (settings.identificationConcurrency) {
        return jsonResponse(
          {
            error: {
              code: "service_schedule_concurrent_update",
              message: "schedule changed",
              details: { expected_revision: 1, actual_revision: 2 }
            }
          },
          409
        );
      }
      const body = JSON.parse(String(init.body)) as Record<string, string>;
      first.laboratory_location_id = body.laboratory_location_id;
      first.laboratory_location_label = body.laboratory_location_label;
      first.revision += 1;
      return jsonResponse({
        operation: "service_schedule_item_location_identified",
        operation_id: "op-identify-location",
        replayed: false,
        schedule_item: first
      });
    }
    if (path === "/api/v1/laboratory-locations" && settings.locationFailure) {
      return jsonResponse(
          { error: { code: "laboratory_location_unavailable", message: "location source unavailable" } },
        503
      );
    }
    return mockBaseApiResponse(path, init);
  });
}

function mockBaseApiResponse(path: string, init?: RequestInit) {
  if (path === "/api/v1/health") return jsonResponse(healthFixture);
  if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
  if (path === "/api/v1/laboratory-locations") {
    return jsonResponse({
      locations: [
        {
          location_id: "LAB-LOCATION-CEM-1",
          label: "Labo CEM 1",
          description: "Poste conduit",
          status: "active",
          revision: 1,
          created_at: "2026-07-14T08:00:00Z",
          updated_at: "2026-07-14T08:00:00Z"
        },
        {
          location_id: "LAB-LOCATION-ANECHOIC",
          label: "Chambre semi-anéchoïque",
          description: "Site rayonné",
          status: "active",
          revision: 1,
          created_at: "2026-07-14T08:00:00Z",
          updated_at: "2026-07-14T08:00:00Z"
        }
      ]
    });
  }
  if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
  if (path.startsWith("/api/v1/equipment/categories/tree")) return jsonResponse({ categories: equipmentCategoryTreeFixture() });
  if (path.includes("/api/v1/equipment/categories/") && path.endsWith("/effective-template")) return jsonResponse({ effective_template: effectiveTemplateFixture() });
  if (path.startsWith("/api/v1/equipment/categories/rf_cable/field-rules")) return jsonResponse({ category_id: "rf_cable", rules: [] });
  if (path.startsWith("/api/v1/equipment/categories")) return jsonResponse({ categories: equipmentCategoriesFixture() });
  if (path.startsWith("/api/v1/equipment/field-definitions")) return jsonResponse({ field_definitions: equipmentFieldDefinitionsFixture() });
  if (path.includes("/api/v1/fleet/assets/") && path.endsWith("/audit-events")) {
    return jsonResponse({ entity_id: path.split("/")[5], audit_events: [] });
  }
  if (path === "/api/v1/metrology/instruments") return jsonResponse({ instruments: [] });
  if (path.includes("/api/v1/metrology/instruments/") && path.endsWith("/audit-events")) {
    return jsonResponse({ audit_events: [] });
  }
  if (path.includes("/api/v1/metrology/instruments/") && path.endsWith("/characterizations")) {
    return jsonResponse({ asset_id: "", characterizations: [] });
  }
  if (path === "/api/v1/metrology/corrections/review-queue") {
    return jsonResponse({ assignments: [] });
  }
  if (path.includes("/api/v1/metrology/instruments/") && path.endsWith("/corrections")) {
    return jsonResponse({ assignments: [] });
  }
  if (path.includes("/api/v1/metrology/instruments/") && path.endsWith("/corrections/resolve")) {
    return jsonResponse({ report: { asset_id: "", intended_use_on: "2026-07-14", execution_context: "accredited", ready: true, resolutions: [] } });
  }
  if (path === "/api/v1/scaling-profiles") {
    return jsonResponse({ aggregate_kind: "scaling-profiles", collection_key: "items", items: [] });
  }
  if (path === "/api/v1/engineering-curves") {
    return jsonResponse({ aggregate_kind: "engineering-curves", collection_key: "items", items: [] });
  }
  if (path === "/api/v1/test-templates/TT-LAB-001") return jsonResponse({ test_template: templateFixture() });
  if (path.includes("/revisions/TT-LAB-001-rev-0001") && !path.endsWith("/definition") && !path.includes("/transitions/")) return jsonResponse({ revision: revisionFixture() });
  if (path === "/api/v1/test-templates/TT-LAB-001/revisions") return jsonResponse({ template_id: "TT-LAB-001", revisions: [revisionFixture()] });
  if (path === "/api/v1/test-templates/TT-LAB-001/audit-events") return jsonResponse({ template_id: "TT-LAB-001", audit_events: auditFixture });
  return jsonResponse({ error: { code: "unexpected", message: `${path} ${init?.method ?? "GET"}` } }, 500);
}

function measurementApiResponse(
  path: string,
  init: RequestInit | undefined,
  state: {
    curveStatus?: string;
    curveChecksum?: string;
    onCurveStatus?: (status: string, checksum: string) => void;
  }
) {
  const collections = [
    "sensor-definitions",
    "scaling-profiles",
    "engineering-curves",
    "daq-channel-profiles",
    "acquisition-channel-recipes"
  ];
  for (const collection of collections) {
    if (path === `/api/v1/${collection}`) {
      if (init?.method === "POST") {
        const body = JSON.parse(String(init.body));
        const aggregate = measurementAggregate(collection, state, body.entity_id, body.definition);
        return jsonResponse({
          operation: `${collection}_created`,
          operation_id: body.operation_id,
          replayed: false,
          item: aggregate,
          revision: aggregate.latest_revision
        });
      }
      return jsonResponse({ aggregate_kind: collection, collection_key: "items", items: [measurementAggregate(collection, state)] });
    }
    if (path.startsWith(`/api/v1/${collection}/`) && path.endsWith("/audit-events")) {
      return jsonResponse({ aggregate_kind: collection, entity_id: path.split("/")[4], audit_events: [] });
    }
    if (path.startsWith(`/api/v1/${collection}/`) && path.endsWith("/revisions")) {
      return jsonResponse({
        aggregate_kind: collection,
        entity_id: path.split("/")[4],
        revisions: [measurementAggregate(collection, state).latest_revision]
      });
    }
    if (path.startsWith(`/api/v1/${collection}/`) && path.includes("/revisions/") && path.endsWith("/definition")) {
      const body = JSON.parse(String(init?.body ?? "{}"));
      const aggregate = measurementAggregate(collection, state, undefined, body.definition);
      return jsonResponse({
        operation: `${collection}_saved`,
        operation_id: body.operation_id,
        replayed: false,
        item: aggregate,
        revision: aggregate.latest_revision
      });
    }
    if (path.startsWith(`/api/v1/${collection}/`) && path.includes("submit-for-review")) {
      state.onCurveStatus?.("under_review", "sha256:2222222222222222222222222222222222222222222222222222222222222222");
      const body = JSON.parse(String(init?.body ?? "{}"));
      const aggregate = measurementAggregate(collection, state);
      return jsonResponse({
        operation: `${collection}_submitted`,
        operation_id: body.operation_id,
        replayed: false,
        item: aggregate,
        revision: aggregate.latest_revision
      });
    }
    if (path.startsWith(`/api/v1/${collection}/`) && path.includes("transitions/approve")) {
      state.onCurveStatus?.("approved", "sha256:3333333333333333333333333333333333333333333333333333333333333333");
      const body = JSON.parse(String(init?.body ?? "{}"));
      const aggregate = measurementAggregate(collection, state);
      return jsonResponse({
        operation: `${collection}_approved`,
        operation_id: body.operation_id,
        replayed: false,
        item: aggregate,
        revision: aggregate.latest_revision
      });
    }
    if (path.startsWith(`/api/v1/${collection}/`) && !path.includes("/revisions/")) {
      return jsonResponse({ aggregate_kind: collection, item: measurementAggregate(collection, state) });
    }
  }
  if (path.endsWith("-definitions/validate")) {
    return jsonResponse({ valid: true, issues: [], definition_checksum: canonicalChecksum("d") });
  }
  if (path.includes("/engineering-curves/") && path.endsWith("/evaluate")) {
    return jsonResponse({
      evaluation: {
        values: { correction_db: 1.25 },
        axis_values: { frequency: 100000000 },
        interpolation: "log_x_linear_y",
        extrapolated: false,
        source_revision_id: "CURVE-DEMO-RF-CABLE-1M-LOSS-rev-0001",
        source_checksum: "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
      }
    });
  }
  return null;
}

function measurementAggregate(
  collection: string,
  state: { curveStatus?: string; curveChecksum?: string },
  entityId?: string,
  definitionOverride?: Record<string, unknown>
) {
  const definition = definitionOverride ?? measurementDefinition(collection);
  const id = entityId ?? String(
    definition.sensor_definition_id ??
      definition.scaling_profile_id ??
      definition.curve_id ??
      definition.daq_channel_profile_id ??
      definition.recipe_id
  );
  const status = collection === "engineering-curves" ? state.curveStatus ?? "draft" : "approved";
  const checksum = collection === "engineering-curves"
    ? state.curveChecksum ?? "sha256:1111111111111111111111111111111111111111111111111111111111111111"
    : "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
  const revision = {
    aggregate_kind: collection.replace(/-/g, "_"),
    revision_id: `${id}-rev-0001`,
    entity_id: id,
    revision_number: 1,
    parent_revision_id: null,
    status,
    definition_schema_version: String(definition.definition_schema_version),
    definition,
    definition_checksum: checksum,
    label: measurementLabel(collection),
    summary_kind: collection,
    created_by: "measurement.author",
    created_at: "2026-07-11T00:00:00Z",
    updated_at: "2026-07-11T00:00:00Z",
    submitted_at: status === "draft" ? null : "2026-07-11T00:10:00Z",
    approved_at: status === "approved" ? "2026-07-11T00:20:00Z" : null
  };
  return {
    identity: {
      aggregate_kind: collection.replace(/-/g, "_"),
      entity_id: id,
      label: measurementLabel(collection),
      summary_kind: collection,
      current_approved_revision_id: status === "approved" ? revision.revision_id : null,
      created_by: "measurement.author",
      created_at: "2026-07-11T00:00:00Z",
      updated_at: "2026-07-11T00:00:00Z"
    },
    current_approved_revision: status === "approved" ? revision : null,
    latest_revision: revision,
    active_draft_revision: status === "draft" ? revision : null
  };
}

function measurementLabel(collection: string) {
  return {
    "sensor-definitions": "Demo Current Probe 10mV/A",
    "scaling-profiles": "Current probe 10 mV/A",
    "engineering-curves": "Demo RF cable loss",
    "daq-channel-profiles": "Demo DAQ AI +/-10 V",
    "acquisition-channel-recipes": "current_A through demo current probe"
  }[collection] ?? collection;
}

function measurementDefinition(collection: string): Record<string, unknown> {
  if (collection === "sensor-definitions") {
    return {
      definition_schema_version: "emc-locus.sensor-definition.v1",
      sensor_definition_id: "SNS-DEMO-CURRENT-PROBE-10MV-A",
      manufacturer: "Demo",
      model_name: "Current Probe 10mV/A",
      sensor_family: "current_probe",
      physical_input_quantity: "current",
      engineering_output_quantity: "current",
      engineering_output_unit: "A",
      electrical_output_quantity: "voltage",
      electrical_output_unit: "V",
      signal_domain: "analog_voltage",
      technology_tags: ["voltage_input"],
      required_excitation: { excitation_kind: "none", external_allowed: false },
      input_mode_requirement: "differential",
      nominal_range: { minimum: -100, maximum: 100, unit: "A" },
      frequency_range: { minimum_hz: 10, maximum_hz: 100000000 },
      scaling_profile_refs: [{ entity_id: "SCL-DEMO-CURRENT-10MV-A", revision_id: "SCL-DEMO-CURRENT-10MV-A-rev-0001", require_approved: true }],
      correction_curve_refs: [],
      metadata: {}
    };
  }
  if (collection === "scaling-profiles") {
    return {
      definition_schema_version: "emc-locus.scaling-profile-definition.v1",
      scaling_profile_id: "SCL-DEMO-CURRENT-10MV-A",
      label: "Current probe 10 mV/A",
      input_quantity: "voltage",
      input_unit: "V",
      output_quantity: "current",
      output_unit: "A",
      signal_representation: "time_domain_samples",
      scaling_kind: "linear",
      parameters: { scale: 100, offset: 0, points: [{ input: 0, output: 0 }, { input: 0.01, output: 1 }] },
      input_limits: { minimum: -10, maximum: 10, handling: "mark_clipped" },
      metadata: {}
    };
  }
  if (collection === "engineering-curves") {
    return {
      definition_schema_version: "emc-locus.engineering-curve-definition.v1",
      curve_id: "CURVE-DEMO-RF-CABLE-1M-LOSS",
      curve_type: "cable_loss",
      label: "Demo RF cable loss",
      signal_representation: "frequency_domain_spectrum",
      independent_axes: [{ axis: "frequency", quantity: "frequency", unit: "Hz" }],
      dependent_values: [{ value_id: "correction_db", quantity: "dimensionless", unit: "dB", component: "amplitude", operation: "add" }],
      units: { frequency: "Hz", correction_db: "dB" },
      points: [
        { axis_values: { frequency: 10000000 }, values: { correction_db: 0.2 } },
        { axis_values: { frequency: 100000000 }, values: { correction_db: 1.25 } },
        { axis_values: { frequency: 1000000000 }, values: { correction_db: 3.8 } }
      ],
      interpolation: "log_x_linear_y",
      extrapolation_policy: "warn",
      validity_domain: {},
      conditions: {},
      metadata: {}
    };
  }
  if (collection === "daq-channel-profiles") {
    return {
      definition_schema_version: "emc-locus.daq-channel-profile-definition.v1",
      daq_channel_profile_id: "DAQ-DEMO-AI-10V-1MS",
      label: "Demo DAQ AI +/-10 V",
      channel_kind: "analog_input",
      signal_domain: "analog_voltage",
      input_quantity: "voltage",
      input_unit: "V",
      supported_ranges: [{ minimum: -10, maximum: 10, unit: "V" }],
      resolution_bits: 16,
      max_sampling_rate: 1000000,
      min_sampling_rate: 1,
      coupling_modes: ["dc", "ac"],
      input_modes: ["single_ended", "differential", "iepe"],
      excitation_capabilities: [{ excitation_kind: "iepe", nominal_value: 4, unit: "mA", external_allowed: false }],
      iepe_support: true,
      metadata: {}
    };
  }
  return {
    definition_schema_version: "emc-locus.acquisition-channel-recipe-definition.v1",
    recipe_id: "REC-DEMO-CURRENT-A",
    label: "current_A through demo current probe",
    output_channel_name: "current_A",
    output_quantity: "current",
    output_unit: "A",
    daq_channel_profile_ref: { entity_id: "DAQ-DEMO-AI-10V-1MS", revision_id: "DAQ-DEMO-AI-10V-1MS-rev-0001", require_approved: true },
    sensor_definition_ref: { entity_id: "SNS-DEMO-CURRENT-PROBE-10MV-A", revision_id: "SNS-DEMO-CURRENT-PROBE-10MV-A-rev-0001", require_approved: true },
    scaling_profile_ref: { entity_id: "SCL-DEMO-CURRENT-10MV-A", revision_id: "SCL-DEMO-CURRENT-10MV-A-rev-0001", require_approved: true },
    correction_curve_refs: [],
    sample_rate: 1000000,
    range: { minimum: -10, maximum: 10, unit: "V" },
    coupling: "dc",
    input_mode: "differential",
    excitation: { excitation_kind: "none", external_allowed: false },
    validation_rules: [],
    metadata: {}
  };
}
