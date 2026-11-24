import { useEffect, useState } from "react";
import { Download, RefreshCw } from "lucide-react";
import { api, safeMessage, type RuntimeStatus } from "./api";

export function RuntimeSetup({ settings = false }: { settings?: boolean }) {
  const [status, setStatus] = useState<RuntimeStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [system, setSystem] = useState(false);
  const [ffmpeg, setFfmpeg] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;
    void api.runtimeStatus().then(value => { if (active) setStatus(value); }).catch(() => {});
    return () => { active = false; };
  }, []);

  if (!status?.supported || (!settings && status.ytDlp && status.mpv)) return null;

  const windows = status.platform === "windows";
  const linux = status.platform === "linux";

  async function install() {
    setBusy(true);
    setError("");
    try { setStatus(await api.installRuntime(system, ffmpeg)); }
    catch (failure) { setError(safeMessage(failure)); }
    finally { setBusy(false); }
  }

  return <section className="runtime-setup" aria-label="Runtime setup">
    <h3>Playback dependencies</h3>
    <dl className="runtime-status">
      <div><dt>yt-dlp</dt><dd>{status.ytDlp ? "Ready" : "Missing"}</dd></div>
      <div><dt>mpv</dt><dd>{status.mpv ? "Ready" : "Missing"}</dd></div>
      <div><dt>ffmpeg (FLAC)</dt><dd>{status.ffmpeg ? "Ready" : "Optional"}</dd></div>
    </dl>
    <p className="runtime-hint">
      {windows
        ? "Downloads verified mpv and yt-dlp into the app's tools folder. Administrator approval is not required."
        : "Downloads verified yt-dlp. mpv and optional ffmpeg can be installed with your distribution's package manager."}
    </p>
    {linux && status.canInstallPackages && (!status.mpv || !status.ffmpeg) && <label className="runtime-option">
      <input type="checkbox" checked={system} disabled={busy}
        onChange={event => setSystem(event.target.checked)} />
      Install system packages (administrator approval)
    </label>}
    {system && !status.ffmpeg && <label className="runtime-option">
      <input type="checkbox" checked={ffmpeg} disabled={busy} onChange={event => setFfmpeg(event.target.checked)} />
      Include ffmpeg
    </label>}
    <div className="runtime-actions">
      {(!status.ytDlp || !status.mpv || (system && ffmpeg && !status.ffmpeg)) &&
        <button className="ghost" disabled={busy || (status.ytDlp && !system)} onClick={() => void install()}>
          <Download size={16} />{busy ? "Installing..." : "Install dependencies"}
        </button>}
      <button className="row-play" title="Check dependencies" aria-label="Check dependencies" disabled={busy}
        onClick={() => void api.runtimeStatus().then(setStatus).catch(failure => setError(safeMessage(failure)))}>
        <RefreshCw size={16} />
      </button>
    </div>
    {error && <p role="alert">{error}</p>}
    {linux && !status.canInstallPackages && !status.mpv &&
      <p role="alert">Install mpv with your distribution's package manager.</p>}
    {windows && (!status.ytDlp || !status.mpv) &&
      <p className="runtime-hint">Restart Fuzzy Music after installing the tools if playback is still unavailable.</p>}
  </section>;
}
