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

// This file could be used as a template for using quic with a different aplication protocol negotiated with ALPN
// The protocol used in this program is called "RealtimeMedia"

const ALPN_NAME: &[u8] = b"RealtimeMedia"; // Application-Layer Protocol Negotiation Name used to define the Quic-Application Protocol used in this program
const SERVER_NAME: &str = "localhost"; // Server "Name" / Domain Name that should ideally be on the server certificate that the client connects to
const CERT_PATH: &str = "security/cert.pem"; // Location of the certificate for the server to use (temporarily used by client to verify server)
const PKEY_PATH: &str = "security/pkey.pem"; // Location of the private key for the server to use

// IPv6 Addresses and Sockets used when sending the client an initial connection addresss
use std::cmp;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
use std::time::Duration;

// Use Inter-Thread Communication Definitions
use crate::communication::{
    ClientCommand, NetworkCommand, NetworkStateConnection, NetworkStateMessage,
    NetworkThreadChannels, ServerCommand, TryRecvError,
};

pub mod rtc;
use rtc::endpoint::Endpoint;
use rtc::SocketAddr;

use self::rtc::RtcQuicHandler;

const MESSAGE_HEADER_SIZE: usize = 3;
const MAX_MESSAGE_SIZE: usize = 65535;

#[derive(Copy, Clone)]
#[repr(u8)]
enum StreamMsgType {
    InvalidType = 0,

    // Server Messages:
    ServerStateRefresh, // NumClientsConnected, ClientIndex, ServerNameLen, ServerName, {ClientXNameLen, ClientXName, ClientXState}... 0
    NewClient,          // ClientNameLen, ClientName, ClientState
    RemoveClient,       // ClientIndex,
    ClientNewState,     // ClientIndex, ClientState
    TransferResponse,   // Transfer ID (1)

    // Not fully working yet:
    MusicTransferResponse, // MusicTransferID (2 bytes)
    MusicIdReady,          // MusicID (2 bytes)

    // Client Messages:
    NewClientAnnounce, // ClientNameLen, ClientName
    NewStateRequest,   // RequestedState
    KeepConnectionAlive,
    TransferRequest, // Data_Len_Size (3), TransferIntention (1)
    TransferData,    // Transfer ID (1), TransferData

    // Not fully working yet:
    MusicTransferRequest, // Packet_Len_Size (8), Data_Len_Size(8), Stereo

    MusicTransferData, // MusicTransferID (2 bytes), MusicTransferData
}

enum StreamMsgIntended {
    Nobody,
    Client,
    Server,
    Anyone, // Might get rid of this type in future
}

impl StreamMsgType {
    fn from_u8(byte: u8) -> Self {
        match byte {
            x if x == StreamMsgType::ServerStateRefresh as u8 => StreamMsgType::ServerStateRefresh,
            x if x == StreamMsgType::NewClient as u8 => StreamMsgType::NewClient,
            x if x == StreamMsgType::RemoveClient as u8 => StreamMsgType::RemoveClient,
            x if x == StreamMsgType::ClientNewState as u8 => StreamMsgType::ClientNewState,
            x if x == StreamMsgType::TransferResponse as u8 => StreamMsgType::TransferResponse,

            x if x == StreamMsgType::MusicTransferResponse as u8 => {
                StreamMsgType::MusicTransferResponse
            }
            x if x == StreamMsgType::MusicIdReady as u8 => StreamMsgType::MusicIdReady,

            x if x == StreamMsgType::NewClientAnnounce as u8 => StreamMsgType::NewClientAnnounce,
            x if x == StreamMsgType::NewStateRequest as u8 => StreamMsgType::NewStateRequest,
            x if x == StreamMsgType::KeepConnectionAlive as u8 => {
                StreamMsgType::KeepConnectionAlive
            }
            x if x == StreamMsgType::TransferRequest as u8 => StreamMsgType::TransferRequest,
            x if x == StreamMsgType::TransferData as u8 => StreamMsgType::TransferData,

            x if x == StreamMsgType::MusicTransferRequest as u8 => {
                StreamMsgType::MusicTransferRequest
            }
            x if x == StreamMsgType::MusicTransferData as u8 => StreamMsgType::MusicTransferData,
            _ => StreamMsgType::InvalidType,
        }
    }

    fn get_value(&self) -> u8 {
        // Requires Copy and Clone derived...?
        *self as u8
    }

    fn intended_for(&self) -> StreamMsgIntended {
        match self {
            StreamMsgType::ServerStateRefresh => StreamMsgIntended::Client,
            StreamMsgType::NewClient => StreamMsgIntended::Client,
            StreamMsgType::RemoveClient => StreamMsgIntended::Client,
            StreamMsgType::ClientNewState => StreamMsgIntended::Client,
            StreamMsgType::TransferResponse => StreamMsgIntended::Client,

            StreamMsgType::MusicTransferResponse => StreamMsgIntended::Client,
            StreamMsgType::MusicIdReady => StreamMsgIntended::Client,

            StreamMsgType::NewClientAnnounce => StreamMsgIntended::Server,
            StreamMsgType::NewStateRequest => StreamMsgIntended::Server,
            StreamMsgType::KeepConnectionAlive => StreamMsgIntended::Server,
            StreamMsgType::TransferRequest => StreamMsgIntended::Server,
            StreamMsgType::TransferData => StreamMsgIntended::Server,

            StreamMsgType::MusicTransferRequest => StreamMsgIntended::Server,

            StreamMsgType::MusicTransferData => StreamMsgIntended::Anyone,

            _ => StreamMsgIntended::Nobody,
        }
    }

