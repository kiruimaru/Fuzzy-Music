import { expect, test } from "@playwright/test";

for (const viewport of [{ width: 1000, height: 720 }, { width: 320, height: 640 }]) {
  test(`search, Play and Stop at ${viewport.width}px with fixture IPC`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.addInitScript(() => {
      let status = { generation: 0, state: "idle", trackId: null as string | null, error: null };
      Object.assign(window, {
        __TAURI_INTERNALS__: {
          invoke: async (command: string, args: { generation: number }) => {
            if (command === "search_tracks") return {
              tracks: [{
                id: "1", title: "A very long track title with Supercalifragilisticexpialidocious",
                author: "Example artist", source: "soundcloud",
                sourceUrl: "https://soundcloud.com/a/b",
                artworkUrl: null, durationMs: null, streamUrl: null,
              }],
              nextPageToken: null,
            };
            if (command === "play_track") status = {
              generation: args.generation, state: "playing", trackId: "1", error: null,
            };
            if (command === "stop_playback") status = {
              generation: args.generation, state: "stopped", trackId: null, error: null,
            };
            if (command === "playback_status") return status;
          },
        },
      });
    });
    await page.goto("/");
    await page.getByLabel("Track or artist").fill("music");
    await page.getByRole("button", { name: "Search", exact: true }).click();
    await expect(page.getByText("Example artist", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: /^Play A very/ }).click();
    await expect(page.getByText("Playing", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Stop", exact: true }).click();
    await expect(page.getByText("Stopped", { exact: true })).toBeVisible();
    await expect.poll(() => page.evaluate(() =>
      document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await page.screenshot({ path: `test-results/search-${viewport.width}.png`, fullPage: true });
  });
}
