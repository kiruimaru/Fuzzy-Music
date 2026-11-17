import { invoke } from "@tauri-apps/api/core";

export type Track = {
  id: string;
  title: string;
  author: string;
  artworkUrl: string | null;
  durationMs: number | null;
  streamUrl: string | null;
  source: "soundcloud";
  sourceUrl: string;
};

export type PlaybackStatus = {
  generation: number;
  state: "idle" | "loading" | "playing" | "stopped" | "error";
  trackId: string | null;
  error: string | null;
};

export const api = {
  search: (generation: number, query: string) =>
    invoke<{ tracks: Track[]; nextPageToken: string | null }>("search_tracks", {
      generation, request: { query, limit: 10 },
    }),
  cancelSearch: (generation: number) => invoke<void>("cancel_search", { generation }),
  play: (generation: number, track: Track) =>
    invoke<void>("play_track", {
      generation, request: { source: track.source, trackUrl: track.sourceUrl },
    }),
  stop: (generation: number) => invoke<void>("stop_playback", { generation }),
  status: () => invoke<PlaybackStatus>("playback_status"),
};

export function safeMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "code" in error) {
    const code = (error as { code: unknown }).code;
    switch (code) {
      case "invalid_input": return "Enter a track title or artist.";
      case "timeout": return "The source timed out. Try again.";
      case "cancelled": return "Cancelled.";
      case "source_unavailable": return "Source unavailable. Check yt-dlp or try another track.";
      case "playback_failed": return "Playback failed. Check mpv and your audio output, or try another track.";
    }
  }
  return "Operation failed. Please retry.";
}
