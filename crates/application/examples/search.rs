use fuzzy_application::{Application, RequestOptions, SearchTracksRequest};
use fuzzy_soundcloud::SoundCloudClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let executable = std::env::args()
        .nth(1)
        .ok_or("Expected yt-dlp executable path")?;
    let app = Application::new(SoundCloudClient::new(executable));
    let options = RequestOptions::default();
    // Retain a clone on the caller to cancel this particular operation.
    let cancellation = options.cancellation.clone();
    let worker = std::thread::spawn(move || {
        app.search_tracks(
            SearchTracksRequest {
                query: "lofi".into(),
                limit: Some(3),
            },
            &options,
        )
    });
    // Joining is appropriate in this CLI only; a UI must await its blocking worker.
    let response = worker.join().map_err(|_| "Search worker failed")??;
    println!("{} tracks", response.tracks.len());
    drop(cancellation);
    Ok(())
}
