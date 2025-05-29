mod window;
mod windows_utils;

use std::time::Duration;

use window::Window;
use windows::{core::{h, Result}, Win32::{System::WinRT::{RoInitialize, RO_INIT_SINGLETHREADED}, UI::{HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2}, WindowsAndMessaging::{DispatchMessageW, GetMessageW, TranslateMessage, MSG}}}, UI::{Color, Composition::{AnimationIterationBehavior, Compositor}}};
use windows_numerics::{Vector2, Vector3};
use windows_utils::{composition::CompositionInterop, dispatcher_queue::shutdown_dispatcher_queue_controller_and_wait};

use crate::windows_utils::dispatcher_queue::create_dispatcher_queue_controller_for_current_thread;

fn main() -> Result<()> {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)?;
    }
    unsafe { RoInitialize(RO_INIT_SINGLETHREADED)? };
    let controller = create_dispatcher_queue_controller_for_current_thread()?;

    // Create our dummy content
    let window = Window::new("Dummy Content", 500, 500)?;
    let compositor = Compositor::new()?;
    let root = compositor.CreateSpriteVisual()?;
    root.SetRelativeSizeAdjustment(Vector2::new(1.0, 1.0))?;
    root.SetBrush(&compositor.CreateColorBrushWithColor(Color { A: 255, R: 0, G: 0, B: 0 })?)?;
    let content = compositor.CreateSpriteVisual()?;
    content.SetRelativeSizeAdjustment(Vector2 { X: 0.33, Y: 0.33 })?;
    content.SetAnchorPoint(Vector2 { X: 0.5, Y: 0.5 })?;
    content.SetRelativeOffsetAdjustment(Vector3 { X: 0.5, Y: 0.5, Z: 0.0})?;
    content.SetBrush(&compositor.CreateColorBrushWithColor(Color { A: 255, R: 255, G: 0, B: 0 })?)?;
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
