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
  state: "idle" | "loading" | "playing" | "paused" | "stopped" | "error";
  trackId: string | null;
  positionMs: number;
  durationMs: number | null;
  volume: number;
  seekable: boolean;
  error: string | null;
};

export type Playlist = {
  id: string;
  title: string;
  author: string;
  artworkUrl: string | null;
  trackCount: number | null;
  durationMs: number | null;
  source: "soundcloud";
  sourceUrl: string;
};

export type PlaylistTracks = {
  playlist: Playlist;
  tracks: Track[];
  skippedTrackCount: number;
};

export const api = {
  search: (generation: number, query: string) =>
    invoke<{ tracks: Track[]; nextPageToken: string | null }>("search_tracks", {
      generation, request: { query, limit: 30 },
    }),
  cancelSearch: (generation: number) => invoke<void>("cancel_search", { generation }),
  play: (generation: number, track: Track) =>
    invoke<void>("play_track", {
      generation, request: { source: track.source, trackUrl: track.sourceUrl },
    }),
  stop: (generation: number) => invoke<void>("stop_playback", { generation }),
  status: () => invoke<PlaybackStatus>("playback_status"),
  searchPlaylists: (generation: number, query: string) =>
    invoke<{ playlists: Playlist[]; nextPageToken: string | null }>("search_playlists", {
      generation, request: { query, limit: 30 },
    }),
  playlistTracks: (generation: number, playlist: Playlist) =>
    invoke<PlaylistTracks>("playlist_tracks", {
      generation, request: { source: playlist.source, playlistUrl: playlist.sourceUrl },
    }),
  cancelPlaylists: (generation: number) => invoke<void>("cancel_playlists", { generation }),
  pause: (generation: number, paused: boolean) => invoke<void>("update_playback", { generation, action: { type: "pause", paused } }),
  seek: (generation: number, positionMs: number) => invoke<void>("update_playback", { generation, action: { type: "seek", position_ms: positionMs } }),
  volume: (generation: number, volume: number) => invoke<void>("update_playback", { generation, action: { type: "volume", volume } }),
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
      case "storage_failed": return "Could not save local data. Check free space and permissions.";
    }
  }
  return "Operation failed. Please retry.";
}
