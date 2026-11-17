mod dependencies;
mod playback;

use fuzzy_application::{
    AppError, Application, CancellationToken, ErrorCode, RequestOptions, ResolveTrackRequest,
    SearchTracksRequest, SearchTracksResponse,
};
use fuzzy_soundcloud::SoundCloudClient;
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
};
use tauri::Manager;

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
    error: Option<String>,
}

impl Default for PlaybackStatus {
    fn default() -> Self {
        Self {
            generation: 0,
            state: "idle",
            track_id: None,
            error: None,
        }
    }
}

#[derive(Default)]
struct Control {
    search: Operation,
    playback: Operation,
    player: Option<playback::Player>,
    status: PlaybackStatus,
    closed: bool,
    active: usize,
}

impl Control {
    fn stop(&mut self, generation: u64) -> Result<(), AppError> {
        self.playback.begin(generation)?;
        self.playback.cancellation.cancel();
        self.player = None;
        self.status = PlaybackStatus {
            generation,
            state: "stopped",
            ..Default::default()
        };
        Ok(())
    }

    fn close(&mut self) {
        self.closed = true;
        self.search.cancellation.cancel();
        self.playback.cancellation.cancel();
        self.player = None;
    }
}

struct Shell {
    network: fuzzy_application::NetworkPolicy,
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
        let result = self.application.search_tracks(request, &options);
        let control = self.control.lock().map_err(|_| internal())?;
        if !control.search.current(generation) || control.closed {
            return Err(cancelled());
        }
        result
    }

    fn play(&self, generation: u64, request: ResolveTrackRequest) -> Result<(), AppError> {
        let options = {
            let mut control = self.control.lock().map_err(|_| internal())?;
            if control.closed {
                return Err(cancelled());
            }
            let options = control.playback.begin(generation)?;
            control.player = None;
            control.status = PlaybackStatus {
                generation,
                state: "loading",
                ..Default::default()
            };
            control.active += 1;
            options
        };
        let _active = Active(self);
        let resolved = self.application.resolve_track(request, &options);
        let mut control = self.control.lock().map_err(|_| internal())?;
        if !control.playback.current(generation) || control.closed {
            return Err(cancelled());
        }
        let result = resolved.and_then(|stream| {
            let player = playback::Player::start_with_network(
                &self.mpv,
                &stream.stream_url,
                Some(&self.network),
            )?;
            control.status.track_id = Some(stream.track.id);
            control.player = Some(player);
            Ok(())
        });
        if let Err(error) = &result {
            control.status.state = "error";
            control.status.error = Some(error.message.clone());
        }
        result
    }

    fn shutdown(&self) {
        let mut control = self.control.lock().unwrap_or_else(|e| e.into_inner());
        control.close();
        while control.active != 0 {
            control = self.idle.wait(control).unwrap_or_else(|e| e.into_inner());
        }
    }

    fn status(&self) -> Result<PlaybackStatus, AppError> {
        let mut control = self.control.lock().map_err(|_| internal())?;
        if let Some(player) = &mut control.player {
            let result = player.poll();
            match result {
                Ok(state) => {
                    control.status.state = state;
                    if state == "stopped" {
                        control.player = None;
                    }
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
async fn play_track(
    shell: tauri::State<'_, Arc<Shell>>,
    generation: u64,
    request: ResolveTrackRequest,
) -> Result<(), AppError> {
    let shell = shell.inner().clone();
    worker(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            shell.play(generation, request)
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
        shell
            .control
            .lock()
            .map_err(|_| internal())?
            .stop(generation)
    })
    .await
}

#[tauri::command]
async fn playback_status(shell: tauri::State<'_, Arc<Shell>>) -> Result<PlaybackStatus, AppError> {
    let shell = shell.inner().clone();
    worker(move || shell.status()).await
}

pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(Shell::new()))
        .invoke_handler(tauri::generate_handler![
            search_tracks,
            cancel_search,
            play_track,
            stop_playback,
            playback_status
        ])
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                // Prevent teardown until owned children have been killed and reaped.
                let shell = window.state::<Arc<Shell>>().inner().clone();
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                }
                let app = window.app_handle().clone();
                tauri::async_runtime::spawn_blocking(move || {
                    shell.shutdown();
                    app.exit(0);
                });
            }
        })
        .build(tauri::generate_context!())
        .expect("Could not initialize Fuzzy Music")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<Arc<Shell>>().shutdown();
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
        shell.play(3, request("unplayable")).unwrap();
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
