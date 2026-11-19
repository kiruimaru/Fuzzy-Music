//! Minimal SoundCloud web/API client for playlist discovery.
//!
//! `yt-dlp` can search tracks and expand a playlist URL, but it cannot search
//! playlists. Playlist search therefore uses the public `api-v2` endpoint, which
//! requires a `client_id` embedded in SoundCloud's own JavaScript bundles.
//! No user account, token or manually obtained key is involved, and the id is
//! cached for the lifetime of the process only.
use crate::{
    Artist, Playlist, SoundCloudError, Track,
    control::{RequestOptions, check_cancelled},
};
use serde::Deserialize;
use std::{
    sync::{Mutex, OnceLock},
    time::Instant,
};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const API: &str = "https://api-v2.soundcloud.com";
const PAGE: &str = "https://soundcloud.com";

fn cached_client_id() -> &'static Mutex<Option<String>> {
    static ID: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    ID.get_or_init(|| Mutex::new(None))
}

/// Drop the cached id so the next call rediscovers it. Called after 401/403.
pub(crate) fn forget_client_id() {
    if let Ok(mut id) = cached_client_id().lock() {
        *id = None;
    }
}

struct Http {
    agent: ureq::Agent,
}

impl Http {
    fn new(
        options: &RequestOptions,
        network: Option<&crate::network::NetworkPolicy>,
    ) -> Result<Self, SoundCloudError> {
        let mut builder = ureq::Agent::config_builder()
            .timeout_global(Some(options.timeout))
            .user_agent(USER_AGENT)
            .http_status_as_error(false);
        // Reuse the same environment-based proxy policy as child processes.
        let proxy = match network {
            Some(policy) => policy.https_proxy()?,
            None => ureq::Proxy::try_from_env(),
        };
        builder = builder.proxy(proxy);
        Ok(Self {
            agent: builder.build().into(),
        })
    }

    fn get(&self, url: &str, options: &RequestOptions) -> Result<(u16, String), SoundCloudError> {
        check_cancelled(options)?;
        let started = Instant::now();
        let mut response = self
            .agent
            .get(url)
            .header("Accept", "application/json")
            .call()
            .map_err(|error| map_error(error, options, started))?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .with_config()
            .limit(8 * 1024 * 1024)
            .read_to_string()
            .map_err(|error| map_error(error, options, started))?;
        check_cancelled(options)?;
        Ok((status, body))
    }
}

fn map_error(error: ureq::Error, options: &RequestOptions, started: Instant) -> SoundCloudError {
    if options.cancellation.is_cancelled() {
        return SoundCloudError::Cancelled;
    }
    let text = error.to_string();
    let timeout = text.to_ascii_lowercase();
    if timeout.contains("timeout") || started.elapsed() >= options.timeout {
        return SoundCloudError::TimedOut {
            timeout: options.timeout,
        };
    }
    // Diagnostics are never forwarded to the UI; only this category is kept.
    SoundCloudError::Network(text)
}

fn get_checked(
    http: &Http,
    url: &str,
    options: &RequestOptions,
) -> Result<String, SoundCloudError> {
    let (status, body) = http.get(url, options)?;
    match status {
        200..=299 => Ok(body),
        _ => Err(SoundCloudError::Network(format!("HTTP status {status}"))),
    }
}

/// Discover the `client_id` from the scripts referenced by a SoundCloud page.
fn discover_client_id(http: &Http, options: &RequestOptions) -> Result<String, SoundCloudError> {
    let page = get_checked(http, PAGE, options)?;
    let mut scripts: Vec<String> = page
        .split("https://a-v2.sndcdn.com/assets/")
        .skip(1)
        .filter_map(|tail| {
            let end = tail.find(".js")?;
            let name = &tail[..end];
            name.chars()
                .all(|c| c.is_ascii_hexdigit() || c == '-')
                .then(|| format!("https://a-v2.sndcdn.com/assets/{name}.js"))
        })
        .collect();
    scripts.sort();
    scripts.dedup();
    if scripts.is_empty() {
        return Err(SoundCloudError::InvalidResponse(
            "SoundCloud page did not reference any scripts".into(),
        ));
    }
    for script in scripts {
        check_cancelled(options)?;
        let Ok(source) = get_checked(http, &script, options) else {
            continue;
        };
        if let Some(id) = extract_client_id(&source) {
            return Ok(id);
        }
    }
    Err(SoundCloudError::InvalidResponse(
        "SoundCloud client id was not found in scripts".into(),
    ))
}

