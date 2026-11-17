// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { App } from "./App";
import { api, safeMessage, type Track } from "./api";

vi.mock("./api", async (original) => ({
  ...await original<typeof import("./api")>(),
  api: {
    search: vi.fn(), cancelSearch: vi.fn().mockResolvedValue(undefined),
    play: vi.fn(), stop: vi.fn().mockResolvedValue(undefined),
    status: vi.fn().mockResolvedValue({ generation: 0, state: "idle" }),
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
