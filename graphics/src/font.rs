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

use std::fmt;

#[derive(Clone)]
pub struct OutlineSegment {
    pub is_quadratic: bool,
    pub x0: f32,
    pub x1: f32,
    pub xq: f32,
    pub y0: f32,
    pub y1: f32,
    pub yq: f32,
}

impl OutlineSegment {
    fn get_x_max(&self) -> f32 {
        if self.is_quadratic {
            self.xq.max(self.x0.max(self.x1))
        } else {
            self.x0.max(self.x1)
        }
    }

    fn scale_new(&self, scaler: f32) -> Self {
        OutlineSegment {
            is_quadratic: self.is_quadratic,
            x0: self.x0 * scaler,
            x1: self.x1 * scaler,
            xq: self.xq * scaler,
            y0: self.y0 * scaler,
            y1: self.y1 * scaler,
            yq: self.yq * scaler,
        }
    }
}

impl fmt::Debug for OutlineSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_quadratic {
            writeln!(
                f,
                "({}, {}) -> ({}, {}) -> ({}, {})",
                self.x0, self.y0, self.xq, self.yq, self.x1, self.y1
            )
        } else {
            writeln!(
                f,
                "({}, {}) -> ({}, {})",
                self.x0, self.y0, self.x1, self.y1
            )
        }
    }
}

#[derive(Clone, Default)]
struct SegmentCrossing {
    x_max: f32,
    add_coverage: Option<f32>,
    sub_coverage: Option<f32>,
}

impl SegmentCrossing {
    fn new(seg: &OutlineSegment, scaler: f32, sample_pixel_y: f32) -> Option<Self> {
        let s = seg.scale_new(scaler);
        //print!("{:?}", s);
        if s.is_quadratic {
            let x_max = s.xq.max(s.x0.max(s.x1));

            //print!("{:?}", s);
            if s.y0 > sample_pixel_y {
                if s.y1 <= sample_pixel_y {
                    let ay = s.y0 - (2.0 * s.yq) + s.y1;
                    let by = s.y0 - s.yq;
                    let cy = s.y0 - sample_pixel_y;
                    let d = ((by * by) - (ay * cy)).max(0.0).sqrt();
                    let t1 = (by - d) / ay;
                    let ax = s.x0 - (2.0 * s.xq) + s.x1;
                    let bx = s.x0 - s.xq;
                    let x1 = (ax * t1 - bx * 2.0) * t1 + s.x0;
                    //if (t1 < 0.0) || (t1 > 1.0) {
                    //println!("t1 | x1: {} | {}", t1, x1);
                    //}

                    Some(Self {
                        x_max,
                        add_coverage: Some(x1),
                        sub_coverage: None,
                    })
                } else if s.yq <= sample_pixel_y {
                    let ay = s.y0 - (2.0 * s.yq) + s.y1;
                    let by = s.y0 - s.yq;
                    let cy = s.y0 - sample_pixel_y;
                    let d = ((by * by) - (ay * cy)).max(0.0).sqrt();
                    let t1 = (by - d) / ay;
                    let t2 = (by + d) / ay;
                    let ax = s.x0 - (2.0 * s.xq) + s.x1;
                    let bx = s.x0 - s.xq;
                    let x1 = (ax * t1 - bx * 2.0) * t1 + s.x0;
                    let x2 = (ax * t2 - bx * 2.0) * t2 + s.x0;
                    //if (t1 < 0.0) || (t1 > 1.0) {
                    //println!("t1 | x1: {} | {}", t1, x1);
                    //}
                    //println!("t2 | x2: {} | {}", t2, x2);
                    Some(Self {
                        x_max,
                        add_coverage: Some(x1),
                        sub_coverage: Some(x2),
                    })
                } else {
                    None
                }
            } else if s.y1 > sample_pixel_y {
                let ay = s.y0 - (2.0 * s.yq) + s.y1;
                let by = s.y0 - s.yq;
                let cy = s.y0 - sample_pixel_y;
                let d = ((by * by) - (ay * cy)).max(0.0).sqrt();
                let t2 = (by + d) / ay;
                let ax = s.x0 - (2.0 * s.xq) + s.x1;
                let bx = s.x0 - s.xq;
                let x2 = (ax * t2 - bx * 2.0) * t2 + s.x0;
                //println!("t2 | x2: {} | {}", t2, x2);
                Some(Self {
                    x_max,
                    add_coverage: None,
                    sub_coverage: Some(x2),
                })
            } else if s.yq > sample_pixel_y {
                let ay = s.y0 - (2.0 * s.yq) + s.y1;
                let by = s.y0 - s.yq;
                let cy = s.y0 - sample_pixel_y;
                let d = ((by * by) - (ay * cy)).max(0.0).sqrt();
                let t1 = (by - d) / ay;
                let t2 = (by + d) / ay;
                let ax = s.x0 - (2.0 * s.xq) + s.x1;
                let bx = s.x0 - s.xq;
                let x1 = (ax * t1 - bx * 2.0) * t1 + s.x0;
                let x2 = (ax * t2 - bx * 2.0) * t2 + s.x0;
                //if (t1 < 0.0) || (t1 > 1.0) {
                //println!("t1 | x1: {} | {}", t1, x1);
                //}
                //println!("t2 | x2: {} | {}", t2, x2);
                Some(Self {
                    x_max,
                    add_coverage: Some(x1),
                    sub_coverage: Some(x2),
                })
            } else {
                None
            }
        } else {
            let x_max = s.x0.max(s.x1);

            //print!("{:?}", s);
            if s.y0 > sample_pixel_y {
                if s.y1 <= sample_pixel_y {
                    let x = (sample_pixel_y - s.y0) * (s.x1 - s.x0) / (s.y1 - s.y0) + s.x0;
                    Some(Self {
                        x_max,
                        add_coverage: Some(x),
                        sub_coverage: None,
                    })
                } else {
                    None
                }
            } else if s.y1 > sample_pixel_y {
                let x = (sample_pixel_y - s.y0) * (s.x1 - s.x0) / (s.y1 - s.y0) + s.x0;
                Some(Self {
                    x_max,
                    add_coverage: None,
                    sub_coverage: Some(x),
                })
            } else {
                None
            }
        }
    }
}

