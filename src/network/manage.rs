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

pub use std::net::SocketAddr;

use std::time::Instant;
use std::collections::BinaryHeap;

use ring::rand::*;

struct DelayedSendPacket {
	data: [u8; MAX_DATAGRAM_SIZE],
	data_len: usize,
	to: SocketAddr,
	instant: Instant
}

impl Ord for DelayedSendPacket {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		#[allow(clippy::comparison_chain)] // Clippy was saying to use match with a cmp here instead... lol THIS is the definition of cmp
		if self.instant > other.instant {
			std::cmp::Ordering::Less
		}
		else if self.instant < other.instant {
			std::cmp::Ordering::Greater
		}
		else {
			std::cmp::Ordering::Equal
		}
    }
}

impl PartialOrd for DelayedSendPacket {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for DelayedSendPacket {
    fn eq(&self, other: &Self) -> bool {
        self.instant == other.instant
    }
}

impl Eq for DelayedSendPacket {}

enum SocketError {
	SendSizeWrong,
	SendBlocked,
	SendOtherIssue,
	RecvBlocked,
	RecvOtherIssue
}

struct SocketManager {
	poll: mio::Poll,
	events: mio::Events,
	socket: mio::net::UdpSocket,
	send_queue: BinaryHeap<DelayedSendPacket>
}

impl SocketManager {
	fn new(bind_addr: SocketAddr) -> (Self, SocketAddr) {
		let socket = mio::net::UdpSocket::bind(bind_addr).unwrap();
		let local_addr = socket.local_addr().unwrap();
		let mut socket_state = SocketManager {
			poll: mio::Poll::new().unwrap(),
			events: mio::Events::with_capacity(1024),
			socket,
			send_queue: BinaryHeap::new()
		};
		socket_state.poll.registry().register(&mut socket_state.socket, mio::Token(0), mio::Interest::READABLE).unwrap();

		(socket_state, local_addr)
	}

	fn sleep_till_recv_data(&mut self, timeout: std::time::Duration) -> bool {
		self.poll.poll(&mut self.events, Some(timeout)).unwrap();
		!self.events.is_empty()
	}

	#[inline]
	fn send_data(&mut self, data: &[u8], to_addr: SocketAddr) -> Option<SocketError> {
		// Drops packet before it enters network stack if it would block
		// Uncertain if it will partially fill socket (could even be OS dependent)
		match self.socket.send_to(data, to_addr) {
			Ok(send_size) => {
				if send_size != data.len() {
					Some(SocketError::SendSizeWrong)
				}
				else {
					None
				}
			},
			Err(err) => {
				if err.kind() == std::io::ErrorKind::WouldBlock {
					Some(SocketError::SendBlocked)
				}
				else {
					Some(SocketError::SendOtherIssue)
				}
			}
		}
	}

