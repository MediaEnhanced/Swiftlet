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

#![allow(unused_variables)]
#![allow(dead_code)]

const SERVERNAME_DEFAULT: &str = "Server";
const USERNAME_DEFAULT: &str = "Client";
const PORT_DEFAULT: u16 = 9001;

const DEBUG_STR: &str = "Debug";
const CONNECTING_STR: &str = "Connecting...";

const AUDIO_FILES: [&str; 3] = [
    "audio/EnterVoice.opus",
    "audio/ExitVoice.opus",
    "audio/song.opus",
];
const TRANSFER_AUDIO: &str = "audio/transfer.opus";

mod communication;
use communication::{
    ClientCommand, ConsoleAudioCommands, ConsoleAudioOutputChannels, ConsoleThreadChannels,
    NetworkCommand, NetworkStateConnection, NetworkStateMessage, TryRecvError,
};

mod network;
use cpal::traits::StreamTrait;
use swiftlet_quic::SocketAddr;

mod audio;

use clap::{ArgAction, Parser};
use crossterm::ExecutableCommand; // Needed to use .execute on Stdout for crossterm setup
use ratatui::{prelude::*, widgets::*};
use std::thread;

use crate::audio::OpusData;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Address to connect to.
    /// Must be in `127.0.0.1:5000` or `[::1]:5000` format
    #[arg(short, long)]
    address: Option<SocketAddr>,

    /// Port to serve on
    #[arg(short, long)]
    port: Option<u16>,

    /// Username to log in with
    #[arg(short, long)]
    username: Option<String>,

    /// Name of the server
    #[arg(short, long)]
    sname: Option<String>,

    /// Option to serve with IPv6
    #[arg(long, action=ArgAction::SetTrue)]
    ipv6: Option<bool>,
}

fn main() -> std::io::Result<()> {
    println!("Networking Audio Program Started");

    // Uncomment if this would be useful (only for debug code?)
    //std::env::set_var("RUST_BACKTRACE", "1");

    // Argument Parsing
    let args = Args::parse();

    // Initialize inter-thread channels common to both clients and servers (a future "headless" server won't need these)
    let (network_channels, console_channels) = communication::create_networking_console_channels();

    // Check if the program started as a Client or a Server
    match args.address {
        // This is a client because we have an address to connect to
        Some(server_address) => {
            let (audio_out_channels, network_audio_out_channels, console_audio_out_channels) =
                communication::create_audio_output_channels();

            // Load in Audio Files
            for (ind, f) in AUDIO_FILES.iter().enumerate() {
                if let Ok(bytes) = std::fs::read(std::path::Path::new(f)) {
                    if let Some(opus_data) =
                        OpusData::convert_ogg_opus_file(&bytes, (ind as u64) + 1)
                    {
                        let _ = console_audio_out_channels
                            .command_send
                            .send(ConsoleAudioCommands::LoadOpus(opus_data));
                    }
                }
            }

            // Start Network Thread
            let user_name = match args.username {
                Some(un) => un,
                None => USERNAME_DEFAULT.to_string(),
            };
            let network_thread_handler = thread::spawn(move || {
                network::client_thread(
                    server_address,
                    user_name,
                    network_channels,
                    network_audio_out_channels,
                )
            });

            // Start Audio Output
            let audio_out_stream = audio::start_audio_output(audio_out_channels);

            // Start Console
            let _ = run_console_client(
                console_channels,
                console_audio_out_channels,
                audio_out_stream,
            );

            // Wait for Network Thread to Finish
            network_thread_handler.join().unwrap();
        }
        None => {
            // No server address was provided, so this is a server
            let port = match args.port {
                Some(p) => p,
                None => PORT_DEFAULT,
            };

            // Start Network Thread
            let server_name = args.sname.unwrap_or(String::from(SERVERNAME_DEFAULT));
            let server_name_console = server_name.clone();
            let network_thread_handler = thread::spawn(move || {
                network::server_thread(args.ipv6, port, server_name.clone(), network_channels)
            });

            // Start Console
            let _ = run_console_server(server_name_console, console_channels);

            // Wait for Network Thread to Finish
            network_thread_handler.join().unwrap();
        }
    }

    println!("Networking Audio Program Quitting");
    Ok(())
}

