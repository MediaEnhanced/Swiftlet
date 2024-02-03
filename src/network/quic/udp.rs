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

// UDP Management Intended for use with QUIC
use std::collections::BinaryHeap;
use std::net::SocketAddr;
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
pub(super) struct SocketManager {
    poll: mio::Poll,
    events: mio::Events,
    socket: mio::net::UdpSocket,
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

impl SocketManager {
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

        let socket_state = SocketManager {
            poll,
            events: mio::Events::with_capacity(1024),
            socket,
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

    pub(super) fn send_check(&mut self) -> Result<(), SocketError> {
        while let Some(delayed_send_packet) = self.send_queue.peek() {
            if delayed_send_packet.instant <= Instant::now() {
                match self.send_queue.pop() {
                    Some(send_packet) => {
                        // Drops packet before it enters network stack if it would block
                        // Uncertain if it will partially fill socket (could even be OS dependent)
                        match self.socket.send_to(
                            &send_packet.data[..send_packet.data_len],
                            send_packet.to_addr,
                        ) {
                            Ok(send_size) => {
                                if send_size != send_packet.data_len {
                                    return Err(SocketError::SendSizeWrong);
                                }
                            }
                            Err(err) => {
                                if err.kind() == std::io::ErrorKind::WouldBlock {
                                    return Err(SocketError::SendBlocked);
                                } else {
                                    return Err(SocketError::SendOtherIssue);
                                }
                            }
                        }
                    }
                    None => {
                        return Ok(()); // Will this EVER be reached?
                    }
                }
            } else {
                return Ok(());
            }
        }
        Ok(())
    }

    // Returns true if there is data to be read
    #[inline]
    pub(super) fn sleep_till_recv_data(&mut self, timeout: std::time::Duration) -> bool {
        match self.poll.poll(&mut self.events, Some(timeout)) {
            Ok(_) => !self.events.is_empty(),
            Err(e) => false,
        }
    }

    pub(super) fn recv_data(
        &mut self,
        data: &mut [u8],
    ) -> Result<(usize, SocketAddr), SocketError> {
        match self.socket.recv_from(data) {
            Ok((recv_size, addr_from)) => Ok((recv_size, addr_from)),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    Err(SocketError::RecvBlocked)
                } else {
                    Err(SocketError::RecvOtherIssue)
                }
            }
        }
    }
}
