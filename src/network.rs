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


const SERVER_NAME: &str = "localhost"; // Server "Name" / Domain Name that should ideally be on the server certificate that the client connects to
const ALPN_NAME: &[u8] = b"networking_audio"; // Application-Layer Protocol Negotiation Name used to define the Quic-Prog(ram) Protocol used in this application
const CERT_PATH: &str = "security/cert.pem"; // Location of the certificate for the server to use (temporarily used by client to verify server)
const PKEY_PATH: &str = "security/pkey.pem"; // Location of the private key for the server to use

const MAX_DATAGRAM_SIZE: usize = 1232; // IPv6 defines a min of 1280 bytes which after IPv6 and UDP headers shrinks to 1232 bytes
// More info can be found here: https://datatracker.ietf.org/doc/html/rfc9000#name-datagram-size
// Quic header reduces max payload data length to ____ bytes
const MAX_SERVER_CONNS: u64 = 32; // Max connections to the server at any given point
const MAIN_STREAM_ID: u64 = 0; // Bidirectional stream ID# used for reliable communication in the application between the server and the client (has to be zero for quiche)

use crate::communication::{ // Use Inter-Thread Communication Definitions
	Sender, Receiver, NetworkThreadChannels, 
	ConsoleCommands, NetworkStateMessage, NetworkStateConnection,
	//NetworkAudioOutputChannels, NetworkAudioPackets
};

pub use std::net::{Ipv6Addr, SocketAddrV6};
use std::{thread, time::{Duration, Instant}}; // IPv6 Addresses and Sockets used when sending the client an initial connection address

mod manage;
use manage::{
	ServerManager, ClientManager, UpdateEvent,
};

#[repr(u8)]
pub enum StreamCommands {
	InvalidCommand,
	ServerStateRefresh, // NumClientsConnected, ClientIndex, ServerNameLen, ServerName, {ClientXNameLen, ClientXName, ClientXState}... 0
	NewClient, // ClientNamLen, ClientName, ClientState
	StateChange, // ClientIndex, ClientState
	NewClientRequest, // ClientNamLen, ClientName
	StateChangeRequest, // ProbableIndex, NewStateRequest
	ClientKeepAlive
}

fn u8_to_str(data: &[u8]) -> String {
	let str_local = match std::str::from_utf8(data) {
		Ok(s) => s,
		Err(err) => {
			let index = err.valid_up_to();
			match std::str::from_utf8(&data[..index]) {
				Ok(s) => s,
				Err(_) => { // Should never happen
					return String::new();
				}
			}
		}
	};
	str_local.to_string()
}

const MAX_CHAR_LENGTH: usize = 32;

struct ClientState {
	id: u64,
	user_name: [u8; MAX_CHAR_LENGTH * 4],
	user_name_len: usize,
	state: u8, // Bit State [reserved, serverMusicConnected, voiceChatConnected, voiceChatLoopback]
}

struct ServerState {
	name: [u8; MAX_CHAR_LENGTH * 4],
	name_len: usize,
	client_states: Vec<ClientState>
}

impl ServerState {
	fn new(server_name: String) -> Self {
		let mut name = [0; 128];
		let mut name_len = 0;

		for (c_ind, c) in server_name.chars().enumerate() {
			if c_ind >= MAX_CHAR_LENGTH {
				break;
			}

			let new_name_len = name_len + c.len_utf8();
			let name_subslice = &mut name[name_len..new_name_len];
			c.encode_utf8(name_subslice);
			name_len = new_name_len;
		}

		if name_len == 0 {
			name[0] = b'S';
			name_len = 1;
		}

		ServerState {
			name,
			name_len,
			client_states: Vec::new()
		}
	}

	#[inline]
	fn find_client_state_index(&self, cs_id: u64) -> Option<usize> {
		self.client_states.iter().position(|cs| cs.id == cs_id)
	}

