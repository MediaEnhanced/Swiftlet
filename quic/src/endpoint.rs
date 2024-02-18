//Media Enhanced Swiftlet Quic Rust Library for Real-time Internet Communications
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

use crate::SocketAddr;
use std::time::{Duration, Instant};

use ring::rand::*;

mod udp;
use udp::{SocketError, UdpSocket};

mod connection;
use connection::{Connection, RecvResult, TimeoutResult};

/// The Endpoint Configuration Structure
pub struct Config {
    /// The quic connection idle timeout in milliseconds
    pub idle_timeout_in_ms: u64,

    /// The quic connection bidirectional stream receive buffer length in bytes
    /// These streams are intended for communicating reliable information
    /// Most applications should probably set this to a multiple of 65536
    pub reliable_stream_buffer: u64,

    /// The quic connection unidirectional stream receive buffer length in bytes
    /// These streams are intended for real-time unreliable information
    /// Most applications should probably set this to a multiple of 65536
    pub unreliable_stream_buffer: u64,

    /// The keep alive timeout duration
    /// If there is a value and the duration has passed since the quic connection had recieved anything
    /// the quic connection will send out a PING to try and keep the connection alive
    pub keep_alive_timeout: Option<Duration>,

    /// The initial main stream recieve buffer size
    /// This could be set to the max size of the expected data to process if there is enough RAM
    pub initial_main_recv_size: usize,

    /// The number of bytes to receive on the main stream before calling main_stream_recv for the first time
    pub main_recv_first_bytes: usize,

    /// The initial background stream recieve buffer size
    /// This could be set to the max size of the expected data to process if there is enough RAM
    pub initial_background_recv_size: usize,

    /// The number of bytes to receive on the background stream before calling main_stream_recv for the first time
    pub background_recv_first_bytes: usize,
}

/// The Quic Endpoint structure
pub struct Endpoint {
    udp: UdpSocket,
    max_payload_size: usize,
    local_addr: SocketAddr,
    connection_config: connection::Config,
    next_connection_id: u64,
    connections: Vec<Connection>,
    rand: SystemRandom,
    config: Config,
    is_server: bool,
    conn_id_seed_key: ring::hmac::Key, // Value matters ONLY if is_server is true
}

/// A Connection ID structure to communicate with the endpoint about a specific connection
/// Should be updated so that endpoint function calls are more efficient
pub struct ConnectionId {
    id: u64,
    probable_index: usize,
}

impl PartialEq for ConnectionId {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Clone for ConnectionId {
    fn clone(&self) -> Self {
        ConnectionId {
            id: self.id,
            probable_index: self.probable_index,
        }
    }
}

impl ConnectionId {
    // Check if this function gets inlined in the future

    /// A way to update the connection id
    pub fn update(&mut self, other: &Self) {
        self.probable_index = other.probable_index;
    }
}

/// Endpoint Errors
pub enum EndpointError {
    /// Error with the UDP socket creation
    SocketCreation,
    /// Error with the Quic Config Creation
    ConfigCreation,
    /// Error with creating or using the randomness structure / functions
    Randomness,
    /// Error trying to perform a client Endpoint operation on a server Endpoint
    IsServer,
    /// Error creating a connection
    ConnectionCreation,
    /// Error closing a connection
    ConnectionClose,
    /// Error getting send data from a connection
    ConnectionSend,
    /// Error sending data on the UDP socket
    SocketSend,
    /// Error receiving data on the UDP socket
    SocketRecv,

    // Error receiving too much data
    //RecvTooMuchData,
    /// Error having a connection process the received data
    ConnectionRecv,
    /// Cannot find connection from Connection ID
    ConnectionNotFound,
    /// Error finishing the connection establishment process and stream creation
    StreamCreation,
    /// Error sending out a PING
    ConnectionPing,
    /// Error sending data on the stream
    StreamSend,
    /// Error receiving data from the stream
    StreamRecv,
}

pub(super) enum EndpointEvent {
    NextTick,
    ConnectionClosing(ConnectionId),
    ConnectionClosed(ConnectionId),
    AlreadyHandled,
    ReceivedData,
    DoneReceiving,
    NoUpdate,
    EstablishedOnce(ConnectionId),
    MainStreamReceived(ConnectionId),
    BackgroundStreamReceived(ConnectionId),
    //StreamReceivedData(StreamReadable),
}

impl Endpoint {
    // Maybe combine new_server and new_client together... but there is hardly any real benefit (and sacrifices readability)

