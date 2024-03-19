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

pub(crate) mod audio;
use swiftlet_audio::opus::OpusData;

use crate::communication::{
    ClientCommand, NetworkCommand, NetworkStateConnection, NetworkStateMessage, PopError,
    TerminalAudioInCommands, TerminalAudioOutCommands, TerminalAudioThreadChannels,
    TerminalNetworkThreadChannels,
};

use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::ExecutableCommand; // Needed to use .execute on Stdout for crossterm setup
use ratatui::{prelude::*, widgets::*};
use swiftlet_quic::endpoint::SocketAddr;

struct Client {
    server_name: String,
    server_address: SocketAddr,
    connections: Vec<NetworkStateConnection>,
    my_conn_ind: Option<usize>,
    debug_title: String,
    debug_string: String,
    debug_lines: u16,
    debug_scroll: u16,
    main_layout: ratatui::layout::Layout,
}

impl Client {
    fn new(server_address: SocketAddr) -> Self {
        let constraints = vec![
            Constraint::Length(6),
            Constraint::Fill(1),
            Constraint::Length(4),
            Constraint::Fill(1),
        ];

        Client {
            server_name: String::from("Connecting..."),
            server_address,
            connections: Vec::new(),
            my_conn_ind: None,
            debug_title: String::from("Debug"),
            debug_string: String::from("Client Console Started!\n"),
            debug_lines: 1,
            debug_scroll: 0,
            main_layout: Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints),
        }
    }

    fn draw_ui(&self, frame: &mut ratatui::Frame) {
        let main_areas = self.main_layout.split(frame.size());

        let server_line = Line::default().spans([
            Span::from(self.server_name.clone()),
            Span::from("  @  "),
            Span::from(self.server_address.to_string()),
        ]);

        if let Some(my_ind) = self.my_conn_ind {
            let username_line = Line::default().spans([
                Span::from("  "),
                Span::from(self.connections[my_ind].name.clone()),
                //Span::from(self.server_address.to_string()), Volume in future
            ]);

            let ivc_span = if (self.connections[my_ind].state & 0x4) == 0 {
                Span::from("< >")
            } else {
                Span::from("<X>")
            };
            let vlb_span = if (self.connections[my_ind].state & 0x8) == 0 {
                Span::from("< >")
            } else {
                Span::from("<X>")
            };
            let lss_span = if (self.connections[my_ind].state & 0x2) == 0 {
                Span::from("< >")
            } else {
                Span::from("<X>")
            };
            let uss_span = if (self.connections[my_ind].state & 0x1) == 0 {
                Span::from("< >")
            } else {
                Span::from("<X>")
            };

            let voice_chat_line = Line::default().spans([
                Span::from("    "),
                ivc_span,
                Span::from(" InVoiceChat  "),
                vlb_span,
                Span::from(" VoiceLoopBack"),
            ]);

            let server_listen_line = Line::default().spans([
                Span::from("    "),
                lss_span,
                Span::from(" ListenToServerSong"),
            ]);

            let upload_song_line = Line::default().spans([
                Span::from("    "),
                uss_span,
                Span::from(" UploadSongToServer"),
            ]);

            let blank_line = Line::default();

            let header_list = List::new([
                server_line,
                username_line,
                voice_chat_line,
                server_listen_line,
                upload_song_line,
                blank_line,
            ]);

            frame.render_widget(header_list, main_areas[0]);

            // Render Connections and their States
            let mut rows = Vec::new();
            for (conn_ind, conn) in self.connections.iter().enumerate() {
                if conn_ind != my_ind {
                    let mut row = Vec::new();
                    let mut username_string = String::from("    ");
                    username_string.push_str(&conn.name);
                    let username_cell = Cell::from(username_string);
                    row.push(username_cell);

                    let mut state_test = 1;
                    for i in 1..8 {
                        if conn.state & state_test > 0 {
                            row.push(Cell::from("<X>"));
                        } else {
                            row.push(Cell::from("< >"));
                        }
                        state_test <<= 1;
                    }

                    rows.push(Row::new(row));
                }
            }

            let header_row = [
                String::from("  Peers"),
                String::from("LSS"),
                String::from("USS"),
                String::from("IVC"),
                String::from("VLB"),
            ];

            let widths = [
                Constraint::Length(38),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(4),
            ];

            let table = Table::new(rows, widths)
                .header(Row::new(header_row))
                .column_spacing(0);

            frame.render_widget(table, main_areas[1]);
        } else {
            //let blank_lines = [Line::default(); 5];
            let header_list = List::new([
                server_line,
                Line::default(),
                Line::default(),
                Line::default(),
                Line::default(),
                Line::default(),
            ]);
            frame.render_widget(header_list, main_areas[0]);

            frame.render_widget(Clear, main_areas[1]);
        }

        frame.render_widget(Clear, main_areas[2]);

        // Render Debug Text
        frame.render_widget(
            Paragraph::new(self.debug_string.as_str())
                .scroll((self.debug_scroll, 0))
                .block(
                    Block::new()
                        .borders(Borders::ALL)
                        .title(self.debug_title.as_str()),
                ),
            main_areas[3],
        );

        // Add scrolling to debug text
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state =
            ScrollbarState::new(self.debug_lines as usize).position(self.debug_scroll as usize);

        frame.render_stateful_widget(
            scrollbar,
            main_areas[3].inner(&Margin {
                vertical: 1,
                horizontal: 0,
            }), // using a inner vertical margin of 1 unit makes the scrollbar inside the block
            &mut scrollbar_state,
        );
    }
}

struct Terminal {
    interface: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
}

impl Terminal {
    fn new() -> std::io::Result<Self> {
        let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
        let interface = ratatui::terminal::Terminal::new(backend)?;
        Ok(Terminal { interface })
    }

