//Media Enhanced Swiftlet Quic Simple Example
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

const ALPN_NAME: &[u8] = b"simple"; // Application-Layer Protocol Negotiation Name used to define the Quic-Application Protocol used in this program
const SERVER_NAME: &str = "localhost"; // Server "Name" / Domain Name that should ideally be on the server certificate that the client connects to
const CERT_PATH: &str = "security/cert.pem"; // Location of the certificate for the server to use (temporarily used by client to verify server)
const PKEY_PATH: &str = "security/pkey.pem"; // Location of the private key for the server to use

use std::collections::BTreeMap;
use std::time::Duration;

use swiftlet_quic::{
    endpoint::{Config, ConnectionEndReason, ConnectionId, Endpoint, SocketAddr},
    EndpointEventCallbacks, EndpointHandler,
};

const MESSAGE_HEADER_SIZE: usize = 3;
const BUFFER_SIZE_PER_CONNECTION: usize = 65536; // 16 KiB

fn main() {
    //crossterm::terminal::enable_raw_mode().unwrap();

    println!("Press the S or C keys to quit the Server or Client threads respectively!");
    println!(
        "The key presses might get read by the other command loop so multiple presses is advised"
    );

    let port = 9001;
    let server_thread_handle =
        std::thread::spawn(move || server_thread(port, "Server".to_string()));
    std::thread::sleep(std::time::Duration::from_secs(1));

    let local_ipv6 = std::net::Ipv6Addr::from([0, 0, 0, 0, 0, 0, 0, 1]);
    let server_address = SocketAddr::V6(std::net::SocketAddrV6::new(local_ipv6, port, 0, 0));
    let client_thread_handle =
        std::thread::spawn(move || client_thread(server_address, "Client".to_string()));

    client_thread_handle.join().unwrap();
    server_thread_handle.join().unwrap();

    let google_ipv6_dns_addr_str = String::from("[2001:4860:4860::8888]:53");
    let remote_address = google_ipv6_dns_addr_str.parse().unwrap();
    let dns_thread_handle =
        std::thread::spawn(move || client_thread(remote_address, "DNS-Client".to_string()));
    dns_thread_handle.join().unwrap();

    //crossterm::terminal::disable_raw_mode().unwrap();
}

fn server_thread(port: u16, server_name: String) {
    let bind_address = SocketAddr::V6(std::net::SocketAddrV6::new(
        std::net::Ipv6Addr::UNSPECIFIED,
        port,
        0,
        0,
    ));

    let config = Config {
        idle_timeout_in_ms: 5000,
        reliable_stream_buffer: 65536,
        unreliable_stream_buffer: 65536,
        keep_alive_timeout: None,
        initial_main_recv_size: BUFFER_SIZE_PER_CONNECTION,
        main_recv_first_bytes: MESSAGE_HEADER_SIZE,
        initial_background_recv_size: BUFFER_SIZE_PER_CONNECTION,
        background_recv_first_bytes: MESSAGE_HEADER_SIZE,
    };

    let mut server_endpoint =
        match Endpoint::new_server(bind_address, ALPN_NAME, CERT_PATH, PKEY_PATH, config) {
            Ok(endpoint) => endpoint,
            Err(_) => {
                println!("Server Endpoint Creation Error!");
                // Can add more detailed print here later
                return;
            }
        };

    let mut server_state = ServerState::new(server_name);
    server_state.send_debug_text("Starting Server Network!\n");

    let mut endpoint_handler = EndpointHandler::new(&mut server_endpoint, &mut server_state);
    match endpoint_handler.run_event_loop(std::time::Duration::from_millis(5)) {
        Ok(true) => {
            println!("Server event loop ended due to the connection_ended callback function returning true!");
        }
        Ok(false) => {
            println!("Server event loop ended due to the tick callback function returning true!");
        }
        Err(e) => {
            println!("Server Error: {:?}", e);
        }
    }

    server_state.send_debug_text("Server Network Thread Exiting\n");
}

