import { test as base, expect, type Page } from "@playwright/test";

export type SeedAuthKind = "anonymous" | "owner" | "outsider";
export type SeedScenario =
  | "default"
  | "empty"
  | "multipage"
  | "multikind"
  | "load_fail";

export type SeedResponse = {
  ok: boolean;
  auth: string;
  scenario: string;
  fault: boolean;
  owned_source_id: string;
  public_source_id: string;
  owned_change: string;
  public_change: string;
  empty_source_id: string;
  multipage_source_id: string;
  multikind_source_id: string;
  page_size: number;
  row_count: number;
  fixture_kind_marker: string;
  alt_kind_marker: string;
  page_late_marker: string;
};

export async function seedAuth(
  page: Page,
  auth: SeedAuthKind,
  options: { scenario?: SeedScenario; fault?: boolean } = {},
) {
  const res = await page.request.post("/api/test/seed-data", {
    data: {
      auth,
      scenario: options.scenario ?? "default",
      fault: options.fault ?? false,
    },
  });
  expect(res.ok()).toBeTruthy();
  return res.json() as Promise<SeedResponse>;
}

async function bootState(page: Page): Promise<"ready" | "error" | "loading"> {
  return page.evaluate(() => {
    const html = document.documentElement;
    if (html.getAttribute("data-orbital-boot-state") === "error") {
      return "error";
    }
    if (html.getAttribute("data-orbital-hydrated") === "true") {
      return "ready";
    }
    return "loading";
  });
}

/** Wait until Orbital boot overlay dismisses (WASM hydrate + `hide_boot_loader`). */
export async function waitForHydrated(page: Page) {
  page.on("pageerror", (err) => {
    console.log(`pageerror: ${err.message}`);
  });
  page.on("console", (msg) => {
    if (msg.type() === "error") {
      console.log(`console.${msg.type()}: ${msg.text()}`);
    }
  });
  page.on("response", async (res) => {
    if (res.status() < 400) {
      return;
    }
    const body = await res.text().catch(() => "");
    console.log(`http ${res.status()} ${res.url()} ${body.slice(0, 400)}`);
  });

  // Large WASM graphs occasionally fail the first load on CI; reload once on boot error.
  for (let attempt = 0; attempt < 2; attempt++) {
    try {
      await expect
        .poll(async () => bootState(page), { timeout: 240_000 })
        .toBe("ready");
      break;
    } catch (err) {
      const state = await bootState(page).catch(() => "loading");
      const panel = await page
        .getByTestId("orbital-boot-error")
        .innerText()
        .catch(() => "");
      console.log(`hydrate wait failed (attempt ${attempt + 1}): state=${state}`);
      if (panel) {
        console.log(`orbital-boot-error: ${panel.slice(0, 800)}`);
      }
      if (state === "error" && attempt === 0) {
        await page.reload({ waitUntil: "domcontentloaded" });
        continue;
      }
      throw err;
    }
  }

  await expect(page.getByTestId("orbital-boot-overlay")).toHaveCount(0, {
    timeout: 60_000,
  });
}

export const test = base;
export { expect };
