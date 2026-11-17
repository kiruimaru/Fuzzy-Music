use command_group::{CommandGroup, GroupChild};
use fuzzy_application::{AppError, ErrorCode};
use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

pub fn failure() -> AppError {
    AppError {
        code: ErrorCode::PlaybackFailed,
        message: "Could not play audio. Check mpv, audio output, or try another track.".into(),
        retryable: true,
    }
}

pub struct Player {
    child: GroupChild,
    ready: Receiver<()>,
    started: Instant,
    playing: bool,
}

impl Player {
    pub fn start(executable: &Path, url: &str) -> Result<Self, AppError> {
        let mut command = Command::new(executable);
        command.args([
            "--no-config",
            "--no-video",
            "--force-window=no",
            "--audio-display=no",
            "--terminal=yes",
            "--input-terminal=no",
            "--input-default-bindings=no",
            "--ytdl=no",
            "--network-timeout=15",
            "--quiet",
            "--term-playing-msg=FUZZY_PLAYING",
            "--",
            url,
        ]);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut group = command.group();
        // command-group overwrites Command creation flags when spawning its job.
        #[cfg(windows)]
        group.creation_flags(0x08000000); // CREATE_NO_WINDOW
        let mut child = group.spawn().map_err(|_| failure())?;
        let stdout = child.inner().stdout.take().expect("piped stdout");
        let (sender, ready) = mpsc::channel();
        std::thread::spawn(move || {
            // Only a fixed marker crosses this pipe; never forward mpv diagnostics.
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) if line.trim() == "FUZZY_PLAYING" => {
                        let _ = sender.send(());
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            child,
            ready,
            started: Instant::now(),
            playing: false,
        })
    }

    pub fn poll(&mut self) -> Result<&'static str, AppError> {
        if self.ready.try_recv().is_ok() {
            self.playing = true;
        }
        if let Some(status) = self.child.try_wait().map_err(|_| failure())? {
            return if status.success() && self.playing {
                Ok("stopped")
            } else {
                Err(failure())
            };
        }
        if !self.playing && self.started.elapsed() > Duration::from_secs(30) {
            return Err(failure());
        }
        Ok(if self.playing { "playing" } else { "loading" })
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
