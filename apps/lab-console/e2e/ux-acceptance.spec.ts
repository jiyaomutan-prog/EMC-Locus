import { expect, test } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const viewports = [
  { width: 1440, height: 900 },
  { width: 1280, height: 720 }
];

test("main operator paths stay clear at supported desktop sizes", async ({ page }, testInfo) => {
  for (const viewport of viewports) {
    const size = `${viewport.width}x${viewport.height}`;
    await page.setViewportSize(viewport);
    await page.goto("/lab/");

    await expect(page.getByText("Agent local")).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    await expect(page.getByRole("heading", { name: "Méthodes d'essai" })).toBeVisible();
    await expect(page.getByText("Aucun template")).toHaveCount(0);
    await expect(page.getByText("bibliothèque API")).toHaveCount(0);
    await assertNoHorizontalOverflow(page);
    await capture(page, testInfo, `methodes-${size}.png`);

    await page.getByRole("button", { name: "Équipements" }).click();
    await page.getByRole("button", { name: "Signaux et corrections" }).click();
    await expect(page.getByRole("heading", { name: "Comment le signal est-il exploité ?" })).toBeVisible();
    await expect(page.getByRole("button", { name: /échantillons temporels/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /spectre en fréquence/ })).toBeVisible();
    await assertNoHorizontalOverflow(page);
    await capture(page, testInfo, `choix-signal-${size}.png`);

    await page.getByRole("button", { name: /spectre en fréquence/ }).click();
    const label = `Pertes câble RF contrôle ${size}`;
    await page.getByLabel("Nom de la correction").fill(label);
    await page.getByRole("button", { name: "Nouvelle réponse" }).click();
    await expect(page.locator(".equipmentStudio").getByRole("heading", { name: label })).toBeVisible();
    await expect(page.getByText("Identifiant interne")).toHaveCount(0);
    await expect(page.getByText("Identifiant personnalisé")).toHaveCount(0);
    await expect(page.getByText("Aucun verdict serveur courant.")).toHaveCount(0);
    await expect(page.getByText("Empreinte de contrôle SHA-256")).toBeHidden();
    await assertNoHorizontalOverflow(page);
    await capture(page, testInfo, `correction-frequentielle-${size}.png`);
  }
});

async function assertNoHorizontalOverflow(page: import("@playwright/test").Page) {
  const dimensions = await page.evaluate(() => ({
    clientWidth: document.documentElement.clientWidth,
    scrollWidth: document.documentElement.scrollWidth
  }));
  expect(dimensions.scrollWidth).toBeLessThanOrEqual(dimensions.clientWidth);
}

async function capture(
  page: import("@playwright/test").Page,
  testInfo: import("@playwright/test").TestInfo,
  name: string
) {
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.waitForTimeout(80);
  const body = await page.screenshot({ animations: "disabled" });
  const evidenceDirectory = path.resolve(process.cwd(), "../../.codex/ux-audit/0.15.1");
  await mkdir(evidenceDirectory, { recursive: true });
  await writeFile(path.join(evidenceDirectory, name), body);
  await testInfo.attach(name, { body, contentType: "image/png" });
}
