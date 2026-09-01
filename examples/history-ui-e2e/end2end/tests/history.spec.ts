import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

test.describe("pw-history-gate", () => {
  test("pw-history-unauth-gated-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "anonymous");
    await page.goto(`/history/e2e_history_source_owned/${seeded.owned_source_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("record-history-access-denied")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByText(seeded.owned_change)).toHaveCount(0);
  });
});

test.describe("pw-history-acl", () => {
  test("pw-history-owner-timeline-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "owner");
    await page.goto(`/history/e2e_history_source_owned/${seeded.owned_source_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("history-e2e-timeline-page")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("record-history-timeline")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByText(seeded.owned_change)).toBeVisible({ timeout: 30_000 });
  });

  test("pw-history-peer-guessed-id-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "outsider");
    await page.goto(`/history/e2e_history_source_owned/${seeded.owned_source_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("record-history-access-denied")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByText(seeded.owned_change)).toHaveCount(0);
  });

  test("pw-history-auth-public-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "outsider");
    await page.goto(`/history/e2e_history_source_a/${seeded.public_source_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("record-history-timeline")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByText(seeded.public_change)).toBeVisible({ timeout: 30_000 });
  });
});

test.describe("pw-history-workflows", () => {
  test("pw-history-empty-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "outsider", { scenario: "empty" });
    await page.goto(`/history/e2e_history_source_b/${seeded.empty_source_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("record-history-access-denied")).toBeHidden({
      timeout: 60_000,
    });
    await expect(page.getByTestId("record-history-timeline")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("history-empty-default")).toBeVisible({ timeout: 30_000 });
    await expect(page.locator("[data-history-entry-id]")).toHaveCount(0);
  });

  test("pw-history-load-failed-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "outsider", { scenario: "load_fail" });
    await page.goto(`/history/e2e_history_source_a/${seeded.public_source_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("record-history-load-failed")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("record-history-access-denied")).toBeHidden();
    await expect(page.getByText(seeded.public_change)).toHaveCount(0);
    await expect(page.locator("[data-history-entry-id]")).toHaveCount(0);
  });

  // Run renderers early: custom kind_views exercise more hydrate surface than
  // stock timelines, and late-suite WASM boot flakes are more common on CI.
  test("pw-history-renderers-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "outsider", { scenario: "multikind" });
    await page.goto(`/history/renderers/e2e_history_source_a/${seeded.multikind_source_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("history-e2e-renderers-page")).toBeVisible({
      timeout: 60_000,
    });
    const fixtureEntry = page.locator("[data-history-entry-id='e2e-multikind-fixture']");
    const altEntry = page.locator("[data-history-entry-id='e2e-multikind-alt']");
    await expect(fixtureEntry.getByTestId("e2e-fixture-custom-row")).toBeVisible({
      timeout: 30_000,
    });
    await expect(fixtureEntry.getByText("Custom renderer")).toBeVisible();
    await expect(fixtureEntry.getByText(seeded.fixture_kind_marker)).toBeVisible();
    await expect(altEntry).toBeVisible();
    await expect(altEntry.getByText(seeded.alt_kind_marker)).toBeVisible();
    await expect(altEntry.getByText("Custom renderer")).toHaveCount(0);
    await expect(altEntry.getByTestId("e2e-fixture-custom-row")).toHaveCount(0);
    await expect(page.getByTestId("e2e-fixture-custom-row")).toHaveCount(1);
  });

  test("pw-history-page-scroll-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "outsider", { scenario: "multipage" });
    await page.goto(`/history/e2e_history_source_a/${seeded.multipage_source_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const timeline = page.getByTestId("record-history-timeline");
    await expect(timeline).toBeVisible({ timeout: 60_000 });
    // Newest-first: page-row-34 is on the first page; page-row-0 requires a second page.
    await expect(timeline.getByText("page-row-34")).toBeVisible({ timeout: 30_000 });
    await expect(timeline.getByText(seeded.page_late_marker)).toHaveCount(0);

    const scroll = timeline.locator(".orbital-history__scroll");
    for (let i = 0; i < 20; i++) {
      await scroll.evaluate((el) => {
        el.scrollTop = el.scrollHeight;
      });
      const late = await timeline.getByText(seeded.page_late_marker).count();
      if (late > 0) {
        break;
      }
      await page.waitForTimeout(250);
    }
    await expect(timeline.getByText(seeded.page_late_marker)).toBeVisible({ timeout: 60_000 });
  });

  test("pw-history-kind-filter-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "outsider", { scenario: "multikind" });
    await page.goto(`/history/kind/e2e_history_source_a/${seeded.multikind_source_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("history-e2e-kind-filter-page")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByText(seeded.fixture_kind_marker)).toBeVisible({ timeout: 30_000 });
    await expect(page.getByText(seeded.alt_kind_marker)).toHaveCount(0);
    await expect(page.getByTestId("record-history-access-denied")).toBeHidden();
  });

  test("pw-history-kind-absent-empty-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "outsider", { scenario: "multikind" });
    await page.goto(`/history/kind-absent/e2e_history_source_a/${seeded.multikind_source_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("history-e2e-kind-absent-page")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("record-history-access-denied")).toBeHidden();
    await expect(page.getByTestId("history-empty-default")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByText(seeded.fixture_kind_marker)).toHaveCount(0);
    await expect(page.getByText(seeded.alt_kind_marker)).toHaveCount(0);
  });
});
