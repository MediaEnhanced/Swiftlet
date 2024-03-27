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

// Socket Address format used within the library
pub use std::net::SocketAddr;
use std::time::{Duration, Instant};

use ring::rand::*;

mod udp;
use udp::{Socket, SocketError};

mod connection;
use connection::{CloseInfo, CloseOrigin, Connection, RecvResult, SendResult, StreamResult};

/// The Endpoint Configuration Structure
///
/// Used when creating a new Endpoint
pub struct Config {
    /// The quic connection idle timeout in milliseconds.
    pub idle_timeout_in_ms: u64,

    /// The quic connection bidirectional stream receive buffer length in bytes.
    ///
    /// These streams are intended for communicating reliable information.
    /// Most applications should probably set this to a multiple of 65536
    pub reliable_stream_buffer: u64,

    /// The quic connection unidirectional stream receive buffer length in bytes.
    ///
    /// These streams are intended for real-time unreliable information.
    /// Most applications should probably set this to a multiple of 65536
    pub unreliable_stream_buffer: u64,

    /// The keep alive timeout duration.
    ///
    /// If there is a value and the duration has passed since the quic connection had recieved anything
    /// the quic connection will send out a PING to try and keep the connection alive.
    /// Any potential keep alives currently occur right before the tick callback function is called.
    pub keep_alive_timeout: Option<Duration>,

    /// The initial main stream recieve buffer size.
    ///
    /// This could be set to the max size of the expected data to process to avoid the minimal resize costs.
    pub initial_main_recv_size: usize,

    /// The number of bytes to receive on the main stream before calling main_stream_recv for the first time.
    ///
    /// If this value is set to 0 it will be changed to 1 during endpoint creation
    pub main_recv_first_bytes: usize,

    /// The initial real-time stream recieve buffer size.
    ///
    /// This could be set to the max size of the expected data to process to avoid the minimal resize costs.
    pub initial_rt_recv_size: usize,

    /// The number of bytes to receive on the real-time stream before calling main_stream_recv for the first time.
    ///
    /// If this value is set to 0 the  
    pub rt_recv_first_bytes: usize,

    /// The initial background stream recieve buffer size.
    ///
    /// This could be set to the max size of the expected data to process to avoid the minimal resize costs.
    pub initial_background_recv_size: usize,

    /// The number of bytes to receive on the background stream before calling main_stream_recv for the first time.
    ///
    /// If this value is set to 0 it will be changed to 1 during endpoint creation
    pub background_recv_first_bytes: usize,
}

/// The Quic Endpoint structure
pub struct Endpoint {
    udp: Socket,
    max_payload_size: usize,
    local_addr: SocketAddr,
    connection_config: connection::Config,
    next_connection_id: u64,
    connections: Vec<Connection>,
    last_valid_index: usize,
    last_recv_index: Option<usize>,
    stream_process_index: Option<(u64, usize)>,
    rand: SystemRandom,
    config: Config,
    is_server: bool,
    conn_id_seed_key: ring::hmac::Key, // Value matters ONLY if is_server is true
}

/// A Connection ID used to communicate with the endpoint about a specific connection.
pub type ConnectionId = u64;

/// Errors that the QUIC Endpoint can return
#[derive(Debug)]
pub enum Error {
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
    /// Error from an unexpected close
    UnexpectedClose,
    /// Error sending data on the UDP socket
    SocketSend,
    /// Error receiving data on the UDP socket
    SocketRecv(std::io::ErrorKind),

    // Error receiving too much data
    //RecvTooMuchData,
    /// Error having a connection process the received data
    ConnectionRecv(connection::Error),
    /// Cannot find connection from Connection ID
    ConnectionNotFound,
    /// Error finishing the connection establishment process and stream creation
    StreamCreation,
    /// Error sending out a PING
    ConnectionPing,
    /// Error sending data on the stream
    StreamSend,
    /// Error receiving data from the stream
    StreamRecv(connection::Error),
}

/// Based on combination of QUIC Transport Error Codes and Endpoint Error Codes
#[derive(Debug)]
#[repr(u64)]
pub enum EndpointCloseReason {
    /// No Error
    NoError = 0, // Enforce that it is zero
    /// Implementation Error
    InternalError,
    /// Server refuses a connection
    ConnectionRefused,
    /// Flow control error
    FlowControlError,
    /// Too many streams opened
    StreamLimitError,
    /// Frame received in invalid stream state
    StreamStateError,
    /// Change to final size
    FinalStateError,
    /// Frame encoding error
    FrameEncodingError,
    /// Error in transport parameters
    TransportParameterError,
    /// Too many connection IDs received
    ConnectionIdLimitError,
    /// Generic protocol violation
    ProtocolViolation,
    /// Invalid Token received
    InvalidToken,
    /// Application error
    ApplicationError,
    /// CRYPTO data buffer overflowed
    CryptoBufferExceeded,
    /// Invalid packet protection update
    KeyUpdateError,
    /// Excessive use of packet protection keys
    AeadLimitReached,
    /// No viable network path exists
    NoViablePath,

