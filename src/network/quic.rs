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

pub(super) use std::net::SocketAddr;
use std::time::Instant;

use ring::rand::*;

mod udp;
use udp::{SocketError, SocketManager};

mod connection;
use connection::{Config, ConnectionManager, Error, Status};

fn create_config(
    alpn: &[u8],
    cert_path: &str,
    pkey_path_option: Option<&str>,
    dgram_queue_len_option: Option<usize>, // To be used in the future... maybe
) -> Result<Config, EndpointError> {
    let mut config = match ConnectionManager::get_default_config() {
        Ok(cfg) => {
            cfg // A quiche Config with default values
        }
        Err(_) => {
            return Err(EndpointError::ConfigCreation);
        }
    };

    if let Some(pkey_path) = pkey_path_option {
        if config.load_cert_chain_from_pem_file(cert_path).is_err() {
            // More specific errors here in future
            return Err(EndpointError::ConfigCreation);
        }

        if config.load_priv_key_from_pem_file(pkey_path).is_err() {
            // More specific errors here in future
            return Err(EndpointError::ConfigCreation);
        }
        config.verify_peer(false);
        config.set_initial_max_streams_bidi(1); // Should be 1 here for server?

        // Enable datagram frames for unreliable realtime data to be sent
        //let dgram_queue_len = MAX_DATAGRAM_SIZE * (MAX_SERVER_CONNS as usize) * 2;
        config.log_keys();
    } else {
        // Temporary solution for client to verify certificate
        if config.load_verify_locations_from_file(cert_path).is_err() {
            // More specific errors here in future
            return Err(EndpointError::ConfigCreation);
        }

        config.verify_peer(true);
        config.set_initial_max_streams_bidi(1);

        //let dgram_queue_len = MAX_DATAGRAM_SIZE * (MAX_SERVER_CONNS as usize) || MAX_DATAGRAM_SIZE;
    }

    // Enable datagram frames for unreliable realtime data to be sent
    // Needs to be fixed in the future
    if let Some(dgram_queue_len) = dgram_queue_len_option {
        config.enable_dgram(true, dgram_queue_len * 10, dgram_queue_len);
    }

    if config.set_application_protos(&[alpn]).is_err() {
        return Err(EndpointError::ConfigCreation);
    }

    config.set_max_idle_timeout(5000); // Use a timeout of infinite when this line is commented out

    config.set_max_recv_udp_payload_size(udp::TARGET_MAX_DATAGRAM_SIZE);
    config.set_max_send_udp_payload_size(udp::TARGET_MAX_DATAGRAM_SIZE);
    config.set_initial_max_data(16_777_216); // 16 MiB
    config.set_initial_max_stream_data_bidi_local(4_194_304); // 4 MiB
    config.set_initial_max_stream_data_bidi_remote(4_194_304); // 4 MiB

    config.set_initial_max_streams_uni(3);
    config.set_initial_max_stream_data_uni(4_194_304); // 4 MiB

    config.set_disable_active_migration(true); // Temporary

    Ok(config)
}

enum NextInstantType {
    NextTick,
    DelayedSend,
    ConnectionTimeout(usize), // usize index always valid...? double check logic later
}

pub struct StreamReadable {
    pub stream_id: u64,
    pub conn_id: u64,
    probable_index: usize,
}

// QUIC Endpoint
pub struct Endpoint {
    udp: SocketManager,
    udp_read_data: [u8; udp::MAX_UDP_LENGTH],
    local_addr: SocketAddr,
    config: connection::Config,
    next_connection_id: u64,
    connections: Vec<ConnectionManager>,
    rand: SystemRandom,
    next_tick_instant: Instant,
    is_server: bool,
    conn_id_seed_key: ring::hmac::Key, // Value matters ONLY if is_server is true
}

pub enum EndpointError {
    SocketCreation,
    ConfigCreation,
    Randomness,
    IsServer,
    ConnectionCreation,
    ConnectionSend,
    SocketSend,
    SocketRecv,
    RecvTooMuchData,
    ConnectionRecv,
    ConnectionNotFound,
    StreamCreation,
    StreamSend,
    StreamSendFilled,
    StreamRecv,
}

pub enum EndpointEvent {
    NoUpdate,
    NextTick,
    PotentiallyReceivedData,
    DoneReceiving,
    NewConnectionStarted,
    FinishedConnectingOnce(u64),
    ConnectionClosed(u64),
    StreamReceivedData(StreamReadable),
}

