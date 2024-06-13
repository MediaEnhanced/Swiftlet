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
const FONT_PATH: &str = "font/roboto/Roboto-Medium.ttf";
const ICON_PATH: &str = "font/symbols/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf"; // Location of the Icon Font
const ICON_CODEPOINTS_PATH: &str =
    "font/symbols/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].codepoints"; // Location of the Icon Font Codepoints

pub(crate) mod audio;
use swiftlet_audio::opus::OpusData;

use crate::communication::{
    ClientCommand, NetworkCommand, NetworkStateConnection, NetworkStateMessage, PopError,
    TerminalAudioInCommands, TerminalAudioOutCommands, TerminalAudioThreadChannels,
    TerminalNetworkThreadChannels,
};

use swiftlet_graphics::color::LinearRGB;
use swiftlet_graphics::font::{Glyphs, TextBuffer};
use swiftlet_graphics::vulkan::{
    PrimitiveColor, PrimitivePosition, PrimitiveRectangleModifier, Primitives2d,
};
use swiftlet_graphics::{DrawJustification, KeyCode};
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
    linear_rgb: LinearRGB,
    text_buffer_opt: Option<TextBuffer>,
    should_draw: bool,
}

impl Client {
    fn new(
        server_address: SocketAddr,
        network_channels: TerminalNetworkThreadChannels,
        audio_channels: TerminalAudioThreadChannels,
        window_dpi: u32,
    ) -> std::io::Result<Self> {
        Ok(Client {
            is_in_vc: false,
            server_name: String::from("Connecting..."),
            server_address,
            connections: Vec::new(),
            my_conn_ind: None,
            network_channels,
            audio_channels,
            already_transfered: false,
            dpi: window_dpi as f32,
            linear_rgb: LinearRGB::new(),
            text_buffer_opt: Some(TextBuffer::default()),
            should_draw: false,
        })
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

impl swiftlet_graphics::Vulkan2dWindowCallbacks for Client {
    fn draw(&mut self, primitives: &mut Primitives2d, glyphs: &Glyphs) {
        // Background Color
        // Could be set as a less dynamic clear color in the future
        let background_color = PrimitiveColor::new_from_linear_rgb_and_alpha(
            self.linear_rgb.get_linear_rgb_from_srgb(0xEEEEEE),
            1.0,
        );
        //let rect_p0 = PrimitivePosition::default();
        let rect_p1 = primitives.get_position_from_percentage(100.0, 100.0);
        primitives.add_rectangle(
            (0.0, 0.0),
            (rect_p1.x, rect_p1.y),
            &background_color,
            PrimitiveRectangleModifier::None,
        );

        let split_pos = primitives.get_position_from_percentage(32.0, 50.0);

        let solid_black_color = PrimitiveColor::new_from_linear_rgb_and_alpha(
            self.linear_rgb.get_linear_rgb_from_srgb(0),
            1.0,
        );
        let solid_white_color = PrimitiveColor::new_from_linear_rgb_and_alpha(
            self.linear_rgb.get_linear_rgb_from_srgb(0xFFFFFF),
            1.0,
        );
        let light_grey_color = PrimitiveColor::new_from_linear_rgb_and_alpha(
            self.linear_rgb.get_linear_rgb_from_srgb(0xC0C0C0),
            1.0,
        );
        let mut text_buffer = match self.text_buffer_opt.take() {
            Some(tb) => tb,
            None => TextBuffer::default(),
        };

        let face_shaper = glyphs.get_font_face_shaper(0).unwrap();
        let server_name_pt_size = 18;
        let server_name_metrics =
            face_shaper.get_ascender_descender_gap(server_name_pt_size, self.dpi);
        let corner_offset = primitives.get_position_from_inch(self.dpi, 0.0625, 0.0625);

        let server_name_baseline = PrimitivePosition {
            x: corner_offset.x,
            y: corner_offset.y + server_name_metrics.0,
        };
        text_buffer.add_text(&self.server_name);
        let glyph_bri =
            face_shaper.create_glyph_buffer_render_info(server_name_pt_size, self.dpi, text_buffer);
        glyph_bri.draw_glyphs(
            primitives,
            &server_name_baseline,
            &solid_black_color,
            2,
            DrawJustification::Left,
        );
        text_buffer = glyph_bri.get_text_buffer();

        let address_pos = PrimitivePosition {
            x: split_pos.x - corner_offset.x,
            y: server_name_baseline.y,
        };
        text_buffer.add_text("@");
        text_buffer.add_text(&self.server_address.to_string());
        let glyph_bri =
            face_shaper.create_glyph_buffer_render_info(server_name_pt_size, self.dpi, text_buffer);
        glyph_bri.draw_glyphs(
            primitives,
            &address_pos,
            &solid_black_color,
            2,
            DrawJustification::Right,
        );
        text_buffer = glyph_bri.get_text_buffer();

        if let Some(my_ind) = self.my_conn_ind {
            let circle_color = PrimitiveColor::new_from_linear_rgb_and_alpha(
                self.linear_rgb.get_linear_rgb_from_srgb(0x228B22),
                1.0,
            );
            let client_name_pt_size = 16;
            let client_name_metrics =
                face_shaper.get_ascender_descender_gap(client_name_pt_size, self.dpi);
            //println!("Font Metrics: {:?}", client_name_metrics);
            let circle_size = (client_name_metrics.0 + client_name_metrics.1) * 1.5;
            //println!("Circle Size: {}", circle_size);
            let mut circle_p0 = PrimitivePosition {
                x: server_name_baseline.x,
                y: server_name_baseline.y + server_name_metrics.1 + server_name_metrics.2,
            };

            primitives.add_rectangle(
                (circle_p0.x, circle_p0.y),
                (circle_size, circle_size),
                &circle_color,
                PrimitiveRectangleModifier::Ellipse,
            );
            let client_advance = circle_size + client_name_metrics.2;

            let y_offset = (circle_size * (0.25 / 1.5)) + client_name_metrics.0;
            let client_baseline_p0 = PrimitivePosition {
                x: circle_p0.x,
                y: circle_p0.y + y_offset,
            };
            let mut client_name_p0 = PrimitivePosition {
                x: client_baseline_p0.x + client_advance,
                y: client_baseline_p0.y,
            };

            text_buffer.add_text(&self.connections[my_ind].name);
            let glyph_bri = face_shaper.create_glyph_buffer_render_info(
                client_name_pt_size,
                self.dpi,
                text_buffer,
            );
            glyph_bri.draw_glyphs(
                primitives,
                &client_name_p0,
                &solid_black_color,
                2,
                DrawJustification::Left,
            );
            text_buffer = glyph_bri.get_text_buffer();

            let mut client_letter_p0 = PrimitivePosition {
                x: client_baseline_p0.x + circle_size * 0.5,
                y: client_baseline_p0.y,
            };
            //client_letter_p0 = primitives.get_position_from_percentage(50.0, 50.0);
            let mut client_letter = self.connections[my_ind].name.chars().next().unwrap_or(' ');
            client_letter = client_letter.to_ascii_uppercase();
            face_shaper.draw_glyph(
                primitives,
                &client_letter_p0,
                &solid_white_color,
                2,
                client_name_pt_size,
                self.dpi,
                client_letter,
                DrawJustification::Center,
            );

            let state_check = self.connections[my_ind].state;

            let icon_height = client_name_metrics.0;
            let mut note_icon_p0 = PrimitivePosition {
                x: split_pos.x - corner_offset.x,
                y: client_baseline_p0.y,
            };
            let icon_color = if (state_check & 0x2) > 0 {
                &solid_black_color
            } else {
                &light_grey_color
            };
            glyphs.draw_icon(
                primitives,
                &note_icon_p0,
                icon_color,
                2,
                3,
                icon_height,
                DrawJustification::Right,
            );
            let icon_dims = glyphs.get_icon_dims(3);

            let mut upload_icon_p0 = PrimitivePosition {
                x: note_icon_p0.x - corner_offset.x - icon_dims.0 * (icon_height / icon_dims.1),
                y: note_icon_p0.y,
            };
            let icon_color = if (state_check & 0x1) > 0 {
                &solid_black_color
            } else {
                &light_grey_color
            };
            glyphs.draw_icon(
                primitives,
                &upload_icon_p0,
                icon_color,
                2,
                2,
                icon_height,
                DrawJustification::Right,
            );
            let icon_dims = glyphs.get_icon_dims(2);

            let mut loop_icon_p0 = PrimitivePosition {
                x: upload_icon_p0.x - corner_offset.x - icon_dims.0 * (icon_height / icon_dims.1),
                y: upload_icon_p0.y,
            };
            let icon_color = if (state_check & 0x8) > 0 {
                &solid_black_color
            } else {
                &light_grey_color
            };
            glyphs.draw_icon(
                primitives,
                &loop_icon_p0,
                icon_color,
                2,
                1,
                icon_height,
                DrawJustification::Right,
            );
            let icon_dims = glyphs.get_icon_dims(1);

            let mut mic_icon_p0 = PrimitivePosition {
                x: loop_icon_p0.x - corner_offset.x - icon_dims.0 * (icon_height / icon_dims.1),
                y: loop_icon_p0.y,
            };
            let icon_color = if (state_check & 0x4) > 0 {
                &solid_black_color
            } else {
                &light_grey_color
            };
            glyphs.draw_icon(
                primitives,
                &mic_icon_p0,
                icon_color,
                2,
                0,
                icon_height,
                DrawJustification::Right,
            );

            // Render Connections and their States
            for conn_ind in 0..self.connections.len() {
                if conn_ind != my_ind {
                    let circle_color = PrimitiveColor::new_from_linear_rgb_and_alpha(
                        self.linear_rgb.get_linear_rgb_from_srgb(0xADD8E6),
                        1.0,
                    );
                    circle_p0.y += client_advance;
                    primitives.add_rectangle(
                        (circle_p0.x, circle_p0.y),
                        (circle_size, circle_size),
                        &circle_color,
                        PrimitiveRectangleModifier::Ellipse,
                    );

                    client_name_p0.y += client_advance;
                    text_buffer.add_text(&self.connections[conn_ind].name);
                    let glyph_bri = face_shaper.create_glyph_buffer_render_info(
                        client_name_pt_size,
                        self.dpi,
                        text_buffer,
                    );
                    glyph_bri.draw_glyphs(
                        primitives,
                        &client_name_p0,
                        &solid_black_color,
                        2,
                        DrawJustification::Left,
                    );
                    text_buffer = glyph_bri.get_text_buffer();

                    client_letter_p0.y += client_advance;
                    let mut client_letter = self.connections[conn_ind]
                        .name
                        .chars()
                        .next()
                        .unwrap_or(' ');
                    client_letter = client_letter.to_ascii_uppercase();
                    face_shaper.draw_glyph(
                        primitives,
                        &client_letter_p0,
                        &solid_white_color,
                        2,
                        client_name_pt_size,
                        self.dpi,
                        client_letter,
                        DrawJustification::Center,
                    );

                    let state_check = self.connections[conn_ind].state;
                    note_icon_p0.y += client_advance;
                    let icon_color = if (state_check & 0x2) > 0 {
                        &solid_black_color
                    } else {
                        &light_grey_color
                    };
                    glyphs.draw_icon(
                        primitives,
                        &note_icon_p0,
                        icon_color,
                        2,
                        3,
                        icon_height,
                        DrawJustification::Right,
                    );

                    upload_icon_p0.y += client_advance;
                    let icon_color = if (state_check & 0x1) > 0 {
                        &solid_black_color
                    } else {
                        &light_grey_color
                    };
                    glyphs.draw_icon(
                        primitives,
                        &upload_icon_p0,
                        icon_color,
                        2,
                        2,
                        icon_height,
                        DrawJustification::Right,
                    );
                    loop_icon_p0.y += client_advance;
                    let icon_color = if (state_check & 0x8) > 0 {
                        &solid_black_color
                    } else {
                        &light_grey_color
                    };
                    glyphs.draw_icon(
                        primitives,
                        &loop_icon_p0,
                        icon_color,
                        2,
                        1,
                        icon_height,
                        DrawJustification::Right,
                    );
                    mic_icon_p0.y += client_advance;
                    let icon_color = if (state_check & 0x4) > 0 {
                        &solid_black_color
                    } else {
                        &light_grey_color
                    };
                    glyphs.draw_icon(
                        primitives,
                        &mic_icon_p0,
                        icon_color,
                        2,
                        0,
                        icon_height,
                        DrawJustification::Right,
                    );
                }
            }
        }

        let turquoise_color = PrimitiveColor::new_from_linear_rgb_and_alpha(
            self.linear_rgb.get_linear_rgb_from_srgb(0x40E0D0),
            1.0,
        );
        let chat_box_top_left = (split_pos.x + corner_offset.x, corner_offset.y);
        let chat_dims = primitives.get_position_from_percentage(80.0, 80.0);
        let chat_box_dims = (
            rect_p1.x - corner_offset.x - chat_box_top_left.0,
            chat_dims.y,
        );
        primitives.add_rectangle(
            chat_box_top_left,
            chat_box_dims,
            &turquoise_color,
            PrimitiveRectangleModifier::None,
        );

        let center_pos = primitives.get_position_from_percentage(50.0, 50.0);
        let help_line_p0 = PrimitivePosition {
            x: center_pos.x,
            y: rect_p1.y - corner_offset.y - server_name_metrics.1,
        };

        text_buffer.add_text(
            "Enter Voice Chat: V; Loopback Voice: L; Upload Music: T; Shared Music Listen: S",
        );
        let glyph_bri =
            face_shaper.create_glyph_buffer_render_info(server_name_pt_size, self.dpi, text_buffer);
        glyph_bri.draw_glyphs(
            primitives,
            &help_line_p0,
            &solid_black_color,
            2,
            DrawJustification::Center,
        );
        text_buffer = glyph_bri.get_text_buffer();

        self.text_buffer_opt = Some(text_buffer);
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

    fn tick(&mut self, glyphs: &mut Glyphs) -> bool {
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
    window: swiftlet_graphics::Vulkan2dWindow,
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

        let mut icons = swiftlet_graphics::font::FontIcons::new_from_files(
            ICON_PATH,
            ICON_CODEPOINTS_PATH,
            ' ',
            16,
            2,
        )
        .unwrap();
        let icon_names = ["mic", "loop", "upload", "play_music", "download", "search"];
        icons.add_icon_outline_data(&icon_names).unwrap();

        let mut glyphs = swiftlet_graphics::font::Glyphs::new_from_font_icons(icons).unwrap();
        glyphs.add_new_font(FONT_PATH, 0).unwrap();
        glyphs.add_glyph_outline_data(0, ' ', '~').unwrap();

        let (window, window_dpi) = match swiftlet_graphics::Vulkan2dWindow::new(
            1280,
            720,
            1 << 25,
            glyphs,
            swiftlet_graphics::Vulkan2dWindowMode::Normal,
        ) {
            Ok(r) => r,
            Err(e) => {
                println!("Window Creation Error: {:?}", e);
                return Err(std::io::Error::from(std::io::ErrorKind::Other));
            }
        };
        // Get (initial) window dpi here in future to pass to initial client startup

        Ok(ClientRunner {
            client: Client::new(server_address, network_channels, audio_channels, window_dpi)?,
            window,
        })
    }

    pub(crate) fn run(&mut self) {
        // Start Client Window Thread Ownership
        self.window
            .run(&mut self.client, std::time::Duration::from_millis(20))
            .unwrap();

        self.client.stop();
    }
}