    /// Create a quic server Endpoint
    pub fn new_server(
        bind_addr: SocketAddr,
        alpn: &[u8],
        cert_path: &str,
        pkey_path: &str,
        config: Config,
    ) -> Result<Self, EndpointError> {
        if let Ok((socket_mgr, local_addr)) = UdpSocket::new(bind_addr) {
            let max_payload_size = udp::TARGET_MAX_DATAGRAM_SIZE;

            let connection_config = match Connection::create_config(
                &[alpn],
                cert_path,
                Some(pkey_path),
                config.idle_timeout_in_ms,
                max_payload_size,
                config.reliable_stream_buffer,
                config.unreliable_stream_buffer,
            ) {
                Ok(cfg) => cfg,
                Err(_) => return Err(EndpointError::ConfigCreation),
            };

            let rand = SystemRandom::new();
            let conn_id_seed_key = match ring::hmac::Key::generate(ring::hmac::HMAC_SHA256, &rand) {
                Ok(key) => key,
                Err(_) => return Err(EndpointError::Randomness),
            };

            let endpoint_manager = Endpoint {
                udp: socket_mgr,
                max_payload_size,
                local_addr,
                connection_config,
                next_connection_id: 1,
                connections: Vec::new(),
                rand,
                config,
                is_server: true,
                conn_id_seed_key,
            };

            Ok(endpoint_manager)
        } else {
            Err(EndpointError::SocketCreation)
        }
    }

    /// Create a quic client Endpoint
    pub fn new_client(
        bind_addr: SocketAddr,
        alpn: &[u8],
        cert_path: &str,
        config: Config,
    ) -> Result<Self, EndpointError> {
        if let Ok((socket_mgr, local_addr)) = UdpSocket::new(bind_addr) {
            let max_payload_size = udp::TARGET_MAX_DATAGRAM_SIZE;

            let connection_config = match Connection::create_config(
                &[alpn],
                cert_path,
                None,
                config.idle_timeout_in_ms,
                max_payload_size,
                config.reliable_stream_buffer,
                config.unreliable_stream_buffer,
            ) {
                Ok(cfg) => cfg,
                Err(_) => return Err(EndpointError::ConfigCreation),
            };

            let rand = SystemRandom::new();
            // Following value doesn't matter but its useful for making sure the SystemRandom is working... I guess
            let conn_id_seed_key = match ring::hmac::Key::generate(ring::hmac::HMAC_SHA256, &rand) {
                Ok(key) => key,
                Err(_) => return Err(EndpointError::Randomness),
            };

            let endpoint_manager = Endpoint {
                udp: socket_mgr,
                max_payload_size,
                local_addr,
                connection_config,
                next_connection_id: 1,
                connections: Vec::new(),
                rand,
                config,
                is_server: false,
                conn_id_seed_key,
            };

            Ok(endpoint_manager)
        } else {
            Err(EndpointError::SocketCreation)
        }
    }

    #[inline]
    fn find_connection_from_cid(&self, cid: &ConnectionId) -> Option<usize> {
        if cid.probable_index < self.connections.len()
            && self.connections[cid.probable_index].matches_id(cid.id)
        {
            Some(cid.probable_index)
        } else {
            // To be changed to binary search later
            self.connections
                .iter()
                .position(|conn| conn.matches_id(cid.id))
        }
    }