fn client_thread(server_address: SocketAddr, user_name: String) {
    let bind_address = SocketAddr::V6(std::net::SocketAddrV6::new(
        std::net::Ipv6Addr::UNSPECIFIED,
        0, // Unspecified bind port (OS chooses)
        0,
        0,
    ));

    let config = Config {
        idle_timeout_in_ms: 5000,
        reliable_stream_buffer: 65536,
        unreliable_stream_buffer: 65536,
        keep_alive_timeout: Some(Duration::from_millis(2000)),
        initial_main_recv_size: BUFFER_SIZE_PER_CONNECTION,
        main_recv_first_bytes: MESSAGE_HEADER_SIZE,
        initial_background_recv_size: BUFFER_SIZE_PER_CONNECTION,
        background_recv_first_bytes: MESSAGE_HEADER_SIZE,
    };

    let mut client_endpoint = match Endpoint::new_client_with_first_connection(
        bind_address,
        ALPN_NAME,
        CERT_PATH,
        server_address,
        SERVER_NAME,
        config,
    ) {
        Ok(endpoint) => endpoint,
        Err(_) => {
            println!("Client Endpoint Creation Error!");
            // Can add more detailed print here later
            return;
        }
    };

    let mut client_handler = ClientHandler::new(user_name);
    client_handler.send_debug_text("Starting Client Network!\n");

    let mut endpoint_handler = EndpointHandler::new(&mut client_endpoint, &mut client_handler);
    match endpoint_handler.run_event_loop(std::time::Duration::from_millis(5)) {
        Ok(true) => {
            println!("Client event loop ended due to the connection_ended callback function returning true!");
        }
        Ok(false) => {
            println!("Client event loop ended due to the tick callback function returning true!");
        }
        Err(e) => {
            println!("Client Error: {:?}", e);
        }
    }

    client_handler.send_debug_text("Client Network Thread Exiting\n");
}

#[derive(Debug)]
#[repr(u64)]
enum ErrorCode {
    NotUsed = 0,
    ServerClosed,
    ClientClosed,
}

impl ErrorCode {
    fn from_u64(value: u64) -> Self {
        match value {
            x if x == ErrorCode::ServerClosed as u64 => ErrorCode::ServerClosed,
            x if x == ErrorCode::ClientClosed as u64 => ErrorCode::ClientClosed,
            _ => ErrorCode::NotUsed,
        }
    }
}

#[repr(u8)]
enum StreamMsgType {
    InvalidType = 0,

    // Server Messages:
    ServerStateRefresh, // NumClientsConnected, ClientIndex, ServerNameLen, ServerName, {ClientXNameLen, ClientXName, ClientXState}... 0
    NewClient,          // ClientNameLen, ClientName, ClientState

    // Client Messages:
    NewClientAnnounce, // ClientNameLen, ClientName
}

impl StreamMsgType {
    #[inline] // Verbose but compiles down to minimal instructions
    fn from_u8(byte: u8) -> Self {
        match byte {
            x if x == StreamMsgType::ServerStateRefresh as u8 => StreamMsgType::ServerStateRefresh,
            x if x == StreamMsgType::NewClient as u8 => StreamMsgType::NewClient,
            x if x == StreamMsgType::NewClientAnnounce as u8 => StreamMsgType::NewClientAnnounce,
            _ => StreamMsgType::InvalidType,
        }
    }

    #[inline]
    fn to_u8(&self) -> u8 {
        match self {
            StreamMsgType::ServerStateRefresh => StreamMsgType::ServerStateRefresh as u8,
            StreamMsgType::NewClient => StreamMsgType::NewClient as u8,
            StreamMsgType::NewClientAnnounce => StreamMsgType::NewClientAnnounce as u8,
            _ => StreamMsgType::InvalidType as u8,
        }
    }

    #[inline]
    fn intended_for_client(&self) -> bool {
        matches!(
            self,
            StreamMsgType::ServerStateRefresh | StreamMsgType::NewClient
        )
    }

    #[inline]
    fn intended_for_server(&self) -> bool {
        matches!(self, StreamMsgType::NewClientAnnounce)
    }

    #[inline]
    fn get_stream_msg(&self) -> Vec<u8> {
        Vec::from([self.to_u8(), 0, 0])
    }
}

#[inline]
fn set_stream_msg_size(vec_data: &mut Vec<u8>) {
    let num_bytes = usize::to_ne_bytes(vec_data.len() - MESSAGE_HEADER_SIZE);
    vec_data[1] = num_bytes[0];
    vec_data[2] = num_bytes[1];
}

