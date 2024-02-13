//Media Enhanced Swiftlet Binaural Demo Example
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

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use swiftlet_binaural::hrtf::Hrtf;
use swiftlet_binaural::ild::Ild;

const HRTF_PATH: &str = "resources/hrtf/IRC1008.3dti-hrtf"; // Location of the HRTF Data
const ILD_PATH: &str = "resources/ILD/HRTF_ILD_48000.3dti-ild"; // Location of the ILD Data
const NFC_ILD_PATH: &str = "resources/ILD/NFC_ILD_48000.3dti-ild"; // Location of the Near Field Compensation ILD Data
const WAV_PATH: &str = "resources/speech.wav"; // Location of the mono speech WAV data

fn main() {
    println!("Binaural Demo Started!");

    let _hrtf = match std::fs::read(std::path::Path::new(HRTF_PATH)) {
        Ok(bytes) => match Hrtf::new_from_3dti_data(&bytes) {
            Some(h) => h,
            None => {
                println!("Could not create hrtf!");
                return;
            }
        },
        Err(_) => {
            println!("Could not find hrtf file!");
            return;
        }
    };

    let _ild = match std::fs::read(std::path::Path::new(NFC_ILD_PATH)) {
        Ok(bytes) => match Ild::new_from_3dti_data(&bytes) {
            Some(i) => i,
            None => {
                println!("Could not create ild!");
                return;
            }
        },
        Err(_) => {
            println!("Could not find ild file!");
            return;
        }
    };

    let raw_audio = match std::fs::read(std::path::Path::new(WAV_PATH)) {
        Ok(bytes) => match RawAudio::new_from_wav(&bytes) {
            Some(ra) => ra,
            None => {
                println!("Could not create raw audio!");
                return;
            }
        },
        Err(_) => {
            println!("Could not find wav file!");
            return;
        }
    };

    // if raw_audio.is_ideal_sample_rate() {
    //     if let Some(stereo_data) = raw_audio.get_stereo() {
    //         println!("Stereo Data Len: {}", stereo_data.len());
    //         if let Some(_audio_playback) = AudioPlayback::play_stereo(stereo_data) {
    //             //audio_playback.wait_til_done();
    //             std::thread::sleep(std::time::Duration::from_secs(5));
    //         }
    //     }
    // }
    // std::thread::sleep(std::time::Duration::from_secs(5));

    if raw_audio.is_ideal_sample_rate() {
        let mut audio_io = match swiftlet_audio::AudioIO::new() {
            Ok(a) => a,
            Err(_) => {
                println!("Could not create Audio IO!");
                return;
            }
        };
        match audio_io.create_output() {
            Some(channels) => {
                if channels == 2 {
                    //println!("Got Here!");
                    if let Some(stereo_data) = raw_audio.get_stereo() {
                        //println!("Got Here!");
                        let mut stereo_position = 0;
                        let mut callback_count = 0;
                        let mut f = move |samples: &mut [f32]| {
                            output_callback(
                                samples,
                                &mut stereo_position,
                                &stereo_data,
                                &mut callback_count,
                            )
                        };

                        audio_io.run_output_event_loop(&mut f);
                    }
                }
            }
            None => {
                println!("Could not create Audio IO Output!");
                return;
            }
        }
    }

    println!("Binaural Demo Ending!");
}

const MAX_CHANNEL_COUNT: usize = 8;

struct RawAudio {
    samples_per_sec: u32,
    channel_count: usize,
    data: [Vec<f32>; MAX_CHANNEL_COUNT], // Vecs should NOT have any heap allocation when using a zero-length new
    channel_format: u64,
}

