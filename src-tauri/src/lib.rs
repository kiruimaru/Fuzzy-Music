mod audio_cache;
mod dependencies;
mod library;
mod playback;

use fuzzy_application::{
    AppError, Application, ArtistTracksRequest, ArtistTracksResponse, CancellationToken, ErrorCode,
    PlaylistTracksRequest, PlaylistTracksResponse, RequestOptions, ResolveTrackRequest,
    SearchPlaylistsRequest, SearchPlaylistsResponse, SearchTracksRequest, SearchTracksResponse,
};
use fuzzy_soundcloud::SoundCloudClient;
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tauri::Manager;

struct LibraryState(Option<Arc<library::LibraryService>>);

impl LibraryState {
    fn get(&self) -> Result<Arc<library::LibraryService>, AppError> {
        self.0.clone().ok_or_else(library::storage_error)
    }
}

/// Nearest neighbours whose audio startup segments are kept in RAM; links only
/// are cached for the rest of the lookahead.
const WARM_NEIGHBOURS: usize = 3;
/// Extractor launches allowed per lookahead cycle. The remaining tracks defer
/// to the next cycle so re-resolve storms stay short and spread out.
const RESOLVE_BUDGET: usize = 16;

fn internal() -> AppError {
    AppError {
        code: ErrorCode::Internal,
        message: "Application operation failed. Please retry.".into(),
        retryable: true,
    }
}

fn cancelled() -> AppError {
    AppError {
        code: ErrorCode::Cancelled,
        message: "Request cancelled.".into(),
        retryable: false,
    }
}

#[derive(Default)]
struct Operation {
    generation: u64,
    cancellation: CancellationToken,
}

impl Operation {
    fn begin(&mut self, generation: u64) -> Result<RequestOptions, AppError> {
        if generation <= self.generation {
            return Err(cancelled());
        }
        self.cancellation.cancel();
        self.generation = generation;
        self.cancellation = CancellationToken::default();
        Ok(RequestOptions {
            cancellation: self.cancellation.clone(),
            ..Default::default()
        })
    }

    fn current(&self, generation: u64) -> bool {
        self.generation == generation && !self.cancellation.is_cancelled()
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackStatus {
    generation: u64,
    state: &'static str,
    track_id: Option<String>,
    position_ms: u64,
    duration_ms: Option<u64>,
    volume: f64,
    seekable: bool,
    error: Option<String>,
    transitions: u64,
}

impl Default for PlaybackStatus {
    fn default() -> Self {
        Self {
            generation: 0,
            state: "idle",
            track_id: None,
            position_ms: 0,
            duration_ms: None,
            volume: 100.0,
            seekable: false,
            error: None,
            transitions: 0,
        }
    }
}

#[derive(Default)]
struct Control {
    search: Operation,
    playlists: Operation,
    artists: Operation,
    playback: Operation,
    player: Option<playback::Player>,
    status: PlaybackStatus,
    preferred_volume: Option<f64>,
    closed: bool,
    active: usize,
    preparation: Operation,
    lookahead: Operation,
    cached_streams: Vec<(
        String,
        fuzzy_application::ResolveTrackResponse,
        std::time::Instant,
    )>,
    prepared: Option<(
        String,
        fuzzy_application::ResolveTrackResponse,
        std::time::Instant,
    )>,
}

impl Control {
    fn stop(&mut self, generation: u64) -> Result<(), AppError> {
        self.playback.begin(generation)?;
        self.playback.cancellation.cancel();
        self.player = None;
        self.preparation.cancellation.cancel();
        self.lookahead.cancellation.cancel();
        self.cached_streams.clear();
        self.prepared = None;
        self.status = PlaybackStatus {
            generation,
            state: "stopped",
            volume: self.preferred_volume.unwrap_or(self.status.volume),
            ..Default::default()
        };
        Ok(())
    }

    fn close(&mut self) {
        self.closed = true;
        self.search.cancellation.cancel();
        self.playlists.cancellation.cancel();
        self.artists.cancellation.cancel();
        self.playback.cancellation.cancel();
        self.preparation.cancellation.cancel();
        self.lookahead.cancellation.cancel();
        self.cached_streams.clear();
        self.prepared = None;
        self.player = None;
    }
}

struct Shell {
    network: fuzzy_application::NetworkPolicy,
    audio_cache: Option<Arc<audio_cache::AudioCache>>,
    application: Application,
    mpv: PathBuf,
    control: Mutex<Control>,
    idle: Condvar,
}

struct Active<'a>(&'a Shell);

impl Drop for Active<'_> {
    fn drop(&mut self) {
        let mut control = self.0.control.lock().unwrap_or_else(|e| e.into_inner());
        control.active -= 1;
        self.0.idle.notify_all();
    }
}

impl Shell {
    fn new() -> Self {
        let network = fuzzy_application::NetworkPolicy::detect();
        eprintln!("Network: {}", network.diagnostic());
        Self {
            audio_cache: audio_cache::AudioCache::new(&network).map(Arc::new),
            application: Application::new(
                SoundCloudClient::new(dependencies::executable("FUZZY_YTDLP", "yt-dlp"))
                    .with_network(network.clone()),
            ),
            network,
            mpv: dependencies::executable("FUZZY_MPV", "mpv"),
            control: Mutex::new(Control::default()),
            idle: Condvar::new(),
        }
    }

    fn search(
        &self,
        generation: u64,
        request: SearchTracksRequest,
    ) -> Result<SearchTracksResponse, AppError> {
        let options = {
            let mut control = self.control.lock().map_err(|_| internal())?;
            if control.closed {
                return Err(cancelled());
            }
            let options = control.search.begin(generation)?;
            control.active += 1;
            options
        };
        let _active = Active(self);
        #[cfg(not(test))]
        let result = self.application.search_tracks_fast(request, &options);
        #[cfg(test)]
        let result = self.application.search_tracks(request, &options);
        let control = self.control.lock().map_err(|_| internal())?;
        if !control.search.current(generation) || control.closed {
            return Err(cancelled());
        }
        result
    }

    fn search_playlists(
        &self,
        generation: u64,
        request: SearchPlaylistsRequest,
    ) -> Result<SearchPlaylistsResponse, AppError> {
        let options = {
            let mut control = self.control.lock().map_err(|_| internal())?;
            if control.closed {
                return Err(cancelled());
            }
            let options = control.playlists.begin(generation)?;
            control.active += 1;
            options
        };
        let _active = Active(self);
        let result = self.application.search_playlists(request, &options);
        let control = self.control.lock().map_err(|_| internal())?;
        if !control.playlists.current(generation) || control.closed {
            return Err(cancelled());
        }
        result
    }

