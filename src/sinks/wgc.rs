use windows::{
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession},
        DirectX::DirectXPixelFormat,
    },
    Win32::{
        Graphics::{Direct3D11::ID3D11Device, Gdi::HMONITOR},
        System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
    },
    core::Result,
};

use crate::windows_utils::d3d::create_direct3d_device;

use super::CaptureSink;

pub struct WgcCaptureSink {
    _item: GraphicsCaptureItem,
    session: GraphicsCaptureSession,
    frame_pool: Direct3D11CaptureFramePool,
}

impl WgcCaptureSink {
    pub fn new(d3d_device: &ID3D11Device, monitor: HMONITOR) -> Result<Self> {
        let device = create_direct3d_device(d3d_device)?;
        let item = create_capture_item_for_monitor(monitor)?;
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            3,
            item.Size()?,
        )?;
        frame_pool.FrameArrived(&TypedEventHandler::<Direct3D11CaptureFramePool, _>::new(
            |frame_pool, _| -> Result<()> {
                let frame_pool: &Direct3D11CaptureFramePool = frame_pool.unwrap();
                let frame = frame_pool.TryGetNextFrame()?;
                frame.Close()?;
                Ok(())
            },
        ))?;
        let session = frame_pool.CreateCaptureSession(&item)?;
        session.SetIsBorderRequired(false)?;
        session.SetIsCursorCaptureEnabled(false)?;
        Ok(Self {
            _item: item,
            session,
            frame_pool,
        })
    }
}

impl CaptureSink for WgcCaptureSink {
    fn start(&mut self) -> Result<()> {
        self.session.StartCapture()?;
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.session.Close()?;
        self.frame_pool.Close()?;
        Ok(())
    }
}

fn create_capture_item_for_monitor(monitor_handle: HMONITOR) -> Result<GraphicsCaptureItem> {
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    unsafe { interop.CreateForMonitor(monitor_handle) }
}
