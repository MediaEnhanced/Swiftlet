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

pub(super) struct AudioDevice {
    info: u64,
}

impl AudioDevice {
    pub(super) fn new() -> Option<Self> {
        Some(AudioDevice { info: 0 })
    }
}

pub(super) struct AudioOutput {
    device: Pcm,
    frame_period: u32,
    //buffer_size: i64,
    channels: u32,
    //channel_mask: u32,
    data_vec: Vec<f32>,
    //volume_control: Audio::ISimpleAudioVolume,
}

impl AudioOutput {
    pub(super) fn new(audio_device: &AudioDevice, desired_period: u32) -> Option<Self> {
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

        if hw_params
            .set_param(alsa::PcmHwParam::NearestPeriod(desired_period as u64))
            .is_err()
        {
            return None;
        }

        if hw_params.set_param(alsa::PcmHwParam::Channels(2)).is_err() {
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

        let num_floats = (desired_period as usize) * 2;

        Some(AudioOutput {
            device: pcm_device,
            frame_period: desired_period,
            channels: 2,
            data_vec: vec![0.0; num_floats],
        })
        // None

        // // Open default playback device
        // let pcm = PCM::new("default", Direction::Playback, false).unwrap();

        // let hwp = HwParams::any(&pcm).unwrap();
        // hwp.set_channels(2).unwrap();
        // hwp.set_rate(48000, ValueOr::Nearest).unwrap();
        // hwp.set_format(Format::float()).unwrap();
        // hwp.set_access(Access::RWInterleaved).unwrap();
        // pcm.hw_params(&hwp).unwrap();

        // // Make sure we don't start the stream too early
        // let hwp2 = pcm.hw_params_current().unwrap();
        // let swp = pcm.sw_params_current().unwrap();
        // swp.set_start_threshold(hwp2.get_buffer_size().unwrap())
        //     .unwrap();
        // pcm.sw_params(&swp).unwrap();

        // drop(swp);
        // drop(hwp2);
        // drop(hwp);

        // Some(AudioOutput {
        //     device: pcm,
        //     channels: 2,
        //     frame_period: desired_period,
        // })

        // let device = match PCM::new("default", Direction::Playback, true) {
        //     Ok(pcm) => pcm,
        //     Err(_) => return None,
        // };

        // // Fill params with a full configuration space for a PCM
        // // The configuration space will be filled with all possible ranges for the PCM device.
        // // Note that the configuration space may be constrained by the currently installed configuration on the PCM device.
        // let parameters = match HwParams::any(&device) {
        //     Ok(hwp) => hwp,
        //     Err(_) => return None,
        // };

        // // Restrict configuration space with the following sets...?
        // if parameters.set_channels(2).is_err() {
        //     return None;
        // }
        // // let channels = match parameters.get_channels() {
        // //     Ok(c) => c,
        // //     Err(_) => return None,
        // // };

        // if parameters.set_rate(48000, ValueOr::Nearest).is_err() {
        //     return None;
        // }
        // if parameters.set_format(Format::float()).is_err() {
        //     return None;
        // }
        // if parameters.set_access(Access::RWInterleaved).is_err() {
        //     return None;
        // }
        // if parameters
        //     .set_period_size_near((desired_period / 4) as Frames, ValueOr::Nearest)
        //     .is_err()
        // {
        //     return None;
        // }
        // if parameters
        //     .set_buffer_size(desired_period as Frames)
        //     .is_err()
        // {
        //     return None;
        // }

        // // Install one PCM hardware configuration chosen from a configuration space and snd_pcm_prepare it.
        // if device.hw_params(&parameters).is_err() {
        //     return None;
        // }

        // //let io = device.io_f32().unwrap();

        // // Retreive current PCM hardware configuration chosen with snd_pcm_hw_params
        // let hwp = match device.hw_params_current() {
        //     Ok(p) => p,
        //     Err(_) => return None,
        // };

        // // Return current software configuration for a PCM
        // let swp = match device.sw_params_current() {
        //     Ok(p) => p,
        //     Err(_) => return None,
        // };

        // // let buffer_size = match hwp.get_buffer_size() {
        // //     Ok(bs) => bs,
        // //     Err(_) => return None,
        // // };

        // // if swp.set_start_threshold(desired_period as i64).is_err() {
        // //     return None;
        // // }
        // // if device.sw_params(&swp).is_err() {
        // //     return None;
        // // }

        // drop(parameters);
        // drop(hwp);
        // drop(swp);

        // Some(AudioOutput {
        //     device,
        //     channels: 2,
        //     //buffer_size: ,
        //     frame_period: desired_period,
        // })
    }

    pub(super) fn get_channels(&self) -> u32 {
        self.channels
    }

    // Returns true if started
    pub(super) fn start(&self) -> bool {
        // Need to do an initial read to clear stuff based on documentation
        self.device.start().is_ok()
    }

    pub(super) fn stop(&self) -> bool {
        self.device.stop().is_ok()
    }

    pub(super) fn wait_for_next_output(
        &mut self,
        millisecond_timeout: u32,
    ) -> Result<Option<&mut [f32]>, Error> {
        match self.device.wait_until_ready(millisecond_timeout as i32) {
            Ok(true) => {
                // Process Frames
                let available_frames = self.device.get_available_frames();

                //println!("Frames Available: {}", available_frames);
                if available_frames >= (self.frame_period as i64) {
                    let float_p = self.data_vec.as_mut_ptr() as *mut f32;
                    let buffer_len = (self.frame_period * self.channels) as usize;
                    let buffer = unsafe { std::slice::from_raw_parts_mut(float_p, buffer_len) };
                    Ok(Some(buffer))
                } else {
                    Ok(None)
                }
            }
            Ok(false) => {
                // Timeout
                Ok(None)
            }
            Err(e) => {
                handle_alsa_error(e);
                //return false;
                Err(Error::Generic)
            }
        }
    }

    pub(super) fn release_output(&self) -> bool {
        match self
            .device
            .write_interleaved_float_frames(&self.data_vec, self.frame_period as u64)
        {
            Ok(frames) => {
                //println!("Frames Written: {}", frames);
                if frames == self.frame_period as u64 {
                    true
                } else {
                    false
                }
            }
            Err(e) => {
                handle_alsa_error(e);
                false
            }
        }
    }

    fn event_loop(&self, callback: &mut dyn FnMut(&mut [f32]) -> bool) -> bool {
        // // Make a sine wave
        // let mut buf = [0i16; 1024];
        // for (i, a) in buf.iter_mut().enumerate() {
        //     *a = ((i as f32 * 2.0 * ::std::f32::consts::PI / 128.0).sin() * 8192.0) as i16
        // }

        // let mut buf2 = [0i16; 1024];
        // for (i, a) in buf2.iter_mut().enumerate() {
        //     *a = ((i as f32 * 2.0 * ::std::f32::consts::PI / 128.0).sin() * 8192.0) as i16
        // }

        // let io = self.device.io_f32().unwrap();

        // let sample_float_divisor = (1 << (16 - 1)) as f32;
        // let sample_float_multiplier = 1.0 / sample_float_divisor;

        // let mut buf3 = [0.0; 2048];
        // let mut ind2 = 0;
        // for (i, a) in buf.iter().enumerate() {
        //     buf3[ind2] = (*a as f32) * sample_float_multiplier;
        //     //buf3[ind2 + 1] = (*a as f32) * sample_float_multiplier;
        //     buf3[ind2 + 1] = (buf2[i] as f32) * sample_float_multiplier;
        //     ind2 += 2;
        // }

        // // Play it back for 2 seconds.
        // for _ in 0..2 * 48000 / 1024 {
        //     assert_eq!(io.writei(&buf3[..]).unwrap(), 1024);
        // }

        // // In case the buffer was larger than 2 seconds, start the stream manually.
        // if self.device.state() != State::Running {
        //     self.device.start().unwrap()
        // };
        // // Wait for the stream to finish playback.
        // self.device.drain().unwrap();

        if !self.start() {
            return false;
        }
        handle_alsa_state(self.device.get_state());

        // let pollfd_blank = alsa::poll::pollfd {
        //     fd: 0,
        //     events: 0,
        //     revents: 0,
        // };

        // let mut pollfd_array = vec![pollfd_blank; self.device.count()];

        let num_floats = (self.frame_period as usize) * (self.channels as usize);
        let mut data_vec = vec![0.0; num_floats];

        loop {
            // In the loop...?
            // if self.device.fill(&mut pollfd_array).is_err() {
            //     return false;
            // }

            // let poll_res = match alsa::poll::poll(&mut pollfd_array, 100) {
            //     Ok(r) => r,
            //     Err(_) => return false,
            // };

            // if poll_res == 0 {
            //     // Weird spot...?
            //     return false;
            // }

            // if pollfd_array[0].revents != 0 {
            //     // Stream wants to be destroyed...?
            //     return false;
            // }

            // let revents = match self.device.revents(&pollfd_array) {
            //     Ok(re) => re,
            //     Err(_) => return false,
            // };
            // if revents.contains(alsa::poll::Flags::ERR) {
            //     return false;
            // }
            // if revents != alsa::poll::Flags::OUT {
            //     continue;
            // }

            match self.device.wait_until_ready(-1) {
                Ok(true) => {
                    // Process Frames
                }
                Ok(false) => {
                    continue;
                }
                Err(e) => {
                    handle_alsa_error(e);
                    return false;
                }
            }
            //handle_alsa_state();

            let available_frames = self.device.get_available_frames();

            //println!("Frames Available: {}", available_frames);
            if available_frames >= (self.frame_period as i64) {
                let float_p = data_vec.as_mut_ptr() as *mut f32;
                let buffer = unsafe { std::slice::from_raw_parts_mut(float_p, num_floats) };
                let callback_quit = callback(buffer);

                match self
                    .device
                    .write_interleaved_float_frames(&data_vec, self.frame_period as u64)
                {
                    Ok(frames) => {
                        //println!("Frames Written: {}", frames);
                        if frames < self.frame_period as u64 {
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

        if !self.stop() {
            return false;
        }
        handle_alsa_state(self.device.get_state());

        true
    }
}

pub(super) struct AudioInput {
    //device: PCM,
}

impl AudioInput {
    //pub(super) fn new() -> Self {}
}
