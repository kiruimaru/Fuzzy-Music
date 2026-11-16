use fuzzy_soundcloud::SoundCloudClient;
use std::{env, path::PathBuf, process::ExitCode, time::Instant};

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let action = args.next().ok_or("Usage: soundcloud-probe search QUERY [--limit 5] [--yt-dlp PATH] | resolve URL [--yt-dlp PATH]")?;
    let input = args
        .next()
        .ok_or("Provide a search query or a SoundCloud track URL")?;
    let mut limit = 5;
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
            _ => return Err(format!("Unknown option: {flag}")),
        }
    }
    let client = SoundCloudClient::new(executable);
    let start = Instant::now();
    let result = match action.as_str() {
        "search" => serde_json::to_value(client.search(&input, limit)?),
        "resolve" => serde_json::to_value(client.resolve(&input)?),
        _ => return Err("Unknown command; use search or resolve".into()),
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
