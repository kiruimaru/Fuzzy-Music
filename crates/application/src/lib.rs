//! Synchronous application boundary. Run operations on a blocking worker, not the UI thread.
pub use fuzzy_soundcloud::network::NetworkPolicy;
pub use fuzzy_soundcloud::{CancellationToken, RequestOptions};
use fuzzy_soundcloud::{SoundCloudClient, SoundCloudError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub title: String,
    pub author: String,
    pub artwork_url: Option<String>,
    pub duration_ms: Option<u64>,
    pub stream_url: Option<String>,
    pub source: Source,
    pub source_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Soundcloud,
    Local,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchTracksRequest {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub title: String,
    pub author: String,
    pub artwork_url: Option<String>,
    pub track_count: Option<u32>,
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub is_album: bool,
    pub source: Source,
    pub source_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPlaylistsRequest {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPlaylistsResponse {
    pub playlists: Vec<Playlist>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTracksRequest {
    pub source: Source,
    pub playlist_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTracksResponse {
    pub playlist: Playlist,
    pub tracks: Vec<Track>,
    pub skipped_track_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub track_count: Option<u32>,
    pub source_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistTracksRequest {
    pub source: Source,
    pub artist_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistTracksResponse {
    pub artist: Artist,
    pub tracks: Vec<Track>,
    pub albums: Vec<Playlist>,
    pub playlists: Vec<Playlist>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedTracksRequest {
    pub source: Source,
    pub track_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedTracksResponse {
    pub tracks: Vec<Track>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchTracksResponse {
    pub tracks: Vec<Track>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveTrackRequest {
    pub source: Source,
    pub track_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveTrackResponse {
    pub track: Track,
    pub stream_url: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidInput,
    NotFound,
    SourceUnavailable,
    Timeout,
    Cancelled,
    PlaybackFailed,
    StorageFailed,
    Internal,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
}

impl From<SoundCloudError> for AppError {
    fn from(error: SoundCloudError) -> Self {
        let (code, message, retryable) = match error {
            SoundCloudError::InvalidInput(_) => {
                (ErrorCode::InvalidInput, "Invalid request.", false)
            }
            SoundCloudError::Cancelled => (ErrorCode::Cancelled, "Request cancelled.", false),
            SoundCloudError::TimedOut { .. } => {
                (ErrorCode::Timeout, "Source request timed out.", true)
            }
            SoundCloudError::Process(_) => (
                ErrorCode::SourceUnavailable,
                "yt-dlp is missing or cannot start. Run setup-windows-runtime.ps1 beside the app on Windows, or repair FUZZY_YTDLP. See README.md.",
                false,
            ),
            SoundCloudError::ExtractorFailed { .. } | SoundCloudError::InvalidResponse(_) => {
                (ErrorCode::SourceUnavailable, "Source request failed.", true)
            }
            SoundCloudError::Network(_) => (
                ErrorCode::SourceUnavailable,
                "Playlist search failed. Check the connection and retry.",
                true,
            ),
        };
        Self {
            code,
            message: message.into(),
            retryable,
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for AppError {}

impl From<fuzzy_soundcloud::Track> for Track {
    fn from(track: fuzzy_soundcloud::Track) -> Self {
        // Bound integers to JavaScript's exact range before casting (casts saturate).
        let duration_ms = track.duration_seconds.and_then(|seconds| {
            let millis = (seconds * 1000.0).round();
            (seconds.is_finite() && seconds >= 0.0 && millis <= 9_007_199_254_740_991.0)
                .then_some(millis as u64)
        });
        Self {
            id: track.id,
            title: track.title,
            author: track
                .uploader
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "Unknown artist".into()),
            artwork_url: track.artwork_url,
            duration_ms,
            stream_url: None,
            source: Source::Soundcloud,
            source_url: track.webpage_url,
        }
    }
}

impl From<fuzzy_soundcloud::Playlist> for Playlist {
    fn from(playlist: fuzzy_soundcloud::Playlist) -> Self {
        let duration_ms = playlist.duration_seconds.and_then(|seconds| {
            let millis = (seconds * 1000.0).round();
            (seconds.is_finite() && seconds >= 0.0 && millis <= 9_007_199_254_740_991.0)
                .then_some(millis as u64)
        });
        Self {
            id: playlist.id,
            title: playlist.title,
            author: playlist
                .author
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "Unknown artist".into()),
            artwork_url: playlist.artwork_url,
            track_count: playlist.track_count,
            duration_ms,
            is_album: playlist.is_album,
            source: Source::Soundcloud,
            source_url: playlist.webpage_url,
        }
    }
}

impl From<fuzzy_soundcloud::Artist> for Artist {
    fn from(artist: fuzzy_soundcloud::Artist) -> Self {
        Self {
            id: artist.id,
            name: artist.name,
            avatar_url: artist.avatar_url,
            track_count: artist.track_count,
            source_url: artist.webpage_url,
        }
    }
}

pub struct Application {
    soundcloud: SoundCloudClient,
}

impl Application {
    pub fn search_tracks_fast(
        &self,
        request: SearchTracksRequest,
        options: &RequestOptions,
    ) -> Result<SearchTracksResponse, AppError> {
        let results = self.soundcloud.search_web_with_options(
            &request.query,
            request.limit.unwrap_or(30),
            options,
        )?;
        Ok(SearchTracksResponse {
            tracks: results.tracks.into_iter().map(Track::from).collect(),
            next_page_token: None,
        })
    }
    pub fn new(soundcloud: SoundCloudClient) -> Self {
        Self { soundcloud }
    }

    pub fn search_tracks(
        &self,
        request: SearchTracksRequest,
        options: &RequestOptions,
    ) -> Result<SearchTracksResponse, AppError> {
        let results = self.soundcloud.search_with_options(
            &request.query,
            request.limit.unwrap_or(10),
            options,
        )?;
        Ok(SearchTracksResponse {
            tracks: results.tracks.into_iter().map(Track::from).collect(),
            next_page_token: None,
        })
    }

    pub fn resolve_track(
        &self,
        request: ResolveTrackRequest,
        options: &RequestOptions,
    ) -> Result<ResolveTrackResponse, AppError> {
        let stream = match request.source {
            Source::Soundcloud => self
                .soundcloud
                .resolve_with_options(&request.track_url, options)?,
            Source::Local => {
                return Err(AppError {
                    code: ErrorCode::InvalidInput,
                    message: "Local tracks must be opened from the music library.".into(),
                    retryable: false,
                });
            }
        };
        Ok(ResolveTrackResponse {
            track: stream.track.into(),
            stream_url: stream.url,
        })
    }

    pub fn search_playlists(
        &self,
        request: SearchPlaylistsRequest,
        options: &RequestOptions,
    ) -> Result<SearchPlaylistsResponse, AppError> {
        let results = self.soundcloud.search_playlists_with_options(
            &request.query,
            request.limit.unwrap_or(10),
            options,
        )?;
        Ok(SearchPlaylistsResponse {
            playlists: results.playlists.into_iter().map(Playlist::from).collect(),
            next_page_token: None,
        })
    }

    pub fn playlist_tracks(
        &self,
        request: PlaylistTracksRequest,
        options: &RequestOptions,
    ) -> Result<PlaylistTracksResponse, AppError> {
        let result = match request.source {
            Source::Soundcloud => self
                .soundcloud
                .playlist_tracks_with_options(&request.playlist_url, options)?,
            Source::Local => {
                return Err(AppError {
                    code: ErrorCode::InvalidInput,
                    message: "Local source playlists are not supported.".into(),
                    retryable: false,
                });
            }
        };
        let skipped_track_count = result
            .playlist
            .track_count
            .unwrap_or(result.tracks.len() as u32)
            .saturating_sub(result.tracks.len() as u32);
        Ok(PlaylistTracksResponse {
            playlist: result.playlist.into(),
            tracks: result.tracks.into_iter().map(Track::from).collect(),
            skipped_track_count,
        })
    }

    pub fn artist_tracks(
        &self,
        request: ArtistTracksRequest,
        options: &RequestOptions,
    ) -> Result<ArtistTracksResponse, AppError> {
        let result = match request.source {
            Source::Soundcloud => self
                .soundcloud
                .artist_tracks_with_options(&request.artist_url, options)?,
            Source::Local => {
                return Err(AppError {
                    code: ErrorCode::InvalidInput,
                    message: "Local source artists are not supported.".into(),
                    retryable: false,
                });
            }
        };
        Ok(ArtistTracksResponse {
            artist: result.artist.into(),
            tracks: result.tracks.into_iter().map(Track::from).collect(),
            albums: result.albums.into_iter().map(Playlist::from).collect(),
            playlists: result.playlists.into_iter().map(Playlist::from).collect(),
        })
    }

    pub fn related_tracks(
        &self,
        request: RelatedTracksRequest,
        options: &RequestOptions,
    ) -> Result<RelatedTracksResponse, AppError> {
        let tracks = match request.source {
            Source::Soundcloud => {
                self.soundcloud
                    .related_tracks_with_options(&request.track_id, 25, options)?
            }
            Source::Local => {
                return Err(AppError {
                    code: ErrorCode::InvalidInput,
                    message: "Radio needs a SoundCloud track.".into(),
                    retryable: false,
                });
            }
        };
        Ok(RelatedTracksResponse {
            tracks: tracks.into_iter().map(Track::from).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    fn track(duration_seconds: Option<f64>) -> fuzzy_soundcloud::Track {
        fuzzy_soundcloud::Track {
            id: "123".into(),
            title: "Example".into(),
            uploader: None,
            duration_seconds,
            webpage_url: "https://soundcloud.com/a/b".into(),
            artwork_url: None,
        }
    }

    #[test]
    fn metadata_and_wire_shape() {
        let dto = Track::from(track(Some(1.2346)));
        let value = serde_json::to_value(&dto).unwrap();
        assert_eq!(
            value,
            json!({"id":"123","title":"Example","author":"Unknown artist","artworkUrl":null,"durationMs":1235,"streamUrl":null,"source":"soundcloud","sourceUrl":"https://soundcloud.com/a/b"})
        );
        assert_eq!(serde_json::from_value::<Track>(value).unwrap(), dto);
        let mut source = track(None);
        source.uploader = Some("  ".into());
        assert_eq!(Track::from(source).author, "Unknown artist");
        let mut source = track(None);
        source.uploader = Some("Artist".into());
        assert_eq!(Track::from(source).author, "Artist");
    }

    #[test]
    fn duration_edge_cases() {
        for duration in [
            None,
            Some(f64::NAN),
            Some(f64::INFINITY),
            Some(f64::NEG_INFINITY),
            Some(-0.001),
            Some(f64::MAX),
            Some(9_007_199_254_741.0),
        ] {
            assert_eq!(Track::from(track(duration)).duration_ms, None);
        }
        assert_eq!(Track::from(track(Some(0.0))).duration_ms, Some(0));
        assert_eq!(Track::from(track(Some(-0.0))).duration_ms, Some(0));
        assert_eq!(Track::from(track(Some(0.0005))).duration_ms, Some(1));
        assert_eq!(Track::from(track(Some(1.234))).duration_ms, Some(1234));
    }

    #[test]
    fn errors_never_include_source_diagnostics() {
        let secret = "stderr token=SECRET https://private.example/secret";
        let cases = [
            (
                SoundCloudError::InvalidInput(secret.into()),
                ErrorCode::InvalidInput,
                false,
            ),
            (
                SoundCloudError::Process(secret.into()),
                ErrorCode::SourceUnavailable,
                false,
            ),
            (
                SoundCloudError::InvalidResponse(secret.into()),
                ErrorCode::SourceUnavailable,
                true,
            ),
            (
                SoundCloudError::ExtractorFailed {
                    status: secret.into(),
                    details: secret.into(),
                },
                ErrorCode::SourceUnavailable,
                true,
            ),
            (
                SoundCloudError::TimedOut {
                    timeout: Duration::from_secs(1),
                },
                ErrorCode::Timeout,
                true,
            ),
            (SoundCloudError::Cancelled, ErrorCode::Cancelled, false),
        ];
        for (source, code, retryable) in cases {
            let error = AppError::from(source);
            assert_eq!(error.code, code);
            assert_eq!(error.retryable, retryable);
            for output in [
                serde_json::to_string(&error).unwrap(),
                format!("{error:?}"),
                error.to_string(),
            ] {
                for forbidden in ["SECRET", "https://", "stderr", "token="] {
                    assert!(!output.contains(forbidden));
                }
            }
            assert_eq!(
                serde_json::from_str::<AppError>(&serde_json::to_string(&error).unwrap()).unwrap(),
                error
            );
        }
    }

    #[test]
    fn requests_and_responses_serialize() {
        let request: SearchTracksRequest =
            serde_json::from_value(json!({"query":"music"})).unwrap();
        assert!(request.limit.is_none());
        for limit in [json!(-1), json!(1.5), json!(1e100), json!("10")] {
            assert!(
                serde_json::from_value::<SearchTracksRequest>(
                    json!({"query":"music","limit":limit})
                )
                .is_err()
            );
        }
        let response = SearchTracksResponse {
            tracks: vec![],
            next_page_token: None,
        };
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            json!({"tracks":[],"nextPageToken":null})
        );
        let request: ResolveTrackRequest = serde_json::from_value(
            json!({"source":"soundcloud","trackUrl":"https://soundcloud.com/a/b"}),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(request).unwrap()["trackUrl"],
            "https://soundcloud.com/a/b"
        );
        let response = ResolveTrackResponse {
            track: track(None).into(),
            stream_url: "https://audio.example/temporary".into(),
        };
        let value = serde_json::to_value(response).unwrap();
        assert_eq!(value["streamUrl"], "https://audio.example/temporary");
        assert!(value["track"]["streamUrl"].is_null());
        assert!(
            serde_json::from_value::<ResolveTrackRequest>(json!({"source":"other","trackUrl":"x"}))
                .is_err()
        );
    }

    #[test]
    fn boundary_preserves_validation_cancellation_and_options() {
        let app = Application::new(SoundCloudClient::new("missing-extractor"));
        let search = || SearchTracksRequest {
            query: "music".into(),
            limit: None,
        };
        let options = RequestOptions::default();
        options.cancellation.cancel();
        assert_eq!(
            app.search_tracks(search(), &options).unwrap_err().code,
            ErrorCode::Cancelled
        );
        assert_eq!(
            app.resolve_track(
                ResolveTrackRequest {
                    source: Source::Soundcloud,
                    track_url: "https://soundcloud.com/a/b".into()
                },
                &options
            )
            .unwrap_err()
            .code,
            ErrorCode::Cancelled
        );
        let options = RequestOptions {
            timeout: Duration::ZERO,
            ..Default::default()
        };
        assert_eq!(
            app.search_tracks(search(), &options).unwrap_err().code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            app.search_tracks(
                SearchTracksRequest {
                    query: " ".into(),
                    limit: Some(1)
                },
                &RequestOptions::default()
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
        for limit in [0, 101] {
            assert_eq!(
                app.search_tracks(
                    SearchTracksRequest {
                        query: "music".into(),
                        limit: Some(limit)
                    },
                    &RequestOptions::default()
                )
                .unwrap_err()
                .code,
                ErrorCode::InvalidInput
            );
        }
    }
}
