//Media Enhanced Swiftlet Audio Rust Library for Low Latency Audio OS I/O
//MIT License
//Copyright (c) 2024 Jared Loewenthal
//
//Permission is hereby granted, free of charge, to any person obtaining a copy
//of this software and associated documentation files (the "Software"), to deal
//in the Software without restriction, including without limitation the rights
//to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//copies of the Software, and to permit persons to whom the Software is
//furnished to do so, subject to the following conditions:
//
//The above copyright notice and this permission notice shall be included in all
//copies or substantial portions of the Software.
//
//THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//SOFTWARE.

// use std::ffi::c_void;
// use std::fmt::Debug;
// use std::mem::size_of;
// use std::ptr;
// use windows::Win32::Foundation;
use windows::core::Error;
//use windows::Win32::Foundation::{BOOL, HANDLE, LUID};
// //use windows::Win32::Foundation::BOOL;
// use windows::core::PCWSTR;

use windows::Win32::Graphics::Dxgi;

pub(super) struct Interface {
    adapter: Dxgi::IDXGIAdapter1,
    adapter_description: Dxgi::DXGI_ADAPTER_DESC1,
}

impl Interface {
    pub(super) fn new(is_dedicated: bool) -> Result<Self, Error> {
        let factory_interface: Dxgi::IDXGIFactory6 = unsafe { Dxgi::CreateDXGIFactory1() }?;

        let gpu_preference = if !is_dedicated {
            Dxgi::DXGI_GPU_PREFERENCE_UNSPECIFIED
        } else {
            Dxgi::DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE
        };

        let adapter: Dxgi::IDXGIAdapter1 =
            unsafe { factory_interface.EnumAdapterByGpuPreference(0, gpu_preference) }?;

        let mut adapter_description = Dxgi::DXGI_ADAPTER_DESC1::default();
        unsafe { adapter.GetDesc1(&mut adapter_description) }?;

        Ok(Interface {
            adapter,
            adapter_description,
        })
    }

    pub(super) fn get_luid(&self) -> [u32; 2] {
        [
            self.adapter_description.AdapterLuid.LowPart,
            self.adapter_description.AdapterLuid.HighPart as u32,
        ]
    }
}
