use crate::{CameraId, EngineResult};

pub trait CameraPipeline {
    fn camera_id(&self) -> &CameraId;

    fn start(&mut self) -> EngineResult<()>;

    fn stop(&mut self) -> EngineResult<()>;

    fn is_running(&self) -> bool;
}
