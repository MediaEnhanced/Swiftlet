//Media Enhanced Swiftlet Cross-Compile Friendly Graphics Triangle Draw Example
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
use swiftlet_graphics::vulkan::{TriangleColorFont, TriangleIndicies, TriangleVertex};

const FONT_PATH: &str = "font/opensans/OpenSans-Regular.ttf"; // Location of the Font

fn main() -> std::io::Result<()> {
    println!("Graphics Window Starting!");

    let triangle_draw = TriangleDraw::new();

    let (mut window, signaler) =
        match swiftlet_graphics::VulkanTriangle::new(1280, 720, 104, &triangle_draw.glyphs) {
            Ok((w, s)) => (w, s),
            Err(e) => {
                println!("Window Creation Error: {:?}", e);
                return Err(std::io::Error::from(std::io::ErrorKind::Other));
            }
        };

    let thread_handle = std::thread::spawn(|| signaler_thread(signaler));

    if let Err(e) = window.run(triangle_draw) {
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
        Err(_e) => return,
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

struct TriangleDraw {
    times_called: u64,
    num_verticies: usize,
    num_triangles: usize,
    x_max: u32,
    y_max: u32,
    x_mult: f32,
    y_mult: f32,
    linear_rgb_lut: [f32; 256],
    glyphs: swiftlet_graphics::font::FontGlyphs,
}

impl TriangleDraw {
    fn new() -> Self {
        let mut linear_rgb_lut = [0.0; 256];
        for (ind, v) in linear_rgb_lut.iter_mut().enumerate() {
            *v = get_linear_rgb_float_from_srgb_byte(ind as u8)
        }
        let font_data = std::fs::read(std::path::Path::new(FONT_PATH)).unwrap();
        let glyphs = swiftlet_graphics::font::FontGlyphs::new(&font_data).unwrap();
        //glyphs.print_outline('2');
        TriangleDraw {
            times_called: 0,
            num_verticies: 0,
            num_triangles: 0,
            x_max: 0,
            y_max: 0,
            x_mult: 0.0,
            y_mult: 0.0,
            linear_rgb_lut,
            glyphs,
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

    fn get_color_font(&self, srgb: u32, alpha: f32) -> TriangleColorFont {
        let mut linear_rgb = self.get_linear_rgb_from_srgb(srgb);
        for l in &mut linear_rgb {
            *l *= alpha;
        }
        TriangleColorFont {
            linear_rgb,
            linear_alpha: alpha,
            font_index: [0; 4],
        }
    }

    //fn draw_triangle(&mut self, p0: TriangleVertex, p1: TriangleVertex, p2: TriangleVertex)

    fn draw_rectangle(
        &mut self,
        p0: TriangleVertex,
        p2: TriangleVertex,
        verticies: &mut [TriangleVertex],
        indicies: &mut [TriangleIndicies],
        colors: &mut [TriangleColorFont],
        srgb: u32,
        alpha: f32,
    ) {
        let p1 = TriangleVertex::new(p0.x, p2.y);
        let p3 = TriangleVertex::new(p2.x, p0.y);
        verticies[self.num_verticies] = p0;
        verticies[self.num_verticies + 1] = p1;
        verticies[self.num_verticies + 2] = p2;
        verticies[self.num_verticies + 3] = p3;

        indicies[self.num_triangles] = TriangleIndicies {
            p0: self.num_verticies as u16,
            p1: (self.num_verticies + 1) as u16,
            p2: (self.num_verticies + 2) as u16,
        };
        indicies[self.num_triangles + 1] = TriangleIndicies {
            p0: (self.num_verticies + 3) as u16,
            p1: self.num_verticies as u16,
            p2: (self.num_verticies + 2) as u16,
        };

        let color_font = self.get_color_font(srgb, alpha);
        colors[self.num_triangles] = color_font;
        colors[self.num_triangles + 1] = color_font;

        self.num_verticies += 4;
        self.num_triangles += 2;
    }

    fn draw_glyph(
        &mut self,
        origin: (f32, f32),
        character: char,
        pt_size: u32,
        verticies: &mut [TriangleVertex],
        indicies: &mut [TriangleIndicies],
        colors: &mut [TriangleColorFont],
        srgb: u32,
        alpha: f32,
    ) {
        let ci = self.glyphs.get_character_info(character, pt_size);
        //println!("W | H: {}, {}", ci.1, ci.2);
        let width = ci.1 * self.x_mult;
        let height = ci.2 * self.y_mult;
        //println!("UV: ({}, {}) ({}, {})", ci.3, ci.4, ci.5, ci.6);
        let p0 = TriangleVertex {
            x: origin.0,
            y: origin.1,
            tex_x: ci.3,
            tex_y: ci.4,
        };
        let p1 = TriangleVertex {
            x: origin.0 + width,
            y: origin.1,
            tex_x: ci.5,
            tex_y: ci.4,
        };
        let p2 = TriangleVertex {
            x: origin.0 + width,
            y: origin.1 - height,
            tex_x: ci.5,
            tex_y: ci.6,
        };
        let p3 = TriangleVertex {
            x: origin.0,
            y: origin.1 - height,
            tex_x: ci.3,
            tex_y: ci.6,
        };
        verticies[self.num_verticies] = p0;
        verticies[self.num_verticies + 1] = p1;
        verticies[self.num_verticies + 2] = p2;
        verticies[self.num_verticies + 3] = p3;

        indicies[self.num_triangles] = TriangleIndicies {
            p0: self.num_verticies as u16,
            p1: (self.num_verticies + 1) as u16,
            p2: (self.num_verticies + 2) as u16,
        };
        indicies[self.num_triangles + 1] = TriangleIndicies {
            p0: (self.num_verticies + 3) as u16,
            p1: self.num_verticies as u16,
            p2: (self.num_verticies + 2) as u16,
        };

        let mut color_font = self.get_color_font(srgb, alpha);

        color_font.font_index = [ci.0, 0, 0, 0];
        colors[self.num_triangles] = color_font;
        colors[self.num_triangles + 1] = color_font;

        self.num_verticies += 4;
        self.num_triangles += 2;
    }

    fn draw_glyph_line(
        &mut self,
        origin: (f32, f32),
        line: &str,
        pt_size: u32,
        verticies: &mut [TriangleVertex],
        indicies: &mut [TriangleIndicies],
        colors: &mut [TriangleColorFont],
        srgb: u32,
        alpha: f32,
    ) {
    }
}

impl swiftlet_graphics::VulkanTriangleCallbacks for TriangleDraw {
    fn draw_triangles(
        &mut self,
        verticies: &mut [TriangleVertex],
        indicies: &mut [TriangleIndicies],
        colors: &mut [TriangleColorFont],
        width: u32,
        height: u32,
    ) -> (u32, u32) {
        self.reset_draw_stats(width, height);

        self.draw_rectangle(
            TriangleVertex::new(-1.0, -1.0),
            TriangleVertex::new(1.0, 1.0),
            verticies,
            indicies,
            colors,
            0xEEEEEE,
            1.0,
        );

        // self.draw_rectangle(
        //     TriangleVertex::new(-0.25, -0.75),
        //     TriangleVertex::new(0.75, 0.75),
        //     verticies,
        //     indicies,
        //     colors,
        //     0xA12312,
        //     1.0,
        // );
        // self.draw_rectangle(
        //     TriangleVertex::new(-0.5, -0.5),
        //     TriangleVertex::new(0.5, 0.5),
        //     verticies,
        //     indicies,
        //     colors,
        //     0x7FB5B5,
        //     0.5,
        // );
        // self.draw_rectangle(
        //     TriangleVertex::new(-0.75, -0.25),
        //     TriangleVertex::new(0.25, 0.25),
        //     verticies,
        //     indicies,
        //     colors,
        //     0x1E213D,
        //     0.5,
        // );
        // self.draw_rectangle(
        //     TriangleVertex::new(-0.875, -0.125),
        //     TriangleVertex::new(0.125, 0.125),
        //     verticies,
        //     indicies,
        //     colors,
        //     0xF3A505,
        //     0.5,
        // );

        let mut origin = self.get_vertex_for_pixel(width >> 1, height >> 1);
        //origin.0 -= self.x_mult * 0.32;
        self.draw_glyph(origin, '2', 32, verticies, indicies, colors, 0, 1.0);

        self.times_called += 1;

        ((self.num_verticies as u32), (self.num_triangles as u32))
    }
}
