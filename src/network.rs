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

const SERVER_NAME: &str = "localhost"; // Server "Name" / Domain Name that should ideally be on the server certificate that the client connects to
const ALPN_NAME: &[u8] = b"networking_audio"; // Application-Layer Protocol Negotiation Name used to define the Quic-Prog(ram) Protocol used in this application
const CERT_PATH: &str = "security/cert.pem"; // Location of the certificate for the server to use (temporarily used by client to verify server)
const PKEY_PATH: &str = "security/pkey.pem"; // Location of the private key for the server to use

const MAX_DATAGRAM_SIZE: usize = 1232; // IPv6 defines a min of 1280 bytes which after IPv6 and UDP headers shrinks to 1232 bytes
                                       // More info can be found here: https://datatracker.ietf.org/doc/html/rfc9000#name-datagram-size
                                       // Quic header reduces max payload data length to ____ bytes
const MAX_SERVER_CONNS: u64 = 32; // Max connections to the server at any given point
const MAIN_STREAM_ID: u64 = 0; // Bidirectional stream ID# used for reliable communication in the application between the server and the client (has to be zero for quiche)

use crate::communication::{
    ConsoleCommands,
    NetworkStateConnection,
    //NetworkAudioOutputChannels, NetworkAudioPackets
    NetworkStateMessage,
    NetworkThreadChannels,
    Receiver,
    // Use Inter-Thread Communication Definitions
    Sender,
};

pub use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::{
    thread,
    time::{Duration, Instant},
}; // IPv6 Addresses and Sockets used when sending the client an initial connection address

mod manage;
use manage::{ClientManager, ServerManager, UpdateEvent};

mod message;
use message::{MessageType, StreamMessage};

use self::manage::StreamReadable;

fn u8_to_str(data: &[u8]) -> String {
    let str_local = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(err) => {
            let index = err.valid_up_to();
            match std::str::from_utf8(&data[..index]) {
                Ok(s) => s,
                Err(_) => {
                    // Should never happen
                    return String::new();
                }
            }
        }
    };
    str_local.to_string()
}

const MAX_CHAR_LENGTH: usize = 32;

struct ClientState {
    id: u64,
    user_name: [u8; MAX_CHAR_LENGTH * 4],
    user_name_len: usize,
    state: u8, // Bit State [reserved, serverMusicConnected, voiceChatConnected, voiceChatLoopback]
    main_recv: StreamMessage,
}

struct ServerState {
    name: [u8; MAX_CHAR_LENGTH * 4],
    name_len: usize,
    client_states: Vec<ClientState>,
    main_send: StreamMessage,
}

impl ServerState {
    fn new(server_name: String) -> Self {
        let mut name = [0; 128];
        let mut name_len = 0;

        for (c_ind, c) in server_name.chars().enumerate() {
            if c_ind >= MAX_CHAR_LENGTH {
                break;
            }

            let new_name_len = name_len + c.len_utf8();
            let name_subslice = &mut name[name_len..new_name_len];
            c.encode_utf8(name_subslice);
            name_len = new_name_len;
        }

        if name_len == 0 {
            name[0] = b'S';
            name_len = 1;
        }

        ServerState {
            name,
            name_len,
            client_states: Vec::new(),
            main_send: StreamMessage::new_send(MessageType::InvalidType),
        }
    }

    #[inline]
    fn find_client_state_index(&self, cs_id: u64) -> Option<usize> {
        self.client_states.iter().position(|cs| cs.id == cs_id)
    }

    #[inline]
    fn find_client_state_index_with_probable(
        &self,
        cs_id: u64,
        probable_index: usize,
    ) -> Option<usize> {
        if probable_index < self.client_states.len()
            && self.client_states[probable_index].id == cs_id
        {
            Some(probable_index)
        } else {
            self.client_states.iter().position(|cs| cs.id == cs_id)
        }
    }

