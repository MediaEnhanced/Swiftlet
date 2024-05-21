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

#![allow(dead_code)] // Temporary

pub mod vulkan;

#[cfg_attr(target_os = "windows", path = "windows/os.rs")]
#[cfg_attr(target_os = "linux", path = "linux/os.rs")]
#[cfg_attr(target_os = "macos", path = "mac/os.rs")]
mod os;
pub use os::OsEventSignaler;
use vulkan::GlyphSegment;
//use os::{AudioInput, AudioOutput, AudioOwner};

pub mod font;

// #[cfg(feature = "opus")]
// pub mod opus;

#[derive(Debug)]
pub enum Error {
    VulkanError(vulkan::Error),
    OsError(os::OsError),
    CannotFindPhysicalDevice,
}

pub struct BasicWindow {
    window: os::OsWindow,
    signal_watcher: os::OsEvent,
}

impl BasicWindow {
    pub fn new(width: u32, height: u32) -> Result<(Self, os::OsEventSignaler), Error> {
        let window = match os::OsWindow::new(width, height) {
            Ok(w) => w,
            Err(e) => return Err(Error::OsError(e)),
        };

        let signal_watcher = match os::OsEvent::new() {
            Ok(t) => t,
            Err(e) => return Err(Error::OsError(e)),
        };
        let signaler = signal_watcher.create_signaler();

        Ok((
            BasicWindow {
                window,
                signal_watcher,
            },
            signaler,
        ))
    }

    pub fn run(&mut self) -> Result<(), Error> {
        loop {
            match self.window.process_messages() {
                Ok(os::OsWindowState::Normal) => {}
                Ok(os::OsWindowState::CloseAttempt) => {
                    if let Err(e) = self.window.close_window() {
                        return Err(Error::OsError(e));
                    }
                }
                Ok(os::OsWindowState::ShouldDrop) => {
                    break;
                }
                Ok(_) => {}
                Err(e) => return Err(Error::OsError(e)),
            }
            match self.signal_watcher.check() {
                Ok(false) => {}
                Ok(true) => println!("Signaler Called!"),
                Err(e) => return Err(Error::OsError(e)),
            }
        }
        Ok(())
    }
}

pub trait VulkanWindowCallbacks {
    fn draw(&mut self, pixel_data: &mut [u32], width: u32, height: u32);
}

pub struct VulkanWindow {
    swapchain_cpu_render: vulkan::SwapchainCpuRender,
    window: os::OsWindow,
    draw_trigger_external: os::OsEvent,
    render_width: u32,
    render_height: u32,
}

impl VulkanWindow {
    pub fn new(width: u32, height: u32) -> Result<(Self, os::OsEventSignaler), Error> {
        let layer_names = [];
        //let layer_names = [vulkan::LAYER_NAME_VALIDATION];

        let extension_names = [
            vulkan::INSTANCE_EXTENSION_NAME_SURFACE,
            vulkan::INSTANCE_EXTENSION_NAME_OS_SURFACE,
            vulkan::INSTANCE_EXTENSION_NAME_DEBUG,
        ];

        let instance = match vulkan::Instance::new(
            "App Name",
            "Engine Name",
            &layer_names,
            &extension_names,
        ) {
            Ok(i) => i,
            Err(e) => return Err(Error::VulkanError(e)),
        };

        let physical_device = match os::get_device_luid() {
            Ok(Some(luid)) => match vulkan::PhysicalDevice::new_from_luid(instance, luid) {
                Ok(Some(d)) => d,
                Ok(None) => return Err(Error::CannotFindPhysicalDevice),
                Err(e) => return Err(Error::VulkanError(e)),
            },
            Ok(None) => match vulkan::PhysicalDevice::new(instance) {
                Ok(Some(d)) => d,
                Ok(None) => return Err(Error::CannotFindPhysicalDevice),
                Err(e) => return Err(Error::VulkanError(e)),
            },
            Err(e) => return Err(Error::OsError(e)),
        };

        let window = match os::OsWindow::new(width, height) {
            Ok(w) => w,
            Err(e) => return Err(Error::OsError(e)),
        };

        let draw_trigger_external = match os::OsEvent::new() {
            Ok(t) => t,
            Err(e) => return Err(Error::OsError(e)),
        };
        let signaler = draw_trigger_external.create_signaler();

        let surface_parameters = window.get_surface_parameters();
        let swapchain = match vulkan::Swapchain::new(physical_device, surface_parameters) {
            Ok(s) => s,
            Err(e) => return Err(Error::VulkanError(e)),
        };

        let swapchain_cpu_render = match vulkan::SwapchainCpuRender::new(swapchain, width, height) {
            Ok(s) => s,
            Err(e) => return Err(Error::VulkanError(e)),
        };

        Ok((
            VulkanWindow {
                swapchain_cpu_render,
                window,
                draw_trigger_external,
                render_width: width,
                render_height: height,
            },
            signaler,
        ))
    }

