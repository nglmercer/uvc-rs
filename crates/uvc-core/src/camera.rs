use std::{fmt, str::FromStr};

use crate::{EngineError, EngineResult, FrameFormat};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CameraId(String);

impl CameraId {
    pub fn new(id: impl Into<String>) -> EngineResult<Self> {
        let id = id.into().trim().to_owned();

        if id.is_empty() {
            return Err(EngineError::InvalidCameraId("<empty>".to_owned()));
        }

        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CameraId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for CameraId {
    type Err = EngineError;

    fn from_str(value: &str) -> EngineResult<Self> {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CameraConfig {
    camera_id: CameraId,
    format: FrameFormat,
    frame_count: Option<u64>,
}

impl CameraConfig {
    pub fn new(camera_id: CameraId, format: FrameFormat) -> Self {
        Self {
            camera_id,
            format,
            frame_count: None,
        }
    }

    pub fn with_frame_count(mut self, frame_count: u64) -> Self {
        self.frame_count = Some(frame_count);
        self
    }

    pub fn camera_id(&self) -> &CameraId {
        &self.camera_id
    }

    pub fn format(&self) -> FrameFormat {
        self.format
    }

    pub fn frame_count(&self) -> Option<u64> {
        self.frame_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_config_tracks_optional_frame_count() {
        let format = FrameFormat::yuyv(640, 480, 30).unwrap();
        let config =
            CameraConfig::new(CameraId::new("cam-1").unwrap(), format).with_frame_count(10);

        assert_eq!(config.camera_id().as_str(), "cam-1");
        assert_eq!(config.frame_count(), Some(10));
    }
}
