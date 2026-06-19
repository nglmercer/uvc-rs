use uvc_core::{EngineError, EngineResult, Frame, FrameBuffer, FrameFormat, FrameSink};

pub trait FrameDecoder {
    fn output_format(&self, input_format: FrameFormat) -> EngineResult<FrameFormat>;

    fn decode(&mut self, frame: &Frame) -> EngineResult<Vec<u8>>;
}

#[derive(Clone, Debug, Default)]
pub struct DecodedFrameSinkAdapter<D, S> {
    decoder: D,
    sink: S,
}

impl<D, S> DecodedFrameSinkAdapter<D, S>
where
    D: FrameDecoder,
    S: FrameSink,
{
    pub fn new(decoder: D, sink: S) -> Self {
        Self { decoder, sink }
    }

    pub fn push_frame(&mut self, frame: Frame) -> EngineResult<()> {
        let output_format = self.decoder.output_format(frame.buffer().format())?;
        let data = self.decoder.decode(&frame)?;
        let decoded = Frame::new(
            frame.camera_id().clone(),
            frame.sequence(),
            FrameBuffer::new(output_format, data)?,
        );
        self.sink.push(decoded)
    }

    pub fn into_inner(self) -> (D, S) {
        (self.decoder, self.sink)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct YuyvToRgbaDecoder;

impl FrameDecoder for YuyvToRgbaDecoder {
    fn output_format(&self, input_format: FrameFormat) -> EngineResult<FrameFormat> {
        Ok(FrameFormat::rgba(
            input_format.width(),
            input_format.height(),
            input_format.fps(),
        )?)
    }

    fn decode(&mut self, frame: &Frame) -> EngineResult<Vec<u8>> {
        let format = frame.buffer().format();
        let width = format.width();
        let height = format.height();
        let expected = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(2))
            .ok_or_else(|| {
                EngineError::InvalidFrameFormat("YUYV frame size overflow".to_owned())
            })?;
        let input = frame.buffer().as_slice();

        if input.len() != expected as usize {
            return Err(EngineError::InvalidFrameSize {
                format: format.to_string(),
                actual: input.len(),
                expected: expected as usize,
            });
        }

        let mut output = vec![0; expected as usize * 2];

        for (index, pair) in input.chunks_exact(4).enumerate() {
            let y0 = pair[0];
            let u = pair[1];
            let y1 = pair[2];
            let v = pair[3];
            let offset = index * 8;

            let (r, g, b) = yuv_to_rgb(y0, u, v);
            output[offset] = r;
            output[offset + 1] = g;
            output[offset + 2] = b;
            output[offset + 3] = 255;

            let (r, g, b) = yuv_to_rgb(y1, u, v);
            output[offset + 4] = r;
            output[offset + 5] = g;
            output[offset + 6] = b;
            output[offset + 7] = 255;
        }

        Ok(output)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Nv12ToRgbaDecoder;

impl FrameDecoder for Nv12ToRgbaDecoder {
    fn output_format(&self, input_format: FrameFormat) -> EngineResult<FrameFormat> {
        Ok(FrameFormat::rgba(
            input_format.width(),
            input_format.height(),
            input_format.fps(),
        )?)
    }

    fn decode(&mut self, frame: &Frame) -> EngineResult<Vec<u8>> {
        let format = frame.buffer().format();
        let width = format.width();
        let height = format.height();
        let y_len = width.checked_mul(height).ok_or_else(|| {
            EngineError::InvalidFrameFormat("NV12 frame size overflow".to_owned())
        })?;
        let uv_len = y_len / 2;
        let expected = y_len + uv_len;
        let input = frame.buffer().as_slice();

        if input.len() != expected as usize {
            return Err(EngineError::InvalidFrameSize {
                format: format.to_string(),
                actual: input.len(),
                expected: expected as usize,
            });
        }

        let mut output = vec![0; width as usize * height as usize * 4];

        for y in 0..height as usize {
            for x in 0..width as usize {
                let y_value = input[y * width as usize + x];
                let uv_index = ((y / 2) * (width as usize / 2) + (x / 2)) * 2;
                let u = input[y_len as usize + uv_index];
                let v = input[y_len as usize + uv_index + 1];
                let pixel_index = (y * width as usize + x) * 4;
                let (r, g, b) = yuv_to_rgb(y_value, u, v);

                output[pixel_index] = r;
                output[pixel_index + 1] = g;
                output[pixel_index + 2] = b;
                output[pixel_index + 3] = 255;
            }
        }

        Ok(output)
    }
}

fn yuv_to_rgb(y: u8, u: u8, v: u8) -> (u8, u8, u8) {
    let y = i32::from(y) - 16;
    let u = i32::from(u) - 128;
    let v = i32::from(v) - 128;

    let r = (298 * y + 409 * v + 128) >> 8;
    let g = (298 * y - 100 * u - 208 * v + 128) >> 8;
    let b = (298 * y + 516 * u + 128) >> 8;

    (clamp_u8(r), clamp_u8(g), clamp_u8(b))
}

fn clamp_u8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use uvc_core::{CameraId, EngineError, frame_channel};

    #[derive(Default)]
    struct VecFrameSink {
        frames: Vec<Frame>,
    }

    impl FrameSink for VecFrameSink {
        fn push(&mut self, frame: Frame) -> EngineResult<()> {
            self.frames.push(frame);
            Ok(())
        }
    }

    #[test]
    fn yuyv_to_rgba_decoder_converts_gray_pixels() {
        let camera_id = CameraId::new("cam-1").unwrap();
        let frame = Frame::new(
            camera_id.clone(),
            7,
            FrameBuffer::new(
                FrameFormat::yuyv(2, 1, 30).unwrap(),
                vec![235, 128, 235, 128],
            )
            .unwrap(),
        );
        let sink = VecFrameSink::default();
        let mut adapter = DecodedFrameSinkAdapter::new(YuyvToRgbaDecoder, sink);

        adapter.push_frame(frame).unwrap();

        let (_, sink) = adapter.into_inner();
        let frame = &sink.frames[0];
        assert_eq!(frame.camera_id(), &camera_id);
        assert_eq!(frame.sequence(), 7);
        assert_eq!(
            frame.buffer().format(),
            FrameFormat::rgba(2, 1, 30).unwrap()
        );
        assert_eq!(
            frame.buffer().as_slice(),
            &[255, 255, 255, 255, 255, 255, 255, 255]
        );
    }

    #[test]
    fn nv12_to_rgba_decoder_converts_gray_pixels() {
        let camera_id = CameraId::new("cam-1").unwrap();
        let frame = Frame::new(
            camera_id.clone(),
            9,
            FrameBuffer::new(
                FrameFormat::nv12(2, 2, 30).unwrap(),
                vec![235, 235, 235, 235, 128, 128],
            )
            .unwrap(),
        );
        let sink = VecFrameSink::default();
        let mut adapter = DecodedFrameSinkAdapter::new(Nv12ToRgbaDecoder, sink);

        adapter.push_frame(frame).unwrap();

        let frames = &adapter.into_inner().1.frames;
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].camera_id(), &camera_id);
        assert_eq!(frames[0].sequence(), 9);
        assert_eq!(
            frames[0].buffer().format(),
            FrameFormat::rgba(2, 2, 30).unwrap()
        );
        assert_eq!(frames[0].buffer().as_slice(), &[255; 16]);
    }

    #[test]
    fn decoded_frame_sink_adapter_reports_closed_sink() {
        let (sender, receiver) = frame_channel(1);
        drop(receiver);
        let frame = Frame::new(
            CameraId::new("cam-1").unwrap(),
            1,
            FrameBuffer::new(
                FrameFormat::yuyv(2, 1, 30).unwrap(),
                vec![235, 128, 235, 128],
            )
            .unwrap(),
        );
        let mut adapter = DecodedFrameSinkAdapter::new(YuyvToRgbaDecoder, sender);

        assert!(matches!(
            adapter.push_frame(frame),
            Err(EngineError::SinkClosed)
        ));
    }
}
