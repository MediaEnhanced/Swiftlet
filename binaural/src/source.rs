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

pub struct Source {
    position: nalgebra::Point3<f32>,
    mono_audio: Vec<f32>,
}

impl Source {
    pub fn new(x_pos: f32, y_pos: f32, z_pos: f32, mono_audio: Vec<f32>) -> Self {
        Source {
            position: nalgebra::Point3::new(x_pos, y_pos, z_pos),
            mono_audio,
        }
    }

    pub(super) fn get_distance_from_position(&self, p2: &nalgebra::Point3<f32>) -> f32 {
        nalgebra::distance(&self.position, p2)
    }

    pub fn get_stereo(&self) -> Vec<f32> {
        let mut stereo = Vec::with_capacity(self.mono_audio.len() * 2);
        for m in &self.mono_audio {
            stereo.push(*m);
            stereo.push(*m);
        }
        stereo
    }
}