struct GlyphOutline {
    x_start: f32,
    y_start: f32,
    x_prev: f32,
    y_prev: f32,
    segments: Vec<OutlineSegment>,
}

impl GlyphOutline {
    fn new() -> Self {
        Self {
            x_start: 0.0,
            y_start: 0.0,
            x_prev: 0.0,
            y_prev: 0.0,
            segments: Vec::new(),
        }
    }

    fn get_sorted_segments_and_reset(&mut self) -> Vec<OutlineSegment> {
        self.segments
            .sort_unstable_by(|a, b| a.get_x_max().partial_cmp(&b.get_x_max()).unwrap().reverse());
        self.segments.shrink_to_fit();

        self.x_start = 0.0;
        self.y_start = 0.0;
        self.x_prev = 0.0;
        self.y_prev = 0.0;

        std::mem::take(&mut self.segments)
    }
}

impl ttf_parser::OutlineBuilder for GlyphOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.x_start = x;
        self.y_start = y;
        self.x_prev = self.x_start;
        self.y_prev = self.y_start;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.segments.push(OutlineSegment {
            is_quadratic: false,
            x0: self.x_prev,
            x1: x,
            xq: 0.0,
            y0: self.y_prev,
            y1: y,
            yq: 0.0,
        });
        self.x_prev = x;
        self.y_prev = y;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.segments.push(OutlineSegment {
            is_quadratic: true,
            x0: self.x_prev,
            x1: x,
            xq: x1,
            y0: self.y_prev,
            y1: y,
            yq: y1,
        });
        self.x_prev = x;
        self.y_prev = y;
    }

    fn curve_to(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32, _x: f32, _y: f32) {
        panic!("Cubic Curves Not Currently Supported!");
    }

    fn close(&mut self) {
        if (self.x_prev != self.x_start) || (self.y_prev != self.y_start) {
            self.segments.push(OutlineSegment {
                is_quadratic: false,
                x0: self.x_prev,
                x1: self.x_start,
                xq: 0.0,
                y0: self.y_prev,
                y1: self.y_start,
                yq: 0.0,
            });
        }
    }
}

#[derive(Clone)]
struct GlyphData {
    horizontal_advance: u16,
    top_left_x: i16,
    top_left_y: i16,
    bottom_right_x: i16,
    bottom_right_y: i16,
    segments: Vec<OutlineSegment>,
}

impl GlyphData {
    fn top_left_scale_and_round(&self, scaler: f32) -> (f32, f32) {
        //Improve actual rounding later
        let x = ((self.top_left_x as f32) * scaler).round() + 0.5;
        let y = ((self.top_left_y as f32) * scaler).round() - 0.5;
        (x, y)
    }

