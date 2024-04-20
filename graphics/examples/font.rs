//Media Enhanced Swiftlet Cross-Compile Friendly Graphics Render Font in Window Example
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

use std::time::Instant;

const FONT_PATH: &str = "font/opensans/OpenSans-Regular.ttf"; // Location of the Opus Song in Ogg file format

fn main() -> std::io::Result<()> {
    println!("Graphics Render Font Example Starting!");

    let font_render = match std::fs::read(std::path::Path::new(FONT_PATH)) {
        Ok(font_file_data) => match FontRender::new(&font_file_data) {
            Some(fr) => fr,
            None => {
                println!("Font Render Creation Error");
                return Err(std::io::Error::from(std::io::ErrorKind::Other));
            }
        },
        Err(e) => {
            println!("Could not find font file!");
            return Err(e);
        }
    };

    let (mut window, signaler) = match swiftlet_graphics::VulkanWindow::new(1280, 720) {
        Ok((w, s)) => (w, s),
        Err(e) => {
            println!("Window Creation Error: {:?}", e);
            return Err(std::io::Error::from(std::io::ErrorKind::Other));
        }
    };

    let thread_handle = std::thread::spawn(|| signaler_thread(signaler));

    if let Err(e) = window.run(font_render) {
        println!("Window Run Error: {:?}", e);
    }
    drop(window);

    println!("Waiting for signaler thread to finish!");
    let _ = thread_handle.join();

    Ok(())
}

fn signaler_thread(mut signaler: swiftlet_graphics::OsEventSignaler) {
    let _ = signaler.signal();
}

struct FontRender {
    glyphs: swiftlet_graphics::font::FontGlyphs,
}

impl FontRender {
    fn new(font_file_data: &[u8]) -> Option<Self> {
        let glyphs = swiftlet_graphics::font::FontGlyphs::new(font_file_data)?;
        //glyphs.print_outline('A');
        Some(FontRender { glyphs })
    }
}

impl swiftlet_graphics::VulkanWindowCallbacks for FontRender {
    fn draw(&mut self, pixel_data: &mut [u32], width: u32, height: u32) {
        // Fill with white
        pixel_data.fill(0xFFFFFF);

        let instant_start = Instant::now();

        let origin_index = (300 * width as usize) + 300;
        pixel_data[origin_index - 1] = 0xFF0000;
        self.glyphs
            .render_character(pixel_data, width as usize, origin_index, 'X', 48);

        let instant_end = Instant::now();
        println!("Draw Profile Duration: {:?}", instant_end - instant_start);
    }
}