    pub fn run(&mut self, mut callback: impl VulkanWindowCallbacks) -> Result<(), Error> {
        // Maybe one-time setup/start code here in future
        loop {
            match self.window.process_messages() {
                Ok(os::OsWindowState::Normal) => {}
                Ok(os::OsWindowState::CloseAttempt) => {
                    if let Err(e) = self.window.close_window() {
                        return Err(Error::OsError(e));
                    }
                }
                Ok(os::OsWindowState::ShouldDrop) => {
                    break;
                }
                Ok(_) => {}
                Err(e) => return Err(Error::OsError(e)),
            }
            match self.draw_trigger_external.check() {
                Ok(false) => {}
                Ok(true) => match self.swapchain_cpu_render.get_buffer() {
                    Ok(data) => {
                        callback.draw(data, self.render_width, self.render_height);
                        match self.swapchain_cpu_render.render() {
                            Ok(_) => {
                                // match self
                                //     .swapchain_cpu_render
                                //     .buffer_check(self.render_width, self.render_height)
                                // {
                                //     Ok(_) => {}
                                //     Err(e) => return Err(Error::VulkanError(e)),
                                // }
                            }
                            Err(e) => return Err(Error::VulkanError(e)),
                        }
                    }
                    Err(e) => return Err(Error::VulkanError(e)),
                },
                Err(e) => return Err(Error::OsError(e)),
            }
        }

        Ok(())
    }
}

pub trait VulkanTriangleCallbacks {
    fn draw_triangles(
        &mut self,
        verticies: &mut [vulkan::TriangleVertex],
        indicies: &mut [vulkan::TriangleIndicies],
        colors: &mut [vulkan::TriangleColorFont],
        width: u32,
        height: u32,
    ) -> (u32, u32);
}

fn create_glyph_data_from_font_glyphs(font_glyphs: &font::FontGlyphs) -> vulkan::GlyphData {
    let num_glyphs = font_glyphs.get_num_glyphs();
    let segment_offsets = font_glyphs.get_segment_offsets();
    let mut segment_data = Vec::with_capacity(segment_offsets[num_glyphs as usize] as usize);
    for glyph_index in 0..num_glyphs {
        let segments = font_glyphs.get_segment_data(glyph_index);
        for s in segments {
            segment_data.push(GlyphSegment::new(
                s.is_quadratic,
                (s.x0, s.y0),
                (s.x1, s.y1),
                (s.xq, s.yq),
            ));
        }
    }

    vulkan::GlyphData {
        num_glyphs,
        num_aliasing: 1,
        segment_offsets,
        segment_data,
    }
}

pub struct VulkanTriangle {
    swapchain_triangle_render: vulkan::SwapchainTriangleRender,
    window: os::OsWindow,
    draw_trigger_external: os::OsEvent,
    render_width: u32,
    render_height: u32,
}

