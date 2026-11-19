import { useEffect, useRef, useState, type FormEvent } from "react";
import { api, safeMessage, type LibraryEntry, type Playlist, type PlaylistEntry, type Track } from "./api";
import { Download, FolderOpen, Heart, ListMusic, ListPlus, RefreshCw, RotateCcw, SkipBack, SkipForward, Trash2, Volume2, X } from "lucide-react";
import { settingsStore, validVolume, type AppSettings, type SettingsStore } from "./settings";
import blackMascot from "./assets/black.png";
import soundcloudIcon from "./assets/soundcloud.png";
import youtubeIcon from "./assets/youtube.png";
import vkIcon from "./assets/vk.png";

// Keep new webview sessions newer than requests retained by the Rust process.
const generationBase = Date.now() * 1000;

function time(milliseconds: number) {
  const seconds = Math.max(0, Math.floor(milliseconds / 1000));
  const minutes = Math.floor(seconds / 60);
  return `${minutes}:${String(seconds % 60).padStart(2, "0")}`;
}

function Artwork({ track, size }: { track: Track | null; size: "small" | "large" }) {
  if (track?.artworkUrl) {
    return <img className={`artwork artwork-${size}`} src={track.artworkUrl} alt="" loading="lazy" />;
  }
  return <div className={`artwork artwork-${size} artwork-fallback`} aria-hidden="true">♪</div>;
}

function SourceIcon({ track }: { track: Track }) {
  const source = track.source === "soundcloud" ? "SoundCloud" :
    track.sourceUrl.startsWith("https://youtube.com/") || track.sourceUrl.startsWith("https://www.youtube.com/") ? "YouTube" :
    track.sourceUrl.startsWith("https://vk.com/") || track.sourceUrl.startsWith("https://vkvideo.ru/") ? "VK" : null;
  const icon = source === "SoundCloud" ? soundcloudIcon : source === "YouTube" ? youtubeIcon : source === "VK" ? vkIcon : null;
  return icon ? <img className="source-icon" src={icon} alt="" title={`Source: ${source}`} aria-label={`Source: ${source}`} /> : null;
}

function PlayIcon() {
  return <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M8 5.5v13l11-6.5z" /></svg>;
}

function PauseIcon() {
  return <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M7 5h3.5v14H7zM13.5 5H17v14h-3.5z" /></svg>;
}

function StopIcon() {
  return <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M6.5 6.5h11v11h-11z" /></svg>;
}

type QueueEntry = { key: number; track: Track };