	#[inline]
	fn recv_data(&mut self, data: &mut [u8]) -> Result<(usize, SocketAddr), SocketError> {
		match self.socket.recv_from(data) {
			Ok((recv_size, addr_from)) => {
				Ok((recv_size, addr_from))
			},
			Err(err) => {
				if err.kind() == std::io::ErrorKind::WouldBlock {
					Err(SocketError::RecvBlocked)
				}
				else {
					Err(SocketError::RecvOtherIssue)
				}
			}
		}
	}
}


const MAX_DATAGRAM_SIZE: usize = 1232; // IPv6 defines a min of 1280 bytes which after IPv6 and UDP headers shrinks to 1232 bytes
// More info can be found here: https://datatracker.ietf.org/doc/html/rfc9000#name-datagram-size
// Quic header reduces max payload data length to ____ bytes

struct ConnectionManager {
	id: u64,
	dcid_matcher: quiche::ConnectionId<'static>,
	connection: quiche::Connection,
	recv_info: quiche::RecvInfo,
	next_timeout_instant: Option<Instant>,
	connected_once: bool
}

impl ConnectionManager {
	fn new(server_name: Option<&str>, id: u64, scid: quiche::ConnectionId<'static>, local_addr: SocketAddr, peer_addr: SocketAddr, config: &mut quiche::Config,
		writer_opt: Option<Box<std::fs::File>>) -> Result<Self, quiche::Error> {
		
		let recv_info = quiche::RecvInfo {
			from: local_addr,
			to: local_addr
		};
		
		if server_name.is_some() {
			let connection = match quiche::connect(server_name, &scid, local_addr, peer_addr, config) {
				Ok(conn) => {
					conn
				},
				Err(err) => {
					return Err(err);
				}
			};

			let next_timeout_instant = connection.timeout_instant();

			let conn_mgr = ConnectionManager {
				id,
				dcid_matcher: scid,
				connection,
				recv_info,
				next_timeout_instant,
				connected_once: false
			};

			Ok(conn_mgr)
		}
		else {
			let connection = match quiche::accept(&scid, None, local_addr, peer_addr, config) {
				Ok(mut conn) => {
					
					if let Some(writer) = writer_opt { // called before recv
						conn.set_keylog(writer);
					}
					conn
				},
				Err(err) => {
					return Err(err);
				}
			};

			let next_timeout_instant = connection.timeout_instant();

			let conn_mgr = ConnectionManager {
				id,
				dcid_matcher: scid,
				connection,
				recv_info,
				next_timeout_instant,
				connected_once: false
			};

			Ok(conn_mgr)
		}
	}

	fn send_data(&mut self, socket_mgr: &mut SocketManager) -> Result<u64, quiche::Error> {
		let mut num_sends = 0;		
		loop {
			let mut next_send_packet = [0; MAX_DATAGRAM_SIZE];
			match self.connection.send(&mut next_send_packet) {
				Ok((write_len, send_info)) => {
					if send_info.at <= Instant::now() {
						match socket_mgr.send_data(&next_send_packet[..write_len], send_info.to) {
							None => num_sends += 1,
							Some(err) => {
								return Err(quiche::Error::Done); // Better Error Handling in Future
							}
						}
					}
					else {
						let delayed_send_packet = DelayedSendPacket {
							data: next_send_packet,
							data_len: write_len,
							to: send_info.to,
							instant: send_info.at
						};
						socket_mgr.send_queue.push(delayed_send_packet);
						num_sends += 1;
					}
				},
				Err(quiche::Error::Done) => {
					self.next_timeout_instant = self.connection.timeout_instant();
					return Ok(num_sends);
				},
				Err(err) => {
					return Err(err);
				}
			}
		}
	}

