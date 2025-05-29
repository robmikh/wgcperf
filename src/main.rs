mod adapter;
mod monitor;
mod pdh;
mod perf;
mod perf_session;
mod pid;
mod sinks;
mod window;
mod windows_utils;

use std::{sync::mpsc::channel, time::Duration};

use adapter::Adapter;
use monitor::Monitor;
use perf_session::PerfSession;
use pid::get_current_dwm_pid;
use sinks::{CaptureSink, dda::DdaCaptureSink, wgc::WgcCaptureSink};
use window::Window;
use windows::{
    System::{DispatcherQueue, DispatcherQueueController, DispatcherQueueHandler},
    UI::{
        Color,
        Composition::{AnimationIterationBehavior, Core::CompositorController},
    },
    Win32::{
        Foundation::HWND,
        Graphics::{
            Dxgi::{CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory1, IDXGIOutput1},
            Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromWindow},
        },
        System::{
            WinRT::{RO_INIT_MULTITHREADED, RoInitialize},
            WindowsProgramming::MulDiv,
        },
        UI::HiDpi::{
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI,
            SetProcessDpiAwarenessContext,
        },
    },
    core::{Interface, Result, h},
};
use windows_numerics::{Vector2, Vector3};
use windows_utils::{
    composition::CompositionInterop,
    d3d::create_d3d_device_on_adapter,
    dxgi::{DxgiAdapterIter, DxgiOutputIter},
};

fn main() -> Result<()> {
    // TODO: Parse args

    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)?;
    }
    unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };

    // TODO: From args
    let monitors = Monitor::enumerate_all()?;
    let monitor_handle =
        unsafe { MonitorFromWindow(HWND(std::ptr::null_mut()), MONITOR_DEFAULTTOPRIMARY) };
    let monitor_info = unsafe {
        let mut info = MONITORINFO::default();
        info.cbSize = std::mem::size_of_val(&info) as u32;
        GetMonitorInfoW(monitor_handle, &mut info).ok()?;
        info
    };
    let work_area = monitor_info.rcWork;
    let monitor_index = monitors
        .iter()
        .position(|x| x.handle() == monitor_handle)
        .expect("Failed to find monitor information!");
    let monitor = &monitors[monitor_index];
    println!("Monitor details:");
    println!("  index: {}", monitor_index);
    println!("  handle: {:010X}", monitor_handle.0 as usize);
    println!("  name: {}", monitor.display_name());
    println!("  frequency: {} Hz", monitor.display_frequency());
    println!();

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

    // Create the UI thread
    let ui_thread = DispatcherQueueController::CreateOnDedicatedThread()?;
    let ui_queue = ui_thread.DispatcherQueue()?;

    // Create our dummy window
    let window = {
        let (sender, receiver) = channel();
        ui_queue.TryEnqueue(&DispatcherQueueHandler::new(move || -> Result<()> {
            let result = Window::new(
                "Dummy Content",
                window_x,
                window_y,
                window_width as u32,
                window_height as u32,
            );
            sender.send(result).unwrap();
            Ok(())
        }))?;
        let window = receiver.recv().unwrap()?;
        window
    };

    // Create our dummy content
    let compositor_controller = {
        let (sender, receiver) = channel();
        ui_queue.TryEnqueue(&DispatcherQueueHandler::new(move || -> Result<()> {
            let result = CompositorController::new();
            sender.send(result).unwrap();
            Ok(())
        }))?;
        let compositor_controller = receiver.recv().unwrap()?;
        compositor_controller
    };
    let compositor = compositor_controller.Compositor()?;
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
    compositor_controller.Commit()?;

    // Show the window
    window.show();

    // Initialize D3D
    let dxgi_factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1()? };
    let dxgi_adapters: Vec<IDXGIAdapter1> = dxgi_factory.iter_adapters().collect();
    let (adapter, output) = dxgi_adapters
        .iter()
        .find_map(|adapter| {
            if let Some(output) = adapter.iter_outputs().find(|output| {
                if let Ok(desc) = unsafe { output.GetDesc() } {
                    desc.Monitor == monitor_handle
                } else {
                    false
                }
            }) {
                Some((adapter.clone(), output))
            } else {
                None
            }
        })
        .expect("Couldn't find the adapter for the given monitor!");
    let d3d_device = create_d3d_device_on_adapter(&adapter)?;

    // TODO: Configurable
    let test_duration = Duration::from_secs(5);
    let rest_duration = Duration::from_secs(1);
    let verbose = false;
    let _output_dir = std::env::current_dir().unwrap();

    // Get the DWM's pid
    let pid = get_current_dwm_pid()?;

    // Collect all adapters
    let adapters = {
        let mut adapters = Vec::with_capacity(dxgi_adapters.len());
        for dxgi_adapter in &dxgi_adapters {
            adapters.push(Adapter::from_dxgi_adapter(&dxgi_adapter)?);
        }
        adapters
    };
    println!("Adapters:");
    for (i, adapter) in adapters.iter().enumerate() {
        println!("  {} - {}", i, adapter.name);
    }
    println!();

    // Record baseline
    println!("Recording baseline...");
    let baseline_samples = run_test(&ui_queue, test_duration, pid, &adapters, verbose)?;
    print_averages(&adapters, &baseline_samples);
    println!();

    // Record WGC
    println!("Recording WGC...");
    let mut wgc_sink = WgcCaptureSink::new(&d3d_device, monitor_handle)?;
    let _wgc_samples = run_and_print_test(
        &mut wgc_sink,
        &ui_queue,
        test_duration,
        rest_duration,
        pid,
        &adapters,
        verbose,
    )?;

    // Record DDA
    println!("Recording DDA...");
    let output: IDXGIOutput1 = output.cast()?;
    let mut dda_sink = DdaCaptureSink::new(&d3d_device, output)?;
    let _dda_samples = run_and_print_test(
        &mut dda_sink,
        &ui_queue,
        test_duration,
        rest_duration,
        pid,
        &adapters,
        verbose,
    )?;

    // Shut down the UI thread and the window
    window.close();
    ui_thread.ShutdownQueueAsync()?.get()?;

    // TODO: Save samples

    Ok(())
}

