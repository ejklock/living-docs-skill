/**
 * Browser-observable gate for Atlas's authoring delete route (ADR 0018,
 * issue 0013 slice B): a live db-mode server whose docs bundle carries the
 * same fixture `web/tests/http.rs`'s own `delete_fixture` seeds — an
 * eligible, relation-free issue (`issues/0001-eligible-issue.md`) and an
 * ineligible ADR (`adr/0001-ineligible-adr.md`) — mirroring
 * `supersede.spec.ts`'s own precedent of driving a spec straight off the
 * equivalent Rust fixture's record shapes.
 *
 * One test runs both scenarios in sequence, the same way `supersede.spec.ts`
 * serializes its own scenarios rather than splitting into independent
 * `test()` blocks that could race a shared live server under Playwright's
 * `fullyParallel` mode:
 *
 * 1. Submitting the delete confirm form on the eligible issue commits: the
 *    browser lands back on the record's own page, its "Deleted" badge is
 *    visible, and reloading the page shows no delete form (ADR 0018: a
 *    soft-deleted record stays viewable but is never delete-eligible again).
 * 2. Submitting the delete confirm form on the ineligible ADR is rejected:
 *    the browser stays on `/delete/{path}`, the error is visible, and
 *    revisiting the ADR's own page shows no "Deleted" badge.
 */
import { test, expect, type Page } from "@playwright/test";

const ELIGIBLE_RECORD_PATH = "issues/0001-eligible-issue.md";
const INELIGIBLE_RECORD_PATH = "adr/0001-ineligible-adr.md";
const ELIGIBLE_RECORD_HREF = `/record/${ELIGIBLE_RECORD_PATH}`;
const INELIGIBLE_RECORD_HREF = `/record/${INELIGIBLE_RECORD_PATH}`;
const INELIGIBLE_DELETE_HREF = `/delete/${INELIGIBLE_RECORD_PATH}`;

function deleteFormButton(page: Page) {
  return page.locator("form.delete-form").getByRole("button", { name: "Delete" });
}

async function submitDeleteForm(page: Page): Promise<void> {
  await deleteFormButton(page).click();
}

test("deleting a record: an eligible relation-free issue commits and shows the Deleted badge, then an ineligible ADR is rejected without a badge change", async ({
  page,
}) => {
  await page.goto(ELIGIBLE_RECORD_HREF);

  await submitDeleteForm(page);

  await expect(page).toHaveURL(new RegExp(`${ELIGIBLE_RECORD_HREF}$`));
  await expect(page.locator("aside span.status-badge")).toHaveText("Deleted");
  await expect(page.locator("form.delete-form")).toHaveCount(0);

  await page.goto(ELIGIBLE_RECORD_HREF);
  await expect(page.locator("aside span.status-badge")).toHaveText("Deleted");
  await expect(page.locator("form.delete-form")).toHaveCount(0);

  await page.goto(INELIGIBLE_RECORD_HREF);
  await submitDeleteForm(page);

  await expect(page).toHaveURL(new RegExp(`${INELIGIBLE_DELETE_HREF}$`));
  await expect(page.locator("p.form-error")).toBeVisible();

  await page.goto(INELIGIBLE_RECORD_HREF);
  await expect(page.locator("aside span.status-badge")).not.toHaveText(
    "Deleted",
  );
});