/// Find a 32-character `client_id` in minified JavaScript.
fn extract_client_id(source: &str) -> Option<String> {
    let mut rest = source;
    while let Some(at) = rest.find("client_id") {
        rest = &rest[at + "client_id".len()..];
        let tail = rest.trim_start();
        let Some(tail) = tail.strip_prefix(':').or_else(|| tail.strip_prefix('=')) else {
            continue;
        };
        let tail = tail.trim_start();
        let Some(quoted) = tail.strip_prefix('"').or_else(|| tail.strip_prefix('\'')) else {
            continue;
        };
        let value: String = quoted
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        if value.len() == 32 {
            return Some(value);
        }
    }
    None
}

fn client_id(
    http: &Http,
    options: &RequestOptions,
    refresh: bool,
) -> Result<String, SoundCloudError> {
    if !refresh
        && let Ok(cached) = cached_client_id().lock()
        && let Some(id) = cached.as_ref()
    {
        return Ok(id.clone());
    }
    let id = discover_client_id(http, options)?;
    if let Ok(mut cached) = cached_client_id().lock() {
        *cached = Some(id.clone());
    }
    Ok(id)
}

/// Signed 32-bit track ids are valid; only lookalikes and negatives are rejected.
fn parse_track_id(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::Number(number) => {
            let id = number.as_i64()?;
            (id > 0).then(|| id.to_string())
        }
        serde_json::Value::String(text) => text
            .parse::<i64>()
            .ok()
            .filter(|id| *id > 0)
            .map(|id| id.to_string()),
        _ => None,
    }
}

#[derive(Deserialize)]
struct User {
    username: Option<String>,
}

#[derive(Deserialize)]
struct ArtistJson {
    id: serde_json::Value,
    username: Option<String>,
    permalink_url: Option<String>,
    avatar_url: Option<String>,
    track_count: Option<i64>,
}

#[derive(Deserialize)]
struct PlaylistJson {
    id: serde_json::Value,
    title: Option<String>,
    permalink_url: Option<String>,
    artwork_url: Option<String>,
    track_count: Option<i64>,
    duration: Option<f64>,
    user: Option<User>,
}

#[derive(Deserialize)]
struct SearchJson {
    #[serde(default)]
    collection: Vec<PlaylistJson>,
}

#[derive(Deserialize)]
struct PlaylistTracksJson {
    #[serde(default)]
    tracks: Vec<TrackJson>,
}

#[derive(Deserialize)]
struct TrackJson {
    id: serde_json::Value,
    title: Option<String>,
    permalink_url: Option<String>,
    artwork_url: Option<String>,
    duration: Option<f64>,
    user: Option<User>,
}

fn media_url(value: Option<String>) -> Option<String> {
    value
        .filter(|url| url.starts_with("https://"))
        .map(|url| url.replace("-large.", "-t500x500."))
}

fn text(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.trim().is_empty())
}

/// Encode a query for a URL path/query component without extra dependencies.
fn encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char)
            }
            b' ' => encoded.push('+'),
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

fn playlist_url(value: Option<String>) -> Option<String> {
    value.filter(|url| {
        url.starts_with("https://soundcloud.com/")
            && url.contains("/sets/")
            && !url.chars().any(char::is_whitespace)
    })
}

fn playlist_from_json(value: PlaylistJson) -> Option<Playlist> {
    let id = parse_track_id(Some(&value.id))?;
    Some(Playlist {
        id,
        title: text(value.title).unwrap_or_else(|| "Untitled playlist".into()),
        author: value.user.and_then(|user| text(user.username)),
        artwork_url: media_url(value.artwork_url),
        track_count: value
            .track_count
            .filter(|count| *count >= 0)
            .map(|count| count as u32),
        duration_seconds: value
            .duration
            .filter(|ms| ms.is_finite() && *ms >= 0.0)
            .map(|ms| ms / 1000.0),
        webpage_url: playlist_url(value.permalink_url)?,
    })
}