impl Endpoint {
    // Maybe combine new_server and new_client together... but there is hardly any real benefit (and sacrifices readability)
    pub fn new_server(
        bind_addr: SocketAddr,
        alpn: &[u8],
        cert_path: &str,
        pkey_path: &str,
    ) -> Result<Self, EndpointError> {
        if let Ok((socket_mgr, local_addr)) = SocketManager::new(bind_addr) {
            let config = match create_config(alpn, cert_path, Some(pkey_path), Some(3)) {
                Ok(cfg) => cfg,
                Err(err) => return Err(EndpointError::SocketCreation),
            };

            let rand = SystemRandom::new();
            let conn_id_seed_key = match ring::hmac::Key::generate(ring::hmac::HMAC_SHA256, &rand) {
                Ok(key) => key,
                Err(_) => return Err(EndpointError::Randomness),
            };

            let endpoint_manager = Endpoint {
                udp: socket_mgr,
                udp_read_data: [0; udp::MAX_UDP_LENGTH],
                local_addr,
                config,
                next_connection_id: 1,
                connections: Vec::new(),
                rand,
                next_tick_instant: Instant::now(),
                is_server: true,
                conn_id_seed_key,
            };

            Ok(endpoint_manager)
        } else {
            Err(EndpointError::SocketCreation)
        }
    }

    pub fn new_client(
        bind_addr: SocketAddr,
        alpn: &[u8],
        cert_path: &str,
    ) -> Result<Self, EndpointError> {
        if let Ok((socket_mgr, local_addr)) = SocketManager::new(bind_addr) {
            let config = match create_config(alpn, cert_path, None, Some(3)) {
                Ok(cfg) => cfg,
                Err(err) => return Err(EndpointError::SocketCreation),
            };

            let rand = SystemRandom::new();
            // Following value doesn't matter but its useful for making sure the SystemRandom is working... I guess
            let conn_id_seed_key = match ring::hmac::Key::generate(ring::hmac::HMAC_SHA256, &rand) {
                Ok(key) => key,
                Err(_) => return Err(EndpointError::Randomness),
            };

            let endpoint_manager = Endpoint {
                udp: socket_mgr,
                udp_read_data: [0; udp::MAX_UDP_LENGTH],
                local_addr,
                config,
                next_connection_id: 1,
                connections: Vec::new(),
                rand,
                next_tick_instant: Instant::now(),
                is_server: false,
                conn_id_seed_key,
            };

            Ok(endpoint_manager)
        } else {
            Err(EndpointError::SocketCreation)
        }
    }

    #[inline]
    fn find_connection_from_id(&self, id: u64) -> Option<usize> {
        self.connections.iter().position(|conn| conn.matches_id(id))
    }

    #[inline]
    fn find_connection_from_id_with_probable_index(
        &self,
        id: u64,
        probable_index: usize,
    ) -> Option<usize> {
        if probable_index < self.connections.len()
            && self.connections[probable_index].matches_id(id)
        {
            Some(probable_index)
        } else {
            self.find_connection_from_id(id)
        }
    }