#[inline]
fn get_stream_msg_size(read_data: &[u8]) -> usize {
    usize::from_ne_bytes([read_data[1], read_data[2], 0, 0, 0, 0, 0, 0])
}

const MAX_CHAR_LENGTH: usize = 32;

struct ClientState {
    main_recv_type: Option<StreamMsgType>,
    user_name: [u8; MAX_CHAR_LENGTH * 4],
    user_name_len: usize,
    state: u8, // Doesn't represent anything
}

impl ClientState {
    fn new(user_name_bytes: &[u8]) -> Option<Self> {
        let mut cs = ClientState {
            main_recv_type: None,
            user_name: [0; MAX_CHAR_LENGTH * 4],
            user_name_len: 0,
            state: 0,
        };
        cs.user_name_len = 0;

        let name_str = match std::str::from_utf8(user_name_bytes) {
            Ok(s) => s,
            Err(err) => {
                let index = err.valid_up_to();
                match std::str::from_utf8(&user_name_bytes[..index]) {
                    Ok(s) => s,
                    Err(_) => {
                        return None;
                    }
                }
            }
        };

        for (c_ind, c) in name_str.chars().enumerate() {
            if c_ind >= MAX_CHAR_LENGTH {
                break;
            }

            let new_name_len = cs.user_name_len + c.len_utf8();
            let name_subslice = &mut cs.user_name[cs.user_name_len..new_name_len];
            c.encode_utf8(name_subslice);
            cs.user_name_len = new_name_len;
        }

        Some(cs)
    }
}

struct ServerState {
    name: [u8; MAX_CHAR_LENGTH * 4],
    name_len: usize,
    command_handler_tick: u64,
    potential_clients: Vec<ConnectionId>,
    client_states: BTreeMap<ConnectionId, ClientState>,
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
            command_handler_tick: 0,
            potential_clients: Vec::new(),
            client_states: BTreeMap::new(),
        }
    }

    #[inline]
    fn send_debug_text(&self, text: &str) {
        print!("{}", text);
    }

    fn add_new_verified_connection(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        read_data: &[u8],
    ) -> bool {
        let username_len = read_data[0] as usize;
        if let Some(cs) = ClientState::new(&read_data[1..username_len + 1]) {
            // Should always be inserted at the end of the BTree due to the ConnectionId properties
            let cs_ind = self.client_states.len();
            self.client_states.insert(cid.clone(), cs);

            // Send new client a state refresh
            let mut send_data = self.create_refresh_data(cs_ind);
            set_stream_msg_size(&mut send_data);
            let _ = endpoint.main_stream_send(cid, send_data);

            let mut send_data = self.create_new_client_data(cid);
            set_stream_msg_size(&mut send_data);

            // Send all other clients a msg about the new client
            for (ind, (conn_id, _cs)) in self.client_states.iter().enumerate() {
                if ind != cs_ind {
                    let _ = endpoint.main_stream_send(conn_id, send_data.clone());
                }
            }
            true
        } else {
            false
        }
    }

    fn remove_connection_state(&mut self, cid: &ConnectionId) -> bool {
        self.client_states.remove(cid).is_some()
    }

    fn create_refresh_data(&self, verified_index: usize) -> Vec<u8> {
        let mut data = StreamMsgType::ServerStateRefresh.get_stream_msg();
        data.push(self.client_states.len() as u8);
        data.push(verified_index as u8);
        data.push(self.name_len as u8);
        data.extend_from_slice(&self.name[..self.name_len]);

        for cs in self.client_states.values() {
            data.push(cs.user_name_len as u8);
            data.extend_from_slice(&cs.user_name[..cs.user_name_len]);
            data.push(cs.state);
        }

        data.push(0);
        data
    }

    fn create_new_client_data(&self, cid: &ConnectionId) -> Vec<u8> {
        let mut data = StreamMsgType::NewClient.get_stream_msg();

        if let Some(cs) = self.client_states.get(cid) {
            data.push(cs.user_name_len as u8);
            data.extend_from_slice(&cs.user_name[..cs.user_name_len]);
            data.push(cs.state);
        }

        data
    }
}

impl EndpointEventCallbacks for ServerState {
    fn connection_started(&mut self, _endpoint: &mut Endpoint, _cid: &ConnectionId) {
        // Nothing to do until a server gets the first recv data from a potential client
    }

