use command_group::{CommandGroup, GroupChild};
use fuzzy_application::{AppError, ErrorCode};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub fn failure() -> AppError {
    AppError {
        code: ErrorCode::PlaybackFailed,
        message: "Could not control audio. Check mpv, audio output, or try another track.".into(),
        retryable: true,
    }
}

/// Keep the last mpv session log next to the app data for field diagnostics.
fn log_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_STATE_HOME").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    let directory = base.join("io.github.kiruimaru.fuzzy-music");
    std::fs::create_dir_all(&directory).ok()?;
    let path = directory.join("mpv.log");
    if std::fs::metadata(&path).is_ok_and(|meta| meta.len() > 2 * 1024 * 1024) {
        let _ = std::fs::remove_file(&path);
    }
    Some(path)
}

pub struct Snapshot {
    pub state: &'static str,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub volume: f64,
    pub seekable: bool,
    pub advanced: bool,
}

/// One owned mpv process driven through `--input-ipc-server`.
///
/// On Windows the single synchronous pipe handle cannot be read and written
/// concurrently (a blocked read locks the pipe), so events are drained with
/// `PeekNamedPipe` before each status snapshot. Unix sockets support a reader
/// thread and are drained from its bounded channel.
pub struct Player {
    child: GroupChild,
    endpoint: PathBuf,
    #[cfg(windows)]
    pipe: std::fs::File,
    #[cfg(windows)]
    pending: Vec<u8>,
    #[cfg(not(windows))]
    input: std::os::unix::net::UnixStream,
    #[cfg(not(windows))]
    messages: std::sync::mpsc::Receiver<Value>,
    started: Instant,
    playing: bool,
    paused: bool,
    position_ms: u64,
    duration_ms: Option<u64>,
    volume: f64,
    seekable: bool,
    failed: bool,
    ended: bool,
    queued: bool,
    advanced: bool,
}

fn milliseconds(value: &Value) -> Option<u64> {
    let seconds = value.as_f64()?;
    (seconds.is_finite() && (0.0..=9_007_199_254_740.0).contains(&seconds))
        .then(|| (seconds * 1000.0).round() as u64)
}

impl Player {
    #[cfg(test)]
    pub fn start_with_network(
        executable: &Path,
        url: &str,
        network: Option<&fuzzy_application::NetworkPolicy>,
    ) -> Result<Self, AppError> {
        Self::start_with_volume(executable, url, network, 100.0)
    }

