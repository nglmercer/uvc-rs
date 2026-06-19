# Rust UVC Engine Project Plan

## Goal

Create a Rust-first Android UVC camera engine workspace that can later be expanded into a production multi-camera engine. The initial implementation should prioritize a compilable workspace, clean crate boundaries, and multi-camera concurrency validation before attempting full Android/libusb integration.

## Initial Repository Structure

Use a Cargo workspace with these crates:

```text
uvc-rs/
  Cargo.toml
  README.md
  LICENSE
  crates/
    uvc-core/
      Cargo.toml
      src/
        lib.rs
        camera.rs
        format.rs
        frame.rs
        pipeline.rs
        error.rs
    uvc-driver/
      Cargo.toml
      src/
        lib.rs
        device.rs
        transfer.rs
        descriptor.rs
        backend.rs
    uvc-jni/
      Cargo.toml
      src/
        lib.rs
        android.rs
        controls.rs
    uvc-cli/
      Cargo.toml
      src/
        main.rs
```

## Crate Responsibilities

### `uvc-core`

Owns pure-Rust data structures and traits:

- `CameraConfig`
- `PixelFormat`
- `FrameFormat`
- `FrameBuffer`
- `FrameReceiver`
- `FrameSink`
- `CameraPipeline`
- `EngineError`

This crate should not depend on `rusb`, `jni`, Android NDK crates, or platform-specific APIs.

### `uvc-driver`

Wraps USB/UVC behavior:

- Device discovery abstraction
- UVC descriptor parsing
- Transfer scheduling abstraction
- Isochronous/bulk transfer state
- Optional `rusb` backend behind a feature flag

Initial backend should be compile-safe on desktop and Android targets.

### `uvc-jni`

Provides Android/Kotlin interop later:

- JNI export functions
- Java/Kotlin error mapping
- Android file-descriptor wrapper
- Native surface/HardwareBuffer hooks

This crate should only be built for Android targets or behind an `android` feature.

### `uvc-cli`

Provides local validation:

- List configured cameras
- Start multiple fake camera pipelines
- Simulate frame generation and frame sink behavior
- Validate concurrent scheduling without hardware

## Recommended Initial Dependencies

Start minimal and add platform-specific dependencies only when needed.

Workspace-level likely dependencies:

- `anyhow`
- `thiserror`
- `tracing`
- `tracing-subscriber`
- `tokio` or `crossbeam`
- `parking_lot`
- `bytemuck`
- `num_enum`

Android/JNI later:

- `jni`
- `ndk`
- `android_logger`

USB later:

- `rusb`
- `libusb-sys`

## Milestone 1 — Workspace Skeleton

Create the Cargo workspace and empty crates.

Validation:

```bash
cargo fmt --all
cargo check --workspace --all-targets
```

Expected result: clean build with no platform-specific dependencies required.

## Milestone 2 — Core Types and Error Model

Implement `uvc-core` types:

- `CameraId`
- `CameraConfig`
- `PixelFormat`
- `FrameFormat`
- `FrameBuffer`
- `FrameSink`
- `EngineError`
- `EngineResult`

Add tests for:

- format validation
- frame buffer ownership
- error conversion
- camera config defaults

Validation:

```bash
cargo test -p uvc-core
cargo check --workspace --all-targets
```

## Milestone 3 — Fake Multi-Camera Pipeline

Implement a deterministic fake backend in `uvc-driver` that generates synthetic frames.

Add a `CameraPipeline` implementation that:

- Starts multiple fake cameras concurrently
- Produces frames through a bounded channel or ring buffer
- Stops cleanly on drop
- Tracks per-camera state

Validation:

```bash
cargo test -p uvc-driver
cargo run -p uvc-cli -- fake-multi --cameras 4
```

This milestone validates the concurrency model before real USB code is introduced.

## Milestone 4 — UVC Descriptor and Format Negotiation

Add descriptor parsing models:

- VideoStream interface
- Format descriptor
- Frame descriptor
- Endpoint descriptor
- Alternate setting selection

Keep parsing feature-gated until real descriptor data is available.

Validation:

- Unit tests with synthetic descriptor byte slices
- Workspace checks

## Milestone 5 — Android FD Wrapper Design

Add a platform-specific module in `uvc-jni`:

- `AndroidUsbDevice`
- `AndroidFdHandle`
- `AndroidDeviceConnection`

Do not implement real `libusb_wrap_sys_device` until the build environment has Android NDK/libusb configured.

Document the intended C ABI boundary:

```rust
pub struct AndroidUsbDevice {
    fd: RawFd,
    vendor_id: u16,
    product_id: u16,
}
```

## Milestone 6 — Real USB Backend

Add optional `rusb` backend behind `rusb` feature.

Implement:

- device handle wrapper
- endpoint selection
- transfer lifecycle
- disconnect handling
- bandwidth-aware alternate setting selection

Validation should first target desktop Linux if available:

```bash
cargo check -p uvc-driver --features rusb
```

Android cross-check:

```bash
cargo check -p uvc-driver --target aarch64-linux-android --features rusb
```

## Milestone 7 — JNI Binding Layer

Add Kotlin-facing JNI exports:

- initialize engine
- start camera
- stop camera
- set control
- get supported formats
- release engine

Use strict ownership rules:

- Rust owns pipeline lifetime
- Kotlin owns permission and USB device lifecycle
- JNI layer only passes opaque handles and error codes

## Milestone 8 — Performance Validation

Add benchmarks or examples for:

- multi-camera fake frame generation
- frame buffer reuse
- bounded channel throughput
- zero-allocation steady-state hot path

Use `criterion` only after the core pipeline is stable.

## Immediate Next Implementation Steps

1. Create the Cargo workspace.
2. Add `uvc-core`, `uvc-driver`, `uvc-jni`, and `uvc-cli`.
3. Implement core types and errors.
4. Implement a fake multi-camera pipeline.
5. Add CLI command to run 2–4 fake cameras concurrently.
6. Add tests and run `cargo fmt --all` and `cargo test --workspace`.

## Important Constraints

- Do not start with real USB/libusb code.
- Do not depend on Android-specific crates in the core crate.
- Keep fake backend first so concurrency can be validated without hardware.
- Keep Android FD wrapping isolated behind `uvc-jni` or an Android feature.
- Avoid JVM allocations in frame processing paths once JNI integration begins.
