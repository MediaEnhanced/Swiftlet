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
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
use std::time::Duration;

// Use Inter-Thread Communication Definitions
use crate::communication::{
    ClientCommand, NetworkAudioOutputChannels, NetworkAudioPackets, NetworkCommand,
    NetworkStateConnection, NetworkStateMessage, NetworkThreadChannels, ServerCommand,
    TryRecvError,
};

pub mod rtc;
use rtc::endpoint::Endpoint;
use rtc::SocketAddr;

use self::rtc::{RtcQuicEvents, RtcQuicHandler};

const MESSAGE_HEADER_SIZE: usize = 3;
const MAX_MESSAGE_SIZE: usize = 65535;
const BUFFER_SIZE_PER_CONNECTION: usize = 4_194_304; // 4 MiB

#[repr(u8)]
enum StreamMsgType {
    InvalidType = 0,

    // Server Messages:
    ServerStateRefresh, // NumClientsConnected, ClientIndex, ServerNameLen, ServerName, {ClientXNameLen, ClientXName, ClientXState}... 0
    NewClient,          // ClientNameLen, ClientName, ClientState
    RemoveClient,       // ClientIndex,
    ClientNewState,     // ClientIndex, ClientState
    MusicIdReady,       // MusicID (1 byte)
    NextMusicPacket,    // Stereo, Music Packet

    // Client Messages:
    NewClientAnnounce, // ClientNameLen, ClientName
    NewStateRequest,   // RequestedState
    MusicRequest,      // MusicID (1 byte)

    // General Messages:
    TransferRequest,  // Data_Len_Size (3), TransferIntention (1)
    TransferResponse, // Transfer ID (2)
    TransferData,     // Header (Includes Transfer ID instead of Size) TransferData
}

enum IntendedFor {
    Nobody,
    Client,
    Server,
    Anyone,
}

impl StreamMsgType {
    #[inline] // Verbose but compiles down to minimal instructions
    fn from_u8(byte: u8) -> Self {
        match byte {
            x if x == StreamMsgType::ServerStateRefresh as u8 => StreamMsgType::ServerStateRefresh,
            x if x == StreamMsgType::NewClient as u8 => StreamMsgType::NewClient,
            x if x == StreamMsgType::RemoveClient as u8 => StreamMsgType::RemoveClient,
            x if x == StreamMsgType::ClientNewState as u8 => StreamMsgType::ClientNewState,
            x if x == StreamMsgType::MusicIdReady as u8 => StreamMsgType::MusicIdReady,
            x if x == StreamMsgType::NextMusicPacket as u8 => StreamMsgType::NextMusicPacket,

            x if x == StreamMsgType::NewClientAnnounce as u8 => StreamMsgType::NewClientAnnounce,
            x if x == StreamMsgType::NewStateRequest as u8 => StreamMsgType::NewStateRequest,
            x if x == StreamMsgType::MusicRequest as u8 => StreamMsgType::MusicRequest,

            x if x == StreamMsgType::TransferRequest as u8 => StreamMsgType::TransferRequest,
            x if x == StreamMsgType::TransferResponse as u8 => StreamMsgType::TransferResponse,
            x if x == StreamMsgType::TransferData as u8 => StreamMsgType::TransferData,

            _ => StreamMsgType::InvalidType,
        }
    }

    #[inline]
    fn to_u8(&self) -> u8 {
        match self {
            StreamMsgType::ServerStateRefresh => StreamMsgType::ServerStateRefresh as u8,
            StreamMsgType::NewClient => StreamMsgType::NewClient as u8,
            StreamMsgType::RemoveClient => StreamMsgType::RemoveClient as u8,
            StreamMsgType::ClientNewState => StreamMsgType::ClientNewState as u8,
            StreamMsgType::MusicIdReady => StreamMsgType::MusicIdReady as u8,
            StreamMsgType::NextMusicPacket => StreamMsgType::NextMusicPacket as u8,

            StreamMsgType::NewClientAnnounce => StreamMsgType::NewClientAnnounce as u8,
            StreamMsgType::NewStateRequest => StreamMsgType::NewStateRequest as u8,
            StreamMsgType::MusicRequest => StreamMsgType::MusicRequest as u8,

            StreamMsgType::TransferRequest => StreamMsgType::TransferRequest as u8,
            StreamMsgType::TransferResponse => StreamMsgType::TransferResponse as u8,
            StreamMsgType::TransferData => StreamMsgType::TransferData as u8,
            _ => IntendedFor::Nobody as u8,
        }
    }