export function App({ settings = settingsStore }: { settings?: SettingsStore }) {
  const [view, setView] = useState<"tracks" | "playlists" | "favourites" | "library" | "settings">("tracks");
  const [queueOpen, setQueueOpen] = useState(false);
  const queuePopupRef = useRef<HTMLDivElement>(null);
  const [query, setQuery] = useState("");
  const [tracks, setTracks] = useState<Track[]>([]);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [openPlaylist, setOpenPlaylist] = useState<{ playlist: Playlist; skipped: number } | null>(null);
  const [playlistLoading, setPlaylistLoading] = useState(false);
  const [searchState, setSearchState] = useState("idle");
  const [searchError, setSearchError] = useState("");
  const [playback, setPlayback] = useState("idle");
  const [playError, setPlayError] = useState("");
  const [selected, setSelected] = useState<Track | null>(null);
  const searchGeneration = useRef(generationBase);
  const playlistGeneration = useRef(generationBase);
  const playbackGeneration = useRef(generationBase);
  const resolving = useRef(false);
  const [position, setPosition] = useState(0);
  const [duration, setDuration] = useState<number | null>(null);
  const [volume, setVolume] = useState(100);
  const [seekable, setSeekable] = useState(false);
  const [seekDraft, setSeekDraft] = useState<number | null>(null);
  const [volumeDraft, setVolumeDraft] = useState<number | null>(null);
  const [controlling, setControlling] = useState(false);
  const active = playback === "playing" || playback === "paused";
  const [queue, setQueue] = useState<Track[]>([]);
  const [queueIndex, setQueueIndex] = useState(-1);
  const queueRef = useRef<Track[]>([]);
  const indexRef = useRef(-1);
  const autoAdvance = useRef(false);
  const prepareGeneration = useRef(generationBase);
  const preparedKey = useRef("");
  const observedTransitions = useRef(0);
  const lookaheadGeneration = useRef(generationBase);
  const lookaheadKey = useRef("");
  const lookaheadTime = useRef(0);
  const lookaheadPending = useRef(false);
  const [sessionQueue, setSessionQueue] = useState<QueueEntry[]>([]);
  const sessionQueueRef = useRef<QueueEntry[]>([]);
  const entryKey = useRef(0);
  const playbackOrigin = useRef<"session" | "playlist">("session");
  const [settingsReady, setSettingsReady] = useState(false);
  const settingsLoaded = useRef(false);
  const [settingsError, setSettingsError] = useState("");
  const preferredVolume = useRef(100);
  const settingsWrites = useRef(Promise.resolve());
  const pendingVolume = useRef<{ generation: number; value: number } | null>(null);
  const volumeSending = useRef(false);
  const [library, setLibrary] = useState<LibraryEntry[]>([]);
  const libraryRef = useRef<LibraryEntry[]>([]);
  const [savedPlaylists, setSavedPlaylists] = useState<PlaylistEntry[]>([]);
  const favouritePlaylists = savedPlaylists.filter(entry => entry.favourite);
  const savedPlaylist = (playlist: Playlist) =>
    savedPlaylists.find(entry => entry.playlist.sourceUrl === playlist.sourceUrl);
  const [libraryError, setLibraryError] = useState("");
  const [scanning, setScanning] = useState(false);
  const [preferences, setPreferences] = useState<AppSettings>({ volume: 100 });

  useEffect(() => {
    document.documentElement.dataset.theme = preferences.theme ?? "dark";
  }, [preferences.theme]);

  useEffect(() => {
    if (!queueOpen) return;
    const closeOnEscape = (event: KeyboardEvent) => { if (event.key === "Escape") setQueueOpen(false); };
    const closeOutside = (event: PointerEvent) => {
      if (!queuePopupRef.current?.contains(event.target as Node)) setQueueOpen(false);
    };
    document.addEventListener("keydown", closeOnEscape);
    document.addEventListener("pointerdown", closeOutside);
    return () => {
      document.removeEventListener("keydown", closeOnEscape);
      document.removeEventListener("pointerdown", closeOutside);
    };
  }, [queueOpen]);

  async function refreshLibrary() {
    try {
      const [entries, playlists] = await Promise.all([api.library(), api.playlistEntries()]);
      libraryRef.current = entries;
      setLibrary(entries);
      setSavedPlaylists(playlists);
      setLibraryError("");
    } catch { setLibraryError("Could not load the music library."); }
  }

  useEffect(() => {
    void refreshLibrary();
    const timer = window.setInterval(() => { void refreshLibrary(); }, 3000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    let disposed = false;
    void settings.getSettings().then(value => {
      if (!validVolume(value.volume)) throw { code: "storage_failed" };
      if (disposed) return;
      preferredVolume.current = value.volume;
      setVolume(value.volume);
      setPreferences(value);
    }).catch(() => {
      if (!disposed) setSettingsError("Could not load volume settings.");
    }).finally(() => {
      if (!disposed) {
        settingsLoaded.current = true;
        setSettingsReady(true);
      }
    });
    return () => { disposed = true; };
  }, [settings]);

  useEffect(() => {
    let disposed = false;
    let polling = false;
    const timer = window.setInterval(async () => {
      if (polling) return;
      polling = true;
      const generation = playbackGeneration.current;
      try {
        const status = await api.status();
        if (!disposed && generation === playbackGeneration.current &&
            status.generation === generation && !resolving.current) {
          if ((status.transitions ?? 0) > observedTransitions.current) {
            observedTransitions.current = status.transitions ?? 0;
            const nextIndex = indexRef.current + 1;
            const next = queueRef.current[nextIndex];
            if (autoAdvance.current && next) {
              indexRef.current = nextIndex;
              setQueueIndex(nextIndex);
              setSelected(next);
              preparedKey.current = "";
            }
          }
          setPlayback(status.state === "loading" ? "starting" : status.state);
          setPosition(status.positionMs ?? 0);
          setDuration(status.durationMs ?? null);
          setVolume(status.volume ?? 100);
          setSeekable(status.seekable ?? false);
          if (status.state === "error") setPlayError(status.error ?? "Playback failed. Please retry.");
          if (status.state === "error") autoAdvance.current = false;
          if (playbackOrigin.current === "playlist" && (status.state === "playing" || status.state === "paused")) {
            const next = queueRef.current[indexRef.current + 1];
            const neighbours: Track[] = [];
            for (let distance = 1; distance <= 40 && neighbours.length < 40; distance++) {
              const nextTrack = queueRef.current[indexRef.current + distance];
              const previousTrack = queueRef.current[indexRef.current - distance];
              if (nextTrack) neighbours.push(nextTrack);
              if (previousTrack && neighbours.length < 40) neighbours.push(previousTrack);
            }
            const key = JSON.stringify(neighbours.map(track => track.sourceUrl));
            if (status.state === "playing" && autoAdvance.current && neighbours.length && !lookaheadPending.current &&
                (key !== lookaheadKey.current || Date.now() - lookaheadTime.current >= 45000)) {
              lookaheadKey.current = key;
              lookaheadTime.current = Date.now();
              lookaheadPending.current = true;
              const current = ++lookaheadGeneration.current;
              void api.prefetch(current, neighbours, generation).catch(() => {}).finally(() => {
                if (current === lookaheadGeneration.current) lookaheadPending.current = false;
              });
            }
            const remaining = status.durationMs === null ? Infinity : status.durationMs - status.positionMs;
            // Resolve close to the transition so signed URLs do not sit unused
            // for an entire long track. Pause suspends speculative work.
            if (autoAdvance.current && status.state === "playing" && next && remaining <= 30000 && preparedKey.current !== next.sourceUrl) {
              preparedKey.current = next.sourceUrl;
              const preparation = ++prepareGeneration.current;
              void api.prepare(preparation, next, generation).catch(() => {
                if (preparation === prepareGeneration.current && generation === playbackGeneration.current) {
                  setPlayError(safeMessage({ code: "playback_failed" }));
                  autoAdvance.current = false;
                }
              });
            }
          }
          if (status.state === "stopped" && autoAdvance.current) {
            const nextIndex = indexRef.current + 1;
            const next = queueRef.current[nextIndex];
            if (next) {
              indexRef.current = nextIndex;
              setQueueIndex(nextIndex);
              void play(next, true);
            } else { autoAdvance.current = false; }
          }
        }
      } catch {
        if (!disposed && generation === playbackGeneration.current && generation > generationBase) {
          setPlayback("error");
          setPlayError("Could not read playback status. Try Stop or Play again.");
        }
      } finally {
        polling = false;
      }
    }, 250);
    return () => {
      disposed = true;
      window.clearInterval(timer);
      void api.cancelSearch(++searchGeneration.current).catch(() => {});
      void api.cancelPlaylists(++playlistGeneration.current).catch(() => {});
      void api.stop(++playbackGeneration.current).catch(() => {});
      void api.cancelPreparation(++prepareGeneration.current).catch(() => {});
    };
  }, []);

  function changeQuery(value: string) {
    setQuery(value);
    setSearchState("idle");
    setSearchError("");
    setTracks([]);
    setPlaylists([]);
    setOpenPlaylist(null);
    void api.cancelSearch(++searchGeneration.current).catch(() => {});
    void api.cancelPlaylists(++playlistGeneration.current).catch(() => {});
  }

  function switchView(next: typeof view) {
    setView(next);
    setOpenPlaylist(null);
    setSearchState("idle");
    setSearchError("");
    setTracks([]);
    setPlaylists([]);
    void api.cancelSearch(++searchGeneration.current).catch(() => {});
    void api.cancelPlaylists(++playlistGeneration.current).catch(() => {});
  }

  async function search(event: FormEvent) {
    event.preventDefault();
    const generation = view === "playlists"
      ? ++playlistGeneration.current
      : ++searchGeneration.current;
    setSearchError("");
    setTracks([]);
    setPlaylists([]);
    setOpenPlaylist(null);
    if (!query.trim()) {
      setSearchState("idle");
      if (view === "playlists") {
        void api.cancelPlaylists(generation).catch(() => {});
      } else {
        void api.cancelSearch(generation).catch(() => {});
      }
      return;
    }
    setSearchState("searching");
    if (view === "playlists") {
      try {
        const response = await api.searchPlaylists(generation, query);
        if (generation !== playlistGeneration.current) return;
        setPlaylists(response.playlists);
        setSearchState(response.playlists.length ? "ready" : "empty");
      } catch (error) {
        if (generation !== playlistGeneration.current) return;
        setSearchState("error");
        setSearchError(safeMessage(error));
      }
      return;
    }
    try {
      const response = await api.search(generation, query);
      if (generation !== searchGeneration.current) return;
      setTracks(response.tracks);
      setSearchState(response.tracks.length ? "ready" : "empty");
    } catch (error) {
      if (generation !== searchGeneration.current) return;
      setSearchState("error");
      setSearchError(safeMessage(error));
    }
  }

  async function openPlaylistTracks(playlist: Playlist) {
    const generation = ++playlistGeneration.current;
    setPlaylistLoading(true);
    setSearchError("");
    setOpenPlaylist(null);
    try {
      const response = await api.playlistTracks(generation, playlist);
      if (generation !== playlistGeneration.current) return;
      setOpenPlaylist({ playlist: response.playlist, skipped: response.skippedTrackCount });
      setTracks(response.tracks);
      setSearchState("ready");
    } catch (error) {
      if (generation !== playlistGeneration.current) return;
      setSearchState("error");
      setSearchError(safeMessage(error));
    } finally {
      if (generation === playlistGeneration.current) setPlaylistLoading(false);
    }
  }

  async function play(track: Track, fromQueue = false) {
    if (!settingsLoaded.current) return;
    if (!fromQueue) {
      const entry = addToQueue(track);
      selectQueue(entry.key);
      return;
    }
    preparedKey.current = "";
    if (playbackOrigin.current === "session") {
      ++lookaheadGeneration.current;
      lookaheadPending.current = false;
      lookaheadKey.current = "";
    }
    observedTransitions.current = 0;
    const generation = ++playbackGeneration.current;
    resolving.current = true;
    setSelected(track);
    setPlayback("resolving");
    setPlayError("");
    setPosition(0);
    setDuration(null);
    setSeekDraft(null);
    setVolumeDraft(null);
    setControlling(false);
    try {
      const local = libraryRef.current.find(entry => (entry.availableOffline || track.source === "local") &&
        entry.track.source === track.source && entry.track.id === track.id);
      if (local) await api.playLibrary(generation, local.id, preferredVolume.current);
      else if (track.source === "local") throw { code: "storage_failed" };
      else await api.play(generation, track, {
        fresh: playbackOrigin.current === "session", volume: preferredVolume.current,
      });
      if (generation === playbackGeneration.current) setPlayback("starting");
    } catch (error) {
      if (generation !== playbackGeneration.current) return;
      autoAdvance.current = false;
      setPlayback("error");
      setPlayError(safeMessage(error));
    } finally {
      if (generation === playbackGeneration.current) resolving.current = false;
    }
  }

  async function stop() {
    autoAdvance.current = false;
    preparedKey.current = "";
    ++lookaheadGeneration.current;
    lookaheadPending.current = false;
    lookaheadKey.current = "";
    void api.cancelPreparation(++prepareGeneration.current).catch(() => {});
    const generation = ++playbackGeneration.current;
    resolving.current = false;
    setPlayback("stopping");
    setControlling(false);
    setPlayError("");
    setSeekDraft(null);
    setVolumeDraft(null);
    try {
      await api.stop(generation);
      if (generation === playbackGeneration.current) {
        setPlayback("stopped");
        setPosition(0);
        setDuration(null);
        setSeekable(false);
        setVolume(preferredVolume.current);
      }
    } catch (error) {
      if (generation !== playbackGeneration.current) return;
      setPlayback("error");
      setPlayError(safeMessage(error));
    }
  }

  function playPlaylist(startIndex = 0) {
    if (!tracks.length || !settingsReady) return;
    ++lookaheadGeneration.current;
    lookaheadPending.current = false;
    lookaheadKey.current = "";
    playbackOrigin.current = "playlist";
    queueRef.current = [...tracks];
    indexRef.current = startIndex;
    setQueue([...tracks]);
    setQueueIndex(startIndex);
    autoAdvance.current = true;
    void play(tracks[startIndex], true);
  }

  function addToQueue(track: Track) {
    const entry = { key: ++entryKey.current, track };
    sessionQueueRef.current = [...sessionQueueRef.current, entry];
    setSessionQueue(sessionQueueRef.current);
    if (playbackOrigin.current === "session") {
      queueRef.current = sessionQueueRef.current.map(item => item.track);
      setQueue(queueRef.current);
    }
    return entry;
  }

  function selectQueue(key: number) {
    if (!settingsReady) return;
    const index = sessionQueueRef.current.findIndex(entry => entry.key === key);
    if (index < 0) return;
    playbackOrigin.current = "session";
    queueRef.current = sessionQueueRef.current.map(entry => entry.track);
    indexRef.current = index;
    setQueue(queueRef.current);
    setQueueIndex(index);
    autoAdvance.current = true;
    void play(queueRef.current[index], true);
  }

  function removeFromQueue(key: number) {
    const index = sessionQueueRef.current.findIndex(entry => entry.key === key);
    if (index < 0) return;
    sessionQueueRef.current = sessionQueueRef.current.filter(entry => entry.key !== key);
    setSessionQueue(sessionQueueRef.current);
    if (playbackOrigin.current !== "session") return;
    queueRef.current = sessionQueueRef.current.map(entry => entry.track);
    setQueue(queueRef.current);
    if (index === indexRef.current) {
      indexRef.current = -1;
      setQueueIndex(-1);
      setSelected(null);
      void stop();
    } else if (index < indexRef.current) {
      indexRef.current--;
      setQueueIndex(indexRef.current);
    }
  }

  function skipQueue(direction: number) {
    const index = indexRef.current + direction;
    const track = queueRef.current[index];
    if (!track) return;
    indexRef.current = index;
    setQueueIndex(index);
    autoAdvance.current = true;
    void play(track, true);
  }

  async function control(operation: (generation: number) => Promise<void>) {
    const generation = playbackGeneration.current;
    setControlling(true);
    setPlayError("");
    setSeekDraft(null);
    setVolumeDraft(null);
    try { await operation(generation); }
    catch (error) {
      if (generation === playbackGeneration.current) setPlayError(safeMessage(error));
    } finally { if (generation === playbackGeneration.current) setControlling(false); }
  }

  function changeVolume(value: number) {
    if (!validVolume(value)) return;
    setVolumeDraft(value);
    pendingVolume.current = { generation: playbackGeneration.current, value };
    if (volumeSending.current) return;
    volumeSending.current = true;
    void (async () => {
      while (pendingVolume.current) {
        const next = pendingVolume.current;
        pendingVolume.current = null;
        if (next.generation !== playbackGeneration.current) continue;
        try {
          await api.volume(next.generation, next.value);
          if (next.generation !== playbackGeneration.current) continue;
          preferredVolume.current = next.value;
          settingsWrites.current = settingsWrites.current.then(async () => {
            const updated = await settings.updateSettings({ volume: next.value });
            setPreferences(current => ({ ...current, volume: updated.volume }));
            setSettingsError("");
          }).catch(() => setSettingsError("Could not save volume settings."));
          if (!pendingVolume.current) {
            setVolume(next.value);
            setVolumeDraft(null);
          }
        } catch (error) {
          if (next.generation === playbackGeneration.current && !pendingVolume.current) {
            setVolumeDraft(null);
            setPlayError(safeMessage(error));
          }
        }
      }
      volumeSending.current = false;
    })();
  }

  async function updatePreferences(patch: Partial<AppSettings>) {
    try {
      const updated = await settings.updateSettings(patch);
      setPreferences(current => ({ ...current, ...patch, ...(patch.musicFolder ? { musicFolder: updated.musicFolder } : {}) }));
      setSettingsError("");
      if (patch.musicFolder) await scanMusic();
    } catch { setSettingsError("Could not save settings."); }
  }

  async function scanMusic() {
    setScanning(true);
    try {
      const result = await api.scanLibrary();
      await refreshLibrary();
      if (result.truncated) setLibraryError("Library scan reached the file limit.");
      else if (result.skipped) setLibraryError(`Could not read ${result.skipped} item(s).`);
    } catch { setLibraryError("Could not scan the music folder."); }
    finally { setScanning(false); }
  }

  async function toggleFavourite(track: Track) {
    const existing = libraryRef.current.find(entry => entry.track.source === track.source && entry.track.id === track.id);
    try {
      await api.favourite(track, !existing?.favourite);
      await refreshLibrary();
    } catch { setLibraryError("Could not update favourites."); }
  }

  async function togglePlaylistFavourite(playlist: Playlist) {
    try {
      await api.favouritePlaylist(playlist, !savedPlaylist(playlist)?.favourite);
      await refreshLibrary();
    } catch { setLibraryError("Could not update favourite playlists."); }
  }

  async function downloadPlaylist(playlist: Playlist) {
    try {
      await api.downloadPlaylist(playlist);
      await refreshLibrary();
    } catch (error) { setLibraryError(safeMessage(error)); }
  }

  function openSavedPlaylist(playlist: Playlist) {
    setView("playlists");
    setOpenPlaylist(null);
    setSearchState("idle");
    setSearchError("");
    setTracks([]);
    setPlaylists([]);
    void openPlaylistTracks(playlist);
  }

  const statusText =
    playback === "resolving" ? "Resolving..." :
    playback === "starting" ? "Starting audio..." :
    playback === "playing" ? "Playing" :
    playback === "paused" ? "Paused" :
    playback === "stopping" ? "Stopping..." :
    playback === "stopped" ? "Stopped" :
    playback === "error" ? playError : "Idle";

  const currentSavedPlaylist = openPlaylist ? savedPlaylist(openPlaylist.playlist) : undefined;

  return <div className="app">
    <aside className="sidebar" aria-label="Navigation">
      <div className="brand"><img className="brand-mark" src={blackMascot} alt="" /><span>Fuzzy Music</span></div>
      <nav className="nav">
        <button aria-label="Search navigation" className={view === "tracks" || view === "playlists" ? "nav-item active" : "nav-item"} onClick={() => switchView("tracks")}>Search</button>
        <button className={view === "favourites" ? "nav-item active" : "nav-item"} onClick={() => switchView("favourites")}>Favourites</button>
        <button className={view === "library" ? "nav-item active" : "nav-item"} onClick={() => switchView("library")}>Music library</button>
        <button className={view === "settings" ? "nav-item active" : "nav-item"} onClick={() => switchView("settings")}>Settings</button>
      </nav>
      <p className="sidebar-note">Streams public SoundCloud tracks. No account or API key required.</p>
    </aside>

    <div className="content">
      {(view === "tracks" || view === "playlists") && <div className="tabs" role="tablist" aria-label="Search type">
        <button role="tab" aria-selected={view === "tracks"} className={view === "tracks" ? "tab active" : "tab"}
          onClick={() => switchView("tracks")}>Tracks</button>
        <button role="tab" aria-selected={view === "playlists"} className={view === "playlists" ? "tab active" : "tab"}
          onClick={() => switchView("playlists")}>Playlists</button>
      </div>}

      {(view === "tracks" || view === "playlists") && <form className="search" onSubmit={search}>
        <label className="sr-only" htmlFor="query">{view === "tracks" ? "Track or artist" : "Playlist name"}</label>
        <span className="search-icon" aria-hidden="true">⌕</span>
        <input id="query" value={query}
          placeholder={view === "tracks" ? "What do you want to listen to?" : "Search public playlists"}
          onChange={event => changeQuery(event.target.value)} />
        <button type="submit" className="primary" disabled={!query.trim()}>Search</button>
        {searchState === "searching" && <button type="button" className="ghost" onClick={() => {
          void api.cancelSearch(++searchGeneration.current).catch(() => {});
          void api.cancelPlaylists(++playlistGeneration.current).catch(() => {});
          setSearchState("cancelled");
        }}>Cancel</button>}
      </form>}

      {(view === "tracks" || view === "playlists") && <p className="results-status" role="status" aria-live="polite">
        {searchState === "idle" ? "Ready" :
         searchState === "searching" ? "Searching..." :
         searchState === "empty" ? (view === "tracks" ? "No tracks found." : "No playlists found.") :
         searchState === "cancelled" ? "Search cancelled." :
         searchState === "error" ? searchError :
         view === "playlists" ? `${playlists.length} ${playlists.length === 1 ? "playlist" : "playlists"}` :
         `${tracks.length} ${tracks.length === 1 ? "track" : "tracks"}`}
      </p>}

      {libraryError && <p className="library-error" role="alert">{libraryError}</p>}
      {view === "settings" && <section className="settings-view" aria-label="Settings">
        <h2>Settings</h2>
        <label htmlFor="download-quality">Download quality</label>
        <select id="download-quality" value={preferences.downloadQuality ?? "best"}
          onChange={event => void updatePreferences({ downloadQuality: event.target.value as AppSettings["downloadQuality"] })}>
          <option value="best">Best available</option>
          <option value="high">High (prefer up to 320 kbps)</option>
          <option value="medium">Medium (prefer up to 192 kbps)</option>
          <option value="standard">Standard (prefer up to 128 kbps)</option>
          <option value="small">Data saver (prefer up to 64 kbps)</option>
          <option value="flac">FLAC (converted)</option>
        </select>
        {preferences.downloadQuality === "flac" && <p className="results-status">Requires ffmpeg. Conversion cannot restore quality lost by the source.</p>}
        <label htmlFor="theme">Theme</label>
        <select id="theme" value={preferences.theme ?? "dark"}
          onChange={event => void updatePreferences({ theme: event.target.value as AppSettings["theme"] })}>
          <option value="dark">Dark</option>
          <option value="light">Light</option>
        </select>
        <label>Music folder</label>
        <div className="folder-setting"><span>{preferences.musicFolder ?? "Unavailable"}</span>
          <button className="row-play" title="Choose music folder" aria-label="Choose music folder" onClick={() => void (async () => {
            try { const folder = await api.chooseFolder(); if (folder) await updatePreferences({ musicFolder: folder }); }
            catch { setSettingsError("Could not choose a music folder."); }
          })()}><FolderOpen /></button>
        </div>
      </section>}

      {(view === "favourites" || view === "library") && <section className="session-queue" aria-label={view === "favourites" ? "Favourite tracks" : "Music library"}>
        <div className="library-heading"><h2>{view === "favourites" ? "Favourites" : "Music library"}</h2>
          <button className="row-play" title="Refresh music folder" aria-label="Refresh music folder" disabled={scanning} onClick={() => void scanMusic()}><RefreshCw /></button>
        </div>
        {library.filter(entry => view === "favourites" ? entry.favourite : entry.availableOffline || entry.track.source === "local").length === 0 &&
          <p className="results-status">{view === "favourites" ? "No favourites yet." : "No local tracks found."}</p>}
        <ol>{library.filter(entry => view === "favourites" ? entry.favourite : entry.availableOffline || entry.track.source === "local").map(entry =>
          <li key={entry.id}>
            <Artwork track={entry.track} size="small" />
            <div className="row-text"><strong>{entry.track.title}</strong><span>{entry.track.author} · {
              entry.availableOffline ? "Available offline" : entry.downloadState === "downloading" ? "Downloading" :
              entry.downloadState === "pending" ? "Waiting to download" : entry.downloadState === "error" ? "Download failed" :
              entry.downloadState === "missing" ? "File missing" : "Online"
            }</span></div>
            {entry.favourite && !entry.availableOffline && (entry.downloadState === "error" || entry.downloadState === "missing" || entry.downloadState === "none") &&
              <button className="row-play" title={`Retry ${entry.track.title}`} aria-label={`Retry ${entry.track.title}`} onClick={() => void api.retryDownload(entry.id).then(refreshLibrary).catch(() => setLibraryError("Could not retry download."))}><RefreshCw /></button>}
            <button className="row-play" title={`Add ${entry.track.title} to queue`} aria-label={`Add ${entry.track.title} to queue`} onClick={() => addToQueue(entry.track)}><ListPlus /></button>
            <button className="row-play" title={`Play ${entry.track.title}`} aria-label={`Play ${entry.track.title}`} disabled={!settingsReady} onClick={() => void play(entry.track)}><PlayIcon /></button>
            <button className={entry.favourite ? "row-play liked" : "row-play"} title={entry.favourite ? `Remove ${entry.track.title} from favourites` : `Add ${entry.track.title} to favourites`}
              aria-label={entry.favourite ? `Remove ${entry.track.title} from favourites` : `Add ${entry.track.title} to favourites`} onClick={() => void toggleFavourite(entry.track)}><Heart /></button>
          </li>)}</ol>
      </section>}

      {view === "favourites" && favouritePlaylists.length > 0 &&
        <section className="session-queue" aria-label="Favourite playlists">
          <div className="library-heading"><h2>Favourite playlists</h2></div>
          <ol className="playlist-rows">{favouritePlaylists.map(entry =>
            <li key={entry.id}>
              {entry.playlist.artworkUrl
                ? <img className="artwork artwork-small" src={entry.playlist.artworkUrl} alt="" loading="lazy" />
                : <div className="artwork artwork-small artwork-fallback" aria-hidden="true">♪</div>}
              <div className="row-text"><strong>{entry.playlist.title}</strong><span>{entry.playlist.author} · {
                entry.state === "ready" ? `Downloaded · ${entry.total} ${entry.total === 1 ? "track" : "tracks"}` :
                entry.total > 0 ? `${entry.downloaded} of ${entry.total} downloaded` :
                entry.playlist.trackCount === null ? "Playlist" : `${entry.playlist.trackCount} ${entry.playlist.trackCount === 1 ? "track" : "tracks"}`
              }</span></div>
              {entry.state !== "ready" && entry.state !== "downloading" &&
                <button className="row-play" title={`Download playlist ${entry.playlist.title}`} aria-label={`Download playlist ${entry.playlist.title}`}
                  onClick={() => void downloadPlaylist(entry.playlist)}><Download /></button>}
              <button className="row-play" title={`Open playlist ${entry.playlist.title}`} aria-label={`Open playlist ${entry.playlist.title}`}
                onClick={() => openSavedPlaylist(entry.playlist)}><ListMusic /></button>
              <button className="row-play liked" title={`Remove playlist ${entry.playlist.title} from favourites`}
                aria-label={`Remove playlist ${entry.playlist.title} from favourites`}
                onClick={() => void togglePlaylistFavourite(entry.playlist)}><Heart /></button>
            </li>)}</ol>
        </section>}

      {view === "playlists" && !openPlaylist &&
        <section aria-label="Playlist results" className="playlist-grid">
          {playlists.map(playlist => {
            const saved = savedPlaylist(playlist);
            return <div key={playlist.id} className="playlist-card-wrap">
              <button className="playlist-card" onClick={() => void openPlaylistTracks(playlist)} aria-label={`Open playlist ${playlist.title}`}>
                {playlist.artworkUrl
                  ? <img className="playlist-cover" src={playlist.artworkUrl} alt="" loading="lazy" />
                  : <div className="playlist-cover artwork-fallback" aria-hidden="true">♪</div>}
                <strong>{playlist.title}</strong>
                <span>{playlist.author}</span>
                <span className="playlist-meta">
                  {playlist.trackCount === null ? "Playlist" : `${playlist.trackCount} ${playlist.trackCount === 1 ? "track" : "tracks"}`}
                </span>
              </button>
              <button className={saved?.favourite ? "row-play liked playlist-card-like" : "row-play playlist-card-like"}
                title={saved?.favourite ? `Remove playlist ${playlist.title} from favourites` : `Favourite playlist ${playlist.title}`}
                aria-label={saved?.favourite ? `Remove playlist ${playlist.title} from favourites` : `Favourite playlist ${playlist.title}`}
                onClick={() => void togglePlaylistFavourite(playlist)}><Heart /></button>
            </div>;
          })}
        </section>}

      {view === "playlists" && !openPlaylist && playlistLoading && <p className="results-status">Loading playlist...</p>}

      {openPlaylist &&
        <section className="playlist-header">
          <button className="ghost small" onClick={() => {
            setOpenPlaylist(null);
            setTracks([]);
            setSearchState(playlists.length ? "ready" : "idle");
          }}>← Back to playlists</button>
          <div className="playlist-hero">
            {openPlaylist.playlist.artworkUrl
              ? <img className="playlist-hero-cover" src={openPlaylist.playlist.artworkUrl} alt="" />
              : <div className="playlist-hero-cover artwork-fallback" aria-hidden="true">♪</div>}
            <div>
              <p className="eyebrow">Playlist</p>
              <h2>{openPlaylist.playlist.title}</h2>
              <p className="playlist-by">
                {openPlaylist.playlist.author} · {tracks.length} of{" "}
                {openPlaylist.playlist.trackCount ?? tracks.length}{" "}
                {(openPlaylist.playlist.trackCount ?? tracks.length) === 1 ? "track" : "tracks"}
              </p>
              {openPlaylist.skipped > 0 &&
                <p className="playlist-note">
                  {openPlaylist.skipped === 1
                    ? "1 track is unavailable and was skipped."
                    : `${openPlaylist.skipped} tracks are unavailable and were skipped.`}
                </p>}
            </div>
          </div>
          <div className="playlist-actions">
            <button className="primary" onClick={() => playPlaylist() } disabled={!tracks.length || !settingsReady}>Play playlist</button>
            <button className={currentSavedPlaylist?.favourite ? "ghost small liked" : "ghost small"}
              title={currentSavedPlaylist?.favourite ? `Remove playlist ${openPlaylist.playlist.title} from favourites` : `Favourite playlist ${openPlaylist.playlist.title}`}
              aria-label={currentSavedPlaylist?.favourite ? `Remove playlist ${openPlaylist.playlist.title} from favourites` : `Favourite playlist ${openPlaylist.playlist.title}`}
              onClick={() => void togglePlaylistFavourite(openPlaylist.playlist)}><Heart /></button>
            <button className="ghost small" disabled={!tracks.length || currentSavedPlaylist?.state === "downloading"}
              title={`Download playlist ${openPlaylist.playlist.title}`} aria-label={`Download playlist ${openPlaylist.playlist.title}`}
              onClick={() => void downloadPlaylist(openPlaylist.playlist)}><Download /></button>
            {currentSavedPlaylist && currentSavedPlaylist.total > 0 &&
              <span className="playlist-by">{currentSavedPlaylist.state === "ready" ? "Available offline" : `${currentSavedPlaylist.downloaded} of ${currentSavedPlaylist.total} downloaded`}</span>}
          </div>
        </section>}

      {(view === "tracks" || view === "playlists") && tracks.length > 0 &&
        <section aria-label="Search results" className="results">
          <div className="results-head"><span>#</span><span /><span>Title</span><span /><span>Time</span><span /><span /><span /></div>
          <ul>
            {tracks.map((track, index) => {
              const current = selected?.id === track.id;
              return <li key={track.id} className={current ? "current" : undefined}>
                <span className="row-index">{current && active ? "▰" : index + 1}</span>
                <Artwork track={track} size="small" />
                <div className="row-text">
                  <strong>{track.title}</strong>
                  <span>{track.author}</span>
                </div>
                <SourceIcon track={track} />
                <span className="row-duration">{track.durationMs === null ? "--:--" : time(track.durationMs)}</span>
                <button className={library.some(entry => entry.track.source === track.source && entry.track.id === track.id && entry.favourite) ? "row-play liked" : "row-play"}
                  title={`Favourite ${track.title}`} aria-label={`Favourite ${track.title}`} onClick={() => void toggleFavourite(track)}><Heart /></button>
                <button className="row-play" title={`Add ${track.title} to queue`}
                  onClick={() => addToQueue(track)} aria-label={`Add ${track.title} to queue`}><ListPlus /></button>
                <button className="row-play" disabled={!settingsReady} title={`Play ${track.title}`} onClick={() => openPlaylist ? playPlaylist(index) : void play(track)} aria-label={`Play ${track.title}`}>
                  <PlayIcon />
                </button>
              </li>;
            })}
          </ul>
        </section>}

      {(view === "tracks" || view === "playlists") && tracks.length === 0 && searchState === "idle" &&
        <section className="empty-state">
          <h2>{view === "tracks" ? "Find something to play" : "Find a playlist"}</h2>
          <p>{view === "tracks"
            ? "Search SoundCloud by track title or artist. Results stream directly through mpv."
            : "Search public SoundCloud playlists, open one and play any track from it."}</p>
        </section>}
    </div>

    <footer className="player" aria-label="Playback">
      <div className="transport">
        <p className="transport-status" role="status" aria-live="polite">{statusText}</p>
        <div className="transport-buttons">
          <button className="icon-btn" disabled={!settingsReady || queueIndex <= 0} onClick={() => skipQueue(-1)} aria-label="Previous track" title="Previous track"><SkipBack /></button>
          <button className="icon-btn" disabled={!active || controlling}
            aria-label={playback === "paused" ? "Resume" : "Pause"}
            onClick={() => void control(generation => api.pause(generation, playback !== "paused"))}>
            {playback === "paused" ? <PlayIcon /> : <PauseIcon />}
          </button>
          <button className="icon-btn" onClick={() => void stop()} disabled={playback === "stopping"} aria-label="Stop">
            <StopIcon />
          </button>
          {selected && <button className="icon-btn" title="Play again" aria-label="Play again" disabled={!settingsReady} onClick={() => {
            autoAdvance.current = true;
            void play(selected, true);
          }}><RotateCcw /></button>}
          <button className="icon-btn" disabled={!settingsReady || queueIndex < 0 || queueIndex + 1 >= queue.length} onClick={() => skipQueue(1)} aria-label="Next track" title="Next track"><SkipForward /></button>
        </div>
        {queueIndex >= 0 && <span className="results-status">{queueIndex + 1} / {queue.length}</span>}
      </div>

      <div className="levels">
        <div className="timeline">
          <label htmlFor="position">Position</label>
          <input id="position" type="range" min="0" max={duration ?? 1} step="1000"
            value={seekDraft ?? Math.min(position, duration ?? 0)} disabled={!active || !seekable || !duration}
            onChange={event => setSeekDraft(Number(event.target.value))}
            onPointerCancel={() => setSeekDraft(null)}
            onBlur={() => setSeekDraft(null)}
            onPointerUp={event => { if (!active || !seekable || !duration) return; const value = Number(event.currentTarget.value); setSeekDraft(null); void control(generation => api.seek(generation, value)); }}
            onKeyUp={event => { if (["ArrowLeft", "ArrowRight", "Home", "End", "PageUp", "PageDown"].includes(event.key)) { const value = Number(event.currentTarget.value); setSeekDraft(null); void control(generation => api.seek(generation, value)); } }} />
          <output>{time(seekDraft ?? position)} / {duration === null ? "--:--" : time(duration)}</output>
        </div>
        <div className="volume">
          <button className="volume-trigger icon-btn" aria-label="Volume controls" title="Volume"><Volume2 /></button>
          <div className="volume-popup">
          <label htmlFor="volume" className="sr-only">Volume</label>
          <input id="volume" type="range" min="0" max="100" step="1" value={volumeDraft ?? volume} disabled={!active}
            onChange={event => changeVolume(Number(event.target.value))} />
          <output>{Math.round(volumeDraft ?? volume)}%</output>
          </div>
        </div>
        <div className="queue-control" ref={queuePopupRef}>
          <button className="icon-btn" aria-label="Queue" title={`Queue (${sessionQueue.length})`}
            aria-expanded={queueOpen} aria-controls="queue-popup" onClick={() => setQueueOpen(open => !open)}><ListMusic /></button>
          {queueOpen && <section id="queue-popup" className="queue-popup session-queue" aria-label="Session queue">
            <div className="queue-heading"><h2>Queue ({sessionQueue.length})</h2>
              <button className="row-play" aria-label="Close queue" title="Close queue" onClick={() => setQueueOpen(false)}><X /></button>
            </div>
            {!sessionQueue.length && <p className="results-status">Queue is empty.</p>}
            <ol>
              {sessionQueue.map((entry, index) => {
                const current = playbackOrigin.current === "session" && queueIndex === index;
                return <li key={entry.key} aria-current={current ? "true" : undefined}>
                  <Artwork track={entry.track} size="small" />
                  <div className="row-text"><strong>{entry.track.title}</strong><span>{entry.track.author}</span></div>
                  <button className="row-play" disabled={!settingsReady} title={`Play queued ${entry.track.title}`}
                    aria-label={`Play queued ${entry.track.title}`} onClick={() => selectQueue(entry.key)}><PlayIcon /></button>
                  <button className="row-play" title={`Remove ${entry.track.title} from queue`}
                    aria-label={`Remove ${entry.track.title} from queue`} onClick={() => removeFromQueue(entry.key)}><Trash2 /></button>
                </li>;
              })}
            </ol>
          </section>}
        </div>
      </div>

      <div className="now-playing">
        <Artwork track={selected} size="small" />
        <div className="now-text"><strong>{selected ? selected.title : "No track selected"}</strong>
          <span>{selected ? selected.author : "Search to begin"}</span></div>
      </div>
      {playError && playback !== "error" && <p className="player-error" role="alert">{playError}</p>}
      {settingsError && <p className="player-error" role="alert">{settingsError}</p>}
    </footer>
  </div>;
}