    fn send(&mut self, verified_index: usize) -> Result<(u64, u64), EndpointError> {
        let mut immediate_sends = 0;
        let mut delayed_sends = 0;
        loop {
            let packet_data = self.udp.get_packet_data();
            match self.connections[verified_index].get_next_send_packet(packet_data) {
                Ok(Some((packet_len, to_addr, instant))) => {
                    match self.udp.send_packet(packet_len, to_addr, instant) {
                        Ok(true) => {
                            immediate_sends += 1;
                        }
                        Ok(false) => {
                            delayed_sends += 1;
                        }
                        Err(_) => {
                            return Err(EndpointError::SocketSend);
                        }
                    }
                }
                Ok(None) => {
                    return Ok((immediate_sends, delayed_sends));
                }
                Err(_) => {
                    return Err(EndpointError::ConnectionSend);
                }
            }
        }
    }

    // This is different than closing the connection
    fn remove_connection(&mut self, verified_index: usize) {
        // Pretty confident that this is truly all there really is to it:
        self.connections.remove(verified_index);
    }

    /// Add a connection for a client Endpoint
    pub fn add_client_connection(
        &mut self,
        peer_addr: SocketAddr,
        server_name: &str,
    ) -> Result<(), EndpointError> {
        if !self.is_server {
            let mut scid_data = Connection::get_empty_cid();
            if self.rand.fill(&mut scid_data).is_err() {
                return Err(EndpointError::Randomness);
            }

            match Connection::new(
                self.next_connection_id,
                peer_addr,
                Some(server_name),
                self.local_addr,
                &scid_data,
                &mut self.connection_config,
                None,
            ) {
                Ok(conn_mgr) => {
                    self.next_connection_id += 1;
                    self.connections.push(conn_mgr);
                    let verified_index = self.connections.len() - 1;
                    self.send(verified_index)?;
                    Ok(())
                }
                Err(_) => Err(EndpointError::ConnectionCreation),
            }
        } else {
            Err(EndpointError::IsServer)
        }
    }

    /// Create a quic client Endpoint with an initial connection
    pub fn new_client_with_first_connection(
        bind_addr: SocketAddr,
        alpn: &[u8],
        cert_path: &str,
        peer_addr: SocketAddr,
        server_name: &str,
        config: Config,
    ) -> Result<Self, EndpointError> {
        let mut endpoint_mgr = Endpoint::new_client(bind_addr, alpn, cert_path, config)?;

        endpoint_mgr.add_client_connection(peer_addr, server_name)?;

        Ok(endpoint_mgr)
    }

    /// Get the number of connections that the Endpoint is managing
    #[inline]
    pub fn get_num_connections(&self) -> usize {
        self.connections.len()
    }

    /// Update the keep alive duration time
    /// Will disable the keep alive functionality if set to None
    #[inline]
    pub fn update_keep_alive_duration(&mut self, duration_opt: Option<Duration>) {
        self.config.keep_alive_timeout = duration_opt;
    }

    fn keep_alive(&mut self) -> Result<u64, EndpointError> {
        let mut num_pings = 0;
        if let Some(duration) = self.config.keep_alive_timeout {
            let before_instant = Instant::now() - duration;
            for verified_index in 0..self.connections.len() {
                match self.connections[verified_index].send_ping_if_before_instant(before_instant) {
                    Ok(false) => {}
                    Ok(true) => {
                        self.send(verified_index)?;
                        num_pings += 1;
                    }
                    Err(_) => {
                        return Err(EndpointError::ConnectionPing);
                    }
                }
            }
        }
        Ok(num_pings)
    }

