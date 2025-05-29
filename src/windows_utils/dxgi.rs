use windows::Win32::Graphics::Dxgi::{IDXGIAdapter1, IDXGIFactory1, IDXGIOutput};

pub struct DxgiAdapterIterator<'a> {
    factory: &'a IDXGIFactory1,
    current_adapter_index: u32,
}

pub trait DxgiAdapterIter {
    fn iter_adapters<'a>(&'a self) -> DxgiAdapterIterator<'a>;
}

impl<'a> Iterator for DxgiAdapterIterator<'a> {
    type Item = IDXGIAdapter1;

    fn next(&mut self) -> Option<Self::Item> {
        let result = unsafe { self.factory.EnumAdapters1(self.current_adapter_index).ok() };
        if result.is_some() {
            self.current_adapter_index += 1;
        }

        result
    }
}

impl DxgiAdapterIter for IDXGIFactory1 {
    fn iter_adapters<'a>(&'a self) -> DxgiAdapterIterator<'a> {
        DxgiAdapterIterator {
            factory: self,
            current_adapter_index: 0,
        }
    }
}

pub struct DxgiOutputIterator<'a> {
    adapter: &'a IDXGIAdapter1,
    current_output_index: u32,
}

pub trait DxgiOutputIter {
    fn iter_outputs<'a>(&'a self) -> DxgiOutputIterator<'a>;
}

impl<'a> Iterator for DxgiOutputIterator<'a> {
    type Item = IDXGIOutput;

    fn next(&mut self) -> Option<Self::Item> {
        let result = unsafe { self.adapter.EnumOutputs(self.current_output_index).ok() };
        if result.is_some() {
            self.current_output_index += 1;
        }

        result
    }
}

impl DxgiOutputIter for IDXGIAdapter1 {
    fn iter_outputs<'a>(&'a self) -> DxgiOutputIterator<'a> {
        DxgiOutputIterator {
            adapter: self,
            current_output_index: 0,
        }
    }
}