	fn recv_data(&mut self, data: &mut [u8], addr_from: SocketAddr, socket_mgr: &mut SocketManager) -> Result<u64, quiche::Error> {
		self.recv_info.from = addr_from;

		// Does it handle potentially coalesced packets?
		match self.connection.recv(data, self.recv_info) {
			Ok(read_size) => {
				self.send_data(socket_mgr) //to handle ACKs
			},
			Err(err) => {
				Err(err)
			}
		}
	}

}

fn create_quiche_config(alpn: &[u8], cert_path: &str, pkey_path_option: Option<&str>, dgram_queue_len_option: Option<usize>) -> Result<quiche::Config, quiche::Error> {
	let mut config = match quiche::Config::new(quiche::PROTOCOL_VERSION) {
		Ok(cfg) => {
			cfg // A quiche Config with default values
		},
		Err(err) => {
			return Err(err);
		}
	};

	if let Some(pkey_path) = pkey_path_option {
		config.load_cert_chain_from_pem_file(cert_path)?;

		config.load_priv_key_from_pem_file(pkey_path)?;
		config.verify_peer(false);
		config.set_initial_max_streams_bidi(1); // Should be 1 here for server?
		
		// Enable datagram frames for unreliable realtime data to be sent
		//let dgram_queue_len = MAX_DATAGRAM_SIZE * (MAX_SERVER_CONNS as usize) * 2;
		config.log_keys();
	}
	else {
		config.load_verify_locations_from_file(cert_path)?; // Temporary solution for client to verify certificate
		
		config.verify_peer(true);
		config.set_initial_max_streams_bidi(1);

		//let dgram_queue_len = MAX_DATAGRAM_SIZE * (MAX_SERVER_CONNS as usize) || MAX_DATAGRAM_SIZE;
	}

	// Enable datagram frames for unreliable realtime data to be sent
	if let Some(dgram_queue_len) = dgram_queue_len_option {
		config.enable_dgram(true, dgram_queue_len * 10, dgram_queue_len);
	}

	config.set_application_protos(&[alpn]);
	
	config.set_max_idle_timeout(5000); // Use a timeout of infinite when this line is commented out

    config.set_max_recv_udp_payload_size(MAX_DATAGRAM_SIZE);
    config.set_max_send_udp_payload_size(MAX_DATAGRAM_SIZE);
    config.set_initial_max_data(16_777_216); // 16 MiB
    config.set_initial_max_stream_data_bidi_local(2_097_152); // 2 MiB
    config.set_initial_max_stream_data_bidi_remote(2_097_152); // 2 MiB

    config.set_initial_max_streams_uni(3);
	config.set_initial_max_stream_data_uni(2_097_152); // 2 MiB

    config.set_disable_active_migration(true); // Temporary

	Ok(config)
}

enum NextInstantType {
	NextTick,
	DelayedSend,
	ConnectionTimeout(usize) // usize index always valid...? double check logic later
}

pub struct StreamRecvData {
	pub conn_id: u64,
	pub stream_id: u64,
	pub data_size: usize,
	pub is_finished: bool
}

pub enum UpdateEvent {
	NoUpdate,
	NextTick,
	ReceivedData,
	PotentiallyReceivedData,
	FinishedReceiving,
	SocketManagerError,
	ConnectionManagerError,
	NewConnectionStarted(u64),
	FinishedConnectingOnce(u64),
	ConnectionClosed(u64),
	StreamReceivedData(StreamRecvData),
	StreamReceivedError,
}

pub struct ServerManager {
	name: String,
	next_connection_id: u64,
	socket_mgr: SocketManager,
	read_data: [u8; 65536],
	rand: SystemRandom,
	conn_id_seed: ring::hmac::Key,
	config: quiche::Config,
	local_addr: SocketAddr,
	connections: Vec::<ConnectionManager>,
	next_tick_instant: Instant
}

impl ServerManager {
	pub fn new(name: String, bind_addr: SocketAddr, alpn: &[u8], cert_path: &str, pkey_path: &str) -> Result<Self, quiche::Error> {
		let (socket_mgr, local_addr) = SocketManager::new(bind_addr);
		
		let rand = SystemRandom::new();
		
		let config = match create_quiche_config(alpn, cert_path, Some(pkey_path), Some(3)) {
			Ok(cfg) => {
				cfg
			},
			Err(err) => {
				return Err(err);
			}
		};

		let conn_id_seed = ring::hmac::Key::generate(ring::hmac::HMAC_SHA256, &rand).unwrap();

		let server_state = ServerManager {
			name,
			next_connection_id: 1,
			socket_mgr,
			read_data: [0; 65536],
			rand,
			conn_id_seed,
			config,
			local_addr,
			connections: Vec::new(),
			next_tick_instant: Instant::now()
		};

		Ok(server_state)
	}

	fn add_connection(&mut self, dcid: &quiche::ConnectionId, peer_addr: SocketAddr) -> Result<usize, quiche::Error> {
		let tag = ring::hmac::sign(&self.conn_id_seed, dcid);
		let conn_id = &tag.as_ref()[..quiche::MAX_CONN_ID_LEN];
		let scid = quiche::ConnectionId::from_ref(conn_id);

		let conn_mgr = match ConnectionManager::new(None, self.next_connection_id, scid.into_owned(), self.local_addr, peer_addr, &mut self.config, None) {
			Ok(cm) => {
				cm
			},
			Err(err) => {
				return Err(err);
			}
		};

		self.next_connection_id += 1;
		self.connections.push(conn_mgr);

		Ok(self.connections.len() - 1)
	}

