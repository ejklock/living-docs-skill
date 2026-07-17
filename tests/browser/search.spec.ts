/**
 * Browser-observable gate for the read-only search and record pages
 * (ADR 0006, issue 0003 slice S3c). Drives a live server seeded from the
 * repo's own docs/: search -> open a result -> record body renders, then
 * a non-matching term renders the empty state instead of an error page.
 */
import { test, expect, type Locator, type Page } from "@playwright/test";

async function submitSearch(page: Page, term: string): Promise<void> {
  await page.locator('input[name="q"]').fill(term);
  await page.getByRole("button", { name: "Search" }).click();
}

function resultLinks(page: Page): Locator {
  return page.locator('ul.results li a[href^="/record/"]');
}

test("search, open a record, then see the empty state for a non-matching term", async ({
  page,
}) => {
  await page.goto("/");

  await submitSearch(page, "adr");

  const firstResult = resultLinks(page).first();
  await expect(firstResult).toBeVisible();

  await firstResult.click();

  await expect(page).toHaveURL(/\/record\//);
  await expect(page.getByRole("link", { name: "← Back to search" })).toBeVisible();
  await expect(page.locator("h1, p").first()).toBeVisible();

  await page.goto("/");

  await submitSearch(page, "zxqvwmplkjhgfd");

  const emptyState = page.locator("p.empty-state");
  await expect(emptyState).toBeVisible();
  await expect(emptyState).toContainText("No results for");
  await expect(resultLinks(page)).toHaveCount(0);
});
