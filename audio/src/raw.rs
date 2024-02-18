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

const MAX_CHANNEL_COUNT: usize = 8;

pub struct Raw {
    samples_per_sec: u32,
    channel_count: usize,
    data: [Vec<f32>; MAX_CHANNEL_COUNT], // Vecs should NOT have any heap allocation when using a zero-length new
    channel_format: u64,
}

impl Raw {
    // A return of None indicates bad input data
    pub fn new_from_wav(d: &[u8]) -> Option<Self> {
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

                return Some(Raw {
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

    pub fn is_ideal_sample_rate(&self) -> bool {
        self.samples_per_sec == 48000
    }

    pub fn get_mono(&self) -> Option<Vec<f32>> {
        if self.channel_count == 1 {
            Some(self.data[0].clone())
        } else if self.channel_count == 2 {
            let num_samples = self.data[0].len();
            if num_samples != self.data[1].len() {
                return None;
            }
            let mut mono = Vec::with_capacity(num_samples);
            for i in 0..num_samples {
                let avg_sample = (self.data[0][i] + self.data[1][i]) * 0.5;
                mono.push(avg_sample);
            }
            Some(mono)
        } else {
            None
        }
    }

    pub fn get_stereo(&self) -> Option<Vec<f32>> {
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
