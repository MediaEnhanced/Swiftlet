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

// Communication Connection Constants
// Bidirectional Stream ID# used for the main reliable communication in the application between the server and the client (started by client)
// This stream has the first (#1) send priority compared to other streams
const MAIN_STREAM_ID: u64 = 0;
const MAIN_STREAM_PRIORITY: u8 = 100; //

// Real-time Unidirectional Stream ID# start constants used for "unreliable" communication in the application
const SERVER_REALTIME_START_ID: u64 = 3;
const CLIENT_REALTIME_START_ID: u64 = 2;

// Bidirectional Stream ID# used for the background reliable communication in the application between the server and the client (started by client)
// This stream has the last send priority compared to other streams
const BACKGROUND_STREAM_ID: u64 = 4;
const BACKGROUND_STREAM_PRIORITY: u8 = 200;

struct StreamRecv {
    captured: usize,
    target: usize,
    data: Option<Vec<u8>>,
}

impl StreamRecv {
    fn empty() -> Self {
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

struct RealtimeRecv {
    id: u64,
    captured: usize,
    initial_target: usize,
    target: usize,
    data: Option<Vec<u8>>,
    count: u64,
}

impl RealtimeRecv {
    fn empty(is_server: bool) -> Self {
        let id = if is_server {
            CLIENT_REALTIME_START_ID
        } else {
            SERVER_REALTIME_START_ID
        };
        RealtimeRecv {
            id,
            captured: 0,
            initial_target: 0,
            target: 0,
            data: None,
            count: 0,
        }
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
    rt_recv: RealtimeRecv,
    rt_send_queue: VecDeque<SendBuffer>,
    rt_send_finished: bool,
    rt_send_stream_id: u64,
    bkgd_recv: StreamRecv,
    bkgd_send_queue: VecDeque<SendBuffer>,
}

pub(super) enum CloseOrigin {
    Unknown, // Uncertain why the connection is closed/closing
    Timeout, // Connection closed/closing due to Idle Timeout
    Local,   // Local connection closed itself
    Peer,    // Peer closed the connection
}

pub(super) struct CloseInfo {
    pub(super) id: u64,         // This connection ID
    pub(super) is_closed: bool, // True only if connection is completely closed
    pub(super) close_origin: CloseOrigin,
    // The following parameters don't really apply to a timeout or unknown closure
    pub(super) is_application_error: bool, // True only if error came from the application
    pub(super) error_code: u64,            // Code associated with the error
}

pub(super) enum SendResult {
    Done,
    CloseInfo(CloseInfo),
    DataToSend((usize, SocketAddr, Instant)),
}

pub(super) enum RecvResult {
    Nothing,
    CloseInitiated,
    CloseInfo(CloseInfo),
    Established(u64),
    StreamProcess(u64),
}

pub(super) enum StreamResult {
    NoMore,
    Nothing,
    MainStreamReadable((Vec<u8>, usize)),
    RealtimeStreamReadable((Vec<u8>, usize, u64)),
    BkgdStreamReadable((Vec<u8>, usize)),
    MainStreamFinished,
    BkgdStreamFinished,
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

            config.set_initial_max_streams_bidi(2); // 1 For Main Communication, 2 and 3 for Unordered Alt Data (like file transfers)
        } else {
            // Temporary solution for client to verify certificate
            // Maybe not return error immediately here?
            config.load_verify_locations_from_file(cert_path)?;
            config.verify_peer(true);

            config.set_initial_max_streams_bidi(0);
        }

        // Enable the ability to log the secret keys for wireshark debugging
        config.log_keys();

        // Malicious Second Chance Add In Future
        config.set_initial_max_streams_uni(100); // Based on 1 second of 10ms Real-time Streams

        config.set_max_idle_timeout(idle_timeout_in_ms);

        config.set_max_recv_udp_payload_size(max_payload_size);
        config.set_max_send_udp_payload_size(max_payload_size);

        config.set_initial_max_stream_data_bidi_local(reliable_stream_buffer);
        config.set_initial_max_stream_data_bidi_remote(reliable_stream_buffer);
        config.set_initial_max_stream_data_uni(unreliable_stream_buffer);

        config
            .set_initial_max_data((reliable_stream_buffer * 2) + (unreliable_stream_buffer * 100));

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

            let mut connection =
                quiche::connect(server_name, &current_scid, local_addr, peer_addr, config)?;

            if let Some(writer) = writer_opt {
                // called before recv
                connection.set_keylog(writer);
            }

            let conn_mgr = Connection {
                id,
                current_scid,
                connection,
                recv_info,
                last_send_instant: Instant::now(),
                next_timeout_instant: None,
                established_once: false,
                main_recv: StreamRecv::empty(),
                main_send_queue: VecDeque::with_capacity(4),
                rt_recv: RealtimeRecv::empty(false),
                rt_send_queue: VecDeque::with_capacity(4),
                rt_send_finished: false,
                rt_send_stream_id: CLIENT_REALTIME_START_ID,
                bkgd_recv: StreamRecv::empty(),
                bkgd_send_queue: VecDeque::with_capacity(4),
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
                main_recv: StreamRecv::empty(),
                main_send_queue: VecDeque::with_capacity(4),
                rt_recv: RealtimeRecv::empty(true),
                rt_send_queue: VecDeque::with_capacity(4),
                rt_send_finished: false,
                rt_send_stream_id: SERVER_REALTIME_START_ID,
                bkgd_recv: StreamRecv::empty(),
                bkgd_send_queue: VecDeque::with_capacity(4),
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

    // Better way to write this?
    pub(super) fn get_close_info(&self) -> Option<CloseInfo> {
        if self.connection.is_closed() {
            if self.connection.is_timed_out() {
                Some(CloseInfo {
                    id: self.id,
                    is_closed: true,
                    close_origin: CloseOrigin::Timeout,
                    is_application_error: false,
                    error_code: 0,
                })
            } else if let Some(conn_info) = self.connection.local_error() {
                Some(CloseInfo {
                    id: self.id,
                    is_closed: true,
                    close_origin: CloseOrigin::Local,
                    is_application_error: conn_info.is_app,
                    error_code: conn_info.error_code,
                })
            } else if let Some(conn_info) = self.connection.peer_error() {
                Some(CloseInfo {
                    id: self.id,
                    is_closed: true,
                    close_origin: CloseOrigin::Peer,
                    is_application_error: conn_info.is_app,
                    error_code: conn_info.error_code,
                })
            } else {
                Some(CloseInfo {
                    id: self.id,
                    is_closed: true,
                    close_origin: CloseOrigin::Unknown,
                    is_application_error: false,
                    error_code: 0,
                })
            }
        } else if self.connection.is_draining() {
            if self.connection.is_timed_out() {
                Some(CloseInfo {
                    id: self.id,
                    is_closed: false,
                    close_origin: CloseOrigin::Timeout,
                    is_application_error: false,
                    error_code: 0,
                })
            } else if let Some(conn_info) = self.connection.local_error() {
                Some(CloseInfo {
                    id: self.id,
                    is_closed: false,
                    close_origin: CloseOrigin::Local,
                    is_application_error: conn_info.is_app,
                    error_code: conn_info.error_code,
                })
            } else if let Some(conn_info) = self.connection.peer_error() {
                Some(CloseInfo {
                    id: self.id,
                    is_closed: false,
                    close_origin: CloseOrigin::Peer,
                    is_application_error: conn_info.is_app,
                    error_code: conn_info.error_code,
                })
            } else {
                Some(CloseInfo {
                    id: self.id,
                    is_closed: false,
                    close_origin: CloseOrigin::Unknown,
                    is_application_error: false,
                    error_code: 0,
                })
            }
        } else {
            None
        }
    }

    pub(super) fn get_next_send_packet(
        &mut self,
        packet_data: &mut [u8],
    ) -> Result<SendResult, Error> {
        match self.connection.send(packet_data) {
            Ok((packet_len, send_info)) => {
                if send_info.at > self.last_send_instant {
                    self.last_send_instant = send_info.at;
                }
                Ok(SendResult::DataToSend((
                    packet_len,
                    send_info.to,
                    send_info.at,
                )))
            }
            Err(quiche::Error::Done) => {
                if let Some(close_info) = self.get_close_info() {
                    if !close_info.is_closed {
                        self.next_timeout_instant = self.connection.timeout_instant();
                    }
                    Ok(SendResult::CloseInfo(close_info))
                } else {
                    self.next_timeout_instant = self.connection.timeout_instant();
                    Ok(SendResult::Done)
                }
            }
            Err(e) => Err(e),
        }
    }

    // Returns None when a timeout occurred
    pub(super) fn handle_possible_timeout(&mut self) -> Option<Option<Instant>> {
        if let Some(timeout_instant) = self.next_timeout_instant {
            let now = Instant::now();
            if timeout_instant <= now {
                // Verifies that a timeout occurred and then processes it
                self.next_timeout_instant = self.connection.timeout_instant();
                if let Some(timeout_verify) = self.next_timeout_instant {
                    if timeout_verify <= now {
                        self.connection.on_timeout();
                        return None;
                    }
                }
            }
        }
        Some(self.next_timeout_instant)
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

    fn rt_stream_send_next(&mut self) -> Result<usize, Error> {
        let mut total_bytes_sent = 0;
        loop {
            // Finish logic should be correct here based on the internals of stream_send()
            let fin = self.rt_send_finished && (self.rt_send_queue.len() == 1);
            if let Some(send_buf) = self.rt_send_queue.front_mut() {
                match self.connection.stream_send(
                    self.rt_send_stream_id,
                    &send_buf.data[send_buf.sent..],
                    fin,
                ) {
                    Ok(bytes_sent) => {
                        total_bytes_sent += bytes_sent;
                        send_buf.sent += bytes_sent;
                        if send_buf.sent >= send_buf.data.len() {
                            self.rt_send_queue.pop_front();
                            if fin {
                                self.rt_send_stream_id += 4;
                                self.rt_send_finished = false;
                            }
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

    pub(super) fn recv_data(
        &mut self,
        data: &mut [u8],
        from_addr: SocketAddr,
    ) -> Result<RecvResult, Error> {
        self.recv_info.from = from_addr;
        if let Err(e) = self.connection.recv(data, self.recv_info) {
            // if let Some(local_err) = self.connection.local_error() {
            //     Some(CloseInfo {
            //         id: self.id,
            //         is_closed: false,
            //         close_origin: CloseOrigin::Local,
            //         is_application_error: local_err.is_app,
            //         error_code: local_err.error_code,
            //     })
            // }
            if self.connection.local_error().is_some() {
                return Ok(RecvResult::CloseInitiated);
            }
            return Err(e);
        }
        if let Some(close_info) = self.get_close_info() {
            return Ok(RecvResult::CloseInfo(close_info));
        }

        if self.established_once {
            self.main_stream_send_next()?;
            self.rt_stream_send_next()?;
            self.bkgd_stream_send_next()?;

            Ok(RecvResult::StreamProcess(self.id))
        } else if self.connection.is_established() {
            Ok(RecvResult::Established(self.id))
        } else {
            Ok(RecvResult::Nothing)
        }
    }

    pub(super) fn finish_establishment(
        &mut self,
        main_recv_data: Vec<u8>,
        main_recv_bytes: usize,
        rt_recv_data: Vec<u8>,
        rt_recv_bytes_initial: usize,
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
        self.rt_recv.initial_target = rt_recv_bytes_initial;
        self.rt_recv.target = self.rt_recv.initial_target;
        self.rt_recv.data = Some(rt_recv_data);
        self.bkgd_recv.target = background_recv_bytes;
        self.bkgd_recv.data = Some(background_recv_data);

        self.established_once = true;
        Ok(())
    }

    // A returned Error::InvalidState indicates something went wrong with the read process
    pub(super) fn stream_process(&mut self) -> Result<StreamResult, Error> {
        if let Some(next_readable_stream) = self.connection.stream_readable_next() {
            if next_readable_stream == MAIN_STREAM_ID {
                if let Some(mut recv_data) = self.main_recv.data.take() {
                    let (bytes_read, is_finished) = self.connection.stream_recv(
                        MAIN_STREAM_ID,
                        &mut recv_data[self.main_recv.captured..self.main_recv.target],
                    )?; // Shouldn't throw a done since it was stated to be readable
                    if !is_finished {
                        self.main_recv.captured += bytes_read;
                        #[allow(clippy::comparison_chain)]
                        if self.main_recv.captured == self.main_recv.target {
                            Ok(StreamResult::MainStreamReadable((
                                recv_data,
                                self.main_recv.target,
                            )))
                        } else if self.main_recv.captured < self.main_recv.target {
                            self.main_recv.data = Some(recv_data);
                            Ok(StreamResult::Nothing)
                        } else {
                            Err(Error::InvalidStreamState(10))
                        }
                    } else {
                        Ok(StreamResult::MainStreamFinished)
                    }
                } else {
                    Err(Error::InvalidStreamState(11))
                }
            } else if next_readable_stream == BACKGROUND_STREAM_ID {
                if let Some(mut recv_data) = self.bkgd_recv.data.take() {
                    let (bytes_read, is_finished) = self.connection.stream_recv(
                        BACKGROUND_STREAM_ID,
                        &mut recv_data[self.bkgd_recv.captured..self.bkgd_recv.target],
                    )?; // Shouldn't throw a done since it was stated to be readable
                    if !is_finished {
                        self.bkgd_recv.captured += bytes_read;
                        #[allow(clippy::comparison_chain)]
                        if self.bkgd_recv.captured == self.bkgd_recv.target {
                            Ok(StreamResult::BkgdStreamReadable((
                                recv_data,
                                self.bkgd_recv.target,
                            )))
                        } else if self.bkgd_recv.captured < self.bkgd_recv.target {
                            self.bkgd_recv.data = Some(recv_data);
                            Ok(StreamResult::Nothing)
                        } else {
                            Err(Error::InvalidStreamState(12))
                        }
                    } else {
                        Ok(StreamResult::BkgdStreamFinished)
                    }
                } else {
                    Err(Error::InvalidStreamState(13))
                }
            } else if !self.connection.stream_finished(next_readable_stream) {
                if let Some(recv_data) = self.rt_recv.data.take() {
                    self.stream_process_realtime(next_readable_stream, recv_data)
                } else {
                    Err(Error::InvalidStreamState(14))
                }
            } else {
                let mut temp_data = [0; 8];
                match self
                    .connection
                    .stream_recv(next_readable_stream, &mut temp_data)
                {
                    Err(Error::StreamReset(_)) => {
                        return Ok(StreamResult::Nothing);
                    }
                    Err(Error::Done) => Ok(StreamResult::Nothing),
                    Err(e) => Err(e),
                    Ok(_) => Ok(StreamResult::Nothing),
                }
            }
        } else {
            Ok(StreamResult::NoMore)
        }
    }

    fn stream_process_realtime(
        &mut self,
        next_readable_stream: u64,
        mut recv_data: Vec<u8>,
    ) -> Result<StreamResult, Error> {
        if next_readable_stream > self.rt_recv.id {
            let stream_id_difference = next_readable_stream - self.rt_recv.id;
            if (stream_id_difference & 0x3) > 0 {
                return Err(Error::InvalidStreamState(15));
            }
            //let rt_period = stream_id_difference >> 2;
            for i in (0..stream_id_difference).step_by(4) {
                match self.connection.stream_shutdown(
                    self.rt_recv.id + i,
                    quiche::Shutdown::Read,
                    self.rt_recv.id + i,
                ) {
                    Ok(_) => {}
                    Err(Error::Done) => {}
                    Err(e) => return Err(e),
                }
            }
            self.rt_recv.id = next_readable_stream;
            self.rt_recv.captured = 0;
            self.rt_recv.count += stream_id_difference >> 2;
            self.rt_recv.target = self.rt_recv.initial_target;
        } else if next_readable_stream != self.rt_recv.id {
            // Look more into but this specifies that next_readable_stream should just be ignored!
            self.rt_recv.data = Some(recv_data);
            return Ok(StreamResult::Nothing);
            // return Err(Error::InvalidStreamState(16));
        }

        if self.rt_recv.target == 0 {
            // Loop for the potential case of resizing recv_data
            loop {
                match self
                    .connection
                    .stream_recv(self.rt_recv.id, &mut recv_data[self.rt_recv.captured..])
                {
                    Ok((bytes_read, is_finished)) => {
                        self.rt_recv.captured += bytes_read;
                        if is_finished {
                            return Ok(StreamResult::RealtimeStreamReadable((
                                recv_data,
                                self.rt_recv.captured,
                                self.rt_recv.count,
                            )));
                        } else if self.rt_recv.captured < recv_data.len() {
                            self.rt_recv.data = Some(recv_data);
                            return Ok(StreamResult::Nothing);
                        } else if self.rt_recv.captured == recv_data.len() {
                            recv_data.resize(self.rt_recv.captured + 65536, 0);
                        } else {
                            return Err(Error::InvalidStreamState(17));
                        }
                    }
                    Err(Error::Done) => {
                        self.rt_recv.data = Some(recv_data);
                        return Ok(StreamResult::Nothing);
                    }
                    // Err(Error::StreamReset(_)) => {
                    //     self.rt_recv.data = Some(recv_data);
                    //     return Ok(StreamResult::Nothing);
                    // }
                    Err(e) => return Err(e),
                }
            }
        } else {
            // println!(
            //     "                           First Length:                               {}",
            //     self.rt_recv.target
            // );
            match self.connection.stream_recv(
                self.rt_recv.id,
                &mut recv_data[self.rt_recv.captured..self.rt_recv.target],
            ) {
                Ok((bytes_read, is_finished)) => {
                    self.rt_recv.captured += bytes_read;
                    if !is_finished {
                        #[allow(clippy::comparison_chain)]
                        if self.rt_recv.captured == self.rt_recv.target {
                            Ok(StreamResult::RealtimeStreamReadable((
                                recv_data,
                                self.rt_recv.target,
                                self.rt_recv.count,
                            )))
                        } else if self.rt_recv.captured < self.rt_recv.target {
                            self.rt_recv.data = Some(recv_data);
                            Ok(StreamResult::Nothing)
                        } else {
                            Err(Error::InvalidStreamState(18))
                        }
                    } else if self.rt_recv.captured == self.rt_recv.target {
                        //self.rt_recv.target = 0; // Why was this here...?
                        Ok(StreamResult::RealtimeStreamReadable((
                            recv_data,
                            self.rt_recv.captured,
                            self.rt_recv.count,
                        )))
                    } else if self.rt_recv.captured < self.rt_recv.target {
                        // Unexpected finish (recoverable)
                        self.rt_recv.id += 4;
                        self.rt_recv.captured = 0;
                        self.rt_recv.count += 1;
                        self.rt_recv.target = self.rt_recv.initial_target;
                        Err(Error::Done)
                    } else {
                        Err(Error::InvalidStreamState(19))
                    }
                }
                Err(Error::Done) => {
                    //self.rt_recv.data = Some(recv_data);
                    //Ok(StreamResult::Nothing)
                    Err(Error::Done)
                }
                // Err(Error::StreamReset(_)) => {
                //     println!("Still Happens!");
                //     self.rt_recv.data = Some(recv_data);
                //     self.rt_recv.id += 4;
                //     self.rt_recv.captured = 0;
                //     self.rt_recv.count += 1;
                //     Ok(StreamResult::Nothing)
                // }
                Err(e) => Err(e),
            }
        }
    }

    // Endpoint Connection Close Error Code
    #[inline]
    pub(super) fn close(&mut self, err: u64, reason: &[u8]) -> Result<bool, Error> {
        self.connection.close(false, err, reason)?;
        Ok(true)
    }

    // Application Connection Close Error Code
    #[inline]
    pub(super) fn app_close(&mut self, err: u64, reason: &[u8]) -> Result<bool, Error> {
        self.connection.close(true, err, reason)?;
        Ok(true)
    }

    pub(super) fn send_ping_if_before_instant(&mut self, instant: Instant) -> Result<bool, Error> {
        if self.established_once {
            if self.last_send_instant > instant {
                Ok(false)
            } else {
                self.connection.send_ack_eliciting()?;
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }

    pub(super) fn get_socket_addr(&self) -> SocketAddr {
        self.recv_info.from
    }

    pub(super) fn main_stream_send(&mut self, data_vec: Vec<u8>) -> Result<usize, Error> {
        self.main_send_queue.push_back(SendBuffer::new(data_vec));
        self.main_stream_send_next()
    }

    // A returned Error::InvalidState indicates something went wrong with the read process
    // A returned Error::Done indicates the stream finished
    pub(super) fn main_stream_read(
        &mut self,
        mut data_vec: Vec<u8>,
        target_len: usize,
    ) -> Result<Option<Vec<u8>>, Error> {
        if target_len > data_vec.len() {
            data_vec.resize(target_len, 0);
        }
        match self
            .connection
            .stream_recv(MAIN_STREAM_ID, &mut data_vec[..target_len])
        {
            Ok((bytes_read, is_finished)) => {
                if !is_finished {
                    #[allow(clippy::comparison_chain)]
                    if bytes_read == target_len {
                        Ok(Some(data_vec))
                    } else if bytes_read < target_len {
                        self.main_recv.captured = bytes_read;
                        self.main_recv.target = target_len;
                        self.main_recv.data = Some(data_vec);
                        Ok(None)
                    } else {
                        Err(Error::InvalidState)
                    }
                } else {
                    Err(Error::Done)
                }
            }
            Err(Error::Done) => {
                self.main_recv.captured = 0;
                self.main_recv.target = target_len;
                self.main_recv.data = Some(data_vec);
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    pub(super) fn rt_stream_send(
        &mut self,
        data_vec_opt: Option<Vec<u8>>,
        last_send_of_time_segment: bool,
    ) -> Result<usize, Error> {
        if self.rt_send_finished {
            // Clear send queue and "finish" / shutdown the current send stream here
            self.rt_send_queue.clear();
            self.connection.stream_shutdown(
                self.rt_send_stream_id,
                quiche::Shutdown::Write,
                self.rt_send_stream_id,
            )?;
            // Increment Stream ID
            self.rt_send_stream_id += 4;
            self.rt_send_finished = false;
        }
        if let Some(data_vec) = data_vec_opt {
            self.rt_send_queue.push_back(SendBuffer::new(data_vec));
        }
        if last_send_of_time_segment {
            self.rt_send_finished = true;
        }

        self.rt_stream_send_next()
    }

    // A returned Error::InvalidState indicates something went wrong with the read process
    pub(super) fn rt_stream_read(
        &mut self,
        mut data_vec: Vec<u8>,
        target_len: usize,
    ) -> Result<Option<(Vec<u8>, usize)>, Error> {
        if self.rt_recv.target == 0 {
            self.rt_recv.id += 4;
            self.rt_recv.captured = 0;
            self.rt_recv.target = target_len;
            self.rt_recv.data = Some(data_vec);
            self.rt_recv.count += 1;
            return Ok(None);
        }

        // Weird quiche?? error occurs if this test isn't here
        // Might get moved around in future
        if self.connection.stream_finished(self.rt_recv.id) {
            self.rt_recv.data = Some(data_vec);
            Ok(None)
        } else if target_len == 0 {
            self.rt_recv.captured = 0;
            self.rt_recv.target = target_len;
            loop {
                match self
                    .connection
                    .stream_recv(self.rt_recv.id, &mut data_vec[self.rt_recv.captured..])
                {
                    Ok((bytes_read, is_finished)) => {
                        self.rt_recv.captured += bytes_read;
                        if is_finished {
                            return Ok(Some((data_vec, self.rt_recv.captured)));
                        } else if self.rt_recv.captured < data_vec.len() {
                            self.rt_recv.data = Some(data_vec);
                            return Ok(None);
                        } else if self.rt_recv.captured == data_vec.len() {
                            data_vec.resize(self.rt_recv.captured + 65536, 0);
                        } else {
                            return Err(Error::InvalidState);
                        }
                    }
                    Err(Error::Done) => {
                        self.rt_recv.data = Some(data_vec);
                        return Ok(None);
                    }
                    Err(e) => return Err(e),
                }
            }
        } else {
            // println!(
            //     "                        Target Length:                               {}",
            //     target_len
            // );
            if target_len > data_vec.len() {
                data_vec.resize(target_len, 0);
            }
            match self
                .connection
                .stream_recv(self.rt_recv.id, &mut data_vec[..target_len])
            {
                Ok((bytes_read, is_finished)) => {
                    // Tests can get rearranged in future
                    if !is_finished {
                        #[allow(clippy::comparison_chain)]
                        if bytes_read == target_len {
                            Ok(Some((data_vec, target_len)))
                        } else if bytes_read < target_len {
                            self.rt_recv.captured = bytes_read;
                            self.rt_recv.target = target_len;
                            self.rt_recv.data = Some(data_vec);
                            Ok(None)
                        } else {
                            Err(Error::InvalidState)
                        }
                    } else if bytes_read == target_len {
                        Ok(Some((data_vec, target_len)))
                    } else if bytes_read < target_len {
                        // Need to change this later
                        // Based on incorrect realtime application protocol stuff
                        Err(Error::InvalidStreamState(20))
                    } else {
                        Err(Error::InvalidState)
                    }
                }
                Err(Error::Done) => {
                    self.rt_recv.captured = 0;
                    self.rt_recv.target = target_len;
                    self.rt_recv.data = Some(data_vec);
                    Ok(None)
                }
                Err(e) => Err(e),
            }
        }
    }

    pub(super) fn bkgd_stream_send(&mut self, data_vec: Vec<u8>) -> Result<usize, Error> {
        self.bkgd_send_queue.push_back(SendBuffer::new(data_vec));
        self.bkgd_stream_send_next()
    }

    // A returned Error::InvalidState indicates something went wrong with the read process
    // A returned Error::Done indicates the stream finished
    pub(super) fn bkgd_stream_read(
        &mut self,
        mut data_vec: Vec<u8>,
        target_len: usize,
    ) -> Result<Option<Vec<u8>>, Error> {
        if target_len > data_vec.len() {
            data_vec.resize(target_len, 0);
        }
        match self
            .connection
            .stream_recv(BACKGROUND_STREAM_ID, &mut data_vec[..target_len])
        {
            Ok((bytes_read, is_finished)) => {
                if !is_finished {
                    #[allow(clippy::comparison_chain)]
                    if bytes_read == target_len {
                        Ok(Some(data_vec))
                    } else if bytes_read < target_len {
                        self.bkgd_recv.captured = bytes_read;
                        self.bkgd_recv.target = target_len;
                        self.bkgd_recv.data = Some(data_vec);
                        Ok(None)
                    } else {
                        Err(Error::InvalidState)
                    }
                } else {
                    Err(Error::Done)
                }
            }
            Err(Error::Done) => {
                self.bkgd_recv.captured = 0;
                self.bkgd_recv.target = target_len;
                self.bkgd_recv.data = Some(data_vec);
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}