	fn remove_connection(&mut self, conn_ind: usize) -> u64 {
		let id = self.connections[conn_ind].id;
		self.connections.remove(conn_ind);
		id
	}

	fn get_next_instant(&self) -> (Instant, NextInstantType) {
		let mut next_instant = self.next_tick_instant;
		let mut next_instant_type = NextInstantType::NextTick; 
		
		if let Some(delayed_send_packet) = self.socket_mgr.send_queue.peek() {
			if delayed_send_packet.instant < next_instant {
				next_instant = delayed_send_packet.instant;
				next_instant_type = NextInstantType::DelayedSend;
			}
		}

		for (conn_ind, conn) in self.connections.iter().enumerate() {
			if let Some(conn_timeout) = conn.connection.timeout_instant() {
				if conn_timeout < next_instant {
					next_instant = conn_timeout;
					next_instant_type = NextInstantType::ConnectionTimeout(conn_ind);
				}
			}
		}

		(next_instant, next_instant_type)
	}

	fn send_check(&mut self) {
		while let Some(delayed_send_packet) = self.socket_mgr.send_queue.peek() {
			if delayed_send_packet.instant <= Instant::now() {
				let sp = self.socket_mgr.send_queue.pop();
				match sp {
					Some(send_packet) => {
						self.socket_mgr.send_data(&send_packet.data[..send_packet.data_len], send_packet.to);
					},
					None => {
						break; // How...?
					}
				}
			}
			else {
				break;
			}
		}
	}

	fn handle_connection_timeout(&mut self, conn_ind: usize) -> bool {
		if let Some(current_connection_timeout) = self.connections[conn_ind].connection.timeout_instant() {
			if current_connection_timeout <= Instant::now() {
				self.connections[conn_ind].connection.on_timeout();
				self.connections[conn_ind].send_data(&mut self.socket_mgr); // Resets timeout instance internally
				self.connections[conn_ind].connection.is_closed()
			}
			else {
				self.connections[conn_ind].next_timeout_instant = Some(current_connection_timeout);
				false
			}
		}
		else {
			self.connections[conn_ind].next_timeout_instant = None; // Changed to another value after a send (or recv?)
			false
		}
	}

	pub fn update(&mut self) -> UpdateEvent {
		let (mut next_instant, mut ni_type) = self.get_next_instant();
		while next_instant <= Instant::now() {
			match ni_type {
				NextInstantType::NextTick => {
					return UpdateEvent::NextTick;
				},
				NextInstantType::DelayedSend => {
					self.send_check();
				},
				NextInstantType::ConnectionTimeout(conn_ind) => {
					if self.handle_connection_timeout(conn_ind) {
						let conn_id = self.remove_connection(conn_ind);
						return UpdateEvent::ConnectionClosed(conn_id);
					}
				}
			}
			(next_instant, ni_type) = self.get_next_instant();
		}

		let sleep_duration = next_instant.duration_since(Instant::now());
		if sleep_duration.as_millis() > 0 && self.socket_mgr.sleep_till_recv_data(sleep_duration) {
			return UpdateEvent::ReceivedData;
		}

		UpdateEvent::PotentiallyReceivedData
	}

	pub fn set_next_tick_instant(&mut self, next_tick_instant: Instant) {
		self.next_tick_instant = next_tick_instant;
	}

