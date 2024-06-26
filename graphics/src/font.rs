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

use rustybuzz::Direction;

#[derive(Debug)]
pub enum Error {
    FileRead(std::io::Error),
    CannotCreateFontFace,
    InvalidCodepointSplit,
    InvalidCodepointParse,
    InvalidCodepointValue,
    NoGlyphIndex(char),

    InvalidFont(usize),
    FontIndexAlreadyExists,
    GlyphNotFoundInFonts(char),
    GlyphOutlineError(char),
    NoGlyphIdInOutlines(u32),
}

pub struct FontIcons {
    data: Vec<u8>,
    codepoints_data: String,
    codepoint_delimiter: char,
    codepoint_radix: u32,
    rays_per_outline_po2: u8,
    // Variable Font Adjustments Here
    outline_data: Vec<GlyphOutlineData>,
}

impl FontIcons {
    pub fn new_from_files(
        icon_font_path: &str,
        icon_font_codepoints_path: &str,
        codepoint_delimiter: char,
        codepoint_radix: u32,
        mut rays_per_outline_po2: u8,
    ) -> Result<Self, Error> {
        let data = match std::fs::read(icon_font_path) {
            Ok(d) => d,
            Err(e) => return Err(Error::FileRead(e)),
        };
        if rustybuzz::Face::from_slice(&data, 0).is_none() {
            return Err(Error::CannotCreateFontFace);
        }

        let codepoints_data = match std::fs::read_to_string(icon_font_codepoints_path) {
            Ok(f) => f,
            Err(e) => return Err(Error::FileRead(e)),
        };

        if rays_per_outline_po2 > 3 {
            rays_per_outline_po2 = 3;
        }
        Ok(Self {
            data,
            codepoints_data,
            codepoint_delimiter,
            codepoint_radix,
            rays_per_outline_po2,
            outline_data: Vec::new(),
        })
    }