	#[inline]
	fn find_client_state_index_with_probable(&self, cs_id: u64, probable_index: usize) -> Option<usize> {
		if probable_index < self.client_states.len() && self.client_states[probable_index].id == cs_id {
			Some(probable_index)
		}
		else {
			self.client_states.iter().position(|cs| cs.id == cs_id)
		}
	}

	fn add_client_state(&mut self, cs_id: u64, user_name: &[u8]) -> Option<usize> {		
		if self.find_client_state_index(cs_id).is_none() {
			let mut name = [0; 128];
			let mut name_len = 0;

			let name_str = match std::str::from_utf8(user_name) {
				Ok(s) => s,
				Err(err) => {
					let index = err.valid_up_to();
					match std::str::from_utf8(&user_name[..index]) {
						Ok(s) => s,
						Err(err) => {
							return None;
						}
					}
				}
			};

			for (c_ind, c) in name_str.chars().enumerate() {
				if c_ind >= MAX_CHAR_LENGTH {
					break;
				}

				let new_name_len = name_len + c.len_utf8();
				let name_subslice = &mut name[name_len..new_name_len];
				c.encode_utf8(name_subslice);
				name_len = new_name_len;
			}

			if name_len == 0 {
				return None;
			}

			let mut client_state = ClientState {
				id: cs_id,
				user_name: name,
				user_name_len: name_len,
				state: 0
			};
			self.client_states.push(client_state);

			Some(self.client_states.len() - 1)
		}
		else {
			None
		}
	}

	fn remove_client_state(&mut self, cs_id: u64) -> bool {
		if let Some(index) = self.find_client_state_index(cs_id) {
			self.client_states.remove(index);
			true
		}
		else {
			false
		}
	}

	fn create_refresh_data(&self) -> Vec<u8> {
		let data_command = [StreamCommands::ServerStateRefresh as u8, self.client_states.len() as u8, 255];
		let mut data_buffer = Vec::from(data_command);

		data_buffer.push(self.name_len as u8);
		data_buffer.extend_from_slice(&self.name[.. self.name_len]);

		for cs in &self.client_states {
			data_buffer.push(cs.user_name_len as u8);
			data_buffer.extend_from_slice(&cs.user_name[.. cs.user_name_len]);
			data_buffer.push(cs.state);
		}
		data_buffer.push(0);

		data_buffer
	}

	fn create_new_client_data(&self, verified_index: usize, data: &mut [u8]) -> Option<usize> {
		let cs = &self.client_states[verified_index];
		let expected_len = 3 + cs.user_name_len;

		if data.len() < expected_len {
			return None;
		}

		data[0] = StreamCommands::NewClient as u8;
		data[1] = cs.user_name_len as u8;

		let state_position = cs.user_name_len + 2;
		for (ind, d) in data[2..state_position].iter_mut().enumerate() {
			*d = cs.user_name[ind];
		}
		data[state_position] = cs.state;

		Some(expected_len)
	}

	fn create_state_change_data(&self, verified_index: usize, data: &mut [u8]) -> Option<usize> {
		if data.len() < 3 {
			return None;
		}

		let cs = &self.client_states[verified_index];
		
		data[0] = StreamCommands::StateChange as u8;
		data[1] = verified_index as u8;
		data[2] = cs.state;

		Some(3)
	}

	fn refresh_update(&self, network_state_send: &Sender<NetworkStateMessage>) {
		let mut state_populate = Vec::<NetworkStateConnection>::new();
		
		for cs in &self.client_states {
			let conn_state = NetworkStateConnection {
				name: u8_to_str(&cs.user_name[.. cs.user_name_len]),
				state: cs.state
			};
			state_populate.push(conn_state);
		}

		let state_update = NetworkStateMessage::ConnectionsRefresh(state_populate);
		let _ = network_state_send.send(state_update);
	}

	fn new_connection_update(&self, verified_index: usize, network_state_send: &Sender<NetworkStateMessage>) {
		let cs = &self.client_states[verified_index];
		let conn_name = u8_to_str(&cs.user_name[.. cs.user_name_len]);
		let state_update = NetworkStateMessage::NewConnection((conn_name, cs.state));
		let _ = network_state_send.send(state_update);
	}

