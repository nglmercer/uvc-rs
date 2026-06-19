use std::{fmt, str::FromStr};

use crate::{EngineError, EngineResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PixelFormat {
    Mjpeg,
    Yuyv,
    H264,
    Nv12,
    Rgba,
}

impl PixelFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mjpeg => "mjpeg",
            Self::Yuyv => "yuyv",
            Self::H264 => "h264",
            Self::Nv12 => "nv12",
            Self::Rgba => "rgba",
        }
    }
}

impl fmt::Display for PixelFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PixelFormat {
    type Err = EngineError;

    fn from_str(value: &str) -> EngineResult<Self> {
        match value.to_ascii_lowercase().as_str() {
            "mjpeg" | "mjpe" => Ok(Self::Mjpeg),
            "yuyv" | "yuy2" => Ok(Self::Yuyv),
            "h264" | "h265" => Ok(Self::H264),
            "nv12" => Ok(Self::Nv12),
            "rgba" => Ok(Self::Rgba),
            _ => Err(EngineError::InvalidFrameFormat(format!(
                "unknown pixel format `{value}`"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct FrameFormat {
    pixel_format: PixelFormat,
    width: u32,
    height: u32,
    fps: u32,
}

impl FrameFormat {
    pub fn new(pixel_format: PixelFormat, width: u32, height: u32, fps: u32) -> EngineResult<Self> {
        if width == 0 {
            return Err(EngineError::InvalidFrameFormat(
                "width must be greater than zero".to_owned(),
            ));
        }

        if height == 0 {
            return Err(EngineError::InvalidFrameFormat(
                "height must be greater than zero".to_owned(),
            ));
        }

        if fps == 0 {
            return Err(EngineError::InvalidFrameFormat(
                "fps must be greater than zero".to_owned(),
            ));
        }

        Ok(Self {
            pixel_format,
            width,
            height,
            fps,
        })
    }

    pub fn mjpeg(width: u32, height: u32, fps: u32) -> EngineResult<Self> {
        Self::new(PixelFormat::Mjpeg, width, height, fps)
    }

    pub fn yuyv(width: u32, height: u32, fps: u32) -> EngineResult<Self> {
        Self::new(PixelFormat::Yuyv, width, height, fps)
    }

    pub fn h264(width: u32, height: u32, fps: u32) -> EngineResult<Self> {
        Self::new(PixelFormat::H264, width, height, fps)
    }

    pub fn nv12(width: u32, height: u32, fps: u32) -> EngineResult<Self> {
        Self::new(PixelFormat::Nv12, width, height, fps)
    }

    pub fn rgba(width: u32, height: u32, fps: u32) -> EngineResult<Self> {
        Self::new(PixelFormat::Rgba, width, height, fps)
    }

    pub fn pixel_format(self) -> PixelFormat {
        self.pixel_format
    }

    pub fn width(self) -> u32 {
        self.width
    }

    pub fn height(self) -> u32 {
        self.height
    }

    pub fn fps(self) -> u32 {
        self.fps
    }

    pub fn expected_len(self) -> Option<usize> {
        match self.pixel_format {
            PixelFormat::Mjpeg | PixelFormat::H264 => None,
            PixelFormat::Yuyv => self.checked_pixel_len(2),
            PixelFormat::Nv12 => self.checked_pixel_len(3).map(|len| len / 2),
            PixelFormat::Rgba => self.checked_pixel_len(4),
        }
    }

    pub fn validate_frame_len(self, len: usize) -> EngineResult<()> {
        match self.expected_len() {
            Some(expected) if len != expected => Err(EngineError::InvalidFrameSize {
                format: self.to_string(),
                actual: len,
                expected,
            }),
            _ => Ok(()),
        }
    }

    fn checked_pixel_len(self, bytes_per_two_pixels: u32) -> Option<usize> {
        let pixels = self.width.checked_mul(self.height)?;
        let bytes = pixels.checked_mul(bytes_per_two_pixels)?;
        usize::try_from(bytes).ok()
    }
}

impl fmt::Display for FrameFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}@{}x{}:{}",
            self.pixel_format, self.width, self.height, self.fps
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_format_parses_common_names() {
        assert_eq!("YUY2".parse::<PixelFormat>().unwrap(), PixelFormat::Yuyv);
        assert_eq!("MJPE".parse::<PixelFormat>().unwrap(), PixelFormat::Mjpeg);
        assert!("unknown".parse::<PixelFormat>().is_err());
    }

    #[test]
    fn expected_len_matches_raw_formats() {
        let yuyv = FrameFormat::yuyv(640, 480, 30).unwrap();
        let nv12 = FrameFormat::nv12(640, 480, 30).unwrap();
        let rgba = FrameFormat::rgba(640, 480, 30).unwrap();
        let mjpeg = FrameFormat::mjpeg(640, 480, 30).unwrap();

        assert_eq!(yuyv.expected_len(), Some(640 * 480 * 2));
        assert_eq!(nv12.expected_len(), Some(640 * 480 * 3 / 2));
        assert_eq!(rgba.expected_len(), Some(640 * 480 * 4));
        assert_eq!(mjpeg.expected_len(), None);
    }
}
