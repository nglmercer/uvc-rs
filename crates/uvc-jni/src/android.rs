use std::os::raw::c_int;

use uvc_core::{CameraId, EngineError, EngineResult};

pub type NativeFileDescriptor = c_int;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AndroidFdHandle {
    fd: NativeFileDescriptor,
}

impl AndroidFdHandle {
    pub fn new(fd: NativeFileDescriptor) -> EngineResult<Self> {
        if fd < 0 {
            return Err(EngineError::InvalidArgument(format!(
                "Android USB file descriptor must be non-negative, got {fd}"
            )));
        }

        Ok(Self { fd })
    }

    pub fn fd(self) -> NativeFileDescriptor {
        self.fd
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AndroidUsbDevice {
    fd: AndroidFdHandle,
    vendor_id: u16,
    product_id: u16,
    camera_id: CameraId,
}

impl AndroidUsbDevice {
    pub fn new(
        fd: NativeFileDescriptor,
        vendor_id: u16,
        product_id: u16,
        camera_id: CameraId,
    ) -> EngineResult<Self> {
        Ok(Self {
            fd: AndroidFdHandle::new(fd)?,
            vendor_id,
            product_id,
            camera_id,
        })
    }

    pub fn fd(self) -> AndroidFdHandle {
        self.fd
    }

    pub fn vendor_id(&self) -> u16 {
        self.vendor_id
    }

    pub fn product_id(&self) -> u16 {
        self.product_id
    }

    pub fn camera_id(&self) -> &CameraId {
        &self.camera_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_fd_handle_rejects_negative_fds() {
        assert!(AndroidFdHandle::new(-1).is_err());
        assert_eq!(AndroidFdHandle::new(42).unwrap().fd(), 42);
    }

    #[test]
    fn android_usb_device_tracks_identity() {
        let device =
            AndroidUsbDevice::new(42, 0x1234, 0x5678, CameraId::new("usb-cam-1").unwrap()).unwrap();

        assert_eq!(device.vendor_id(), 0x1234);
        assert_eq!(device.product_id(), 0x5678);
        assert_eq!(device.camera_id().as_str(), "usb-cam-1");
    }
}