    #[inline]
    fn find_connection_from_dcid(&self, dcid: &[u8]) -> Option<usize> {
        self.connections
            .iter()
            .position(|conn| conn.matches_dcid(dcid))
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
                    self.connections[verified_index].new_timeout_instant();
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

    fn add_server_connection(&mut self, dcid: &[u8], peer_addr: SocketAddr) -> Option<usize> {
        if self.is_server {
            let tag = ring::hmac::sign(&self.conn_id_seed_key, dcid);
            let scid_data = tag.as_ref();

            let writer_opt = match self.connections.len() {
                0 => match std::fs::File::create("security/key.log") {
                    Ok(file) => Some(Box::new(file)),
                    Err(_) => None,
                },
                _ => None,
            };

            match ConnectionManager::new(
                self.next_connection_id,
                peer_addr,
                None,
                self.local_addr,
                scid_data,
                &mut self.config,
                writer_opt,
            ) {
                Ok(conn_mgr) => {
                    self.next_connection_id += 1;
                    self.connections.push(conn_mgr);
                    let verified_index = self.connections.len() - 1;
                    if self.send(verified_index).is_err() {
                        return None;
                    }
                    Some(verified_index)
                }
                Err(_) => None,
            }
        } else {
            None
        }
    }

    pub fn add_client_connection(
        &mut self,
        peer_addr: SocketAddr,
        server_name: &str,
    ) -> Result<(), EndpointError> {
        if !self.is_server {
            let mut scid_data = ConnectionManager::get_empty_cid();
            if self.rand.fill(&mut scid_data).is_err() {
                return Err(EndpointError::Randomness);
            }

            match ConnectionManager::new(
                self.next_connection_id,
                peer_addr,
                Some(server_name),
                self.local_addr,
                &scid_data,
                &mut self.config,
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

    pub fn new_client_with_first_connection(
        bind_addr: SocketAddr,
        alpn: &[u8],
        cert_path: &str,
        peer_addr: SocketAddr,
        server_name: &str,
    ) -> Result<Self, EndpointError> {
        let mut endpoint_mgr = Endpoint::new_client(bind_addr, alpn, cert_path)?;

        endpoint_mgr.add_client_connection(peer_addr, server_name)?;

        Ok(endpoint_mgr)
    }

    #[inline]
    pub fn get_num_connections(&self) -> usize {
        self.connections.len()
    }

    pub fn set_next_tick_instant(&mut self, next_tick_instant: Instant) {
        self.next_tick_instant = next_tick_instant;
    }

    fn get_next_instant(&self) -> (Instant, NextInstantType) {
        let mut next_instant = self.next_tick_instant;
        let mut next_instant_type = NextInstantType::NextTick;

        if let Some(send_instant) = self.udp.next_send_instant() {
            if send_instant < next_instant {
                next_instant = send_instant;
                next_instant_type = NextInstantType::DelayedSend;
            }
        }

        for (conn_ind, conn) in self.connections.iter().enumerate() {
            if let Some(conn_timeout) = conn.get_timeout_instant() {
                if conn_timeout < next_instant {
                    next_instant = conn_timeout;
                    next_instant_type = NextInstantType::ConnectionTimeout(conn_ind);
                }
            }
        }

        (next_instant, next_instant_type)
    }

    pub fn update(&mut self) -> Result<EndpointEvent, EndpointError> {
        let (mut next_instant, mut ni_type) = self.get_next_instant();
        while next_instant <= Instant::now() {
            match ni_type {
                NextInstantType::NextTick => {
                    return Ok(EndpointEvent::NextTick);
                }
                NextInstantType::DelayedSend => {
                    if self.udp.send_check().is_err() {
                        return Err(EndpointError::SocketSend);
                    }
                }
                NextInstantType::ConnectionTimeout(verified_index) => {
                    if self.connections[verified_index].handle_timeout() {
                        self.send(verified_index)?;
                        match self.connections[verified_index].get_status() {
                            Status::Closed(conn_id) => {
                                self.remove_connection(verified_index);
                                return Ok(EndpointEvent::ConnectionClosed(conn_id));
                            }
                            _ => {
                                // Do something here in future maybe
                            }
                        }
                    }
                }
            }
            (next_instant, ni_type) = self.get_next_instant();
        }

        let sleep_duration = next_instant.duration_since(Instant::now());
        if sleep_duration.as_millis() > 0 {
            self.udp.sleep_till_recv_data(sleep_duration);
        }

        Ok(EndpointEvent::PotentiallyReceivedData)
    }

    pub fn recv(&mut self) -> Result<EndpointEvent, EndpointError> {
        match self.udp.recv_data(&mut self.udp_read_data) {
            Ok((recv_size, from_addr)) => {
                // Only bother to look at a datagram that is less than or equal to the target
                if recv_size <= udp::TARGET_MAX_DATAGRAM_SIZE {
                    if let Some((dcid, new_conn_possibility)) =
                        ConnectionManager::recv_header_analyze(
                            &mut self.udp_read_data[..recv_size],
                            self.is_server,
                        )
                    {
                        if let Some(verified_index) = self.find_connection_from_dcid(dcid.as_ref())
                        {
                            match self.connections[verified_index]
                                .recv_data_process(&mut self.udp_read_data[..recv_size], from_addr)
                            {
                                Ok(bytes_processed) => {
                                    // Maybe check bytes_processed in future
                                    match self.send(verified_index) {
                                        Ok(num_sends) => {
                                            // Maybe do something with num_sends in future
                                            match self.connections[verified_index].get_status() {
                                                Status::StreamReadable((conn_id, stream_id)) => {
                                                    let stream_readable = StreamReadable {
                                                        stream_id,
                                                        conn_id,
                                                        probable_index: verified_index,
                                                    };

                                                    Ok(EndpointEvent::StreamReceivedData(
                                                        stream_readable,
                                                    ))
                                                }
                                                Status::Closed(conn_id) => {
                                                    self.remove_connection(verified_index);
                                                    Ok(EndpointEvent::ConnectionClosed(conn_id))
                                                }
                                                Status::Established(conn_id) => Ok(
                                                    EndpointEvent::FinishedConnectingOnce(conn_id),
                                                ),
                                                _ => Ok(EndpointEvent::NoUpdate),
                                            }
                                        }
                                        Err(e) => Err(e),
                                    }
                                }
                                Err(_) => Err(EndpointError::ConnectionRecv),
                            }
                        } else if new_conn_possibility {
                            if let Some(verified_index) =
                                self.add_server_connection(dcid.as_ref(), from_addr)
                            {
                                match self.connections[verified_index].recv_data_process(
                                    &mut self.udp_read_data[..recv_size],
                                    from_addr,
                                ) {
                                    Ok(bytes_processed) => {
                                        // Maybe check bytes_processed in future
                                        match self.send(verified_index) {
                                            Ok(num_sends) => {
                                                // Maybe do something with num_sends in future
                                                Ok(EndpointEvent::NewConnectionStarted)
                                            }
                                            Err(e) => Err(e),
                                        }
                                    }
                                    Err(e) => Err(EndpointError::ConnectionRecv),
                                }
                            } else {
                                Ok(EndpointEvent::NoUpdate)
                            }
                        } else {
                            Ok(EndpointEvent::NoUpdate)
                        }
                    } else {
                        Ok(EndpointEvent::NoUpdate)
                    }
                } else {
                    Err(EndpointError::RecvTooMuchData)
                }
            }
            Err(SocketError::RecvBlocked) => Ok(EndpointEvent::DoneReceiving),
            Err(_) => Err(EndpointError::SocketRecv),
        }
    }

    pub fn close_connection(
        &mut self,
        conn_id: u64,
        error_code: u64,
    ) -> Result<bool, EndpointError> {
        if let Some(verified_index) = self.find_connection_from_id(conn_id) {
            match self.connections[verified_index].close(error_code, b"reason") {
                Ok(_) => match self.send(verified_index) {
                    Ok(_) => Ok(true),
                    Err(e) => Err(e),
                },
                Err(_) => Err(EndpointError::StreamCreation),
            }
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }

    pub fn create_stream(
        &mut self,
        conn_id: u64,
        stream_id: u64,
        priority: u8,
    ) -> Result<bool, EndpointError> {
        // Assumes that is called only after connection is established
        if let Some(verified_index) = self.find_connection_from_id(conn_id) {
            match self.connections[verified_index].create_stream(stream_id, priority) {
                Ok(res) => Ok(res),
                Err(_) => Err(EndpointError::StreamCreation),
            }
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }

    pub fn send_stream_data(
        &mut self,
        conn_id: u64,
        stream_id: u64,
        data: &[u8],
        is_final: bool,
    ) -> Result<(u64, u64), EndpointError> {
        if let Some(verified_index) = self.find_connection_from_id(conn_id) {
            match self.connections[verified_index].stream_send(stream_id, data, is_final) {
                Ok(bytes_sent) => {
                    // Check bytes sent against data.len() in future
                    self.send(verified_index)
                }
                Err(Error::Done) => Err(EndpointError::StreamSendFilled),
                Err(_) => Err(EndpointError::StreamSend),
            }
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }

    pub fn recv_stream_data(
        &mut self,
        stream_readable: &StreamReadable,
        data: &mut [u8],
    ) -> Result<(usize, bool), EndpointError> {
        if let Some(verified_index) = self.find_connection_from_id_with_probable_index(
            stream_readable.conn_id,
            stream_readable.probable_index,
        ) {
            match self.connections[verified_index].stream_recv(stream_readable.stream_id, data) {
                Ok((bytes_recv, finished)) => match self.send(verified_index) {
                    Ok(_) => Ok((bytes_recv, finished)),
                    Err(e) => Err(e),
                },
                Err(Error::Done) => Ok((0, false)),
                Err(_) => Err(EndpointError::StreamRecv),
            }
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }
}
