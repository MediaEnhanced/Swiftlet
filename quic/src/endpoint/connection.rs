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

use crate::endpoint::SocketAddr;
use std::collections::VecDeque;
use std::time::Instant;

pub(super) use quiche::Config;
pub(super) use quiche::Error;

// Real-time Communication Connection Constants
const MAIN_STREAM_ID: u64 = 0; // Bidirectional stream ID# used for reliable communication in the application between the server and the client
                               // This stream is started by the Client when it announces itself to the server when it connects to it

const MAIN_STREAM_PRIORITY: u8 = 100;
const BACKGROUND_STREAM_ID: u64 = 4;
const BACKGROUND_STREAM_PRIORITY: u8 = 200;
//const SERVER_REALTIME_START_ID: u64 = 3;
//const CLIENT_REALTIME_START_ID: u64 = 2;

struct StreamRecv {
    captured: usize,
    target: usize,
    data: Option<Vec<u8>>,
}

impl StreamRecv {
    fn new() -> Self {
        StreamRecv {
            captured: 0,
            target: 0,
            data: None,
        }
    }
}

struct SendBuffer {
    data: Vec<u8>,
    sent: usize,
}

impl SendBuffer {
    fn new(data: Vec<u8>) -> Self {
        SendBuffer { data, sent: 0 }
    }
}

// QUIC Connection (Using the quiche crate)
pub(super) struct Connection {
    id: u64,                                     // ID to be used by the application
    current_scid: quiche::ConnectionId<'static>, // Current SCID used by this connection
    connection: quiche::Connection,              // quiche Connection
    recv_info: quiche::RecvInfo,
    last_send_instant: Instant, // Used for sending PING / ACK_Elicting if it's been a while
    next_timeout_instant: Option<Instant>,
    established_once: bool,
    main_recv: StreamRecv,
    main_send_queue: VecDeque<SendBuffer>,
    background_recv: StreamRecv,
    bkgd_send_queue: VecDeque<SendBuffer>,
}

pub(super) enum RecvResult {
    Closed(u64),
    Draining(u64),
    Established(u64),
    Nothing,
    Closing(u64),
    ReliableBufferMissing,
    MainStreamReadable(u64),
    BkgdStreamReadable(u64),
    StreamReadable((u64, u64)),
}

pub(super) enum TimeoutResult {
    Nothing(Option<Instant>),
    Closed(u64),
    Draining(u64),
    Happened,
}

impl Connection {
    pub(super) fn create_config(
        alpns: &[&[u8]],
        cert_path: &str,
        pkey_path_option: Option<&str>,
        idle_timeout_in_ms: u64,
        max_payload_size: usize,
        reliable_stream_buffer: u64,
        unreliable_stream_buffer: u64,
    ) -> Result<Config, Error> {
        // A quiche Config with default values
        let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)?;

        config.set_application_protos(alpns)?;

        // Do different config things if it is a server vs a client based on pkey path availability
        if let Some(pkey_path) = pkey_path_option {
            // Maybe not return error immediately here?
            config.load_cert_chain_from_pem_file(cert_path)?;
            config.load_priv_key_from_pem_file(pkey_path)?;
            config.verify_peer(false);

            config.set_initial_max_streams_bidi(3); // 1 For Main Communication, 2 and 3 for Unordered Alt Data (like file transfers)

            // Enable the ability to log the secret keys for wireshark debugging
            config.log_keys();
        } else {
            // Temporary solution for client to verify certificate
            // Maybe not return error immediately here?
            config.load_verify_locations_from_file(cert_path)?;
            config.verify_peer(true);

            config.set_initial_max_streams_bidi(0);
        }

        config.set_initial_max_streams_uni(4); // Not sure... based on future testing

        config.set_max_idle_timeout(idle_timeout_in_ms);

        config.set_max_recv_udp_payload_size(max_payload_size);
        config.set_max_send_udp_payload_size(max_payload_size);

        config.set_initial_max_stream_data_bidi_local(reliable_stream_buffer);
        config.set_initial_max_stream_data_bidi_remote(reliable_stream_buffer);
        config.set_initial_max_stream_data_uni(unreliable_stream_buffer);

        config.set_initial_max_data(reliable_stream_buffer + (unreliable_stream_buffer * 4));

        config.enable_pacing(true); // Default that I confirm

        config.set_disable_active_migration(true); // Temporary

        // Enable datagram frames for unreliable data to be sent