struct ConsoleStateCommon {
    title_string: String,
    debug_string: String,
    debug_lines: u16,
    debug_scroll: u16,
    connections: Vec<NetworkStateConnection>,
}

fn run_console_server(servername: String, channels: ConsoleThreadChannels) -> std::io::Result<()> {
    // Start Console Here:
    crossterm::terminal::enable_raw_mode().unwrap();
    std::io::stdout().execute(crossterm::terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    let mut terminal = ratatui::terminal::Terminal::new(backend)?;

    let mut state_common = ConsoleStateCommon {
        title_string: servername,
        debug_string: String::from("Server Console Started!\n"),
        debug_lines: 1,
        debug_scroll: 0,
        connections: Vec::new(),
    };

    let mut should_draw = true;

    loop {
        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            // Bool?
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                // Bool?
                if key.kind == crossterm::event::KeyEventKind::Press {
                    if key.code == crossterm::event::KeyCode::Char('q') {
                        break;
                    } else if key.code == crossterm::event::KeyCode::Up
                        && state_common.debug_scroll > 0
                    {
                        state_common.debug_scroll -= 1;
                        should_draw = true;
                    } else if key.code == crossterm::event::KeyCode::Down
                        && state_common.debug_scroll < (state_common.debug_lines - 1)
                    {
                        state_common.debug_scroll += 1;
                        should_draw = true;
                    }
                }
            }
        }

        loop {
            match channels.network_state_recv.try_recv() {
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {
                            break;
                        }
                        TryRecvError::Disconnected => {
                            //state_common.debug_string.push_str("Network Debug Recv Disconnected!!!\n");
                            //state_common.debug_lines += 1;
                            break;
                        }
                    }
                }
                Ok(recv_state_cmd) => {
                    match recv_state_cmd {
                        NetworkStateMessage::ServerNameChange(server_name) => {
                            state_common.title_string = server_name;
                        }
                        NetworkStateMessage::ConnectionsRefresh((_, connection_state_vec)) => {
                            state_common.connections = connection_state_vec;
                        }
                        NetworkStateMessage::NewConnection((user_name, state)) => {
                            let conn_state = NetworkStateConnection {
                                name: user_name,
                                state,
                            };
                            state_common.connections.push(conn_state);
                        }
                        NetworkStateMessage::StateChange((entry, state)) => {
                            state_common.connections[entry].state = state;
                        }
                    }
                    should_draw = true;
                }
            }
        }

        loop {
            match channels.network_debug_recv.try_recv() {
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {
                            break;
                        }
                        TryRecvError::Disconnected => {
                            //state_common.debug_string.push_str("Network Debug Recv Disconnected!!!\n");
                            //state_common.debug_lines += 1;
                            break;
                        }
                    }
                }
                Ok(recv_string) => {
                    state_common.debug_string.push_str(&recv_string);
                    state_common.debug_lines += 1;
                    should_draw = true;
                }
            }
        }

        if should_draw {
            terminal.draw(|frame| console_ui(frame, &state_common, None))?;
            should_draw = false;
        }
    }

    let _ = channels.command_send.send(NetworkCommand::Stop(42));

    // Cleanup Console Here:
    std::io::stdout().execute(crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}