	fn state_change_update(&self, verified_index: usize, network_state_send: &Sender<NetworkStateMessage>) {
		let cs = &self.client_states[verified_index];
		let state_update = NetworkStateMessage::StateChange((verified_index, cs.state));
		let _ = network_state_send.send(state_update);
	}

}

pub fn server_thread(port: u16, server_name: String, channels: NetworkThreadChannels) {
	let local_addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);
	let bind_addr = manage::SocketAddr::V6(SocketAddrV6::new(local_addr, port, 0, 0));
	let mut server_mgr = match ServerManager::new(server_name.clone(), bind_addr, ALPN_NAME, CERT_PATH, PKEY_PATH) {
		Ok(ss) => ss,
		Err(err) => {
			let _ = channels.network_debug_send.send("Server state creation error!\n");
			return;
		}
	};

	let mut server_state = ServerState::new(server_name);
	let mut stream_read: [u8; 65536] = [0; 65536];

	let mut tick_duration = Duration::from_millis(5);
	let start_instant = Instant::now();
	let mut next_tick_instant = start_instant;
	let mut command_handler_ticks = 0;
	loop { // Master "Event" Loop
		
		match server_mgr.update() { // Sleeps when it can (ie. waiting for next tick / recv data and time is > 1ms)
			UpdateEvent::ReceivedData => {
				server_read_loop(&mut stream_read, &mut server_state, &mut server_mgr, &channels);
			},
			UpdateEvent::NextTick => {
				next_tick_instant += tick_duration; // Does not currently check for skipped ticks / assumes computer processes all
				server_mgr.set_next_tick_instant(next_tick_instant);

				// Eventually handle data that gets sent at set intervals
				command_handler_ticks += 1;
				if command_handler_ticks >= 100 { // Handle Commands Every 100 Ticks (0.5 sec)
					//let _ = channels.network_debug_send.send("Server Command Handling\n");
					if server_command_handler(&mut server_mgr, &channels.command_recv) {
						break;
					}
					command_handler_ticks = 0;
				}
			},
			UpdateEvent::PotentiallyReceivedData => {
				server_read_loop(&mut stream_read, &mut server_state, &mut server_mgr, &channels);
			},
			UpdateEvent::ConnectionClosed(conn_id) => {
				if server_state.remove_client_state(conn_id) {
					let mut refresh_data = server_state.create_refresh_data();
					for (cs_ind, cs) in server_state.client_states.iter().enumerate() {
						refresh_data[2] = cs_ind as u8;
						//let _ = channels.network_debug_send.send("Server Send Close Refresh\n");
						server_mgr.send_stream_data(cs.id, MAIN_STREAM_ID, &refresh_data, false);
					}
					server_state.refresh_update(&channels.network_state_send);
				}
			},
			_ => {

			}
		}
	}

	// Eventual Friendly Server Cleanup Here

	let _ = channels.network_debug_send.send("Server Network Thread Exiting\n");
}


