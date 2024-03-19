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
use std::time::{Duration, Instant};

// Use Inter-Thread Communication Definitions
#[cfg(feature = "client")]
use crate::communication::{ClientCommand, NetworkAudioOutPackets, NetworkAudioThreadChannels};
use crate::communication::{
    NetworkCommand, NetworkStateConnection, NetworkStateMessage, NetworkTerminalThreadChannels,
    PopError, PushError, ServerCommand,
};

// Use quic sub-library for internet communications
use swiftlet_quic::{
    endpoint::{Config, ConnectionEndReason, ConnectionId, Endpoint, SocketAddr},
    EndpointEventCallbacks, EndpointHandler,
};

const BUFFER_SIZE_PER_CONNECTION: usize = 4_194_304; // 4 MiB

mod protocol;
use protocol::{set_stream_msg_size, StreamMsgType, TransferIntention};

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
    rt_send: bool,
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
            rt_send: false,
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
    terminal_channels: NetworkTerminalThreadChannels,
    command_handler_tick: u64,
    potential_clients: Vec<ConnectionId>,
    client_states: Vec<ClientState>,
    next_transfer_id: u16,
    music_storage: Vec<MusicStorage>,
    music_playback: Option<MusicPlayback>,
}

impl ServerState {
    fn new(server_name: String, terminal_channels: NetworkTerminalThreadChannels) -> Self {
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
            terminal_channels,
            command_handler_tick: 0,
            potential_clients: Vec::new(),
            client_states: Vec::new(),
            next_transfer_id: 1,
            music_storage: Vec::new(),
            music_playback: None,
        }
    }

    #[inline]
    fn send_debug_text(&mut self, text: &str) {
        let _ = self.terminal_channels.debug_send.push(text.to_string());
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
        //set_stream_msg_size(&mut send_data);

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
                    //set_stream_msg_size(&mut send_data);

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
            let send_data = self.create_refresh_data(cs_ind);
            let _ = endpoint.main_stream_send(cid, send_data);

            // Send all other clients a msg about the new client
            for (ind, conn) in self.client_states.iter().enumerate() {
                if ind != cs_ind {
                    let send_data = self.create_new_client_data(cs_ind);
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

        set_stream_msg_size(&mut data);
        data
    }

    fn create_new_client_data(&self, verified_index: usize) -> Vec<u8> {
        let mut data = StreamMsgType::NewClient.get_send_data_vec(None);
        let cs = &self.client_states[verified_index];

        data.push(cs.user_name_len as u8);
        data.extend_from_slice(&cs.user_name[..cs.user_name_len]);
        data.push(cs.state);

        set_stream_msg_size(&mut data);
        data
    }

    fn create_state_change_data(&self, verified_index: usize) -> Vec<u8> {
        let mut data = StreamMsgType::ClientNewState.get_send_data_vec(Some(2));
        let cs = &self.client_states[verified_index];
        data.push(verified_index as u8);
        data.push(cs.state);

        data
    }

    fn refresh_update(&mut self) {
        let mut state_populate = Vec::<NetworkStateConnection>::new();

        for cs in &self.client_states {
            let conn_state = NetworkStateConnection {
                name: u8_to_str(&cs.user_name[..cs.user_name_len]),
                state: cs.state,
            };
            state_populate.push(conn_state);
        }

        let state_update = NetworkStateMessage::ConnectionsRefresh((None, state_populate));
        let _ = self.terminal_channels.state_send.push(state_update);
    }

    fn new_connection_update(&mut self, verified_index: usize) {
        let cs = &self.client_states[verified_index];
        let conn_name = u8_to_str(&cs.user_name[..cs.user_name_len]);
        let state_update = NetworkStateMessage::NewConnection((conn_name, cs.state));
        let _ = self.terminal_channels.state_send.push(state_update);
    }

    fn state_change_update(&mut self, verified_index: usize) {
        let cs = &self.client_states[verified_index];
        let state_update = NetworkStateMessage::StateChange((verified_index, cs.state));
        let _ = self.terminal_channels.state_send.push(state_update);
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
            let _ = self.terminal_channels.debug_send.push(ended_reason);

            // Temporarily (inefficiently) used for removing of clients
            for vi in 0..self.client_states.len() {
                let send_data = self.create_refresh_data(vi);
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
        let _ = self.terminal_channels.debug_send.push(ending_reason);
    }

    fn tick(&mut self, endpoint: &mut Endpoint) -> bool {
        if let Some(playback) = &mut self.music_playback {
            playback.tick += 1; // 4 ticks should be the 20ms music currently hidden requirement
            if playback.tick >= 4 {
                let mut send_data = StreamMsgType::NextMusicPacket.get_send_data_vec(None);
                //let mut send_data = Vec::new();
                send_data.push(playback.stereo_byte);

                let len =
                    self.music_storage[playback.storage_index].packet_len[playback.packet_num];
                let next_offset = playback.data_offset + (len as usize);
                send_data.extend_from_slice(
                    &self.music_storage[playback.storage_index].packet_data
                        [playback.data_offset..next_offset],
                );
                set_stream_msg_size(&mut send_data);

                playback.data_offset = next_offset;
                playback.packet_num += 1;
                if playback.packet_num
                    >= self.music_storage[playback.storage_index].packet_len.len()
                {
                    playback.packet_num = 0;
                    playback.data_offset = 0;
                }
                playback.tick = 0;

                let mut num_listeners = 0;
                for cs in self.client_states.iter_mut() {
                    if (cs.state & 2) > 0 {
                        // Makes copies here which isn't ideal (especially one more than number of sends)
                        let _ = endpoint.rt_stream_send(&cs.cid, Some(send_data.clone()), false);
                        cs.rt_send = true;
                        num_listeners += 1;
                    }
                }
                if num_listeners == 0 {
                    self.music_playback = None;
                }
            }
        }

        for cs in self.client_states.iter_mut() {
            if cs.rt_send {
                let _ = endpoint.rt_stream_send(&cs.cid, None, true);
                cs.rt_send = false;
            }
        }

        self.command_handler_tick += 1;
        if self.command_handler_tick >= 10 {
            loop {
                match self.terminal_channels.command_recv.pop() {
                    Err(PopError::Empty) => break,
                    Ok(NetworkCommand::Server(server_cmd)) => {
                        self.handle_commands(endpoint, server_cmd)
                    }
                    Ok(NetworkCommand::Stop(int)) => {
                        for cs in &self.client_states {
                            let _ = endpoint.close_connection(&cs.cid, 4);
                        }
                        return true;
                    }
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
                    Some(protocol::MESSAGE_HEADER_SIZE)
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
                Some(protocol::MESSAGE_HEADER_SIZE)
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

    fn rt_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        read_data: &[u8],
        rt_id: u64,
    ) -> usize {
        // let debug_string = format!(
        //     "Rt Id: {}, len: {}, byte: {}, size: {}\n",
        //     rt_id,
        //     read_data.len(),
        //     read_data[0],
        //     u16::from_le_bytes([read_data[1], read_data[2]])
        // );
        // let _ = self.terminal_channels.debug_send.send(debug_string);
        if let Some((msg_type, size)) =
            StreamMsgType::from_header(&read_data[..protocol::MESSAGE_HEADER_SIZE])
        {
            match msg_type {
                StreamMsgType::VoiceDataPacket => {
                    if let Some(vi) = self.find_connection_index_from_cid(cid) {
                        if (self.client_states[vi].state & 0x4) > 0 {
                            //self.send_debug_text("Got Voice Data Packet!\n");
                            let mut send_data = StreamMsgType::VoiceDataPacket
                                .get_send_data_vec(Some(size as usize));
                            let vi_bytes = usize::to_le_bytes(vi);
                            send_data.push(vi_bytes[0]);
                            send_data.push(vi_bytes[1]);
                            send_data.extend_from_slice(&read_data[5..]);

                            for (i, cs) in self.client_states.iter_mut().enumerate() {
                                if i == vi {
                                    if (cs.state & 0x8) > 0 {
                                        let _ = endpoint.rt_stream_send(
                                            &cs.cid,
                                            Some(send_data.clone()),
                                            false,
                                        );
                                        cs.rt_send = true;
                                    }
                                } else if (cs.state & 0x4) > 0 {
                                    // Makes copies here which isn't ideal (especially one more than number of sends)
                                    let _ = endpoint.rt_stream_send(
                                        &cs.cid,
                                        Some(send_data.clone()),
                                        false,
                                    );
                                    cs.rt_send = true;
                                }
                            }
                        } else {
                            // Malicious Client Watch Here in Future
                        }
                    } else {
                        let _ = endpoint.close_connection(cid, 32);
                    }
                }
                _ => {
                    let _ = endpoint.close_connection(cid, 31);
                }
            }
        } else {
            let _ = endpoint.close_connection(cid, 30);
        }
        0
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
                    Some(protocol::MESSAGE_HEADER_SIZE)
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
    terminal_channels: NetworkTerminalThreadChannels,
    command_handler_tick: u64,
    cid_option: Option<ConnectionId>, // Focus Connection ID
    main_recv_type: Option<StreamMsgType>,
    rt_recv_type: Option<StreamMsgType>,
    rt_recv_expected_id: u64,
    avg_voice_send: u64,
    background_recv_type: Option<StreamMsgType>,
    transfer_data: Option<Vec<u8>>,
    audio_channels: NetworkAudioThreadChannels,
    callback_count: u64,
    last_instant: Instant,
    avg_duration: Duration,
}

#[cfg(feature = "client")]
impl ClientHandler {
    fn new(
        user_name: String,
        terminal_channels: NetworkTerminalThreadChannels,
        audio_channels: NetworkAudioThreadChannels,
    ) -> Self {
        ClientHandler {
            user_name,
            terminal_channels,
            command_handler_tick: 0,
            cid_option: None,
            main_recv_type: None,
            rt_recv_type: None,
            rt_recv_expected_id: 0,
            avg_voice_send: 0,
            background_recv_type: None,
            transfer_data: None,
            audio_channels,
            callback_count: 0,
            last_instant: Instant::now(),
            avg_duration: Duration::from_millis(0),
        }
    }

    #[inline]
    fn send_debug(&mut self, s: String) -> bool {
        match self.terminal_channels.debug_send.push(s) {
            Ok(_) => false,
            Err(PushError::Full(_)) => panic!("Network Client: Debug Send Full!"),
        }
    }

    #[inline]
    fn send_debug_str(&mut self, s: &str) -> bool {
        self.send_debug(s.to_string())
    }

    #[inline]
    fn send_debug_text(&mut self, text: &str) {
        let _ = self.send_debug(text.to_string());
    }

    fn handle_commands(&mut self, endpoint: &mut Endpoint, cmd: ClientCommand) {
        match cmd {
            ClientCommand::StateChange(new_state_requested) => {
                if let Some(cid) = &self.cid_option {
                    let mut send_data = StreamMsgType::NewStateRequest.get_send_data_vec(Some(1));
                    send_data.push(new_state_requested);
                    //set_stream_msg_size(&mut send_data);
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
                            (transfer_data.len() - protocol::MESSAGE_HEADER_SIZE).to_ne_bytes();

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

    fn handle_limited_commands(&mut self, endpoint: &mut Endpoint) -> bool {
        loop {
            match self.terminal_channels.command_recv.pop() {
                Err(PopError::Empty) => break,
                Ok(NetworkCommand::Client(ClientCommand::ServerConnect(server_address))) => {
                    let _ = endpoint.add_client_connection(server_address, SERVER_NAME);
                    return true;
                }
                Ok(NetworkCommand::Stop(int)) => return true,
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

    fn handle_state_refresh(&mut self, read_data: &[u8]) {
        let conn_ind = read_data[1] as usize;

        let mut name_end: usize = (read_data[2] + 3).into();
        let server_name = u8_to_str(&read_data[3..name_end]);
        let name_update = NetworkStateMessage::ServerNameChange(server_name);
        let _ = self.terminal_channels.state_send.push(name_update);

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
        let _ = self.terminal_channels.state_send.push(state_update);
    }

    fn handle_new_client(&mut self, read_data: &[u8]) {
        let name_end: usize = (read_data[0] + 1).into();
        let client_name = u8_to_str(&read_data[1..name_end]);
        let new_conn = NetworkStateMessage::NewConnection((client_name, read_data[name_end]));
        let _ = self.terminal_channels.state_send.push(new_conn);
    }

    fn handle_client_new_state(&mut self, read_data: &[u8]) {
        let conn_pos = read_data[0] as usize;
        let new_state = read_data[1];

        if (new_state & 0x2) == 0 {
            let _ = self
                .audio_channels
                .packet_send
                .push(NetworkAudioOutPackets::MusicStop(255));
        }
        // if (new_state & 0x4) == 0 {
        //     let _ = self
        //         .audio_channels
        //         .packet_send
        //         .send(NetworkAudioOutPackets::VoiceStop(255));
        // }

        let new_conn = NetworkStateMessage::StateChange((conn_pos, new_state));
        let _ = self.terminal_channels.state_send.push(new_conn);
    }
}

#[cfg(feature = "client")]
impl EndpointEventCallbacks for ClientHandler {
    fn connection_started(&mut self, endpoint: &mut Endpoint, cid: &ConnectionId) {
        let _ = self
            .terminal_channels
            .debug_send
            .push("Announcing Self to Server!\n".to_string());
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
                let _ = self.terminal_channels.debug_send.push(ended_reason);
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
                let _ = self.terminal_channels.debug_send.push(ending_reason);
            }
        }
    }

    fn tick(&mut self, endpoint: &mut Endpoint) -> bool {
        self.callback_count += 1;
        let current_instant = Instant::now();
        let current_duration = current_instant - self.last_instant;
        self.avg_duration += current_duration;

        if (self.callback_count % 200) == 0 {
            // let s = format!(
            //     "Avg Client Tick Callback Timing: {:?}\n",
            //     self.avg_duration / 200
            // );
            // if self.send_debug(s) {
            //     return true;
            // }
            self.avg_duration = Duration::from_millis(0);
        }

        if let Some(cid) = &self.cid_option {
            let mut pkt_times = 0;
            loop {
                match self.audio_channels.packet_recv.pop() {
                    Err(PopError::Empty) => break,
                    Ok(pkt) => {
                        let channel_latency = Instant::now() - pkt.instant;
                        //if channel_latency > Duration::from_millis(6) {
                        let s = format!(
                            "Channel Latency: {:?}; Since Last Tick: {:?}\n",
                            channel_latency, current_duration
                        );
                        let _ = self.terminal_channels.debug_send.push(s);
                        //}
                        pkt_times += 1;
                        self.avg_voice_send += 1;
                        let mut send_data =
                            StreamMsgType::VoiceDataPacket.get_send_data_vec(Some(pkt.len + 2));
                        send_data.push(0);
                        send_data.push(0);
                        send_data.extend_from_slice(&pkt.data[..pkt.len]);

                        let _ = endpoint.rt_stream_send(cid, Some(send_data), true);
                    }
                }
            }
            if pkt_times > 1 {
                // let s = format!("Voice Sends: {}: {}\n", self.callback_count, pkt_times);
                // if self.send_debug(s) {
                //     return true;
                // }
            }
        }
        if (self.callback_count % 500) == 0 {
            // let s = format!("Avg Voice Send: {}\n", self.avg_voice_send);
            // if self.send_debug(s) {
            //     return true;
            // }
            self.avg_voice_send = 0;
        }

        self.command_handler_tick += 1;
        if self.command_handler_tick >= 25 {
            loop {
                match self.terminal_channels.command_recv.pop() {
                    Err(PopError::Empty) => break,
                    Ok(NetworkCommand::Client(client_cmd)) => {
                        self.handle_commands(endpoint, client_cmd);
                    }
                    Ok(NetworkCommand::Stop(int)) => {
                        if let Some(cid) = &self.cid_option {
                            let _ = endpoint.close_connection(cid, 8);
                        }
                        return true;
                    }
                    Ok(NetworkCommand::Server(_)) => {}
                }
            }
            self.command_handler_tick = 0;
        }

        self.last_instant = current_instant;
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
                        Some(protocol::MESSAGE_HEADER_SIZE)
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
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        read_data: &[u8],
        rt_id: u64,
    ) -> usize {
        if let Some(my_cid) = &mut self.cid_option {
            if *my_cid == *cid {
                // let debug_string = format!("Rt Id: {}, len: {}\n", rt_id, read_data.len());
                // let _ = self.terminal_channels.debug_send.send(debug_string);
                if self.rt_recv_expected_id != rt_id {
                    let diff = rt_id - self.rt_recv_expected_id;
                    if diff > 1 {
                        let debug_string = format!("Realtime Recv Packet Skip: {}\n", diff);
                        let _ = self.terminal_channels.debug_send.push(debug_string);
                    }

                    // Skipped IDs
                    self.rt_recv_type = None;
                    self.rt_recv_expected_id = rt_id;
                }
                if let Some(msg_type) = self.rt_recv_type.take() {
                    match msg_type {
                        StreamMsgType::VoiceDataPacket => {
                            let vec_data = Vec::from(&read_data[2..]);
                            let voice_id = u16::from_le_bytes([read_data[0], read_data[1]]);
                            let _ = self
                                .audio_channels
                                .packet_send
                                .push(NetworkAudioOutPackets::VoiceData((voice_id, vec_data)));
                            protocol::MESSAGE_HEADER_SIZE
                        }
                        StreamMsgType::NextMusicPacket => {
                            //self.send_debug_text("Music Packet!\n");
                            let vec_data = Vec::from(read_data);
                            let _ = self
                                .audio_channels
                                .packet_send
                                .push(NetworkAudioOutPackets::MusicPacket((255, vec_data)));
                            protocol::MESSAGE_HEADER_SIZE
                        }
                        _ => {
                            //self.send_debug_text("Hmm!\n");
                            let _ = endpoint.close_connection(cid, 44);
                            0
                        }
                    }
                } else if let Some((new_msg_type, size)) = StreamMsgType::from_header(read_data) {
                    // let debug_string =
                    //     format!("MsgTyp: {}, size: {}\n", new_msg_type.to_u8(), size);
                    // let _ = self.terminal_channels.debug_send.send(debug_string);
                    match new_msg_type {
                        StreamMsgType::VoiceDataPacket => {
                            self.rt_recv_type = Some(new_msg_type);
                            size as usize
                        }
                        StreamMsgType::NextMusicPacket => {
                            //self.send_debug_text("Next Music Packet!\n");
                            self.rt_recv_type = Some(new_msg_type);
                            size as usize
                        }
                        _ => {
                            let _ = endpoint.close_connection(cid, 43);
                            0
                        }
                    }
                } else {
                    // Invalid Header
                    let _ = endpoint.close_connection(cid, 42);
                    0
                }
            } else {
                let _ = endpoint.close_connection(cid, 41);
                0
            }
        } else {
            // Invalid Header
            let _ = endpoint.close_connection(cid, 40);
            0
        }
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
                        Some(protocol::MESSAGE_HEADER_SIZE)
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
    mut terminal_channels: NetworkTerminalThreadChannels,
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
        main_recv_first_bytes: protocol::MESSAGE_HEADER_SIZE,
        initial_rt_recv_size: 65536,
        rt_recv_first_bytes: 0,
        initial_background_recv_size: 65536,
        background_recv_first_bytes: protocol::MESSAGE_HEADER_SIZE,
    };

    let mut server_endpoint =
        match Endpoint::new_server(bind_address, ALPN_NAME, CERT_PATH, PKEY_PATH, config) {
            Ok(endpoint) => endpoint,
            Err(err) => {
                let _ = terminal_channels
                    .debug_send
                    .push("Server Endpoint Creation Error!\n".to_string());
                // Can add more detailed print here later
                return;
            }
        };

    let mut server_state = ServerState::new(server_name, terminal_channels);
    server_state.send_debug_text("Starting Server Network!\n");

    let mut rtc_handler = EndpointHandler::new(&mut server_endpoint, &mut server_state);
    match rtc_handler.run_event_loop(std::time::Duration::from_millis(5)) {
        Ok(_) => {}
        Err(e) => {
            let error_print = format!("Server Error: {:?}\n", e);
            let _ = server_state.terminal_channels.debug_send.push(error_print);
        }
    }

    // Eventual Friendly Server Cleanup Here

    server_state.send_debug_text("Server Network Thread Exiting\n");
}

#[cfg(feature = "client")]
pub(crate) fn client_thread(
    server_address: SocketAddr,
    user_name: String,
    mut terminal_channels: NetworkTerminalThreadChannels,
    audio_channels: NetworkAudioThreadChannels,
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
        main_recv_first_bytes: protocol::MESSAGE_HEADER_SIZE,
        initial_rt_recv_size: 65536,
        rt_recv_first_bytes: protocol::MESSAGE_HEADER_SIZE,
        initial_background_recv_size: 65536,
        background_recv_first_bytes: protocol::MESSAGE_HEADER_SIZE,
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
            let _ = terminal_channels
                .debug_send
                .push("Client Endpoint Creation Error!\n".to_string());
            // Can add more detailed print here later
            return;
        }
    };

    let mut client_handler = ClientHandler::new(user_name, terminal_channels, audio_channels);
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
                let _ = client_handler
                    .terminal_channels
                    .debug_send
                    .push(error_print);
                break;
            }
        }
    }

    // Eventual Friendly Client Cleanup Here

    client_handler.send_debug_text("Client Network Thread Exiting!\n");
}