	pub fn recv_data(&mut self, stream_recv_data: &mut [u8]) -> UpdateEvent {
		match self.socket_mgr.recv_data(&mut self.read_data) {
			Ok((recv_size, addr_from)) => {
				if recv_size <= MAX_DATAGRAM_SIZE { // Only look at datagram if it is less than or equal to the max
					let recv_packet = &mut self.read_data[..recv_size];
					if let Ok(packet_header) = quiche::Header::from_slice(recv_packet, quiche::MAX_CONN_ID_LEN) {
						if let Some(conn_ind) = self.connections.iter().position(|conn| conn.dcid_matcher == packet_header.dcid) {
							
							
							let conn_mgr = &mut self.connections[conn_ind];
							
							conn_mgr.recv_data(recv_packet, addr_from, &mut self.socket_mgr);
							if conn_mgr.connected_once {			
								if conn_mgr.connection.is_closed() {
									let removed_id = self.remove_connection(conn_ind);
									return UpdateEvent::ConnectionClosed(removed_id);
								}
								else if let Some(next_readable_stream) = conn_mgr.connection.stream_readable_next() {
									match conn_mgr.connection.stream_recv(next_readable_stream, stream_recv_data) {
										Ok((data_size, is_finished)) => {
											
											conn_mgr.send_data(&mut self.socket_mgr);
											
											let recv_data = StreamRecvData {
												conn_id: conn_mgr.id,
												stream_id: next_readable_stream,
												data_size,
												is_finished
											};
											
											return UpdateEvent::StreamReceivedData(recv_data);
										},
										Err(err) => {
											return UpdateEvent::StreamReceivedError;
										}
									}
								}
							}
							else if conn_mgr.connection.is_established() {
								conn_mgr.connected_once = true;
								return UpdateEvent::FinishedConnectingOnce(conn_mgr.id);
							}
							else if conn_mgr.connection.is_closed() {
								self.remove_connection(conn_ind);
							}
						}
						else if packet_header.ty == quiche::Type::Initial && quiche::version_is_supported(packet_header.version) { // New Connection
							if let Ok(conn_ind) = self.add_connection(&packet_header.dcid, addr_from) {
								self.connections[conn_ind].recv_data(&mut self.read_data[..recv_size], addr_from, &mut self.socket_mgr); // Does not handle errors
								return UpdateEvent::NewConnectionStarted(self.connections[conn_ind].id);
							}
						}
					}
				}
				UpdateEvent::NoUpdate
			},
			Err(SocketError::RecvBlocked) => {
				UpdateEvent::FinishedReceiving
			},
			Err(_) => {
				UpdateEvent::SocketManagerError
			}
		}
	}

	pub fn create_stream(&mut self, conn_id: u64, stream_id: u64, priority: u8) -> Result<(), quiche::Error> {
		// Assumes that is called only after connection is established
		if let Some(conn_ind) = self.connections.iter().position(|conn| conn.id == conn_id) {
			self.connections[conn_ind].connection.stream_priority(stream_id, priority, true)
		}
		else {
			Ok(())
		}
	}

	pub fn send_stream_data(&mut self, conn_id: u64, stream_id: u64, data: &[u8], is_final: bool) -> Result<u64, quiche::Error> {
		
		if let Some(conn_ind) = self.connections.iter().position(|conn| conn.id == conn_id) {
			// Do connection checking here in future
			let conn_mgr = &mut self.connections[conn_ind];
			match conn_mgr.connection.stream_send(stream_id, data, is_final) {
				Ok(bytes_written) => {
					if bytes_written == data.len() {
						match conn_mgr.send_data(&mut self.socket_mgr) {
							Ok(num_sends) => {
								Ok(num_sends)
							},
							Err(err) => {
								Err(err)
							}
						}
					}
					else {
						Err(quiche::Error::BufferTooShort)
					}
				},
				Err(err) => {
					Err(err)
				}
			}
		}
		else {
			Ok(0)
		}
		
	}
	
}

pub struct ClientManager {
	user_name: String,
	socket_mgr: SocketManager,
	read_data: [u8; 65536],
	rand: SystemRandom,
	config: quiche::Config,
	conn_mgr: ConnectionManager,
	next_tick_instant: Instant
}

impl ClientManager {
	pub fn new(user_name: String, bind_addr: SocketAddr, peer_addr: SocketAddr, alpn: &[u8], cert_path: &str, server_name: &str) -> Result<Self, quiche::Error> {
		let (mut socket_mgr, local_addr) = SocketManager::new(bind_addr);
		
		let rand = SystemRandom::new();
		
		let mut config = match create_quiche_config(alpn, cert_path, None, Some(3)) {
			Ok(cfg) => {
				cfg
			},
			Err(err) => {
				return Err(err);
			}
		};
		
		
		let mut scid_ref = [0; quiche::MAX_CONN_ID_LEN];
		rand.fill(&mut scid_ref[..]).unwrap();
		let scid = quiche::ConnectionId::from_ref(&scid_ref);

		let mut conn_mgr = match ConnectionManager::new(Some(server_name), 1, scid.into_owned(), local_addr, peer_addr, &mut config, None) {
			Ok(cm) => {
				cm
			},
			Err(err) => {
				return Err(err);
			}
		};
		conn_mgr.send_data(&mut socket_mgr);

		let client_state = ClientManager {
			user_name,
			socket_mgr,
			read_data: [0; 65536],
			rand,
			config,
			conn_mgr,
			next_tick_instant: Instant::now(),
		};

		Ok(client_state)
	}

