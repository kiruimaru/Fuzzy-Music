use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::PathBuf, process::Command};

#[derive(Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub uploader: Option<String>,
    pub duration_seconds: Option<f64>,
    pub webpage_url: String,
    pub artwork_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResults {
    pub query: String,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Serialize)]
pub struct ResolvedStream {
    pub track: Track,
    pub url: String,
    pub protocol: Option<String>,
    pub format_id: Option<String>,
    pub audio_codec: Option<String>,
}

/// Console feasibility probe. All network/extractor work stays outside the UI.
pub struct SoundCloudClient {
    executable: PathBuf,
}

impl SoundCloudClient {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
        }
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<SearchResults, String> {
        let query = query.trim();
        if query.is_empty() {
            return Err("Search query must not be empty".into());
        }
        if !(1..=20).contains(&limit) {
            return Err("For this probe, limit must be between 1 and 20".into());
        }
        let target = format!("scsearch{limit}:{query}");
        let data = self.extract(&target, true)?;
        let entries = data
            .get("entries")
            .and_then(Value::as_array)
            .ok_or("Extractor returned an invalid search response (missing entries)")?;
        let tracks = entries
            .iter()
            .map(parse_track)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SearchResults {
            query: query.into(),
            tracks,
        })
    }

    pub fn resolve(&self, track_url: &str) -> Result<ResolvedStream, String> {
        if !is_soundcloud_url(track_url) {
            return Err("Expected an HTTPS SoundCloud track URL".into());
        }
        let data = self.extract(track_url, false)?;
        let url =
            optional_string(&data, "url").ok_or("This track has no available audio stream")?;
        if !url.starts_with("https://") {
            return Err("Extractor returned a non-HTTPS stream URL".into());
        }
        Ok(ResolvedStream {
            track: parse_track(&data)?,
            url,
            protocol: optional_string(&data, "protocol"),
            format_id: optional_string(&data, "format_id"),
            audio_codec: optional_string(&data, "acodec"),
        })
    }

    fn extract(&self, target: &str, flat: bool) -> Result<Value, String> {
        let mut command = Command::new(&self.executable);
        command.args([
            "--ignore-config",
            "--no-plugin-dirs",
            "--no-cache-dir",
            "--skip-download",
            "--dump-single-json",
            "--no-progress",
            "--socket-timeout",
            "15",
            "--retries",
            "1",
            "--extractor-retries",
            "1",
        ]);
        if flat {
            command.arg("--flat-playlist");
        } else {
            command.args(["--no-playlist", "--format", "bestaudio"]);
        }
        // No shell: queries and URLs are passed as a single argument after --.
        let output = command.arg("--").arg(target).output().map_err(|e| {
            format!(
                "Could not start yt-dlp at {}: {e}",
                self.executable.display()
            )
        })?;
        if !output.status.success() {
            return Err(format!(
                "SoundCloud extractor failed ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        if !output.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr).trim());
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Extractor returned invalid JSON: {e}"))
    }
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn is_soundcloud_url(url: &str) -> bool {
    [
        "https://soundcloud.com/",
        "https://www.soundcloud.com/",
        "https://on.soundcloud.com/",
    ]
    .iter()
    .any(|prefix| url.starts_with(prefix))
        && !url.chars().any(char::is_whitespace)
}

fn parse_track(value: &Value) -> Result<Track, String> {
    let required =
        |key| optional_string(value, key).ok_or_else(|| format!("Track metadata is missing {key}"));
    let webpage_url = required("webpage_url")?;
    if !is_soundcloud_url(&webpage_url) {
        return Err("Track metadata contains an unexpected source URL".into());
    }
    Ok(Track {
        id: required("id")?,
        title: required("title")?,
        uploader: optional_string(value, "uploader"),
        duration_seconds: value.get("duration").and_then(Value::as_f64),
        webpage_url,
        artwork_url: optional_string(value, "thumbnail").or_else(|| {
            value
                .get("thumbnails")
                .and_then(Value::as_array)
                .and_then(|items| {
                    items
                        .iter()
                        .rev()
                        .find_map(|item| optional_string(item, "url"))
                })
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_optional_metadata_does_not_break_search_results() {
        let value = serde_json::json!({
            "id": "123", "title": "Example", "webpage_url": "https://soundcloud.com/author/track",
            "duration": null, "uploader": null, "thumbnail": null
        });
        let track = parse_track(&value).unwrap();
        assert!(track.duration_seconds.is_none());
        assert!(track.uploader.is_none());
        assert!(track.artwork_url.is_none());
    }

    #[test]
    fn flat_search_thumbnail_list_is_supported() {
        let value = serde_json::json!({
            "id": "123", "title": "Example", "webpage_url": "https://soundcloud.com/author/track",
            "thumbnails": [{"url": "https://i1.sndcdn.com/small.jpg"}, {"url": "https://i1.sndcdn.com/original.jpg"}]
        });
        assert_eq!(
            parse_track(&value).unwrap().artwork_url.as_deref(),
            Some("https://i1.sndcdn.com/original.jpg")
        );
    }

    #[test]
    fn source_url_validation_rejects_lookalike_domains_and_non_https() {
        for url in [
            "https://soundcloud.com.evil.example/track",
            "http://soundcloud.com/a/b",
            "https://soundcloud.com@evil.example/track",
        ] {
            assert!(!is_soundcloud_url(url));
        }
        assert!(is_soundcloud_url("https://soundcloud.com/a/b"));
    }

    #[test]
    fn invalid_input_is_rejected_before_starting_extractor() {
        let client = SoundCloudClient::new("does-not-exist");
        assert!(client.search("  ", 5).unwrap_err().contains("empty"));
        assert!(client.search("music", 0).unwrap_err().contains("limit"));
        assert!(
            client
                .resolve("https://example.com/")
                .unwrap_err()
                .contains("SoundCloud")
        );
    }
}
