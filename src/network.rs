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
pub(super) use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

// Use Inter-Thread Communication Definitions
use crate::communication::{
    ClientCommand, NetworkCommand, NetworkStateConnection, NetworkStateMessage,
    NetworkThreadChannels, Sender, ServerCommand, TryRecvError,
};

mod quic;
use quic::{Endpoint, EndpointError, EndpointEvent, StreamReadable};

// StreamMessage* structures (in the message module) are used to write/send and recv/read information between RealtimeMedia connections
mod message;
use message::{StreamMsgIntended, StreamMsgRecv, StreamMsgSend, StreamMsgType};

// ALPN Constants
const MAIN_STREAM_ID: u64 = 0; // Bidirectional stream ID# used for reliable communication in the application between the server and the client
                               // This stream is started by the Client when it announces itself to the server when it connects to it
const SERVER_REALTIME_START_ID: u64 = 3;

struct RealtimeMediaTransfer {
    data: Vec<u8>,
    size: usize,
    bytes_transfered: usize,
}

struct RealtimeMediaConnection {
    id: u64,
    main_recv: StreamMsgRecv,
    last_activity_tick: u64,
    transfer_send: Option<RealtimeMediaTransfer>,
    transfer_recv: Option<RealtimeMediaTransfer>,
}

impl RealtimeMediaConnection {
    fn new(conn_id: u64, stream_msg_recv: StreamMsgRecv, current_tick: u64) -> Self {
        RealtimeMediaConnection {
            id: conn_id,
            main_recv: stream_msg_recv,
            last_activity_tick: current_tick,
            transfer_send: None,
            transfer_recv: None,
        }
    }

    fn send_main_stream_data(
        &mut self,
        endpoint: &mut Endpoint,
        data: &[u8],
        current_tick: u64,
    ) -> Option<(u64, u64)> {
        // Handle errors in future and possibly print num sends...?
        match endpoint.send_stream_data(self.id, MAIN_STREAM_ID, data, false) {
            Ok((immediate_sends, delayed_sends)) => {
                self.last_activity_tick = current_tick;
                Some((immediate_sends, delayed_sends))
            }
            Err(e) => None,
        }
    }

    fn recv_transfer_data(&mut self) -> bool {
        if let Some(media_transfer) = &mut self.transfer_recv {
            let read_data = self.main_recv.get_data_to_read();
            media_transfer.data.extend_from_slice(&read_data[1..]);
            if media_transfer.data.len() >= media_transfer.size {
                return true;
                //} else if media_transfer.data.len() > media_transfer.size {
                //    return true;
                //self.send_debug_text("Got Unexpected TransferData Messages!\n");
            }
        }
        false
    }
}

enum RealtimeMediaTypeData {
    Server(ServerState),
    Client(ClientHandler),
}

struct RealtimeMediaEndpoint {
    current_tick: u64,
    endpoint: Endpoint,
    channels: NetworkThreadChannels,
    connections: Vec<RealtimeMediaConnection>,
    msg_send: StreamMsgSend,
    type_data: RealtimeMediaTypeData,
}

impl RealtimeMediaEndpoint {
    fn new(
        endpoint: Endpoint,
        channels: NetworkThreadChannels,
        type_data: RealtimeMediaTypeData,
    ) -> Self {
        RealtimeMediaEndpoint {
            current_tick: 0,
            endpoint,
            channels,
            connections: Vec::new(),
            msg_send: StreamMsgSend::new(StreamMsgType::InvalidType),
            type_data,
        }
    }

    #[inline]
    fn send_debug_text(&self, text: &str) {
        let _ = self.channels.network_debug_send.send(text.to_string());
    }

    #[inline]
    fn find_connection_index(&self, conn_id: u64) -> Option<usize> {
        self.connections.iter().position(|conn| conn.id == conn_id)
    }

    #[inline]
    fn find_connection_index_with_probable(
        &self,
        conn_id: u64,
        probable_index: usize,
    ) -> Option<usize> {
        if probable_index < self.connections.len() && self.connections[probable_index].id == conn_id
        {
            Some(probable_index)
        } else {
            self.connections.iter().position(|conn| conn.id == conn_id)
        }
    }