	pub fn close_connection(&mut self, error_code: u64) {
		self.conn_mgr.connection.close(false, error_code, b"reason");
		self.conn_mgr.send_data(&mut self.socket_mgr);
	}

	pub fn is_connection_closed(&self) -> bool {
		self.conn_mgr.connection.is_closed()
	}

	pub fn new_connection(&mut self, peer_addr: SocketAddr, alpn: &[u8], cert_path: &str, server_name: &str) -> bool {
		if self.conn_mgr.connection.is_closed() {
			let mut scid_ref = [0; quiche::MAX_CONN_ID_LEN];
			self.rand.fill(&mut scid_ref[..]).unwrap();
			let scid = quiche::ConnectionId::from_ref(&scid_ref);

			let local_addr = self.conn_mgr.recv_info.to;
			let mut conn_mgr = match ConnectionManager::new(Some(server_name), 1, scid.into_owned(), local_addr, peer_addr, &mut self.config, None) {
				Ok(cm) => {
					cm
				},
				Err(err) => {
					return false;
				}
			};
			conn_mgr.send_data(&mut self.socket_mgr);

			self.conn_mgr = conn_mgr;

			true
		}
		else {
			false
		}
	}

	fn get_next_instant(&self) -> (Instant, NextInstantType) {
		let mut next_instant = self.next_tick_instant;
		let mut next_instant_type = NextInstantType::NextTick; 
		
		if let Some(delayed_send_packet) = self.socket_mgr.send_queue.peek() {
			if delayed_send_packet.instant < next_instant {
				next_instant = delayed_send_packet.instant;
				next_instant_type = NextInstantType::DelayedSend;
			}
		}

		if let Some(conn_timeout) = self.conn_mgr.connection.timeout_instant() {
			if conn_timeout < next_instant {
				next_instant = conn_timeout;
				next_instant_type = NextInstantType::ConnectionTimeout(1);
			}
		}

		(next_instant, next_instant_type)
	}

	fn send_check(&mut self) {
		while let Some(delayed_send_packet) = self.socket_mgr.send_queue.peek() {
			if delayed_send_packet.instant <= Instant::now() {
				let sp = self.socket_mgr.send_queue.pop();
				match sp {
					Some(send_packet) => {
						self.socket_mgr.send_data(&send_packet.data[..send_packet.data_len], send_packet.to);
					},
					None => {
						break; // How...?
					}
				}
			}
			else {
				break;
			}
		}
	}

	fn handle_connection_timeout(&mut self) -> bool {
		if let Some(current_connection_timeout) = self.conn_mgr.connection.timeout_instant() {
			if current_connection_timeout <= Instant::now() {
				self.conn_mgr.connection.on_timeout();
				self.conn_mgr.send_data(&mut self.socket_mgr); // Resets timeout instance internally
				self.conn_mgr.connection.is_closed()
			}
			else {
				self.conn_mgr.next_timeout_instant = Some(current_connection_timeout);
				false
			}
		}
		else {
			self.conn_mgr.next_timeout_instant = None; // Changed to another value after a send (or recv?)
			false
		}
	}

