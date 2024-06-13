//Media Enhanced Swiftlet Cross-Compile Friendly Graphics Triangle Glyph Draw Example
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

use std::time::Duration;
use swiftlet_graphics::vulkan::{
    TriangleColorGlyph, TriangleIndicies, TriangleVertex, TriglyphInputData,
};
use swiftlet_graphics::KeyCode;

const FONT_PATH: &str = "font/firasans/FiraSans-Regular.ttf"; // Location of the Font

fn main() -> std::io::Result<()> {
    println!("Graphics Window Starting!");

    let mut triangle_example = TriglyphExample::new();

    let (mut window, signaler) = match swiftlet_graphics::VulkanTriglyph::new(
        1280,
        720,
        104 * 8,
        triangle_example.glyphs.get_glyph_outline_data(),
        true,
    ) {
        Ok((w, s)) => (w, s),
        Err(e) => {
            println!("Window Creation Error: {:?}", e);
            return Err(std::io::Error::from(std::io::ErrorKind::Other));
        }
    };

    let thread_handle = std::thread::spawn(|| signaler_thread(signaler));

    if let Err(e) = window.run(&mut triangle_example) {
        println!("Window Run Error: {:?}", e);
    }
    drop(window);

    println!("Waiting for signaler thread to finish!");
    let _ = thread_handle.join();

    Ok(())
}

fn signaler_thread(mut signaler: swiftlet_graphics::OsEventSignaler) {
    // loop {
    //     match signaler.signal() {
    //         Ok(_) => {}
    //         Err(_e) => break,
    //     }
    //     std::thread::sleep(Duration::from_secs(1));
    // }
    match signaler.signal() {
        Ok(_) => {}
        Err(_e) => return,
    }
    std::thread::sleep(Duration::from_secs(1));
    match signaler.signal() {
        Ok(_) => {}
        Err(_e) => {}
    }
}

// double getLinearSRGBChannelValuefromSRGBChannelByte(uint8_t v) {
// 	//Uses the sRGB transfer function and operates on a sRGB channel byte independently of the others
// 	double base = ((double) v) / 255.0; // for 8-bit value
// 	if (base > 0.04045) { // > ? ... Doesn't matter for 8-bit values 0 -> 10
// 		base = (base + 0.055) / 1.055; // Test for > 1?

// 		double power = 2.4;
// 		//double result = pow(base, power);
// 		double result = cr_log2(base);
// 		result *= power;
// 		result = cr_exp2(result);

// 		return result;
// 	}
// 	else {
// 		double result = base / 12.92;
// 		return result;
// 	}
// }

fn get_linear_rgb_float_from_srgb_byte(byte_value: u8) -> f32 {
    let base = (byte_value as f32) / 255.0;
    if base > 0.04045 {
        let adjusted_base = (base + 0.055) / 1.055;
        adjusted_base.powf(2.4)
    } else {
        base / 12.92
    }
}

struct TriglyphExample {
    times_called: u64,
    dpi: f32,
    num_verticies: usize,
    num_triangles: usize,
    x_max: u32,
    y_max: u32,
    x_mult: f32,
    y_mult: f32,
    linear_rgb_lut: [f32; 256],
    glyphs: swiftlet_graphics::font::Glyphs,
    state: [bool; 3],
    should_draw: bool,
}

impl TriglyphExample {
    fn new() -> Self {
        let mut linear_rgb_lut = [0.0; 256];
        for (ind, v) in linear_rgb_lut.iter_mut().enumerate() {
            *v = get_linear_rgb_float_from_srgb_byte(ind as u8)
        }
        let mut glyphs =
            swiftlet_graphics::font::Glyphs::new_from_font_file(FONT_PATH, 0, 2, "en").unwrap();
        glyphs.add_glyph_outline_data(0, ' ', '~').unwrap();
        TriglyphExample {
            times_called: 0,
            dpi: 92.36,
            num_verticies: 0,
            num_triangles: 0,
            x_max: 0,
            y_max: 0,
            x_mult: 0.0,
            y_mult: 0.0,
            linear_rgb_lut,
            glyphs,
            state: [false; 3],
            should_draw: false,
        }
    }

    fn reset_draw_stats(&mut self, width: u32, height: u32) {
        self.num_verticies = 0;
        self.num_triangles = 0;
        self.x_max = width;
        self.y_max = height;
        self.x_mult = 2.0 / (width as f32);
        self.y_mult = 2.0 / (height as f32);
    }

