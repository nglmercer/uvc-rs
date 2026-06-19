use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use tracing::debug;
use uvc_core::{
    CameraConfig, CameraId, CameraPipeline, EngineError, EngineResult, Frame, FrameBuffer,
    FrameFormat, FrameSender, PixelFormat,
};

#[derive(Debug)]
pub struct FakeFrameGenerator {
    format: FrameFormat,
    sequence: u64,
}

impl FakeFrameGenerator {
    pub fn new(format: FrameFormat) -> Self {
        Self {
            format,
            sequence: 0,
        }
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn next_frame(&mut self, camera_id: &CameraId) -> Frame {
        let sequence = self.sequence;
        self.sequence += 1;

        Frame::new(
            camera_id.clone(),
            sequence,
            FrameBuffer::new(self.format, fake_payload(self.format, sequence))
                .expect("fake payload matches frame format"),
        )
    }
}

#[derive(Debug)]
pub struct FakeCameraPipeline {
    config: CameraConfig,
    sink: FrameSender,
    running: bool,
    stop_flag: Option<Arc<AtomicBool>>,
    thread: Option<JoinHandle<()>>,
}

impl FakeCameraPipeline {
    pub fn new(config: CameraConfig, sink: FrameSender) -> Self {
        Self {
            config,
            sink,
            running: false,
            stop_flag: None,
            thread: None,
        }
    }

    pub fn config(&self) -> &CameraConfig {
        &self.config
    }

    fn run_loop(config: CameraConfig, sink: FrameSender, stop_flag: Arc<AtomicBool>) {
        let mut generator = FakeFrameGenerator::new(config.format());
        let frame_interval = frame_interval(config.format());

        while !stop_flag.load(Ordering::Relaxed) {
            if config
                .frame_count()
                .is_some_and(|frame_count| generator.sequence() >= frame_count)
            {
                break;
            }

            let frame = generator.next_frame(config.camera_id());

            if let Err(error) = sink.send(frame) {
                debug!(camera_id = %config.camera_id(), %error, "fake camera sink closed");
                break;
            }

            thread::sleep(frame_interval);
        }
    }
}

impl CameraPipeline for FakeCameraPipeline {
    fn camera_id(&self) -> &CameraId {
        self.config.camera_id()
    }

    fn start(&mut self) -> EngineResult<()> {
        if self.running {
            return Err(EngineError::AlreadyRunning(
                self.config.camera_id().to_string(),
            ));
        }

        let stop_flag = Arc::new(AtomicBool::new(false));
        let thread_stop_flag = Arc::clone(&stop_flag);
        let thread = {
            let config = self.config.clone();
            let sink = self.sink.clone();

            thread::spawn(move || Self::run_loop(config, sink, thread_stop_flag))
        };

        self.stop_flag = Some(stop_flag);
        self.thread = Some(thread);
        self.running = true;

        Ok(())
    }

    fn stop(&mut self) -> EngineResult<()> {
        if !self.running {
            return Err(EngineError::NotRunning(self.config.camera_id().to_string()));
        }

        if let Some(stop_flag) = &self.stop_flag {
            stop_flag.store(true, Ordering::Relaxed);
        }

        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| EngineError::Backend("fake camera worker panicked".to_owned()))?;
        }

        self.running = false;
        self.stop_flag = None;

        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running
    }
}

impl Drop for FakeCameraPipeline {
    fn drop(&mut self) {
        if self.running {
            let _ = self.stop();
        }
    }
}

#[derive(Debug)]
pub struct FakeMultiCameraEngine {
    pipelines: Vec<FakeCameraPipeline>,
}

impl FakeMultiCameraEngine {
    pub fn spawn(configs: Vec<CameraConfig>, sink: FrameSender) -> EngineResult<Self> {
        let mut pipelines = Vec::with_capacity(configs.len());

        for config in configs {
            let mut pipeline = FakeCameraPipeline::new(config, sink.clone());

            if let Err(error) = pipeline.start() {
                let _ = stop_pipelines(&mut pipelines);
                return Err(error);
            }

            pipelines.push(pipeline);
        }

        Ok(Self { pipelines })
    }

