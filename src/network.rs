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
// The protocol used in this program is called "swiftlet"

const ALPN_NAME: &[u8] = b"swiftlet"; // Application-Layer Protocol Negotiation Name used to define the Quic-Application Protocol used in this program
const SERVER_NAME: &str = "localhost"; // Server "Name" / Domain Name that should ideally be on the server certificate that the client connects to
const CERT_PATH: &str = "security/cert.pem"; // Location of the certificate for the server to use (temporarily used by client to verify server)
const PKEY_PATH: &str = "security/pkey.pem"; // Location of the private key for the server to use

// IPv6 Addresses and Sockets used when sending the client an initial connection addresss
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
#[cfg(feature = "client")]
use std::time::Duration;

// Use Inter-Thread Communication Definitions
#[cfg(feature = "client")]
use crate::communication::{ClientCommand, NetworkAudioOutputChannels, NetworkAudioPackets};
use crate::communication::{
    NetworkCommand, NetworkStateConnection, NetworkStateMessage, NetworkThreadChannels,
    ServerCommand, TryRecvError,
};

// Use quic sub-library for internet communications
use swiftlet_quic::{
    endpoint::{Config, ConnectionEndReason, ConnectionId, Endpoint, SocketAddr},
    EndpointEventCallbacks, EndpointHandler,
};

const MESSAGE_HEADER_SIZE: usize = 3;
const MAX_MESSAGE_SIZE: usize = 65535;
const BUFFER_SIZE_PER_CONNECTION: usize = 4_194_304; // 4 MiB

// All stream message data (application protocol information) is always in little endian form
#[repr(u8)]
enum StreamMsgType {
    InvalidType = 0, // Enforce that it is zero

    // Server Messages:
    ServerStateRefresh, // NumClientsConnected, ClientIndex, ServerNameLen, ServerName, {ClientXNameLen, ClientXName, ClientXState}... 0
    NewClient,          // ClientNameLen, ClientName, ClientState
    RemoveClient,       // ClientIndex,
    ClientNewState,     // ClientIndex, ClientState
    MusicIdReady,       // MusicID (1 byte)
    NextMusicPacket,    // Stereo, Music Packet

    // General Messages:
    TransferRequest,  // Data_Len_Size (3), TransferIntention (1)
    TransferResponse, // Transfer ID (2)
    TransferData,     // Header (Includes Transfer ID instead of Size) TransferData

    // Client Messages:
    NewClientAnnounce, // ClientNameLen, ClientName
    NewStateRequest,   // RequestedState
    MusicRequest,      // MusicID (1 byte)
}

impl StreamMsgType {
    #[inline] // Verbose but compiles down to minimal instructions
    fn from_u8(byte: u8) -> Self {
        match byte {
            x if x == Self::ServerStateRefresh as u8 => Self::ServerStateRefresh,
            x if x == Self::NewClient as u8 => Self::NewClient,
            x if x == Self::RemoveClient as u8 => Self::RemoveClient,
            x if x == Self::ClientNewState as u8 => Self::ClientNewState,
            x if x == Self::MusicIdReady as u8 => Self::MusicIdReady,
            x if x == Self::NextMusicPacket as u8 => Self::NextMusicPacket,

            x if x == Self::TransferRequest as u8 => Self::TransferRequest,
            x if x == Self::TransferResponse as u8 => Self::TransferResponse,
            x if x == Self::TransferData as u8 => Self::TransferData,

            x if x == Self::NewClientAnnounce as u8 => Self::NewClientAnnounce,
            x if x == Self::NewStateRequest as u8 => Self::NewStateRequest,
            x if x == Self::MusicRequest as u8 => Self::MusicRequest,

            _ => Self::InvalidType,
        }
    }

    #[inline]
    fn from_header(header: &[u8]) -> Option<(Self, u16)> {
        if header.len() == MESSAGE_HEADER_SIZE {
            Some((
                Self::from_u8(header[0]),
                u16::from_le_bytes([header[1], header[2]]),
            ))
        } else {
            None
        }
    }