    /// bottom-left pt of each pixel
    fn get_vertex_for_pixel(&self, mut x: u32, mut y: u32) -> (f32, f32) {
        if x > self.x_max {
            x = self.x_max;
        }
        if y > self.y_max {
            y = self.y_max;
        }
        let x_pos = ((x as f32) * self.x_mult) + -1.0;
        let y_pos = ((y as f32) * self.y_mult) + -1.0;
        (x_pos, y_pos)
    }

    fn get_linear_rgb_from_srgb(&self, srgb: u32) -> [f32; 3] {
        let red_ind = ((srgb >> 16) & 0xFF) as usize;
        let green_ind = ((srgb >> 8) & 0xFF) as usize;
        let blue_ind = (srgb & 0xFF) as usize;
        [
            self.linear_rgb_lut[red_ind],
            self.linear_rgb_lut[green_ind],
            self.linear_rgb_lut[blue_ind],
        ]
    }

    fn get_color_glyph(&self, srgb: u32, alpha: f32) -> TriangleColorGlyph {
        let mut linear_rgb = self.get_linear_rgb_from_srgb(srgb);
        for l in &mut linear_rgb {
            *l *= alpha;
        }
        TriangleColorGlyph {
            linear_rgb,
            linear_alpha: alpha,
            glyph_index: u32::MAX,
            rays_per_outline_po2: 0,
            reserved: [0; 2],
        }
    }

    //fn draw_triangle(&mut self, p0: TriangleVertex, p1: TriangleVertex, p2: TriangleVertex)

    fn draw_rectangle(
        &mut self,
        p0: TriangleVertex,
        p2: TriangleVertex,
        input_data: &mut TriglyphInputData,
        srgb: u32,
        alpha: f32,
    ) {
        let p1 = TriangleVertex::new(p0.x, p2.y);
        let p3 = TriangleVertex::new(p2.x, p0.y);
        input_data.verticies[self.num_verticies] = p0;
        input_data.verticies[self.num_verticies + 1] = p1;
        input_data.verticies[self.num_verticies + 2] = p2;
        input_data.verticies[self.num_verticies + 3] = p3;

        input_data.indicies[self.num_triangles] = TriangleIndicies {
            p0: self.num_verticies as u16,
            p1: (self.num_verticies + 1) as u16,
            p2: (self.num_verticies + 2) as u16,
        };
        input_data.indicies[self.num_triangles + 1] = TriangleIndicies {
            p0: (self.num_verticies + 3) as u16,
            p1: self.num_verticies as u16,
            p2: (self.num_verticies + 2) as u16,
        };

        let color_font = self.get_color_glyph(srgb, alpha);
        input_data.info[self.num_triangles] = color_font;
        input_data.info[self.num_triangles + 1] = color_font;

        self.num_verticies += 4;
        self.num_triangles += 2;
    }

    fn draw_glyph_line(
        &mut self,
        mut pos: (f32, f32),
        line: &str,
        pt_size: (u32, u32),
        input_data: &mut TriglyphInputData,
        srgb: u32,
        alpha: f32,
    ) {
        let mut color_glyph = self.get_color_glyph(srgb, alpha);
        self.glyphs.push_text_line(line);
        let render_info = self
            .glyphs
            .get_glyph_line_render_info(0, pt_size.0, self.dpi)
            .unwrap();

        //println!("Render Info Length: {}", render_info.len());
        for glri in render_info {
            if (glri.dimensions.0 == 0.0) || (glri.dimensions.1 == 0.0) {
                pos.0 += glri.advance * self.x_mult;
                continue;
            }
            //println!("Render Info {:?}", glri);
            let xy0 = (
                pos.0 + (glri.offset.0 * self.x_mult),
                pos.1 - (glri.offset.1 * self.y_mult),
            );
            let xy1 = (
                xy0.0 + (glri.dimensions.0 * self.x_mult),
                xy0.1 - (glri.dimensions.1 * self.y_mult),
            );
            let p0 = TriangleVertex {
                x: xy0.0,
                y: xy0.1,
                tex_x: glri.p0.0,
                tex_y: glri.p0.1,
            };
            let p1 = TriangleVertex {
                x: xy1.0,
                y: xy0.1,
                tex_x: glri.p1.0,
                tex_y: glri.p0.1,
            };
            let p2 = TriangleVertex {
                x: xy1.0,
                y: xy1.1,
                tex_x: glri.p1.0,
                tex_y: glri.p1.1,
            };
            let p3 = TriangleVertex {
                x: xy0.0,
                y: xy1.1,
                tex_x: glri.p0.0,
                tex_y: glri.p1.1,
            };
            input_data.verticies[self.num_verticies] = p0;
            input_data.verticies[self.num_verticies + 1] = p1;
            input_data.verticies[self.num_verticies + 2] = p2;
            input_data.verticies[self.num_verticies + 3] = p3;

            input_data.indicies[self.num_triangles] = TriangleIndicies {
                p0: self.num_verticies as u16,
                p1: (self.num_verticies + 1) as u16,
                p2: (self.num_verticies + 2) as u16,
            };
            input_data.indicies[self.num_triangles + 1] = TriangleIndicies {
                p0: (self.num_verticies + 3) as u16,
                p1: self.num_verticies as u16,
                p2: (self.num_verticies + 2) as u16,
            };

            color_glyph.glyph_index = glri.outline;
            color_glyph.rays_per_outline_po2 = pt_size.1;
            input_data.info[self.num_triangles] = color_glyph;
            input_data.info[self.num_triangles + 1] = color_glyph;

            self.num_verticies += 4;
            self.num_triangles += 2;

            pos.0 += glri.advance * self.x_mult;
        }
    }
}