    fn add_client_state(&mut self, cs_id: u64, stream_msg: StreamMessage) -> Option<usize> {
        if self.find_client_state_index(cs_id).is_none() {
            if let Some(readable_data) = stream_msg.get_data_to_read() {
                let username_len = readable_data[0] as usize;

                let mut name = [0; 128];
                let mut name_len = 0;

                let username = &readable_data[1..username_len + 1];

                let name_str = match std::str::from_utf8(username) {
                    Ok(s) => s,
                    Err(err) => {
                        let index = err.valid_up_to();
                        match std::str::from_utf8(&username[..index]) {
                            Ok(s) => s,
                            Err(err) => {
                                return None;
                            }
                        }
                    }
                };

                for (c_ind, c) in name_str.chars().enumerate() {
                    if c_ind >= MAX_CHAR_LENGTH {
                        break;
                    }

                    let new_name_len = name_len + c.len_utf8();
                    let name_subslice = &mut name[name_len..new_name_len];
                    c.encode_utf8(name_subslice);
                    name_len = new_name_len;
                }

                if name_len == 0 {
                    return None;
                }

                let mut client_state = ClientState {
                    id: cs_id,
                    user_name: name,
                    user_name_len: name_len,
                    state: 0,
                    main_recv: stream_msg,
                };
                self.client_states.push(client_state);

                Some(self.client_states.len() - 1)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn remove_client_state(&mut self, cs_id: u64) -> bool {
        if let Some(index) = self.find_client_state_index(cs_id) {
            self.client_states.remove(index);
            true
        } else {
            false
        }
    }

    fn create_refresh_data(&mut self) -> bool {
        if self.main_send.refresh_send(MessageType::ServerStateRefresh) {
            if let Some(write_data) = self.main_send.get_data_to_write() {
                let mut write_size = 3;
                write_data[0] = self.client_states.len() as u8;
                write_data[1] = 255;
                write_data[2] = self.name_len as u8;
                write_data[write_size..(write_size + self.name_len)]
                    .copy_from_slice(&self.name[..self.name_len]);
                write_size += self.name_len;

                for cs in &self.client_states {
                    write_data[write_size] = cs.user_name_len as u8;
                    write_size += 1;
                    write_data[write_size..(write_size + cs.user_name_len)]
                        .copy_from_slice(&cs.user_name[..cs.user_name_len]);
                    write_size += cs.user_name_len;
                    write_data[write_size] = cs.state;
                    write_size += 1;
                }

                write_data[write_size] = 0;
                write_size += 1;
                //println!("Value: {}", write_size);
                self.main_send.update_data_write(write_size);

                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn create_new_client_data(&mut self, verified_index: usize) -> bool {
        if self.main_send.refresh_send(MessageType::NewClient) {
            if let Some(write_data) = self.main_send.get_data_to_write() {
                let cs = &self.client_states[verified_index];

                write_data[0] = cs.user_name_len as u8;
                let mut write_size = 1;
                write_data[write_size..(write_size + cs.user_name_len)]
                    .copy_from_slice(&cs.user_name[..cs.user_name_len]);
                write_size += cs.user_name_len;

                write_data[write_size] = cs.state;
                write_size += 1;

                self.main_send.update_data_write(write_size);

                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn create_state_change_data(&mut self, verified_index: usize) -> bool {
        if self.main_send.refresh_send(MessageType::ClientNewState) {
            if let Some(write_data) = self.main_send.get_data_to_write() {
                let cs = &self.client_states[verified_index];

                write_data[0] = verified_index as u8;
                write_data[1] = cs.state;

                self.main_send.update_data_write(2);

                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn refresh_update(&self, network_state_send: &Sender<NetworkStateMessage>) {
        let mut state_populate = Vec::<NetworkStateConnection>::new();

        for cs in &self.client_states {
            let conn_state = NetworkStateConnection {
                name: u8_to_str(&cs.user_name[..cs.user_name_len]),
                state: cs.state,
            };
            state_populate.push(conn_state);
        }

        let state_update = NetworkStateMessage::ConnectionsRefresh((None, state_populate));
        let _ = network_state_send.send(state_update);
    }

    fn new_connection_update(
        &self,
        verified_index: usize,
        network_state_send: &Sender<NetworkStateMessage>,
    ) {
        let cs = &self.client_states[verified_index];
        let conn_name = u8_to_str(&cs.user_name[..cs.user_name_len]);
        let state_update = NetworkStateMessage::NewConnection((conn_name, cs.state));
        let _ = network_state_send.send(state_update);
    }

    fn state_change_update(
        &self,
        verified_index: usize,
        network_state_send: &Sender<NetworkStateMessage>,
    ) {
        let cs = &self.client_states[verified_index];
        let state_update = NetworkStateMessage::StateChange((verified_index, cs.state));
        let _ = network_state_send.send(state_update);
    }
}

pub fn server_thread(
    use_ipv6: Option<bool>,
    port: u16,
    server_name: String,
    channels: NetworkThreadChannels,
) {
    let mut bind_address = match use_ipv6 {
        Some(ipv6) => match ipv6 {
            true => manage::SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port, 0, 0)),
            false => manage::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)),
        },
        None => manage::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)),
    };

    let mut server_mgr = match ServerManager::new(
        server_name.clone(),
        bind_address,
        ALPN_NAME,
        CERT_PATH,
        PKEY_PATH,
    ) {
        Ok(ss) => ss,
        Err(err) => {
            let _ = channels
                .network_debug_send
                .send("Server state creation error!\n");
            return;
        }
    };

    let mut server_state = ServerState::new(server_name);

    let mut tick_duration = Duration::from_millis(5);
    let start_instant = Instant::now();
    let mut next_tick_instant = start_instant;
    let mut command_handler_ticks = 0;
    loop {
        // Master "Event" Loop

        match server_mgr.update() {
            // Sleeps when it can (ie. waiting for next tick / recv data and time is > 1ms)
            UpdateEvent::ReceivedData => {
                server_read_loop(&mut server_state, &mut server_mgr, &channels);
            }
            UpdateEvent::NextTick => {
                next_tick_instant += tick_duration; // Does not currently check for skipped ticks / assumes computer processes all
                server_mgr.set_next_tick_instant(next_tick_instant);

                // Eventually handle data that gets sent at set intervals
                command_handler_ticks += 1;
                if command_handler_ticks >= 100 {
                    // Handle Commands Every 100 Ticks (0.5 sec)
                    //let _ = channels.network_debug_send.send("Server Command Handling\n");
                    if server_command_handler(&mut server_mgr, &channels.command_recv) {
                        break;
                    }
                    command_handler_ticks = 0;
                }
            }
            UpdateEvent::PotentiallyReceivedData => {
                server_read_loop(&mut server_state, &mut server_mgr, &channels);
            }
            UpdateEvent::ConnectionClosed(conn_id) => {
                if server_state.remove_client_state(conn_id) {
                    if server_state.create_refresh_data() {
                        if let Some(mut_data) = server_state.main_send.get_mut_data_to_send() {
                            for (cs_ind, cs) in server_state.client_states.iter().enumerate() {
                                mut_data[message::MESSAGE_HEADER_SIZE + 1] = cs_ind as u8;
                                server_mgr.send_stream_data(cs.id, MAIN_STREAM_ID, mut_data, false);
                            }
                        }
                    }

                    server_state.refresh_update(&channels.network_state_send);
                }
            }
            _ => {}
        }
    }

    // Eventual Friendly Server Cleanup Here

    let _ = channels
        .network_debug_send
        .send("Server Network Thread Exiting\n");
}

pub fn client_thread(
    server_address: SocketAddr,
    user_name: String,
    channels: NetworkThreadChannels,
) {
    let bind_address = match server_address.is_ipv6() {
        true => manage::SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0)),
        false => manage::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)),
    };

    let mut client_mgr = match ClientManager::new(
        user_name.clone(),
        bind_address,
        server_address,
        ALPN_NAME,
        CERT_PATH,
        SERVER_NAME,
    ) {
        Ok(cs) => cs,
        Err(err) => {
            let _ = channels
                .network_debug_send
                .send("Client state creation error!\n");
            return;
        }
    };

    let mut main_recv = None;
    let mut main_send = StreamMessage::new_send(MessageType::InvalidType);
    let mut tick_duration = Duration::from_millis(5);
    let mut command_handler_ticks = 0;
    let mut keep_alive_ticks = 0;
    loop {
        // Master "Event" Loop

        let start_instant = Instant::now();
        let mut next_tick_instant = start_instant;
        let mut new_connection_potential = true;
        loop {
            match client_mgr.update() {
                // Sleeps when it can (ie. waiting for next tick / recv data and time is > 1ms)
                UpdateEvent::ReceivedData => {
                    if client_read_loop(
                        &mut main_recv,
                        &mut main_send,
                        &user_name,
                        &mut client_mgr,
                        &channels,
                    ) {
                        break;
                    }
                }
                UpdateEvent::NextTick => {
                    next_tick_instant += tick_duration; // Does not currently check for skipped ticks / assumes computer processes all
                    client_mgr.set_next_tick_instant(next_tick_instant);

                    // Eventually handle data that gets sent at set intervals
                    keep_alive_ticks += 1;
                    if keep_alive_ticks >= 200 {
                        // Send a Keep Alive every 200 Ticks (1 sec)
                        main_send.refresh_send(MessageType::KeepConnectionAlive);
                        if let Some(send_data) = main_send.get_data_to_send() {
                            client_mgr.send_stream_data(MAIN_STREAM_ID, send_data, false);
                        }
                        keep_alive_ticks = 0;
                    }

                    command_handler_ticks += 1;
                    if command_handler_ticks >= 4 {
                        // Handle Commands Every 4 Ticks (20 ms)
                        if client_command_handler(
                            &mut main_send,
                            &mut client_mgr,
                            true,
                            &channels.command_recv,
                            &channels.network_debug_send,
                        ) {
                            new_connection_potential = false;
                            break;
                        }
                        command_handler_ticks = 0;
                    }
                }
                UpdateEvent::PotentiallyReceivedData => {
                    if client_read_loop(
                        &mut main_recv,
                        &mut main_send,
                        &user_name,
                        &mut client_mgr,
                        &channels,
                    ) {
                        break;
                    }
                }
                UpdateEvent::ConnectionClosed(conn_id) => {
                    break;
                }
                _ => {}
            }
        }

        if new_connection_potential {
            loop {
                thread::sleep(Duration::from_millis(100));
                if client_command_handler(
                    &mut main_send,
                    &mut client_mgr,
                    false,
                    &channels.command_recv,
                    &channels.network_debug_send,
                ) {
                    break;
                }
            }
            if client_mgr.is_connection_closed() {
                break;
            }
        } else {
            break;
        }
    }

    // Eventual Friendly Client Cleanup Here

    let _ = channels
        .network_debug_send
        .send("Client Network Thread Exiting\n");
}

fn server_read_loop(
    server_state: &mut ServerState,
    server_mgr: &mut ServerManager,
    channels: &NetworkThreadChannels,
) {
    loop {
        match server_mgr.recv_data() {
            UpdateEvent::StreamReceivedData(stream_readable) => {
                if stream_readable.stream_id == MAIN_STREAM_ID {
                    if let Some(verified_index) =
                        server_state.find_client_state_index(stream_readable.conn_id)
                    {
                        server_process_main_stream_data(
                            stream_readable,
                            verified_index,
                            server_mgr,
                            server_state,
                            channels,
                        );
                    } else {
                        // See if first data sent by the new client is what was expected... else close connection
                        let mut header_data = [0; message::MESSAGE_HEADER_SIZE];
                        match server_mgr.recv_stream_data(&stream_readable, &mut header_data) {
                            Ok((header_bytes, header_fin)) => {
                                if header_bytes == message::MESSAGE_HEADER_SIZE && !header_fin {
                                    let mut stream_message = StreamMessage::new_recv(header_data);
                                    match stream_message.get_message_type() {
                                        MessageType::NewClientAnnounce => {
                                            if let Some(data_recv) =
                                                stream_message.get_data_to_recv()
                                            {
                                                // Expects NewClientAnnounce Message to be readable all at once (fair assumption)
                                                match server_mgr
                                                    .recv_stream_data(&stream_readable, data_recv)
                                                {
                                                    Ok((recv_bytes, fin)) => {
                                                        if !header_fin {
                                                            stream_message
                                                                .update_data_recv(recv_bytes);
                                                            if let Some(index) = server_state
                                                                .add_client_state(
                                                                    stream_readable.conn_id,
                                                                    stream_message,
                                                                )
                                                            {
                                                                if server_state
                                                                    .create_refresh_data()
                                                                {
                                                                    if let Some(mut_data) =
                                                                        server_state
                                                                            .main_send
                                                                            .get_mut_data_to_send()
                                                                    {
                                                                        mut_data[message::MESSAGE_HEADER_SIZE + 1] = index as u8;
                                                                        server_mgr
                                                                            .send_stream_data(
                                                                                stream_readable
                                                                                    .conn_id,
                                                                                MAIN_STREAM_ID,
                                                                                mut_data,
                                                                                false,
                                                                            );
                                                                    }
                                                                }

                                                                if server_state
                                                                    .create_new_client_data(index)
                                                                {
                                                                    if let Some(send_data) =
                                                                        server_state
                                                                            .main_send
                                                                            .get_data_to_send()
                                                                    {
                                                                        for (cs_ind, cs) in
                                                                            server_state
                                                                                .client_states
                                                                                .iter()
                                                                                .enumerate()
                                                                        {
                                                                            if cs_ind != index {
                                                                                server_mgr.send_stream_data(cs.id, MAIN_STREAM_ID, send_data, false);
                                                                            }
                                                                        }
                                                                    }
                                                                }

                                                                server_state.new_connection_update(
                                                                    index,
                                                                    &channels.network_state_send,
                                                                );
                                                            }
                                                        } else {
                                                            // Close the connection
                                                        }
                                                    }
                                                    Err(s) => {
                                                        // Probably close the connection but also print out error
                                                    }
                                                }
                                            }
                                        }
                                        _ => {
                                            // Close the connection
                                        }
                                    }
                                } else {
                                    // Close the connection
                                }
                            }
                            Err(s) => {
                                // Probably close the connection but also print out error
                            }
                        }
                    }
                }
            }
            UpdateEvent::FinishedReceiving => {
                break;
            }
            UpdateEvent::ConnectionClosed(conn_id) => {
                if server_state.remove_client_state(conn_id) {
                    if server_state.create_refresh_data() {
                        if let Some(mut_data) = server_state.main_send.get_mut_data_to_send() {
                            for (cs_ind, cs) in server_state.client_states.iter().enumerate() {
                                mut_data[message::MESSAGE_HEADER_SIZE + 1] = cs_ind as u8;
                                server_mgr.send_stream_data(cs.id, MAIN_STREAM_ID, mut_data, false);
                            }
                        }
                    }

                    server_state.refresh_update(&channels.network_state_send);
                }
            }
            UpdateEvent::NoUpdate => {
                // NO break
            }
            UpdateEvent::FinishedConnectingOnce(_) => {
                // No error
            }
            UpdateEvent::NewConnectionStarted(_) => {
                // NO break
            }
            UpdateEvent::SocketManagerError => {
                let _ = channels.network_debug_send.send("Server Socket Error!\n");
                break;
            }
            _ => {
                // Some form of error
                let _ = channels
                    .network_debug_send
                    .send("Server Manager Recv Error!\n");
                break;
            }
        }
    }
}

fn client_read_loop(
    main_stream_read: &mut Option<StreamMessage>,
    main_send: &mut StreamMessage,
    user_name: &str,
    client_mgr: &mut ClientManager,
    channels: &NetworkThreadChannels,
) -> bool {
    loop {
        match client_mgr.recv_data() {
            UpdateEvent::StreamReceivedData(stream_readable) => {
                if stream_readable.stream_id == MAIN_STREAM_ID {
                    if let Some(stream_msg_recv) = main_stream_read {
                        client_process_main_stream_data(
                            stream_readable,
                            stream_msg_recv,
                            main_send,
                            client_mgr,
                            channels,
                        );
                    } else {
                        let mut header_data = [0; message::MESSAGE_HEADER_SIZE];
                        match client_mgr.recv_stream_data(&stream_readable, &mut header_data) {
                            Ok((header_bytes, header_fin)) => {
                                if header_bytes == message::MESSAGE_HEADER_SIZE && !header_fin {
                                    let mut stream_message = StreamMessage::new_recv(header_data);
                                    match stream_message.get_message_type() {
                                        MessageType::ServerStateRefresh => {
                                            //let _ = channels.network_debug_send.send("Server Refresh Header Got!\n");

                                            *main_stream_read = Some(stream_message);
                                            if let Some(stream_msg_recv) = main_stream_read {
                                                client_process_main_stream_data(
                                                    stream_readable,
                                                    stream_msg_recv,
                                                    main_send,
                                                    client_mgr,
                                                    channels,
                                                );
                                            }
                                        }
                                        _ => {
                                            // Close the connection
                                        }
                                    }
                                } else {
                                    // Close the connection
                                }
                            }
                            Err(e) => {
                                // Probably close the connection but also print out error
                            }
                        }
                    }
                }
            }
            UpdateEvent::FinishedReceiving => {
                break;
            }
            UpdateEvent::ConnectionClosed(_) => {
                let _ = channels
                    .network_debug_send
                    .send("Client Connection Closed!\n");
                return true;
            }
            UpdateEvent::FinishedConnectingOnce(_) => {
                client_mgr.create_stream(MAIN_STREAM_ID, 100);

                main_send.refresh_send(MessageType::NewClientAnnounce);
                if let Some(write_data) = main_send.get_data_to_write() {
                    let _ = channels.network_debug_send.send("Client Announce Sent!\n");
                    let mut start_index = 1;
                    for (c_ind, c) in user_name.chars().enumerate() {
                        if c_ind >= MAX_CHAR_LENGTH {
                            break;
                        }

                        let new_start_index = start_index + c.len_utf8();
                        let c_subslice = &mut write_data[start_index..new_start_index];
                        c.encode_utf8(c_subslice);
                        start_index = new_start_index;
                    }
                    write_data[0] = (start_index - 1) as u8;

                    main_send.update_data_write(start_index);

                    if let Some(send_data) = main_send.get_data_to_send() {
                        client_mgr.send_stream_data(MAIN_STREAM_ID, send_data, false);
                    }
                }
            }
            UpdateEvent::NoUpdate => {
                // NO break
            }
            _ => {
                // Some form of error
                let _ = channels
                    .network_debug_send
                    .send("Client Manager Recv Error!\n");
                break;
            }
        }
    }
    false
}

fn server_process_main_stream_data(
    stream_readable: StreamReadable,
    verified_index: usize,
    server_mgr: &mut ServerManager,
    server_state: &mut ServerState,
    channels: &NetworkThreadChannels,
) {
    let mut close_connection = false;

    loop {
        let stream_msg = &mut server_state.client_states[verified_index].main_recv;
        if stream_msg.is_done_recving() {
            let mut header_data = [0; message::MESSAGE_HEADER_SIZE];
            match server_mgr.recv_stream_data(&stream_readable, &mut header_data) {
                Ok((header_bytes, header_fin)) => {
                    if header_bytes == message::MESSAGE_HEADER_SIZE && !header_fin {
                        stream_msg.refresh_recv(header_data);
                        if let Some(data_recv) = stream_msg.get_data_to_recv() {
                            match server_mgr.recv_stream_data(&stream_readable, data_recv) {
                                Ok((recv_bytes, recv_fin)) => {
                                    stream_msg.update_data_recv(recv_bytes);
                                    if recv_fin {
                                        close_connection = true;
                                    }
                                }
                                Err(e) => {
                                    // Probably close the connection but also print out error
                                }
                            }
                        }
                    } else if header_bytes == 0 && !header_fin {
                        break;
                    } else {
                        close_connection = true;
                        break;
                    }
                }
                Err(e) => match e {
                    quiche::Error::Done => break,
                    _ => {
                        let _ = channels.network_debug_send.send("Error close??\n");
                    }
                },
            }
        } else if let Some(data_recv) = stream_msg.get_data_to_recv() {
            match server_mgr.recv_stream_data(&stream_readable, data_recv) {
                Ok((recv_bytes, recv_fin)) => {
                    stream_msg.update_data_recv(recv_bytes);
                    if recv_fin {
                        close_connection = true;
                    }
                }
                Err(e) => {
                    // Probably close the connection but also print out error
                }
            }
        }

        if let Some(data_read) = stream_msg.get_data_to_read() {
            match stream_msg.get_message_type() {
                MessageType::NewStateRequest => {
                    let potential_new_state = data_read[0];
                    // In future check if server will allow state change here!
                    server_state.client_states[verified_index].state = potential_new_state;

                    let mut msg_send = StreamMessage::new_send(MessageType::ClientNewState);
                    if let Some(write_data) = msg_send.get_data_to_write() {
                        write_data[0] = verified_index as u8;
                        write_data[1] = server_state.client_states[verified_index].state;

                        msg_send.update_data_write(2);

                        if let Some(send_data) = msg_send.get_data_to_send() {
                            for cs in server_state.client_states.iter() {
                                server_mgr.send_stream_data(
                                    cs.id,
                                    MAIN_STREAM_ID,
                                    send_data,
                                    false,
                                );
                            }
                        }
                    }

                    server_state.state_change_update(verified_index, &channels.network_state_send);
                }
                MessageType::FileTransferRequest => {
                    // Write later
                }
                _ => {}
            }
        } else {
            break;
        }
    }

    if close_connection {
        let _ = channels
            .network_debug_send
            .send("Server Connection Should Close!\n");
    }
}

fn client_process_main_stream_data(
    stream_readable: StreamReadable,
    stream_msg_recv: &mut StreamMessage,
    main_send: &mut StreamMessage,
    client_mgr: &mut ClientManager,
    channels: &NetworkThreadChannels,
) {
    let mut close_connection = false;

    loop {
        if stream_msg_recv.is_done_recving() {
            let mut header_data = [0; message::MESSAGE_HEADER_SIZE];
            match client_mgr.recv_stream_data(&stream_readable, &mut header_data) {
                Ok((header_bytes, header_fin)) => {
                    if header_bytes == message::MESSAGE_HEADER_SIZE && !header_fin {
                        stream_msg_recv.refresh_recv(header_data);
                        if let Some(data_recv) = stream_msg_recv.get_data_to_recv() {
                            match client_mgr.recv_stream_data(&stream_readable, data_recv) {
                                Ok((recv_bytes, recv_fin)) => {
                                    stream_msg_recv.update_data_recv(recv_bytes);
                                    if recv_fin {
                                        close_connection = true;
                                    }
                                }
                                Err(e) => {
                                    // Probably close the connection but also print out error
                                }
                            }
                        }
                    } else if header_bytes == 0 && !header_fin {
                        break;
                    } else {
                        close_connection = true;
                        break;
                    }
                }
                Err(e) => match e {
                    quiche::Error::Done => break,
                    _ => {
                        let _ = channels.network_debug_send.send("Error close??\n");
                    }
                },
            }
        } else if let Some(data_recv) = stream_msg_recv.get_data_to_recv() {
            match client_mgr.recv_stream_data(&stream_readable, data_recv) {
                Ok((recv_bytes, recv_fin)) => {
                    //println!("Value: {}", recv_bytes);
                    stream_msg_recv.update_data_recv(recv_bytes);
                    if recv_fin {
                        close_connection = true;
                    }
                }
                Err(e) => {
                    // Probably close the connection but also print out error
                }
            }
        }

        if let Some(data_read) = stream_msg_recv.get_data_to_read() {
            match stream_msg_recv.get_message_type() {
                MessageType::ServerStateRefresh => {
                    // State Refresh

                    let conn_ind = data_read[1] as usize;

                    let mut name_end: usize = (data_read[2] + 3).into();
                    let mut server_name = u8_to_str(&data_read[3..name_end]);
                    let name_update = NetworkStateMessage::ServerNameChange(server_name);
                    let _ = channels.network_state_send.send(name_update);

                    let mut state_populate = Vec::<NetworkStateConnection>::new();

                    let mut name_len: usize = data_read[name_end].into();
                    while name_len != 0 {
                        let name_start = name_end + 1;
                        name_end = name_len + name_start;
                        let mut client_name = u8_to_str(&data_read[name_start..name_end]);

                        let conn_state = NetworkStateConnection {
                            name: client_name,
                            state: data_read[name_end],
                        };

                        state_populate.push(conn_state);

                        name_end += 1;
                        name_len = data_read[name_end].into();
                    }

                    let state_update =
                        NetworkStateMessage::ConnectionsRefresh((Some(conn_ind), state_populate));
                    let _ = channels.network_state_send.send(state_update);

                    let _ = channels.network_debug_send.send("Server Refresh Recv!!\n");
                }
                MessageType::NewClient => {
                    let mut name_end: usize = (data_read[0] + 1).into();
                    let mut client_name = u8_to_str(&data_read[1..name_end]);
                    let new_conn =
                        NetworkStateMessage::NewConnection((client_name, data_read[name_end]));
                    let _ = channels.network_state_send.send(new_conn);
                }
                MessageType::ClientNewState => {
                    let conn_pos = data_read[0] as usize;
                    let new_state = data_read[1];

                    let new_conn = NetworkStateMessage::StateChange((conn_pos, new_state));
                    let _ = channels.network_state_send.send(new_conn);
                }
                _ => {}
            }
        } else {
            break;
        }
    }

    if close_connection {
        let _ = channels
            .network_debug_send
            .send("Client Connection Should Close!\n");
    }
}

fn server_command_handler(
    server_state: &mut ServerManager,
    command_recv: &Receiver<ConsoleCommands>,
) -> bool {
    loop {
        match command_recv.try_recv() {
            Err(try_recv_error) => {
                // match try_recv_error {
                // 	TryRecvError::Empty => {
                // 		//break;
                // 	},
                // 	TryRecvError::Disconnected => {
                // 		//break;
                // 	}
                // }
                break;
            }
            Ok(cmd_recv) => {
                match cmd_recv {
                    ConsoleCommands::NetworkingStop(int) => {
                        //endpoint.close(quinn::VarInt::from_u64(int).unwrap(), b"shutdown");
                        return true;
                    }
                    ConsoleCommands::ServerConnectionClose(int) => {}
                    _ => {}
                }
            }
        }
    }
    false
}

fn client_command_handler(
    main_send: &mut StreamMessage,
    client_mgr: &mut ClientManager,
    connected: bool,
    command_recv: &Receiver<ConsoleCommands>,
    debug_send: &Sender<&'static str>,
) -> bool {
    loop {
        match command_recv.try_recv() {
            Err(try_recv_error) => {
                // match try_recv_error {
                // 	TryRecvError::Empty => {
                // 		//break;
                // 	},
                // 	TryRecvError::Disconnected => {
                // 		//break;
                // 	}
                // }
                break;
            }
            Ok(cmd_recv) => {
                if connected {
                    match cmd_recv {
                        ConsoleCommands::ClientStateChange(new_state_requested) => {
                            main_send.refresh_send(MessageType::NewStateRequest);
                            if let Some(write_data) = main_send.get_data_to_write() {
                                write_data[0] = new_state_requested;

                                main_send.update_data_write(1);

                                if let Some(send_data) = main_send.get_data_to_send() {
                                    client_mgr.send_stream_data(MAIN_STREAM_ID, send_data, false);
                                }
                            }
                        }
                        ConsoleCommands::ClientConnectionClose => {
                            client_mgr.close_connection(42);
                        }
                        ConsoleCommands::NetworkingStop(int) => {
                            client_mgr.close_connection(int);
                            return true;
                        }
                        _ => {}
                    }
                } else {
                    match cmd_recv {
                        ConsoleCommands::ClientReconnect(server_address) => {
                            return client_mgr.new_connection(
                                server_address,
                                ALPN_NAME,
                                CERT_PATH,
                                SERVER_NAME,
                            );
                        }
                        ConsoleCommands::NetworkingStop(int) => {
                            return true;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    false
}