    pub fn start_with_volume(
        executable: &Path,
        url: &str,
        network: Option<&fuzzy_application::NetworkPolicy>,
        volume: f64,
    ) -> Result<Self, AppError> {
        let unique = format!(
            "fuzzy-mpv-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| failure())?
                .as_nanos()
        );
        #[cfg(windows)]
        let endpoint = PathBuf::from(format!(r"\\.\pipe\{unique}"));
        #[cfg(not(windows))]
        let endpoint = std::env::temp_dir().join(unique);

        let mut command = Command::new(executable);
        if let Some(network) = network {
            network.apply_with_loopback_bypass(&mut command);
        }
        command.arg(format!("--input-ipc-server={}", endpoint.display()));
        if let Some(log) = log_path() {
            command.arg(format!("--log-file={}", log.display()));
            command.arg("--msg-level=all=info");
        }
        command.arg(format!("--volume={volume}"));
        command.args([
            "--no-config",
            "--no-video",
            "--force-window=no",
            "--audio-display=no",
            "--terminal=no",
            "--input-terminal=no",
            "--input-default-bindings=no",
            "--ytdl=no",
            "--network-timeout=15",
            "--demuxer-max-bytes=16M",
            "--demuxer-max-back-bytes=4M",
            "--idle=yes",
            "--gapless-audio=yes",
            "--prefetch-playlist=yes",
            "--",
            url,
        ]);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut group = command.group();
        // command-group overwrites Command creation flags when spawning its job.
        #[cfg(windows)]
        group.creation_flags(0x08000000); // CREATE_NO_WINDOW
        let mut child = group.spawn().map_err(|_| dependency_failure())?;

        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            #[cfg(windows)]
            let connected = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&endpoint);
            #[cfg(not(windows))]
            let connected = std::os::unix::net::UnixStream::connect(&endpoint);
            match connected {
                Ok(connection) => {
                    let mut player = Self::connected(child, endpoint, connection)?;
                    player.volume = volume;
                    return Ok(player);
                }
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(error) => {
                    eprintln!("mpv: IPC connection failed ({:?})", error.kind());
                    let _ = child.kill();
                    let _ = child.wait();
                    #[cfg(not(windows))]
                    let _ = std::fs::remove_file(&endpoint);
                    return Err(failure());
                }
            }
        }
    }

    #[cfg(windows)]
    fn connected(
        child: GroupChild,
        endpoint: PathBuf,
        connection: std::fs::File,
    ) -> Result<Self, AppError> {
        let mut player = Self {
            child,
            endpoint,
            pipe: connection,
            pending: Vec::new(),
            started: Instant::now(),
            playing: false,
            paused: false,
            position_ms: 0,
            duration_ms: None,
            volume: 100.0,
            seekable: false,
            failed: false,
            ended: false,
            queued: false,
            advanced: false,
        };
        player.observe()?;
        Ok(player)
    }

    #[cfg(not(windows))]
    fn connected(
        child: GroupChild,
        endpoint: PathBuf,
        connection: std::os::unix::net::UnixStream,
    ) -> Result<Self, AppError> {
        let reader = connection.try_clone().map_err(|_| failure())?;
        let (sender, messages) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            for line in BufReader::new(reader).lines() {
                let Ok(line) = line else { break };
                // Diagnostics and malformed lines never cross the application boundary.
                if let Ok(value) = serde_json::from_str::<Value>(&line)
                    && sender.send(value).is_err()
                {
                    break;
                }
            }
        });
        let mut player = Self {
            child,
            endpoint,
            input: connection,
            messages,
            started: Instant::now(),
            playing: false,
            paused: false,
            position_ms: 0,
            duration_ms: None,
            volume: 100.0,
            seekable: false,
            failed: false,
            ended: false,
            queued: false,
            advanced: false,
        };
        player.observe()?;
        Ok(player)
    }

    fn observe(&mut self) -> Result<(), AppError> {
        for (id, property) in ["time-pos", "duration", "pause", "volume", "seekable"]
            .iter()
            .enumerate()
        {
            self.command(json!(["observe_property", id + 1, property]))?;
        }
        Ok(())
    }

    fn command(&mut self, command: Value) -> Result<(), AppError> {
        let mut bytes =
            serde_json::to_vec(&json!({ "command": command })).map_err(|_| failure())?;
        bytes.push(b'\n');
        #[cfg(windows)]
        let result = self.pipe.write_all(&bytes);
        #[cfg(not(windows))]
        let result = self.input.write_all(&bytes);
        result.map_err(|_| failure())
    }

    pub fn pause(&mut self, paused: bool) -> Result<(), AppError> {
        self.command(json!(["set_property", "pause", paused]))
    }

    pub fn seek(&mut self, position_ms: u64) -> Result<(), AppError> {
        let Some(duration) = self.duration_ms else {
            return Err(failure());
        };
        if !self.seekable {
            return Err(failure());
        }
        let position = position_ms.min(duration);
        self.command(json!(["seek", position as f64 / 1000.0, "absolute+exact"]))
    }

    pub fn volume(&mut self, volume: f64) -> Result<(), AppError> {
        self.command(json!(["set_property", "volume", volume]))
    }

    pub fn load(&mut self, url: &str) -> Result<(), AppError> {
        // Consume events from the old file before resetting readiness.
        self.drain();
        self.command(json!(["loadfile", url, "replace"]))?;
        self.command(json!(["set_property", "pause", false]))?;
        self.playing = false;
        self.paused = false;
        self.ended = false;
        self.queued = false;
        self.advanced = false;
        self.failed = false;
        self.position_ms = 0;
        self.duration_ms = None;
        self.seekable = false;
        self.started = Instant::now();
        Ok(())
    }

    /// Queue only while the current file is still playing. Otherwise the caller
    /// can fall back to the ordinary explicit play path.
    pub fn append(&mut self, url: &str) -> Result<bool, AppError> {
        self.drain();
        if self.ended || !self.playing || self.failed || self.queued {
            return Ok(false);
        }
        self.command(json!(["loadfile", url, "append"]))?;
        self.queued = true;
        Ok(true)
    }

    fn accept(&mut self, message: &Value) {
        if message["error"]
            .as_str()
            .is_some_and(|error| error != "success")
        {
            eprintln!("mpv: IPC command returned an error");
            self.failed = true;
        }
        match message["event"].as_str() {
            Some("start-file") => {
                if self.ended && self.queued {
                    self.queued = false;
                    self.advanced = true;
                }
                self.started = Instant::now();
                self.ended = false;
                self.playing = false;
                self.position_ms = 0;
                self.duration_ms = None;
                self.seekable = false;
            }
            Some("end-file") if message["reason"] == "error" => {
                eprintln!("mpv: media playback ended with an error");
                self.failed = true;
            }
            Some("end-file") if message["reason"] == "eof" => self.ended = true,
            Some("file-loaded") => self.playing = true,
            Some("property-change") => match message["name"].as_str() {
                Some("time-pos") => {
                    if let Some(position) = milliseconds(&message["data"]) {
                        self.position_ms = position;
                        self.playing = true;
                    }
                }
                Some("duration") => self.duration_ms = milliseconds(&message["data"]),
                Some("pause") => self.paused = message["data"].as_bool().unwrap_or(false),
                Some("volume") => {
                    if let Some(volume) = message["data"].as_f64() {
                        self.volume = volume.clamp(0.0, 100.0);
                    }
                }
                Some("seekable") => self.seekable = message["data"].as_bool().unwrap_or(false),
                _ => {}
            },
            _ => {}
        }
    }

    #[cfg(windows)]
    fn drain(&mut self) {
        use std::{io::Read, os::windows::io::AsRawHandle};
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn PeekNamedPipe(
                handle: *mut std::ffi::c_void,
                buffer: *mut std::ffi::c_void,
                size: u32,
                read: *mut u32,
                available: *mut u32,
                remaining: *mut u32,
            ) -> i32;
        }
        loop {
            let mut available = 0u32;
            let ok = unsafe {
                PeekNamedPipe(
                    self.pipe.as_raw_handle().cast(),
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut available,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 || available == 0 {
                return;
            }
            let mut chunk = vec![0u8; available.min(65_536) as usize];
            let read = match self.pipe.read(&mut chunk) {
                Ok(0) => return,
                Ok(read) => read,
                Err(_) => {
                    eprintln!("mpv: IPC pipe read failed");
                    self.failed = true;
                    return;
                }
            };
            self.pending.extend_from_slice(&chunk[..read]);
            while let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
                let line: Vec<u8> = self.pending.drain(..=end).collect();
                if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                    self.accept(&value);
                }
            }
        }
    }

    #[cfg(not(windows))]
    fn drain(&mut self) {
        while let Ok(message) = self.messages.try_recv() {
            self.accept(&message);
        }
    }

    pub fn poll(&mut self) -> Result<Snapshot, AppError> {
        self.drain();
        if self.failed {
            return Err(failure());
        }
        let state = if let Some(status) = self.child.try_wait().map_err(|_| failure())? {
            if status.success() && self.playing {
                "stopped"
            } else {
                return Err(failure());
            }
        } else if self.ended && self.queued {
            "loading"
        } else if self.ended {
            "stopped"
        } else if !self.playing && self.started.elapsed() > Duration::from_secs(30) {
            eprintln!("mpv: timed out waiting for playback");
            return Err(failure());
        } else if !self.playing {
            "loading"
        } else if self.paused {
            "paused"
        } else {
            "playing"
        };
        Ok(Snapshot {
            state,
            position_ms: self.position_ms,
            duration_ms: self.duration_ms,
            volume: self.volume,
            seekable: self.seekable,
            advanced: std::mem::take(&mut self.advanced),
        })
    }
}