    pub(super) fn get_next_event(
        &mut self,
        next_tick_instant: Instant,
    ) -> Result<EndpointEvent, EndpointError> {
        let mut next_instant = if next_tick_instant > Instant::now() {
            next_tick_instant
        } else {
            self.keep_alive()?;
            return Ok(EndpointEvent::NextTick);
        };
        let mut conn_timeout_opt = None;

        match self.udp.send_check() {
            Ok(send_count) => {
                if send_count > 0 && next_tick_instant <= Instant::now() {
                    self.keep_alive()?;
                    return Ok(EndpointEvent::NextTick);
                }
            }
            Err(_) => {
                return Err(EndpointError::SocketSend);
            }
        }

        for verified_index in 0..self.connections.len() {
            match self.connections[verified_index].handle_possible_timeout() {
                TimeoutResult::Nothing(Some(timeout_instant)) => {
                    if timeout_instant < next_instant {
                        next_instant = timeout_instant;
                        conn_timeout_opt = Some(verified_index);
                    }
                }
                TimeoutResult::Closed(conn_id) => {
                    self.remove_connection(verified_index);
                    return Ok(EndpointEvent::ConnectionClosed(ConnectionId {
                        id: conn_id,
                        probable_index: verified_index,
                    }));
                }
                TimeoutResult::Draining(conn_id) => {
                    return Ok(EndpointEvent::ConnectionClosing(ConnectionId {
                        id: conn_id,
                        probable_index: verified_index,
                    }));
                }
                TimeoutResult::Happened => {
                    self.send(verified_index)?;

                    match self.udp.send_check() {
                        Ok(_) => {
                            if next_instant <= Instant::now() {
                                if let Some(vi) = conn_timeout_opt {
                                    match self.connections[vi].handle_possible_timeout() {
                                        TimeoutResult::Happened => {
                                            self.send(vi)?;
                                            return Ok(EndpointEvent::AlreadyHandled);
                                        }
                                        TimeoutResult::Closed(conn_id) => {
                                            self.remove_connection(vi);
                                            return Ok(EndpointEvent::ConnectionClosed(
                                                ConnectionId {
                                                    id: conn_id,
                                                    probable_index: verified_index,
                                                },
                                            ));
                                        }
                                        TimeoutResult::Draining(conn_id) => {
                                            return Ok(EndpointEvent::ConnectionClosing(
                                                ConnectionId {
                                                    id: conn_id,
                                                    probable_index: verified_index,
                                                },
                                            ));
                                        }
                                        _ => {
                                            return Ok(EndpointEvent::AlreadyHandled);
                                        }
                                    }
                                } else {
                                    self.keep_alive()?;
                                    return Ok(EndpointEvent::NextTick);
                                }
                            }
                        }
                        Err(_) => {
                            return Err(EndpointError::SocketSend);
                        }
                    }
                }
                _ => {}
            }
        }

        let mut send_check_timeout = false;
        if let Some(next_send_check_instant) = self.udp.next_send_instant() {
            if next_send_check_instant < next_instant {
                next_instant = next_send_check_instant;
                send_check_timeout = true;
            }
        }

        let sleep_duration = next_instant.duration_since(Instant::now());
        if self.udp.sleep_till_recv_data(sleep_duration) {
            Ok(EndpointEvent::ReceivedData)
        } else if send_check_timeout {
            match self.udp.send_check() {
                Ok(_) => Ok(EndpointEvent::AlreadyHandled),
                Err(_) => Err(EndpointError::SocketSend),
            }
        } else if let Some(vi) = conn_timeout_opt {
            match self.connections[vi].handle_possible_timeout() {
                TimeoutResult::Happened => {
                    self.send(vi)?;
                    Ok(EndpointEvent::AlreadyHandled)
                }
                TimeoutResult::Closed(conn_id) => {
                    self.remove_connection(vi);
                    Ok(EndpointEvent::ConnectionClosed(ConnectionId {
                        id: conn_id,
                        probable_index: vi,
                    }))
                }
                TimeoutResult::Draining(conn_id) => {
                    Ok(EndpointEvent::ConnectionClosing(ConnectionId {
                        id: conn_id,
                        probable_index: vi,
                    }))
                }
                _ => Ok(EndpointEvent::AlreadyHandled),
            }
        } else {
            self.keep_alive()?;
            Ok(EndpointEvent::NextTick)
        }
    }

