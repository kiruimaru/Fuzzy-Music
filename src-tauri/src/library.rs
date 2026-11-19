use fuzzy_application::{
    AppError, CancellationToken, ErrorCode, Playlist, RequestOptions, Source, Track,
};
use fuzzy_soundcloud::{DownloadQuality, SoundCloudClient, network::NetworkPolicy};
use lofty::{
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::Accessor,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    thread::JoinHandle,
    time::Duration,
};

pub fn storage_error() -> AppError {
    AppError {
        code: ErrorCode::StorageFailed,
        message: "Could not access the music library. Check the folder and free disk space.".into(),
        retryable: true,
    }
}

fn invalid() -> AppError {
    AppError {
        code: ErrorCode::InvalidInput,
        message: "Invalid library settings or track.".into(),
        retryable: false,
    }
}

/// Accept only public HTTPS SoundCloud pages; reject credentials and lookalikes.
fn soundcloud_url(value: &str) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    url.scheme() == "https"
        && matches!(
            url.host_str(),
            Some("soundcloud.com" | "www.soundcloud.com" | "m.soundcloud.com")
        )
        && url.username().is_empty()
        && url.password().is_none()
}

fn valid_playlist(playlist: &Playlist) -> Result<(), AppError> {
    if playlist.source != Source::Soundcloud
        || playlist.id.is_empty()
        || playlist.id.len() > 256
        || playlist.title.len() > 4096
        || playlist.source_url.len() > 4096
        || !soundcloud_url(&playlist.source_url)
    {
        return Err(invalid());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub volume: f64,
    pub download_quality: DownloadQuality,
    pub music_folder: String,
    pub theme: Theme,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsPatch {
    pub volume: Option<f64>,
    pub download_quality: Option<DownloadQuality>,
    pub music_folder: Option<String>,
    pub theme: Option<Theme>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub id: i64,
    pub track: Track,
    pub favourite: bool,
    pub download_state: String,
    pub error: Option<String>,
    pub available_offline: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistEntry {
    pub id: i64,
    pub playlist: Playlist,
    pub favourite: bool,
    pub downloaded: u32,
    pub total: u32,
    /// `idle` (not downloaded), `downloading`, `ready` or `error`.
    pub state: String,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub imported: usize,
    pub skipped: usize,
    pub truncated: bool,
}

pub struct Library {
    connection: Mutex<Connection>,
}

impl Library {
    pub fn open(data: &Path, default_music: &Path) -> Result<Self, AppError> {
        fs::create_dir_all(data).map_err(|_| storage_error())?;
        let connection =
            Connection::open(data.join("library.sqlite")).map_err(|_| storage_error())?;
        connection
            .busy_timeout(Duration::from_secs(3))
            .map_err(|_| storage_error())?;
        let version: u32 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|_| storage_error())?;
        if version > 3 {
            return Err(storage_error());
        }
        connection.execute_batch(
            "BEGIN IMMEDIATE;
             CREATE TABLE IF NOT EXISTS settings (
               id INTEGER PRIMARY KEY CHECK(id=1), volume REAL NOT NULL CHECK(volume BETWEEN 0 AND 100),
               download_quality TEXT NOT NULL, music_folder TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS media (
               id INTEGER PRIMARY KEY, source TEXT NOT NULL, source_id TEXT NOT NULL,
               track_json TEXT NOT NULL, path TEXT UNIQUE, favourite INTEGER NOT NULL DEFAULT 0,
               download_state TEXT NOT NULL DEFAULT 'none', error TEXT,
               UNIQUE(source, source_id));
             CREATE TABLE IF NOT EXISTS playlists (
               id INTEGER PRIMARY KEY, source TEXT NOT NULL, source_id TEXT NOT NULL,
               playlist_json TEXT NOT NULL, favourite INTEGER NOT NULL DEFAULT 0,
               UNIQUE(source, source_id));
             CREATE TABLE IF NOT EXISTS playlist_tracks (
               playlist_id INTEGER NOT NULL, media_id INTEGER NOT NULL, position INTEGER NOT NULL,
               PRIMARY KEY(playlist_id, media_id));
             COMMIT;"
        ).map_err(|_| storage_error())?;
        if version < 2 {
            connection
                .execute_batch(
                    "BEGIN IMMEDIATE;
                 ALTER TABLE settings ADD COLUMN theme TEXT NOT NULL DEFAULT '\"dark\"';
                 PRAGMA user_version=2;
                 COMMIT;",
                )
                .map_err(|_| storage_error())?;
        }
        if version < 3 {
            connection
                .execute_batch(
                    "BEGIN IMMEDIATE;
                 PRAGMA user_version=3;
                 COMMIT;",
                )
                .map_err(|_| storage_error())?;
        }
        connection
            .execute(
                "INSERT OR IGNORE INTO settings(id,volume,download_quality,music_folder) VALUES(1,100,'\"best\"',?1)",
                [default_music.to_string_lossy().as_ref()],
            )
            .map_err(|_| storage_error())?;
        connection
            .execute(
                "UPDATE media SET download_state='pending' WHERE download_state='downloading'",
                [],
            )
            .map_err(|_| storage_error())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn settings(&self) -> Result<AppSettings, AppError> {
        let connection = self.connection.lock().map_err(|_| storage_error())?;
        let (volume, quality, music_folder, theme): (f64, String, String, String) = connection
            .query_row(
                "SELECT volume,download_quality,music_folder,theme FROM settings WHERE id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|_| storage_error())?;
        Ok(AppSettings {
            volume,
            download_quality: serde_json::from_str(&quality).map_err(|_| storage_error())?,
            music_folder,
            theme: serde_json::from_str(&theme).map_err(|_| storage_error())?,
        })
    }

    pub fn update_settings(&self, patch: SettingsPatch) -> Result<AppSettings, AppError> {
        if patch
            .volume
            .is_some_and(|v| !v.is_finite() || !(0.0..=100.0).contains(&v))
        {
            return Err(invalid());
        }
        if let Some(folder) = &patch.music_folder {
            let path = Path::new(folder);
            if !path.is_absolute() || !path.is_dir() {
                return Err(invalid());
            }
            // Verify write access without touching existing user files.
            tempfile::NamedTempFile::new_in(path).map_err(|_| storage_error())?;
        }
        let connection = self.connection.lock().map_err(|_| storage_error())?;
        let quality = patch
            .download_quality
            .map(|v| serde_json::to_string(&v))
            .transpose()
            .map_err(|_| invalid())?;
        let theme = patch
            .theme
            .map(|v| serde_json::to_string(&v))
            .transpose()
            .map_err(|_| invalid())?;
        connection.execute(
            "UPDATE settings SET volume=COALESCE(?1,volume), download_quality=COALESCE(?2,download_quality),
             music_folder=COALESCE(?3,music_folder), theme=COALESCE(?4,theme) WHERE id=1",
            params![patch.volume, quality, patch.music_folder, theme],
        ).map_err(|_| storage_error())?;
        drop(connection);
        self.settings()
    }

    pub fn entries(&self) -> Result<Vec<LibraryEntry>, AppError> {
        let connection = self.connection.lock().map_err(|_| storage_error())?;
        let mut statement = connection.prepare("SELECT id,track_json,favourite,download_state,error,path FROM media ORDER BY id DESC").map_err(|_| storage_error())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(|_| storage_error())?;
        rows.map(|row| {
            let (id, track, favourite, mut download_state, error, path) =
                row.map_err(|_| storage_error())?;
            let available_offline = path.as_ref().is_some_and(|p| Path::new(p).is_file());
            if path.is_some() && !available_offline {
                download_state = "missing".into();
            }
            Ok(LibraryEntry {
                id,
                track: serde_json::from_str(&track).map_err(|_| storage_error())?,
                favourite,
                download_state,
                error,
                available_offline,
            })
        })
        .collect()
    }

    pub fn entry(&self, id: i64) -> Result<LibraryEntry, AppError> {
        self.entries()?
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(invalid)
    }

    pub fn path(&self, id: i64) -> Result<PathBuf, AppError> {
        let connection = self.connection.lock().map_err(|_| storage_error())?;
        let path: Option<String> = connection
            .query_row("SELECT path FROM media WHERE id=?1", [id], |row| row.get(0))
            .optional()
            .map_err(|_| storage_error())?
            .flatten();
        let path = path
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .ok_or_else(|| AppError {
                code: ErrorCode::NotFound,
                message: "Local audio is missing. Refresh the library or download it again.".into(),
                retryable: true,
            })?;
        Ok(path)
    }

    fn is_playlist_track(&self, id: i64) -> Result<bool, AppError> {
        self.connection
            .lock()
            .map_err(|_| storage_error())?
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM playlist_tracks WHERE media_id=?1)",
                [id],
                |row| row.get(0),
            )
            .map_err(|_| storage_error())
    }

    fn needs_download(&self, id: i64) -> bool {
        self.entry(id).is_ok_and(|entry| {
            !entry.available_offline
                && (entry.favourite || self.is_playlist_track(id).unwrap_or(false))
        })
    }

    pub fn favourite(&self, mut track: Track, liked: bool) -> Result<i64, AppError> {
        if track.id.is_empty()
            || track.id.len() > 256
            || track.title.len() > 4096
            || track.source_url.len() > 4096
        {
            return Err(invalid());
        }
        track.stream_url = None;
        if track.source == Source::Local {
            let id = track
                .id
                .strip_prefix("local-")
                .and_then(|v| v.parse::<i64>().ok())
                .ok_or_else(invalid)?;
            let connection = self.connection.lock().map_err(|_| storage_error())?;
            let changed = connection
                .execute(
                    "UPDATE media SET favourite=?1 WHERE id=?2 AND source='local'",
                    params![liked, id],
                )
                .map_err(|_| storage_error())?;
            if changed == 0 {
                return Err(invalid());
            }
            return Ok(id);
        }
        if !soundcloud_url(&track.source_url) {
            return Err(invalid());
        }
        let connection = self.connection.lock().map_err(|_| storage_error())?;
        connection.execute(
            "INSERT INTO media(source,source_id,track_json,favourite) VALUES('soundcloud',?1,?2,?3)
             ON CONFLICT(source,source_id) DO UPDATE SET track_json=excluded.track_json,favourite=excluded.favourite",
            params![track.id, serde_json::to_string(&track).map_err(|_| invalid())?, liked],
        ).map_err(|_| storage_error())?;
        connection
            .query_row(
                "SELECT id FROM media WHERE source='soundcloud' AND source_id=?1",
                [track.id],
                |row| row.get(0),
            )
            .map_err(|_| storage_error())
    }

    pub fn playlist_favourite(&self, playlist: Playlist, liked: bool) -> Result<i64, AppError> {
        valid_playlist(&playlist)?;
        let connection = self.connection.lock().map_err(|_| storage_error())?;
        connection
            .execute(
                "INSERT INTO playlists(source,source_id,playlist_json,favourite) VALUES('soundcloud',?1,?2,?3)
                 ON CONFLICT(source,source_id) DO UPDATE SET playlist_json=excluded.playlist_json,favourite=excluded.favourite",
                params![
                    playlist.id,
                    serde_json::to_string(&playlist).map_err(|_| invalid())?,
                    liked
                ],
            )
            .map_err(|_| storage_error())?;
        connection
            .query_row(
                "SELECT id FROM playlists WHERE source='soundcloud' AND source_id=?1",
                [playlist.id],
                |row| row.get(0),
            )
            .map_err(|_| storage_error())
    }

    pub fn playlists(&self) -> Result<Vec<PlaylistEntry>, AppError> {
        let connection = self.connection.lock().map_err(|_| storage_error())?;
        let mut statement = connection
            .prepare("SELECT id,playlist_json,favourite FROM playlists ORDER BY id DESC")
            .map_err(|_| storage_error())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            })
            .map_err(|_| storage_error())?;
        let mut entries = Vec::new();
        for row in rows {
            let (id, playlist, favourite) = row.map_err(|_| storage_error())?;
            let (downloaded, total, active, failed) = playlist_counts(&connection, id)?;
            let state = if total > 0 && downloaded == total {
                "ready"
            } else if active {
                "downloading"
            } else if failed {
                "error"
            } else {
                "idle"
            };
            entries.push(PlaylistEntry {
                id,
                playlist: serde_json::from_str(&playlist).map_err(|_| storage_error())?,
                favourite,
                downloaded,
                total,
                state: state.into(),
            });
        }
        Ok(entries)
    }

    /// Register a playlist's tracks for optional offline download. Returns the
    /// media ids to enqueue; the playlist itself is bookmarked separately.
    pub fn playlist_download_queue(
        &self,
        playlist: Playlist,
        tracks: Vec<Track>,
    ) -> Result<Vec<i64>, AppError> {
        valid_playlist(&playlist)?;
        let mut connection = self.connection.lock().map_err(|_| storage_error())?;
        let transaction = connection.transaction().map_err(|_| storage_error())?;
        transaction
            .execute(
                "INSERT INTO playlists(source,source_id,playlist_json) VALUES('soundcloud',?1,?2)
                 ON CONFLICT(source,source_id) DO UPDATE SET playlist_json=excluded.playlist_json",
                params![
                    playlist.id,
                    serde_json::to_string(&playlist).map_err(|_| invalid())?
                ],
            )
            .map_err(|_| storage_error())?;
        let playlist_id: i64 = transaction
            .query_row(
                "SELECT id FROM playlists WHERE source='soundcloud' AND source_id=?1",
                [playlist.id],
                |row| row.get(0),
            )
            .map_err(|_| storage_error())?;
        let mut ids = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (position, mut track) in tracks.into_iter().take(1000).enumerate() {
            if track.source != Source::Soundcloud || !seen.insert(track.id.clone()) {
                continue;
            }
            if !soundcloud_url(&track.source_url) {
                continue;
            }
            track.stream_url = None;
            transaction
                .execute(
                    "INSERT INTO media(source,source_id,track_json) VALUES('soundcloud',?1,?2)
                     ON CONFLICT(source,source_id) DO UPDATE SET track_json=excluded.track_json",
                    params![
                        track.id,
                        serde_json::to_string(&track).map_err(|_| invalid())?
                    ],
                )
                .map_err(|_| storage_error())?;
            let media_id: i64 = transaction
                .query_row(
                    "SELECT id FROM media WHERE source='soundcloud' AND source_id=?1",
                    [&track.id],
                    |row| row.get(0),
                )
                .map_err(|_| storage_error())?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO playlist_tracks(playlist_id,media_id,position) VALUES(?1,?2,?3)",
                    params![playlist_id, media_id, position as i64],
                )
                .map_err(|_| storage_error())?;
            transaction
                .execute(
                    "UPDATE media SET download_state='pending',error=NULL
                     WHERE id=?1 AND path IS NULL AND download_state NOT IN ('pending','downloading')",
                    [media_id],
                )
                .map_err(|_| storage_error())?;
            ids.push(media_id);
        }
        transaction.commit().map_err(|_| storage_error())?;
        Ok(ids)
    }

    /// Stop pending and in-flight downloads of a bookmarked playlist without
    /// removing its bookmark or track list. Returns the affected media ids.
    pub fn playlist_download_cancel(&self, playlist: &Playlist) -> Result<Vec<i64>, AppError> {
        valid_playlist(playlist)?;
        let mut connection = self.connection.lock().map_err(|_| storage_error())?;
        let transaction = connection.transaction().map_err(|_| storage_error())?;
        let playlist_id: Option<i64> = transaction
            .query_row(
                "SELECT id FROM playlists WHERE source='soundcloud' AND source_id=?1",
                [&playlist.id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| storage_error())?;
        let Some(playlist_id) = playlist_id else {
            return Ok(Vec::new());
        };
        let mut statement = transaction
            .prepare(
                "SELECT m.id FROM media m JOIN playlist_tracks p ON p.media_id=m.id
                 WHERE p.playlist_id=?1 AND m.path IS NULL
                   AND m.download_state IN ('pending','downloading')",
            )
            .map_err(|_| storage_error())?;
        let ids = statement
            .query_map([playlist_id], |row| row.get::<_, i64>(0))
            .map_err(|_| storage_error())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| storage_error())?;
        drop(statement);
        for id in &ids {
            transaction
                .execute(
                    "UPDATE media SET download_state='none',error=NULL WHERE id=?1 AND path IS NULL",
                    [id],
                )
                .map_err(|_| storage_error())?;
        }
        transaction.commit().map_err(|_| storage_error())?;
        Ok(ids)
    }

    fn state(&self, id: i64, state: &str, error: Option<&str>) -> Result<(), AppError> {
        self.connection
            .lock()
            .map_err(|_| storage_error())?
            .execute(
                "UPDATE media SET download_state=?1,error=?2 WHERE id=?3",
                params![state, error, id],
            )
            .map_err(|_| storage_error())?;
        Ok(())
    }

    pub fn scan(&self) -> Result<ScanResult, AppError> {
        let settings = self.settings()?;
        let root = PathBuf::from(settings.music_folder);
        fs::create_dir_all(&root).map_err(|_| storage_error())?;
        let mut result = ScanResult::default();
        for (count, file) in walkdir::WalkDir::new(root)
            .follow_links(false)
            .max_depth(16)
            .into_iter()
            .filter_entry(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
            .enumerate()
        {
            if count >= 20_000 {
                result.truncated = true;
                break;
            }
            let Ok(file) = file else {
                result.skipped += 1;
                continue;
            };
            if !file.file_type().is_file() || !audio_extension(file.path()) {
                continue;
            }
            let path = file.path().canonicalize().map_err(|_| storage_error())?;
            let mut connection = self.connection.lock().map_err(|_| storage_error())?;
            let exists: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM media WHERE path=?1)",
                    [path.to_string_lossy()],
                    |row| row.get(0),
                )
                .map_err(|_| storage_error())?;
            if exists {
                continue;
            }
            drop(connection);
            let tagged = match Probe::open(&path).and_then(|probe| probe.read()) {
                Ok(tagged) => tagged,
                Err(_) => {
                    result.skipped += 1;
                    continue;
                }
            };
            let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
            let title = tag
                .and_then(|tag| tag.title())
                .map(|v| v.into_owned())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| {
                    path.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                });
            let author = tag
                .and_then(|tag| tag.artist())
                .map(|v| v.into_owned())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "Unknown artist".into());
            connection = self.connection.lock().map_err(|_| storage_error())?;
            let transaction = connection.transaction().map_err(|_| storage_error())?;
            let inserted = transaction.execute(
                "INSERT OR IGNORE INTO media(source,source_id,track_json,path,download_state) VALUES('local','','{}',?1,'ready')",
                [path.to_string_lossy()],
            ).map_err(|_| storage_error())?;
            if inserted != 0 {
                let id = transaction.last_insert_rowid();
                let track = Track {
                    id: format!("local-{id}"),
                    title,
                    author,
                    artwork_url: None,
                    duration_ms: Some(
                        tagged
                            .properties()
                            .duration()
                            .as_millis()
                            .min(9_007_199_254_740_991) as u64,
                    ),
                    stream_url: None,
                    source: Source::Local,
                    source_url: format!("local:{id}"),
                };
                transaction
                    .execute(
                        "UPDATE media SET source_id=?1,track_json=?2 WHERE id=?3",
                        params![
                            track.id,
                            serde_json::to_string(&track).map_err(|_| storage_error())?,
                            id
                        ],
                    )
                    .map_err(|_| storage_error())?;
                result.imported += 1;
            }
            transaction.commit().map_err(|_| storage_error())?;
        }
        Ok(result)
    }
}

