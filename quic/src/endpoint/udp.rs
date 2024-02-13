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

// UDP Management Intended for use with QUIC
use std::collections::BinaryHeap;

use std::time::Instant;

pub(super) const MAX_UDP_LENGTH: usize = 65536;

// QUIC defines a minimum UDP maximum datagram(payload) size of 1200 bytes for both IPv4 and IPv6
//  https://datatracker.ietf.org/doc/html/rfc9000#name-datagram-size
//pub(super) const MIN_MAX_DATAGRAM_SIZE: usize = 1200;
// The target maximum datagram size is based on the IPv6 standard minimum of 1280 bytes (that cannot be fragmented)
//  which after the non-extended IPv6 and UDP headers are subtracted becomes 1232 bytes
// Modern IPv4 networks SHOULD be able to handle this target max datagram size (need source links HERE)
pub(super) const TARGET_MAX_DATAGRAM_SIZE: usize = 1232;

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

// UDP Socket Manager (Using the mio crate)
pub(super) struct UdpSocket {
    poll: mio::Poll,
    events: mio::Events,
    socket: mio::net::UdpSocket,
    read_data: [u8; MAX_UDP_LENGTH],
    //prev_recv_size: usize,
    //prev_recv_from: Option<SocketAddr>,
    packet: [u8; TARGET_MAX_DATAGRAM_SIZE],
    send_queue: BinaryHeap<DelayedSendPacket>,
}

pub(super) enum SocketError {
    CouldNotCreate,
    SendSizeWrong,
    SendBlocked,
    SendOtherIssue,
    RecvBlocked,
    RecvOtherIssue,
}

impl UdpSocket {
    pub(super) fn new(bind_addr: SocketAddr) -> Result<(Self, SocketAddr), SocketError> {
        let mut socket = match mio::net::UdpSocket::bind(bind_addr) {
            Ok(s) => s,
            Err(_) => return Err(SocketError::CouldNotCreate),
        };

        let local_addr = match socket.local_addr() {
            Ok(la) => la,
            Err(_) => return Err(SocketError::CouldNotCreate),
        };

        let poll = match mio::Poll::new() {
            Ok(p) => p,
            Err(_) => return Err(SocketError::CouldNotCreate),
        };

        match poll
            .registry()
            .register(&mut socket, mio::Token(0), mio::Interest::READABLE)
        {
            Ok(_) => {}
            Err(_) => return Err(SocketError::CouldNotCreate),
        }

        let socket_state = UdpSocket {
            poll,
            events: mio::Events::with_capacity(1024),
            socket,
            read_data: [0; MAX_UDP_LENGTH],
            //prev_recv_size: 0,
            //prev_recv_from: None,
            packet: [0; TARGET_MAX_DATAGRAM_SIZE],
            send_queue: BinaryHeap::new(),
        };

        Ok((socket_state, local_addr))
    }

    #[inline]
    pub(super) fn get_packet_data(&mut self) -> &mut [u8] {
        &mut self.packet
    }

    pub(super) fn send_packet(
        &mut self,
        len: usize,
        to_addr: SocketAddr,
        instant: Instant,
    ) -> Result<bool, SocketError> {
        if instant <= Instant::now() {
            // Drops packet before it enters network stack if it would block
            // Uncertain if it will partially fill socket (could even be OS dependent)
            match self.socket.send_to(&self.packet[..len], to_addr) {
                Ok(send_size) => {
                    if send_size != len {
                        Err(SocketError::SendSizeWrong)
                    } else {
                        Ok(true)
                    }
                }
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::WouldBlock {
                        Err(SocketError::SendBlocked)
                    } else {
                        Err(SocketError::SendOtherIssue)
                    }
                }
            }
        } else {
            //println!("Delayed Packet!");
            let delayed_send_packet = DelayedSendPacket {
                data: self.packet, // It copies it...?
                data_len: len,
                to_addr,
                instant,
            };
            self.send_queue.push(delayed_send_packet);

            Ok(false)
        }
    }

    #[inline]
    pub(super) fn next_send_instant(&self) -> Option<Instant> {
        self.send_queue
            .peek()
            .map(|delayed_send_packet| delayed_send_packet.instant)
    }

    pub(super) fn send_check(&mut self) -> Result<u64, SocketError> {
        let mut sends = 0;
        while let Some(delayed_send_packet) = self.send_queue.peek() {
            if delayed_send_packet.instant <= Instant::now() {
                // Drops packet before it enters network stack if it would block
                // Uncertain if it will partially fill socket (could even be OS dependent)
                match self.socket.send_to(
                    &delayed_send_packet.data[..delayed_send_packet.data_len],
                    delayed_send_packet.to_addr,
                ) {
                    Ok(send_size) => {
                        if send_size != delayed_send_packet.data_len {
                            return Err(SocketError::SendSizeWrong);
                        }
                        sends += 1;
                    }
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::WouldBlock {
                            return Err(SocketError::SendBlocked);
                        } else {
                            return Err(SocketError::SendOtherIssue);
                        }
                    }
                }
                self.send_queue.pop();
            } else {
                return Ok(sends);
            }
        }
        Ok(sends)
    }

    // Returns true if there is data to be read
    // It should capture "missed" events between calls and return without delay
    #[inline]
    pub(super) fn sleep_till_recv_data(&mut self, timeout: std::time::Duration) -> bool {
        match self.poll.poll(&mut self.events, Some(timeout)) {
            Ok(_) => !self.events.is_empty(),
            Err(_) => false,
        }
    }

    // #[inline]
    // pub(super) fn has_data_to_recv(&mut self) -> Result<bool, SocketError> {
    //     match self.socket.recv_from(&mut self.read_data) {
    //         Ok((recv_size, addr_from)) => {
    //             self.prev_recv_size = recv_size;
    //             self.prev_recv_from = Some(addr_from);
    //             Ok(true)
    //         }
    //         Err(err) => {
    //             if err.kind() == std::io::ErrorKind::WouldBlock {
    //                 Ok(false)
    //             } else {
    //                 Err(SocketError::RecvOtherIssue)
    //             }
    //         }
    //     }
    // }

    #[inline]
    pub(super) fn get_next_recv_data(&mut self) -> Result<(&mut [u8], SocketAddr), SocketError> {
        // if let Some(prev_addr_from) = self.prev_recv_from {
        //     self.prev_recv_from = None;
        //     //let recv_size = self.prev_recv_size;
        //     //self.prev_recv_size = 0;
        //     //Ok((&mut self.read_data[..recv_size], prev_addr_from))
        //     Ok((&mut self.read_data[..self.prev_recv_size], prev_addr_from))
        // } else {
        match self.socket.recv_from(&mut self.read_data) {
            Ok((recv_size, addr_from)) => Ok((&mut self.read_data[..recv_size], addr_from)),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    Err(SocketError::RecvBlocked)
                } else {
                    Err(SocketError::RecvOtherIssue)
                }
            }
        }
        // }
    }
}
