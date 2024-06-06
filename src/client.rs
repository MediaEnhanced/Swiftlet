//Media Enhanced Swiftlet Rust Realtime Media Internet Communications
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

const AUDIO_FILES: [&str; 3] = [
    "audio/EnterVoice.opus",
    "audio/ExitVoice.opus",
    "audio/song.opus",
];
const TRANSFER_AUDIO: &str = "audio/transfer.opus";
const FONT_PATH: &str = "font/roboto/Roboto-Regular.ttf";

pub(crate) mod audio;
use swiftlet_audio::opus::OpusData;

use crate::communication::{
    ClientCommand, NetworkCommand, NetworkStateConnection, NetworkStateMessage, PopError,
    TerminalAudioInCommands, TerminalAudioOutCommands, TerminalAudioThreadChannels,
    TerminalNetworkThreadChannels,
};

use swiftlet_graphics::vulkan::{
    TriangleColorGlyph, TriangleIndicies, TriangleVertex, TriglyphInputData,
};
use swiftlet_graphics::KeyCode;
use swiftlet_quic::endpoint::SocketAddr;

struct Client {
    is_in_vc: bool,
    server_name: String,
    server_address: SocketAddr,
    connections: Vec<NetworkStateConnection>,
    my_conn_ind: Option<usize>,

    network_channels: TerminalNetworkThreadChannels,
    audio_channels: TerminalAudioThreadChannels,
    already_transfered: bool,

    dpi: f32,
    num_verticies: usize,
    num_triangles: usize,
    x_max: u32,
    y_max: u32,
    x_mult: f32,
    y_mult: f32,
    linear_rgb_lut: [f32; 256],
    glyphs: swiftlet_graphics::font::Glyphs,
    should_draw: bool,
}

impl Client {
    fn new(
        server_address: SocketAddr,
        network_channels: TerminalNetworkThreadChannels,
        audio_channels: TerminalAudioThreadChannels,
        glyphs: swiftlet_graphics::font::Glyphs,
    ) -> std::io::Result<Self> {
        let mut linear_rgb_lut = [0.0; 256];
        for (ind, v) in linear_rgb_lut.iter_mut().enumerate() {
            *v = swiftlet_graphics::color::get_linear_rgb_float_from_srgb_byte(ind as u8)
        }

        Ok(Client {
            is_in_vc: false,
            server_name: String::from("Connecting..."),
            server_address,
            connections: Vec::new(),
            my_conn_ind: None,
            network_channels,
            audio_channels,
            already_transfered: false,
            dpi: 92.36,
            num_verticies: 0,
            num_triangles: 0,
            x_max: 0,
            y_max: 0,
            x_mult: 0.0,
            y_mult: 0.0,
            linear_rgb_lut,
            glyphs,
            should_draw: false,
        })
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

    fn new_state(&mut self, state: u8) {
        if state & 4 > 0 {
            if !self.is_in_vc {
                self.is_in_vc = true;

                let _ = self
                    .audio_channels
                    .output_cmd_send
                    .push(TerminalAudioOutCommands::PlayOpus(1));

                let _ = self
                    .audio_channels
                    .input_cmd_send
                    .push(TerminalAudioInCommands::Start);
            }
        } else if self.is_in_vc {
            self.is_in_vc = false;

            let _ = self
                .audio_channels
                .input_cmd_send
                .push(TerminalAudioInCommands::Stop);

            let _ = self
                .audio_channels
                .output_cmd_send
                .push(TerminalAudioOutCommands::PlayOpus(2));
        }
    }

    fn stop(&mut self) {
        let _ = self
            .network_channels
            .command_send
            .push(NetworkCommand::Stop(42));

        let _ = self
            .audio_channels
            .input_cmd_send
            .push(TerminalAudioInCommands::Quit);
        let _ = self
            .audio_channels
            .input_cmd_send
            .push(TerminalAudioInCommands::Quit);
    }
}

impl swiftlet_graphics::VulkanTriglyphCallbacks for Client {
    fn draw(&mut self, input_data: &mut TriglyphInputData, width: u32, height: u32) -> (u32, u32) {
        self.reset_draw_stats(width, height);
        self.draw_rectangle(
            TriangleVertex::new(-1.0, -1.0),
            TriangleVertex::new(1.0, 1.0),
            input_data,
            0xEEEEEE,
            1.0,
        );

        let server_name_pt_size = 18;
        let server_name_line_info = self
            .glyphs
            .get_font_line_info(0, server_name_pt_size, self.dpi)
            .unwrap();
        let mut pos = self.get_vertex_for_pixel(4, 4);
        pos.1 += server_name_line_info.0 * self.y_mult;

        let mut line = self.server_name.clone();
        line.push_str("  @  ");
        line.push_str(&self.server_address.to_string());
        let mut pt_size = (server_name_pt_size, 2);
        self.draw_glyph_line(pos, &line, pt_size, input_data, 0, 1.0);
        pos.0 += 20.0 * self.x_mult;
        pos.1 += server_name_line_info.1 * self.y_mult;

        let connection_name_pt_size = 16;
        let connection_name_line_info = self
            .glyphs
            .get_font_line_info(0, connection_name_pt_size, self.dpi)
            .unwrap();
        pt_size.0 = connection_name_pt_size;
        if let Some(my_ind) = self.my_conn_ind {
            line.clear();
            line.push_str(&self.connections[my_ind].name);
            pos.1 += connection_name_line_info.0 * self.y_mult;
            self.draw_glyph_line(pos, &line, pt_size, input_data, 0, 1.0);
            pos.1 += connection_name_line_info.1 * self.y_mult;

            // line.clear();
            // let state_check = self.connections[my_ind].state;
            // if (state_check & 0x4) > 0 {
            //     line.push_str("In Voice Chat!  ");
            // }
            // if (state_check & 0x8) > 0 {
            //     line.push_str("Voice Looped Back  ");
            // }
            // if (state_check & 0x2) > 0 {
            //     line.push_str("Listening to Server Song  ");
            // }
            // if (state_check & 0x1) > 0 {
            //     line.push_str("Uploading Song to Server");
            // }
            // pos.0 += 20.0 * self.x_mult;
            // pos.1 += 30.0 * self.y_mult;
            // self.draw_glyph_line(pos, &line, pt_size, input_data, 0, 1.0);
            // pos.0 -= 20.0 * self.x_mult;
            // pos.1 += 30.0 * self.y_mult;

            // Render Connections and their States

            for conn_ind in 0..self.connections.len() {
                if conn_ind != my_ind {
                    line.clear();
                    line.push_str(&self.connections[conn_ind].name);
                    pos.1 += connection_name_line_info.0 * self.y_mult;
                    self.draw_glyph_line(pos, &line, pt_size, input_data, 0, 1.0);
                    pos.1 += connection_name_line_info.1 * self.y_mult;

                    // line.clear();
                    // let state_check = self.connections[conn_ind].state;
                    // if (state_check & 0x4) > 0 {
                    //     line.push_str("In Voice Chat!  ");
                    // }
                    // if (state_check & 0x8) > 0 {
                    //     line.push_str("Voice Looped Back  ");
                    // }
                    // if (state_check & 0x2) > 0 {
                    //     line.push_str("Listening to Server Song  ");
                    // }
                    // if (state_check & 0x1) > 0 {
                    //     line.push_str("Uploading Song to Server");
                    // }
                    // pos.0 += 20.0 * self.x_mult;
                    // pos.1 += 30.0 * self.y_mult;
                    // self.draw_glyph_line(pos, &line, pt_size, input_data, 0, 1.0);
                    // pos.0 -= 20.0 * self.x_mult;
                    // pos.1 += 30.0 * self.y_mult;
                }
            }
        }

        ((self.num_verticies as u32), (self.num_triangles as u32))
    }