    fn get_max_size(&self) -> usize {
        match self {
            StreamMsgType::ServerStateRefresh => MAX_MESSAGE_SIZE,
            StreamMsgType::NewClient => MESSAGE_HEADER_SIZE + 256,
            StreamMsgType::RemoveClient => MESSAGE_HEADER_SIZE + 1,
            StreamMsgType::ClientNewState => MESSAGE_HEADER_SIZE + 2,
            StreamMsgType::TransferResponse => MESSAGE_HEADER_SIZE + 1,

            StreamMsgType::MusicTransferResponse => MESSAGE_HEADER_SIZE + 3,
            StreamMsgType::MusicIdReady => MESSAGE_HEADER_SIZE + 3,

            StreamMsgType::NewClientAnnounce => MESSAGE_HEADER_SIZE + 256,
            StreamMsgType::NewStateRequest => MESSAGE_HEADER_SIZE + 1,
            StreamMsgType::KeepConnectionAlive => MESSAGE_HEADER_SIZE,
            StreamMsgType::TransferRequest => MESSAGE_HEADER_SIZE + 4,
            StreamMsgType::TransferData => MAX_MESSAGE_SIZE,

            StreamMsgType::MusicTransferRequest => MESSAGE_HEADER_SIZE + 3,

            StreamMsgType::MusicTransferData => MAX_MESSAGE_SIZE,

            _ => 3,
        }
    }

    fn get_stream_msg(&self) -> Vec<u8> {
        // Maybe adjust capacity based on get_max_size in future
        Vec::from([self.get_value(), 0, 0])
    }
}

#[inline]
fn set_stream_msg_size(vec_data: &mut Vec<u8>) {
    let num_bytes = usize::to_ne_bytes(vec_data.len() - MESSAGE_HEADER_SIZE);
    vec_data[1] = num_bytes[1];
    vec_data[2] = num_bytes[0];
}

#[inline]
fn get_stream_msg_size(read_data: &[u8]) -> usize {
    usize::from_ne_bytes([read_data[2], read_data[1], 0, 0, 0, 0, 0, 0])
}

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

struct MusicStorage {
    is_stereo: bool,
    packet_len: Vec<u16>,
    packet_data: Vec<u8>,
    remaining_data: Option<(usize, usize)>,
    prev_byte: Option<u8>,
}

impl MusicStorage {
    fn new_blank(is_stereo: bool, packet_len_size: usize, packet_data_size: usize) -> Self {
        MusicStorage {
            is_stereo,
            packet_len: Vec::with_capacity(packet_len_size),
            packet_data: Vec::with_capacity(packet_data_size),
            remaining_data: Some((packet_len_size, packet_data_size)),
            prev_byte: None,
        }
    }

    fn load_in(&mut self, data: &[u8]) -> bool {
        // Return represents more data to go
        if let Some((mut packet_len_remaining, mut packet_data_remaining)) = self.remaining_data {
            let mut data_pos = 0;

            if let Some(pb) = self.prev_byte {
                self.packet_len
                    .push(u16::from_ne_bytes([pb, data[data_pos]]));
                self.prev_byte = None;
                packet_len_remaining -= 1;
                data_pos += 1;
            }

            if packet_len_remaining > 0 {
                let max_packet_lens = (data.len() - data_pos) >> 1;
                let is_leftover =
                    max_packet_lens < packet_len_remaining && ((data.len() - data_pos) & 1) > 0;
                let mut loop_min = cmp::min(max_packet_lens, packet_len_remaining);
                while loop_min > 0 {
                    self.packet_len
                        .push(u16::from_ne_bytes([data[data_pos], data[data_pos + 1]]));
                    data_pos += 2;
                    packet_len_remaining -= 1;
                    loop_min -= 1;
                }
                if is_leftover {
                    self.prev_byte = Some(data[data.len() - 1]);
                }
            }

            if packet_len_remaining == 0 && packet_data_remaining > 0 {
                let data_remaining_len = data.len() - data_pos;
                let slice_min = cmp::min(data_remaining_len, packet_data_remaining);
                self.packet_data
                    .extend_from_slice(&data[data_pos..(data_pos + slice_min)]);
                packet_data_remaining -= slice_min;
            }

            if packet_data_remaining > 0 {
                self.remaining_data = Some((packet_len_remaining, packet_data_remaining));
                false
            } else {
                self.remaining_data = None;
                true
            }
        } else {
            true
        }
    }
}

struct ClientState {
    id: u64,
    probable_index: usize,
    msg_type_recv: Option<StreamMsgType>,
    user_name: [u8; MAX_CHAR_LENGTH * 4],
    user_name_len: usize,
    state: u8, // Bit State [transferingMusic, serverMusicConnected, voiceChatConnected, voiceChatLoopback]
}

