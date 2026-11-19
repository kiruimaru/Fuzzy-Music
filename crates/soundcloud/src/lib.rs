use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::PathBuf, process::Command};

mod control;
mod download;
mod error;
pub mod network;
mod process;
mod web;
pub use control::{CancellationToken, RequestOptions};
pub use download::DownloadQuality;
pub use error::SoundCloudError;

#[derive(Debug, Serialize)]
pub struct Playlist {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub artwork_url: Option<String>,
    pub track_count: Option<u32>,
    pub duration_seconds: Option<f64>,
    pub webpage_url: String,
}

#[derive(Debug, Serialize)]
pub struct PlaylistSearchResults {
    pub query: String,
    pub playlists: Vec<Playlist>,
}

#[derive(Debug, Serialize)]
pub struct PlaylistTracks {
    pub playlist: Playlist,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Serialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub track_count: Option<u32>,
    pub webpage_url: String,
}

#[derive(Debug, Serialize)]
pub struct ArtistTracks {
    pub artist: Artist,
    pub tracks: Vec<Track>,
    pub albums: Vec<Playlist>,
    pub playlists: Vec<Playlist>,
}

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
    network: Option<network::NetworkPolicy>,
}

impl SoundCloudClient {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            network: None,
        }
    }

    pub fn with_network(mut self, network: network::NetworkPolicy) -> Self {
        self.network = Some(network);
        self
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<SearchResults, SoundCloudError> {
        self.search_with_options(query, limit, &RequestOptions::default())
    }

    pub fn search_with_options(
        &self,
        query: &str,
        limit: usize,
        options: &RequestOptions,
    ) -> Result<SearchResults, SoundCloudError> {
        let query = query.trim();
        if query.is_empty() {
            return Err(SoundCloudError::InvalidInput(
                "Search query must not be empty".into(),
            ));
        }
        // `yt-dlp` follows `scsearch` pagination, so larger pages are allowed.
        if !(1..=100).contains(&limit) {
            return Err(SoundCloudError::InvalidInput(
                "Track limit must be between 1 and 100".into(),
            ));
        }
        let target = format!("scsearch{limit}:{query}");
        let data = self.extract(&target, true, options)?;
        let entries = data
            .get("entries")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                SoundCloudError::InvalidResponse(
                    "Extractor returned an invalid search response (missing entries)".into(),
                )
            })?;
        let tracks = entries
            .iter()
            .map(parse_track)
            .collect::<Result<Vec<_>, _>>()
            .map_err(SoundCloudError::InvalidResponse)?;
        Ok(SearchResults {
            query: query.into(),
            tracks,
        })
    }

    pub fn resolve(&self, track_url: &str) -> Result<ResolvedStream, SoundCloudError> {
        self.resolve_with_options(track_url, &RequestOptions::default())
    }

    pub fn search_playlists(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<PlaylistSearchResults, SoundCloudError> {
        self.search_playlists_with_options(query, limit, &RequestOptions::default())
    }

    pub fn search_playlists_with_options(
        &self,
        query: &str,
        limit: usize,
        options: &RequestOptions,
    ) -> Result<PlaylistSearchResults, SoundCloudError> {
        let query = query.trim();
        if query.is_empty() {
            return Err(SoundCloudError::InvalidInput(
                "Search query must not be empty".into(),
            ));
        }
        if !(1..=50).contains(&limit) {
            return Err(SoundCloudError::InvalidInput(
                "Playlist limit must be between 1 and 50".into(),
            ));
        }
        Ok(PlaylistSearchResults {
            query: query.into(),
            playlists: web::search(options, query, limit, self.network.as_ref())?,
        })
    }

    pub fn playlist_tracks(&self, playlist_url: &str) -> Result<PlaylistTracks, SoundCloudError> {
        self.playlist_tracks_with_options(playlist_url, &RequestOptions::default())
    }

    pub fn playlist_tracks_with_options(
        &self,
        playlist_url: &str,
        options: &RequestOptions,
    ) -> Result<PlaylistTracks, SoundCloudError> {
        if !is_soundcloud_url(playlist_url) || !playlist_url.contains("/sets/") {
            return Err(SoundCloudError::InvalidInput(
                "Expected an HTTPS SoundCloud playlist URL".into(),
            ));
        }
        let (playlist, tracks) = web::playlist(options, playlist_url, self.network.as_ref())?;
        Ok(PlaylistTracks { playlist, tracks })
    }

    pub fn artist_tracks(&self, artist_url: &str) -> Result<ArtistTracks, SoundCloudError> {
        self.artist_tracks_with_options(artist_url, &RequestOptions::default())
    }

    pub fn artist_tracks_with_options(
        &self,
        artist_url: &str,
        options: &RequestOptions,
    ) -> Result<ArtistTracks, SoundCloudError> {
        if !is_artist_url(artist_url) {
            return Err(SoundCloudError::InvalidInput(
                "Expected an HTTPS SoundCloud artist URL".into(),
            ));
        }
        web::artist(options, artist_url, self.network.as_ref())
    }

    pub fn resolve_with_options(
        &self,
        track_url: &str,
        options: &RequestOptions,
    ) -> Result<ResolvedStream, SoundCloudError> {
        if !is_soundcloud_url(track_url) {
            return Err(SoundCloudError::InvalidInput(
                "Expected an HTTPS SoundCloud track URL".into(),
            ));
        }
        let data = self.extract(track_url, false, options)?;
        let url = optional_string(&data, "url").ok_or_else(|| {
            SoundCloudError::InvalidResponse("This track has no available audio stream".into())
        })?;
        if !url.starts_with("https://") {
            return Err(SoundCloudError::InvalidResponse(
                "Extractor returned a non-HTTPS stream URL".into(),
            ));
        }
        Ok(ResolvedStream {
            track: parse_track(&data).map_err(SoundCloudError::InvalidResponse)?,
            url,
            protocol: optional_string(&data, "protocol"),
            format_id: optional_string(&data, "format_id"),
            audio_codec: optional_string(&data, "acodec"),
        })
    }

    fn extract(
        &self,
        target: &str,
        flat: bool,
        options: &RequestOptions,
    ) -> Result<Value, SoundCloudError> {
        let mut command = Command::new(&self.executable);
        if let Some(network) = &self.network {
            network.apply(&mut command);
        }
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
        let output = process::run(command.arg("--").arg(target), options)?;
        if !output.status.success() {
            return Err(SoundCloudError::ExtractorFailed {
                status: output.status.to_string(),
                details: String::from_utf8_lossy(&output.stderr).trim().into(),
            });
        }
        if self.network.is_none() && !output.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr).trim());
        }
        serde_json::from_slice(&output.stdout).map_err(|e| {
            SoundCloudError::InvalidResponse(format!("Extractor returned invalid JSON: {e}"))
        })
    }

    /// Fast metadata-only search using the same web transport as playlists.
    /// The extractor remains available for probes and audio resolution.
    pub fn search_web_with_options(
        &self,
        query: &str,
        limit: usize,
        options: &RequestOptions,
    ) -> Result<SearchResults, SoundCloudError> {
        let query = query.trim();
        if query.is_empty() || !(1..=100).contains(&limit) {
            return Err(SoundCloudError::InvalidInput(
                "Expected a query and limit 1..100".into(),
            ));
        }
        Ok(SearchResults {
            query: query.into(),
            tracks: web::search_tracks(options, query, limit, self.network.as_ref())?,
        })
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

fn is_artist_url(url: &str) -> bool {
    is_soundcloud_url(url)
        && artist_path(url).is_some_and(|path| {
            let path = path.trim_end_matches('/');
            !path.is_empty()
                && !path.contains('/')
                && !path.contains(['?', '#'])
                && !url.ends_with("//")
        })
}

/// A SoundCloud artist page has exactly one path segment after the domain.
fn artist_path(url: &str) -> Option<&str> {
    ["https://soundcloud.com/", "https://www.soundcloud.com/"]
        .iter()
        .find_map(|prefix| url.strip_prefix(prefix))
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
        assert!(matches!(
            client.search("  ", 5),
            Err(SoundCloudError::InvalidInput(_))
        ));
        assert!(matches!(
            client.search("music", 0),
            Err(SoundCloudError::InvalidInput(_))
        ));
        assert!(
            client
                .resolve("https://example.com/")
                .unwrap_err()
                .to_string()
                .contains("SoundCloud")
        );
    }

    #[test]
    fn artist_urls_are_validated_before_network_access() {
        let client = SoundCloudClient::new("does-not-exist");
        for url in [
            "https://soundcloud.com/artist/track",
            "https://soundcloud.com/artist/sets/mix",
            "https://soundcloud.com/",
            "https://soundcloud.com//",
            "https://soundcloud.com/artist?tab=tracks",
            "http://soundcloud.com/artist",
            "https://soundcloud.com.evil.test/artist",
            "https://example.com/artist",
        ] {
            assert!(
                matches!(
                    client.artist_tracks(url),
                    Err(SoundCloudError::InvalidInput(_))
                ),
                "{url} must be rejected"
            );
        }
        assert!(is_artist_url("https://soundcloud.com/artist"));
        assert!(is_artist_url("https://soundcloud.com/artist/"));
        assert!(is_artist_url("https://www.soundcloud.com/artist"));
    }
}