    pub fn start_all(&mut self) -> EngineResult<()> {
        for pipeline in &mut self.pipelines {
            pipeline.start()?;
        }

        Ok(())
    }

    pub fn stop_all(&mut self) -> EngineResult<()> {
        stop_pipelines(&mut self.pipelines)
    }

    pub fn pipelines(&self) -> &[FakeCameraPipeline] {
        &self.pipelines
    }
}

impl Drop for FakeMultiCameraEngine {
    fn drop(&mut self) {
        let _ = stop_pipelines(&mut self.pipelines);
    }
}

fn stop_pipelines(pipelines: &mut [FakeCameraPipeline]) -> EngineResult<()> {
    let mut first_error = None;

    for pipeline in pipelines {
        if let Err(error) = pipeline.stop() {
            first_error.get_or_insert(error);
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn frame_interval(format: FrameFormat) -> Duration {
    Duration::from_secs_f64(1.0 / f64::from(format.fps()))
}

fn fake_payload(format: FrameFormat, sequence: u64) -> Vec<u8> {
    match format.pixel_format() {
        PixelFormat::Mjpeg | PixelFormat::H264 => vec![(sequence & 0xff) as u8; 64],
        PixelFormat::Yuyv | PixelFormat::Nv12 => raw_pattern(format, sequence),
        PixelFormat::Rgba => rgba_pattern(format, sequence),
    }
}

fn raw_pattern(format: FrameFormat, sequence: u64) -> Vec<u8> {
    let len = format
        .expected_len()
        .expect("raw formats have an expected frame length");
    let mut data = vec![0u8; len];

    for (index, byte) in data.iter_mut().enumerate() {
        *byte = ((index as u64 + sequence) & 0xff) as u8;
    }

    data
}

fn rgba_pattern(format: FrameFormat, sequence: u64) -> Vec<u8> {
    let len = format
        .expected_len()
        .expect("rgba format has an expected frame length");
    let mut data = vec![0u8; len];

    for pixel in 0..(format.width() * format.height()) {
        let offset = pixel as usize * 4;
        data[offset] = (pixel & 0xff) as u8;
        data[offset + 1] = ((pixel >> 8) & 0xff) as u8;
        data[offset + 2] = (sequence & 0xff) as u8;
        data[offset + 3] = 255;
    }

    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, time::Duration};
    use uvc_core::frame_channel;

    #[test]
    fn fake_frame_generator_produces_valid_yuyv_frames() {
        let mut generator = FakeFrameGenerator::new(FrameFormat::yuyv(4, 4, 30).unwrap());
        let camera_id = CameraId::new("cam-1").unwrap();

        let frame = generator.next_frame(&camera_id);

        assert_eq!(frame.camera_id(), &camera_id);
        assert_eq!(frame.sequence(), 0);
        assert_eq!(frame.buffer().len(), 4 * 4 * 2);
        assert_eq!(generator.sequence(), 1);
    }

    #[test]
    fn fake_multi_camera_engine_streams_multiple_cameras() {
        let (sender, receiver) = frame_channel(8);
        let configs = (0..4)
            .map(|index| {
                CameraConfig::new(
                    CameraId::new(format!("cam-{index}")).unwrap(),
                    FrameFormat::yuyv(8, 8, 120).unwrap(),
                )
                .with_frame_count(2)
            })
            .collect();

        let mut engine = FakeMultiCameraEngine::spawn(configs, sender).unwrap();
        let mut seen = HashSet::new();

        while seen.len() < 4 {
            let frame = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
            seen.insert(frame.camera_id().clone());
        }

        engine.stop_all().unwrap();

        assert_eq!(seen.len(), 4);
        assert!(
            engine
                .pipelines()
                .iter()
                .all(|pipeline| !pipeline.is_running())
        );
    }
}
