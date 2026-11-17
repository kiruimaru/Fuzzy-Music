import { useEffect, useRef, useState, type FormEvent } from "react";
import { api, safeMessage, type Track } from "./api";

// Keep new webview sessions newer than requests retained by the Rust process.
const generationBase = Date.now() * 1000;

export function App() {
  const [query, setQuery] = useState("");
  const [tracks, setTracks] = useState<Track[]>([]);
  const [searchState, setSearchState] = useState("idle");
  const [searchError, setSearchError] = useState("");
  const [playback, setPlayback] = useState("idle");
  const [playError, setPlayError] = useState("");
  const [selected, setSelected] = useState<Track | null>(null);
  const searchGeneration = useRef(generationBase);
  const playbackGeneration = useRef(generationBase);
  const resolving = useRef(false);

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
          setPlayback(status.state === "loading" ? "starting" : status.state);
          setPlayError(status.state === "error" ? status.error ?? "Playback failed. Please retry." : "");
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
      void api.stop(++playbackGeneration.current).catch(() => {});
    };
  }, []);

  function changeQuery(value: string) {
    setQuery(value);
    setSearchState("idle");
    setSearchError("");
    setTracks([]);
    void api.cancelSearch(++searchGeneration.current).catch(() => {});
  }

  async function search(event: FormEvent) {
    event.preventDefault();
    const generation = ++searchGeneration.current;
    setSearchError("");
    setTracks([]);
    if (!query.trim()) {
      setSearchState("idle");
      void api.cancelSearch(generation).catch(() => {});
      return;
    }
    setSearchState("searching");
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

  async function play(track: Track) {
    const generation = ++playbackGeneration.current;
    resolving.current = true;
    setSelected(track);
    setPlayback("resolving");
    setPlayError("");
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
    const generation = ++playbackGeneration.current;
    resolving.current = false;
    setPlayback("stopping");
    setPlayError("");
    try {
      await api.stop(generation);
      if (generation === playbackGeneration.current) setPlayback("stopped");
    } catch (error) {
      if (generation !== playbackGeneration.current) return;
      setPlayback("error");
      setPlayError(safeMessage(error));
    }
  }

  return <main>
    <h1>Fuzzy Music</h1>
    <form onSubmit={search}>
      <label htmlFor="query">Track or artist</label>
      <div className="search">
        <input id="query" value={query} onChange={event => changeQuery(event.target.value)} />
        <button type="submit" disabled={!query.trim()}>Search</button>
        {searchState === "searching" && <button type="button" onClick={() => {
          void api.cancelSearch(++searchGeneration.current).catch(() => {});
          setSearchState("cancelled");
        }}>Cancel</button>}
      </div>
    </form>
    <p role="status" aria-live="polite">
      {searchState === "idle" ? "Ready" :
       searchState === "searching" ? "Searching..." :
       searchState === "empty" ? "No tracks found." :
       searchState === "cancelled" ? "Search cancelled." :
       searchState === "error" ? searchError : `${tracks.length} ${tracks.length === 1 ? "track" : "tracks"}`}
    </p>
    <section aria-label="Playback" className="playback">
      <p>{selected ? `${selected.title} - ${selected.author}` : "No track selected"}</p>
      <p role="status" aria-live="polite">
        {playback === "resolving" ? "Resolving..." :
         playback === "starting" ? "Starting audio..." :
         playback === "playing" ? "Playing" :
         playback === "stopping" ? "Stopping..." :
         playback === "stopped" ? "Stopped" :
         playback === "error" ? playError : "Idle"}
      </p>
      <button onClick={() => void stop()} disabled={playback === "stopping"}>Stop</button>
      {selected && <button onClick={() => void play(selected)}>Play again</button>}
    </section>
    <ul aria-label="Search results">
      {tracks.map(track => <li key={track.id}>
        <div><strong>{track.title}</strong><span>{track.author}</span></div>
        <button onClick={() => void play(track)} aria-label={`Play ${track.title}`}>Play</button>
      </li>)}
    </ul>
  </main>;
}