fn run_test(
    thread: &DispatcherQueue,
    duration: Duration,
    pid: u32,
    adapters: &[Adapter],
    verbose: bool,
) -> Result<Vec<(f64, Vec<f64>)>> {
    let mut result = Vec::with_capacity(adapters.len());
    let samples = PerfSession::run_on_thread(&thread, duration, pid, &adapters, verbose)?;
    for samples in samples {
        let average = if !samples.is_empty() {
            let sum: f64 = samples.iter().sum();
            sum / samples.len() as f64
        } else {
            0.0
        };
        result.push((average, samples));
    }
    Ok(result)
}

fn print_averages(adapters: &[Adapter], adapter_samples: &[(f64, Vec<f64>)]) {
    println!("Average GPU 3D engine utilization by adapter:");
    for (i, (adapter, (utilization, _))) in adapters.iter().zip(adapter_samples).enumerate() {
        println!("  {} - {:6.2}% - {}", i, utilization, adapter.name);
    }
}

fn run_and_print_test<C: CaptureSink>(
    sink: &mut C,
    thread: &DispatcherQueue,
    test_duration: Duration,
    rest_duration: Duration,
    pid: u32,
    adapters: &[Adapter],
    verbose: bool,
) -> Result<Vec<(f64, Vec<f64>)>> {
    sink.start()?;
    let samples = run_test(&thread, test_duration, pid, &adapters, verbose)?;
    sink.stop()?;
    print_averages(&adapters, &samples);
    println!();
    std::thread::sleep(rest_duration);
    Ok(samples)
}
