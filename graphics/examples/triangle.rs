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
use swiftlet_graphics::vulkan::{TriangleIndicies, TriangleVertex};

fn main() -> std::io::Result<()> {
    println!("Graphics Window Starting!");

    let triangle_draw = TriangleDraw::new();

    let (mut window, signaler) = match swiftlet_graphics::VulkanTriangle::new(1440, 900, 100) {
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
    loop {
        std::thread::sleep(Duration::from_secs(1));
        match signaler.signal() {
            Ok(_) => {}
            Err(_e) => break,
        }
    }
}

struct TriangleDraw {
    times_called: u64,
}

impl TriangleDraw {
    fn new() -> Self {
        TriangleDraw { times_called: 0 }
    }
}

impl swiftlet_graphics::VulkanTriangleCallbacks for TriangleDraw {
    fn draw_triangles(
        &mut self,
        verticies: &mut [TriangleVertex],
        indicies: &mut [TriangleIndicies],
        width: u32,
        height: u32,
    ) -> (u32, u32) {
        if (self.times_called & 1) == 0 {
            verticies[0] = TriangleVertex { x: 0.0, y: 0.0 };
            verticies[1] = TriangleVertex { x: 0.5, y: 0.0 };
            verticies[2] = TriangleVertex { x: 0.5, y: 0.5 };
        } else {
            verticies[0] = TriangleVertex { x: 0.0, y: 0.0 };
            verticies[1] = TriangleVertex { x: -0.5, y: 0.0 };
            verticies[2] = TriangleVertex { x: -0.5, y: 0.5 };
        }

        indicies[0] = TriangleIndicies {
            p0: 0,
            p1: 1,
            p2: 2,
        };

        self.times_called += 1;

        (3, 1)
    }
}