impl ClientState {
    fn new(conn_id: u64, probable_index: usize, user_name_bytes: &[u8]) -> Option<Self> {
        let mut cs = ClientState {
            id: conn_id,
            probable_index,
            msg_type_recv: None,
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
    channels: NetworkThreadChannels,
    command_handler_tick: u64,
    potential_clients: Vec<u64>,
    client_states: Vec<ClientState>,
    music_storage: Vec<MusicStorage>,
}

impl ServerState {
    fn new(server_name: String, channels: NetworkThreadChannels) -> Self {
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
            channels,
            command_handler_tick: 0,
            potential_clients: Vec::new(),
            client_states: Vec::new(),
            music_storage: Vec::new(),
        }
    }

    #[inline]
    fn send_debug_text(&self, text: &str) {
        let _ = self.channels.network_debug_send.send(text.to_string());
    }

    #[inline]
    fn find_connection_index(&self, conn_id: u64) -> Option<usize> {
        // Can use binary search later since the client states are ordered by id#
        self.client_states.iter().position(|cs| cs.id == conn_id)
    }

    #[inline]
    fn find_connection_index_with_probable(
        &self,
        conn_id: u64,
        probable_index: usize,
    ) -> Option<usize> {
        if probable_index < self.client_states.len()
            && self.client_states[probable_index].id == conn_id
        {
            Some(probable_index)
        } else {
            self.client_states.iter().position(|cs| cs.id == conn_id)
        }
    }

    fn handle_commands(&self, endpoint: &mut Endpoint, cmd: ServerCommand) {
        match cmd {
            ServerCommand::ConnectionClose(probable_index) => {}
        }
    }

    fn handle_stream_msg(
        &mut self,
        endpoint: &mut Endpoint,
        verified_index: usize,
        msg_type: StreamMsgType,
        read_data: &[u8],
    ) {
        match msg_type {
            StreamMsgType::NewStateRequest => {
                let potential_new_state = read_data[0];
                // In future check if server will allow state change here!
                self.client_states[verified_index].state = potential_new_state;
                let mut send_data = StreamMsgType::ClientNewState.get_stream_msg();
                send_data.push(verified_index as u8);
                send_data.push(self.client_states[verified_index].state);
                set_stream_msg_size(&mut send_data);

                for cs in self.client_states.iter() {
                    let _ = endpoint.send_reliable_stream_data(
                        cs.id,
                        cs.probable_index,
                        send_data.clone(),
                    );
                }

                self.state_change_update(verified_index);
            }
            // StreamMsgType::TransferRequest => {
            //     if self.connections[verified_index].transfer_recv.is_none() {
            //         let transfer_size = usize::from_ne_bytes([
            //             read_data[0],
            //             read_data[1],
            //             read_data[2],
            //             0,
            //             0,
            //             0,
            //             0,
            //             0,
            //         ]);
            //         let media_transfer = RealtimeMediaTransfer {
            //             data: Vec::new(),
            //             size: transfer_size,
            //             bytes_transfered: 0,
            //         };
            //         self.connections[verified_index].transfer_recv = Some(media_transfer);

            //         self.msg_send.refresh_send(StreamMsgType::TransferResponse);
            //         let write_data = self.msg_send.get_data_to_write();
            //         write_data[0] = 33;
            //         self.msg_send.update_data_write(1);
            //         let send_data = self.msg_send.get_data_to_send();
            //         self.connections[verified_index].send_main_stream_data(
            //             &mut self.endpoint,
            //             send_data,
            //             self.current_tick,
            //         );

            //         server_state.client_states[verified_index].state |= 1;
            //         self.msg_send.refresh_send(StreamMsgType::ClientNewState);
            //         let write_data = self.msg_send.get_data_to_write();
            //         write_data[0] = verified_index as u8;
            //         write_data[1] = server_state.client_states[verified_index].state;
            //         self.msg_send.update_data_write(2);
            //         let send_data = self.msg_send.get_data_to_send();
            //         for conn in self.connections.iter_mut() {
            //             conn.send_main_stream_data(
            //                 &mut self.endpoint,
            //                 send_data,
            //                 self.current_tick,
            //             );
            //         }

            //         server_state
            //             .state_change_update(verified_index, &self.channels.network_state_send);
            //     }
            // }
            // StreamMsgType::TransferData => {
            //     if self.connections[verified_index].transfer_recv.is_some() {
            //         let mut done = false;
            //         if self.connections[verified_index].recv_transfer_data() {
            //             // Finished Receiving
            //             if let RealtimeMediaTypeData::Server(server_state) = &mut self.type_data {
            //                 done = true;
            //                 server_state.client_states[verified_index].state &= 0xFE;

            //                 self.msg_send.refresh_send(StreamMsgType::ClientNewState);
            //                 let write_data = self.msg_send.get_data_to_write();
            //                 write_data[0] = verified_index as u8;
            //                 write_data[1] = server_state.client_states[verified_index].state;
            //                 self.msg_send.update_data_write(2);
            //                 let send_data = self.msg_send.get_data_to_send();
            //                 for conn in self.connections.iter_mut() {
            //                     conn.send_main_stream_data(
            //                         &mut self.endpoint,
            //                         send_data,
            //                         self.current_tick,
            //                     );
            //                 }

            //                 server_state.state_change_update(
            //                     verified_index,
            //                     &self.channels.network_state_send,
            //                 );
            //                 self.connections[verified_index].transfer_recv = None;
            //             }
            //         }
            //         if done {
            //             self.send_debug_text("Finished the Transfer!!!\n");
            //         }
            //     } else {
            //         self.send_debug_text("Got Unexpected TransferData Messages!\n");
            //     }
            // }

            // StreamMsgType::MusicTransferRequest => {
            //     let packet_len_size = usize::from_ne_bytes([
            //         read_data[0],
            //         read_data[1],
            //         read_data[2],
            //         read_data[3],
            //         0,
            //         0,
            //         0,
            //         0,
            //     ]);

            //     let data_size = usize::from_ne_bytes([
            //         read_data[8],
            //         read_data[9],
            //         read_data[10],
            //         read_data[11],
            //         0,
            //         0,
            //         0,
            //         0,
            //     ]);

            //     let is_stereo = read_data[16] > 0;

            //     server_state.music_storage.push(MusicStorage::new_blank(
            //         is_stereo,
            //         packet_len_size,
            //         data_size,
            //     ));

            //     server_state
            //         .main_send
            //         .refresh_send(MessageType::MusicTransferResponse);

            //     let write_data = server_state.main_send.get_data_to_write();
            //     let num_bytes = (server_state.music_storage.len() - 1).to_ne_bytes();

            //     write_data[0] = num_bytes[0];
            //     write_data[1] = num_bytes[1];

            //     server_state.main_send.update_data_write(2);

            //     let data_send = server_state.main_send.get_data_to_send();
            //     server_endpoint.send_stream_data(
            //         stream_readable.conn_id,
            //         MAIN_STREAM_ID,
            //         data_send,
            //         false,
            //     );
            // }
            // StreamMsgType::MusicTransferData => {
            //     let music_storage_index =
            //         usize::from_ne_bytes([read_data[0], read_data[1], 0, 0, 0, 0, 0, 0]);

            //     if music_storage_index < server_state.music_storage.len()
            //         && server_state.music_storage[music_storage_index].load_in(&read_data[2..])
            //     {
            //         let write_data = server_state.main_send.get_data_to_write();
            //         write_data[0] = read_data[0];
            //         write_data[1] = read_data[1];

            //         server_state.main_send.update_data_write(2);

            //         let data_send = server_state.main_send.get_data_to_send();
            //         server_endpoint.send_stream_data(
            //             stream_readable.conn_id,
            //             MAIN_STREAM_ID,
            //             data_send,
            //             false,
            //         );
            //     }
            // }
            StreamMsgType::NewClientAnnounce => {
                // If this is reached the client is possibly malicious and should be closed
            }
            _ => {}
        }
    }

    fn add_new_verified_connection(
        &mut self,
        endpoint: &mut Endpoint,
        conn_id: u64,
        verified_index: usize,
        read_data: &[u8],
    ) -> bool {
        let username_len = read_data[0] as usize;
        if let Some(cs) = ClientState::new(conn_id, verified_index, &read_data[1..username_len + 1])
        {
            let cs_ind = self.client_states.len();
            self.client_states.push(cs);

            // Send new client a state refresh
            let mut send_data = self.create_refresh_data(cs_ind);
            set_stream_msg_size(&mut send_data);
            let _ = endpoint.send_reliable_stream_data(conn_id, verified_index, send_data);

            // Send all other clients a msg about the new client
            for (ind, conn) in self.client_states.iter().enumerate() {
                if ind != cs_ind {
                    let mut send_data = self.create_new_client_data(cs_ind);
                    set_stream_msg_size(&mut send_data);
                    let _ =
                        endpoint.send_reliable_stream_data(conn.id, conn.probable_index, send_data);
                }
            }

            self.new_connection_update(cs_ind);

            true
        } else {
            false
        }
    }

    fn remove_connection_state(&mut self, conn_id: u64) -> bool {
        if let Some(verified_index) = self.find_connection_index(conn_id) {
            self.client_states.remove(verified_index);
            true
        } else {
            false
        }
    }

    fn create_refresh_data(&mut self, verified_index: usize) -> Vec<u8> {
        let mut data = StreamMsgType::ServerStateRefresh.get_stream_msg();
        data.push(self.client_states.len() as u8);
        data.push(verified_index as u8);
        data.push(self.name_len as u8);
        data.extend_from_slice(&self.name[..self.name_len]);

        for cs in &self.client_states {
            data.push(cs.user_name_len as u8);
            data.extend_from_slice(&cs.user_name[..cs.user_name_len]);
            data.push(cs.state);
        }

        data.push(0);
        data
    }

    fn create_new_client_data(&self, verified_index: usize) -> Vec<u8> {
        let mut data = StreamMsgType::NewClient.get_stream_msg();
        let cs = &self.client_states[verified_index];

        data.push(cs.user_name_len as u8);
        data.extend_from_slice(&cs.user_name[..cs.user_name_len]);
        data.push(cs.state);

        data
    }

    fn create_state_change_data(&self, verified_index: usize) -> Vec<u8> {
        let mut data = StreamMsgType::ClientNewState.get_stream_msg();
        let cs = &self.client_states[verified_index];
        data.push(verified_index as u8);
        data.push(cs.state);

        data
    }

    fn refresh_update(&self) {
        let mut state_populate = Vec::<NetworkStateConnection>::new();

        for cs in &self.client_states {
            let conn_state = NetworkStateConnection {
                name: u8_to_str(&cs.user_name[..cs.user_name_len]),
                state: cs.state,
            };
            state_populate.push(conn_state);
        }

        let state_update = NetworkStateMessage::ConnectionsRefresh((None, state_populate));
        let _ = self.channels.network_state_send.send(state_update);
    }

    fn new_connection_update(&self, verified_index: usize) {
        let cs = &self.client_states[verified_index];
        let conn_name = u8_to_str(&cs.user_name[..cs.user_name_len]);
        let state_update = NetworkStateMessage::NewConnection((conn_name, cs.state));
        let _ = self.channels.network_state_send.send(state_update);
    }

    fn state_change_update(&self, verified_index: usize) {
        let cs = &self.client_states[verified_index];
        let state_update = NetworkStateMessage::StateChange((verified_index, cs.state));
        let _ = self.channels.network_state_send.send(state_update);
    }
}

impl rtc::RtcQuicEvents for ServerState {
    fn connection_started(&mut self, endpoint: &mut Endpoint, conn_id: u64, verified_index: usize) {
        // Nothing to do until a server gets the first recv data from a potential client
    }