    fn playlist_tracks(
        &self,
        generation: u64,
        request: PlaylistTracksRequest,
    ) -> Result<PlaylistTracksResponse, AppError> {
        let options = {
            let mut control = self.control.lock().map_err(|_| internal())?;
            if control.closed {
                return Err(cancelled());
            }
            let options = control.playlists.begin(generation)?;
            control.active += 1;
            options
        };
        let _active = Active(self);
        let result = self.application.playlist_tracks(request, &options);
        let control = self.control.lock().map_err(|_| internal())?;
        if !control.playlists.current(generation) || control.closed {
            return Err(cancelled());
        }
        result
    }

    fn artist_tracks(
        &self,
        generation: u64,
        request: ArtistTracksRequest,
    ) -> Result<ArtistTracksResponse, AppError> {
        let options = {
            let mut control = self.control.lock().map_err(|_| internal())?;
            if control.closed {
                return Err(cancelled());
            }
            let options = control.artists.begin(generation)?;
            control.active += 1;
            options
        };
        let _active = Active(self);
        let result = self.application.artist_tracks(request, &options);
        let control = self.control.lock().map_err(|_| internal())?;
        if !control.artists.current(generation) || control.closed {
            return Err(cancelled());
        }
        result
    }

    #[cfg(test)]
    fn play(&self, generation: u64, request: ResolveTrackRequest) -> Result<(), AppError> {
        self.play_with_options(generation, request, false, None)
    }

    fn play_with_options(
        &self,
        generation: u64,
        request: ResolveTrackRequest,
        fresh: bool,
        volume: Option<f64>,
    ) -> Result<(), AppError> {
        if let Some(volume) = volume
            && (!volume.is_finite() || !(0.0..=100.0).contains(&volume))
        {
            return Err(AppError {
                code: ErrorCode::InvalidInput,
                message: "Volume must be between 0 and 100.".into(),
                retryable: false,
            });
        }
        let track_url = request.track_url.clone();
        let (options, prepared) = {
            let mut control = self.control.lock().map_err(|_| internal())?;
            if control.closed {
                return Err(cancelled());
            }
            let options = control.playback.begin(generation)?;
            let prepared = control.prepared.take().filter(|(url, stream, when)| {
                !fresh
                    && *url == request.track_url
                    && audio_cache::link_is_fresh(
                        &stream.stream_url,
                        *when,
                        std::time::Duration::from_secs(60),
                    )
            });
            control.cached_streams.retain(|(_, stream, when)| {
                audio_cache::link_is_fresh(
                    &stream.stream_url,
                    *when,
                    std::time::Duration::from_secs(60),
                )
            });
            let cached = control
                .cached_streams
                .iter()
                .position(|(url, _, _)| !fresh && *url == request.track_url)
                .map(|index| control.cached_streams[index].1.clone());
            control.preparation.cancellation.cancel();
            if fresh {
                control.lookahead.cancellation.cancel();
                // Stop old playback before resolving a replacement. Its EOF must
                // never play an appended successor under the new generation.
                control.player = None;
            }
            if let Some(volume) = volume {
                control.preferred_volume = Some(volume);
            }
            control.status = PlaybackStatus {
                generation,
                state: "loading",
                volume: control.preferred_volume.unwrap_or(control.status.volume),
                ..Default::default()
            };
            control.active += 1;
            (options, prepared.map(|(_, stream, _)| stream).or(cached))
        };
        let _active = Active(self);
        let resolved = match prepared {
            Some(stream) => Ok(stream),
            None => self.application.resolve_track(request, &options),
        };
        let mut control = self.control.lock().map_err(|_| internal())?;
        if !control.playback.current(generation) || control.closed {
            return Err(cancelled());
        }
        let result = resolved.and_then(|stream| {
            if !control
                .cached_streams
                .iter()
                .any(|(key, _, _)| key == &track_url)
            {
                if control.cached_streams.len() >= 41 {
                    control.cached_streams.remove(0);
                }
                control.cached_streams.push((
                    track_url.clone(),
                    stream.clone(),
                    std::time::Instant::now(),
                ));
            }
            let playback_url = (!fresh)
                .then(|| {
                    self.audio_cache
                        .as_ref()
                        .and_then(|cache| cache.get(&track_url))
                })
                .flatten()
                .unwrap_or_else(|| stream.stream_url.clone());
            if !fresh
                && let Some(cache) = &self.audio_cache
                && cache.get(&track_url).is_none()
            {
                let cache = Arc::clone(cache);
                let epoch = cache.epoch();
                let key = track_url.clone();
                let source = stream.stream_url.clone();
                std::thread::spawn(move || cache.warm_at(&key, &source, epoch));
            }
            let playback_volume = control.status.volume;
            if let Some(player) = control.player.as_mut() {
                player.load(&playback_url)?;
                player.volume(playback_volume)?;
            } else {
                let player = playback::Player::start_with_volume(
                    &self.mpv,
                    &playback_url,
                    Some(&self.network),
                    playback_volume,
                )?;
                control.player = Some(player);
            }
            control.status.track_id = Some(stream.track.id);
            Ok(())
        });
        if let Err(error) = &result {
            control.player = None;
            control.status.state = "error";
            control.status.error = Some(error.message.clone());
        }
        result
    }

    fn shutdown(&self) {
        if let Some(cache) = &self.audio_cache {
            cache.clear();
        }
        let mut control = self.control.lock().unwrap_or_else(|e| e.into_inner());
        control.close();
        while control.active != 0 {
            control = self.idle.wait(control).unwrap_or_else(|e| e.into_inner());
        }
    }

    fn play_local(
        &self,
        generation: u64,
        path: PathBuf,
        track_id: String,
        volume: Option<f64>,
    ) -> Result<(), AppError> {
        if volume.is_some_and(|v| !v.is_finite() || !(0.0..=100.0).contains(&v)) {
            return Err(AppError {
                code: ErrorCode::InvalidInput,
                message: "Invalid volume.".into(),
                retryable: false,
            });
        }
        let mut control = self.control.lock().map_err(|_| internal())?;
        if control.closed {
            return Err(cancelled());
        }
        control.playback.begin(generation)?;
        control.preparation.cancellation.cancel();
        control.lookahead.cancellation.cancel();
        control.prepared = None;
        control.player = None;
        if let Some(volume) = volume {
            control.preferred_volume = Some(volume);
        }
        let playback_volume = control.preferred_volume.unwrap_or(control.status.volume);
        control.status = PlaybackStatus {
            generation,
            state: "loading",
            track_id: Some(track_id),
            volume: playback_volume,
            ..Default::default()
        };
        let result = playback::Player::start_with_volume(
            &self.mpv,
            path.to_str().ok_or_else(internal)?,
            Some(&self.network),
            playback_volume,
        );
        match result {
            Ok(player) => {
                control.player = Some(player);
                Ok(())
            }
            Err(error) => {
                control.status.state = "error";
                control.status.error = Some(error.message.clone());
                Err(error)
            }
        }
    }

