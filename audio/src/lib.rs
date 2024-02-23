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

#![allow(dead_code)] // Temporary

pub mod raw;

#[cfg_attr(target_os = "windows", path = "windows.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "mac.rs")]
mod os;
use os::AudioDevice;

pub enum Error {
    DeviceCreation,
}

pub struct AudioIO {
    device: os::AudioDevice,
}

impl AudioIO {
    pub fn new() -> Result<Self, Error> {
        let device = match AudioDevice::new() {
            Some(d) => d,
            None => return Err(Error::DeviceCreation),
        };
        Ok(AudioIO { device })
    }

    pub fn create_output(&mut self, desired_period: u32) -> Option<u32> {
        self.device.create_or_edit_output(desired_period)
    }

    pub fn run_output_event_loop(&self, callback: &mut dyn FnMut(&mut [f32]) -> bool) -> bool {
        self.device.event_loop_output(callback)
    }
}
