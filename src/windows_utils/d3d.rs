use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN;
use windows::Win32::Graphics::Direct3D11::D3D11_CREATE_DEVICE_DEBUG;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
    D3D11CreateDevice, ID3D11Device,
};
use windows::Win32::Graphics::Dxgi::{IDXGIAdapter1, IDXGIDevice};
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use windows::core::{Interface, Result};

fn create_d3d_device_with_adapter(
    adapter: &IDXGIAdapter1,
    flags: D3D11_CREATE_DEVICE_FLAG,
    device: *mut Option<ID3D11Device>,
) -> Result<()> {
    unsafe {
        D3D11CreateDevice(
            adapter,
            D3D_DRIVER_TYPE_UNKNOWN,
            HMODULE(std::ptr::null_mut()),
            flags,
            None,
            D3D11_SDK_VERSION,
            Some(device),
            None,
            None,
        )
    }
}

pub fn create_d3d_device_on_adapter(adapter: &IDXGIAdapter1) -> Result<ID3D11Device> {
    let mut device = None;
    let flags = {
        let mut flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
        if cfg!(feature = "dxdebug") {
            flags |= D3D11_CREATE_DEVICE_DEBUG;
        }
        flags
    };
    create_d3d_device_with_adapter(adapter, flags, &mut device)?;
    Ok(device.unwrap())
}

pub fn create_direct3d_device(d3d_device: &ID3D11Device) -> Result<IDirect3DDevice> {
    let dxgi_device: IDXGIDevice = d3d_device.cast()?;
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)? };
    inspectable.cast()
}