fn run_console_client(
    channels: ConsoleThreadChannels,
    audio_out_channels: ConsoleAudioOutputChannels,
    audio_out_stream: Option<audio::Stream>,
) -> std::io::Result<()> {
    // Start Console Here:
    crossterm::terminal::enable_raw_mode().unwrap();
    std::io::stdout().execute(crossterm::terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    let mut terminal = ratatui::terminal::Terminal::new(backend)?;

    let mut state_common = ConsoleStateCommon {
        title_string: CONNECTING_STR.to_string(),
        debug_string: String::from("Client Console Started!\n"),
        debug_lines: 1,
        debug_scroll: 0,
        connections: Vec::new(),
    };

    let mut my_conn_index: Option<usize> = None;
    let mut is_in_vc = false;
    let mut already_transfered = false;

    let mut should_draw = true;

    loop {
        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            // Bool?
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                // Bool?
                if key.kind == crossterm::event::KeyEventKind::Press {
                    match key.code {
                        crossterm::event::KeyCode::Up => {
                            if state_common.debug_scroll > 0 {
                                state_common.debug_scroll -= 1;
                                should_draw = true;
                            }
                        }
                        crossterm::event::KeyCode::Down => {
                            if state_common.debug_scroll < (state_common.debug_lines - 1) {
                                state_common.debug_scroll += 1;
                                should_draw = true;
                            }
                        }
                        crossterm::event::KeyCode::Char(c) => {
                            let uc = c.to_ascii_uppercase();
                            if uc == 'Q' {
                                break;
                            } else if uc == 'M' && audio_out_stream.is_some() {
                                let _ = audio_out_channels
                                    .command_send
                                    .send(ConsoleAudioCommands::PlayOpus(3));
                            } else if uc == 'V' {
                                if let Some(ind) = my_conn_index {
                                    let state_change = state_common.connections[ind].state ^ 4;
                                    let _ = channels.command_send.send(NetworkCommand::Client(
                                        ClientCommand::StateChange(state_change),
                                    ));
                                }
                            } else if uc == 'T' && !already_transfered {
                                if let Ok(bytes) =
                                    std::fs::read(std::path::Path::new(TRANSFER_AUDIO))
                                {
                                    if let Some(opus_data) =
                                        OpusData::convert_ogg_opus_file(&bytes, 45)
                                    {
                                        let _ = channels.command_send.send(NetworkCommand::Client(
                                            ClientCommand::MusicTransfer(opus_data),
                                        ));
                                        already_transfered = true;
                                    }
                                }
                            } else if uc == 'S' {
                                if let Some(ind) = my_conn_index {
                                    let state_change = state_common.connections[ind].state ^ 2;
                                    let _ = channels.command_send.send(NetworkCommand::Client(
                                        ClientCommand::StateChange(state_change),
                                    ));
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
            match channels.network_state_recv.try_recv() {
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {
                            break;
                        }
                        TryRecvError::Disconnected => {
                            //state_common.debug_string.push_str("Network Debug Recv Disconnected!!!\n");
                            //state_common.debug_lines += 1;
                            break;
                        }
                    }
                }
                Ok(recv_state_cmd) => {
                    match recv_state_cmd {
                        NetworkStateMessage::StateChange((entry, state)) => {
                            state_common.connections[entry].state = state;
                            if let Some(ind) = my_conn_index {
                                if entry == ind {
                                    if state & 4 > 0 {
                                        if !is_in_vc {
                                            is_in_vc = true;
                                            if audio_out_stream.is_some() {
                                                let _ = audio_out_channels
                                                    .command_send
                                                    .send(ConsoleAudioCommands::PlayOpus(1));
                                            }
                                        }
                                    } else if is_in_vc {
                                        is_in_vc = false;
                                        if audio_out_stream.is_some() {
                                            let _ = audio_out_channels
                                                .command_send
                                                .send(ConsoleAudioCommands::PlayOpus(2));
                                        }
                                    }
                                }
                            }
                        }
                        NetworkStateMessage::NewConnection((user_name, state)) => {
                            let conn_state = NetworkStateConnection {
                                name: user_name,
                                state,
                            };
                            state_common.connections.push(conn_state);
                        }
                        NetworkStateMessage::ServerNameChange(server_name) => {
                            state_common.title_string = server_name;
                        }
                        NetworkStateMessage::ConnectionsRefresh((
                            new_conn_index,
                            connection_state_vec,
                        )) => {
                            my_conn_index = new_conn_index;
                            state_common.connections = connection_state_vec;
                            if let Some(conn_ind) = my_conn_index {
                                let state_test = state_common.connections[conn_ind].state;
                                if state_test & 4 > 0 {
                                    if !is_in_vc {
                                        is_in_vc = true;
                                        if audio_out_stream.is_some() {
                                            let _ = audio_out_channels
                                                .command_send
                                                .send(ConsoleAudioCommands::PlayOpus(1));
                                        }
                                    }
                                } else if is_in_vc {
                                    is_in_vc = false;
                                    if audio_out_stream.is_some() {
                                        let _ = audio_out_channels
                                            .command_send
                                            .send(ConsoleAudioCommands::PlayOpus(2));
                                    }
                                }
                            }
                        }
                    }
                    should_draw = true;
                }
            }
        }

        loop {
            match channels.network_debug_recv.try_recv() {
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {
                            break;
                        }
                        TryRecvError::Disconnected => {
                            //state_common.debug_string.push_str("Network Debug Recv Disconnected!!!\n");
                            //state_common.debug_lines += 1;
                            break;
                        }
                    }
                }
                Ok(recv_string) => {
                    state_common.debug_string.push_str(&recv_string);
                    state_common.debug_lines += 1;
                    should_draw = true;
                }
            }
        }

        loop {
            match audio_out_channels.debug_recv.try_recv() {
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {
                            break;
                        }
                        TryRecvError::Disconnected => {
                            //state_common.debug_string.push_str("Network Debug Recv Disconnected!!!\n");
                            //state_common.debug_lines += 1;
                            break;
                        }
                    }
                }
                Ok(recv_string) => {
                    state_common.debug_string.push_str(recv_string);
                    state_common.debug_lines += 1;
                    should_draw = true;
                }
            }
        }

        if should_draw {
            terminal.draw(|frame| console_ui(frame, &state_common, my_conn_index))?;
            should_draw = false;
        }
    }

    let _ = channels.command_send.send(NetworkCommand::Stop(42));

    if let Some(audio_out) = audio_out_stream {
        let _ = audio_out.pause();
    }

    // Cleanup Console Here:
    std::io::stdout().execute(crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}

