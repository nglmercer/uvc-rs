use std::{
    process::ExitCode,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use tracing_subscriber::EnvFilter;
use uvc_core::{
    CameraConfig, CameraId, EngineError, FrameFormat, FrameReceiver, PixelFormat, frame_channel,
};
use uvc_driver::FakeMultiCameraEngine;

#[derive(Debug)]
struct FakeMultiArgs {
    cameras: usize,
    seconds: u64,
    fps: u32,
    width: u32,
    height: u32,
    format: PixelFormat,
}

impl Default for FakeMultiArgs {
    fn default() -> Self {
        Self {
            cameras: 4,
            seconds: 3,
            fps: 30,
            width: 640,
            height: 480,
            format: PixelFormat::Yuyv,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_owned());

    match command.as_str() {
        "fake-multi" => run_fake_multi(args),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => bail!("unknown command `{other}`"),
    }
}

fn run_fake_multi<I>(args: I) -> Result<()>
where
    I: IntoIterator<Item = String>,
{
    let options = parse_fake_multi_args(args)?;
    let format = FrameFormat::new(options.format, options.width, options.height, options.fps)
        .context("invalid fake camera format")?;
    let configs = (0..options.cameras)
        .map(|index| {
            CameraConfig::new(
                CameraId::new(format!("cam-{index}")).expect("generated camera id is non-empty"),
                format,
            )
        })
        .collect();

    let (sender, receiver) = frame_channel(options.cameras * 4);
    let mut engine =
        FakeMultiCameraEngine::spawn(configs, sender).context("failed to start fake cameras")?;

    println!(
        "started {} fake cameras at {} using {}",
        options.cameras, format, options.format
    );

    let total_frames = receive_for_duration(&receiver, Duration::from_secs(options.seconds))?;

    engine
        .stop_all()
        .context("failed to stop fake cameras cleanly")?;

    println!("received {total_frames} frames");

    Ok(())
}

fn receive_for_duration(receiver: &FrameReceiver, duration: Duration) -> Result<u64> {
    let deadline = Instant::now() + duration;
    let mut total_frames = 0u64;

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let timeout = remaining.min(Duration::from_millis(200));

        match receiver.recv_timeout(timeout) {
            Ok(frame) => {
                total_frames += 1;
                println!(
                    "frame camera={} sequence={} bytes={}",
                    frame.camera_id(),
                    frame.sequence(),
                    frame.buffer().len()
                );
            }
            Err(EngineError::Timeout) => {}
            Err(error) => return Err(error.into()),
        }
    }

    Ok(total_frames)
}

fn parse_fake_multi_args<I>(args: I) -> Result<FakeMultiArgs>
where
    I: IntoIterator<Item = String>,
{
    let mut options = FakeMultiArgs::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cameras" => options.cameras = parse_usize(args.next(), "--cameras")?,
            "--seconds" => options.seconds = parse_u64(args.next(), "--seconds")?,
            "--fps" => options.fps = parse_u32(args.next(), "--fps")?,
            "--width" => options.width = parse_u32(args.next(), "--width")?,
            "--height" => options.height = parse_u32(args.next(), "--height")?,
            "--format" => {
                let value = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--format requires a value"))?;
                options.format = value.parse()?;
            }
            other => bail!("unknown fake-multi option `{other}`"),
        }
    }

    if options.cameras == 0 {
        bail!("--cameras must be greater than zero");
    }

    Ok(options)
}

fn parse_usize(value: Option<String>, name: &str) -> Result<usize> {
    value
        .ok_or_else(|| anyhow::anyhow!("{name} requires a value"))?
        .parse::<usize>()
        .with_context(|| format!("invalid value for {name}"))
}

fn parse_u64(value: Option<String>, name: &str) -> Result<u64> {
    value
        .ok_or_else(|| anyhow::anyhow!("{name} requires a value"))?
        .parse::<u64>()
        .with_context(|| format!("invalid value for {name}"))
}

fn parse_u32(value: Option<String>, name: &str) -> Result<u32> {
    value
        .ok_or_else(|| anyhow::anyhow!("{name} requires a value"))?
        .parse::<u32>()
        .with_context(|| format!("invalid value for {name}"))
}

fn print_usage() {
    println!(
        "uvc-cli fake-multi [--cameras N] [--seconds N] [--fps N] [--width N] [--height N] [--format mjpeg|yuyv|h264|nv12|rgba]"
    );
}
