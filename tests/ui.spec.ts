import { expect, test } from "@playwright/test";

for (const viewport of [{ width: 1000, height: 720 }, { width: 320, height: 640 }]) {
  test(`search, Play and Stop at ${viewport.width}px with fixture IPC`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.addInitScript(() => {
      let status = { generation: 0, state: "idle", trackId: null as string | null, error: null,
        positionMs: 0, durationMs: 180000, volume: 100, seekable: true };
      Object.assign(window, {
        __TAURI_INTERNALS__: {
          invoke: async (command: string, args: { generation: number; action: { type: string; paused: boolean; volume: number; position_ms: number } }) => {
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
              ...status, generation: args.generation, state: "playing", trackId: "1", error: null,
            };
            if (command === "stop_playback") status = {
              ...status, generation: args.generation, state: "stopped", trackId: null, error: null,
            };
            if (command === "playback_status") return status;
            if (command === "search_playlists") return {
              playlists: [{
                id: "9", title: "Chill mix", author: "Lofi Girl", artworkUrl: null,
                trackCount: 51, durationMs: null, source: "soundcloud",
                sourceUrl: "https://soundcloud.com/lofi_girl/sets/chill",
              }],
              nextPageToken: null,
            };
            if (command === "playlist_tracks") return {
              playlist: {
                id: "9", title: "Chill mix", author: "Lofi Girl", artworkUrl: null,
                trackCount: 51, durationMs: null, source: "soundcloud",
                sourceUrl: "https://soundcloud.com/lofi_girl/sets/chill",
              },
              tracks: [{
                id: "1", title: "Rook", author: "Yasumu", source: "soundcloud",
                sourceUrl: "https://soundcloud.com/yasumu87442/rook",
                artworkUrl: null, durationMs: 110000, streamUrl: null,
              }],
              skippedTrackCount: 0,
            };
            if (command === "cancel_playlists") return;
            if (command === "update_playback") {
              if (args.action.type === "pause") status.state = args.action.paused ? "paused" : "playing";
              if (args.action.type === "volume") status.volume = args.action.volume;
              if (args.action.type === "seek") status.positionMs = args.action.position_ms;
            }
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
    await page.getByRole("button", { name: "Pause", exact: true }).click();
    await expect(page.getByText("Paused", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Resume", exact: true }).click();
    await expect(page.getByText("Playing", { exact: true })).toBeVisible();
    await page.getByLabel("Volume").fill("40");
    await page.getByLabel("Volume").press("ArrowRight");
    await expect(page.locator(".volume output")).toHaveText("41%");
    await page.getByLabel("Position").fill("30000");
    await page.getByLabel("Position").press("ArrowRight");
    await expect(page.locator(".timeline output")).toHaveText("0:31 / 3:00");
    await page.getByRole("button", { name: "Stop", exact: true }).click();
    await expect(page.getByText("Stopped", { exact: true })).toBeVisible();
    await expect.poll(() => page.evaluate(() =>
      document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await page.screenshot({ path: `test-results/search-${viewport.width}.png`, fullPage: true });
  });
}

test("playlist search and open at 1000px with fixture IPC", async ({ page }) => {
  await page.setViewportSize({ width: 1000, height: 720 });
  await page.addInitScript(() => {
    let status = { generation: 0, state: "idle", trackId: null as string | null, error: null,
      positionMs: 0, durationMs: null, volume: 100, seekable: false };
    const playlist = {
      id: "9", title: "Chill mix", author: "Lofi Girl", artworkUrl: null,
      trackCount: 51, durationMs: null, source: "soundcloud",
      sourceUrl: "https://soundcloud.com/lofi_girl/sets/chill",
    };
    Object.assign(window, {
      __TAURI_INTERNALS__: {
        invoke: async (command: string) => {
          if (command === "search_playlists") return { playlists: [playlist], nextPageToken: null };
          if (command === "playlist_tracks") return {
            playlist,
            tracks: [{
              id: "1", title: "Rook", author: "Yasumu", source: "soundcloud",
              sourceUrl: "https://soundcloud.com/yasumu87442/rook",
              artworkUrl: null, durationMs: 110000, streamUrl: null,
            }],
            skippedTrackCount: 2,
          };
          if (command === "playback_status") return status;
          return undefined;
        },
      },
    });
  });
  await page.goto("/");
  await page.getByRole("tab", { name: "Playlists" }).click();
  await page.getByLabel("Playlist name").fill("chill");
  await page.getByRole("button", { name: "Search", exact: true }).click();
  await expect(page.getByRole("button", { name: "Open playlist Chill mix" })).toBeVisible();
  await page.getByRole("button", { name: "Open playlist Chill mix" }).click();
  await expect(page.getByText("Lofi Girl · 1 of 51 tracks")).toBeVisible();
  await expect(page.getByText("2 tracks are unavailable and were skipped.")).toBeVisible();
  await expect(page.getByText("Rook", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: /Back to playlists/ })).toBeVisible();
  await expect.poll(() => page.evaluate(() =>
    document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await page.screenshot({ path: "test-results/playlists-1000.png", fullPage: true });
});

test("player bar stays visible with a long playlist", async ({ page }) => {
  await page.setViewportSize({ width: 1000, height: 620 });
  await page.addInitScript(() => {
    const playlist = {
      id: "9", title: "Long mix", author: "Lofi Girl", artworkUrl: null,
      trackCount: 60, durationMs: null, source: "soundcloud",
      sourceUrl: "https://soundcloud.com/lofi_girl/sets/long",
    };
    const tracks = Array.from({ length: 60 }, (_, index) => ({
      id: String(index + 1), title: `Track ${index + 1}`, author: "Artist",
      source: "soundcloud", sourceUrl: `https://soundcloud.com/a/t${index + 1}`,
      artworkUrl: null, durationMs: 180000, streamUrl: null,
    }));
    Object.assign(window, {
      __TAURI_INTERNALS__: {
        invoke: async (command: string) => {
          if (command === "search_playlists") return { playlists: [playlist], nextPageToken: null };
          if (command === "playlist_tracks") return { playlist, tracks, skippedTrackCount: 0 };
          if (command === "playback_status") return {
            generation: 0, state: "idle", trackId: null, error: null,
            positionMs: 0, durationMs: null, volume: 100, seekable: false,
          };
          return undefined;
        },
      },
    });
  });
  await page.goto("/");
  await page.getByRole("tab", { name: "Playlists" }).click();
  await page.getByLabel("Playlist name").fill("long");
  await page.getByRole("button", { name: "Search", exact: true }).click();
  await page.getByRole("button", { name: "Open playlist Long mix" }).click();
  await expect(page.getByText("Track 60", { exact: true })).toBeAttached();

  const player = page.getByRole("contentinfo", { name: "Playback" });
  await expect(player).toBeVisible();
  const box = await player.boundingBox();
  const viewport = page.viewportSize()!;
  expect(box).not.toBeNull();
  expect(box!.y + box!.height).toBeLessThanOrEqual(viewport.height + 1);
  expect(box!.height).toBeGreaterThan(80);

  // The list scrolls inside the content pane, not by pushing the player away.
  const scrolled = await page.evaluate(() => {
    const content = document.querySelector(".content") as HTMLElement | null;
    if (!content) return null;
    content.scrollTop = content.scrollHeight;
    return { scrollable: content.scrollHeight > content.clientHeight, top: content.scrollTop };
  });
  expect(scrolled?.scrollable).toBe(true);
  expect(scrolled!.top).toBeGreaterThan(0);
  await expect(player).toBeVisible();
  await expect(page.getByRole("button", { name: "Stop" })).toBeVisible();
  await page.screenshot({ path: "test-results/long-playlist-1000.png" });
});
