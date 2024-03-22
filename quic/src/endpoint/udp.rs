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

// UDP Management Intended for use with QUIC

use crate::endpoint::SocketAddr;

#[cfg_attr(target_os = "windows", path = "udp/windows.rs")]
#[cfg_attr(target_os = "linux", path = "udp/mio.rs")]
#[cfg_attr(target_os = "macos", path = "udp/mio.rs")]
mod os;
//use os::{AudioInput, AudioOutput, AudioOwner};

use std::collections::BinaryHeap;
use std::time::Instant;

#[allow(dead_code)]
pub(super) const MAX_UDP_LENGTH: usize = 65536;

// QUIC defines a minimum UDP maximum datagram(payload) size of 1200 bytes for both IPv4 and IPv6
//  https://datatracker.ietf.org/doc/html/rfc9000#name-datagram-size
//pub const MIN_MAX_DATAGRAM_SIZE: usize = 1200;
// The target maximum datagram size is based on the IPv6 standard minimum of 1280 bytes (that cannot be fragmented)
//  which after the non-extended IPv6 and UDP headers are subtracted becomes 1232 bytes
// Modern IPv4 networks SHOULD be able to handle this target max datagram size (need source links HERE)
pub(super) const TARGET_MAX_DATAGRAM_SIZE: usize = 1232;

// UDP Socket Manager (Using the mio crate)
pub(super) struct Socket {
    os_socket: os::UdpSocket,
    delayed_sends: BinaryHeap<DelayedSendPacket>,
}

#[derive(Debug)]
pub(super) enum SocketError {
    CouldNotCreate,
    BadLocalAddress,
    RecvBlocked,
}

impl Socket {
    pub(super) fn new(ipv6_mode: bool, bind_port: u16) -> Result<(Self, SocketAddr), SocketError> {
        let os_socket = match os::UdpSocket::new(ipv6_mode, bind_port) {
            Some(s) => s,
            None => return Err(SocketError::CouldNotCreate),
        };

        let local_addr = match os_socket.get_local_address() {
            Some(la) => la,
            None => return Err(SocketError::BadLocalAddress),
        };

        let socket = Socket {
            os_socket,
            delayed_sends: BinaryHeap::new(),
        };

        Ok((socket, local_addr))
    }

    // Returns true if there is data to be read
    // It should capture "missed" events between calls and return without delay in this case
    #[inline]
    pub(super) fn sleep_till_recv_data(&mut self, timeout_duration: std::time::Duration) -> bool {
        // Possible timeout_duration parameter (safety) check here in future
        self.os_socket.sleep_till_next_recv(timeout_duration)
    }

    #[inline]
    pub(super) fn get_next_recv_data(&mut self) -> Result<(&mut [u8], SocketAddr), SocketError> {
        match self.os_socket.get_next_recv() {
            Some((recv_size, addr_from)) => Ok((recv_size, addr_from)),
            None => Err(SocketError::RecvBlocked),
        }
    }

    #[inline]
    pub(super) fn done_with_recv_data(&mut self) {
        self.os_socket.done_with_recv();
    }

    #[inline]
    pub(super) fn get_next_send_data(&mut self) -> &mut [u8] {
        self.os_socket.get_next_send()
    }

    pub(super) fn done_with_send_data(
        &mut self,
        to_addr: SocketAddr,
        len: usize,
        instant: Instant,
    ) -> Result<bool, SocketError> {
        if instant <= Instant::now() {
            self.os_socket.done_with_send(to_addr, len);
            Ok(true)
        } else {
            let delayed_send_packet = DelayedSendPacket {
                data: self.os_socket.get_next_send()[..TARGET_MAX_DATAGRAM_SIZE]
                    .try_into()
                    .unwrap(),
                data_len: len,
                to_addr,
                instant,
            };
            self.delayed_sends.push(delayed_send_packet);

            Ok(false)
        }
    }

    #[inline]
    pub(super) fn next_send_instant(&self) -> Option<Instant> {
        self.delayed_sends
            .peek()
            .map(|delayed_send_packet| delayed_send_packet.instant)
    }

    pub(super) fn send_check(&mut self) -> Result<u64, SocketError> {
        let mut sends = 0;
        while let Some(delayed_send_packet) = self.delayed_sends.peek() {
            if delayed_send_packet.instant <= Instant::now() {
                let next_send = self.os_socket.get_next_send();
                next_send[..delayed_send_packet.data_len]
                    .copy_from_slice(&delayed_send_packet.data[..delayed_send_packet.data_len]);
                self.os_socket
                    .done_with_send(delayed_send_packet.to_addr, delayed_send_packet.data_len);
                sends += 1;
                self.delayed_sends.pop();
            } else {
                return Ok(sends);
            }
        }
        Ok(sends)
    }
}

// A delayed send packet contains data that is sent from the socket only AFTER an Instant is reached
struct DelayedSendPacket {
    data: [u8; TARGET_MAX_DATAGRAM_SIZE],
    data_len: usize,
    to_addr: SocketAddr,
    instant: Instant,
}

// In order to compare delayed send packets to find the highest priority (lowest Instant) the Ord trait is implemented
impl Ord for DelayedSendPacket {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        #[allow(clippy::comparison_chain)]
        // Clippy was saying to use match with a cmp here instead... lol THIS is the definition of cmp
        if self.instant > other.instant {
            std::cmp::Ordering::Less
        } else if self.instant < other.instant {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    }
}

// The Ord trait requires PartialOrd and Eq be implemented as well
impl PartialOrd for DelayedSendPacket {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for DelayedSendPacket {}

// The Eq trait requires PartialEq be implemented as well
impl PartialEq for DelayedSendPacket {
    fn eq(&self, other: &Self) -> bool {
        self.instant == other.instant
    }
}
