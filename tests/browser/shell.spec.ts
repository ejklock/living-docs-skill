/**
 * Browser-observable gate for the three-pane doc-site shell (ADR 0015,
 * issue 0008 slice S2). Drives a live server seeded from the repo's own
 * docs/: the nav tree is visible on the search page, and clicking a nav
 * entry navigates to the record page where the body renders, the nav
 * persists, and the clicked entry carries aria-current="page".
 */
import { test, expect, type Locator, type Page } from "@playwright/test";

function navGroupHeadings(page: Page): Locator {
  return page.locator("nav h2");
}

function navLinks(page: Page): Locator {
  return page.locator('nav a[href^="/record/"]');
}

test("the nav tree is visible and navigating a record entry keeps it present with the active entry marked", async ({
  page,
}) => {
  await page.goto("/");

  await expect(page.locator("nav")).toBeVisible();
  await expect(navGroupHeadings(page).first()).toBeVisible();

  const firstLink = navLinks(page).first();
  await expect(firstLink).toBeVisible();
  const targetHref = await firstLink.getAttribute("href");
  expect(targetHref).toBeTruthy();

  await firstLink.click();

  await expect(page).toHaveURL(/\/record\//);
  await expect(page.locator("main h1").first()).toBeVisible();
  await expect(page.locator("nav")).toBeVisible();

  const activeLink = page.locator(`nav a[href="${targetHref}"]`);
  await expect(activeLink).toHaveAttribute("aria-current", "page");
});
