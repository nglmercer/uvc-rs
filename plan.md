# Project Specification: High-Performance Multi-Cam UVC Engine in Rust for Android

## 1. Objective & Vision
We want to build a modern, high-performance, and safe Android UVC (USB Video Class) camera engine written entirely in **Rust** (with a lightweight Kotlin/Java JNI binding layer). The engine must support **"multi-road" (simultaneous multi-camera concurrent streaming)** via a single USB Host/Hub, overcoming the memory leaks, architecture technical debt, and threading overhead common in legacy C/Java-based solutions (like saki4510t).

---

## 2. Technical Stack & Target Environment
* **Core Logic:** Rust (Targeting `aarch64-linux-android`, `armv7-linux-androideabi`).
* **Interop Layer:** `jni-rs` crate (Rust to/from Kotlin/Java).
* **USB Communication:** `rusb` crate (Rust bindings for `libusb-1.0`) running in user-space.
* **Android SDK Compatibility:** Android 10 (API 29) to Android 14+ (API 34+).
* **Concurrence Model:** Rust `tokio` or `crossbeam` for safe, ultra-low latency multi-threaded frame processing.

---

## 3. Architecture Overview (The Flow)
1. **Android Framework Layer (Kotlin):** Manages OS USB permissions via `UsbManager`, obtains the file descriptor (`fd`) of the USB device, and passes it securely to Rust via JNI.
2. **JNI / Interop Layer (Rust):** Bridges Android types to Rust structs.
3. **Core Driver Layer (Rust):** Wraps `libusb` using the passed native file descriptor (`libusb_wrap_sys_device`). Decodes UVC descriptors and manages isochronous/bulk transfers.
4. **Processing Pipeline (Rust):** Parallel threads decode MJPEG/YUY2 frames concurrently per camera, uploading raw frame buffers directly into Android `Surface` or `HardwareBuffer` objects.

---

## 4. System Requirements & Features

### Core Features
* **True Multi-Camera Concurrency (Multi-road):** Ability to spin up multiple isolated instances of UVC camera pipelines bound to different USB devices simultaneously.
* **Zero-Permission Root Access:** Must operate entirely in user-space by consuming the file descriptor (`fd`) provided by Android's `UsbDeviceConnection.getFileDescriptor()`.
* **UVC Control Engine:** API to query and set camera controls dynamically (Brightness, Contrast, Sharpness, Manual/Auto Focus, Exposure, Zoom).
* **Dynamic Resolution & Format Negotiation:** Auto-detect supported streams (MJPEG, YUYV, H.264) and select optimal endpoints.
* **Frame Pipeline Hooks:** Provide a clean Rust trait interface to pipe raw decoded frames (RGBA/YUV420p) into custom machine learning models (e.g., ONNX Runtime) or Android surfaces.

### Performance Requirements
* **Low-Memory Overhead:** Zero allocations in the steady-state render loop. Frame buffers must be recycled via a thread-safe ring buffer pool.
* **GC-Independence:** Frame decoding, color conversion, and packet management must happen strictly inside Rust's memory boundary to completely bypass Android's JVM Garbage Collector spikes.

---

## 5. Critical Technical Challenges to Solve

### A. USB Bus Bandwidth & Isochronous Transfers (The "Multi-Road" Bottleneck)
* **Problem:** Multiple full-HD cameras streaming raw YUV will easily saturate the USB controller bus, crashing with `LIBUSB_TRANSFER_ERROR` or `No space left on device`.
* **Plan:** The engine must enforce **MJPEG/H.264** compression streaming from the cameras, perform native hardware-accelerated/SIMD decoding in Rust, and allow manual adjustment of `wMaxPacketSize` and `bAlternateSetting` to throttle bandwidth allocations dynamically.

### B. Android Native File Descriptor Integration
* **Problem:** `libusb` usually scans the system `/dev/bus/usb/`, which is blocked by Android SELinux policies without root.
* **Plan:** Implement `libusb_wrap_sys_device` inside the Rust initialization sequence. We must convert the Android `fd` into a native USB device handle seamlessly.

### C. Fast Memory Copying to Android (Zero-Copy Goal)
* **Problem:** Passing millions of frame bytes from Rust memory back into the Java/Kotlin Virtual Machine creates massive CPU overhead.
* **Plan:** Utilize Android's `ANativeWindow` via the `ndk` crate or create direct `java.nio.ByteBuffer` instances mapping directly to Rust's native memory allocations.

### D. Thread Safety and Device Disconnections
* **Problem:** Sudden physical unplugging of a USB Hub with 3 cameras active can cause dangling pointers and segfaults in native code.
* **Plan:** Leverage Rust’s ownership and lifetimes (`Arc<Mutex<CameraContext>>` or channels) to safely drop, unbind, and close `libusb` resources gracefully upon sudden disconnect signals.

---

## 6. Prompt for the LLM Architect
> "Act as a Senior Principal Systems Engineer expert in Rust, the Android NDK, and the UVC protocol. Review the specification above. 
> 
> Please provide:
> 1. A highly modular **Crate Structure / Architecture Design** for this project (breaking it down into core, jni, and driver modules).
> 2. A proof-of-concept **Rust code snippet** demonstrating how to wrap an Android USB File Descriptor using `rusb` / `libusb-sys`.
> 3. A step-by-step **Implementation Plan** split into Milestones, prioritizing the multi-camera concurrency validation first."
