// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { App } from "./App";
import { api, safeMessage, type Playlist, type Track } from "./api";

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

test("failed preparation is retried while the same track is playing", async () => {
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
  await waitFor(() => expect(api.prepare).toHaveBeenCalledTimes(2), { timeout: 4000 });
  expect(vi.mocked(api.prepare).mock.calls[1][1]).toEqual(second);
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
