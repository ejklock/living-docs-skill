/**
 * Browser-observable gate for Atlas's authoring supersede route (ADR 0016,
 * issue 0012): a live db-mode server whose docs bundle carries the same
 * starter ADR record `web/tests/http.rs`'s `authoring_fixture` seeds
 * (`adr/0001-starter-record.md`, per `edit.spec.ts`'s own precedent) plus a
 * second record this spec supersedes it into
 * (`adr/0002-second-starter-record.md`) — mirroring
 * `web/tests/http.rs`'s own `supersede_fixture`, which seeds exactly this
 * pair for the equivalent Rust coverage.
 *
 * One test runs both scenarios in sequence against that single pair, the
 * same way `edit.spec.ts` serializes its own scenarios rather than
 * splitting into independent `test()` blocks that could race a shared live
 * server under Playwright's `fullyParallel` mode:
 *
 * 1. Submitting the supersede confirm form with the second record's number
 *    commits: the browser lands back on the old record's page, its status
 *    badge and supersede chain reflect the new link, and the new record's
 *    own page links back under "Supersedes" (ADR 0016, doc-gate on write).
 * 2. A second supersede attempt against a target number no record has is
 *    rejected: the old record's page re-renders with the error visible and
 *    the rejected submission's own value preserved in the form, and
 *    revisiting both record pages shows the chain from step 1 unchanged.
 */
import { test, expect, type Page } from "@playwright/test";

const OLD_RECORD_PATH = "adr/0001-starter-record.md";
const NEW_RECORD_PATH = "adr/0002-second-starter-record.md";
const OLD_RECORD_HREF = `/record/${OLD_RECORD_PATH}`;
const NEW_RECORD_HREF = `/record/${NEW_RECORD_PATH}`;
const SUPERSEDE_HREF = `/supersede/${OLD_RECORD_PATH}`;

function supersedeNumberInput(page: Page) {
  return page.locator('form.supersede-form input[name="new"]');
}

async function submitSupersedeForm(page: Page, number: string): Promise<void> {
  await supersedeNumberInput(page).fill(number);
  await page.getByRole("button", { name: "Supersede" }).click();
}

test("superseding a record: a valid supersede commits and both pages reflect the new chain, then a nonexistent target is rejected without discarding the committed chain", async ({
  page,
}) => {
  await page.goto(OLD_RECORD_HREF);

  await submitSupersedeForm(page, "0002");

  await expect(page).toHaveURL(new RegExp(`${OLD_RECORD_HREF}$`));
  await expect(page.locator("aside span.status-badge")).toHaveText(
    "Superseded",
  );
  await expect(
    page.locator(`aside a[href="${NEW_RECORD_HREF}"]`),
  ).toBeVisible();

  await page.goto(NEW_RECORD_HREF);
  await expect(page.getByText("Supersedes")).toBeVisible();
  await expect(
    page.locator(`aside a[href="${OLD_RECORD_HREF}"]`),
  ).toBeVisible();

  await page.goto(OLD_RECORD_HREF);
  await submitSupersedeForm(page, "9999");

  await expect(page).toHaveURL(new RegExp(`${SUPERSEDE_HREF}$`));
  await expect(page.locator("p.form-error")).toBeVisible();
  await expect(supersedeNumberInput(page)).toHaveValue("9999");

  await page.goto(OLD_RECORD_HREF);
  await expect(page.locator("aside span.status-badge")).toHaveText(
    "Superseded",
  );
  await expect(
    page.locator(`aside a[href="${NEW_RECORD_HREF}"]`),
  ).toBeVisible();

  await page.goto(NEW_RECORD_HREF);
  await expect(page.getByText("Supersedes")).toBeVisible();
  await expect(
    page.locator(`aside a[href="${OLD_RECORD_HREF}"]`),
  ).toBeVisible();
});