fn dependency_failure() -> AppError {
    AppError {
        code: ErrorCode::PlaybackFailed,
        message: "mpv is missing or cannot start. Run setup-windows-runtime.ps1 beside the app on Windows, or repair FUZZY_MPV. See README.md.".into(),
        retryable: false,
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        #[cfg(not(windows))]
        let _ = std::fs::remove_file(&self.endpoint);
        #[cfg(windows)]
        let _ = &self.endpoint;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_values_are_safe() {
        assert_eq!(milliseconds(&json!(1.234)), Some(1234));
        assert_eq!(milliseconds(&json!(-1)), None);
        assert_eq!(milliseconds(&Value::Null), None);
        assert_eq!(milliseconds(&json!(1e30)), None);
    }

    #[test]
    #[ignore = "Requires FUZZY_MPV and FUZZY_TEST_AUDIO; may produce sound"]
    fn real_mpv_controls() {
        let executable = std::env::var_os("FUZZY_MPV").expect("FUZZY_MPV");
        let audio = std::env::var("FUZZY_TEST_AUDIO").expect("FUZZY_TEST_AUDIO");
        let mut player =
            Player::start_with_volume(Path::new(&executable), &audio, None, 35.0).unwrap();
        fn wait(player: &mut Player, check: impl Fn(&Snapshot) -> bool) -> Snapshot {
            let start = Instant::now();
            loop {
                let snapshot = player.poll().unwrap();
                if check(&snapshot) {
                    return snapshot;
                }
                assert!(
                    start.elapsed() < Duration::from_secs(5),
                    "state={} position={}",
                    snapshot.state,
                    snapshot.position_ms
                );
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        wait(&mut player, |s| {
            s.state == "playing" && s.seekable && s.duration_ms.is_some() && s.volume == 35.0
        });
        player.pause(true).unwrap();
        let paused = wait(&mut player, |s| s.state == "paused");
        std::thread::sleep(Duration::from_millis(300));
        assert!(
            player
                .poll()
                .unwrap()
                .position_ms
                .abs_diff(paused.position_ms)
                < 100
        );
        player.seek(5000).unwrap();
        wait(&mut player, |s| {
            s.state == "paused" && s.position_ms.abs_diff(5000) < 200
        });
        player.volume(25.0).unwrap();
        wait(&mut player, |s| (s.volume - 25.0).abs() < 0.1);
        player.pause(false).unwrap();
        wait(&mut player, |s| {
            s.state == "playing" && s.position_ms > 5200
        });
        let pid = player.child.inner().id();
        player.seek(11500).unwrap();
        wait(&mut player, |s| s.state == "stopped");
        let transition = Instant::now();
        player.load(&audio).unwrap();
        wait(&mut player, |s| {
            s.state == "playing" && s.position_ms < 3000
        });
        assert_eq!(player.child.inner().id(), pid);
        println!(
            "Local EOF -> same-process replay: {} ms",
            transition.elapsed().as_millis()
        );
        println!("Real mpv pause/seek/volume/resume passed with the same owned process {pid}");
    }
}

#[cfg(test)]
mod dependency_tests {
    use super::*;

    #[test]
    fn missing_player_reports_repair_without_private_path() {
        let error = Player::start_with_network(
            Path::new("nonexistent-private-directory/mpv.exe"),
            "https://example.com/audio",
            None,
        )
        .err()
        .expect("missing executable must fail");
        assert!(error.message.contains("setup-windows-runtime.ps1"));
        assert!(!error.message.contains("private-directory"));
        assert!(!error.retryable);
    }

    #[test]
    fn corrupt_player_reports_repair() {
        let path =
            std::env::temp_dir().join(format!("fuzzy-corrupt-mpv-{}.exe", std::process::id()));
        std::fs::write(&path, b"not an executable").unwrap();
        let error = Player::start_with_network(&path, "https://example.com/audio", None)
            .err()
            .expect("corrupt executable must fail");
        std::fs::remove_file(path).unwrap();
        assert!(error.message.contains("mpv is missing or cannot start"));
        assert!(!error.retryable);
    }
}
