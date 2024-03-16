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
    ConsoleThreadChannels, NetworkCommand, NetworkStateConnection, NetworkStateMessage,
    TryRecvError,
};

mod network;
use swiftlet_quic::endpoint::SocketAddr;

#[cfg(feature = "client")]
mod client;

use crossterm::ExecutableCommand; // Needed to use .execute on Stdout for crossterm setup
use ratatui::{prelude::*, widgets::*};
use std::thread;

#[derive(bpaf::Bpaf, Debug)]
#[bpaf(options, version)]
struct Arguments {
    /// Client Username when an address is specified, otherwise the Server name
    #[bpaf(short, long, argument("STRING"))]
    name: String,

    /// Optional Server Address that the program will attempt to connect to while operating as a Client.
    /// If not specified then the program will operate as a Server.
    /// Must be in `127.0.0.1:443` or `[::1]:443` format.
    /// If the port is set to 0 then the port argument will be used instead (program default if unset).
    #[bpaf(short, long, argument("SOCKET"))]
    address: Option<SocketAddr>,

    /// Port to use when operating as a Server or the default value to use with the connecting address
    #[bpaf(short, long, argument("NUM"), fallback(PORT_DEFAULT))]
    port: u16,

    /// Use IPv4 when the program is operating as a Server
    #[bpaf(long)]
    ipv4: bool,

    /// Enable Rust Backtrace.
    /// Only useful when program was built in debug mode
    #[bpaf(long)]
    backtrace: bool,
}

fn main() -> std::io::Result<()> {
    println!("Swiftlet Program Started");

    // Argument Parsing
    let args = arguments().run();

    if args.backtrace {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    // Initialize inter-thread channels common to both clients and servers (a future "headless" server won't need these)
    let (network_channels, console_channels) = communication::create_networking_console_channels();

    // Check if the program started as a Client or a Server
    match args.address {
        // This is a client because we have an address to connect to
        Some(mut server_address) => {
            if server_address.port() == 0 {
                server_address.set_port(args.port);
            }

            #[cfg(feature = "client")]
            {
                let (audio_out_channels, network_audio_out_channels, console_audio_out_channels) =
                    communication::create_audio_output_channels();

                let mut client = client::Client::new(server_address, console_audio_out_channels);

                // Start Network Thread
                let network_thread_handler = thread::spawn(move || {
                    network::client_thread(
                        server_address,
                        args.name,
                        network_channels,
                        network_audio_out_channels,
                    )
                });

                // Start Audio Thread
                let audio_thread_handler =
                    thread::spawn(move || client::audio::audio_thread(audio_out_channels));

                // Start Console
                let _ = client.run_console(console_channels);

                // Wait for Network Thread to Finish
                network_thread_handler.join().unwrap();
                audio_thread_handler.join().unwrap();
            }
            #[cfg(not(feature = "client"))]
            println!("Client Not Enabled");
        }
        None => {
            // No server address was provided, so the program is operating as a Server

            // Start Network Thread
            let server_name = args.name.clone();
            let network_thread_handler = thread::spawn(move || {
                network::server_thread(args.ipv4, args.port, server_name, network_channels)
            });

            // Start Console
            let _ = run_console_server(args.name, console_channels);

            // Wait for Network Thread to Finish
            network_thread_handler.join().unwrap();
        }
    }

    println!("Swiftlet Quitting");
    Ok(())
}

struct ConsoleStateCommon {
    title_string: String,
    debug_string: String,
    debug_lines: u16,
    debug_scroll: u16,
    connections: Vec<NetworkStateConnection>,
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
        Constraint::Length(33),
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
