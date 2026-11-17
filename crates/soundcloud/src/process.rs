use crate::{RequestOptions, SoundCloudError};
use command_group::{CommandGroup, GroupChild};
use std::{
    io::{self, Read},
    process::{Command, Output, Stdio},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};

const POLL_INTERVAL: Duration = Duration::from_millis(20);
const STDOUT_LIMIT: u64 = 8 * 1024 * 1024;
const STDERR_LIMIT: u64 = 1024 * 1024;

// Kill the whole process group/job, including the Windows yt-dlp bootloader child.
// Always reap it, including early returns after IO errors or cancellation.
struct ProcessGroup(GroupChild);
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn drain(reader: impl Read + Send + 'static, limit: u64) -> Receiver<io::Result<Vec<u8>>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = (|| {
            let mut bytes = Vec::new();
            reader.take(limit + 1).read_to_end(&mut bytes)?;
            if bytes.len() as u64 > limit {
                return Err(io::Error::other("Extractor output exceeded the size limit"));
            }
            Ok(bytes)
        })();
        let _ = sender.send(result);
    });
    receiver
}

fn receive(
    receiver: &Receiver<io::Result<Vec<u8>>>,
    output: &mut Option<Vec<u8>>,
) -> Result<(), SoundCloudError> {
    if output.is_none() {
        match receiver.try_recv() {
            Ok(result) => {
                *output = Some(result.map_err(|e| SoundCloudError::Process(e.to_string()))?)
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                return Err(SoundCloudError::Process(
                    "Extractor output reader stopped unexpectedly".into(),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn run(
    command: &mut Command,
    options: &RequestOptions,
) -> Result<Output, SoundCloudError> {
    let start = Instant::now();
    if options.cancellation.is_cancelled() {
        return Err(SoundCloudError::Cancelled);
    }
    if options.timeout.is_zero() {
        return Err(SoundCloudError::InvalidInput(
            "Request timeout must be positive".into(),
        ));
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut group = command.group();
    // command-group overwrites Command creation flags when spawning its job.
    #[cfg(windows)]
    group.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let mut group = ProcessGroup(
        group
            .spawn()
            .map_err(|e| SoundCloudError::Process(format!("Could not start extractor: {e}")))?,
    );
    let stdout = drain(
        group
            .0
            .inner()
            .stdout
            .take()
            .ok_or_else(|| SoundCloudError::Process("Missing stdout pipe".into()))?,
        STDOUT_LIMIT,
    );
    let stderr = drain(
        group
            .0
            .inner()
            .stderr
            .take()
            .ok_or_else(|| SoundCloudError::Process("Missing stderr pipe".into()))?,
        STDERR_LIMIT,
    );
    let (mut out, mut err, mut status) = (None, None, None);
    loop {
        if options.cancellation.is_cancelled() {
            return Err(SoundCloudError::Cancelled);
        }
        if start.elapsed() >= options.timeout {
            return Err(SoundCloudError::TimedOut {
                timeout: options.timeout,
            });
        }
        receive(&stdout, &mut out)?;
        receive(&stderr, &mut err)?;
        if status.is_none() {
            status = group
                .0
                .try_wait()
                .map_err(|e| SoundCloudError::Process(e.to_string()))?;
        }
        if let Some(status) = status
            && out.is_some()
            && err.is_some()
        {
            return Ok(Output {
                status,
                stdout: out.take().unwrap(),
                stderr: err.take().unwrap(),
            });
        }
        thread::sleep(POLL_INTERVAL.min(options.timeout.saturating_sub(start.elapsed())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CancellationToken;
    use std::{
        fs,
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn fixture(mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--ignored",
                "--exact",
                "process::tests::child_fixture",
                "--nocapture",
            ])
            .env("FUZZY_PROCESS_FIXTURE", mode);
        command
    }

    // Executed only as a child of the tests below, never during the normal suite.
    #[test]
    #[ignore]
    fn child_fixture() {
        match std::env::var("FUZZY_PROCESS_FIXTURE").unwrap().as_str() {
            "noisy" => {
                let writer =
                    thread::spawn(|| io::stderr().write_all(&vec![b'e'; 256 * 1024]).unwrap());
                io::stdout().write_all(&vec![b'o'; 256 * 1024]).unwrap();
                writer.join().unwrap();
            }
            "descendant" => {
                let mut child = fixture("heartbeat").spawn().unwrap();
                child.wait().unwrap();
            }
            "heartbeat" => {
                let path = std::env::var_os("FUZZY_PROCESS_MARKER").unwrap();
                for _ in 0..500 {
                    fs::write(&path, format!("{:?}", SystemTime::now())).unwrap();
                    thread::sleep(Duration::from_millis(20));
                }
            }
            _ => thread::sleep(Duration::from_secs(10)),
        }
    }

    #[test]
    fn timeout_stops_a_hung_process() {
        let options = RequestOptions {
            timeout: Duration::from_millis(250),
            ..Default::default()
        };
        let start = Instant::now();
        assert!(matches!(
            run(&mut fixture("hang"), &options),
            Err(SoundCloudError::TimedOut { .. })
        ));
        assert!(start.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn pre_cancelled_request_does_not_start_a_process() {
        let options = RequestOptions::default();
        options.cancellation.cancel();
        assert!(matches!(
            run(&mut Command::new("missing-extractor"), &options),
            Err(SoundCloudError::Cancelled)
        ));
    }

    #[test]
    fn stdout_and_stderr_are_drained_concurrently() {
        let options = RequestOptions {
            timeout: Duration::from_secs(5),
            ..Default::default()
        };
        let output = run(&mut fixture("noisy"), &options).unwrap();
        assert!(output.status.success());
        assert!(output.stdout.len() >= 256 * 1024);
        assert!(output.stderr.len() >= 256 * 1024);
    }

    #[test]
    fn cancellation_stops_the_process_and_its_descendant() {
        let marker = std::env::temp_dir().join(format!(
            "fuzzy-process-{}-{}.txt",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cancellation = CancellationToken::default();
        let options = RequestOptions {
            timeout: Duration::from_secs(5),
            cancellation: cancellation.clone(),
        };
        let mut command = fixture("descendant");
        command.env("FUZZY_PROCESS_MARKER", &marker);
        let worker = thread::spawn(move || run(&mut command, &options));
        let start = Instant::now();
        while !marker.exists() && start.elapsed() < Duration::from_secs(3) {
            thread::sleep(Duration::from_millis(10));
        }
        let started = marker.exists();
        cancellation.cancel();
        let result = worker.join().unwrap();
        assert!(started, "Fixture descendant must start before cancellation");
        assert!(matches!(result, Err(SoundCloudError::Cancelled)));
        let stopped = fs::read(&marker).unwrap();
        thread::sleep(Duration::from_millis(150));
        assert_eq!(
            stopped,
            fs::read(&marker).unwrap(),
            "Descendant must stop writing after cancellation"
        );
        fs::remove_file(marker).unwrap();
    }
}
