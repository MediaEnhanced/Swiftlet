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

mod wasapi;

pub(super) struct AudioOwner {
    owner: wasapi::ComOwner,
}

impl AudioOwner {
    pub(super) fn new() -> Option<Self> {
        let owner = wasapi::ComOwner::new()?;

        Some(AudioOwner { owner })
    }
}

pub(super) struct AudioOutput<'a> {
    owner: &'a AudioOwner,
    device: wasapi::OutputDevice,
}

impl<'a> AudioOutput<'a> {
    pub(super) fn new(audio_owner: &'a AudioOwner, desired_period: u32) -> Option<Self> {
        let device = match wasapi::OutputDevice::new(desired_period) {
            Some(d) => d,
            None => return None,
        };

        let audio_output = AudioOutput {
            owner: audio_owner,
            device,
        };

        Some(audio_output)
    }

    pub(super) fn get_channels(&self) -> u32 {
        self.device.get_channels()
    }

    pub(super) fn run_callback_loop(&self, callback: impl crate::OutputCallback) -> bool {
        self.device.run_output_event_loop(callback)
    }
}

pub(super) struct AudioInput<'a> {
    owner: &'a AudioOwner,
    device: wasapi::InputDevice,
}

impl<'a> AudioInput<'a> {
    pub(super) fn new(
        audio_owner: &'a AudioOwner,
        desired_period: u32,
        channels: u32,
    ) -> Option<Self> {
        let device = match wasapi::InputDevice::new(desired_period, channels) {
            Some(d) => d,
            None => return None,
        };

        let audio_input = AudioInput {
            owner: audio_owner,
            device,
        };

        Some(audio_input)
    }

    pub(super) fn get_channels(&self) -> u32 {
        self.device.get_channels()
    }

    pub(super) fn run_callback_loop(&self, callback: impl crate::InputCallback) -> bool {
        self.device.run_input_event_loop(callback)
    }
}