    #[inline]
    fn intended_for(&self) -> IntendedFor {
        match self {
            StreamMsgType::ServerStateRefresh => IntendedFor::Client,
            StreamMsgType::NewClient => IntendedFor::Client,
            StreamMsgType::RemoveClient => IntendedFor::Client,
            StreamMsgType::ClientNewState => IntendedFor::Client,
            StreamMsgType::MusicIdReady => IntendedFor::Client,
            StreamMsgType::NextMusicPacket => IntendedFor::Client,

            StreamMsgType::NewClientAnnounce => IntendedFor::Server,
            StreamMsgType::NewStateRequest => IntendedFor::Server,
            StreamMsgType::MusicRequest => IntendedFor::Server,

            StreamMsgType::TransferRequest => IntendedFor::Anyone,
            StreamMsgType::TransferResponse => IntendedFor::Anyone,
            StreamMsgType::TransferData => IntendedFor::Anyone,
            _ => IntendedFor::Nobody,
        }
    }

    #[inline]
    fn get_stream_msg(&self) -> Vec<u8> {
        Vec::from([self.to_u8(), 0, 0])
    }
}

#[repr(u8)]
enum TransferIntention {
    Deletion = 0,
    Music,
}

impl TransferIntention {
    #[inline]
    fn from_u8(byte: u8) -> Self {
        match byte {
            x if x == TransferIntention::Music as u8 => TransferIntention::Music,
            _ => TransferIntention::Deletion,
        }
    }