    fn start(&self) -> std::io::Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        std::io::stdout().execute(crossterm::terminal::EnterAlternateScreen)?;
        Ok(())
    }

    fn stop(&self) -> std::io::Result<()> {
        std::io::stdout().execute(crossterm::terminal::LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    }

    fn draw(&mut self, f: impl Fn(&mut ratatui::Frame<'_>)) -> std::io::Result<()> {
        self.interface.draw(f)?;
        Ok(())
    }
}

pub(crate) struct ClientTerminal {
    client: Client,
    terminal: Terminal,
    network_channels: TerminalNetworkThreadChannels,
    audio_channels: TerminalAudioThreadChannels,
    is_in_vc: bool,
}

impl ClientTerminal {
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

        Ok(ClientTerminal {
            client: Client::new(server_address),
            terminal: Terminal::new()?,
            network_channels,
            audio_channels,
            is_in_vc: false,
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
                .push(TerminalAudioInCommands::Pause);

            let _ = self
                .audio_channels
                .output_cmd_send
                .push(TerminalAudioOutCommands::PlayOpus(2));
        }
    }

    pub(crate) fn run_terminal(&mut self) -> std::io::Result<()> {
        // Start Console Here:
        self.terminal.start()?;

        let mut already_transfered = false;
        let mut should_draw = true;

        loop {
            if crossterm::event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = crossterm::event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Up => {
                                if self.client.debug_scroll > 0 {
                                    self.client.debug_scroll -= 1;
                                    should_draw = true;
                                }
                            }
                            KeyCode::Down => {
                                if self.client.debug_scroll < (self.client.debug_lines - 1) {
                                    self.client.debug_scroll += 1;
                                    should_draw = true;
                                }
                            }
                            KeyCode::Char(c) => {
                                let uc = c.to_ascii_uppercase();
                                if uc == 'Q' {
                                    break;
                                } else if uc == 'M' {
                                    let _ = self
                                        .audio_channels
                                        .output_cmd_send
                                        .push(TerminalAudioOutCommands::PlayOpus(3));
                                } else if uc == 'V' {
                                    if let Some(ind) = self.client.my_conn_ind {
                                        let state_change = self.client.connections[ind].state ^ 4;
                                        let _ = self.network_channels.command_send.push(
                                            NetworkCommand::Client(ClientCommand::StateChange(
                                                state_change,
                                            )),
                                        );
                                    }
                                } else if uc == 'L' {
                                    if let Some(ind) = self.client.my_conn_ind {
                                        let state_change = self.client.connections[ind].state ^ 8;
                                        let _ = self.network_channels.command_send.push(
                                            NetworkCommand::Client(ClientCommand::StateChange(
                                                state_change,
                                            )),
                                        );
                                    }
                                } else if uc == 'T' && !already_transfered {
                                    if let Ok(bytes) =
                                        std::fs::read(std::path::Path::new(TRANSFER_AUDIO))
                                    {
                                        if let Some(opus_data) =
                                            OpusData::create_from_ogg_file(&bytes, 45)
                                        {
                                            let _ = self.network_channels.command_send.push(
                                                NetworkCommand::Client(
                                                    ClientCommand::MusicTransfer(opus_data),
                                                ),
                                            );
                                            already_transfered = true;
                                        }
                                    }
                                } else if uc == 'S' {
                                    if let Some(ind) = self.client.my_conn_ind {
                                        let state_change = self.client.connections[ind].state ^ 2;
                                        let _ = self.network_channels.command_send.push(
                                            NetworkCommand::Client(ClientCommand::StateChange(
                                                state_change,
                                            )),
                                        );
                                    }
                                }
                            }
                            _ => {
                                // Handle more cases in future
                            }
                        }
                    }
                }
            }

            loop {
                match self.network_channels.state_recv.pop() {
                    Err(PopError::Empty) => {
                        break;
                    }
                    Ok(recv_state_cmd) => {
                        match recv_state_cmd {
                            NetworkStateMessage::StateChange((entry, state)) => {
                                self.client.connections[entry].state = state;
                                if let Some(ind) = self.client.my_conn_ind {
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
                                self.client.connections.push(conn_state);
                            }
                            NetworkStateMessage::ServerNameChange(server_name) => {
                                self.client.server_name = server_name;
                            }
                            NetworkStateMessage::ConnectionsRefresh((
                                new_conn_index,
                                connection_state_vec,
                            )) => {
                                self.client.my_conn_ind = new_conn_index;
                                self.client.connections = connection_state_vec;
                                if let Some(conn_ind) = self.client.my_conn_ind {
                                    self.new_state(self.client.connections[conn_ind].state)
                                }
                            }
                        }
                        should_draw = true;
                    }
                }
            }

            loop {
                match self.network_channels.debug_recv.pop() {
                    Err(PopError::Empty) => {
                        break;
                    }
                    Ok(recv_string) => {
                        self.client.debug_string.push_str(&recv_string);
                        self.client.debug_lines += 1;
                        should_draw = true;
                    }
                }
            }

            loop {
                match self.audio_channels.debug_recv.pop() {
                    Err(PopError::Empty) => {
                        break;
                    }
                    Ok(recv_string) => {
                        self.client.debug_string.push_str(&recv_string);
                        self.client.debug_lines += 1;
                        should_draw = true;
                    }
                }
            }

            if should_draw {
                self.terminal.draw(|frame| self.client.draw_ui(frame))?;
                should_draw = false;
            }
        }

        let _ = self
            .network_channels
            .command_send
            .push(NetworkCommand::Stop(42));

        // Cleanup Console Here:
        self.terminal.stop()
    }
}
