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

use std::collections::HashMap;

#[derive(Hash, Eq, PartialEq)]
struct Key {
    azimuth: i32,   // Distance to the center of the head, in millimeters
    elevation: i32, // Azimuth angle of interaural coordinates, in degrees
}

struct BiquadFilters {
    first: [f32; 5],
    second: [f32; 5],
}

pub struct Ild {
    sampling_rate: u32,
    map: HashMap<Key, BiquadFilters>,
}

impl Ild {
    // A return of None indicates bad input data
    pub fn new_from_3dti_data(d: &[u8]) -> Option<Self> {
        // Make sure there is a minimal amount of data
        if d.len() < 13 {
            return None;
        }
        // Check if data was in Little Endian form
        if d[0] != 1 {
            return None;
        }

        let sampling_rate = u32::from_le_bytes([d[1], d[2], d[3], d[4]]);
        let map_entries = usize::from_le_bytes([d[5], d[6], d[7], d[8], d[9], d[10], d[11], d[12]]);
        let map_data_size = map_entries * (12 * 4);

        if d.len() < (map_data_size + 13) {
            return None;
        }

        let mut d_pos = 13;
        let mut map = HashMap::with_capacity(map_entries);
        for _ in 0..map_entries {
            let key = Key {
                azimuth: i32::from_le_bytes([d[d_pos], d[d_pos + 1], d[d_pos + 2], d[d_pos + 3]]),
                elevation: i32::from_le_bytes([
                    d[d_pos + 4],
                    d[d_pos + 5],
                    d[d_pos + 6],
                    d[d_pos + 7],
                ]),
            };
            d_pos += 8;
            let first = [
                f32::from_le_bytes([d[d_pos], d[d_pos + 1], d[d_pos + 2], d[d_pos + 3]]),
                f32::from_le_bytes([d[d_pos + 4], d[d_pos + 5], d[d_pos + 6], d[d_pos + 7]]),
                f32::from_le_bytes([d[d_pos + 8], d[d_pos + 9], d[d_pos + 10], d[d_pos + 11]]),
                f32::from_le_bytes([d[d_pos + 12], d[d_pos + 13], d[d_pos + 14], d[d_pos + 15]]),
                f32::from_le_bytes([d[d_pos + 16], d[d_pos + 17], d[d_pos + 18], d[d_pos + 19]]),
            ];
            d_pos += 20;
            let second = [
                f32::from_le_bytes([d[d_pos], d[d_pos + 1], d[d_pos + 2], d[d_pos + 3]]),
                f32::from_le_bytes([d[d_pos + 4], d[d_pos + 5], d[d_pos + 6], d[d_pos + 7]]),
                f32::from_le_bytes([d[d_pos + 8], d[d_pos + 9], d[d_pos + 10], d[d_pos + 11]]),
                f32::from_le_bytes([d[d_pos + 12], d[d_pos + 13], d[d_pos + 14], d[d_pos + 15]]),
                f32::from_le_bytes([d[d_pos + 16], d[d_pos + 17], d[d_pos + 18], d[d_pos + 19]]),
            ];
            d_pos += 20;
            let filters = BiquadFilters { first, second };
            map.insert(key, filters);
        }

        println!("ILD Ending Position: {}", d_pos);

        Some(Ild { sampling_rate, map })
    }
}