    pub(super) fn recv(&mut self) -> Result<EndpointEvent, EndpointError> {
        match self.udp.get_next_recv_data() {
            Ok((recv_data, from_addr)) => {
                // Only bother to look at a datagram that is less than or equal to the target
                if recv_data.len() <= self.max_payload_size {
                    if let Some((dcid, new_conn_possibility)) =
                        Connection::recv_header_analyze(recv_data, self.is_server)
                    {
                        let mut verified_index_opt = self
                            .connections
                            .iter()
                            .position(|conn| conn.matches_dcid(&dcid));

                        if verified_index_opt.is_none() && new_conn_possibility && self.is_server {
                            let tag = ring::hmac::sign(&self.conn_id_seed_key, &dcid);
                            let scid_data = tag.as_ref();

                            let writer_opt = match self.connections.len() {
                                0 => match std::fs::File::create("security/key.log") {
                                    Ok(file) => Some(Box::new(file)),
                                    Err(_) => None,
                                },
                                _ => None,
                            };

                            match Connection::new(
                                self.next_connection_id,
                                from_addr,
                                None,
                                self.local_addr,
                                scid_data,
                                &mut self.connection_config,
                                writer_opt,
                            ) {
                                Ok(conn_mgr) => {
                                    self.next_connection_id += 1;
                                    verified_index_opt = Some(self.connections.len());
                                    self.connections.push(conn_mgr);
                                }
                                Err(_) => return Ok(EndpointEvent::NoUpdate),
                            }
                        }

                        if let Some(verified_index) = verified_index_opt {
                            match self.connections[verified_index]
                                .recv_data_process(recv_data, from_addr)
                            {
                                Ok(RecvResult::MainStreamReadable(conn_id)) => {
                                    self.send(verified_index)?;
                                    Ok(EndpointEvent::MainStreamReceived(ConnectionId {
                                        id: conn_id,
                                        probable_index: verified_index,
                                    }))
                                }
                                Ok(RecvResult::BkgdStreamReadable(conn_id)) => {
                                    self.send(verified_index)?;
                                    Ok(EndpointEvent::BackgroundStreamReceived(ConnectionId {
                                        id: conn_id,
                                        probable_index: verified_index,
                                    }))
                                }
                                Ok(RecvResult::Closed(conn_id)) => {
                                    self.remove_connection(verified_index);
                                    Ok(EndpointEvent::ConnectionClosed(ConnectionId {
                                        id: conn_id,
                                        probable_index: verified_index,
                                    }))
                                }
                                Ok(RecvResult::Draining(conn_id)) => {
                                    Ok(EndpointEvent::ConnectionClosing(ConnectionId {
                                        id: conn_id,
                                        probable_index: verified_index,
                                    }))
                                }
                                Ok(RecvResult::Established(conn_id)) => {
                                    self.send(verified_index)?;

                                    // let mut main_recv_data_old =
                                    //     Vec::with_capacity(self.config.initial_main_recv_size);
                                    // main_recv_data_old
                                    //     .resize(self.config.initial_main_recv_size, 0);
                                    let main_recv_data =
                                        vec![0; self.config.initial_main_recv_size]; // Faster according to clippy and github issue discussion but unsure why...?

                                    let background_recv_data =
                                        vec![0; self.config.initial_background_recv_size];

                                    if self.connections[verified_index]
                                        .finish_establishment(
                                            main_recv_data,
                                            self.config.main_recv_first_bytes,
                                            background_recv_data,
                                            self.config.background_recv_first_bytes,
                                        )
                                        .is_err()
                                    {
                                        return Err(EndpointError::StreamCreation);
                                    }

                                    Ok(EndpointEvent::EstablishedOnce(ConnectionId {
                                        id: conn_id,
                                        probable_index: verified_index,
                                    }))
                                }
                                Ok(RecvResult::StreamReadable(_)) => {
                                    self.send(verified_index)?;
                                    //Maybe error out here and close connection in future?

                                    // let stream_readable = StreamReadable {
                                    //     stream_id,
                                    //     conn_id,
                                    //     probable_index: verified_index,
                                    // };

                                    // Ok(EndpointEvent::StreamReceivedData(stream_readable))

                                    Ok(EndpointEvent::NoUpdate)
                                }
                                Ok(_) => {
                                    self.send(verified_index)?;
                                    Ok(EndpointEvent::NoUpdate)
                                }

                                Err(_) => Err(EndpointError::ConnectionRecv),
                            }
                        } else {
                            Ok(EndpointEvent::NoUpdate)
                        }
                    } else {
                        Ok(EndpointEvent::NoUpdate)
                    }
                } else {
                    //Err(EndpointError::RecvTooMuchData)
                    Ok(EndpointEvent::NoUpdate)
                }
            }
            Err(SocketError::RecvBlocked) => Ok(EndpointEvent::DoneReceiving),
            Err(_) => Err(EndpointError::SocketRecv),
        }
    }

