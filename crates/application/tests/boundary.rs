use fuzzy_application::{
    Application, ErrorCode, RequestOptions, ResolveTrackRequest, SearchTracksRequest, Source,
};
use fuzzy_soundcloud::SoundCloudClient;
use std::{path::PathBuf, process::Command, time::Duration};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn missing_and_corrupt_extractor_have_actionable_safe_errors() {
    let directory = std::env::temp_dir().join(format!("fuzzy-api-damaged-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let fixture = Fixture(directory);
    let path = fixture.0.join("yt-dlp.exe");
    for corrupt in [false, true] {
        if corrupt {
            std::fs::write(&path, b"not an executable").unwrap();
        }
        let app = Application::new(SoundCloudClient::new(&path));
        let error = app
            .search_tracks(
                SearchTracksRequest {
                    query: "music".into(),
                    limit: None,
                },
                &RequestOptions::default(),
            )
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::SourceUnavailable);
        assert!(error.message.contains("setup-windows-runtime.ps1"));
        assert!(!error.message.contains("fuzzy-api-damaged"));
        assert!(!error.retryable);
    }
}

#[test]
fn extractor_to_application_offline() {
    let directory = std::env::temp_dir().join(format!(
        "fuzzy-api-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let fixture = Fixture(directory);
    let executable = fixture
        .0
        .join(format!("extractor{}", std::env::consts::EXE_SUFFIX));
    let output = Command::new("rustc")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/extractor.rs"
        ))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let app = Application::new(SoundCloudClient::new(executable));
    let search = |query: &str| SearchTracksRequest {
        query: query.into(),
        limit: None,
    };
    let response = app
        .search_tracks(search("music"), &RequestOptions::default())
        .unwrap();
    let track = response.tracks.into_iter().next().unwrap();
    assert_eq!(track.author, "Unknown artist");
    assert_eq!(track.duration_ms, Some(1235));
    assert!(track.stream_url.is_none());
    let response = app
        .resolve_track(
            ResolveTrackRequest {
                source: Source::Soundcloud,
                track_url: track.source_url,
            },
            &RequestOptions::default(),
        )
        .unwrap();
    assert_eq!(response.track.id, track.id);
    assert_eq!(response.stream_url, "https://audio.example/temporary");
    let error = app
        .search_tracks(search("fail"), &RequestOptions::default())
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::SourceUnavailable);
    assert!(!serde_json::to_string(&error).unwrap().contains("SECRET"));
    let options = RequestOptions {
        timeout: Duration::from_millis(100),
        ..Default::default()
    };
    assert_eq!(
        app.search_tracks(search("hang"), &options)
            .unwrap_err()
            .code,
        ErrorCode::Timeout
    );
    let options = RequestOptions::default();
    let cancellation = options.cancellation.clone();
    let worker = std::thread::spawn(move || app.search_tracks(search("hang"), &options));
    std::thread::sleep(Duration::from_millis(100));
    cancellation.cancel();
    assert_eq!(
        worker.join().unwrap().unwrap_err().code,
        ErrorCode::Cancelled
    );
}