    fn connection_ended(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        reason: ConnectionEndReason,
        _remaining_connections: usize,
    ) -> bool {
        if self.remove_connection_state(cid) {
            match reason {
                ConnectionEndReason::LocalApplication(code) => println!(
                    "Server Connection Ended Reason: {:?}: {:?}",
                    reason,
                    ErrorCode::from_u64(code)
                ),
                ConnectionEndReason::PeerApplication(code) => println!(
                    "Server Connection Ended Reason: {:?}: {:?}",
                    reason,
                    ErrorCode::from_u64(code)
                ),
                _ => println!("Server Connection Ended Reason: {:?}", reason),
            }
            // Temporarily (inefficiently) used for removing of clients
            for (verified_index, (conn_id, _cs)) in self.client_states.iter().enumerate() {
                let mut send_data = self.create_refresh_data(verified_index);
                set_stream_msg_size(&mut send_data);
                let _ = endpoint.main_stream_send(conn_id, send_data);
            }
        }
        false
    }

    fn tick(&mut self, endpoint: &mut Endpoint) -> bool {
        self.command_handler_tick += 1;
        if self.command_handler_tick >= 10 {
            if crossterm::event::poll(std::time::Duration::from_millis(0)).is_ok_and(|v| v) {
                if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                    if key.kind == crossterm::event::KeyEventKind::Press {
                        match key.code {
                            crossterm::event::KeyCode::Char(c) => {
                                let uc = c.to_ascii_uppercase();
                                if uc == 'S' {
                                    for conn_id in self.client_states.keys() {
                                        let _ = endpoint.close_connection(
                                            conn_id,
                                            ErrorCode::ServerClosed as u64,
                                        );
                                    }
                                    return true;
                                }
                            }
                            _ => {
                                // Do Nothing
                            }
                        }
                    }
                }
            }
            self.command_handler_tick = 0;
        }

        false
    }

    fn main_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        read_data: &[u8],
    ) -> Option<usize> {
        if let Some(cs) = self.client_states.get_mut(cid) {
            //self.client_states[vi].cid.update(cid);
            if let Some(_msg_type) = cs.main_recv_type.take() {
                // if self.handle_stream_msg(endpoint, vi, msg_type, read_data) {
                //     Some(MESSAGE_HEADER_SIZE)
                // } else {
                None // Close Connection
                     // }
            } else {
                let new_msg_type = StreamMsgType::from_u8(read_data[0]);
                if new_msg_type.intended_for_server() {
                    cs.main_recv_type = Some(new_msg_type);
                    Some(get_stream_msg_size(read_data))
                } else {
                    None
                }
            }
        } else if let Some(pot_ind) = self
            .potential_clients
            .iter()
            .position(|p_cid| *p_cid == *cid)
        {
            self.potential_clients.remove(pot_ind);
            if self.add_new_verified_connection(endpoint, cid, read_data) {
                Some(MESSAGE_HEADER_SIZE)
            } else {
                None // Close Connection
            }
        } else if read_data.len() == MESSAGE_HEADER_SIZE {
            // Check to see if it's a new valid server
            match StreamMsgType::from_u8(read_data[0]) {
                StreamMsgType::NewClientAnnounce => {
                    self.potential_clients.push(cid.clone());
                    Some(get_stream_msg_size(read_data))
                }
                _ => {
                    None // Close Connection
                }
            }
        } else {
            None // Close Connection
        }
    }
}

struct ClientHandler {
    user_name: String,
    command_handler_tick: u64,
    cid_option: Option<ConnectionId>, // Focus Connection ID
    main_recv_type: Option<StreamMsgType>,
}

impl ClientHandler {
    fn new(user_name: String) -> Self {
        ClientHandler {
            user_name,
            command_handler_tick: 0,
            cid_option: None,
            main_recv_type: None,
        }
    }

    #[inline]
    fn send_debug_text(&self, text: &str) {
        print!("{}", text);
    }

    fn handle_stream_msg(
        &mut self,
        _endpoint: &mut Endpoint,
        _cid: &ConnectionId,
        msg_type: StreamMsgType,
        _read_data: &[u8],
    ) -> bool {
        match msg_type {
            StreamMsgType::ServerStateRefresh => {
                self.send_debug_text("Client Recv Server State Refresh!\n");
            }
            StreamMsgType::NewClient => {
                self.send_debug_text("Client Recv Info about other Client!\n");
            }
            _ => {
                return false;
            }
        }
        true
    }

