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

// Temporary Allows
#![allow(unused_imports)]
#![allow(unused_variables)]

mod coreaudio;
use coreaudio::Device;

fn handle_coreaudio_error(e: coreaudio::Error) {
    println!("Coreaudio Error: {:?}", e);
}

#[derive(Debug)]
pub(super) enum Error {
    Generic,
}

pub(super) struct AudioOwner {
    info: u64,
}

impl AudioOwner {
    pub(super) fn new() -> Option<Self> {
        Some(AudioOwner { info: 0 })
    }
}

pub(super) struct AudioOutput<'a> {
    owner: &'a AudioOwner,
    device: Device,
    frame_period: u32,
    //buffer_size: i64,
    channels: u32,
    //channel_mask: u32,
    //volume_control: Audio::ISimpleAudioVolume,
}

impl<'a> AudioOutput<'a> {
    pub(super) fn new(audio_owner: &'a AudioOwner, desired_period: u32) -> Option<Self> {
        //Open default playback device
        let device = match Device::new_from_default_playback(48000, desired_period) {
            Ok(o) => o,
            Err(e) => {
                handle_coreaudio_error(e);
                return None;
            }
        };

        let channels = match device.get_num_channels() {
            Ok(c) => c,
            Err(e) => {
                handle_coreaudio_error(e);
                return None;
            }
        };

        // if let Err(e) = device.print_stream_format() {
        //     handle_coreaudio_error(e);
        //     return None;
        // }

        // if let Err(e) = device.print_device_period() {
        //     handle_coreaudio_error(e);
        //     return None;
        // }

        // let mut callback_count = 0;
        // let mut f = move |samples: &mut [f32]| output_callback(samples, &mut callback_count);
        // if let Err(e) = device.run_output_callback_loop(&mut f) {
        //     handle_coreaudio_error(e);
        //     return None;
        // }

        // println!("Got Here!");

        Some(AudioOutput {
            owner: audio_owner,
            device,
            frame_period: desired_period,
            channels,
        })
    }

    pub(super) fn get_channels(&self) -> u32 {
        self.channels
    }

    pub(super) fn run_callback_loop(
        &self,
        mut callback: impl crate::OutputCallback + 'static,
    ) -> bool {
        let mut closure = move |samples: &mut [f32]| callback.output_callback(samples);
        self.device.run_output_callback_loop(&mut closure).is_ok()
    }
}

pub(super) struct AudioInput<'a> {
    owner: &'a AudioOwner,
    device: Device,
    frame_period: u32,
    //buffer_size: i64,
    channels: u32,
    //channel_mask: u32,
    //volume_control: Audio::ISimpleAudioVolume,
}

impl<'a> AudioInput<'a> {
    pub(super) fn new(
        audio_owner: &'a AudioOwner,
        desired_period: u32,
        channels: u32,
    ) -> Option<Self> {
        //Open default playback device
        let device = match Device::new_from_default_capture(48000, desired_period) {
            Ok(o) => o,
            Err(e) => {
                handle_coreaudio_error(e);
                return None;
            }
        };

        Some(AudioInput {
            owner: audio_owner,
            device,
            frame_period: desired_period,
            channels,
        })
    }

    pub(super) fn get_channels(&self) -> u32 {
        self.channels
    }

    pub(super) fn run_callback_loop(
        &self,
        mut callback: impl crate::InputCallback + 'static,
    ) -> bool {
        let mut closure = move |samples: &[f32]| callback.input_callback(samples);
        self.device
            .run_input_callback_loop(self.channels, &mut closure)
            .is_ok()
    }
}
