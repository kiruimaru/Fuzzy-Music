import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export type Track = {
  id: string;
  title: string;
  author: string;
  artworkUrl: string | null;
  durationMs: number | null;
  streamUrl: string | null;
  source: "soundcloud" | "local";
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
  transitions?: number;
};

export type Playlist = {
  id: string;
  title: string;
  author: string;
  artworkUrl: string | null;
  trackCount: number | null;
  durationMs: number | null;
  isAlbum: boolean;
  source: "soundcloud";
  sourceUrl: string;
};

export type PlaylistTracks = {
  playlist: Playlist;
  tracks: Track[];
  skippedTrackCount: number;
};

export type Artist = {
  id: string;
  name: string;
  avatarUrl: string | null;
  trackCount: number | null;
  sourceUrl: string;
};

export type ArtistTracks = {
  artist: Artist;
  tracks: Track[];
  albums: Playlist[];
  playlists: Playlist[];
};

export type LibraryEntry = {
  id: number;
  track: Track;
  favourite: boolean;
  downloadState: "none" | "pending" | "downloading" | "ready" | "error" | "missing";
  error: string | null;
  availableOffline: boolean;
};

export type PlaylistEntry = {
  id: number;
  playlist: Playlist;
  favourite: boolean;
  downloaded: number;
  total: number;
  state: "idle" | "downloading" | "ready" | "error";
};

export const api = {
  library: () => invoke<LibraryEntry[]>("library_entries"),
  scanLibrary: () => invoke<{ imported: number; skipped: number; truncated: boolean }>("scan_library"),
  favourite: (track: Track, liked: boolean) => invoke<void>("set_favourite", { track, liked }),
  playlistEntries: () => invoke<PlaylistEntry[]>("playlist_entries"),
  favouritePlaylist: (playlist: Playlist, liked: boolean) =>
    invoke<void>("set_playlist_favourite", { playlist, liked }),
  downloadPlaylist: (playlist: Playlist) => invoke<void>("download_playlist", { playlist }),
  cancelPlaylistDownload: (playlist: Playlist) => invoke<void>("cancel_playlist_download", { playlist }),
  retryDownload: (id: number) => invoke<void>("retry_download", { id }),
  removeTrack: (id: number) => invoke<void>("remove_library_track", { id }),
  chooseFolder: () => invoke<string | null>("choose_music_folder"),
  chooseBackground: () => invoke<string | null>("choose_background_image"),
  clearBackground: () => invoke<void>("clear_background_image"),
  playLibrary: (generation: number, id: number, volume: number) =>
    invoke<void>("play_library_track", { generation, id, volume }),
  search: (generation: number, query: string) =>
    invoke<{ tracks: Track[]; nextPageToken: string | null }>("search_tracks", {
      generation, request: { query, limit: 30 },
    }),
  cancelSearch: (generation: number) => invoke<void>("cancel_search", { generation }),
  play: (generation: number, track: Track, options?: { fresh?: boolean; volume?: number }) =>
    invoke<void>("play_track", {
      generation, request: { source: track.source, trackUrl: track.sourceUrl },
      ...options,
    }),
  stop: (generation: number) => invoke<void>("stop_playback", { generation }),
  prepare: (generation: number, track: Track, playbackGeneration: number) => invoke<void>("prepare_track", {
    generation, playbackGeneration, request: { source: track.source, trackUrl: track.sourceUrl },
  }),
  prefetch: (generation: number, tracks: Track[], playbackGeneration: number) => invoke<void>("prefetch_tracks", {
    generation, playbackGeneration, requests: tracks.map(track => ({ source: track.source, trackUrl: track.sourceUrl })),
  }),
  cancelPreparation: (generation: number) => invoke<void>("cancel_preparation", { generation }),
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
  artistTracks: (generation: number, artistUrl: string) =>
    invoke<ArtistTracks>("artist_tracks", {
      generation, request: { source: "soundcloud", artistUrl },
    }),
  cancelArtists: (generation: number) => invoke<void>("cancel_artists", { generation }),
  pause: (generation: number, paused: boolean) => invoke<void>("update_playback", { generation, action: { type: "pause", paused } }),
  seek: (generation: number, positionMs: number) => invoke<void>("update_playback", { generation, action: { type: "seek", position_ms: positionMs } }),
  volume: (generation: number, volume: number) => invoke<void>("update_playback", { generation, action: { type: "volume", volume } }),
};

export function backgroundUrl(path: string) {
  return convertFileSrc(path);
}

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