pub fn client_thread(server_address: SocketAddrV6, user_name: String, channels: NetworkThreadChannels) {
	let bind_addr = "[::]:0".parse().unwrap();
	let peer_addr = manage::SocketAddr::V6(server_address);
	let mut client_mgr = match ClientManager::new(user_name.clone(), bind_addr, peer_addr, ALPN_NAME, CERT_PATH, SERVER_NAME) {
		Ok(cs) => cs,
		Err(err) => {
			let _ = channels.network_debug_send.send("Client state creation error!\n");
			return;
		}
	};

	let mut stream_read: [u8; 65536] = [0; 65536];
	let mut tick_duration = Duration::from_millis(5);
	let mut command_handler_ticks = 0;
	let mut keep_alive_ticks = 0;
	loop { // Master "Event" Loop
		
		let start_instant = Instant::now();
		let mut next_tick_instant = start_instant;
		let mut new_connection_potential = true;
		loop {
			match client_mgr.update() { // Sleeps when it can (ie. waiting for next tick / recv data and time is > 1ms)
				UpdateEvent::ReceivedData => {
					if client_read_loop(&mut stream_read, &user_name, &mut client_mgr, &channels) {
						break;
					}
				},
				UpdateEvent::NextTick => {
					next_tick_instant += tick_duration; // Does not currently check for skipped ticks / assumes computer processes all
					client_mgr.set_next_tick_instant(next_tick_instant);
	
					// Eventually handle data that gets sent at set intervals
					keep_alive_ticks += 1;
					if keep_alive_ticks >= 200 { // Send a Keep Alive every 200 Ticks (1 sec)
						let keep_alive_data = [StreamCommands::ClientKeepAlive as u8, 0];
						client_mgr.send_stream_data(MAIN_STREAM_ID, &keep_alive_data, false);
					}

					command_handler_ticks += 1;
					if command_handler_ticks >= 4 { // Handle Commands Every 4 Ticks (20 ms)
						if client_command_handler(&mut client_mgr, true, &channels.command_recv, &channels.network_debug_send) {
							new_connection_potential = false;
							break;
						}
						command_handler_ticks = 0;
					}
					
				},
				UpdateEvent::PotentiallyReceivedData => {
					if client_read_loop(&mut stream_read, &user_name, &mut client_mgr, &channels) {
						break;
					}
				},
				UpdateEvent::ConnectionClosed(conn_id) => {
					break;
				},
				_ => {
	
				}
			}
		}
		
		if new_connection_potential {
			loop {
				thread::sleep(Duration::from_millis(100));
				if client_command_handler(&mut client_mgr, false, &channels.command_recv, &channels.network_debug_send) {
					break;
				}
			}
			if client_mgr.is_connection_closed() {
				break;
			}
		}
		else {
			break;
		}
		
	}

	// Eventual Friendly Client Cleanup Here

	let _ = channels.network_debug_send.send("Client Network Thread Exiting\n");
}


fn server_read_loop(stream_read: &mut [u8], server_state: &mut ServerState, server_mgr: &mut ServerManager, channels: &NetworkThreadChannels) {
	loop {
		match server_mgr.recv_data(stream_read) {
			UpdateEvent::StreamReceivedData(recv_data_info) => {
				if recv_data_info.stream_id == MAIN_STREAM_ID {
					server_process_main_stream_data(&stream_read[..recv_data_info.data_size], recv_data_info.conn_id, server_mgr, server_state,
						&channels.network_state_send, &channels.network_debug_send);
				}
			},
			UpdateEvent::FinishedReceiving => {
				break;
			},
			UpdateEvent::ConnectionClosed(conn_id) => {
				if server_state.remove_client_state(conn_id) {
					let mut refresh_data = server_state.create_refresh_data();
					for (cs_ind, cs) in server_state.client_states.iter().enumerate() {
						refresh_data[2] = cs_ind as u8;
						server_mgr.send_stream_data(cs.id, MAIN_STREAM_ID, &refresh_data, false);
					}
					server_state.refresh_update(&channels.network_state_send);
				}
			},	
			UpdateEvent::NoUpdate => {
				// NO break
			},
			UpdateEvent::FinishedConnectingOnce(_) => {

			},
			UpdateEvent::NewConnectionStarted(_) => {
				// NO break
			},
			_ => { // Some form of error
				let _ = channels.network_debug_send.send("Server Manager Recv Error!\n");
				break;
			}
		}
	}
}