    /// Main stream finished
    MainStreamFinished,
    /// Background stream finished
    BackgroundStreamFinished,

    /// TLS Alert Start
    CryptoErrorStart = 0x0100,
    /// TLS Alert End
    CryptoErrorEnd = 0x01FF,
}

impl EndpointCloseReason {
    #[inline] // Verbose but compiles down to minimal instructions
    fn from_u64(value: u64) -> Self {
        match value {
            x if x == EndpointCloseReason::InternalError as u64 => {
                EndpointCloseReason::InternalError
            }
            x if x == EndpointCloseReason::ConnectionRefused as u64 => {
                EndpointCloseReason::ConnectionRefused
            }
            x if x == EndpointCloseReason::FlowControlError as u64 => {
                EndpointCloseReason::FlowControlError
            }
            x if x == EndpointCloseReason::StreamLimitError as u64 => {
                EndpointCloseReason::StreamLimitError
            }
            x if x == EndpointCloseReason::StreamStateError as u64 => {
                EndpointCloseReason::StreamStateError
            }
            x if x == EndpointCloseReason::FinalStateError as u64 => {
                EndpointCloseReason::FinalStateError
            }
            x if x == EndpointCloseReason::FrameEncodingError as u64 => {
                EndpointCloseReason::FrameEncodingError
            }
            x if x == EndpointCloseReason::TransportParameterError as u64 => {
                EndpointCloseReason::TransportParameterError
            }
            x if x == EndpointCloseReason::ConnectionIdLimitError as u64 => {
                EndpointCloseReason::ConnectionIdLimitError
            }
            x if x == EndpointCloseReason::ProtocolViolation as u64 => {
                EndpointCloseReason::ProtocolViolation
            }
            x if x == EndpointCloseReason::InvalidToken as u64 => EndpointCloseReason::InvalidToken,
            x if x == EndpointCloseReason::ApplicationError as u64 => {
                EndpointCloseReason::ApplicationError
            }
            x if x == EndpointCloseReason::CryptoBufferExceeded as u64 => {
                EndpointCloseReason::CryptoBufferExceeded
            }
            x if x == EndpointCloseReason::KeyUpdateError as u64 => {
                EndpointCloseReason::KeyUpdateError
            }
            x if x == EndpointCloseReason::AeadLimitReached as u64 => {
                EndpointCloseReason::AeadLimitReached
            }
            x if x == EndpointCloseReason::NoViablePath as u64 => EndpointCloseReason::NoViablePath,

            x if x == EndpointCloseReason::MainStreamFinished as u64 => {
                EndpointCloseReason::MainStreamFinished
            }
            x if x == EndpointCloseReason::BackgroundStreamFinished as u64 => {
                EndpointCloseReason::BackgroundStreamFinished
            }

            // Need to adjust this to cover more errors
            x if x == EndpointCloseReason::CryptoErrorStart as u64 => {
                EndpointCloseReason::CryptoErrorStart
            }
            x if x == EndpointCloseReason::CryptoErrorEnd as u64 => {
                EndpointCloseReason::CryptoErrorEnd
            }

            _ => EndpointCloseReason::NoError,
        }
    }
}

/// Reason the connection has ended / is ending
#[derive(Debug)]
pub enum ConnectionEndReason {
    /// Not sure of the reason
    Uncertain,
    /// Idle Timeout
    IdleTimeout,
    /// Local Endpoint Error
    LocalEndpoint(EndpointCloseReason),
    /// Peer Endpoint Error
    PeerEndpoint(EndpointCloseReason),
    /// Local Application Error
    LocalApplication(u64),
    /// Peer Application Error
    PeerApplication(u64),
}

impl ConnectionEndReason {
    fn from_close_info(close_info: &CloseInfo) -> Self {
        match close_info.close_origin {
            CloseOrigin::Timeout => ConnectionEndReason::IdleTimeout,
            CloseOrigin::Local => {
                if close_info.is_application_error {
                    ConnectionEndReason::LocalApplication(close_info.error_code)
                } else {
                    ConnectionEndReason::LocalEndpoint(EndpointCloseReason::from_u64(
                        close_info.error_code,
                    ))
                }
            }
            CloseOrigin::Peer => {
                if close_info.is_application_error {
                    ConnectionEndReason::PeerApplication(close_info.error_code)
                } else {
                    ConnectionEndReason::PeerEndpoint(EndpointCloseReason::from_u64(
                        close_info.error_code,
                    ))
                }
            }
            _ => ConnectionEndReason::Uncertain,
        }
    }
}

pub(super) enum NextEvent {
    AlreadyHandled,
    Tick,
    ConnectionEnded((ConnectionId, ConnectionEndReason)),
    ConnectionEnding((ConnectionId, ConnectionEndReason)),
    ReceivedData,
}