    fn status(&self) -> Result<PlaybackStatus, AppError> {
        let mut control = self.control.lock().map_err(|_| internal())?;
        if let Some(player) = &mut control.player {
            let result = player.poll();
            match result {
                Ok(snapshot) => {
                    if snapshot.advanced
                        && let Some((_, stream, _)) = control.prepared.take()
                    {
                        control.status.track_id = Some(stream.track.id);
                        control.status.transitions += 1;
                    }
                    control.status.state = snapshot.state;
                    control.status.position_ms = snapshot.position_ms;
                    control.status.duration_ms = snapshot.duration_ms;
                    control.status.volume = snapshot.volume;
                    control.status.seekable = snapshot.seekable;
                }
                Err(error) => {
                    control.player = None;
                    control.status.state = "error";
                    control.status.error = Some(error.message);
                }
            }
        }
        Ok(control.status.clone())
    }
}

#[tauri::command]
async fn get_settings(
    library: tauri::State<'_, LibraryState>,
) -> Result<library::AppSettings, AppError> {
    let library = library.get()?;
    worker(move || library.library.settings()).await
}

#[tauri::command]
async fn update_settings(
    library: tauri::State<'_, LibraryState>,
    settings: library::SettingsPatch,
) -> Result<library::AppSettings, AppError> {
    let library = library.get()?;
    worker(move || library.library.update_settings(settings)).await
}

#[tauri::command]
async fn library_entries(
    library: tauri::State<'_, LibraryState>,
) -> Result<Vec<library::LibraryEntry>, AppError> {
    let library = library.get()?;
    worker(move || library.library.entries()).await
}

#[tauri::command]
async fn scan_library(
    library: tauri::State<'_, LibraryState>,
) -> Result<library::ScanResult, AppError> {
    let library = library.get()?;
    worker(move || library.library.scan()).await
}

#[tauri::command]
async fn set_favourite(
    library: tauri::State<'_, LibraryState>,
    track: fuzzy_application::Track,
    liked: bool,
) -> Result<(), AppError> {
    let library = library.get()?;
    worker(move || library.set_favourite(track, liked)).await
}

#[tauri::command]
async fn playlist_entries(
    library: tauri::State<'_, LibraryState>,
) -> Result<Vec<library::PlaylistEntry>, AppError> {
    let library = library.get()?;
    worker(move || library.library.playlists()).await
}

#[tauri::command]
async fn set_playlist_favourite(
    library: tauri::State<'_, LibraryState>,
    playlist: fuzzy_application::Playlist,
    liked: bool,
) -> Result<(), AppError> {
    let library = library.get()?;
    worker(move || library.set_playlist_favourite(playlist, liked)).await
}

#[tauri::command]
async fn download_playlist(
    shell: tauri::State<'_, Arc<Shell>>,
    library: tauri::State<'_, LibraryState>,
    playlist: fuzzy_application::Playlist,
) -> Result<(), AppError> {
    let shell = shell.inner().clone();
    let library = library.get()?;
    worker(move || {
        let options = RequestOptions {
            timeout: std::time::Duration::from_secs(180),
            cancellation: CancellationToken::default(),
        };
        let response = shell.application.playlist_tracks(
            PlaylistTracksRequest {
                source: playlist.source.clone(),
                playlist_url: playlist.source_url.clone(),
            },
            &options,
        )?;
        library.download_playlist(response.playlist, response.tracks)
    })
    .await
}

#[tauri::command]
async fn cancel_playlist_download(
    library: tauri::State<'_, LibraryState>,
    playlist: fuzzy_application::Playlist,
) -> Result<(), AppError> {
    let library = library.get()?;
    worker(move || library.cancel_playlist_download(&playlist)).await
}

#[tauri::command]
async fn retry_download(library: tauri::State<'_, LibraryState>, id: i64) -> Result<(), AppError> {
    let library = library.get()?;
    worker(move || library.enqueue(id)).await
}

#[tauri::command]
async fn remove_library_track(
    library: tauri::State<'_, LibraryState>,
    id: i64,
) -> Result<(), AppError> {
    let library = library.get()?;
    worker(move || library.remove(id)).await
}

#[tauri::command]
async fn play_library_track(
    shell: tauri::State<'_, Arc<Shell>>,
    library: tauri::State<'_, LibraryState>,
    generation: u64,
    id: i64,
    volume: Option<f64>,
) -> Result<(), AppError> {
    let shell = shell.inner().clone();
    let library = library.get()?;
    worker(move || {
        let result = library
            .library
            .entry(id)
            .and_then(|entry| library.library.path(id).map(|path| (entry, path)));
        match result {
            Ok((entry, path)) => shell.play_local(generation, path, entry.track.id, volume),
            Err(error) => {
                // Invalid or missing audio is still a new playback intent.
                shell
                    .control
                    .lock()
                    .map_err(|_| internal())?
                    .stop(generation)?;
                Err(error)
            }
        }
    })
    .await
}

#[tauri::command]
async fn choose_music_folder(app: tauri::AppHandle) -> Result<Option<String>, AppError> {
    use tauri_plugin_dialog::DialogExt;
    worker(move || {
        app.dialog()
            .file()
            .blocking_pick_folder()
            .map(|path| {
                path.into_path()
                    .map_err(|_| library::storage_error())
                    .map(|path| path.to_string_lossy().into_owned())
            })
            .transpose()
    })
    .await
}