impl RawAudio {
    // A return of None indicates bad input data
    fn new_from_wav(d: &[u8]) -> Option<Self> {
        // Make sure there is a minimal amount of data
        if d.len() < 8 {
            return None;
        }

        let riff_id = [b'R', b'I', b'F', b'F']; // Magic Number Pattern (0x46464952 LE)
        if d[0..4] != riff_id {
            return None;
        }

        let riff_chunk_size = u32::from_ne_bytes([d[4], d[5], d[6], d[7]]) as usize;
        if d.len() < (8 + riff_chunk_size) {
            return None;
        }
        // if data was in valid .wav format the data length is good at this point!

        let wave_id = [b'W', b'A', b'V', b'E']; // Magic Number Pattern (0x45564157 LE)
        if d[8..12] != wave_id {
            return None;
        }

        let wave_fmt_id = [b'f', b'm', b't', b' ']; // Magic Number Pattern (0x20746D66 LE)
        if d[12..16] != wave_fmt_id {
            return None;
        }

        let wave_fmt_len = u32::from_ne_bytes([d[16], d[17], d[18], d[19]]) as usize;
        let wave_fmt_tag = u16::from_ne_bytes([d[20], d[21]]);
        // The only currently supported tag is PCM (1) and it expects a length of 16
        if (wave_fmt_tag != 1) || (wave_fmt_len != 16) {
            return None;
        }

        let channel_count = u16::from_ne_bytes([d[22], d[23]]) as usize;
        if (channel_count > MAX_CHANNEL_COUNT) || (channel_count == 0) {
            return None;
        }

        let samples_per_sec = u32::from_ne_bytes([d[24], d[25], d[26], d[27]]); // Sampling Frequency
        let _avg_bytes_per_sec = u32::from_ne_bytes([d[28], d[29], d[30], d[31]]);
        let block_align = u16::from_ne_bytes([d[32], d[33]]);

        let mut d_pos = 34;
        // Specific to PCM Wave format (indicated by tag)
        let bits_per_sample = u16::from_ne_bytes([d[d_pos], d[d_pos + 1]]);
        d_pos += 2;

        let bytes_per_sample = ((bits_per_sample + 7) >> 3) as usize;
        let frame_bytes = block_align as usize;
        if frame_bytes != (bytes_per_sample * channel_count) {
            return None;
        }
        let sample_float_divisor = (1 << (bits_per_sample - 1)) as f32;
        println!("Float Divisor: {}", sample_float_divisor);
        let sample_float_multiplier = 1.0 / sample_float_divisor;

        while d_pos < d.len() {
            let next_id = [d[d_pos], d[d_pos + 1], d[d_pos + 2], d[d_pos + 3]];
            let next_len =
                u32::from_ne_bytes([d[d_pos + 4], d[d_pos + 5], d[d_pos + 6], d[d_pos + 7]])
                    as usize;
            d_pos += 8;

            if next_id == [b'd', b'a', b't', b'a'] {
                let num_frames = next_len / frame_bytes;
                let mut data = match channel_count {
                    1 => [
                        Vec::with_capacity(num_frames),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                    ],
                    2 => [
                        Vec::with_capacity(num_frames),
                        Vec::with_capacity(num_frames),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                    ],
                    _ => return None,
                };
                for _ in 0..num_frames {
                    for c in data.iter_mut().take(channel_count) {
                        let sample = match bytes_per_sample {
                            1 => i8::from_ne_bytes([d[d_pos]]) as f32,
                            2 => i16::from_ne_bytes([d[d_pos], d[d_pos + 1]]) as f32,
                            3 => {
                                (i32::from_ne_bytes([0, d[d_pos], d[d_pos + 1], d[d_pos + 2]]) >> 8)
                                    as f32
                            }
                            4 => i32::from_ne_bytes([
                                d[d_pos],
                                d[d_pos + 1],
                                d[d_pos + 2],
                                d[d_pos + 3],
                            ]) as f32,
                            _ => return None,
                        };
                        // Need to handle unusual bits_per_sample here in future
                        let sample_float = sample * sample_float_multiplier;

                        c.push(sample_float);

                        d_pos += bytes_per_sample;
                    }
                }

                println!("WAV Ending Position: {}", d_pos);
                println!("Channel Len: {}", data[0].len());

                return Some(RawAudio {
                    samples_per_sec,
                    channel_count,
                    data,
                    channel_format: 1,
                });
            } else {
                d_pos += next_len;
            }
        }

        None
    }

    fn is_ideal_sample_rate(&self) -> bool {
        self.samples_per_sec == 48000
    }

    fn get_stereo(&self) -> Option<Vec<f32>> {
        if self.channel_count == 1 {
            let mut stereo = Vec::with_capacity(self.data[0].len() * 2);
            for d in &self.data[0] {
                stereo.push(*d);
                stereo.push(*d);
            }
            Some(stereo)
        } else if self.channel_count == 2 {
            let num_samples = self.data[0].len();
            if num_samples != self.data[1].len() {
                return None;
            }
            let mut stereo = Vec::with_capacity(num_samples * 2);
            for i in 0..num_samples {
                stereo.push(self.data[0][i]);
                stereo.push(self.data[1][i]);
            }
            Some(stereo)
        } else {
            None
        }
    }
}