    fn connection_closing(&mut self, endpoint: &mut Endpoint, conn_id: u64) {
        if self.remove_connection_state(conn_id) {
            // Temporarily (inefficiently) used for removing of clients
            for vi in 0..self.client_states.len() {
                let mut send_data = self.create_refresh_data(vi);
                set_stream_msg_size(&mut send_data);
                let _ = endpoint.send_reliable_stream_data(
                    self.client_states[vi].id,
                    self.client_states[vi].probable_index,
                    send_data,
                );
            }
            self.refresh_update();
        }
    }

    fn connection_closed(
        &mut self,
        endpoint: &mut Endpoint,
        conn_id: u64,
        remaining_connections: usize,
    ) -> bool {
        if self.remove_connection_state(conn_id) {
            // Temporarily (inefficiently) used for removing of clients
            for vi in 0..self.client_states.len() {
                let mut send_data = self.create_refresh_data(vi);
                set_stream_msg_size(&mut send_data);
                let _ = endpoint.send_reliable_stream_data(
                    self.client_states[vi].id,
                    self.client_states[vi].probable_index,
                    send_data,
                );
            }
            self.refresh_update();
        }
        false
    }

    fn tick(&mut self, endpoint: &mut Endpoint) -> bool {
        self.command_handler_tick += 1;
        if self.command_handler_tick >= 10 {
            loop {
                match self.channels.command_recv.try_recv() {
                    Err(TryRecvError::Empty) => break,
                    Ok(NetworkCommand::Server(server_cmd)) => {
                        self.handle_commands(endpoint, server_cmd)
                    }
                    Ok(NetworkCommand::Stop(int)) => return true,
                    Err(_) => return true, // Other recv errors
                    Ok(NetworkCommand::Client(_)) => {}
                }
            }
            self.command_handler_tick = 0;
        }

        false
    }