    pub fn add_icon_outline_data(&mut self, icon_names: &[&str]) -> Result<(), Error> {
        let font_face = match rustybuzz::Face::from_slice(&self.data, 0) {
            Some(f) => f,
            None => return Err(Error::CannotCreateFontFace),
        };

        let mut num_icons = icon_names.len();
        let outline_start_position = self.outline_data.len();
        self.outline_data
            .resize_with(num_icons, || GlyphOutlineData::new(u32::MAX));
        for l in self.codepoints_data.lines() {
            let (name, code_point) = match l.split_once(self.codepoint_delimiter) {
                Some((name, cp_str)) => {
                    let cp = match u32::from_str_radix(cp_str, self.codepoint_radix) {
                        Ok(v) => match char::from_u32(v) {
                            Some(c) => c,
                            None => return Err(Error::InvalidCodepointValue),
                        },
                        Err(_e) => return Err(Error::InvalidCodepointParse),
                    };
                    (name, cp)
                }
                None => return Err(Error::InvalidCodepointSplit),
            };

            for (ind, n) in icon_names.iter().enumerate() {
                if *n == name {
                    //println!("Found Icon: {}, with cp: {}", *n, code_point);
                    if let Some(glyph_id) = font_face.glyph_index(code_point) {
                        let builder = &mut self.outline_data[outline_start_position + ind];
                        let _bounding_box = match font_face.outline_glyph(glyph_id, builder) {
                            Some(bb) => bb,
                            None => {
                                if builder.get_num_segments() > 0 {
                                    return Err(Error::GlyphOutlineError(code_point));
                                } else {
                                    rustybuzz::ttf_parser::Rect {
                                        x_min: 0,
                                        y_min: 0,
                                        x_max: 0,
                                        y_max: 0,
                                    }
                                }
                            }
                        };
                        //println!("Icon Segment Count: {}", builder.get_num_segments());
                        // Compare bounding box in future
                        builder.sort_segments_and_create_additional_segments(
                            self.rays_per_outline_po2,
                        );
                    } else {
                        return Err(Error::NoGlyphIndex(code_point));
                    }
                    num_icons -= 1;
                    if num_icons > 0 {
                        break;
                    } else {
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }
}

struct FontInfo {
    data_start_index: usize,
    data_end_index: usize,
    index: u32,
    dpi_scale: f32,
    outline_offset: usize,
}

pub struct Glyphs {
    pub(super) num_icons: usize,
    pub(super) outline_data: Vec<GlyphOutlineData>,
    font_data: Vec<u8>,
    font_infos: Vec<FontInfo>,
    rays_per_outline_po2: u8,
    unicode_buffer_opt: Option<rustybuzz::UnicodeBuffer>,
    shape_features: Vec<rustybuzz::Feature>,
    line_render_info: Vec<GlyphLineRenderInfo>,
}

impl Glyphs {
    pub fn new_from_font_icons(font_icons: FontIcons) -> Result<Self, Error> {
        let num_icons = font_icons.outline_data.len();
        let outline_data = font_icons.outline_data;
        //println!("Num Segments: {}", outline_data[0].get_num_segments());

        let mut unicode_buffer = rustybuzz::UnicodeBuffer::new();
        //unicode_buffer.set_language(rustybuzz::Language(String::from(language)));
        unicode_buffer.set_direction(rustybuzz::Direction::LeftToRight);
        //unicode_buffer.set_cluster_level(rustybuzz::BufferClusterLevel::Characters);

        Ok(Self {
            num_icons,
            outline_data,
            font_data: Vec::new(),
            font_infos: Vec::new(),
            rays_per_outline_po2: font_icons.rays_per_outline_po2,
            unicode_buffer_opt: Some(unicode_buffer),
            shape_features: Vec::new(),
            line_render_info: Vec::new(),
        })
    }

    pub fn new_from_font_file(
        font_path: &str,
        font_index: u32,
        mut rays_per_outline_po2: u8,
        _language: &str,
    ) -> Result<Self, Error> {
        let font_data = match std::fs::read(font_path) {
            Ok(d) => d,
            Err(e) => return Err(Error::FileRead(e)),
        };
        let face = match rustybuzz::Face::from_slice(&font_data, font_index) {
            Some(f) => f,
            None => return Err(Error::CannotCreateFontFace),
        };
        let font_infos = vec![FontInfo {
            data_start_index: 0,
            data_end_index: font_data.len(),
            index: font_index,
            dpi_scale: 1.0 / (72.0 * (face.units_per_em() as f32)),
            outline_offset: 0,
        }];
        let mut unicode_buffer = rustybuzz::UnicodeBuffer::new();
        //unicode_buffer.set_language(rustybuzz::Language(String::from(language)));
        unicode_buffer.set_direction(rustybuzz::Direction::LeftToRight);
        //unicode_buffer.set_cluster_level(rustybuzz::BufferClusterLevel::Characters);

        if rays_per_outline_po2 > 3 {
            rays_per_outline_po2 = 3;
        }
        Ok(Self {
            num_icons: 0,
            outline_data: Vec::new(),
            font_data,
            font_infos,
            rays_per_outline_po2,
            unicode_buffer_opt: Some(unicode_buffer),
            shape_features: Vec::new(),
            line_render_info: Vec::new(),
        })
    }

    pub fn add_new_font(&mut self, font_path: &str, font_index: u32) -> Result<(), Error> {
        let font_data_offset = self.font_data.len();
        let units_per_em = match std::fs::read(font_path) {
            Ok(d) => {
                let units_per_em = match rustybuzz::Face::from_slice(&d, font_index) {
                    Some(f) => f.units_per_em(),
                    None => return Err(Error::CannotCreateFontFace),
                };
                self.font_data.extend_from_slice(&d);
                units_per_em
            }
            Err(e) => return Err(Error::FileRead(e)),
        };
        self.font_infos.push(FontInfo {
            data_start_index: font_data_offset,
            data_end_index: self.font_data.len(),
            index: font_index,
            dpi_scale: 1.0 / (72.0 * (units_per_em as f32)),
            outline_offset: self.outline_data.len(),
        });

        Ok(())
    }

    pub fn add_new_font_from_bytes(
        &mut self,
        font_bytes: &[u8],
        font_index: u32,
    ) -> Result<(), Error> {
        let font_data_offset = self.font_data.len();
        let units_per_em = match rustybuzz::Face::from_slice(font_bytes, font_index) {
            Some(f) => f.units_per_em(),
            None => return Err(Error::CannotCreateFontFace),
        };
        self.font_data.extend_from_slice(font_bytes);
        self.font_infos.push(FontInfo {
            data_start_index: font_data_offset,
            data_end_index: self.font_data.len(),
            index: font_index,
            dpi_scale: 1.0 / (72.0 * (units_per_em as f32)),
            outline_offset: self.outline_data.len(),
        });

        Ok(())
    }

    pub fn get_icon_dims(&self, icon: u32) -> (f32, f32) {
        let icon_id = icon as usize;
        if icon_id < self.num_icons {
            let icon_outline = &self.outline_data[icon_id];
            if !icon_outline.segments.is_empty() {
                (
                    icon_outline.x_max - icon_outline.x_min,
                    icon_outline.y_max - icon_outline.y_min,
                )
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        }
    }

    fn does_font_exist(&self, font: usize) -> Result<(), Error> {
        if font < self.font_infos.len() {
            Ok(())
        } else {
            Err(Error::InvalidFont(self.font_infos.len()))
        }
    }

    pub fn add_new_font_index(&mut self, font: usize, font_index: u32) -> Result<(), Error> {
        self.does_font_exist(font)?;

        let font_info = &self.font_infos[font];
        for fi in &self.font_infos {
            if (fi.data_start_index == font_info.data_start_index)
                && (fi.data_end_index == font_info.data_end_index)
                && (fi.index == font_info.index)
            {
                return Err(Error::FontIndexAlreadyExists);
            }
        }

        let _face = match rustybuzz::Face::from_slice(
            &self.font_data[font_info.data_start_index..font_info.data_end_index],
            font_index,
        ) {
            Some(f) => f,
            None => return Err(Error::CannotCreateFontFace),
        };
        self.font_infos.push(FontInfo {
            data_start_index: font_info.data_start_index,
            data_end_index: font_info.data_end_index,
            index: font_index,
            dpi_scale: font_info.dpi_scale,
            outline_offset: self.outline_data.len(),
        });

        Ok(())
    }

    pub fn get_font_face(&self, font: usize) -> Result<rustybuzz::Face, Error> {
        self.does_font_exist(font)?;
        let font_info = &self.font_infos[font];

        match rustybuzz::Face::from_slice(
            &self.font_data[font_info.data_start_index..font_info.data_end_index],
            font_info.index,
        ) {
            Some(f) => Ok(f),
            None => Err(Error::CannotCreateFontFace),
        }
    }

    pub fn add_glyph_outline_data(
        &mut self,
        font: usize,
        code_point_start: char,
        code_point_end: char,
    ) -> Result<(), Error> {
        let font_face = self.get_font_face(font)?;
        let outline_index_start = if font == 0 {
            self.num_icons
        } else {
            self.font_infos[font - 1].outline_offset
        };
        let outline_index_end = self.font_infos[font].outline_offset;

        let num_code_points = (code_point_end as usize) - (code_point_start as usize);
        let mut new_outline_data: Vec<GlyphOutlineData> = Vec::with_capacity(num_code_points);
        for cp in code_point_start..=code_point_end {
            if let Some(glyph_id) = font_face.glyph_index(cp) {
                let glyph_id_value = glyph_id.0 as u32;
                match self.outline_data[outline_index_start..outline_index_end]
                    .binary_search_by(|od| od.glyph_id.cmp(&glyph_id_value))
                {
                    Ok(_found_ind) => {
                        continue;
                    }
                    Err(_insert_ind) => {}
                }
                let mut found_glyph_id = false;
                for od in &new_outline_data {
                    if od.glyph_id == glyph_id_value {
                        found_glyph_id = true;
                        break;
                    }
                }
                if found_glyph_id {
                    continue;
                }

                let mut god = GlyphOutlineData::new(glyph_id_value);
                let _bounding_box = match font_face.outline_glyph(glyph_id, &mut god) {
                    Some(bb) => bb,
                    None => {
                        if god.get_num_segments() > 0 {
                            return Err(Error::GlyphOutlineError(cp));
                        } else {
                            rustybuzz::ttf_parser::Rect {
                                x_min: 0,
                                y_min: 0,
                                x_max: 0,
                                y_max: 0,
                            }
                        }
                    }
                };
                // Compare bounding box in future
                god.sort_segments_and_create_additional_segments(self.rays_per_outline_po2);
                new_outline_data.push(god);
            } else {
                return Err(Error::NoGlyphIndex(cp));
            }
        }

        let new_outline_count = new_outline_data.len();
        self.outline_data.reserve(new_outline_count);
        let mut tail = self.outline_data.split_off(outline_index_end);
        self.outline_data.append(&mut new_outline_data);
        self.outline_data.append(&mut tail);

        let outline_index_end = outline_index_end + new_outline_count;
        self.outline_data[outline_index_start..outline_index_end]
            .sort_unstable_by(|a, b| a.glyph_id.cmp(&b.glyph_id));

        for fi in &mut self.font_infos[font..] {
            fi.outline_offset += new_outline_count;
        }

        Ok(())
    }

    pub fn get_glyph_outline_data(&self) -> (&[GlyphOutlineData], u8) {
        (&self.outline_data, self.rays_per_outline_po2)
    }

    pub fn get_font_line_info(
        &self,
        font: usize,
        pt_size: u32,
        dpi: f32,
    ) -> Result<(f32, f32), Error> {
        let font_face = self.get_font_face(font)?;
        let dpi_scale = self.font_infos[font].dpi_scale;
        let scale = (pt_size as f32) * dpi_scale * dpi;
        let ascender = (font_face.ascender() as f32) * scale;
        let descender = (-font_face.descender() as f32) * scale;
        let line_gap = match font_face.line_gap() {
            0 => ((font_face.ascender() - font_face.descender()) as f32) * 0.2 * scale,
            other => (other as f32) * scale,
        };
        Ok((ascender, descender + line_gap))
    }

    pub fn push_text_line(&mut self, text_line: &str) {
        if let Some(unicode_buffer) = &mut self.unicode_buffer_opt {
            unicode_buffer.push_str(text_line);
        } else {
            panic!("How did this happen?");
        }
    }

    pub fn get_glyph_line_render_info(
        &mut self,
        font: usize,
        pt_size: u32,
        dpi: f32,
    ) -> Result<&[GlyphLineRenderInfo], Error> {
        if let Some(unicode_buffer) = self.unicode_buffer_opt.take() {
            let font_face = self.get_font_face(font)?;
            let outline_index_start = if font == 0 {
                0
            } else {
                self.font_infos[font - 1].outline_offset
            };
            let outline_index_end = self.font_infos[font].outline_offset;
            let dpi_scale = self.font_infos[font].dpi_scale;
            let scale = (pt_size as f32) * dpi_scale * dpi; // 92.36;
            let dp = 1.0 / scale;
            let glyph_buffer = rustybuzz::shape(&font_face, &self.shape_features, unicode_buffer);
            self.line_render_info.clear();
            let glyph_infos = glyph_buffer.glyph_infos();
            let glyph_positions = glyph_buffer.glyph_positions();
            for (gp_ind, gp) in glyph_positions.iter().enumerate() {
                let glyph_id = glyph_infos[gp_ind].glyph_id;
                // Could cache certain high probability glyphs in future
                let outline_index = match self.outline_data[outline_index_start..outline_index_end]
                    .binary_search_by(|od| od.glyph_id.cmp(&glyph_id))
                {
                    Ok(found_ind) => found_ind,
                    Err(_insert_ind) => return Err(Error::NoGlyphIdInOutlines(glyph_id)),
                };
                let lri = if self.outline_data[outline_index].get_num_segments() > 0 {
                    let u_min = self.outline_data[outline_index].x_min - dp;
                    let u_max = self.outline_data[outline_index].x_max + dp;
                    let v_min = self.outline_data[outline_index].y_min - dp;
                    let v_max = self.outline_data[outline_index].y_max + dp;
                    let pixel_width = (u_max - u_min) * scale;
                    let pixel_height = (v_max - v_min) * scale;
                    GlyphLineRenderInfo {
                        outline: outline_index as u32,
                        advance: (gp.x_advance as f32) * scale,
                        offset: (
                            (gp.x_offset as f32 + self.outline_data[outline_index].x_min) * scale,
                            (gp.y_offset as f32 + self.outline_data[outline_index].y_min) * scale,
                        ),
                        dimensions: (pixel_width, pixel_height),
                        p0: (u_min, v_min),
                        p1: (u_max, v_max),
                    }
                } else {
                    GlyphLineRenderInfo {
                        outline: outline_index as u32,
                        advance: (gp.x_advance as f32) * scale,
                        offset: (0.0, 0.0),
                        dimensions: (0.0, 0.0),
                        p0: (0.0, 0.0),
                        p1: (0.0, 0.0),
                    }
                };

                self.line_render_info.push(lri);
            }
            self.unicode_buffer_opt = Some(glyph_buffer.clear());
            Ok(&self.line_render_info)
        } else {
            panic!("How did this get reached?");
        }
    }

    pub fn get_font_face_shaper(&self, font: usize) -> Result<GlyphFaceShaper, Error> {
        let font_face = self.get_font_face(font)?;
        let outline_index_start = if font == 0 {
            0
        } else {
            self.font_infos[font - 1].outline_offset
        };
        let outline_index_end = self.font_infos[font].outline_offset;
        let dpi_scale = self.font_infos[font].dpi_scale;
        let plan = rustybuzz::ShapePlan::new(
            &font_face,
            Direction::LeftToRight,
            Some(rustybuzz::script::UNKNOWN),
            None,
            &self.shape_features,
        );

        Ok(GlyphFaceShaper {
            font_face,
            plan,
            dpi_scale,
            outline_index_offset: outline_index_start,
            outline_indicies: &self.outline_data[outline_index_start..outline_index_end],
        })
    }
}

pub type GlyphOutlinePoint = (f32, f32);

pub struct GlyphOutlineSegment {
    pub p0: GlyphOutlinePoint,
    pub p1: GlyphOutlinePoint,
    pub pq: Option<GlyphOutlinePoint>,
    pub x_max: f32,
}

pub struct GlyphOutlineData {
    pub(super) glyph_id: u32,
    p0: GlyphOutlinePoint,
    p1: GlyphOutlinePoint,
    segments: Vec<GlyphOutlineSegment>,
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
}

const COSINE_CALC: [f32; 8] = [
    1.0,
    0.0,
    #[allow(clippy::approx_constant)]
    0.70710678,
    #[allow(clippy::approx_constant)]
    0.70710678,
    0.38268343,
    0.9238795,
    0.9238795,
    0.38268343,
];
const SINE_CALC: [f32; 8] = [
    0.0,
    1.0,
    #[allow(clippy::approx_constant)]
    -0.70710678,
    #[allow(clippy::approx_constant)]
    0.70710678,
    -0.9238795,
    -0.38268343,
    0.38268343,
    0.9238795,
];

impl GlyphOutlineData {
    fn new(glyph_id: u32) -> Self {
        Self {
            glyph_id,
            p0: (0.0, 0.0),
            p1: (0.0, 0.0),
            segments: Vec::new(),
            x_min: f32::MAX,
            x_max: f32::MIN,
            y_min: f32::MAX,
            y_max: f32::MIN,
        }
    }

    fn sort_segments_and_create_additional_segments(&mut self, rays_per_outline_po2: u8) {
        self.segments
            .sort_unstable_by(|a, b| a.x_max.partial_cmp(&b.x_max).unwrap().reverse());
        if rays_per_outline_po2 == 0 {
            self.segments.shrink_to_fit();
            return;
        }

        let num_segements_per_ray = self.segments.len();
        let num_additional_rays = ((1 << rays_per_outline_po2) as usize) - 1;
        self.segments
            .reserve(num_segements_per_ray * num_additional_rays);
        for ar in 0..num_additional_rays {
            let cos = COSINE_CALC[ar + 1];
            let sin = SINE_CALC[ar + 1];

            let segment_start_index = self.segments.len();
            for seg_ind in 0..num_segements_per_ray {
                let seg = &self.segments[seg_ind];
                let p0 = (
                    (seg.p0.0 * cos) - (seg.p0.1 * sin),
                    (seg.p0.0 * sin) + (seg.p0.1 * cos),
                );
                let p1 = (
                    (seg.p1.0 * cos) - (seg.p1.1 * sin),
                    (seg.p1.0 * sin) + (seg.p1.1 * cos),
                );
                let (pq, x_max) = if let Some(q) = seg.pq {
                    let qx = (q.0 * cos) - (q.1 * sin);
                    (
                        Some((qx, (q.0 * sin) + (q.1 * cos))),
                        qx.max(p0.0.max(p1.0)),
                    )
                } else {
                    (None, p0.0.max(p1.0))
                };
                let rotated_gos = GlyphOutlineSegment { p0, p1, pq, x_max };
                self.segments.push(rotated_gos);
            }
            self.segments[segment_start_index..]
                .sort_unstable_by(|a, b| a.x_max.partial_cmp(&b.x_max).unwrap().reverse());
        }

        self.segments.shrink_to_fit();
    }

    pub fn get_num_segments(&self) -> u32 {
        self.segments.len() as u32
    }

    pub fn get_segment_data(&self) -> &[GlyphOutlineSegment] {
        &self.segments
    }

    pub fn set_render_info(
        &self,
        tex_min: &mut GlyphOutlinePoint,
        tex_max: &mut GlyphOutlinePoint,
    ) -> bool {
        if !self.segments.is_empty() {
            tex_min.0 = self.x_min;
            tex_min.1 = self.y_min;
            tex_max.0 = self.x_max;
            tex_max.1 = self.y_max;
            true
        } else {
            false
        }
    }
}

impl rustybuzz::ttf_parser::OutlineBuilder for GlyphOutlineData {
    fn move_to(&mut self, x: f32, y: f32) {
        self.p1 = (x, y);
        self.p0 = self.p1;

        if x < self.x_min {
            self.x_min = x;
        }
        if x > self.x_max {
            self.x_max = x;
        }
        if y < self.y_min {
            self.y_min = y;
        }
        if y > self.y_max {
            self.y_max = y;
        }
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let x_max = x.max(self.p0.0);
        self.segments.push(GlyphOutlineSegment {
            p0: self.p0,
            p1: (x, y),
            pq: None,
            x_max,
        });
        self.p0 = (x, y);

        if x < self.x_min {
            self.x_min = x;
        }
        if x > self.x_max {
            self.x_max = x;
        }
        if y < self.y_min {
            self.y_min = y;
        }
        if y > self.y_max {
            self.y_max = y;
        }
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let x_max = x1.max(x.max(self.p0.0));
        self.segments.push(GlyphOutlineSegment {
            p0: self.p0,
            p1: (x, y),
            pq: Some((x1, y1)),
            x_max,
        });
        self.p0 = (x, y);

        if x1 < self.x_min {
            self.x_min = x1;
        }
        if x1 > self.x_max {
            self.x_max = x1;
        }
        if y1 < self.y_min {
            self.y_min = y1;
        }
        if y1 > self.y_max {
            self.y_max = y1;
        }
        if x < self.x_min {
            self.x_min = x;
        }
        if x > self.x_max {
            self.x_max = x;
        }
        if y < self.y_min {
            self.y_min = y;
        }
        if y > self.y_max {
            self.y_max = y;
        }
    }

    fn curve_to(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32, _x: f32, _y: f32) {
        panic!("Cubic Curves Not Currently Supported!");
    }

    fn close(&mut self) {
        if self.p0 != self.p1 {
            let x_max = self.p1.0.max(self.p0.0);
            self.segments.push(GlyphOutlineSegment {
                p0: self.p0,
                p1: self.p1,
                pq: None,
                x_max,
            });
        }
    }
}

#[derive(Debug)]
pub struct GlyphLineRenderInfo {
    pub outline: u32,
    pub advance: f32,
    pub offset: GlyphOutlinePoint,
    pub dimensions: GlyphOutlinePoint,
    pub p0: GlyphOutlinePoint,
    pub p1: GlyphOutlinePoint,
}

pub struct GlyphFaceShaper<'a> {
    pub(super) font_face: rustybuzz::Face<'a>,
    plan: rustybuzz::ShapePlan,
    pub(super) dpi_scale: f32,
    pub(super) outline_index_offset: usize,
    pub(super) outline_indicies: &'a [GlyphOutlineData],
}

impl<'a> GlyphFaceShaper<'a> {
    pub fn get_ascender_descender_gap(&self, pt_size: u32, dpi: f32) -> (f32, f32, f32) {
        let scale = (pt_size as f32) * self.dpi_scale * dpi;
        let a = self.font_face.ascender();
        let d = self.font_face.descender();
        let ascender = (a as f32) * scale;
        let descender = (-d as f32) * scale;
        let line_gap = match self.font_face.line_gap() {
            0 => ((a - d) as f32) * 0.2 * scale,
            other => (other as f32) * scale,
        };
        (ascender, descender, line_gap)
    }

    pub fn create_glyph_buffer_render_info(
        &self,
        pt_size: u32,
        dpi: f32,
        mut text_buffer: TextBuffer,
    ) -> GlyphBufferRenderInfo {
        text_buffer
            .unicode_buffer
            .set_script(rustybuzz::script::UNKNOWN);
        let glyph_buffer =
            rustybuzz::shape_with_plan(&self.font_face, &self.plan, text_buffer.unicode_buffer);
        let scale = (pt_size as f32) * self.dpi_scale * dpi;
        GlyphBufferRenderInfo {
            glyph_buffer,
            scale,
            dp: 1.0 / scale,
            outline_index_offset: self.outline_index_offset as u32,
            outline_indicies: self.outline_indicies,
        }
    }
}

#[derive(Default)]
pub struct TextBuffer {
    unicode_buffer: rustybuzz::UnicodeBuffer,
}

// impl Default for TextBuffer {
//     fn default() -> Self {
//         let mut unicode_buffer = rustybuzz::UnicodeBuffer::default();
//         unicode_buffer.set_script(rustybuzz::script::UNKNOWN);
//         Self { unicode_buffer }
//     }
// }

impl TextBuffer {
    pub fn add_text(&mut self, text: &str) {
        self.unicode_buffer.push_str(text);
    }
}

pub struct GlyphBufferRenderInfo<'a> {
    pub(super) glyph_buffer: rustybuzz::GlyphBuffer,
    pub(super) scale: f32,
    pub(super) dp: f32,
    pub(super) outline_index_offset: u32,
    pub(super) outline_indicies: &'a [GlyphOutlineData],
}

impl<'a> GlyphBufferRenderInfo<'a> {
    pub fn get_text_buffer(self) -> TextBuffer {
        TextBuffer {
            unicode_buffer: self.glyph_buffer.clear(),
        }
    }
}