impl VulkanTriangle {
    /// max_triangles needs to be a multiple of 8
    pub fn new(
        width: u32,
        height: u32,
        max_triangles: u32,
        font_glyphs: &font::FontGlyphs,
    ) -> Result<(Self, os::OsEventSignaler), Error> {
        //let layer_names = [];
        let layer_names = [vulkan::LAYER_NAME_VALIDATION];

        let extension_names = [
            vulkan::INSTANCE_EXTENSION_NAME_SURFACE,
            vulkan::INSTANCE_EXTENSION_NAME_OS_SURFACE,
            vulkan::INSTANCE_EXTENSION_NAME_DEBUG,
        ];

        let instance = match vulkan::Instance::new(
            "App Name",
            "Engine Name",
            &layer_names,
            &extension_names,
        ) {
            Ok(i) => i,
            Err(e) => return Err(Error::VulkanError(e)),
        };

        let physical_device = match os::get_device_luid() {
            Ok(Some(luid)) => match vulkan::PhysicalDevice::new_from_luid(instance, luid) {
                Ok(Some(d)) => d,
                Ok(None) => return Err(Error::CannotFindPhysicalDevice),
                Err(e) => return Err(Error::VulkanError(e)),
            },
            Ok(None) => match vulkan::PhysicalDevice::new(instance) {
                Ok(Some(d)) => d,
                Ok(None) => return Err(Error::CannotFindPhysicalDevice),
                Err(e) => return Err(Error::VulkanError(e)),
            },
            Err(e) => return Err(Error::OsError(e)),
        };

        let window = match os::OsWindow::new(width, height) {
            Ok(w) => w,
            Err(e) => return Err(Error::OsError(e)),
        };

        let draw_trigger_external = match os::OsEvent::new() {
            Ok(t) => t,
            Err(e) => return Err(Error::OsError(e)),
        };
        let signaler = draw_trigger_external.create_signaler();

        let surface_parameters = window.get_surface_parameters();
        let swapchain = match vulkan::Swapchain::new(physical_device, surface_parameters) {
            Ok(s) => s,
            Err(e) => return Err(Error::VulkanError(e)),
        };

        let glyph_data = create_glyph_data_from_font_glyphs(font_glyphs);

        let swapchain_triangle_render =
            match vulkan::SwapchainTriangleRender::new(swapchain, max_triangles, glyph_data) {
                Ok(s) => s,
                Err(e) => return Err(Error::VulkanError(e)),
            };

        Ok((
            VulkanTriangle {
                swapchain_triangle_render,
                window,
                draw_trigger_external,
                render_width: width,
                render_height: height,
            },
            signaler,
        ))
    }

    pub fn run(&mut self, mut callback: impl VulkanTriangleCallbacks) -> Result<(), Error> {
        // Maybe one-time setup/start code here in future
        loop {
            match self.window.process_messages() {
                Ok(os::OsWindowState::Normal) => {}
                Ok(os::OsWindowState::CloseAttempt) => {
                    if let Err(e) = self.window.close_window() {
                        return Err(Error::OsError(e));
                    }
                }
                Ok(os::OsWindowState::ShouldDrop) => {
                    break;
                }
                Ok(_) => {}
                Err(e) => return Err(Error::OsError(e)),
            }
            match self.draw_trigger_external.check() {
                Ok(false) => {}
                Ok(true) => match self.swapchain_triangle_render.get_verticies_and_indicies() {
                    Ok((verticies, indicies, colors)) => {
                        let (num_verticies, num_triangles) = callback.draw_triangles(
                            verticies,
                            indicies,
                            colors,
                            self.render_width,
                            self.render_height,
                        );
                        if let Err(e) = self.swapchain_triangle_render.render(
                            num_verticies,
                            num_triangles,
                            self.render_width,
                            self.render_height,
                        ) {
                            return Err(Error::VulkanError(e));
                        }
                    }
                    Err(e) => return Err(Error::VulkanError(e)),
                },
                Err(e) => return Err(Error::OsError(e)),
            }
        }

        Ok(())
    }
}