#[tauri::command]
async fn choose_background_image(app: tauri::AppHandle) -> Result<Option<String>, AppError> {
    use tauri_plugin_dialog::DialogExt;
    worker(move || {
        let picked = app
            .dialog()
            .file()
            .add_filter(
                "Images",
                &[
                    "png", "jpg", "jpeg", "webp", "gif", "bmp", "avif", "heic", "tif", "tiff",
                    "svg",
                ],
            )
            .add_filter("All files", &["*"])
            .blocking_pick_file();
        let Some(picked) = picked else {
            return Ok(None);
        };
        let source = picked.into_path().map_err(|_| library::storage_error())?;
        let directory = app
            .path()
            .app_data_dir()
            .map_err(|_| library::storage_error())?
            .join("background");
        std::fs::create_dir_all(&directory).map_err(|_| library::storage_error())?;
        for entry in std::fs::read_dir(&directory)
            .map_err(|_| library::storage_error())?
            .flatten()
        {
            let _ = std::fs::remove_file(entry.path());
        }
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .filter(|value| {
                (1..=8).contains(&value.len())
                    && value
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric())
            })
            .unwrap_or_else(|| "image".into());
        let target = directory.join(format!("background.{extension}"));
        std::fs::copy(&source, &target).map_err(|_| library::storage_error())?;
        Ok(Some(target.to_string_lossy().into_owned()))
    })
    .await
}

#[tauri::command]
async fn clear_background_image(app: tauri::AppHandle) -> Result<(), AppError> {
    worker(move || {
        let directory = app
            .path()
            .app_data_dir()
            .map_err(|_| library::storage_error())?
            .join("background");
        if directory.is_dir() {
            for entry in std::fs::read_dir(&directory)
                .map_err(|_| library::storage_error())?
                .flatten()
            {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        Ok(())
    })
    .await
}

impl Shell {
    fn prepare(
        &self,
        generation: u64,
        playback_generation: u64,
        request: ResolveTrackRequest,
    ) -> Result<(), AppError> {
        let options = {
            let mut control = self.control.lock().map_err(|_| internal())?;
            if control.closed || !control.playback.current(playback_generation) {
                return Err(cancelled());
            }
            let options = control.preparation.begin(generation)?;
            control.prepared = None;
            control.active += 1;
            options
        };
        let _active = Active(self);
        let url = request.track_url.clone();
        let cached = {
            let mut control = self.control.lock().map_err(|_| internal())?;
            control.cached_streams.retain(|(_, stream, when)| {
                audio_cache::link_is_fresh(
                    &stream.stream_url,
                    *when,
                    std::time::Duration::from_secs(60),
                )
            });
            control
                .cached_streams
                .iter()
                .position(|(key, stream, when)| {
                    *key == url
                        && audio_cache::link_is_fresh(
                            &stream.stream_url,
                            *when,
                            std::time::Duration::from_secs(30),
                        )
                })
                .map(|index| {
                    let (_, stream, when) = &control.cached_streams[index];
                    (stream.clone(), *when)
                })
        };
        let result = match cached {
            Some(stream) => Ok(stream),
            None => self
                .application
                .resolve_track(request, &options)
                .map(|stream| (stream, std::time::Instant::now())),
        };
        let mut control = self.control.lock().map_err(|_| internal())?;
        if control.closed
            || !control.preparation.current(generation)
            || !control.playback.current(playback_generation)
        {
            return Err(cancelled());
        }
        let (stream, resolved_at) = result?;
        let playback_url = self
            .audio_cache
            .as_ref()
            .and_then(|cache| cache.get(&url))
            .unwrap_or_else(|| stream.stream_url.clone());
        if let Some(player) = control.player.as_mut() {
            player.append(&playback_url)?;
        }
        control.prepared = Some((url, stream, resolved_at));
        Ok(())
    }
}

impl Shell {
    fn prefetch(
        &self,
        generation: u64,
        playback_generation: u64,
        requests: Vec<ResolveTrackRequest>,
    ) -> Result<(), AppError> {
        if requests.len() > 40 {
            return Err(internal());
        }
        let options = {
            let mut control = self.control.lock().map_err(|_| internal())?;
            if control.closed || !control.playback.current(playback_generation) {
                return Err(cancelled());
            }
            let options = control.lookahead.begin(generation)?;
            control.active += 1;
            options
        };
        let _active = Active(self);
        let mut seen = std::collections::HashSet::new();
        let pending = Mutex::new(
            requests
                .into_iter()
                .filter(|request| seen.insert(request.track_url.clone()))
                .enumerate()
                .map(|(index, request)| (request, index < WARM_NEIGHBOURS))
                .collect::<std::collections::VecDeque<_>>(),
        );
        // Only a bounded number of extractor runs per cycle; leftover tracks
        // are retried by the next periodic lookahead request.
        let resolves = AtomicUsize::new(RESOLVE_BUDGET);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let options = &options;
                let pending = &pending;
                let resolves = &resolves;
                scope.spawn(move || {
                    loop {
                        let (request, warm) = {
                            let Ok(mut pending) = pending.lock() else {
                                return;
                            };
                            let Some((request, warm)) = pending.pop_front() else {
                                return;
                            };
                            (request, warm)
                        };
                        let cached = {
                            let Ok(mut control) = self.control.lock() else {
                                return;
                            };
                            if control.closed || !control.lookahead.current(generation) {
                                return;
                            }
                            control.cached_streams.retain(|(_, stream, when)| {
                                audio_cache::link_is_fresh(
                                    &stream.stream_url,
                                    *when,
                                    std::time::Duration::from_secs(45),
                                )
                            });
                            control
                                .cached_streams
                                .iter()
                                .find(|(key, _, _)| *key == request.track_url)
                                .map(|(_, stream, when)| (stream.clone(), *when))
                        };
                        let url = request.track_url.clone();
                        let (stream, resolved_at) = match cached {
                            Some(cached) => cached,
                            None => {
                                // Stop launching extractors once the cycle
                                // budget is spent; deferred tracks are retried
                                // by the next lookahead cycle.
                                if resolves
                                    .try_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                                        value.checked_sub(1)
                                    })
                                    .is_err()
                                {
                                    return;
                                }
                                match self.application.resolve_track(request, options) {
                                    Ok(stream) => (stream, std::time::Instant::now()),
                                    Err(_) => continue,
                                }
                            }
                        };
                        let Ok(mut control) = self.control.lock() else {
                            return;
                        };
                        if control.closed || !control.lookahead.current(generation) {
                            return;
                        }
                        control.cached_streams.retain(|(key, _, _)| *key != url);
                        if control.cached_streams.len() >= 41 {
                            control.cached_streams.remove(0);
                        }
                        let stream_url = stream.stream_url.clone();
                        control
                            .cached_streams
                            .push((url.clone(), stream, resolved_at));
                        drop(control);
                        if warm
                            && !options.cancellation.is_cancelled()
                            && let Some(cache) = &self.audio_cache
                        {
                            cache.warm_cancellable(&url, &stream_url, &options.cancellation);
                        }
                    }
                });
            }
        });
        let control = self.control.lock().map_err(|_| internal())?;
        if control.closed || !control.lookahead.current(generation) {
            return Err(cancelled());
        }
        Ok(())
    }
}

