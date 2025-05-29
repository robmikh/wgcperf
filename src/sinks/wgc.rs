use std::{
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};

use windows::{
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{
            Direct3D11CaptureFramePool, GraphicsCaptureDirtyRegionMode, GraphicsCaptureItem,
            GraphicsCaptureSession,
        },
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
    num_frames: Arc<AtomicUsize>,
}

impl WgcCaptureSink {
    pub fn new(
        d3d_device: &ID3D11Device,
        monitor: HMONITOR,
        use_dirty_rects: bool,
    ) -> Result<Self> {
        let device = create_direct3d_device(d3d_device)?;
        let item = create_capture_item_for_monitor(monitor)?;
        let num_frames = Arc::new(AtomicUsize::new(0));
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            3,
            item.Size()?,
        )?;
        frame_pool.FrameArrived(&TypedEventHandler::<Direct3D11CaptureFramePool, _>::new({
            let num_frames = num_frames.clone();
            move |frame_pool, _| -> Result<()> {
                let frame_pool: &Direct3D11CaptureFramePool = frame_pool.unwrap();
                let frame = frame_pool.TryGetNextFrame()?;
                num_frames.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                frame.Close()?;
                Ok(())
            }
        }))?;
        let session = frame_pool.CreateCaptureSession(&item)?;
        session.SetIsBorderRequired(false)?;
        session.SetIsCursorCaptureEnabled(false)?;
        if use_dirty_rects {
            session.SetDirtyRegionMode(GraphicsCaptureDirtyRegionMode::ReportAndRender)?;
        }
        // There's a bug where setting 0 won't work until we set something non-zero first
        session.SetMinUpdateInterval(Duration::from_millis(1).into())?;
        session.SetMinUpdateInterval(Duration::from_millis(0).into())?;
        Ok(Self {
            _item: item,
            session,
            frame_pool,
            num_frames,
        })
    }
}

impl CaptureSink for WgcCaptureSink {
    fn start(&mut self) -> Result<()> {
        self.session.StartCapture()?;
        Ok(())
    }

    fn stop(&mut self) -> Result<usize> {
        self.session.Close()?;
        self.frame_pool.Close()?;
        let num_frames = self.num_frames.load(std::sync::atomic::Ordering::SeqCst);
        Ok(num_frames)
    }
}

fn create_capture_item_for_monitor(monitor_handle: HMONITOR) -> Result<GraphicsCaptureItem> {
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    unsafe { interop.CreateForMonitor(monitor_handle) }
}