    fn close_connection(&mut self, verified_index: usize, error_code: u64) {
        let _ = self
            .endpoint
            .close_connection(self.connections[verified_index].id, error_code);
        self.connections.remove(verified_index);
        self.closing_connection_specifics(verified_index);
    }

    fn connection_closing(&mut self, conn_id: u64) {
        if let Some(verified_index) = self.find_connection_index(conn_id) {
            self.connections.remove(verified_index);
            self.closing_connection_specifics(verified_index);
        }
    }

    fn closing_connection_specifics(&mut self, verified_index: usize) {
        self.send_debug_text("Connection Closing!\n");
        match &mut self.type_data {
            RealtimeMediaTypeData::Server(server_state) => {
                server_state.remove_connection_state(verified_index);

                // Temporarily (inefficiently) used for removing of clients
                self.msg_send
                    .refresh_send(StreamMsgType::ServerStateRefresh);
                let write_bytes =
                    server_state.create_refresh_data(self.msg_send.get_data_to_write());
                self.msg_send.update_data_write(write_bytes);
                let mut_data = self.msg_send.get_mut_data_to_send();
                for (verified_index, conn) in self.connections.iter_mut().enumerate() {
                    mut_data[message::MESSAGE_HEADER_SIZE + 1] = verified_index as u8;
                    conn.send_main_stream_data(&mut self.endpoint, mut_data, self.current_tick);
                }

                server_state.refresh_update(&self.channels.network_state_send);
            }
            RealtimeMediaTypeData::Client(client_handler) => {
                // Nothing Yet
            }
        }
    }

    // Returns true if the thread should maybe call this event loop again (ie. new Server to connect to via commands)
    fn run_event_loop(&mut self) -> bool {
        let tick_duration = std::time::Duration::from_millis(5); // Might make dynamic in future...
        let start_instant = std::time::Instant::now();
        let mut next_tick_instant = start_instant;
        let mut command_handler_ticks = 0;

        loop {
            // This update sleeps when waiting for the next instant or recv udp data and the duration is >= 1ms
            match self.endpoint.update() {
                Ok(EndpointEvent::PotentiallyReceivedData) => {
                    self.run_recv_loop();
                }
                Ok(EndpointEvent::NextTick) => {
                    next_tick_instant += tick_duration; // Does not currently check for skipped ticks / assumes computer processes all
                    self.endpoint.set_next_tick_instant(next_tick_instant);
                    self.current_tick += 1;

                    // Handle looping over keep_alives for every client connection
                    self.keep_client_connections_alive();

                    // Eventually handle data that gets sent at set intervals
                    command_handler_ticks += 1;
                    if command_handler_ticks >= 10 {
                        // Handle Commands Every 10 Ticks (50ms)

                        if self.handle_incoming_commands() {
                            return false;
                        }
                        command_handler_ticks = 0;
                    }
                }
                Ok(EndpointEvent::ConnectionClosed(conn_id)) => {
                    // Need to process event for when a connection has StartedClosing instead here in future
                    self.connection_closing(conn_id);
                }
                Err(_) => {
                    self.send_debug_text("Event Loop Endpoint Error");
                }
                _ => {
                    self.send_debug_text("Event Loop Section Should Never Be Reached");
                }
            }
        }
    }

    fn keep_client_connections_alive(&mut self) {
        if let RealtimeMediaTypeData::Client(client_handler) = &self.type_data {
            for conn in self.connections.iter_mut() {
                if self.current_tick > conn.last_activity_tick + 400 {
                    self.msg_send
                        .refresh_send(StreamMsgType::KeepConnectionAlive);
                    conn.send_main_stream_data(
                        &mut self.endpoint,
                        self.msg_send.get_data_to_send(),
                        self.current_tick,
                    );
                }
            }
        }
    }

