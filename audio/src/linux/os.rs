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

mod alsa;
use alsa::{Pcm, PcmState};

fn handle_alsa_error(e: alsa::Error) {
    match e {
        alsa::Error::Generic((num, s)) => {
            println!("Alsa Generic Error: {}; {}", num, s);
        }
        alsa::Error::StringCreation(num) => {
            println!("Alsa String Creation Error: {}", num);
        }
    }
}

fn handle_alsa_state(s: PcmState) {
    //println!("Alsa PCM State: {:?}", s);
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
    device: Pcm,
    frame_period: u32,
    channels: u32,
    //channel_mask: u32,
    //volume_control: Audio::ISimpleAudioVolume,
}

impl<'a> AudioOutput<'a> {
    pub(super) fn new(audio_owner: &'a AudioOwner, desired_period: u32) -> Option<Self> {
        // Open default playback device
        let pcm_device = match alsa::Pcm::new_from_default_playback() {
            Ok(p) => p,
            Err(e) => {
                handle_alsa_error(e);
                return None;
            }
        };

        handle_alsa_state(pcm_device.get_state());

        let hw_params = match alsa::PcmHwParams::any_from_pcm(&pcm_device) {
            Ok(p) => p,
            Err(e) => {
                handle_alsa_error(e);
                return None;
            }
        };

        if hw_params
            .set_param(alsa::PcmHwParam::NearestRate(48000))
            .is_err()
        {
            return None;
        }
        if hw_params.set_param(alsa::PcmHwParam::FormatFloat).is_err() {
            return None;
        }
        if hw_params
            .set_param(alsa::PcmHwParam::BufferInterleaved)
            .is_err()
        {
            return None;
        }

        if hw_params.set_param(alsa::PcmHwParam::Channels(2)).is_err() {
            return None;
        }

        if hw_params
            .set_param(alsa::PcmHwParam::NearestPeriod(desired_period as u64))
            .is_err()
        {
            return None;
        }

        if hw_params
            .set_param(alsa::PcmHwParam::NearestBufferSize(desired_period as u64))
            .is_err()
        {
            return None;
        }

        if pcm_device.set_hw_params(&hw_params).is_err() {
            return None;
        }
        drop(hw_params); // Manual Drop Necessary...?

        handle_alsa_state(pcm_device.get_state());

        // let hw_params = match als::PcmHwParams::current_from_pcm(&pcm_device) {
        //     Ok(p) => p,
        //     Err(e) => {
        //         handle_alsa_error(e);
        //         return None;
        //     }
        // };

        // let sw_params = match als::PcmSwParams::current_from_pcm(&pcm_device) {
        //     Ok(p) => p,
        //     Err(e) => {
        //         handle_alsa_error(e);
        //         return None;
        //     }
        // };

        // drop(sw_params);

        let num_floats = (desired_period as usize) * 2;

        Some(AudioOutput {
            owner: audio_owner,
            device: pcm_device,
            frame_period: desired_period,
            channels: 2,
        })
    }

    pub(super) fn get_channels(&self) -> u32 {
        self.channels
    }

    // Returns true if started
    fn start(&self) -> bool {
        // Need to do an initial read to clear stuff based on documentation
        self.device.start().is_ok()
    }

    fn stop(&self) -> bool {
        self.device.stop().is_ok()
    }

    pub(super) fn run_callback_loop(&self, mut callback: impl crate::OutputCallback) -> bool {
        let buffer_len = (self.frame_period * self.channels) as usize;
        let mut data_vec = vec![0.0 as f32; buffer_len];

        if !self.start() {
            return false;
        }
        loop {
            match self.device.wait_until_ready(15) {
                Ok(true) => {
                    // Process Frames
                    let available_frames = self.device.get_available_frames();
                    //println!("Frames Available: {}", available_frames);
                    if available_frames >= (self.frame_period as i64) {
                        let callback_quit = callback.output_callback(&mut data_vec);
                        match self
                            .device
                            .write_interleaved_float_frames(&data_vec, self.frame_period as u64)
                        {
                            Ok(frames) => {
                                //println!("Frames Written: {}", frames);
                                if frames != self.frame_period as u64 {
                                    return false;
                                }
                            }
                            Err(e) => {
                                handle_alsa_error(e);
                                return false;
                            }
                        }
                        if callback_quit {
                            break;
                        }
                    }
                }
                Ok(false) => {
                    // Timeout
                }
                Err(e) => {
                    // Alsa Wait Error
                    handle_alsa_error(e);
                    return false;
                }
            }
        }

        self.stop()
    }
}