fn track_from_json(value: TrackJson) -> Option<Track> {
    let id = parse_track_id(Some(&value.id))?;
    let webpage_url = value.permalink_url.filter(|url| {
        url.starts_with("https://soundcloud.com/") && !url.chars().any(char::is_whitespace)
    })?;
    Some(Track {
        id,
        title: text(value.title)?,
        uploader: value.user.and_then(|user| text(user.username)),
        duration_seconds: value
            .duration
            .filter(|ms| ms.is_finite() && *ms >= 0.0)
            .map(|ms| ms / 1000.0),
        webpage_url,
        artwork_url: media_url(value.artwork_url),
    })
}

fn artist_from_json(value: ArtistJson) -> Option<Artist> {
    let id = parse_track_id(Some(&value.id))?;
    let webpage_url = value.permalink_url.filter(|url| {
        url.starts_with("https://soundcloud.com/") && !url.chars().any(char::is_whitespace)
    })?;
    Some(Artist {
        id,
        name: text(value.username).unwrap_or_else(|| "Unknown artist".into()),
        avatar_url: media_url(value.avatar_url),
        track_count: value
            .track_count
            .filter(|count| *count >= 0)
            .map(|count| count as u32),
        webpage_url,
    })
}

/// Artist tracks arrive either as a bare array or as `{collection: [...]}`.
fn track_collection(value: serde_json::Value) -> Result<Vec<TrackJson>, SoundCloudError> {
    if let Ok(list) = serde_json::from_value::<Vec<TrackJson>>(value.clone()) {
        return Ok(list);
    }
    #[derive(Deserialize)]
    struct Response {
        #[serde(default)]
        collection: Vec<TrackJson>,
    }
    serde_json::from_value::<Response>(value)
        .map(|response| response.collection)
        .map_err(|_| {
            SoundCloudError::InvalidResponse("SoundCloud returned unexpected artist tracks".into())
        })
}

fn decode(body: &str) -> Result<serde_json::Value, SoundCloudError> {
    serde_json::from_str(body)
        .map_err(|_| SoundCloudError::InvalidResponse("SoundCloud returned invalid JSON".into()))
}

fn api_json(
    http: &Http,
    path: &str,
    options: &RequestOptions,
    refresh: bool,
) -> Result<serde_json::Value, SoundCloudError> {
    let id = client_id(http, options, refresh)?;
    let separator = if path.contains('?') { '&' } else { '?' };
    let body = get_checked(
        http,
        &format!("{API}{path}{separator}client_id={id}"),
        options,
    )?;
    decode(&body)
}

/// Search public SoundCloud playlists (sets).
pub(crate) fn search(
    options: &RequestOptions,
    query: &str,
    limit: usize,
    network: Option<&crate::network::NetworkPolicy>,
) -> Result<Vec<Playlist>, SoundCloudError> {
    let http = Http::new(options, network)?;
    let path = format!("/search/playlists?q={}&limit={limit}", encode(query.trim()));
    // A rejected cached id is dropped once and rediscovered before failing.
    let value = match api_json(&http, &path, options, false) {
        Ok(value) => value,
        Err(SoundCloudError::Network(status)) if status.starts_with("HTTP status 40") => {
            forget_client_id();
            api_json(&http, &path, options, true)?
        }
        Err(error) => return Err(error),
    };
    let parsed: SearchJson = serde_json::from_value(value).map_err(|_| {
        SoundCloudError::InvalidResponse("Unexpected playlist search response".into())
    })?;
    Ok(parsed
        .collection
        .into_iter()
        .filter_map(playlist_from_json)
        .collect())
}

