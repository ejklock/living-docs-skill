/**
 * Browser-observable gate for the record page's metadata panel (ADR 0015,
 * issue 0008 slice S3). Drives a live server seeded from the repo's own
 * docs/: ADR 0015 itself is Accepted and tagged [web, ux, frontend], so its
 * own record page is used to assert the aside renders the doc type, a
 * status badge, and a tag chip — without depending on the live corpus
 * containing a supersede pair (that chain is covered by the seeded HTTP
 * tests in web/tests/http.rs).
 */
import { test, expect } from "@playwright/test";

const ADR_0015_PATH =
  "/record/adr/0015-web-ux-follows-the-three-pane-doc-site-archetype-with-search-first-cmd-k-palette.md";

test("the metadata panel shows the doc type, status badge, and tags for a live record", async ({
  page,
}) => {
  await page.goto(ADR_0015_PATH);

  const aside = page.locator("aside");
  await expect(aside).toBeVisible();
  await expect(aside).toContainText("ADR");

  const statusBadge = aside.locator("span.status-badge");
  await expect(statusBadge).toBeVisible();
  await expect(statusBadge).toHaveText("Accepted");

  const tagChip = aside.locator("span.tag", { hasText: "web" });
  await expect(tagChip).toBeVisible();

  await expect(page.locator("nav")).toBeVisible();
  await expect(page.locator("main")).toBeVisible();
});