pub(super) enum RecvEvent {
    NoUpdate,
    DoneReceiving,
    ConnectionEnded((ConnectionId, ConnectionEndReason)),
    ConnectionEnding((ConnectionId, ConnectionEndReason)),
    EstablishedOnce(ConnectionId),
    MainStreamReceived((ConnectionId, usize, Vec<u8>, usize)),
    RealtimeReceived(ConnectionId, usize, Vec<u8>, usize, u64),
    BackgroundStreamReceived((ConnectionId, usize, Vec<u8>, usize)),
}

pub(super) enum ReadInfo {
    DoneReceiving,
    ReadData((Vec<u8>, usize)),
    ConnectionEnded(ConnectionEndReason),
    ConnectionEnding(ConnectionEndReason),
}

impl Endpoint {
    // Maybe combine new_server and new_client together... but there is hardly any real benefit (and sacrifices readability)

    /// Create a QUIC Server Endpoint
    pub fn new_server(
        ipv6_mode: bool,
        bind_port: u16,
        alpn: &[u8],
        cert_path: &str,
        pkey_path: &str,
        mut config: Config,
    ) -> Result<Self, Error> {
        if let Ok((socket_mgr, local_addr)) = Socket::new(ipv6_mode, bind_port) {
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
                Err(_) => return Err(Error::ConfigCreation),
            };

            let rand = SystemRandom::new();
            let conn_id_seed_key = match ring::hmac::Key::generate(ring::hmac::HMAC_SHA256, &rand) {
                Ok(key) => key,
                Err(_) => return Err(Error::Randomness),
            };

            if config.initial_main_recv_size == 0 {
                config.initial_main_recv_size = 1;
            }

            if config.initial_background_recv_size == 0 {
                config.initial_background_recv_size = 1;
            }

            let endpoint_manager = Endpoint {
                udp: socket_mgr,
                max_payload_size,
                local_addr,
                connection_config,
                next_connection_id: 1,
                connections: Vec::new(),
                last_valid_index: 0,
                last_recv_index: None,
                stream_process_index: None,
                rand,
                config,
                is_server: true,
                conn_id_seed_key,
            };

            Ok(endpoint_manager)
        } else {
            Err(Error::SocketCreation)
        }
    }

    /// Create a QUIC Client Endpoint
    pub fn new_client(
        ipv6_mode: bool,
        alpn: &[u8],
        cert_path: &str,
        mut config: Config,
    ) -> Result<Self, Error> {
        if let Ok((socket_mgr, local_addr)) = Socket::new(ipv6_mode, 0) {
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
                Err(_) => return Err(Error::ConfigCreation),
            };

            let rand = SystemRandom::new();
            // Following value doesn't matter but its useful for making sure the SystemRandom is working... I guess
            let conn_id_seed_key = match ring::hmac::Key::generate(ring::hmac::HMAC_SHA256, &rand) {
                Ok(key) => key,
                Err(_) => return Err(Error::Randomness),
            };

            if config.initial_main_recv_size == 0 {
                config.initial_main_recv_size = 1;
            }

            if config.initial_background_recv_size == 0 {
                config.initial_background_recv_size = 1;
            }

            let endpoint_manager = Endpoint {
                udp: socket_mgr,
                max_payload_size,
                local_addr,
                connection_config,
                next_connection_id: 1,
                connections: Vec::new(),
                last_valid_index: 0,
                last_recv_index: None,
                stream_process_index: None,
                rand,
                config,
                is_server: false,
                conn_id_seed_key,
            };

            Ok(endpoint_manager)
        } else {
            Err(Error::SocketCreation)
        }
    }

    #[inline]
    fn find_connection_from_cid(&self, cid: ConnectionId) -> Option<usize> {
        // To be changed to a binary search later depending on how many total connections there are
        for i in self.last_valid_index..self.connections.len() {
            if self.connections[i].matches_id(cid) {
                return Some(i);
            }
        }
        #[allow(clippy::manual_find)]
        for i in 0..self.last_valid_index {
            if self.connections[i].matches_id(cid) {
                return Some(i);
            }
        }
        None
    }

    fn send(&mut self, verified_index: usize) -> Result<Option<CloseInfo>, Error> {
        //let mut immediate_sends = 0;
        //let mut delayed_sends = 0;
        loop {
            let packet_data = self.udp.get_next_send_data();
            match self.connections[verified_index].get_next_send_packet(packet_data) {
                Ok(SendResult::DataToSend((packet_len, to_addr, instant))) => {
                    match self.udp.done_with_send_data(to_addr, packet_len, instant) {
                        Ok(true) => {
                            //immediate_sends += 1;
                        }
                        Ok(false) => {
                            //delayed_sends += 1;
                        }
                        Err(_) => {
                            return Err(Error::SocketSend);
                        }
                    }
                }
                Ok(SendResult::Done) => {
                    //return Ok((immediate_sends, delayed_sends));
                    return Ok(None);
                }
                Ok(SendResult::CloseInfo(close_info)) => {
                    return Ok(Some(close_info));
                }
                Err(_) => {
                    return Err(Error::ConnectionSend);
                }
            }
        }
    }

    // This is different than closing the connection
    fn remove_connection(&mut self, verified_index: usize) {
        // Pretty confident that this is truly all there really is to it:
        self.connections.remove(verified_index);
    }

    /// Add a connection for a Client Endpoint
    ///
    /// Must be used on a Client and not a Server otherwise an error will be thrown
    pub fn add_client_connection(
        &mut self,
        peer_addr: SocketAddr,
        server_name: &str,
    ) -> Result<(), Error> {
        if !self.is_server {
            let mut scid_data = Connection::get_empty_cid();
            if self.rand.fill(&mut scid_data).is_err() {
                return Err(Error::Randomness);
            }

            let writer_opt = match self.connections.len() {
                0 => match std::fs::File::create("clientKey.log") {
                    Ok(file) => Some(Box::new(file)),
                    Err(_) => None,
                },
                _ => None,
            };

            match Connection::new(
                self.next_connection_id,
                peer_addr,
                Some(server_name),
                self.local_addr,
                &scid_data,
                &mut self.connection_config,
                writer_opt,
            ) {
                Ok(conn_mgr) => {
                    self.next_connection_id += 1;
                    self.connections.push(conn_mgr);
                    let verified_index = self.connections.len() - 1;
                    if self.send(verified_index)?.is_some() {
                        Err(Error::UnexpectedClose)
                    } else {
                        Ok(())
                    }
                }
                Err(_) => Err(Error::ConnectionCreation),
            }
        } else {
            Err(Error::IsServer)
        }
    }

    /// Create a QUIC Client Endpoint with an initial connection
    pub fn new_client_with_first_connection(
        ipv6_mode: bool,
        alpn: &[u8],
        cert_path: &str,
        peer_addr: SocketAddr,
        server_name: &str,
        config: Config,
    ) -> Result<Self, Error> {
        let mut endpoint_mgr = Endpoint::new_client(ipv6_mode, alpn, cert_path, config)?;

        endpoint_mgr.add_client_connection(peer_addr, server_name)?;

        Ok(endpoint_mgr)
    }

    /// Get the number of connections that the Endpoint is managing
    #[inline]
    pub fn get_num_connections(&self) -> usize {
        self.connections.len()
    }

    /// Update the keep alive duration time
    ///
    /// Will disable the keep alive functionality if set to None
    #[inline]
    pub fn update_keep_alive_duration(&mut self, duration_opt: Option<Duration>) {
        self.config.keep_alive_timeout = duration_opt;
    }

    fn keep_alive(&mut self) -> Result<u64, Error> {
        let mut num_pings = 0;
        if let Some(duration) = self.config.keep_alive_timeout {
            let before_instant = Instant::now() - duration;
            for verified_index in 0..self.connections.len() {
                match self.connections[verified_index].send_ping_if_before_instant(before_instant) {
                    Ok(false) => {}
                    Ok(true) => {
                        if self.send(verified_index)?.is_some() {
                            return Err(Error::UnexpectedClose);
                        }
                        num_pings += 1;
                    }
                    Err(_) => {
                        return Err(Error::ConnectionPing);
                    }
                }
            }
        }
        Ok(num_pings)
    }

    pub(super) fn get_next_event(
        &mut self,
        next_tick_instant: Instant,
    ) -> Result<NextEvent, Error> {
        let mut next_instant = if next_tick_instant > Instant::now() {
            next_tick_instant
        } else {
            self.keep_alive()?;
            return Ok(NextEvent::Tick);
        };
        let mut conn_timeout_opt: Option<usize> = None;

        match self.udp.send_check() {
            Ok(send_count) => {
                if send_count > 0 && next_tick_instant <= Instant::now() {
                    self.keep_alive()?;
                    return Ok(NextEvent::Tick);
                }
            }
            Err(_) => {
                return Err(Error::SocketSend);
            }
        }

        for verified_index in 0..self.connections.len() {
            match self.connections[verified_index].handle_possible_timeout() {
                None => {
                    if let Some(close_info) = self.send(verified_index)? {
                        let connection_id = close_info.id;
                        self.last_valid_index = verified_index;
                        let end_reason = ConnectionEndReason::from_close_info(&close_info);
                        if close_info.is_closed {
                            self.remove_connection(verified_index);
                            return Ok(NextEvent::ConnectionEnded((connection_id, end_reason)));
                        }
                        return Ok(NextEvent::ConnectionEnding((connection_id, end_reason)));
                    }
                    match self.udp.send_check() {
                        Ok(_) => {
                            if next_instant <= Instant::now() {
                                if let Some(vi) = conn_timeout_opt {
                                    if self.connections[vi].handle_possible_timeout().is_none() {
                                        if let Some(close_info) = self.send(vi)? {
                                            let connection_id = close_info.id;
                                            self.last_valid_index = vi;
                                            let end_reason =
                                                ConnectionEndReason::from_close_info(&close_info);
                                            if close_info.is_closed {
                                                self.remove_connection(vi);
                                                return Ok(NextEvent::ConnectionEnded((
                                                    connection_id,
                                                    end_reason,
                                                )));
                                            }
                                            return Ok(NextEvent::ConnectionEnding((
                                                connection_id,
                                                end_reason,
                                            )));
                                        }
                                    }
                                    return Ok(NextEvent::AlreadyHandled);
                                } else {
                                    self.keep_alive()?;
                                    return Ok(NextEvent::Tick);
                                }
                            }
                        }
                        Err(_) => {
                            return Err(Error::SocketSend);
                        }
                    }
                }
                Some(Some(timeout_instant)) => {
                    if timeout_instant < next_instant {
                        next_instant = timeout_instant;
                        conn_timeout_opt = Some(verified_index);
                    }
                }
                Some(None) => {
                    // Do nothing
                }
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
            Ok(NextEvent::ReceivedData)
        } else if send_check_timeout {
            match self.udp.send_check() {
                Ok(_) => Ok(NextEvent::AlreadyHandled),
                Err(_) => Err(Error::SocketSend),
            }
        } else if let Some(vi) = conn_timeout_opt {
            if self.connections[vi].handle_possible_timeout().is_none() {
                if let Some(close_info) = self.send(vi)? {
                    let connection_id = close_info.id;
                    self.last_valid_index = vi;
                    let end_reason = ConnectionEndReason::from_close_info(&close_info);
                    if close_info.is_closed {
                        self.remove_connection(vi);
                        Ok(NextEvent::ConnectionEnded((connection_id, end_reason)))
                    } else {
                        Ok(NextEvent::ConnectionEnding((connection_id, end_reason)))
                    }
                } else {
                    Ok(NextEvent::AlreadyHandled)
                }
            } else {
                Ok(NextEvent::AlreadyHandled)
            }
        } else {
            self.keep_alive()?;
            Ok(NextEvent::Tick)
        }
    }

    fn stream_process(
        &mut self,
        connection_id: u64,
        verified_index: usize,
    ) -> Result<RecvEvent, Error> {
        self.last_valid_index = verified_index;
        match self.connections[verified_index].stream_process() {
            Ok(StreamResult::NoMore) => {
                self.stream_process_index = None;
                if self.send(verified_index)?.is_none() {
                    Ok(RecvEvent::NoUpdate)
                } else {
                    Err(Error::UnexpectedClose)
                }
            }
            Ok(StreamResult::MainStreamReadable((data_vec, len))) => {
                // if self.send(verified_index)?.is_none() {
                Ok(RecvEvent::MainStreamReceived((
                    connection_id,
                    verified_index,
                    data_vec,
                    len,
                )))
                // } else {
                //     Err(Error::UnexpectedClose)
                // }
            }
            Ok(StreamResult::RealtimeStreamReadable((data_vec, len, rt_id))) => {
                // if self.send(verified_index)?.is_none() {
                Ok(RecvEvent::RealtimeReceived(
                    connection_id,
                    verified_index,
                    data_vec,
                    len,
                    rt_id,
                ))
                // } else {
                //     Err(Error::UnexpectedClose)
                // }
            }
            Ok(StreamResult::BkgdStreamReadable((data_vec, len))) => {
                // if self.send(verified_index)?.is_none() {
                Ok(RecvEvent::BackgroundStreamReceived((
                    connection_id,
                    verified_index,
                    data_vec,
                    len,
                )))
                // } else {
                //     Err(Error::UnexpectedClose)
                // }
            }
            Ok(StreamResult::Nothing) => Ok(RecvEvent::NoUpdate),
            Ok(StreamResult::MainStreamFinished) => {
                if let Some(close_info) =
                    self.connection_close(verified_index, EndpointCloseReason::MainStreamFinished)?
                {
                    let end_reason = ConnectionEndReason::from_close_info(&close_info);
                    if close_info.is_closed {
                        self.remove_connection(verified_index);
                        Ok(RecvEvent::ConnectionEnded((connection_id, end_reason)))
                    } else {
                        Ok(RecvEvent::ConnectionEnding((connection_id, end_reason)))
                    }
                } else {
                    Ok(RecvEvent::NoUpdate)
                }
            }
            Ok(StreamResult::BkgdStreamFinished) => {
                if let Some(close_info) = self.connection_close(
                    verified_index,
                    EndpointCloseReason::BackgroundStreamFinished,
                )? {
                    let end_reason = ConnectionEndReason::from_close_info(&close_info);
                    if close_info.is_closed {
                        self.remove_connection(verified_index);
                        Ok(RecvEvent::ConnectionEnded((connection_id, end_reason)))
                    } else {
                        Ok(RecvEvent::ConnectionEnding((connection_id, end_reason)))
                    }
                } else {
                    Ok(RecvEvent::NoUpdate)
                }
            }

            Err(e) => Err(Error::StreamRecv(e)),
        }
    }

    pub(super) fn recv(&mut self) -> Result<RecvEvent, Error> {
        // Gotta Process Coallesced Stream Packets Here!
        if let Some((connection_id, verified_index)) = self.stream_process_index {
            let evt = self.stream_process(connection_id, verified_index)?;
            if self.stream_process_index.is_some() {
                return Ok(evt);
            }
        }

        let mut send_ind_opt = None;
        let res = match self.udp.get_next_recv_data() {
            Ok((recv_data, from_addr)) => {
                // Only bother to look at a datagram that is less than or equal to the target
                if recv_data.len() <= self.max_payload_size {
                    if let Some((dcid, new_conn_possibility)) =
                        Connection::recv_header_analyze(recv_data, self.is_server)
                    {
                        let mut verified_index_opt = match self.last_recv_index {
                            Some(ind) => {
                                if self.connections[ind].matches_dcid(&dcid) {
                                    Some(ind)
                                } else {
                                    send_ind_opt = Some(ind);
                                    self.last_recv_index = self
                                        .connections
                                        .iter()
                                        .position(|conn| conn.matches_dcid(&dcid));
                                    self.last_recv_index
                                }
                            }
                            None => {
                                self.last_recv_index = self
                                    .connections
                                    .iter()
                                    .position(|conn| conn.matches_dcid(&dcid));
                                self.last_recv_index
                            }
                        };

                        if verified_index_opt.is_none() && new_conn_possibility && self.is_server {
                            let tag = ring::hmac::sign(&self.conn_id_seed_key, &dcid);
                            let scid_data = tag.as_ref();

                            let writer_opt = match self.connections.len() {
                                0 => match std::fs::File::create("key.log") {
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
                                Err(_) => {
                                    self.udp.done_with_recv_data();
                                    return Ok(RecvEvent::NoUpdate);
                                }
                            }
                        }

                        if let Some(verified_index) = verified_index_opt {
                            match self.connections[verified_index].recv_data(recv_data, from_addr) {
                                Ok(RecvResult::StreamProcess(conn_id)) => {
                                    self.stream_process_index = Some((conn_id, verified_index));
                                    self.stream_process(conn_id, verified_index)
                                }
                                Ok(RecvResult::CloseInfo(close_info)) => {
                                    let connection_id = close_info.id;
                                    self.last_valid_index = verified_index;
                                    let end_reason =
                                        ConnectionEndReason::from_close_info(&close_info);
                                    if close_info.is_closed {
                                        self.remove_connection(verified_index);
                                        Ok(RecvEvent::ConnectionEnded((connection_id, end_reason)))
                                    } else {
                                        Ok(RecvEvent::ConnectionEnding((connection_id, end_reason)))
                                    }
                                }
                                Ok(RecvResult::Established(conn_id)) => {
                                    if self.send(verified_index)?.is_none() {
                                        // let mut main_recv_data_old =
                                        //     Vec::with_capacity(self.config.initial_main_recv_size);
                                        // main_recv_data_old
                                        //     .resize(self.config.initial_main_recv_size, 0);
                                        let main_recv_data =
                                            vec![0; self.config.initial_main_recv_size]; // Faster according to clippy and github issue discussion but unsure why...?

                                        let rt_recv_data =
                                            vec![0; self.config.initial_rt_recv_size];

                                        let background_recv_data =
                                            vec![0; self.config.initial_background_recv_size];

                                        if self.connections[verified_index]
                                            .finish_establishment(
                                                main_recv_data,
                                                self.config.main_recv_first_bytes,
                                                rt_recv_data,
                                                self.config.rt_recv_first_bytes,
                                                background_recv_data,
                                                self.config.background_recv_first_bytes,
                                            )
                                            .is_ok()
                                        {
                                            self.last_valid_index = verified_index;
                                            Ok(RecvEvent::EstablishedOnce(conn_id))
                                        } else {
                                            Err(Error::StreamCreation)
                                        }
                                    } else {
                                        Err(Error::UnexpectedClose)
                                    }
                                }
                                Ok(RecvResult::Nothing) => {
                                    if self.send(verified_index)?.is_none() {
                                        Ok(RecvEvent::NoUpdate)
                                    } else {
                                        Err(Error::UnexpectedClose)
                                    }
                                }
                                Ok(RecvResult::CloseInitiated) => {
                                    // This is an unexpected spot but allowable
                                    Ok(RecvEvent::NoUpdate)
                                }
                                Err(e) => Err(Error::ConnectionRecv(e)),
                            }
                        } else {
                            Ok(RecvEvent::NoUpdate)
                        }
                    } else {
                        Ok(RecvEvent::NoUpdate)
                    }
                } else {
                    //Err(EndpointError::RecvTooMuchData)
                    Ok(RecvEvent::NoUpdate)
                }
            }
            Err(SocketError::RecvBlocked) => {
                if let Some(last_recv_ind) = self.last_recv_index {
                    if self.send(last_recv_ind)?.is_none() {
                        self.last_recv_index = None;
                        Ok(RecvEvent::DoneReceiving)
                    } else {
                        Err(Error::UnexpectedClose)
                    }
                } else {
                    Ok(RecvEvent::DoneReceiving)
                }
            }
            //Err(SocketError::RecvOtherIssue(e)) => Err(Error::SocketRecv(e)),
            Err(_) => Err(Error::SocketRecv(std::io::ErrorKind::WouldBlock)),
        };
        self.udp.done_with_recv_data();

        if let Some(send_ind) = send_ind_opt {
            if self.send(send_ind)?.is_none() {
                res
            } else {
                Err(Error::UnexpectedClose)
            }
        } else {
            res
        }
    }

    // Close a connection with a given error code value
    fn connection_close(
        &mut self,
        verified_index: usize,
        reason: EndpointCloseReason,
    ) -> Result<Option<CloseInfo>, Error> {
        match self.connections[verified_index].close(reason as u64, b"reason") {
            Ok(_) => {
                let close_info_opt = self.send(verified_index)?;
                Ok(close_info_opt)
            }
            Err(_) => Err(Error::ConnectionClose),
        }
    }

    /// Close a connection with a given error code value
    ///
    /// Returns true when connection close process has started
    pub fn close_connection(&mut self, cid: &ConnectionId, error_code: u64) -> Result<bool, Error> {
        if let Some(verified_index) = self.find_connection_from_cid(*cid) {
            match self.connections[verified_index].app_close(error_code, b"app-reason") {
                Ok(_) => {
                    if self.send(verified_index)?.is_some() {
                        Err(Error::UnexpectedClose)
                    } else {
                        Ok(true)
                    }
                }
                Err(_) => Err(Error::ConnectionClose),
            }
        } else {
            Err(Error::ConnectionNotFound)
        }
    }

    /// Get the socket address for a connection
    ///
    /// This address could change if a backend connection migration happens (not currently implemented / expected)
    pub fn get_connection_socket_addr(&mut self, cid: &ConnectionId) -> Result<SocketAddr, Error> {
        if let Some(verified_index) = self.find_connection_from_cid(*cid) {
            Ok(self.connections[verified_index].get_socket_addr())
        } else {
            Err(Error::ConnectionNotFound)
        }
    }

    /// Send data over the main stream. This data is queued up if it cannot be sent immediately.
    ///
    /// The main stream is a reliable (ordered) stream that focuses on communicating
    /// high-priority, small(ish) messages between the server and client.
    ///
    /// A reminder that the Endpoint connection will be taking ownership of the data so it can be sent out when possible
    pub fn main_stream_send(
        &mut self,
        cid: &ConnectionId,
        send_data: Vec<u8>,
    ) -> Result<(), Error> {
        if let Some(verified_index) = self.find_connection_from_cid(*cid) {
            match self.connections[verified_index].main_stream_send(send_data) {
                Ok(_) => {
                    if self.send(verified_index)?.is_some() {
                        Err(Error::UnexpectedClose)
                    } else {
                        Ok(())
                    }
                }
                Err(_) => Err(Error::StreamSend),
            }
        } else {
            Err(Error::ConnectionNotFound)
        }
    }

    pub(super) fn main_stream_read(
        &mut self,
        verified_index: usize,
        data_vec: Vec<u8>,
        target_len_opt: Option<usize>,
    ) -> Result<ReadInfo, Error> {
        if let Some(mut target_len) = target_len_opt {
            if target_len == 0 {
                target_len = self.config.initial_main_recv_size;
            }
            match self.connections[verified_index].main_stream_read(data_vec, target_len) {
                Ok(vec_data_opt) => {
                    // if self.send(verified_index)?.is_none() {
                    if let Some(vec_data) = vec_data_opt {
                        Ok(ReadInfo::ReadData((vec_data, target_len)))
                    } else {
                        Ok(ReadInfo::DoneReceiving)
                    }
                    // } else {
                    //     Err(Error::UnexpectedClose)
                    // }
                }
                Err(connection::Error::Done) => {
                    if let Some(close_info) = self
                        .connection_close(verified_index, EndpointCloseReason::MainStreamFinished)?
                    {
                        let end_reason = ConnectionEndReason::from_close_info(&close_info);
                        if close_info.is_closed {
                            self.remove_connection(verified_index);
                            Ok(ReadInfo::ConnectionEnded(end_reason))
                        } else {
                            Ok(ReadInfo::ConnectionEnding(end_reason))
                        }
                    } else {
                        Ok(ReadInfo::DoneReceiving)
                    }
                }
                Err(_) => Err(Error::StreamSend),
            }
        } else if let Some(close_info) =
            self.connection_close(verified_index, EndpointCloseReason::MainStreamFinished)?
        {
            let end_reason = ConnectionEndReason::from_close_info(&close_info);
            if close_info.is_closed {
                self.remove_connection(verified_index);
                Ok(ReadInfo::ConnectionEnded(end_reason))
            } else {
                Ok(ReadInfo::ConnectionEnding(end_reason))
            }
        } else {
            Ok(ReadInfo::DoneReceiving)
        }
    }

    /// Send data over the real-time stream. This data is queued up if it cannot be sent immediately.
    ///
    /// The real-time "stream" is different than the main stream because it uses multiple
    /// incremental QUIC unidirectional streams in the backend where each stream id represents
    /// a single time segment that has unreliability the moment when the next single time segment
    /// arrives before the previous stream (time segment) had finished.
    ///
    /// When last_send_of_time_segment is set to true, the currently transmitting time segment will
    /// finish after sending all the data that is in the queue (including the optional send_data) and
    /// the real-time stream id will be incremented for any future real-time stream send data.
    ///
    /// If not all of the send data for a previous real-time stream id has made it to the peer by the
    /// time the next time segement real-time stream data is ready to be sent (with a call to this function)
    /// then the send queue will be cleared (unreliable transmission) and the next send data will take its place.
    ///
    /// A reminder that the Endpoint connection will be taking ownership of the data so it can be sent out when possible
    pub fn rt_stream_send(
        &mut self,
        cid: &ConnectionId,
        send_data: Option<Vec<u8>>,
        last_send_of_time_segment: bool,
    ) -> Result<(), Error> {
        if let Some(verified_index) = self.find_connection_from_cid(*cid) {
            match self.connections[verified_index]
                .rt_stream_send(send_data, last_send_of_time_segment)
            {
                Ok(_) => {
                    if self.send(verified_index)?.is_some() {
                        Err(Error::UnexpectedClose)
                    } else {
                        Ok(())
                    }
                }
                Err(_) => Err(Error::StreamSend),
            }
        } else {
            Err(Error::ConnectionNotFound)
        }
    }

    pub(super) fn rt_stream_read(
        &mut self,
        verified_index: usize,
        data_vec: Vec<u8>,
        target_len: usize,
    ) -> Result<Option<(Vec<u8>, usize)>, Error> {
        match self.connections[verified_index].rt_stream_read(data_vec, target_len) {
            Ok(vec_info_opt) => {
                // if self.send(verified_index)?.is_none() {
                if let Some((vec_data, vec_len)) = vec_info_opt {
                    Ok(Some((vec_data, vec_len)))
                } else {
                    Ok(None)
                }
                // } else {
                //     Err(Error::UnexpectedClose)
                // }
            }
            Err(_) => Err(Error::StreamSend),
        }
    }

    /// Send data over the background stream. This data is queued up if it cannot be sent immediately.
    ///
    /// The background stream is a reliable (ordered) stream that focuses on communicating
    /// large(ish) messages between the server and client such as a file transfer.
    ///
    /// A reminder that the Endpoint connection will be taking ownership of the data so it can be sent out when possible
    pub fn background_stream_send(
        &mut self,
        cid: &ConnectionId,
        send_data: Vec<u8>,
    ) -> Result<(), Error> {
        if let Some(verified_index) = self.find_connection_from_cid(*cid) {
            match self.connections[verified_index].bkgd_stream_send(send_data) {
                Ok(_) => {
                    if self.send(verified_index)?.is_some() {
                        Err(Error::UnexpectedClose)
                    } else {
                        Ok(())
                    }
                }
                Err(_) => Err(Error::StreamSend),
            }
        } else {
            Err(Error::ConnectionNotFound)
        }
    }

    pub(super) fn background_stream_read(
        &mut self,
        verified_index: usize,
        data_vec: Vec<u8>,
        target_len_opt: Option<usize>,
    ) -> Result<ReadInfo, Error> {
        if let Some(mut target_len) = target_len_opt {
            if target_len == 0 {
                target_len = self.config.initial_background_recv_size;
            }
            match self.connections[verified_index].bkgd_stream_read(data_vec, target_len) {
                Ok(vec_data_opt) => {
                    // if self.send(verified_index)?.is_none() {
                    if let Some(vec_data) = vec_data_opt {
                        Ok(ReadInfo::ReadData((vec_data, target_len)))
                    } else {
                        Ok(ReadInfo::DoneReceiving)
                    }
                    // } else {
                    //     Err(Error::UnexpectedClose)
                    // }
                }
                Err(connection::Error::Done) => {
                    if let Some(close_info) = self
                        .connection_close(verified_index, EndpointCloseReason::MainStreamFinished)?
                    {
                        let end_reason = ConnectionEndReason::from_close_info(&close_info);
                        if close_info.is_closed {
                            self.remove_connection(verified_index);
                            Ok(ReadInfo::ConnectionEnded(end_reason))
                        } else {
                            Ok(ReadInfo::ConnectionEnding(end_reason))
                        }
                    } else {
                        Ok(ReadInfo::DoneReceiving)
                    }
                }
                Err(_) => Err(Error::StreamSend),
            }
        } else if let Some(close_info) =
            self.connection_close(verified_index, EndpointCloseReason::MainStreamFinished)?
        {
            let end_reason = ConnectionEndReason::from_close_info(&close_info);
            if close_info.is_closed {
                self.remove_connection(verified_index);
                Ok(ReadInfo::ConnectionEnded(end_reason))
            } else {
                Ok(ReadInfo::ConnectionEnding(end_reason))
            }
        } else {
            Ok(ReadInfo::DoneReceiving)
        }
    }

    // pub(super) fn connection_send(&mut self, verified_index: usize) -> Result<(), Error> {
    //     self.send(verified_index)?;
    //     Ok(())
    // }
}
