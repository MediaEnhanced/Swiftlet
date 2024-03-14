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

use swiftlet_audio::raw::Raw;
use swiftlet_binaural::hrtf::Hrtf;
use swiftlet_binaural::ild::Ild;
use swiftlet_binaural::source::Source;
use swiftlet_binaural::{Listener, ListenerEffects};

const HRTF_PATH: &str = "resources/hrtf/IRC1008.3dti-hrtf"; // Location of the HRTF Data
const ILD_PATH: &str = "resources/ILD/HRTF_ILD_48000.3dti-ild"; // Location of the ILD Data
const NFC_ILD_PATH: &str = "resources/ILD/NFC_ILD_48000.3dti-ild"; // Location of the Near Field Compensation ILD Data
const WAV_PATH: &str = "resources/speech.wav"; // Location of the mono speech WAV data

fn main() {
    println!("Binaural Demo Started!");

    let hrtf = match std::fs::read(std::path::Path::new(HRTF_PATH)) {
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

    let ild = match std::fs::read(std::path::Path::new(NFC_ILD_PATH)) {
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

    let mut listener = Listener::new(hrtf, ild, None);
    listener.rotate(90.0);
    let effects = ListenerEffects {
        far_distance: false,
        distance_attenuation: false,
    };

    let raw_audio = match std::fs::read(std::path::Path::new(WAV_PATH)) {
        Ok(bytes) => match Raw::new_from_wav(&bytes) {
            Some(r) => r,
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

    // Convert raw audio_here in future
    if raw_audio.is_ideal_sample_rate() {
        if let Some(mono_audio) = raw_audio.get_mono() {
            let source = Source::new(1.0, 1.0, 1.0, mono_audio);
            let stereo_data = listener.process_source(&source, &effects);

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

            match swiftlet_audio::run_output(480, 2, &mut f) {
                Ok(true) => println!("Played the whole song!"),
                Ok(false) => println!("Playback loop ended sooner than expected!"),
                Err(e) => println!("Playback Error: {:?}", e),
            }
        }
    }

    println!("Binaural Demo Ending!");
}

fn output_callback(
    samples: &mut [f32],
    stereo_position: &mut usize,
    stereo_data: &[f32],
    callback_count: &mut u64,
) -> bool {
    *callback_count += 1;

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
