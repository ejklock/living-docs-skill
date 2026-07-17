/**
 * Browser-observable gate for the web project filter (ADR 0005, issue 0005
 * slice 0005-C2). Drives a live server seeded from a read-model with two or
 * more projects: the filter lists every seeded project, narrowing to one
 * scopes the results and labels each with its project, and a non-matching
 * term within that scope renders the empty state instead of an error page.
 *
 * The seeded fixture is not assumed to contain any specific project names —
 * the spec discovers a project that has at least one hit for "adr" (the
 * same term the unscoped search spec already relies on matching this
 * corpus's ADR records) and scopes to it.
 */
import { test, expect, type Locator, type Page } from "@playwright/test";

function projectSelect(page: Page): Locator {
  return page.locator('select[name="project"]');
}

function resultItems(page: Page): Locator {
  return page.locator("ul.results li");
}

function projectLabels(page: Page): Locator {
  return page.locator("ul.results li span.project-label");
}

async function submitSearch(page: Page, term: string): Promise<void> {
  await page.locator('input[name="q"]').fill(term);
  await page.getByRole("button", { name: "Search" }).click();
}

test("project filter lists seeded projects, scopes results, and labels the empty state", async ({
  page,
}) => {
  await page.goto("/");

  const select = projectSelect(page);
  await expect(select).toBeVisible();
  const projectOptions = select.locator("option:not([value=''])");
  const projectCount = await projectOptions.count();
  expect(projectCount).toBeGreaterThanOrEqual(2);

  await submitSearch(page, "adr");

  const firstLabel = projectLabels(page).first();
  await expect(firstLabel).toBeVisible();
  const targetProject = await firstLabel.textContent();
  expect(targetProject).toBeTruthy();

  await select.selectOption(targetProject as string);
  await submitSearch(page, "adr");

  await expect(select).toHaveValue(targetProject as string);

  const scopedCount = await resultItems(page).count();
  expect(scopedCount).toBeGreaterThan(0);
  const scopedLabels = await projectLabels(page).allTextContents();
  for (const label of scopedLabels) {
    expect(label).toBe(targetProject);
  }

  await submitSearch(page, "zxqvwmplkjhgfd");

  await expect(select).toHaveValue(targetProject as string);
  const emptyState = page.locator("p.empty-state");
  await expect(emptyState).toBeVisible();
  await expect(emptyState).toContainText("No results for");
  await expect(resultItems(page)).toHaveCount(0);
});