#[tauri::command]
async fn prefetch_tracks(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
    playback_generation: u64,
    requests: Vec<ResolveTrackRequest>,
) -> Result<(), AppError> {
    let shell = shell.inner().clone();
    worker(move || shell.prefetch(generation, playback_generation, requests)).await
}

#[tauri::command]
async fn prepare_track(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
    playback_generation: u64,
    request: ResolveTrackRequest,
) -> Result<(), AppError> {
    let shell = shell.inner().clone();
    worker(move || shell.prepare(generation, playback_generation, request)).await
}

#[tauri::command]
async fn cancel_preparation(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
) -> Result<(), AppError> {
    let shell = shell.inner().clone();
    worker(move || {
        let mut control = shell.control.lock().map_err(|_| internal())?;
        control.preparation.begin(generation)?;
        control.preparation.cancellation.cancel();
        control.prepared = None;
        Ok(())
    })
    .await
}

async fn worker<T: Send + 'static>(
    task: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|_| internal())?
}

#[tauri::command]
async fn search_tracks(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
    request: SearchTracksRequest,
) -> Result<SearchTracksResponse, AppError> {
    let shell = shell.inner().clone();
    worker(move || shell.search(generation, request)).await
}

#[tauri::command]
async fn cancel_search(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
) -> Result<(), AppError> {
    let shell = shell.inner().clone();
    worker(move || {
        let mut control = shell.control.lock().map_err(|_| internal())?;
        control.search.begin(generation)?;
        control.search.cancellation.cancel();
        Ok(())
    })
    .await
}

#[tauri::command]
async fn search_playlists(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
    request: SearchPlaylistsRequest,
) -> Result<SearchPlaylistsResponse, AppError> {
    let shell = shell.inner().clone();
    worker(move || shell.search_playlists(generation, request)).await
}

#[tauri::command]
async fn playlist_tracks(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
    request: PlaylistTracksRequest,
) -> Result<PlaylistTracksResponse, AppError> {
    let shell = shell.inner().clone();
    worker(move || shell.playlist_tracks(generation, request)).await
}

#[tauri::command]
async fn cancel_playlists(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
) -> Result<(), AppError> {
    let shell = shell.inner().clone();
    worker(move || {
        let mut control = shell.control.lock().map_err(|_| internal())?;
        control.playlists.begin(generation)?;
        control.playlists.cancellation.cancel();
        Ok(())
    })
    .await
}

#[tauri::command]
async fn artist_tracks(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
    request: ArtistTracksRequest,
) -> Result<ArtistTracksResponse, AppError> {
    let shell = shell.inner().clone();
    worker(move || shell.artist_tracks(generation, request)).await
}

#[tauri::command]
async fn cancel_artists(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
) -> Result<(), AppError> {
    let shell = shell.inner().clone();
    worker(move || {
        let mut control = shell.control.lock().map_err(|_| internal())?;
        control.artists.begin(generation)?;
        control.artists.cancellation.cancel();
        Ok(())
    })
    .await
}

#[tauri::command]
async fn play_track(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
    request: ResolveTrackRequest,
    fresh: Option<bool>,
    volume: Option<f64>,
) -> Result<(), AppError> {
    let shell = shell.inner().clone();
    worker(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            shell.play_with_options(generation, request, fresh.unwrap_or(false), volume)
        }))
        .unwrap_or_else(|_| Err(internal()));
        if let Err(error) = &result
            && let Ok(mut control) = shell.control.lock()
            && control.playback.current(generation)
        {
            control.status.state = "error";
            control.status.error = Some(error.message.clone());
        }
        result
    })
    .await
}

#[tauri::command]
async fn stop_playback(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
) -> Result<(), AppError> {
    let shell = shell.inner().clone();
    worker(move || {
        let result = shell
            .control
            .lock()
            .map_err(|_| internal())?
            .stop(generation);
        if result.is_ok()
            && let Some(cache) = &shell.audio_cache
        {
            cache.clear();
        }
        result
    })
    .await
}