    fn debug_text(&mut self, text: &'static str) {
        self.send_debug_text(text);
    }

    fn reliable_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        conn_id: u64,
        verified_index: usize,
        read_data: &[u8],
    ) -> Option<usize> {
        if let Some(vi) = self.find_connection_index(conn_id) {
            self.client_states[vi].probable_index = verified_index;
            if let Some(msg_type) = self.client_states[vi].msg_type_recv {
                self.handle_stream_msg(endpoint, vi, msg_type, read_data);
                self.client_states[vi].msg_type_recv = None;
                Some(MESSAGE_HEADER_SIZE)
            } else {
                self.client_states[vi].msg_type_recv = Some(StreamMsgType::from_u8(read_data[0]));
                Some(get_stream_msg_size(read_data))
            }
        } else if let Some(pot_ind) = self
            .potential_clients
            .iter()
            .position(|p_cid| *p_cid == conn_id)
        {
            self.potential_clients.remove(pot_ind);
            if self.add_new_verified_connection(endpoint, conn_id, verified_index, read_data) {
                Some(MESSAGE_HEADER_SIZE)
            } else {
                // Close connection here in future
                None
            }
        } else if read_data.len() >= MESSAGE_HEADER_SIZE {
            // Check to see if it's a new valid server
            match StreamMsgType::from_u8(read_data[0]) {
                StreamMsgType::NewClientAnnounce => {
                    self.potential_clients.push(conn_id);
                    Some(get_stream_msg_size(read_data))
                }
                _ => {
                    // Close connection here in future
                    None
                }
            }
        } else {
            Some(MESSAGE_HEADER_SIZE - read_data.len())
        }
    }
}

struct ClientHandler {
    user_name: String,
    channels: NetworkThreadChannels,
    command_handler_tick: u64,
    connection_id: Option<u64>,
    probable_index: usize, // Only valid if connection_id.is_some()
    msg_type_recv: Option<StreamMsgType>,
    //focus_id: u64,
}

impl ClientHandler {
    fn new(user_name: String, channels: NetworkThreadChannels) -> Self {
        ClientHandler {
            user_name,
            channels,
            command_handler_tick: 0,
            connection_id: None,
            probable_index: 0,
            msg_type_recv: None,
            //focus_id: 0,
        }
    }

    #[inline]
    fn send_debug_text(&self, text: &str) {
        let _ = self.channels.network_debug_send.send(text.to_string());
    }

