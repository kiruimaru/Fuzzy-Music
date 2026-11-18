use fuzzy_soundcloud::{RequestOptions, SoundCloudClient};
use std::{
    env,
    path::PathBuf,
    process::ExitCode,
    thread,
    time::{Duration, Instant},
};

fn seconds(value: Option<String>) -> Result<Duration, String> {
    let seconds: f64 = value
        .ok_or("Missing duration")?
        .parse()
        .map_err(|_| "Duration must be a number")?;
    let duration =
        Duration::try_from_secs_f64(seconds).map_err(|_| "Duration must be finite and positive")?;
    if duration.is_zero() {
        return Err("Duration must be positive".into());
    }
    Ok(duration)
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let action = args.next().ok_or("Usage: soundcloud-probe search QUERY [--limit 5] [--yt-dlp PATH] | resolve URL [--yt-dlp PATH]")?;
    let input = args
        .next()
        .ok_or("Provide a search query or a SoundCloud track URL")?;
    let mut limit = 5;
    let mut options = RequestOptions::default();
    let mut cancel_after = None;
    let mut executable = env::var_os("FUZZY_YTDLP")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("yt-dlp"));
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--limit" => {
                limit = args
                    .next()
                    .ok_or("Missing --limit value")?
                    .parse()
                    .map_err(|_| "Limit must be a positive integer")?
            }
            "--yt-dlp" => executable = args.next().ok_or("Missing --yt-dlp path")?.into(),
            "--timeout" => options.timeout = seconds(args.next())?,
            "--cancel-after" => cancel_after = Some(seconds(args.next())?),
            _ => return Err(format!("Unknown option: {flag}")),
        }
    }
    let client = SoundCloudClient::new(executable);
    if let Some(delay) = cancel_after {
        let cancellation = options.cancellation.clone();
        thread::spawn(move || {
            thread::sleep(delay);
            cancellation.cancel();
        });
    }
    let start = Instant::now();
    let result = match action.as_str() {
        "search" => serde_json::to_value(
            client
                .search_with_options(&input, limit, &options)
                .map_err(|e| e.to_string())?,
        ),
        "resolve" => serde_json::to_value(
            client
                .resolve_with_options(&input, &options)
                .map_err(|e| e.to_string())?,
        ),
        "search-web" => serde_json::to_value(
            client
                .search_web_with_options(&input, limit, &options)
                .map_err(|e| e.to_string())?,
        ),
        "playlists" => serde_json::to_value(
            client
                .search_playlists_with_options(&input, limit, &options)
                .map_err(|e| e.to_string())?,
        ),
        "playlist" => serde_json::to_value(
            client
                .playlist_tracks_with_options(&input, &options)
                .map_err(|e| e.to_string())?,
        ),
        _ => return Err("Unknown command; use search, playlists, playlist or resolve".into()),
    }
    .map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
    );
    eprintln!("Completed in {:.2}s", start.elapsed().as_secs_f64());
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
