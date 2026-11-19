use crate::{RequestOptions, SoundCloudClient, SoundCloudError, is_soundcloud_url, process};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DownloadQuality {
    #[default]
    Best,
    High,
    Medium,
    Standard,
    Small,
    Flac,
}

impl SoundCloudClient {
    /// Download into an application-owned staging directory, never into memory.
    /// Lower presets prefer source formats below a bitrate ceiling; no upsampling.
    pub fn download_audio(
        &self,
        track_url: &str,
        directory: &Path,
        quality: DownloadQuality,
        options: &RequestOptions,
    ) -> Result<PathBuf, SoundCloudError> {
        if !is_soundcloud_url(track_url) || !directory.is_dir() {
            return Err(SoundCloudError::InvalidInput(
                "Invalid audio download request".into(),
            ));
        }
        let format = match quality {
            DownloadQuality::Best => "bestaudio/best",
            DownloadQuality::High => "bestaudio[abr<=320]/bestaudio/best",
            DownloadQuality::Medium => "bestaudio[abr<=192]/bestaudio/best",
            DownloadQuality::Standard => "bestaudio[abr<=128]/bestaudio/best",
            DownloadQuality::Small => "bestaudio[abr<=64]/bestaudio/best",
            DownloadQuality::Flac => "bestaudio/best",
        };
        let mut command = Command::new(&self.executable);
        if let Some(network) = &self.network {
            network.apply(&mut command);
        }
        command.args([
            "--ignore-config",
            "--no-plugin-dirs",
            "--no-cache-dir",
            "--no-playlist",
            "--no-progress",
            "--no-warnings",
            "--no-simulate",
            "--no-overwrites",
            "--socket-timeout",
            "15",
            "--retries",
            "1",
            "--fragment-retries",
            "1",
            "--extractor-retries",
            "1",
            "--concurrent-fragments",
            "1",
            "--max-filesize",
            "512M",
            "--format",
            format,
            "--print",
            "after_move:%(filepath)j",
        ]);
        if quality == DownloadQuality::Flac {
            command.args(["--extract-audio", "--audio-format", "flac"]);
        }
        command
            .arg("--output")
            .arg(directory.join("audio.%(ext)s"))
            .arg("--")
            .arg(track_url);
        let output = process::run(&mut command, options)?;
        if !output.status.success() {
            return Err(SoundCloudError::ExtractorFailed {
                status: output.status.to_string(),
                details: "Audio download failed".into(),
            });
        }
        let path: PathBuf = serde_json::from_slice::<String>(&output.stdout)
            .map(PathBuf::from)
            .map_err(|_| SoundCloudError::InvalidResponse("Missing downloaded audio".into()))?;
        let path = path
            .canonicalize()
            .map_err(|_| SoundCloudError::InvalidResponse("Missing downloaded audio".into()))?;
        let root = directory
            .canonicalize()
            .map_err(|_| SoundCloudError::InvalidResponse("Invalid download directory".into()))?;
        let metadata = path
            .metadata()
            .map_err(|_| SoundCloudError::InvalidResponse("Missing downloaded audio".into()))?;
        if !path.starts_with(root)
            || !metadata.is_file()
            || metadata.len() == 0
            || metadata.len() > 512 * 1024 * 1024
        {
            return Err(SoundCloudError::InvalidResponse(
                "Invalid downloaded audio".into(),
            ));
        }
        Ok(path)
    }
}