    fn create_announce_data(&self) -> Vec<u8> {
        let mut data = StreamMsgType::NewClientAnnounce.get_stream_msg();

        let len_pos = data.len();
        data.push(0); // Temp Length push
        let mut char_subslice = [0, 0, 0, 0];
        let mut num_chars = 0;
        for (c_ind, c) in self.user_name.chars().enumerate() {
            if c_ind >= MAX_CHAR_LENGTH {
                break;
            }

            num_chars += c.len_utf8() as u8;
            c.encode_utf8(&mut char_subslice);
            data.extend_from_slice(&char_subslice[..c.len_utf8()]);
        }
        data[len_pos] = num_chars;

        data
    }
}

impl EndpointEventCallbacks for ClientHandler {
    fn connection_started(&mut self, endpoint: &mut Endpoint, cid: &ConnectionId) {
        println!("Announcing Self to Server!");
        let mut send_data = self.create_announce_data();
        set_stream_msg_size(&mut send_data);
        let _ = endpoint.main_stream_send(cid, send_data);
    }

    fn connection_ended(
        &mut self,
        _endpoint: &mut Endpoint,
        cid: &ConnectionId,
        reason: ConnectionEndReason,
        remaining_connections: usize,
    ) -> bool {
        if let Some(my_conn_id) = &self.cid_option {
            if *my_conn_id == *cid {
                self.cid_option = None;
                self.main_recv_type = None;
            }
        }
        match reason {
            ConnectionEndReason::LocalApplication(code) => println!(
                "Client Connection Ended Reason: {:?}: {:?}",
                reason,
                ErrorCode::from_u64(code)
            ),
            ConnectionEndReason::PeerApplication(code) => println!(
                "Client Connection Ended Reason: {:?}: {:?}",
                reason,
                ErrorCode::from_u64(code)
            ),
            _ => println!("Client Connection Ended Reason: {:?}", reason),
        }

        // Stop the Endpoint Handler event loop if there are zero client connections
        remaining_connections == 0
    }

    fn tick(&mut self, endpoint: &mut Endpoint) -> bool {
        self.command_handler_tick += 1;
        if self.command_handler_tick >= 10 {
            if crossterm::event::poll(std::time::Duration::from_millis(0)).is_ok_and(|v| v) {
                if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                    if key.kind == crossterm::event::KeyEventKind::Press {
                        match key.code {
                            crossterm::event::KeyCode::Char(c) => {
                                let uc = c.to_ascii_uppercase();
                                if uc == 'C' {
                                    if let Some(cid) = &self.cid_option {
                                        let _ = endpoint
                                            .close_connection(cid, ErrorCode::ClientClosed as u64);
                                    }
                                    return true;
                                }
                            }
                            _ => {
                                // Do Nothing
                            }
                        }
                    }
                }
            }
            self.command_handler_tick = 0;
        }

        false
    }

    fn main_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        read_data: &[u8],
    ) -> Option<usize> {
        if let Some(my_cid) = &mut self.cid_option {
            if *my_cid == *cid {
                my_cid.update(cid);
                if let Some(msg_type) = self.main_recv_type.take() {
                    if self.handle_stream_msg(endpoint, cid, msg_type, read_data) {
                        Some(MESSAGE_HEADER_SIZE)
                    } else {
                        None // Close Connection
                    }
                } else {
                    let new_msg_type = StreamMsgType::from_u8(read_data[0]);
                    if new_msg_type.intended_for_client() {
                        self.main_recv_type = Some(new_msg_type);
                        Some(get_stream_msg_size(read_data))
                    } else {
                        None // Close Connection
                    }
                }
            } else {
                // Weird state to be in considering logic below...
                None // Close Connection
            }
        } else if read_data.len() == MESSAGE_HEADER_SIZE {
            // Check to see if it's a new valid server
            match StreamMsgType::from_u8(read_data[0]) {
                StreamMsgType::ServerStateRefresh => {
                    self.cid_option = Some(cid.clone());
                    self.main_recv_type = Some(StreamMsgType::ServerStateRefresh);
                    Some(get_stream_msg_size(read_data))
                }
                _ => {
                    None // Close Connection
                }
            }
        } else {
            None // Close Connection
        }
    }
}
