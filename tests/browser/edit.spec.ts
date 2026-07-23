/**
 * Browser-observable gate for Atlas's authoring edit route (ADR 0016,
 * issue 0011): a live db-mode server whose docs bundle carries the same
 * starter ADR record `web/tests/http.rs`'s `authoring_fixture` seeds so
 * `DbDocStore::new`'s project-must-exist precondition holds
 * (`adr/0001-starter-record.md`) — this spec edits that same record rather
 * than creating a new one, so it never depends on `create.spec.ts` having
 * run first.
 *
 * One test runs every scenario in sequence against that single record, the
 * same way `create.spec.ts` serializes its own two submissions rather than
 * splitting into independent `test()` blocks that could race a shared live
 * server under Playwright's `fullyParallel` mode:
 *
 * 1. A valid edit commits, bumps `revision`, and the record page renders the
 *    new content (ADR 0016 decision 2, doc-gate on write).
 * 2. An edit whose body references a link target absent from the fixture
 *    (`superseded_by: 9999`) fails the `check` gate deterministically,
 *    re-renders the form with the error and the rejected submission's OWN
 *    content preserved, and the stored record stays exactly what step 1
 *    left it as.
 * 3. A second edit reusing the CAPTURED, now-stale `base_revision` from
 *    before step 1 is rejected with the conflict message, and the
 *    reloaded form shows the CURRENT (step 1's) server content — never the
 *    rejected second submission — per ADR 0016's "reject, never merge".
 */
import { test, expect, type Page } from "@playwright/test";

const STARTER_RECORD_PATH = "adr/0001-starter-record.md";
const EDIT_HREF = `/edit/${STARTER_RECORD_PATH}`;
const RECORD_HREF = `/record/${STARTER_RECORD_PATH}`;

const FIRST_EDIT_MARKER = "Edited via Atlas browser gate — first commit.";
const BROKEN_EDIT_MARKER = "This broken edit must never be stored.";
const STALE_EDIT_MARKER = "This conflicting edit must never be stored.";

const FIRST_EDIT_CONTENT = `---
type: ADR
title: Starter Record
description: Edited via Atlas for the browser gate.
status: Accepted
supersedes:
superseded_by:
tags: []
timestamp: 2026-07-21T00:00:00Z
---

# Starter Record

${FIRST_EDIT_MARKER}
`;

const BROKEN_EDIT_CONTENT = `---
type: ADR
title: Starter Record
description: A broken edit that references a missing target.
status: Superseded
supersedes:
superseded_by: 9999
tags: []
timestamp: 2026-07-21T00:00:00Z
---

# Starter Record

${BROKEN_EDIT_MARKER}
`;

const STALE_EDIT_CONTENT = `---
type: ADR
title: Starter Record
description: A conflicting edit reusing a stale revision.
status: Accepted
supersedes:
superseded_by:
tags: []
timestamp: 2026-07-21T00:00:00Z
---

# Starter Record

${STALE_EDIT_MARKER}
`;

function contentTextarea(page: Page) {
  return page.locator('textarea[name="content"]');
}

function baseRevisionField(page: Page) {
  return page.locator('input[name="base_revision"]');
}

async function overrideBaseRevision(page: Page, value: string): Promise<void> {
  await baseRevisionField(page).evaluate((element, revision) => {
    (element as HTMLInputElement).value = revision;
  }, value);
}

async function submitEditForm(page: Page, content: string): Promise<void> {
  await contentTextarea(page).fill(content);
  await page.getByRole("button", { name: "Save" }).click();
}

test("editing a record: a valid edit commits, a broken-link edit fails the check gate, and a stale-revision edit is rejected without discarding the committed content", async ({
  page,
}) => {
  await page.goto(EDIT_HREF);
  const originalRevision = await baseRevisionField(page).inputValue();

  await submitEditForm(page, FIRST_EDIT_CONTENT);

  await expect(page).toHaveURL(new RegExp(`${RECORD_HREF}$`));
  await expect(page.getByText(FIRST_EDIT_MARKER)).toBeVisible();

  await page.goto(EDIT_HREF);
  await submitEditForm(page, BROKEN_EDIT_CONTENT);

  await expect(page).toHaveURL(new RegExp(`${EDIT_HREF}$`));
  await expect(page.locator("p.form-error")).toBeVisible();
  await expect(contentTextarea(page)).toHaveValue(
    new RegExp(BROKEN_EDIT_MARKER),
  );

  await page.goto(RECORD_HREF);
  await expect(page.getByText(FIRST_EDIT_MARKER)).toBeVisible();
  await expect(page.getByText(BROKEN_EDIT_MARKER)).toHaveCount(0);

  await page.goto(EDIT_HREF);
  await overrideBaseRevision(page, originalRevision);
  await submitEditForm(page, STALE_EDIT_CONTENT);

  await expect(page).toHaveURL(new RegExp(`${EDIT_HREF}$`));
  await expect(page.locator("p.form-error")).toBeVisible();
  await expect(contentTextarea(page)).toHaveValue(
    new RegExp(FIRST_EDIT_MARKER),
  );
  await expect(contentTextarea(page)).not.toHaveValue(
    new RegExp(STALE_EDIT_MARKER),
  );

  await page.goto(RECORD_HREF);
  await expect(page.getByText(FIRST_EDIT_MARKER)).toBeVisible();
  await expect(page.getByText(STALE_EDIT_MARKER)).toHaveCount(0);
});
