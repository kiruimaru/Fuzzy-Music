use std::{fmt, time::Duration};

#[derive(Debug)]
pub enum SoundCloudError {
    InvalidInput(String),
    Cancelled,
    TimedOut { timeout: Duration },
    Process(String),
    ExtractorFailed { status: String, details: String },
    InvalidResponse(String),
}

impl fmt::Display for SoundCloudError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message)
            | Self::Process(message)
            | Self::InvalidResponse(message) => f.write_str(message),
            Self::Cancelled => f.write_str("SoundCloud request cancelled"),
            Self::TimedOut { timeout } => write!(
                f,
                "SoundCloud request timed out after {:.2}s",
                timeout.as_secs_f64()
            ),
            Self::ExtractorFailed { status, details } => {
                write!(f, "SoundCloud extractor failed ({status}): {details}")
            }
        }
    }
}

impl std::error::Error for SoundCloudError {}
