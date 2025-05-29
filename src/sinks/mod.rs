pub trait CaptureSink {
    fn start(&mut self) -> windows::core::Result<()>;
    fn stop(&mut self) -> windows::core::Result<()>;
}

pub mod dda;
pub mod wgc;
