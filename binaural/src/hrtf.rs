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
struct Orientation {
    azimuth: i32,   // Azimuth angle in degrees
    elevation: i32, // Elevation angle in degrees
}

// Head-related impulse response (Originally: THRIRStruct)
struct ImpulseResponse {
    left_delay: u64,
    right_delay: u64,
    left_data: Vec<f32>,  // Length indicated by ir_data_len
    right_data: Vec<f32>, // Length indicated by ir_data_len
}

pub struct Hrtf {
    sampling_rate: u32,
    ir_data_len: u32,
    distance_of_measurement: f32,
    map: HashMap<Orientation, ImpulseResponse>,
}

impl Hrtf {
    // A return of None indicates bad input data
    pub fn new_from_3dti_data(d: &[u8]) -> Option<Self> {
        // Make sure there is a minimal amount of data
        if d.len() < 21 {
            return None;
        }

        // Check if data was in Little Endian form
        if d[0] != 1 {
            return None;
        }

        let sampling_rate = u32::from_le_bytes([d[1], d[2], d[3], d[4]]);
        let ir_data_len = u32::from_le_bytes([d[5], d[6], d[7], d[8]]);
        let distance_of_measurement = f32::from_le_bytes([d[9], d[10], d[11], d[12]]);
        let map_entries =
            usize::from_le_bytes([d[13], d[14], d[15], d[16], d[17], d[18], d[19], d[20]]);
        let map_data_size = map_entries * (40 + (ir_data_len as usize * 8));

        // Check if data is long enough
        if d.len() < (map_data_size + 21) {
            return None;
        }

        let mut d_pos = 21;
        let mut map = HashMap::with_capacity(map_entries);
        for _ in 0..map_entries {
            let orientation = Orientation {
                azimuth: i32::from_le_bytes([d[d_pos], d[d_pos + 1], d[d_pos + 2], d[d_pos + 3]]),
                elevation: i32::from_le_bytes([
                    d[d_pos + 4],
                    d[d_pos + 5],
                    d[d_pos + 6],
                    d[d_pos + 7],
                ]),
            };
            d_pos += 8;
            let left_delay = u64::from_le_bytes([
                d[d_pos],
                d[d_pos + 1],
                d[d_pos + 2],
                d[d_pos + 3],
                d[d_pos + 4],
                d[d_pos + 5],
                d[d_pos + 6],
                d[d_pos + 7],
            ]);
            d_pos += 8;
            let right_delay = u64::from_le_bytes([
                d[d_pos],
                d[d_pos + 1],
                d[d_pos + 2],
                d[d_pos + 3],
                d[d_pos + 4],
                d[d_pos + 5],
                d[d_pos + 6],
                d[d_pos + 7],
            ]);
            d_pos += 8;

            let size_check = usize::from_le_bytes([
                d[d_pos],
                d[d_pos + 1],
                d[d_pos + 2],
                d[d_pos + 3],
                d[d_pos + 4],
                d[d_pos + 5],
                d[d_pos + 6],
                d[d_pos + 7],
            ]);
            d_pos += 8;
            if size_check != (ir_data_len as usize) {
                return None;
            }
            let mut left_data = Vec::with_capacity(size_check);
            for _ in 0..size_check {
                left_data.push(f32::from_le_bytes([
                    d[d_pos],
                    d[d_pos + 1],
                    d[d_pos + 2],
                    d[d_pos + 3],
                ]));
                d_pos += 4;
            }

            let size_check = usize::from_le_bytes([
                d[d_pos],
                d[d_pos + 1],
                d[d_pos + 2],
                d[d_pos + 3],
                d[d_pos + 4],
                d[d_pos + 5],
                d[d_pos + 6],
                d[d_pos + 7],
            ]);
            d_pos += 8;
            if size_check != (ir_data_len as usize) {
                return None;
            }
            let mut right_data = Vec::with_capacity(size_check);
            for _ in 0..size_check {
                right_data.push(f32::from_le_bytes([
                    d[d_pos],
                    d[d_pos + 1],
                    d[d_pos + 2],
                    d[d_pos + 3],
                ]));
                d_pos += 4;
            }

            let ir = ImpulseResponse {
                left_delay,
                right_delay,
                left_data,
                right_data,
            };
            map.insert(orientation, ir);
        }

        println!("HRTF Ending Position: {}", d_pos);

        Some(Hrtf {
            sampling_rate,
            ir_data_len,
            distance_of_measurement,
            map,
        })
    }
}
