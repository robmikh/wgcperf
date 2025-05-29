use std::{
    sync::{Arc, atomic::AtomicUsize},
    thread::JoinHandle,
};

use windows::{
    Win32::Graphics::{
        Direct3D11::ID3D11Device,
        Dxgi::{DXGI_OUTDUPL_FRAME_INFO, IDXGIOutput1, IDXGIOutputDuplication},
    },
    core::Result,
};

use crate::windows_utils::event::Event;

use super::CaptureSink;

pub struct DdaCaptureSink {
    d3d_device: ID3D11Device,
    output: IDXGIOutput1,
    duplication: Option<IDXGIOutputDuplication>,
    stop_event: Event,
    capture_thread: Option<JoinHandle<Result<()>>>,
    num_frames: Arc<AtomicUsize>,
}

struct DuplicationSmuggler(IDXGIOutputDuplication);
unsafe impl Send for DuplicationSmuggler {}

impl DdaCaptureSink {
    pub fn new(d3d_device: &ID3D11Device, output: IDXGIOutput1) -> Result<Self> {
        let event = Event::new()?;

        Ok(Self {
            d3d_device: d3d_device.clone(),
            output,
            duplication: None,
            stop_event: event,
            capture_thread: None,
            num_frames: Arc::new(AtomicUsize::new(0)),
        })
    }
}

impl CaptureSink for DdaCaptureSink {
    fn start(&mut self) -> windows::core::Result<()> {
        if self.duplication.is_none() && !self.stop_event.is_signaled()? {
            let duplication = unsafe { self.output.DuplicateOutput(&self.d3d_device)? };
            let capture_thread = std::thread::spawn({
                let duplication = DuplicationSmuggler(duplication.clone());
                let event = self.stop_event.clone();
                let num_frames = self.num_frames.clone();
                move || -> Result<()> {
                    let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
                    let mut resource = None;
                    while !event.is_signaled()? {
                        unsafe {
                            duplication
                                .0
                                .AcquireNextFrame(100, &mut frame_info, &mut resource)?
                        };
                        resource = None;
                        num_frames.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        unsafe {
                            duplication.0.ReleaseFrame()?;
                        }
                    }
                    Ok(())
                }
            });
            self.duplication = Some(duplication);
            self.capture_thread = Some(capture_thread);
        }
        Ok(())
    }

    fn stop(&mut self) -> windows::core::Result<usize> {
        let num_frames = if let Some(thread) = self.capture_thread.take() {
            self.stop_event.signal()?;
            thread.join().unwrap()?;
            self.duplication = None;
            self.num_frames.load(std::sync::atomic::Ordering::SeqCst)
        } else {
            0
        };
        Ok(num_frames)
    }
}
