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
pub use os::KeyCode;
pub use os::OsEventSignaler;
use vulkan::GlyphSegment;
//use os::{AudioInput, AudioOutput, AudioOwner};

pub mod color;
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

fn create_glyph_data_from_glyph_outline_data(
    glyph_outline_data: &[font::GlyphOutlineData],
    rays_per_outline_po2: u8,
) -> vulkan::GlyphData {
    let num_glyphs = glyph_outline_data.len() as u32;

    let num_offsets = glyph_outline_data.len() + 1;
    let additional_len = (4 - (num_offsets & 0x3)) & 0x3;
    let mut segment_offsets = Vec::with_capacity(num_offsets + additional_len);
    let mut offset = 0;
    segment_offsets.push(offset);
    for g in glyph_outline_data {
        offset += g.get_num_segments();
        segment_offsets.push(offset);
    }
    for _i in 0..additional_len {
        segment_offsets.push(0);
    }

    let mut segment_data = Vec::with_capacity(segment_offsets[num_glyphs as usize] as usize);
    for g in glyph_outline_data {
        let segments = g.get_segment_data();
        for s in segments {
            let glyph_segment = if let Some((xq, yq)) = s.pq {
                GlyphSegment {
                    is_quad: 1.0,
                    y0: s.p0.1,
                    y1: s.p1.1,
                    yq,
                    xmax: s.x_max,
                    x0: s.p0.0,
                    x1: s.p1.0,
                    xq,
                }
            } else {
                GlyphSegment {
                    is_quad: 0.0,
                    y0: s.p0.1,
                    y1: s.p1.1,
                    yq: 0.0,
                    xmax: s.x_max,
                    x0: s.p0.0,
                    x1: s.p1.0,
                    xq: 0.0,
                }
            };
            segment_data.push(glyph_segment);
        }
    }

    println!("Rays_per_outline: {}", 1 << rays_per_outline_po2);

    vulkan::GlyphData {
        num_glyphs,
        num_aliasing: rays_per_outline_po2 as u32,
        segment_offsets,
        segment_data,
    }
}

pub trait VulkanTriglyphCallbacks {
    fn draw(
        &mut self,
        input_data: &mut vulkan::TriglyphInputData,
        width: u32,
        height: u32,
    ) -> (u32, u32);

    fn key_pressed(&mut self, key_code: KeyCode) -> bool;

    fn tick(&mut self) -> bool;
}

pub struct VulkanTriglyph {
    swapchain_triglyph_render: vulkan::SwapchainTriglyphRender,
    window: os::OsWindow,
    draw_trigger_external: os::OsEvent,
    render_width: u32,
    render_height: u32,
}

impl VulkanTriglyph {
    /// max_triangles needs to be a multiple of 8
    pub fn new(
        width: u32,
        height: u32,
        max_triangles: u32,
        glyph_outline_data: (&[font::GlyphOutlineData], u8),
        use_validation_layers: bool,
    ) -> Result<(Self, os::OsEventSignaler), Error> {
        let layer_names = if use_validation_layers {
            vec![vulkan::LAYER_NAME_VALIDATION]
        } else {
            vec![]
        };
        //let layer_names = [];
        //let layer_names = ;

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

        let glyph_data =
            create_glyph_data_from_glyph_outline_data(glyph_outline_data.0, glyph_outline_data.1);

        let swapchain_triangle_render =
            match vulkan::SwapchainTriglyphRender::new(swapchain, max_triangles, glyph_data) {
                Ok(s) => s,
                Err(e) => return Err(Error::VulkanError(e)),
            };

        Ok((
            VulkanTriglyph {
                swapchain_triglyph_render: swapchain_triangle_render,
                window,
                draw_trigger_external,
                render_width: width,
                render_height: height,
            },
            signaler,
        ))
    }

    pub fn run(&mut self, callback: &mut impl VulkanTriglyphCallbacks) -> Result<(), Error> {
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
                Ok(os::OsWindowState::KeyPressed(key_code)) => {
                    if callback.key_pressed(key_code) {
                        if let Err(e) = self.window.close_window() {
                            return Err(Error::OsError(e));
                        }
                    }
                    continue;
                }
                Ok(_) => {}
                Err(e) => return Err(Error::OsError(e)),
            }
            let should_draw = callback.tick();
            if should_draw {
                match self.swapchain_triglyph_render.get_data() {
                    Ok(mut input_data) => {
                        let (num_verticies, num_triangles) =
                            callback.draw(&mut input_data, self.render_width, self.render_height);
                        if let Err(e) = self.swapchain_triglyph_render.render(
                            num_verticies,
                            num_triangles,
                            self.render_width,
                            self.render_height,
                        ) {
                            return Err(Error::VulkanError(e));
                        }
                    }
                    Err(e) => return Err(Error::VulkanError(e)),
                }
            }
            match self.draw_trigger_external.check() {
                Ok(false) => {}
                Ok(true) => {
                    if !should_draw {
                        match self.swapchain_triglyph_render.get_data() {
                            Ok(mut input_data) => {
                                let (num_verticies, num_triangles) = callback.draw(
                                    &mut input_data,
                                    self.render_width,
                                    self.render_height,
                                );
                                if let Err(e) = self.swapchain_triglyph_render.render(
                                    num_verticies,
                                    num_triangles,
                                    self.render_width,
                                    self.render_height,
                                ) {
                                    return Err(Error::VulkanError(e));
                                }
                            }
                            Err(e) => return Err(Error::VulkanError(e)),
                        }
                    }
                }
                Err(e) => return Err(Error::OsError(e)),
            }
        }

        Ok(())
    }
}

