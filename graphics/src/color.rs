//Media Enhanced Swiftlet Graphics Rust Library using Vulkan
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

pub fn get_linear_rgb_float_from_srgb_byte(byte_value: u8) -> f32 {
    let base = (byte_value as f32) / 255.0;
    if base > 0.04045 {
        let adjusted_base = (base + 0.055) / 1.055;
        adjusted_base.powf(2.4)
    } else {
        base / 12.92
    }
}

pub struct LinearRGB {
    srgb_lut: [f32; 256],
}

impl LinearRGB {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut srgb_lut = [0.0; 256];
        for (ind, v) in srgb_lut.iter_mut().enumerate() {
            *v = get_linear_rgb_float_from_srgb_byte(ind as u8)
        }

        Self { srgb_lut }
    }

    pub fn get_linear_rgb_from_srgb(&self, srgb: u32) -> [f32; 3] {
        let red_ind = ((srgb >> 16) & 0xFF) as usize;
        let green_ind = ((srgb >> 8) & 0xFF) as usize;
        let blue_ind = (srgb & 0xFF) as usize;
        [
            self.srgb_lut[red_ind],
            self.srgb_lut[green_ind],
            self.srgb_lut[blue_ind],
        ]
    }
}
