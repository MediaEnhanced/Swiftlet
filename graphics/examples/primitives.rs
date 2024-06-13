//Media Enhanced Swiftlet Cross-Compile Friendly Graphics Primitive Draw Example
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

use swiftlet_graphics::color::LinearRGB;
use swiftlet_graphics::font::{GlyphBufferRenderInfo, Glyphs, TextBuffer};
use swiftlet_graphics::vulkan::{
    PrimitiveColor, PrimitivePosition, PrimitiveRectangleModifier, Primitives2d,
};
use swiftlet_graphics::{DrawJustification, KeyCode};

const FONT_PATH: &str = "font/roboto/Roboto-Regular.ttf"; // Location of the Font
const ICON_PATH: &str = "font/symbols/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf"; // Location of the Icon Font
const ICON_CODEPOINTS_PATH: &str =
    "font/symbols/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].codepoints"; // Location of the Icon Font Codepoints

fn main() -> std::io::Result<()> {
    println!("Graphics Window Starting!");

    let mut icons = swiftlet_graphics::font::FontIcons::new_from_files(
        ICON_PATH,
        ICON_CODEPOINTS_PATH,
        ' ',
        16,
        2,
    )
    .unwrap();
    let icon_names = ["download", "search"];
    icons.add_icon_outline_data(&icon_names).unwrap();

    let mut glyphs = swiftlet_graphics::font::Glyphs::new_from_font_icons(icons).unwrap();
    glyphs.add_new_font(FONT_PATH, 0).unwrap();
    glyphs.add_glyph_outline_data(0, ' ', '~').unwrap();

    let (mut window, window_dpi) = match swiftlet_graphics::Vulkan2dWindow::new(
        1280,
        720,
        1 << 25,
        glyphs,
        swiftlet_graphics::Vulkan2dWindowMode::ValidationDebug,
    ) {
        Ok(r) => r,
        Err(e) => {
            println!("Window Creation Error: {:?}", e);
            return Err(std::io::Error::from(std::io::ErrorKind::Other));
        }
    };

    let mut primitive_draw = PrimitiveDraw::new(window_dpi);

    if let Err(e) = window.run(&mut primitive_draw, std::time::Duration::from_millis(20)) {
        println!("Window Run Error: {:?}", e);
    }

    Ok(())
}

struct PrimitiveDraw {
    times_called: u64,

    rect_pos: Option<PrimitivePosition>,
    rect_size: PrimitivePosition,

    dpi: f32,
    linear_rgb: LinearRGB,
    text_buffer_opt: Option<TextBuffer>,

    state: [bool; 3],
    should_draw: bool,
}

impl PrimitiveDraw {
    fn new(window_dpi: u32) -> Self {
        Self {
            times_called: 0,
            rect_pos: None,
            rect_size: PrimitivePosition::default(),
            dpi: window_dpi as f32,
            linear_rgb: LinearRGB::new(),
            text_buffer_opt: Some(TextBuffer::default()),
            state: [false; 3],
            should_draw: true,
        }
    }
}

impl swiftlet_graphics::Vulkan2dWindowCallbacks for PrimitiveDraw {
    fn draw(&mut self, primitives: &mut Primitives2d, glyphs: &Glyphs) {
        //println!("Try To Draw!");
        let background_color = PrimitiveColor::new_from_linear_rgb_and_alpha(
            self.linear_rgb.get_linear_rgb_from_srgb(0xEEEEEE),
            1.0,
        );
        let rect_p0 = PrimitivePosition::default();
        let rect_p1 = primitives.get_position_from_percentage(100.0, 100.0);
        primitives.add_rectangle(
            (0.0, 0.0),
            (rect_p1.x, rect_p1.y),
            &background_color,
            PrimitiveRectangleModifier::None,
        );

        let mut text_buffer = match self.text_buffer_opt.take() {
            Some(tb) => tb,
            None => TextBuffer::default(),
        };
        let solid_black_color = PrimitiveColor::new_from_linear_rgb_and_alpha(
            self.linear_rgb.get_linear_rgb_from_srgb(0),
            1.0,
        );

        let face_shaper = glyphs.get_font_face_shaper(0).unwrap();
        let server_name_pt_size = 18;
        let server_name_metrics =
            face_shaper.get_ascender_descender_gap(server_name_pt_size, self.dpi);
        let mut glyph_baseline = primitives.get_position_from_percentage(50.0, 4.0);
        glyph_baseline.y += server_name_metrics.0;

        text_buffer.add_text("The quick brown fox");
        text_buffer.add_text(" jumped");
        text_buffer.add_text(" over the lazy dog!");

        let glyph_bri =
            face_shaper.create_glyph_buffer_render_info(server_name_pt_size, self.dpi, text_buffer);
        glyph_bri.draw_glyphs(
            primitives,
            &glyph_baseline,
            &solid_black_color,
            2,
            DrawJustification::Center,
        );

        glyph_baseline.y += server_name_metrics.1 + server_name_metrics.2 + server_name_metrics.0;
        glyph_bri.draw_glyphs(
            primitives,
            &glyph_baseline,
            &solid_black_color,
            0,
            DrawJustification::Left,
        );
        let text_buffer = glyph_bri.get_text_buffer();

        let color = PrimitiveColor::new_from_linear_rgb_and_alpha(
            self.linear_rgb.get_linear_rgb_from_srgb(0xAA00),
            1.0,
        );
        if self.rect_pos.is_none() {
            self.rect_pos = Some(primitives.get_position_from_percentage(25.0, 25.0));
            self.rect_size = primitives.get_position_from_percentage(25.0, 25.0);
        }
        if let Some(p0) = &self.rect_pos {
            primitives.add_rectangle(
                (p0.x, p0.y),
                (self.rect_size.x, self.rect_size.y),
                &color,
                PrimitiveRectangleModifier::Ellipse,
            );
        }

        let icon_p0 = primitives.get_position_from_percentage(50.0, 50.0);
        glyphs.draw_icon(
            primitives,
            &icon_p0,
            &solid_black_color,
            2,
            0,
            40.0,
            DrawJustification::Center,
        );

        self.text_buffer_opt = Some(text_buffer);
    }

    fn key_pressed(&mut self, key_code: KeyCode) -> bool {
        //println!("Got Key Code: {:?}", key_code);
        match key_code {
            KeyCode::Enter => self.state[0] = !self.state[0],
            KeyCode::UpArrow => self.should_draw = true,
            KeyCode::RightArrow => {
                if let Some(pos) = &mut self.rect_pos {
                    pos.x += 0.25;
                    self.should_draw = true;
                }
            }
            KeyCode::LeftArrow => {
                if let Some(pos) = &mut self.rect_pos {
                    pos.x -= 1.0;
                    self.should_draw = true;
                }
            }
            KeyCode::Char('A') => {}
            KeyCode::Char(c) => {
                println!("Char Pressed: {}", c);
            }
            KeyCode::Chars(chars) => {
                for c in &chars.0[..chars.1] {
                    println!("Chars Pressed: {}", c);
                }
            }
            _ => {}
        }
        false
    }

    fn tick(&mut self, _glyphs: &mut Glyphs) -> bool {
        if !self.should_draw {
            false
        } else {
            self.should_draw = false;
            true
        }
    }
}
