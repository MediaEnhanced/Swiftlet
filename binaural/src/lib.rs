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

// Only supports a sampling rate of 48000 Hz for right now

#![allow(dead_code)] // Temporary

pub mod hrtf;
pub mod ild;
pub mod source;

use hrtf::Hrtf;
use ild::Ild;
use source::Source;

pub struct ListenerEffects {
    pub far_distance: bool,
    pub distance_attenuation: bool,
}

pub struct Listener {
    hrtf: Hrtf,
    ild: Ild,
    head_radius: f32,
    position: nalgebra::Point3<f32>,
    orientation: nalgebra::Quaternion<f32>,
}

impl Listener {
    pub fn new(hrtf: Hrtf, ild: Ild, head_radius_option: Option<f32>) -> Self {
        let head_radius = head_radius_option.unwrap_or(0.0875);
        Listener {
            hrtf,
            ild,
            head_radius,
            position: nalgebra::Point3::new(0.0, 0.0, 0.0),
            //orientation: nalgebra::Quaternion::new(0.0, 0.0, 0.0, 0.0),
            orientation: nalgebra::Quaternion::default(),
        }
    }

    pub fn rotate(&mut self, degrees: f32) {
        self.orientation.i = degrees;
    }

    pub fn process_source(&self, source: &Source, effects: &ListenerEffects) -> Vec<f32> {
        if source.get_distance_from_position(&self.position) <= self.head_radius {
            return source.get_stereo();
        }

        let stereo_data = Vec::new();

        if effects.far_distance {
            // Do something here in future
        }

        if effects.distance_attenuation {
            // Do something here in future
        }

        stereo_data
    }
}
