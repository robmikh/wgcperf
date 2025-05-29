use windows::{
    Win32::Graphics::DirectWrite::{DWRITE_FACTORY_TYPE, DWriteCreateFactory, IDWriteFactory},
    core::Result,
};

pub fn create_dwrite_factory(factory_type: DWRITE_FACTORY_TYPE) -> Result<IDWriteFactory> {
    unsafe { DWriteCreateFactory(factory_type) }
}