#[tauri::command]
async fn playback_status(shell: tauri::State<'_, Arc<Shell>>) -> Result<PlaybackStatus, AppError> {
    let shell = shell.inner().clone();
    worker(move || shell.status()).await
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum PlaybackAction {
    Pause { paused: bool },
    Seek { position_ms: u64 },
    Volume { volume: f64 },
}

impl Shell {
    fn update_playback(&self, generation: u64, action: PlaybackAction) -> Result<(), AppError> {
        let mut control = self.control.lock().map_err(|_| internal())?;
        if control.closed || !control.playback.current(generation) {
            return Err(cancelled());
        }
        let player = control.player.as_mut().ok_or_else(playback::failure)?;
        match action {
            PlaybackAction::Pause { paused } => player.pause(paused),
            PlaybackAction::Seek { position_ms } => player.seek(position_ms),
            PlaybackAction::Volume { volume } => {
                if !volume.is_finite() || !(0.0..=100.0).contains(&volume) {
                    return Err(AppError {
                        code: ErrorCode::InvalidInput,
                        message: "Volume must be between 0 and 100.".into(),
                        retryable: false,
                    });
                }
                player.volume(volume)?;
                control.preferred_volume = Some(volume);
                Ok(())
            }
        }
    }
}

#[tauri::command]
async fn update_playback(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
    action: PlaybackAction,
) -> Result<(), AppError> {
    let shell = shell.inner().clone();
    worker(move || shell.update_playback(generation, action)).await
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(Shell::new()))
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            let music = app
                .path()
                .audio_dir()
                .unwrap_or_else(|_| data.clone())
                .join("Fuzzy Music");
            let service = library::Library::open(&data, &music).ok().map(|library| {
                let service = library::LibraryService::start(
                    library,
                    dependencies::executable("FUZZY_YTDLP", "yt-dlp"),
                    fuzzy_application::NetworkPolicy::detect(),
                );
                let startup = Arc::clone(&service);
                std::thread::spawn(move || {
                    let _ = startup.library.scan();
                    let _ = startup.resume();
                });
                service
            });
            app.manage(LibraryState(service));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            library_entries,
            scan_library,
            set_favourite,
            playlist_entries,
            set_playlist_favourite,
            download_playlist,
            cancel_playlist_download,
            retry_download,
            remove_library_track,
            play_library_track,
            choose_music_folder,
            choose_background_image,
            clear_background_image,
            search_tracks,
            cancel_search,
            search_playlists,
            playlist_tracks,
            cancel_playlists,
            artist_tracks,
            cancel_artists,
            play_track,
            stop_playback,
            playback_status,
            update_playback,
            prepare_track,
            prefetch_tracks,
            cancel_preparation
        ])
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                // Prevent teardown until owned children have been killed and reaped.
                let shell = window.state::<Arc<Shell>>().inner().clone();
                let library = window.state::<LibraryState>().0.clone();
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                }
                let app = window.app_handle().clone();
                tauri::async_runtime::spawn_blocking(move || {
                    shell.shutdown();
                    if let Some(library) = library {
                        library.shutdown();
                    }
                    app.exit(0);
                });
            }
        })
        .build(tauri::generate_context!())
        .expect("Could not initialize Fuzzy Music")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<Arc<Shell>>().shutdown();
                if let Some(library) = &app.state::<LibraryState>().0 {
                    library.shutdown();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process::Command,
        time::{Duration, Instant},
    };

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "fuzzy-shell-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&path).unwrap();
            let result = Command::new("rustc")
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tests/fixtures/tool.rs"
                ))
                .arg("-o")
                .arg(path.join(format!("tool{}", std::env::consts::EXE_SUFFIX)))
                .output()
                .unwrap();
            assert!(result.status.success(), "{:?}", result);
            Self(path)
        }

        fn shell(&self) -> Arc<Shell> {
            let executable = self.0.join(format!("tool{}", std::env::consts::EXE_SUFFIX));
            Arc::new(Shell {
                network: fuzzy_application::NetworkPolicy::default(),
                audio_cache: None,
                application: Application::new(SoundCloudClient::new(&executable)),
                mpv: executable,
                control: Mutex::new(Control::default()),
                idle: Condvar::new(),
            })
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn request(path: &str) -> ResolveTrackRequest {
        ResolveTrackRequest {
            source: fuzzy_application::Source::Soundcloud,
            track_url: format!("https://soundcloud.com/a/{path}"),
        }
    }

    fn wait_active(shell: &Shell) {
        let start = Instant::now();
        while shell.control.lock().unwrap().active == 0 {
            assert!(start.elapsed() < Duration::from_secs(3));
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn replacement_and_out_of_order_requests_are_cancelled() {
        let mut operation = Operation::default();
        let first = operation.begin(1).unwrap();
        let second = operation.begin(3).unwrap();
        assert!(first.cancellation.is_cancelled());
        assert!(operation.begin(2).is_err());
        assert!(operation.begin(3).is_err());
        assert!(!operation.current(1));
        assert!(operation.current(3));
        assert!(!second.cancellation.is_cancelled());
    }

    #[test]
    fn preparation_is_bound_to_playback_and_stop_clears_it() {
        let fixture = Fixture::new();
        let shell = fixture.shell();
        shell.play(1, request("ok")).unwrap();
        shell.prepare(1, 1, request("ok")).unwrap();
        assert!(shell.control.lock().unwrap().prepared.is_some());
        shell.control.lock().unwrap().stop(2).unwrap();
        assert!(shell.control.lock().unwrap().prepared.is_none());
        assert_eq!(
            shell.prepare(2, 1, request("ok")).unwrap_err().code,
            ErrorCode::Cancelled
        );
        shell.shutdown();
    }

    #[test]
    fn prepared_stream_is_consumed_without_running_another_extractor() {
        let fixture = Fixture::new();
        let mut shell = fixture.shell();
        shell.play(1, request("ok")).unwrap();
        shell.prepare(1, 1, request("ok")).unwrap();
        Arc::get_mut(&mut shell).unwrap().application =
            Application::new(SoundCloudClient::new(fixture.0.join("missing")));
        shell.play(2, request("ok")).unwrap();
        assert!(shell.control.lock().unwrap().prepared.is_none());
        shell.shutdown();
    }

    #[test]
    fn lookahead_is_used_for_manual_play_and_cleared_on_stop() {
        let fixture = Fixture::new();
        let mut shell = fixture.shell();
        shell.play(1, request("ok")).unwrap();
        shell
            .prefetch(
                1,
                1,
                vec![
                    request("next"),
                    request("ok"),
                    request("after"),
                    request("four"),
                    request("five"),
                    request("six"),
                ],
            )
            .unwrap();
        assert_eq!(shell.control.lock().unwrap().cached_streams.len(), 6);
        Arc::get_mut(&mut shell).unwrap().application =
            Application::new(SoundCloudClient::new(fixture.0.join("missing")));
        shell.play(2, request("next")).unwrap();
        assert_eq!(shell.control.lock().unwrap().cached_streams.len(), 6);
        shell.play(3, request("ok")).unwrap();
        assert_eq!(shell.control.lock().unwrap().cached_streams.len(), 6);
        shell.control.lock().unwrap().stop(4).unwrap();
        assert!(shell.control.lock().unwrap().cached_streams.is_empty());
        shell.shutdown();
    }

    #[test]
    fn lookahead_resolves_nearby_tracks_concurrently() {
        let fixture = Fixture::new();
        let shell = fixture.shell();
        shell.play(1, request("ok")).unwrap();
        let start = Instant::now();
        shell
            .prefetch(
                1,
                1,
                (0..4)
                    .map(|index| request(&format!("slow{index}")))
                    .collect(),
            )
            .unwrap();
        assert!(
            start.elapsed() < Duration::from_millis(2700),
            "lookahead ran sequentially"
        );
        assert_eq!(shell.control.lock().unwrap().cached_streams.len(), 5);
        shell.shutdown();
    }

    #[test]
    fn a_slow_track_does_not_block_later_prefetches() {
        let fixture = Fixture::new();
        let shell = fixture.shell();
        shell.play(1, request("ok")).unwrap();
        let background = shell.clone();
        let worker = std::thread::spawn(move || {
            let requests = std::iter::once(request("slow_block"))
                .chain((0..8).map(|i| request(&format!("fast{i}"))))
                .collect();
            background.prefetch(1, 1, requests).unwrap();
        });
        let deadline = Instant::now() + Duration::from_millis(1800);
        loop {
            if shell
                .control
                .lock()
                .unwrap()
                .cached_streams
                .iter()
                .any(|(key, _, _)| key.ends_with("fast7"))
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "a slow track blocked later requests"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        worker.join().unwrap();
        shell.shutdown();
    }

    #[test]
    fn manual_switch_does_not_cancel_playlist_lookahead() {
        let fixture = Fixture::new();
        let shell = fixture.shell();
        shell.play(1, request("ok")).unwrap();
        let background = shell.clone();
        let worker =
            std::thread::spawn(move || background.prefetch(1, 1, vec![request("slow_block")]));
        wait_active(&shell);
        shell.play(2, request("next")).unwrap();
        worker.join().unwrap().unwrap();
        assert!(
            shell
                .control
                .lock()
                .unwrap()
                .cached_streams
                .iter()
                .any(|(key, _, _)| key.ends_with("slow_block"))
        );
        shell.shutdown();
    }

    #[test]
    fn repeated_forward_and_backward_switches_reuse_cached_links() {
        let fixture = Fixture::new();
        let mut shell = fixture.shell();
        shell.play(1, request("track0")).unwrap();
        shell
            .prefetch(
                1,
                1,
                (1..=8).map(|i| request(&format!("track{i}"))).collect(),
            )
            .unwrap();
        Arc::get_mut(&mut shell).unwrap().application =
            Application::new(SoundCloudClient::new(fixture.0.join("missing")));
        for (generation, index) in (2..).zip((1..=8).chain((0..8).rev()).chain(1..=8)) {
            shell
                .play(generation, request(&format!("track{index}")))
                .unwrap();
        }
        shell.shutdown();
    }

    #[test]
    fn prepared_track_advances_in_player_without_another_play_command() {
        let fixture = Fixture::new();
        let shell = fixture.shell();
        shell.play(1, request("ok")).unwrap();
        let start = Instant::now();
        while shell.status().unwrap().state != "playing" {
            assert!(start.elapsed() < Duration::from_secs(3));
            std::thread::sleep(Duration::from_millis(10));
        }
        shell.prepare(1, 1, request("next")).unwrap();
        shell
            .control
            .lock()
            .unwrap()
            .player
            .as_mut()
            .unwrap()
            .seek(175000)
            .unwrap();
        let start = Instant::now();
        loop {
            let status = shell.status().unwrap();
            if status.transitions == 1 {
                assert_eq!(status.generation, 1);
                assert_eq!(status.state, "playing");
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(3));
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(shell.control.lock().unwrap().prepared.is_none());
        shell.shutdown();
    }

    #[test]
    fn stop_invalidates_resolve_even_if_play_arrives_late_and_allows_retry() {
        let mut control = Control::default();
        let pending = control.playback.begin(1).unwrap();
        control.stop(2).unwrap();
        assert!(pending.cancellation.is_cancelled());
        assert!(!control.playback.current(1));
        assert!(control.playback.begin(1).is_err());
        assert_eq!(control.status.state, "stopped");
        assert!(control.playback.begin(3).is_ok());
    }

    #[test]
    fn close_cancels_both_operations() {
        let mut control = Control::default();
        let search = control.search.begin(1).unwrap();
        let resolve = control.playback.begin(1).unwrap();
        control.close();
        assert!(search.cancellation.is_cancelled());
        assert!(resolve.cancellation.is_cancelled());
        assert!(control.closed);
    }

    #[test]
    fn stop_during_real_resolve_worker_never_starts_player() {
        let fixture = Fixture::new();
        let shell = fixture.shell();
        let worker_shell = shell.clone();
        let worker = std::thread::spawn(move || worker_shell.play(1, request("hang")));
        wait_active(&shell);
        shell.control.lock().unwrap().stop(2).unwrap();
        assert_eq!(
            worker.join().unwrap().unwrap_err().code,
            ErrorCode::Cancelled
        );
        assert!(shell.control.lock().unwrap().player.is_none());
        shell.play(3, request("ok")).unwrap();
        let start = Instant::now();
        while shell.status().unwrap().state != "playing" {
            assert!(start.elapsed() < Duration::from_secs(3));
            std::thread::sleep(Duration::from_millis(10));
        }
        shell.shutdown();
        assert!(shell.control.lock().unwrap().player.is_none());
    }

    #[test]
    fn close_waits_for_extractor_cleanup_and_rejects_late_requests() {
        let fixture = Fixture::new();
        let shell = fixture.shell();
        let worker_shell = shell.clone();
        let worker = std::thread::spawn(move || {
            worker_shell.search(
                1,
                SearchTracksRequest {
                    query: "hang".into(),
                    limit: None,
                },
            )
        });
        wait_active(&shell);
        shell.shutdown();
        assert_eq!(
            worker.join().unwrap().unwrap_err().code,
            ErrorCode::Cancelled
        );
        assert_eq!(shell.control.lock().unwrap().active, 0);
        assert_eq!(
            shell.play(1, request("ok")).unwrap_err().code,
            ErrorCode::Cancelled
        );
    }

    #[test]
    fn empty_source_failure_and_missing_mpv_allow_retry_without_diagnostics() {
        let fixture = Fixture::new();
        let mut shell = fixture.shell();
        let shell_mut = Arc::get_mut(&mut shell).unwrap();
        shell_mut.mpv = fixture.0.join("missing-mpv");
        assert!(
            shell
                .search(
                    1,
                    SearchTracksRequest {
                        query: "empty".into(),
                        limit: None,
                    }
                )
                .unwrap()
                .tracks
                .is_empty()
        );
        let error = shell.play(1, request("fail")).unwrap_err();
        assert!(!serde_json::to_string(&error).unwrap().contains("SECRET"));
        assert_eq!(shell.status().unwrap().state, "error");
        assert_eq!(
            shell.play(2, request("ok")).unwrap_err().code,
            ErrorCode::PlaybackFailed
        );
        assert_eq!(shell.status().unwrap().state, "error");
        Arc::get_mut(&mut shell).unwrap().mpv = fixture
            .0
            .join(format!("tool{}", std::env::consts::EXE_SUFFIX));
        assert!(shell.play(3, request("unplayable")).is_err());
        let start = Instant::now();
        while shell.status().unwrap().state != "error" {
            assert!(start.elapsed() < Duration::from_secs(3));
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(shell.control.lock().unwrap().player.is_none());
        shell.play(4, request("ok")).unwrap();
        shell.shutdown();
    }

    #[test]
    fn controls_preserve_player_and_reject_stale_generations() {
        let fixture = Fixture::new();
        let shell = fixture.shell();
        shell.play(1, request("ok")).unwrap();
        let wait = |state: &str, position: u64, volume: f64| {
            let start = Instant::now();
            loop {
                let status = shell.status().unwrap();
                if status.state == state
                    && status.position_ms == position
                    && status.volume == volume
                {
                    break;
                }
                assert!(start.elapsed() < Duration::from_secs(3));
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        wait("playing", 0, 100.0);
        shell
            .update_playback(1, PlaybackAction::Pause { paused: true })
            .unwrap();
        wait("paused", 0, 100.0);
        shell
            .update_playback(1, PlaybackAction::Seek { position_ms: 45000 })
            .unwrap();
        wait("paused", 45000, 100.0);
        shell
            .update_playback(1, PlaybackAction::Volume { volume: 30.0 })
            .unwrap();
        wait("paused", 45000, 30.0);
        shell
            .update_playback(1, PlaybackAction::Pause { paused: false })
            .unwrap();
        wait("playing", 45000, 30.0);
        assert_eq!(
            shell
                .update_playback(1, PlaybackAction::Volume { volume: 101.0 })
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            shell
                .update_playback(0, PlaybackAction::Pause { paused: true })
                .unwrap_err()
                .code,
            ErrorCode::Cancelled
        );
        shell.control.lock().unwrap().stop(2).unwrap();
        assert_eq!(
            shell
                .update_playback(1, PlaybackAction::Pause { paused: false })
                .unwrap_err()
                .code,
            ErrorCode::Cancelled
        );
        assert!(shell.control.lock().unwrap().player.is_none());
        shell.shutdown();
    }

    #[test]
    fn session_selection_resolves_fresh_instead_of_using_a_cached_link() {
        let fixture = Fixture::new();
        let mut shell = fixture.shell();
        shell.play(1, request("ok")).unwrap();
        assert!(!shell.control.lock().unwrap().cached_streams.is_empty());
        Arc::get_mut(&mut shell).unwrap().application =
            Application::new(SoundCloudClient::new(fixture.0.join("missing-extractor")));
        // The source playlist fast path can still reuse its previously resolved link.
        shell.play(2, request("ok")).unwrap();
        let error = shell
            .play_with_options(3, request("ok"), true, Some(25.0))
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::SourceUnavailable);
        assert_eq!(shell.status().unwrap().state, "error");
        assert!(shell.control.lock().unwrap().player.is_none());
        shell.shutdown();
    }

    #[test]
    fn fresh_replacement_stops_old_player_before_resolve_and_stop_cancels_it() {
        let fixture = Fixture::new();
        let shell = fixture.shell();
        shell.play(1, request("ok")).unwrap();
        shell.prepare(1, 1, request("next")).unwrap();
        let resolving = {
            let shell = Arc::clone(&shell);
            std::thread::spawn(move || {
                shell.play_with_options(2, request("slow"), true, Some(35.0))
            })
        };
        wait_active(&shell);
        {
            let mut control = shell.control.lock().unwrap();
            assert!(control.player.is_none());
            assert!(control.prepared.is_none());
            assert_eq!(control.status.generation, 2);
            assert_eq!(control.status.volume, 35.0);
            control.stop(3).unwrap();
        }
        assert_eq!(
            resolving.join().unwrap().unwrap_err().code,
            ErrorCode::Cancelled
        );
        let status = shell.status().unwrap();
        assert_eq!(status.state, "stopped");
        assert_eq!(status.volume, 35.0);
        assert!(shell.control.lock().unwrap().player.is_none());
        shell.shutdown();
    }

    #[test]
    fn preferred_volume_survives_stop_and_is_used_for_new_players() {
        let fixture = Fixture::new();
        let shell = fixture.shell();
        for (generation, volume) in [(1, 25.0), (3, 0.0)] {
            shell
                .play_with_options(generation, request("ok"), true, Some(volume))
                .unwrap();
            assert_eq!(shell.status().unwrap().volume, volume);
            shell.control.lock().unwrap().stop(generation + 1).unwrap();
            assert_eq!(shell.status().unwrap().volume, volume);
        }
        shell
            .play_with_options(5, request("ok"), true, None)
            .unwrap();
        shell
            .update_playback(5, PlaybackAction::Volume { volume: 15.0 })
            .unwrap();
        // Stop can arrive before the next property poll acknowledges the volume.
        shell.control.lock().unwrap().stop(6).unwrap();
        assert_eq!(shell.status().unwrap().volume, 15.0);
        assert_eq!(
            shell
                .play_with_options(7, request("ok"), true, Some(f64::NAN))
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert!(shell.control.lock().unwrap().player.is_none());
        shell.shutdown();
    }

    #[test]
    fn indexed_local_playback_uses_owned_player_and_rejects_stale_commands() {
        let fixture = Fixture::new();
        let shell = fixture.shell();
        let path = fixture.0.join("local.wav");
        std::fs::write(&path, b"fixture").unwrap();
        shell
            .play_local(1, path.clone(), "local-1".into(), Some(32.0))
            .unwrap();
        let status = shell.status().unwrap();
        assert_eq!(status.track_id.as_deref(), Some("local-1"));
        assert_eq!(status.volume, 32.0);
        shell.control.lock().unwrap().stop(2).unwrap();
        assert_eq!(
            shell
                .play_local(1, path, "local-1".into(), Some(32.0))
                .unwrap_err()
                .code,
            ErrorCode::Cancelled
        );
        assert!(shell.control.lock().unwrap().player.is_none());
        shell.shutdown();
    }

    #[test]
    #[ignore = "Requires network, yt-dlp, mpv and an audio output; may produce sound"]
    fn live_soundcloud_play_stop_replay() {
        let shell = Shell::new();
        let response = shell
            .search(
                1,
                SearchTracksRequest {
                    query: "lofi hip hop".into(),
                    limit: Some(2),
                },
            )
            .unwrap();
        let track = response.tracks.into_iter().next().expect("live results");
        println!("Live result: {} - {}", track.title, track.author);
        for generation in [1, 3] {
            shell
                .play(
                    generation,
                    ResolveTrackRequest {
                        source: fuzzy_application::Source::Soundcloud,
                        track_url: track.source_url.clone(),
                    },
                )
                .unwrap();
            let start = Instant::now();
            loop {
                let status = shell.status().unwrap();
                assert_ne!(status.state, "error", "{:?}", status.error);
                if status.state == "playing" {
                    break;
                }
                assert!(start.elapsed() < Duration::from_secs(35));
                std::thread::sleep(Duration::from_millis(100));
            }
            println!("mpv reported audio playback");
            std::thread::sleep(Duration::from_secs(2));
            shell.control.lock().unwrap().stop(generation + 1).unwrap();
            assert_eq!(shell.status().unwrap().state, "stopped");
        }
        shell.shutdown();
    }
}