    #[inline]
    fn to_u8(&self) -> u8 {
        match self {
            Self::ServerStateRefresh => Self::ServerStateRefresh as u8,
            Self::NewClient => Self::NewClient as u8,
            Self::RemoveClient => Self::RemoveClient as u8,
            Self::ClientNewState => Self::ClientNewState as u8,
            Self::MusicIdReady => Self::MusicIdReady as u8,
            Self::NextMusicPacket => Self::NextMusicPacket as u8,

            Self::TransferRequest => Self::TransferRequest as u8,
            Self::TransferResponse => Self::TransferResponse as u8,
            Self::TransferData => Self::TransferData as u8,

            Self::NewClientAnnounce => Self::NewClientAnnounce as u8,
            Self::NewStateRequest => Self::NewStateRequest as u8,
            Self::MusicRequest => Self::MusicRequest as u8,

            _ => Self::InvalidType as u8,
        }
    }

    #[inline]
    fn intended_for_client(&self) -> bool {
        matches!(
            self,
            Self::ServerStateRefresh
                | Self::NewClient
                | Self::RemoveClient
                | Self::ClientNewState
                | Self::MusicIdReady
                | Self::NextMusicPacket
                | Self::TransferRequest
                | Self::TransferResponse
                | Self::TransferData
        )
    }

    #[inline]
    fn intended_for_server(&self) -> bool {
        matches!(
            self,
            Self::TransferRequest
                | Self::TransferResponse
                | Self::TransferData
                | Self::NewClientAnnounce
                | Self::NewStateRequest
                | Self::MusicRequest
        )
    }

