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

use crate::network::rtc::SocketAddr;
use std::time::{Duration, Instant};

use ring::rand::*;

mod udp;
use udp::{SocketError, UdpSocket};

mod connection;
use connection::{Config, Connection, RecvResult, TimeoutResult};

// pub struct StreamReadable {
//     pub stream_id: u64,
//     pub conn_id: u64,
//     probable_index: usize,
// }

// QUIC Endpoint
pub struct Endpoint {
    udp: UdpSocket,
    max_payload_size: usize,
    recv_data_capacity: usize,
    local_addr: SocketAddr,
    config: Config,
    next_connection_id: u64,
    connections: Vec<Connection>,
    rand: SystemRandom,
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
    ConnectionPing,
    StreamSend,
    StreamSendFilled,
    StreamRecv,
}

pub enum EndpointEvent {
    NextTick,
    ConnectionClosing(u64),
    ConnectionClosed(u64),
    AlreadyHandled,
    ReceivedData,
    DoneReceiving,
    NoUpdate,
    NewConnectionStarted,
    EstablishedOnce((u64, usize)),
    ReliableStreamReceived((u64, usize)),
    //StreamReceivedData(StreamReadable),
}

impl Endpoint {
    // Maybe combine new_server and new_client together... but there is hardly any real benefit (and sacrifices readability)
    pub fn new_server(
        bind_addr: SocketAddr,
        alpn: &[u8],
        cert_path: &str,
        pkey_path: &str,
        reliable_stream_buffer: u64,
    ) -> Result<Self, EndpointError> {
        if let Ok((socket_mgr, local_addr)) = UdpSocket::new(bind_addr) {
            let max_payload_size = udp::TARGET_MAX_DATAGRAM_SIZE;

            let config = match Connection::create_config(
                &[alpn],
                cert_path,
                Some(pkey_path),
                5000,
                max_payload_size,
                reliable_stream_buffer,
                65536,
            ) {
                Ok(cfg) => cfg,
                Err(err) => return Err(EndpointError::ConfigCreation),
            };

            let rand = SystemRandom::new();
            let conn_id_seed_key = match ring::hmac::Key::generate(ring::hmac::HMAC_SHA256, &rand) {
                Ok(key) => key,
                Err(_) => return Err(EndpointError::Randomness),
            };

            let endpoint_manager = Endpoint {
                udp: socket_mgr,
                max_payload_size,
                recv_data_capacity: reliable_stream_buffer as usize,
                local_addr,
                config,
                next_connection_id: 1,
                connections: Vec::new(),
                rand,
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
        reliable_stream_buffer: u64,
    ) -> Result<Self, EndpointError> {
        if let Ok((socket_mgr, local_addr)) = UdpSocket::new(bind_addr) {
            let max_payload_size = udp::TARGET_MAX_DATAGRAM_SIZE;

            let config = match Connection::create_config(
                &[alpn],
                cert_path,
                None,
                5000,
                max_payload_size,
                reliable_stream_buffer,
                65536,
            ) {
                Ok(cfg) => cfg,
                Err(err) => return Err(EndpointError::ConfigCreation),
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
                recv_data_capacity: reliable_stream_buffer as usize,
                local_addr,
                config,
                next_connection_id: 1,
                connections: Vec::new(),
                rand,
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
        // To be changed to binary search later
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
                &mut self.config,
                self.recv_data_capacity,
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
        reliable_stream_buffer: u64,
    ) -> Result<Self, EndpointError> {
        let mut endpoint_mgr =
            Endpoint::new_client(bind_addr, alpn, cert_path, reliable_stream_buffer)?;

        endpoint_mgr.add_client_connection(peer_addr, server_name)?;

        Ok(endpoint_mgr)
    }

    #[inline]
    pub fn get_num_connections(&self) -> usize {
        self.connections.len()
    }

    pub(super) fn get_next_event(
        &mut self,
        next_tick_instant: Instant,
    ) -> Result<EndpointEvent, EndpointError> {
        let mut next_instant = if next_tick_instant > Instant::now() {
            next_tick_instant
        } else {
            return Ok(EndpointEvent::NextTick);
        };
        let mut conn_timeout_opt = None;

        match self.udp.send_check() {
            Ok(send_count) => {
                if send_count > 0 && next_tick_instant <= Instant::now() {
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
                    return Ok(EndpointEvent::ConnectionClosed(conn_id));
                }
                TimeoutResult::Draining(conn_id) => {
                    return Ok(EndpointEvent::ConnectionClosing(conn_id));
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
                                            return Ok(EndpointEvent::ConnectionClosed(conn_id));
                                        }
                                        TimeoutResult::Draining(conn_id) => {
                                            return Ok(EndpointEvent::ConnectionClosing(conn_id));
                                        }
                                        _ => {
                                            return Ok(EndpointEvent::AlreadyHandled);
                                        }
                                    }
                                } else {
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

        // match self.udp.has_data_to_recv() {
        //     Ok(false) => {
        //         // Most likely case... could copy remaining function contents into this...
        //     }
        //     Ok(true) => {
        //         return Ok(EndpointEvent::ReceivedData);
        //     }
        //     Err(_) => {
        //         return Err(EndpointError::SocketRecv);
        //     }
        // }

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
                    Ok(EndpointEvent::ConnectionClosed(conn_id))
                }
                TimeoutResult::Draining(conn_id) => Ok(EndpointEvent::ConnectionClosing(conn_id)),
                _ => Ok(EndpointEvent::AlreadyHandled),
            }
        } else {
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
                                &mut self.config,
                                self.recv_data_capacity,
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
                                Ok(RecvResult::ReliableReadTarget(conn_id)) => {
                                    self.send(verified_index)?;
                                    Ok(EndpointEvent::ReliableStreamReceived((
                                        conn_id,
                                        verified_index,
                                    )))
                                }
                                Ok(RecvResult::StreamReadable((conn_id, stream_id))) => {
                                    self.send(verified_index)?;

                                    // let stream_readable = StreamReadable {
                                    //     stream_id,
                                    //     conn_id,
                                    //     probable_index: verified_index,
                                    // };

                                    // Ok(EndpointEvent::StreamReceivedData(stream_readable))

                                    Ok(EndpointEvent::NoUpdate)
                                }
                                Ok(RecvResult::Closed(conn_id)) => {
                                    self.remove_connection(verified_index);
                                    Ok(EndpointEvent::ConnectionClosed(conn_id))
                                }
                                Ok(RecvResult::Draining(conn_id)) => {
                                    Ok(EndpointEvent::ConnectionClosing(conn_id))
                                }
                                Ok(RecvResult::Established(conn_id)) => {
                                    self.send(verified_index)?;
                                    if !self.is_server {
                                        match self.connections[verified_index]
                                            .create_reliable_stream()
                                        {
                                            Ok(res) => Ok(EndpointEvent::EstablishedOnce((
                                                conn_id,
                                                verified_index,
                                            ))),
                                            Err(_) => Err(EndpointError::StreamCreation),
                                        }
                                    } else {
                                        Ok(EndpointEvent::EstablishedOnce((
                                            conn_id,
                                            verified_index,
                                        )))
                                    }
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

    pub fn send_out_ping_past_duration(
        &mut self,
        duration: Duration,
    ) -> Result<u64, EndpointError> {
        let mut num_pings = 0;
        for verified_index in 0..self.connections.len() {
            match self.connections[verified_index].send_ping_if_neccessary(duration) {
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
        Ok(num_pings)
    }

    // pub fn create_stream(
    //     &mut self,
    //     conn_id: u64,
    //     stream_id: u64,
    //     priority: u8,
    // ) -> Result<bool, EndpointError> {
    //     // Assumes that is called only after connection is established
    //     if let Some(verified_index) = self.find_connection_from_id(conn_id) {
    //         match self.connections[verified_index].create_stream(stream_id, priority) {
    //             Ok(res) => Ok(res),
    //             Err(_) => Err(EndpointError::StreamCreation),
    //         }
    //     } else {
    //         Err(EndpointError::ConnectionNotFound)
    //     }
    // }

    pub fn send_reliable_stream_data(
        &mut self,
        conn_id: u64,
        probable_index: usize,
        send_data: Vec<u8>,
    ) -> Result<(u64, u64), EndpointError> {
        if let Some(verified_index) =
            self.find_connection_from_id_with_probable_index(conn_id, probable_index)
        {
            match self.connections[verified_index].stream_reliable_send(send_data) {
                Ok(s) => self.send(verified_index),
                Err(_) => Err(EndpointError::StreamSend),
            }
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }

    pub(super) fn recv_reliable_stream_data(
        &mut self,
        conn_id: u64,
        probable_index: usize,
        read_data_copy: &mut [u8],
    ) -> Result<(Option<usize>, Option<Vec<u8>>), EndpointError> {
        if let Some(verified_index) =
            self.find_connection_from_id_with_probable_index(conn_id, probable_index)
        {
            match self.connections[verified_index].stream_reliable_read(read_data_copy) {
                Ok((x, y)) => Ok((x, y)),
                Err(_) => Err(EndpointError::StreamSend),
            }
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }

    pub(super) fn set_reliable_stream_recv_target(
        &mut self,
        conn_id: u64,
        probable_index: usize,
        next_target: usize,
    ) -> Result<(), EndpointError> {
        if let Some(verified_index) =
            self.find_connection_from_id_with_probable_index(conn_id, probable_index)
        {
            self.connections[verified_index].stream_reliable_next_read_target(next_target);
            Ok(())
        } else {
            Err(EndpointError::ConnectionNotFound)
        }
    }

    // pub fn send_stream_data(
    //     &mut self,
    //     conn_id: u64,
    //     stream_id: u64,
    //     data: &[u8],
    //     is_final: bool,
    // ) -> Result<(u64, u64), EndpointError> {
    //     if let Some(verified_index) = self.find_connection_from_id(conn_id) {
    //         match self.connections[verified_index].stream_send(stream_id, data, is_final) {
    //             Ok(bytes_sent) => {
    //                 // Check bytes sent against data.len() in future
    //                 self.send(verified_index)
    //             }
    //             Err(Error::Done) => Err(EndpointError::StreamSendFilled),
    //             Err(_) => Err(EndpointError::StreamSend),
    //         }
    //     } else {
    //         Err(EndpointError::ConnectionNotFound)
    //     }
    // }

    // pub fn recv_stream_data(
    //     &mut self,
    //     stream_readable: &StreamReadable,
    //     data: &mut [u8],
    // ) -> Result<(usize, bool), EndpointError> {
    //     if let Some(verified_index) = self.find_connection_from_id_with_probable_index(
    //         stream_readable.conn_id,
    //         stream_readable.probable_index,
    //     ) {
    //         match self.connections[verified_index].stream_recv(stream_readable.stream_id, data) {
    //             Ok((bytes_recv, finished)) => match self.send(verified_index) {
    //                 Ok(_) => Ok((bytes_recv, finished)),
    //                 Err(e) => Err(e),
    //             },
    //             Err(Error::Done) => Ok((0, false)),
    //             Err(_) => Err(EndpointError::StreamRecv),
    //         }
    //     } else {
    //         Err(EndpointError::ConnectionNotFound)
    //     }
    // }
}