fn client_read_loop(stream_read: &mut [u8], user_name: &str, client_mgr: &mut ClientManager, channels: &NetworkThreadChannels) -> bool {
	loop {
		match client_mgr.recv_data(stream_read) {
			UpdateEvent::StreamReceivedData(recv_data_info) => {
				if recv_data_info.stream_id == MAIN_STREAM_ID {
					client_process_main_stream_data(&stream_read[..recv_data_info.data_size], client_mgr, 
						&channels.network_state_send, &channels.network_debug_send);
				}
			},
			UpdateEvent::FinishedReceiving => {
				break;
			},
			UpdateEvent::ConnectionClosed(_) => {
				let _ = channels.network_debug_send.send("Client Connection Closed!\n");
				return true;
			},
			UpdateEvent::FinishedConnectingOnce(_) => {
				client_mgr.create_stream(MAIN_STREAM_ID, 100);

				let mut write_buffer = [0; 256];
				write_buffer[0] = StreamCommands::NewClientRequest as u8;
				
				let mut start_index = 2;
				for (c_ind, c) in user_name.chars().enumerate() {
					if c_ind >= MAX_CHAR_LENGTH {
						break;
					}
	
					let new_start_index = start_index + c.len_utf8();
					let c_subslice = &mut write_buffer[start_index..new_start_index];
					c.encode_utf8(c_subslice);
					start_index = new_start_index;
				}
				write_buffer[1] = (start_index - 2) as u8;
				
				client_mgr.send_stream_data(MAIN_STREAM_ID, &write_buffer[..start_index], false);
			},
			UpdateEvent::NoUpdate => {
				// NO break
			},
			_ => { // Some form of error
				let _ = channels.network_debug_send.send("Client Manager Recv Error!\n");
				break;
			}
		}
	}
	false
}

fn server_process_main_stream_data(recv_data: &[u8], conn_id: u64, server_mgr: &mut ServerManager, server_state: &mut ServerState,
	state_send: &Sender<NetworkStateMessage>, debug_send: &Sender<&'static str>) { // Needs better error handling
	if recv_data[0] == StreamCommands::NewClientRequest as u8 {
		let client_name_len = recv_data[1] as usize;
		if client_name_len > 0 && client_name_len <= (MAX_CHAR_LENGTH * 4) {
			if let Some(index) = server_state.add_client_state(conn_id, &recv_data[2.. client_name_len+2]) {
				let mut refresh_data = server_state.create_refresh_data();
				refresh_data[2] = index as u8;
				server_mgr.send_stream_data(conn_id, MAIN_STREAM_ID, &refresh_data, false);

				let mut write_buffer = [0; 256];
				if let Some(len) = server_state.create_new_client_data(index, &mut write_buffer) {
					for cs in server_state.client_states.iter() {
						if cs.id != conn_id {
							server_mgr.send_stream_data(cs.id, MAIN_STREAM_ID, &write_buffer[..len], false);
						}
					}
				}

				server_state.new_connection_update(index, state_send);
			}
		}
	}
	else if recv_data[0] == StreamCommands::StateChangeRequest as u8 {
		let probable_index = recv_data[1] as usize;
		if let Some(cs_index) = server_state.find_client_state_index_with_probable(conn_id, probable_index) {
			let potential_new_state = recv_data[2];
			// In future check if server will allow state change here!
			server_state.client_states[cs_index].state = potential_new_state;

			let mut write_buffer = [0; 16];
			if let Some(len) = server_state.create_state_change_data(cs_index, &mut write_buffer) {
				for cs in server_state.client_states.iter() {
					server_mgr.send_stream_data(cs.id, MAIN_STREAM_ID, &write_buffer[..len], false);
				}
			}

			server_state.state_change_update(cs_index, state_send);
		}
	}
}

