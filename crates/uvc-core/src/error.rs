use thiserror::Error;

pub type EngineResult<T> = Result<T, EngineError>;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("invalid camera id `{0}`")]
    InvalidCameraId(String),

    #[error("invalid frame format: {0}")]
    InvalidFrameFormat(String),

    #[error("frame size `{actual}` does not match expected size `{expected}` for `{format}`")]
    InvalidFrameSize {
        format: String,
        actual: usize,
        expected: usize,
    },

    #[error("frame sink is closed")]
    SinkClosed,

    #[error("pipeline `{0}` is already running")]
    AlreadyRunning(String),

    #[error("pipeline `{0}` is not running")]
    NotRunning(String),

    #[error("operation timed out")]
    Timeout,

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("backend error: {0}")]
    Backend(String),
}