impl swiftlet_graphics::VulkanTriglyphCallbacks for TriglyphExample {
    fn draw(&mut self, input_data: &mut TriglyphInputData, width: u32, height: u32) -> (u32, u32) {
        self.reset_draw_stats(width, height);

        self.draw_rectangle(
            TriangleVertex::new(-1.0, -1.0),
            TriangleVertex::new(1.0, 1.0),
            input_data,
            0xEEEEEE,
            1.0,
        );

        if self.state[0] {
            self.draw_rectangle(
                TriangleVertex::new(-0.25, -0.75),
                TriangleVertex::new(0.75, 0.75),
                input_data,
                0xA12312,
                1.0,
            );
        }

        self.draw_rectangle(
            TriangleVertex::new(-0.5, -0.5),
            TriangleVertex::new(0.5, 0.5),
            input_data,
            0x7FB5B5,
            0.5,
        );
        self.draw_rectangle(
            TriangleVertex::new(-0.75, -0.25),
            TriangleVertex::new(0.25, 0.25),
            input_data,
            0x1E213D,
            0.5,
        );
        self.draw_rectangle(
            TriangleVertex::new(-0.875, -0.125),
            TriangleVertex::new(0.125, 0.125),
            input_data,
            0xF3A505,
            0.5,
        );

        let mut origin = self.get_vertex_for_pixel(width >> 1, height >> 2);
        //origin.0 -= self.x_mult * 0.32;
        self.draw_glyph_line(origin, "1234567890", (32, 0), input_data, 0, 1.0);
        origin = self.get_vertex_for_pixel(20, 50);
        origin.0 -= self.x_mult * 0.25;
        origin.1 -= self.y_mult * 0.5;
        self.draw_glyph_line(
            origin,
            "The quick brown fox jumped over the lazy dog!",
            (32, 0),
            input_data,
            0,
            1.0,
        );
        origin.1 += 50.0 * self.y_mult;
        self.draw_glyph_line(
            origin,
            "The quick brown fox jumped over the lazy dog!",
            (32, 2),
            input_data,
            0,
            1.0,
        );

        origin = self.get_vertex_for_pixel(20, 400);
        self.draw_glyph_line(origin, "w W", (40, 0), input_data, 0, 1.0);

        origin = self.get_vertex_for_pixel(width >> 2, height >> 1);
        self.draw_glyph_line(origin, "@", (128, 0), input_data, 0, 1.0);
        origin.0 += 150.0 * self.x_mult;
        self.draw_glyph_line(origin, "@", (128, 1), input_data, 0, 1.0);
        origin.0 += 150.0 * self.x_mult;
        self.draw_glyph_line(origin, "@", (128, 2), input_data, 0, 1.0);

        self.times_called += 1;

        ((self.num_verticies as u32), (self.num_triangles as u32))
    }

    fn key_pressed(&mut self, key_code: KeyCode) -> bool {
        //println!("Got Key Code: {:?}", key_code);
        match key_code {
            KeyCode::Enter => self.state[0] = !self.state[0],
            KeyCode::UpArrow => self.should_draw = true,
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

    fn tick(&mut self) -> bool {
        if !self.should_draw {
            false
        } else {
            self.should_draw = false;
            true
        }
    }
}
