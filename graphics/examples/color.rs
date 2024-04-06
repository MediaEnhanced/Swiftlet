//Media Enhanced Swiftlet Cross-Compile Friendly Graphics Window Example
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

fn main() -> std::io::Result<()> {
    println!("Graphics Window Starting!");

    let simple_display = SimpleDisplay::new();

    let (mut window, signaler) = match swiftlet_graphics::Window::new(1440, 900) {
        Ok((w, s)) => (w, s),
        Err(e) => {
            println!("Window Creation Error: {:?}", e);
            return Err(std::io::Error::from(std::io::ErrorKind::Other));
        }
    };

    let thread_handle = std::thread::spawn(|| signaler_thread(signaler));

    if let Err(e) = window.run(simple_display) {
        println!("Window Run Error: {:?}", e);
    }
    drop(window);

    println!("Waiting for signaler thread to finish!");
    let _ = thread_handle.join();

    Ok(())
}

fn signaler_thread(signaler: swiftlet_graphics::OsEventSignaler) {
    loop {
        std::thread::sleep(Duration::from_secs(1));
        match signaler.signal() {
            Ok(_) => {}
            Err(_e) => break,
        }
    }
}

struct SimpleDisplay {
    color: u64,
    //font: Vec<u8>,
}

impl SimpleDisplay {
    fn new() -> Self {
        SimpleDisplay {
            color: 0,
            //font: Vec::new(),
        }
    }
}

impl swiftlet_graphics::WindowCallbacks for SimpleDisplay {
    fn draw(&mut self, pixel_data: &mut [u32], width: u32, height: u32) {
        if pixel_data.len() != (width * height) as usize {
            println!("Pixel Data Len: {}", pixel_data.len());
        }

        let pixel_color = match self.color {
            0 => 0xFF0000FF,
            1 => 0xFF00FF00,
            2 => 0xFFFF0000,
            3 => 0xFFFFFFFF,
            _ => 0xFF000000,
        };

        // Draw Logic Here
        for d in pixel_data {
            *d = pixel_color;
        }
        // for h in 0..height as usize {
        //     pixel_data[h * width as usize] = pixel_color;
        // }

        self.color += 1;
        if self.color >= 4 {
            self.color = 0;
        }
    }
}
