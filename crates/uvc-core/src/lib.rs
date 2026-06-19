pub mod camera;
pub mod error;
pub mod format;
pub mod frame;
pub mod pipeline;

pub use camera::{CameraConfig, CameraId};
pub use error::{EngineError, EngineResult};
pub use format::{FrameFormat, PixelFormat};
pub use frame::{Frame, FrameBuffer, FrameReceiver, FrameSender, FrameSink, frame_channel};
pub use pipeline::CameraPipeline;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn camera_id_rejects_empty_values() {
        assert!(CameraId::new("").is_err());
        assert!(CameraId::new("  ").is_err());
        assert_eq!(CameraId::new("cam-1").unwrap().as_str(), "cam-1");
    }

    #[test]
    fn frame_format_rejects_zero_fps() {
        assert!(FrameFormat::new(PixelFormat::Yuyv, 640, 480, 0).is_err());
    }

    #[test]
    fn frame_buffer_validates_yuyv_size() {
        let format = FrameFormat::new(PixelFormat::Yuyv, 640, 480, 30).unwrap();

        assert!(FrameBuffer::new(format, vec![0; 640 * 480]).is_err());
        assert_eq!(
            FrameBuffer::new(format, vec![0; 640 * 480 * 2])
                .unwrap()
                .len(),
            640 * 480 * 2
        );
    }

    #[test]
    fn frame_channel_delivers_bounded_frames() {
        let (sender, receiver) = frame_channel(2);
        let format = FrameFormat::new(PixelFormat::Yuyv, 2, 2, 30).unwrap();
        let frame = Frame::new(
            CameraId::new("cam-1").unwrap(),
            7,
            FrameBuffer::new(format, vec![0; 2 * 2 * 2]).unwrap(),
        );

        sender.send(frame.clone()).unwrap();

        let received = receiver.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(received, frame);
    }
}