pub(super) struct AudioInput<'a> {
    owner: &'a AudioOwner,
    device: Pcm,
    frame_period: u32,
    channels: u32,
}

impl<'a> AudioInput<'a> {
    pub(super) fn new(
        audio_owner: &'a AudioOwner,
        desired_period: u32,
        channels: u32,
    ) -> Option<Self> {
        // Open default playback device
        let pcm_device = match alsa::Pcm::new_from_default_capture() {
            Ok(p) => p,
            Err(e) => {
                handle_alsa_error(e);
                return None;
            }
        };

        handle_alsa_state(pcm_device.get_state());

        let hw_params = match alsa::PcmHwParams::any_from_pcm(&pcm_device) {
            Ok(p) => p,
            Err(e) => {
                handle_alsa_error(e);
                return None;
            }
        };

        if hw_params
            .set_param(alsa::PcmHwParam::NearestRate(48000))
            .is_err()
        {
            return None;
        }
        if hw_params.set_param(alsa::PcmHwParam::FormatFloat).is_err() {
            return None;
        }
        if hw_params
            .set_param(alsa::PcmHwParam::BufferInterleaved)
            .is_err()
        {
            return None;
        }

        if hw_params.set_param(alsa::PcmHwParam::Channels(1)).is_err() {
            return None;
        }

        if hw_params
            .set_param(alsa::PcmHwParam::NearestPeriod(desired_period as u64))
            .is_err()
        {
            return None;
        }

        if hw_params
            .set_param(alsa::PcmHwParam::NearestBufferSize(desired_period as u64))
            .is_err()
        {
            return None;
        }

        if pcm_device.set_hw_params(&hw_params).is_err() {
            return None;
        }
        drop(hw_params); // Manual Drop Necessary...?

        Some(AudioInput {
            owner: audio_owner,
            device: pcm_device,
            frame_period: desired_period,
            channels: 1,
        })
    }

    pub(super) fn get_channels(&self) -> u32 {
        self.channels
    }

    // Returns true if started
    fn start(&self) -> bool {
        // Need to do an initial read to clear stuff based on documentation
        self.device.start().is_ok()
    }

    fn stop(&self) -> bool {
        self.device.stop().is_ok()
    }

    pub(super) fn run_callback_loop(&self, mut callback: impl crate::InputCallback) -> bool {
        let buffer_len = (self.frame_period * self.channels) as usize;
        let mut data_vec = vec![0.0 as f32; buffer_len];

        if !self.start() {
            return false;
        }
        loop {
            match self.device.wait_until_ready(15) {
                Ok(true) => {
                    // Process Frames
                    let available_frames = self.device.get_available_frames();
                    //println!("Avail Frames: {}", available_frames);
                    match self
                        .device
                        .read_interleaved_float_frames(&mut data_vec, self.frame_period as u64)
                    {
                        Ok(frames) => {
                            if frames == self.frame_period as u64 {
                                if callback.input_callback(&data_vec) {
                                    break;
                                }
                            } else {
                                return false;
                            }
                        }
                        Err(e) => {
                            println!("False!");
                            handle_alsa_error(e);
                            return false;
                        }
                    }
                }
                Ok(false) => {}
                Err(e) => {
                    // Alsa Wait Error
                    handle_alsa_error(e);
                    return false;
                }
            }
        }

        self.stop()
    }
}
