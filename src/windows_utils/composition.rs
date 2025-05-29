use windows::{
    UI::Composition::{Compositor, Desktop::DesktopWindowTarget},
    Win32::{Foundation::HWND, System::WinRT::Composition::ICompositorDesktopInterop},
    core::{Interface, Result},
};

pub trait CompositionInterop {
    fn create_desktop_window_target(
        &self,
        window: HWND,
        is_topmost: bool,
    ) -> Result<DesktopWindowTarget>;
}

impl CompositionInterop for Compositor {
    fn create_desktop_window_target(
        &self,
        window: HWND,
        is_topmost: bool,
    ) -> Result<DesktopWindowTarget> {
        let compositor_desktop: ICompositorDesktopInterop = self.cast()?;
        unsafe { compositor_desktop.CreateDesktopWindowTarget(window, is_topmost) }
    }
}