/// Load one public playlist with its tracks in a single request.
pub(crate) fn playlist(
    options: &RequestOptions,
    playlist_url: &str,
    network: Option<&crate::network::NetworkPolicy>,
) -> Result<(Playlist, Vec<Track>), SoundCloudError> {
    let http = Http::new(options, network)?;
    let resolved = match api_json(
        &http,
        &format!("/resolve?url={}", encode(playlist_url)),
        options,
        false,
    ) {
        Ok(value) => value,
        Err(SoundCloudError::Network(status)) if status.starts_with("HTTP status 40") => {
            forget_client_id();
            api_json(
                &http,
                &format!("/resolve?url={}", encode(playlist_url)),
                options,
                true,
            )?
        }
        Err(error) => return Err(error),
    };
    let id = parse_track_id(resolved.get("id")).ok_or_else(|| {
        SoundCloudError::InvalidResponse("SoundCloud returned an unexpected playlist".into())
    })?;
    let full = api_json(&http, &format!("/playlists/{id}"), options, false)?;
    let playlist: PlaylistJson = serde_json::from_value(full.clone()).map_err(|_| {
        SoundCloudError::InvalidResponse("SoundCloud returned an unexpected playlist".into())
    })?;
    let playlist = playlist_from_json(playlist).ok_or_else(|| {
        SoundCloudError::InvalidResponse("Playlist metadata is incomplete".into())
    })?;
    let body: PlaylistTracksJson = serde_json::from_value(full).map_err(|_| {
        SoundCloudError::InvalidResponse("SoundCloud returned unexpected playlist tracks".into())
    })?;
    // Large playlists return some entries as bare stubs ({id, kind, policy}).
    // Their metadata must be fetched by id; otherwise they would be dropped.
    // Order is preserved by remembering each stub's position and filling it in.
    let total = body.tracks.len();
    let mut tracks: Vec<Option<Track>> = Vec::with_capacity(total);
    let mut pending: Vec<(usize, String)> = Vec::new();
    for (index, entry) in body.tracks.into_iter().enumerate() {
        let stub_id = parse_track_id(Some(&entry.id));
        let track = track_from_json(entry);
        if track.is_none()
            && let Some(id) = stub_id
        {
            pending.push((index, id));
        }
        tracks.push(track);
    }
    for chunk in pending.chunks(50) {
        check_cancelled(options)?;
        let ids = chunk
            .iter()
            .map(|(_, id)| id.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let path = format!("/tracks?ids={ids}");
        let value = api_json(&http, &path, options, false)?;
        // A rejected ids response must not discard already parsed entries.
        let Ok(resolved) = serde_json::from_value::<Vec<TrackJson>>(value) else {
            continue;
        };
        for track in resolved.into_iter().filter_map(track_from_json) {
            if let Some((position, _)) = chunk.iter().find(|(_, id)| *id == track.id) {
                tracks[*position] = Some(track);
            }
        }
    }
    let tracks: Vec<Track> = tracks.into_iter().flatten().collect();
    Ok((playlist, tracks))
}

/// Load one public artist profile and a page of their tracks.
pub(crate) fn artist(
    options: &RequestOptions,
    artist_url: &str,
    network: Option<&crate::network::NetworkPolicy>,
) -> Result<(Artist, Vec<Track>), SoundCloudError> {
    check_cancelled(options)?;
    let http = Http::new(options, network)?;
    let path = format!("/resolve?url={}", encode(artist_url));
    let resolved = match api_json(&http, &path, options, false) {
        Ok(value) => value,
        Err(SoundCloudError::Network(status)) if status.starts_with("HTTP status 40") => {
            forget_client_id();
            api_json(&http, &path, options, true)?
        }
        Err(error) => return Err(error),
    };
    let profile: ArtistJson = serde_json::from_value(resolved).map_err(|_| {
        SoundCloudError::InvalidResponse("SoundCloud returned an unexpected artist".into())
    })?;
    let artist = artist_from_json(profile)
        .ok_or_else(|| SoundCloudError::InvalidResponse("Artist metadata is incomplete".into()))?;
    check_cancelled(options)?;
    let value = api_json(
        &http,
        &format!("/users/{}/tracks?limit=50", artist.id),
        options,
        false,
    )?;
    let tracks = track_collection(value)?
        .into_iter()
        .filter_map(track_from_json)
        .take(50)
        .collect();
    Ok((artist, tracks))
}

pub(crate) fn search_tracks(
    options: &RequestOptions,
    query: &str,
    limit: usize,
    network: Option<&crate::network::NetworkPolicy>,
) -> Result<Vec<Track>, SoundCloudError> {
    check_cancelled(options)?;
    let http = Http::new(options, network)?;
    let path = format!("/search/tracks?q={}&limit={limit}", encode(query));
    let value = match api_json(&http, &path, options, false) {
        Ok(value) => value,
        Err(SoundCloudError::Network(status))
            if status == "HTTP status 401" || status == "HTTP status 403" =>
        {
            forget_client_id();
            api_json(&http, &path, options, true)?
        }
        Err(error) => return Err(error),
    };
    #[derive(Deserialize)]
    struct Response {
        collection: Vec<TrackJson>,
    }
    let response: Response = serde_json::from_value(value)
        .map_err(|_| SoundCloudError::InvalidResponse("Unexpected track search response".into()))?;
    Ok(response
        .collection
        .into_iter()
        .filter_map(track_from_json)
        .take(limit)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_id_is_found_in_minified_javascript() {
        let source = r#"var a={client_id:"0123456789abcdef0123456789abcdef",x:1}"#;
        assert_eq!(
            extract_client_id(source).as_deref(),
            Some("0123456789abcdef0123456789abcdef")
        );
        assert_eq!(
            extract_client_id("client_id='ffffffffffffffffffffffffffffffff'").as_deref(),
            Some("ffffffffffffffffffffffffffffffff")
        );
        assert_eq!(extract_client_id("no identifier here"), None);
        assert_eq!(extract_client_id("client_id:\"too-short\""), None);
    }

    #[test]
    fn query_encoding_is_url_safe() {
        assert_eq!(encode("lofi chill"), "lofi+chill");
        assert_eq!(encode("a&b=c/d"), "a%26b%3Dc%2Fd");
        assert_eq!(encode("\u{00FC}n\u{00EF}code"), "%C3%BCn%C3%AFcode");
    }

    #[test]
    fn invalid_playlist_urls_are_rejected_before_network_access() {
        let client = crate::SoundCloudClient::new("yt-dlp");
        for url in [
            "https://example.com/user/sets/x",
            "http://soundcloud.com/user/sets/x",
            "https://soundcloud.com/user/track",
            "javascript:alert(1)",
        ] {
            assert!(matches!(
                client.playlist_tracks(url),
                Err(SoundCloudError::InvalidInput(_))
            ));
        }
    }

    #[test]
    fn bare_playlist_stubs_are_detected_for_metadata_refetch() {
        // Large playlists return {id, kind, policy} entries without metadata.
        let stub: serde_json::Value = serde_json::json!({
            "id": 2362658306_u64, "kind": "track", "policy": "MONETIZE"
        });
        let stub = serde_json::from_value::<TrackJson>(stub).unwrap();
        assert!(track_from_json(stub).is_none());

        let complete: serde_json::Value = serde_json::json!({
            "id": 2362658306_u64, "title": "Kickflip", "permalink_url": "https://soundcloud.com/hoogway/kickflip",
            "duration": 120000, "user": {"username": "hoogway"}
        });
        let complete = serde_json::from_value::<TrackJson>(complete).unwrap();
        let track = track_from_json(complete).expect("complete track");
        assert_eq!(track.id, "2362658306");
        assert_eq!(track.title, "Kickflip");
        assert_eq!(track.duration_seconds, Some(120.0));
    }

    #[test]
    fn artist_metadata_is_parsed_and_incomplete_profiles_are_rejected() {
        let value: ArtistJson = serde_json::from_value(serde_json::json!({
            "id": 42, "username": "Artist", "permalink_url": "https://soundcloud.com/artist",
            "avatar_url": "https://i1.sndcdn.com/avatar-large.jpg", "track_count": 12
        }))
        .unwrap();
        let artist = artist_from_json(value).expect("artist profile");
        assert_eq!(artist.id, "42");
        assert_eq!(artist.name, "Artist");
        assert_eq!(artist.track_count, Some(12));
        assert_eq!(
            artist.avatar_url.as_deref(),
            Some("https://i1.sndcdn.com/avatar-t500x500.jpg")
        );

        let missing: ArtistJson = serde_json::from_value(serde_json::json!({"id": 7})).unwrap();
        assert!(artist_from_json(missing).is_none());
    }

    #[test]
    fn artist_track_lists_support_array_and_collection_shapes() {
        let track = serde_json::json!({
            "id": 1, "title": "One", "permalink_url": "https://soundcloud.com/artist/one"
        });
        let array = serde_json::Value::Array(vec![track.clone()]);
        assert_eq!(track_collection(array).unwrap().len(), 1);
        let collection = serde_json::json!({ "collection": [track] });
        assert_eq!(track_collection(collection).unwrap().len(), 1);
    }
}