    #[inline]
    fn to_u8(&self) -> u8 {
        match self {
            TransferIntention::Music => TransferIntention::Music as u8,
            _ => TransferIntention::Deletion as u8,
        }
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
}

impl MusicStorage {
    fn new(read_data: &[u8]) -> Self {
        let is_stereo = read_data[0] == 1;

        let mut read_data_ind = 9;
        let packet_len_size = usize::from_ne_bytes(read_data[1..9].try_into().unwrap());
        let mut packet_len = Vec::with_capacity(packet_len_size);
        for i in 0..packet_len_size {
            packet_len.push(u16::from_ne_bytes([
                read_data[read_data_ind],
                read_data[read_data_ind + 1],
            ]));
            read_data_ind += 2;
        }

        let packet_data_size = usize::from_ne_bytes(
            read_data[read_data_ind..read_data_ind + 8]
                .try_into()
                .unwrap(),
        );
        read_data_ind += 8;
        let packet_data = Vec::from(&read_data[read_data_ind..read_data_ind + packet_data_size]);

        MusicStorage {
            is_stereo,
            packet_len,
            packet_data,
        }
    }
}

struct MusicPlayback {
    storage_index: usize,
    tick: u64,
    packet_num: usize,
    data_offset: usize,
    stereo_byte: u8,
}

impl MusicPlayback {
    fn new(index: usize, is_stereo: bool) -> Self {
        MusicPlayback {
            storage_index: index,
            tick: 0,
            packet_num: 0,
            data_offset: 0,
            stereo_byte: is_stereo as u8,
        }
    }
}

struct TransferInfo {
    id: u16,
    target: TransferIntention,
    size: usize,
}

struct ClientState {
    id: u64,
    probable_index: usize,
    msg_type_recv: Option<StreamMsgType>,
    transfers: Vec<TransferInfo>,
    transfer_id_recv: Option<u16>,
    user_name: [u8; MAX_CHAR_LENGTH * 4],
    user_name_len: usize,
    state: u8, // Bit State [fileTransfer, musicServer, connectedVoice, voiceLoopback]
}

impl ClientState {
    fn new(conn_id: u64, probable_index: usize, user_name_bytes: &[u8]) -> Option<Self> {
        let mut cs = ClientState {
            id: conn_id,
            probable_index,
            msg_type_recv: None,
            transfers: Vec::new(),
            transfer_id_recv: None,
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
    next_transfer_id: u16,
    music_storage: Vec<MusicStorage>,
    music_playback: Option<MusicPlayback>,
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
            next_transfer_id: 1,
            music_storage: Vec::new(),
            music_playback: None,
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

    fn update_client_state(&mut self, endpoint: &mut Endpoint, verified_index: usize) {
        let mut send_data = StreamMsgType::ClientNewState.get_stream_msg();
        send_data.push(verified_index as u8);
        send_data.push(self.client_states[verified_index].state);
        set_stream_msg_size(&mut send_data);

        for cs in self.client_states.iter() {
            let _ = endpoint.send_reliable_stream_data(cs.id, cs.probable_index, send_data.clone());
        }

        self.state_change_update(verified_index);
    }

    fn handle_stream_msg(
        &mut self,
        endpoint: &mut Endpoint,
        verified_index: usize,
        msg_type: StreamMsgType,
        read_data: &[u8],
    ) -> bool {
        match msg_type {
            StreamMsgType::NewStateRequest => {
                let mut potential_new_state = read_data[0];
                if (potential_new_state & 2) > 0 {
                    if !self.music_storage.is_empty() {
                        if self.music_playback.is_none() {
                            self.music_playback =
                                Some(MusicPlayback::new(0, self.music_storage[0].is_stereo));
                        }
                    } else {
                        potential_new_state &= 0xFD;
                    }
                }

                // In future check if server will allow state change here!
                self.client_states[verified_index].state = potential_new_state;
                self.update_client_state(endpoint, verified_index);
            }
            StreamMsgType::TransferRequest => {
                let transfer_size =
                    usize::from_ne_bytes([read_data[0], read_data[1], read_data[2], 0, 0, 0, 0, 0]);
                let info_string = format!("Data transfer request: {}\n", transfer_size);
                self.send_debug_text(info_string.as_str());
                if transfer_size <= BUFFER_SIZE_PER_CONNECTION {
                    // More Checking before acception in future

                    let trans_id_bytes = u16::to_ne_bytes(self.next_transfer_id);
                    let trans_info = TransferInfo {
                        id: self.next_transfer_id,
                        target: TransferIntention::from_u8(read_data[3]),
                        size: transfer_size,
                    };
                    self.client_states[verified_index]
                        .transfers
                        .push(trans_info);

                    self.next_transfer_id += 1; // Future rollover stuff or different scheme here

                    let mut send_data = StreamMsgType::TransferResponse.get_stream_msg();
                    send_data.push(trans_id_bytes[0]);
                    send_data.push(trans_id_bytes[1]);
                    set_stream_msg_size(&mut send_data);

                    let _ = endpoint.send_reliable_stream_data(
                        self.client_states[verified_index].id,
                        self.client_states[verified_index].probable_index,
                        send_data,
                    );
                }
            }
            StreamMsgType::TransferData => {
                if let Some(transfer_id) =
                    self.client_states[verified_index].transfer_id_recv.take()
                {
                    if let Some(transfer_ind) = self.client_states[verified_index]
                        .transfers
                        .iter()
                        .position(|transfers| transfers.id == transfer_id)
                    {
                        self.client_states[verified_index].state &= 0xFE;
                        self.update_client_state(endpoint, verified_index);

                        match self.client_states[verified_index].transfers[transfer_ind].target {
                            TransferIntention::Music => {
                                self.music_storage.push(MusicStorage::new(read_data));
                                let mut send_data = StreamMsgType::MusicIdReady.get_stream_msg();
                                send_data.push(self.music_storage.len() as u8);
                                set_stream_msg_size(&mut send_data);

                                for cs in self.client_states.iter() {
                                    let _ = endpoint.send_reliable_stream_data(
                                        cs.id,
                                        cs.probable_index,
                                        send_data.clone(),
                                    );
                                }
                            }
                            _ => {
                                // Deletion... so do nothing
                            }
                        }
                    }
                }
            }
            _ => {
                return false;
            }
        }
        true
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
        if let Some(playback) = &mut self.music_playback {
            playback.tick += 1; // 4 ticks should be the 20ms music currently hidden requirement
            if playback.tick >= 4 {
                let mut send_data = StreamMsgType::NextMusicPacket.get_stream_msg();
                send_data.push(playback.stereo_byte);

                let len =
                    self.music_storage[playback.storage_index].packet_len[playback.packet_num];
                let next_offset = playback.data_offset + (len as usize);
                send_data.extend_from_slice(
                    &self.music_storage[playback.storage_index].packet_data
                        [playback.data_offset..next_offset],
                );
                playback.data_offset = next_offset;
                playback.packet_num += 1;
                if playback.packet_num
                    >= self.music_storage[playback.storage_index].packet_len.len()
                {
                    playback.packet_num = 0;
                    playback.data_offset = 0;
                }
                playback.tick = 0;

                set_stream_msg_size(&mut send_data);

                let mut num_listeners = 0;
                for cs in self.client_states.iter() {
                    if (cs.state & 2) > 0 {
                        let _ = endpoint.send_reliable_stream_data(
                            cs.id,
                            cs.probable_index,
                            send_data.clone(), // Makes copies here which isn't ideal
                                               // Especially one more than number of sends
                        );
                        num_listeners += 1;
                    }
                }
                if num_listeners == 0 {
                    self.music_playback = None;
                }
            }
        }

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
            if let Some(msg_type) = self.client_states[vi].msg_type_recv.take() {
                if self.handle_stream_msg(endpoint, vi, msg_type, read_data) {
                    Some(MESSAGE_HEADER_SIZE)
                } else {
                    None // Close Connection
                }
            } else {
                let new_msg_type = StreamMsgType::from_u8(read_data[0]);
                match new_msg_type.intended_for() {
                    IntendedFor::Server => {
                        self.client_states[vi].msg_type_recv = Some(new_msg_type);
                        Some(get_stream_msg_size(read_data))
                    }
                    IntendedFor::Anyone => {
                        match new_msg_type {
                            StreamMsgType::TransferData => {
                                let trans_id = u16::from_ne_bytes([read_data[1], read_data[2]]);
                                let trans_size_opt = self.client_states[vi]
                                    .transfers
                                    .iter()
                                    .find(|ti| ti.id == trans_id)
                                    .map(|trans_info| trans_info.size);

                                if let Some(trans_size) = trans_size_opt {
                                    self.client_states[vi].msg_type_recv = Some(new_msg_type);
                                    self.client_states[vi].transfer_id_recv = Some(trans_id);

                                    self.client_states[vi].state |= 0x01;
                                    self.update_client_state(endpoint, vi);

                                    Some(trans_size)
                                } else {
                                    None // Close Connection
                                }
                            }
                            _ => {
                                self.client_states[vi].msg_type_recv = Some(new_msg_type);
                                Some(get_stream_msg_size(read_data))
                            }
                        }
                    }
                    _ => {
                        None // Close Connection
                    }
                }
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
                None // Close Connection
            }
        } else if read_data.len() >= MESSAGE_HEADER_SIZE {
            // Check to see if it's a new valid server
            match StreamMsgType::from_u8(read_data[0]) {
                StreamMsgType::NewClientAnnounce => {
                    self.potential_clients.push(conn_id);
                    Some(get_stream_msg_size(read_data))
                }
                _ => {
                    None // Close Connection
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
    connection_id: Option<u64>, // Focus Connection ID
    probable_index: usize,      // Only valid if connection_id.is_some()
    msg_type_recv: Option<StreamMsgType>,
    transfer_data: Option<Vec<u8>>,
    audio_channels: NetworkAudioOutputChannels,
}

impl ClientHandler {
    fn new(
        user_name: String,
        channels: NetworkThreadChannels,
        audio_channels: NetworkAudioOutputChannels,
    ) -> Self {
        ClientHandler {
            user_name,
            channels,
            command_handler_tick: 0,
            connection_id: None,
            probable_index: 0,
            msg_type_recv: None,
            transfer_data: None,
            audio_channels,
        }
    }

    #[inline]
    fn send_debug_text(&self, text: &str) {
        let _ = self.channels.network_debug_send.send(text.to_string());
    }

    fn handle_commands(&mut self, endpoint: &mut Endpoint, cmd: ClientCommand) {
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
                if let Some(conn_id) = self.connection_id {
                    if self.transfer_data.is_none() {
                        let mut transfer_data = StreamMsgType::TransferData.get_stream_msg();
                        od.add_to_vec(&mut transfer_data);
                        let size_in_bytes =
                            (transfer_data.len() - MESSAGE_HEADER_SIZE).to_ne_bytes();

                        let mut send_data = StreamMsgType::TransferRequest.get_stream_msg();
                        send_data.push(size_in_bytes[0]);
                        send_data.push(size_in_bytes[1]);
                        send_data.push(size_in_bytes[2]);
                        send_data.push(TransferIntention::Music as u8);

                        self.transfer_data = Some(transfer_data);

                        set_stream_msg_size(&mut send_data);
                        let _ = endpoint.send_reliable_stream_data(
                            conn_id,
                            self.probable_index,
                            send_data,
                        );
                    }
                }
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
        &mut self,
        endpoint: &mut Endpoint,
        conn_id: u64,
        msg_type: StreamMsgType,
        read_data: &[u8],
    ) -> bool {
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
            StreamMsgType::TransferResponse => {
                if let Some(mut t_data) = self.transfer_data.take() {
                    //self.send_debug_text("Got Here\n");
                    t_data[1] = read_data[0];
                    t_data[2] = read_data[1];
                    let _ =
                        endpoint.send_reliable_stream_data(conn_id, self.probable_index, t_data);
                }
            }
            StreamMsgType::MusicIdReady => {
                self.debug_text("Music ID is ready!\n");
            }
            StreamMsgType::NextMusicPacket => {
                //self.debug_text("Music Packet Came In!\n");
                let vec_data = Vec::from(read_data);
                let _ = self
                    .audio_channels
                    .packet_send
                    .send(NetworkAudioPackets::MusicPacket((1, vec_data)));
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

        if (new_state & 2) == 0 {
            let _ = self
                .audio_channels
                .packet_send
                .send(NetworkAudioPackets::MusicStop(1));
        }

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
                    Ok(NetworkCommand::Stop(int)) => {
                        if let Some(conn_id) = self.connection_id {
                            let _ = endpoint.close_connection(conn_id, self.probable_index, 8);
                        }
                        return true;
                    }
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
                if let Some(msg_type) = self.msg_type_recv.take() {
                    if self.handle_stream_msg(endpoint, conn_id, msg_type, read_data) {
                        Some(MESSAGE_HEADER_SIZE)
                    } else {
                        None // Close Connection
                    }
                } else {
                    let new_msg_type = StreamMsgType::from_u8(read_data[0]);
                    match new_msg_type.intended_for() {
                        IntendedFor::Client => {
                            self.msg_type_recv = Some(new_msg_type);
                            Some(get_stream_msg_size(read_data))
                        }
                        IntendedFor::Anyone => {
                            self.msg_type_recv = Some(new_msg_type);
                            Some(get_stream_msg_size(read_data))
                        }
                        _ => {
                            None // Close Connection
                        }
                    }
                }
            } else {
                // Weird state to be in considering logic below...
                None // Close Connection
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
                    None // Close Connection
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

    let server_endpoint = match Endpoint::new_server(
        bind_address,
        ALPN_NAME,
        CERT_PATH,
        PKEY_PATH,
        BUFFER_SIZE_PER_CONNECTION,
    ) {
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

    let mut rtc_handler = RtcQuicHandler::new(
        server_endpoint,
        &mut server_state,
        BUFFER_SIZE_PER_CONNECTION,
    );

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
    network_audio_out_channels: NetworkAudioOutputChannels,
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
        BUFFER_SIZE_PER_CONNECTION,
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

    let mut client_handler = ClientHandler::new(user_name, channels, network_audio_out_channels);
    client_handler.send_debug_text("Starting Client Network!\n");
    let mut rtc_handler = RtcQuicHandler::new(
        client_endpoint,
        &mut client_handler,
        BUFFER_SIZE_PER_CONNECTION,
    );

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