    fn bottom_right_scale_and_round(&self, scaler: f32) -> (f32, f32) {
        //Improve actual rounding later
        let x = ((self.bottom_right_x as f32) * scaler).round() - 0.5;
        let y = ((self.bottom_right_y as f32) * scaler).round() + 0.5;
        (x, y)
    }
}

pub struct FontGlyphs {
    dpi_scale: f32,
    single_byte_data: Vec<GlyphData>, // Space -> Tilde in UTF-8
}

impl FontGlyphs {
    pub fn new(font_data: &[u8]) -> Option<Self> {
        let face = match ttf_parser::Face::parse(font_data, 0) {
            Ok(f) => f,
            Err(_e) => return None,
        };

        // println!(
        //     "Face Width, Height: {:?}, {:?}",
        //     face.units_per_em(),
        //     face.height()
        // );
        let dpi_scale = 1.0 / (72.0 * (face.units_per_em() as f32));

        let space = ' ';
        let glyph_id = match face.glyph_index(space) {
            Some(id) => id,
            None => return None,
        };

        let mut single_byte_data = Vec::with_capacity(94);

        let horizontal_advance = face.glyph_hor_advance(glyph_id).unwrap(); //unwrap for now...
        let mut glyph_outline = GlyphOutline::new();
        let bounding_box =
            face.outline_glyph(glyph_id, &mut glyph_outline)
                .unwrap_or(ttf_parser::Rect {
                    x_min: 0,
                    y_min: 0,
                    x_max: 0,
                    y_max: 0,
                });

        let space_glyph_data = GlyphData {
            horizontal_advance,
            top_left_x: bounding_box.x_min,
            top_left_y: bounding_box.y_max,
            bottom_right_x: bounding_box.x_max,
            bottom_right_y: bounding_box.y_min,
            segments: glyph_outline.get_sorted_segments_and_reset(),
        };
        single_byte_data.push(space_glyph_data.clone());

        for code_point in '!'..='~' {
            match face.glyph_index(code_point) {
                Some(glyph_id) => {
                    let horizontal_advance = face.glyph_hor_advance(glyph_id).unwrap(); //unwrap for now...
                    let bounding_box = face.outline_glyph(glyph_id, &mut glyph_outline).unwrap_or(
                        ttf_parser::Rect {
                            x_min: 0,
                            y_min: 0,
                            x_max: 0,
                            y_max: 0,
                        },
                    );
                    let glyph_data = GlyphData {
                        horizontal_advance,
                        top_left_x: bounding_box.x_min,
                        top_left_y: bounding_box.y_max,
                        bottom_right_x: bounding_box.x_max,
                        bottom_right_y: bounding_box.y_min,
                        segments: glyph_outline.get_sorted_segments_and_reset(),
                    };
                    single_byte_data.push(glyph_data);
                }
                None => single_byte_data.push(space_glyph_data.clone()),
            }
        }

        Some(Self {
            dpi_scale,
            single_byte_data,
        })
    }

    pub fn print_outline(&self, character: char) {
        if (' '..='~').contains(&character) {
            let byte_data_index = (character as usize) - (' ' as usize);
            println!(
                "{} Outline Data: {:?}",
                character, self.single_byte_data[byte_data_index].segments,
            );
        }
    }