// pub enum TickActions<'a> {
//     ShouldDraw,
//     NewFontData((&'a [u8], u32)),
//     NewFontIndex((usize, u32)),
//     AddOutlineData((usize, char, char)),
//     //New Icons, New Images
// }

pub trait Vulkan2dWindowCallbacks {
    fn draw(&mut self, primitives: &mut vulkan::Primitives2d, glyphs: &font::Glyphs);

    /// Return true if the window should be closed
    fn key_pressed(&mut self, key_code: KeyCode) -> bool;

    /// Return true if the draw callback should be called
    fn tick(&mut self, glyphs: &mut font::Glyphs) -> bool;
}

pub struct Vulkan2dWindow {
    //jpegxl decomp
    //icons
    glyphs: font::Glyphs,
    render: vulkan::TwoDimensionRender,
    //draw_trigger_external: os::OsEvent,
    window: os::OsWindow,
}

impl Vulkan2dWindow {
    pub fn new(
        width: u32,
        height: u32,
        reserved_cpu_mem: usize,
        glyphs: font::Glyphs,
        use_validation_layers: bool,
        //) -> Result<(Self, os::OsEventSignaler), Error> {
    ) -> Result<Self, Error> {
        let (layer_names, extension_names) = if use_validation_layers {
            (
                vec![vulkan::LAYER_NAME_VALIDATION],
                vec![
                    vulkan::INSTANCE_EXTENSION_NAME_SURFACE,
                    vulkan::INSTANCE_EXTENSION_NAME_OS_SURFACE,
                    vulkan::INSTANCE_EXTENSION_NAME_DEBUG,
                ],
            )
        } else {
            (
                vec![],
                vec![
                    vulkan::INSTANCE_EXTENSION_NAME_SURFACE,
                    vulkan::INSTANCE_EXTENSION_NAME_OS_SURFACE,
                ],
            )
        };

        let instance = match vulkan::Instance::new(
            "SwiftletVulkan2dApp",
            "SwiftletVulkan2dEngine",
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
        println!("Window DPI: {}", window.get_dpi());

        // let draw_trigger_external = match os::OsEvent::new() {
        //     Ok(t) => t,
        //     Err(e) => return Err(Error::OsError(e)),
        // };
        // let signaler = draw_trigger_external.create_signaler();

        let surface_parameters = window.get_surface_parameters();
        let swapchain = match vulkan::Swapchain::new(physical_device, surface_parameters) {
            Ok(s) => s,
            Err(e) => return Err(Error::VulkanError(e)),
        };

        let glyph_outline_data = glyphs.get_glyph_outline_data();
        let glyph_data =
            create_glyph_data_from_glyph_outline_data(glyph_outline_data.0, glyph_outline_data.1);

        let render = match vulkan::TwoDimensionRender::new(swapchain, reserved_cpu_mem, glyph_data)
        {
            Ok(s) => s,
            Err(e) => return Err(Error::VulkanError(e)),
        };

        // Ok((
        //     Vulkan2dWindow {
        //         glyphs,
        //         render,
        //         //draw_trigger_external,
        //         window,
        //     },
        //     signaler,
        // ))

        Ok(Vulkan2dWindow {
            glyphs,
            render,
            window,
        })
    }

    pub fn run(
        &mut self,
        callback: &mut impl Vulkan2dWindowCallbacks,
        min_time_between_processing_window_msgs: std::time::Duration,
    ) -> Result<(), Error> {
        let timer = match os::OsWait::new() {
            Ok(t) => t,
            Err(e) => return Err(Error::OsError(e)),
        };

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
                Ok(os::OsWindowState::KeyPressed(key_code)) => {
                    if callback.key_pressed(key_code) {
                        if let Err(e) = self.window.close_window() {
                            return Err(Error::OsError(e));
                        }
                    }
                    continue;
                }
                Ok(_) => {}
                Err(e) => return Err(Error::OsError(e)),
            }
            let next_process_instant =
                std::time::Instant::now() + min_time_between_processing_window_msgs;
            if callback.tick(&mut self.glyphs) {
                match self.render.get_primitives() {
                    Ok(mut primitives) => {
                        callback.draw(&mut primitives, &self.glyphs);
                        let (num_verticies, num_triangles) = primitives.get_num_verts_and_tris();
                        if let Err(e) = self.render.render(num_verticies, num_triangles) {
                            return Err(Error::VulkanError(e));
                        }
                    }
                    Err(e) => return Err(Error::VulkanError(e)),
                }
            }
            let current_instant = std::time::Instant::now();
            if current_instant > next_process_instant {
                let timeout_duration = current_instant - next_process_instant;
                if let Err(e) = timer.wait(timeout_duration) {
                    return Err(Error::OsError(e));
                }
            }
        }

        Ok(())
    }
}
