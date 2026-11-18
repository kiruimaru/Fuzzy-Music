import { useEffect, useRef, useState, type FormEvent } from "react";
import { api, safeMessage, type Playlist, type Track } from "./api";

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

function PlayIcon() {
  return <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M8 5.5v13l11-6.5z" /></svg>;
}

function PauseIcon() {
  return <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M7 5h3.5v14H7zM13.5 5H17v14h-3.5z" /></svg>;
}

function StopIcon() {
  return <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M6.5 6.5h11v11h-11z" /></svg>;
}

export function App() {
  const [view, setView] = useState<"tracks" | "playlists">("tracks");
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
  const retryPreparationAfter = useRef(0);
  const observedTransitions = useRef(0);
  const lookaheadGeneration = useRef(generationBase);
  const lookaheadKey = useRef("");
  const lookaheadTime = useRef(0);
  const lookaheadPending = useRef(false);

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
              retryPreparationAfter.current = 0;
            }
          }
          setPlayback(status.state === "loading" ? "starting" : status.state);
          setPosition(status.positionMs ?? 0);
          setDuration(status.durationMs ?? null);
          setVolume(status.volume ?? 100);
          setSeekable(status.seekable ?? false);
          setPlayError(status.state === "error" ? status.error ?? "Playback failed. Please retry." : "");
          if (status.state === "playing" || status.state === "paused") {
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
            if (status.state === "playing" && next && remaining <= 30000 && preparedKey.current !== next.sourceUrl && Date.now() >= retryPreparationAfter.current) {
              preparedKey.current = next.sourceUrl;
              const preparation = ++prepareGeneration.current;
              void api.prepare(preparation, next, generation).catch(() => {
                if (preparation === prepareGeneration.current && generation === playbackGeneration.current) {
                  preparedKey.current = "";
                  retryPreparationAfter.current = Date.now() + 2000;
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

  function switchView(next: "tracks" | "playlists") {
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
    if (!fromQueue) {
      autoAdvance.current = false;
      queueRef.current = [];
      indexRef.current = -1;
      setQueue([]);
      setQueueIndex(-1);
    }
    preparedKey.current = "";
    if (!fromQueue) {
      ++lookaheadGeneration.current;
      lookaheadPending.current = false;
      lookaheadKey.current = "";
    }
    retryPreparationAfter.current = 0;
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
    try {
      await api.play(generation, track);
      if (generation === playbackGeneration.current) setPlayback("starting");
    } catch (error) {
      if (generation !== playbackGeneration.current) return;
      setPlayback("error");
      setPlayError(safeMessage(error));
    } finally {
      if (generation === playbackGeneration.current) resolving.current = false;
    }
  }

  async function stop() {
    autoAdvance.current = false;
    preparedKey.current = "";
    void api.cancelPreparation(++prepareGeneration.current).catch(() => {});
    const generation = ++playbackGeneration.current;
    resolving.current = false;
    setPlayback("stopping");
    setPlayError("");
    setSeekDraft(null);
    setVolumeDraft(null);
    try {
      await api.stop(generation);
      if (generation === playbackGeneration.current) setPlayback("stopped");
    } catch (error) {
      if (generation !== playbackGeneration.current) return;
      setPlayback("error");
      setPlayError(safeMessage(error));
    }
  }

  function playPlaylist(startIndex = 0) {
    if (!tracks.length) return;
    queueRef.current = [...tracks];
    indexRef.current = startIndex;
    setQueue([...tracks]);
    setQueueIndex(startIndex);
    autoAdvance.current = true;
    void play(tracks[startIndex], true);
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
    } finally { setControlling(false); }
  }

  const statusText =
    playback === "resolving" ? "Resolving..." :
    playback === "starting" ? "Starting audio..." :
    playback === "playing" ? "Playing" :
    playback === "paused" ? "Paused" :
    playback === "stopping" ? "Stopping..." :
    playback === "stopped" ? "Stopped" :
    playback === "error" ? playError : "Idle";

  return <div className="app">
    <aside className="sidebar" aria-label="Navigation">
      <div className="brand"><span className="brand-mark">♪</span><span>Fuzzy Music</span></div>
      <nav className="nav">
        <span className="nav-item active" aria-current="page">Search</span>
        <span className="nav-item soon">Favourites <em>soon</em></span>
        <span className="nav-item">Playlists <em>beta</em></span>
      </nav>
      <p className="sidebar-note">Streams public SoundCloud tracks. No account or API key required.</p>
    </aside>

    <div className="content">
      <div className="tabs" role="tablist" aria-label="Search type">
        <button role="tab" aria-selected={view === "tracks"} className={view === "tracks" ? "tab active" : "tab"}
          onClick={() => switchView("tracks")}>Tracks</button>
        <button role="tab" aria-selected={view === "playlists"} className={view === "playlists" ? "tab active" : "tab"}
          onClick={() => switchView("playlists")}>Playlists</button>
      </div>

      <form className="search" onSubmit={search}>
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
      </form>

      <p className="results-status" role="status" aria-live="polite">
        {searchState === "idle" ? "Ready" :
         searchState === "searching" ? "Searching..." :
         searchState === "empty" ? (view === "tracks" ? "No tracks found." : "No playlists found.") :
         searchState === "cancelled" ? "Search cancelled." :
         searchState === "error" ? searchError :
         view === "playlists" ? `${playlists.length} ${playlists.length === 1 ? "playlist" : "playlists"}` :
         `${tracks.length} ${tracks.length === 1 ? "track" : "tracks"}`}
      </p>

      {view === "playlists" && !openPlaylist &&
        <section aria-label="Playlist results" className="playlist-grid">
          {playlists.map(playlist => <button key={playlist.id} className="playlist-card"
            onClick={() => void openPlaylistTracks(playlist)} aria-label={`Open playlist ${playlist.title}`}>
            {playlist.artworkUrl
              ? <img className="playlist-cover" src={playlist.artworkUrl} alt="" loading="lazy" />
              : <div className="playlist-cover artwork-fallback" aria-hidden="true">♪</div>}
            <strong>{playlist.title}</strong>
            <span>{playlist.author}</span>
            <span className="playlist-meta">
              {playlist.trackCount === null ? "Playlist" : `${playlist.trackCount} ${playlist.trackCount === 1 ? "track" : "tracks"}`}
            </span>
          </button>)}
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
          <button className="primary" onClick={() => playPlaylist() } disabled={!tracks.length}>Play playlist</button>
        </section>}

      {tracks.length > 0 &&
        <section aria-label="Search results" className="results">
          <div className="results-head"><span>#</span><span>Title</span><span>Time</span><span /></div>
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
                <span className="row-duration">{track.durationMs === null ? "--:--" : time(track.durationMs)}</span>
                <button className="row-play" onClick={() => openPlaylist ? playPlaylist(index) : void play(track)} aria-label={`Play ${track.title}`}>
                  <PlayIcon />
                </button>
              </li>;
            })}
          </ul>
        </section>}

      {tracks.length === 0 && searchState === "idle" &&
        <section className="empty-state">
          <h2>{view === "tracks" ? "Find something to play" : "Find a playlist"}</h2>
          <p>{view === "tracks"
            ? "Search SoundCloud by track title or artist. Results stream directly through mpv."
            : "Search public SoundCloud playlists, open one and play any track from it."}</p>
        </section>}
    </div>

    <footer className="player" aria-label="Playback">
      <div className="now-playing">
        <Artwork track={selected} size="small" />
        <div className="now-text">
          <strong>{selected ? selected.title : "No track selected"}</strong>
          <span>{selected ? selected.author : "Search to begin"}</span>
        </div>
      </div>

      <div className="transport">
        <p className="transport-status" role="status" aria-live="polite">{statusText}</p>
        <div className="transport-buttons">
          <button className="ghost small" disabled={queueIndex <= 0} onClick={() => skipQueue(-1)} aria-label="Previous track">‹</button>
          <button className="icon-btn" disabled={!active || controlling}
            aria-label={playback === "paused" ? "Resume" : "Pause"}
            onClick={() => void control(generation => api.pause(generation, playback !== "paused"))}>
            {playback === "paused" ? <PlayIcon /> : <PauseIcon />}
          </button>
          <button className="icon-btn" onClick={() => void stop()} disabled={playback === "stopping"} aria-label="Stop">
            <StopIcon />
          </button>
          {selected && <button className="ghost small" onClick={() => void play(selected)}>Play again</button>}
          <button className="ghost small" disabled={queueIndex < 0 || queueIndex + 1 >= queue.length} onClick={() => skipQueue(1)} aria-label="Next track">›</button>
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
          <label htmlFor="volume">Volume</label>
          <input id="volume" type="range" min="0" max="100" step="1" value={volumeDraft ?? volume} disabled={!active}
            onChange={event => setVolumeDraft(Number(event.target.value))}
            onPointerCancel={() => setVolumeDraft(null)}
            onBlur={() => setVolumeDraft(null)}
            onPointerUp={event => { if (!active) return; const value = Number(event.currentTarget.value); setVolumeDraft(null); void control(generation => api.volume(generation, value)); }}
            onKeyUp={event => { if (["ArrowLeft", "ArrowRight", "Home", "End", "PageUp", "PageDown"].includes(event.key)) { const value = Number(event.currentTarget.value); setVolumeDraft(null); void control(generation => api.volume(generation, value)); } }} />
          <output>{Math.round(volumeDraft ?? volume)}%</output>
        </div>
      </div>

      {playError && playback !== "error" && <p className="player-error" role="alert">{playError}</p>}
    </footer>
  </div>;
}
