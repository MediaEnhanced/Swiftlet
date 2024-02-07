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

// IPv6 Addresses and Sockets used when sending the client an initial connection addresss

pub(crate) use std::net::SocketAddr;

use std::time::{Duration, Instant};

pub mod endpoint;
use endpoint::{Endpoint, EndpointError, EndpointEvent};

pub enum RtcQuicError {
    Unexpected,
    EndpointError,
}

pub trait RtcQuicEvents {
    fn connection_started(&mut self, endpoint: &mut Endpoint, conn_id: u64, verified_index: usize);
    fn connection_closing(&mut self, endpoint: &mut Endpoint, conn_id: u64);
    fn connection_closed(
        &mut self,
        endpoint: &mut Endpoint,
        conn_id: u64,
        remaining_connections: usize,
    ) -> bool;
    fn tick(&mut self, endpoint: &mut Endpoint) -> bool;
    fn debug_text(&mut self, text: &'static str);

    fn reliable_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        conn_id: u64,
        verified_index: usize,
        read_data: &[u8],
    ) -> Option<usize>;
    //fn stream_started(&mut self, conn_id: u64, verified_index: usize);
}

pub struct RtcQuicHandler<'a> {
    current_tick: u64,
    endpoint: Endpoint,
    events: &'a mut dyn RtcQuicEvents,
    read_data: [u8; 256],
}

impl<'a> RtcQuicHandler<'a> {
    pub fn new(endpoint: Endpoint, events: &'a mut dyn RtcQuicEvents) -> Self {
        RtcQuicHandler {
            current_tick: 0,
            endpoint,
            events,
            read_data: [0; 256],
        }
    }

    // Returns true if the thread should maybe call this event loop again (ie. new Server to connect to via commands)
    pub fn run_event_loop(
        &mut self,
        tick_duration: Duration,
    ) -> Result<Option<&mut Endpoint>, RtcQuicError> {
        let start_instant = Instant::now();
        let mut next_tick_instant = start_instant;

        loop {
            // This update sleeps when waiting for the next instant or recv udp data and the duration is >= 1ms
            match self.endpoint.get_next_event(next_tick_instant) {
                Ok(EndpointEvent::ReceivedData) => {
                    if self.run_recv_loop()? {
                        return Ok(Some(&mut self.endpoint));
                    }
                }
                Ok(EndpointEvent::NextTick) => {
                    next_tick_instant += tick_duration; // Does not currently check for skipped ticks / assumes computer processes all
                    self.current_tick += 1;

                    if self.events.tick(&mut self.endpoint) {
                        return Ok(None);
                    }
                }
                Ok(EndpointEvent::ConnectionClosing(conn_id)) => {
                    // Need to process event for when a connection has StartedClosing instead here in future
                    self.events.connection_closing(&mut self.endpoint, conn_id);
                }
                Ok(EndpointEvent::ConnectionClosed(conn_id)) => {
                    let remaining_connections = self.endpoint.get_num_connections();
                    if self.events.connection_closed(
                        &mut self.endpoint,
                        conn_id,
                        remaining_connections,
                    ) {
                        return Ok(Some(&mut self.endpoint));
                    }
                }
                Ok(EndpointEvent::AlreadyHandled) => {
                    // Do Nothing and try to call get_next_event ASAP
                }
                Err(_) => {
                    self.events.debug_text("Event Loop Endpoint Error");
                }
                _ => {
                    self.events.debug_text("Unexpected Event Ok 1\n");
                    return Err(RtcQuicError::Unexpected);
                }
            }
        }
    }

    fn run_recv_loop(&mut self) -> Result<bool, RtcQuicError> {
        loop {
            match self.endpoint.recv() {
                Ok(EndpointEvent::DoneReceiving) => {
                    return Ok(false);
                }
                Ok(EndpointEvent::ReliableStreamReceived((conn_id, verified_index))) => loop {
                    match self.endpoint.recv_reliable_stream_data(
                        conn_id,
                        verified_index,
                        &mut self.read_data,
                    ) {
                        Ok((read_bytes_option, vec_option)) => {
                            if let Some(read_bytes) = read_bytes_option {
                                if let Some(next_recv_target) = self.events.reliable_stream_recv(
                                    &mut self.endpoint,
                                    conn_id,
                                    verified_index,
                                    &self.read_data[..read_bytes],
                                ) {
                                    let _ = self.endpoint.set_reliable_stream_recv_target(
                                        conn_id,
                                        verified_index,
                                        next_recv_target,
                                    );
                                }
                            } else if let Some(data_vec) = vec_option {
                                if let Some(next_recv_target) = self.events.reliable_stream_recv(
                                    &mut self.endpoint,
                                    conn_id,
                                    verified_index,
                                    &data_vec,
                                ) {
                                    let _ = self.endpoint.set_reliable_stream_recv_target(
                                        conn_id,
                                        verified_index,
                                        next_recv_target,
                                    );
                                }
                            } else {
                                break;
                            }
                        }
                        Err(e) => {
                            self.events.debug_text("Stream Read Error!\n");
                            break;
                        }
                    }
                },
                Ok(EndpointEvent::ConnectionClosing(conn_id)) => {
                    // Need to process event for when a connection has StartedClosing instead here in future
                    self.events.connection_closing(&mut self.endpoint, conn_id);
                }
                Ok(EndpointEvent::ConnectionClosed(conn_id)) => {
                    let remaining_connections = self.endpoint.get_num_connections();
                    if self.events.connection_closed(
                        &mut self.endpoint,
                        conn_id,
                        remaining_connections,
                    ) {
                        return Ok(true);
                    }
                }
                Ok(EndpointEvent::EstablishedOnce((conn_id, verified_index))) => {
                    self.events
                        .connection_started(&mut self.endpoint, conn_id, verified_index);
                }
                Ok(EndpointEvent::NewConnectionStarted) => {
                    self.events.debug_text("New Connection!\n");
                }
                Ok(EndpointEvent::NoUpdate) => {
                    // Do nothing and call recv again
                }
                Err(e) => {
                    match e {
                        EndpointError::StreamRecv => self.events.debug_text("Stream Recv Error!\n"),
                        _ => self.events.debug_text("General Endpoint Recv Error!\n"),
                    }
                    return Err(RtcQuicError::EndpointError);
                }
                Ok(_) => {
                    self.events.debug_text("Unexpected Event Ok 2\n");
                    return Err(RtcQuicError::Unexpected);
                }
            }
        }
    }
}
