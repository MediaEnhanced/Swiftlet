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

// UDP Socket Manager (Using the mio crate)
pub(super) struct UdpSocket {
    is_ipv6: bool,
    is_server: bool,
    socket: mio::net::UdpSocket,
    poll: mio::Poll,
    events: mio::Events,
    read_data: [u8; super::MAX_UDP_LENGTH],
    packet: [u8; super::TARGET_MAX_DATAGRAM_SIZE],
}

impl UdpSocket {
    pub(super) fn new(ipv6_mode: bool, bind_port: u16) -> Option<Self> {
        let bind_addr = if ipv6_mode {
            SocketAddr::V6(std::net::SocketAddrV6::new(
                std::net::Ipv6Addr::UNSPECIFIED,
                bind_port,
                0,
                0,
            ))
        } else {
            SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::UNSPECIFIED,
                bind_port,
            ))
        };

        let mut socket = match mio::net::UdpSocket::bind(bind_addr) {
            Ok(s) => s,
            Err(_e) => return None,
        };

        let poll = match mio::Poll::new() {
            Ok(p) => p,
            Err(_e) => return None,
        };

        match poll
            .registry()
            .register(&mut socket, mio::Token(0), mio::Interest::READABLE)
        {
            Ok(_) => {}
            Err(_e) => return None,
        }

        let socket_state = UdpSocket {
            is_ipv6: ipv6_mode,
            is_server: bind_port != 0,
            socket,
            poll,
            events: mio::Events::with_capacity(1024),
            read_data: [0; super::MAX_UDP_LENGTH],
            packet: [0; super::TARGET_MAX_DATAGRAM_SIZE],
        };

        Some(socket_state)
    }

    pub(super) fn get_local_address(&self) -> Option<SocketAddr> {
        match self.socket.local_addr() {
            Ok(s) => Some(s),
            Err(_e) => None,
        }
    }

    pub(super) fn sleep_till_next_recv(&mut self, timeout_duration: std::time::Duration) -> bool {
        match self.poll.poll(&mut self.events, Some(timeout_duration)) {
            Ok(_) => !self.events.is_empty(),
            Err(_) => false,
        }
    }

    pub(super) fn get_next_recv(&mut self) -> Option<(&mut [u8], SocketAddr)> {
        match self.socket.recv_from(&mut self.read_data) {
            Ok((recv_size, addr_from)) => Some((&mut self.read_data[..recv_size], addr_from)),
            Err(e) => {
                let kind = e.kind();
                if kind == std::io::ErrorKind::WouldBlock {
                    None
                } else {
                    panic!("UDP Socket MIO Recv From Error: {:?}", kind);
                }
            }
        }
    }

    pub(super) fn done_with_recv(&mut self) {}

    pub(super) fn get_next_send(&mut self) -> &mut [u8] {
        &mut self.packet
    }

    pub(super) fn done_with_send(&mut self, address: SocketAddr, data_len: usize) {
        match self.socket.send_to(&self.packet[..data_len], address) {
            Ok(send_size) => {
                if send_size == data_len {
                    // Nothing
                } else {
                    panic!("UDP Socket MIO Send Size Wrong!");
                }
            }
            Err(e) => {
                let kind = e.kind();
                if kind == std::io::ErrorKind::WouldBlock {
                    panic!("UDP Socket MIO Send Blocked!");
                } else {
                    panic!("UDP Socket MIO Send To Error: {:?}", kind);
                }
            }
        }
    }
}