fn output_callback(
    samples: &mut [f32],
    stereo_position: &mut usize,
    stereo_data: &Vec<f32>,
    callback_count: &mut u64,
) -> bool {
    *callback_count += 1;

    if *callback_count % 100 == 0 {
        println!("Sec: {}", *callback_count / 100);
    }

    let samples_len = samples.len();
    if samples_len != 960 {
        println!("{}, Samples: {}", *callback_count, samples_len);
        if samples_len == 0 {
            return true;
        }
    }

    let remaining_samples = stereo_data.len() - *stereo_position;
    let copy_len = usize::min(remaining_samples, samples_len);
    let end_position = *stereo_position + copy_len;
    samples[..copy_len].copy_from_slice(&stereo_data[*stereo_position..end_position]);
    *stereo_position = end_position;
    if *stereo_position == stereo_data.len() {
        for s in &mut samples[copy_len..] {
            *s = 0.0;
        }
        *stereo_position = 0;
        return true;
    }

    false
}

struct AudioPlayback {
    stream: cpal::Stream,
}

impl AudioPlayback {
    fn play_stereo(stereo_data: Vec<f32>) -> Option<Self> {
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            _ => {
                return None;
            }
        };

        let mut supported_configs_range = match device.supported_output_configs() {
            Ok(scr) => scr,
            Err(_) => {
                return None;
            }
        };

        let config = supported_configs_range
            .find(|c| c.max_sample_rate().0 == 48000 && c.min_sample_rate().0 == 48000);

        config.as_ref()?;

        let config = config.unwrap();

        if config.sample_format() != cpal::SampleFormat::F32 {
            println!("Supported Config Format is Not F32");
            return None;
        }

        if config.channels() != 2 {
            println!("Supported Config is Not Stereo");
            return None;
        }

        match config.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => {
                if 480 >= *min && 480 <= *max {
                    println!("480 Buffer supported!");
                } else {
                    println!("Supported Config is Buffer Unknown");
                    return None;
                }
            }
            cpal::SupportedBufferSize::Unknown => {
                println!("Supported Config is Buffer Unknown");
                return None;
            }
        }

        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate: cpal::SampleRate(48000),
            buffer_size: cpal::BufferSize::Fixed(480),
        };

        let mut stereo_position = 0;
        let mut callback_count = 0;

        let stream_result = device.build_output_stream(
            &config,
            move |samples, info| {
                AudioPlayback::stereo_callback(
                    samples,
                    info,
                    &mut stereo_position,
                    &stereo_data,
                    &mut callback_count,
                )
            },
            AudioPlayback::error_callback,
            None,
        );

        match stream_result {
            Ok(stream) => match stream.play() {
                Ok(_) => Some(AudioPlayback { stream }),
                Err(_) => None,
            },
            Err(err) => {
                match err {
                    cpal::BuildStreamError::DeviceNotAvailable => {
                        println!("Audio Output Device Not Available!");
                    }
                    cpal::BuildStreamError::StreamConfigNotSupported => {
                        println!("Audio Output Stream Config Not Supported!");
                    }
                    cpal::BuildStreamError::InvalidArgument => {
                        println!("Audio Output Device Creation Invalid Argument!");
                    }
                    cpal::BuildStreamError::StreamIdOverflow => {
                        println!("Audio Output Stream Config Not Supported!");
                    }
                    cpal::BuildStreamError::BackendSpecific { err } => {
                        println!("Audio Output Backend Specific: {}", err);
                    }
                }
                None
            }
        }
    }

    fn wait_til_done(&self) {
        std::thread::sleep(std::time::Duration::from_secs(15));
    }

    fn stereo_callback(
        samples: &mut [f32],
        _info: &cpal::OutputCallbackInfo,
        stereo_position: &mut usize,
        stereo_data: &Vec<f32>,
        callback_count: &mut u64,
    ) {
        *callback_count += 1;

        let samples_len = samples.len();
        if samples_len != 960 {
            println!("{}, Samples: {}", *callback_count, samples_len);
        }

        let remaining_samples = stereo_data.len() - *stereo_position;
        let copy_len = usize::min(remaining_samples, samples_len);
        let end_position = *stereo_position + copy_len;
        samples[..copy_len].copy_from_slice(&stereo_data[*stereo_position..end_position]);
        *stereo_position = end_position;
        if *stereo_position == stereo_data.len() {
            for s in &mut samples[copy_len..] {
                *s = 0.0;
            }
            *stereo_position = 0;
        }
    }

    fn error_callback(err: cpal::StreamError) {
        match err {
            cpal::StreamError::DeviceNotAvailable => {
                println!("Audio Output Device Not Available!\n");
            }
            cpal::StreamError::BackendSpecific { err } => {
                println!("Audio Output Backend Specific: {}", err);
            }
        }
    }
}
