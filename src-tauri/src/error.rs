use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("No file loaded")]
    NoFileLoaded,
    #[error("Seek out of range: {0} ms")]
    SeekOutOfRange(u64),
    #[error("Marker not found: {0}")]
    MarkerNotFound(Uuid),
    #[error("Validation failed: {0}")]
    ValidationError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Decode error: {0}")]
    Decode(String),
    #[error("Audio output error: {0}")]
    AudioOutput(String),
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("Dialog cancelled")]
    DialogCancelled,
    #[error("ffmpeg sidecar not available: {0}")]
    FfmpegNotFound(String),
    #[error("ffmpeg failed on segment '{0}': {1}")]
    FfmpegFailed(String, String),
}

// Tauri requires command errors to be serializable so they can be sent over IPC.
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(
        &self,
        s: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_variants_serialize_to_display_string() {
        let uuid = uuid::Uuid::new_v4();
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let csv_err = csv::Error::from(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe"));

        let cases: Vec<AppError> = vec![
            AppError::NoFileLoaded,
            AppError::SeekOutOfRange(500),
            AppError::MarkerNotFound(uuid),
            AppError::ValidationError("bad input".into()),
            AppError::Io(io_err),
            AppError::Decode("corrupt".into()),
            AppError::AudioOutput("no device".into()),
            AppError::Csv(csv_err),
            AppError::DialogCancelled,
            AppError::FfmpegNotFound("/usr/bin/ffmpeg".into()),
            AppError::FfmpegFailed("seg1".into(), "exit code 1".into()),
        ];

        for e in cases {
            let display = format!("{}", e);
            let serialized = serde_json::to_string(&e).unwrap();
            assert_eq!(serialized, format!("\"{}\"", display));
        }
    }
}