    fn key_pressed(&mut self, key_code: KeyCode) -> bool {
        //println!("Got Key Code: {:?}", key_code);
        match key_code {
            KeyCode::UpArrow => {
                // if self.debug_scroll > 0 {
                //     self.debug_scroll -= 1;
                //     self.should_draw = true;
                // }
            }
            KeyCode::DownArrow => {
                // if self.debug_scroll < (self.debug_lines - 1) {
                //     self.debug_scroll += 1;
                //     self.should_draw = true;
                // }
            }
            KeyCode::Char(c) => {
                let uc = c.to_ascii_uppercase();
                if uc == 'Q' {
                    return true;
                } else if uc == 'M' {
                    let _ = self
                        .audio_channels
                        .output_cmd_send
                        .push(TerminalAudioOutCommands::PlayOpus(3));
                } else if uc == 'V' {
                    if let Some(ind) = self.my_conn_ind {
                        let state_change = self.connections[ind].state ^ 4;
                        let _ = self
                            .network_channels
                            .command_send
                            .push(NetworkCommand::Client(ClientCommand::StateChange(
                                state_change,
                            )));
                    }
                } else if uc == 'L' {
                    if let Some(ind) = self.my_conn_ind {
                        let state_change = self.connections[ind].state ^ 8;
                        let _ = self
                            .network_channels
                            .command_send
                            .push(NetworkCommand::Client(ClientCommand::StateChange(
                                state_change,
                            )));
                    }
                } else if uc == 'T' && !self.already_transfered {
                    if let Ok(bytes) = std::fs::read(std::path::Path::new(TRANSFER_AUDIO)) {
                        if let Some(opus_data) = OpusData::create_from_ogg_file(&bytes, 45) {
                            let _ =
                                self.network_channels
                                    .command_send
                                    .push(NetworkCommand::Client(ClientCommand::MusicTransfer(
                                        opus_data,
                                    )));
                            self.already_transfered = true;
                        }
                    }
                } else if uc == 'U' {
                    let _ = self
                        .network_channels
                        .command_send
                        .push(NetworkCommand::Client(ClientCommand::UploadTest(8)));
                } else if uc == 'S' {
                    if let Some(ind) = self.my_conn_ind {
                        let state_change = self.connections[ind].state ^ 2;
                        let _ = self
                            .network_channels
                            .command_send
                            .push(NetworkCommand::Client(ClientCommand::StateChange(
                                state_change,
                            )));
                    }
                }
            }
            KeyCode::Chars(chars) => {
                for c in &chars.0[..chars.1] {
                    println!("Chars Pressed: {}", c);
                }
            }
            _ => {
                // Handle more cases in future
            }
        }
        false
    }