fn console_ui(frame: &mut ratatui::Frame, state: &ConsoleStateCommon, my_state: Option<usize>) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(frame.size());

    // Render Connections and their States
    let mut rows = Vec::new();

    for (conn_ind, conn) in state.connections.iter().enumerate() {
        let mut row = Vec::new();
        let username_cell = Cell::from(conn.name.clone());

        if let Some(test_ind) = my_state {
            if test_ind == conn_ind {
                let username_cell =
                    username_cell.style(Style::default().add_modifier(Modifier::BOLD));
                row.push(username_cell);
            } else {
                row.push(username_cell);
            }
        } else {
            row.push(username_cell);
        }

        let mut state_test = 1;
        for i in 1..4 {
            if conn.state & state_test > 0 {
                row.push(Cell::from("X"));
            } else {
                row.push(Cell::from(" "));
            }
            state_test <<= 1;
        }

        rows.push(Row::new(row));
    }

    let header_row = [
        String::from("Name"),
        String::from("T"),
        String::from("S"),
        String::from("V"),
        String::from("L"),
    ];

    let widths = [
        Constraint::Min(32),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ];

    let table = Table::new(rows, widths)
        .header(Row::new(header_row))
        .column_spacing(1)
        .block(
            ratatui::widgets::Block::new()
                .borders(Borders::ALL)
                .title(state.title_string.as_str()),
        );

    frame.render_widget(table, layout[0]);

    // Render Debug Text
    frame.render_widget(
        Paragraph::new(state.debug_string.as_str())
            .scroll((state.debug_scroll, 0))
            .block(Block::new().borders(Borders::ALL).title(DEBUG_STR)),
        layout[1],
    );

    // Add scrolling to debug text
    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state =
        ScrollbarState::new(state.debug_lines as usize).position(state.debug_scroll as usize);

    frame.render_stateful_widget(
        scrollbar,
        layout[1].inner(&Margin {
            vertical: 1,
            horizontal: 0,
        }), // using a inner vertical margin of 1 unit makes the scrollbar inside the block
        &mut scrollbar_state,
    );
}
