//Media Enhanced Swiftlet Binaural Rust Library for Audio Conversions using HRTF Data
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

// Based on the 3dti_AudioToolkit library
// https://github.com/3DTune-In/3dti_AudioToolkit

#![allow(dead_code)] // Temporary

pub mod hrtf;
pub mod ild;
pub mod source;

pub struct Core {
    audio_state: AudioState,
    resampling_step: i32,
    listener: Listener,
}

impl Core {
    pub fn new(sample_rate: u32, buffer_size: u32, resampling_step: i32) -> Self {
        Core {
            audio_state: AudioState::new(sample_rate, buffer_size),
            resampling_step,
            listener: Listener::new(0.0875),
        }
    }
}

struct Listener {
    head_radius: f32,
}

impl Listener {
    fn new(head_radius: f32) -> Self {
        Listener { head_radius }
    }
}

// Can add a default later maybe...
struct AudioState {
    sample_rate: u32, // Sample Rate in Hz
    buffer_size: u32, // Number of samples for each channel
}

impl AudioState {
    fn new(sample_rate: u32, buffer_size: u32) -> Self {
        AudioState {
            sample_rate,
            buffer_size,
        }
    }
}
