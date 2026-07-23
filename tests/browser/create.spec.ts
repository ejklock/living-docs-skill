/**
 * Browser-observable gate for Atlas's authoring create route (ADR 0016,
 * issue 0010 slice 3): a live db-mode server whose docs bundle carries the
 * same check-passing fixture `cli/tests/db_authoring.rs` seeds for its own
 * `--backend db new` tests — a bundle-root `index.md` linking to
 * `adr/index.md`, which pre-lists this spec's own happy-path slug
 * (`0002-<CREATE_TITLE slugified>`), plus the ADR template's own
 * placeholder link targets (`research/NNNN-<slug>.md`, `prd/NNNN-<slug>.md`,
 * `adr/url`) so `write_checked`'s in-transaction `check` has something to
 * resolve against, and a starter ADR record so the default project already
 * exists before authoring (`DbDocStore::new`'s own precondition).
 *
 * Two submissions run in the same test: the first selects `doc_type=adr`,
 * lands on the fixture's pre-listed slug, and commits. The second selects
 * `doc_type=issue` instead — the issue template references
 * `/adr/NNNN-<slug>.md`, a link target the fixture's placeholder seeding
 * never covers, so `check` rejects it deterministically regardless of
 * `adr/index.md` freshness. `write_checked` rolls back and the form
 * re-renders with the violation visible and no navigation, mirroring the
 * equivalent Rust coverage in `web/tests/http.rs`.
 */
import { test, expect, type Page } from "@playwright/test";

const CREATE_TITLE = "Atlas Create Smoke Test";
const REJECTED_ISSUE_TITLE = "Atlas Create Smoke Test Issue";

async function submitCreateForm(
  page: Page,
  docType: string,
  title: string,
): Promise<void> {
  await page.goto("/new");
  await page.locator('select[name="doc_type"]').selectOption(docType);
  await page.locator('input[name="title"]').fill(title);
  await page.getByRole("button", { name: "Create" }).click();
}

test("creating a record persists it and opens its page, then an issue submission fails the check gate and re-renders the form", async ({
  page,
}) => {
  await submitCreateForm(page, "adr", CREATE_TITLE);

  await expect(page).toHaveURL(/\/record\/adr\//);
  await expect(page.getByText(CREATE_TITLE).first()).toBeVisible();

  await submitCreateForm(page, "issue", REJECTED_ISSUE_TITLE);

  await expect(page).toHaveURL(/\/new$/);
  await expect(page.locator('input[name="title"]')).toHaveValue(
    REJECTED_ISSUE_TITLE,
  );
  await expect(page.locator("p.form-error")).toBeVisible();
});