fn playlist_counts(
    connection: &Connection,
    playlist_id: i64,
) -> Result<(u32, u32, bool, bool), AppError> {
    let mut statement = connection
        .prepare(
            "SELECT m.path,m.download_state FROM playlist_tracks pt
             JOIN media m ON m.id=pt.media_id WHERE pt.playlist_id=?1",
        )
        .map_err(|_| storage_error())?;
    let rows = statement
        .query_map([playlist_id], |row| {
            Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| storage_error())?;
    let (mut downloaded, mut total, mut active, mut failed) = (0u32, 0u32, false, false);
    for row in rows {
        let (path, state) = row.map_err(|_| storage_error())?;
        total += 1;
        if path.as_ref().is_some_and(|path| Path::new(path).is_file()) {
            downloaded += 1;
        } else if matches!(state.as_str(), "pending" | "downloading") {
            active = true;
        } else if matches!(state.as_str(), "error" | "missing") {
            failed = true;
        }
    }
    Ok((downloaded, total, active, failed))
}

fn audio_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|v| v.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "mp3" | "flac" | "wav" | "m4a" | "mp4" | "aac" | "ogg" | "opus" | "aif" | "aiff"
            )
        })
}

pub struct LibraryService {
    pub library: Library,
    sender: Mutex<Option<SyncSender<i64>>>,
    tokens: Mutex<HashMap<i64, CancellationToken>>,
    // Ids explicitly cancelled with their playlist. The worker must not
    // re-enqueue them through the favourite-toggle safety net.
    cancelled: Mutex<HashSet<i64>>,
    worker: Mutex<Option<JoinHandle<()>>>,
    closing: AtomicBool,
}

