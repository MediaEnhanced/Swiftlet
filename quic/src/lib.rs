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

// SocketAddr structure expected for programs to use when interfacing with this library
pub use std::net::SocketAddr;

use std::time::{Duration, Instant};

pub mod endpoint;
use endpoint::{ConnectionId, Endpoint, EndpointError, EndpointEvent};

pub enum Error {
    Unexpected,
    EndpointError,
}

pub trait Events {
    fn connection_started(&mut self, endpoint: &mut Endpoint, cid: &ConnectionId);
    fn connection_ended(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        remaining_connections: usize,
    ) -> bool;
    fn connection_ending_warning(&mut self, endpoint: &mut Endpoint, cid: &ConnectionId);
    fn tick(&mut self, endpoint: &mut Endpoint) -> bool;
    fn debug_text(&mut self, text: &'static str);

    fn main_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        read_data: &[u8],
    ) -> Option<usize>;

    fn background_stream_recv(
        &mut self,
        endpoint: &mut Endpoint,
        cid: &ConnectionId,
        read_data: &[u8],
    ) -> Option<usize>;
}

pub struct Handler<'a> {
    current_tick: u64,
    endpoint: Endpoint,
    events: &'a mut dyn Events,
}

impl<'a> Handler<'a> {
    pub fn new(endpoint: Endpoint, events: &'a mut dyn Events) -> Self {
        Handler {
            current_tick: 0,
            endpoint,
            events,
        }
    }

    // Returns true if the thread should maybe call this event loop again (ie. new Server to connect to via commands)
    pub fn run_event_loop(
        &mut self,
        tick_duration: Duration,
    ) -> Result<Option<&mut Endpoint>, Error> {
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
                Ok(EndpointEvent::ConnectionClosed(cid)) => {
                    let remaining_connections = self.endpoint.get_num_connections();
                    if self
                        .events
                        .connection_ended(&mut self.endpoint, &cid, remaining_connections)
                    {
                        return Ok(Some(&mut self.endpoint));
                    }
                }
                Ok(EndpointEvent::ConnectionClosing(_cid)) => {
                    // Do nothing right now...?
                }
                Ok(EndpointEvent::AlreadyHandled) => {
                    // Do Nothing and try to call get_next_event ASAP
                }
                Err(_) => {
                    self.events.debug_text("Event Loop Endpoint Error");
                }
                _ => {
                    self.events.debug_text("Unexpected Event Ok 1\n");
                    return Err(Error::Unexpected);
                }
            }
        }
    }

    fn run_recv_loop(&mut self) -> Result<bool, Error> {
        loop {
            match self.endpoint.recv() {
                Ok(EndpointEvent::DoneReceiving) => {
                    return Ok(false);
                }
                Ok(EndpointEvent::MainStreamReceived(cid)) => loop {
                    match self.endpoint.main_stream_recv(&cid) {
                        Ok((read_bytes_option, vec_option)) => {
                            if let Some(read_bytes) = read_bytes_option {
                                let vec_data = match vec_option {
                                    Some(v) => v,
                                    None => {
                                        vec![0; 4096] // Backup and shouldn't ever be called
                                    }
                                };
                                if let Some(next_recv_target) = self.events.main_stream_recv(
                                    &mut self.endpoint,
                                    &cid,
                                    &vec_data[..read_bytes],
                                ) {
                                    let _ = self.endpoint.main_stream_set_target(
                                        &cid,
                                        next_recv_target,
                                        vec_data,
                                    );
                                } else {
                                    let _ = self.endpoint.close_connection(&cid, 16);
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        Err(_) => {
                            self.events.debug_text("Stream Read Error!\n");
                            break;
                        }
                    }
                },
                Ok(EndpointEvent::BackgroundStreamReceived(cid)) => loop {
                    match self.endpoint.background_stream_recv(&cid) {
                        Ok((read_bytes_option, vec_option)) => {
                            if let Some(read_bytes) = read_bytes_option {
                                let vec_data = match vec_option {
                                    Some(v) => v,
                                    None => {
                                        vec![0; 4096] // Backup and shouldn't ever be called
                                    }
                                };
                                if let Some(next_recv_target) = self.events.background_stream_recv(
                                    &mut self.endpoint,
                                    &cid,
                                    &vec_data[..read_bytes],
                                ) {
                                    let _ = self.endpoint.background_stream_set_target(
                                        &cid,
                                        next_recv_target,
                                        vec_data,
                                    );
                                } else {
                                    let _ = self.endpoint.close_connection(&cid, 16);
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        Err(_) => {
                            self.events.debug_text("Stream Read Error!\n");
                            break;
                        }
                    }
                },
                Ok(EndpointEvent::ConnectionClosed(cid)) => {
                    let remaining_connections = self.endpoint.get_num_connections();
                    if self
                        .events
                        .connection_ended(&mut self.endpoint, &cid, remaining_connections)
                    {
                        return Ok(true);
                    }
                }
                Ok(EndpointEvent::ConnectionClosing(_cid)) => {
                    // Do nothing right now...?
                }
                Ok(EndpointEvent::EstablishedOnce(cid)) => {
                    self.events.connection_started(&mut self.endpoint, &cid);
                }
                Ok(EndpointEvent::NoUpdate) => {
                    // Do nothing and call recv again
                }
                Err(e) => {
                    match e {
                        EndpointError::StreamRecv => self.events.debug_text("Stream Recv Error!\n"),
                        _ => self.events.debug_text("General Endpoint Recv Error!\n"),
                    }
                    return Err(Error::EndpointError);
                }
                Ok(_) => {
                    self.events.debug_text("Unexpected Event Ok 2\n");
                    return Err(Error::Unexpected);
                }
            }
        }
    }
}
