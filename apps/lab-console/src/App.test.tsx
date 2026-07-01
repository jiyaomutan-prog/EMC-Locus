import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { App } from "./App";
import {
  auditFixture,
  healthFixture,
  jsonResponse,
  revisionFixture,
  storageFixture,
  templateFixture
} from "./test/fixtures";

const fetchMock = vi.fn();

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
      return jsonResponse({ valid: true, issues: [], definition_checksum: "sha256:bbbb" });
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
  test("renders an empty API library without fake business rows", async () => {
    mockBaseApi([]);

    render(<App />);

    expect(await screen.findByText("Aucun template")).toBeInTheDocument();
    expect(screen.queryByText("CEM-2026-001")).not.toBeInTheDocument();
    expect(screen.queryByText("Client demo")).not.toBeInTheDocument();
  });

  test("loads templates, filters them, and opens the draft studio", async () => {
    mockBaseApi([templateFixture()]);
    const user = userEvent.setup();

    render(<App />);

    expect(await screen.findByText("Inrush current template")).toBeInTheDocument();
    await user.type(screen.getByLabelText("Recherche template"), "inrush");
    expect(screen.getByText("TT-LAB-001")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Brouillon" }));

    expect(await screen.findByText("Template Studio")).toBeInTheDocument();
    expect(screen.getByText("Non modifie")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Variables" }));
    expect(screen.getByDisplayValue("repeat_count")).toBeInTheDocument();
  });

  test("edits variables, validates, saves, submits, approves, and derives through API calls", async () => {
    mockBaseApi([templateFixture()]);
    const user = userEvent.setup();

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Brouillon" }));
    await user.click(screen.getByRole("button", { name: "Variables" }));
    await user.click(screen.getByRole("button", { name: "Ajouter une variable" }));
    await user.click(screen.getByRole("button", { name: /Valider/ }));
    expect(await screen.findByText("Definition valide")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Sauvegarder/ }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("/definition"), expect.any(Object)));
    await user.click(screen.getByRole("button", { name: /Valider/ }));
    await user.click(screen.getByRole("button", { name: /Soumettre/ }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("submit-for-review"), expect.any(Object)));
  });

  test("shows CAS conflict without dropping local edits", async () => {
    mockBaseApi([templateFixture()]);
    fetchMock.mockImplementationOnce(() => jsonResponse(healthFixture));
    const user = userEvent.setup();

    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Brouillon" }));
    await user.click(screen.getByLabelText("Titre definition"));
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
                expected_definition_checksum: "sha256:local",
                actual_definition_checksum: "sha256:server"
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

    await user.click(await screen.findByRole("button", { name: /Creer/ }));
    await user.type(screen.getByLabelText("Identifiant"), "TT-NEW-001");
    await user.type(screen.getByLabelText("Titre"), "New template");
    await user.click(screen.getByRole("button", { name: "Creer le brouillon" }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith("/api/v1/test-templates", expect.objectContaining({ method: "POST" })));

    await user.click(screen.getByRole("button", { name: /Bibliotheque/ }));
    await user.click(screen.getByRole("button", { name: /Cloner/ }));
    await user.selectOptions(screen.getByLabelText("Source approuvee"), "TT-LAB-001|TT-LAB-001-rev-0001");
    await user.type(screen.getByLabelText("Nouvel identifiant"), "TT-CLONE-001");
    await user.type(screen.getByLabelText("Nouveau titre"), "Clone template");
    await user.click(screen.getByRole("button", { name: "Cloner vers un nouveau template" }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("/clone"), expect.objectContaining({ method: "POST" })));
  });
});

function mockBaseApiResponse(path: string, init?: RequestInit) {
  if (path === "/api/v1/health") return jsonResponse(healthFixture);
  if (path === "/api/v1/storage/status") return jsonResponse(storageFixture);
  if (path === "/api/v1/test-templates") return jsonResponse({ test_templates: [templateFixture()] });
  if (path === "/api/v1/test-templates/TT-LAB-001") return jsonResponse({ test_template: templateFixture() });
  if (path.includes("/revisions/TT-LAB-001-rev-0001") && !path.endsWith("/definition") && !path.includes("/transitions/")) return jsonResponse({ revision: revisionFixture() });
  if (path === "/api/v1/test-templates/TT-LAB-001/revisions") return jsonResponse({ template_id: "TT-LAB-001", revisions: [revisionFixture()] });
  if (path === "/api/v1/test-templates/TT-LAB-001/audit-events") return jsonResponse({ template_id: "TT-LAB-001", audit_events: auditFixture });
  return jsonResponse({ error: { code: "unexpected", message: `${path} ${init?.method ?? "GET"}` } }, 500);
}
