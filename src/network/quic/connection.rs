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

use std::net::SocketAddr;
use std::time::Instant;

pub(super) use quiche::Config;
pub(super) use quiche::Error;

// QUIC Connection Manager (Using the quiche crate)
pub(super) struct ConnectionManager {
    id: u64, // ID to be used by the application (once defined)
    current_scid: quiche::ConnectionId<'static>, // Current SCID used by this connection
    connection: quiche::Connection,
    recv_info: quiche::RecvInfo,
    next_timeout_instant: Option<Instant>,
    connected_once: bool,
}

pub(super) enum Status {
    Uncertain,
    Closed(u64),
    Established(u64),
    StreamReadable((u64, u64)),
}

impl ConnectionManager {
    pub(super) fn get_default_config() -> Result<quiche::Config, Error> {
        quiche::Config::new(quiche::PROTOCOL_VERSION)
    }

    #[inline]
    pub(super) fn get_empty_cid() -> [u8; quiche::MAX_CONN_ID_LEN] {
        [0; quiche::MAX_CONN_ID_LEN]
    }

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

            let next_timeout_instant = connection.timeout_instant();

            let conn_mgr = ConnectionManager {
                id,
                current_scid,
                connection,
                recv_info,
                next_timeout_instant,
                connected_once: false,
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

            let next_timeout_instant = connection.timeout_instant();

            let conn_mgr = ConnectionManager {
                id,
                current_scid,
                connection,
                recv_info,
                next_timeout_instant,
                connected_once: false,
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

    #[inline]
    pub(super) fn get_next_send_packet(
        &mut self,
        packet_data: &mut [u8],
    ) -> Result<Option<(usize, SocketAddr, Instant)>, Error> {
        match self.connection.send(packet_data) {
            Ok((packet_len, send_info)) => Ok(Some((packet_len, send_info.to, send_info.at))),
            Err(quiche::Error::Done) => Ok(None),
            Err(e) => Err(e),
        }
    }

    #[inline]
    pub(super) fn get_timeout_instant(&self) -> Option<Instant> {
        self.next_timeout_instant
    }

    #[inline]
    pub(super) fn new_timeout_instant(&mut self) {
        self.next_timeout_instant = self.connection.timeout_instant();
    }

    // Returns true if a timeout occurred
    #[inline]
    pub(super) fn handle_timeout(&mut self) -> bool {
        // Verifies that a timeout occurred and then processes it
        self.next_timeout_instant = self.connection.timeout_instant();
        if let Some(current_connection_timeout) = self.next_timeout_instant {
            if current_connection_timeout <= Instant::now() {
                self.connection.on_timeout();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    #[inline]
    pub(super) fn recv_data_process(
        &mut self,
        data: &mut [u8],
        from_addr: SocketAddr,
    ) -> Result<usize, Error> {
        self.recv_info.from = from_addr;
        self.connection.recv(data, self.recv_info)
        //let result = ;
        //self.next_timeout_instant = self.get_timeout_instant();
        //result
    }

    pub(super) fn get_status(&mut self) -> Status {
        if self.connected_once {
            if self.connection.is_closed() {
                Status::Closed(self.id)
            } else if let Some(next_readable_stream) = self.connection.stream_readable_next() {
                Status::StreamReadable((self.id, next_readable_stream))
            } else {
                Status::Uncertain
            }
        } else if self.connection.is_established() {
            self.connected_once = true;
            Status::Established(self.id)
        } else if self.connection.is_closed() {
            Status::Closed(self.id)
        } else {
            Status::Uncertain
        }
    }

    #[inline]
    pub(super) fn close(&mut self, err: u64, reason: &[u8]) -> Result<bool, Error> {
        self.connection.close(false, err, reason)?;
        Ok(true)
    }

    #[inline]
    pub(super) fn create_stream(&mut self, stream_id: u64, urgency: u8) -> Result<bool, Error> {
        self.connection.stream_priority(stream_id, urgency, true)?;
        Ok(true)
    }

    #[inline]
    pub(super) fn stream_send(
        &mut self,
        stream_id: u64,
        data: &[u8],
        fin: bool,
    ) -> Result<usize, Error> {
        self.connection.stream_send(stream_id, data, fin)
    }

    #[inline]
    pub(super) fn stream_recv(
        &mut self,
        stream_id: u64,
        data: &mut [u8],
    ) -> Result<(usize, bool), Error> {
        self.connection.stream_recv(stream_id, data)
    }
}