    fn handle_commands(&self, endpoint: &mut Endpoint, cmd: ClientCommand) {
        match cmd {
            ClientCommand::StateChange(new_state_requested) => {
                if let Some(conn_id) = self.connection_id {
                    let mut send_data = StreamMsgType::NewStateRequest.get_stream_msg();
                    send_data.push(new_state_requested);
                    set_stream_msg_size(&mut send_data);
                    let _ =
                        endpoint.send_reliable_stream_data(conn_id, self.probable_index, send_data);
                }
            }
            ClientCommand::ServerConnect(server_address) => {
                let _ = endpoint.add_client_connection(server_address, SERVER_NAME);
            }
            ClientCommand::MusicTransfer(od) => {
                // if !self.connections.is_empty() {
                //     let transfer_data = od.to_bytes();
                //     let transfer_size = transfer_data.len();

                //     let info_string = format!("Data transfer size: {}\n", transfer_size);
                //     self.send_debug_text(info_string.as_str());

                //     let media_transfer = RealtimeMediaTransfer {
                //         data: transfer_data,
                //         size: transfer_size,
                //         bytes_transfered: 0,
                //     };
                //     self.connections[0].transfer_send = Some(media_transfer);

                //     self.msg_send.refresh_send(StreamMsgType::TransferRequest);
                //     let write_data = self.msg_send.get_data_to_write();

                //     let size_in_bytes = transfer_size.to_ne_bytes();
                //     write_data[0] = size_in_bytes[0];
                //     write_data[1] = size_in_bytes[1];
                //     write_data[2] = size_in_bytes[2];
                //     write_data[3] = 1; // Indicating for deletion after fully received

                //     self.msg_send.update_data_write(4);
                //     let send_data = self.msg_send.get_data_to_send();
                //     self.connections[0].send_main_stream_data(
                //         &mut self.endpoint,
                //         send_data,
                //         self.current_tick,
                //     );
                // }

                // if self.connections.len() > 0 {
                //     let (stereo_byte, packet_len_size, data_len_size, od_data) = od.to_data();

                //     self.msg_send
                //         .refresh_send(StreamMsgType::MusicTransferRequest);
                //     let write_data = self.msg_send.get_data_to_write();

                //     write_data[0..8].copy_from_slice(&packet_len_size.to_ne_bytes());
                //     write_data[8..16].copy_from_slice(&data_len_size.to_ne_bytes());

                //     write_data[16] = stereo_byte;

                //     // let num_bytes = od_data.len().to_ne_bytes();
                //     // write_data[17] = num_bytes[0];
                //     // write_data[18] = num_bytes[1];
                //     // write_data[19] = num_bytes[2];

                //     self.msg_send.update_data_write(17);

                //     let send_data = self.msg_send.get_data_to_send();
                //     let conn_id = self.connections[0].id;
                //     self.endpoint
                //         .send_stream_data(conn_id, MAIN_STREAM_ID, send_data, false);

                //     self.connections[0].transfer_send = Some(RealtimeMediaTransfer {
                //         data: od_data,
                //         size: od_data.len(),
                //         bytes_transfered: 0,
                //     });
                // }
            }
        }
    }

    fn handle_limited_commands(&self, endpoint: &mut Endpoint) -> bool {
        loop {
            match self.channels.command_recv.try_recv() {
                Err(TryRecvError::Empty) => break,
                Ok(NetworkCommand::Client(ClientCommand::ServerConnect(server_address))) => {
                    let _ = endpoint.add_client_connection(server_address, SERVER_NAME);
                    return true;
                }
                Ok(NetworkCommand::Stop(int)) => return true,
                Err(_) => return true, // Other recv errors
                Ok(_) => {}
            }
        }
        false
    }