    pub fn render_character(
        &self,
        pixel_data: &mut [u32],
        pitch: usize,
        origin_index: usize,
        character: char,
        pt_size: u32,
    ) {
        let byte_data_index = if (' '..='~').contains(&character) {
            (character as usize) - (' ' as usize)
        } else {
            return;
        };

        let scaler = pt_size as f32 * self.dpi_scale * 92.36;
        let (top_left_x, top_left_y) =
            &self.single_byte_data[byte_data_index].top_left_scale_and_round(scaler);
        let (bottom_right_x, bottom_right_y) =
            &self.single_byte_data[byte_data_index].bottom_right_scale_and_round(scaler);

        let top_left_index =
            origin_index - (pitch * (top_left_y - 0.5) as usize) + (top_left_x - 0.5) as usize;

        let num_pixels_x = (bottom_right_x - top_left_x) as usize + 1;
        let num_pixels_y = (top_left_y - bottom_right_y) as usize + 1;

        let num_segments = self.single_byte_data[byte_data_index].segments.len();
        let mut crossing_segments = vec![SegmentCrossing::default(); num_pixels_y * num_segments];
        let mut num_crossings = vec![0; num_pixels_y];
        #[allow(clippy::needless_range_loop)]
        for pixel_y in 0..num_pixels_y {
            let start_index = pixel_y * num_segments;
            let mut index = start_index;
            let sample_pixel_y = top_left_y - (pixel_y as f32);
            for s in &self.single_byte_data[byte_data_index].segments {
                if let Some(sc) = SegmentCrossing::new(s, scaler, sample_pixel_y) {
                    crossing_segments[index] = sc;
                    index += 1;
                }
            }
            num_crossings[pixel_y] = index - start_index;
        }
        #[allow(clippy::needless_range_loop)]
        for pixel_y in 0..num_pixels_y {
            let mut pixel_index = top_left_index + (pixel_y * pitch);
            for pixel_x in 0..num_pixels_x {
                let mut coverage = 0.0;

                let sample_pixel_x = top_left_x + (pixel_x as f32) - 0.5;
                let start_index = pixel_y * num_segments;
                #[allow(clippy::needless_range_loop)]
                for sc_index in start_index..(start_index + num_crossings[pixel_y]) {
                    if sample_pixel_x > crossing_segments[sc_index].x_max {
                        break;
                    }

                    if let Some(add_coverage) = crossing_segments[sc_index].add_coverage {
                        let coverage_dif = add_coverage - sample_pixel_x;
                        if coverage_dif >= 1.0 {
                            coverage += 1.0;
                        } else if coverage_dif > 0.0 {
                            coverage += coverage_dif
                        }
                    }
                    if let Some(sub_coverage) = crossing_segments[sc_index].sub_coverage {
                        let coverage_dif = sub_coverage - sample_pixel_x;
                        if coverage_dif >= 1.0 {
                            coverage -= 1.0;
                        } else if coverage_dif > 0.0 {
                            coverage -= coverage_dif
                        }
                    }
                }

                //println!("Sample Pixel: {}", coverage);
                let sub_value = (255.0 * coverage.abs().clamp(0.0, 1.0)) as u32;
                if sub_value != 0 {
                    //println!("Coverage: {}", coverage);
                    pixel_data[pixel_index] = 0;
                    pixel_data[pixel_index] =
                        0xFFFFFF - (sub_value << 16) - (sub_value << 8) - sub_value;
                }
                pixel_index += 1;
            }
        }
    }

    pub fn get_num_glyphs(&self) -> u32 {
        self.single_byte_data.len() as u32
    }

    pub fn get_segment_offsets(&self) -> Vec<u32> {
        let num_offsets = self.single_byte_data.len() + 1;
        let additional_len = (4 - (num_offsets & 0x3)) & 0x3;
        let mut segment_offsets = Vec::with_capacity(num_offsets + additional_len);
        let mut offset = 0;
        segment_offsets.push(offset);
        for g in &self.single_byte_data {
            offset += g.segments.len() as u32;
            segment_offsets.push(offset);
        }
        for _i in 0..additional_len {
            segment_offsets.push(0);
        }
        segment_offsets
    }

    pub fn get_segment_data(&self, glyph_index: u32) -> &[OutlineSegment] {
        &self.single_byte_data[glyph_index as usize].segments
    }

    pub fn get_character_info(
        &self,
        character: char,
        pt_size: u32,
    ) -> (u32, f32, f32, f32, f32, f32, f32) {
        if (' '..='~').contains(&character) {
            let index = (character as usize) - (' ' as usize);
            let scale = (pt_size as f32) * self.dpi_scale * 92.36;
            let bottom_left_x = self.single_byte_data[index].top_left_x as f32;
            let bottom_left_y = self.single_byte_data[index].bottom_right_y as f32;
            let top_right_x = self.single_byte_data[index].bottom_right_x as f32;
            let top_right_y = self.single_byte_data[index].top_left_y as f32;
            let pixel_width = ((top_right_x - bottom_left_x) * scale) + 2.0;
            let pixel_height = ((top_right_y - bottom_left_y) * scale) + 2.0;
            let dx = 1.0 / scale;
            //println!("Scale: {}", scale);
            (
                index as u32,
                pixel_width,
                pixel_height,
                bottom_left_x - dx,
                bottom_left_y - dx,
                top_right_x + dx,
                top_right_y + dx,
            )
        } else {
            (0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
        }
    }
}
