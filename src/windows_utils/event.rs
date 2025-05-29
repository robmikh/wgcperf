use std::time::Duration;

use windows::{
    Win32::{
        Foundation::{
            CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE, WAIT_OBJECT_0,
            WAIT_TIMEOUT,
        },
        System::Threading::{
            CreateEventW, GetCurrentProcess, INFINITE, SetEvent, WaitForSingleObject,
        },
    },
    core::Result,
};

pub struct Event(HANDLE);

unsafe impl Send for Event {}

impl Drop for Event {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

impl Clone for Event {
    fn clone(&self) -> Self {
        let handle = unsafe {
            let mut handle = HANDLE::default();
            DuplicateHandle(
                GetCurrentProcess(),
                self.0,
                GetCurrentProcess(),
                &mut handle,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )
            .unwrap();
            handle
        };
        Self(handle)
    }
}

#[allow(dead_code)]
pub enum EventTimeout {
    None,
    Infinite,
    Duration(Duration),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum WaitResult {
    Signaled,
    TimedOut,
}

impl Event {
    pub fn new() -> Result<Self> {
        let handle = unsafe { CreateEventW(None, false, false, None)? };
        Ok(Self(handle))
    }

    pub fn wait(&self, timeout: EventTimeout) -> Result<WaitResult> {
        let timeout = match timeout {
            EventTimeout::None => 0,
            EventTimeout::Infinite => INFINITE,
            EventTimeout::Duration(duration) => duration.as_millis() as u32,
        };
        let result = unsafe { WaitForSingleObject(self.0, timeout) };
        match result {
            WAIT_OBJECT_0 => Ok(WaitResult::Signaled),
            WAIT_TIMEOUT => Ok(WaitResult::TimedOut),
            _ => Err(windows::core::Error::from_win32()),
        }
    }

    pub fn is_signaled(&self) -> Result<bool> {
        let result = self.wait(EventTimeout::None)?;
        Ok(result == WaitResult::Signaled)
    }

    pub fn signal(&self) -> Result<()> {
        unsafe { SetEvent(self.0) }
    }
}
