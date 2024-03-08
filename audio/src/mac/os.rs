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
use coreaudio::Object;

fn handle_coreaudio_error(e: coreaudio::Error) {
    println!("Coreaudio Error: {:?}", e);
}

#[derive(Debug)]
pub(super) enum Error {
    Generic,
}

pub(super) struct AudioDevice {
    info: u64,
}

impl AudioDevice {
    pub(super) fn new() -> Option<Self> {
        Some(AudioDevice { info: 0 })
    }
}

pub(super) struct AudioOutput {
    device: Object,
    frame_period: u32,
    //buffer_size: i64,
    channels: u32,
    //channel_mask: u32,
    data_vec: Vec<f32>,
    //volume_control: Audio::ISimpleAudioVolume,
}

impl AudioOutput {
    pub(super) fn new(audio_device: &AudioDevice, desired_period: u32) -> Option<Self> {
        //Open default playback device
        let _device = match Object::new_from_default_playback() {
            Ok(o) => o,
            Err(e) => {
                handle_coreaudio_error(e);
                return None;
            }
        };

        // handle_alsa_state(pcm_device.get_state());

        // let hw_params = match alsa::PcmHwParams::any_from_pcm(&pcm_device) {
        //     Ok(p) => p,
        //     Err(e) => {
        //         handle_alsa_error(e);
        //         return None;
        //     }
        // };

        // if hw_params
        //     .set_param(alsa::PcmHwParam::NearestRate(48000))
        //     .is_err()
        // {
        //     return None;
        // }
        // if hw_params.set_param(alsa::PcmHwParam::FormatFloat).is_err() {
        //     return None;
        // }
        // if hw_params
        //     .set_param(alsa::PcmHwParam::BufferInterleaved)
        //     .is_err()
        // {
        //     return None;
        // }

        // if hw_params
        //     .set_param(alsa::PcmHwParam::NearestPeriod(desired_period as u64))
        //     .is_err()
        // {
        //     return None;
        // }

        // if hw_params.set_param(alsa::PcmHwParam::Channels(2)).is_err() {
        //     return None;
        // }
        // if pcm_device.set_hw_params(&hw_params).is_err() {
        //     return None;
        // }
        // drop(hw_params); // Manual Drop Necessary...?

        // handle_alsa_state(pcm_device.get_state());

        // // let hw_params = match als::PcmHwParams::current_from_pcm(&pcm_device) {
        // //     Ok(p) => p,
        // //     Err(e) => {
        // //         handle_alsa_error(e);
        // //         return None;
        // //     }
        // // };

        // // let sw_params = match als::PcmSwParams::current_from_pcm(&pcm_device) {
        // //     Ok(p) => p,
        // //     Err(e) => {
        // //         handle_alsa_error(e);
        // //         return None;
        // //     }
        // // };

        // let num_floats = (desired_period as usize) * 2;

        // Some(AudioOutput {
        //     device: pcm_device,
        //     frame_period: desired_period,
        //     channels: 2,
        //     data_vec: vec![0.0; num_floats],
        // })
        None
    }

    pub(super) fn get_channels(&self) -> u32 {
        self.channels
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

pub(super) struct AudioInput {
    //device: PCM,
}

impl AudioInput {
    //pub(super) fn new() -> Self {}
}
