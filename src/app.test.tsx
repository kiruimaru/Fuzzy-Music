// @vitest-environment jsdom
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { App } from "./App";
import { api, safeMessage, type Playlist, type Track } from "./api";
import { createMemorySettingsStore, type SettingsStore } from "./settings";

vi.mock("./api", async (original) => ({
  ...await original<typeof import("./api")>(),
  api: {
    search: vi.fn(), cancelSearch: vi.fn().mockResolvedValue(undefined),
    play: vi.fn(), stop: vi.fn().mockResolvedValue(undefined),
    status: vi.fn().mockResolvedValue({ generation: 0, state: "idle" }),
    searchPlaylists: vi.fn(),
    prepare: vi.fn().mockResolvedValue(undefined),
    prefetch: vi.fn().mockResolvedValue(undefined),
    cancelPreparation: vi.fn().mockResolvedValue(undefined),
    playlistTracks: vi.fn(),
    cancelPlaylists: vi.fn().mockResolvedValue(undefined),
    pause: vi.fn().mockResolvedValue(undefined), seek: vi.fn().mockResolvedValue(undefined),
    volume: vi.fn().mockResolvedValue(undefined),
  },
}));

const track: Track = {
  id: "1", title: "Example", author: "Artist", artworkUrl: null,
  durationMs: null, streamUrl: null, source: "soundcloud",
  sourceUrl: "https://soundcloud.com/artist/example",
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

function search(query: string) {
  fireEvent.change(screen.getByLabelText("Track or artist"), { target: { value: query } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
}

afterEach(() => { cleanup(); vi.clearAllMocks(); });
beforeEach(() => {
  vi.resetAllMocks();
  for (const method of [api.cancelSearch, api.stop, api.prepare, api.prefetch,
    api.cancelPreparation, api.cancelPlaylists, api.pause, api.seek, api.volume]) {
    vi.mocked(method).mockResolvedValue(undefined);
  }
  vi.mocked(api.status).mockResolvedValue({
    generation: 0, state: "idle", trackId: null, error: null,
    positionMs: 0, durationMs: null, volume: 100, seekable: false,
  });
});

test("stale search success cannot replace newer results", async () => {
  const old = deferred<{ tracks: Track[]; nextPageToken: null }>();
  vi.mocked(api.search).mockReturnValueOnce(old.promise)
    .mockResolvedValueOnce({ tracks: [{ ...track, title: "New result" }], nextPageToken: null });
  render(<App />);
  search("old");
  search("new");
  await screen.findByText("New result");
  await act(async () => old.resolve({ tracks: [track], nextPageToken: null }));
  expect(screen.queryByText("Example")).toBeNull();
  expect(screen.getByText("New result")).toBeTruthy();
});

test("clear and explicit cancel finish loading and ignore late errors", async () => {
  const old = deferred<{ tracks: Track[]; nextPageToken: null }>();
  vi.mocked(api.search).mockReturnValue(old.promise);
  render(<App />);
  search("old");
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(screen.getByText("Search cancelled.")).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Track or artist"), { target: { value: "" } });
  await act(async () => old.reject({ code: "timeout" }));
  expect(screen.getByText("Ready")).toBeTruthy();
  expect(screen.queryByText("Searching...")).toBeNull();
});

test("empty and safe error allow a new search", async () => {
  vi.mocked(api.search).mockResolvedValueOnce({ tracks: [], nextPageToken: null })
    .mockRejectedValueOnce({ code: "timeout", message: "SECRET" })
    .mockResolvedValueOnce({ tracks: [track], nextPageToken: null });
  render(<App />);
  search("empty");
  await screen.findByText("No tracks found.");
  search("error");
  await screen.findByText("The source timed out. Try again.");
  expect(screen.queryByText("SECRET")).toBeNull();
  search("retry");
  await screen.findByText("Example");
});

test("Stop during resolve ignores late completion and Play can retry", async () => {
  const pending = deferred<void>();
  vi.mocked(api.search).mockResolvedValue({ tracks: [track], nextPageToken: null });
  vi.mocked(api.play).mockReturnValueOnce(pending.promise).mockResolvedValueOnce(undefined);
  render(<App />);
  search("music");
  fireEvent.click(await screen.findByRole("button", { name: "Play Example" }));
  expect(screen.getByText("Resolving...")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Stop" }));
  await screen.findByText("Stopped");
  await act(async () => pending.resolve());
  expect(screen.getByText("Stopped")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Play again" }));
  await waitFor(() => expect(api.play).toHaveBeenCalledTimes(2));
  await screen.findByText("Starting audio...");
});

test("errors never echo arbitrary IPC diagnostics", () => {
  expect(safeMessage("token=SECRET")).not.toContain("SECRET");
  expect(safeMessage({ code: "playback_failed", message: "SECRET" })).not.toContain("SECRET");
});

const playlist: Playlist = {
  id: "9", title: "Chill mix", author: "Lofi Girl", artworkUrl: null,
  trackCount: 2, durationMs: null, source: "soundcloud",
  sourceUrl: "https://soundcloud.com/lofi_girl/sets/chill",
};

test("playlist search and open reuse the same player", async () => {
  vi.mocked(api.searchPlaylists).mockResolvedValue({ playlists: [playlist], nextPageToken: null });
  vi.mocked(api.playlistTracks).mockResolvedValue({
    playlist, tracks: [track], skippedTrackCount: 1,
  });
  vi.mocked(api.play).mockResolvedValue(undefined);
  render(<App />);
  fireEvent.click(screen.getByRole("tab", { name: "Playlists" }));
  fireEvent.change(screen.getByLabelText("Playlist name"), { target: { value: "chill" } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  fireEvent.click(await screen.findByRole("button", { name: "Open playlist Chill mix" }));
  await screen.findByText("Chill mix");
  expect(api.searchPlaylists).toHaveBeenCalledTimes(1);
  expect(api.playlistTracks).toHaveBeenCalledWith(expect.any(Number), playlist);
  expect(screen.getByText("1 track is unavailable and was skipped.")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Play Example" }));
  await waitFor(() => expect(api.play).toHaveBeenCalledTimes(1));
  expect(screen.getByText("Starting audio...")).toBeTruthy();
});

test("playlist errors stay safe and do not show diagnostics", async () => {
  vi.mocked(api.searchPlaylists).mockRejectedValue({ code: "source_unavailable", message: "SECRET" });
  render(<App />);
  fireEvent.click(screen.getByRole("tab", { name: "Playlists" }));
  fireEvent.change(screen.getByLabelText("Playlist name"), { target: { value: "x" } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  await screen.findByText("Source unavailable. Check yt-dlp or try another track.");
  expect(screen.queryByText("SECRET")).toBeNull();
});

test("playlist prefetches its successor, advances on EOF and Stop prevents another advance", async () => {
  const second = { ...track, id: "2", title: "Second", sourceUrl: "https://soundcloud.com/a/second" };
  vi.mocked(api.searchPlaylists).mockResolvedValue({ playlists: [playlist], nextPageToken: null });
  vi.mocked(api.playlistTracks).mockResolvedValue({ playlist, tracks: [track, second], skippedTrackCount: 0 });
  vi.mocked(api.play).mockResolvedValue(undefined);
  let state: "playing" | "stopped" = "playing";
  vi.mocked(api.status).mockImplementation(async () => ({
    generation: vi.mocked(api.play).mock.calls.at(-1)?.[0] ?? 0,
    state, trackId: "1", error: null, positionMs: 170000, durationMs: 180000, volume: 50, seekable: true,
  }));
  render(<App />);
  fireEvent.click(screen.getByRole("tab", { name: "Playlists" }));
  fireEvent.change(screen.getByLabelText("Playlist name"), { target: { value: "chill" } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  fireEvent.click(await screen.findByRole("button", { name: "Open playlist Chill mix" }));
  fireEvent.click(await screen.findByRole("button", { name: "Play playlist" }));
  await waitFor(() => expect(api.prepare).toHaveBeenCalledWith(expect.any(Number), second, expect.any(Number)));
  expect(api.play).toHaveBeenCalledTimes(1);
  state = "stopped";
  await waitFor(() => expect(api.play).toHaveBeenCalledTimes(2));
  expect(vi.mocked(api.play).mock.calls[1][1]).toEqual(second);
  expect(screen.getByText("2 / 2")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Stop" }));
  await screen.findByText("Stopped");
  await act(async () => { await new Promise(resolve => setTimeout(resolve, 350)); });
  expect(api.play).toHaveBeenCalledTimes(2);
  expect(api.cancelPreparation).toHaveBeenCalled();
});

test("failed successor preparation waits for manual selection instead of retrying forever", async () => {
  const second = { ...track, id: "2", sourceUrl: "https://soundcloud.com/a/second" };
  vi.mocked(api.searchPlaylists).mockResolvedValue({ playlists: [playlist], nextPageToken: null });
  vi.mocked(api.playlistTracks).mockResolvedValue({ playlist, tracks: [track, second], skippedTrackCount: 0 });
  vi.mocked(api.play).mockResolvedValue(undefined);
  vi.mocked(api.prepare).mockRejectedValueOnce(new Error("temporary failure")).mockResolvedValue(undefined);
  vi.mocked(api.status).mockImplementation(async () => ({
    generation: vi.mocked(api.play).mock.calls.at(-1)?.[0] ?? 0,
    state: "playing", trackId: "1", error: null, positionMs: 170000,
    durationMs: 180000, volume: 50, seekable: true,
  }));
  render(<App />);
  fireEvent.click(screen.getByRole("tab", { name: "Playlists" }));
  fireEvent.change(screen.getByLabelText("Playlist name"), { target: { value: "chill" } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  fireEvent.click(await screen.findByRole("button", { name: "Open playlist Chill mix" }));
  fireEvent.click(await screen.findByRole("button", { name: "Play playlist" }));
  await waitFor(() => expect(api.prepare).toHaveBeenCalledTimes(1));
  await screen.findByRole("alert");
  await act(async () => { await new Promise(resolve => setTimeout(resolve, 600)); });
  expect(api.prepare).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole("button", { name: "Next track" }));
  await waitFor(() => expect(api.play).toHaveBeenCalledTimes(2));
  expect(vi.mocked(api.play).mock.calls[1][1]).toEqual(second);
});

test("lookahead starts early and includes the next two playlist tracks", async () => {
  const second = { ...track, id: "2", sourceUrl: "https://soundcloud.com/a/second" };
  const third = { ...track, id: "3", sourceUrl: "https://soundcloud.com/a/third" };
  vi.mocked(api.searchPlaylists).mockResolvedValue({ playlists: [playlist], nextPageToken: null });
  vi.mocked(api.playlistTracks).mockResolvedValue({ playlist, tracks: [track, second, third], skippedTrackCount: 0 });
  vi.mocked(api.play).mockResolvedValue(undefined);
  vi.mocked(api.status).mockImplementation(async () => ({
    generation: vi.mocked(api.play).mock.calls.at(-1)?.[0] ?? 0,
    state: "playing", trackId: "1", error: null, positionMs: 1000,
    durationMs: 180000, volume: 50, seekable: true,
  }));
  render(<App />);
  fireEvent.click(screen.getByRole("tab", { name: "Playlists" }));
  fireEvent.change(screen.getByLabelText("Playlist name"), { target: { value: "chill" } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  fireEvent.click(await screen.findByRole("button", { name: "Open playlist Chill mix" }));
  fireEvent.click(await screen.findByRole("button", { name: "Play playlist" }));
  await waitFor(() => expect(api.prefetch).toHaveBeenCalledWith(expect.any(Number), [second, third], expect.any(Number)));
  expect(api.prepare).not.toHaveBeenCalled();
});

test("manual skips keep the existing playlist prefetch running", async () => {
  const pending = deferred<void>();
  const second = { ...track, id: "2", title: "Second", sourceUrl: "https://soundcloud.com/a/second" };
  vi.mocked(api.searchPlaylists).mockResolvedValue({ playlists: [playlist], nextPageToken: null });
  vi.mocked(api.playlistTracks).mockResolvedValue({ playlist, tracks: [track, second], skippedTrackCount: 0 });
  vi.mocked(api.play).mockResolvedValue(undefined);
  vi.mocked(api.prefetch).mockReturnValueOnce(pending.promise);
  vi.mocked(api.status).mockImplementation(async () => ({
    generation: vi.mocked(api.play).mock.calls.at(-1)?.[0] ?? 0,
    state: "playing", trackId: "1", error: null, positionMs: 1000,
    durationMs: 180000, volume: 50, seekable: true,
  }));
  render(<App />);
  fireEvent.click(screen.getByRole("tab", { name: "Playlists" }));
  fireEvent.change(screen.getByLabelText("Playlist name"), { target: { value: "chill" } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  fireEvent.click(await screen.findByRole("button", { name: "Open playlist Chill mix" }));
  fireEvent.click(await screen.findByRole("button", { name: "Play playlist" }));
  await waitFor(() => expect(api.prefetch).toHaveBeenCalledTimes(1));
  fireEvent.click(screen.getByRole("button", { name: "Next track" }));
  await waitFor(() => expect(api.play).toHaveBeenCalledTimes(2));
  await act(async () => { await new Promise(resolve => setTimeout(resolve, 350)); });
  expect(api.prefetch).toHaveBeenCalledTimes(1);
  await act(async () => pending.resolve());
});

test("lookahead prioritizes neighbouring tracks in both directions", async () => {
  const items = Array.from({ length: 7 }, (_, index) => ({
    ...track, id: String(index), title: `Track ${index}`, sourceUrl: `https://soundcloud.com/a/track${index}`,
  }));
  vi.mocked(api.searchPlaylists).mockResolvedValue({ playlists: [playlist], nextPageToken: null });
  vi.mocked(api.playlistTracks).mockResolvedValue({ playlist, tracks: items, skippedTrackCount: 0 });
  vi.mocked(api.play).mockResolvedValue(undefined);
  vi.mocked(api.status).mockImplementation(async () => ({
    generation: vi.mocked(api.play).mock.calls.at(-1)?.[0] ?? 0,
    state: "playing", trackId: "3", error: null, positionMs: 1000,
    durationMs: 180000, volume: 50, seekable: true,
  }));
  render(<App />);
  fireEvent.click(screen.getByRole("tab", { name: "Playlists" }));
  fireEvent.change(screen.getByLabelText("Playlist name"), { target: { value: "chill" } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  fireEvent.click(await screen.findByRole("button", { name: "Open playlist Chill mix" }));
  fireEvent.click(await screen.findByRole("button", { name: "Play Track 3" }));
  await waitFor(() => expect(api.prefetch).toHaveBeenCalledWith(expect.any(Number),
    [items[4], items[2], items[5], items[1], items[6], items[0]], expect.any(Number)));
});

test("lookahead includes forty tracks at the start of a large playlist", async () => {
  const items = Array.from({ length: 45 }, (_, index) => ({
    ...track, id: String(index), title: `Track ${index}`, sourceUrl: `https://soundcloud.com/a/track${index}`,
  }));
  vi.mocked(api.searchPlaylists).mockResolvedValue({ playlists: [playlist], nextPageToken: null });
  vi.mocked(api.playlistTracks).mockResolvedValue({ playlist, tracks: items, skippedTrackCount: 0 });
  vi.mocked(api.play).mockResolvedValue(undefined);
  vi.mocked(api.status).mockImplementation(async () => ({
    generation: vi.mocked(api.play).mock.calls.at(-1)?.[0] ?? 0,
    state: "playing", trackId: "0", error: null, positionMs: 1000,
    durationMs: 180000, volume: 50, seekable: true,
  }));
  render(<App />);
  fireEvent.click(screen.getByRole("tab", { name: "Playlists" }));
  fireEvent.change(screen.getByLabelText("Playlist name"), { target: { value: "chill" } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  fireEvent.click(await screen.findByRole("button", { name: "Open playlist Chill mix" }));
  fireEvent.click(await screen.findByRole("button", { name: "Play playlist" }));
  await waitFor(() => expect(api.prefetch).toHaveBeenCalledWith(expect.any(Number), items.slice(1, 41), expect.any(Number)));
});

test("player transition updates the queue without starting the next track again", async () => {
  const second = { ...track, id: "2", title: "Second", sourceUrl: "https://soundcloud.com/a/second" };
  vi.mocked(api.searchPlaylists).mockResolvedValue({ playlists: [playlist], nextPageToken: null });
  vi.mocked(api.playlistTracks).mockResolvedValue({ playlist, tracks: [track, second], skippedTrackCount: 0 });
  vi.mocked(api.play).mockResolvedValue(undefined);
  let transitions = 0;
  vi.mocked(api.status).mockImplementation(async () => ({
    generation: vi.mocked(api.play).mock.calls.at(-1)?.[0] ?? 0,
    state: "playing", trackId: transitions ? "2" : "1", transitions, error: null,
    positionMs: 170000, durationMs: 180000, volume: 50, seekable: true,
  }));
  render(<App />);
  fireEvent.click(screen.getByRole("tab", { name: "Playlists" }));
  fireEvent.change(screen.getByLabelText("Playlist name"), { target: { value: "chill" } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  fireEvent.click(await screen.findByRole("button", { name: "Open playlist Chill mix" }));
  fireEvent.click(await screen.findByRole("button", { name: "Play playlist" }));
  await waitFor(() => expect(api.prepare).toHaveBeenCalled());
  transitions = 1;
  await screen.findByText("2 / 2");
  expect(api.play).toHaveBeenCalledTimes(1);
});

test("controls use the active playback generation without replaying the track", async () => {
  vi.mocked(api.search).mockResolvedValue({ tracks: [track], nextPageToken: null });
  vi.mocked(api.play).mockResolvedValue(undefined);
  let paused = false;
  vi.mocked(api.status).mockImplementation(async () => ({
    generation: vi.mocked(api.play).mock.calls.at(-1)?.[0] ?? 0,
    state: paused ? "paused" : "playing", trackId: "1", error: null,
    positionMs: 15000, durationMs: 180000, volume: 60, seekable: true,
  }));
  vi.mocked(api.pause).mockImplementation(async (_generation, value) => { paused = value; });
  render(<App />);
  search("music");
  fireEvent.click(await screen.findByRole("button", { name: "Play Example" }));
  await screen.findByText("Playing");
  const generation = vi.mocked(api.play).mock.calls.at(-1)![0];
  fireEvent.click(screen.getByRole("button", { name: "Pause" }));
  await screen.findByText("Paused");
  fireEvent.click(screen.getByRole("button", { name: "Resume" }));
  await screen.findByText("Playing");
  const position = screen.getByLabelText("Position");
  fireEvent.change(position, { target: { value: "45000" } });
  fireEvent.pointerUp(position);
  await waitFor(() => expect(api.seek).toHaveBeenCalledWith(generation, 45000));
  const volume = screen.getByLabelText("Volume");
  fireEvent.change(volume, { target: { value: "25" } });
  fireEvent.keyUp(volume, { key: "ArrowLeft" });
  await waitFor(() => expect(api.volume).toHaveBeenCalledWith(generation, 25));
  expect(api.pause).toHaveBeenNthCalledWith(1, generation, true);
  expect(api.pause).toHaveBeenNthCalledWith(2, generation, false);
  expect(api.play).toHaveBeenCalledTimes(1);
});

const queueTracks = [track, { ...track, id: "2", title: "Second", sourceUrl: "https://soundcloud.com/a/second" },
  { ...track, id: "3", title: "Third", sourceUrl: "https://soundcloud.com/a/third" }];

async function openSessionQueue(items = queueTracks, settings?: SettingsStore) {
  vi.mocked(api.search).mockResolvedValue({ tracks: items, nextPageToken: null });
  vi.mocked(api.play).mockResolvedValue(undefined);
  render(<App settings={settings ?? createMemorySettingsStore()} />);
  search("music");
  for (const item of items) {
    fireEvent.click(await screen.findByRole("button", { name: `Add ${item.title} to queue` }));
  }
  fireEvent.click(screen.getByRole("tab", { name: `Queue (${items.length})` }));
  return screen.getByRole("region", { name: "Session queue" });
}

function mockQueueStatus() {
  const observed = { state: "playing" as "playing" | "paused" | "stopped" | "error", durationMs: 180000 as number | null, seekable: true };
  vi.mocked(api.status).mockImplementation(async () => {
    const latest = vi.mocked(api.play).mock.calls.at(-1);
    return {
      generation: latest?.[0] ?? 0, trackId: latest?.[1].id ?? null,
      state: observed.state, error: observed.state === "error" ? "Playback failed." : null,
      positionMs: 1000, durationMs: observed.durationMs, volume: latest?.[2]?.volume ?? 100,
      seekable: observed.seekable,
    };
  });
  return observed;
}

test("session queue supports selection and boundaries, and survives search and Stop", async () => {
  mockQueueStatus();
  const list = await openSessionQueue();
  expect(api.play).not.toHaveBeenCalled();
  expect(within(list).getAllByRole("listitem")).toHaveLength(3);
  fireEvent.click(within(list).getByRole("button", { name: "Play queued Example" }));
  await screen.findByText("Playing");
  expect((screen.getByRole("button", { name: "Previous track" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Next track" }));
  await screen.findByText("2 / 3");
  fireEvent.click(screen.getByRole("button", { name: "Next track" }));
  await screen.findByText("3 / 3");
  expect((screen.getByRole("button", { name: "Next track" }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Previous track" }));
  await screen.findByText("2 / 3");
  fireEvent.click(screen.getByRole("button", { name: "Stop" }));
  await screen.findByText("Stopped");
  fireEvent.click(screen.getByRole("tab", { name: "Tracks" }));
  search("different search");
  await screen.findByRole("button", { name: "Add Third to queue" });
  fireEvent.click(screen.getByRole("tab", { name: "Queue (3)" }));
  expect(within(screen.getByRole("region", { name: "Session queue" })).getAllByRole("listitem")).toHaveLength(3);
  expect(api.play).toHaveBeenCalledTimes(4);
  expect(api.play).toHaveBeenCalledWith(expect.any(Number), queueTracks[0], { fresh: true, volume: 100 });
  expect(api.prefetch).not.toHaveBeenCalled();
  expect(api.prepare).not.toHaveBeenCalled();
});

test("session EOF advances once per track, then stops without wrapping", async () => {
  const observed = mockQueueStatus();
  const list = await openSessionQueue();
  fireEvent.click(within(list).getByRole("button", { name: "Play queued Example" }));
  await screen.findByText("Playing");
  observed.state = "stopped";
  await waitFor(() => expect(api.play).toHaveBeenCalledTimes(3));
  await screen.findByText("Stopped");
  await act(async () => { await new Promise(resolve => setTimeout(resolve, 550)); });
  expect(api.play).toHaveBeenCalledTimes(3);
  expect(vi.mocked(api.play).mock.calls.map(call => call[1].id)).toEqual(["1", "2", "3"]);
  expect(screen.getByText("3 / 3")).toBeTruthy();
});

test("deleting before and after current keeps playback and the correct successor", async () => {
  mockQueueStatus();
  const list = await openSessionQueue();
  fireEvent.click(within(list).getByRole("button", { name: "Play queued Second" }));
  await screen.findByText("Playing");
  fireEvent.click(within(list).getByRole("button", { name: "Remove Example from queue" }));
  expect(screen.getByText("1 / 2")).toBeTruthy();
  fireEvent.click(within(list).getByRole("button", { name: "Remove Third from queue" }));
  expect(screen.getByText("1 / 1")).toBeTruthy();
  expect(api.stop).not.toHaveBeenCalled();
  expect(api.play).toHaveBeenCalledTimes(1);
  expect((screen.getByRole("button", { name: "Next track" }) as HTMLButtonElement).disabled).toBe(true);
});

test("removing a resolving current item stops and rejects late resolve and old EOF", async () => {
  const list = await openSessionQueue();
  const pending = deferred<void>();
  vi.mocked(api.play).mockReturnValueOnce(pending.promise);
  fireEvent.click(within(list).getByRole("button", { name: "Play queued Example" }));
  const generation = vi.mocked(api.play).mock.calls.at(-1)![0];
  fireEvent.click(within(list).getByRole("button", { name: "Remove Example from queue" }));
  await screen.findByText("Stopped");
  expect(within(list).getAllByRole("listitem")).toHaveLength(2);
  expect(screen.getByText("No track selected")).toBeTruthy();
  vi.mocked(api.status).mockResolvedValue({
    generation, state: "stopped", trackId: "1", positionMs: 0, durationMs: null,
    volume: 100, seekable: false, error: null, transitions: 1,
  });
  await act(async () => { pending.resolve(); await new Promise(resolve => setTimeout(resolve, 550)); });
  expect(api.play).toHaveBeenCalledTimes(1);
  expect(screen.getByText("Stopped")).toBeTruthy();
  fireEvent.click(within(list).getByRole("button", { name: "Play queued Second" }));
  await waitFor(() => expect(api.play).toHaveBeenCalledTimes(2));
});

test("duplicate tracks are separate queue entries; deleting current stops without picking another", async () => {
  mockQueueStatus();
  await openSessionQueue([track]);
  fireEvent.click(screen.getByRole("tab", { name: "Tracks" }));
  search("duplicate");
  fireEvent.click(await screen.findByRole("button", { name: "Add Example to queue" }));
  fireEvent.click(screen.getByRole("tab", { name: "Queue (2)" }));
  const list = screen.getByRole("region", { name: "Session queue" });
  fireEvent.click(within(list).getAllByRole("button", { name: "Play queued Example" })[1]);
  await screen.findByText("2 / 2");
  fireEvent.click(within(list).getAllByRole("button", { name: "Remove Example from queue" })[0]);
  expect(screen.getByText("1 / 1")).toBeTruthy();
  expect(api.stop).not.toHaveBeenCalled();
  fireEvent.click(within(list).getByRole("button", { name: "Remove Example from queue" }));
  await screen.findByText("Stopped");
  expect(screen.getByText("Queue is empty.")).toBeTruthy();
  expect(api.play).toHaveBeenCalledTimes(1);
});

test("track error does not skip or retry and manual Next can recover", async () => {
  const observed = mockQueueStatus();
  const list = await openSessionQueue();
  fireEvent.click(within(list).getByRole("button", { name: "Play queued Example" }));
  await screen.findByText("Playing");
  observed.state = "error";
  await screen.findByText("Playback failed.");
  observed.state = "stopped";
  await act(async () => { await new Promise(resolve => setTimeout(resolve, 550)); });
  expect(api.play).toHaveBeenCalledTimes(1);
  observed.state = "playing";
  fireEvent.click(screen.getByRole("button", { name: "Next track" }));
  await waitFor(() => expect(api.play).toHaveBeenCalledTimes(2));
  expect(vi.mocked(api.play).mock.calls[1][1].id).toBe("2");
});

test("unknown duration or unseekable streams disable the timeline", async () => {
  const observed = mockQueueStatus();
  observed.durationMs = null;
  const list = await openSessionQueue();
  fireEvent.click(within(list).getByRole("button", { name: "Play queued Example" }));
  await screen.findByText("Playing");
  expect((screen.getByLabelText("Position") as HTMLInputElement).disabled).toBe(true);
  expect(screen.getByText("0:01 / --:--")).toBeTruthy();
  observed.durationMs = 180000;
  observed.seekable = false;
  await screen.findByText("0:01 / 3:00");
  expect((screen.getByLabelText("Position") as HTMLInputElement).disabled).toBe(true);
  expect(api.seek).not.toHaveBeenCalled();
});

test("volume is restored before Play, saved, and reused after Stop and another selection", async () => {
  mockQueueStatus();
  const settings = createMemorySettingsStore({ volume: 35 });
  const save = vi.spyOn(settings, "updateSettings");
  const list = await openSessionQueue(queueTracks, settings);
  fireEvent.click(within(list).getByRole("button", { name: "Play queued Example" }));
  await screen.findByText("Playing");
  expect(vi.mocked(api.play).mock.calls[0][2]).toEqual({ fresh: true, volume: 35 });
  const slider = screen.getByLabelText("Volume");
  fireEvent.change(slider, { target: { value: "25" } });
  fireEvent.keyUp(slider, { key: "ArrowLeft" });
  await waitFor(() => expect(save).toHaveBeenCalledWith({ volume: 25 }));
  fireEvent.click(screen.getByRole("button", { name: "Stop" }));
  await screen.findByText("Stopped");
  fireEvent.click(within(list).getByRole("button", { name: "Play queued Third" }));
  await waitFor(() => expect(api.play).toHaveBeenCalledTimes(2));
  expect(vi.mocked(api.play).mock.calls[1][2]?.volume).toBe(25);
  expect(await settings.getSettings()).toEqual({ volume: 25 });
});

test("settings write failures are shown safely and writes are serialized", async () => {
  mockQueueStatus();
  const pending = deferred<{ volume: number }>();
  const settings: SettingsStore = {
    getSettings: vi.fn().mockResolvedValue({ volume: 20 }),
    updateSettings: vi.fn().mockReturnValueOnce(pending.promise).mockRejectedValueOnce("SECRET"),
  };
  const list = await openSessionQueue(queueTracks, settings);
  fireEvent.click(within(list).getByRole("button", { name: "Play queued Example" }));
  await screen.findByText("Playing");
  const slider = screen.getByLabelText("Volume");
  fireEvent.change(slider, { target: { value: "25" } });
  fireEvent.keyUp(slider, { key: "ArrowLeft" });
  await waitFor(() => expect(settings.updateSettings).toHaveBeenCalledTimes(1));
  fireEvent.change(slider, { target: { value: "30" } });
  fireEvent.keyUp(slider, { key: "ArrowLeft" });
  await waitFor(() => expect(api.volume).toHaveBeenCalledTimes(2));
  expect(settings.updateSettings).toHaveBeenCalledTimes(1);
  await act(async () => pending.resolve({ volume: 25 }));
  await screen.findByText("Could not save volume settings.");
  expect(settings.updateSettings).toHaveBeenLastCalledWith({ volume: 30 });
  expect(screen.queryByText("SECRET")).toBeNull();
});

test("a late error from the previously selected queue entry cannot replace the new selection", async () => {
  mockQueueStatus();
  const list = await openSessionQueue();
  const pending = deferred<void>();
  vi.mocked(api.play).mockReturnValueOnce(pending.promise).mockResolvedValueOnce(undefined);
  fireEvent.click(within(list).getByRole("button", { name: "Play queued Example" }));
  const oldGeneration = vi.mocked(api.play).mock.calls[0][0];
  fireEvent.click(within(list).getByRole("button", { name: "Play queued Second" }));
  await screen.findByText("Playing");
  await act(async () => pending.reject({ code: "timeout" }));
  expect(screen.queryByText("The source timed out. Try again.")).toBeNull();
  expect(screen.getByText("2 / 3")).toBeTruthy();
  expect(vi.mocked(api.play).mock.calls[1][0]).toBeGreaterThan(oldGeneration);
  expect(within(list).getAllByRole("listitem")[1].getAttribute("aria-current")).toBe("true");
});

test("initial settings read gates Play and a failed read falls back without overwriting storage", async () => {
  vi.mocked(api.search).mockResolvedValue({ tracks: [track], nextPageToken: null });
  vi.mocked(api.play).mockResolvedValue(undefined);
  const pending = deferred<{ volume: number }>();
  const settings: SettingsStore = {
    getSettings: () => pending.promise,
    updateSettings: vi.fn(),
  };
  render(<App settings={settings} />);
  search("music");
  const button = await screen.findByRole("button", { name: "Play Example" });
  expect((button as HTMLButtonElement).disabled).toBe(true);
  await act(async () => pending.reject("SECRET"));
  await screen.findByText("Could not load volume settings.");
  expect((button as HTMLButtonElement).disabled).toBe(false);
  fireEvent.click(button);
  await waitFor(() => expect(api.play).toHaveBeenCalledWith(expect.any(Number), track, { fresh: true, volume: 100 }));
  expect(settings.updateSettings).not.toHaveBeenCalled();
  expect(screen.queryByText("SECRET")).toBeNull();
});
