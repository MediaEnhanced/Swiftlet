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

    pub(super) fn run_callback_loop(&self, callback: &mut crate::OutputCallback) -> bool {
        self.device.run_output_callback_loop(callback).is_ok()
    }

    // Returns true if started
    pub(super) fn start(&self) -> bool {
        // Need to do an initial read to clear stuff based on documentation
        //self.device.start().is_ok()
        false
    }

    pub(super) fn stop(&self) -> bool {
        //self.device.stop().is_ok()
        false
    }

    pub(super) fn wait_for_next_output(
        &mut self,
        millisecond_timeout: u32,
    ) -> Result<Option<&mut [f32]>, Error> {
        // match self.device.wait_until_ready(millisecond_timeout as i32) {
        //     Ok(true) => {
        //         // Process Frames
        //         let available_frames = self.device.get_available_frames();

        //         //println!("Frames Available: {}", available_frames);
        //         if available_frames >= (self.frame_period as i64) {
        //             let float_p = self.data_vec.as_mut_ptr() as *mut f32;
        //             let buffer_len = (self.frame_period * self.channels) as usize;
        //             let buffer = unsafe { std::slice::from_raw_parts_mut(float_p, buffer_len) };
        //             Ok(Some(buffer))
        //         } else {
        //             Ok(None)
        //         }
        //     }
        //     Ok(false) => {
        //         // Timeout
        //         Ok(None)
        //     }
        //     Err(e) => {
        //         //handle_alsa_error(e);
        //         Err(Error::Generic)
        //     }
        // }

        Ok(None)
    }

    pub(super) fn release_output(&self) -> bool {
        // match self
        //     .device
        //     .write_interleaved_float_frames(&self.data_vec, self.frame_period as u64)
        // {
        //     Ok(frames) => {
        //         //println!("Frames Written: {}", frames);
        //         if frames == self.frame_period as u64 {
        //             true
        //         } else {
        //             false
        //         }
        //     }
        //     Err(e) => {
        //         //handle_alsa_error(e);
        //         false
        //     }
        // }

        false
    }
}

fn output_callback(samples: &mut [f32], callback_count: &mut u64) -> bool {
    *callback_count += 1;

    let samples_len = samples.len();
    println!("{}, Samples: {}", *callback_count, samples_len);

    // if samples_len != 960 {
    //     println!("{}, Samples: {}", *callback_count, samples_len);
    //     if samples_len == 0 {
    //         return true;
    //     }
    // }

    if *callback_count >= 20 {
        return true;
    }

    false
}

pub(super) struct AudioInput {
    //device: PCM,
}

impl AudioInput {
    //pub(super) fn new() -> Self {}
}