    // Optimized enough for compiler...?
    #[inline]
    fn get_send_data_vec(&self, body_capacity: Option<usize>) -> Vec<u8> {
        if let Some(body_size) = body_capacity {
            let mut send_data = Vec::with_capacity(body_size + MESSAGE_HEADER_SIZE);
            send_data.push(self.to_u8());
            send_data.push(0);
            send_data.push(0);
            send_data
        } else {
            Vec::from([self.to_u8(), 0, 0])
        }
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
fn set_stream_msg_size(vec_data: &mut [u8]) {
    let num_bytes = usize::to_le_bytes(vec_data.len() - MESSAGE_HEADER_SIZE);
    vec_data[1] = num_bytes[0];
    vec_data[2] = num_bytes[1];
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
            packet_len.push(u16::from_le_bytes([
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
    cid: ConnectionId,
    main_recv_type: Option<StreamMsgType>,
    bkgd_recv_type: Option<StreamMsgType>,
    transfers: Vec<TransferInfo>,
    transfer_id_recv: Option<u16>,
    user_name: [u8; MAX_CHAR_LENGTH * 4],
    user_name_len: usize,
    state: u8, // Bit State [fileTransfer, musicServer, connectedVoice, voiceLoopback]
}

impl ClientState {
    fn new(cid: ConnectionId, user_name_bytes: &[u8]) -> Option<Self> {
        let mut cs = ClientState {
            cid,
            main_recv_type: None,
            bkgd_recv_type: None,
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
    potential_clients: Vec<ConnectionId>,
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
    fn find_connection_index_from_cid(&self, cid: &ConnectionId) -> Option<usize> {
        // Can use binary search later since the client states are ordered by id#
        self.client_states.iter().position(|cs| cs.cid == *cid)
    }

    fn handle_commands(&self, endpoint: &mut Endpoint, cmd: ServerCommand) {
        match cmd {
            ServerCommand::ConnectionClose(probable_index) => {}
        }
    }

    fn update_client_state(&mut self, endpoint: &mut Endpoint, verified_index: usize) {
        let mut send_data = StreamMsgType::ClientNewState.get_send_data_vec(Some(2));
        send_data.push(verified_index as u8);
        send_data.push(self.client_states[verified_index].state);
        set_stream_msg_size(&mut send_data);

        for cs in self.client_states.iter() {
            let _ = endpoint.main_stream_send(&cs.cid, send_data.clone());
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

                    let trans_id_bytes = u16::to_le_bytes(self.next_transfer_id);
                    let trans_info = TransferInfo {
                        id: self.next_transfer_id,
                        target: TransferIntention::from_u8(read_data[3]),
                        size: transfer_size,
                    };
                    self.client_states[verified_index]
                        .transfers
                        .push(trans_info);

                    self.next_transfer_id += 1; // Future rollover stuff or different scheme here

                    let mut send_data = StreamMsgType::TransferResponse.get_send_data_vec(Some(2));
                    send_data.push(trans_id_bytes[0]);
                    send_data.push(trans_id_bytes[1]);
                    set_stream_msg_size(&mut send_data);

                    let _ = endpoint
                        .main_stream_send(&self.client_states[verified_index].cid, send_data);
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
                                let mut send_data =
                                    StreamMsgType::MusicIdReady.get_send_data_vec(None);
                                send_data.push(self.music_storage.len() as u8);
                                set_stream_msg_size(&mut send_data);

                                for cs in self.client_states.iter() {
                                    let _ = endpoint.main_stream_send(&cs.cid, send_data.clone());
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
        cid: &ConnectionId,
        read_data: &[u8],
    ) -> bool {
        let username_len = read_data[0] as usize;
        if let Some(cs) = ClientState::new(*cid, &read_data[1..username_len + 1]) {
            let cs_ind = self.client_states.len();
            self.client_states.push(cs);

            // Send new client a state refresh
            let mut send_data = self.create_refresh_data(cs_ind);
            set_stream_msg_size(&mut send_data);
            let _ = endpoint.main_stream_send(cid, send_data);

            // Send all other clients a msg about the new client
            for (ind, conn) in self.client_states.iter().enumerate() {
                if ind != cs_ind {
                    let mut send_data = self.create_new_client_data(cs_ind);
                    set_stream_msg_size(&mut send_data);
                    let _ = endpoint.main_stream_send(&conn.cid, send_data);
                }
            }

            self.new_connection_update(cs_ind);

            true
        } else {
            false
        }
    }

    fn remove_connection_state(&mut self, cid: &ConnectionId) -> bool {
        if let Some(verified_index) = self.find_connection_index_from_cid(cid) {
            self.client_states.remove(verified_index);
            true
        } else {
            false
        }
    }

    fn create_refresh_data(&mut self, verified_index: usize) -> Vec<u8> {
        let mut data = StreamMsgType::ServerStateRefresh.get_send_data_vec(None);
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
        let mut data = StreamMsgType::NewClient.get_send_data_vec(None);
        let cs = &self.client_states[verified_index];

        data.push(cs.user_name_len as u8);
        data.extend_from_slice(&cs.user_name[..cs.user_name_len]);
        data.push(cs.state);

        data
    }

    fn create_state_change_data(&self, verified_index: usize) -> Vec<u8> {
        let mut data = StreamMsgType::ClientNewState.get_send_data_vec(Some(2));
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

impl EndpointEventCallbacks for ServerState {
    fn connection_started(&mut self, endpoint: &mut Endpoint, cid: &ConnectionId) {
        // Nothing to do until a server gets the first recv data from a potential client
    }

    fn connection_ended(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        reason: ConnectionEndReason,
        remaining_connections: usize,
    ) -> bool {
        if self.remove_connection_state(cid) {
            let ended_reason = format!("Server Connection Ended Reason: {:?}\n", reason);
            let _ = self.channels.network_debug_send.send(ended_reason);

            // Temporarily (inefficiently) used for removing of clients
            for vi in 0..self.client_states.len() {
                let mut send_data = self.create_refresh_data(vi);
                set_stream_msg_size(&mut send_data);
                let _ = endpoint.main_stream_send(&self.client_states[vi].cid, send_data);
            }
            self.refresh_update();
        }
        false
    }

    fn connection_ending_warning(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        reason: ConnectionEndReason,
    ) {
        let ending_reason = format!("Server Connection Ending Reason: {:?}\n", reason);
        let _ = self.channels.network_debug_send.send(ending_reason);
    }

    fn tick(&mut self, endpoint: &mut Endpoint) -> bool {
        if let Some(playback) = &mut self.music_playback {
            playback.tick += 1; // 4 ticks should be the 20ms music currently hidden requirement
            if playback.tick >= 4 {
                //let mut send_data = StreamMsgType::NextMusicPacket.get_send_data_vec(None);
                let mut send_data = Vec::new();
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

                //set_stream_msg_size(&mut send_data);

                let mut num_listeners = 0;
                for cs in self.client_states.iter() {
                    if (cs.state & 2) > 0 {
                        //let _ = endpoint.main_stream_send(
                        let _ = endpoint.rt_stream_send(
                            &cs.cid,
                            //send_data.clone(), // Makes copies here which isn't ideal (especially one more than number of sends)
                            Some(send_data.clone()),
                            true,
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
                    Ok(NetworkCommand::Stop(int)) => {
                        for cs in &self.client_states {
                            let _ = endpoint.close_connection(&cs.cid, 4);
                        }
                        return true;
                    }
                    Err(_) => return true, // Other recv errors
                    #[cfg(feature = "client")]
                    Ok(NetworkCommand::Client(_)) => {}
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
        if let Some(vi) = self.find_connection_index_from_cid(cid) {
            if let Some(msg_type) = self.client_states[vi].main_recv_type.take() {
                if self.handle_stream_msg(endpoint, vi, msg_type, read_data) {
                    Some(MESSAGE_HEADER_SIZE)
                } else {
                    None // Close Connection
                }
            } else if let Some((new_msg_type, size)) = StreamMsgType::from_header(read_data) {
                if new_msg_type.intended_for_server() {
                    match new_msg_type {
                        StreamMsgType::TransferData => {
                            let trans_id = size;
                            let trans_size_opt = self.client_states[vi]
                                .transfers
                                .iter()
                                .find(|ti| ti.id == trans_id)
                                .map(|trans_info| trans_info.size);

                            if let Some(trans_size) = trans_size_opt {
                                self.client_states[vi].main_recv_type = Some(new_msg_type);
                                self.client_states[vi].transfer_id_recv = Some(trans_id);

                                self.client_states[vi].state |= 0x01;
                                self.update_client_state(endpoint, vi);

                                Some(trans_size)
                            } else {
                                None // Close Connection
                            }
                        }
                        _ => {
                            self.client_states[vi].main_recv_type = Some(new_msg_type);
                            Some(size as usize)
                        }
                    }
                } else {
                    None // Not intended for server
                }
            } else {
                None // Header invalid
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
        } else if let Some((StreamMsgType::NewClientAnnounce, size)) =
            StreamMsgType::from_header(read_data)
        {
            self.potential_clients.push(*cid);
            Some(size as usize)
        } else {
            None // Close Connection
        }
    }

    fn background_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        read_data: &[u8],
    ) -> Option<usize> {
        if let Some(vi) = self.find_connection_index_from_cid(cid) {
            if let Some(msg_type) = self.client_states[vi].bkgd_recv_type.take() {
                if self.handle_stream_msg(endpoint, vi, msg_type, read_data) {
                    Some(MESSAGE_HEADER_SIZE)
                } else {
                    None // Close Connection
                }
            } else if let Some((new_msg_type, size)) = StreamMsgType::from_header(read_data) {
                if new_msg_type.intended_for_server() {
                    match new_msg_type {
                        StreamMsgType::TransferData => {
                            let trans_id = size;
                            let trans_size_opt = self.client_states[vi]
                                .transfers
                                .iter()
                                .find(|ti| ti.id == trans_id)
                                .map(|trans_info| trans_info.size);

                            if let Some(trans_size) = trans_size_opt {
                                self.client_states[vi].bkgd_recv_type = Some(new_msg_type);
                                self.client_states[vi].transfer_id_recv = Some(trans_id);

                                self.client_states[vi].state |= 0x01;
                                self.update_client_state(endpoint, vi);

                                Some(trans_size)
                            } else {
                                None // Close Connection
                            }
                        }
                        _ => {
                            self.client_states[vi].bkgd_recv_type = Some(new_msg_type);
                            Some(size as usize)
                        }
                    }
                } else {
                    None // Not intended for server
                }
            } else {
                None // Header invalid
            }
        } else {
            None // Close Connection
        }
    }
}

#[cfg(feature = "client")]
struct ClientHandler {
    user_name: String,
    channels: NetworkThreadChannels,
    command_handler_tick: u64,
    cid_option: Option<ConnectionId>, // Focus Connection ID
    main_recv_type: Option<StreamMsgType>,
    background_recv_type: Option<StreamMsgType>,
    transfer_data: Option<Vec<u8>>,
    audio_channels: NetworkAudioOutputChannels,
}

#[cfg(feature = "client")]
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
            cid_option: None,
            main_recv_type: None,
            background_recv_type: None,
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
                if let Some(cid) = &self.cid_option {
                    let mut send_data = StreamMsgType::NewStateRequest.get_send_data_vec(Some(1));
                    send_data.push(new_state_requested);
                    set_stream_msg_size(&mut send_data);
                    let _ = endpoint.main_stream_send(cid, send_data);
                }
            }
            ClientCommand::ServerConnect(server_address) => {
                let _ = endpoint.add_client_connection(server_address, SERVER_NAME);
            }
            ClientCommand::MusicTransfer(od) => {
                if let Some(cid) = &self.cid_option {
                    if self.transfer_data.is_none() {
                        let mut transfer_data = StreamMsgType::TransferData.get_send_data_vec(None);
                        od.add_to_vec(&mut transfer_data);
                        let size_in_bytes =
                            (transfer_data.len() - MESSAGE_HEADER_SIZE).to_ne_bytes();

                        let mut send_data = StreamMsgType::TransferRequest.get_send_data_vec(None);
                        send_data.push(size_in_bytes[0]);
                        send_data.push(size_in_bytes[1]);
                        send_data.push(size_in_bytes[2]);
                        send_data.push(TransferIntention::Music as u8);

                        self.transfer_data = Some(transfer_data);

                        set_stream_msg_size(&mut send_data);
                        let _ = endpoint.main_stream_send(cid, send_data);
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
        cid: &ConnectionId,
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
                    let _ = endpoint.main_stream_send(cid, t_data);
                }
            }
            StreamMsgType::MusicIdReady => {
                self.send_debug_text("Music ID is ready!\n");
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
        let mut data = StreamMsgType::NewClientAnnounce.get_send_data_vec(None);

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

#[cfg(feature = "client")]
impl EndpointEventCallbacks for ClientHandler {
    fn connection_started(&mut self, endpoint: &mut Endpoint, cid: &ConnectionId) {
        let _ = self
            .channels
            .network_debug_send
            .send("Announcing Self to Server!\n".to_string());
        let mut send_data = self.create_announce_data();
        set_stream_msg_size(&mut send_data);
        let _ = endpoint.main_stream_send(cid, send_data);
    }

    fn connection_ended(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        reason: ConnectionEndReason,
        remaining_connections: usize,
    ) -> bool {
        if let Some(my_conn_id) = &self.cid_option {
            if *my_conn_id == *cid {
                self.cid_option = None;
                self.main_recv_type = None;
                let ended_reason = format!("Client Connection Ended Reason: {:?}\n", reason);
                let _ = self.channels.network_debug_send.send(ended_reason);
            }
        }

        // There might need to be more logic here
        remaining_connections == 0
    }

    fn connection_ending_warning(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        reason: ConnectionEndReason,
    ) {
        if let Some(my_conn_id) = &self.cid_option {
            if *my_conn_id == *cid {
                self.cid_option = None;
                self.main_recv_type = None;
                let ending_reason = format!("Client Connection Ending Reason: {:?}\n", reason);
                let _ = self.channels.network_debug_send.send(ending_reason);
            }
        }
    }

    fn tick(&mut self, endpoint: &mut Endpoint) -> bool {
        //let _ = endpoint.send_out_ping_past_duration(Duration::from_millis(2000));

        self.command_handler_tick += 1;
        if self.command_handler_tick >= 10 {
            loop {
                match self.channels.command_recv.try_recv() {
                    Err(TryRecvError::Empty) => break,
                    Ok(NetworkCommand::Client(client_cmd)) => {
                        self.handle_commands(endpoint, client_cmd);
                    }
                    Ok(NetworkCommand::Stop(int)) => {
                        if let Some(cid) = &self.cid_option {
                            let _ = endpoint.close_connection(cid, 8);
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

    fn main_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        read_data: &[u8],
    ) -> Option<usize> {
        if let Some(my_cid) = &mut self.cid_option {
            if *my_cid == *cid {
                if let Some(msg_type) = self.main_recv_type.take() {
                    if self.handle_stream_msg(endpoint, cid, msg_type, read_data) {
                        Some(MESSAGE_HEADER_SIZE)
                    } else {
                        None // Close Connection
                    }
                } else if let Some((new_msg_type, size)) = StreamMsgType::from_header(read_data) {
                    if new_msg_type.intended_for_client() {
                        self.main_recv_type = Some(new_msg_type);
                        Some(size as usize)
                    } else {
                        None // Not intended for client
                    }
                } else {
                    None // Invalid Header
                }
            } else {
                // Weird state to be in considering logic below...
                None // Close Connection
            }
        } else if let Some((StreamMsgType::ServerStateRefresh, size)) =
            StreamMsgType::from_header(read_data)
        {
            self.cid_option = Some(*cid);
            self.main_recv_type = Some(StreamMsgType::ServerStateRefresh);
            Some(size as usize)
        } else {
            None // Invalid Header
        }
    }

    fn rt_stream_recv(
        &mut self,
        _endpoint: &mut Endpoint,
        _cid: &ConnectionId,
        read_data: &[u8],
        rt_id: u64,
    ) -> usize {
        let vec_data = Vec::from(read_data);
        let _ = self
            .audio_channels
            .packet_send
            .send(NetworkAudioPackets::MusicPacket((1, vec_data)));
        if (rt_id % 500) == 0 && rt_id != 0 {
            self.send_debug_text("10 Seconds Passed\n");
        }

        0
    }

    fn background_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        read_data: &[u8],
    ) -> Option<usize> {
        if let Some(my_cid) = &mut self.cid_option {
            if *my_cid == *cid {
                if let Some(msg_type) = self.background_recv_type.take() {
                    if self.handle_stream_msg(endpoint, cid, msg_type, read_data) {
                        Some(MESSAGE_HEADER_SIZE)
                    } else {
                        None // Close Connection
                    }
                } else if let Some((new_msg_type, size)) = StreamMsgType::from_header(read_data) {
                    if new_msg_type.intended_for_client() {
                        self.background_recv_type = Some(new_msg_type);
                        Some(size as usize)
                    } else {
                        None // Not intended for client
                    }
                } else {
                    None // Invalid Header
                }
            } else {
                None // Close Connection
            }
        } else {
            None // Close Connection
        }
    }
}

pub(crate) fn server_thread(
    use_ipv4: bool,
    port: u16,
    server_name: String,
    channels: NetworkThreadChannels,
) {
    let bind_address = match use_ipv4 {
        false => SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port, 0, 0)),
        true => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)),
    };

    let config = Config {
        idle_timeout_in_ms: 5000,
        reliable_stream_buffer: 65536,
        unreliable_stream_buffer: 65536,
        keep_alive_timeout: None,
        initial_main_recv_size: BUFFER_SIZE_PER_CONNECTION,
        main_recv_first_bytes: MESSAGE_HEADER_SIZE,
        initial_rt_recv_size: 65536,
        rt_recv_first_bytes: 0,
        initial_background_recv_size: 65536,
        background_recv_first_bytes: MESSAGE_HEADER_SIZE,
    };

    let mut server_endpoint =
        match Endpoint::new_server(bind_address, ALPN_NAME, CERT_PATH, PKEY_PATH, config) {
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

    let mut rtc_handler = EndpointHandler::new(&mut server_endpoint, &mut server_state);
    match rtc_handler.run_event_loop(std::time::Duration::from_millis(5)) {
        Ok(_) => {}
        Err(e) => {
            let error_print = format!("Server Error: {:?}\n", e);
            let _ = server_state.channels.network_debug_send.send(error_print);
        }
    }

    // Eventual Friendly Server Cleanup Here

    server_state.send_debug_text("Server Network Thread Exiting\n");
}

#[cfg(feature = "client")]
pub(crate) fn client_thread(
    server_address: SocketAddr,
    user_name: String,
    channels: NetworkThreadChannels,
    network_audio_out_channels: NetworkAudioOutputChannels,
) {
    let bind_address = match server_address.is_ipv6() {
        true => SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0)),
        false => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)),
    };

    let config = Config {
        idle_timeout_in_ms: 5000,
        reliable_stream_buffer: 65536,
        unreliable_stream_buffer: 65536,
        keep_alive_timeout: Some(Duration::from_millis(2000)),
        initial_main_recv_size: BUFFER_SIZE_PER_CONNECTION,
        main_recv_first_bytes: MESSAGE_HEADER_SIZE,
        initial_rt_recv_size: 65536,
        rt_recv_first_bytes: 0,
        initial_background_recv_size: 65536,
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
        Err(err) => {
            let _ = channels
                .network_debug_send
                .send("Client Endpoint Creation Error!\n".to_string());
            // Can add more detailed print here later
            return;
        }
    };

    let mut client_handler = ClientHandler::new(user_name, channels, network_audio_out_channels);
    client_handler.send_debug_text("Starting Client Network!\n");

    loop {
        let mut endpoint_handler = EndpointHandler::new(&mut client_endpoint, &mut client_handler);
        match endpoint_handler.run_event_loop(std::time::Duration::from_millis(5)) {
            Ok(true) => {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    if client_handler.handle_limited_commands(&mut client_endpoint) {
                        break;
                    }
                }
                if client_endpoint.get_num_connections() == 0 {
                    break;
                }
            }
            Ok(false) => {
                break;
            }
            Err(e) => {
                let error_print = format!("Client Error: {:?}\n", e);
                let _ = client_handler.channels.network_debug_send.send(error_print);
                break;
            }
        }
    }

    // Eventual Friendly Client Cleanup Here

    client_handler.send_debug_text("Client Network Thread Exiting!\n");
}