    fn handle_stream_msg(
        &self,
        endpoint: &mut Endpoint,
        msg_type: StreamMsgType,
        read_data: &[u8],
    ) {
        match msg_type {
            StreamMsgType::ServerStateRefresh => {
                // State Refresh
                self.handle_state_refresh(read_data);
            }
            StreamMsgType::NewClient => {
                self.handle_new_client(read_data);
            }
            StreamMsgType::ClientNewState => {
                self.handle_client_new_state(read_data);
            }
            // StreamMsgType::TransferResponse => {
            //     let id_byte = read_data[0];
            //     if self.connections[verified_index].transfer_send.is_some() {
            //         loop {
            //             self.msg_send.refresh_send(StreamMsgType::TransferData);
            //             let write_data = self.msg_send.get_data_to_write();
            //             write_data[0] = id_byte;

            //             if let Some(transfer_media) =
            //                 &mut self.connections[verified_index].transfer_send
            //             {
            //                 let write_len = write_data.len() - 1;
            //                 let remaining_len =
            //                     transfer_media.data.len() - transfer_media.bytes_transfered;
            //                 let min_len = cmp::min(write_len, remaining_len);
            //                 if min_len == 0 {
            //                     break;
            //                 }
            //                 let transfer_end = min_len + transfer_media.bytes_transfered;
            //                 write_data[1..(1 + min_len)].copy_from_slice(
            //                     &transfer_media.data[transfer_media.bytes_transfered..transfer_end],
            //                 );
            //                 transfer_media.bytes_transfered = transfer_end;
            //                 self.msg_send.update_data_write(min_len + 1);
            //                 //let info_string = format!("Data written: {}\n", min_len + 1);
            //                 //self.send_debug_text(info_string.as_str());
            //             }

            //             let send_data = self.msg_send.get_data_to_send();
            //             match self.endpoint.send_stream_data(
            //                 self.connections[verified_index].id,
            //                 MAIN_STREAM_ID,
            //                 send_data,
            //                 false,
            //             ) {
            //                 Ok((i_sends, d_sends)) => {
            //                     self.connections[verified_index].last_activity_tick =
            //                         self.current_tick;
            //                     let info_string = format!("Sends: {} {}\n", i_sends, d_sends);
            //                     self.send_debug_text(info_string.as_str());
            //                 }
            //                 Err(e) => match e {
            //                     EndpointError::StreamSendFilled => {
            //                         self.send_debug_text("Stream Send Filled!\n")
            //                     }
            //                     _ => self.send_debug_text("Generic Stream Send Err!\n"),
            //                 },
            //             }
            //         }

            //         self.send_debug_text("Sent File Transfer!\n");
            //     }
            // }
            // StreamMsgType::MusicTransferResponse => {
            //     if let Some(music_data) = &client_handler.music_tranfer {
            //         let mut music_data_start = 0;
            //         loop {
            //             client_handler
            //                 .main_send
            //                 .refresh_send(MessageType::MusicTransferData);
            //             let write_data = client_handler.main_send.get_data_to_write();

            //             write_data[0] = data_read[0];
            //             write_data[1] = data_read[1];

            //             let write_len = write_data.len() - 2;
            //             let music_len = music_data.len() - music_data_start;

            //             let min_len = cmp::min(write_len, music_len);
            //             write_data[2..min_len].copy_from_slice(
            //                 &music_data[music_data_start..music_data_start + min_len],
            //             );
            //             music_data_start += min_len;

            //             client_handler.main_send.update_data_write(2 + min_len);

            //             let send_data = client_handler.main_send.get_data_to_send();

            //             client_endpoint.send_stream_data(MAIN_STREAM_ID, send_data, false);

            //             if music_data_start >= music_data.len() {
            //                 break;
            //             }
            //         }
            //     }
            // }
            _ => {}
        }
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

    fn handle_state_refresh(&self, read_data: &[u8]) {
        let conn_ind = read_data[1] as usize;

        let mut name_end: usize = (read_data[2] + 3).into();
        let server_name = u8_to_str(&read_data[3..name_end]);
        let name_update = NetworkStateMessage::ServerNameChange(server_name);
        let _ = self.channels.network_state_send.send(name_update);

        let mut state_populate = Vec::<NetworkStateConnection>::new();

        let mut name_len: usize = read_data[name_end].into();
        while name_len != 0 {
            let name_start = name_end + 1;
            name_end = name_len + name_start;
            let client_name = u8_to_str(&read_data[name_start..name_end]);

            let conn_state = NetworkStateConnection {
                name: client_name,
                state: read_data[name_end],
            };

            state_populate.push(conn_state);

            name_end += 1;
            name_len = read_data[name_end].into();
        }

        //client_handler.focus_id

        let state_update =
            NetworkStateMessage::ConnectionsRefresh((Some(conn_ind), state_populate));
        let _ = self.channels.network_state_send.send(state_update);
    }

    fn handle_new_client(&self, read_data: &[u8]) {
        let name_end: usize = (read_data[0] + 1).into();
        let client_name = u8_to_str(&read_data[1..name_end]);
        let new_conn = NetworkStateMessage::NewConnection((client_name, read_data[name_end]));
        let _ = self.channels.network_state_send.send(new_conn);
    }

    fn handle_client_new_state(&self, read_data: &[u8]) {
        let conn_pos = read_data[0] as usize;
        let new_state = read_data[1];

        let new_conn = NetworkStateMessage::StateChange((conn_pos, new_state));
        let _ = self.channels.network_state_send.send(new_conn);
    }
}

impl rtc::RtcQuicEvents for ClientHandler {
    fn connection_started(&mut self, endpoint: &mut Endpoint, conn_id: u64, verified_index: usize) {
        let _ = self
            .channels
            .network_debug_send
            .send("Announcing Self to Server!\n".to_string());
        let mut send_data = self.create_announce_data();
        set_stream_msg_size(&mut send_data);
        let _ = endpoint.send_reliable_stream_data(conn_id, verified_index, send_data);
    }

    fn connection_closing(&mut self, endpoint: &mut Endpoint, conn_id: u64) {
        if let Some(my_conn_id) = self.connection_id {
            if my_conn_id == conn_id {
                self.connection_id = None;
                self.probable_index = 0;
                self.msg_type_recv = None;
            }
        }
    }

    fn connection_closed(
        &mut self,
        endpoint: &mut Endpoint,
        conn_id: u64,
        remaining_connections: usize,
    ) -> bool {
        if let Some(my_conn_id) = self.connection_id {
            if my_conn_id == conn_id {
                self.connection_id = None;
                self.probable_index = 0;
                self.msg_type_recv = None;
            }
        }

        // There might need to be more logic here
        remaining_connections == 0
    }

