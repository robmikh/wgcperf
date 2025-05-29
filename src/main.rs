mod pdh;
mod perf;
mod window;
mod windows_utils;

use std::time::Duration;

use window::Window;
use windows::{
    UI::{
        Color,
        Composition::{AnimationIterationBehavior, Compositor},
    },
    Win32::{
        Foundation::HWND,
        Graphics::Gdi::{
            GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromWindow,
        },
        System::{
            WinRT::{RO_INIT_SINGLETHREADED, RoInitialize},
            WindowsProgramming::MulDiv,
        },
        UI::{
            HiDpi::{
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI,
                SetProcessDpiAwarenessContext,
            },
            WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, TranslateMessage},
        },
    },
    core::{Result, h},
};
use windows_numerics::{Vector2, Vector3};
use windows_utils::{
    composition::CompositionInterop,
    dispatcher_queue::shutdown_dispatcher_queue_controller_and_wait,
};

use crate::windows_utils::dispatcher_queue::create_dispatcher_queue_controller_for_current_thread;

fn main() -> Result<()> {
    // TODO: Parse args

    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)?;
    }
    unsafe { RoInitialize(RO_INIT_SINGLETHREADED)? };
    let controller = create_dispatcher_queue_controller_for_current_thread()?;

    // TODO: From args
    let monitor_handle =
        unsafe { MonitorFromWindow(HWND(std::ptr::null_mut()), MONITOR_DEFAULTTOPRIMARY) };
    let monitor_info = unsafe {
        let mut info = MONITORINFO::default();
        info.cbSize = std::mem::size_of_val(&info) as u32;
        GetMonitorInfoW(monitor_handle, &mut info).ok()?;
        info
    };
    let work_area = monitor_info.rcWork;
    println!("Monitor details:");
    println!("  handle: {:010X}", monitor_handle.0 as usize);
    // TODO: Monitor index
    // TODO: Display name
    // TODO: Refresh rate

    // Compute window position
    let dpi = unsafe {
        let mut dpix = 0;
        let mut dpiy = 0;
        GetDpiForMonitor(monitor_handle, MDT_EFFECTIVE_DPI, &mut dpix, &mut dpiy)?;
        assert_eq!(dpix, dpiy);
        dpix
    };
    let window_width = unsafe { MulDiv(500, dpi as i32, 96) };
    let window_height = unsafe { MulDiv(500, dpi as i32, 96) };
    let work_area_width = work_area.right - work_area.left;
    let work_area_height = work_area.bottom - work_area.top;
    let window_x = ((work_area_width - window_width) / 2) + work_area.left;
    let window_y = ((work_area_height - window_height) / 2) + work_area.top;

    // Create our dummy content
    let window = Window::new(
        "Dummy Content",
        window_x,
        window_y,
        window_width as u32,
        window_height as u32,
    )?;
    let compositor = Compositor::new()?;
    let root = compositor.CreateSpriteVisual()?;
    root.SetRelativeSizeAdjustment(Vector2::new(1.0, 1.0))?;
    root.SetBrush(&compositor.CreateColorBrushWithColor(Color {
        A: 255,
        R: 0,
        G: 0,
        B: 0,
    })?)?;
    let content = compositor.CreateSpriteVisual()?;
    content.SetRelativeSizeAdjustment(Vector2 { X: 0.33, Y: 0.33 })?;
    content.SetAnchorPoint(Vector2 { X: 0.5, Y: 0.5 })?;
    content.SetRelativeOffsetAdjustment(Vector3 {
        X: 0.5,
        Y: 0.5,
        Z: 0.0,
    })?;
    content.SetBrush(&compositor.CreateColorBrushWithColor(Color {
        A: 255,
        R: 255,
        G: 0,
        B: 0,
    })?)?;
    root.Children()?.InsertAtTop(&content)?;
    let target = compositor.create_desktop_window_target(window.handle(), false)?;
    target.SetRoot(&root)?;

    // Animate the content
    let easing = compositor.CreateLinearEasingFunction()?;
    let animation = compositor.CreateScalarKeyFrameAnimation()?;
    animation.InsertKeyFrameWithEasingFunction(0.0, 0.0, &easing)?;
    animation.InsertKeyFrameWithEasingFunction(1.0, 360.0, &easing)?;
    animation.SetDuration(Duration::from_secs(3).into())?;
    animation.SetIterationBehavior(AnimationIterationBehavior::Forever)?;
    content.StartAnimation(h!("RotationAngleInDegrees"), &animation)?;

    // Show the window
    window.show();

    // TODO: Configurable
    let test_duration = Duration::from_secs(5);

    // TODO: Record baseline
    // TODO: Record WGC
    // TODO: Record DDA

    let mut message = MSG::default();
    unsafe {
        while GetMessageW(&mut message, None, 0, 0).into() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    // TODO: Cleanup
    let _ = shutdown_dispatcher_queue_controller_and_wait(&controller, message.wParam.0 as i32)?;

    Ok(())
}