    /// Close a connection with a given error code number
    pub fn close_connection(
        &mut self,
        cid: &ConnectionId,
        error_code: u64,
    ) -> Result<bool, EndpointError> {
        if let Some(verified_index) = self.find_connection_from_cid(cid) {
            match self.connections[verified_index].close(error_code, b"reason") {
                Ok(_) => match self.send(verified_index) {
                    Ok(_) => Ok(true),
                    Err(e) => Err(e),
                },
                Err(_) => Err(EndpointError::ConnectionClose),
            }
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }

    /// Send data over the main stream
    /// A reminder that the Endpoint connection will be taking ownership of the data so it can be sent out when possible
    pub fn main_stream_send(
        &mut self,
        cid: &ConnectionId,
        send_data: Vec<u8>,
    ) -> Result<(u64, u64), EndpointError> {
        if let Some(verified_index) = self.find_connection_from_cid(cid) {
            match self.connections[verified_index].main_stream_send(send_data) {
                Ok(_) => self.send(verified_index),
                Err(_) => Err(EndpointError::StreamSend),
            }
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }

    pub(super) fn main_stream_recv(
        &mut self,
        cid: &ConnectionId,
    ) -> Result<(Option<usize>, Option<Vec<u8>>), EndpointError> {
        if let Some(verified_index) = self.find_connection_from_cid(cid) {
            match self.connections[verified_index].main_stream_read() {
                Ok((x, y)) => Ok((x, y)),
                Err(_) => Err(EndpointError::StreamSend),
            }
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }

    pub(super) fn main_stream_set_target(
        &mut self,
        cid: &ConnectionId,
        next_target: usize,
        vec_data_return: Vec<u8>,
    ) -> Result<(), EndpointError> {
        if let Some(verified_index) = self.find_connection_from_cid(cid) {
            self.connections[verified_index].main_stream_next_target(next_target, vec_data_return);
            Ok(())
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }

    /// Send data over the background stream
    /// A reminder that the Endpoint connection will be taking ownership of the data so it can be sent out when possible
    pub fn background_stream_send(
        &mut self,
        cid: &ConnectionId,
        send_data: Vec<u8>,
    ) -> Result<(u64, u64), EndpointError> {
        if let Some(verified_index) = self.find_connection_from_cid(cid) {
            match self.connections[verified_index].bkgd_stream_send(send_data) {
                Ok(_) => self.send(verified_index),
                Err(_) => Err(EndpointError::StreamSend),
            }
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }

    pub(super) fn background_stream_recv(
        &mut self,
        cid: &ConnectionId,
    ) -> Result<(Option<usize>, Option<Vec<u8>>), EndpointError> {
        if let Some(verified_index) = self.find_connection_from_cid(cid) {
            match self.connections[verified_index].bkgd_stream_read() {
                Ok((x, y)) => Ok((x, y)),
                Err(_) => Err(EndpointError::StreamSend),
            }
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }

    pub(super) fn background_stream_set_target(
        &mut self,
        cid: &ConnectionId,
        next_target: usize,
        vec_data_return: Vec<u8>,
    ) -> Result<(), EndpointError> {
        if let Some(verified_index) = self.find_connection_from_cid(cid) {
            self.connections[verified_index].bkgd_stream_next_target(next_target, vec_data_return);
            Ok(())
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }
}