    fn run_recv_loop(&mut self) {
        loop {
            match self.endpoint.recv() {
                Ok(EndpointEvent::DoneReceiving) => {
                    break;
                }
                Ok(event) => {
                    //self.send_debug_text("Event Processed:\n");
                    match event {
                        EndpointEvent::StreamReceivedData(stream_readable) => {
                            if stream_readable.stream_id == MAIN_STREAM_ID {
                                self.recv_main_stream_data(&stream_readable);
                            }
                            // Handle other stream ids here in future
                        }
                        EndpointEvent::ConnectionClosed(conn_id) => {
                            // Need to process event for when a connection has StartedClosing instead here in future
                            self.connection_closing(conn_id);
                        }
                        EndpointEvent::FinishedConnectingOnce(conn_id) => {
                            self.client_announce(conn_id);
                        }
                        EndpointEvent::NewConnectionStarted => {
                            //self.send_debug_text("New Connection!\n");
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    match e {
                        EndpointError::StreamRecv => self.send_debug_text("Stream Recv Error!\n"),
                        EndpointError::RecvTooMuchData => {
                            self.send_debug_text("Recv Too Much Data Error!\n")
                        }
                        EndpointError::ConnectionSend => {
                            self.send_debug_text("Connection Send Error!\n")
                        }
                        EndpointError::SocketSend => self.send_debug_text("Socket Send Error!\n"),
                        EndpointError::SocketRecv => self.send_debug_text("Socket Recv Error!\n"),
                        EndpointError::ConnectionRecv => {
                            self.send_debug_text("Connection Recv Error!\n")
                        }
                        EndpointError::StreamSend => self.send_debug_text("Stream Send Error!\n"),
                        EndpointError::StreamSendFilled => {
                            self.send_debug_text("Stream Send Filled Error!\n")
                        }
                        EndpointError::SocketCreation => {
                            self.send_debug_text("Socket Creation Error!\n")
                        }
                        _ => self.send_debug_text("General Endpoint Recv Error!\n"),
                    }
                    break;
                }
            }
        }
    }

    fn client_announce(&mut self, conn_id: u64) {
        if let RealtimeMediaTypeData::Client(client_handler) = &mut self.type_data {
            let _ = self
                .channels
                .network_debug_send
                .send("Announcing Self to Server!\n".to_string());
            self.msg_send.refresh_send(StreamMsgType::NewClientAnnounce);
            let write_bytes =
                client_handler.create_announce_data(self.msg_send.get_data_to_write());
            self.msg_send.update_data_write(write_bytes);

            let _ = self.endpoint.create_stream(conn_id, MAIN_STREAM_ID, 100);
            let _ = self.endpoint.send_stream_data(
                conn_id,
                MAIN_STREAM_ID,
                self.msg_send.get_data_to_send(),
                false,
            );
        }
    }

    fn recv_main_stream_data(&mut self, stream_readable: &StreamReadable) {
        let verified_index = match self.find_connection_index(stream_readable.conn_id) {
            Some(vi) => vi,
            None => {
                // Must add app connection (server verifies client first)
                let mut header_data = [0; message::MESSAGE_HEADER_SIZE];
                match self
                    .endpoint
                    .recv_stream_data(stream_readable, &mut header_data)
                {
                    Ok((header_bytes, header_fin)) => {
                        if header_bytes == message::MESSAGE_HEADER_SIZE && !header_fin {
                            let stream_msg = StreamMsgRecv::new(header_data);
                            match stream_msg.get_message_type() {
                                StreamMsgType::NewClientAnnounce => {
                                    // Expects NewClientAnnounce to have been enitrely in ONE UDP
                                    // So perform the verification of the client and remaining setup now
                                    if let Some(vi) = self.server_add_new_verified_connection(
                                        stream_msg,
                                        stream_readable,
                                    ) {
                                        vi
                                    } else {
                                        let _ = self
                                            .endpoint
                                            .close_connection(stream_readable.conn_id, 20);
                                        return;
                                    }
                                }
                                StreamMsgType::ServerStateRefresh => {
                                    if let RealtimeMediaTypeData::Client(client_handler) =
                                        &mut self.type_data
                                    {
                                        self.connections.push(RealtimeMediaConnection::new(
                                            stream_readable.conn_id,
                                            stream_msg,
                                            self.current_tick,
                                        ));

                                        self.connections.len() - 1
                                    } else {
                                        let _ = self
                                            .endpoint
                                            .close_connection(stream_readable.conn_id, 21);
                                        return;
                                    }
                                }
                                _ => {
                                    let _ =
                                        self.endpoint.close_connection(stream_readable.conn_id, 22);
                                    return;
                                }
                            }
                        } else if header_bytes > 0 || header_fin {
                            let _ = self.endpoint.close_connection(stream_readable.conn_id, 23);
                            return;
                        } else {
                            self.send_debug_text("Weird Case\n");
                            return;
                        }
                    }
                    Err(e) => {
                        // Probably close the connection
                        self.send_debug_text("Recv Stream Error!\n");
                        return;
                    }
                }
            }
        };

        loop {
            if self.connections[verified_index].main_recv.is_done_recving() {
                let mut header_data = [0; message::MESSAGE_HEADER_SIZE];
                match self
                    .endpoint
                    .recv_stream_data(stream_readable, &mut header_data)
                {
                    Ok((header_bytes, header_fin)) => {
                        if header_bytes == message::MESSAGE_HEADER_SIZE && !header_fin {
                            self.connections[verified_index]
                                .main_recv
                                .refresh_recv(header_data);
                            self.connections[verified_index].last_activity_tick = self.current_tick;
                        } else if header_bytes > 0 || header_fin {
                            self.close_connection(verified_index, 24);
                            return;
                        } else {
                            return;
                        }
                    }
                    Err(e) => {
                        self.send_debug_text("Stream Recv Error!");
                        return;
                    }
                }
            } else {
                let data_recv = self.connections[verified_index]
                    .main_recv
                    .get_data_to_recv();
                match self.endpoint.recv_stream_data(stream_readable, data_recv) {
                    Ok((recv_bytes, recv_fin)) => {
                        if recv_bytes > 0 {
                            // if let RealtimeMediaTypeData::Server(server_state) = &self.type_data {
                            //     let info_string = format!("Recv Bytes: {}\n", recv_bytes);
                            //     self.send_debug_text(info_string.as_str());
                            // }

                            self.connections[verified_index]
                                .main_recv
                                .update_data_recv(recv_bytes);
                            self.connections[verified_index].last_activity_tick = self.current_tick;

                            if let Some(intention) = self.connections[verified_index]
                                .main_recv
                                .get_done_intention()
                            {
                                match intention {
                                    StreamMsgIntended::Server => {
                                        self.process_server_stream_data(verified_index);
                                    }
                                    StreamMsgIntended::Client => {
                                        self.process_client_stream_data(verified_index);
                                    }
                                    StreamMsgIntended::Anyone => match self.type_data {
                                        RealtimeMediaTypeData::Server(_) => {
                                            self.process_server_stream_data(verified_index);
                                        }
                                        RealtimeMediaTypeData::Client(_) => {
                                            self.process_client_stream_data(verified_index);
                                        }
                                    },
                                    _ => {}
                                }
                            }
                            if recv_fin {
                                self.close_connection(verified_index, 25);
                                return;
                            }
                        } else {
                            return;
                        }
                    }
                    Err(e) => {
                        self.send_debug_text("Stream Recv Error!");
                        return;
                    }
                }
            }
        }
    }

    fn server_add_new_verified_connection(
        &mut self,
        mut stream_msg: StreamMsgRecv,
        stream_readable: &StreamReadable,
    ) -> Option<usize> {
        if let RealtimeMediaTypeData::Server(server_state) = &mut self.type_data {
            match self
                .endpoint
                .recv_stream_data(stream_readable, stream_msg.get_data_to_recv())
            {
                Ok((recv_bytes, recv_fin)) => {
                    stream_msg.update_data_recv(recv_bytes);
                    if !recv_fin {
                        if stream_msg.is_done_recving() {
                            let read_data = stream_msg.get_data_to_read();
                            let vi = self.connections.len();

                            // Add Connection if possible
                            if server_state.add_connection_state(vi, read_data) {
                                self.connections.push(RealtimeMediaConnection::new(
                                    stream_readable.conn_id,
                                    stream_msg,
                                    self.current_tick,
                                ));

                                // Send new client a state refresh
                                self.msg_send
                                    .refresh_send(StreamMsgType::ServerStateRefresh);
                                let write_bytes = server_state
                                    .create_refresh_data(self.msg_send.get_data_to_write());
                                self.msg_send.update_data_write(write_bytes);
                                let mut_data = self.msg_send.get_mut_data_to_send();

                                mut_data[message::MESSAGE_HEADER_SIZE + 1] = vi as u8;
                                self.connections[vi].send_main_stream_data(
                                    &mut self.endpoint,
                                    mut_data,
                                    self.current_tick,
                                );

                                // Send all other clients a msg about the new client
                                self.msg_send.refresh_send(StreamMsgType::NewClient);
                                let write_bytes = server_state
                                    .create_new_client_data(vi, self.msg_send.get_data_to_write());
                                self.msg_send.update_data_write(write_bytes);
                                let send_data = self.msg_send.get_data_to_send();
                                for (ind, conn) in self.connections.iter_mut().enumerate() {
                                    if ind != vi {
                                        conn.send_main_stream_data(
                                            &mut self.endpoint,
                                            send_data,
                                            self.current_tick,
                                        );
                                    }
                                }
                                server_state
                                    .new_connection_update(vi, &self.channels.network_state_send);

                                Some(vi)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Err(e) => {
                    self.send_debug_text("Stream Recv Error!");
                    None
                }
            }
        } else {
            None
        }
    }

    fn process_server_stream_data(&mut self, verified_index: usize) {
        if let RealtimeMediaTypeData::Server(server_state) = &mut self.type_data {
            let read_data = self.connections[verified_index]
                .main_recv
                .get_data_to_read();
            match self.connections[verified_index]
                .main_recv
                .get_message_type()
            {
                StreamMsgType::NewStateRequest => {
                    let potential_new_state = read_data[0];
                    // In future check if server will allow state change here!
                    server_state.client_states[verified_index].state = potential_new_state;

                    self.msg_send.refresh_send(StreamMsgType::ClientNewState);
                    let write_data = self.msg_send.get_data_to_write();
                    write_data[0] = verified_index as u8;
                    write_data[1] = server_state.client_states[verified_index].state;

                    self.msg_send.update_data_write(2);

                    let send_data = self.msg_send.get_data_to_send();
                    for conn in self.connections.iter_mut() {
                        conn.send_main_stream_data(
                            &mut self.endpoint,
                            send_data,
                            self.current_tick,
                        );
                    }

                    server_state
                        .state_change_update(verified_index, &self.channels.network_state_send);
                }
                StreamMsgType::TransferRequest => {
                    if self.connections[verified_index].transfer_recv.is_none() {
                        let transfer_size = usize::from_ne_bytes([
                            read_data[0],
                            read_data[1],
                            read_data[2],
                            0,
                            0,
                            0,
                            0,
                            0,
                        ]);
                        let media_transfer = RealtimeMediaTransfer {
                            data: Vec::new(),
                            size: transfer_size,
                            bytes_transfered: 0,
                        };
                        self.connections[verified_index].transfer_send = Some(media_transfer);

                        self.msg_send.refresh_send(StreamMsgType::TransferResponse);
                        let write_data = self.msg_send.get_data_to_write();
                        write_data[0] = 33;
                        self.msg_send.update_data_write(1);
                        let send_data = self.msg_send.get_data_to_send();
                        self.connections[verified_index].send_main_stream_data(
                            &mut self.endpoint,
                            send_data,
                            self.current_tick,
                        );

                        server_state.client_states[verified_index].state |= 1;
                        self.msg_send.refresh_send(StreamMsgType::ClientNewState);
                        let write_data = self.msg_send.get_data_to_write();
                        write_data[0] = verified_index as u8;
                        write_data[1] = server_state.client_states[verified_index].state;
                        self.msg_send.update_data_write(2);
                        let send_data = self.msg_send.get_data_to_send();
                        for conn in self.connections.iter_mut() {
                            conn.send_main_stream_data(
                                &mut self.endpoint,
                                send_data,
                                self.current_tick,
                            );
                        }

                        server_state
                            .state_change_update(verified_index, &self.channels.network_state_send);
                    }
                }
                StreamMsgType::TransferData => {
                    self.send_debug_text("Got Here!\n");
                    if self.connections[verified_index].transfer_recv.is_some() {
                        let mut done = false;
                        if self.connections[verified_index].recv_transfer_data() {
                            // Finished Receiving
                            if let RealtimeMediaTypeData::Server(server_state) = &mut self.type_data
                            {
                                done = true;
                                server_state.client_states[verified_index].state &= 0xFE;

                                self.msg_send.refresh_send(StreamMsgType::ClientNewState);
                                let write_data = self.msg_send.get_data_to_write();
                                write_data[0] = verified_index as u8;
                                write_data[1] = server_state.client_states[verified_index].state;
                                self.msg_send.update_data_write(2);
                                let send_data = self.msg_send.get_data_to_send();
                                for conn in self.connections.iter_mut() {
                                    conn.send_main_stream_data(
                                        &mut self.endpoint,
                                        send_data,
                                        self.current_tick,
                                    );
                                }

                                server_state.state_change_update(
                                    verified_index,
                                    &self.channels.network_state_send,
                                );
                                self.connections[verified_index].transfer_recv = None;
                            }
                        }
                        if done {
                            self.send_debug_text("Finished the Transfer!!!\n");
                        }
                    } else {
                        self.send_debug_text("Got Unexpected TransferData Messages!\n");
                    }
                }

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
                // StreamMsgType::NewClientAnnounce => {
                //     // Already handled by calling function since it only gets sent as the first message by a new client
                // }
                _ => {}
            }
        }
    }

    fn process_client_stream_data(&mut self, verified_index: usize) {
        if let RealtimeMediaTypeData::Client(client_handler) = &mut self.type_data {
            let read_data = self.connections[verified_index]
                .main_recv
                .get_data_to_read();
            match self.connections[verified_index]
                .main_recv
                .get_message_type()
            {
                StreamMsgType::ServerStateRefresh => {
                    // State Refresh
                    client_handler
                        .handle_state_refresh(read_data, &self.channels.network_state_send);
                }
                StreamMsgType::NewClient => {
                    client_handler.handle_new_client(read_data, &self.channels.network_state_send);
                }
                StreamMsgType::ClientNewState => {
                    client_handler
                        .handle_client_new_state(read_data, &self.channels.network_state_send);
                }
                StreamMsgType::TransferResponse => {
                    let id_byte = read_data[0];
                    if self.connections[verified_index].transfer_send.is_some() {
                        loop {
                            self.msg_send.refresh_send(StreamMsgType::TransferData);
                            let write_data = self.msg_send.get_data_to_write();
                            write_data[0] = id_byte;

                            if let Some(transfer_media) =
                                &mut self.connections[verified_index].transfer_send
                            {
                                let write_len = write_data.len() - 1;
                                let remaining_len =
                                    transfer_media.data.len() - transfer_media.bytes_transfered;
                                let min_len = cmp::min(write_len, remaining_len);
                                if min_len == 0 {
                                    break;
                                }
                                let transfer_end = min_len + transfer_media.bytes_transfered;
                                write_data[1..(1 + min_len)].copy_from_slice(
                                    &transfer_media.data
                                        [transfer_media.bytes_transfered..transfer_end],
                                );
                                transfer_media.bytes_transfered = transfer_end;
                                self.msg_send.update_data_write(min_len + 1);
                                //let info_string = format!("Data written: {}\n", min_len + 1);
                                //self.send_debug_text(info_string.as_str());
                            }

                            let send_data = self.msg_send.get_data_to_send();
                            match self.endpoint.send_stream_data(
                                self.connections[verified_index].id,
                                MAIN_STREAM_ID,
                                send_data,
                                false,
                            ) {
                                Ok((i_sends, d_sends)) => {
                                    self.connections[verified_index].last_activity_tick =
                                        self.current_tick;
                                    let info_string = format!("Sends: {} {}\n", i_sends, d_sends);
                                    self.send_debug_text(info_string.as_str());
                                }
                                Err(e) => match e {
                                    EndpointError::StreamSendFilled => {
                                        self.send_debug_text("Stream Send Filled!\n")
                                    }
                                    _ => self.send_debug_text("Generic Stream Send Err!\n"),
                                },
                            }
                        }

                        self.send_debug_text("Sent File Transfer!\n");
                    }
                }
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
    }

    fn handle_incoming_commands(&mut self) -> bool {
        loop {
            match self.channels.command_recv.try_recv() {
                Err(TryRecvError::Empty) => return false,
                Ok(NetworkCommand::Stop(int)) => return true,
                Ok(NetworkCommand::Client(client_cmd)) => {
                    self.handle_client_command(client_cmd);
                }
                Ok(NetworkCommand::Server(server_cmd)) => {
                    self.handle_server_command(server_cmd);
                }
                Err(_) => return true, // Other recv errors
            }
        }
    }

    fn handle_client_command(&mut self, cmd: ClientCommand) {
        if let RealtimeMediaTypeData::Client(client_handler) = &mut self.type_data {
            match cmd {
                ClientCommand::StateChange(new_state_requested) => {
                    if !self.connections.is_empty() {
                        self.msg_send.refresh_send(StreamMsgType::NewStateRequest);
                        let write_data = self.msg_send.get_data_to_write();
                        write_data[0] = new_state_requested;
                        self.msg_send.update_data_write(1);
                        let send_data = self.msg_send.get_data_to_send();
                        self.connections[0].send_main_stream_data(
                            &mut self.endpoint,
                            send_data,
                            self.current_tick,
                        );
                    }
                }
                ClientCommand::ServerConnect(server_address) => {
                    let _ = self
                        .endpoint
                        .add_client_connection(server_address, SERVER_NAME);
                }
                ClientCommand::MusicTransfer(od) => {
                    if !self.connections.is_empty() {
                        let transfer_data = od.to_bytes();
                        let transfer_size = transfer_data.len();

                        let info_string = format!("Data transfer size: {}\n", transfer_size);
                        self.send_debug_text(info_string.as_str());

                        let media_transfer = RealtimeMediaTransfer {
                            data: transfer_data,
                            size: transfer_size,
                            bytes_transfered: 0,
                        };
                        self.connections[0].transfer_send = Some(media_transfer);

                        self.msg_send.refresh_send(StreamMsgType::TransferRequest);
                        let write_data = self.msg_send.get_data_to_write();

                        let size_in_bytes = transfer_size.to_ne_bytes();
                        write_data[0] = size_in_bytes[0];
                        write_data[1] = size_in_bytes[1];
                        write_data[2] = size_in_bytes[2];
                        write_data[3] = 1; // Indicating for deletion after fully received

                        self.msg_send.update_data_write(4);
                        let send_data = self.msg_send.get_data_to_send();
                        self.connections[0].send_main_stream_data(
                            &mut self.endpoint,
                            send_data,
                            self.current_tick,
                        );
                    }

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
    }

    fn handle_server_command(&mut self, cmd: ServerCommand) {
        if let RealtimeMediaTypeData::Server(server_state) = &mut self.type_data {
            match cmd {
                ServerCommand::ConnectionClose(probable_index) => {}
            }
        }
    }
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
    user_name: [u8; MAX_CHAR_LENGTH * 4],
    user_name_len: usize,
    state: u8, // Bit State [transferingMusic, serverMusicConnected, voiceChatConnected, voiceChatLoopback]
}

impl ClientState {
    fn new(user_name_bytes: &[u8]) -> Option<Self> {
        let mut cs = ClientState {
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
    client_states: Vec<ClientState>,
    music_storage: Vec<MusicStorage>,
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
            music_storage: Vec::new(),
        }
    }

    fn add_connection_state(&mut self, verified_index: usize, read_data: &[u8]) -> bool {
        if verified_index == self.client_states.len() {
            let username_len = read_data[0] as usize;
            if let Some(cs) = ClientState::new(&read_data[1..username_len + 1]) {
                self.client_states.push(cs);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn remove_connection_state(&mut self, verified_index: usize) -> bool {
        if verified_index < self.client_states.len() {
            self.client_states.remove(verified_index);
            true
        } else {
            false
        }
    }

    fn create_refresh_data(&mut self, write_data: &mut [u8]) -> usize {
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
        write_size
    }

    fn create_new_client_data(&mut self, verified_index: usize, write_data: &mut [u8]) -> usize {
        let cs = &self.client_states[verified_index];

        write_data[0] = cs.user_name_len as u8;
        let mut write_size = 1;
        write_data[write_size..(write_size + cs.user_name_len)]
            .copy_from_slice(&cs.user_name[..cs.user_name_len]);
        write_size += cs.user_name_len;

        write_data[write_size] = cs.state;
        write_size += 1;

        write_size
    }

    fn create_state_change_data(&mut self, verified_index: usize, write_data: &mut [u8]) -> usize {
        let cs = &self.client_states[verified_index];

        write_data[0] = verified_index as u8;
        write_data[1] = cs.state;

        2
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

struct ClientHandler {
    user_name: String,
    //focus_id: u64,
}

impl ClientHandler {
    fn new(user_name: String) -> Self {
        ClientHandler {
            user_name,
            //focus_id: 0,
        }
    }

    fn create_announce_data(&self, write_data: &mut [u8]) -> usize {
        let mut start_index = 1;
        for (c_ind, c) in self.user_name.chars().enumerate() {
            if c_ind >= MAX_CHAR_LENGTH {
                break;
            }

            let new_start_index = start_index + c.len_utf8();
            let c_subslice = &mut write_data[start_index..new_start_index];
            c.encode_utf8(c_subslice);
            start_index = new_start_index;
        }
        write_data[0] = (start_index - 1) as u8;
        start_index
    }

    fn handle_state_refresh(&self, read_data: &[u8], state_send: &Sender<NetworkStateMessage>) {
        let conn_ind = read_data[1] as usize;

        let mut name_end: usize = (read_data[2] + 3).into();
        let server_name = u8_to_str(&read_data[3..name_end]);
        let name_update = NetworkStateMessage::ServerNameChange(server_name);
        let _ = state_send.send(name_update);

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
        let _ = state_send.send(state_update);
    }

    fn handle_new_client(&self, read_data: &[u8], state_send: &Sender<NetworkStateMessage>) {
        let name_end: usize = (read_data[0] + 1).into();
        let client_name = u8_to_str(&read_data[1..name_end]);
        let new_conn = NetworkStateMessage::NewConnection((client_name, read_data[name_end]));
        let _ = state_send.send(new_conn);
    }

    fn handle_client_new_state(&self, read_data: &[u8], state_send: &Sender<NetworkStateMessage>) {
        let conn_pos = read_data[0] as usize;
        let new_state = read_data[1];

        let new_conn = NetworkStateMessage::StateChange((conn_pos, new_state));
        let _ = state_send.send(new_conn);
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
            true => quic::SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port, 0, 0)),
            false => quic::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)),
        },
        None => quic::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)),
    };

    let server_endpoint = match Endpoint::new_server(bind_address, ALPN_NAME, CERT_PATH, PKEY_PATH)
    {
        Ok(endpoint) => endpoint,
        Err(err) => {
            let _ = channels
                .network_debug_send
                .send("Server Endpoint Creation Error!\n".to_string());
            // Can add more detailed print here later
            return;
        }
    };

    let server_state = ServerState::new(server_name);
    let rm_type = RealtimeMediaTypeData::Server(server_state);

    let mut realtime_media_handler = RealtimeMediaEndpoint::new(server_endpoint, channels, rm_type);
    realtime_media_handler.send_debug_text("Starting Server Network!\n");

    realtime_media_handler.run_event_loop();

    // Eventual Friendly Server Cleanup Here

    realtime_media_handler.send_debug_text("Server Network Thread Exiting\n");
}

pub fn client_thread(
    server_address: SocketAddr,
    user_name: String,
    channels: NetworkThreadChannels,
) {
    let bind_address = match server_address.is_ipv6() {
        true => quic::SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0)),
        false => quic::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)),
    };

    let client_endpoint = match Endpoint::new_client_with_first_connection(
        bind_address,
        ALPN_NAME,
        CERT_PATH,
        server_address,
        SERVER_NAME,
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

    let rm_type = RealtimeMediaTypeData::Client(ClientHandler::new(user_name));
    let mut realtime_media_handler = RealtimeMediaEndpoint::new(client_endpoint, channels, rm_type);
    realtime_media_handler.send_debug_text("Starting Client Network!\n");

    'client_thread: loop {
        // If
        if realtime_media_handler.run_event_loop() {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(50));
                if realtime_media_handler.handle_incoming_commands() {
                    break 'client_thread;
                }
                if realtime_media_handler.endpoint.get_num_connections() > 0 {
                    break;
                }
            }
        } else {
            break;
        }
    }

    // Eventual Friendly Client Cleanup Here

    realtime_media_handler.send_debug_text("Client Network Thread Exiting!\n");
}