    fn tick(&mut self, endpoint: &mut Endpoint) -> bool {
        let _ = endpoint.send_out_ping_past_duration(Duration::from_millis(2000));

        self.command_handler_tick += 1;
        if self.command_handler_tick >= 10 {
            loop {
                match self.channels.command_recv.try_recv() {
                    Err(TryRecvError::Empty) => break,
                    Ok(NetworkCommand::Client(client_cmd)) => {
                        self.handle_commands(endpoint, client_cmd);
                    }
                    Ok(NetworkCommand::Stop(int)) => return true,
                    Err(_) => return true, // Other recv errors
                    Ok(NetworkCommand::Server(_)) => {}
                }
            }
            self.command_handler_tick = 0;
        }

        false
    }

    fn debug_text(&mut self, text: &'static str) {
        self.send_debug_text(text);
    }

    fn reliable_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        conn_id: u64,
        verified_index: usize,
        read_data: &[u8],
    ) -> Option<usize> {
        if let Some(my_conn_id) = self.connection_id {
            if my_conn_id == conn_id {
                self.probable_index = verified_index;
                if let Some(msg_type) = self.msg_type_recv {
                    self.handle_stream_msg(endpoint, msg_type, read_data);
                    self.msg_type_recv = None;
                    Some(MESSAGE_HEADER_SIZE)
                } else {
                    self.msg_type_recv = Some(StreamMsgType::from_u8(read_data[0]));
                    Some(get_stream_msg_size(read_data))
                }
            } else {
                // Weird state to be in considering logic below...
                // Close connection here in future
                None
            }
        } else if read_data.len() >= MESSAGE_HEADER_SIZE {
            // Check to see if it's a new valid server
            match StreamMsgType::from_u8(read_data[0]) {
                StreamMsgType::ServerStateRefresh => {
                    self.connection_id = Some(conn_id);
                    self.msg_type_recv = Some(StreamMsgType::ServerStateRefresh);
                    Some(get_stream_msg_size(read_data))
                }
                _ => {
                    // Close connection here in future
                    None
                }
            }
        } else {
            Some(MESSAGE_HEADER_SIZE - read_data.len())
        }
    }
}

pub fn server_thread(
    use_ipv6: Option<bool>,
    port: u16,
    server_name: String,
    channels: NetworkThreadChannels,
) {
    let bind_address = match use_ipv6 {
        Some(ipv6) => match ipv6 {
            true => SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port, 0, 0)),
            false => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)),
        },
        None => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)),
    };

    let server_endpoint =
        match Endpoint::new_server(bind_address, ALPN_NAME, CERT_PATH, PKEY_PATH, 4_194_304) {
            Ok(endpoint) => endpoint,
            Err(err) => {
                let _ = channels
                    .network_debug_send
                    .send("Server Endpoint Creation Error!\n".to_string());
                // Can add more detailed print here later
                return;
            }
        };

    let mut server_state = ServerState::new(server_name, channels);
    server_state.send_debug_text("Starting Server Network!\n");

    let mut rtc_handler = RtcQuicHandler::new(server_endpoint, &mut server_state);

    match rtc_handler.run_event_loop(std::time::Duration::from_millis(5)) {
        Ok(_) => {}
        Err(_) => {
            server_state.send_debug_text("Server Event Loop Error\n");
        }
    }

    // Eventual Friendly Server Cleanup Here

    server_state.send_debug_text("Server Network Thread Exiting\n");
}

pub fn client_thread(
    server_address: SocketAddr,
    user_name: String,
    channels: NetworkThreadChannels,
) {
    let bind_address = match server_address.is_ipv6() {
        true => SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0)),
        false => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)),
    };

    let client_endpoint = match Endpoint::new_client_with_first_connection(
        bind_address,
        ALPN_NAME,
        CERT_PATH,
        server_address,
        SERVER_NAME,
        4_194_304,
    ) {
        Ok(endpoint) => endpoint,
        Err(err) => {
            let _ = channels
                .network_debug_send
                .send("Client Endpoint Creation Error!\n".to_string());
            // Can add more detailed print here later
            return;
        }
    };

    // Not ideal but works for now...
    let command_handler = channels.command_recv.clone();

    let mut client_handler = ClientHandler::new(user_name, channels);
    client_handler.send_debug_text("Starting Client Network!\n");
    let mut rtc_handler = RtcQuicHandler::new(client_endpoint, &mut client_handler);

    loop {
        // If
        match rtc_handler.run_event_loop(std::time::Duration::from_millis(5)) {
            Ok(Some(endpoint)) => {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    let should_quit = loop {
                        match command_handler.try_recv() {
                            Err(TryRecvError::Empty) => break false,
                            Ok(NetworkCommand::Client(ClientCommand::ServerConnect(
                                server_address,
                            ))) => {
                                let _ = endpoint.add_client_connection(server_address, SERVER_NAME);
                                break true;
                            }
                            Ok(NetworkCommand::Stop(int)) => break true,
                            Err(_) => break true, // Other recv errors
                            Ok(_) => {}
                        }
                    };
                    if should_quit {
                        break;
                    }
                }
                if endpoint.get_num_connections() > 0 {
                    break;
                }
            }
            Ok(None) => {
                break;
            }
            Err(_) => {
                break;
            }
        }
    }

    // Eventual Friendly Client Cleanup Here

    client_handler.send_debug_text("Client Network Thread Exiting!\n");
}