        Ok(config)
    }

    #[inline]
    pub(super) fn get_empty_cid() -> [u8; quiche::MAX_CONN_ID_LEN] {
        [0; quiche::MAX_CONN_ID_LEN]
    }

    // returns true if this packet could be a new connection
    pub(super) fn recv_header_analyze(
        data: &mut [u8],
        is_server: bool,
    ) -> Option<(quiche::ConnectionId<'static>, bool)> {
        if let Ok(packet_header) = quiche::Header::from_slice(data, quiche::MAX_CONN_ID_LEN) {
            if is_server
                && packet_header.ty == quiche::Type::Initial
                && quiche::version_is_supported(packet_header.version)
            {
                // This gets reached even when Type is Handshake... look into further
                Some((packet_header.dcid, true))
            } else {
                Some((packet_header.dcid, false))
            }
        } else {
            None
        }
    }

    pub(super) fn new(
        id: u64,
        peer_addr: SocketAddr,
        server_name: Option<&str>,
        local_addr: SocketAddr,
        scid_data: &[u8],
        config: &mut quiche::Config,
        writer_opt: Option<Box<std::fs::File>>,
    ) -> Result<Self, Error> {
        let recv_info = quiche::RecvInfo {
            from: local_addr,
            to: local_addr,
        };

        // Do some connectionID length testing here in future
        let scid = quiche::ConnectionId::from_ref(&scid_data[..quiche::MAX_CONN_ID_LEN]);
        let current_scid = scid.into_owned();

        if server_name.is_some() {
            // Create client connection

            let connection =
                quiche::connect(server_name, &current_scid, local_addr, peer_addr, config)?;

            let conn_mgr = Connection {
                id,
                current_scid,
                connection,
                recv_info,
                last_send_instant: Instant::now(),
                next_timeout_instant: None,
                established_once: false,
                main_recv: StreamRecv::new(),
                main_send_queue: VecDeque::new(),
                background_recv: StreamRecv::new(),
                bkgd_send_queue: VecDeque::new(),
            };

            Ok(conn_mgr)
        } else {
            // Create server connection
            let connection =
                match quiche::accept(&current_scid, None, local_addr, peer_addr, config) {
                    Ok(mut conn) => {
                        if let Some(writer) = writer_opt {
                            // called before recv
                            conn.set_keylog(writer);
                        }
                        conn
                    }
                    Err(err) => {
                        return Err(err);
                    }
                };

            let conn_mgr = Connection {
                id,
                current_scid,
                connection,
                recv_info,
                last_send_instant: Instant::now(),
                next_timeout_instant: None,
                established_once: false,
                main_recv: StreamRecv::new(),
                main_send_queue: VecDeque::new(),
                background_recv: StreamRecv::new(),
                bkgd_send_queue: VecDeque::new(),
            };

            Ok(conn_mgr)
        }
    }

    #[inline]
    pub(super) fn matches_id(&self, id: u64) -> bool {
        self.id == id
    }

    #[inline]
    pub(super) fn matches_dcid(&self, dcid: &[u8]) -> bool {
        self.current_scid.as_ref() == dcid
    }

    pub(super) fn get_next_send_packet(
        &mut self,
        packet_data: &mut [u8],
    ) -> Result<Option<(usize, SocketAddr, Instant)>, Error> {
        match self.connection.send(packet_data) {
            Ok((packet_len, send_info)) => {
                if send_info.at > self.last_send_instant {
                    self.last_send_instant = send_info.at;
                }
                Ok(Some((packet_len, send_info.to, send_info.at)))
            }
            Err(quiche::Error::Done) => {
                self.next_timeout_instant = self.connection.timeout_instant();
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    pub(super) fn handle_possible_timeout(&mut self) -> TimeoutResult {
        if let Some(timeout_instant) = self.next_timeout_instant {
            let now = Instant::now();
            if timeout_instant <= now {
                // Verifies that a timeout occurred and then processes it
                self.next_timeout_instant = self.connection.timeout_instant();
                if let Some(timeout_verify) = self.next_timeout_instant {
                    if timeout_verify <= now {
                        self.connection.on_timeout();
                        if self.connection.is_closed() {
                            TimeoutResult::Closed(self.id)
                        } else if self.connection.is_draining() {
                            TimeoutResult::Draining(self.id)
                        } else {
                            TimeoutResult::Happened
                        }
                    } else {
                        TimeoutResult::Nothing(self.next_timeout_instant)
                    }
                } else {
                    TimeoutResult::Nothing(self.next_timeout_instant)
                }
            } else {
                TimeoutResult::Nothing(self.next_timeout_instant)
            }
        } else {
            TimeoutResult::Nothing(self.next_timeout_instant)
        }
    }

    fn main_stream_send_next(&mut self) -> Result<usize, Error> {
        let mut total_bytes_sent = 0;
        loop {
            if let Some(send_buf) = self.main_send_queue.front_mut() {
                match self.connection.stream_send(
                    MAIN_STREAM_ID,
                    &send_buf.data[send_buf.sent..],
                    false,
                ) {
                    Ok(bytes_sent) => {
                        total_bytes_sent += bytes_sent;
                        send_buf.sent += bytes_sent;
                        if send_buf.sent >= send_buf.data.len() {
                            self.main_send_queue.pop_front();
                        } else {
                            return Ok(total_bytes_sent);
                        }
                    }
                    Err(Error::Done) => {
                        return Ok(total_bytes_sent);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            } else {
                return Ok(total_bytes_sent);
            }
        }
    }

    fn bkgd_stream_send_next(&mut self) -> Result<usize, Error> {
        let mut total_bytes_sent = 0;
        loop {
            if let Some(send_buf) = self.bkgd_send_queue.front_mut() {
                match self.connection.stream_send(
                    BACKGROUND_STREAM_ID,
                    &send_buf.data[send_buf.sent..],
                    false,
                ) {
                    Ok(bytes_sent) => {
                        total_bytes_sent += bytes_sent;
                        send_buf.sent += bytes_sent;
                        if send_buf.sent >= send_buf.data.len() {
                            self.bkgd_send_queue.pop_front();
                        } else {
                            return Ok(total_bytes_sent);
                        }
                    }
                    Err(Error::Done) => {
                        return Ok(total_bytes_sent);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            } else {
                return Ok(total_bytes_sent);
            }
        }
    }

    pub(super) fn recv_data_process(
        &mut self,
        data: &mut [u8],
        from_addr: SocketAddr,
    ) -> Result<RecvResult, Error> {
        self.recv_info.from = from_addr;
        let _ = self.connection.recv(data, self.recv_info)?;
        // Maybe check bytes_processed in future
        if self.established_once {
            if self.connection.is_closed() {
                Ok(RecvResult::Closed(self.id))
            } else if self.connection.is_draining() {
                Ok(RecvResult::Draining(self.id))
            } else {
                self.main_stream_send_next()?;
                self.bkgd_stream_send_next()?;

                if let Some(next_readable_stream) = self.connection.stream_readable_next() {
                    if next_readable_stream == MAIN_STREAM_ID {
                        if self.main_recv.captured >= self.main_recv.target {
                            Ok(RecvResult::MainStreamReadable(self.id))
                        } else if let Some(recv_data) = &mut self.main_recv.data {
                            let (bytes_read, is_finished) = self.connection.stream_recv(
                                MAIN_STREAM_ID,
                                &mut recv_data[self.main_recv.captured..self.main_recv.target],
                            )?; // Shouldn't throw a done since it was stated to be readable
                            if !is_finished {
                                self.main_recv.captured += bytes_read;
                                if self.main_recv.captured >= self.main_recv.target {
                                    Ok(RecvResult::MainStreamReadable(self.id))
                                } else {
                                    Ok(RecvResult::Nothing)
                                }
                            } else {
                                self.connection.close(false, 1, b"Stream0Finished")?;
                                Ok(RecvResult::Closing(self.id))
                            }
                        } else {
                            Ok(RecvResult::Nothing)
                        }
                    } else if next_readable_stream == BACKGROUND_STREAM_ID {
                        if self.background_recv.captured >= self.background_recv.target {
                            Ok(RecvResult::BkgdStreamReadable(self.id))
                        } else if let Some(recv_data) = &mut self.background_recv.data {
                            let (bytes_read, is_finished) = self.connection.stream_recv(
                                BACKGROUND_STREAM_ID,
                                &mut recv_data
                                    [self.background_recv.captured..self.background_recv.target],
                            )?; // Shouldn't throw a done since it was stated to be readable
                            if !is_finished {
                                self.background_recv.captured += bytes_read;
                                if self.background_recv.captured >= self.background_recv.target {
                                    Ok(RecvResult::BkgdStreamReadable(self.id))
                                } else {
                                    Ok(RecvResult::Nothing)
                                }
                            } else {
                                self.connection.close(false, 1, b"Stream4Finished")?;
                                Ok(RecvResult::Closing(self.id))
                            }
                        } else {
                            Ok(RecvResult::Nothing)
                        }
                    } else {
                        Ok(RecvResult::StreamReadable((self.id, next_readable_stream)))
                    }
                } else {
                    Ok(RecvResult::ReliableBufferMissing)
                }
            }
        } else if self.connection.is_established() {
            Ok(RecvResult::Established(self.id))
        } else if self.connection.is_closed() {
            Ok(RecvResult::Closed(self.id))
        } else if self.connection.is_draining() {
            Ok(RecvResult::Draining(self.id))
        } else {
            Ok(RecvResult::Nothing)
        }
    }

    pub(super) fn finish_establishment(
        &mut self,
        main_recv_data: Vec<u8>,
        main_recv_bytes: usize,
        background_recv_data: Vec<u8>,
        background_recv_bytes: usize,
    ) -> Result<(), Error> {
        // Create streams depending on connection type:
        if !self.connection.is_server() {
            self.connection
                .stream_priority(MAIN_STREAM_ID, MAIN_STREAM_PRIORITY, true)?;
            self.connection.stream_priority(
                BACKGROUND_STREAM_ID,
                BACKGROUND_STREAM_PRIORITY,
                true,
            )?;
        }

        self.main_recv.target = main_recv_bytes;
        self.main_recv.data = Some(main_recv_data);
        self.background_recv.target = background_recv_bytes;
        self.background_recv.data = Some(background_recv_data);

        self.established_once = true;
        Ok(())
    }

    #[inline]
    pub(super) fn close(&mut self, err: u64, reason: &[u8]) -> Result<bool, Error> {
        self.connection.close(false, err, reason)?;
        Ok(true)
    }

    pub(super) fn send_ping_if_before_instant(&mut self, instant: Instant) -> Result<bool, Error> {
        if self.last_send_instant > instant {
            Ok(false)
        } else {
            self.connection.send_ack_eliciting()?;
            Ok(true)
        }
    }

    pub(super) fn main_stream_send(&mut self, data_vec: Vec<u8>) -> Result<usize, Error> {
        self.main_send_queue.push_back(SendBuffer::new(data_vec));
        self.main_stream_send_next()
    }

    pub(super) fn main_stream_next_target(&mut self, next_target: usize, mut data_vec: Vec<u8>) {
        if next_target > data_vec.len() {
            data_vec.resize(next_target, 0);
        }
        self.main_recv.captured = 0;
        self.main_recv.target = next_target;
        self.main_recv.data = Some(data_vec);
    }

    pub(super) fn main_stream_read(&mut self) -> Result<(Option<usize>, Option<Vec<u8>>), Error> {
        if self.main_recv.captured >= self.main_recv.target {
            Ok((Some(self.main_recv.target), self.main_recv.data.take()))
        } else if let Some(recv_data) = &mut self.main_recv.data {
            match self.connection.stream_recv(
                MAIN_STREAM_ID,
                &mut recv_data[self.main_recv.captured..self.main_recv.target],
            ) {
                Ok((bytes_read, is_finished)) => {
                    if !is_finished {
                        self.main_recv.captured += bytes_read;
                        if self.main_recv.captured >= self.main_recv.target {
                            Ok((Some(self.main_recv.target), self.main_recv.data.take()))
                        } else {
                            Ok((None, None))
                        }
                    } else {
                        self.connection.close(false, 1, b"Stream0Finished")?;

                        // Maybe add closing awareness here later
                        //Ok(RecvResult::Closing(self.id))
                        Ok((None, None))
                    }
                }
                Err(Error::Done) => Ok((None, None)),
                Err(e) => Err(e),
            }
        } else {
            Err(Error::InvalidState) // Temporarily used to indicate No recv_data buffer
        }
    }

    pub(super) fn bkgd_stream_send(&mut self, data_vec: Vec<u8>) -> Result<usize, Error> {
        self.bkgd_send_queue.push_back(SendBuffer::new(data_vec));
        self.bkgd_stream_send_next()
    }

    pub(super) fn bkgd_stream_next_target(&mut self, next_target: usize, mut data_vec: Vec<u8>) {
        if next_target > data_vec.len() {
            data_vec.resize(next_target, 0);
        }
        self.background_recv.captured = 0;
        self.background_recv.target = next_target;
        self.background_recv.data = Some(data_vec);
    }

    pub(super) fn bkgd_stream_read(&mut self) -> Result<(Option<usize>, Option<Vec<u8>>), Error> {
        if self.background_recv.captured >= self.background_recv.target {
            Ok((
                Some(self.background_recv.target),
                self.background_recv.data.take(),
            ))
        } else if let Some(recv_data) = &mut self.background_recv.data {
            match self.connection.stream_recv(
                MAIN_STREAM_ID,
                &mut recv_data[self.background_recv.captured..self.background_recv.target],
            ) {
                Ok((bytes_read, is_finished)) => {
                    if !is_finished {
                        self.background_recv.captured += bytes_read;
                        if self.background_recv.captured >= self.background_recv.target {
                            Ok((
                                Some(self.background_recv.target),
                                self.background_recv.data.take(),
                            ))
                        } else {
                            Ok((None, None))
                        }
                    } else {
                        self.connection.close(false, 1, b"Stream4Finished")?;

                        // Maybe add closing awareness here later
                        //Ok(RecvResult::Closing(self.id))
                        Ok((None, None))
                    }
                }
                Err(Error::Done) => Ok((None, None)),
                Err(e) => Err(e),
            }
        } else {
            Err(Error::InvalidState) // Temporarily used to indicate No recv_data buffer
        }
    }
}