impl LibraryService {
    pub fn start(library: Library, executable: PathBuf, network: NetworkPolicy) -> Arc<Self> {
        // Room for a whole playlist of downloads without failing the queue.
        let (sender, receiver) = mpsc::sync_channel(1024);
        let service = Arc::new(Self {
            library,
            sender: Mutex::new(Some(sender)),
            tokens: Mutex::new(HashMap::new()),
            cancelled: Mutex::new(HashSet::new()),
            worker: Mutex::new(None),
            closing: AtomicBool::new(false),
        });
        let weak = Arc::downgrade(&service);
        let worker = std::thread::spawn(move || {
            let client = SoundCloudClient::new(executable).with_network(network);
            while let Ok(id) = receiver.recv() {
                let Some(service) = weak.upgrade() else { break };
                if service.closing.load(Ordering::Acquire) {
                    break;
                }
                // A playlist cancellation interrupts work that is already running
                // and skips ids that are still waiting in the channel.
                let cancelled = service.take_cancelled(id);
                let token = service
                    .tokens
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get(&id)
                    .cloned();
                let Some(token) = token else { continue };
                if !cancelled && !token.is_cancelled() {
                    let result = service.download(id, &client, token.clone());
                    if let Err(error) = result {
                        let state = if token.is_cancelled() {
                            "none"
                        } else {
                            "error"
                        };
                        let _ = service.library.state(
                            id,
                            state,
                            (!token.is_cancelled()).then_some(error.message.as_str()),
                        );
                    }
                } else {
                    let _ = service.library.state(id, "none", None);
                }
                service
                    .tokens
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&id);
                // A playlist cancellation must never be undone by the requeue
                // safety net used for rapid favourite toggling.
                if token.is_cancelled()
                    && !cancelled
                    && !service.take_cancelled(id)
                    && !service.closing.load(Ordering::Acquire)
                    && service.library.needs_download(id)
                {
                    let _ = service.enqueue(id);
                }
            }
        });
        *service.worker.lock().unwrap_or_else(|e| e.into_inner()) = Some(worker);
        service
    }

    pub fn set_favourite(&self, track: Track, liked: bool) -> Result<(), AppError> {
        let id = self.library.favourite(track, liked)?;
        if liked {
            self.enqueue(id)?;
        } else if let Some(token) = self.tokens.lock().map_err(|_| storage_error())?.get(&id) {
            token.cancel();
        }
        Ok(())
    }

    pub fn set_playlist_favourite(&self, playlist: Playlist, liked: bool) -> Result<(), AppError> {
        self.library.playlist_favourite(playlist, liked)?;
        Ok(())
    }

    /// Download every track of a bookmarked playlist for offline playback.
    pub fn download_playlist(
        &self,
        playlist: Playlist,
        tracks: Vec<Track>,
    ) -> Result<(), AppError> {
        for id in self.library.playlist_download_queue(playlist, tracks)? {
            self.enqueue(id)?;
        }
        Ok(())
    }

    /// Stop pending and in-flight downloads for a playlist. The bookmark and its
    /// track list stay intact so a later download request can resume them.
    pub fn cancel_playlist_download(&self, playlist: &Playlist) -> Result<(), AppError> {
        let ids = self.library.playlist_download_cancel(playlist)?;
        if ids.is_empty() {
            return Ok(());
        }
        let mut cancelled = self.cancelled.lock().map_err(|_| storage_error())?;
        let tokens = self.tokens.lock().map_err(|_| storage_error())?;
        for id in ids {
            cancelled.insert(id);
            if let Some(token) = tokens.get(&id) {
                token.cancel();
            }
        }
        Ok(())
    }

    pub fn enqueue(&self, id: i64) -> Result<(), AppError> {
        self.cancelled
            .lock()
            .map_err(|_| storage_error())?
            .remove(&id);
        let entry = self.library.entry(id)?;
        if entry.track.source == Source::Local || entry.available_offline {
            return Ok(());
        }
        if !entry.favourite && !self.library.is_playlist_track(id)? {
            return Ok(());
        }
        let mut tokens = self.tokens.lock().map_err(|_| storage_error())?;
        if self.closing.load(Ordering::Acquire) {
            return Err(storage_error());
        }
        if tokens.contains_key(&id) {
            return Ok(());
        }
        tokens.insert(id, CancellationToken::default());
        self.library.state(id, "pending", None)?;
        if self
            .sender
            .lock()
            .map_err(|_| storage_error())?
            .as_ref()
            .is_none_or(|sender| sender.try_send(id).is_err())
        {
            tokens.remove(&id);
            self.library.state(
                id,
                "error",
                Some("Download queue is full. Retry after current downloads finish."),
            )?;
        }
        Ok(())
    }

    pub fn resume(&self) -> Result<(), AppError> {
        for entry in self.library.entries()? {
            if entry.download_state == "pending"
                && (entry.favourite || self.library.is_playlist_track(entry.id)?)
            {
                self.enqueue(entry.id)?;
            }
        }
        Ok(())
    }

    fn download(
        &self,
        id: i64,
        client: &SoundCloudClient,
        cancellation: CancellationToken,
    ) -> Result<(), AppError> {
        let entry = self.library.entry(id)?;
        if !entry.favourite && !self.library.is_playlist_track(id)? {
            return Ok(());
        }
        self.library.state(id, "downloading", None)?;
        let settings = self.library.settings()?;
        let folder = PathBuf::from(settings.music_folder);
        fs::create_dir_all(&folder).map_err(|_| storage_error())?;
        let staging = tempfile::Builder::new()
            .prefix(".fuzzy-download-")
            .tempdir_in(&folder)
            .map_err(|_| storage_error())?;
        let options = RequestOptions {
            timeout: Duration::from_secs(600),
            cancellation: cancellation.clone(),
        };
        let audio = client
            .download_audio(
                &entry.track.source_url,
                staging.path(),
                settings.download_quality,
                &options,
            )
            .map_err(AppError::from)?;
        if cancellation.is_cancelled() {
            return Err(AppError {
                code: ErrorCode::Cancelled,
                message: "Download cancelled.".into(),
                retryable: false,
            });
        }
        let title: String = entry
            .track
            .title
            .chars()
            .filter(|c| !c.is_control())
            .map(|c| if "<>:\"/\\|?*".contains(c) { '_' } else { c })
            .take(50)
            .collect();
        let extension = audio
            .extension()
            .and_then(|v| v.to_str())
            .filter(|v| v.chars().all(|c| c.is_ascii_alphanumeric()))
            .ok_or_else(storage_error)?;
        let filename = format!(
            "{} [fuzzy-{id}].{extension}",
            title.trim_matches([' ', '.'])
        );
        let destination = folder.join(filename);
        let mut file = tempfile::NamedTempFile::new_in(&folder).map_err(|_| storage_error())?;
        std::io::copy(
            &mut fs::File::open(audio).map_err(|_| storage_error())?,
            &mut file,
        )
        .map_err(|_| storage_error())?;
        file.as_file().sync_all().map_err(|_| storage_error())?;
        if cancellation.is_cancelled() {
            return Err(AppError {
                code: ErrorCode::Cancelled,
                message: "Download cancelled.".into(),
                retryable: false,
            });
        }
        // Prevent a concurrent folder scan from indexing the new file before
        // its SoundCloud record receives the path.
        let connection = self
            .library
            .connection
            .lock()
            .map_err(|_| storage_error())?;
        // Atomic publication without replacing a user's existing file.
        file.persist_noclobber(&destination)
            .map_err(|_| storage_error())?;
        let destination = destination.canonicalize().map_err(|_| storage_error())?;
        connection
            .execute(
                "UPDATE media SET path=?1,download_state='ready',error=NULL WHERE id=?2",
                params![destination.to_string_lossy(), id],
            )
            .map_err(|_| storage_error())?;
        Ok(())
    }

    fn take_cancelled(&self, id: i64) -> bool {
        self.cancelled
            .lock()
            .map(|mut set| set.remove(&id))
            .unwrap_or(false)
    }

    pub fn shutdown(&self) {
        self.closing.store(true, Ordering::Release);
        for token in self
            .tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
        {
            token.cancel();
        }
        self.sender.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(worker) = self.worker.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{process::Command, time::Instant};

    fn track(id: &str) -> Track {
        Track {
            id: id.into(),
            title: "Test / track".into(),
            author: "Artist".into(),
            artwork_url: None,
            duration_ms: None,
            stream_url: Some("https://secret.example/temporary".into()),
            source: Source::Soundcloud,
            source_url: format!("https://soundcloud.com/artist/{id}"),
        }
    }

    fn fixture_executable(root: &Path) -> PathBuf {
        let executable = root.join(format!("tool{}", std::env::consts::EXE_SUFFIX));
        let output = Command::new("rustc")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/tool.rs"
            ))
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        executable
    }

    fn wait_for(service: &LibraryService, id: i64, state: &str) -> LibraryEntry {
        let start = Instant::now();
        loop {
            let entry = service.library.entry(id).unwrap();
            if entry.download_state == state {
                return entry;
            }
            assert!(
                start.elapsed() < Duration::from_secs(8),
                "expected {state}, got {}",
                entry.download_state
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn settings_and_favourites_persist_without_temporary_urls() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        fs::create_dir(&music).unwrap();
        let library = Library::open(dir.path(), &music).unwrap();
        let id = library.favourite(track("one"), true).unwrap();
        assert_eq!(id, library.favourite(track("one"), true).unwrap());
        assert!(
            library
                .update_settings(SettingsPatch {
                    volume: Some(101.0),
                    ..Default::default()
                })
                .is_err()
        );
        library
            .update_settings(SettingsPatch {
                volume: Some(27.0),
                download_quality: Some(DownloadQuality::Standard),
                theme: Some(Theme::Light),
                ..Default::default()
            })
            .unwrap();
        drop(library);
        let library = Library::open(dir.path(), &music).unwrap();
        assert_eq!(library.settings().unwrap().volume, 27.0);
        assert_eq!(library.settings().unwrap().theme, Theme::Light);
        assert_eq!(
            library.settings().unwrap().download_quality,
            DownloadQuality::Standard
        );
        let entries = library.entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].favourite);
        assert_eq!(entries[0].track.stream_url, None);
        let raw = fs::read(dir.path().join("library.sqlite")).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("secret.example"));
        assert!(
            library
                .favourite(
                    Track {
                        source_url: "https://soundcloud.com.evil.test/x".into(),
                        ..track("bad")
                    },
                    true
                )
                .is_err()
        );
        library.favourite(track("one"), false).unwrap();
        assert!(!library.entry(id).unwrap().favourite);
    }

    #[test]
    fn upgrades_version_one_settings_without_losing_data() {
        let dir = tempfile::tempdir().unwrap();
        let connection = Connection::open(dir.path().join("library.sqlite")).unwrap();
        connection.execute_batch(
            "CREATE TABLE settings(id INTEGER PRIMARY KEY, volume REAL NOT NULL, download_quality TEXT NOT NULL, music_folder TEXT NOT NULL);
             INSERT INTO settings VALUES(1,42,'\"small\"','existing');
             PRAGMA user_version=1;",
        ).unwrap();
        drop(connection);
        let library = Library::open(dir.path(), dir.path()).unwrap();
        let settings = library.settings().unwrap();
        assert_eq!(settings.volume, 42.0);
        assert_eq!(settings.download_quality, DownloadQuality::Small);
        assert_eq!(settings.music_folder, "existing");
        assert_eq!(settings.theme, Theme::Dark);
    }

    #[test]
    fn downloads_once_then_unlike_keeps_audio_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        fs::create_dir(&music).unwrap();
        let executable = fixture_executable(dir.path());
        let service = LibraryService::start(
            Library::open(dir.path(), &music).unwrap(),
            executable,
            NetworkPolicy::default(),
        );
        service.set_favourite(track("one"), true).unwrap();
        let id = service.library.entries().unwrap()[0].id;
        let entry = wait_for(&service, id, "ready");
        assert!(entry.available_offline);
        let path = service.library.path(id).unwrap();
        assert!(path.starts_with(music.canonicalize().unwrap()));
        assert!(
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .contains("Test _ track")
        );
        service.set_favourite(track("one"), false).unwrap();
        assert!(path.exists());
        service.set_favourite(track("one"), true).unwrap();
        assert_eq!(service.library.entries().unwrap().len(), 1);
        service.shutdown();
        drop(service);
        let reopened = Library::open(dir.path(), &music).unwrap();
        assert!(reopened.entry(id).unwrap().available_offline);
        assert_eq!(reopened.path(id).unwrap(), path);
        fs::remove_file(path).unwrap();
        assert_eq!(reopened.entry(id).unwrap().download_state, "missing");
    }

    #[test]
    fn bad_download_is_safe_and_rapid_relike_restarts() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        fs::create_dir(&music).unwrap();
        let executable = fixture_executable(dir.path());
        let service = LibraryService::start(
            Library::open(dir.path(), &music).unwrap(),
            executable,
            NetworkPolicy::default(),
        );
        service.set_favourite(track("fail"), true).unwrap();
        let id = service.library.entries().unwrap()[0].id;
        let failed = wait_for(&service, id, "error");
        assert!(!failed.error.unwrap().contains("SECRET"));
        service.set_favourite(track("slow"), true).unwrap();
        let slow = service
            .library
            .entries()
            .unwrap()
            .into_iter()
            .find(|e| e.track.id == "slow")
            .unwrap()
            .id;
        wait_for(&service, slow, "downloading");
        service.set_favourite(track("slow"), false).unwrap();
        service.set_favourite(track("slow"), true).unwrap();
        assert!(wait_for(&service, slow, "ready").available_offline);
        service.shutdown();
    }

    fn playlist() -> Playlist {
        Playlist {
            id: "mix-1".into(),
            title: "Mix".into(),
            author: "Artist".into(),
            artwork_url: None,
            track_count: Some(2),
            duration_ms: None,
            source: Source::Soundcloud,
            source_url: "https://soundcloud.com/artist/sets/mix-1".into(),
        }
    }

    #[test]
    fn playlists_are_bookmarked_with_optional_offline_download() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        fs::create_dir(&music).unwrap();
        let executable = fixture_executable(dir.path());
        let service = LibraryService::start(
            Library::open(dir.path(), &music).unwrap(),
            executable,
            NetworkPolicy::default(),
        );
        service.set_playlist_favourite(playlist(), true).unwrap();
        let saved = service.library.playlists().unwrap();
        assert_eq!(saved.len(), 1);
        assert!(saved[0].favourite);
        assert_eq!(saved[0].total, 0);
        // A bookmark alone must not start downloads.
        assert!(service.library.entries().unwrap().is_empty());
        service
            .download_playlist(playlist(), vec![track("one"), track("two"), track("one")])
            .unwrap();
        let entries = service.library.entries().unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|entry| !entry.favourite));
        for entry in &entries {
            assert!(wait_for(&service, entry.id, "ready").available_offline);
        }
        let saved = service.library.playlists().unwrap();
        assert_eq!((saved[0].downloaded, saved[0].total), (2, 2));
        assert_eq!(saved[0].state, "ready");
        service.set_playlist_favourite(playlist(), false).unwrap();
        let saved = service.library.playlists().unwrap();
        assert!(!saved[0].favourite);
        assert!(
            service
                .library
                .entries()
                .unwrap()
                .iter()
                .all(|entry| entry.available_offline)
        );
        service.shutdown();
    }

    #[test]
    fn playlist_download_can_be_cancelled_without_losing_bookmark_or_favourites() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        fs::create_dir(&music).unwrap();
        let executable = fixture_executable(dir.path());
        let service = LibraryService::start(
            Library::open(dir.path(), &music).unwrap(),
            executable,
            NetworkPolicy::default(),
        );
        service.set_playlist_favourite(playlist(), true).unwrap();
        service
            .download_playlist(playlist(), vec![track("slow_block_one"), track("two")])
            .unwrap();
        let entries = service.library.entries().unwrap();
        let first = entries
            .iter()
            .find(|entry| entry.track.id == "slow_block_one")
            .unwrap()
            .id;
        let second = entries
            .iter()
            .find(|entry| entry.track.id == "two")
            .unwrap()
            .id;
        wait_for(&service, first, "downloading");
        service.cancel_playlist_download(&playlist()).unwrap();
        assert_eq!(wait_for(&service, first, "none").download_state, "none");
        assert_eq!(
            service.library.entry(second).unwrap().download_state,
            "none"
        );
        // The worker must not requeue cancelled ids after its download aborts.
        let deadline = Instant::now() + Duration::from_secs(4);
        while Instant::now() < deadline {
            assert_eq!(service.library.entry(first).unwrap().download_state, "none");
            assert_eq!(
                service.library.entry(second).unwrap().download_state,
                "none"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
        // Bookmark survives, tracks stay outside favourites and nothing is saved.
        let saved = service.library.playlists().unwrap();
        assert!(saved[0].favourite);
        assert_eq!(saved[0].state, "idle");
        assert_eq!(saved[0].downloaded, 0);
        assert!(
            service
                .library
                .entries()
                .unwrap()
                .iter()
                .all(|entry| !entry.favourite && !entry.available_offline)
        );
        service.shutdown();
    }

    #[test]
    fn scans_music_and_preserves_existing_files_when_folder_changes() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music");
        fs::create_dir(&music).unwrap();
        let data = dir.path().join("data");
        let audio = music.join("Local track.wav");
        let mut wav = b"RIFF\x26\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x01\x00\x40\x1f\x00\x00\x80\x3e\x00\x00\x02\x00\x10\x00data\x02\x00\x00\x00".to_vec();
        wav.extend_from_slice(&[0, 0]);
        fs::write(&audio, wav).unwrap();
        let library = Library::open(&data, &music).unwrap();
        assert_eq!(library.scan().unwrap().imported, 1);
        assert_eq!(library.scan().unwrap().imported, 0);
        let entry = library.entries().unwrap().remove(0);
        assert_eq!(entry.track.source, Source::Local);
        assert_eq!(entry.track.title, "Local track");
        assert!(entry.available_offline);
        library.favourite(entry.track.clone(), true).unwrap();
        assert!(library.entry(entry.id).unwrap().favourite);
        let other = dir.path().join("other");
        fs::create_dir(&other).unwrap();
        library
            .update_settings(SettingsPatch {
                music_folder: Some(other.to_string_lossy().into()),
                ..Default::default()
            })
            .unwrap();
        library.scan().unwrap();
        assert!(audio.exists());
        assert!(library.entry(entry.id).unwrap().available_offline);
    }
}