fn client_process_main_stream_data(recv_data: &[u8], client_mgr: &mut ClientManager, state_send: &Sender<NetworkStateMessage>, debug_send: &Sender<&'static str>) {
	if recv_data[0] == StreamCommands::ServerStateRefresh as u8 { // Valid connection
		let _ = debug_send.send("Server Refresh Recv\n");
		
		let conn_ind = recv_data[2] as usize;

		let mut name_end: usize = (recv_data[3] + 4).into();
		let mut server_name = u8_to_str(&recv_data[4..name_end]);
		let name_update = NetworkStateMessage::ServerNameChange(server_name);
		let _ = state_send.send(name_update);

		let mut state_populate = Vec::<NetworkStateConnection>::new();

		let mut name_len: usize = recv_data[name_end].into();
		while name_len != 0 {
			let name_start = name_end + 1;
			name_end = name_len + name_start;
			let mut client_name = u8_to_str(&recv_data[name_start..name_end]);

			let conn_state = NetworkStateConnection {
				name: client_name,
				state: recv_data[name_end]
			};

			state_populate.push(conn_state);

			name_end += 1;
			name_len = recv_data[name_end].into();
		}

		let state_update = NetworkStateMessage::ConnectionsRefresh(state_populate);
		let _ = state_send.send(state_update);

		let index_update = NetworkStateMessage::SetConnectionIndex(conn_ind);
		let _ = state_send.send(index_update);
	}
	else if recv_data[0] == StreamCommands::NewClient as u8 { // Valid connection
		//let _ = network_debug_send.send("New Client with Name\n");
		
		let mut name_end: usize = (recv_data[1] + 2).into();
		let mut client_name = u8_to_str(&recv_data[2..name_end]);
		let new_conn = NetworkStateMessage::NewConnection((client_name, recv_data[name_end]));
		let _ = state_send.send(new_conn);
	}
	else if recv_data[0] == StreamCommands::StateChange as u8 { // Valid connection
		//let _ = network_debug_send.send("State Change Recv\n");
		
		let conn_pos = recv_data[1] as usize;
		let new_state = recv_data[2];
		
		let new_conn = NetworkStateMessage::StateChange((conn_pos, new_state));
		let _ = state_send.send(new_conn);
	}
}


fn server_command_handler(server_state: &mut ServerManager, command_recv: &Receiver<ConsoleCommands>) -> bool {
	loop {
		match command_recv.try_recv() {
			Err(try_recv_error) => {
				// match try_recv_error {
				// 	TryRecvError::Empty => {
				// 		//break;
				// 	},
				// 	TryRecvError::Disconnected => {
				// 		//break;
				// 	}
				// }
				break;
			},
			Ok(cmd_recv) => {
				match cmd_recv {
					ConsoleCommands::NetworkingStop(int) => {
						//endpoint.close(quinn::VarInt::from_u64(int).unwrap(), b"shutdown");
						return true;
					},
					ConsoleCommands::ServerConnectionClose(int) => {

					},
					_ => {

					}
				}
			}
		}
	}
	false
}

fn client_command_handler(client_mgr: &mut ClientManager, connected: bool, command_recv: &Receiver<ConsoleCommands>, debug_send: &Sender<&'static str>) -> bool {
	loop {
		match command_recv.try_recv() {
			Err(try_recv_error) => {
				// match try_recv_error {
				// 	TryRecvError::Empty => {
				// 		//break;
				// 	},
				// 	TryRecvError::Disconnected => {
				// 		//break;
				// 	}
				// }
				break;
			},
			Ok(cmd_recv) => {
				if connected {
					match cmd_recv {
						ConsoleCommands::ClientStateChange((prob_ind, state)) => {
							let data = [StreamCommands::StateChangeRequest as u8, prob_ind, state];
							client_mgr.send_stream_data(MAIN_STREAM_ID, &data, false);
							//let _ = debug_send.send("State Change Update!\n");
						},
						ConsoleCommands::ClientConnectionClose => {
							client_mgr.close_connection(42);
						},
						ConsoleCommands::NetworkingStop(int) => {
							client_mgr.close_connection(int);
							return true;
						},
						_ => {
	
						}
					}
				}
				else {
					match cmd_recv {
						ConsoleCommands::ClientReconnect(server_address) => {
							let peer_addr = manage::SocketAddr::V6(server_address);
							return client_mgr.new_connection(peer_addr,  ALPN_NAME, CERT_PATH, SERVER_NAME);
						},
						ConsoleCommands::NetworkingStop(int) => {
							return true;
						},
						_ => {
	
						}
					}
				}
				
			}
		}
	}
	false
}