    fn tick(&mut self) -> bool {
        loop {
            match self.network_channels.state_recv.pop() {
                Err(PopError::Empty) => {
                    break;
                }
                Ok(recv_state_cmd) => {
                    match recv_state_cmd {
                        NetworkStateMessage::StateChange((entry, state)) => {
                            self.connections[entry].state = state;
                            if let Some(ind) = self.my_conn_ind {
                                if entry == ind {
                                    self.new_state(state);
                                }
                            }
                        }
                        NetworkStateMessage::NewConnection((user_name, state)) => {
                            let conn_state = NetworkStateConnection {
                                name: user_name,
                                state,
                            };
                            self.connections.push(conn_state);
                        }
                        NetworkStateMessage::ServerNameChange(server_name) => {
                            self.server_name = server_name;
                        }
                        NetworkStateMessage::ConnectionsRefresh((
                            new_conn_index,
                            connection_state_vec,
                        )) => {
                            self.my_conn_ind = new_conn_index;
                            self.connections = connection_state_vec;
                            if let Some(conn_ind) = self.my_conn_ind {
                                self.new_state(self.connections[conn_ind].state)
                            }
                        }
                    }
                    self.should_draw = true;
                }
            }
        }

        loop {
            match self.network_channels.debug_recv.pop() {
                Err(PopError::Empty) => {
                    break;
                }
                Ok(recv_string) => {
                    print!("{}", recv_string);
                }
            }
        }

        loop {
            match self.audio_channels.output_debug_recv.pop() {
                Err(PopError::Empty) => {
                    break;
                }
                Ok(recv_string) => {
                    print!("{}", recv_string);
                }
            }
        }

        loop {
            match self.audio_channels.input_debug_recv.pop() {
                Err(PopError::Empty) => {
                    break;
                }
                Ok(recv_string) => {
                    print!("{}", recv_string);
                }
            }
        }

        if !self.should_draw {
            false
        } else {
            self.should_draw = false;
            true
        }
    }
}

pub(crate) struct ClientRunner {
    client: Client,
    window: swiftlet_graphics::VulkanTriglyph,
    signaler: swiftlet_graphics::OsEventSignaler,
}

impl ClientRunner {
    pub(crate) fn new(
        server_address: SocketAddr,
        network_channels: TerminalNetworkThreadChannels,
        mut audio_channels: TerminalAudioThreadChannels,
    ) -> std::io::Result<Self> {
        // Load in Audio Files
        for (ind, f) in AUDIO_FILES.iter().enumerate() {
            if let Ok(bytes) = std::fs::read(std::path::Path::new(f)) {
                if let Some(opus_data) = OpusData::create_from_ogg_file(&bytes, (ind as u64) + 1) {
                    let _ = audio_channels
                        .output_cmd_send
                        .push(TerminalAudioOutCommands::LoadOpus(opus_data));
                }
            }
        }

        let mut glyphs =
            swiftlet_graphics::font::Glyphs::new_from_font_file(FONT_PATH, 0, 2, "en").unwrap();
        glyphs.add_glyph_outline_data(0, ' ', '~').unwrap();
        let (window, signaler) = match swiftlet_graphics::VulkanTriglyph::new(
            1280,
            720,
            104 * 8,
            glyphs.get_glyph_outline_data(),
            true,
        ) {
            Ok((w, s)) => (w, s),
            Err(e) => {
                println!("Window Creation Error: {:?}", e);
                return Err(std::io::Error::from(std::io::ErrorKind::Other));
            }
        };
        // Get (initial) window dpi here in future to pass to initial client startup

        Ok(ClientRunner {
            client: Client::new(server_address, network_channels, audio_channels, glyphs)?,
            window,
            signaler,
        })
    }

    pub(crate) fn run(&mut self) {
        // Start Client Window Thread Ownership
        self.window.run(&mut self.client).unwrap();

        self.client.stop();
    }
}
