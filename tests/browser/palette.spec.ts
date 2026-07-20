/**
 * Browser-observable gate for the Cmd+K search palette (ADR 0015, issue
 * 0008 slice S4). Drives a live server seeded from the repo's own docs/:
 * with JavaScript enabled, Control+K opens the overlay, typing a live-corpus
 * term renders ranked result links, activating one navigates to its record
 * page, and Escape closes the overlay again. With JavaScript disabled, the
 * palette never appears — the plain GET /?q= search form stays the fully
 * functional fallback (ADR 0015's progressive-enhancement decision).
 */
import { test, expect, type Locator, type Page } from "@playwright/test";

function paletteOverlay(page: Page): Locator {
  return page.locator("#palette-overlay");
}

function paletteInput(page: Page): Locator {
  return page.locator("#palette-input");
}

function paletteResultLinks(page: Page): Locator {
  return page.locator("#palette-results ul.results li a[href^='/record/']");
}

function paletteAdrResultLink(page: Page): Locator {
  return page.locator("#palette-results ul.results li a[href^='/record/adr/']");
}

test("Control+K opens the palette, a matching term renders a result, activating it navigates, and Escape closes it", async ({
  page,
}) => {
  await page.goto("/");
  await expect(paletteOverlay(page)).toBeHidden();

  await page.keyboard.press("Control+K");
  await expect(paletteOverlay(page)).toBeVisible();
  await expect(paletteInput(page)).toBeFocused();

  await paletteInput(page).fill("palette");

  await expect(paletteResultLinks(page).first()).toBeVisible();
  const adrResult = paletteAdrResultLink(page).first();
  await expect(adrResult).toBeVisible();

  await adrResult.click();
  await expect(page).toHaveURL(/\/record\/adr\//);

  await page.keyboard.press("Control+K");
  await expect(paletteOverlay(page)).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(paletteOverlay(page)).toBeHidden();
});

test.describe("with JavaScript disabled", () => {
  test.use({ javaScriptEnabled: false });

  test("the plain search form remains the fully functional fallback", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("#palette-overlay")).toBeHidden();

    await page.locator('input[name="q"]').fill("palette");
    await page.getByRole("button", { name: "Search" }).click();

    const adrResult = page.locator('ul.results li a[href^="/record/adr/"]').first();
    await expect(adrResult).toBeVisible();

    await adrResult.click();

    await expect(page).toHaveURL(/\/record\/adr\//);
    await expect(page.locator("main")).toBeVisible();
  });
});
