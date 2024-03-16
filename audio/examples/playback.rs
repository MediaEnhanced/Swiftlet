//Media Enhanced Swiftlet Basic Opus Audio Playback Example
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

const SONG_PATH: &str = "audio/song.opus"; // Location of the Opus Song in Ogg file format

fn main() {
    println!("Opus Audio Playback Example Started!");

    let opus_data = match std::fs::read(std::path::Path::new(SONG_PATH)) {
        Ok(ogg_file_bytes) => {
            match swiftlet_audio::opus::OpusData::create_from_ogg_file(&ogg_file_bytes, 1) {
                Some(od) => od,
                None => {
                    println!("Could not create opus data!");
                    return;
                }
            }
        }
        Err(_) => {
            println!("Could not find opus song file!");
            return;
        }
    };

    if let Some(stereo_data) = opus_data.get_stereo() {
        let playback = Playback {
            stereo_position: 0,
            callback_count: 0,
            stereo_data,
        };

        match swiftlet_audio::run_output(480, 2, playback) {
            Ok(true) => println!("Played the whole song!"),
            Ok(false) => println!("Playback loop ended sooner than expected!"),
            Err(e) => println!("Playback Error: {:?}", e),
        }
    }

    println!("Playback Example Ended!");
}

struct Playback {
    stereo_position: usize,
    callback_count: u64,
    stereo_data: Vec<f32>,
}

impl swiftlet_audio::OutputCallback for Playback {
    fn output_callback(&mut self, samples: &mut [f32]) -> bool {
        self.callback_count += 1;

        let samples_len = samples.len();
        if samples_len != 960 {
            println!("{}, Samples: {}", self.callback_count, samples_len);
            if samples_len == 0 {
                return true;
            }
        }

        let remaining_samples = self.stereo_data.len() - self.stereo_position;
        let copy_len = usize::min(remaining_samples, samples_len);
        let end_position = self.stereo_position + copy_len;
        samples[..copy_len].copy_from_slice(&self.stereo_data[self.stereo_position..end_position]);
        self.stereo_position = end_position;
        if self.stereo_position == self.stereo_data.len() {
            for s in &mut samples[copy_len..] {
                *s = 0.0;
            }
            self.stereo_position = 0;
            return true;
        }

        false
    }
}
