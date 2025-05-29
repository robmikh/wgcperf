use std::collections::HashMap;

use windows::{
    Win32::{
        Devices::Display::{
            DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
            DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME,
            DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS,
            QueryDisplayConfig,
        },
        Foundation::{LPARAM, RECT, WIN32_ERROR},
        Graphics::Gdi::{
            DEVMODEW, ENUM_CURRENT_SETTINGS, EnumDisplayMonitors, EnumDisplaySettingsW,
            GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
        },
    },
    core::{BOOL, PCWSTR, Result},
};

fn get_display_config_path_infos() -> Result<Vec<DISPLAYCONFIG_PATH_INFO>> {
    let mut num_paths = 0;
    let mut num_modes = 0;
    unsafe {
        GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut num_paths, &mut num_modes).ok()?
    };
    let mut path_infos = vec![DISPLAYCONFIG_PATH_INFO::default(); num_paths as usize];
    let mut mode_infos = vec![DISPLAYCONFIG_MODE_INFO::default(); num_modes as usize];
    unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut num_paths,
            path_infos.as_mut_ptr(),
            &mut num_modes,
            mode_infos.as_mut_ptr(),
            None,
        )
        .ok()?;
    }
    path_infos.resize(num_paths as usize, DISPLAYCONFIG_PATH_INFO::default());
    Ok(path_infos)
}

fn build_device_name_to_display_name_map() -> Result<HashMap<String, String>> {
    let path_infos = get_display_config_path_infos()?;
    let mut result = HashMap::new();
    for path_info in path_infos {
        // Get the device name
        let mut device_name = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                adapterId: path_info.sourceInfo.adapterId,
                id: path_info.sourceInfo.id,
            },
            ..Default::default()
        };
        unsafe {
            WIN32_ERROR(DisplayConfigGetDeviceInfo(&mut device_name.header) as u32).ok()?;
        }
        let len = device_name
            .viewGdiDeviceName
            .iter()
            .position(|x| *x == 0)
            .unwrap_or(device_name.viewGdiDeviceName.len());
        let name = String::from_utf16(&device_name.viewGdiDeviceName[..len])?;

        // Get the display name.
        let mut display_name_info = DISPLAYCONFIG_TARGET_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                size: std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                adapterId: path_info.targetInfo.adapterId,
                id: path_info.targetInfo.id,
            },
            ..Default::default()
        };
        unsafe {
            WIN32_ERROR(DisplayConfigGetDeviceInfo(&mut display_name_info.header) as u32).ok()?;
        }
        let len = display_name_info
            .monitorFriendlyDeviceName
            .iter()
            .position(|x| *x == 0)
            .unwrap_or(display_name_info.monitorFriendlyDeviceName.len());
        let display_name = String::from_utf16(&display_name_info.monitorFriendlyDeviceName[..len])?;

        result.insert(name, display_name);
    }
    Ok(result)
}

fn get_all_display_handles() -> Result<Vec<HMONITOR>> {
    unsafe {
        let mut display_handles = Vec::<HMONITOR>::new();
        EnumDisplayMonitors(
            None,
            None,
            Some(enum_display_monitors),
            LPARAM(&mut display_handles as *mut _ as isize),
        )
        .ok()?;
        Ok(display_handles)
    }
}

unsafe extern "system" fn enum_display_monitors(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let display_handles = unsafe { (lparam.0 as *mut Vec<HMONITOR>).as_mut().unwrap() };
    display_handles.push(monitor);
    true.into()
}

pub struct Monitor {
    _device_name: String,
    display_name: String,
    display_frequency: u32,
    handle: HMONITOR,
}

impl Monitor {
    pub fn enumerate_all() -> Result<Vec<Self>> {
        let device_name_to_display_name = build_device_name_to_display_name_map()?;
        let handles = get_all_display_handles()?;

        let mut monitors = Vec::with_capacity(handles.len());
        for handle in handles {
            // Get the monitor rect and device name.
            let mut monitor_info = MONITORINFOEXW::default();
            monitor_info.monitorInfo.cbSize = std::mem::size_of_val(&monitor_info) as u32;
            unsafe {
                GetMonitorInfoW(handle, &mut monitor_info.monitorInfo).ok()?;
            }
            let len = monitor_info
                .szDevice
                .iter()
                .position(|x| *x == 0)
                .unwrap_or(monitor_info.szDevice.len());
            let device_name = String::from_utf16(&monitor_info.szDevice[..len])?;

            let display_name = device_name_to_display_name
                .get(&device_name)
                .unwrap()
                .clone();

            let mut dev_mode = DEVMODEW::default();
            unsafe {
                EnumDisplaySettingsW(
                    PCWSTR(monitor_info.szDevice.as_ptr()),
                    ENUM_CURRENT_SETTINGS,
                    &mut dev_mode,
                )
                .ok()?
            }
            let display_frequency = dev_mode.dmDisplayFrequency;

            monitors.push(Monitor {
                _device_name: device_name,
                display_name,
                display_frequency,
                handle,
            });
        }

        Ok(monitors)
    }

    pub fn handle(&self) -> HMONITOR {
        self.handle
    }

    pub fn display_frequency(&self) -> u32 {
        self.display_frequency
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}
