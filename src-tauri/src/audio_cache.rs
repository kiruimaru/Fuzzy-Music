//! Bounded HLS startup segments, kept only in memory and served on loopback.
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use url::Url;

const MAX_SEGMENT: u64 = 2 * 1024 * 1024;
const SEGMENTS: usize = 12;
const INITIAL_SEGMENTS: usize = 3;
const MAX_TRACK_BYTES: usize = 3 * 1024 * 1024;
const MAX_TRACKS: usize = 41; // Current track plus forty neighbours.
// SoundCloud signs stream links for roughly two hours; a 55-minute cap keeps
// entries comfortably valid at use time while halving refresh storms.
const MAX_CACHE_AGE: Duration = Duration::from_secs(55 * 60);
struct Entry {
    key: String,
    valid_until: Instant,
    files: HashMap<String, Arc<Vec<u8>>>,
}
struct State {
    entries: Vec<Entry>,
}
pub struct AudioCache {
    state: Arc<Mutex<State>>,
    base: String,
    agent: ureq::Agent,
    sequence: AtomicU64,
    epoch: AtomicU64,
}
impl AudioCache {
    pub fn new(network: &fuzzy_application::NetworkPolicy) -> Option<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").ok()?;
        let port = listener.local_addr().ok()?.port();
        let state = Arc::new(Mutex::new(State {
            entries: Vec::new(),
        }));
        let server_state = state.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let state = server_state.clone();
                std::thread::spawn(move || serve(stream, state));
            }
        });
        let proxy = network.https_proxy().ok()?;
        let agent = ureq::Agent::config_builder()
            .proxy(proxy)
            .timeout_global(Some(Duration::from_secs(10)))
            .build()
            .into();
        Some(Self {
            state,
            base: format!("http://127.0.0.1:{port}"),
            agent,
            sequence: AtomicU64::new(0),
            epoch: AtomicU64::new(0),
        })
    }
    pub fn clear(&self) {
        if let Ok(mut state) = self.state.lock() {
            self.epoch.fetch_add(1, Ordering::SeqCst);
            state.entries.clear();
        }
    }
    pub fn get(&self, key: &str) -> Option<String> {
        let mut state = self.state.lock().ok()?;
        // Expired entries cannot become valid again; drop them on lookup.
        state.entries.retain(|e| Instant::now() < e.valid_until);
        let entry = state.entries.iter().find(|e| e.key == key)?;
        let name = entry.files.keys().find(|s| s.ends_with("/index.m3u8"))?;
        Some(format!("{}{name}", self.base))
    }
    #[cfg(test)]
    pub fn warm(&self, key: &str, stream: &str) {
        self.warm_at(key, stream, self.epoch());
    }
    pub fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::SeqCst)
    }
    pub fn warm_at(&self, key: &str, stream: &str, epoch: u64) {
        self.warm_inner(key, stream, epoch, None);
    }
    pub fn warm_cancellable(
        &self,
        key: &str,
        stream: &str,
        cancellation: &fuzzy_application::CancellationToken,
    ) {
        self.warm_inner(key, stream, self.epoch(), Some(cancellation));
    }
    fn warm_inner(
        &self,
        key: &str,
        stream: &str,
        epoch: u64,
        cancellation: Option<&fuzzy_application::CancellationToken>,
    ) {
        if epoch != self.epoch() {
            return;
        }
        let Ok(url) = Url::parse(stream) else { return };
        if !allowed_url(&url) || self.get(key).is_some() {
            return;
        }
        let Ok(mut response) = self.agent.get(stream).call() else {
            return;
        };
        let Ok(manifest) = response
            .body_mut()
            .with_config()
            .limit(1024 * 1024)
            .read_to_string()
        else {
            return;
        };
        let mut valid_until = Instant::now() + url_lifetime(&url, Duration::from_secs(50));
        // Master playlists, encrypted streams and byte ranges need a different proxy.
        if !manifest.starts_with("#EXTM3U")
            || manifest.contains("#EXT-X-STREAM-INF")
            || manifest
                .lines()
                .any(|line| line.contains("URI=") && !line.starts_with("#EXT-X-MAP:"))
            || manifest.contains("#EXT-X-BYTERANGE")
            || manifest.contains("BYTERANGE=")
            || manifest
                .lines()
                .filter(|line| line.starts_with("#EXT-X-MAP:"))
                .count()
                > 1
        {
            return;
        }
        let token = format!(
            "{:x}{:x}",
            std::process::id(),
            self.sequence.fetch_add(1, Ordering::Relaxed)
        );
        let mut files = HashMap::new();
        let mut output = String::new();
        let mut count = 0;
        let lines: Vec<_> = manifest.lines().collect();
        let mut bytes_cached = 0;
        let mut published = false;
        for (index, line) in lines.iter().copied().enumerate() {
            if let Some(attributes) = line.strip_prefix("#EXT-X-MAP:") {
                let Some(start) = attributes.find("URI=\"") else {
                    return;
                };
                let value_start = start + 5;
                let Some(end) = attributes[value_start..].find('"') else {
                    return;
                };
                let value_end = value_start + end;
                let Ok(init) = url.join(&attributes[value_start..value_end]) else {
                    return;
                };
                if !allowed_url(&init) {
                    return;
                }
                valid_until =
                    valid_until.min(Instant::now() + url_lifetime(&init, Duration::from_secs(50)));
                let Ok(mut response) = self.agent.get(init.as_str()).call() else {
                    return;
                };
                let Ok(bytes) = response
                    .body_mut()
                    .with_config()
                    .limit(MAX_SEGMENT)
                    .read_to_vec()
                else {
                    return;
                };
                if bytes.is_empty() {
                    return;
                }
                bytes_cached += bytes.len();
                let path = format!("/{token}/init");
                files.insert(path.clone(), Arc::new(bytes));
                output.push_str("#EXT-X-MAP:");
                output.push_str(&attributes[..value_start]);
                output.push_str(&self.base);
                output.push_str(&path);
                output.push_str(&attributes[value_end..]);
            } else if line.is_empty() || line.starts_with('#') {
                output.push_str(line);
            } else {
                let Ok(segment) = url.join(line.trim()) else {
                    return;
                };
                if !allowed_url(&segment) {
                    return;
                }
                valid_until = valid_until
                    .min(Instant::now() + url_lifetime(&segment, Duration::from_secs(50)));
                if count < SEGMENTS && bytes_cached < MAX_TRACK_BYTES {
                    let Ok(mut response) = self.agent.get(segment.as_str()).call() else {
                        return;
                    };
                    let Ok(bytes) = response
                        .body_mut()
                        .with_config()
                        .limit(MAX_SEGMENT)
                        .read_to_vec()
                    else {
                        return;
                    };
                    if bytes.is_empty() {
                        return;
                    }
                    if bytes_cached + bytes.len() <= MAX_TRACK_BYTES {
                        bytes_cached += bytes.len();
                        let path = format!("/{token}/{count}");
                        output.push_str(&format!("{}{path}", self.base));
                        files.insert(path, Arc::new(bytes));
                        count += 1;
                    } else {
                        bytes_cached = MAX_TRACK_BYTES;
                        output.push_str(segment.as_str());
                    }
                } else {
                    output.push_str(segment.as_str());
                }
            }
            output.push('\n');
            if count == INITIAL_SEGMENTS && !published {
                let mut partial = output.clone();
                for remaining in &lines[index + 1..] {
                    if remaining.is_empty() || remaining.starts_with('#') {
                        partial.push_str(remaining);
                    } else if let Ok(segment) = url.join(remaining.trim()) {
                        if !allowed_url(&segment) {
                            return;
                        }
                        valid_until = valid_until
                            .min(Instant::now() + url_lifetime(&segment, Duration::from_secs(50)));
                        partial.push_str(segment.as_str());
                    } else {
                        return;
                    }
                    partial.push('\n');
                }
                let mut initial = files.clone();
                initial.insert(
                    format!("/{token}/index.m3u8"),
                    Arc::new(partial.into_bytes()),
                );
                self.publish(key, epoch, initial, valid_until);
                published = true;
            }
            if epoch != self.epoch() || cancellation.is_some_and(|token| token.is_cancelled()) {
                return;
            }
        }
        if count == 0 {
            return;
        }
        files.insert(
            format!("/{token}/index.m3u8"),
            Arc::new(output.into_bytes()),
        );
        self.publish(key, epoch, files, valid_until);
    }
    fn publish(
        &self,
        key: &str,
        epoch: u64,
        files: HashMap<String, Arc<Vec<u8>>>,
        valid_until: Instant,
    ) {
        if let Ok(mut state) = self.state.lock() {
            if epoch != self.epoch.load(Ordering::SeqCst) {
                return;
            }
            state.entries.retain(|e| e.key != key);
            if state.entries.len() >= MAX_TRACKS {
                state.entries.remove(0);
            }
            state.entries.push(Entry {
                key: key.to_owned(),
                valid_until,
                files,
            });
        }
    }
}
fn url_lifetime(url: &Url, fallback: Duration) -> Duration {
    let expires = url
        .query_pairs()
        .find(|(key, _)| key.eq_ignore_ascii_case("expires"))
        .and_then(|(_, value)| value.parse::<u64>().ok());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs());
    match (expires, now) {
        (Some(expires), Some(now)) => {
            Duration::from_secs(expires.saturating_sub(now).saturating_sub(120)).min(MAX_CACHE_AGE)
        }
        _ => fallback,
    }
}
pub fn link_is_fresh(stream: &str, resolved_at: Instant, fallback: Duration) -> bool {
    let Ok(url) = Url::parse(stream) else {
        return false;
    };
    let elapsed = resolved_at.elapsed();
    elapsed < MAX_CACHE_AGE
        && if url
            .query_pairs()
            .any(|(key, value)| key.eq_ignore_ascii_case("expires") && value.parse::<u64>().is_ok())
        {
            !url_lifetime(&url, fallback).is_zero()
        } else {
            elapsed < fallback
        }
}
fn allowed_url(url: &Url) -> bool {
    url.scheme() == "https"
        || (cfg!(test) && url.scheme() == "http" && url.host_str() == Some("127.0.0.1"))
}
fn serve(mut stream: TcpStream, state: Arc<Mutex<State>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut request = [0u8; 8192];
    let Ok(n) = stream.read(&mut request) else {
        return;
    };
    let Ok(text) = std::str::from_utf8(&request[..n]) else {
        return;
    };
    let mut first = text.lines().next().unwrap_or("").split_whitespace();
    if first.next() != Some("GET") {
        return;
    }
    let path = first.next().unwrap_or("");
    let data = state
        .lock()
        .ok()
        .and_then(|s| s.entries.iter().find_map(|e| e.files.get(path).cloned()));
    if let Some(data) = data {
        let kind = if path.ends_with(".m3u8") {
            "application/vnd.apple.mpegurl"
        } else {
            "application/octet-stream"
        };
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            data.len()
        );
        let _ = stream.write_all(&data);
    } else {
        let _ = stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_links_are_reused_with_a_safety_margin_and_unsigned_links_expire() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let signed = format!("https://audio.example/test?expires={}", now + 3600);
        let expiring = format!("https://audio.example/test?expires={}", now + 60);
        let old = Instant::now() - Duration::from_secs(70);
        assert_eq!(
            url_lifetime(&Url::parse(&signed).unwrap(), Duration::from_secs(50)),
            MAX_CACHE_AGE
        );
        assert!(link_is_fresh(&signed, old, Duration::from_secs(60)));
        assert!(!link_is_fresh(
            &expiring,
            Instant::now(),
            Duration::from_secs(60)
        ));
        assert!(!link_is_fresh(
            "https://audio.example/test",
            old,
            Duration::from_secs(60)
        ));
        assert!(link_is_fresh(
            &signed,
            Instant::now() - Duration::from_secs(3180),
            Duration::from_secs(60)
        ));
        assert!(!link_is_fresh(
            &signed,
            Instant::now() - Duration::from_secs(3361),
            Duration::from_secs(60)
        ));
    }
    #[test]
    #[ignore = "Requires network and FUZZY_YTDLP"]
    fn live_soundcloud_startup_cache() {
        let network = fuzzy_application::NetworkPolicy::detect();
        let client =
            fuzzy_soundcloud::SoundCloudClient::new(std::env::var_os("FUZZY_YTDLP").unwrap())
                .with_network(network.clone());
        let stream = client
            .resolve_with_options(
                "https://soundcloud.com/hephestosmusic/super-mario-world-game-over",
                &fuzzy_soundcloud::RequestOptions::default(),
            )
            .unwrap();
        let cache = AudioCache::new(&network).unwrap();
        cache.warm("test", &stream.url);
        assert!(cache.get("test").is_some(), "stream was not cached");
        if let Some(mpv) = std::env::var_os("FUZZY_MPV") {
            let mut command = std::process::Command::new(mpv);
            network.apply_with_loopback_bypass(&mut command);
            let status = command
                .args([
                    "--no-config",
                    "--no-video",
                    "--ao=null",
                    "--length=2",
                    "--really-quiet",
                    "--",
                ])
                .arg(cache.get("test").unwrap())
                .status()
                .unwrap();
            assert!(status.success(), "mpv could not play the cached stream");
        }
    }
    #[test]
    fn preloads_segments_and_rewrites_manifest_for_mpv() {
        let source = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = source.local_addr().unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (continue_tx, continue_rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            for (index, connection) in source.incoming().take(5).enumerate() {
                let mut connection = connection.unwrap();
                let mut request = [0u8; 2048];
                let n = connection.read(&mut request).unwrap();
                let text = std::str::from_utf8(&request[..n]).unwrap();
                if index == 4 {
                    started_tx.send(()).unwrap();
                    continue_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                }
                let data: &[u8] = if text.starts_with("GET /list") {
                    b"#EXTM3U\n#EXTINF:1,\na.ts\n#EXTINF:1,\nb.ts\n#EXTINF:1,\nc.ts\n#EXTINF:1,\nd.ts\n#EXT-X-ENDLIST\n"
                } else {
                    b"audio"
                };
                write!(
                    connection,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    data.len()
                )
                .unwrap();
                connection.write_all(data).unwrap();
            }
        });
        let cache =
            Arc::new(AudioCache::new(&fuzzy_application::NetworkPolicy::default()).unwrap());
        let warm_cache = cache.clone();
        let warmer =
            std::thread::spawn(move || warm_cache.warm("track", &format!("http://{address}/list")));
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(
            cache.get("track").is_some(),
            "initial segments should be ready before the remaining downloads"
        );
        continue_tx.send(()).unwrap();
        warmer.join().unwrap();
        server.join().unwrap();
        let agent: ureq::Agent = ureq::Agent::config_builder().proxy(None).build().into();
        let body = agent
            .get(cache.get("track").unwrap())
            .call()
            .unwrap()
            .body_mut()
            .read_to_string()
            .unwrap();
        let segments: Vec<_> = body.lines().filter(|line| !line.starts_with('#')).collect();
        assert_eq!(segments.len(), 4);
        assert!(segments.iter().all(|url| url.starts_with(&cache.base)));
        assert_eq!(
            agent
                .get(segments[0])
                .call()
                .unwrap()
                .body_mut()
                .read_to_vec()
                .unwrap(),
            b"audio"
        );
    }
    #[test]
    fn loopback_serves_cached_bytes_and_clear_invalidates_them() {
        let cache = AudioCache::new(&fuzzy_application::NetworkPolicy::default()).unwrap();
        let mut files = HashMap::new();
        files.insert("/test/index.m3u8".into(), Arc::new(b"#EXTM3U\n".to_vec()));
        files.insert("/test/0".into(), Arc::new(vec![1, 2, 3]));
        cache.state.lock().unwrap().entries.push(Entry {
            key: "track".into(),
            valid_until: Instant::now() + Duration::from_secs(50),
            files,
        });
        let url = cache.get("track").unwrap();
        let agent: ureq::Agent = ureq::Agent::config_builder().proxy(None).build().into();
        let body = agent
            .get(&url)
            .call()
            .unwrap()
            .body_mut()
            .read_to_string()
            .unwrap();
        assert_eq!(body, "#EXTM3U\n");
        let segment = agent
            .get(format!("{}/test/0", cache.base))
            .call()
            .unwrap()
            .body_mut()
            .read_to_vec()
            .unwrap();
        assert_eq!(segment, vec![1, 2, 3]);
        cache.clear();
        assert!(cache.get("track").is_none());
        assert!(agent.get(&url).call().is_err());
    }
}