	pub fn update(&mut self) -> UpdateEvent {
		let (mut next_instant, mut ni_type) = self.get_next_instant();
		while next_instant <= Instant::now() {
			match ni_type {
				NextInstantType::NextTick => {
					return UpdateEvent::NextTick;
				},
				NextInstantType::DelayedSend => {
					self.send_check();
				},
				NextInstantType::ConnectionTimeout(_) => {
					if self.handle_connection_timeout() {
						return UpdateEvent::ConnectionClosed(1);
					}
				}
			}
			(next_instant, ni_type) = self.get_next_instant();
		}

		let sleep_duration = next_instant.duration_since(Instant::now());
		if sleep_duration.as_millis() > 0 && self.socket_mgr.sleep_till_recv_data(sleep_duration) {
			return UpdateEvent::ReceivedData;
		}

		UpdateEvent::PotentiallyReceivedData
	}

	pub fn set_next_tick_instant(&mut self, next_tick_instant: Instant) {
		self.next_tick_instant = next_tick_instant;
	}

	pub fn recv_data(&mut self, stream_recv_data: &mut [u8]) -> UpdateEvent {
		match self.socket_mgr.recv_data(&mut self.read_data) {
			Ok((recv_size, addr_from)) => {
				if recv_size <= MAX_DATAGRAM_SIZE { // Only look at datagram if it is less than or equal to the max
					match self.conn_mgr.recv_data(&mut self.read_data[..recv_size], addr_from, &mut self.socket_mgr) {
						Ok(num_sends) => {
							if self.conn_mgr.connected_once {			
								if self.conn_mgr.connection.is_closed() {
									return UpdateEvent::ConnectionClosed(1);
								}
								else if let Some(next_readable_stream) = self.conn_mgr.connection.stream_readable_next() {
									match self.conn_mgr.connection.stream_recv(next_readable_stream, stream_recv_data) {
										Ok((data_size, is_finished)) => {
											self.conn_mgr.send_data(&mut self.socket_mgr);

											let recv_data = StreamRecvData {
												conn_id: 1,
												stream_id: next_readable_stream,
												data_size,
												is_finished
											};
											
											return UpdateEvent::StreamReceivedData(recv_data);
										},
										Err(err) => {
											return UpdateEvent::StreamReceivedError;
										}
									}
								}
							}
							else if self.conn_mgr.connection.is_established() {
								self.conn_mgr.connected_once = true;
								return UpdateEvent::FinishedConnectingOnce(1);
							}
							else if self.conn_mgr.connection.is_closed() {
								return UpdateEvent::ConnectionClosed(1);
							}
							UpdateEvent::NoUpdate
						},
						Err(_) => {
							UpdateEvent::ConnectionManagerError
						}
					}
				}
				else {
					UpdateEvent::NoUpdate
				}
			},
			Err(SocketError::RecvBlocked) => {
				UpdateEvent::FinishedReceiving
			},
			Err(_) => {
				UpdateEvent::SocketManagerError
			}
		}
	}

	pub fn create_stream(&mut self, stream_id: u64, priority: u8) -> Result<(), quiche::Error> {
		self.conn_mgr.connection.stream_priority(stream_id, priority, true)
	}

	pub fn send_stream_data(&mut self, stream_id: u64, data: &[u8], is_final: bool) -> Result<u64, quiche::Error> {
		// Do connection checking here in future
		match self.conn_mgr.connection.stream_send(stream_id, data, is_final) {
			Ok(bytes_written) => {
				if bytes_written == data.len() {
					match self.conn_mgr.send_data(&mut self.socket_mgr) {
						Ok(num_sends) => {
							Ok(num_sends)
						},
						Err(err) => {
							Err(err)
						}
					}
				}
				else {
					Err(quiche::Error::BufferTooShort)
				}
			},
			Err(err) => {
				Err(err)
			}
		}
	}

	

}



