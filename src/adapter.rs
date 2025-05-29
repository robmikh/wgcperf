use windows::{
    Win32::{Foundation::LUID, Graphics::Dxgi::IDXGIAdapter1},
    core::Result,
};

pub struct Adapter {
    pub name: String,
    pub luid: LUID,
}

impl Adapter {
    pub fn from_dxgi_adapter(adapter: &IDXGIAdapter1) -> Result<Self> {
        unsafe {
            let desc = adapter.GetDesc1()?;
            let luid = desc.AdapterLuid;

            let name_utf16 = &desc.Description[..desc
                .Description
                .iter()
                .position(|x| *x == 0)
                .unwrap_or(desc.Description.len())];
            let name = String::from_utf16(name_utf16)?;

            Ok(Self { name, luid })
        }
    }
}
